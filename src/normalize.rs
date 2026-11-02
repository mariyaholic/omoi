use serde::{Deserialize, Serialize};
use url::Url;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemKey {
    pub platform: String,
    pub external_id: String,
}
pub struct Normalized {
    pub key: ItemKey,
    pub kind: Option<&'static str>,
}
fn positive(s: &str) -> Option<String> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let s = s.trim_start_matches('0');
    if s.is_empty() {
        None
    } else {
        Some(s.into())
    }
}
fn locale(s: &str) -> bool {
    (s.len() == 2 && s.bytes().all(|b| b.is_ascii_lowercase()))
        || (s.len() == 5
            && s.as_bytes()[2] == b'-'
            && s.bytes()
                .enumerate()
                .all(|(i, b)| i == 2 || b.is_ascii_lowercase()))
}
pub fn normalize(raw: &str, unix_s: i64, offset: chrono::FixedOffset) -> Option<Normalized> {
    let bytes = raw.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'%'
            && (i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit())
        {
            return None;
        }
    }
    let u = Url::parse(raw).ok()?;
    if !u.username().is_empty() || u.password().is_some() || u.port().is_some() {
        return None;
    }
    let path = u.path();
    let lower = path.to_ascii_lowercase();
    if ["%2f", "%5c", "%00"].iter().any(|s| lower.contains(s)) {
        return None;
    }
    let host = u.host_str().unwrap_or("");
    let parts: Vec<_> = path
        .strip_suffix('/')
        .unwrap_or(path)
        .trim_start_matches('/')
        .split('/')
        .collect();
    let query: Vec<_> = u.query_pairs().collect();
    for key in ["v", "p", "gti"] {
        if query.iter().filter(|(k, _)| k == key).count() > 1 {
            return None;
        }
    }
    let q = |key: &str| {
        query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_ref())
    };
    let platform;
    let id: String;
    let mut kind = "video";
    if u.scheme() == "file" {
        if !host.is_empty() && host != "localhost" {
            return None;
        }
        let mut local = u.clone();
        local.set_query(None);
        local.set_fragment(None);
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        if ![
            "mp4", "mkv", "webm", "avi", "mov", "m4v", "mp3", "flac", "m4a", "ogg", "wav",
        ]
        .contains(&ext.as_str())
        {
            return None;
        }
        let mut path = local.path().to_owned();
        if path.len() > 3 && path.as_bytes()[2] == b':' && path.as_bytes()[1].is_ascii_alphabetic()
        {
            path.replace_range(1..2, &path[1..2].to_ascii_uppercase());
        }
        platform = "local";
        id = format!("file:{path}");
    } else {
        if !["http", "https"].contains(&u.scheme()) {
            return None;
        }
        match host {
            "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com"
            | "youtu.be" | "www.youtu.be" => {
                id = match parts.as_slice() {
                    [one] if host.ends_with("youtu.be") => (*one).into(),
                    ["watch"] => q("v")?.into(),
                    [route, video] if ["shorts", "live", "embed"].contains(route) => {
                        kind = match *route {
                            "shorts" => "short",
                            "live" => "live",
                            _ => "video",
                        };
                        (*video).into()
                    }
                    _ => return None,
                };
                if id.len() != 11
                    || !id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                {
                    return None;
                }
                platform = "youtube";
            }
            "bilibili.com" | "www.bilibili.com" | "m.bilibili.com" => {
                if let ["video", video] = parts.as_slice() {
                    let base = if video.starts_with("BV")
                        && video.len() == 12
                        && video[2..].bytes().all(|b| b.is_ascii_alphanumeric())
                    {
                        (*video).into()
                    } else {
                        let n = video.strip_prefix("av")?;
                        format!("av{}", positive(n)?)
                    };
                    let p = positive(q("p").unwrap_or("1"))?;
                    if p.parse::<u32>().ok()? > 10000 {
                        return None;
                    }
                    id = format!("{base}:p{p}");
                    platform = "bilibili";
                } else {
                    return None;
                }
            }
            "netflix.com" | "www.netflix.com" => {
                if let ["watch", n] = parts.as_slice() {
                    id = positive(n)?;
                    platform = "netflix";
                } else {
                    return None;
                }
            }
            "twitch.tv" | "www.twitch.tv" | "m.twitch.tv" => {
                platform = "twitch";
                match parts.as_slice() {
                    ["videos", n] => id = format!("vod:{}", positive(n)?),
                    [channel] => {
                        let c = channel.to_ascii_lowercase();
                        if c.is_empty()
                            || c.len() > 25
                            || !c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                            || [
                                "directory",
                                "downloads",
                                "settings",
                                "subscriptions",
                                "inventory",
                                "wallet",
                                "search",
                                "login",
                                "signup",
                                "prime",
                                "turbo",
                                "jobs",
                                "p",
                                "products",
                                "videos",
                                "clip",
                                "clips",
                            ]
                            .contains(&c.as_str())
                        {
                            return None;
                        }
                        let date = chrono::DateTime::from_timestamp(unix_s, 0)?
                            .with_timezone(&offset)
                            .format("%Y-%m-%d");
                        id = format!("live:{c}:{date}");
                        kind = "live";
                    }
                    _ => return None,
                }
            }
            "disneyplus.com" | "www.disneyplus.com" => {
                let s = if parts.first().is_some_and(|s| s.len() == 5 && locale(s)) {
                    &parts[1..]
                } else {
                    &parts[..]
                };
                if let [route, token] = s {
                    if !["video", "play"].contains(route)
                        || !(8..=80).contains(&token.len())
                        || !token
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    {
                        return None;
                    }
                    let chunks: Vec<_> = token.split('-').collect();
                    let uuid = chunks.iter().map(|s| s.len()).eq([8, 4, 4, 4, 12])
                        && chunks
                            .iter()
                            .all(|s| s.bytes().all(|b| b.is_ascii_hexdigit()));
                    id = if uuid {
                        token.to_ascii_lowercase()
                    } else {
                        (*token).into()
                    };
                    platform = "disney";
                } else {
                    return None;
                }
            }
            "crunchyroll.com" | "www.crunchyroll.com" => {
                let s = if parts.first().is_some_and(|s| locale(s)) {
                    &parts[1..]
                } else {
                    &parts[..]
                };
                if !(s.len() == 2 || s.len() == 3)
                    || s[0] != "watch"
                    || s[1].len() != 9
                    || !s[1].bytes().all(|b| b.is_ascii_alphanumeric())
                {
                    return None;
                }
                id = s[1].to_ascii_uppercase();
                platform = "crunchyroll";
            }
            _ => {
                let h = host.strip_prefix("www.").unwrap_or(host);
                let prime = h == "primevideo.com";
                let amazon = [
                    "amazon.com",
                    "amazon.co.uk",
                    "amazon.de",
                    "amazon.co.jp",
                    "amazon.in",
                    "amazon.fr",
                    "amazon.it",
                    "amazon.es",
                    "amazon.ca",
                    "amazon.com.au",
                    "amazon.com.br",
                    "amazon.com.mx",
                ]
                .contains(&h);
                if !prime && !amazon {
                    return None;
                }
                let token = match parts.as_slice() {
                    ["detail", n] | ["detail", _, n] if prime => *n,
                    ["gp", "video", "detail", n] => *n,
                    ["gp", "video", "detail", _, n] if amazon => *n,
                    ["dp", n] | ["gp", "aw", "d", n] if amazon && q("video") == Some("1") => *n,
                    _ => return None,
                };
                if !(10..=32).contains(&token.len())
                    || !token.bytes().all(|b| b.is_ascii_alphanumeric())
                {
                    return None;
                }
                platform = "prime";
                id = token.to_ascii_uppercase();
            }
        }
    }
    if id.len() > 512 {
        return None;
    }
    Some(Normalized {
        key: ItemKey {
            platform: platform.into(),
            external_id: id,
        },
        kind: Some(kind),
    })
}
