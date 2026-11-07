// Extensions the user turned on from this machine; the list stays outside the storage because ant.db travels with it.
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, write_atomic};
use crate::services::watch::{local_data_dir, storage_key};

const FILE: &str = "extensions-confirmed.txt";

pub(super) fn file() -> R<PathBuf> {
    Ok(local_data_dir()?.join(FILE))
}

fn read(file: &Path) -> R<String> {
    match fs::read_to_string(file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        other => other.map_err(|e| io_err("read file", file, &e)),
    }
}

fn prefix(root: &Path, id: &str) -> String {
    format!("{} {id} ", storage_key(root))
}

pub(super) fn listed(file: &Path, root: &Path, id: &str, fingerprint: &str) -> R<bool> {
    let line = format!("{}{fingerprint}", prefix(root, id));
    Ok(read(file)?.lines().any(|l| l == line))
}

fn others(text: &str, prefix: &str) -> String {
    text.lines().filter(|l| !l.starts_with(prefix)).map(|l| format!("{l}\n")).collect()
}

pub(super) fn add(file: &Path, root: &Path, id: &str, fingerprint: &str) -> R<()> {
    let prefix = prefix(root, id);
    let kept = others(&read(file)?, &prefix);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| io_err("create folder", dir, &e))?;
    }
    write_atomic(file, format!("{kept}{prefix}{fingerprint}\n").as_bytes())
}

pub(super) fn remove(file: &Path, root: &Path, id: &str) -> R<()> {
    let text = read(file)?;
    let kept = others(&text, &prefix(root, id));
    if kept == text {
        return Ok(());
    }
    write_atomic(file, kept.as_bytes())
}
