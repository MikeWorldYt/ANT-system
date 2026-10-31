// Edit commands: each mutation runs under the storage lock, then refreshes the indexes it changed.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{locked, review_emitter, run_locked, Storage};
use crate::ant_core::shortcuts::create_shortcuts;
use crate::config::{EditableType, NewKind, WkbStatus};
use crate::mutations::{self, CopyAnswer, DeletePreview, MoveDirection, Mutation, PendingRecycle};
use crate::error::{FmError, R};
use crate::index::{reindex, reindex_paths, review_index, search_index, Emit};
use crate::tree::tree_store;

pub(super) fn with_index(
    root: &Path,
    result: R<Mutation>,
    emit: Emit,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> R<Mutation> {
    let mut m = result?;
    // Same rule as the search index: a full snapshot means paths the mutation never touched may have changed.
    let failures = if m.tree.is_none() {
        reindex_paths(root, emit, &m.before, &m.touched, create, rebuild)
    } else {
        reindex(root, emit, rebuild)
    };
    m.warnings.extend(failures.into_iter().map(|f| format!("Review index: {f}")));
    with_search_index(root, Ok(m))
}

pub(super) fn with_search_index(root: &Path, result: R<Mutation>) -> R<Mutation> {
    let mut m = result?;
    // Only a local change updates in place; a full snapshot comes with a synced yaml or an unknown scope.
    if m.tree.is_none() {
        match search_index::update_paths(root, &m.touched_nodes(), &m.touched) {
            Ok(()) => return Ok(m),
            Err(e) => m.warnings.push(format!("Search index: the quick update failed, so it was rebuilt in full. {e}")),
        }
    }
    // A local change or after_commit's empty fallback has no full tree, so the yaml is indexed instead of wiping the index.
    let indexed = match m.tree.as_ref().filter(|t| !t.nodes.is_empty()) {
        Some(tree) => search_index::rebuild_from_nodes(root, &tree.nodes),
        None => tree_store::load(root).and_then(|data| search_index::rebuild(root, &data)),
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
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
    recycle: impl FnOnce(PendingRecycle) -> Vec<String>,
) -> R<Mutation> {
    let (mut m, pending) = locked(storage, |root| {
        let (m, pending) = mutations::delete_node(root, rel, flags.0, flags.1)?;
        Ok((with_index(root, Ok(m), emit, create, rebuild)?, pending))
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
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
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
pub async fn answer_copy(app: AppHandle, rel_parts: Vec<String>, answer: CopyAnswer) -> R<()> {
    run_locked(app, move |root| mutations::answer_copy(root, &rel_parts, answer)).await
}

#[tauri::command]
pub async fn add_node(app: AppHandle, rel_parts: Vec<String>, kind: NewKind) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::add_next(root, &rel_parts, kind);
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn add_child(app: AppHandle, rel_parts: Vec<String>, kind: NewKind) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::add_node(root, &rel_parts, kind);
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn move_node(app: AppHandle, rel_parts: Vec<String>, direction: MoveDirection) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::move_node(root, &rel_parts, direction);
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn move_folder(app: AppHandle, source: Vec<String>, destination: Vec<String>, copy: bool) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::move_folder(root, &source, &destination, copy);
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn resolve_invalid(app: AppHandle, rel_parts: Vec<String>, resolution: mutations::Resolution) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = mutations::resolve_invalid(root, &rel_parts, resolution);
        with_index(root, result, &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
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
        delete_flow(&storage, &rel_parts, flags, &emit, create_shortcuts, review_index::rebuild, mutations::recycle_staged)
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}
