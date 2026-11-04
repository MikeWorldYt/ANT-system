// Drop and paste commands: files from Explorer are placed under the storage lock, moved or copied as Explorer would.
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::clipboard::{self, Paste};
use super::edit::with_search_index;
use super::run_locked;
use crate::error::R;
use crate::mutations::{self, DropOutcome};
#[cfg(windows)]
use crate::platform::windows::input::ctrl_held;

// `replace` names the workboards the user chose to swap for the one already in the storage.
#[tauri::command]
pub async fn drop_files(app: AppHandle, target: Vec<String>, paths: Vec<String>, replace: Vec<String>) -> R<DropOutcome> {
    let copy = ctrl_held();
    run_locked(app, move |root| {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let replace: Vec<PathBuf> = replace.into_iter().map(PathBuf::from).collect();
        place(root, &target, &paths, copy, &replace)
    })
    .await
}

// Paste follows the clipboard's cut or copy, never the Ctrl key.
#[tauri::command]
pub async fn paste_files(app: AppHandle, target: Vec<String>) -> R<DropOutcome> {
    run_locked(app, move |root| {
        let Paste { paths, copy } = clipboard::paste_plan(clipboard::read()?)?;
        place(root, &target, &paths, copy, &[])
    })
    .await
}

fn place(root: &Path, target: &[String], paths: &[PathBuf], copy: bool, replace: &[PathBuf]) -> R<DropOutcome> {
    let DropOutcome { mutation, placed, conflicts } = mutations::drop_files(root, target, paths, copy, replace, &mutations::send_to_recycle_bin)?;
    Ok(DropOutcome { mutation: with_search_index(root, Ok(mutation))?, placed, conflicts })
}

#[cfg(not(windows))]
fn ctrl_held() -> bool {
    false
}

// Files already in the storage move (or copy with Ctrl) into a folder, through the same rules as files dropped from Explorer.
#[tauri::command]
pub async fn move_files_in(app: AppHandle, sources: Vec<Vec<String>>, target: Vec<String>, copy: bool) -> R<DropOutcome> {
    run_locked(app, move |root| {
        let paths = sources.iter().map(|rel| mutations::file_path(root, rel)).collect::<R<Vec<PathBuf>>>()?;
        place(root, &target, &paths, copy, &[])
    })
    .await
}
