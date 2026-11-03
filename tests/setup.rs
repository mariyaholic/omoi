use omoi::{
    db,
    setup::{self, Choice, Hello, Setup},
};
use rusqlite::Connection;
fn db() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    db::migrate(&mut c).unwrap();
    c
}
fn change(c: &mut Connection, generation: u64, choice: &str) -> Setup {
    setup::change(
        c,
        serde_json::from_value(serde_json::json!({"generation":generation,"choice":choice}))
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn resumable_setup_without_catalogue_migration() {
    let mut c = db();
    assert_eq!(Setup::load(&c).unwrap().choice, Choice::NotChosen);
    let s = change(&mut c, 0, "skipped");
    assert_eq!(s.choice, Choice::Skipped);
    let s = change(&mut c, 1, "enabled");
    assert_eq!(s.client_state(100), "unknown");
    assert!(setup::change(
        &mut c,
        serde_json::from_value(serde_json::json!({"generation":1,"choice":"stopped"})).unwrap()
    )
    .is_err());
    let s = change(&mut c, 2, "stopped");
    assert_eq!(s.binding, Setup::load(&c).unwrap().binding);
    assert_eq!(change(&mut c, 3, "enabled").client_state(100), "unknown");
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM items", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn client_contact_is_not_recording_and_expires() {
    let mut c = db();
    change(&mut c, 0, "enabled");
    for (version, protocol, permission, state) in [
        ("0.1.1", 1, true, "incompatible"),
        ("0.2.0", 2, true, "incompatible"),
        ("0.2.0", 1, false, "permission_needed"),
        ("0.2.0", 1, true, "hello_observed"),
    ] {
        let s = setup::observe(
            &mut c,
            Hello {
                version: version.into(),
                protocol,
                permission,
            },
        )
        .unwrap();
        let now = s.last_seen.unwrap();
        assert_eq!(s.client_state(now), state);
        assert_eq!(s.client_state(now + 46), "unknown");
        assert_eq!(s.last_event, None);
        assert_eq!(s.last_processed, None);
    }
    change(&mut c, 1, "stopped");
    assert!(setup::observe(
        &mut c,
        Hello {
            version: "0.2.0".into(),
            protocol: 1,
            permission: true
        }
    )
    .is_err());
}
#[test]
fn progress_and_profile_changes_preserve_capture_epoch() {
    let mut c = db();
    let before = change(&mut c, 0, "enabled");
    let after = setup::change(
        &mut c,
        serde_json::from_value(
            serde_json::json!({"generation":1,"step":"play","profiles":["edge:fixture"]}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(before.capture_epoch, after.capture_epoch);
    assert_ne!(before.generation, after.generation);
    assert!(change(&mut c, 2, "stopped").capture_epoch > after.capture_epoch);
    assert!(Hello {
        protocol: 1,
        version: "0.2.1".into(),
        permission: true
    }
    .compatible());
}
