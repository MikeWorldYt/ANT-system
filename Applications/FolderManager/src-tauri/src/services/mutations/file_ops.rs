// Files in review folders are listed from disk, so these operations act on the file itself and leave the yaml tree alone.
use std::fs;
use std::path::{Path, PathBuf};

use super::recycle::move_to_bin;
use super::{finish, load_synced_at, parent_of, Mutation};

use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, join_rel, validate_component, StorageLock};
use crate::services::tree::tree_node::lists_files;

// The file a review folder lists; folders, links and missing paths are refused.
pub fn file_path(root: &Path, rel: &[String]) -> R<PathBuf> {
    rel.iter().try_for_each(|part| validate_component(part))?;
    if !lists_files(parent_of(rel)) {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only files in the storage can be changed here".to_string() });
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

// The Recycle Bin takes the file from its own folder under the lock, as it takes folders.
pub fn delete_file(root: &Path, rel: &[String], recycle: impl FnOnce(&Path) -> Result<(), String>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let abs = file_path(root, rel)?;
    let parent = parent_of(rel).to_vec();
    let (data, synced) = load_synced_at(root, &parent)?;
    move_to_bin(&abs, recycle)?;
    Ok(finish(root, &data, synced, None, Vec::new(), vec![parent]))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn only_real_files_can_be_renamed_or_deleted() {
        let dir = review_with_file("file-refuse");
        assert!(rename_file(dir.path(), &rel(&["100.A", "readme.txt"]), "x.txt").is_ok());
        assert!(dir.path().join("100.A/x.txt").is_file());
        assert!(rename_file(dir.path(), &rel(&["100.A", "missing.txt"]), "y.txt").is_err());
        assert!(delete_file(dir.path(), &rel(&["100.A", "98.A"]), |_| Ok(())).is_err());
    }

    #[test]
    fn a_deleted_review_file_is_sent_from_its_own_folder() {
        let dir = review_with_file("file-delete");
        let mut sent = Vec::new();
        let m = delete_file(dir.path(), &rel(&["100.A", "98.A", "notes.txt"]), |p| {
            sent.push(p.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(sent, [dir.path().join("100.A/98.A/notes.txt")]);
        assert!(m.warnings.is_empty());
        assert!(dir.path().join("100.A/98.A").is_dir());
    }
}
