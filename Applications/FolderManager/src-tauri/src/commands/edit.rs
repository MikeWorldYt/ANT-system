// Edit commands: each mutation runs under the storage lock, then refreshes the indexes it changed.
use std::path::Path;

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{locked, review_emitter, run_locked, Storage};
use crate::config::{EditableType, NewKind, WkbStatus};
use crate::mutations::{self, DeletePreview, MoveDirection, Mutation, PendingRecycle};
use crate::error::{FmError, R};
use crate::index::{reindex, review_index, search_index, Emit};
use crate::tree::tree_store;

pub(super) fn with_index(
    root: &Path,
    result: R<Mutation>,
    emit: Emit,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> R<Mutation> {
    let mut m = result?;
    m.warnings.extend(reindex(root, emit, rebuild).into_iter().map(|f| format!("Review index: {f}")));
    with_search_index(root, Ok(m))
}

pub(super) fn with_search_index(root: &Path, result: R<Mutation>) -> R<Mutation> {
    let mut m = result?;
    // A synced yaml may have changed paths the mutation never touched, so only an unsynced tree updates in place.
    if !m.touched.is_empty() && !m.tree.yaml_changed {
        match search_index::update_paths(root, &m.tree.nodes, &m.touched) {
            Ok(()) => return Ok(m),
            Err(e) => m.warnings.push(format!("Search index: the quick update failed, so it was rebuilt in full. {e}")),
        }
    }
    // An empty tree may be after_commit's fallback, so the yaml is indexed instead of wiping the index.
    let indexed = if m.tree.nodes.is_empty() {
        tree_store::load(root).and_then(|data| search_index::rebuild(root, &data))
    } else {
        search_index::rebuild_from_nodes(root, &m.tree.nodes)
    };
    if let Err(e) = indexed {
        m.warnings.push(format!("Search index: {e}"));
    }
    Ok(m)
}

// The recycle runs after the storage lock is released so a slow Recycle Bin never blocks other commands.
pub(super) fn delete_flow(
    storage: &Storage,
    rel: &[String],
    flags: (bool, bool),
    emit: Emit,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
    recycle: impl FnOnce(PendingRecycle) -> Vec<String>,
) -> R<Mutation> {
    let (mut m, pending) = locked(storage, |root| {
        let (m, pending) = mutations::delete_node(root, rel, flags.0, flags.1)?;
        Ok((with_index(root, Ok(m), emit, rebuild)?, pending))
    })?;
    if let Some(pending) = pending {
        m.warnings.extend(recycle(pending));
    }
    Ok(m)
}

#[tauri::command]
pub async fn rename_node(app: AppHandle, rel_parts: Vec<String>, new_editable: String) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::rename_node(root, &rel_parts, &new_editable);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn set_node_type(
    app: AppHandle,
    rel_parts: Vec<String>,
    type_base: EditableType,
    status: Option<WkbStatus>,
) -> R<Mutation> {
    run_locked(app, move |root| {
        with_search_index(root, mutations::set_node_type(root, &rel_parts, type_base, status))
    })
    .await
}

#[tauri::command]
pub async fn set_favorite(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_locked(app, move |root| with_search_index(root, mutations::set_favorite(root, &rel_parts, on))).await
}

#[tauri::command]
pub async fn set_user_flag(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_locked(app, move |root| with_search_index(root, mutations::set_user_flag(root, &rel_parts, on))).await
}

#[tauri::command]
pub async fn set_node_details(app: AppHandle, rel_parts: Vec<String>, status: Option<WkbStatus>, owner: Option<String>) -> R<Mutation> {
    run_locked(app, move |root| {
        with_search_index(root, mutations::set_details(root, &rel_parts, status, owner.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn add_node(app: AppHandle, rel_parts: Vec<String>, kind: NewKind) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::add_next(root, &rel_parts, kind);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn move_node(app: AppHandle, rel_parts: Vec<String>, direction: MoveDirection) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::move_node(root, &rel_parts, direction);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn inspect_delete(app: AppHandle, rel_parts: Vec<String>) -> R<DeletePreview> {
    run_locked(app, move |root| mutations::inspect_delete(root, &rel_parts)).await
}

#[tauri::command]
pub async fn delete_node(
    app: AppHandle,
    rel_parts: Vec<String>,
    expect_ghost: bool,
    confirm_non_empty: bool,
) -> R<Mutation> {
    tauri::async_runtime::spawn_blocking(move || {
        let flags = (expect_ghost, confirm_non_empty);
        let emit = review_emitter(&app);
        let storage = app.state::<Storage>();
        delete_flow(&storage, &rel_parts, flags, &emit, review_index::rebuild, mutations::recycle_staged)
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}
