// Hands a storage folder to other apps: on the Windows clipboard for Explorer, or as a shortcut outside the storage.
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::AppHandle;

use super::run_locked;
use crate::platform::shortcuts::create_shortcuts;
use crate::domain::tree_types::flags_in;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::join_rel;
use crate::platform::clipboard;
use crate::services::mutations::{self, link_in, locate, shortcut_plan, Target};
use crate::services::tree::tree_store::{self, get_node};

#[derive(Debug, Serialize)]
pub struct ShortcutCreated {
    pub name: String,
    pub folder: String,
}

// Files and folders go on the clipboard together, as a cut or a copy; a folder refused for cut refuses the whole set.
#[tauri::command]
pub async fn copy_items(app: AppHandle, targets: Vec<Vec<String>>, cut: bool) -> R<()> {
    run_locked(app, move |root| {
        let data = tree_store::load_paths(root, &targets)?;
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
        let data = tree_store::load_paths(root, std::slice::from_ref(&target))?;
        let (abs, link) = shortcut_plan(root, &data, &target, Path::new(&folder))?;
        place_link(abs, &link, folder)
    })
    .await
}

#[tauri::command]
pub async fn create_file_shortcut_in(app: AppHandle, target: Vec<String>, folder: String) -> R<ShortcutCreated> {
    run_locked(app, move |root| {
        let abs = crate::services::mutations::file_path(root, &target)?;
        let (abs, link) = link_in(root, abs, Path::new(&folder))?;
        place_link(abs, &link, folder)
    })
    .await
}

fn place_link(abs: PathBuf, link: &Path, folder: String) -> R<ShortcutCreated> {
    let failures = create_shortcuts(&[(abs, link.to_path_buf())]);
    if !failures.is_empty() {
        return Err(FmError::Internal { message: failures.join("; ") });
    }
    let name = link.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(ShortcutCreated { name, folder })
}

