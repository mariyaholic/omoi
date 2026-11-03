use omoi::native::{self, Registration};
use std::io::Cursor;
#[test]
fn frames_reject_truncation_oversize_and_invalid_json() {
    for bytes in [
        vec![1],
        vec![5, 0, 0, 0, b'{'],
        vec![0, 0, 0, 0],
        vec![1, 0, 1, 0],
        vec![1, 0, 0, 0, b'x'],
    ] {
        assert!(native::read_frame(&mut Cursor::new(bytes)).is_err());
    }
    assert!(native::read_frame(&mut Cursor::new(vec![]))
        .unwrap()
        .is_none());
    let v = serde_json::json!({"type":"status"});
    let mut bytes = vec![];
    native::write_frame(&mut bytes, &v).unwrap();
    assert_eq!(
        native::read_frame(&mut Cursor::new(bytes)).unwrap(),
        Some(v)
    );
}
#[test]
fn origins_are_exact_and_no_wildcards() {
    let origin = format!("chrome-extension://{}/", "a".repeat(32));
    let r = Registration {
        home: std::env::temp_dir(),
        allowed_origins: vec![origin.clone()],
    };
    assert!(r.permits(&origin));
    for s in [
        "*",
        "https://example.com",
        &format!("{origin}extra"),
        &origin.replace('a', "b"),
    ] {
        assert!(!r.permits(s));
    }
    assert!(!native::valid_origin("chrome-extension://*/"));
}
