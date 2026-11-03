use omoi::{db, sessionize};
use rusqlite::Connection;
use serde_json::json;
fn db() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db::migrate(&mut c).unwrap();
    c
}
fn event(seq: i64, at: i64, counter: i64, state: &str) -> serde_json::Value {
    json!({"event_id":format!("test-{seq}"),"producer_id":"test","seq":seq,"url":"https://www.youtube.com/watch?v=abcdefghijk","at_ms":1_700_000_000_000i64+at*1000,"state":state,"position_ms":counter*1000,"cumulative_played_ms":counter*1000,"title":"Fixture","channel":"Studio","duration_s":600})
}
#[test]
fn playback_pause_retry_and_restart() {
    let mut c = db();
    for (seq, at, counter, state) in [
        (1, 0, 0, "playing"),
        (2, 30, 30, "playing"),
        (3, 60, 60, "paused"),
        (4, 120, 60, "playing"),
        (5, 180, 120, "stopped"),
    ] {
        let e = event(seq, at, counter, state);
        sessionize::enqueue(&mut c, &e.to_string(), 28800).unwrap();
        let tx = c.transaction().unwrap();
        sessionize::run(&tx).unwrap();
        tx.commit().unwrap();
        sessionize::enqueue(&mut c, &e.to_string(), 28800).unwrap();
    }
    let tx = c.transaction().unwrap();
    sessionize::run(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT sum(played_s) FROM sessions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        120
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(sessionize::total(&c, 1).unwrap(), 120);
}
#[test]
fn playback_gap_seek_and_rollback() {
    let mut c = db();
    for (seq, at, counter) in [(1, 0, 0), (2, 600, 600), (3, 610, 610)] {
        sessionize::enqueue(
            &mut c,
            &event(seq, at, counter, "playing").to_string(),
            28800,
        )
        .unwrap();
    }
    {
        let tx = c.transaction().unwrap();
        sessionize::run(&tx).unwrap();
    }
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    let tx = c.transaction().unwrap();
    sessionize::run(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(sessionize::total(&c, 1).unwrap(), 10);
    assert_eq!(
        c.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    let mut bad = event(4, 611, 611, "playing");
    bad["url"] = json!("https://evil.invalid/watch?v=abcdefghijk");
    assert!(sessionize::enqueue(&mut c, &bad.to_string(), 28800).is_err());
}
#[test]
fn metadata_tags_validate_and_update_fts_atomically() {
    let mut c = db();
    c.execute(
        "INSERT INTO items(platform,external_id,title) VALUES('youtube','abcdefghijk','Old')",
        [],
    )
    .unwrap();
    let mut edit = json!({"id":1,"title":"New title","channel":"Studio","kind":"documentary","duration_s":120,"tags":["Space","space","Science"]});
    omoi::metadata::edit(&mut c, &edit).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM item_tags", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM items_fts WHERE items_fts MATCH 'New'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    edit["duration_s"] = json!(-1);
    assert!(omoi::metadata::edit(&mut c, &edit).is_err());
    assert_eq!(
        c.query_row("SELECT duration_s FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        120
    );
}

#[test]
fn overlapping_tabs_do_not_double_count() {
    let mut c = db();
    for producer in ["one", "two"] {
        for (seq, at) in [(0, 0), (1, 60)] {
            let mut e = event(seq, at, at, "playing");
            e["producer_id"] = json!(producer);
            e["event_id"] = json!(format!("{producer}-{seq}"));
            sessionize::enqueue(&mut c, &e.to_string(), 28800).unwrap();
        }
    }
    let tx = c.transaction().unwrap();
    sessionize::run(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT sum(played_s) FROM sessions", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        120
    );
    assert_eq!(sessionize::total(&c, 1).unwrap(), 60);
}

#[test]
fn upgrade_preserves_existing_catalogue_and_manual_kind() {
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch(omoi::db::schema::INITIAL).unwrap();
    c.execute_batch("PRAGMA user_version=1; INSERT INTO items(platform,external_id,title) VALUES('youtube','abcdefghijk','Existing')").unwrap();
    db::migrate(&mut c).unwrap();
    omoi::metadata::edit(&mut c,&json!({"id":1,"title":"Corrected","channel":"Owner","kind":"video","duration_s":42,"tags":[]})).unwrap();
    let tx = c.transaction().unwrap();
    omoi::dedupe::ingest(
        &tx,
        &omoi::dedupe::Candidate {
            key: omoi::normalize::ItemKey {
                platform: "youtube".into(),
                external_id: "abcdefghijk".into(),
            },
            started_at: 1,
            source: "edge".into(),
            title: Some("Imported".into()),
            kind: Some("short".into()),
            approx_duration_s: None,
        },
    )
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT title,kind FROM items", [], |r| Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?
        )))
        .unwrap(),
        ("Corrected".into(), "video".into())
    );
}

#[test]
fn listener_requires_exact_host_token_and_absent_origin() {
    let token = "a".repeat(64);
    assert!(omoi::listener::authorized(
        "127.0.0.1:43821",
        None,
        &token,
        "127.0.0.1:43821",
        &token
    ));
    for origin in ["null", "https://www.youtube.com", "chrome-extension://x"] {
        assert!(!omoi::listener::authorized(
            "127.0.0.1:43821",
            Some(origin),
            &token,
            "127.0.0.1:43821",
            &token
        ));
    }
    assert!(!omoi::listener::authorized(
        "localhost:43821",
        None,
        &token,
        "127.0.0.1:43821",
        &token
    ));
}

#[test]
fn inbox_caps_count_and_body_bytes() {
    let mut c = db();
    c.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<100000) INSERT INTO raw_events(at,source,body) SELECT x,'fixture','{}' FROM n").unwrap();
    sessionize::enqueue(&mut c, &event(0, 0, 0, "playing").to_string(), 28800).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        100000
    );
    c.execute("DELETE FROM raw_events", []).unwrap();
    c.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<1300) INSERT INTO raw_events(at,source,body) SELECT x,'fixture',replace(hex(zeroblob(8000)),'0','x') FROM n").unwrap();
    sessionize::enqueue(&mut c, &event(1, 1, 1, "playing").to_string(), 28800).unwrap();
    assert!(
        c.query_row(
            "SELECT sum(length(CAST(body AS BLOB))) FROM raw_events",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap()
            <= 20000000
    );
    assert!(
        c.query_row(
            "SELECT value FROM playback_stats WHERE key='dropped'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap()
            > 1
    );
}

#[test]
fn live_stream_identity_survives_midnight() {
    let mut c = db();
    for (seq, at) in [(0, 1792857599000i64), (1, 1792857601000i64)] {
        let mut e = event(seq, 0, seq * 2, "playing");
        e["url"] = json!("https://www.twitch.tv/maria");
        e["at_ms"] = json!(at);
        sessionize::enqueue(&mut c, &e.to_string(), 28800).unwrap();
    }
    let tx = c.transaction().unwrap();
    sessionize::run(&tx).unwrap();
    tx.commit().unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
