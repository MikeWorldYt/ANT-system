// Edit commands: each mutation runs under the storage lock, then refreshes the indexes it changed.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{locked, off_thread, review_emitter, run_locked};
use crate::app::storage_session::Storage;
use crate::platform::recycle::send_to_recycle_bin;
use crate::platform::shortcuts::create_shortcuts;
use crate::domain::config::{EditableType, NewKind, WkbStatus};
use crate::services::mutations::{self, CopyAnswer, DeletePreview, MoveDirection, Mutation};
use crate::domain::error::R;
use crate::services::index::{reindex, reindex_paths, review_index, search_index, Emit};
use crate::services::tree::tree_store;

pub(super) fn with_index(
    root: &Path,
    result: R<Mutation>,
    emit: Emit,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> R<Mutation> {
    let mut m = result?;
    // An unknown scope (no touched paths) or a failed reload leaves nothing to update in place, so it is rebuilt in full.
    let failures = if known_scope(&m) {
        reindex_paths(root, emit, &m.before, &m.touched, create, rebuild)
    } else {
        reindex(root, emit, rebuild)
    };
    m.warnings.extend(failures.into_iter().map(|f| format!("Review index: {f}")));
    with_search_index(root, Ok(m))
}

pub(super) fn with_search_index(root: &Path, result: R<Mutation>) -> R<Mutation> {
    let mut m = result?;
    if known_scope(&m) {
        match search_index::update_paths(root, &m.touched_nodes(), &m.touched) {
            Ok(()) => return Ok(m),
            Err(e) => m.warnings.push(format!("Search index: the quick update failed, so it was rebuilt in full. {e}")),
        }
    }
    // after_commit's empty fallback has no tree, so the yaml is indexed instead of wiping the index.
    let indexed = match m.tree.as_ref().filter(|t| !t.nodes.is_empty()) {
        Some(tree) => search_index::rebuild_from_nodes(root, &tree.nodes),
        None => tree_store::load(root).and_then(|data| search_index::rebuild(root, &data)),
    };
    if let Err(e) = indexed {
        m.warnings.push(format!("Search index: {e}"));
    }
    Ok(m)
}

fn known_scope(m: &Mutation) -> bool {
    !m.touched.is_empty() && m.tree.as_ref().is_none_or(|t| !t.nodes.is_empty())
}

// Mutations that can add, move or drop 998.Review links rebuild that index along with the search fields.
async fn run_indexed<F>(app: AppHandle, f: F) -> R<Mutation>
where
    F: FnOnce(&Path) -> R<Mutation> + Send + 'static,
{
    let emitter = app.clone();
    run_locked(app, move |root| {
        with_index(root, f(root), &review_emitter(&emitter), create_shortcuts, review_index::rebuild)
    })
    .await
}

async fn run_search_indexed<F>(app: AppHandle, f: F) -> R<Mutation>
where
    F: FnOnce(&Path) -> R<Mutation> + Send + 'static,
{
    run_locked(app, move |root| with_search_index(root, f(root))).await
}

pub(super) fn delete_flow(
    storage: &Storage,
    rel: &[String],
    flags: (bool, bool),
    emit: Emit,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> R<Mutation> {
    locked(storage, |root| {
        let m = mutations::delete_node(root, rel, flags.0, flags.1, recycle)?;
        with_index(root, Ok(m), emit, create, rebuild)
    })
}

#[tauri::command]
pub async fn rename_node(app: AppHandle, rel_parts: Vec<String>, new_editable: String) -> R<Mutation> {
    run_indexed(app, move |root| mutations::rename_node(root, &rel_parts, &new_editable)).await
}

#[tauri::command]
pub async fn rename_file(app: AppHandle, rel_parts: Vec<String>, new_name: String) -> R<Mutation> {
    run_locked(app, move |root| mutations::rename_file(root, &rel_parts, &new_name)).await
}

#[tauri::command]
pub async fn delete_file(app: AppHandle, rel_parts: Vec<String>) -> R<Mutation> {
    off_thread(move || {
        let storage = app.state::<Storage>();
        locked(&storage, |root| mutations::delete_file(root, &rel_parts, send_to_recycle_bin))
    })
    .await?
}

#[tauri::command]
pub async fn set_node_type(
    app: AppHandle,
    rel_parts: Vec<String>,
    type_base: EditableType,
    status: Option<WkbStatus>,
) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_node_type(root, &rel_parts, type_base, status)).await
}

#[tauri::command]
pub async fn set_favorite(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_favorite(root, &rel_parts, on)).await
}

