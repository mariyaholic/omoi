use omoi::{
    db,
    dedupe::{self, Candidate},
    metadata,
    normalize::ItemKey,
    sessionize, ui,
};
use rusqlite::Connection;
use serde_json::json;

fn database() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db::migrate(&mut c).unwrap();
    c
}
fn candidate(key: &str, at: i64) -> Candidate {
    Candidate {
        key: ItemKey {
            platform: "youtube".into(),
            external_id: key.into(),
        },
        started_at: at,
        source: "chrome".into(),
        title: Some("Browser title".into()),
        kind: None,
        approx_duration_s: None,
    }
}
fn ingest(c: &mut Connection, row: &Candidate) {
    let tx = c.transaction().unwrap();
    dedupe::ingest(&tx, row).unwrap();
    tx.commit().unwrap();
}
fn event(seq: i64, at: i64, played: i64) -> serde_json::Value {
    json!({"event_id":format!("event-{seq}"),"producer_id":"producer","seq":seq,
        "url":"https://www.youtube.com/watch?v=abcdefghijk","at_ms":at,
        "state":"playing","position_ms":played,"cumulative_played_ms":played,
        "title":null,"channel":null,"duration_s":null})
}
fn process(c: &mut Connection) {
    let tx = c.transaction().unwrap();
    sessionize::run(&tx).unwrap();
    tx.commit().unwrap();
}

#[test]
fn deleted_identity_cannot_be_reused_by_a_stale_edit() {
    let mut c = database();
    ingest(&mut c, &candidate("abcdefghijk", 1000));
    assert!(ui::delete_item(&mut c, 1).unwrap());
    ingest(&mut c, &candidate("12345678901", 2000));
    let id: i64 = c
        .query_row("SELECT id FROM items", [], |r| r.get(0))
        .unwrap();
    assert_ne!(id, 1);
    assert!(metadata::edit(
        &mut c,
        &json!({"id":1,"title":"stale","channel":"","kind":"","duration_s":null,"tags":[]})
    )
    .is_err());
}

