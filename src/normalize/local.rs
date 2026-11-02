use url::Url;
pub(super) fn identity(raw: &str, url: &Url) -> Option<String> {
    let rest = raw.split_once(':')?.1;
    if rest.starts_with("////") {
        return None;
    }
    if !(rest.starts_with("///") || rest.to_ascii_lowercase().starts_with("//localhost/")) {
        return None;
    }
    let mut decoded = Vec::with_capacity(url.path().len());
    let mut bytes = url.path().bytes();
    while let Some(b) = bytes.next() {
        decoded.push(if b == b'%' {
            let hi = (bytes.next()? as char).to_digit(16)?;
            let lo = (bytes.next()? as char).to_digit(16)?;
            (hi * 16 + lo) as u8
        } else {
            b
        });
    }
    let decoded = String::from_utf8(decoded).ok()?;
    if !decoded.starts_with('/')
        || decoded.starts_with("//")
        || decoded.chars().any(|c| c.is_control() || c == '\\')
    {
        return None;
    }
    let mut segments = Vec::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => (),
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    let mut path = format!("/{}", segments.join("/"));
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    if ![
        "mp4", "mkv", "webm", "avi", "mov", "m4v", "mp3", "flac", "m4a", "ogg", "wav",
    ]
    .contains(&ext.as_str())
    {
        return None;
    }
    if path.len() > 3 && path.as_bytes()[2] == b':' && path.as_bytes()[1].is_ascii_alphabetic() {
        path.replace_range(1..2, &path[1..2].to_ascii_uppercase());
    }
    let mut canonical = Url::parse("file:///").ok()?;
    canonical.set_path(&path.replace('%', "%25"));
    Some(format!("file:{}", canonical.path()))
}
