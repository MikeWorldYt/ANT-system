// Read-only commands for the system Downloads folder; they take only names below it and never touch the storage lock.
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::off_thread;
use super::open::launcher;
use crate::domain::error::{FmError, R};
use crate::platform::clipboard;
use crate::services::downloads;
use crate::services::preview;
use crate::services::tree::tree_node::TreeNode;

fn base(app: &AppHandle) -> R<PathBuf> {
    app.path().download_dir().map_err(|e| FmError::Internal { message: format!("Could not find the Downloads folder: {e}") })
}

#[tauri::command]
pub async fn list_downloads(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    off_thread(move || downloads::list(&base(&app)?, &rel_parts)).await?
}

// Files open with the system's default app, folders in the file manager.
#[tauri::command]
pub async fn open_download(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    off_thread(move || {
        let path = downloads::path_of(&base(&app)?, &rel_parts)?;
        if !path.exists() {
            return Err(FmError::NotFound { rel_parts });
        }
        launcher(&path).spawn().map(|_| ()).map_err(|e| FmError::Internal { message: format!("Could not open the item: {e}") })
    })
    .await?
}

// Copy only: Downloads is not ours to cut from.
#[tauri::command]
pub async fn copy_downloads(app: AppHandle, targets: Vec<Vec<String>>) -> R<()> {
    off_thread(move || {
        let base = base(&app)?;
        let paths = targets.iter().map(|rel| downloads::path_of(&base, rel)).collect::<R<Vec<PathBuf>>>()?;
        clipboard::write(&paths, clipboard::drop_effect(false))
    })
    .await?
}

#[tauri::command]
pub async fn download_image(app: AppHandle, rel_parts: Vec<String>, large: bool, preview: Option<bool>) -> R<Option<String>> {
    off_thread(move || {
        let path = downloads::path_of(&base(&app)?, &rel_parts)?;
        if !path.is_file() {
            return Ok(None);
        }
        if preview == Some(true) { preview::preview_image(&path) } else { preview::file_image(&path, large) }
    })
    .await?
}
