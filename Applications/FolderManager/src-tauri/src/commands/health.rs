// Commands of the startup health check: the state of the Storage and its ant.db, and the ways to recover it.
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{run_locked, Storage};
use crate::backup::{self, LatestBackup};
use crate::error::{FmError, R};
use crate::storage::recovery::{self, db_state, DbState};

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StorageHealth {
    Ok { path: String },
    NoDatabase { path: String, backup: Option<LatestBackup> },
    NotFound { path: String },
}

pub fn health(start: &Path, root: Option<&Path>) -> StorageHealth {
    let Some(root) = root else { return StorageHealth::NotFound { path: start.display().to_string() } };
    let path = root.display().to_string();
    if db_state(root) == DbState::Healthy {
        return StorageHealth::Ok { path };
    }
    let backup = backup::newest_default(root).unwrap_or_else(|e| {
        eprintln!("Could not list the backups: {e}");
        None
    });
    StorageHealth::NoDatabase { path, backup }
}

// The storage lock is not taken: the check also runs when there is no Storage to lock.
#[tauri::command]
pub async fn storage_health(app: AppHandle) -> R<StorageHealth> {
    tauri::async_runtime::spawn_blocking(move || {
        let storage = app.state::<Storage>();
        health(&storage.start(), storage.root().ok().as_deref())
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })
}

#[tauri::command]
pub async fn use_latest_backup(app: AppHandle, stamp: String) -> R<()> {
    run_locked(app, move |root| backup::restore_latest(root, &stamp)).await
}

#[tauri::command]
pub async fn prepare_scan(app: AppHandle, stamp: String) -> R<()> {
    run_locked(app, move |root| recovery::prepare_scan(root, &stamp)).await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::DATA_DIR;
    use crate::storage::db::{db_path, Store};
    use crate::test_support::TempDir;

    #[test]
    fn the_health_check_tells_each_state_apart() {
        let dir = TempDir::new("health");
        let root = dir.path();
        let path = root.display().to_string();
        assert_eq!(health(Path::new("gone"), None), StorageHealth::NotFound { path: "gone".into() });
        assert_eq!(health(root, Some(root)), StorageHealth::NoDatabase { path: path.clone(), backup: None });
        assert!(!root.join(DATA_DIR).exists());
        dir.mkdir("999.Settings/03.Backup");
        fs::write(root.join("999.Settings/03.Backup/26-10-24-09-05_ANT-fm-backup.zip"), "").unwrap();
        dir.mkdir(DATA_DIR);
        fs::write(db_path(root), "not a database ".repeat(512)).unwrap();
        let backup = Some(LatestBackup { name: "26-10-24-09-05_ANT-fm-backup.zip".into(), label: "2026-10-24 09:05".into() });
        assert_eq!(health(root, Some(root)), StorageHealth::NoDatabase { path: path.clone(), backup });
        fs::remove_file(db_path(root)).unwrap();
        Store::open_or_create(root).unwrap();
        assert_eq!(health(root, Some(root)), StorageHealth::Ok { path });
    }

    #[test]
    fn health_states_serialize_with_a_state_tag() {
        let json = serde_yaml::to_string(&StorageHealth::NoDatabase { path: "p".into(), backup: None }).unwrap();
        assert!(json.contains("state: noDatabase") && json.contains("backup: null"), "{json}");
    }
}
