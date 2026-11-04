// The Windows user and the schtasks.exe process the scheduled tasks are managed through.
use crate::domain::error::{FmError, R};

pub(crate) fn current_user() -> R<String> {
    let name = std::env::var("USERNAME").map_err(|e| FmError::Internal { message: format!("Could not read the Windows user name: {e}") })?;
    Ok(match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{name}"),
        _ => name,
    })
}

pub(crate) fn schtasks(args: &[String]) -> R<std::process::Output> {
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
