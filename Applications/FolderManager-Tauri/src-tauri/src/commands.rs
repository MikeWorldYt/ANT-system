// Tauri command layer: runs the pure core under the storage lock off the UI thread.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::config::{CORE_FOLDER, MAX_DEPTH, SETTINGS_FOLDER};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, validate_component};
use crate::tree_node::{build_children, build_tree, TreeNode, TreeSnapshot};
use crate::yaml_store::{self, yaml_path};

const SKIP_FILE: &str = ".skip_version";

pub struct Storage {
    pub start: PathBuf,
    pub root: Option<PathBuf>,
    pub lock: Mutex<()>,
}

impl Storage {
    pub fn resolve(start: PathBuf) -> Self {
        let root = crate::names::find_root(&start);
        Self { start, root, lock: Mutex::new(()) }
    }

    pub fn root(&self) -> R<&Path> {
        self.root
            .as_deref()
            .ok_or_else(|| FmError::StorageNotFound { start: self.start.display().to_string() })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    root: String,
    yaml_path: String,
    app_version: String,
    max_depth: usize,
}

// The closure runs with the storage lock held and receives the resolved root.
pub async fn run_locked<T, F>(app: AppHandle, f: F) -> R<T>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> R<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        let root = storage.root()?;
        let _guard = storage.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f(root)
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}

pub fn snapshot(root: &Path) -> R<TreeSnapshot> {
    let mut data = yaml_store::load(root)?;
    let changed = yaml_store::sync(root, &mut data)?;
    if changed {
        yaml_store::save(root, &data)?;
    }
    let mut tree = build_tree(root, &data, changed)?;
    fill_children(root, &data, &mut tree.nodes)?;
    Ok(tree)
}

fn fill_children(root: &Path, data: &serde_yaml::Mapping, nodes: &mut [TreeNode]) -> R<()> {
    for node in nodes {
        if node.has_children && node.depth + 1 < MAX_DEPTH {
            let mut kids = build_children(root, data, &node.rel_parts)?;
            fill_children(root, data, &mut kids)?;
            node.children = Some(kids);
        }
    }
    Ok(())
}

fn skip_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER).join(SKIP_FILE)
}

#[tauri::command]
pub async fn get_storage(app: AppHandle) -> R<StorageInfo> {
    run_locked(app, |root| {
        Ok(StorageInfo {
            root: root.to_string_lossy().into_owned(),
            yaml_path: yaml_path(root).to_string_lossy().into_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            max_depth: MAX_DEPTH,
        })
    })
    .await
}

#[tauri::command]
pub async fn load_tree(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, snapshot).await
}

#[tauri::command]
pub async fn refresh(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, snapshot).await
}

#[tauri::command]
pub async fn list_children(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let data = yaml_store::load(root)?;
        let mut kids = build_children(root, &data, &rel_parts)?;
        fill_children(root, &data, &mut kids)?;
        Ok(kids)
    })
    .await
}

#[tauri::command]
pub async fn get_skipped_version(app: AppHandle) -> R<String> {
    run_locked(app, |root| {
        let path = skip_path(root);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text.trim().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(io_err("read skipped version", &path, &e)),
        }
    })
    .await
}

#[tauri::command]
pub async fn skip_version(app: AppHandle, version: String) -> R<()> {
    run_locked(app, move |root| {
        let path = skip_path(root);
        std::fs::write(&path, version.trim()).map_err(|e| io_err("save skipped version", &path, &e))
    })
    .await
}
