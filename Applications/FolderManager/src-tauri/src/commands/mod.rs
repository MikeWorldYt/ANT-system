// Tauri command layer: runs the pure core under the storage lock off the UI thread.
mod backup;
mod debug;
mod downloads;
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
mod ui_bridge;
mod update;
mod upgrade;

use std::path::Path;

use tauri::{AppHandle, Emitter, Manager};

use crate::app::storage_session::{Storage, StorageInfo};
use crate::domain::error::{FmError, R};
use crate::services::index::ReviewIndexEvent;

pub use backup::*;
pub use debug::*;
pub use downloads::*;
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
pub use ui_bridge::*;
pub use update::*;
pub use upgrade::*;


const REVIEW_EVENT: &str = "fm://review-index";

// A panicked worker surfaces as an Internal error instead of tearing down the command.
pub(crate) async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| FmError::Internal { message: e.to_string() })
}

pub fn locked<T>(storage: &Storage, f: impl FnOnce(&Path) -> R<T>) -> R<T> {
    let _guard = storage.guard();
    let root = storage.root()?;
    f(&root)
}

// The closure runs with the storage lock held and receives the resolved root.
pub async fn run_locked<T, F>(app: AppHandle, f: F) -> R<T>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> R<T> + Send + 'static,
{
    let result = off_thread(move || locked(&app.state::<Storage>(), f)).await?;
    log_failure(&result);
    result
}

// Reads only: no storage lock, so a change in progress is neither waited for nor held up.
pub async fn run_unlocked<T, F>(app: AppHandle, f: F) -> R<T>
where
    T: Send + 'static,
    F: FnOnce(&Storage, &Path) -> R<T> + Send + 'static,
{
    let result = off_thread(move || {
        let storage = app.state::<Storage>();
        let root = storage.root()?;
        f(&storage, &root)
    })
    .await?;
    log_failure(&result);
    result
}

fn log_failure<T>(result: &R<T>) {
    if let Err(err) = result {
        // The command's own error matters more to the caller than a failed log write.
        if let Err(log) = crate::services::debug_log::line("error", &err.to_string()) {
            eprintln!("{log}");
        }
    }
}

fn review_emitter(app: &AppHandle) -> impl Fn(&ReviewIndexEvent) -> Result<(), String> + '_ {
    move |event| app.emit(REVIEW_EVENT, event).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_storage(app: AppHandle) -> R<StorageInfo> {
    run_locked(app, StorageInfo::of).await
}
