// Lock file that keeps two app instances from changing one storage at the same time.
use std::fs::{self, File, TryLockError};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use super::io_err;
use super::lock_os::{create_owned, pid_alive, release};
use crate::domain::error::{FmError, R};
use crate::infra::storage::core_dir::data_dir;
use crate::infra::storage::db::hide;
use crate::platform::random::hex_token;

pub(crate) const LOCK_FILE: &str = ".FolderManager.lock";
// Builds without a token hold their lock no longer than the 300 s Recycle Bin timeout, so an older token-less one is abandoned.
const STALE_LOCK: Duration = Duration::from_secs(600);
const LOCK_WAIT: Duration = Duration::from_secs(3);
const LOCK_POLL: Duration = Duration::from_millis(50);
const LOCK_OP: &str = "lock storage";
const BUSY: &str = "another Folder Manager is changing this storage; retry in a moment";
const STALE: &str = "the storage was left locked by a Folder Manager that is no longer running; press Refresh to unlock it";

// Serialises mutations across processes, which the in-process Mutex cannot see; the OS lock on `file` ends with its process.
pub struct StorageLock {
    path: PathBuf,
    file: File,
    stamp: String,
}

impl StorageLock {
    pub fn acquire(root: &Path) -> R<Self> {
        Self::acquire_within(root, LOCK_WAIT)
    }

    // A lock held by a live change is waited out; one whose owner died is reported at once.
    pub fn acquire_within(root: &Path, wait: Duration) -> R<Self> {
        let started = Instant::now();
        loop {
            let late = started.elapsed() >= wait;
            match Self::try_acquire(root) {
                Ok(lock) => return Ok(lock),
                Err(Attempt::Failed(e)) => return Err(e),
                Err(Attempt::Held(path)) if probe(&path).is_some_and(|p| p.dead) => return Err(lock_error(&path, STALE)),
                Err(Attempt::Held(path)) if late => return Err(lock_error(&path, BUSY)),
                Err(Attempt::Pending(e)) if late => return Err(e),
                Err(_) => std::thread::sleep(LOCK_POLL),
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
        let token = hex_token(16).map_err(|e| Attempt::Failed(io_err(LOCK_OP, &path, &e)))?;
        let file = match create_owned(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => return Err(Attempt::Held(path)),
            // A lock file being deleted refuses access on Windows until its last handle closes.
            Err(e) if e.kind() == ErrorKind::PermissionDenied => return Err(Attempt::Pending(io_err(LOCK_OP, &path, &e))),
            Err(e) => return Err(Attempt::Failed(io_err(LOCK_OP, &path, &e))),
        };
        // A probe holds the OS lock only for an instant; where the filesystem has no locks, the PID alone is written, as older builds did.
        let stamp = match file.lock() {
            Ok(()) => format!("{} {token}", std::process::id()),
            Err(_) => std::process::id().to_string(),
        };
        let mut lock = Self { path, file, stamp };
        lock.file.write_all(lock.stamp.as_bytes()).map_err(|e| Attempt::Failed(io_err(LOCK_OP, &lock.path, &e)))?;
        Ok(lock)
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        // A fresh mtime keeps a Refresh that opened the file a moment ago from judging it abandoned once unlocked.
        let released = self.file.set_modified(SystemTime::now()).and(release(&self.file, &self.path, &self.stamp));
        // A lock file left behind is reported by the next mutation and lifted by Refresh, so a failure is only logged.
        if let Err(e) = released {
            eprintln!("Could not remove the storage lock {}: {e}", self.path.display());
        }
    }
}

enum Attempt {
    Held(PathBuf),
    Pending(FmError),
    Failed(FmError),
}

struct Probe {
    _file: File,
    dead: bool,
}

// The OS lock answers for owners that took it; an unlocked file is judged by its age and, without a token, by its PID.
fn probe(path: &Path) -> Option<Probe> {
    let mut file = File::open(path).ok()?;
    let locked = match file.try_lock() {
        Ok(()) => true,
        Err(TryLockError::WouldBlock) => return Some(Probe { _file: file, dead: false }),
        Err(TryLockError::Error(_)) => false,
    };
    let age = file.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
    let mut text = Vec::new();
    let read = file.read_to_end(&mut text).is_ok();
    let dead = read && age.is_some_and(|age| age >= LOCK_WAIT && abandoned(&String::from_utf8_lossy(&text), locked, age));
    Some(Probe { _file: file, dead })
}

fn abandoned(text: &str, locked: bool, age: Duration) -> bool {
    let mut parts = text.split_whitespace();
    let pid = parts.next().and_then(|p| p.parse::<u32>().ok());
    match parts.next() {
        Some(_) if locked => true,
        _ => age >= STALE_LOCK || !pid.is_some_and(pid_alive),
    }
}

fn lock_error(path: &Path, message: &str) -> FmError {
    FmError::Io { op: LOCK_OP.to_string(), path: path.display().to_string(), message: message.to_string(), os_code: None }
}

// The manual refresh lifts a lock whose owner died, so no one has to find and delete the file; a live owner's lock always stays.
pub(crate) fn clear_stale(root: &Path) -> R<bool> {
    let path = data_dir(root).join(LOCK_FILE);
    let Some(probe) = probe(&path).filter(|p| p.dead) else { return Ok(false) };
    // Removed while the probe still holds the OS lock, so a second Refresh cannot judge and remove a newer lock in between.
    fs::remove_file(&path).map_err(|e| io_err(LOCK_OP, &path, &e))?;
    drop(probe);
    Ok(true)
}

// Only a lock held by a live change is worth waiting for again; a stale one needs the user.
pub(crate) fn is_busy(err: &FmError) -> bool {
    matches!(err, FmError::Io { op, message, .. } if op == LOCK_OP && message == BUSY)
}

#[cfg(test)]
#[path = "lock_tests.rs"]
mod tests;
