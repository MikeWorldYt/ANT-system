// Moves the files of a review folder into plain subfolders, as the dictionary groups them, creating the ones missing.
use std::collections::BTreeSet;
use std::path::Path;

use serde_yaml::Mapping;

use super::project_groups::check_in_review;
use super::{child_rel, finish, load_synced_at, locate, persist, scope, Mutation};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{validate_component, Journal, StorageLock};
use crate::services::tree::depth;
use crate::services::tree::tree_store::{self, get_node_mut};

// Each move is (file, subfolder, new name).
pub fn move_into_folders(root: &Path, folder: &[String], moves: &[(String, String, String)]) -> R<Mutation> {
    const OP: &str = "group by dictionary";
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, folder)?;
    check_in_review(&data, folder)?;
    let to = locate(root, &data, folder)?;
    if to.ghost {
        return Err(FmError::GhostNotAllowed { op: OP.to_string() });
    }
    let mut journal = Journal::new();
    let mut made = BTreeSet::new();
    let mut used = BTreeSet::new();
    for (file, sub, name) in moves {
        let attempt = (|| {
            for part in [file, sub, name] {
                validate_component(part)?;
            }
            let dir = to.abs.join(sub);
            if !dir.is_dir() {
                journal.mkdir(&dir)?;
                made.insert(sub.clone());
            }
            let target = dir.join(name);
            if target.exists() {
                return Err(FmError::AlreadyExists { name: format!("{sub}/{name}") });
            }
            journal.rename(&to.abs.join(file), &target)
        })();
        if let Err(e) = attempt {
            return Err(journal.abort(OP, e));
        }
        used.insert(sub.clone());
    }
    let limit = depth::limit(root)?;
    for sub in &made {
        let rel = child_rel(folder, sub);
        if rel.len() > limit {
            continue;
        }
        let mut entry = Mapping::new();
        if let Err(e) = tree_store::sync_subtree(root, &rel, &mut entry) {
            return Err(journal.abort(OP, e));
        }
        match get_node_mut(&mut data, folder) {
            Some(slot) => slot.insert(sub.clone().into(), entry.into()),
            None => return Err(journal.abort(OP, FmError::NotFound { rel_parts: folder.to_vec() })),
        };
    }
    let mut touched = vec![folder.to_vec()];
    touched.extend(used.iter().map(|sub| child_rel(folder, sub)));
    persist(root, &data, journal, OP, &scope(&synced, touched.clone()))?;
    Ok(finish(root, &data, false, Some(folder.to_vec()), Vec::new(), scope(&synced, touched)))
}
