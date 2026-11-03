use rusqlite::Connection;
use std::{
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "omoi-pipeline-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn setup(&self) {
        let p = self.0.join("History");
        let c = Connection::open(&p).unwrap();
        c.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY,url TEXT,title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY,url INTEGER,visit_time INTEGER,visit_duration INTEGER); INSERT INTO urls VALUES(1,'https://youtube.com/watch?v=dQw4w9WgXcQ','A title'),(2,'https://example.com/','Not a video'); INSERT INTO visits VALUES(1,1,13300000000000000,2500000),(2,1,13300000010000000,1000000),(3,2,13300000020000000,0);").unwrap();
        std::fs::create_dir(self.0.join("home")).unwrap();
        let path = format!("chrome:{}", p.display());
        std::fs::write(
            self.0.join("home/config.toml"),
            format!(
                "profile_paths=[{}]\n",
                serde_json::to_string(&path).unwrap()
            ),
        )
        .unwrap();
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_omoi"))
            .args(args)
            .env("OMOI_HOME", self.0.join("home"))
            .env("LOCALAPPDATA", self.0.join("empty"))
            .env("APPDATA", self.0.join("empty"))
            .env("HOME", self.0.join("empty"))
            .env("XDG_CONFIG_HOME", self.0.join("empty"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn collect_week_idempotent() {
    let f = Fixture::new();
    f.setup();
    let a = f.run(&["collect"]);
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    let v: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    assert_eq!(v["views_inserted"], 1);
    assert_eq!(v["rows_rejected"], 1);
    let b = f.run(&["collect"]);
    assert!(b.status.success());
    let v: serde_json::Value = serde_json::from_slice(&b.stdout).unwrap();
    assert_eq!(v["profiles_unchanged"], 1);
    let e = f.run(&["export", "--json"]);
    assert!(e.status.success());
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&e.stdout).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["title"], "A title");
    assert_eq!(rows[0]["approx_duration_s"], 2);
}

