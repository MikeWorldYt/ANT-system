// The Customise tab of the Properties window: which support folders a node may hold, and adding or quitting them like any other edit.
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::edit::run_indexed;
use super::{locked, off_thread, review_emitter, run_locked};
use crate::app::storage_session::Storage;
use crate::domain::error::R;
use crate::platform::shortcuts::create_shortcuts;
use crate::services::index::review_index;
use crate::services::mutations::{self, Mutation, OutcomeDefaults, SupportRow};
use crate::services::{downloads, edit, held_trash};

#[derive(Serialize, Clone)]
struct Changed<'a> {
    paths: usize,
    folders: [&'a [String]; 1],
    rescan: bool,
    dictionary: bool,
}

// The edit runs in the Properties window, so the main window learns of it the way it learns of any change on disk.
fn announce(app: &AppHandle, node: &[String]) {
    let event = Changed { paths: 1, folders: [node], rescan: false, dictionary: false };
    if let Err(e) = app.emit(crate::STORAGE_CHANGED, event) {
        eprintln!("Could not send {}: {e}", crate::STORAGE_CHANGED);
    }
}

// Null when the item is not a storage node that can hold support folders, so the window shows no Customise tab.
#[tauri::command]
pub async fn support_nodes(app: AppHandle, rel_parts: Vec<String>) -> R<Option<Vec<SupportRow>>> {
    if downloads::split_root(&rel_parts).is_some() {
        return Ok(None);
    }
    run_locked(app, move |root| mutations::support_rows(root, &rel_parts)).await
}

// Null when the item is neither a node holding an Outcomes folder nor one, so the window shows no Worknode Settings.
#[tauri::command]
pub async fn outcome_defaults(app: AppHandle, rel_parts: Vec<String>) -> R<Option<OutcomeDefaults>> {
    if downloads::split_root(&rel_parts).is_some() {
        return Ok(None);
    }
    run_locked(app, move |root| mutations::outcome_defaults(root, &rel_parts)).await
}

#[tauri::command]
pub async fn add_support_node(app: AppHandle, rel_parts: Vec<String>, kind: String) -> R<Mutation> {
    let node = rel_parts.clone();
    let done = run_indexed(app.clone(), move |root| mutations::add_support(root, &rel_parts, &kind)).await?;
    announce(&app, &node);
    Ok(done)
}

#[tauri::command]
pub async fn remove_support_node(app: AppHandle, rel_parts: Vec<String>, name: String) -> R<Mutation> {
    let handle = app.clone();
    let node = rel_parts.clone();
    let done = off_thread(move || {
        let emit = review_emitter(&handle);
        let storage = handle.state::<Storage>();
        locked(&storage, |root| {
            edit::remove_support(root, &rel_parts, &name, &emit, create_shortcuts, review_index::rebuild, |p| super::transfer::recycle_tracked(p, |p| held_trash::hold(root, p)))
        })
    })
    .await??;
    announce(&app, &node);
    Ok(done)
}
