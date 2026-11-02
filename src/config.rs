use crate::readers::Browser;
use crate::{Error, Result};
use serde::Deserialize;
use std::{collections::BTreeSet, io::Read, path::Path};

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub browsers: Vec<Browser>,
    pub profile_paths: Vec<String>,
    pub profile_excludes: Vec<String>,
    pub local_utc_offset: String,
    pub busy_timeout_ms: u64,
    pub journal_size_limit_bytes: u64,
    pub reader_batch_rows: usize,
    pub snapshot_retries: usize,
    pub log_file_bytes: u64,
    pub log_files: usize,
    pub thumb_cache_bytes: u64,
    pub listener_port: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            browsers: vec![
                Browser::Chrome,
                Browser::Edge,
                Browser::Brave,
                Browser::Firefox,
            ],
            profile_paths: vec![],
            profile_excludes: vec![],
            local_utc_offset: "+08:00".into(),
            busy_timeout_ms: 5000,
            journal_size_limit_bytes: 1048576,
            reader_batch_rows: 512,
            snapshot_retries: 3,
            log_file_bytes: 1048576,
            log_files: 3,
            thumb_cache_bytes: 100000000,
            listener_port: 43821,
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(Error::Io {
                    context: "read config",
                    source,
                })
            }
        };
        let mut text = String::new();
        file.take(65537)
            .read_to_string(&mut text)
            .map_err(|source| Error::Io {
                context: "read config",
                source,
            })?;
        Self::parse(&text)
    }
    pub fn parse(text: &str) -> Result<Self> {
        if text.len() > 65536 {
            return Err(Error::Config("config exceeds 64 KiB".into()));
        }
        let mut map = serde_json::Map::new();
        let mut keys = BTreeSet::new();
        for (index, line) in text.lines().enumerate() {
            let fail = || Error::Config(format!("invalid config at line {}", index + 1));
            let line = strip_comment(line).trim();
            if line.is_empty() {
                continue;
            }
            let (key, raw) = line.split_once('=').ok_or_else(fail)?;
            let key = key.trim();
            let raw = raw.trim();
            if key.is_empty()
                || !key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                || !keys.insert(key.to_owned())
            {
                return Err(fail());
            }
            let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| fail())?;
            match &value {
                serde_json::Value::String(_) | serde_json::Value::Bool(_) => (),
                serde_json::Value::Array(a) if a.iter().all(|v| v.is_string()) => (),
                serde_json::Value::Number(n)
                    if n.as_u64().is_some() && raw.bytes().all(|b| b.is_ascii_digit()) => {}
                _ => return Err(fail()),
            }
            // Validate each field independently to retain its source line on errors.
            let mut one = serde_json::Map::new();
            one.insert(key.into(), value.clone());
            serde_json::from_value::<Self>(one.into())
                .map_err(|_| fail())?
                .validate()
                .map_err(|_| fail())?;
            map.insert(key.into(), value);
        }
        let config: Self =
            serde_json::from_value(map.into()).map_err(|e| Error::Config(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }
    fn validate(&self) -> Result<()> {
        let valid = (1..=60000).contains(&self.busy_timeout_ms)
            && self.journal_size_limit_bytes <= 16777216
            && (1..=2048).contains(&self.reader_batch_rows)
            && (1..=5).contains(&self.snapshot_retries)
            && (65536..=4194304).contains(&self.log_file_bytes)
            && (1..=5).contains(&self.log_files)
            && self.thumb_cache_bytes > 0
            && self.listener_port >= 1024;
        if !valid || parse_offset(&self.local_utc_offset).is_none() {
            return Err(Error::Config("setting out of range".into()));
        }
        for (entries, paths) in [(&self.profile_paths, true), (&self.profile_excludes, false)] {
            for entry in entries {
                let (browser, value) = entry
                    .split_once(':')
                    .ok_or_else(|| Error::Config("invalid profile selector".into()))?;
                if !["chrome", "edge", "brave", "firefox"].contains(&browser)
                    || value.is_empty()
                    || (paths && !Path::new(value).is_absolute())
                {
                    return Err(Error::Config("invalid profile selector".into()));
                }
            }
        }
        Ok(())
    }
    pub fn offset_seconds(&self) -> i32 {
        parse_offset(&self.local_utc_offset).expect("validated config")
    }
}
fn parse_offset(text: &str) -> Option<i32> {
    let b = text.as_bytes();
    if b.len() != 6
        || !matches!(b[0], b'+' | b'-')
        || b[3] != b':'
        || ![b[1], b[2], b[4], b[5]].iter().all(u8::is_ascii_digit)
    {
        return None;
    }
    let hours = ((b[1] - b'0') * 10 + b[2] - b'0') as i32;
    let minutes = ((b[4] - b'0') * 10 + b[5] - b'0') as i32;
    if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
        return None;
    }
    Some((hours * 3600 + minutes * 60) * if b[0] == b'-' { -1 } else { 1 })
}
fn strip_comment(line: &str) -> &str {
    let (mut quoted, mut escaped) = (false, false);
    for (i, c) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted;
        }
        if c == '#' && !quoted {
            return &line[..i];
        }
    }
    line
}
