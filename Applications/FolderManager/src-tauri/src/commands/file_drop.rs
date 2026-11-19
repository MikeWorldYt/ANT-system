// Drop and paste commands: files from Explorer are placed under the storage lock, moved or copied as Explorer would.
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::services::index::after_mutation::with_search_index;
use super::run_locked;
use super::transfer::track;
use crate::domain::error::R;
use crate::platform::clipboard::{self, Paste};
use crate::platform::recycle::send_to_recycle_bin;
use crate::services::mutations::{self, DropOutcome};
use crate::services::ops_log;
#[cfg(windows)]
use crate::platform::windows::input::ctrl_held;

// `replace` names the worknodes the user chose to swap for the one already in the storage.
#[tauri::command]
pub async fn drop_files(app: AppHandle, target: Vec<String>, paths: Vec<String>, replace: Vec<String>) -> R<DropOutcome> {
    let copy = ctrl_held();
    let sender = app.clone();
    run_locked(app, move |root| {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let replace: Vec<PathBuf> = replace.into_iter().map(PathBuf::from).collect();
        let _progress = track(&sender, &paths, &place_name(root, &target));
        place(root, &target, &paths, copy, &replace)
    })
    .await
}

// Paste follows the clipboard's cut or copy, never the Ctrl key.
#[tauri::command]
pub async fn paste_files(app: AppHandle, target: Vec<String>) -> R<DropOutcome> {
    let sender = app.clone();
    run_locked(app, move |root| {
        let Paste { paths, copy } = clipboard::paste_plan(clipboard::read()?)?;
        let _progress = track(&sender, &paths, &place_name(root, &target));
        place(root, &target, &paths, copy, &[])
    })
    .await
}

fn place(root: &Path, target: &[String], paths: &[PathBuf], copy: bool, replace: &[PathBuf]) -> R<DropOutcome> {
    let done = mutations::drop_files(root, target, paths, copy, replace, &send_to_recycle_bin);
    ops_log::outcome(root, if copy { "copy in" } else { "move in" }, &listed(root, paths, target), &done);
    indexed(root, done?)
}

fn listed(root: &Path, paths: &[PathBuf], target: &[String]) -> String {
    let names: Vec<String> = paths.iter().map(|p| ops_log::rel(root, p)).collect();
    format!("{} → {}", names.join(", "), target.join("/"))
}

pub(super) fn place_name(root: &Path, target: &[String]) -> String {
    target.last().cloned().unwrap_or_else(|| root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

fn indexed(root: &Path, DropOutcome { mutation, placed, conflicts }: DropOutcome) -> R<DropOutcome> {
    Ok(DropOutcome { mutation: with_search_index(root, Ok(mutation))?, placed, conflicts })
}

#[cfg(not(windows))]
fn ctrl_held() -> bool {
    false
}

// Files already in the storage move (or copy with Ctrl) into a folder, through the same rules as files dropped from Explorer.
#[tauri::command]
pub async fn move_files_in(app: AppHandle, sources: Vec<Vec<String>>, target: Vec<String>, copy: bool) -> R<DropOutcome> {
    let sender = app.clone();
    run_locked(app, move |root| {
        let paths = sources.iter().map(|rel| mutations::source_path(root, rel)).collect::<R<Vec<PathBuf>>>()?;
        let _progress = track(&sender, &paths, &place_name(root, &target));
        let done = mutations::move_in(root, &target, &paths, copy, &send_to_recycle_bin);
        ops_log::outcome(root, if copy { "copy files" } else { "move files" }, &listed(root, &paths, &target), &done);
        indexed(root, done?)
    })
    .await
}
