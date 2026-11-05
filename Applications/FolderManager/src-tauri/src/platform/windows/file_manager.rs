// Opens a folder in Explorer.
use std::path::Path;
use std::process::Command;

pub(crate) fn launcher(dir: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer");
    // Explorer falls back to Documents when the path mixes forward slashes.
    cmd.arg(dir.to_string_lossy().replace('/', "\\")).creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW);
    cmd
}

// Opens the parent folder in Explorer with the item selected; /select needs its path raw, as Explorer parses the quotes itself.
pub(crate) fn revealer(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer");
    cmd.raw_arg(format!("/select,\"{}\"", path.to_string_lossy().replace('/', "\\"))).creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW);
    cmd
}
