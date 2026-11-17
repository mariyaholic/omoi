#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    std::process::exit(5);
}

#[cfg(windows)]
fn main() {
    if let Err(error) = desktop::run() {
        if std::env::args().any(|a| a == "--check") {
            eprintln!("{error}");
        } else {
            use windows::{
                core::PCWSTR,
                Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
            };
            let text: Vec<u16> = format!("Omoi could not open.\n\n{error}\n\nIf Omoi is already open, use its existing window. If WebView2 is missing, install Microsoft Edge WebView2 Evergreen Runtime from Microsoft, then reopen Omoi. See SETUP.md.")
                .encode_utf16().chain(Some(0)).collect();
            unsafe {
                MessageBoxW(
                    None,
                    PCWSTR(text.as_ptr()),
                    windows::core::w!("Omoi"),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod desktop {
    use omoi::paths::{PathEnvironment, Paths};
    use std::{
        error::Error,
        os::windows::process::CommandExt,
        time::{Duration, Instant},
    };
    use tao::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::{Icon, WindowBuilder},
    };
    use wry::{NewWindowResponse, WebContext, WebViewBuilder};

    fn open_external(value: &str) {
        if url::Url::parse(value).is_ok_and(|u| u.scheme() == "https" && u.host_str().is_some()) {
            let _ = std::process::Command::new("rundll32.exe")
                .args(["url.dll,FileProtocolHandler", value])
                .creation_flags(0x08000000)
                .spawn();
        }
    }

    pub fn run() -> Result<(), Box<dyn Error>> {
        let check = std::env::args().any(|a| a == "--check");
        if std::env::args().skip(1).any(|a| a != "--check") {
            return Err("Unexpected desktop argument".into());
        }
        if check && std::env::var_os("OMOI_HOME").is_none() {
            return Err("--check requires an isolated OMOI_HOME".into());
        }
        let env = PathEnvironment::capture();
        let paths = Paths::resolve(&env)?;
        let profile = paths.home.join("webview");
        // A stable desktop origin preserves the webview's appearance preferences.
        // The ordinary browser UI has its own port (43822).
        let server = omoi::ui::start_desktop(paths, env, if check { 0 } else { 43823 })?;
        let origin = server
            .launch_url
            .split("/#")
            .next()
            .ok_or("Invalid launch URL")?
            .to_owned();
        let launch = serde_json::to_string(&server.launch_url)?;
        let origin_json = serde_json::to_string(&origin)?;
        let init = format!(
            "if(location.origin==={origin_json}){{history.replaceState(null,'',{launch});}}"
        );
        let events = EventLoopBuilder::<bool>::with_user_event().build();
        let proxy = events.create_proxy();
        let pixels = include_bytes!("../../assets/icon-32.rgba").to_vec();
        let window = WindowBuilder::new()
            .with_title("Omoi")
            .with_visible(!check)
            .with_inner_size(LogicalSize::new(1120., 780.))
            .with_min_inner_size(LogicalSize::new(360., 480.))
            .with_window_icon(Some(Icon::from_rgba(pixels, 32, 32)?))
            .build(&events)?;
        let mut context = WebContext::new(Some(profile));
        let allowed = origin.clone();
        let mut builder = WebViewBuilder::new_with_web_context(&mut context)
            .with_url(&server.launch_url)
            .with_initialization_script(init)
            .with_devtools(false)
            .with_navigation_handler(move |value| {
                let local = url::Url::parse(&value)
                    .is_ok_and(|u| u.origin().ascii_serialization() == allowed && u.path() == "/");
                if !local && !check {
                    open_external(&value);
                }
                local
            })
            .with_new_window_req_handler(move |value, _| {
                if !check {
                    open_external(&value);
                }
                NewWindowResponse::Deny
            });
        if check {
            builder = builder
                .with_ipc_handler(move |request| {
                    let _ = proxy.send_event(request.body() == "ready");
                })
                .with_initialization_script(include_str!("../../ui/desktop-check.js"));
        }
        let mut webview = Some(builder.build(&window)?);
        let mut server = Some(server);
        let deadline = Instant::now() + Duration::from_secs(20);
        events.run(move |event, _, flow| {
            *flow = if check {
                ControlFlow::WaitUntil(deadline)
            } else {
                ControlFlow::Wait
            };
            let result = match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => Some(0),
                Event::UserEvent(ready) if check => Some(if ready { 0 } else { 1 }),
                Event::NewEvents(_) if check && Instant::now() >= deadline => Some(1),
                _ => None,
            };
            if let Some(code) = result {
                window.set_visible(false);
                webview.take();
                server.take();
                *flow = ControlFlow::ExitWithCode(code);
            }
        });
    }
}
