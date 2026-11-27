// Helpers the file-based AI clients (Claude Desktop, Codex, OpenCode) share: guarded read and write, PATH lookup.
use std::fs;
use std::path::{Path, PathBuf};

use super::internal;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{write_atomic, write_atomic_io};

pub(super) fn io_err(op: &str, path: &Path, e: std::io::Error) -> FmError {
    FmError::Io { op: op.into(), path: path.display().to_string(), message: e.to_string(), os_code: e.raw_os_error() }
}

pub(super) fn read_text(file: &Path) -> R<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err("read", file, e)),
    }
}

// The client rewrites its own file, so a change since the read wins and the user's previous file stays in the .bak.
pub(super) fn write_checked(file: &Path, read: Option<&str>, text: &str) -> R<()> {
    if read_text(file)?.as_deref() != read {
        return Err(internal(format!("{} changed while Folder Manager was updating it; nothing was written, try again", file.display())));
    }
    if let Some(previous) = read.filter(|t| !t.trim().is_empty()) {
        let backup = file.with_file_name(format!("{}.bak", file.file_name().unwrap_or_default().to_string_lossy()));
        write_atomic_io(&backup, previous.as_bytes()).map_err(|e| io_err("back up", &backup, e))?;
    }
    write_atomic(file, text.as_bytes())
}

pub(super) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

pub(super) fn find_on_path(names: &[&str]) -> Option<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).flat_map(|dir| names.iter().map(move |n| dir.join(n))).find(|p| p.is_file())
}

pub(super) fn on_path(names: &[&str]) -> bool {
    find_on_path(names).is_some()
}

pub(super) fn same_exe(a: &str, b: &str) -> bool {
    if cfg!(windows) { a.eq_ignore_ascii_case(b) } else { a == b }
}
