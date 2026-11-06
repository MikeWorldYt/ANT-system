// The storage the app has open: its root, the start path it was found from, the in-process lock and the read connection that serves searches.
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, RwLock};

use serde::Serialize;

use crate::domain::error::{FmError, R};
use crate::infra::storage::db::Store;

pub struct Storage {
    pub start: RwLock<PathBuf>,
    pub root: RwLock<Option<PathBuf>>,
    pub lock: Mutex<()>,
    pub(crate) reader: Mutex<Option<(PathBuf, Store)>>,
}

impl Storage {
    pub fn resolve(start: PathBuf) -> Self {
        let root = crate::infra::scan::find_root(&start);
        Self { start: RwLock::new(start), root: RwLock::new(root), lock: Mutex::new(()), reader: Mutex::new(None) }
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
        self.close_reader();
        write(&self.root, crate::infra::scan::find_root(&start));
        self.set_start(start);
    }

    pub fn set_root(&self, root: PathBuf) {
        self.close_reader();
        write(&self.root, Some(root));
    }

    // Runs a read on the kept connection, opened on first use and again after the root changes.
    pub fn with_reader<T>(&self, f: impl FnOnce(&Store) -> R<T>) -> R<T> {
        let root = self.root()?;
        let mut slot = self.reader.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !slot.as_ref().is_some_and(|(open_root, _)| *open_root == root) {
            *slot = Some((root.clone(), Store::open_or_create(&root)?));
        }
        match slot.as_ref() {
            Some((_, store)) => f(store),
            None => Err(FmError::StorageNotFound { start: self.start().display().to_string() }),
        }
    }

    // An open handle would block a rename or a restore of the files it reads, so those close it first.
    pub fn close_reader(&self) {
        *self.reader.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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
