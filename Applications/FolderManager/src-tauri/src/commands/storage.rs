// Commands of the Storage settings page: rename the root folder, undo the last rename and the daily sync task.
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, WebviewWindow};

use super::{off_thread, run_locked};
use crate::app::storage_session::Storage;
use crate::services::rename::{rename_flow, revert_target};
use crate::domain::error::R;
use crate::infra::storage::rename::{self, RenameStatus};
use crate::infra::storage::{location, pointer};
use crate::services::{sync, updater};

pub fn window_title(name: &str) -> String {
    format!("ANT System Manager v.{} ({name})", updater::CURRENT_VERSION)
}

async fn renamed<F>(window: WebviewWindow, stamp: String, target: F) -> R<String>
where
    F: FnOnce(&Path) -> R<String> + Send + 'static,
{
    switched(window, move |storage| rename_flow(storage, &stamp, target)).await
}

// Both a rename and a path change leave the window on a new root, which it then names and remembers.
async fn switched<F>(window: WebviewWindow, flow: F) -> R<String>
where
    F: FnOnce(&Storage) -> R<PathBuf> + Send + 'static,
{
    let app = window.app_handle().clone();
    let root = off_thread(move || flow(&app.state::<Storage>())).await??;
    Ok(adopt(&window, &root))
}

fn adopt(window: &WebviewWindow, root: &Path) -> String {
    if let Err(e) = window.set_title(&window_title(&rename::folder_name(root))) {
        eprintln!("Could not update the window title: {e}");
    }
    pointer::remember(window.app_handle(), root);
    root.to_string_lossy().into_owned()
}

pub fn change_flow(storage: &Storage, text: &str) -> R<PathBuf> {
    let path = location::clean_path(text)?;
    let root = location::storage_root(&path)?;
    let _guard = storage.guard();
    storage.set_root(root.clone());
    storage.set_start(path);
    Ok(root)
}

#[tauri::command]
pub async fn change_storage_path(window: WebviewWindow, path: String) -> R<String> {
    switched(window, move |storage| change_flow(storage, &path)).await
}

#[tauri::command]
pub async fn rename_storage(window: WebviewWindow, new_name: String, stamp: String) -> R<String> {
    renamed(window, stamp, move |_| Ok(new_name)).await
}

#[tauri::command]
pub async fn revert_storage_rename(window: WebviewWindow, stamp: String) -> R<String> {
    renamed(window, stamp, revert_target).await
}

#[tauri::command]
pub async fn storage_rename_status(app: AppHandle) -> R<Option<RenameStatus>> {
    run_locked(app, |root| Ok(rename::status(root))).await
}

#[tauri::command]
pub async fn sync_schedule(app: AppHandle, on: bool) -> R<()> {
    run_locked(app, move |root| sync::apply(on, root)).await
}
