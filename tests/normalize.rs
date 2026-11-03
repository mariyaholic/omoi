#[test]
fn normalizer_url_vectors() {
    let doc = include_str!("../docs/omoi-architecture-detail.md");
    let table = doc
        .split("| Input URL | Expected")
        .nth(1)
        .unwrap()
        .split("Boundary vectors")
        .next()
        .unwrap();
    let time = chrono::DateTime::parse_from_rfc3339("2026-10-24T16:30:00Z")
        .unwrap()
        .timestamp();
    let offset = chrono::FixedOffset::east_opt(28800).unwrap();
    let mut count = 0;
    for line in table.lines().filter(|l| l.starts_with("| `")) {
        let cols: Vec<_> = line.split('|').collect();
        let url = cols[1].trim().trim_matches('`');
        let expected = cols[2].trim();
        let got = omoi::normalize::normalize(url, time, offset);
        if expected == "—" {
            assert!(got.is_none(), "{url}");
        } else {
            let (p, id) = expected.split_once(", ").unwrap();
            let got = got.unwrap_or_else(|| panic!("{url}"));
            assert_eq!(
                (got.key.platform.as_str(), got.key.external_id.as_str()),
                (p, id),
                "{url}"
            );
        }
        count += 1;
    }
    assert!(count > 45);
}
#[test]
fn local_path_case() {
    let offset = chrono::FixedOffset::east_opt(28800).unwrap();
    let key = |s| omoi::normalize::normalize(s, 0, offset).map(|v| v.key.external_id);
    assert_eq!(
        key("file:///c:/Video/%41%20Film.m%70%34"),
        Some("file:/C:/Video/A%20Film.mp4".into())
    );
    assert_eq!(
        key("file:///C:/Video/%e6%98%9f.mp4"),
        key("file:///C:/Video/星.mp4")
    );
    assert_ne!(key("file:///C:/Video/A.mp4"), key("file:///C:/Video/a.mp4"));
    assert!(key("file:///C:/Video/%ff.mp4").is_none());
    assert!(key("file:relative.mp4").is_none());
    assert!(key("file:////server/movie.mp4").is_none());
    assert_eq!(
        key("file:///C:/Video/%2520.mp4"),
        Some("file:/C:/Video/%2520.mp4".into())
    );
}

#[test]
fn twitch_local_midnight() {
    let offset = chrono::FixedOffset::east_opt(28800).unwrap();
    for (at, expected) in [
        ("2026-10-24T15:59:59Z", "live:maria:2026-10-24"),
        ("2026-10-24T16:00:00Z", "live:maria:2026-10-25"),
    ] {
        let time = chrono::DateTime::parse_from_rfc3339(at)
            .unwrap()
            .timestamp();
        let value =
            omoi::normalize::normalize("https://www.twitch.tv/maria", time, offset).unwrap();
        assert_eq!(value.key.external_id, expected);
    }
}

#[test]
fn normalizer_identity_params() {
    let offset = chrono::FixedOffset::east_opt(28800).unwrap();
    let key = |url: &str| omoi::normalize::normalize(url, 0, offset).map(|v| v.key.external_id);
    let base = "https://www.bilibili.com/video/BV1xx411c7mD";
    assert_eq!(key(base), key(&format!("{base}?p=1")));
    assert_ne!(key(base), key(&format!("{base}?p=2")));
    for url in [
        "https://youtube.com/watch?v=abcdefghijk&v=12345678901",
        "https://youtube.com/watch?v=abcdefghijk&%76=abcdefghijk",
        "https://www.bilibili.com/video/BV1xx411c7mD?p=1&p=2",
    ] {
        assert!(key(url).is_none(), "{url}");
    }
}
