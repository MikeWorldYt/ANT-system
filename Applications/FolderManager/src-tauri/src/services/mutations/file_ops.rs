// Files in review folders are listed from disk, so these operations act on the file itself and leave the tree alone.
use std::fs;
use std::path::{Path, PathBuf};

use super::recycle::move_to_bin;
use super::{finish, scope, load_synced_at, parent_of, Mutation};

use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{rename_no_replace, resolve, validate_component, StorageLock};
use crate::services::tree::tree_node::lists_files;

// The file a review folder lists; folders, links and missing paths are refused.
pub fn file_path(root: &Path, rel: &[String]) -> R<PathBuf> {
    let path = resolve(root, rel)?;
    if !lists_files(parent_of(rel)) {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only files in the storage can be changed here".to_string() });
    }
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_file() => Ok(path),
        _ => Err(FmError::NotFound { rel_parts: rel.to_vec() }),
    }
}

// A folder the database does not track is moved like a file, straight on disk.
pub fn source_path(root: &Path, rel: &[String]) -> R<PathBuf> {
    let path = resolve(root, rel)?;
    if path.is_dir() && !rel.is_empty() && crate::services::tree::tree_store::get_node(&crate::services::tree::tree_store::load(root)?, rel).is_none() {
        return Ok(path);
    }
    file_path(root, rel)
}

pub fn rename_file(root: &Path, rel: &[String], new_name: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let from = file_path(root, rel)?;
    validate_component(new_name)?;
    let to = from.with_file_name(new_name);
    rename_no_replace("rename file", &from, &to)?;
    let parent = parent_of(rel).to_vec();
    let (data, synced) = load_synced_at(root, &parent)?;
    let mut renamed = parent.clone();
    renamed.push(new_name.to_string());
    Ok(finish(root, &data, false, Some(renamed), Vec::new(), scope(&synced, vec![parent])))
}

// The Recycle Bin takes the file from its own folder under the lock, as it takes folders.
pub fn delete_file(root: &Path, rel: &[String], recycle: impl FnOnce(&Path) -> Result<(), String>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let abs = file_path(root, rel)?;
    let parent = parent_of(rel).to_vec();
    let (data, synced) = load_synced_at(root, &parent)?;
    move_to_bin(&abs, recycle)?;
    Ok(finish(root, &data, false, None, Vec::new(), scope(&synced, vec![parent])))
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
    fn a_case_only_rename_changes_the_name_on_disk() {
        let dir = review_with_file("file-case");
        let m = rename_file(dir.path(), &rel(&["100.A", "98.A", "notes.txt"]), "NOTES.txt").unwrap();
        assert_eq!(crate::test_support::names(&dir.path().join("100.A/98.A")), ["NOTES.txt"]);
        assert_eq!(m.select, Some(rel(&["100.A", "98.A", "NOTES.txt"])));
    }

    #[test]
    fn a_taken_name_is_refused_and_both_files_stay() {
        let dir = review_with_file("file-taken");
        let folder = dir.path().join("100.A/98.A");
        std::fs::write(folder.join("plan.txt"), "plan").unwrap();
        let err = rename_file(dir.path(), &rel(&["100.A", "98.A", "notes.txt"]), "plan.txt").unwrap_err();
        assert!(matches!(&err, FmError::AlreadyExists { name } if name == "plan.txt"), "{err:?}");
        assert_eq!(std::fs::read_to_string(folder.join("notes.txt")).unwrap(), "x");
        assert_eq!(std::fs::read_to_string(folder.join("plan.txt")).unwrap(), "plan");
    }

    // `mklink` reads a forward slash as a switch, so the path is rebuilt with native separators.
    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let link: PathBuf = link.components().collect();
        let target: PathBuf = target.components().collect();
        let out = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(&target).output().unwrap();
        assert!(out.status.success(), "no junction: {}", String::from_utf8_lossy(&out.stdout));
    }

    fn assert_invalid_path<T: std::fmt::Debug>(result: R<T>) {
        assert!(matches!(result, Err(FmError::InvalidPath { .. })), "{result:?}");
    }

    #[cfg(windows)]
    #[test]
    fn files_behind_a_junction_are_refused_and_stay_where_they_are() {
        let dir = review_with_file("file-junction");
        let outside = crate::test_support::TempDir::new("file-junction-outside");
        std::fs::write(outside.path().join("secret.txt"), "keep").unwrap();
        junction(&dir.path().join("100.A/linked"), outside.path());
        let behind = rel(&["100.A", "linked", "secret.txt"]);
        assert_invalid_path(rename_file(dir.path(), &behind, "x.txt"));
        assert_invalid_path(delete_file(dir.path(), &behind, |_| Ok(())));
        assert_invalid_path(file_path(dir.path(), &behind));
        assert_invalid_path(source_path(dir.path(), &behind));
        assert_invalid_path(source_path(dir.path(), &rel(&["100.A", "linked"])));
        assert_eq!(std::fs::read_to_string(outside.path().join("secret.txt")).unwrap(), "keep");
        assert_eq!(crate::test_support::names(outside.path()), ["secret.txt"]);
        std::fs::remove_dir(dir.path().join("100.A/linked")).unwrap();
    }

    #[test]
    fn files_of_the_app_folder_are_refused_at_any_depth() {
        let dir = review_with_file("file-ant");
        dir.mkdir("100.A/.ant");
        std::fs::write(dir.path().join("100.A/.ant/info.json"), "{}").unwrap();
        assert!(dir.path().join(".ant/ant.db").is_file());
        for path in [rel(&[".ant", "ant.db"]), rel(&["100.A", ".ant", "info.json"]), rel(&["100.A", ".ANT", "info.json"])] {
            assert_invalid_path(delete_file(dir.path(), &path, |_| panic!("sent to the bin")));
            assert_invalid_path(rename_file(dir.path(), &path, "x.json"));
            assert_invalid_path(source_path(dir.path(), &path));
        }
        assert_invalid_path(source_path(dir.path(), &rel(&[".ant"])));
        assert!(dir.path().join(".ant/ant.db").is_file() && dir.path().join("100.A/.ant/info.json").is_file());
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
