// Gives every node its 98. review folder back when it went missing or was left under an old name.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::ant_core::names::editable_part;
use crate::ant_core::tree_types::{child_entries, type_base};
use crate::config::MAX_DEPTH;
use crate::error::R;
use crate::fs_safe::{io_err, join_rel, StorageLock};
use crate::tree::tree_node::is_reserved;
use crate::tree::tree_store::{self, delete_key, get_node};

// Returns whether anything changed, so the caller reloads the tree.
pub fn restore(root: &Path) -> R<bool> {
    let _lock = StorageLock::acquire(root)?;
    let mut data = tree_store::load(root)?;
    let mut changed = false;
    let mut owners = Vec::new();
    collect(&data, &mut Vec::new(), &mut owners);
    for owner in owners {
        let expected = format!("98.{}", editable_part(owner.last().map_or("", String::as_str)));
        let dir = join_rel(root, &owner);
        if !dir.is_dir() {
            continue;
        }
        let mut review = owner.clone();
        review.push(expected);
        let target = join_rel(root, &review);
        if !target.is_dir() {
            fs::create_dir(&target).map_err(|e| io_err("create folder", &target, &e))?;
            changed = true;
        }
        // An entry for a review folder that no longer exists on disk would show as a ghost beside the live one.
        for stale in stale_reviews(&data, &owner, &dir, &review[review.len() - 1]) {
            changed |= delete_key(&mut data, &owner, &stale);
        }
    }
    if changed {
        tree_store::sync(root, &mut data)?;
        tree_store::save(root, &data)?;
    }
    Ok(changed)
}

fn collect(node: &Mapping, parts: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    for (name, child) in child_entries(node) {
        parts.push(name.to_string());
        let is_node = child.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "NODE");
        if is_node && !is_reserved(&parts[..1]) {
            out.push(parts.clone());
        }
        if parts.len() < MAX_DEPTH {
            collect(child, parts, out);
        }
        parts.pop();
    }
}

fn stale_reviews(data: &Mapping, owner: &[String], dir: &Path, keep: &str) -> Vec<String> {
    let Some(node) = get_node(data, owner) else { return Vec::new() };
    child_entries(node)
        .filter(|(name, child)| {
            *name != keep
                && name.starts_with("98.")
                && child.get("type").and_then(Value::as_str).is_some_and(|t| type_base(t) == "REV")
                && !dir.join(name).is_dir()
        })
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
}
