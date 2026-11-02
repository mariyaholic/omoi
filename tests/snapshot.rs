use omoi::readers::snapshot::{self, TempDir};
#[test]
fn snapshot_retry_bound() {
    let d = TempDir::create(&std::env::temp_dir()).unwrap();
    let source = d.0.join("History");
    std::fs::write(&source, b"a").unwrap();
    let root = d.0.join("copies");
    let mut attempts = 0;
    let result = snapshot::copy_observed(&source, &root, 3, || {
        attempts += 1;
        std::fs::write(&source, vec![b'x'; attempts + 1]).unwrap();
    });
    assert!(result.is_err());
    assert_eq!(attempts, 3);
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
}
#[test]
fn snapshot_preserves_sidecars_and_cleans_up() {
    let d = TempDir::create(&std::env::temp_dir()).unwrap();
    let source = d.0.join("History");
    std::fs::write(&source, b"main").unwrap();
    std::fs::write(snapshot::sidecar(&source, "-wal"), b"wal").unwrap();
    std::fs::write(snapshot::sidecar(&source, "-journal"), b"journal").unwrap();
    let s = snapshot::copy(&source, &d.0.join("copies"), 1).unwrap();
    assert_eq!(
        std::fs::read(snapshot::sidecar(&s.path, "-wal")).unwrap(),
        b"wal"
    );
    assert_eq!(
        std::fs::read(snapshot::sidecar(&s.path, "-journal")).unwrap(),
        b"journal"
    );
    let path = s.dir.0.clone();
    drop(s);
    assert!(!path.exists());
}
#[test]
fn wal_only_change_fingerprint() {
    let d = TempDir::create(&std::env::temp_dir()).unwrap();
    let p = d.0.join("History");
    std::fs::write(&p, b"main").unwrap();
    let before = snapshot::fingerprint(&p).unwrap();
    std::fs::write(snapshot::sidecar(&p, "-wal"), b"wal").unwrap();
    let after = snapshot::fingerprint(&p).unwrap();
    assert_eq!(before.main, after.main);
    assert_ne!(before, after);
}
