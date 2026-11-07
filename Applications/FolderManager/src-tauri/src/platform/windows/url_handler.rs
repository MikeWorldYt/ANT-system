// Hands a link to the default browser or mail app through the shell's URL handler.
use std::process::Command;

use super::system::system_command;

pub(crate) fn launcher() -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = system_command("rundll32.exe");
    cmd.arg("url.dll,FileProtocolHandler").creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW);
    cmd
}
