// Commands of the Storage settings page and the Storage picker: switch, create, rename the root folder and the daily sync task.
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, WebviewWindow};

use super::{off_thread, run_locked, run_unlocked};
use crate::app::storage_session::Storage;
use crate::services::rename::{rename_flow, revert_target};
use crate::domain::error::{FmError, R};
use crate::infra::storage::rename::{self, RenameStatus};
use crate::infra::storage::{known, location, pointer};
use crate::services::backup::{schedule, DEFAULT_FREQUENCY};
use crate::services::{scheduler, sync, updater};

pub fn window_title(name: &str) -> String {
    format!("ANT System Manager v.{} ({name})", updater::CURRENT_VERSION)
}

// The old tasks hold a path that is gone and would only fail unseen; the frontend then creates the new ones.
fn retire_tasks(old: &Path) {
    let backup = schedule::apply(false, DEFAULT_FREQUENCY, old);
    if let Err(e) = backup.and(sync::apply(false, old)) {
        eprintln!("Could not remove the scheduled tasks of {}: {e}", old.display());
    }
}

fn rename_and_retire(storage: &Storage, stamp: &str, target: impl FnOnce(&Path) -> R<String>, retire: impl FnOnce(&Path)) -> R<PathBuf> {
    let old = storage.root()?;
    let new = rename_flow(storage, stamp, target)?;
    retire(&old);
    Ok(new)
}

async fn renamed<F>(window: WebviewWindow, stamp: String, target: F) -> R<String>
where
    F: FnOnce(&Path) -> R<String> + Send + 'static,
{
    switched(window, move |storage| rename_and_retire(storage, &stamp, target, retire_tasks)).await
}

// Both a rename and a path change leave the window on a new root, which it then names and remembers.
async fn switched<F>(window: WebviewWindow, flow: F) -> R<String>
where
    F: FnOnce(&Storage) -> R<PathBuf> + Send + 'static,
{
    let app = window.app_handle().clone();
    let root = off_thread(move || flow(&app.state::<Storage>())).await??;
    Ok(adopt(&window, &root))
}

fn adopt(window: &WebviewWindow, root: &Path) -> String {
    if let Err(e) = window.set_title(&window_title(&rename::folder_name(root))) {
        eprintln!("Could not update the window title: {e}");
    }
    pointer::remember(window.app_handle(), root);
    root.to_string_lossy().into_owned()
}

pub fn change_flow(storage: &Storage, text: &str) -> R<PathBuf> {
    let path = location::clean_path(text)?;
    let root = location::storage_root(&path)?;
    let _guard = storage.guard();
    storage.set_root(root.clone());
    storage.set_start(path);
    Ok(root)
}

