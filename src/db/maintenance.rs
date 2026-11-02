use crate::Result;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
pub fn optimize_due(conn: &mut Connection, now: i64) -> Result<bool> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(super::map_busy)?;
    let last: Option<String> = tx
        .query_row(
            "SELECT cursor FROM checkpoints WHERE source='maintenance:optimize'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if last
        .and_then(|s| s.parse::<i64>().ok())
        .is_some_and(|t| now.saturating_sub(t) < 7 * 86400)
    {
        return Ok(false);
    }
    tx.execute_batch("PRAGMA optimize")?;
    tx.execute("INSERT INTO checkpoints VALUES('maintenance:optimize',?1) ON CONFLICT(source) DO UPDATE SET cursor=excluded.cursor",[now.to_string()])?;
    tx.commit()?;
    Ok(true)
}
