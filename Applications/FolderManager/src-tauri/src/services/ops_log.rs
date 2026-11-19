// An always-on trail of what was moved, copied, deleted or restored in the storage, kept in its .ant folder so it outlives temp cleanups.
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::config::DATA_DIR;
use crate::services::backup::stamp::local_stamp;

const FILE: &str = "operations.log";
const OLD: &str = "operations.old.log";
// Past this size the log starts over, keeping one older file.
const MAX_BYTES: u64 = 5 * 1024 * 1024;

fn path(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(FILE)
}

// A log that cannot be written must never stop the operation it describes, so failures only reach stderr.
pub fn record(root: &Path, action: &str, detail: &str) {
    let file = path(root);
    if fs::metadata(&file).is_ok_and(|m| m.len() > MAX_BYTES) {
        if let Err(e) = fs::rename(&file, root.join(DATA_DIR).join(OLD)) {
            eprintln!("Could not rotate {}: {e}", file.display());
        }
    }
    let line = format!("{} | {action} | {detail}\n", local_stamp());
    let written = OpenOptions::new().create(true).append(true).open(&file).and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = written {
        eprintln!("Could not write {}: {e}", file.display());
    }
}

// Records the outcome too, so a failed attempt is as visible as a done one.
pub fn outcome<T, E: Display>(root: &Path, action: &str, detail: &str, result: &Result<T, E>) {
    match result {
        Ok(_) => record(root, action, detail),
        Err(e) => record(root, &format!("{action} failed"), &format!("{detail} ({e})")),
    }
}

pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn lines_are_appended_with_their_outcome() {
        let dir = TempDir::new("ops-log");
        fs::create_dir_all(dir.path().join(DATA_DIR)).unwrap();
        outcome::<(), String>(dir.path(), "delete", "100.A/01.B", &Ok(()));
        outcome::<(), String>(dir.path(), "move", "x → y", &Err("locked".into()));
        let text = fs::read_to_string(path(dir.path())).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].ends_with("| delete | 100.A/01.B"), "{text}");
        assert!(lines[1].ends_with("| move failed | x → y (locked)"), "{text}");
    }
}
