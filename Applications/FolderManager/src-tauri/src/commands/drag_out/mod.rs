// Drags storage folders and files out to Explorer or the desktop, offered only as a move.
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
use crate::ant_core::tree_types::flags_in;
use crate::config::MAIN_FLAG;
use crate::fs_safe::join_rel;
use crate::mutations::{self, locate, Target};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, get_node};

const DROPEFFECT_MOVE: u32 = 2;
// Only a move: Explorer would otherwise default to copy across drives and leave the original behind.
const ALLOWED_EFFECTS: u32 = DROPEFFECT_MOVE;

#[tauri::command]
pub async fn start_drag_out(app: AppHandle, targets: Vec<Vec<String>>) -> R<u32> {
    let paths = run_locked(app.clone(), move |root| {
        let data = tree_store::load(root)?;
        targets.iter().map(|target| drag_plan(root, &data, target)).collect::<R<Vec<PathBuf>>>()
    })
    .await?;
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        if tx.send(drag_folders(&paths)).is_err() {
            eprintln!("The drag finished after its command was dropped");
        }
    })
    .map_err(|e| FmError::Internal { message: format!("Could not start the drag: {e}") })?;
    // The drag runs a modal loop on the UI thread, so the result is awaited off the async runtime.
    let effect = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| FmError::Internal { message: e.to_string() })?
        .map_err(|_| FmError::Internal { message: "The drag ended without a result".to_string() })?;
    effect
}

pub(super) fn drag_plan(root: &Path, data: &Mapping, target: &[String]) -> R<PathBuf> {
    if join_rel(root, target).is_file() {
        return mutations::file_path(root, target);
    }
    let Target { abs, ghost, .. } = locate(root, data, target)?;
    if is_reserved(target) {
        return Err(FmError::Reserved { rel_parts: target.to_vec() });
    }
    if target.len() == 1 || get_node(data, target).map(flags_in).unwrap_or_default().iter().any(|f| f == MAIN_FLAG) {
        return Err(FmError::InvalidPath { rel_parts: target.to_vec(), reason: "main categories cannot be moved".to_string() });
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
