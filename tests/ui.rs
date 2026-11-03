use omoi::{
    db,
    ui::{authorized, catalogue, HistoryQuery},
};
use rusqlite::Connection;

fn fixture() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    db::migrate(&mut c).unwrap();
    for id in 1..=61 {
        c.execute("INSERT INTO items(id,platform,external_id,title,channel) VALUES(?1,'youtube',?1,?2,'Studio')", rusqlite::params![id, format!("Video {id}")]).unwrap();
        c.execute(
            "INSERT INTO views(item_id,started_at,source) VALUES(?1,?1,'edge:test')",
            [id],
        )
        .unwrap();
    }
    c
}

#[test]
fn ui_pages_filters_and_literal_search() {
    let c = fixture();
    let mut q = HistoryQuery::default();
    let page = catalogue(&c, &q).unwrap();
    assert_eq!(page["total"], 61);
    assert_eq!(page["items"].as_array().unwrap().len(), 50);
    assert_eq!(page["items"][0]["id"], 61);
    q.offset = 50;
    assert_eq!(
        catalogue(&c, &q).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        11
    );
    q.offset = 0;
    q.query = "Video 61".into();
    assert_eq!(catalogue(&c, &q).unwrap()["total"], 1);
    q.query = "\" OR * --".into();
    assert_eq!(catalogue(&c, &q).unwrap()["total"], 0);
    q.query.clear();
    q.platform = "netflix".into();
    assert_eq!(catalogue(&c, &q).unwrap()["total"], 0);
    q.platform.clear();
    q.channel = "Studio".into();
    q.after = 60;
    assert_eq!(catalogue(&c, &q).unwrap()["total"], 2);
    q.after = 0;
    q.literal = true;
    q.query = "deo 61".into();
    assert_eq!(catalogue(&c, &q).unwrap()["total"], 1);
    q.offset = -1;
    assert!(catalogue(&c, &q).is_err());
}

#[test]
fn ui_delete_cascades_and_updates_fts() {
    let mut c = fixture();
    assert!(omoi::ui::delete_item(&mut c, 61).unwrap());
    assert!(!omoi::ui::delete_item(&mut c, 61).unwrap());
    assert_eq!(
        c.query_row("SELECT count(*) FROM views WHERE item_id=61", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM items_fts WHERE items_fts MATCH '\"Video 61\"'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn ui_history_remains_bounded_after_catalogue_growth() {
    let mut c = fixture();
    c.execute("DELETE FROM items WHERE id>1", []).unwrap();
    c.execute_batch("ANALYZE").unwrap();
    let tx = c.transaction().unwrap();
    for id in 2..=10001 {
        tx.execute(
            "INSERT INTO items(id,platform,external_id,title) VALUES(?1,'youtube',?1,'Video')",
            [id],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO views(item_id,started_at,source) VALUES(?1,?1,'edge:test')",
            [id],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    let start = std::time::Instant::now();
    let page = catalogue(&c, &HistoryQuery::default()).unwrap();
    assert_eq!(page["total"], 10001);
    assert_eq!(page["items"][0]["id"], 10001);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(3),
        "stale statistics caused a repeated table scan"
    );
}

#[test]
fn ui_auth_rejects_foreign_hosts_origins_and_tokens() {
    let token = "a".repeat(64);
    assert!(authorized(
        "127.0.0.1:43822",
        Some("http://127.0.0.1:43822"),
        &token,
        "127.0.0.1:43822",
        &token
    ));
    assert!(!authorized(
        "evil.test:43822",
        None,
        &token,
        "127.0.0.1:43822",
        &token
    ));
    assert!(!authorized(
        "127.0.0.1:43822",
        Some("null"),
        &token,
        "127.0.0.1:43822",
        &token
    ));
    assert!(!authorized(
        "127.0.0.1:43822",
        Some("https://evil.test"),
        &token,
        "127.0.0.1:43822",
        &token
    ));
    assert!(!authorized(
        "127.0.0.1:43822",
        None,
        "",
        "127.0.0.1:43822",
        &token
    ));
}

#[test]
fn desktop_server_lifetime_auth_and_startup_collision() {
    use omoi::{
        paths::{PathEnvironment, Paths},
        readers::snapshot::TempDir,
    };
    use std::{
        collections::BTreeMap,
        io::{Read, Write},
        net::TcpStream,
    };
    let temp = TempDir::create(&std::env::temp_dir().join("omoi-desktop-tests")).unwrap();
    let env_for = |home: &std::path::Path| PathEnvironment {
        windows: cfg!(windows),
        vars: BTreeMap::from([("OMOI_HOME".into(), home.as_os_str().to_owned())]),
    };
    std::fs::write(temp.0.join("config.toml"), "browsers=[]\n").unwrap();
    let env = env_for(&temp.0);
    let server = omoi::ui::start_desktop(Paths::resolve(&env).unwrap(), env, 0).unwrap();
    let url = url::Url::parse(&server.launch_url).unwrap();
    let host = format!("127.0.0.1:{}", url.port().unwrap());
    let token = url.fragment().unwrap().strip_prefix("token=").unwrap();
    for (path, mime, expected) in [
        (
            "/icon.svg",
            "image/svg+xml",
            include_bytes!("../assets/icon.svg").as_slice(),
        ),
        (
            "/favicon.ico",
            "image/x-icon",
            include_bytes!("../assets/icon.ico").as_slice(),
        ),
    ] {
        let mut stream = TcpStream::connect(&host).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let split = response.windows(4).position(|s| s == b"\r\n\r\n").unwrap();
        let headers = String::from_utf8_lossy(&response[..split]);
        assert!(headers.starts_with("HTTP/1.1 200"));
        assert!(headers.contains(&format!("Content-Type: {mime}")));
        assert_eq!(&response[split + 4..], expected);
    }
    for (supplied, expected) in [("", "403"), (token, "200")] {
        let mut stream = TcpStream::connect(&host).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        write!(stream, "GET /api/info HTTP/1.1\r\nHost: {host}\r\nX-Omoi-Token: {supplied}\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}")),
            "{response}"
        );
        if supplied == token {
            assert!(response.contains("\"idle_seconds\":0"));
        }
    }
    let other = temp.0.join("must-not-be-created");
    let env = env_for(&other);
    assert!(
        omoi::ui::start_desktop(Paths::resolve(&env).unwrap(), env, url.port().unwrap()).is_err()
    );
    assert!(!other.exists());
    drop(server);
    assert!(TcpStream::connect(&host).is_err());
}
