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
