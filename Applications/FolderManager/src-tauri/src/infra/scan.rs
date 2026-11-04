// Finds the storage root and lists the visible subfolders of a folder.
use std::path::{Path, PathBuf};

use crate::domain::config::{is_hidden_dir, SETTINGS_FOLDER};
use crate::domain::error::R;
use crate::infra::fs_safe::io_err;

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let abs = std::path::absolute(start).unwrap_or_else(|_| start.to_path_buf());
    if abs.file_name().is_some_and(|n| n == SETTINGS_FOLDER) {
        return abs.parent().map(Path::to_path_buf);
    }
    let parent = abs.parent();
    std::iter::once(Some(abs.as_path()))
        .chain(std::iter::once(parent))
        .flatten()
        .find(|p| p.join(SETTINGS_FOLDER).is_dir())
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
        if !is_hidden_dir(&name) && entry.path().is_dir() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}
