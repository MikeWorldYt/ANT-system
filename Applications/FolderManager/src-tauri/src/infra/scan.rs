// Finds the storage root and lists the visible subfolders of a folder.
use std::path::{Path, PathBuf};

use crate::domain::config::{is_hidden_dir, BACKUP_FOLDER, DATA_DIR, DB_FILENAME, FOOTPRINT_FILENAME, INFO_FILENAME, SETTINGS_FOLDER};
use crate::domain::error::R;
use crate::infra::fs_safe::io_err;

// A root has its own .ant/ (older storages: 999.Settings); a node's .ant/ holds only its info and drag files, so it does not count.
pub fn is_storage_root(dir: &Path) -> bool {
    let data = dir.join(DATA_DIR);
    dir.join(SETTINGS_FOLDER).is_dir()
        || (data.is_dir() && (data.join(DB_FILENAME).is_file() || !(data.join(INFO_FILENAME).exists() || data.join(FOOTPRINT_FILENAME).exists())))
}

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let abs = std::path::absolute(start).unwrap_or_else(|_| start.to_path_buf());
    if abs.file_name().is_some_and(|n| n == SETTINGS_FOLDER) {
        return abs.parent().map(Path::to_path_buf);
    }
    let parent = abs.parent();
    std::iter::once(Some(abs.as_path()))
        .chain(std::iter::once(parent))
        .flatten()
        .find(|p| is_storage_root(p))
        .map(Path::to_path_buf)
}

pub fn list_subdirs(dir: &Path) -> R<Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(Vec::new()),
        Err(e) => return Err(io_err("list folder", dir, &e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // The root's backup folder is the app's, so its views never list it.
        if !is_hidden_dir(&name) && entry.path().is_dir() && !(name.eq_ignore_ascii_case(BACKUP_FOLDER) && is_storage_root(dir)) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_root_has_its_own_ant_folder_while_a_node_data_folder_does_not_count() {
        let dir = TempDir::new("scan-root");
        dir.mkdir("100.A/.ant");
        std::fs::write(dir.path().join("100.A/.ant/info.json"), "{}").unwrap();
        assert!(!is_storage_root(&dir.path().join("100.A")) && find_root(&dir.path().join("100.A")).is_none());
        dir.mkdir(".ant");
        assert_eq!(find_root(&dir.path().join("100.A")).as_deref(), Some(dir.path()));
        let legacy = TempDir::new("scan-legacy");
        legacy.mkdir("999.Settings");
        assert!(is_storage_root(legacy.path()));
    }

    #[test]
    fn the_root_backup_folder_is_hidden_but_a_node_may_have_one() {
        let dir = TempDir::new("scan-backup");
        for d in [".ant", "backup", "100.A/backup"] {
            dir.mkdir(d);
        }
        assert_eq!(list_subdirs(dir.path()).unwrap(), ["100.A"]);
        assert_eq!(list_subdirs(&dir.path().join("100.A")).unwrap(), ["backup"]);
    }
}
