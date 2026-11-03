use omoi::{readers::*, Error};

#[test]
fn cursor_json_roundtrip() {
    let stamp = FileStamp {
        present: true,
        len: 17,
        mtime_ns: -123456789012345,
    };
    let cursor = Cursor {
        version: 1,
        last_native_us: 100,
        last_visit_id: 7,
        fingerprint: Some(Fingerprint {
            main: stamp.clone(),
            wal: FileStamp {
                len: 18,
                ..stamp.clone()
            },
            journal: FileStamp {
                present: false,
                ..stamp
            },
        }),
    };
    let json = serde_json::to_string(&cursor).unwrap();
    assert_eq!(serde_json::from_str::<Cursor>(&json).unwrap(), cursor);
    assert!(
        serde_json::from_str::<Cursor>(&json.replace("\"version\":1", "\"version\":2")).is_err()
    );
    assert_eq!(Cursor::default().last_native_us, i64::MIN);
}

#[test]
fn error_exit_mapping() {
    assert_eq!(Error::Config("bad".into()).exit_code(), 2);
    assert_eq!(Error::Busy.exit_code(), 4);
    assert_eq!(Error::Unsupported("platform").exit_code(), 5);
    assert_eq!(Error::Db(rusqlite::Error::InvalidQuery).exit_code(), 1);
}

#[test]
fn source_id_distinguishes_profiles() {
    let root = std::env::temp_dir();
    let a = root.join("omoi-fixture-a").join("Default");
    let b = root.join("omoi-fixture-b").join("Default");
    let id = SourceId::from_absolute_path(Browser::Chrome, &a).unwrap();
    assert_ne!(
        id,
        SourceId::from_absolute_path(Browser::Chrome, &b).unwrap()
    );
    assert_eq!(
        id,
        SourceId::from_absolute_path(Browser::Chrome, &a).unwrap()
    );
    assert_ne!(id, SourceId::from_absolute_path(Browser::Edge, &a).unwrap());
    assert!(
        SourceId::from_absolute_path(Browser::Chrome, std::path::Path::new("relative")).is_err()
    );
}
