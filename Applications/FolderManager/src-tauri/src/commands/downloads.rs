// Commands for the system Downloads folder; they take only names below it and never touch the storage lock. Only a batch renames or deletes there.
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::off_thread;
use super::open::{launcher, revealer};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::validate_component;
use crate::platform::{clipboard, recycle::send_to_recycle_bin};
use crate::services::downloads;
use crate::services::preview;
use crate::services::tree::tree_node::TreeNode;

// The automations take the Downloads folder's paths as the rest of its commands do: its root name first, then the names below it.
pub(super) fn below_downloads(folder: &[String]) -> Option<&[String]> {
    folder.split_first().filter(|(first, _)| *first == downloads::ROOT).map(|(_, rest)| rest)
}

pub(super) fn base(app: &AppHandle) -> R<PathBuf> {
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

#[tauri::command]
pub async fn reveal_download(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    off_thread(move || {
        let path = downloads::path_of(&base(&app)?, &rel_parts)?;
        revealer(&path).spawn().map(|_| ()).map_err(|e| FmError::Internal { message: format!("Could not open the folder: {e}") })
    })
    .await?
}

// A name already taken is refused, so a rename never replaces another download.
#[tauri::command]
pub async fn rename_download(app: AppHandle, rel_parts: Vec<String>, new_name: String) -> R<()> {
    off_thread(move || {
        let from = downloads::path_of(&base(&app)?, &rel_parts)?;
        let name = new_name.trim();
        validate_component(name)?;
        let to = from.with_file_name(name);
        let same = from.to_string_lossy().eq_ignore_ascii_case(&to.to_string_lossy());
        if to.exists() && !same {
            return Err(FmError::InvalidName { name: name.to_string(), reason: "a file with this name is already there".to_string() });
        }
        std::fs::rename(&from, &to).map_err(|e| FmError::Internal { message: format!("Could not rename {}: {e}", from.display()) })
    })
    .await?
}

// Files dropped from another app are copied in, never moved: the source belongs to that app. A name already taken is skipped.
#[tauri::command]
pub async fn drop_into_downloads(app: AppHandle, rel_parts: Vec<String>, paths: Vec<PathBuf>) -> R<Vec<String>> {
    off_thread(move || {
        let dir = downloads::path_of(&base(&app)?, &rel_parts)?;
        let mut added = Vec::new();
        for path in paths.iter().filter(|p| p.is_file()) {
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            let to = dir.join(&name);
            if to.exists() {
                continue;
            }
            std::fs::copy(path, &to).map_err(|e| FmError::Internal { message: format!("Could not copy {}: {e}", path.display()) })?;
            added.push(name);
        }
        Ok(added)
    })
    .await?
}

#[tauri::command]
pub async fn delete_downloads(app: AppHandle, targets: Vec<Vec<String>>) -> R<()> {
    off_thread(move || {
        let base = base(&app)?;
        for rel in &targets {
            let path = downloads::path_of(&base, rel)?;
            super::transfer::recycle_tracked(&path, send_to_recycle_bin).map_err(|e| FmError::Internal { message: format!("Could not delete {}: {e}", path.display()) })?;
        }
        Ok(())
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
