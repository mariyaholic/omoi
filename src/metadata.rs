//! Local edits have explicit per-field provenance and survive later ingestion.
use crate::{Error, Result};
use rusqlite::{params, Connection, Transaction};
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Edit {
    id: i64,
    title: String,
    channel: String,
    kind: String,
    duration_s: Option<i64>,
    tags: Vec<String>,
}
pub fn edit(c: &mut Connection, value: &Value) -> Result<()> {
    let e: Edit = serde_json::from_value(value.clone())
        .map_err(|_| Error::Config("Invalid item fields".into()))?;
    if e.id <= 0
        || e.title.len() > 4096
        || e.channel.len() > 1024
        || e.kind.len() > 64
        || e.duration_s.is_some_and(|n| !(0..=31_536_000).contains(&n))
        || e.tags.len() > 100
        || e.tags
            .iter()
            .any(|t| t.trim().is_empty() || t.len() > 128 || t.chars().any(char::is_control))
    {
        return Err(Error::Config("Invalid metadata or tags".into()));
    }
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if tx.execute(
        "UPDATE items SET title=?1,channel=?2,kind=?3,duration_s=?4,metadata_rank=100 WHERE id=?5",
        params![
            e.title.trim(),
            e.channel.trim(),
            e.kind.trim(),
            e.duration_s,
            e.id
        ],
    )? != 1
    {
        return Err(Error::Config("Item no longer exists".into()));
    }
    for field in ["title", "channel", "kind", "duration_s"] {
        tx.execute("INSERT INTO metadata_fields VALUES(?1,?2,'manual') ON CONFLICT(item_id,field) DO UPDATE SET source='manual'",params![e.id,field])?;
    }
    tx.execute("DELETE FROM item_tags WHERE item_id=?", [e.id])?;
    for tag in e.tags {
        tx.execute(
            "INSERT OR IGNORE INTO item_tags VALUES(?1,?2,'manual')",
            params![e.id, tag.trim().to_lowercase()],
        )?;
    }
    tx.commit()?;
    Ok(())
}
pub fn observed(
    tx: &Transaction<'_>,
    id: i64,
    title: Option<&str>,
    channel: Option<&str>,
    duration: Option<i64>,
) -> Result<()> {
    for (field, value) in [("title", title), ("channel", channel)] {
        if let Some(v) = value.filter(|s| !s.trim().is_empty()) {
            let sql=format!("UPDATE items SET {field}=?1 WHERE id=?2 AND ({field} IS NULL OR {field}='') AND NOT EXISTS(SELECT 1 FROM metadata_fields WHERE item_id=?2 AND field=?3)");
            if tx.execute(&sql, params![v.trim(), id, field])? > 0 {
                tx.execute(
                    "INSERT INTO metadata_fields VALUES(?1,?2,'userscript')",
                    params![id, field],
                )?;
            }
        }
    }
    if let Some(n) = duration {
        if tx.execute("UPDATE items SET duration_s=?1 WHERE id=?2 AND duration_s IS NULL AND NOT EXISTS(SELECT 1 FROM metadata_fields WHERE item_id=?2 AND field='duration_s')",params![n,id])?>0 {tx.execute("INSERT INTO metadata_fields VALUES(?1,'duration_s','userscript')",[id])?;}
    }
    Ok(())
}
