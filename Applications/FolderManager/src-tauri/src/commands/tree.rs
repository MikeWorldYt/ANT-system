// Tree commands: the first load, a refresh that also rebuilds 998.Review, and the children of one folder.
use std::path::Path;

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{off_thread, review_emitter, run_locked, run_unlocked};
use crate::app::storage_session::Storage;
use crate::services::tree::media;
use crate::domain::model::Copies;
use crate::domain::error::R;
use crate::infra::fs_safe::resolve;
use crate::services::index::{node_names, reindex_data, review_folders, review_index, Emit};
use crate::services::snapshot::{snapshot, snapshot_data};
use crate::services::tree::root_check::{self, RootCheck};
use crate::services::tree::level;
use crate::services::tree::tree_node::{lists_files, TreeNode, TreeSnapshot};

// Only a manual refresh (`full`) rebuilds the search and review indexes; automatic ones just sync the tree.
pub(super) fn refresh_flow(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>, full: bool) -> R<TreeSnapshot> {
    node_names::tidy(root)?;
    let (mut tree, mut data) = snapshot_data(root, Copies::Tail, full)?;
    if review_folders::restore(root)? {
        let copies = tree.copies;
        (tree, data) = snapshot_data(root, Copies::Tail, full)?;
        tree.copies.extend(copies);
        tree.yaml_changed = true;
    }
    if full {
        reindex_data(root, emit, &data, rebuild);
    }
    Ok(tree)
}

#[tauri::command]
pub async fn load_tree(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, |root| {
        media::ensure(root)?;
        snapshot(root, Copies::Tail)
    })
    .await
}

#[tauri::command]
pub async fn refresh(app: AppHandle, full: bool) -> R<TreeSnapshot> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        media::ensure(root)?;
        refresh_flow(root, &review_emitter(&emitter), review_index::rebuild, full)
    })
    .await
}

#[tauri::command]
pub async fn check_root(app: AppHandle) -> R<RootCheck> {
    run_locked(app, root_check::check_root).await
}

#[tauri::command]
pub async fn list_children(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    run_unlocked(app, move |_, root| level::children(root, &rel_parts)).await
}

// Reads without the storage lock, so a slow shell thumbnail never holds up a change.
#[tauri::command]
pub async fn file_image(app: AppHandle, rel_parts: Vec<String>, large: bool) -> R<Option<String>> {
    off_thread(move || {
        let root = app.state::<Storage>().root()?;
        let parent = &rel_parts[..rel_parts.len().saturating_sub(1)];
        if !lists_files(parent) {
            return Ok(None);
        }
        crate::services::preview::file_image(&resolve(&root, &rel_parts)?, large)
    })
    .await?
}
