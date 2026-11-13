// Lock file that keeps two app instances from changing one storage at the same time.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::io_err;
use crate::domain::error::{FmError, R};
use crate::infra::storage::core_dir::data_dir;
use crate::infra::storage::db::hide;

pub(crate) const LOCK_FILE: &str = ".FolderManager.lock";
const STALE_LOCK: Duration = Duration::from_secs(120);
const LOCK_WAIT: Duration = Duration::from_secs(3);
const LOCK_POLL: Duration = Duration::from_millis(50);
const LOCK_OP: &str = "lock storage";
const BUSY: &str = "another Folder Manager is changing this storage; retry in a moment";

// Serialises mutations across processes, which the in-process Mutex cannot see.
pub struct StorageLock {
    path: PathBuf,
}

impl StorageLock {
    pub fn acquire(root: &Path) -> R<Self> {
        Self::acquire_within(root, LOCK_WAIT)
    }

    // A lock held for a few milliseconds by another instance is waited out; an old one is reported at once.
    pub fn acquire_within(root: &Path, wait: Duration) -> R<Self> {
        let started = std::time::Instant::now();
        loop {
            match Self::try_acquire(root) {
                Ok(lock) => return Ok(lock),
                Err(Attempt::Failed(e)) => return Err(e),
                Err(Attempt::Held(path)) if started.elapsed() >= wait || lock_age(&path) >= Some(STALE_LOCK) => return Err(busy_error(&path)),
                Err(Attempt::Held(_)) => std::thread::sleep(LOCK_POLL),
            }
        }
    }

    fn try_acquire(root: &Path) -> Result<Self, Attempt> {
        let dir = data_dir(root);
        if !dir.is_dir() {
            fs::create_dir_all(&dir).map_err(|e| Attempt::Failed(io_err("create folder", &dir, &e)))?;
            hide(&dir);
        }
        let path = dir.join(LOCK_FILE);
        let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Err(Attempt::Held(path)),
            Err(e) => return Err(Attempt::Failed(io_err(LOCK_OP, &path, &e))),
        };
        let lock = Self { path };
        std::io::Write::write_all(&mut file, std::process::id().to_string().as_bytes())
            .map_err(|e| Attempt::Failed(io_err(LOCK_OP, &lock.path, &e)))?;
        Ok(lock)
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        // A lock file left behind is reported as stale by the next mutation, which tells the user what to do.
        let _ = fs::remove_file(&self.path);
    }
}

enum Attempt {
    Held(PathBuf),
    Failed(FmError),
}

fn lock_age(path: &Path) -> Option<Duration> {
    fs::metadata(path).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok())
}

// An old lock may belong to a crashed run or to a slow one, so it is reported and never removed here.
fn busy_error(path: &Path) -> FmError {
    let message = match lock_age(path) {
        Some(age) if age >= STALE_LOCK => format!(
            "the storage has been locked for {} minutes; if no other Folder Manager is open, press Refresh to unlock it",
            age.as_secs() / 60
        ),
        _ => BUSY.to_string(),
    };
    FmError::Io { op: LOCK_OP.to_string(), path: path.display().to_string(), message, os_code: None }
}

// The manual refresh lifts a lock left by a crash, so no one has to find and delete the file; a lock younger than the wait may be a live change and stays.
pub(crate) fn clear_stale(root: &Path) -> R<bool> {
    let path = data_dir(root).join(LOCK_FILE);
    if !lock_age(&path).is_some_and(|age| age >= LOCK_WAIT) {
        return Ok(false);
    }
    fs::remove_file(&path).map_err(|e| io_err(LOCK_OP, &path, &e))?;
    Ok(true)
}

// Only a lock held by a live change is worth waiting for again; a stale one needs the user.
pub(crate) fn is_busy(err: &FmError) -> bool {
    matches!(err, FmError::Io { op, message, .. } if op == LOCK_OP && message == BUSY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refresh_clears_an_old_lock_and_leaves_a_fresh_one() {
        let dir = crate::test_support::TempDir::new("lock-clear");
        let path = data_dir(dir.path()).join(LOCK_FILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "1").unwrap();
        assert!(!clear_stale(dir.path()).unwrap() && path.exists());
        fs::File::options().write(true).open(&path).unwrap().set_modified(std::time::SystemTime::now() - LOCK_WAIT * 2).unwrap();
        assert!(clear_stale(dir.path()).unwrap() && !path.exists());
        assert!(StorageLock::acquire(dir.path()).is_ok());
    }
    use crate::test_support::TempDir;

    #[test]
    fn storage_lock_is_exclusive_and_released_on_drop() {
        let dir = TempDir::new("lock");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let busy = StorageLock::acquire_within(dir.path(), Duration::ZERO).err().unwrap();
        assert!(matches!(&busy, FmError::Io { message, .. } if message.contains("another Folder Manager")), "{busy:?}");
        drop(held);
        drop(StorageLock::acquire(dir.path()).unwrap());
        assert!(!dir.path().join(".ant").join(LOCK_FILE).exists());
    }

    #[test]
    fn a_short_lock_is_waited_out() {
        let dir = TempDir::new("lock-wait");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            drop(held);
        });
        let started = std::time::Instant::now();
        drop(StorageLock::acquire_within(dir.path(), Duration::from_secs(5)).unwrap());
        assert!(started.elapsed() >= Duration::from_millis(100));
        releaser.join().unwrap();
    }

    #[test]
    fn stale_storage_lock_is_reported_and_kept() {
        let dir = TempDir::new("lock-stale");
        let path = dir.path().join(".ant").join(LOCK_FILE);
        dir.mkdir(".ant");
        let file = fs::File::create(&path).unwrap();
        file.set_modified(std::time::SystemTime::now() - Duration::from_secs(600)).unwrap();
        drop(file);
        let err = StorageLock::acquire(dir.path()).err().unwrap();
        assert!(matches!(&err, FmError::Io { message, .. } if message.contains("press Refresh")), "{err:?}");
        assert!(path.is_file());
    }
}