#[tauri::command]
pub async fn change_storage_path(window: WebviewWindow, path: String) -> R<String> {
    switched(window, move |storage| change_flow(storage, &path)).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownStorage {
    path: String,
    name: String,
    exists: bool,
}

#[tauri::command]
pub async fn known_storages(app: AppHandle) -> R<Vec<KnownStorage>> {
    off_thread(move || {
        let file = known_file(&app)?;
        let list = known::read(&file).into_iter().map(|p| KnownStorage { name: rename::folder_name(&p), exists: crate::infra::scan::find_root(&p).is_some(), path: p.to_string_lossy().into_owned() });
        Ok(list.collect())
    })
    .await?
}

#[tauri::command]
pub async fn forget_storage(app: AppHandle, path: String) -> R<()> {
    off_thread(move || {
        let file = known_file(&app)?;
        known::forget(&file, Path::new(&path)).map_err(|e| crate::infra::fs_safe::io_err("update the Storage list", &file, &e))
    })
    .await?
}

fn known_file(app: &AppHandle) -> R<PathBuf> {
    known::app_file(app).map_err(|message| FmError::Internal { message })
}

// Only Storages on the list are touched, and never the open one, whose watchers and tasks hold its path.
fn listed_other(app: &AppHandle, path: &Path) -> R<PathBuf> {
    let file = known_file(app)?;
    if !known::listed(&file, path) {
        return Err(FmError::Internal { message: "That Storage is not on the list.".to_string() });
    }
    if app.state::<Storage>().root().is_ok_and(|r| r.to_string_lossy().eq_ignore_ascii_case(&path.to_string_lossy())) {
        return Err(FmError::Internal { message: "Switch to another Storage before changing this one.".to_string() });
    }
    Ok(file)
}

fn relocate_listed(app: &AppHandle, from: &Path, to: &Path) -> R<String> {
    let file = listed_other(app, from)?;
    location::relocate(from, to)?;
    known::replace(&file, from, to).map_err(|e| crate::infra::fs_safe::io_err("update the Storage list", &file, &e))?;
    Ok(to.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn rename_listed_storage(app: AppHandle, path: String, name: String) -> R<String> {
    off_thread(move || {
        let from = PathBuf::from(&path);
        let parent = from.parent().ok_or_else(|| FmError::Internal { message: "A drive cannot be renamed.".to_string() })?;
        relocate_listed(&app, &from, &parent.join(location::check_name(&name)?))
    })
    .await?
}

#[tauri::command]
pub async fn move_listed_storage(app: AppHandle, path: String, destination: String) -> R<String> {
    off_thread(move || {
        let from = PathBuf::from(&path);
        let name = from.file_name().ok_or_else(|| FmError::Internal { message: "A drive cannot be moved.".to_string() })?.to_owned();
        relocate_listed(&app, &from, &location::clean_path(&destination)?.join(name))
    })
    .await?
}

#[tauri::command]
pub async fn reveal_storage(app: AppHandle, path: String) -> R<()> {
    off_thread(move || {
        let path = PathBuf::from(path);
        if !known::listed(&known_file(&app)?, &path) {
            return Err(FmError::Internal { message: "That Storage is not on the list.".to_string() });
        }
        super::open::revealer(&path).spawn().map(|_| ()).map_err(|e| FmError::Internal { message: format!("Could not show the folder: {e}") })
    })
    .await?
}

#[derive(serde::Serialize)]
pub struct StorageLocation {
    label: String,
    path: String,
}

#[tauri::command]
pub fn storage_locations(app: AppHandle) -> Vec<StorageLocation> {
    location::locations(app.path().home_dir().ok()).into_iter().map(|(label, p)| StorageLocation { label, path: p.to_string_lossy().into_owned() }).collect()
}

#[tauri::command]
pub async fn create_storage(window: WebviewWindow, parent: String, name: String) -> R<String> {
    switched(window, move |storage| {
        let root = location::create(&location::clean_path(&parent)?, &name)?;
        change_flow(storage, &root.to_string_lossy())
    })
    .await
}

#[tauri::command]
pub async fn rename_storage(window: WebviewWindow, new_name: String, stamp: String) -> R<String> {
    renamed(window, stamp, move |_| Ok(new_name)).await
}

#[tauri::command]
pub async fn revert_storage_rename(window: WebviewWindow, stamp: String) -> R<String> {
    renamed(window, stamp, revert_target).await
}

#[tauri::command]
pub async fn storage_rename_status(app: AppHandle) -> R<Option<RenameStatus>> {
    run_unlocked(app, |_, root| Ok(rename::status(root))).await
}

#[tauri::command]
pub async fn sync_schedule(app: AppHandle, on: bool) -> R<()> {
    run_locked(app, move |root| sync::apply(on, root)).await
}

// Read only, so a check that waits on PowerShell never holds up a change.
#[tauri::command]
pub async fn scheduled_task_problems(app: AppHandle) -> R<Vec<String>> {
    run_unlocked(app, |_, root| scheduler::problems(&[&schedule::TASK, &sync::TASK, &sync::WATCH_TASK], root)).await
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::error::FmError;
    use crate::test_support::{storage_in, TempDir};

    const STAMP: &str = "26-10-24-09-05";

    #[test]
    fn a_rename_retires_the_tasks_of_the_old_path_and_a_failed_one_does_not() {
        let dir = TempDir::new("rename-retire");
        let old = storage_in(&dir, "A");
        let storage = Storage::resolve(old.clone());
        let retired = RefCell::new(Vec::new());
        let refuse = |_: &Path| -> R<String> { Err(FmError::Internal { message: "refused".to_string() }) };
        assert!(rename_and_retire(&storage, STAMP, refuse, |path| retired.borrow_mut().push(path.to_path_buf())).is_err());
        assert!(retired.borrow().is_empty());
        rename_and_retire(&storage, STAMP, |_| Ok("B".to_string()), |path| retired.borrow_mut().push(path.to_path_buf())).unwrap();
        assert_eq!(*retired.borrow(), [old]);
    }
}
