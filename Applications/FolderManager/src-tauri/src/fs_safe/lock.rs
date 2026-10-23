// Lock file that keeps two app instances from changing one storage at the same time.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::io_err;
use crate::config::{CORE_FOLDER, SETTINGS_FOLDER};
use crate::error::{FmError, R};

const LOCK_FILE: &str = ".FolderManager.lock";
const STALE_LOCK: Duration = Duration::from_secs(120);

// Serialises mutations across processes, which the in-process Mutex cannot see.
pub struct StorageLock {
    path: PathBuf,
}

impl StorageLock {
    pub fn acquire(root: &Path) -> R<Self> {
        let dir = root.join(SETTINGS_FOLDER).join(CORE_FOLDER);
        fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
        let path = dir.join(LOCK_FILE);
        let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(busy_error(&path)),
            Err(e) => return Err(io_err("lock storage", &path, &e)),
        };
        let lock = Self { path };
        std::io::Write::write_all(&mut file, std::process::id().to_string().as_bytes())
            .map_err(|e| io_err("lock storage", &lock.path, &e))?;
        Ok(lock)
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        // A lock file left behind is reported as stale by the next mutation, which tells the user what to do.
        let _ = fs::remove_file(&self.path);
    }
}

// An old lock may belong to a crashed run or to a slow one, so it is reported and never removed here.
fn busy_error(path: &Path) -> FmError {
    let age = fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
    let message = match age {
        Some(age) if age >= STALE_LOCK => format!(
            "the storage has been locked for {} minutes; if no other Folder Manager is open, delete this file and retry",
            age.as_secs() / 60
        ),
        _ => "another Folder Manager is changing this storage; retry in a moment".to_string(),
    };
    FmError::Io { op: "lock storage".to_string(), path: path.display().to_string(), message, os_code: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_store::test_support::TempDir;

    #[test]
    fn storage_lock_is_exclusive_and_released_on_drop() {
        let dir = TempDir::new("lock");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let busy = StorageLock::acquire(dir.path()).err().unwrap();
        assert!(matches!(&busy, FmError::Io { message, .. } if message.contains("another Folder Manager")), "{busy:?}");
        drop(held);
        drop(StorageLock::acquire(dir.path()).unwrap());
        assert!(!dir.path().join(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE).exists());
    }

    #[test]
    fn stale_storage_lock_is_reported_and_kept() {
        let dir = TempDir::new("lock-stale");
        let path = dir.path().join(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE);
        dir.mkdir("999.Settings/00.Core");
        let file = fs::File::create(&path).unwrap();
        file.set_modified(std::time::SystemTime::now() - Duration::from_secs(600)).unwrap();
        drop(file);
        let err = StorageLock::acquire(dir.path()).err().unwrap();
        assert!(matches!(&err, FmError::Io { message, .. } if message.contains("delete this file")), "{err:?}");
        assert!(path.is_file());
    }
}
