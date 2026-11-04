// Windows scheduled tasks that rerun the app headless (`--backup`, `--sync`, `--watch`) while it is closed.
use std::path::Path;

use crate::error::{FmError, R};

pub struct Task {
    pub name: &'static str,
    pub flag: &'static str,
    pub start: &'static str,
    pub what: &'static str,
}

pub fn args(task: &Task, schedule: Option<&str>, exe: &Path, root: &Path) -> Vec<String> {
    let Some(schedule) = schedule else {
        return ["/Delete", "/F", "/TN", task.name].map(String::from).to_vec();
    };
    let run = format!("\"{}\" {} \"{}\"", exe.display(), task.flag, root.display());
    ["/Create", "/F", "/SC", schedule, "/TN", task.name, "/TR", &run, "/ST", task.start].map(String::from).to_vec()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// schtasks /Create cannot lift the default 72 h execution limit, so a logon task needs its XML.
pub fn logon_xml(task: &Task, user: &str, exe: &Path, root: &Path) -> String {
    let user = escape(user);
    let command = escape(&exe.display().to_string());
    let arguments = escape(&format!("{} \"{}\"", task.flag, root.display()));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Hidden>true</Hidden>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
      <Arguments>{arguments}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

#[cfg(windows)]
fn current_user() -> R<String> {
    let name = std::env::var("USERNAME").map_err(|e| FmError::Internal { message: format!("Could not read the Windows user name: {e}") })?;
    Ok(match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{name}"),
        _ => name,
    })
}

#[cfg(windows)]
pub fn apply_logon(task: &Task, on: bool, root: &Path) -> R<()> {
    if !on {
        return apply(task, None, root);
    }
    let exe = std::env::current_exe().map_err(|e| FmError::Internal { message: format!("Could not find the app: {e}") })?;
    let user = current_user()?;
    // Task Scheduler reads a UTF-16 file with a byte order mark.
    let bytes: Vec<u8> = std::iter::once(0xFEFF_u16).chain(logon_xml(task, &user, &exe, root).encode_utf16()).flat_map(u16::to_le_bytes).collect();
    let file = std::env::temp_dir().join(format!("ant-watch-task-{}.xml", std::process::id()));
    std::fs::write(&file, bytes).map_err(|e| FmError::Internal { message: format!("Could not write the task file: {e}") })?;
    let args = ["/Create".to_string(), "/F".into(), "/TN".into(), task.name.into(), "/XML".into(), file.display().to_string()];
    let out = schtasks(&args);
    if let Err(e) = std::fs::remove_file(&file) {
        eprintln!("Could not remove the task file: {e}");
    }
    let out = out?;
    if out.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(FmError::Internal { message: format!("Could not update the scheduled {}: {detail}", task.what) })
}

#[cfg(not(windows))]
pub fn apply_logon(task: &Task, _on: bool, _root: &Path) -> R<()> {
    Err(FmError::Internal { message: format!("Scheduled {} is only available on Windows for now", task.what) })
}

#[cfg(windows)]
fn schtasks(args: &[String]) -> R<std::process::Output> {
    use std::os::windows::process::CommandExt;
    // An absolute path keeps a schtasks.exe planted next to the app or on PATH from running.
    let exe = std::env::var_os("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join(r"System32\schtasks.exe"))
        .unwrap_or_else(|| "schtasks.exe".into());
    std::process::Command::new(exe)
        .args(args)
        .creation_flags(crate::fs_safe::CREATE_NO_WINDOW)
        .output()
        .map_err(|e| FmError::Internal { message: format!("Could not start schtasks: {e}") })
}

#[cfg(windows)]
pub fn apply(task: &Task, schedule: Option<&str>, root: &Path) -> R<()> {
    let exe = std::env::current_exe().map_err(|e| FmError::Internal { message: format!("Could not find the app: {e}") })?;
    let args = args(task, schedule, &exe, root);
    // Querying first makes a missing task a success without parsing localized error text.
    if schedule.is_none() && !schtasks(&["/Query".into(), "/TN".into(), task.name.into()])?.status.success() {
        return Ok(());
    }
    let out = schtasks(&args)?;
    if out.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(FmError::Internal { message: format!("Could not update the scheduled {}: {detail}", task.what) })
}

#[cfg(not(windows))]
pub fn apply(task: &Task, _schedule: Option<&str>, _root: &Path) -> R<()> {
    Err(FmError::Internal { message: format!("Scheduled {} is only available on Windows for now", task.what) })
}
