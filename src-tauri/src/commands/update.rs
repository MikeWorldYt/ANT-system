// Update commands: the version check, the skipped version kept in the storage, and the hand-off to the updater.
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use super::run_locked;
use crate::config::{CORE_FOLDER, SETTINGS_FOLDER};
use crate::error::{FmError, R};
use crate::fs_safe::io_err;
use crate::updater;

const SKIP_FILE: &str = ".skip_version";

pub(super) fn skip_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER).join(SKIP_FILE)
}

fn read_skipped(root: &Path) -> R<String> {
    let path = skip_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text.trim().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(io_err("read skipped version", &path, &e)),
    }
}

// A manual check offers the newest version again even after it was skipped.
pub(super) fn skipped_for(root: &Path, manual: bool) -> R<String> {
    if manual {
        return Ok(String::new());
    }
    read_skipped(root)
}

#[derive(Serialize)]
pub struct UpdateInfo {
    current: String,
    latest: String,
}

// The download runs outside the storage lock so a slow network never blocks the tree.
#[tauri::command]
pub async fn check_update(app: AppHandle, manual: bool) -> R<Option<UpdateInfo>> {
    let latest = tauri::async_runtime::spawn_blocking(updater::latest_version)
        .await
        .map_err(|e| FmError::Internal { message: e.to_string() })?;
    let Some(latest) = latest.filter(|v| updater::is_newer(v, updater::CURRENT_VERSION)) else { return Ok(None) };
    let skipped = run_locked(app, move |root| skipped_for(root, manual)).await?;
    Ok((latest != skipped).then(|| UpdateInfo { current: updater::CURRENT_VERSION.to_string(), latest }))
}

#[tauri::command]
pub async fn start_update(app: AppHandle) -> R<()> {
    run_locked(app.clone(), |root| updater::launch_updater(root)).await?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn skip_version(app: AppHandle, version: String) -> R<()> {
    run_locked(app, move |root| {
        let path = skip_path(root);
        std::fs::write(&path, version.trim()).map_err(|e| io_err("save skipped version", &path, &e))
    })
    .await
}
