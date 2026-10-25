// Commands of the Backup settings page: export, the automatic backup, the list, restore and the OS schedule.
use tauri::AppHandle;

use super::run_locked;
use crate::backup::{self, schedule, BackupEntry, DEFAULT_FREQUENCY};
use crate::error::R;

#[tauri::command]
pub async fn backup_now(app: AppHandle, stamp: String) -> R<String> {
    run_locked(app, move |root| backup::create(root, &stamp).map(|p| p.to_string_lossy().into_owned())).await
}

#[tauri::command]
pub async fn backup_if_due(app: AppHandle, stamp: String) -> R<Option<String>> {
    run_locked(app, move |root| Ok(backup::if_due(root, &stamp)?.map(|p| p.to_string_lossy().into_owned()))).await
}

#[tauri::command]
pub async fn list_backups(app: AppHandle) -> R<Vec<BackupEntry>> {
    run_locked(app, |root| backup::list(root)).await
}

#[tauri::command]
pub async fn restore_backup(app: AppHandle, name: String, stamp: String) -> R<()> {
    run_locked(app, move |root| backup::restore(root, &name, &stamp)).await
}

#[tauri::command]
pub async fn backup_schedule(app: AppHandle, on: bool) -> R<()> {
    run_locked(app, move |root| {
        let settings = backup::settings(root)?;
        let frequency = settings.get("backupFrequency").map_or(DEFAULT_FREQUENCY, String::as_str);
        schedule::apply(on, frequency, root)
    })
    .await
}