#[test]
fn historical_reconciliation_does_not_resurrect_a_deleted_item() {
    let mut c = database();
    ingest(&mut c, &candidate("abcdefghijk", 1000));
    ui::delete_item(&mut c, 1).unwrap();
    ingest(&mut c, &candidate("abcdefghijk", 1000));
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    ingest(
        &mut c,
        &candidate("abcdefghijk", chrono::Utc::now().timestamp() + 1),
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn title_provenance_follows_the_actual_winning_writer() {
    let mut c = database();
    let mut row = candidate("abcdefghijk", 1000);
    row.title = None;
    ingest(&mut c, &row);
    let tx = c.transaction().unwrap();
    metadata::observed(&tx, 1, Some("Player title"), None, None).unwrap();
    tx.commit().unwrap();
    ingest(&mut c, &candidate("abcdefghijk", 1000));
    let pair:(String,String)=c.query_row("SELECT title,source FROM items JOIN metadata_fields ON item_id=items.id WHERE field='title'",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(pair, ("Browser title".into(), "chrome".into()));
}

#[test]
fn fractional_interval_preserves_an_exact_second() {
    let mut c = database();
    for e in [event(0, 100000, 0), event(1, 101062, 1000)] {
        sessionize::enqueue(&mut c, &e.to_string(), 0).unwrap();
    }
    process(&mut c);
    assert_eq!(sessionize::total(&c, 1).unwrap(), 1);
}

#[test]
fn duplicate_receipt_cannot_create_a_live_identity() {
    let mut c = database();
    let e = event(0, 1700000000000, 0);
    sessionize::enqueue(&mut c, &e.to_string(), 0).unwrap();
    let mut duplicate = e;
    duplicate["producer_id"] = json!("other");
    duplicate["url"] = json!("https://www.twitch.tv/maria");
    assert!(!sessionize::enqueue(&mut c, &duplicate.to_string(), 0).unwrap());
    assert_eq!(
        c.query_row("SELECT count(*) FROM playback_live_keys", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn live_identity_changes_after_an_ended_broadcast() {
    let mut c = database();
    for (seq, at, state) in [
        (0, 1700000000000, "playing"),
        (1, 1700000001000, "stopped"),
        (2, 1700086400000, "playing"),
    ] {
        let mut e = event(seq, at, seq * 1000);
        e["url"] = json!("https://www.twitch.tv/maria");
        e["state"] = json!(state);
        sessionize::enqueue(&mut c, &e.to_string(), 0).unwrap();
        process(&mut c);
    }
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn rejected_live_stop_cannot_split_midnight_identity() {
    for batched in [false, true] {
        let mut c = database();
        let midnight = 1_700_006_400_000i64;
        for (seq, offset, counter, state) in [
            (0, -30000, 0, "playing"),
            (1, -10000, 20000, "playing"),
            (2, -5000, 10000, "stopped"),
            (3, 10000, 40000, "playing"),
        ] {
            let mut e = event(seq, midnight + offset, counter);
            e["url"] = json!("https://www.twitch.tv/maria");
            e["state"] = json!(state);
            sessionize::enqueue(&mut c, &e.to_string(), 28800).unwrap();
            if !batched {
                process(&mut c);
            }
        }
        process(&mut c);
        assert_eq!(
            c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(sessionize::total(&c, 1).unwrap(), 40);
        assert_eq!(
            c.query_row(
                "SELECT value FROM playback_stats WHERE key='late'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        let identity: (String, i64, bool) = c
            .query_row(
                "SELECT external_id,last_seq,stopped FROM playback_live_keys",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(identity, ("live:maria:2023-11-15".into(), 3, false));
    }
}

#[test]
fn legacy_enqueue_frontier_cannot_override_accepted_live_state() {
    let mut c = database();
    let midnight = 1_700_006_400_000i64;
    let mut first = event(0, midnight - 10000, 0);
    first["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &first.to_string(), 28800).unwrap();
    process(&mut c);
    // An older writer could advance this table for an event never accepted by
    // playback_state. Only the latter is authoritative after upgrade/reopen.
    c.execute(
        "UPDATE playback_live_keys SET external_id='live:maria:2023-11-16',last_seq=100,stopped=1",
        [],
    )
    .unwrap();
    let mut next = event(1, midnight + 10000, 20000);
    next["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &next.to_string(), 28800).unwrap();
    process(&mut c);
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(sessionize::total(&c, 1).unwrap(), 20);
}

#[test]
fn evicted_unprocessed_event_can_be_retried() {
    let mut c = database();
    let first = event(0, 1700000000000, 0);
    sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap();
    c.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<100000) INSERT INTO raw_events(at,source,body) SELECT x,'fixture','{}' FROM n").unwrap();
    sessionize::enqueue(&mut c, &event(1, 1700000001000, 1000).to_string(), 0).unwrap();
    assert!(sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap());
}

#[test]
fn orphaned_logging_path_does_not_disable_logging() {
    let temp = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    std::fs::write(temp.0.join("logging.lock"), b"").unwrap();
    omoi::logging::record(&temp.0, &omoi::config::Config::default(), None, 0).unwrap();
}

#[test]
fn durable_frontier_rejects_late_events_after_reopen() {
    let temp = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    let path = temp.0.join("history.sqlite");
    {
        let mut c = Connection::open(&path).unwrap();
        db::migrate(&mut c).unwrap();
        for e in [event(0, 100000, 0), event(2, 102000, 2000)] {
            sessionize::enqueue(&mut c, &e.to_string(), 0).unwrap();
        }
        process(&mut c);
    }
    let mut c = Connection::open(&path).unwrap();
    sessionize::enqueue(&mut c, &event(1, 101000, 1000).to_string(), 0).unwrap();
    process(&mut c);
    assert_eq!(sessionize::total(&c, 1).unwrap(), 2);
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    sessionize::enqueue(&mut c, &event(3, 103000, 3000).to_string(), 0).unwrap();
    process(&mut c);
    assert_eq!(sessionize::total(&c, 1).unwrap(), 3);
}

#[test]
fn pending_playback_cannot_resurrect_a_deleted_item() {
    let mut c = database();
    ingest(&mut c, &candidate("abcdefghijk", 100));
    sessionize::enqueue(&mut c, &event(0, 100000, 0).to_string(), 0).unwrap();
    ui::delete_item(&mut c, 1).unwrap();
    process(&mut c);
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn pending_live_continuation_cannot_resurrect_after_midnight_deletion() {
    let mut c = database();
    let midnight = 1_700_006_400_000i64;
    let mut first = event(0, midnight - 10000, 0);
    first["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &first.to_string(), 28800).unwrap();
    process(&mut c);
    let mut next = event(1, midnight + 10000, 20000);
    next["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &next.to_string(), 28800).unwrap();
    assert!(ui::delete_item(&mut c, 1).unwrap());
    process(&mut c);
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn queued_live_continuation_survives_batch_boundary_deletion_and_reopen() {
    let temp = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    let path = temp.0.join("history.sqlite");
    let mut c = Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db::migrate(&mut c).unwrap();
    let midnight = 1_700_092_800_000i64;
    let mut first = event(0, midnight - 1000, 0);
    first["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap();
    for index in 0..255 {
        let mut filler = event(0, midnight - 500, 0);
        filler["event_id"] = json!(format!("filler-{index}"));
        filler["producer_id"] = json!(format!("filler-{index}"));
        sessionize::enqueue(&mut c, &filler.to_string(), 0).unwrap();
    }
    let mut next = event(1, midnight + 1000, 2000);
    next["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &next.to_string(), 0).unwrap();
    let raw: String = c
        .query_row(
            "SELECT body FROM raw_events ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&raw).unwrap()["deletion_key"].is_null());
    assert_eq!(sessionize::process_batch(&mut c).unwrap(), 256);
    let item: i64 = c
        .query_row("SELECT id FROM items WHERE platform='twitch'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(ui::delete_item(&mut c, item).unwrap());
    drop(c);
    let mut c = Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    assert_eq!(sessionize::process_batch(&mut c).unwrap(), 0);
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM items WHERE platform='twitch'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn long_deleted_live_backlog_advances_only_validated_continuity_across_reopen() {
    let temp = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    let path = temp.0.join("history.sqlite");
    let mut c = Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db::migrate(&mut c).unwrap();
    let start = 1_700_092_800_000i64 - 60000;
    let mut first = event(0, start, 0);
    first["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap();
    process(&mut c);
    let old_key: String = c
        .query_row("SELECT external_id FROM items", [], |r| r.get(0))
        .unwrap();
    for index in 0..255 {
        let mut filler = event(0, start + 1, 0);
        filler["event_id"] = json!(format!("filler-{index}"));
        filler["producer_id"] = json!(format!("filler-{index}"));
        sessionize::enqueue(&mut c, &filler.to_string(), 0).unwrap();
    }
    for seq in 1..=8 {
        let mut next = event(seq, start + seq * 120000, seq * 120000);
        next["url"] = first["url"].clone();
        sessionize::enqueue(&mut c, &next.to_string(), 0).unwrap();
    }
    // A counter-regressing stop must not end the suppressed chain or advance
    // its sequence frontier, even though it claims a higher sequence number.
    let mut rejected = event(100, start + 130000, 0);
    rejected["url"] = first["url"].clone();
    rejected["state"] = json!("stopped");
    sessionize::enqueue(&mut c, &rejected.to_string(), 0).unwrap();
    ui::delete_item(&mut c, 1).unwrap();
    assert_eq!(sessionize::process_batch(&mut c).unwrap(), 255);
    assert_eq!(
        c.query_row(
            "SELECT seq FROM playback_frontier WHERE producer='producer'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(c);
    let mut c = Connection::open(&path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    assert_eq!(sessionize::process_batch(&mut c).unwrap(), 0);
    let frontier: (i64, String, i64, bool) = c
        .query_row(
            "SELECT seq,external_id,at_ms,stopped FROM playback_frontier WHERE producer='producer'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(frontier, (8, old_key, start + 960000, false));
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM items WHERE platform='twitch'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM playback_state WHERE producer='producer'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM raw_events", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row(
            "SELECT value FROM playback_stats WHERE key='late'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn deleted_live_continuity_respects_queued_stop_channel_and_nonlive_transitions() {
    for transition in ["stop", "channel", "nonlive"] {
        let mut c = database();
        let midnight = 1_700_092_800_000i64;
        let mut first = event(0, midnight - 60000, 0);
        first["url"] = json!("https://www.twitch.tv/maria");
        sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap();
        process(&mut c);
        let old: String = c
            .query_row("SELECT external_id FROM items", [], |r| r.get(0))
            .unwrap();
        let mut boundary = event(1, midnight - 30000, 30000);
        if transition == "stop" {
            boundary["url"] = first["url"].clone();
            boundary["state"] = json!("stopped");
        } else if transition == "channel" {
            boundary["url"] = json!("https://www.twitch.tv/other");
        }
        sessionize::enqueue(&mut c, &boundary.to_string(), 0).unwrap();
        let mut next = event(2, midnight + 1000, 61000);
        next["url"] = first["url"].clone();
        sessionize::enqueue(&mut c, &next.to_string(), 0).unwrap();
        ui::delete_item(&mut c, 1).unwrap();
        process(&mut c);
        let current: String = c
            .query_row(
                "SELECT external_id FROM playback_frontier WHERE producer='producer'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(current, old, "transition {transition}");
        assert!(current.starts_with("live:maria:"));
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM items WHERE external_id=?1",
                [&current],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            c.query_row(
                "SELECT count(*) FROM items WHERE external_id=?1",
                [&old],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn nonlive_playback_does_not_retain_a_deletion_frontier() {
    let mut c = database();
    sessionize::enqueue(&mut c, &event(0, 100000, 0).to_string(), 0).unwrap();
    process(&mut c);
    assert_eq!(
        c.query_row("SELECT count(*) FROM playback_frontier", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    ui::delete_item(&mut c, 1).unwrap();
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
        0
    );
}

#[test]
fn legacy_preview_and_selected_migration_allow_mixed_historical_offsets() {
    let mut c = database();
    let at = 1_700_092_800_000i64 - 1000;
    for index in 0..2 {
        let mut e = event(index, at, 0);
        e["url"] = json!("https://www.twitch.tv/maria");
        let body =
            json!({"event":e,"key":{"platform":"twitch","external_id":"live:maria:2023-11-16"}})
                .to_string();
        c.execute(
            "INSERT INTO raw_events(at,source,body) VALUES(?1,'userscript',?2)",
            rusqlite::params![at, body],
        )
        .unwrap();
    }
    let before: String = c
        .query_row("SELECT group_concat(body) FROM raw_events", [], |r| {
            r.get(0)
        })
        .unwrap();
    let preview = sessionize::preview_legacy_live(&c, 28800, 0).unwrap();
    assert_eq!(preview.rows.len(), 2);
    assert_eq!(preview.next_after_id, 2);
    assert_eq!(
        c.query_row("SELECT group_concat(body) FROM raw_events", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        before
    );
    assert!(sessionize::migrate_legacy_live(&mut c, 0, &[]).is_err());
    assert!(
        sessionize::migrate_legacy_live(&mut c, 0, &["event-0".into(), "missing".into()]).is_err()
    );
    assert_eq!(
        c.query_row("SELECT group_concat(body) FROM raw_events", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        before
    );
    assert_eq!(
        sessionize::migrate_legacy_live(&mut c, 0, &["event-0".into()]).unwrap(),
        1
    );
    let remaining = sessionize::preview_legacy_live(&c, 28800, 0).unwrap();
    assert_eq!(remaining.rows.len(), 1);
    assert_eq!(remaining.rows[0].event_id, "event-1");
    assert_eq!(
        sessionize::migrate_legacy_live(&mut c, 28800, &["event-1".into()]).unwrap(),
        1
    );
    let keys: Vec<String> = c
        .prepare("SELECT body FROM raw_events ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| {
            serde_json::from_str::<serde_json::Value>(&r.unwrap()).unwrap()["key"]["external_id"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_ne!(keys[0], keys[1]);
    assert!(sessionize::migrate_legacy_live(&mut c, 28800, &["event-0".into()]).is_err());
}

#[test]
fn legacy_poisoned_live_queue_requires_explicit_timezone_before_rekeying() {
    for offset in [0, 28800, -18000] {
        let mut c = database();
        let midnight = 1_700_092_800_000i64 - i64::from(offset) * 1000;
        let mut stopped = event(0, midnight - 2000, 1000);
        stopped["url"] = json!("https://www.twitch.tv/maria");
        stopped["state"] = json!("stopped");
        sessionize::enqueue(&mut c, &stopped.to_string(), offset).unwrap();
        process(&mut c);
        let yesterday: String = c
            .query_row("SELECT external_id FROM items", [], |r| r.get(0))
            .unwrap();
        // Seed bodies exactly in the old Stored shape, pinned by old enqueue.
        for (seq, at, counter) in [(1, midnight - 1000, 900), (2, midnight + 1000, 3000)] {
            let mut e = event(seq, at, counter);
            e["url"] = json!("https://www.twitch.tv/maria");
            let body =
                json!({"event":e,"key":{"platform":"twitch","external_id":yesterday}}).to_string();
            c.execute(
                "INSERT INTO raw_events(at,source,body) VALUES(?1,'userscript',?2)",
                rusqlite::params![at, body],
            )
            .unwrap();
            c.execute(
                "INSERT INTO playback_receipts VALUES(?1,'producer',?2)",
                rusqlite::params![format!("event-{seq}"), seq],
            )
            .unwrap();
        }
        let before: String = c
            .query_row("SELECT group_concat(body) FROM raw_events", [], |r| {
                r.get(0)
            })
            .unwrap();
        {
            let tx = c.transaction().unwrap();
            let error = sessionize::run(&tx).unwrap_err().to_string();
            assert!(error.contains("Legacy Twitch queue"), "{error}");
        }
        assert_eq!(
            c.query_row("SELECT group_concat(body) FROM raw_events", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            before
        );
        assert_eq!(
            sessionize::migrate_legacy_live(&mut c, offset, &["event-1".into(), "event-2".into()])
                .unwrap(),
            2
        );
        assert_eq!(
            sessionize::migrate_legacy_live(&mut c, offset, &["event-1".into(), "event-2".into()])
                .unwrap(),
            0
        );
        process(&mut c);
        assert_eq!(
            c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let current: String = c
            .query_row(
                "SELECT external_id FROM items ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(current, yesterday);
        assert_eq!(
            c.query_row("SELECT count(*) FROM playback_receipts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
}

#[test]
fn legacy_timezone_migration_retains_deletion_identity_and_rolls_back_errors() {
    let mut c = database();
    let midnight = 1_700_092_800_000i64;
    let mut first = event(0, midnight - 1000, 0);
    first["url"] = json!("https://www.twitch.tv/maria");
    sessionize::enqueue(&mut c, &first.to_string(), 0).unwrap();
    process(&mut c);
    let key: String = c
        .query_row("SELECT external_id FROM items", [], |r| r.get(0))
        .unwrap();
    let mut e = event(1, midnight + 1000, 2000);
    e["url"] = json!("https://www.twitch.tv/maria");
    let body = json!({"event":e,"key":{"platform":"twitch","external_id":key}}).to_string();
    c.execute(
        "INSERT INTO raw_events(at,source,body) VALUES(?1,'userscript',?2)",
        rusqlite::params![midnight + 1000, body],
    )
    .unwrap();
    c.execute(
        "INSERT INTO raw_events(at,source,body) VALUES(0,'fixture','broken')",
        [],
    )
    .unwrap();
    assert!(sessionize::migrate_legacy_live(&mut c, 0, &["event-1".into()]).is_err());
    assert_eq!(
        c.query_row(
            "SELECT body FROM raw_events WHERE source='userscript'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        body
    );
    c.execute("DELETE FROM raw_events WHERE source='fixture'", [])
        .unwrap();
    assert!(sessionize::migrate_legacy_live(&mut c, 100000, &["event-1".into()]).is_err());
    assert_eq!(
        sessionize::migrate_legacy_live(&mut c, 0, &["event-1".into()]).unwrap(),
        1
    );
    ui::delete_item(&mut c, 1).unwrap();
    process(&mut c);
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
