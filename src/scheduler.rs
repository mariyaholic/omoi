//! Explicit, per-user installation. Pure XML generation never changes system state.
use crate::{Error, Result};
pub struct TaskSpec<'a> {
    pub sid: &'a str,
    pub start: &'a str,
    pub helper: &'a str,
    pub home: &'a str,
    pub directory: &'a str,
}
fn xml(s: &str) -> Result<String> {
    if s.chars().any(|c| c.is_control()) {
        return Err(Error::Config(
            "control character in task specification".into(),
        ));
    }
    Ok(s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"))
}
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for c in s.chars() {
        if c == '\\' {
            slashes += 1;
            continue;
        }
        if c == '"' {
            out.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            out.push_str(&"\\".repeat(slashes));
        }
        slashes = 0;
        out.push(c);
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}
pub fn task_xml(spec: &TaskSpec<'_>) -> Result<String> {
    let sid = xml(spec.sid)?;
    let start = xml(spec.start)?;
    let helper = xml(spec.helper)?;
    let directory = xml(spec.directory)?;
    let args = xml(&format!("--home {} collect", quote(spec.home)))?;
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
<RegistrationInfo><Description>Omoi local browser history collection</Description></RegistrationInfo>
<Triggers><TimeTrigger><Repetition><Interval>PT1H</Interval><StopAtDurationEnd>false</StopAtDurationEnd></Repetition><StartBoundary>{start}</StartBoundary><Enabled>true</Enabled></TimeTrigger><LogonTrigger><Enabled>true</Enabled><UserId>{sid}</UserId><Delay>PT30S</Delay></LogonTrigger></Triggers>
<Principals><Principal id="Owner"><UserId>{sid}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
<Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>true</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>true</StopIfGoingOnBatteries><AllowHardTerminate>true</AllowHardTerminate><StartWhenAvailable>true</StartWhenAvailable><RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><Hidden>true</Hidden><RunOnlyIfIdle>false</RunOnlyIfIdle><WakeToRun>false</WakeToRun><ExecutionTimeLimit>PT10M</ExecutionTimeLimit><Priority>7</Priority></Settings>
<Actions Context="Owner"><Exec><Command>{helper}</Command><Arguments>{args}</Arguments><WorkingDirectory>{directory}</WorkingDirectory></Exec></Actions></Task>"#
    ))
}
#[cfg(windows)]
fn powershell() -> Result<std::process::Command> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var_os("SystemRoot")
        .ok_or_else(|| Error::Config("SystemRoot unavailable".into()))?;
    let mut cmd = std::process::Command::new(
        std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
    );
    cmd.creation_flags(0x08000000);
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
    ]);
    Ok(cmd)
}
#[cfg(windows)]
pub fn execute(mode: &str, home: &std::path::Path) -> Result<()> {
    use chrono::Timelike;
    let run = crate::readers::snapshot::TempDir::create(&std::env::temp_dir())?;
    let script = run.0.join("schedule.ps1");
    std::fs::write(&script, include_str!("scheduler/windows.ps1")).map_err(|source| Error::Io {
        context: "scheduler script",
        source,
    })?;
    let task = run.0.join("task.xml");
    if mode == "install" {
        let exe = std::env::current_exe().map_err(|source| Error::Io {
            context: "scheduler executable",
            source,
        })?;
        let dir = exe
            .parent()
            .ok_or_else(|| Error::Config("missing executable directory".into()))?;
        let helper = dir.join("omoiw.exe");
        if !helper.is_file() {
            return Err(Error::Config(
                "install omoiw.exe adjacent to omoi.exe first".into(),
            ));
        }
        let sid = powershell()?
            .args([
                "-Command",
                "[Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
            ])
            .output()
            .map_err(|source| Error::Io {
                context: "scheduler identity",
                source,
            })?;
        if !sid.status.success() {
            return Err(Error::Config("unable to resolve current SID".into()));
        }
        let sid = String::from_utf8(sid.stdout).map_err(|_| Error::Config("invalid SID".into()))?;
        let start = (chrono::Local::now() + chrono::Duration::hours(1))
            .with_minute(0)
            .and_then(|v| v.with_second(0))
            .ok_or_else(|| Error::Config("invalid next hour".into()))?
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
        let text = task_xml(&TaskSpec {
            sid: sid.trim(),
            start: &start,
            helper: helper
                .to_str()
                .ok_or_else(|| Error::Config("non-Unicode helper path".into()))?,
            home: home
                .to_str()
                .ok_or_else(|| Error::Config("non-Unicode home".into()))?,
            directory: dir
                .to_str()
                .ok_or_else(|| Error::Config("non-Unicode directory".into()))?,
        })?;
        std::fs::write(&task, text).map_err(|source| Error::Io {
            context: "scheduler XML",
            source,
        })?;
    }
    let result = powershell()?
        .arg("-File")
        .arg(script)
        .arg(mode)
        .arg(task)
        .output()
        .map_err(|source| Error::Io {
            context: "scheduler process",
            source,
        })?;
    if !result.status.success() {
        return Err(Error::Config(format!(
            "scheduler {mode} failed (exit {:?})",
            result.status.code()
        )));
    }
    print!("{}", String::from_utf8_lossy(&result.stdout));
    Ok(())
}
#[cfg(not(windows))]
pub fn execute(_: &str, _: &std::path::Path) -> Result<()> {
    Err(Error::Unsupported("Windows Task Scheduler"))
}
