use omoi::{
    config::Config,
    db,
    listener::Managed,
    paths::{PathEnvironment, Paths},
    setup,
};
use std::{
    collections::BTreeMap,
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};
fn home() -> (omoi::readers::snapshot::TempDir, Paths) {
    let t = omoi::readers::snapshot::TempDir::create(&std::env::temp_dir()).unwrap();
    let p = Paths::resolve(&PathEnvironment {
        windows: cfg!(windows),
        vars: BTreeMap::from([("OMOI_HOME".into(), t.0.clone().into_os_string())]),
    })
    .unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::fs::write(&p.config, format!("listener_port={port}\nbrowsers=[]\n")).unwrap();
    let mut c = db::open(&p.db, &Config::default(), true).unwrap();
    db::migrate(&mut c).unwrap();
    setup::change(
        &mut c,
        serde_json::from_value(serde_json::json!({"generation":0,"choice":"enabled"})).unwrap(),
    )
    .unwrap();
    (t, p)
}
#[test]
fn owned_listener_releases_port_with_unfinished_client() {
    let (_t, p) = home();
    let port = Config::load(&p.config).unwrap().listener_port;
    let managed = Managed::start(p.clone()).unwrap();
    let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
    use std::io::Write;
    stream.write_all(b"POST /event HTTP/1.1\r\nHost:").unwrap();
    std::thread::sleep(Duration::from_millis(150));
    let start = Instant::now();
    drop(managed);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).is_ok());
}
#[test]
fn collision_preserves_owner_and_credential() {
    let (_t, p) = home();
    let managed = Managed::start(p.clone()).unwrap();
    let credential = std::fs::read(p.home.join("listener/token")).unwrap();
    assert!(Managed::start(p.clone()).is_err());
    assert_eq!(
        credential,
        std::fs::read(p.home.join("listener/token")).unwrap()
    );
    assert_eq!(*managed.state.lock().unwrap(), "ready");
    drop(managed);
}
#[test]
fn startup_drains_pending_without_a_process_click() {
    let (_t, p) = home();
    let mut c = db::open(&p.db, &Config::default(), true).unwrap();
    let event = serde_json::json!({"event_id":"pending:0","producer_id":"pending","seq":0,"url":"https://www.youtube.com/watch?v=abcdefghijk","at_ms":1700000000000i64,"state":"playing","position_ms":0,"cumulative_played_ms":0,"title":"Startup fixture","channel":null,"duration_s":60});
    omoi::sessionize::enqueue(&mut c, &event.to_string(), 0).unwrap();
    let _managed = Managed::start(p).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while c
        .query_row("SELECT count(*) FROM raw_events", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap()
        != 0
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        c.query_row("SELECT count(*) FROM sessions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}
