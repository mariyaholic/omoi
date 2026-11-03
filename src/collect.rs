use crate::{
    config::Config,
    dedupe::{self, Candidate},
    paths::{PathEnvironment, Paths},
    readers::{
        discover::discover,
        snapshot::{self, io, TempDir},
        sqlite::SqliteReader,
        Cursor, HistoryReader,
    },
    Error, Result,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    time::Instant,
};
#[derive(Default, Serialize)]
pub struct CollectReport {
    pub dry_run: bool,
    pub profiles_found: usize,
    pub profiles_excluded: usize,
    pub profiles_unchanged: usize,
    pub profiles_ok: usize,
    pub profiles_failed: usize,
    pub rows_read: u64,
    pub rows_rejected: u64,
    pub items_inserted: u64,
    pub views_inserted: u64,
    pub views_merged: u64,
    pub metadata_updated: u64,
    pub copy_ms: u128,
    pub read_normalize_ms: u128,
    pub write_ms: u128,
    pub total_ms: u128,
    pub warnings: Vec<String>,
}
struct Prepared {
    source: String,
    prior: Option<String>,
    cursor: Cursor,
    spool: PathBuf,
}
fn prior(conn: &Connection, source: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT cursor FROM checkpoints WHERE source=?",
            [source],
            |r| r.get(0),
        )
        .optional()?)
}
pub fn collect(
    config: &Config,
    paths: &Paths,
    env: &PathEnvironment,
    dry_run: bool,
) -> Result<CollectReport> {
    collect_selected(config, paths, env, dry_run, None)
}

