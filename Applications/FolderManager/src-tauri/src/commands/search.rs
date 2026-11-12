// Search commands, answered from the index in ant.db; the slow pass walks the folders on disk.
use tauri::{AppHandle, Manager};

use super::{off_thread, run_unlocked};
use crate::app::storage_session::Storage;
use crate::domain::search::{parse_query, suggest_key};
use crate::domain::error::R;
use crate::services::index::disk_search::{self, FileHit};
use crate::infra::storage::db::{Store, Suggestion};

#[tauri::command]
pub async fn search_nodes(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    run_unlocked(app, move |storage, _| storage.with_reader(|store| store.search(&parse_query(&query)))).await
}

#[tauri::command]
pub async fn list_favorites(app: AppHandle) -> R<Vec<Vec<String>>> {
    run_unlocked(app, |storage, _| storage.with_reader(Store::favorites)).await
}

#[tauri::command]
pub async fn list_review(app: AppHandle) -> R<Vec<Vec<String>>> {
    run_unlocked(app, |storage, _| storage.with_reader(Store::reviews)).await
}

#[tauri::command]
pub async fn list_workboards(app: AppHandle) -> R<Vec<Vec<String>>> {
    run_unlocked(app, |storage, _| storage.with_reader(Store::workboards)).await
}

// Runs off the storage lock: a broad walk must not hold up edits.
#[tauri::command]
pub async fn search_disk(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    off_thread(move || {
        let root = app.state::<Storage>().root()?;
        disk_search::search_disk(&root, &parse_query(&query))
    })
    .await?
}

// Same walk as search_disk, matching file names inside the folders.
#[tauri::command]
pub async fn search_files(app: AppHandle, query: String) -> R<Vec<FileHit>> {
    off_thread(move || {
        let root = app.state::<Storage>().root()?;
        disk_search::search_files(&root, &parse_query(&query))
    })
    .await?
}

#[tauri::command]
pub async fn suggest_search(app: AppHandle, key: String, prefix: String) -> R<Vec<Suggestion>> {
    run_unlocked(app, move |storage, _| storage.with_reader(|store| store.suggest(&suggest_key(&key), &prefix))).await
}
