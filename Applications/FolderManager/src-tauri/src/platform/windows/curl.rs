// The curl.exe Windows ships, started without a console window.
use std::process::Command;

use super::system::system_command;

pub(crate) fn curl() -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = system_command("curl.exe");
    cmd.creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW);
    cmd
}
