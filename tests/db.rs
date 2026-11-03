use omoi::db;
use rusqlite::Connection;
fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    c
}
#[test]
fn migration_0001_idempotent() {
    let mut c = conn();
    db::migrate(&mut c).unwrap();
    db::migrate(&mut c).unwrap();
    assert_eq!(db::info(&c).unwrap().user_version, 2);
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
    c.execute_batch("PRAGMA user_version=3").unwrap();
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
