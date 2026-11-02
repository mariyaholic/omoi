//! Bounded history-reader contracts. Paths used for source IDs must already be canonical.
pub mod discover;
pub mod snapshot;
pub mod sqlite;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    Chrome,
    Edge,
    Brave,
    Firefox,
}

impl Browser {
    pub fn name(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Edge => "edge",
            Self::Brave => "brave",
            Self::Firefox => "firefox",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceId(pub String);

impl SourceId {
    /// Encode a canonical absolute path without losing non-Unicode OS characters.
    /// Discovery canonicalizes existing profile paths before calling this method.
    pub fn from_absolute_path(browser: Browser, path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(Error::Config("source path must be absolute".into()));
        }
        #[cfg(windows)]
        let (os, bytes) = {
            use std::os::windows::ffi::OsStrExt;
            (
                "win",
                path.as_os_str()
                    .encode_wide()
                    .flat_map(u16::to_le_bytes)
                    .collect::<Vec<_>>(),
            )
        };
        #[cfg(unix)]
        let (os, bytes) = {
            use std::os::unix::ffi::OsStrExt;
            ("unix", path.as_os_str().as_bytes().to_vec())
        };
        let mut id = format!("{}:{os}:", browser.name());
        use std::fmt::Write;
        for byte in bytes {
            write!(&mut id, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(Self(id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStamp {
    pub present: bool,
    pub len: u64,
    pub mtime_ns: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub main: FileStamp,
    pub wal: FileStamp,
    pub journal: FileStamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    #[serde(deserialize_with = "cursor_version")]
    pub version: u32,
    pub last_native_us: i64,
    pub last_visit_id: i64,
    pub fingerprint: Option<Fingerprint>,
}

fn cursor_version<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    let version = u32::deserialize(deserializer)?;
    if version == 1 {
        Ok(version)
    } else {
        Err(serde::de::Error::custom("unsupported cursor version"))
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            version: 1,
            last_native_us: i64::MIN,
            last_visit_id: i64::MIN,
            fingerprint: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawVisit {
    pub source: SourceId,
    pub browser: Browser,
    pub visit_id: i64,
    pub native_us: i64,
    pub unix_us: i64,
    pub url: String,
    pub title: Option<String>,
    pub approx_duration_s: Option<i64>,
}

pub struct Batch {
    pub scanned: usize,
    pub visits: Vec<RawVisit>,
    pub next: Cursor,
    pub exhausted: bool,
}

pub trait HistoryReader {
    fn id(&self) -> &SourceId;
    fn read_since(&mut self, cursor: &Cursor, limit: usize) -> Result<Batch>;
}
