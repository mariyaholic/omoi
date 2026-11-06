//! On-demand loopback UI. Bounded, sequential HTTP; no runtime or resident poller.
use crate::{
    config::Config,
    paths::{PathEnvironment, Paths},
    Error, Result,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct HistoryQuery {
    pub query: String,
    pub platform: String,
    pub channel: String,
    pub after: i64,
    pub offset: i64,
    pub literal: bool,
}

pub fn catalogue(c: &Connection, q: &HistoryQuery) -> Result<Value> {
    if q.query.len() > 1000 || q.channel.len() > 1000 || !(0..=10_000_000).contains(&q.offset) {
        return Err(Error::Config("search or page is too large".into()));
    }
    // Quoted FTS phrase is data, never user-supplied FTS syntax. Literal mode uses instr.
    let phrase = format!("\"{}\"", q.query.replace('"', "\"\""));
    let search = if q.literal {
        "instr(lower(coalesce(i.title,'')||' '||coalesce(i.channel,'')||' '||coalesce(i.series,'')),lower(?1))>0"
    } else {
        "i.id IN (SELECT rowid FROM items_fts WHERE items_fts MATCH ?6)"
    };
    // Avoid an outer join on a correlated rowid: stale one-row ANALYZE statistics
    // can turn it into a full views scan for every item after a large import.
    let latest = "FROM views INDEXED BY views_item_time WHERE item_id=i.id ORDER BY started_at DESC,id DESC LIMIT 1";
    let last_time = format!("(SELECT started_at {latest})");
    let filter = format!("(?1='' OR {search}) AND (?2='' OR i.platform=?2) AND (?3='' OR i.channel=?3) AND (?4=0 OR {last_time}>=?4)");
    let from = "FROM items i";
    let tx = c.unchecked_transaction()?;
    // Keep ?6 bound even when literal mode does not use FTS.
    let total: i64 = tx.query_row(
        &format!("SELECT count(*) {from} WHERE {filter} AND ?5>=0 AND length(?6)>=0"),
        params![q.query, q.platform, q.channel, q.after, q.offset, phrase],
        |r| r.get(0),
    )?;
    let mut stmt = tx.prepare(&format!("SELECT i.id,i.platform,substr(i.title,1,4096),substr(i.channel,1,1024),i.kind,i.duration_s,{last_time} AS last_opened,(SELECT substr(source,1,128) {latest}),(SELECT approx_duration_s {latest}) {from} WHERE {filter} AND length(?6)>=0 ORDER BY last_opened DESC,i.id DESC LIMIT 50 OFFSET ?5"))?;
    let rows = stmt.query_map(params![q.query,q.platform,q.channel,q.after,q.offset,phrase], |r| Ok(json!({
        "id":r.get::<_,i64>(0)?,"platform":r.get::<_,String>(1)?,"title":r.get::<_,Option<String>>(2)?,
        "channel":r.get::<_,Option<String>>(3)?,"kind":r.get::<_,Option<String>>(4)?,"duration_s":r.get::<_,Option<i64>>(5)?,
        "last_opened":r.get::<_,Option<i64>>(6)?,"source":r.get::<_,Option<String>>(7)?,"tab_seconds":r.get::<_,Option<i64>>(8)?
    })))?.collect::<std::result::Result<Vec<_>,_>>()?;
    drop(stmt);
    tx.commit()?;
    Ok(json!({"items":rows,"total":total,"offset":q.offset,"page_size":50}))
}

pub fn delete_item(c: &mut Connection, id: i64) -> Result<bool> {
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute("INSERT INTO deleted_items(platform,external_id,through) SELECT platform,external_id,?2 FROM items WHERE id=?1 ON CONFLICT(platform,external_id) DO UPDATE SET through=max(through,excluded.through)",params![id,chrono::Utc::now().timestamp()])?;
    let count = tx.execute("DELETE FROM items WHERE id=?", [id])?;
    tx.commit()?;
    Ok(count == 1)
}

fn item(c: &Connection, id: i64) -> Result<Value> {
    let mut value = c.query_row("SELECT id,platform,external_id,substr(title,1,4096),substr(channel,1,1024),kind,duration_s FROM items WHERE id=?", [id], |r|Ok(json!({
        "id":r.get::<_,i64>(0)?,"platform":r.get::<_,String>(1)?,"external_id":r.get::<_,String>(2)?,"title":r.get::<_,Option<String>>(3)?,
        "channel":r.get::<_,Option<String>>(4)?,"kind":r.get::<_,Option<String>>(5)?,"duration_s":r.get::<_,Option<i64>>(6)?
    }))).optional()?.ok_or_else(|| Error::Config("item no longer exists".into()))?;
    let mut stmt = c.prepare("SELECT started_at,substr(source,1,128),approx_duration_s FROM views WHERE item_id=? ORDER BY started_at DESC,id DESC LIMIT 50")?;
    value["visits"] = json!(stmt.query_map([id],|r|Ok(json!({"at":r.get::<_,i64>(0)?,"source":r.get::<_,String>(1)?,"tab_seconds":r.get::<_,Option<i64>>(2)?})))?.collect::<std::result::Result<Vec<_>,_>>()?);
    value["visit_count"] = json!(c.query_row(
        "SELECT count(*) FROM views WHERE item_id=?",
        [id],
        |r| r.get::<_, i64>(0)
    )?);
    let mut tags = c.prepare(
        "SELECT substr(tag,1,256) FROM item_tags WHERE item_id=? ORDER BY tag LIMIT 100",
    )?;
    value["tags"] = json!(tags
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?);
    value["played_seconds"] = json!(crate::sessionize::total(c, id)?);
    Ok(value)
}

pub fn authorized(
    host: &str,
    origin: Option<&str>,
    supplied: &str,
    expected_host: &str,
    token: &str,
) -> bool {
    host == expected_host
        && origin.is_none_or(|s| s == format!("http://{expected_host}"))
        && supplied.len() == 64
        && token.len() == 64
        && supplied
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

pub(crate) fn random_token() -> Result<String> {
    let mut bytes = [0u8; 32];
    #[cfg(windows)]
    unsafe {
        use windows::Win32::Security::Cryptography::{
            BCryptGenRandom, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        };
        BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG)
            .ok()
            .map_err(|_| Error::Config("OS randomness unavailable".into()))?;
    }
    #[cfg(not(windows))]
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(io)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn io(source: std::io::Error) -> Error {
    Error::Io {
        context: "local UI",
        source,
    }
}

pub(crate) struct Request {
    pub method: String,
    pub target: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}
fn read_request(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
    stop: Option<&std::sync::atomic::AtomicBool>,
) -> std::io::Result<usize> {
    let deadline = deadline.min(Instant::now() + Duration::from_secs(2));
    loop {
        if stop.is_some_and(|s| s.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(remaining.min(Duration::from_millis(100))))?;
        match stream.read(buffer) {
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            result => return result,
        }
    }
}
pub(crate) fn request_cancellable(
    stream: &mut TcpStream,
    max_header: usize,
    max_body: usize,
    stop: Option<&std::sync::atomic::AtomicBool>,
) -> std::io::Result<Request> {
    let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid request");
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.set_nodelay(true)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
            if index + 4 > max_header {
                return Err(invalid());
            }
            break index + 4;
        }
        if bytes.len() >= max_header || Instant::now() >= deadline {
            return Err(invalid());
        }
        let mut buffer = [0u8; 2048];
        let n = read_request(stream, &mut buffer, deadline, stop)?;
        if n == 0 || Instant::now() >= deadline {
            return Err(invalid());
        }
        bytes.extend_from_slice(&buffer[..n]);
    };
    let text = std::str::from_utf8(&bytes[..header_end]).map_err(|_| invalid())?;
    let mut lines = text.split("\r\n");
    let parts = lines
        .next()
        .unwrap_or_default()
        .split(' ')
        .collect::<Vec<_>>();
    if parts.len() != 3
        || parts[2] != "HTTP/1.1"
        || !parts[1].starts_with('/')
        || parts[1].starts_with("//")
    {
        return Err(invalid());
    }
    let mut headers = BTreeMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let (k, v) = line.split_once(':').ok_or_else(invalid)?;
        if k.is_empty()
            || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || headers
                .insert(k.to_ascii_lowercase(), v.trim().to_owned())
                .is_some()
        {
            return Err(invalid());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err(invalid());
    }
    let length = headers
        .get("content-length")
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(|_| invalid())?
        .unwrap_or(0);
    if length > max_body {
        return Err(invalid());
    }
    let mut body = vec![0; length];
    let mut read = bytes.len() - header_end;
    if read > length {
        return Err(invalid());
    }
    body[..read].copy_from_slice(&bytes[header_end..]);
    while read < length {
        if Instant::now() >= deadline {
            return Err(invalid());
        }
        let n = read_request(stream, &mut body[read..], deadline, stop)?;
        if n == 0 || Instant::now() >= deadline {
            return Err(invalid());
        }
        read += n;
    }
    Ok(Request {
        method: parts[0].into(),
        target: parts[1].into(),
        headers,
        body,
    })
}

