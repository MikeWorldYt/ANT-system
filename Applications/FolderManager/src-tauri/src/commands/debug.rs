// Commands of the Advanced settings page: the debug log, its folder and the webview devtools.
use tauri::{AppHandle, Manager};

use super::open::launcher;
use super::{locked, run_locked, Storage};
use crate::debug_log::{self, sysinfo};
use crate::error::{FmError, R};

#[tauri::command]
pub async fn start_debug_log(app: AppHandle, stamp: String) -> R<String> {
    tauri::async_runtime::spawn_blocking(move || {
        // The probes take about a second, so they run before the storage lock is taken.
        let info = sysinfo::collect();
        let root = locked(&app.state::<Storage>(), |root| Ok(root.to_path_buf()))?;
        debug_log::start(&stamp, &root, &info).map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}

#[tauri::command]
pub fn stop_debug_log() -> R<()> {
    debug_log::stop()
}

#[tauri::command]
pub fn log_debug(level: String, message: String) -> R<()> {
    debug_log::line(&level, &message)
}

#[tauri::command]
pub async fn open_logs_folder(app: AppHandle) -> R<()> {
    run_locked(app, |root| {
        let dir = debug_log::logs_dir(root);
        std::fs::create_dir_all(&dir)
            .map_err(|e| FmError::Internal { message: format!("Could not create {}: {e}", dir.display()) })?;
        launcher(&dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| FmError::Internal { message: format!("Could not open the folder: {e}") })
    })
    .await
}

#[tauri::command]
pub fn open_devtools(window: tauri::WebviewWindow) -> R<()> {
    window.open_devtools();
    Ok(())
}
