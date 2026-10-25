// Windows scheduled tasks that rerun the app headless (`--backup`, `--sync`) while it is closed.
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
