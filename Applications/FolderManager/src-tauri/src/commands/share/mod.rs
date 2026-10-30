// Hands a storage folder to other apps: on the Windows clipboard for Explorer, or as a shortcut outside the storage.
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::Mapping;
use tauri::AppHandle;

use super::clipboard;
use super::run_locked;
use crate::ant_core::shortcuts::create_shortcuts;
use crate::ant_core::tree_types::flags_in;
use crate::error::{FmError, R};
use crate::fs_safe::io_err;
use crate::mutations::{free_name, locate, Target};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, get_node};

#[cfg(windows)]
const LINK_EXT: &str = ".lnk";
#[cfg(not(windows))]
const LINK_EXT: &str = "";

#[derive(Debug, Serialize)]
pub struct ShortcutCreated {
    pub name: String,
    pub folder: String,
}

#[tauri::command]
pub async fn copy_folder(app: AppHandle, target: Vec<String>, cut: bool) -> R<()> {
    run_locked(app, move |root| {
        let data = tree_store::load(root)?;
        let Target { abs, ghost, node_type } = locate(root, &data, &target)?;
        let flags = get_node(&data, &target).map(flags_in).unwrap_or_default();
        let effect = clipboard::folder_effect(&target, &node_type, &flags, ghost, cut)?;
        clipboard::write(&abs, effect)
    })
    .await
}

#[tauri::command]
pub async fn create_shortcut_in(app: AppHandle, target: Vec<String>, folder: String) -> R<ShortcutCreated> {
    run_locked(app, move |root| {
        let data = tree_store::load(root)?;
        let (abs, link) = shortcut_plan(root, &data, &target, Path::new(&folder))?;
        let failures = create_shortcuts(&[(abs, link.clone())]);
        if !failures.is_empty() {
            return Err(FmError::Internal { message: failures.join("; ") });
        }
        let name = link.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        Ok(ShortcutCreated { name, folder })
    })
    .await
}

// The link path comes without its extension, because create_shortcuts appends it.
pub(super) fn shortcut_plan(root: &Path, data: &Mapping, target: &[String], dir: &Path) -> R<(PathBuf, PathBuf)> {
    let Target { abs, ghost, .. } = locate(root, data, target)?;
    if is_reserved(target) {
        return Err(FmError::Reserved { rel_parts: target.to_vec() });
    }
    if ghost {
        return Err(FmError::GhostNotAllowed { op: "create a shortcut to".to_string() });
    }
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
