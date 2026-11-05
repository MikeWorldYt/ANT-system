// Opens a folder of the storage in the file manager the operating system uses by default.
#[cfg(not(windows))]
use std::path::Path;
#[cfg(not(windows))]
use std::process::Command;

use tauri::AppHandle;

use super::run_locked;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{join_rel, validate_component};
use crate::services::tree::review_links;
use crate::services::tree::tree_store;

// Each system has its own launcher for its default file manager.
#[cfg(windows)]
pub(super) use crate::platform::windows::file_manager::{launcher, revealer};

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

// Other systems have no select flag, so the parent folder opens instead.
#[cfg(not(windows))]
fn revealer(path: &Path) -> Command {
    launcher(path.parent().unwrap_or(path))
}

// Opens the folder that holds the item, with the item selected, as "here it is".
#[tauri::command]
pub async fn reveal_node(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let path = join_rel(root, &rel_parts);
        if !path.exists() {
            return Err(FmError::NotFound { rel_parts });
        }
        revealer(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| FmError::Internal { message: format!("Could not show the folder: {e}") })
    })
    .await
}

#[tauri::command]
pub async fn open_node(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let dir = match review_links::link_folder(root, &tree_store::load(root)?, &rel_parts)? {
            Some(dir) => dir,
            None => join_rel(root, &rel_parts),
        };
        // Files open with the system's default app, folders in the file manager.
        if !dir.exists() {
            return Err(FmError::NotFound { rel_parts });
        }
        launcher(&dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| FmError::Internal { message: format!("Could not open the folder: {e}") })
    })
    .await
}
