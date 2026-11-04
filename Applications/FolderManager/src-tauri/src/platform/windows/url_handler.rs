// Hands a link to the default browser or mail app through the shell's URL handler.
use std::process::Command;

pub(crate) fn launcher() -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("rundll32");
    cmd.arg("url.dll,FileProtocolHandler").creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}