fn response(stream: &mut TcpStream, status: u16, mime: &str, data: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Service Unavailable",
    };
    let header=format!("HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; font-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'\r\n\r\n",data.len());
    let deadline = Instant::now() + Duration::from_secs(30);
    write_until(stream, header.as_bytes(), deadline)?;
    write_until(stream, data, deadline)
}

fn write_until(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> std::io::Result<()> {
    while !bytes.is_empty() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "response deadline",
            ));
        }
        stream.set_write_timeout(Some(remaining.min(Duration::from_secs(5))))?;
        let count = stream.write(&bytes[..bytes.len().min(16384)])?;
        if count == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        bytes = &bytes[count..];
    }
    Ok(())
}
pub(crate) fn json_response(
    stream: &mut TcpStream,
    status: u16,
    value: Value,
) -> std::io::Result<()> {
    response(
        stream,
        status,
        "application/json; charset=utf-8",
        &serde_json::to_vec(&value).expect("JSON value"),
    )
}
fn number(q: &BTreeMap<String, String>, key: &str, default: i64) -> Result<i64> {
    q.get(key)
        .map(|v| {
            v.parse()
                .map_err(|_| Error::Config(format!("invalid {key}")))
        })
        .unwrap_or(Ok(default))
}

fn api(
    req: &Request,
    paths: &Paths,
    env: &PathEnvironment,
    idle_seconds: u64,
    capture: &mut CaptureRuntime,
) -> Result<Value> {
    let config = Config::load(&paths.config)?;
    let url = url::Url::parse(&format!("http://127.0.0.1{}", req.target))
        .map_err(|_| Error::Config("invalid URL".into()))?;
    let q = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect::<BTreeMap<_, _>>();
    let c = crate::db::open(&paths.db, &config, false)?;
    match (req.method.as_str(), url.path()) {
        ("GET", "/api/setup") => setup_status(&c, capture),
        ("POST", "/api/setup") => {
            let change = serde_json::from_slice(&req.body)
                .map_err(|_| Error::Config("Invalid setup change".into()))?;
            let mut writer = crate::db::open(&paths.db, &config, true)?;
            crate::setup::change(&mut writer, change)?;
            capture.reconcile(paths, &writer)?;
            setup_status(&writer, capture)
        }
        ("POST", "/api/setup/token") => {
            if crate::setup::Setup::load(&c)?.choice != crate::setup::Choice::Enabled {
                return Err(Error::Config("Enable capture first".into()));
            }
            Ok(json!({"token":crate::listener::token(&paths.home)?,"port":config.listener_port}))
        }
        ("GET", "/api/setup/script") => {
            Ok(json!({"name":"omoi.user.js","content":include_str!("../userscripts/omoi.user.js")}))
        }
        ("POST", "/api/edit") => {
            let body: Value = serde_json::from_slice(&req.body)
                .map_err(|_| Error::Config("Invalid metadata".into()))?;
            let mut writer = crate::db::open(&paths.db, &config, true)?;
            crate::metadata::edit(&mut writer, &body)?;
            Ok(json!({"saved":true}))
        }
        ("POST", "/api/playback/process") => {
            let mut writer = crate::db::open(&paths.db, &config, true)?;
            let tx = writer.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let count = crate::sessionize::run(&tx)?;
            tx.commit()?;
            Ok(json!({"processed":count}))
        }
        ("GET", "/api/playback") => Ok(
            json!({"port":config.listener_port,"token_file":paths.home.join("listener/token"),"pending":c.query_row("SELECT count(*) FROM raw_events",[],|r|r.get::<_,i64>(0))?,"dropped":c.query_row("SELECT value FROM playback_stats WHERE key='dropped'",[],|r|r.get::<_,i64>(0))?}),
        ),
        ("POST", "/api/schedule") => {
            let body: Value = serde_json::from_slice(&req.body)
                .map_err(|_| Error::Config("Invalid scheduling request".into()))?;
            let action = body["action"]
                .as_str()
                .filter(|s| ["install", "remove", "status"].contains(s))
                .ok_or_else(|| Error::Config("Invalid scheduling action".into()))?;
            if action == "install" {
                let profiles: Vec<String> = serde_json::from_value(body["profiles"].clone())
                    .map_err(|_| {
                        Error::Config("Select profiles before enabling automatic collection".into())
                    })?;
                if profiles.is_empty() {
                    return Err(Error::Config("Select at least one profile".into()));
                }
                let discovered = crate::readers::discover::discover(&config, env)?;
                if profiles
                    .iter()
                    .any(|id| !discovered.profiles.iter().any(|p| &p.source.0 == id))
                {
                    return Err(Error::Config(
                        "Profile selection changed. Refresh Sources.".into(),
                    ));
                }
                let mut writer = crate::db::open(&paths.db, &config, true)?;
                let current = crate::setup::Setup::load(&writer)?;
                crate::setup::change(
                    &mut writer,
                    crate::setup::Change {
                        generation: current.generation,
                        choice: None,
                        step: None,
                        background: None,
                        profiles: Some(profiles),
                    },
                )?;
            }
            Ok(json!({"output":crate::scheduler::result(action,&paths.home)?}))
        }
        ("GET", "/api/info") => {
            let mut p = c.prepare("SELECT DISTINCT platform FROM items ORDER BY platform")?;
            let platforms = p
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(
                json!({"catalogue":paths.db,"config":paths.config,"platforms":platforms,"items":c.query_row("SELECT count(*) FROM items",[],|r|r.get::<_,i64>(0))?,"idle_seconds":idle_seconds}),
            )
        }
        ("GET", "/api/history") => catalogue(
            &c,
            &HistoryQuery {
                query: q.get("q").cloned().unwrap_or_default(),
                platform: q.get("platform").cloned().unwrap_or_default(),
                channel: q.get("channel").cloned().unwrap_or_default(),
                after: number(&q, "after", 0)?,
                offset: number(&q, "offset", 0)?,
                literal: q.get("mode").is_some_and(|m| m == "literal"),
            },
        ),
        ("GET", "/api/item") => item(&c, number(&q, "id", 0)?),
        ("GET", "/api/suggestions") => Ok(serde_json::to_value(crate::recommend::shelves(
            &c,
            chrono::Utc::now().timestamp(),
            q.get("item").map(|_| number(&q, "item", 0)).transpose()?,
            number(&q, "seconds", 1800)?,
        )?)
        .expect("serializable shelves")),
        ("GET", "/api/sources") => {
            let d = crate::readers::discover::discover(&config, env)?;
            Ok(
                json!({"profiles":d.profiles.iter().map(|p|json!({"id":p.source.0,"browser":p.browser,"name":p.name})).collect::<Vec<_>>(),"excluded":d.excluded,"warnings":d.warnings,"failures":d.failures,"selected":crate::setup::Setup::load(&c)?.profiles}),
            )
        }
        ("POST", "/api/delete") => {
            let body: Value = serde_json::from_slice(&req.body)
                .map_err(|_| Error::Config("invalid JSON".into()))?;
            let id = body["id"]
                .as_i64()
                .filter(|id| *id > 0)
                .ok_or_else(|| Error::Config("invalid item".into()))?;
            let mut writer = crate::db::open(&paths.db, &config, true)?;
            Ok(json!({"deleted":delete_item(&mut writer,id)?}))
        }
        ("POST", "/api/collect") => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Selected {
                profiles: Vec<String>,
            }
            let selected: Selected = serde_json::from_slice(&req.body)
                .map_err(|_| Error::Config("invalid profile selection".into()))?;
            drop(c);
            let result = crate::collect::collect_selected(
                &config,
                paths,
                env,
                false,
                Some(&selected.profiles),
            );
            let code = match &result {
                Ok(r) if r.profiles_failed > 0 => 3,
                Ok(_) => 0,
                Err(e) => e.exit_code(),
            };
            let _ = crate::logging::record(&paths.logs, &config, result.as_ref().ok(), code);
            Ok(serde_json::to_value(result?).expect("serializable report"))
        }
        _ => Err(Error::Config("unknown operation".into())),
    }
}

