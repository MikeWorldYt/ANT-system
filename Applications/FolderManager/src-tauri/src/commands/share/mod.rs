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
use crate::fs_safe::{io_err, join_rel};
use crate::mutations::{self, free_name, locate, Target};
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

// Files and folders go on the clipboard together, as a cut or a copy; a folder refused for cut refuses the whole set.
#[tauri::command]
pub async fn copy_items(app: AppHandle, targets: Vec<Vec<String>>, cut: bool) -> R<()> {
    run_locked(app, move |root| {
        let data = tree_store::load(root)?;
        let mut paths = Vec::with_capacity(targets.len());
        for target in &targets {
            if join_rel(root, target).is_file() {
                paths.push(mutations::file_path(root, target)?);
                continue;
            }
            let Target { abs, ghost, node_type } = locate(root, &data, target)?;
            let flags = get_node(&data, target).map(flags_in).unwrap_or_default();
            clipboard::folder_effect(target, &node_type, &flags, ghost, cut)?;
            paths.push(abs);
        }
        clipboard::write(&paths, clipboard::drop_effect(cut))
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

#[tauri::command]
pub async fn create_file_shortcut_in(app: AppHandle, target: Vec<String>, folder: String) -> R<ShortcutCreated> {
    run_locked(app, move |root| {
        let abs = crate::mutations::file_path(root, &target)?;
        let (abs, link) = link_in(root, abs, Path::new(&folder))?;
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
    link_in(root, abs, dir)
}

// Folders and files share these rules: the link goes into a folder outside the storage, under a free name.
pub(super) fn link_in(root: &Path, abs: PathBuf, dir: &Path) -> R<(PathBuf, PathBuf)> {
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
