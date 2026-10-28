// Intake command: sorts dropped files under the storage lock, off the UI thread.
use tauri::AppHandle;

use super::run_locked;
use crate::error::R;
use crate::intake::IntakeReport;

#[tauri::command]
pub async fn run_intake(app: AppHandle) -> R<IntakeReport> {
    run_locked(app, |root| crate::intake::run(root)).await
}