#[tauri::command]
pub async fn set_important(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_important(root, &rel_parts, on)).await
}

#[tauri::command]
pub async fn set_link_hidden(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_link_hidden(root, &rel_parts, on)).await
}

#[tauri::command]
pub async fn set_review_filter(app: AppHandle, hidden: Vec<String>, favorites: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_review_filter(root, &hidden, favorites)).await
}

#[tauri::command]
pub async fn set_user_flag(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_user_flag(root, &rel_parts, on)).await
}

#[tauri::command]
pub async fn set_node_details(app: AppHandle, rel_parts: Vec<String>, status: Option<WkbStatus>, owner: Option<String>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_details(root, &rel_parts, status, owner.as_deref())).await
}

#[tauri::command]
pub async fn set_board(app: AppHandle, cards: Vec<mutations::BoardCard>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_board(root, &cards)).await
}

#[tauri::command]
pub async fn set_outcome_default(app: AppHandle, rel_parts: Vec<String>, key: String, value: Option<String>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_default(root, &rel_parts, &key, value.as_deref())).await
}

#[tauri::command]
pub async fn set_node_output(app: AppHandle, rel_parts: Vec<String>, output: Option<String>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_output(root, &rel_parts, output.as_deref())).await
}

#[tauri::command]
pub async fn deliver_output(app: AppHandle, rel_parts: Vec<String>, date: String, all: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::deliver(root, &rel_parts, &date, all)).await
}

#[tauri::command]
pub async fn restore_output(app: AppHandle, rel_parts: Vec<String>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::restore(root, &rel_parts)).await
}

#[tauri::command]
pub async fn set_node_hidden(app: AppHandle, rel_parts: Vec<String>, on: bool) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_hidden(root, &rel_parts, on)).await
}

#[tauri::command]
pub async fn set_hidden_nodes(app: AppHandle, parent: Vec<String>, hidden: Vec<String>) -> R<Mutation> {
    run_search_indexed(app, move |root| mutations::set_hidden_children(root, &parent, &hidden)).await
}

#[tauri::command]
pub async fn list_hidden_choices(app: AppHandle, parent: Vec<String>) -> R<Vec<mutations::HiddenChoice>> {
    run_locked(app, move |root| mutations::hidden_choices(root, &parent)).await
}

#[tauri::command]
pub async fn answer_copy(app: AppHandle, rel_parts: Vec<String>, answer: CopyAnswer) -> R<()> {
    run_locked(app, move |root| mutations::answer_copy(root, &rel_parts, answer)).await
}

#[tauri::command]
pub async fn add_node(app: AppHandle, rel_parts: Vec<String>, kind: NewKind) -> R<Mutation> {
    run_indexed(app, move |root| mutations::add_next(root, &rel_parts, kind)).await
}

#[tauri::command]
pub async fn add_child(app: AppHandle, rel_parts: Vec<String>, kind: NewKind) -> R<Mutation> {
    run_indexed(app, move |root| mutations::add_node(root, &rel_parts, kind)).await
}

#[tauri::command]
pub async fn move_node(app: AppHandle, rel_parts: Vec<String>, direction: MoveDirection) -> R<Mutation> {
    run_indexed(app, move |root| mutations::move_node(root, &rel_parts, direction)).await
}

#[tauri::command]
pub async fn move_folder(app: AppHandle, source: Vec<String>, destination: Vec<String>, copy: bool) -> R<Mutation> {
    run_indexed(app, move |root| mutations::move_folder(root, &source, &destination, copy)).await
}

#[tauri::command]
pub async fn preview_invalid(app: AppHandle, rows: Vec<mutations::Pending>) -> R<Vec<String>> {
    run_locked(app, move |root| mutations::preview_names(root, &rows)).await
}

#[tauri::command]
pub async fn resolve_invalid(app: AppHandle, rel_parts: Vec<String>, resolution: mutations::Resolution) -> R<Mutation> {
    run_indexed(app, move |root| mutations::resolve_invalid(root, &rel_parts, resolution)).await
}

#[tauri::command]
pub async fn apply_support_upgrade(app: AppHandle, rel_parts: Vec<String>) -> R<Mutation> {
    run_indexed(app, move |root| crate::services::upgrade::apply_support_upgrade(root, &rel_parts)).await
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
    off_thread(move || {
        let flags = (expect_ghost, confirm_non_empty);
        let emit = review_emitter(&app);
        let storage = app.state::<Storage>();
        delete_flow(&storage, &rel_parts, flags, &emit, create_shortcuts, review_index::rebuild, send_to_recycle_bin)
    })
    .await?
}
