use omoi::{
    config::Config,
    paths::PathEnvironment,
    readers::{discover::discover, Browser},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "omoi-discover-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, p: &str) -> PathBuf {
        let p = self.0.join(p);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, []).unwrap();
        p
    }
    fn env(&self) -> PathEnvironment {
        let mut e = PathEnvironment {
            windows: false,
            vars: Default::default(),
        };
        e.vars
            .insert("HOME".into(), self.0.clone().into_os_string());
        e
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn chromium_all_profiles() {
    let f = Fixture::new();
    for n in [
        "Default",
        "Profile 2",
        "custom",
        "System Profile",
        "Guest Profile",
    ] {
        f.file(&format!(".config/google-chrome/{n}/History"));
    }
    let d = discover(&Config::default(), &f.env()).unwrap();
    assert_eq!(d.profiles.len(), 3);
}
#[test]
fn firefox_ini_paths() {
    let f = Fixture::new();
    let a = f.file(".mozilla/firefox/Profiles/a/places.sqlite");
    let b = f.file("outside/places.sqlite");
    let ini = f.file(".mozilla/firefox/profiles.ini");
    std::fs::write(ini,format!("[Profile0]\nName=work\nIsRelative=1\nPath=Profiles/a\n[Profile1]\nName=other\nIsRelative=0\nPath={}\n",b.parent().unwrap().display())).unwrap();
    let d = discover(&Config::default(), &f.env()).unwrap();
    assert_eq!(d.profiles.len(), 2);
    assert!(d
        .profiles
        .iter()
        .any(|p| p.path == a.canonicalize().unwrap() && p.name == "work"));
}

#[test]
fn firefox_uninitialized_profile_is_reported_as_skipped() {
    let f = Fixture::new();
    let ini = f.file(".mozilla/firefox/profiles.ini");
    std::fs::create_dir_all(f.0.join(".mozilla/firefox/Profiles/empty")).unwrap();
    f.file(".mozilla/firefox/Profiles/active/places.sqlite");
    std::fs::write(ini,"[Profile0]\nName=empty\nIsRelative=1\nPath=Profiles/empty\n[Profile1]\nName=active\nIsRelative=1\nPath=Profiles/active\n").unwrap();
    let d = discover(&Config::default(), &f.env()).unwrap();
    assert_eq!(d.profiles.len(), 1);
    assert!(d.failures.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("no history database")));
}
#[test]
fn discovery_missing_and_denied() {
    let f = Fixture::new();
    assert!(discover(&Config::default(), &f.env())
        .unwrap()
        .profiles
        .is_empty());
    let mut c = Config::default();
    c.profile_paths.push(format!(
        "firefox:{}",
        f.0.join("missing/places.sqlite").display()
    ));
    let d = discover(&c, &f.env()).unwrap();
    assert_eq!(d.failures.len(), 1);
}
#[test]
fn explicit_profile_filter() {
    let f = Fixture::new();
    let p = f.file("outside/places.sqlite");
    let c = Config {
        browsers: vec![Browser::Chrome],
        profile_paths: vec![format!("firefox:{}", p.display())],
        ..Config::default()
    };
    assert!(discover(&c, &f.env()).unwrap().profiles.is_empty());
}
#[test]
fn profile_excludes_applied() {
    let f = Fixture::new();
    f.file(".config/google-chrome/Profile 2/History");
    let c = Config {
        profile_excludes: vec!["chrome:profile 2".into(), "firefox:absent".into()],
        ..Config::default()
    };
    let d = discover(&c, &f.env()).unwrap();
    assert!(d.profiles.is_empty());
    assert_eq!(d.excluded, 1);
    assert_eq!(d.warnings.len(), 1);
}