pub fn serve(
    paths: Paths,
    env: PathEnvironment,
    port: u16,
    idle_seconds: u64,
    open: bool,
) -> Result<()> {
    serve_inner(paths, env, port, idle_seconds, open, None)
}

/// Dropping the server cancels active network I/O, then joins the worker after
/// any in-progress catalogue operation completes. Transactions are not aborted.
pub struct DesktopServer {
    pub launch_url: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    active: std::sync::Arc<std::sync::Mutex<Option<TcpStream>>>,
    thread: Option<std::thread::JoinHandle<Result<()>>>,
}
struct DesktopControl {
    ready: std::sync::mpsc::SyncSender<String>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    active: std::sync::Arc<std::sync::Mutex<Option<TcpStream>>>,
}
struct ActiveRequest<'a>(&'a std::sync::Mutex<Option<TcpStream>>);
impl Drop for ActiveRequest<'_> {
    fn drop(&mut self) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
    }
}
impl Drop for DesktopServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(stream) = self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn start_desktop(paths: Paths, env: PathEnvironment, port: u16) -> Result<DesktopServer> {
    let (ready, receiver) = std::sync::mpsc::sync_channel(1);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let active = std::sync::Arc::new(std::sync::Mutex::new(None));
    let control = DesktopControl {
        ready,
        stop: stop.clone(),
        active: active.clone(),
    };
    let thread = std::thread::spawn(move || serve_inner(paths, env, port, 0, false, Some(control)));
    match receiver.recv() {
        Ok(launch_url) => Ok(DesktopServer {
            launch_url,
            stop,
            active,
            thread: Some(thread),
        }),
        Err(_) => {
            thread
                .join()
                .map_err(|_| Error::Config("Desktop server stopped during startup".into()))??;
            Err(Error::Config("Desktop server did not start".into()))
        }
    }
}
fn serve_inner(
    paths: Paths,
    env: PathEnvironment,
    port: u16,
    idle_seconds: u64,
    open: bool,
    desktop: Option<DesktopControl>,
) -> Result<()> {
    // Bind first: a collision must not mutate the catalogue.
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(io)?;
    listener.set_nonblocking(true).map_err(io)?;
    let host = listener.local_addr().map_err(io)?.to_string();
    let token = random_token()?;
    std::fs::create_dir_all(&paths.home).map_err(io)?;
    let config = Config::load(&paths.config)?;
    let mut conn = crate::db::open(&paths.db, &config, true)?;
    crate::db::migrate(&mut conn)?;
    let _app = crate::setup::app_lock(&paths.home)?;
    let mut capture = CaptureRuntime::default();
    capture.reconcile(&paths, &conn)?;
    drop(conn);
    let launch_url = format!("http://{host}/#token={token}");
    if let Some(control) = &desktop {
        control
            .ready
            .send(launch_url.clone())
            .map_err(|_| Error::Config("Desktop window closed during startup".into()))?;
    } else {
        println!("{launch_url}");
        std::io::stdout().flush().map_err(io)?;
    }
    if open {
        #[cfg(windows)]
        let opened = {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", &launch_url])
                .creation_flags(0x08000000)
                .spawn()
        };
        #[cfg(not(windows))]
        let opened = std::process::Command::new("xdg-open")
            .arg(&launch_url)
            .spawn();
        if opened.is_err() {
            eprintln!("Browser launch failed. Open the printed URL manually.");
        }
    }
    let mut active = Instant::now();
    // Browsers open speculative idle connections. Park a bounded number until
    // readable rather than blocking real API traffic behind each idle socket.
    let mut pending: Vec<(TcpStream, Instant)> = Vec::new();
    while match &desktop {
        Some(control) => !control.stop.load(std::sync::atomic::Ordering::Relaxed),
        None => active.elapsed() < Duration::from_secs(idle_seconds),
    } {
        for _ in 0..16 {
            match listener.accept() {
                Ok((stream, _)) if pending.len() < 16 => {
                    stream.set_nonblocking(true).map_err(io)?;
                    pending.push((stream, Instant::now()));
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(io(e)),
            }
        }
        let ready=pending.iter().position(|(stream,at)|at.elapsed()>Duration::from_secs(2)||!matches!(stream.peek(&mut [0u8;1]),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
        let Some(index) = ready else {
            std::thread::sleep(Duration::from_millis(if pending.is_empty() {
                100
            } else {
                10
            }));
            continue;
        };
        let (mut stream, at) = pending.swap_remove(index);
        if at.elapsed() > Duration::from_secs(2)
            && matches!(stream.peek(&mut [0u8;1]),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
        {
            continue;
        }
        stream.set_nonblocking(false).map_err(io)?;
        let _active = if let Some(control) = &desktop {
            let mut active = control
                .active
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if control.stop.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            *active = Some(stream.try_clone().map_err(io)?);
            Some(ActiveRequest(&control.active))
        } else {
            None
        };
        let req = match request_cancellable(
            &mut stream,
            16384,
            65536,
            desktop.as_ref().map(|c| c.stop.as_ref()),
        ) {
            Ok(r) => r,
            Err(_) => {
                let _ = json_response(
                    &mut stream,
                    400,
                    json!({"error":"Invalid or oversized request"}),
                );
                continue;
            }
        };
        let supplied_host = req
            .headers
            .get("host")
            .map(String::as_str)
            .unwrap_or_default();
        if supplied_host != host {
            let _ = json_response(&mut stream, 403, json!({"error":"Forbidden"}));
            continue;
        }
        if req.target.starts_with("/api/") {
            let origin = req.headers.get("origin").map(String::as_str);
            let supplied = req
                .headers
                .get("x-omoi-token")
                .map(String::as_str)
                .unwrap_or_default();
            if !authorized(supplied_host, origin, supplied, &host, &token)
                || (req.method != "GET"
                    && (origin != Some(format!("http://{host}").as_str())
                        || req.headers.get("content-type").map(String::as_str)
                            != Some("application/json")))
            {
                let _ = json_response(
                    &mut stream,
                    403,
                    json!({"error":"Session unavailable. Open the launch URL from omoi ui."}),
                );
                continue;
            }
            if req.method == "GET"
                && ["/api/export.json", "/api/export.csv"].contains(&req.target.as_str())
            {
                if download(&mut stream, &paths, req.target.ends_with(".json")).is_err() {
                    let _ = json_response(&mut stream, 400, json!({"error":"Export failed"}));
                }
                active = Instant::now();
                continue;
            }
            match api(&req, &paths, &env, idle_seconds, &mut capture) {
                Ok(v) => {
                    let _ = json_response(&mut stream, 200, v);
                }
                Err(e) => {
                    let message=match e {Error::Config(s)=>s,Error::Busy=>"Catalogue busy. Retry shortly.".into(),_=>"Operation failed. Check catalogue permissions and configuration, then retry.".into()};
                    let _ = json_response(&mut stream, 400, json!({"error":message}));
                }
            }
            active = Instant::now();
        } else if req.method == "GET" {
            if req.target == "/favicon.ico" {
                let _ = response(
                    &mut stream,
                    200,
                    "image/x-icon",
                    include_bytes!("../assets/icon.ico"),
                );
                continue;
            }
            let asset = match req.target.as_str() {
                "/icon.svg" => Some(("image/svg+xml", include_str!("../assets/icon.svg"))),
                "/forms.css" => Some(("text/css; charset=utf-8", include_str!("../ui/forms.css"))),
                "/" => Some(("text/html; charset=utf-8", include_str!("../ui/index.html"))),
                "/app.js" => Some((
                    "text/javascript; charset=utf-8",
                    include_str!("../ui/app.js"),
                )),
                "/app.css" => Some(("text/css; charset=utf-8", include_str!("../ui/app.css"))),
                "/style.css" => Some((
                    "text/css; charset=utf-8",
                    include_str!("../prototype/style.css"),
                )),
                "/inspiration.css" => Some((
                    "text/css; charset=utf-8",
                    include_str!("../prototype/inspiration.css"),
                )),
                "/tokens.css" => Some((
                    "text/css; charset=utf-8",
                    include_str!("../prototype/tokens.css"),
                )),
                _ => None,
            };
            if let Some((mime, data)) = asset {
                let _ = response(&mut stream, 200, mime, data.as_bytes());
            } else {
                let _ = json_response(&mut stream, 404, json!({"error":"Not found"}));
            }
        } else {
            let _ = json_response(&mut stream, 403, json!({"error":"Forbidden"}));
        }
    }
    Ok(())
}

fn download(stream: &mut TcpStream, paths: &Paths, json: bool) -> Result<()> {
    let config = Config::load(&paths.config)?;
    let c = crate::db::open(&paths.db, &config, false)?;
    let temp = crate::readers::snapshot::TempDir::create(&paths.tmp)?;
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(temp.0.join("export"))
        .map_err(io)?;
    crate::export::export(&c, json, &mut f)?;
    use std::io::Seek;
    f.rewind().map_err(io)?;
    let ext = if json { "json" } else { "csv" };
    let header=format!("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"omoi-history.{ext}\"\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'\r\n\r\n",f.metadata().map_err(io)?.len());
    let deadline = Instant::now() + Duration::from_secs(30);
    write_until(stream, header.as_bytes(), deadline).map_err(io)?;
    let mut buffer = [0u8; 16384];
    loop {
        let count = f.read(&mut buffer).map_err(io)?;
        if count == 0 {
            break;
        }
        write_until(stream, &buffer[..count], deadline).map_err(io)?;
    }
    Ok(())
}

#[derive(Default)]
struct CaptureRuntime {
    owned: Option<crate::listener::Managed>,
    error: Option<String>,
}
impl CaptureRuntime {
    fn reconcile(&mut self, paths: &Paths, c: &Connection) -> Result<()> {
        if crate::setup::Setup::load(c)?.choice != crate::setup::Choice::Enabled {
            self.owned.take();
            self.error = None;
            return Ok(());
        }
        if self.owned.is_none() {
            match crate::listener::Managed::start(paths.clone()) {
                Ok(service)=>{self.owned=Some(service);self.error=None;}
                Err(Error::Io{source,..}) if source.kind()==std::io::ErrorKind::AddrInUse => self.error=Some("collision: close the other listener yourself, then Retry. No process was stopped.".into()),
                Err(_)=>self.error=Some("failed: check catalogue permissions and listener configuration, then Retry.".into()),
            }
        }
        Ok(())
    }
}
fn setup_status(c: &Connection, capture: &CaptureRuntime) -> Result<Value> {
    let s = crate::setup::Setup::load(c)?;
    let pending: i64 = c.query_row("SELECT count(*) FROM raw_events", [], |r| r.get(0))?;
    let transport = capture
        .owned
        .as_ref()
        .map(|m| m.state.lock().unwrap().clone())
        .unwrap_or_else(|| {
            if capture.error.is_some() {
                "collision_or_failed"
            } else {
                "stopped"
            }
            .into()
        });
    let data = if transport == "busy_or_failed" {
        "busy_or_failed"
    } else if pending > 0 {
        "queued"
    } else if s.last_processed.is_some() {
        "recorded"
    } else if s.last_event.is_some() {
        "receiving"
    } else {
        "no_event_yet"
    };
    Ok(
        json!({"client":s.client_state(chrono::Utc::now().timestamp()),"transport":transport,"data":data,"pending":pending,"error":capture.error,"app_version":env!("CARGO_PKG_VERSION"),"protocol":crate::setup::PROTOCOL,"client_version":crate::setup::CLIENT_VERSION,"settings":s}),
    )
}
