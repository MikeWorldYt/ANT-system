// Files in review folders are listed from disk, so these operations act on the file itself and leave the yaml tree alone.
use std::fs;
use std::path::{Path, PathBuf};

use super::delete::stage_in_trash;
use super::recycle::{PendingRecycle, IN_FLIGHT};
use super::{finish, load_synced_at, parent_of, Mutation};

use crate::error::{FmError, R};
use crate::fs_safe::{io_err, join_rel, validate_component, Journal, StorageLock};
use crate::tree::tree_node::holds_files;
use crate::tree::tree_store;

// The file a review folder lists; folders, links and missing paths are refused.
pub fn file_path(root: &Path, rel: &[String]) -> R<PathBuf> {
    rel.iter().try_for_each(|part| validate_component(part))?;
    let data = tree_store::load(root)?;
    if !holds_files(&data, parent_of(rel)) {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only files in review folders can be changed here".to_string() });
    }
    let path = join_rel(root, rel);
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_file() => Ok(path),
        _ => Err(FmError::NotFound { rel_parts: rel.to_vec() }),
    }
}

pub fn rename_file(root: &Path, rel: &[String], new_name: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let from = file_path(root, rel)?;
    validate_component(new_name)?;
    let to = from.with_file_name(new_name);
    if to.exists() {
        return Err(FmError::AlreadyExists { name: new_name.to_string() });
    }
    fs::rename(&from, &to).map_err(|e| io_err("rename file", &from, &e))?;
    let parent = parent_of(rel).to_vec();
    let (data, synced) = load_synced_at(root, &parent)?;
    let mut renamed = parent.clone();
    renamed.push(new_name.to_string());
    Ok(finish(root, &data, synced, Some(renamed), Vec::new(), vec![parent]))
}

// The file goes to .ant-trash now and to the Recycle Bin after the storage lock is released.
pub fn delete_file(root: &Path, rel: &[String]) -> R<(Mutation, PendingRecycle)> {
    let _lock = StorageLock::acquire(root)?;
    let abs = file_path(root, rel)?;
    let parent = parent_of(rel).to_vec();
    let (data, synced) = load_synced_at(root, &parent)?;
    let name = rel.last().cloned().unwrap_or_default();
    let mut journal = Journal::new();
    let staged = stage_in_trash(root, &mut journal, &abs, &name)?;
    journal.commit();
    IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).push(staged.clone());
    let pending = PendingRecycle { root: root.to_path_buf(), staged, name };
    Ok((finish(root, &data, synced, None, Vec::new(), vec![parent]), pending))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TRASH_DIR;
    use crate::test_support::{rel, storage};

    fn review_with_file(tag: &str) -> crate::test_support::TempDir {
        let dir = storage(tag, &["100.A/98.A"], "100.A:\n  98.A:\n    type: REV-L2\n");
        std::fs::write(dir.path().join("100.A/98.A/notes.txt"), "x").unwrap();
        std::fs::write(dir.path().join("100.A/readme.txt"), "x").unwrap();
        dir
    }

    #[test]
    fn a_review_file_is_renamed_on_disk_and_selected_under_its_new_name() {
        let dir = review_with_file("file-rename");
        let m = rename_file(dir.path(), &rel(&["100.A", "98.A", "notes.txt"]), "plan.txt").unwrap();
        assert!(dir.path().join("100.A/98.A/plan.txt").is_file());
        assert!(!dir.path().join("100.A/98.A/notes.txt").exists());
        assert_eq!(m.select, Some(rel(&["100.A", "98.A", "plan.txt"])));
    }

    #[test]
    fn only_files_in_review_folders_can_be_renamed_or_deleted() {
        let dir = review_with_file("file-refuse");
        assert!(rename_file(dir.path(), &rel(&["100.A", "readme.txt"]), "x.txt").is_err());
        assert!(delete_file(dir.path(), &rel(&["100.A", "readme.txt"])).is_err());
        assert!(dir.path().join("100.A/readme.txt").is_file());
    }

    #[test]
    fn a_deleted_review_file_leaves_its_folder_and_waits_in_the_trash() {
        let dir = review_with_file("file-delete");
        let (_, pending) = delete_file(dir.path(), &rel(&["100.A", "98.A", "notes.txt"])).unwrap();
        assert!(!dir.path().join("100.A/98.A/notes.txt").exists());
        assert!(pending.staged.starts_with(dir.path().join(TRASH_DIR)));
        assert!(pending.staged.is_file());
    }
}
