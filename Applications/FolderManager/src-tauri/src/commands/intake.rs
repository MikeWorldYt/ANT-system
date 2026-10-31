// Intake command: sorts dropped files under the storage lock, off the UI thread.
use tauri::AppHandle;

use super::run_locked;
use crate::error::R;
use crate::fs_safe::validate_component;
use crate::intake::IntakeReport;

#[tauri::command]
pub async fn run_intake(app: AppHandle) -> R<IntakeReport> {
    run_locked(app, |root| crate::intake::run(root)).await
}

#[tauri::command]
pub async fn acknowledge_intake(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        crate::intake::acknowledge(root, &rel_parts)
    })
    .await
}
