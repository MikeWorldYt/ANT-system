// Commands of the Extensions settings page; downloads run outside the storage lock so a slow network never blocks the tree.
use tauri::AppHandle;

use super::run_locked;
use crate::error::{FmError, R};
use crate::extensions::{self, ExtensionInfo, ExtensionPreview, RegistryEntry};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> R<T> + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| FmError::Internal { message: e.to_string() })?
}

#[tauri::command]
pub async fn list_extensions(app: AppHandle) -> R<Vec<ExtensionInfo>> {
    run_locked(app, |root| extensions::list(root)).await
}

#[tauri::command]
pub async fn set_extension_enabled(app: AppHandle, id: String, enabled: bool) -> R<()> {
    run_locked(app, move |root| extensions::set_enabled(root, &id, enabled)).await
}

#[tauri::command]
pub async fn read_extension_entry(app: AppHandle, id: String) -> R<String> {
    run_locked(app, move |root| extensions::read_entry(root, &id)).await
}

#[tauri::command]
pub async fn remove_extension(app: AppHandle, id: String) -> R<()> {
    run_locked(app, move |root| extensions::remove(root, &id)).await
}

#[tauri::command]
pub async fn list_registry(app: AppHandle) -> R<Vec<RegistryEntry>> {
    let items = blocking(extensions::fetch_registry).await?;
    run_locked(app, move |root| Ok(extensions::entries(root, items))).await
}

#[tauri::command]
pub async fn install_registry_extension(app: AppHandle, id: String) -> R<ExtensionInfo> {
    extensions::check_id(&id)?;
    let (item, bytes) = blocking(move || {
        let item = extensions::fetch_registry()?
            .into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| FmError::Internal { message: format!("{id} is not in the extension library") })?;
        let bytes = extensions::archive(&item)?;
        Ok((item, bytes))
    })
    .await?;
    run_locked(app, move |root| extensions::from_registry(root, &item, &bytes)).await
}

#[tauri::command]
pub async fn inspect_extension_repo(app: AppHandle, url: String) -> R<ExtensionPreview> {
    let (owner, repo) = extensions::parse_repo_url(&url).ok_or_else(|| FmError::Internal {
        message: "Paste a GitHub address like https://github.com/owner/repo".to_string(),
    })?;
    let canonical = extensions::repo_url(&owner, &repo);
    let bytes = blocking(move || extensions::repo_zip(&owner, &repo)).await?;
    run_locked(app, move |root| extensions::stage_repo(root, &canonical, &bytes)).await
}

#[tauri::command]
pub async fn install_inspected_extension(app: AppHandle, token: String) -> R<ExtensionInfo> {
    run_locked(app, move |root| extensions::from_staging(root, &token)).await
}
