// Network backup folders the user set on this machine; the list stays outside the storage because ant.db travels with it.
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, write_atomic};
use crate::services::watch::local_data_dir;

const FILE: &str = "backup-folders.txt";

pub(super) fn file() -> R<PathBuf> {
    Ok(local_data_dir()?.join(FILE))
}

fn read(file: &Path) -> R<String> {
    match fs::read_to_string(file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        other => other.map_err(|e| io_err("read file", file, &e)),
    }
}

pub(super) fn listed(file: &Path, folder: &str) -> R<bool> {
    Ok(read(file)?.lines().any(|line| line == folder))
}

pub(super) fn add(file: &Path, folder: &str) -> R<()> {
    let text = read(file)?;
    if text.lines().any(|line| line == folder) {
        return Ok(());
    }
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| io_err("create folder", dir, &e))?;
    }
    write_atomic(file, format!("{text}{folder}\n").as_bytes())
}
