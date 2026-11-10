use omoi::db;
use rusqlite::Connection;
fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    c
}
#[test]
fn v2_upgrade_preserves_identity_and_rolls_back_on_conflict() {
    let mut c = conn();
    c.execute_batch(include_str!("../src/db/0001.sql")).unwrap();
    c.execute_batch(include_str!("../src/db/0002.sql")).unwrap();
    c.execute_batch("PRAGMA user_version=2; INSERT INTO items(id,platform,external_id,title) VALUES(42,'youtube','abcdefghijk','Original'); CREATE TABLE deleted_items(conflict);").unwrap();
    assert!(db::migrate(&mut c).is_err());
    assert_eq!(db::info(&c).unwrap().user_version, 2);
    assert!(c.prepare("SELECT * FROM item_sequence").is_err());
    c.execute_batch("DROP TABLE deleted_items").unwrap();
    db::migrate(&mut c).unwrap();
    assert_eq!(
        c.query_row("SELECT last_id FROM item_sequence", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        42
    );
    assert_eq!(
        c.query_row("SELECT title FROM items WHERE id=42", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "Original"
    );
    c.execute(
        "INSERT INTO items_fts(items_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}
#[test]
fn migration_0001_idempotent() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    db::migrate(&mut c).unwrap();
    assert_eq!(db::info(&c).unwrap().user_version, db::schema::VERSION);
    for table in [
        "items",
        "views",
        "sessions",
        "item_tags",
        "channel_tags",
        "checkpoints",
        "raw_events",
    ] {
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
#[test]
fn migration_rollback() {
    let mut c = conn();
    assert!(
        db::migrations::apply_initial(&mut c, "CREATE TABLE partial(id); INVALID SQL;").is_err()
    );
    assert_eq!(db::info(&c).unwrap().user_version, 0);
    assert!(c.prepare("SELECT * FROM partial").is_err());
}
#[test]
fn reject_future_schema() {
    let mut c = conn();
    c.execute_batch(&format!("PRAGMA user_version={}", db::schema::VERSION + 1))
        .unwrap();
    assert!(matches!(
        db::migrate(&mut c),
        Err(omoi::Error::SchemaTooNew { .. })
    ));
    let mut c = conn();
    c.execute_batch("CREATE TABLE foreign_data(id)").unwrap();
    assert!(db::migrate(&mut c).is_err());
}
#[test]
fn fts_insert_update_delete() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    c.execute(
        "INSERT INTO items(platform,external_id,title) VALUES('youtube','abc','alpha')",
        [],
    )
    .unwrap();
    let count = |term: &str| {
        c.query_row(
            "SELECT count(*) FROM items_fts WHERE items_fts MATCH ?",
            [term],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
    };
    assert_eq!(count("alpha"), 1);
    c.execute("UPDATE items SET title='beta'", []).unwrap();
    assert_eq!(count("alpha"), 0);
    assert_eq!(count("beta"), 1);
    c.execute("DELETE FROM items", []).unwrap();
    assert_eq!(count("beta"), 0);
    c.execute(
        "INSERT INTO items_fts(items_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}
#[test]
fn foreign_key_cascade() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    c.execute_batch("INSERT INTO items(id,platform,external_id) VALUES(1,'youtube','abc'); INSERT INTO views(item_id,started_at,source) VALUES(1,0,'test'); INSERT INTO sessions(item_id,start,end,played_s,source) VALUES(1,0,1,1,'test'); INSERT INTO item_tags VALUES(1,'tag','test'); DELETE FROM items;").unwrap();
    for table in ["views", "sessions", "item_tags"] {
        assert_eq!(
            c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn required_columns() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    c.prepare("SELECT channel_id FROM items").unwrap();
    c.prepare("SELECT source FROM sessions").unwrap();
}

#[test]
fn v3_upgrade_seeds_only_accepted_frontier_and_rolls_back_conflicts() {
    let mut c = conn();
    for sql in [
        include_str!("../src/db/0001.sql"),
        include_str!("../src/db/0002.sql"),
        include_str!("../src/db/0003.sql"),
    ] {
        c.execute_batch(sql).unwrap();
    }
    c.execute_batch("PRAGMA user_version=3; INSERT INTO items(id,platform,external_id) VALUES(1,'twitch','live:maria:2023-11-16'); INSERT INTO sessions(id,item_id,start,end,played_s,source) VALUES(1,1,0,1,1,'userscript'); INSERT INTO playback_state VALUES('producer',1,1,1,1000,1000,1000,0); INSERT INTO playback_live_keys VALUES('producer','maria','live:maria:poison',999,999999,1); CREATE TABLE playback_frontier(conflict);").unwrap();
    assert!(db::migrate(&mut c).is_err());
    assert_eq!(db::info(&c).unwrap().user_version, 3);
    c.execute_batch("DROP TABLE playback_frontier").unwrap();
    c.execute_batch("INSERT INTO items(id,platform,external_id) VALUES(2,'youtube','abcdefghijk'); INSERT INTO sessions(id,item_id,start,end,played_s,source) VALUES(2,2,0,1,1,'userscript'); INSERT INTO playback_state VALUES('nonlive',1,2,2,1000,1000,1000,0);").unwrap();
    db::migrate(&mut c).unwrap();
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM playback_frontier WHERE producer='nonlive'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let frontier: (String, i64) = c
        .query_row("SELECT external_id,seq FROM playback_frontier", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(frontier, ("live:maria:2023-11-16".into(), 1));
    c.execute_batch("DELETE FROM items").unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM playback_state", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM playback_frontier", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    db::migrate(&mut c).unwrap();
    assert_eq!(db::info(&c).unwrap().user_version, 4);
}

#[test]
fn existing_writable_connection_never_creates_missing_database() {
    let temp = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    let path = temp.0.join("absent.sqlite");
    assert!(db::open_existing_writable(&path, &omoi::config::Config::default()).is_err());
    assert!(!path.exists());
}

#[test]
fn info_reports_counts() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    c.execute(
        "INSERT INTO items(platform,external_id) VALUES('youtube','test')",
        [],
    )
    .unwrap();
    let info = db::info(&c).unwrap();
    assert_eq!(info.counts["items"], 1);
    assert_eq!(info.counts["views"], 0);
    assert_eq!(info.counts.len(), 7);
    assert!(info.fts5);
}

#[test]
fn weekly_optimize_is_idempotent() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    assert!(db::maintenance::optimize_due(&mut c, 1000000).unwrap());
    assert!(!db::maintenance::optimize_due(&mut c, 1000001).unwrap());
    assert!(db::maintenance::optimize_due(&mut c, 1000000 + 7 * 86400).unwrap());
}
