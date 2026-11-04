// Search commands, answered from the index in ant.db; the slow pass walks the folders on disk.
use tauri::{AppHandle, Manager};

use super::run_locked;
use crate::app::storage_session::Storage;
use crate::domain::search::{canonical_key, parse_query};
use crate::domain::error::{FmError, R};
use crate::services::index::disk_search;
use crate::infra::storage::db::{Store, Suggestion};

#[tauri::command]
pub async fn search_nodes(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    run_locked(app, move |root| {
        let store = Store::open_or_create(root)?;
        store.search(&parse_query(&query))
    })
    .await
}

#[tauri::command]
pub async fn list_favorites(app: AppHandle) -> R<Vec<Vec<String>>> {
    run_locked(app, |root| Store::open_or_create(root)?.favorites()).await
}

#[tauri::command]
pub async fn list_workboards(app: AppHandle) -> R<Vec<Vec<String>>> {
    run_locked(app, |root| Store::open_or_create(root)?.workboards()).await
}

// Runs off the storage lock: a broad walk must not hold up edits.
#[tauri::command]
pub async fn search_disk(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = app.state::<Storage>().root()?;
        disk_search::search_disk(&root, &parse_query(&query))
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}

#[tauri::command]
pub async fn suggest_search(app: AppHandle, key: String, prefix: String) -> R<Vec<Suggestion>> {
    run_locked(app, move |root| {
        let store = Store::open_or_create(root)?;
        let key = if key.eq_ignore_ascii_case("in") { "in".to_string() } else { canonical_key(&key) };
        store.suggest(&key, &prefix)
    })
    .await
}
