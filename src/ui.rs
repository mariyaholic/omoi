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
pub(crate) fn request(
    stream: &mut TcpStream,
    max_header: usize,
    max_body: usize,
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
        let n = stream.read(&mut buffer)?;
        if n == 0 {
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
        let n = stream.read(&mut body[read..])?;
        if n == 0 {
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
    write!(stream,"HTTP/1.1 {status} {reason}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; font-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'\r\n\r\n",data.len())?;
    stream.write_all(data)
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

fn api(req: &Request, paths: &Paths, env: &PathEnvironment, idle_seconds: u64) -> Result<Value> {
    let config = Config::load(&paths.config)?;
    let url = url::Url::parse(&format!("http://127.0.0.1{}", req.target))
        .map_err(|_| Error::Config("invalid URL".into()))?;
    let q = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect::<BTreeMap<_, _>>();
    let c = crate::db::open(&paths.db, &config, false)?;
    match (req.method.as_str(), url.path()) {
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
                json!({"profiles":d.profiles.iter().map(|p|json!({"id":p.source.0,"browser":p.browser,"name":p.name})).collect::<Vec<_>>(),"excluded":d.excluded,"warnings":d.warnings,"failures":d.failures}),
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

/// Owns the desktop's loopback server. Dropping it finishes the current request
/// and shuts down the server without terminating an in-progress transaction.
pub struct DesktopServer {
    pub launch_url: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<()>>>,
}
struct DesktopControl {
    ready: std::sync::mpsc::SyncSender<String>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for DesktopServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn start_desktop(paths: Paths, env: PathEnvironment, port: u16) -> Result<DesktopServer> {
    let (ready, receiver) = std::sync::mpsc::sync_channel(1);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let control = DesktopControl {
        ready,
        stop: stop.clone(),
    };
    let thread = std::thread::spawn(move || serve_inner(paths, env, port, 0, false, Some(control)));
    match receiver.recv() {
        Ok(launch_url) => Ok(DesktopServer {
            launch_url,
            stop,
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
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };
        let (mut stream, at) = pending.swap_remove(index);
        if at.elapsed() > Duration::from_secs(2) {
            continue;
        }
        stream.set_nonblocking(false).map_err(io)?;
        let req = match request(&mut stream, 16384, 65536) {
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
            match api(&req, &paths, &env, idle_seconds) {
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
    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"omoi-history.{ext}\"\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'\r\n\r\n",f.metadata().map_err(io)?.len()).map_err(io)?;
    std::io::copy(&mut f, stream).map_err(io)?;
    Ok(())
}
