// Marks the .ant folder hidden, as a dot-folder is not hidden on Windows.
use std::path::Path;

// A failed attrib only leaves the folder visible, so it is reported on stderr and the save goes on.
pub(crate) fn hide(dir: &Path) {
    use std::os::windows::process::CommandExt;
    let hidden = std::process::Command::new("attrib").arg("+h").arg(dir).creation_flags(crate::platform::windows::powershell::CREATE_NO_WINDOW).status();
    match hidden {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("Could not hide {}: attrib exited with {status}", dir.display()),
        Err(e) => eprintln!("Could not hide {}: {e}", dir.display()),
    }
}
