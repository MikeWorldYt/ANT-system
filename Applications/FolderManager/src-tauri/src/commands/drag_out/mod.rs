// Drags a storage folder out to Explorer or the desktop, offered as a copy or, with Alt, a shortcut.
#[cfg(windows)]
mod com;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde_yaml::Mapping;
use tauri::AppHandle;

use super::run_locked;
use crate::error::{FmError, R};
use crate::mutations::{locate, Target};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store;

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_LINK: u32 = 4;
// Without MOVE the target can never take the folder out of the storage.
const ALLOWED_EFFECTS: u32 = DROPEFFECT_COPY | DROPEFFECT_LINK;

#[tauri::command]
pub async fn start_drag_out(app: AppHandle, target: Vec<String>) -> R<()> {
    let path = run_locked(app.clone(), move |root| drag_plan(root, &tree_store::load(root)?, &target)).await?;
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        if tx.send(drag_folders(&[path])).is_err() {
            eprintln!("The drag finished after its command was dropped");
        }
    })
    .map_err(|e| FmError::Internal { message: format!("Could not start the drag: {e}") })?;
    // The drag runs a modal loop on the UI thread, so the result is awaited off the async runtime.
    let effect = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| FmError::Internal { message: e.to_string() })?
        .map_err(|_| FmError::Internal { message: "The drag ended without a result".to_string() })?;
    effect.map(|_| ())
}

pub(super) fn drag_plan(root: &Path, data: &Mapping, target: &[String]) -> R<PathBuf> {
    let Target { abs, ghost, .. } = locate(root, data, target)?;
    if is_reserved(target) {
        return Err(FmError::Reserved { rel_parts: target.to_vec() });
    }
    if ghost {
        return Err(FmError::GhostNotAllowed { op: "drag out".to_string() });
    }
    if !abs.is_dir() {
        return Err(FmError::NotFound { rel_parts: target.to_vec() });
    }
    Ok(abs)
}

#[cfg(windows)]
use com::drag_folders;

#[cfg(not(windows))]
fn drag_folders(_paths: &[PathBuf]) -> R<u32> {
    Err(FmError::Internal { message: "Drag out is only available on Windows for now".to_string() })
}
