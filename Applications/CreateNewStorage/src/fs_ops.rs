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
    fn atomic_write_replaces_existing_file() {
        let tmp = TempDir::new("atomic");
        let file = tmp.path().join("Tree.yaml");
        write_file_atomic(&file, b"old").unwrap();
        write_file_atomic(&file, b"new").unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"new");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }
}
