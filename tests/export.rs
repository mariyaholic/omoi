use omoi::{db, export, Error};
use rusqlite::Connection;
use std::io::{self, Write};
fn catalogue() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    db::migrate(&mut c).unwrap();
    c.execute(
        "INSERT INTO items(platform,external_id,title) VALUES('youtube','abcdefghijk',?1)",
        ["A, \"quoted\"\nname"],
    )
    .unwrap();
    c.execute_batch(
        "INSERT INTO views(item_id,started_at,source) VALUES(1,20,'chrome'),(1,10,'firefox')",
    )
    .unwrap();
    c
}
#[test]
fn csv_escaping() {
    let c = catalogue();
    let mut bytes = vec![];
    export::export(&c, false, &mut bytes).unwrap();
    let rows = csv::Reader::from_reader(&bytes[..])
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(&rows[0][4], "A, \"quoted\"\nname");
    assert_eq!(&rows[0][7], "10");
    assert_eq!(&rows[1][7], "20");
}
#[test]
fn json_null_and_order() {
    let c = catalogue();
    let mut bytes = vec![];
    export::export(&c, true, &mut bytes).unwrap();
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["started_at"], 10);
    assert_eq!(rows[1]["started_at"], 20);
    assert!(rows[0]["approx_duration_s"].is_null());
}
struct Broken;
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn export_broken_pipe() {
    let c = catalogue();
    // Exceed CSV's buffer so the failure occurs during serialize, not just final flush.
    c.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<1000) INSERT INTO views(item_id,started_at,source) SELECT 1,x+100,'chrome' FROM n").unwrap();
    for json in [false, true] {
        assert!(
            matches!(export::export(&c, json, &mut Broken), Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::BrokenPipe)
        );
    }
}

#[test]
fn json_export_batches_writes() {
    #[derive(Default)]
    struct Sink {
        writes: usize,
        bytes: usize,
    }
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes += 1;
            self.bytes += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let c = catalogue();
    c.execute_batch("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<1000) INSERT INTO views(item_id,started_at,source) SELECT 1,x+100,'chrome' FROM n").unwrap();
    let mut sink = Sink::default();
    export::export(&c, true, &mut sink).unwrap();
    assert!(sink.bytes > 100000);
    assert!(
        sink.writes < 1000,
        "JSON export made {} underlying writes for 1002 rows",
        sink.writes
    );
}
