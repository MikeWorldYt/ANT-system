// Intake command: sorts dropped files under the storage lock, off the UI thread.
use tauri::AppHandle;

use super::run_locked;
use crate::domain::error::R;
use crate::infra::fs_safe::validate_component;
use crate::services::intake::IntakeReport;

#[tauri::command]
pub async fn run_intake(app: AppHandle) -> R<IntakeReport> {
    run_locked(app, crate::services::intake::run).await
}

#[tauri::command]
pub async fn acknowledge_intake(app: AppHandle, rel_parts: Vec<String>) -> R<()> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        crate::services::intake::acknowledge(root, &rel_parts)
    })
    .await
}
