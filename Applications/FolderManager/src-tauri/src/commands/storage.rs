// Commands of the Storage settings page: rename the root folder, undo the last rename and the daily sync task.
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, WebviewWindow};

use super::run_locked;
use crate::app::storage_session::Storage;
use crate::services::rename::{rename_flow, revert_target};
use crate::domain::error::{FmError, R};
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
    let app = window.app_handle().clone();
    let new_root = tauri::async_runtime::spawn_blocking(move || rename_flow(&app.state::<Storage>(), &stamp, target))
        .await
        .map_err(|e| FmError::Internal { message: e.to_string() })??;
    Ok(adopt(&window, &new_root))
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
    let _guard = storage.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    storage.set_root(root.clone());
    storage.set_start(path);
    Ok(root)
}

#[tauri::command]
pub async fn change_storage_path(window: WebviewWindow, path: String) -> R<String> {
    let app = window.app_handle().clone();
    let root = tauri::async_runtime::spawn_blocking(move || change_flow(&app.state::<Storage>(), &path))
        .await
        .map_err(|e| FmError::Internal { message: e.to_string() })??;
    Ok(adopt(&window, &root))
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
