//! Durable telemetry processing. Browser visits never imply playback.
use crate::{normalize::ItemKey, Error, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Event {
    event_id: String,
    producer_id: String,
    seq: i64,
    url: String,
    at_ms: i64,
    state: String,
    position_ms: i64,
    cumulative_played_ms: i64,
    title: Option<String>,
    channel: Option<String>,
    duration_s: Option<i64>,
}
#[derive(Deserialize, Serialize)]
struct Stored {
    event: Event,
    key: ItemKey,
    #[serde(default)]
    live_offset: Option<i32>,
    #[serde(default)]
    deletion_key: Option<ItemKey>,
}

fn live_channel(key: &ItemKey) -> Option<&str> {
    (key.platform == "twitch" && key.external_id.starts_with("live:"))
        .then(|| key.external_id.split(':').nth(1))
        .flatten()
}

fn continuing_live_key(
    tx: &Transaction<'_>,
    event: &Event,
    key: &ItemKey,
) -> Result<Option<ItemKey>> {
    let Some(channel) = live_channel(key) else {
        return Ok(None);
    };
    let accepted: Option<String> = tx.query_row(
        "SELECT i.external_id FROM playback_state p JOIN items i ON i.id=p.item_id WHERE p.producer=?1 AND p.stopped=0 AND p.at_ms<=?2 AND p.at_ms>=?3 AND i.platform='twitch'",
        params![event.producer_id,event.at_ms,event.at_ms.saturating_sub(300000)],
        |r|r.get(0),
    ).optional()?;
    Ok(accepted
        .filter(|id| id.starts_with(&format!("live:{channel}:")))
        .map(|external_id| ItemKey {
            platform: "twitch".into(),
            external_id,
        }))
}
pub fn available() -> bool {
    true
}
pub fn enqueue(c: &mut Connection, body: &str, offset: i32) -> Result<bool> {
    enqueue_checked(c, body, offset, None)
}
pub fn enqueue_checked(
    c: &mut Connection,
    body: &str,
    offset: i32,
    binding: Option<(&str, u64)>,
) -> Result<bool> {
    let invalid = || Error::Config("Invalid playback event".into());
    if body.len() > 16384 {
        return Err(invalid());
    }
    let mut e: Event = serde_json::from_str(body).map_err(|_| invalid())?;
    let ident = |s: &str| {
        !s.is_empty()
            && s.len() <= 128
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    };
    if !ident(&e.event_id)
        || !ident(&e.producer_id)
        || e.seq < 0
        || e.seq > 9_007_199_254_740_991
        || !(0..=4_102_444_800_000i64).contains(&e.at_ms)
        || !(0..=31_536_000_000i64).contains(&e.position_ms)
        || !(0..=31_536_000_000i64).contains(&e.cumulative_played_ms)
        || !["playing", "paused", "stopped"].contains(&e.state.as_str())
        || e.title.as_ref().is_some_and(|s| s.len() > 4096)
        || e.channel.as_ref().is_some_and(|s| s.len() > 1024)
        || e.duration_s.is_some_and(|s| !(0..=31_536_000).contains(&s))
    {
        return Err(invalid());
    }
    let key = crate::normalize::normalize(
        &e.url,
        e.at_ms / 1000,
        chrono::FixedOffset::east_opt(offset).ok_or_else(invalid)?,
    )
    .ok_or_else(invalid)?
    .key;
    if key.platform == "local" {
        return Err(invalid());
    }
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut setup = crate::setup::Setup::load(&tx)?;
    if matches!(
        setup.choice,
        crate::setup::Choice::Stopped | crate::setup::Choice::Skipped
    ) {
        return Err(Error::Config("Capture stopped in Omoi".into()));
    }
    if let Some((binding, generation)) = binding {
        if setup.choice != crate::setup::Choice::Enabled
            || setup.binding != binding
            || setup.capture_epoch != generation
        {
            return Err(Error::Config(
                "Capture consent or catalogue binding changed; reconnect".into(),
            ));
        }
    }
    if setup.choice == crate::setup::Choice::Enabled {
        if setup.accept_after_ms.is_some_and(|at| e.at_ms < at) {
            return Err(Error::Config(
                "Playback predates current capture consent".into(),
            ));
        }
        // Legacy clients cannot receive a Stop command while the owned port is
        // closed. Reject their old queue above and give continuing counters a
        // server-side epoch so stopped time can never become later credit.
        if binding.is_none() {
            e.producer_id = format!("epoch{}:{}", setup.capture_epoch, e.producer_id);
        }
    }
    if tx.execute(
        "INSERT OR IGNORE INTO playback_receipts VALUES(?1,?2,?3)",
        params![e.event_id, e.producer_id, e.seq],
    )? == 0
    {
        return Ok(false);
    }
    let at = e.at_ms;
    // Retain the accepted identity as a deletion alias even if item deletion
    // later cascades the playback frontier before this raw event is processed.
    let deletion_key = continuing_live_key(&tx, &e, &key)?;
    let body = serde_json::to_string(&Stored {
        event: e,
        key,
        live_offset: Some(offset),
        deletion_key,
    })
    .map_err(|_| invalid())?;
    if body.len() > 16384 {
        return Err(invalid());
    }
    tx.execute(
        "INSERT INTO raw_events(at,source,body) VALUES(?1,'userscript',?2)",
        params![at, body],
    )?;
    let (mut count, mut bytes): (i64, i64) = tx.query_row(
        "SELECT count(*),coalesce(sum(length(CAST(body AS BLOB))),0) FROM raw_events",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    while count > 100000 || bytes > 20000000 {
        let (id, size, body): (i64, i64, String) = tx.query_row(
            "SELECT id,length(CAST(body AS BLOB)),body FROM raw_events ORDER BY id LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        if let Ok(stored) = serde_json::from_str::<Stored>(&body) {
            tx.execute(
                "DELETE FROM playback_receipts WHERE event_id=?1 AND producer=?2 AND seq=?3",
                params![
                    stored.event.event_id,
                    stored.event.producer_id,
                    stored.event.seq
                ],
            )?;
        }
        tx.execute("DELETE FROM raw_events WHERE id=?", [id])?;
        tx.execute(
            "UPDATE playback_stats SET value=value+1 WHERE key='dropped'",
            [],
        )?;
        count -= 1;
        bytes -= size;
    }
    if !setup.binding.is_empty() {
        setup.last_event = Some(chrono::Utc::now().timestamp());
        setup.save(&tx)?;
    }
    tx.commit()?;
    Ok(true)
}
pub fn run(tx: &Transaction<'_>) -> Result<u64> {
    run_limited(tx, usize::MAX)
}

/// Explicit owner-selected timezone migration for old raw Twitch bodies. No
/// current timezone is guessed; original deletion identity and receipts remain.
pub fn migrate_legacy_live(c: &mut Connection, offset: i32) -> Result<u64> {
    let zone = chrono::FixedOffset::east_opt(offset)
        .ok_or_else(|| Error::Config("Invalid migration offset".into()))?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let mut after = 0;
    let mut count = 0;
    loop {
        let rows = tx
            .prepare("SELECT id,body FROM raw_events WHERE id>?1 ORDER BY id LIMIT 256")?
            .query_map([after], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if rows.is_empty() {
            break;
        }
        for (id, body) in rows {
            after = id;
            let mut stored: Stored = serde_json::from_str(&body).map_err(|_| {
                Error::Config("Invalid stored playback event; migration rolled back".into())
            })?;
            if stored.live_offset.is_some() || live_channel(&stored.key).is_none() {
                continue;
            }
            if stored.deletion_key.is_some() {
                return Err(Error::Config("Incomplete stored playback context; migration rolled back without replacing deletion identity".into()));
            }
            let key =
                crate::normalize::normalize(&stored.event.url, stored.event.at_ms / 1000, zone)
                    .ok_or_else(|| {
                        Error::Config("Invalid stored playback URL; migration rolled back".into())
                    })?
                    .key;
            if live_channel(&key) != live_channel(&stored.key) {
                return Err(Error::Config(
                    "Stored playback channel mismatch; migration rolled back".into(),
                ));
            }
            stored.deletion_key = Some(stored.key);
            stored.key = key;
            stored.live_offset = Some(offset);
            let body = serde_json::to_string(&stored)
                .map_err(|_| Error::Config("Cannot serialize migration".into()))?;
            if body.len() > 16384 {
                return Err(Error::Config(
                    "Migrated event exceeds size limit; migration rolled back".into(),
                ));
            }
            tx.execute(
                "UPDATE raw_events SET body=?1 WHERE id=?2",
                params![body, id],
            )?;
            count += 1;
        }
    }
    let bytes: i64 = tx.query_row(
        "SELECT coalesce(sum(length(CAST(body AS BLOB))),0) FROM raw_events",
        [],
        |r| r.get(0),
    )?;
    if bytes > 20000000 {
        return Err(Error::Config(
            "Migrated queue exceeds inbox size limit; migration rolled back".into(),
        ));
    }
    tx.commit()?;
    Ok(count)
}
/// Managed capture commits at most 256 raw events per transaction.
pub fn run_limited(tx: &Transaction<'_>, batches: usize) -> Result<u64> {
    let mut count = 0;
    for _ in 0..batches {
        let rows = tx
            .prepare("SELECT id,body FROM raw_events ORDER BY at,id LIMIT 256")?
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if rows.is_empty() {
            break;
        }
        for (raw, body) in rows {
            let stored: Stored = serde_json::from_str(&body)
                .map_err(|_| Error::Config("Invalid stored playback event".into()))?;
            let e = stored.event;
            let mut key = stored.key;
            let alias_deleted = match stored.deletion_key.as_ref() {
                Some(alias) => crate::dedupe::forgotten(tx, alias, e.at_ms / 1000)?,
                None => false,
            };
            if alias_deleted || crate::dedupe::forgotten(tx, &key, e.at_ms / 1000)? {
                tx.execute("DELETE FROM raw_events WHERE id=?", [raw])?;
                continue;
            }
            let previous:Option<(i64,i64,i64,i64,i64,i64,bool)>=tx.query_row("SELECT seq,item_id,session_id,at_ms,counter_ms,played_ms,stopped FROM playback_state WHERE producer=?",[&e.producer_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
            if previous
                .is_some_and(|p| e.seq <= p.0 || e.at_ms < p.3 || e.cumulative_played_ms < p.4)
            {
                tx.execute(
                    "UPDATE playback_stats SET value=value+1 WHERE key='late'",
                    [],
                )?;
                tx.execute("DELETE FROM raw_events WHERE id=?", [raw])?;
                continue;
            }
            // Only accepted processing state may choose a live identity.
            // Enqueue order and legacy playback_live_keys can include events
            // that the monotonic frontier rejects, so neither is authoritative.
            let channel = live_channel(&key).map(str::to_owned);
            if channel.is_some() {
                if let Some(accepted) = continuing_live_key(tx, &e, &key)? {
                    key = accepted;
                } else if let Some(offset) = stored.live_offset {
                    key = crate::normalize::normalize(
                        &e.url,
                        e.at_ms / 1000,
                        chrono::FixedOffset::east_opt(offset).ok_or_else(|| {
                            Error::Config("Invalid stored playback offset".into())
                        })?,
                    )
                    .ok_or_else(|| Error::Config("Invalid stored playback URL".into()))?
                    .key;
                } else {
                    return Err(Error::Config("Legacy Twitch queue has ambiguous broadcast dates; queue preserved. Back up the catalogue, then explicitly choose the historical timezone with: omoi db migrate-playback --utc-offset +08:00 (replace the example offset)".into()));
                }
            }
            if crate::dedupe::forgotten(tx, &key, e.at_ms / 1000)? {
                tx.execute("DELETE FROM raw_events WHERE id=?", [raw])?;
                continue;
            }
            if let Some(channel) = &channel {
                // Keep the legacy compatibility table current, but never use an
                // enqueue-derived row to override the accepted playback state.
                tx.execute("INSERT INTO playback_live_keys(producer,channel,external_id,last_seq,last_at_ms,stopped) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(producer,channel) DO UPDATE SET external_id=excluded.external_id,last_seq=excluded.last_seq,last_at_ms=excluded.last_at_ms,stopped=excluded.stopped",params![e.producer_id,channel,key.external_id,e.seq,e.at_ms,e.state=="stopped"])?;
            }
            crate::dedupe::ingest(
                tx,
                &crate::dedupe::Candidate {
                    key: key.clone(),
                    started_at: e.at_ms / 1000,
                    source: "userscript".into(),
                    title: None,
                    kind: None,
                    approx_duration_s: None,
                },
            )?;
            let id: i64 = tx.query_row(
                "SELECT id FROM items WHERE platform=?1 AND external_id=?2",
                params![key.platform, key.external_id],
                |r| r.get(0),
            )?;
            crate::metadata::observed(
                tx,
                id,
                e.title.as_deref(),
                e.channel.as_deref(),
                e.duration_s,
            )?;
            let adjacent = previous.filter(|p| p.1 == id && !p.6 && e.at_ms - p.3 <= 300000);
            let (session, played) = if let Some(p) = adjacent {
                let delta = (e.cumulative_played_ms - p.4).clamp(0, e.at_ms - p.3);
                if delta > 0 {
                    tx.execute("INSERT INTO playback_intervals(session_id,start_ms,end_ms,played_ms) VALUES(?1,?2,?3,?4)",params![p.2,p.3,e.at_ms,delta])?;
                }
                (p.2, p.5 + delta)
            } else {
                tx.execute("INSERT INTO sessions(item_id,start,end,played_s,max_position_s,source) VALUES(?1,?2,?3,0,?4,'userscript')",params![id,e.at_ms/1000,(e.at_ms+999)/1000,e.position_ms/1000])?;
                (tx.last_insert_rowid(), 0)
            };
            tx.execute("UPDATE sessions SET end=?1,played_s=?2,max_position_s=max(max_position_s,?3) WHERE id=?4",params![(e.at_ms+999)/1000,played/1000,e.position_ms/1000,session])?;
            tx.execute("INSERT INTO playback_state VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(producer) DO UPDATE SET seq=excluded.seq,item_id=excluded.item_id,session_id=excluded.session_id,at_ms=excluded.at_ms,counter_ms=excluded.counter_ms,played_ms=excluded.played_ms,stopped=excluded.stopped",params![e.producer_id,e.seq,id,session,e.at_ms,e.cumulative_played_ms,played,e.state=="stopped"])?;
            tx.execute("DELETE FROM raw_events WHERE id=?", [raw])?;
            count += 1;
        }
    }
    if count > 0 {
        let mut setup = crate::setup::Setup::load(tx)?;
        if !setup.binding.is_empty() {
            setup.last_processed = Some(chrono::Utc::now().timestamp());
            setup.revision = setup.revision.saturating_add(1);
            setup.save(tx)?;
        }
    }
    Ok(count)
}
pub fn process_batch(c: &mut Connection) -> Result<u64> {
    if !c.query_row("SELECT EXISTS(SELECT 1 FROM raw_events)", [], |r| {
        r.get::<_, bool>(0)
    })? {
        return Ok(0);
    }
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let count = run_limited(&tx, 1)?;
    tx.commit()?;
    Ok(count)
}
/// Merge overlapping telemetry intervals using maximum observed density, never sum tabs.
pub fn total(c: &Connection, item: i64) -> Result<i64> {
    // SQLite can sort on disk; keep only simultaneously active intervals in RAM.
    let mut q=c.prepare("SELECT p.start_ms AS at,p.id,1 AS start,1.0*p.played_ms/(p.end_ms-p.start_ms) AS density FROM playback_intervals p JOIN sessions s ON s.id=p.session_id WHERE s.item_id=?1 AND p.end_ms>p.start_ms UNION ALL SELECT p.end_ms,p.id,0,0.0 FROM playback_intervals p JOIN sessions s ON s.id=p.session_id WHERE s.item_id=?1 AND p.end_ms>p.start_ms ORDER BY at,start")?;
    let points = q.query_map([item], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, bool>(2)?,
            r.get::<_, f64>(3)?,
        ))
    })?;
    let mut active = std::collections::BTreeMap::<i64, f64>::new();
    let mut prev = 0;
    let mut sum = 0.0;
    for point in points {
        let (at, id, start, density) = point?;
        sum += (at - prev) as f64 * active.values().copied().fold(0.0, f64::max);
        if start {
            active.insert(id, density);
        } else {
            active.remove(&id);
        }
        prev = at;
    }
    // Inputs measure whole milliseconds. Round numerical integration back to
    // that resolution before truncating seconds (1000/1062 must remain 1 s).
    Ok(sum.round() as i64 / 1000)
}
