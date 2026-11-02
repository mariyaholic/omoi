use crate::{Error, Result};
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf};

/// Injectable environment; tests never mutate the process environment.
pub struct PathEnvironment {
    pub windows: bool,
    pub vars: BTreeMap<String, OsString>,
}
impl PathEnvironment {
    pub fn capture() -> Self {
        let vars = [
            "OMOI_HOME",
            "LOCALAPPDATA",
            "APPDATA",
            "HOME",
            "XDG_DATA_HOME",
            "XDG_CONFIG_HOME",
        ]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|v| (key.to_owned(), v)))
        .collect();
        Self {
            windows: cfg!(windows),
            vars,
        }
    }
    pub fn absolute(&self, key: &str) -> Result<Option<PathBuf>> {
        match self.vars.get(key).filter(|v| !v.is_empty()) {
            Some(v) => {
                let path = PathBuf::from(v);
                if !path.is_absolute() {
                    return Err(Error::Config(format!("{key} must be absolute")));
                }
                Ok(Some(path))
            }
            None => Ok(None),
        }
    }
    fn required(&self, key: &str) -> Result<PathBuf> {
        self.absolute(key)?
            .ok_or_else(|| Error::Config(format!("missing {key}")))
    }
}

#[derive(Debug)]
pub struct Paths {
    pub home: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub logs: PathBuf,
    pub tmp: PathBuf,
    pub thumbs: PathBuf,
}
impl Paths {
    pub fn resolve(env: &PathEnvironment) -> Result<Self> {
        let home = match env.absolute("OMOI_HOME")? {
            Some(home) => home,
            None if env.windows => env.required("LOCALAPPDATA")?.join("omoi"),
            None => match env.absolute("XDG_DATA_HOME")? {
                Some(root) => root.join("omoi"),
                None => env.required("HOME")?.join(".local/share/omoi"),
            },
        };
        Ok(Self {
            config: home.join("config.toml"),
            db: home.join("history.sqlite"),
            logs: home.join("logs"),
            tmp: home.join("tmp"),
            thumbs: home.join("thumbs"),
            home,
        })
    }
}
