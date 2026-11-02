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
