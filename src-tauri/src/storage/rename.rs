// Renaming the Storage root folder, with a log under .ant/debug that tells which rename can be undone.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::debug_log::{logs_dir, valid_stamp};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, validate_component};

const SUFFIX: &str = "_ANT-fm-storage-renamed";
const IN_USE: &str = "The Storage folder could not be renamed because a program is using a file inside it. \
    Close every program that is using the Storage (Explorer windows, editors, this app if it was started \
    from a file inside the Storage) and try again";

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameStatus {
    pub old: String,
    pub new: String,
    pub revertible: bool,
}

fn invalid(name: &str, reason: &str) -> FmError {
    FmError::InvalidName { name: name.to_string(), reason: reason.to_string() }
}

pub fn folder_name(root: &Path) -> String {
    root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn parent_of(root: &Path) -> R<&Path> {
    root.parent().ok_or_else(|| FmError::Internal { message: format!("{} has no parent folder", root.display()) })
}

pub fn log_text(old: &str, new: &str) -> String {
    format!("# Storage renamed\n\n| | |\n| --- | --- |\n| Old name | {old} |\n| New name | {new} |\n")
}

pub fn parse_log(text: &str) -> Option<(String, String)> {
    let row = |key: &str| {
        text.lines().find_map(|line| {
            let value = line.trim().strip_prefix(&format!("| {key} |"))?.strip_suffix('|')?.trim();
            validate_component(value).is_ok().then(|| value.to_string())
        })
    };
    Some((row("Old name")?, row("New name")?))
}

fn is_log_name(name: &str) -> bool {
    let Some((stamp, rest)) = name.split_once(SUFFIX) else { return false };
    valid_stamp(stamp) && rest.ends_with(".md")
}

fn newest_log(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_log_name(n))
        .max()
        .map(|n| dir.join(n))
}

fn free_log(dir: &Path, stamp: &str) -> PathBuf {
    let first = dir.join(format!("{stamp}{SUFFIX}.md"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stamp}{SUFFIX}_{n}.md"))).find(|p| !p.exists()).unwrap_or(first)
}

pub fn write_log(root: &Path, stamp: &str, old: &str, new: &str) -> R<PathBuf> {
    let dir = logs_dir(root);
    fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
    let path = free_log(&dir, stamp);
    let mut file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| io_err("create file", &path, &e))?;
    file.write_all(log_text(old, new).as_bytes()).map_err(|e| io_err("write file", &path, &e))?;
    Ok(path)
}

pub fn status(root: &Path) -> Option<RenameStatus> {
    let text = fs::read_to_string(newest_log(&logs_dir(root))?).ok()?;
    let (old, new) = parse_log(&text)?;
    let revertible = folder_name(root) == new && root.parent().is_some_and(|p| !p.join(&old).exists());
    Some(RenameStatus { old, new, revertible })
}

pub fn check_stamp(stamp: &str) -> R<()> {
    if valid_stamp(stamp) {
        return Ok(());
    }
    Err(FmError::Internal { message: format!("Invalid rename stamp {stamp}") })
}

// Windows refuses to rename a folder that is the working directory of a running process, this one included.
pub fn rename_root(root: &Path, new_name: &str) -> R<PathBuf> {
    validate_component(new_name)?;
    if folder_name(root) == new_name {
        return Err(invalid(new_name, "the Storage already has this name"));
    }
    let parent = parent_of(root)?;
    let target = parent.join(new_name);
    if target.exists() {
        return Err(invalid(new_name, "a folder with this name already exists next to the Storage"));
    }
    if std::env::current_dir().is_ok_and(|cwd| cwd.starts_with(root)) {
        std::env::set_current_dir(parent).map_err(|e| io_err("change directory", parent, &e))?;
    }
    fs::rename(root, &target).map_err(|e| FmError::Internal { message: format!("{IN_USE} ({e})") })?;
    Ok(target)
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod tests;
