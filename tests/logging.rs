use omoi::{collect::CollectReport, config::Config, logging, readers::snapshot::TempDir};
#[test]
fn logs_global_cap_and_redaction() {
    let dir = TempDir::create(&std::env::temp_dir()).unwrap();
    let config = Config {
        log_file_bytes: 512,
        log_files: 2,
        ..Config::default()
    };
    let report = CollectReport {
        warnings: vec!["https://private.example/watch?token=secret".into()],
        ..Default::default()
    };
    for _ in 0..50 {
        logging::record(&dir.0, &config, Some(&report), 0).unwrap();
    }
    let files: Vec<_> = std::fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "log"))
        .collect();
    assert!(files.len() <= 2);
    let mut size = 0;
    for path in files {
        let text = std::fs::read_to_string(path).unwrap();
        size += text.len();
        assert!(!text.contains("secret"));
        for line in text.lines() {
            assert!(line.len() < 4096);
            serde_json::from_str::<serde_json::Value>(line).unwrap();
        }
    }
    assert!(size <= 1024);
}
#[test]
fn log_lock_fallback() {
    let dir = TempDir::create(&std::env::temp_dir()).unwrap();
    let lock = dir.0.join("logging.lock");
    std::fs::write(&lock, b"occupied").unwrap();
    let owner = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    owner.try_lock().unwrap();
    assert!(logging::record(&dir.0, &Config::default(), None, 1).is_err());
    drop(owner);
    assert_eq!(std::fs::read(lock).unwrap(), b"occupied");
}
