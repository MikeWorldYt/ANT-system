// The storage the app has open: its root, the start path it was found from and the in-process lock.
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, RwLock};

use serde::Serialize;

use crate::domain::error::{FmError, R};

pub struct Storage {
    pub start: RwLock<PathBuf>,
    pub root: RwLock<Option<PathBuf>>,
    pub lock: Mutex<()>,
}

impl Storage {
    pub fn resolve(start: PathBuf) -> Self {
        let root = crate::infra::scan::find_root(&start);
        Self { start: RwLock::new(start), root: RwLock::new(root), lock: Mutex::new(()) }
    }

    pub fn root(&self) -> R<PathBuf> {
        read(&self.root).ok_or_else(|| FmError::StorageNotFound { start: self.start().display().to_string() })
    }

    pub fn start(&self) -> PathBuf {
        read(&self.start)
    }

    pub fn set_start(&self, start: PathBuf) {
        write(&self.start, start);
    }

    pub fn reset(&self, start: PathBuf) {
        write(&self.root, crate::infra::scan::find_root(&start));
        self.set_start(start);
    }

    pub fn set_root(&self, root: PathBuf) {
        write(&self.root, Some(root));
    }

    pub fn guard(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// A panic while a path was held leaves it intact, so the lock is used as is.
fn read<T: Clone>(lock: &RwLock<T>) -> T {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

fn write<T>(lock: &RwLock<T>, value: T) {
    *lock.write().unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub root: String,
    pub app_version: String,
    pub max_depth: usize,
    pub depth_options: Vec<usize>,
    pub depth_warning: Option<String>,
}
