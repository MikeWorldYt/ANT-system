// Gives every node its 98. review folder back when it went missing or was left under an old name.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::ant_core::names::{editable_part, list_subdirs};
use crate::ant_core::tree_types::{child_entries, type_base};
use crate::error::R;
use crate::mutations::free_name;
use crate::fs_safe::{io_err, join_rel, StorageLock};
use crate::tree::depth;
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, delete_key, get_node};

// Returns whether anything changed, so the caller reloads the tree.
pub fn restore(root: &Path) -> R<bool> {
    let _lock = StorageLock::acquire(root)?;
    let mut data = tree_store::load(root)?;
    let mut changed = false;
    let mut owners = Vec::new();
    collect(&data, &mut Vec::new(), &mut owners, depth::limit(root)?);
    for owner in owners {
        let expected = format!("98.{}", editable_part(owner.last().map_or("", String::as_str)));
        let dir = join_rel(root, &owner);
        if !dir.is_dir() {
            continue;
        }
        let (keep, renamed) = review_name(&mut data, &owner, &dir, expected)?;
        changed |= renamed;
        let mut review = owner.clone();
        review.push(keep.clone());
        let target = join_rel(root, &review);
        if !target.is_dir() {
            fs::create_dir(&target).map_err(|e| io_err("create folder", &target, &e))?;
            changed = true;
        }
        changed |= merge_extra_reviews(&mut data, &owner, &dir, &keep)?;
        // An entry for a review folder that no longer exists on disk would show as a ghost beside the live one.
        for stale in stale_reviews(&data, &owner, &dir, &keep) {
            changed |= delete_key(&mut data, &owner, &stale);
        }
    }
    if changed {
        tree_store::sync(root, &mut data)?;
        tree_store::save(root, &data)?;
    }
    Ok(changed)
}

fn collect(node: &Mapping, parts: &mut Vec<String>, out: &mut Vec<Vec<String>>, limit: usize) {
    for (name, child) in child_entries(node) {
        parts.push(name.to_string());
        let is_node = child.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "NODE");
        if is_node && !is_reserved(&parts[..1]) {
            out.push(parts.clone());
        }
        if parts.len() < limit {
            collect(child, parts, out, limit);
        }
        parts.pop();
    }
}

// A review folder left under the node's old name after an outside rename is the node's review, so it takes the node's name.
fn review_name(data: &mut Mapping, owner: &[String], dir: &Path, expected: String) -> R<(String, bool)> {
    let names = list_subdirs(dir)?;
    if names.contains(&expected) {
        return Ok((expected, false));
    }
    let Some(old) = old_review(data, owner, &names) else { return Ok((expected, false)) };
    let from = dir.join(&old);
    fs::rename(&from, dir.join(&expected)).map_err(|e| io_err("rename folder", &from, &e))?;
    tree_store::rename_key(data, owner, &old, &expected);
    Ok((expected, true))
}

