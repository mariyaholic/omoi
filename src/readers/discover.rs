//! Enumerate profile locations only; never read history contents here.
use super::{Browser, SourceId};
use crate::{config::Config, paths::PathEnvironment, Result};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub struct Profile {
    pub browser: Browser,
    pub name: String,
    pub path: PathBuf,
    pub source: SourceId,
}
#[derive(Debug, Default)]
pub struct Discovery {
    pub profiles: Vec<Profile>,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
    pub excluded: usize,
}

pub fn roots(env: &PathEnvironment) -> Result<Vec<(Browser, PathBuf)>> {
    let mut out = vec![];
    if env.windows {
        if let Some(local) = env.absolute("LOCALAPPDATA")? {
            for (b, p) in [
                (Browser::Chrome, "Google/Chrome/User Data"),
                (Browser::Edge, "Microsoft/Edge/User Data"),
                (Browser::Brave, "BraveSoftware/Brave-Browser/User Data"),
            ] {
                out.push((b, local.join(p)));
            }
        }
        if let Some(roaming) = env.absolute("APPDATA")? {
            out.push((Browser::Firefox, roaming.join("Mozilla/Firefox")));
        }
    } else {
        let home = env.absolute("HOME")?;
        if let Some(config) = env
            .absolute("XDG_CONFIG_HOME")?
            .or_else(|| home.as_ref().map(|p| p.join(".config")))
        {
            for (b, p) in [
                (Browser::Chrome, "google-chrome"),
                (Browser::Edge, "microsoft-edge"),
                (Browser::Brave, "BraveSoftware/Brave-Browser"),
            ] {
                out.push((b, config.join(p)));
            }
        }
        if let Some(home) = home {
            out.push((Browser::Firefox, home.join(".mozilla/firefox")));
        }
    }
    Ok(out)
}
fn children(root: &Path, d: &mut Discovery) -> Vec<PathBuf> {
    match std::fs::read_dir(root) {
        Ok(entries) => entries
            .filter_map(|entry| match entry {
                Ok(e) => match e.file_type() {
                    Ok(t) if t.is_dir() && !t.is_symlink() => Some(e.path()),
                    Ok(_) => None,
                    Err(_) => {
                        d.failures.push("profile metadata unreadable".into());
                        None
                    }
                },
                Err(_) => {
                    d.failures.push("profile directory entry unreadable".into());
                    None
                }
            })
            .collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
        Err(_) => {
            d.failures.push("profile root unreadable".into());
            vec![]
        }
    }
}
fn add(d: &mut Discovery, browser: Browser, name: String, path: PathBuf, explicit: bool) {
    match std::fs::metadata(&path) {
        Ok(m) if m.is_file() => match path.canonicalize() {
            Ok(path) => match SourceId::from_absolute_path(browser, &path) {
                Ok(source) => d.profiles.push(Profile {
                    browser,
                    name,
                    path,
                    source,
                }),
                Err(_) => d.failures.push("invalid profile identity".into()),
            },
            Err(_) => d.failures.push("profile identity unavailable".into()),
        },
        Err(e) if !explicit && e.kind() == std::io::ErrorKind::NotFound => (),
        _ => d.failures.push("profile database unavailable".into()),
    }
}
fn firefox(root: &Path, d: &mut Discovery) {
    let ini = root.join("profiles.ini");
    let read = (|| -> std::io::Result<String> {
        use std::io::Read;
        let mut text = String::new();
        std::fs::File::open(ini)?
            .take(65537)
            .read_to_string(&mut text)?;
        if text.len() > 65536 {
            return Err(std::io::Error::other("profile index too large"));
        }
        Ok(text)
    })();
    match read {
        Ok(text) => {
            let (mut active, mut name, mut path, mut relative) =
                (false, String::new(), String::new(), true);
            let flush = |d: &mut Discovery,
                         active: bool,
                         name: &str,
                         path: &str,
                         relative: bool| {
                if active && !path.is_empty() {
                    let dir = if relative {
                        root.join(path)
                    } else {
                        PathBuf::from(path)
                    };
                    if dir.is_absolute() {
                        if matches!(std::fs::metadata(dir.join("places.sqlite")), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
                        {
                            d.warnings
                                .push("Firefox profile has no history database; skipped".into());
                            return;
                        }
                        add(
                            d,
                            Browser::Firefox,
                            if name.is_empty() {
                                basename(&dir)
                            } else {
                                name.to_owned()
                            },
                            dir.join("places.sqlite"),
                            true,
                        );
                    } else {
                        d.failures.push("invalid Firefox profile path".into());
                    }
                }
            };
            for line in text.lines().chain(std::iter::once("[end]")) {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    flush(d, active, &name, &path, relative);
                    active = line.starts_with("[Profile");
                    name.clear();
                    path.clear();
                    relative = true;
                } else if active {
                    if let Some((k, v)) = line.split_once('=') {
                        match k.trim() {
                            "Name" => name = v.trim().into(),
                            "Path" => path = v.trim().into(),
                            "IsRelative" => relative = v.trim() != "0",
                            _ => (),
                        }
                    }
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => d.failures.push("Firefox profile index unreadable".into()),
    }
    for parent in [root.to_path_buf(), root.join("Profiles")] {
        for dir in children(&parent, d) {
            add(
                d,
                Browser::Firefox,
                basename(&dir),
                dir.join("places.sqlite"),
                false,
            );
        }
    }
}
fn basename(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
pub fn discover(config: &Config, env: &PathEnvironment) -> Result<Discovery> {
    let mut d = Discovery::default();
    for (browser, root) in roots(env)? {
        if !config.browsers.contains(&browser) {
            continue;
        }
        if browser == Browser::Firefox {
            firefox(&root, &mut d);
        } else {
            for dir in children(&root, &mut d) {
                let name = basename(&dir);
                if !["System Profile", "Guest Profile"].contains(&name.as_str()) {
                    add(&mut d, browser, name, dir.join("History"), false);
                }
            }
        }
    }
    for raw in &config.profile_paths {
        let (b, path) = raw
            .split_once(':')
            .ok_or_else(|| crate::Error::Config("invalid profile selector".into()))?;
        if let Some(browser) = config.browsers.iter().find(|b2| b2.name() == b) {
            let path = PathBuf::from(path);
            let name = path.parent().map(basename).unwrap_or_default();
            add(&mut d, *browser, name, path, true);
        }
    }
    let mut seen = BTreeSet::new();
    d.profiles.retain(|p| seen.insert(p.source.0.clone()));
    let mut matched = vec![false; config.profile_excludes.len()];
    d.profiles.retain(|p| {
        let mut exclude = false;
        for (i, selector) in config.profile_excludes.iter().enumerate() {
            if let Some((b, n)) = selector.split_once(':') {
                if b == p.browser.name()
                    && (n.eq_ignore_ascii_case(&p.name)
                        || p.path
                            .parent()
                            .is_some_and(|dir| n.eq_ignore_ascii_case(&basename(dir))))
                {
                    matched[i] = true;
                    exclude = true;
                }
            }
        }
        if exclude {
            d.excluded += 1;
        }
        !exclude
    });
    for found in matched {
        if !found {
            d.warnings.push("profile exclusion matched nothing".into());
        }
    }
    d.profiles.sort_by(|a, b| a.source.0.cmp(&b.source.0));
    Ok(d)
}
