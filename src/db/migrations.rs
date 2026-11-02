use super::schema;
use crate::{Error, Result};
use rusqlite::{Connection, TransactionBehavior};

pub fn migrate(conn: &mut Connection) -> Result<()> {
    apply_initial(conn, schema::INITIAL)
}

/// Transactional migration runner; accepting SQL also enables rollback fault tests.
pub fn apply_initial(conn: &mut Connection, sql: &str) -> Result<()> {
    let fts: bool = conn.query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
        r.get(0)
    })?;
    if !fts {
        return Err(Error::Schema("FTS5 unavailable".into()));
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(super::map_busy)?;
    let version: u32 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version > schema::VERSION {
        return Err(Error::SchemaTooNew {
            found: version,
            supported: schema::VERSION,
        });
    }
    if version == 0 {
        let objects: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if objects != 0 {
            return Err(Error::Schema("nonempty unversioned database".into()));
        }
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", schema::VERSION)?;
    }
    tx.commit()?;
    Ok(())
}