// Older builds left extra 98.* folders; their contents move into the one the node keeps, and the emptied folders go.
fn merge_extra_reviews(data: &mut Mapping, owner: &[String], dir: &Path, keep: &str) -> R<bool> {
    let target = dir.join(keep);
    let extras: Vec<String> = list_subdirs(dir)?.into_iter().filter(|name| name != keep && name.starts_with("98.")).collect();
    for extra in &extras {
        let from = dir.join(extra);
        for entry in fs::read_dir(&from).map_err(|e| io_err("list folder", &from, &e))? {
            let entry = entry.map_err(|e| io_err("list folder", &from, &e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let landing = free_name(&target, &name, &[])?;
            fs::rename(entry.path(), target.join(&landing)).map_err(|e| io_err("move file", &entry.path(), &e))?;
        }
        fs::remove_dir(&from).map_err(|e| io_err("remove folder", &from, &e))?;
        delete_key(data, owner, extra);
    }
    Ok(!extras.is_empty())
}

fn old_review(data: &Mapping, owner: &[String], names: &[String]) -> Option<String> {
    let node = get_node(data, owner)?;
    child_entries(node)
        .find(|(name, child)| name.starts_with("98.") && is_review(child) && names.iter().any(|n| n.as_str() == *name))
        .map(|(name, _)| name.to_string())
}

fn is_review(child: &Mapping) -> bool {
    child.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "REV")
}

fn stale_reviews(data: &Mapping, owner: &[String], dir: &Path, keep: &str) -> Vec<String> {
    let Some(node) = get_node(data, owner) else { return Vec::new() };
    child_entries(node)
        .filter(|(name, child)| *name != keep && name.starts_with("98.") && is_review(child) && !dir.join(name).is_dir())
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{storage, yaml};

    #[test]
    fn a_missing_review_folder_is_recreated_and_a_stale_ghost_dropped() {
        let dir = storage(
            "review-folders",
            &["100.ACADEMIC/98.ACADEMIC", "100.ACADEMIC/01.Math", "100.ACADEMIC/90.Res"],
            "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2}\n  90.Res: {type: RES}\n  98.Review: {type: REV-L2}\n  98.ACADEMIC: {type: REV-L2}\n",
        );
        assert!(restore(dir.path()).unwrap());
        assert!(dir.path().join("100.ACADEMIC/01.Math/98.Math").is_dir());
        assert!(!dir.path().join("100.ACADEMIC/90.Res/98.Res").exists());
        let data = yaml(dir.path());
        assert!(get_node(&data, &["100.ACADEMIC".to_string(), "98.Review".to_string()]).is_none());
        assert!(get_node(&data, &["100.ACADEMIC".to_string(), "01.Math".to_string(), "98.Math".to_string()]).is_some());
        assert!(!restore(dir.path()).unwrap());
    }

    #[test]
    fn extra_review_folders_merge_into_the_one_the_node_keeps() {
        let dir = storage(
            "review-merge",
            &["100.ACADEMIC/01.Math/98.Math", "100.ACADEMIC/01.Math/98.Old", "100.ACADEMIC/01.Math/98.Older"],
            "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2, 98.Math: {type: REV-L3}, 98.Old: {type: REV-L3}, 98.Older: {type: REV-L3}}\n",
        );
        fs::write(dir.path().join("100.ACADEMIC/01.Math/98.Math/notes.txt"), "kept").unwrap();
        fs::write(dir.path().join("100.ACADEMIC/01.Math/98.Old/notes.txt"), "old").unwrap();
        assert!(restore(dir.path()).unwrap());
        assert!(!dir.path().join("100.ACADEMIC/01.Math/98.Old").exists());
        assert!(!dir.path().join("100.ACADEMIC/01.Math/98.Older").exists());
        assert_eq!(fs::read_to_string(dir.path().join("100.ACADEMIC/01.Math/98.Math/notes.txt")).unwrap(), "kept");
        assert_eq!(fs::read_to_string(dir.path().join("100.ACADEMIC/01.Math/98.Math/notes (2).txt")).unwrap(), "old");
        let data = yaml(dir.path());
        assert!(get_node(&data, &["100.ACADEMIC".to_string(), "01.Math".to_string(), "98.Old".to_string()]).is_none());
        assert!(!restore(dir.path()).unwrap());
    }

    #[test]
    fn a_review_folder_left_under_the_old_name_is_renamed_to_follow_the_node() {
        let dir = storage(
            "review-adopt",
            &["100.ACADEMIC/98.ACADEMIC", "100.ACADEMIC/01.Maths/98.Old"],
            "100.ACADEMIC:\n  type: NODE-L1\n  98.ACADEMIC: {type: REV-L1}\n  01.Maths: {type: NODE-L2, 98.Old: {type: REV-L3}}\n",
        );
        assert!(restore(dir.path()).unwrap());
        assert!(!dir.path().join("100.ACADEMIC/01.Maths/98.Old").exists());
        assert!(dir.path().join("100.ACADEMIC/01.Maths/98.Maths").is_dir());
        let data = yaml(dir.path());
        assert!(get_node(&data, &["100.ACADEMIC".to_string(), "01.Maths".to_string(), "98.Old".to_string()]).is_none());
        assert!(get_node(&data, &["100.ACADEMIC".to_string(), "01.Maths".to_string(), "98.Maths".to_string()]).is_some());
        assert!(!restore(dir.path()).unwrap());
    }
}
