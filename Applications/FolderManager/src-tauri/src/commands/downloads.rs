// Commands for the system Downloads folder and the other native folders; they take a root marker and only names below it, and never touch the storage lock.
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use super::off_thread;
use super::open::{launcher, revealer};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::validate_component;
use crate::platform::clipboard::Paste;
use crate::platform::{clipboard, recycle::send_to_recycle_bin};
use crate::services::downloads;
use crate::services::mutations::copy_whole;
use crate::services::preview;
use crate::services::tree::tree_node::TreeNode;

// The automations take the Downloads folder's paths as the rest of its commands do: its root name first, then the names below it.
pub(super) fn below_downloads(folder: &[String]) -> Option<&[String]> {
    folder.split_first().filter(|(first, _)| *first == downloads::ROOT).map(|(_, rest)| rest)
}

pub(super) fn base(app: &AppHandle) -> R<PathBuf> {
    base_of(app, downloads::ROOT)
}

// Each native folder resolves through Tauri's path resolver, as Downloads always has.
pub fn base_of(app: &AppHandle, root: &str) -> R<PathBuf> {
    let p = app.path();
    let dir = match root {
        downloads::ROOT => p.download_dir(),
        "::Documents" => p.document_dir(),
        "::Pictures" => p.picture_dir(),
        "::Music" => p.audio_dir(),
        "::Videos" => p.video_dir(),
        "::Desktop" => p.desktop_dir(),
        _ => return Err(FmError::NotFound { rel_parts: vec![root.to_string()] }),
    };
    dir.map_err(|e| FmError::Internal { message: format!("Could not find the {} folder: {e}", root.trim_start_matches(':')) })
}

// A whole path inside a native folder, root marker first; None when it belongs to the storage.
pub(super) fn system_path(app: &AppHandle, parts: &[String]) -> Option<R<PathBuf>> {
    downloads::split_root(parts).map(|(root, rest)| downloads::path_of(&base_of(app, root)?, rest))
}

// The calls name their folder; one that names none is Downloads, as every call was before the other folders.
fn root_of(app: &AppHandle, root: Option<String>) -> R<PathBuf> {
    base_of(app, root.as_deref().unwrap_or(downloads::ROOT))
}

// The other native folders this machine has, so the root lists only those.
#[tauri::command]
pub async fn system_folders(app: AppHandle) -> R<Vec<String>> {
    off_thread(move || downloads::SYSTEM_ROOTS.iter().filter(|r| base_of(&app, r).is_ok_and(|p| p.is_dir())).map(|r| r.to_string()).collect()).await
}

#[tauri::command]
pub async fn list_downloads(app: AppHandle, root: Option<String>, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    off_thread(move || {
        let root = root.unwrap_or_else(|| downloads::ROOT.to_string());
        downloads::list(&base_of(&app, &root)?, &root, &rel_parts)
    })
    .await?
}

// Files open with the system's default app, folders in the file manager.
#[tauri::command]
pub async fn open_download(app: AppHandle, root: Option<String>, rel_parts: Vec<String>) -> R<()> {
    off_thread(move || {
        let path = downloads::path_of(&root_of(&app, root)?, &rel_parts)?;
        if !path.exists() {
            return Err(FmError::NotFound { rel_parts });
        }
        launcher(&path).spawn().map(|_| ()).map_err(|e| FmError::Internal { message: format!("Could not open the item: {e}") })
    })
    .await?
}

#[tauri::command]
pub async fn reveal_download(app: AppHandle, root: Option<String>, rel_parts: Vec<String>) -> R<()> {
    off_thread(move || {
        let path = downloads::path_of(&root_of(&app, root)?, &rel_parts)?;
        revealer(&path).spawn().map(|_| ()).map_err(|e| FmError::Internal { message: format!("Could not open the folder: {e}") })
    })
    .await?
}

