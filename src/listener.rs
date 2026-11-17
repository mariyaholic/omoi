//! Explicit, append-only loopback telemetry receiver. No catalogue read API.
use crate::{config::Config, paths::Paths, Error, Result};
use std::{io::Write, net::TcpListener, path::Path};
fn io(source: std::io::Error) -> Error {
    Error::Io {
        context: "playback listener",
        source,
    }
}
pub(crate) fn token(home: &Path) -> Result<String> {
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
        let staging = crate::readers::snapshot::TempDir::create(&dir)?;
        let complete = staging.0.join("token");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&complete).map_err(io)?;
        f.write_all(value.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(io)?;
        drop(f);
        // Publish only complete, synced bytes, without replacing an existing
        // credential if another initializer won the race.
        if let Err(error) = std::fs::hard_link(&complete, &file) {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(io(error));
            }
        }
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
        let (status, value) = receive(&mut stream, &mut c, &config, &host, &token, None);
        let _ = crate::ui::json_response(&mut stream, status, value);
    }
    Ok(())
}

/// Owns only its thread and listening socket. Collisions never touch another process.
pub struct Managed {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    active: std::sync::Arc<std::sync::Mutex<Option<std::net::TcpStream>>>,
    worker: Option<std::thread::JoinHandle<()>>,
    pub state: std::sync::Arc<std::sync::Mutex<String>>,
}
impl Managed {
    pub fn start(paths: Paths) -> Result<Self> {
        let mut config = Config::load(&paths.config)?;
        config.busy_timeout_ms = 250;
        let listener =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, config.listener_port)).map_err(io)?;
        listener.set_nonblocking(true).map_err(io)?;
        let host = listener.local_addr().map_err(io)?.to_string();
        let token = token(&paths.home)?;
        let mut c = crate::db::open(&paths.db, &config, true)?;
        c.pragma_update(None, "cache_size", -256)?;
        c.pragma_update(None, "synchronous", "FULL")?;
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let active = std::sync::Arc::new(std::sync::Mutex::new(None::<std::net::TcpStream>));
        let state = std::sync::Arc::new(std::sync::Mutex::new("ready".to_owned()));
        let (flag, socket, status) = (stop.clone(), active.clone(), state.clone());
        let worker = std::thread::spawn(move || {
            let mut next = std::time::Instant::now();
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                if std::time::Instant::now() >= next {
                    // A bounded transaction and retry delay, including on startup.
                    let result = crate::sessionize::process_batch(&mut c);
                    *status.lock().unwrap() = if result.is_ok() {
                        "ready"
                    } else {
                        "busy_or_failed"
                    }
                    .into();
                    next = std::time::Instant::now() + std::time::Duration::from_secs(1);
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        *socket.lock().unwrap() = stream.try_clone().ok();
                        let reply =
                            receive(&mut stream, &mut c, &config, &host, &token, Some(&flag));
                        if !flag.load(std::sync::atomic::Ordering::Relaxed) {
                            let _ = crate::ui::json_response(&mut stream, reply.0, reply.1);
                        }
                        socket.lock().unwrap().take();
                        next = std::time::Instant::now();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::park_timeout(std::time::Duration::from_millis(100))
                    }
                    Err(_) => {
                        *status.lock().unwrap() = "failed".into();
                        break;
                    }
                }
            }
        });
        Ok(Self {
            stop,
            active,
            worker: Some(worker),
            state,
        })
    }
}
impl Drop for Managed {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(stream) = self.active.lock().unwrap().as_ref() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}
fn receive(
    stream: &mut std::net::TcpStream,
    c: &mut rusqlite::Connection,
    config: &Config,
    host: &str,
    token: &str,
    stop: Option<&std::sync::atomic::AtomicBool>,
) -> (u16, serde_json::Value) {
    let r = match crate::ui::request_cancellable(stream, 8192, 16384, stop) {
        Ok(r) => r,
        Err(_) => return (400, serde_json::json!({"error":"Invalid request"})),
    };
    if r.method != "POST"
        || !["/event", "/hello"].contains(&r.target.as_str())
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
            host,
            token,
        )
    {
        return (
            403,
            serde_json::json!({"error":"Playback request forbidden"}),
        );
    }
    let result = (|| -> Result<serde_json::Value> {
        if r.target == "/hello" {
            let hello = serde_json::from_slice(&r.body)
                .map_err(|_| Error::Config("Invalid client hello".into()))?;
            let s = crate::setup::observe(c, hello)?;
            return Ok(
                serde_json::json!({"protocol":1,"generation":s.capture_epoch,"compatible":s.hello.as_ref().is_some_and(|h|h.compatible())}),
            );
        }
        let body = std::str::from_utf8(&r.body)
            .map_err(|_| Error::Config("Invalid playback event".into()))?;
        Ok(
            serde_json::json!({"accepted":crate::sessionize::enqueue(c,body,config.offset_seconds()?)?}),
        )
    })();
    match result {
        Ok(value) => (200, value),
        Err(Error::Config(message)) => (400, serde_json::json!({"error":message})),
        Err(_) => (
            503,
            serde_json::json!({"error":"Playback storage unavailable; retry"}),
        ),
    }
}
