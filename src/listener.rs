//! Explicit, append-only loopback telemetry receiver. No catalogue read API.
use crate::{config::Config, paths::Paths, Error, Result};
use std::{io::Write, net::TcpListener, path::Path};
fn io(source: std::io::Error) -> Error {
    Error::Io {
        context: "playback listener",
        source,
    }
}
fn token(home: &Path) -> Result<String> {
    let dir = home.join("listener");
    std::fs::create_dir_all(&dir).map_err(io)?;
    #[cfg(windows)]
    {
        let temp = crate::readers::snapshot::TempDir::create(&std::env::temp_dir())?;
        let script = temp.0.join("protect.ps1");
        std::fs::write(&script, include_str!("listener/protect.ps1")).map_err(io)?;
        let result = crate::scheduler::powershell()?
            .arg("-File")
            .arg(script)
            .arg(&dir)
            .output()
            .map_err(io)?;
        if !result.status.success() {
            return Err(Error::Config(
                "Cannot protect listener token directory".into(),
            ));
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    let file = dir.join("token");
    if !file.exists() {
        let value = crate::ui::random_token()?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&file).map_err(io)?;
        f.write_all(value.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(io)?;
    }
    let value = std::fs::read_to_string(file).map_err(io)?;
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Config("Invalid listener token file".into()));
    }
    Ok(value)
}
pub fn authorized(
    host: &str,
    origin: Option<&str>,
    supplied: &str,
    expected: &str,
    token: &str,
) -> bool {
    origin.is_none() && crate::ui::authorized(host, None, supplied, expected, token)
}
pub fn serve(paths: Paths) -> Result<()> {
    let config = Config::load(&paths.config)?;
    let listener =
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, config.listener_port)).map_err(io)?;
    let host = listener.local_addr().map_err(io)?.to_string();
    std::fs::create_dir_all(&paths.home).map_err(io)?;
    let token = token(&paths.home)?;
    let mut c = crate::db::open(&paths.db, &config, true)?;
    // The append-only listener has a smaller resident budget than the collector.
    // A full inbox must not fill the collector's 2 MiB page cache here.
    c.pragma_update(None, "cache_size", -256)?;
    crate::db::migrate(&mut c)?;
    // Successful acknowledgements survive a power failure, not just a process restart.
    c.pragma_update(None, "synchronous", "FULL")?;
    println!("Playback listener: http://{host}");
    std::io::stdout().flush().map_err(io)?;
    for stream in listener.incoming() {
        let mut stream = stream.map_err(io)?;
        let result = (|| -> Result<serde_json::Value> {
            let r = crate::ui::request(&mut stream, 8192, 16384).map_err(io)?;
            if r.method != "POST"
                || r.target != "/event"
                || r.headers.get("content-type").map(String::as_str) != Some("application/json")
                || !authorized(
                    r.headers
                        .get("host")
                        .map(String::as_str)
                        .unwrap_or_default(),
                    r.headers.get("origin").map(String::as_str),
                    r.headers
                        .get("x-omoi-token")
                        .map(String::as_str)
                        .unwrap_or_default(),
                    &host,
                    &token,
                )
            {
                return Err(Error::Config("Forbidden".into()));
            }
            let body =
                std::str::from_utf8(&r.body).map_err(|_| Error::Config("Invalid UTF-8".into()))?;
            Ok(
                serde_json::json!({"accepted":crate::sessionize::enqueue(&mut c,body,config.offset_seconds()?)?}),
            )
        })();
        let (status, value) = match result {
            Ok(v) => (200, v),
            Err(_) => (403, serde_json::json!({"error":"Playback event rejected"})),
        };
        let _ = crate::ui::json_response(&mut stream, status, value);
    }
    Ok(())
}
