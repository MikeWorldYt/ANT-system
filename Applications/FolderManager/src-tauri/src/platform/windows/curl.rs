// The curl.exe Windows ships, started without a console window.
use std::process::Command;

// An absolute path keeps a curl.exe planted next to the app or on PATH from running.
pub(crate) fn curl() -> Command {
    use std::os::windows::process::CommandExt;
    let exe = std::env::var_os("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join(r"System32\curl.exe"))
        .unwrap_or_else(|| std::path::PathBuf::from("curl.exe"));
    let mut cmd = Command::new(exe);
    cmd.creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}