/// UI collection may narrow discovery to explicit source identities. It never broadens config scope.
pub fn collect_selected(
    config: &Config,
    paths: &Paths,
    env: &PathEnvironment,
    dry_run: bool,
    selected: Option<&[String]>,
) -> Result<CollectReport> {
    match collect_once(config, paths, env, dry_run, selected) {
        Err(Error::Busy) => collect_once(config, paths, env, dry_run, selected),
        result => result,
    }
}
fn collect_once(
    config: &Config,
    paths: &Paths,
    env: &PathEnvironment,
    dry_run: bool,
    selected: Option<&[String]>,
) -> Result<CollectReport> {
    let start = Instant::now();
    let temp = std::env::temp_dir();
    let run = TempDir::create(if dry_run { &temp } else { &paths.tmp })?;
    let dbpath = if dry_run {
        run.0.join("dry.sqlite")
    } else {
        paths.db.clone()
    };
    if dry_run && paths.db.exists() {
        let source = crate::db::open(&paths.db, config, false)?;
        let mut dest = Connection::open(&dbpath)?;
        let backup = rusqlite::backup::Backup::new(&source, &mut dest)?;
        backup.run_to_completion(128, std::time::Duration::from_millis(10), None)?;
    }
    let mut conn = crate::db::open(&dbpath, config, true)?;
    crate::db::migrate(&mut conn)?;
    let mut discovery = discover(config, env)?;
    if let Some(selected) = selected {
        if selected.is_empty()
            || selected
                .iter()
                .any(|id| !discovery.profiles.iter().any(|p| &p.source.0 == id))
        {
            return Err(Error::Config(
                "selected profiles are unavailable; refresh Sources".into(),
            ));
        }
        discovery
            .profiles
            .retain(|p| selected.contains(&p.source.0));
    }
    let mut report = CollectReport {
        dry_run,
        profiles_found: discovery.profiles.len() + discovery.failures.len(),
        profiles_excluded: discovery.excluded,
        profiles_failed: discovery.failures.len(),
        warnings: discovery.warnings,
        ..Default::default()
    };
    report.warnings.extend(discovery.failures);
    let mut prepared = vec![];
    for (index, profile) in discovery.profiles.iter().enumerate() {
        let old = prior(&conn, &profile.source.0)?;
        let mut cursor = match &old {
            Some(s) => {
                serde_json::from_str(s).map_err(|_| Error::Schema("invalid checkpoint".into()))?
            }
            None => Cursor::default(),
        };
        let copy_start = Instant::now();
        let fp = match snapshot::fingerprint(&profile.path) {
            Ok(fp) => fp,
            Err(_) => {
                report.profiles_failed += 1;
                report.warnings.push("profile inaccessible".into());
                continue;
            }
        };
        if cursor.fingerprint.as_ref() == Some(&fp) {
            report.profiles_unchanged += 1;
            continue;
        }
        let snap = match snapshot::copy(&profile.path, &run.0, config.snapshot_retries) {
            Ok(s) => s,
            Err(
                e @ Error::Io {
                    context: "temporary root" | "temporary directory",
                    ..
                },
            ) => return Err(e),
            Err(Error::Io { context, source })
                if matches!(source.raw_os_error(), Some(28 | 112)) =>
            {
                return Err(Error::Io { context, source })
            }
            Err(_) => {
                report.profiles_failed += 1;
                report.warnings.push("snapshot failed".into());
                continue;
            }
        };
        cursor.fingerprint = Some(snap.fingerprint.clone());
        report.copy_ms += copy_start.elapsed().as_millis();
        let mut reader = match SqliteReader::open(profile.source.clone(), profile.browser, snap) {
            Ok(r) => r,
            Err(_) => {
                report.profiles_failed += 1;
                report
                    .warnings
                    .push("browser schema or integrity failure".into());
                continue;
            }
        };
        if reader.reset_if_replaced(&mut cursor).is_err() {
            report.profiles_failed += 1;
            continue;
        }
        let null_rows = reader.null_timestamp_rows()?;
        report.rows_read += null_rows;
        report.rows_rejected += null_rows;
        let spool = run.0.join(format!("{index}.jsonl"));
        let file = std::fs::File::create(&spool).map_err(|e| io("spool create", e))?;
        let mut writer = BufWriter::new(file);
        let read_start = Instant::now();
        let mut failed = false;
        loop {
            let batch = match reader.read_since(&cursor, config.reader_batch_rows) {
                Ok(b) => b,
                Err(_) => {
                    failed = true;
                    break;
                }
            };
            report.rows_read += batch.scanned as u64;
            report.rows_rejected += (batch.scanned - batch.visits.len()) as u64;
            for visit in batch.visits {
                let seconds = visit.unix_us.div_euclid(1000000);
                let Some(n) = crate::normalize::normalize(
                    &visit.url,
                    seconds,
                    chrono::FixedOffset::east_opt(config.offset_seconds()?)
                        .ok_or_else(|| Error::Config("invalid UTC offset".into()))?,
                ) else {
                    report.rows_rejected += 1;
                    continue;
                };
                let candidate = Candidate {
                    key: n.key,
                    started_at: seconds,
                    source: profile.browser.name().into(),
                    title: visit.title,
                    kind: n.kind.map(str::to_owned),
                    approx_duration_s: visit.approx_duration_s,
                };
                serde_json::to_writer(&mut writer, &candidate)
                    .map_err(|e| Error::Export(e.to_string()))?;
                writer.write_all(b"\n").map_err(|e| io("spool write", e))?;
            }
            cursor = batch.next;
            if batch.exhausted {
                break;
            }
        }
        writer.flush().map_err(|e| io("spool flush", e))?;
        report.read_normalize_ms += read_start.elapsed().as_millis();
        if failed {
            report.profiles_failed += 1;
            report
                .warnings
                .push("profile read failed; no rows committed".into());
            continue;
        }
        report.profiles_ok += 1;
        prepared.push(Prepared {
            source: profile.source.0.clone(),
            prior: old,
            cursor,
            spool,
        });
    }
    let write_start = Instant::now();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(crate::db::map_busy)?;
    for p in &prepared {
        if prior(&tx, &p.source)? != p.prior {
            return Err(Error::Busy);
        }
    }
    for p in prepared {
        let reader =
            BufReader::new(std::fs::File::open(&p.spool).map_err(|e| io("spool read", e))?);
        for line in reader.lines() {
            let line = line.map_err(|e| io("spool read", e))?;
            let row: Candidate =
                serde_json::from_str(&line).map_err(|e| Error::Export(e.to_string()))?;
            let out = dedupe::ingest(&tx, &row)?;
            report.items_inserted += out.items_inserted;
            report.views_inserted += out.views_inserted;
            report.views_merged += out.views_merged;
            report.metadata_updated += out.metadata_updated;
        }
        tx.execute("INSERT INTO checkpoints(source,cursor) VALUES(?1,?2) ON CONFLICT(source) DO UPDATE SET cursor=excluded.cursor",rusqlite::params![p.source,serde_json::to_string(&p.cursor).map_err(|e|Error::Export(e.to_string()))?])?;
    }
    crate::sessionize::run(&tx)?;
    tx.commit()?;
    if crate::db::maintenance::optimize_due(&mut conn, chrono::Utc::now().timestamp()).is_err() {
        report.warnings.push("weekly optimization deferred".into());
    }
    report.write_ms = write_start.elapsed().as_millis();
    if conn
        .execute_batch("PRAGMA wal_checkpoint(PASSIVE)")
        .is_err()
    {
        report.warnings.push("checkpoint deferred".into());
    }
    if report.profiles_found == 0 {
        report.warnings.push("no profiles found".into());
    }
    report.total_ms = start.elapsed().as_millis();
    Ok(report)
}
