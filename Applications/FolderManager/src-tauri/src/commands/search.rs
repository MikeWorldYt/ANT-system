// Search commands, answered from the index in ant.db instead of walking the folders.
use tauri::AppHandle;

use super::run_locked;
use crate::ant_core::search::{canonical_key, parse_query};
use crate::error::R;
use crate::storage::db::{Store, Suggestion};

#[tauri::command]
pub async fn search_nodes(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    run_locked(app, move |root| {
        let store = Store::open_or_create(root)?;
        store.search(&parse_query(&query))
    })
    .await
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
