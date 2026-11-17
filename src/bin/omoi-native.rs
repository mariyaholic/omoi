//! Stdout is exclusively native messaging frames. No console subsystem suppression:
//! Edge owns redirected stdio (including on Windows).
use omoi::{
    native::{self, Registration, Session},
    paths::{PathEnvironment, Paths},
};
fn main() {
    if run().is_err() {
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let origin = args.first().ok_or("Missing extension origin")?;
    if args.len() > 2
        || args.get(1).is_some_and(|s| {
            !s.strip_prefix("--parent-window=")
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
    {
        return Err("Invalid native arguments".into());
    }
    let exe = std::env::current_exe()?;
    let file = std::fs::File::open(
        exe.parent()
            .ok_or("Missing host directory")?
            .join("omoi-host.json"),
    )?;
    use std::io::Read;
    let registration: Registration = serde_json::from_reader(file.take(16385))?;
    if !registration.permits(origin) {
        return Err("Extension origin forbidden".into());
    }
    let env = PathEnvironment {
        windows: cfg!(windows),
        vars: std::collections::BTreeMap::from([(
            "OMOI_HOME".into(),
            registration.home.into_os_string(),
        )]),
    };
    let mut session = match Session::open(Paths::resolve(&env)?) {
        Ok(s) => s,
        Err(_) => {
            native::write_frame(
                &mut std::io::stdout(),
                &serde_json::json!({"ok":false,"retry":false,"error":"Open a compatible Omoi app and enable capture; check the registered catalogue location."}),
            )?;
            return Ok(());
        }
    };
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut input = std::io::stdin().lock();
        while let Ok(Some(v)) = native::read_frame(&mut input) {
            if send.send(v).is_err() {
                break;
            }
        }
    });
    let mut last = std::time::Instant::now();
    // One bounded processing attempt per second; process exits after 60 seconds
    // without a frame. EOF exits promptly, even if browser worker disappeared.
    loop {
        match receive.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(value) => {
                last = std::time::Instant::now();
                native::write_frame(&mut std::io::stdout(), &session.handle(value))?;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(_) => {
                if last.elapsed() >= std::time::Duration::from_secs(60) {
                    break;
                }
                let _ = session.drain_if_allowed();
            }
        }
    }
    Ok(())
}
