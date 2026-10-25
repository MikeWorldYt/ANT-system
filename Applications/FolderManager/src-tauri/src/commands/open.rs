// Opens a folder of the storage in the file manager the operating system uses by default.
use std::path::Path;
use std::process::Command;

use tauri::AppHandle;

use super::run_locked;
use crate::error::{FmError, R};
use crate::fs_safe::{join_rel, validate_component};

// Each system has its own launcher for its default file manager.
#[cfg(windows)]
pub(super) fn launcher(dir: &Path) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("explorer");
    // Explorer falls back to Documents when the path mixes forward slashes.
    cmd.arg(dir.to_string_lossy().replace('/', "\\")).creation_flags(crate::fs_safe::CREATE_NO_WINDOW);
    cmd
}

#[cfg(target_os = "macos")]
pub(super) fn launcher(dir: &Path) -> Command {
    let mut cmd = Command::new("open");
    cmd.arg(dir);
    cmd
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(super) fn launcher(dir: &Path) -> Command {
    let mut cmd = Command::new("xdg-open");
    cmd.arg(dir);
    cmd
}

#[tauri::command]
pub async fn open_node(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let dir = join_rel(root, &rel_parts);
        if !dir.is_dir() {
            return Err(FmError::NotFound { rel_parts });
        }
        launcher(&dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| FmError::Internal { message: format!("Could not open the folder: {e}") })
    })
    .await
}
