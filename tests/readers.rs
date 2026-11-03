use omoi::readers::{
    snapshot::{self, TempDir},
    sqlite::SqliteReader,
    Browser, Cursor, HistoryReader, SourceId,
};
use rusqlite::Connection;
fn reader(browser: Browser, sql: &str) -> (TempDir, SqliteReader) {
    let dir = TempDir::create(&std::env::temp_dir().join("omoi-reader-tests")).unwrap();
    let path = dir.0.join("History");
    let c = Connection::open(&path).unwrap();
    c.execute_batch(sql).unwrap();
    drop(c);
    let snap = snapshot::copy(&path, &dir.0.join("tmp"), 3).unwrap();
    let r = SqliteReader::open(SourceId("fixture".into()), browser, snap).unwrap();
    (dir, r)
}
#[test]
fn chrome_tuple_pagination_and_epoch() {
    let (_dir, mut r) = reader(Browser::Chrome, "CREATE TABLE urls(id,url,title); CREATE TABLE visits(id,url,visit_time,visit_duration); INSERT INTO urls VALUES(1,'https://youtube.com/watch?v=abcdefghijk','Title'); INSERT INTO visits VALUES(1,1,11644473600000000,2500000),(2,1,11644473600000000,-1),(3,1,11644473601000000,NULL);");
    let first = r.read_since(&Cursor::default(), 2).unwrap();
    assert_eq!(first.visits.len(), 2);
    assert_eq!(first.visits[0].unix_us, 0);
    assert_eq!(first.visits[0].approx_duration_s, Some(2));
    assert_eq!(first.visits[1].approx_duration_s, Some(0));
    let second = r.read_since(&first.next, 2).unwrap();
    assert_eq!(second.visits.len(), 1);
    assert_eq!(second.visits[0].visit_id, 3);
    assert_eq!(second.visits[0].unix_us, 1_000_000);
    assert_eq!(second.visits[0].approx_duration_s, None);
    assert!(r.read_since(&second.next, 2).unwrap().visits.is_empty());
}
#[test]
fn chrome_duration_optional_and_reader_batch_bound() {
    let (_dir, mut r) = reader(Browser::Chrome, "CREATE TABLE urls(id,url,title); CREATE TABLE visits(id,url,visit_time); INSERT INTO urls VALUES(1,'https://youtube.com/watch?v=abcdefghijk','Title'); WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<1200) INSERT INTO visits SELECT x,1,11644473600000000 FROM n;");
    let mut cursor = Cursor::default();
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let batch = r.read_since(&cursor, 512).unwrap();
        assert!(batch.scanned <= 512);
        for row in batch.visits {
            assert!(seen.insert(row.visit_id));
            assert_eq!(row.approx_duration_s, None);
        }
        cursor = batch.next;
        if batch.exhausted {
            break;
        }
    }
    assert_eq!(seen.len(), 1200);
    assert!(r.read_since(&cursor, 0).is_err());
}
#[test]
fn firefox_prtime_join_and_cursor_reset() {
    let (_dir, mut r) = reader(Browser::Firefox, "CREATE TABLE moz_places(id,url,title); CREATE TABLE moz_historyvisits(id,place_id,visit_date); INSERT INTO moz_places VALUES(1,'https://youtube.com/watch?v=abcdefghijk','Firefox title'); INSERT INTO moz_historyvisits VALUES(1,1,-1),(2,1,1000001),(3,1,1000001);");
    let first = r.read_since(&Cursor::default(), 2).unwrap();
    assert_eq!(first.visits[0].unix_us.div_euclid(1_000_000), -1);
    assert_eq!(first.visits[1].unix_us.div_euclid(1_000_000), 1);
    assert_eq!(first.visits[1].title.as_deref(), Some("Firefox title"));
    assert_eq!(first.visits[1].approx_duration_s, None);
    let second = r.read_since(&first.next, 2).unwrap();
    assert_eq!(second.visits[0].visit_id, 3);
    let mut future = Cursor {
        last_native_us: 9_000_000,
        last_visit_id: 10,
        ..Default::default()
    };
    r.reset_if_replaced(&mut future).unwrap();
    assert_eq!(r.read_since(&future, 512).unwrap().visits.len(), 3);
}