#[test]
fn collect_write_failure_rolls_back_items_and_checkpoints() {
    let f = Fixture::new();
    f.setup();
    assert!(f.run(&["db", "migrate"]).status.success());
    let c = Connection::open(f.0.join("home/history.sqlite")).unwrap();
    c.execute_batch("CREATE TRIGGER fail_view BEFORE INSERT ON views BEGIN SELECT RAISE(ABORT,'injected write failure'); END").unwrap();
    let failed = f.run(&["collect"]);
    assert!(!failed.status.success());
    for table in ["items", "views", "checkpoints"] {
        assert_eq!(
            c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    c.execute_batch("DROP TRIGGER fail_view").unwrap();
    let retry = f.run(&["collect"]);
    assert!(retry.status.success());
    let report: serde_json::Value = serde_json::from_slice(&retry.stdout).unwrap();
    assert_eq!(report["views_inserted"], 1);
}
#[test]
fn collect_dry_run() {
    let f = Fixture::new();
    f.setup();
    let r = f.run(&["collect", "--dry-run"]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert!(!f.0.join("home/history.sqlite").exists());
    let v: serde_json::Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(v["views_inserted"], 1);
}
#[test]
fn cli_surface() {
    let f = Fixture::new();
    assert!(f.run(&["--help"]).status.success());
    assert_eq!(f.run(&["export", "--csv", "--json"]).status.code(), Some(2));
    assert_eq!(
        f.run(&["collect", "--browser", "invalid"]).status.code(),
        Some(2)
    );
    assert!(f.run(&["paths"]).status.success());
    assert!(!f.0.join("home").exists());
}

#[test]
fn malformed_rows_are_counted_and_checkpointed() {
    let f = Fixture::new();
    f.setup();
    let c = Connection::open(f.0.join("History")).unwrap();
    c.execute(
        "INSERT INTO urls VALUES(3,?1,'oversized')",
        ["x".repeat(8193)],
    )
    .unwrap();
    c.execute_batch("INSERT INTO visits VALUES(4,3,13300003000000000,0),(5,1,NULL,0)")
        .unwrap();
    drop(c);
    let r = f.run(&["collect"]);
    assert!(r.status.success());
    let report: serde_json::Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(report["rows_read"], 5);
    assert_eq!(report["rows_rejected"], 3);
    let r = f.run(&["collect"]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&r.stdout).unwrap()["profiles_unchanged"],
        1
    );
}

#[test]
fn concurrent_collectors_never_duplicate() {
    let f = Fixture::new();
    f.setup();
    std::thread::scope(|scope| {
        let a = scope.spawn(|| f.run(&["collect"]));
        let b = scope.spawn(|| f.run(&["collect"]));
        for r in [a.join().unwrap(), b.join().unwrap()] {
            assert!(
                matches!(r.status.code(), Some(0 | 4)),
                "{}",
                String::from_utf8_lossy(&r.stderr)
            );
        }
    });
    let r = f.run(&["collect"]);
    assert!(r.status.success());
    let e = f.run(&["export", "--json"]);
    assert_eq!(
        serde_json::from_slice::<Vec<serde_json::Value>>(&e.stdout)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn collect_partial_and_export_no_clobber() {
    let f = Fixture::new();
    f.setup();
    let bad = f.0.join("corrupt");
    std::fs::write(&bad, b"not sqlite").unwrap();
    let config = f.0.join("home/config.toml");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text = text.replace(
        "]",
        &format!(
            ",{}]",
            serde_json::to_string(&format!("firefox:{}", bad.display())).unwrap()
        ),
    );
    std::fs::write(config, text).unwrap();
    let r = f.run(&["collect"]);
    assert_eq!(
        r.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&r.stderr)
    );
    let db = Connection::open(f.0.join("home/history.sqlite")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM checkpoints WHERE source NOT LIKE 'maintenance:%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    drop(db);
    let out = f.0.join("result.csv");
    let out = out.to_str().unwrap();
    assert!(f.run(&["export", "--csv", "--out", out]).status.success());
    let before = std::fs::read(out).unwrap();
    assert!(!f.run(&["export", "--csv", "--out", out]).status.success());
    assert_eq!(before, std::fs::read(out).unwrap());
}

#[test]
fn incremental_and_fixed_anchor() {
    let f = Fixture::new();
    f.setup();
    assert!(f.run(&["collect"]).status.success());
    let c = Connection::open(f.0.join("History")).unwrap();
    c.execute_batch(
        "INSERT INTO visits VALUES(4,1,13300001200000000,0),(5,1,13300002400000000,0);",
    )
    .unwrap();
    drop(c);
    assert!(f.run(&["collect"]).status.success());
    let r = f.run(&["export", "--json"]);
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(f.run(&["collect", "--dry-run"]).status.success());
    let r = f.run(&["export", "--json"]);
    assert_eq!(
        rows,
        serde_json::from_slice::<Vec<serde_json::Value>>(&r.stdout).unwrap()
    );
}

#[test]
fn firefox_wal_snapshot() {
    let f = Fixture::new();
    let profile = f.0.join("places.sqlite");
    let c = Connection::open(&profile).unwrap();
    c.execute_batch("PRAGMA journal_mode=WAL;PRAGMA wal_autocheckpoint=0;CREATE TABLE moz_places(id INTEGER PRIMARY KEY,url TEXT,title TEXT);CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY,place_id INTEGER,visit_date INTEGER);INSERT INTO moz_places VALUES(1,'https://netflix.com/watch/42','Movie');INSERT INTO moz_historyvisits VALUES(1,1,1700000000000000);").unwrap();
    std::fs::create_dir(f.0.join("home")).unwrap();
    std::fs::write(
        f.0.join("home/config.toml"),
        format!(
            "profile_paths=[{}]",
            serde_json::to_string(&format!("firefox:{}", profile.display())).unwrap()
        ),
    )
    .unwrap();
    let r = f.run(&["collect"]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    c.execute_batch("INSERT INTO moz_historyvisits VALUES(2,1,1700004000000000)")
        .unwrap();
    let r = f.run(&["collect"]);
    assert!(r.status.success());
    let e = f.run(&["export", "--json"]);
    assert_eq!(
        serde_json::from_slice::<Vec<serde_json::Value>>(&e.stdout)
            .unwrap()
            .len(),
        2
    );
    drop(c);
}
