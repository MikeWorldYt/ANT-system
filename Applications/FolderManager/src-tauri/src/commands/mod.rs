// Tauri command layer: runs the pure core under the storage lock off the UI thread.
mod backup;
mod debug;
mod drag_out;
mod edit;
mod extensions;
mod file_drop;
mod health;
mod intake;
mod open;
mod search;
pub mod settings;
mod share;
mod storage;
#[cfg(test)]
mod tests;
mod tree;
mod update;

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};

use crate::domain::config::{DEPTH_CEILING, MAX_DEPTH};
use crate::app::storage_session::{Storage, StorageInfo};
use crate::domain::error::{FmError, R};
use crate::services::index::ReviewIndexEvent;
use crate::services::tree::depth;
use crate::services::updater;

pub use backup::*;
pub use debug::*;
pub use drag_out::*;
pub use edit::*;
pub use extensions::*;
pub use file_drop::*;
pub use health::*;
pub use intake::*;
pub use open::*;
pub use search::*;
pub use share::*;
pub use storage::*;
pub use tree::*;
pub use update::*;


const REVIEW_EVENT: &str = "fm://review-index";

pub fn locked<T>(storage: &Storage, f: impl FnOnce(&Path) -> R<T>) -> R<T> {
    let _guard = storage.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = storage.root()?;
    f(&root)
}

// The closure runs with the storage lock held and receives the resolved root.
pub async fn run_locked<T, F>(app: AppHandle, f: F) -> R<T>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> R<T> + Send + 'static,
{
    let result = tauri::async_runtime::spawn_blocking(move || locked(&app.state::<Storage>(), f))
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?;
    if let Err(err) = &result {
        // The command's own error matters more to the caller than a failed log write.
        if let Err(log) = crate::services::debug_log::line("error", &err.to_string()) {
            eprintln!("{log}");
        }
    }
    result
}

fn review_emitter(app: &AppHandle) -> impl Fn(&ReviewIndexEvent) -> Result<(), String> + '_ {
    move |event| app.emit(REVIEW_EVENT, event).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_storage(app: AppHandle) -> R<StorageInfo> {
    run_locked(app, |root| {
        let setting = depth::read(root)?;
        Ok(StorageInfo {
            root: root.to_string_lossy().into_owned(),
            app_version: updater::CURRENT_VERSION.to_string(),
            max_depth: setting.limit,
            depth_options: (MAX_DEPTH..=DEPTH_CEILING).collect(),
            depth_warning: setting.warning,
        })
    })
    .await
}
