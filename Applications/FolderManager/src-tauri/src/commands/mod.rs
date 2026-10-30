// Tauri command layer: runs the pure core under the storage lock off the UI thread.
mod backup;
mod clipboard;
mod debug;
mod drag_out;
mod edit;
mod extensions;
mod health;
mod intake;
mod open;
mod search;
pub mod settings;
mod storage;
#[cfg(test)]
mod tests;
mod tree;
mod update;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{DEPTH_CEILING, MAX_DEPTH};
use crate::error::{FmError, R};
use crate::index::ReviewIndexEvent;
use crate::tree::depth;
use crate::updater;

pub use backup::*;
pub use debug::*;
pub use drag_out::*;
pub use edit::*;
pub use extensions::*;
pub use health::*;
pub use intake::*;
pub use open::*;
pub use search::*;
pub use storage::*;
pub use tree::*;
pub use update::*;

const REVIEW_EVENT: &str = "fm://review-index";

pub struct Storage {
    pub start: RwLock<PathBuf>,
    pub root: RwLock<Option<PathBuf>>,
    pub lock: Mutex<()>,
}

impl Storage {
    pub fn resolve(start: PathBuf) -> Self {
        let root = crate::ant_core::names::find_root(&start);
        Self { start: RwLock::new(start), root: RwLock::new(root), lock: Mutex::new(()) }
    }

    pub fn root(&self) -> R<PathBuf> {
        self.root
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .ok_or_else(|| FmError::StorageNotFound { start: self.start().display().to_string() })
    }

    pub fn start(&self) -> PathBuf {
        self.start.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    pub fn set_start(&self, start: PathBuf) {
        *self.start.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = start;
    }

    pub fn reset(&self, start: PathBuf) {
        *self.root.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = crate::ant_core::names::find_root(&start);
        self.set_start(start);
    }

    pub fn set_root(&self, root: PathBuf) {
        *self.root.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(root);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    root: String,
    app_version: String,
    max_depth: usize,
    depth_options: Vec<usize>,
    depth_warning: Option<String>,
}

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
        if let Err(log) = crate::debug_log::line("error", &err.to_string()) {
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
