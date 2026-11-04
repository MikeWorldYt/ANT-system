// Tree commands: the first load, a refresh that also rebuilds 998.Review, and the children of one folder.
use std::path::Path;

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{review_emitter, run_locked, Storage};
use crate::domain::model::Copies;
use crate::error::{FmError, R};
use crate::fs_safe::{join_rel, validate_component};
use crate::index::{node_names, reindex, review_folders, review_index, Emit};
use crate::services::snapshot::snapshot;
use crate::tree::root_check::{self, RootCheck};
use crate::tree::tree_node::{build_children, fill_children, lists_files, TreeNode, TreeSnapshot};
use crate::tree::tree_store;

pub(super) fn refresh_flow(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> R<TreeSnapshot> {
    node_names::tidy(root)?;
    let mut tree = snapshot(root, Copies::Tail)?;
    if review_folders::restore(root)? {
        let copies = tree.copies;
        tree = snapshot(root, Copies::Tail)?;
        tree.copies.extend(copies);
        tree.yaml_changed = true;
    }
    reindex(root, emit, rebuild);
    Ok(tree)
}

#[tauri::command]
pub async fn load_tree(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, |root| snapshot(root, Copies::Tail)).await
}

#[tauri::command]
pub async fn refresh(app: AppHandle) -> R<TreeSnapshot> {
    let emitter = app.clone();
    run_locked(app, move |root| refresh_flow(root, &review_emitter(&emitter), review_index::rebuild)).await
}

#[tauri::command]
pub async fn check_root(app: AppHandle) -> R<RootCheck> {
    run_locked(app, |root| root_check::check_root(root)).await
}

#[tauri::command]
pub async fn list_children(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let data = tree_store::load(root)?;
        let mut kids = build_children(root, &data, &rel_parts)?;
        fill_children(root, &data, &mut kids)?;
        Ok(kids)
    })
    .await
}

// Reads without the storage lock, so a slow shell thumbnail never holds up a change.
#[tauri::command]
pub async fn file_image(app: AppHandle, rel_parts: Vec<String>, large: bool) -> R<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = app.state::<Storage>().root()?;
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let parent = &rel_parts[..rel_parts.len().saturating_sub(1)];
        if !lists_files(parent) {
            return Ok(None);
        }
        crate::preview::file_image(&join_rel(&root, &rel_parts), large)
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}
