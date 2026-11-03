use crate::{normalize::ItemKey, Result};
use rusqlite::{OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct Candidate {
    pub key: ItemKey,
    pub started_at: i64,
    pub source: String,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub approx_duration_s: Option<i64>,
}
#[derive(Default)]
pub struct IngestOutcome {
    pub items_inserted: u64,
    pub views_inserted: u64,
    pub views_merged: u64,
    pub metadata_updated: u64,
}
fn rank(source: &str) -> i64 {
    match source {
        "chrome" | "edge" | "brave" => 20,
        "firefox" => 10,
        _ => 0,
    }
}
pub fn ingest(tx: &Transaction<'_>, r: &Candidate) -> Result<IngestOutcome> {
    let mut out = IngestOutcome { items_inserted: tx.execute("INSERT INTO items(platform,external_id) VALUES(?1,?2) ON CONFLICT(platform,external_id) DO NOTHING",rusqlite::params![r.key.platform,r.key.external_id])? as u64, ..Default::default() };
    let (id, title, oldrank, kind): (i64, Option<String>, i64, Option<String>) = tx.query_row(
        "SELECT id,title,metadata_rank,kind FROM items WHERE platform=?1 AND external_id=?2",
        rusqlite::params![r.key.platform, r.key.external_id],
        |v| Ok((v.get(0)?, v.get(1)?, v.get(2)?, v.get(3)?)),
    )?;
    let newtitle = r.title.as_deref().map(str::trim).filter(|t| {
        !t.is_empty()
            && ![
                "youtube",
                "netflix",
                "twitch",
                "prime video",
                "disney+",
                "crunchyroll",
                "bilibili",
            ]
            .contains(&t.to_ascii_lowercase().as_str())
    });
    if let Some(t) = newtitle.filter(|_| title.is_none() || rank(&r.source) > oldrank) {
        out.metadata_updated += tx.execute(
            "UPDATE items SET title=?1,metadata_rank=?2 WHERE id=?3",
            rusqlite::params![t, rank(&r.source), id],
        )? as u64;
    }
    if kind.is_none()
        || (kind.as_deref() == Some("video") && matches!(r.kind.as_deref(), Some("short" | "live")))
    {
        tx.execute(
            "UPDATE items SET kind=?1 WHERE id=?2 AND NOT EXISTS(SELECT 1 FROM metadata_fields WHERE item_id=?2 AND field='kind' AND source='manual')",
            rusqlite::params![r.kind, id],
        )?;
    }
    let existing:Option<(i64,String,Option<i64>)>=tx.query_row("SELECT id,source,approx_duration_s FROM views WHERE item_id=?1 AND started_at BETWEEN ?2 AND ?3 ORDER BY abs(started_at-?4),started_at,id LIMIT 1",rusqlite::params![id,r.started_at.saturating_sub(1800),r.started_at.saturating_add(1800),r.started_at],|v|Ok((v.get(0)?,v.get(1)?,v.get(2)?))).optional()?;
    if let Some((view, source, duration)) = existing {
        let winner = if rank(&r.source) > rank(&source)
            || (rank(&r.source) == rank(&source) && r.source < source)
        {
            &r.source
        } else {
            &source
        };
        tx.execute(
            "UPDATE views SET source=?1,approx_duration_s=?2 WHERE id=?3",
            rusqlite::params![winner, duration.max(r.approx_duration_s), view],
        )?;
        out.views_merged = 1;
    } else {
        tx.execute(
            "INSERT INTO views(item_id,started_at,source,approx_duration_s) VALUES(?1,?2,?3,?4)",
            rusqlite::params![id, r.started_at, r.source, r.approx_duration_s],
        )?;
        out.views_inserted = 1;
    }
    Ok(out)
}
