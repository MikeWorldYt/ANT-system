// Folder creation that remembers what this run made, so a failure removes only those folders.
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn make_dirs_tracked(path: &Path, created: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut missing = Vec::new();
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.as_os_str().is_empty() || dir.exists() {
            break;
        }
        missing.push(dir.to_path_buf());
        current = dir.parent();
    }
    fs::create_dir_all(path).map_err(|e| format!("Could not create {}: {e}", path.display()))?;
    created.extend(missing.into_iter().rev());
    Ok(())
}

// Deepest first; returns the folders that could not be removed and leaves only those in `created`.
pub fn rollback_created(created: &mut Vec<PathBuf>, log: &mut dyn FnMut(Result<&Path, (&Path, String)>)) -> Vec<PathBuf> {
    let mut remaining = Vec::new();
    for path in created.drain(..).rev() {
        match fs::remove_dir_all(&path) {
            Ok(()) => log(Ok(&path)),
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => {
                log(Err((&path, e.to_string())));
                remaining.push(path);
            }
        }
    }
    remaining.reverse();
    created.extend(remaining.iter().cloned());
    remaining
}

// Written next to the target and renamed over it, so a failure never leaves a half-written file.
pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or_else(|| format!("{} has no parent folder", path.display()))?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let tmp = dir.join(format!(".ant-{}-{stamp}.tmp", std::process::id()));
    let result = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|_| f.sync_all()))
        .and_then(|_| fs::rename(&tmp, path));
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let cleanup = match fs::remove_file(&tmp) {
                Ok(()) => String::new(),
                Err(c) if c.kind() == ErrorKind::NotFound => String::new(),
                Err(c) => format!(" (temporary file {} left: {c})", tmp.display()),
            };
            Err(format!("Could not write {}: {e}{cleanup}", path.display()))
        }
    }
}

// Existing files are copied aside before being replaced, so a failed run can put them back.
#[derive(Default)]
pub struct Backups(Vec<(PathBuf, PathBuf)>);

impl Backups {
    pub fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if path.exists() {
            let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
            let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
            name.push(format!(".ant-bak-{}-{stamp}", std::process::id()));
            let bak = path.with_file_name(name);
            fs::copy(path, &bak).map_err(|e| format!("Could not back up {} before replacing it: {e}", path.display()))?;
            self.0.push((path.to_path_buf(), bak));
        }
        write_file_atomic(path, bytes)
    }

    // Success: the replaced files are final and the copies go away; failures only leave a copy behind.
    pub fn discard(&mut self, log: &mut dyn FnMut(&Path, String)) {
        for (_, bak) in self.0.drain(..) {
            if let Err(e) = fs::remove_file(&bak) {
                log(&bak, format!("backup copy not removed: {e}"));
            }
        }
    }

    // Failure: every replaced file gets its old content back; returns the paths that could not be restored.
    pub fn restore(&mut self, log: &mut dyn FnMut(Result<&Path, (&Path, String)>)) -> Vec<PathBuf> {
        let mut failed = Vec::new();
        for (path, bak) in self.0.drain(..).rev() {
            match fs::rename(&bak, &path) {
                Ok(()) => log(Ok(&path)),
                Err(e) => {
                    log(Err((&path, format!("{e}; the old content is in {}", bak.display()))));
                    failed.push(path);
                }
            }
        }
        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TempDir;

    #[test]
    fn rollback_removes_only_created_folders_deepest_first() {
        let tmp = TempDir::new("rollback");
        let existing = tmp.path().join("existing");
        fs::create_dir(&existing).unwrap();
        let mut created = Vec::new();
        make_dirs_tracked(&existing.join("a").join("b"), &mut created).unwrap();
        make_dirs_tracked(&tmp.path().join("new"), &mut created).unwrap();
        assert_eq!(created.len(), 3);
        let mut removed = Vec::new();
        let left = rollback_created(&mut created, &mut |r| removed.push(r.map(Path::to_path_buf).map_err(|_| ())));
        assert!(left.is_empty() && created.is_empty());
        assert_eq!(removed[0], Ok(tmp.path().join("new")));
        assert_eq!(removed[1], Ok(existing.join("a").join("b")));
        assert!(existing.exists() && !existing.join("a").exists());
    }

    #[test]
    fn backups_restore_replaced_files_and_discard_leaves_the_new_ones() {
        let tmp = TempDir::new("backups");
        let (kept, fresh) = (tmp.path().join("a.md"), tmp.path().join("b.md"));
        fs::write(&kept, b"old").unwrap();
        let mut backups = Backups::default();
        backups.write(&kept, b"new").unwrap();
        backups.write(&fresh, b"x").unwrap();
        assert_eq!(fs::read(&kept).unwrap(), b"new");
        assert!(backups.restore(&mut |_| {}).is_empty());
        assert_eq!(fs::read(&kept).unwrap(), b"old");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 2);
        backups.write(&kept, b"newer").unwrap();
        backups.discard(&mut |_, _| {});
        assert_eq!(fs::read(&kept).unwrap(), b"newer");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 2);
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let tmp = TempDir::new("atomic");
        let file = tmp.path().join("ant.db");
        write_file_atomic(&file, b"old").unwrap();
        write_file_atomic(&file, b"new").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"new");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }
}
