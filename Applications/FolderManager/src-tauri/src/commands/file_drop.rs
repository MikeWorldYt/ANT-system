// Drop and paste commands: files from Explorer are placed under the storage lock, moved or copied as Explorer would.
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::clipboard::{self, Paste};
use super::edit::with_search_index;
use super::run_locked;
use crate::error::R;
use crate::mutations::{self, DropOutcome};

#[tauri::command]
pub async fn drop_files(app: AppHandle, target: Vec<String>, paths: Vec<String>) -> R<DropOutcome> {
    let copy = ctrl_held();
    run_locked(app, move |root| {
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        place(root, &target, &paths, copy)
    })
    .await
}

// Paste follows the clipboard's cut or copy, never the Ctrl key.
#[tauri::command]
pub async fn paste_files(app: AppHandle, target: Vec<String>) -> R<DropOutcome> {
    run_locked(app, move |root| {
        let Paste { paths, copy } = clipboard::paste_plan(clipboard::read()?)?;
        place(root, &target, &paths, copy)
    })
    .await
}

fn place(root: &Path, target: &[String], paths: &[PathBuf], copy: bool) -> R<DropOutcome> {
    let DropOutcome { mutation, placed } = mutations::drop_files(root, target, paths, copy)?;
    Ok(DropOutcome { mutation: with_search_index(root, Ok(mutation))?, placed })
}

// Tauri's drop event carries no modifier keys, so the Ctrl state is read when the command arrives.
#[cfg(windows)]
fn ctrl_held() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    // SAFETY: GetAsyncKeyState only reads the keyboard state; the high bit is set while the key is down.
    unsafe { GetAsyncKeyState(i32::from(VK_CONTROL)) < 0 }
}

#[cfg(not(windows))]
fn ctrl_held() -> bool {
    false
}