// A name already taken is refused, so a rename never replaces another download.
#[tauri::command]
pub async fn rename_download(app: AppHandle, root: Option<String>, rel_parts: Vec<String>, new_name: String) -> R<()> {
    off_thread(move || {
        let from = downloads::path_of(&root_of(&app, root)?, &rel_parts)?;
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
pub async fn drop_into_downloads(app: AppHandle, root: Option<String>, rel_parts: Vec<String>, paths: Vec<PathBuf>) -> R<Vec<String>> {
    off_thread(move || {
        let root = root.unwrap_or_else(|| downloads::ROOT.to_string());
        let dir = downloads::path_of(&base_of(&app, &root)?, &rel_parts)?;
        let _progress = super::transfer::track(&app, &paths, root.trim_start_matches(':'), true);
        let mut added = Vec::new();
        for path in paths.iter().filter(|p| p.is_file()) {
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            let to = dir.join(&name);
            if to.exists() {
                continue;
            }
            crate::services::transfer::copy(path, &to).map_err(|e| {
                // A cancelled or failed copy leaves no half-written file behind.
                if let Err(r) = std::fs::remove_file(&to) {
                    eprintln!("Could not remove the partial copy {}: {r}", to.display());
                }
                FmError::Internal { message: format!("Could not copy {}: {e}", path.display()) }
            })?;
            added.push(name);
        }
        Ok(added)
    })
    .await?
}

// The first of "name", "name (2)"... not taken in the folder; a file keeps its extension after the number.
fn free_name(dir: &Path, name: &str, file: bool) -> String {
    let (stem, ext) = match name.rfind('.').filter(|&i| file && i > 0) {
        Some(i) => name.split_at(i),
        None => (name, ""),
    };
    (1..).map(|n| if n == 1 { name.to_string() } else { format!("{stem} ({n}){ext}") }).find(|c| !dir.join(c).exists()).unwrap_or_default()
}

// Never overwrites: a name already taken gets a number, so the new folder is always a fresh one.
#[tauri::command]
pub async fn create_download_folder(app: AppHandle, root: Option<String>, rel_parts: Vec<String>, name: String) -> R<String> {
    off_thread(move || {
        let dir = downloads::path_of(&root_of(&app, root)?, &rel_parts)?;
        validate_component(&name)?;
        let made = free_name(&dir, &name, false);
        std::fs::create_dir(dir.join(&made)).map_err(|e| FmError::Internal { message: format!("Could not create the folder {made}: {e}") })?;
        Ok(made)
    })
    .await?
}

// What the clipboard holds is copied, or moved after a cut; a name already taken gets a number, so nothing is replaced.
#[tauri::command]
pub async fn paste_into_downloads(app: AppHandle, root: Option<String>, rel_parts: Vec<String>) -> R<Vec<String>> {
    off_thread(move || {
        let root = root.unwrap_or_else(|| downloads::ROOT.to_string());
        let dir = downloads::path_of(&base_of(&app, &root)?, &rel_parts)?;
        let Paste { paths, copy } = clipboard::paste_plan(clipboard::read()?)?;
        let _progress = super::transfer::track(&app, &paths, root.trim_start_matches(':'), copy);
        let mut added = Vec::new();
        for from in &paths {
            let Some(name) = from.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
            if dir.starts_with(from) {
                return Err(FmError::InvalidName { name, reason: "a folder cannot be pasted into itself".to_string() });
            }
            if !copy && from.parent() == Some(dir.as_path()) {
                continue;
            }
            let made = free_name(&dir, &name, from.is_file());
            place_pasted(from, &dir.join(&made), copy)?;
            added.push(made);
        }
        Ok(added)
    })
    .await?
}

// A cut tries a plain rename first; across drives it becomes a copy whose original then goes to the Recycle Bin.
fn place_pasted(from: &Path, to: &Path, copy: bool) -> R<()> {
    if !copy && std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    let copied = if from.is_dir() {
        copy_whole(from, to)
    } else {
        crate::services::transfer::copy(from, to).map(|_| ()).map_err(|e| {
            // A cancelled or failed copy leaves no half-written file behind.
            if let Err(r) = std::fs::remove_file(to) {
                eprintln!("Could not remove the partial copy {}: {r}", to.display());
            }
            e.to_string()
        })
    };
    copied.map_err(|e| FmError::Internal { message: format!("Could not copy {}: {e}", from.display()) })?;
    if !copy {
        send_to_recycle_bin(from).map_err(|e| FmError::Internal { message: format!("Copied {}, but the original could not be removed: {e}", from.display()) })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_downloads(app: AppHandle, root: Option<String>, targets: Vec<Vec<String>>) -> R<()> {
    off_thread(move || {
        let base = root_of(&app, root)?;
        for rel in &targets {
            let path = downloads::path_of(&base, rel)?;
            super::transfer::recycle_tracked(&path, send_to_recycle_bin).map_err(|e| FmError::Internal { message: format!("Could not delete {}: {e}", path.display()) })?;
        }
        Ok(())
    })
    .await?
}

// A cut pasted into the storage moves the files there; the originals go to the Recycle Bin, as with a drag.
#[tauri::command]
pub async fn copy_downloads(app: AppHandle, root: Option<String>, targets: Vec<Vec<String>>, cut: bool) -> R<()> {
    off_thread(move || {
        let base = root_of(&app, root)?;
        let paths = targets.iter().map(|rel| downloads::path_of(&base, rel)).collect::<R<Vec<PathBuf>>>()?;
        clipboard::write(&paths, clipboard::drop_effect(cut))
    })
    .await?
}

#[tauri::command]
pub async fn download_image(app: AppHandle, root: Option<String>, rel_parts: Vec<String>, large: bool, preview: Option<bool>) -> R<Option<String>> {
    off_thread(move || {
        let path = downloads::path_of(&root_of(&app, root)?, &rel_parts)?;
        if !path.is_file() {
            return Ok(None);
        }
        if preview == Some(true) { preview::preview_image(&path) } else { preview::file_image(&path, large) }
    })
    .await?
}
