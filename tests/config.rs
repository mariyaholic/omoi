use omoi::{
    config::Config,
    paths::{PathEnvironment, Paths},
};
use std::ffi::OsString;

#[test]
fn home_override_precedes_os() {
    for windows in [true, false] {
        let root = std::env::temp_dir().join("omoi-config-fixture");
        let mut env = PathEnvironment {
            windows,
            vars: Default::default(),
        };
        env.vars
            .insert("OMOI_HOME".into(), root.clone().into_os_string());
        assert_eq!(Paths::resolve(&env).unwrap().home, root);
    }
}

#[test]
fn linux_xdg_fallback() {
    let mut env = PathEnvironment {
        windows: false,
        vars: Default::default(),
    };
    let root = std::env::temp_dir();
    env.vars
        .insert("HOME".into(), root.clone().into_os_string());
    assert_eq!(
        Paths::resolve(&env).unwrap().home,
        root.join(".local/share/omoi")
    );
    env.vars
        .insert("XDG_DATA_HOME".into(), OsString::from("relative"));
    assert!(Paths::resolve(&env).is_err());
}

#[test]
fn config_defaults_and_errors() {
    let c = Config::parse("").unwrap();
    assert_eq!(c.reader_batch_rows, 512);
    assert_eq!(c.local_utc_offset, "+08:00");
    for text in [
        "unknown = 3",
        "log_files=2\nlog_files=3",
        "[table]",
        "busy_timeout_ms=0",
        "reader_batch_rows=2049",
        "local_utc_offset=\"+14:01\"",
        "listener_port=65536",
        "browsers=[\"unknown\"]",
        "log_files=1.0",
        "log_files=1e0",
    ] {
        assert!(Config::parse(text).is_err(), "{text}");
    }
    let mut env = PathEnvironment {
        windows: true,
        vars: Default::default(),
    };
    env.vars.insert("OMOI_HOME".into(), "relative".into());
    assert!(Paths::resolve(&env).is_err());
    let absent = std::env::temp_dir()
        .join(format!("omoi-absent-config-{}", std::process::id()))
        .join("config.toml");
    assert_eq!(Config::load(&absent).unwrap().reader_batch_rows, 512);
}

#[test]
fn config_comments_in_strings() {
    let c = Config::parse(
        "profile_excludes=[\"chrome:work#1\"] # comment\nlocal_utc_offset=\"-03:30\"",
    )
    .unwrap();
    assert_eq!(c.profile_excludes, ["chrome:work#1"]);
    assert_eq!(c.offset_seconds().unwrap(), -12600);
    assert!(Config::parse(&" ".repeat(65537)).is_err());
}
