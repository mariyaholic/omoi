//! Explainable local shelves. No network and no inferred playback from browser visits.
use crate::{Error, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct Suggestion {
    pub id: i64,
    pub platform: String,
    pub title: Option<String>,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct Shelves {
    pub continue_watching: Vec<Suggestion>,
    pub rediscover: Vec<Suggestion>,
    pub similar: Vec<Suggestion>,
    pub timebox: Vec<Suggestion>,
    pub unavailable: Vec<String>,
}
fn query(
    conn: &Connection,
    sql: &str,
    values: &[&dyn rusqlite::ToSql],
    reason: &str,
) -> Result<Vec<Suggestion>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(values, |r| {
        Ok(Suggestion {
            id: r.get(0)?,
            platform: r.get(1)?,
            title: r.get(2)?,
            reason: reason.into(),
        })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}
pub fn shelves(conn: &Connection, now: i64, seed: Option<i64>, seconds: i64) -> Result<Shelves> {
    if !(1..=86400).contains(&seconds) {
        return Err(Error::Config("time budget must be 1..86400 seconds".into()));
    }
    let tx = conn.unchecked_transaction()?;
    let continue_watching=query(&tx,"SELECT i.id,i.platform,i.title FROM items i JOIN sessions s ON s.item_id=i.id WHERE i.duration_s>0 AND s.id=(SELECT s2.id FROM sessions s2 WHERE s2.item_id=i.id ORDER BY s2.end DESC,s2.id DESC LIMIT 1) AND s.max_position_s>=i.duration_s*0.1 AND s.max_position_s<i.duration_s*0.9 ORDER BY s.end DESC,i.id LIMIT 20",&[],"latest measured session is between 10% and 90%; browser visits alone do not qualify")?;
    let cutoff = now.saturating_sub(30 * 86400);
    let rediscover=query(&tx,"SELECT i.id,i.platform,i.title FROM items i JOIN views v ON v.item_id=i.id GROUP BY i.id HAVING max(v.started_at)<?1 ORDER BY count(*) DESC,max(v.started_at) DESC,i.id LIMIT 20",&[&cutoff],"previously opened, with no recorded visit in the last 30 days")?;
    let similar = if let Some(seed) = seed {
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE id=?1)",
            params![seed],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(Error::Config("recommendation seed does not exist".into()));
        }
        query(&tx,"SELECT i.id,i.platform,i.title FROM items i JOIN item_tags t ON t.item_id=i.id JOIN item_tags seed ON seed.tag=t.tag AND seed.item_id=?1 WHERE i.id!=?1 GROUP BY i.id ORDER BY count(DISTINCT t.tag) DESC,i.id LIMIT 20",&[&seed],"shares catalogue tags with the selected item")?
    } else {
        vec![]
    };
    let timebox=query(&tx,"SELECT id,platform,title FROM items WHERE duration_s>0 AND duration_s<=?1 ORDER BY duration_s DESC,id LIMIT 20",&[&seconds],"known duration fits the selected time budget")?;
    tx.commit()?;
    Ok(Shelves{continue_watching,rediscover,similar,timebox,unavailable:vec!["New from channels requires RSS/enrichment (not connected)".into(),"TMDB recommendations require a configured metadata adapter (not connected)".into(),"Continue watching needs measured sessions; similar needs tags; timebox needs known durations. Empty shelves are not filled with guesses.".into()]})
}
