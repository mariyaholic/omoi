pub mod maintenance;
pub mod migrations;
pub mod schema;
use crate::{config::Config, Error, Result};
pub use migrations::migrate;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{path::Path, time::Duration};

pub fn open(path: &Path, config: &Config, writable: bool) -> Result<Connection> {
    let flags = if writable {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let conn = Connection::open_with_flags(path, flags)?;
    conn.busy_timeout(Duration::from_millis(config.busy_timeout_ms))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    if writable {
        let mode: String = conn
            .pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))
            .map_err(map_busy)?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(Error::Schema("WAL unavailable".into()));
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "journal_size_limit", config.journal_size_limit_bytes)?;
        conn.pragma_update(None, "wal_autocheckpoint", 256)?;
    } else {
        conn.pragma_update(None, "query_only", true)?;
    }
    conn.execute_batch("PRAGMA cache_size=-2048; PRAGMA mmap_size=0; PRAGMA temp_store=FILE;")?;
    Ok(conn)
}
pub(crate) fn map_busy(err: rusqlite::Error) -> Error {
    match &err {
        rusqlite::Error::SqliteFailure(code, _)
            if matches!(
                code.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            ) =>
        {
            Error::Busy
        }
        _ => Error::Db(err),
    }
}
#[derive(Debug, Serialize)]
pub struct Info {
    pub path: Option<String>,
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub counts: std::collections::BTreeMap<String, i64>,
    pub user_version: u32,
    pub sqlite_version: String,
    pub fts5: bool,
}
pub fn info(conn: &Connection) -> Result<Info> {
    let tx = conn.unchecked_transaction()?;
    let result = info_snapshot(&tx)?;
    tx.commit()?;
    Ok(result)
}
fn info_snapshot(conn: &Connection) -> Result<Info> {
    let mut counts = std::collections::BTreeMap::new();
    for table in [
        "items",
        "views",
        "sessions",
        "item_tags",
        "channel_tags",
        "checkpoints",
        "raw_events",
    ] {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
            [table],
            |r| r.get(0),
        )?;
        if exists {
            counts.insert(
                table.to_owned(),
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?,
            );
        }
    }
    let path = conn.path().filter(|s| !s.is_empty()).map(str::to_owned);
    let size = |p: &str| -> Result<u64> {
        match std::fs::metadata(p) {
            Ok(meta) => Ok(meta.len()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(source) => Err(Error::Io {
                context: "database size",
                source,
            }),
        }
    };
    let (db_bytes, wal_bytes) = match path.as_deref() {
        Some(p) => (size(p)?, size(&format!("{p}-wal"))?),
        None => (0, 0),
    };
    Ok(Info {
        path,
        db_bytes,
        wal_bytes,
        counts,
        user_version: conn.pragma_query_value(None, "user_version", |r| r.get(0))?,
        sqlite_version: conn.query_row("SELECT sqlite_version()", [], |r| r.get(0))?,
        fts5: conn.query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
            r.get(0)
        })?,
    })
}
