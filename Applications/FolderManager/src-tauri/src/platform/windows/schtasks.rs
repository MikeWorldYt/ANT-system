// The Windows user and the schtasks.exe process the scheduled tasks are managed through.
use super::system::system_tool;
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
    let exe = system_tool("schtasks.exe").map_err(|e| FmError::Internal { message: format!("Could not start schtasks: {e}") })?;
    std::process::Command::new(exe)
        .args(args)
        .creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW)
        .output()
        .map_err(|e| FmError::Internal { message: format!("Could not start schtasks: {e}") })
}
