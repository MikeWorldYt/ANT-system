// When an automatic backup is due, and the Windows scheduled task that runs it while the app is closed.
use std::path::Path;

use crate::error::{FmError, R};

const TASK_NAME: &str = "ANT System Manager Backup";
const DAY: u64 = 86_400;

pub fn period_secs(frequency: &str) -> Option<u64> {
    match frequency {
        "day" => Some(DAY),
        "week" => Some(7 * DAY),
        "month" => Some(30 * DAY),
        _ => None,
    }
}

// A clock set backwards makes `last` lie in the future, which waits instead of backing up on every launch.
pub fn is_due(auto: bool, frequency: &str, last: Option<u64>, now: u64) -> bool {
    match (auto, period_secs(frequency), last) {
        (true, Some(_), None) => true,
        (true, Some(period), Some(last)) => now.saturating_sub(last) >= period,
        _ => false,
    }
}

pub fn schtasks_args(on: bool, frequency: &str, exe: &Path, root: &Path) -> R<Vec<String>> {
    if !on {
        return Ok(["/Delete", "/F", "/TN", TASK_NAME].map(String::from).to_vec());
    }
    let schedule = match frequency {
        "day" => "DAILY",
        "week" => "WEEKLY",
        "month" => "MONTHLY",
        _ => return Err(FmError::Internal { message: format!("Unknown backup frequency {frequency}") }),
    };
    let run = format!("\"{}\" --backup \"{}\"", exe.display(), root.display());
    Ok(["/Create", "/F", "/SC", schedule, "/TN", TASK_NAME, "/TR", &run, "/ST", "12:00"].map(String::from).to_vec())
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
pub fn apply(on: bool, frequency: &str, root: &Path) -> R<()> {
    let exe = std::env::current_exe().map_err(|e| FmError::Internal { message: format!("Could not find the app: {e}") })?;
    let args = schtasks_args(on, frequency, &exe, root)?;
    // Querying first makes a missing task a success without parsing localized error text.
    if !on && !schtasks(&["/Query".into(), "/TN".into(), TASK_NAME.into()])?.status.success() {
        return Ok(());
    }
    let out = schtasks(&args)?;
    if out.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(FmError::Internal { message: format!("Could not update the scheduled backup: {detail}") })
}

#[cfg(not(windows))]
pub fn apply(on: bool, frequency: &str, root: &Path) -> R<()> {
    schtasks_args(on, frequency, Path::new(""), root)?;
    Err(FmError::Internal { message: "Scheduled backups are only available on Windows for now".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_backup_is_due_once_the_period_has_passed() {
        assert!(is_due(true, "day", None, 100));
        assert!(!is_due(false, "day", None, 100));
        assert!(!is_due(true, "year", None, 100));
        assert!(!is_due(true, "day", Some(1000), 1000 + DAY - 1));
        assert!(is_due(true, "day", Some(1000), 1000 + DAY));
        assert!(!is_due(true, "week", Some(0), 6 * DAY));
        assert!(is_due(true, "month", Some(0), 30 * DAY));
        assert!(!is_due(true, "day", Some(5000), 10));
    }

    #[test]
    fn schtasks_arguments_follow_the_frequency() {
        let args = schtasks_args(true, "week", Path::new("C:/App/fm.exe"), Path::new("D:/My Storage")).unwrap();
        let expected = ["/Create", "/F", "/SC", "WEEKLY", "/TN", TASK_NAME, "/TR"];
        assert_eq!(args[..7], expected);
        assert_eq!(args[7], "\"C:/App/fm.exe\" --backup \"D:/My Storage\"");
        assert_eq!(args[8..], ["/ST", "12:00"]);
        assert_eq!(schtasks_args(true, "day", Path::new("a"), Path::new("b")).unwrap()[3], "DAILY");
        assert_eq!(schtasks_args(true, "month", Path::new("a"), Path::new("b")).unwrap()[3], "MONTHLY");
        assert!(schtasks_args(true, "hour", Path::new("a"), Path::new("b")).is_err());
        assert_eq!(schtasks_args(false, "", Path::new("a"), Path::new("b")).unwrap(), ["/Delete", "/F", "/TN", TASK_NAME]);
    }
}
