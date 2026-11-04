// The storage the app has open: its root, the start path it was found from and the in-process lock.
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use serde::Serialize;

use crate::error::{FmError, R};

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
    pub root: String,
    pub app_version: String,
    pub max_depth: usize,
    pub depth_options: Vec<usize>,
    pub depth_warning: Option<String>,
}
