// Edit commands: each mutation runs under the storage lock, then refreshes the indexes it changed.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;
use tauri::{AppHandle, Manager};

use super::{locked, off_thread, review_emitter, run_locked, run_unlocked};
use crate::app::storage_session::Storage;
use crate::platform::shortcuts::create_shortcuts;
use crate::domain::config::{EditableType, NewKind, WkbStatus};
use crate::services::mutations::{self, CopyAnswer, DeletePreview, IdTarget, MoveDirection, Mutation, ReorderPlan};
use crate::domain::error::R;
use crate::services::{automations, edit, held_trash};
use crate::services::index::{review_index, Emit};

// Mutations that can add, move or drop 998.Review links rebuild that index along with the search fields.
async fn run_indexed<F>(app: AppHandle, f: F) -> R<Mutation>
where
    F: FnOnce(&Path) -> R<Mutation> + Send + 'static,
{
    let emitter = app.clone();
    run_locked(app, move |root| edit::indexed(root, &review_emitter(&emitter), f)).await
}

async fn run_search_indexed<F>(app: AppHandle, f: F) -> R<Mutation>
where
    F: FnOnce(&Path) -> R<Mutation> + Send + 'static,
{
    run_locked(app, move |root| edit::search_indexed(root, f)).await
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
    locked(storage, |root| edit::delete_node(root, rel, flags, emit, create, rebuild, recycle))
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
        locked(&storage, |root| mutations::delete_file(root, &rel_parts, |p| super::transfer::recycle_tracked(p, |p| held_trash::hold(root, p))))
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
pub async fn set_user_flag(app: AppHandle, rel_parts: Vec<String>, flag: Option<String>, on: bool) -> R<Mutation> {
    let flag = flag.unwrap_or_else(|| crate::domain::config::DATA_FLAG.to_string());
    run_search_indexed(app, move |root| mutations::set_user_flag(root, &rel_parts, &flag, on)).await
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
pub async fn create_template(app: AppHandle, identifier: String, name: String, description: Option<String>) -> R<Vec<String>> {
    run_locked(app, move |root| mutations::create_template(root, &identifier, &name, description.as_deref())).await
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
pub async fn set_review_automation(app: AppHandle, rel_parts: Vec<String>, automation: String, on: bool) -> R<Mutation> {
    run_locked(app, move |root| mutations::set_review_automation(root, &rel_parts, &automation, on)).await
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn plan_automations(app: AppHandle, folder: Vec<String>, only: Vec<String>, reduce: bool, sort: bool, group: bool, add_date: bool, remove_date: bool, remove_symbols: bool) -> R<Vec<automations::PlannedFile>> {
    let picks = automations::Picks { reduce, sort, group, add_date, remove_date, remove_symbols };
    if let Some(sub) = super::downloads::below_downloads(&folder) {
        let (base, sub) = (super::downloads::base(&app)?, sub.to_vec());
        return run_locked(app, move |root| automations::downloads::plan(root, &base, &sub, &only, picks)).await;
    }
    run_locked(app, move |root| automations::plan(root, &folder, &only, picks)).await
}

#[tauri::command]
pub async fn apply_automations(app: AppHandle, folder: Vec<String>, rows: Vec<automations::PlannedFile>) -> R<automations::chain::ApplyOutcome> {
    if let Some(sub) = super::downloads::below_downloads(&folder) {
        let (base, sub) = (super::downloads::base(&app)?, sub.to_vec());
        return run_locked(app, move |root| automations::downloads::apply(root, &base, &sub, &rows)).await;
    }
    run_locked(app, move |root| automations::apply(root, &folder, &rows).map(|changed| automations::chain::ApplyOutcome { changed, chained: Vec::new() })).await
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
pub async fn reorder_plan(app: AppHandle, parent_rel_parts: Vec<String>) -> R<ReorderPlan> {
    run_unlocked(app, move |_, root| mutations::reorder_plan(root, &parent_rel_parts)).await
}

#[tauri::command]
pub async fn reorder_ids(app: AppHandle, parent_rel_parts: Vec<String>, targets: Vec<IdTarget>) -> R<Mutation> {
    run_indexed(app, move |root| mutations::reorder_ids(root, &parent_rel_parts, &targets)).await
}

#[tauri::command]
pub async fn move_folder(app: AppHandle, source: Vec<String>, destination: Vec<String>, copy: bool) -> R<Mutation> {
    let sender = app.clone();
    run_indexed(app, move |root| {
        let _progress = super::transfer::track(&sender, &[mutations::source_path(root, &source)?], &super::file_drop::place_name(root, &destination));
        let done = mutations::move_folder(root, &source, &destination, copy);
        crate::services::ops_log::outcome(root, if copy { "copy folder" } else { "move folder" }, &format!("{} → {}", source.join("/"), destination.join("/")), &done);
        done
    })
    .await
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
pub async fn merge_extra_review(app: AppHandle, rel_parts: Vec<String>) -> R<Mutation> {
    run_indexed(app, move |root| crate::services::upgrade::merge_extra_review(root, &rel_parts)).await
}

#[tauri::command]
pub async fn can_undo_project_sort(app: AppHandle) -> R<bool> {
    run_locked(app, |root| Ok(mutations::can_undo_project_sort(root))).await
}

#[tauri::command]
pub async fn undo_project_sort(app: AppHandle) -> R<Mutation> {
    run_indexed(app, mutations::undo_project_sort).await
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
        let root = storage.root()?;
        delete_flow(&storage, &rel_parts, flags, &emit, create_shortcuts, review_index::rebuild, |p| super::transfer::recycle_tracked(p, |p| held_trash::hold(&root, p)))
    })
    .await?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    rel_parts: Vec<String>,
    left: usize,
}

// Ctrl+Z after a deletion: the newest held item goes back where it was, and the tree reads it again from disk.
#[tauri::command]
pub async fn undo_delete(app: AppHandle) -> R<Option<Restored>> {
    run_locked(app, move |root| {
        let Some(back) = held_trash::undo_last(root)? else { return Ok(None) };
        let rel_parts = back.strip_prefix(root).map(|r| r.iter().map(|p| p.to_string_lossy().into_owned()).collect()).unwrap_or_default();
        Ok(Some(Restored { rel_parts, left: held_trash::pending(root) }))
    })
    .await
}
