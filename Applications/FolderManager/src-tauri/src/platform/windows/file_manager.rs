// Opens a folder in Explorer.
use std::path::Path;
use std::process::Command;

pub(crate) fn launcher(dir: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer");
    // Explorer falls back to Documents when the path mixes forward slashes.
    cmd.arg(dir.to_string_lossy().replace('/', "\\")).creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}
