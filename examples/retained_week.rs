//! Explicit owner-run local acceptance. Only aggregate counts leave temporary storage.
use clap::Parser;
use omoi::{
    config::Config,
    paths::{PathEnvironment, Paths},
    readers::{self, Browser, Cursor, HistoryReader},
    Result,
};
use rusqlite::{params, Connection};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
#[derive(Parser)]
struct Args {
    /// Required consent to read local browser history snapshots for the last seven days.
    #[arg(long)]
    consent_local_history: bool,
    #[arg(long)]
    out: PathBuf,
}
fn run(a: Args) -> Result<()> {
    if !a.consent_local_history {
        return Err(omoi::Error::Config(
            "Pass --consent-local-history for this owner-authorized acceptance test".into(),
        ));
    }
    let env = PathEnvironment::capture();
    let original_paths = Paths::resolve(&env)?;
    let config = Config::load(&original_paths.config)?;
    let discovery = readers::discover::discover(&config, &env)?;
    let temp =
        readers::snapshot::TempDir::create(&std::env::temp_dir().join("omoi-retained-week"))?;
    let cutoff = chrono::Utc::now().timestamp() - 7 * 86400;
    let mut expected = BTreeSet::new();
    let mut reports = vec![];
    let mut profile_paths = vec![];
    for (index, p) in discovery.profiles.iter().enumerate() {
        let result = (|| -> Result<serde_json::Value> {
            let snap = readers::snapshot::copy(&p.path, &temp.0.join("snapshots"), 3)?;
            let mut reader =
                readers::sqlite::SqliteReader::open(p.source.clone(), p.browser, snap)?;
            let target = temp.0.join(format!("profile-{index}.sqlite"));
            let mut staged = Connection::open(&target)?;
            let firefox = p.browser == Browser::Firefox;
            staged.execute_batch(if firefox {"CREATE TABLE moz_places(id INTEGER PRIMARY KEY,url TEXT,title TEXT);CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY,place_id INTEGER,visit_date INTEGER);"} else {"CREATE TABLE urls(id INTEGER PRIMARY KEY,url TEXT,title TEXT);CREATE TABLE visits(id INTEGER PRIMARY KEY,url INTEGER,visit_time INTEGER,visit_duration INTEGER);"})?;
            let mut cursor = Cursor {
                last_native_us: cutoff * 1_000_000 + if firefox { 0 } else { 11644473600000000 },
                ..Cursor::default()
            };
            let tx = staged.transaction()?;
            let mut scanned = 0;
            let mut invalid = 0;
            let mut accepted = 0;
            let mut irrelevant = 0;
            let mut retained = 0;
            let mut platforms = BTreeMap::<String, usize>::new();
            loop {
                let batch = reader.read_since(&cursor, 512)?;
                scanned += batch.scanned;
                invalid += batch.scanned - batch.visits.len();
                for v in batch.visits {
                    if v.unix_us / 1_000_000 > chrono::Utc::now().timestamp() {
                        invalid += 1;
                        continue;
                    }
                    retained += 1;
                    if let Some(n) = omoi::normalize::normalize(
                        &v.url,
                        v.unix_us / 1_000_000,
                        chrono::FixedOffset::east_opt(config.offset_seconds()?).unwrap(),
                    ) {
                        accepted += 1;
                        *platforms.entry(n.key.platform.clone()).or_default() += 1;
                        expected.insert((n.key.platform, n.key.external_id));
                    } else {
                        irrelevant += 1;
                    }
                    if firefox {
                        tx.execute(
                            "INSERT INTO moz_places VALUES(?1,?2,?3)",
                            params![v.visit_id, v.url, v.title],
                        )?;
                        tx.execute(
                            "INSERT INTO moz_historyvisits VALUES(?1,?1,?2)",
                            params![v.visit_id, v.native_us],
                        )?;
                    } else {
                        tx.execute(
                            "INSERT INTO urls VALUES(?1,?2,?3)",
                            params![v.visit_id, v.url, v.title],
                        )?;
                        tx.execute(
                            "INSERT INTO visits VALUES(?1,?1,?2,?3)",
                            params![
                                v.visit_id,
                                v.native_us,
                                v.approx_duration_s.map(|s| s * 1_000_000)
                            ],
                        )?;
                    }
                }
                cursor = batch.next;
                if batch.exhausted {
                    break;
                }
            }
            tx.commit()?;
            drop(staged);
            profile_paths.push(format!("{}:{}", p.browser.name(), target.display()));
            Ok(
                json!({"profile":index+1,"browser":p.browser.name(),"scanned":scanned,"reader_rejected":invalid,"retained_valid_rows":retained,"normalized_video_visits":accepted,"irrelevant_rows":irrelevant,"platform_counts":platforms,"status":"pass"}),
            )
        })();
        reports.push(result.unwrap_or_else(|e|json!({"profile":index+1,"browser":p.browser.name(),"status":"failed","exit_code":e.exit_code()})));
    }
    let empty = temp.0.join("empty");
    let home = temp.0.join("catalogue");
    let mut isolated = PathEnvironment {
        windows: cfg!(windows),
        vars: BTreeMap::new(),
    };
    for key in [
        "LOCALAPPDATA",
        "APPDATA",
        "HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ] {
        isolated
            .vars
            .insert(key.into(), empty.clone().into_os_string());
    }
    isolated
        .vars
        .insert("OMOI_HOME".into(), home.into_os_string());
    let paths = Paths::resolve(&isolated)?;
    let staged_config = Config {
        profile_paths,
        local_utc_offset: config.local_utc_offset,
        ..Config::default()
    };
    let first = omoi::collect::collect(&staged_config, &paths, &isolated, false)?;
    let again = omoi::collect::collect(&staged_config, &paths, &isolated, false)?;
    let mut c = omoi::db::open(&paths.db, &staged_config, true)?;
    let actual: i64 = c.query_row("SELECT count(*) FROM items", [], |r| r.get(0))?;
    let tx = c.transaction()?;
    let integrity: String = tx.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    tx.commit()?;
    let passed = discovery.failures.is_empty()
        && reports.iter().all(|r| r["status"] == "pass")
        && first.profiles_failed == 0
        && again.views_inserted == 0
        && actual == expected.len() as i64
        && integrity == "ok";
    let receipt = json!({"result":if passed{"pass"}else{"failed"},"scope":"existing last seven days; read-only browser snapshots; isolated catalogue; no cookies or network; temporary personal data deleted before receipt is written","cutoff_unix_seconds":cutoff,"profiles":reports,"discovery_failures":discovery.failures.len(),"discovery_warnings":discovery.warnings,"excluded":discovery.excluded,"first":{"items_inserted":first.items_inserted,"views_inserted":first.views_inserted,"views_merged":first.views_merged,"rows_rejected":first.rows_rejected,"profiles_failed":first.profiles_failed},"rerun":{"views_inserted":again.views_inserted,"profiles_unchanged":again.profiles_unchanged},"expected_unique_items":expected.len(),"actual_unique_items":actual,"integrity":integrity});
    drop(c);
    drop(expected);
    let temp_path = temp.0.clone();
    drop(temp);
    if temp_path.exists() {
        return Err(omoi::Error::Config(
            "Temporary acceptance data cleanup failed".into(),
        ));
    }
    std::fs::write(a.out, serde_json::to_vec_pretty(&receipt).unwrap()).map_err(|source| {
        omoi::Error::Io {
            context: "aggregate acceptance receipt",
            source,
        }
    })?;
    println!("{}", receipt);
    if !passed {
        return Err(omoi::Error::Config(
            "Retained-week acceptance failed; see aggregate receipt".into(),
        ));
    }
    Ok(())
}
fn main() {
    if let Err(e) = run(Args::parse()) {
        eprintln!("Acceptance did not pass (code {})", e.exit_code());
        std::process::exit(e.exit_code() as i32);
    }
}
