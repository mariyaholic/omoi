use omoi::{
    config::Config,
    db,
    dedupe::{self, Candidate},
    normalize::ItemKey,
    readers::snapshot::TempDir,
};
use rusqlite::Connection;
#[test]
fn read_only_no_creation_or_mutation() {
    let dir = TempDir::create(&std::env::temp_dir().join("omoi-db-tests")).unwrap();
    let path = dir.0.join("catalogue.sqlite");
    assert!(db::open(&path, &Config::default(), false).is_err());
    assert!(!path.exists());
    let mut writable = db::open(&path, &Config::default(), true).unwrap();
    db::migrate(&mut writable).unwrap();
    let mode: String = writable
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    drop(writable);
    let before = std::fs::read(&path).unwrap();
    let c = db::open(&path, &Config::default(), false).unwrap();
    assert!(c
        .execute(
            "INSERT INTO items(platform,external_id) VALUES('youtube','abcdefghijk')",
            []
        )
        .is_err());
    assert_eq!(db::info(&c).unwrap().counts["items"], 0);
    drop(c);
    assert_eq!(before, std::fs::read(&path).unwrap());
}
fn candidate(source: &str, title: &str, time: i64, duration: i64) -> Candidate {
    Candidate {
        key: ItemKey {
            platform: "youtube".into(),
            external_id: "abcdefghijk".into(),
        },
        started_at: time,
        source: source.into(),
        title: Some(title.into()),
        kind: Some("video".into()),
        approx_duration_s: Some(duration),
    }
}
#[test]
fn dedupe_richness_approx_max_and_rollback() {
    let mut c = Connection::open_in_memory().unwrap();
    db::migrate(&mut c).unwrap();
    {
        let tx = c.transaction().unwrap();
        dedupe::ingest(&tx, &candidate("firefox", "Firefox title", 0, 10)).unwrap();
        dedupe::ingest(&tx, &candidate("chrome", "Chrome title", 60, 20)).unwrap();
        let row: (i64, String, i64) = tx
            .query_row(
                "SELECT started_at,source,approx_duration_s FROM views",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, (0, "chrome".into(), 20));
        let title: String = tx
            .query_row("SELECT title FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Chrome title");
        // Intentionally drop without commit, including FTS and metadata changes.
    }
    let info = db::info(&c).unwrap();
    assert_eq!(info.counts["items"], 0);
    assert_eq!(info.counts["views"], 0);
    let n: i64 = c
        .query_row(
            "SELECT count(*) FROM items_fts WHERE items_fts MATCH 'Chrome'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 0);
}
#[test]
fn sessionize_empty_inbox() {
    assert!(omoi::sessionize::available());
    let mut c = rusqlite::Connection::open_in_memory().unwrap();
    omoi::db::migrate(&mut c).unwrap();
    let tx = c.transaction().unwrap();
    assert_eq!(omoi::sessionize::run(&tx).unwrap(), 0);
}
#[cfg(not(windows))]
#[test]
fn schedule_linux_stub() {
    for mode in ["install", "remove", "status"] {
        assert!(matches!(
            omoi::scheduler::execute(mode, std::path::Path::new("/nonexistent/omoi-test")),
            Err(omoi::Error::Unsupported(_))
        ));
    }
    assert_eq!(
        std::process::Command::new(env!("CARGO_BIN_EXE_omoiw"))
            .status()
            .unwrap()
            .code(),
        Some(5)
    );
}
