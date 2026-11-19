// Where a shortcut to a storage folder or file goes: a folder outside the storage, under a free name.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use super::{locate, Target};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::io_err;
use crate::infra::naming::free_name;
use crate::services::tree::tree_node::is_reserved;

#[cfg(windows)]
const LINK_EXT: &str = ".lnk";
#[cfg(not(windows))]
const LINK_EXT: &str = "";

// The link path comes without its extension, because create_shortcuts appends it.
pub fn shortcut_plan(root: &Path, data: &Mapping, target: &[String], dir: &Path) -> R<(PathBuf, PathBuf)> {
    let Target { abs, ghost, .. } = locate(root, data, target)?;
    if is_reserved(target) {
        return Err(FmError::Reserved { rel_parts: target.to_vec() });
    }
    if ghost {
        return Err(FmError::GhostNotAllowed { op: "create a shortcut to".to_string() });
    }
    link_in(root, abs, dir)
}

// Folders and files share these rules: the link goes into a folder outside the storage, under a free name.
pub fn link_in(root: &Path, abs: PathBuf, dir: &Path) -> R<(PathBuf, PathBuf)> {
    let refuse = |message: &str| FmError::Io {
        op: "create shortcut in".to_string(),
        path: dir.display().to_string(),
        message: message.to_string(),
        os_code: None,
    };
    let real_dir = fs::canonicalize(dir).map_err(|e| io_err("open folder", dir, &e))?;
    if !real_dir.is_dir() {
        return Err(refuse("not a folder"));
    }
    let real_root = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    if real_dir.starts_with(&real_root) {
        return Err(refuse("pick a folder outside the storage"));
    }
    let name = abs.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let file = free_name(dir, &format!("{name}{LINK_EXT}"), &[])?;
    let stem = file.strip_suffix(LINK_EXT).unwrap_or(&file);
    Ok((abs, dir.join(stem)))
}
