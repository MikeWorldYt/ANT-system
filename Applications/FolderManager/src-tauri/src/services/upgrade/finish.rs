// Moves the root leftovers of an old storage (999.Settings, 998.Review) into .ant/ and backup/, then writes the done marker.
use std::fs;
use std::path::Path;

use super::{internal, is_done, scan, write_marker};
use crate::domain::config::{BACKUP_FOLDER, CORE_FOLDER, DATA_DIR, FOOTPRINT_FILENAME, INFO_FILENAME, REVIEW_FOLDER, SETTINGS_FOLDER};
use crate::domain::error::R;
use crate::infra::fs_safe::{dir_entries, io_err, is_link, StorageLock, LOCK_FILE};
use crate::infra::storage::core_dir::{data_dir, legacy_core_dir};
use crate::infra::storage::db::Store;
use crate::services::backup;
use crate::services::index::search_index;
use crate::services::tree::tree_store::{self, delete_key};

const LEGACY_BACKUP_DIR: &str = "03.Backup";

// Fails before any marker exists, so the next launch asks again; what is left behind on purpose is reported as warnings.
pub fn finish_support_upgrade(root: &Path, stamp: &str) -> R<Vec<String>> {
    let _lock = StorageLock::acquire(root)?;
    if is_done(root) {
        return Ok(Vec::new());
    }
    if !scan::legacy_folders(root)?.is_empty() {
        return Err(internal("Support folders still wait for an answer"));
    }
    backup::create(root, stamp)?;
    let mut warnings = Vec::new();
    move_core(root, &mut warnings)?;
    move_backups(root, &mut warnings)?;
    let settings = root.join(SETTINGS_FOLDER);
    if settings.is_dir() && !prune(&settings)? {
        warnings.push(format!("{SETTINGS_FOLDER} was kept because it still holds files of yours"));
    }
    remove_review(root, &mut warnings)?;
    forget_removed(root)?;
    write_marker(root)?;
    Ok(warnings)
}

fn move_core(root: &Path, warnings: &mut Vec<String>) -> R<()> {
    let core = legacy_core_dir(root);
    if !core.is_dir() {
        return Ok(());
    }
    let data = data_dir(root);
    for entry in dir_entries(&core, "list folder")? {
        let entry = entry?;
        let (from, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let to = data.join(&name);
        if name == LOCK_FILE {
            fs::remove_file(&from).map_err(|e| io_err("delete file", &from, &e))?;
        } else if to.exists() {
            warnings.push(format!("{name} was left in {SETTINGS_FOLDER}/{CORE_FOLDER} because .ant/ already has one"));
        } else {
            fs::rename(&from, &to).map_err(|e| io_err("move file", &from, &e))?;
        }
    }
    Ok(())
}

fn move_backups(root: &Path, warnings: &mut Vec<String>) -> R<()> {
    let old = root.join(SETTINGS_FOLDER).join(LEGACY_BACKUP_DIR);
    let new = root.join(BACKUP_FOLDER);
    if old.is_dir() {
        fs::create_dir_all(&new).map_err(|e| io_err("create folder", &new, &e))?;
        for entry in dir_entries(&old, "list folder")? {
            let entry = entry?;
            let (from, to) = (entry.path(), new.join(entry.file_name()));
            if to.exists() {
                warnings.push(format!("{} was left in the old backup folder because backup/ already has one", entry.file_name().to_string_lossy()));
            } else {
                fs::rename(&from, &to).map_err(|e| io_err("move file", &from, &e))?;
            }
        }
    }
    if points_to_old_backups(root) {
        Store::open_or_create(root)?.set_setting("backupFolder", "")?;
    }
    Ok(())
}

fn points_to_old_backups(root: &Path) -> bool {
    let normal = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let old = normal(&root.join(SETTINGS_FOLDER).join(LEGACY_BACKUP_DIR).to_string_lossy());
    backup::settings(root).ok().and_then(|s| s.get("backupFolder").cloned()).is_some_and(|folder| normal(&folder).starts_with(&old))
}

// Removes empty folders below `dir` and `dir` itself; true when it is gone.
fn prune(dir: &Path) -> R<bool> {
    for entry in dir_entries(dir, "list folder")? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| io_err("inspect path", &entry.path(), &e))?;
        if meta.is_dir() && !is_link(&meta) {
            prune(&entry.path())?;
        }
    }
    Ok(fs::remove_dir(dir).is_ok())
}

// Only the shortcuts and the filter file are the app's own; anything else keeps the folder in place.
fn remove_review(root: &Path, warnings: &mut Vec<String>) -> R<()> {
    let review = root.join(REVIEW_FOLDER);
    let Ok(meta) = fs::symlink_metadata(&review) else { return Ok(()) };
    if !meta.is_dir() || is_link(&meta) {
        warnings.push(format!("{REVIEW_FOLDER} was kept because it is a link"));
        return Ok(());
    }
    for entry in dir_entries(&review, "list folder")? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
        let is_shortcut = is_link(&meta) || path.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
        if is_shortcut && !meta.is_dir() {
            fs::remove_file(&path).map_err(|e| io_err("delete file", &path, &e))?;
        }
    }
    let own = review.join(DATA_DIR);
    for file in [INFO_FILENAME, FOOTPRINT_FILENAME] {
        match fs::remove_file(own.join(file)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io_err("delete file", &own.join(file), &e)),
            _ => {}
        }
    }
    if !prune(&review)? {
        warnings.push(format!("{REVIEW_FOLDER} was kept because it still holds files of yours"));
    }
    Ok(())
}

// The tree entries of root folders that are gone go with them, and the search fields are rebuilt from what is left.
fn forget_removed(root: &Path) -> R<()> {
    let mut data = tree_store::load(root)?;
    let mut changed = false;
    for name in [REVIEW_FOLDER, SETTINGS_FOLDER] {
        if !root.join(name).exists() {
            changed |= delete_key(&mut data, &[], name);
        }
    }
    if changed {
        tree_store::save(root, &data)?;
    }
    search_index::rebuild(root, &data)
}
