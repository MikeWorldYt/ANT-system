// Deletions wait in the storage's hidden .ant-trash for two hours, so Ctrl+Z can put them back; then, or when the app closes, they go to the Recycle Bin.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::domain::config::TRASH_DIR;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::io_err;

pub const HOLD: Duration = Duration::from_secs(2 * 60 * 60);
const ORIGIN: &str = "origin.txt";

fn trash(root: &Path) -> PathBuf {
    root.join(TRASH_DIR)
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default()
}

// Each deletion gets a folder named for its time, holding the item and the path it came from.
pub fn hold(root: &Path, path: &Path) -> Result<(), String> {
    let rel = path.strip_prefix(root).map_err(|_| format!("{} is outside the storage", path.display()))?;
    let name = path.file_name().ok_or_else(|| format!("{} has no name", path.display()))?;
    let bin = trash(root);
    let fresh = !bin.exists();
    let entry = (0..)
        .map(|n| bin.join(format!("{}-{n}", now_ms())))
        .find(|p| !p.exists())
        .ok_or("no free name in the trash")?;
    fs::create_dir_all(&entry).map_err(|e| io_err("create the trash entry", &entry, &e).to_string())?;
    if fresh {
        crate::infra::storage::db::hide(&bin);
    }
    fs::write(entry.join(ORIGIN), rel.to_string_lossy().as_bytes()).map_err(|e| io_err("note the origin", &entry, &e).to_string())?;
    if let Err(e) = fs::rename(path, entry.join(name)) {
        if let Err(r) = fs::remove_dir_all(&entry) {
            eprintln!("Could not remove the empty trash entry {}: {r}", entry.display());
        }
        return Err(io_err("move to the trash", path, &e).to_string());
    }
    Ok(())
}

struct Entry {
    dir: PathBuf,
    item: PathBuf,
    origin: PathBuf,
    held_ms: u128,
}

fn entries(root: &Path) -> Vec<Entry> {
    let mut found: Vec<Entry> = fs::read_dir(trash(root))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let dir = e.path();
            let held_ms = e.file_name().to_string_lossy().split('-').next()?.parse().ok()?;
            let origin = root.join(fs::read_to_string(dir.join(ORIGIN)).ok()?.trim());
            let item = fs::read_dir(&dir).ok()?.flatten().map(|f| f.path()).find(|p| p.file_name().is_some_and(|n| n != ORIGIN))?;
            Some(Entry { dir, item, origin, held_ms })
        })
        .collect();
    found.sort_by_key(|e| e.held_ms);
    found
}

pub fn pending(root: &Path) -> usize {
    entries(root).len()
}

// The newest deletion goes back where it was; its parent folder is made again if it went too.
pub fn undo_last(root: &Path) -> R<Option<PathBuf>> {
    let Some(last) = entries(root).pop() else { return Ok(None) };
    if last.origin.exists() {
        return Err(FmError::Internal { message: format!("{} already exists, so the deleted item was not put back", last.origin.display()) });
    }
    if let Some(parent) = last.origin.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("make the folder again", parent, &e))?;
    }
    fs::rename(&last.item, &last.origin).map_err(|e| io_err("put back from the trash", &last.item, &e))?;
    if let Err(e) = fs::remove_dir_all(&last.dir) {
        eprintln!("Could not remove the trash entry {}: {e}", last.dir.display());
    }
    Ok(Some(last.origin))
}

// Sends held items older than `older_than` (every item when None) to the Recycle Bin; one that will not go stays for the next pass.
pub fn flush(root: &Path, older_than: Option<Duration>, recycle: impl Fn(&Path) -> Result<(), String>) -> Vec<String> {
    let limit = older_than.map(|d| now_ms().saturating_sub(d.as_millis()));
    let mut failures = Vec::new();
    for e in entries(root).into_iter().filter(|e| limit.is_none_or(|l| e.held_ms <= l)) {
        match recycle(&e.item) {
            Ok(()) => {
                if let Err(r) = fs::remove_dir_all(&e.dir) {
                    failures.push(format!("{}: {r}", e.dir.display()));
                }
            }
            Err(why) => failures.push(format!("{}: {why}", e.item.display())),
        }
    }
    failures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_held_item_comes_back_and_a_flushed_one_goes_to_the_recycler() {
        let dir = TempDir::new("held-trash");
        let root = dir.path();
        fs::create_dir_all(root.join("100.A/01.B")).unwrap();
        fs::write(root.join("100.A/01.B/x.txt"), "x").unwrap();
        hold(root, &root.join("100.A/01.B")).unwrap();
        assert!(!root.join("100.A/01.B").exists());
        assert_eq!(pending(root), 1);
        assert_eq!(undo_last(root).unwrap(), Some(root.join("100.A/01.B")));
        assert!(root.join("100.A/01.B/x.txt").is_file());
        assert_eq!(pending(root), 0);

        hold(root, &root.join("100.A/01.B/x.txt")).unwrap();
        let sent = std::cell::RefCell::new(Vec::new());
        assert!(flush(root, Some(HOLD), |p| { sent.borrow_mut().push(p.to_path_buf()); Ok(()) }).is_empty());
        assert!(sent.borrow().is_empty(), "a fresh deletion waits out the hold");
        assert!(flush(root, None, |p| { sent.borrow_mut().push(p.to_path_buf()); fs::remove_file(p).map_err(|e| e.to_string()) }).is_empty());
        assert_eq!(sent.borrow().len(), 1);
        assert_eq!(pending(root), 0);
        assert_eq!(undo_last(root).unwrap(), None);
    }
}
