// Commands of the one-time support nodes upgrade: the check and the final root move.
use tauri::AppHandle;

use super::run_locked;
use crate::domain::error::R;
use crate::services::debug_log::valid_stamp;
use crate::services::upgrade::{self, SupportCheck};

#[tauri::command]
pub async fn check_support_upgrade(app: AppHandle) -> R<SupportCheck> {
    run_locked(app, |root| upgrade::check_support_upgrade(root)).await
}

#[tauri::command]
pub async fn finish_support_upgrade(app: AppHandle, stamp: String) -> R<Vec<String>> {
    run_locked(app, move |root| {
        if !valid_stamp(&stamp) {
            return Err(crate::domain::error::FmError::Internal { message: format!("Invalid backup stamp {stamp}") });
        }
        upgrade::finish_support_upgrade(root, &stamp)
    })
    .await
}
