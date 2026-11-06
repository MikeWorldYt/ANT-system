// Leftover 98.* review folders of older builds: nothing touches them until the upgrade is done and the user agrees to merge each one.
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use super::is_done;
use crate::domain::error::{FmError, R};
use crate::domain::names::{editable_part, is_legacy_support, support_kind};
use crate::domain::tree_types::child_entries;
use crate::infra::fs_safe::{dir_entries, io_err, join_rel, validate_component, Journal, StorageLock};
use crate::infra::naming::free_name;
use crate::infra::scan::list_subdirs;
use crate::services::index::review_folders::{collect, is_review, kept_review};
use crate::services::mutations::{child_rel, finish, load_synced_at, locate, persist, scope, split, subtrees, sync_branch_drift, Mutation};
use crate::services::tree::depth;
use crate::services::tree::tree_store::{self, delete_key, get_node};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraReview {
    pub rel_parts: Vec<String>,
    pub name: String,
    pub keep: String,
}

pub fn extra_reviews(root: &Path) -> R<Vec<ExtraReview>> {
    if !is_done(root) {
        return Ok(Vec::new());
    }
    let data = tree_store::load(root)?;
    let mut owners = Vec::new();
    collect(&data, &mut Vec::new(), &mut owners, depth::limit(root)?);
    let mut found = Vec::new();
    for owner in owners {
        let dir = join_rel(root, &owner);
        if !dir.is_dir() {
            continue;
        }
        let names = list_subdirs(&dir)?;
        let Some(keep) = kept_review(&names, editable_part(owner.last().map_or("", String::as_str))) else { continue };
        for name in names.iter().filter(|name| **name != keep && is_legacy_review(&data, &owner, name)) {
            found.push(ExtraReview { rel_parts: child_rel(&owner, name), name: name.clone(), keep: keep.clone() });
        }
    }
    Ok(found)
}

// The name alone is not enough: a user's folder can start with 98. too, so the tree must also call it a review.
fn is_legacy_review(data: &Mapping, owner: &[String], name: &str) -> bool {
    is_legacy_support(name) && support_kind(name) == Some("REV") && get_node(data, owner).is_some_and(|node| child_entries(node).any(|(child_name, child)| child_name == name && is_review(child)))
}

// The folder itself goes after the tree is saved, because a rollback needs it to put the files back.
pub fn merge_extra_review(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    rel.iter().try_for_each(|part| validate_component(part))?;
    let Some(extra) = extra_reviews(root)?.into_iter().find(|extra| extra.rel_parts == rel) else {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "not a leftover review folder".to_string() });
    };
    let (name, owner) = split(rel)?;
    let keep_rel = child_rel(owner, &extra.keep);
    let (mut data, mut synced) = load_synced_at(root, owner)?;
    let from = locate(root, &data, rel)?.abs;
    let into = locate(root, &data, &keep_rel)?.abs;
    let before = subtrees(&data, &[rel.to_vec()]);
    let mut journal = Journal::new();
    match move_contents(&mut journal, &from, &into).and_then(|()| sync_branch_drift(root, &mut data, &keep_rel)) {
        Ok(drift) => synced.absorb(drift),
        Err(e) => return Err(journal.abort("merge review", e)),
    }
    delete_key(&mut data, owner, name);
    let touched = scope(&synced, vec![rel.to_vec(), keep_rel]);
    persist(root, &data, journal, "merge review", &touched)?;
    let warnings = fs::remove_dir(&from).err().map(|e| io_err("remove folder", &from, &e).to_string()).into_iter().collect();
    Ok(finish(root, &data, false, None, warnings, touched).replacing(before))
}

// The journal's rename refuses an occupied name, so a file that appears after the free name was picked stops the merge instead of being replaced.
fn move_contents(journal: &mut Journal, from: &Path, into: &Path) -> R<()> {
    for entry in dir_entries(from, "list folder")? {
        let entry = entry?;
        let landing = free_name(into, &entry.file_name().to_string_lossy(), &[])?;
        journal.rename(&entry.path(), &into.join(landing))?;
    }
    Ok(())
}
