// Tree commands: the first load, a refresh that also rebuilds 998.Review, and the children of one folder.
use std::path::Path;

use serde_yaml::Mapping;
use tauri::AppHandle;

use super::{review_emitter, run_locked};
use crate::config::MAX_DEPTH;
use crate::error::R;
use crate::fs_safe::validate_component;
use crate::index::{reindex, review_folders, review_index, Emit};
use crate::tree::snapshot;
use crate::tree::tree_node::{build_children, fill_children, TreeNode, TreeSnapshot};
use crate::tree::tree_store;

pub(super) fn refresh_flow(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> R<TreeSnapshot> {
    let mut tree = snapshot(root)?;
    if review_folders::restore(root)? {
        tree = snapshot(root)?;
        tree.yaml_changed = true;
    }
    reindex(root, emit, rebuild);
    Ok(tree)
}

#[tauri::command]
pub async fn load_tree(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, snapshot).await
}

#[tauri::command]
pub async fn refresh(app: AppHandle) -> R<TreeSnapshot> {
    let emitter = app.clone();
    run_locked(app, move |root| refresh_flow(root, &review_emitter(&emitter), review_index::rebuild)).await
}

#[tauri::command]
pub async fn list_children(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        if rel_parts.len() >= MAX_DEPTH {
            return Ok(Vec::new());
        }
        let data = tree_store::load(root)?;
        let mut kids = build_children(root, &data, &rel_parts)?;
        fill_children(root, &data, &mut kids)?;
        Ok(kids)
    })
    .await
}
