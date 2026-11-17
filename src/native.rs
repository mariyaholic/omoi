//! Native messaging: exact extension origins, bounded frames and catalogue binding.
use crate::{
    config::Config,
    paths::Paths,
    setup::{Hello, Setup},
    Error, Result,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
pub const MAX_FRAME: usize = 32768;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub home: std::path::PathBuf,
    pub allowed_origins: Vec<String>,
}
impl Registration {
    pub fn permits(&self, origin: &str) -> bool {
        self.home.is_absolute()
            && !self.allowed_origins.is_empty()
            && self.allowed_origins.len() <= 2
            && self.allowed_origins.iter().all(|s| valid_origin(s))
            && self.allowed_origins.iter().any(|s| s == origin)
    }
}
pub fn valid_origin(s: &str) -> bool {
    s.strip_prefix("chrome-extension://")
        .and_then(|s| s.strip_suffix('/'))
        .is_some_and(|s| s.len() == 32 && s.bytes().all(|b| (b'a'..=b'p').contains(&b)))
}
pub fn read_frame(reader: &mut impl Read) -> std::io::Result<Option<Value>> {
    let mut size = [0; 4];
    match reader.read(&mut size[..1])? {
        0 => return Ok(None),
        _ => reader.read_exact(&mut size[1..])?,
    }
    let size = u32::from_le_bytes(size) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid native frame size",
        ));
    }
    let mut body = vec![0; size];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid native JSON"))
}
pub fn write_frame(writer: &mut impl Write, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value)?;
    if body.len() > MAX_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Native reply too large",
        ));
    }
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Hello {
        protocol_min: u32,
        protocol_max: u32,
        version: String,
        permission: bool,
        binding: Option<String>,
    },
    Event {
        binding: String,
        generation: u64,
        event: Value,
    },
    Status,
}
pub struct Session {
    pub paths: Paths,
    config: Config,
    c: rusqlite::Connection,
    negotiated: Option<(String, u64)>,
}
impl Session {
    pub fn open(paths: Paths) -> Result<Self> {
        let mut config = Config::load(&paths.config)?;
        config.busy_timeout_ms = 250;
        if !paths.db.is_file() {
            return Err(Error::Config("Open Omoi and enable capture first".into()));
        }
        let mut c = crate::db::open(&paths.db, &config, true)?;
        crate::db::migrate(&mut c)?;
        c.pragma_update(None, "cache_size", -256)?;
        c.pragma_update(None, "synchronous", "FULL")?;
        Ok(Self {
            paths,
            config,
            c,
            negotiated: None,
        })
    }
    pub fn process(&mut self) -> Result<u64> {
        crate::sessionize::process_batch(&mut self.c)
    }
    fn allowed(&self) -> Result<Setup> {
        let s = Setup::load(&self.c)?;
        if s.choice != crate::setup::Choice::Enabled {
            return Err(Error::Config("Capture disabled. Enable it in Omoi.".into()));
        }
        if !s.background && !crate::setup::app_present(&self.paths.home) {
            return Err(Error::Config(
                "Open Omoi, or explicitly allow companion background capture.".into(),
            ));
        }
        Ok(s)
    }
    pub fn handle(&mut self, value: Value) -> Value {
        match self.handle_inner(value) {
            Ok(v) => v,
            Err(Error::Config(message)) => {
                json!({"ok":false,"retry":false,"drop":message=="Invalid playback event","error":message})
            }
            Err(_) => {
                json!({"ok":false,"retry":true,"error":"Catalogue unavailable or busy. Retry shortly."})
            }
        }
    }
    fn handle_inner(&mut self, value: Value) -> Result<Value> {
        let request: Request = serde_json::from_value(value)
            .map_err(|_| Error::Config("Invalid native request".into()))?;
        let s = self.allowed()?;
        match request {
            Request::Hello {
                protocol_min,
                protocol_max,
                version,
                permission,
                binding,
            } => {
                self.negotiated = None;
                if binding.as_ref().is_some_and(|b| b != &s.binding) {
                    return Err(Error::Config(
                        "Catalogue changed. Disconnect and explicitly connect to this catalogue."
                            .into(),
                    ));
                }
                let hello = Hello {
                    protocol: if protocol_min <= 1 && protocol_max >= 1 {
                        1
                    } else {
                        0
                    },
                    version,
                    permission,
                };
                let compatible = hello.compatible();
                crate::setup::observe(&mut self.c, hello)?;
                if !compatible {
                    return Err(Error::Config(
                        "Upgrade Omoi or its companion: supported protocol 1, client 0.2.x (minimum 0.2.0).".into(),
                    ));
                }
                if !permission {
                    return Err(Error::Config(
                        "Approve supported-site access in the companion.".into(),
                    ));
                }
                self.negotiated = Some((s.binding.clone(), s.capture_epoch));
                Ok(
                    json!({"ok":true,"protocol":1,"binding":s.binding,"generation":s.capture_epoch,"app_version":env!("CARGO_PKG_VERSION")}),
                )
            }
            Request::Event {
                binding,
                generation,
                event,
            } => {
                if self.negotiated.as_ref() != Some(&(binding.clone(), generation)) {
                    return Err(Error::Config("Hello required before events".into()));
                }
                let accepted = crate::sessionize::enqueue_checked(
                    &mut self.c,
                    &event.to_string(),
                    self.config.offset_seconds()?,
                    Some((&binding, generation)),
                )?;
                let _ = self.process(); // Ack is for durable enqueue; busy processing retains the inbox.
                Ok(json!({"ok":true,"accepted":accepted}))
            }
            Request::Status => {
                let pending: i64 =
                    self.c
                        .query_row("SELECT count(*) FROM raw_events", [], |r| r.get(0))?;
                Ok(
                    json!({"ok":true,"binding":s.binding,"generation":s.capture_epoch,"pending":pending,"last_event":s.last_event,"last_processed":s.last_processed}),
                )
            }
        }
    }
    pub fn drain_if_allowed(&mut self) -> Result<()> {
        self.allowed()?;
        self.process()?;
        Ok(())
    }
}
