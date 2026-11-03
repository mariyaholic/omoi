//! Bounded setup metadata in a reserved checkpoint, independent of catalogue schema.
use crate::{Error, Result};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
pub const KEY: &str = "omoi:setup:v1";
pub const PROTOCOL: u32 = 1;
pub const CLIENT_VERSION: &str = "0.2.0";
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    #[default]
    NotChosen,
    Skipped,
    Enabled,
    Stopped,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub protocol: u32,
    pub version: String,
    pub permission: bool,
}
impl Hello {
    pub fn compatible(&self) -> bool {
        let parts = self
            .version
            .split('.')
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>();
        self.protocol == PROTOCOL && parts.is_ok_and(|v| v.len() == 3 && v[0] == 0 && v[1] == 2)
    }
    pub fn validate(&self) -> Result<()> {
        if self.version.len() > 32
            || !self
                .version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
        {
            return Err(Error::Config("Invalid client version".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Setup {
    pub version: u32,
    pub generation: u64,
    pub capture_epoch: u64,
    pub binding: String,
    pub choice: Choice,
    pub step: String,
    pub background: bool,
    pub profiles: Vec<String>,
    pub hello: Option<Hello>,
    pub last_seen: Option<i64>,
    pub last_event: Option<i64>,
    pub last_processed: Option<i64>,
    pub revision: u64,
}
impl Default for Setup {
    fn default() -> Self {
        Self {
            version: 1,
            generation: 0,
            capture_epoch: 0,
            binding: String::new(),
            choice: Choice::NotChosen,
            step: "choose".into(),
            background: false,
            profiles: vec![],
            hello: None,
            last_seen: None,
            last_event: None,
            last_processed: None,
            revision: 0,
        }
    }
}
impl Setup {
    pub fn load(c: &Connection) -> Result<Self> {
        let value: Option<String> = c
            .query_row(
                "SELECT cursor FROM checkpoints WHERE source=?",
                [KEY],
                |r| r.get(0),
            )
            .optional()?;
        let s: Self = match value {
            None => Self::default(),
            Some(s) if s.len() <= 65536 => serde_json::from_str(&s).map_err(|_| {
                Error::Config("Invalid setup settings; restore a compatible backup".into())
            })?,
            _ => return Err(Error::Config("Setup settings too large".into())),
        };
        if s.version != 1 {
            return Err(Error::Config("Setup settings require a newer Omoi".into()));
        }
        Ok(s)
    }
    pub fn save(&self, c: &Connection) -> Result<()> {
        let text = serde_json::to_string(self).expect("setup serializable");
        if text.len() > 65536 {
            return Err(Error::Config("Setup selection too large".into()));
        }
        c.execute("INSERT INTO checkpoints(source,cursor) VALUES(?1,?2) ON CONFLICT(source) DO UPDATE SET cursor=excluded.cursor", rusqlite::params![KEY,text])?;
        Ok(())
    }
    pub fn client_state(&self, now: i64) -> &'static str {
        if self.choice != Choice::Enabled
            || self.last_seen.is_none_or(|at| now < at || now - at > 45)
        {
            return "unknown";
        }
        match &self.hello {
            None => "unknown",
            Some(h) if !h.compatible() => "incompatible",
            Some(h) if !h.permission => "permission_needed",
            Some(_) => "hello_observed",
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub generation: u64,
    pub choice: Option<Choice>,
    pub step: Option<String>,
    pub background: Option<bool>,
    pub profiles: Option<Vec<String>>,
}
pub fn change(c: &mut Connection, change: Change) -> Result<Setup> {
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut s = Setup::load(&tx)?;
    if s.generation != change.generation {
        return Err(Error::Config(
            "Setup changed in another window. Refresh and retry.".into(),
        ));
    }
    let consent_before = (s.choice, s.background);
    if let Some(step) = change.step {
        if ![
            "choose",
            "companion",
            "permission",
            "connect",
            "play",
            "bridge",
        ]
        .contains(&step.as_str())
        {
            return Err(Error::Config("Invalid setup step".into()));
        }
        s.step = step;
    }
    if let Some(profiles) = change.profiles {
        if profiles.len() > 256 || profiles.iter().any(|p| p.len() > 4096) {
            return Err(Error::Config("Profile scope too large".into()));
        }
        s.profiles = profiles;
    }
    if let Some(background) = change.background {
        s.background = background;
    }
    if let Some(choice) = change.choice {
        if choice != s.choice {
            s.hello = None;
            s.last_seen = None;
        }
        s.choice = choice;
    }
    if s.binding.is_empty() {
        s.binding = crate::ui::random_token()?;
    }
    if consent_before != (s.choice, s.background) {
        s.capture_epoch = s
            .capture_epoch
            .checked_add(1)
            .ok_or_else(|| Error::Config("Capture epoch exhausted".into()))?;
        s.hello = None;
        s.last_seen = None;
    }
    s.generation = s
        .generation
        .checked_add(1)
        .ok_or_else(|| Error::Config("Setup generation exhausted".into()))?;
    s.save(&tx)?;
    tx.commit()?;
    Ok(s)
}
pub fn observe(c: &mut Connection, hello: Hello) -> Result<Setup> {
    hello.validate()?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut s = Setup::load(&tx)?;
    if s.choice != Choice::Enabled {
        return Err(Error::Config("Enable playback capture in Omoi".into()));
    }
    s.hello = Some(hello);
    s.last_seen = Some(chrono::Utc::now().timestamp());
    s.save(&tx)?;
    tx.commit()?;
    Ok(s)
}

/// An OS lock proves app lifetime, including after a crash. A pathname alone does not.
pub fn app_lock(home: &std::path::Path) -> Result<std::fs::File> {
    let f = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(home.join("app.lock"))
        .map_err(|source| Error::Io {
            context: "app ownership",
            source,
        })?;
    f.try_lock().map_err(|_| {
        Error::Config("This catalogue is already open in Omoi. Use its existing window.".into())
    })?;
    Ok(f)
}
pub fn app_present(home: &std::path::Path) -> bool {
    let Ok(f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(home.join("app.lock"))
    else {
        return false;
    };
    matches!(f.try_lock(), Err(std::fs::TryLockError::WouldBlock))
}
