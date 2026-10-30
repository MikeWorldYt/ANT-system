// Single-instance guard of `--watch`, kept outside the storage so no open file blocks renaming it.
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use crate::error::{FmError, R};
use crate::fs_safe::io_err;
use crate::storage::db::hash;

// Must match `identifier` in tauri.conf.json, so the guard sits in the same folder as the app's local data.
const APP_ID: &str = "com.mikeworldyt.foldermanager";

// The headless mode has no AppHandle, so this mirrors Tauri's app_local_data_dir from the environment.
pub(super) fn guard_dir() -> R<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))
    };
    let base = base.ok_or_else(|| FmError::Internal { message: "No local app data folder to keep the watch guard in".to_string() })?;
    Ok(base.join(APP_ID).join("watch"))
}

// Windows paths ignore case, so two spellings of one storage must share a guard.
pub(super) fn guard_name(root: &Path) -> String {
    let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let text = canonical.to_string_lossy();
    let key = if cfg!(windows) { text.to_lowercase() } else { text.into_owned() };
    format!("watch-{}.lock", hash(&key))
}

// An OS file lock dies with the process, so a crashed watcher never leaves a stale guard behind.
pub(super) fn acquire(dir: &Path, root: &Path) -> R<File> {
    std::fs::create_dir_all(dir).map_err(|e| io_err("create folder", dir, &e))?;
    let path = dir.join(guard_name(root));
    let file = OpenOptions::new().write(true).create(true).truncate(false).open(&path).map_err(|e| io_err("open watch guard", &path, &e))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(FmError::Io {
            op: "lock watch guard".to_string(),
            path: path.display().to_string(),
            message: "another watcher is already running for this storage".to_string(),
            os_code: None,
        }),
        Err(TryLockError::Error(e)) => Err(io_err("lock watch guard", &path, &e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_app_id_matches_the_tauri_identifier() {
        assert!(include_str!("../../tauri.conf.json").contains(&format!("\"identifier\": \"{APP_ID}\"")));
        assert!(guard_dir().unwrap().ends_with(Path::new(APP_ID).join("watch")));
    }

    #[test]
    fn the_guard_name_is_a_hash_of_the_canonical_root() {
        let dir = TempDir::new("watch-name");
        dir.mkdir("Store");
        let name = guard_name(&dir.path().join("Store"));
        assert_eq!(guard_name(&dir.path().join("Store/../Store")), name);
        assert_ne!(guard_name(dir.path()), name);
        assert!(name.starts_with("watch-") && name.ends_with(".lock") && name.len() == "watch-.lock".len() + 16);
        #[cfg(windows)]
        assert_eq!(guard_name(&dir.path().join("STORE")), name);
    }

    #[test]
    fn the_guard_is_exclusive_and_leaves_the_storage_untouched() {
        let guards = TempDir::new("watch-guards");
        let storage = TempDir::new("watch-guarded");
        let held = acquire(guards.path(), storage.path()).unwrap();
        let busy = acquire(guards.path(), storage.path()).err().unwrap();
        assert!(busy.to_string().contains("another watcher"), "{busy}");
        drop(held);
        drop(acquire(guards.path(), storage.path()).unwrap());
        assert!(std::fs::read_dir(storage.path()).unwrap().next().is_none());
    }
}
