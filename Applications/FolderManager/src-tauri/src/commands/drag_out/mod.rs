// Drags storage folders and files out to Explorer or the desktop, offered only as a move.
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde_yaml::Mapping;
use tauri::AppHandle;

use super::{off_thread, run_locked};
use crate::domain::error::{FmError, R};
use crate::domain::tree_types::flags_in;
use crate::domain::config::MAIN_FLAG;
use crate::infra::fs_safe::join_rel;
use crate::services::mutations::{self, locate, Target};
use crate::platform::drag_out::drag_folders;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::{self, get_node};

#[tauri::command]
pub async fn start_drag_out(app: AppHandle, targets: Vec<Vec<String>>) -> R<u32> {
    let paths = run_locked(app.clone(), move |root| {
        let data = tree_store::load_paths(root, &targets)?;
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
    let effect = off_thread(move || rx.recv())
        .await?
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
