// Internal drag and drop: a data node or a workboard moves, or is copied, into another folder of the storage.
mod copy;
mod rules;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde_yaml::Mapping;

use super::{child_rel, ensure_free, finish, load_synced_at, locate, parent_of, persist, split, subtrees, Mutation};
use crate::error::{FmError, R};
use crate::fs_safe::{Journal, StorageLock};
use crate::tree::depth;
use crate::tree::tree_store::{self, delete_key, get_node, get_node_mut};

pub fn move_folder(root: &Path, source: &[String], destination: &[String], copy: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, mut synced) = load_synced_at(root, parent_of(source))?;
    if tree_store::sync_branch(root, &mut data, destination)? {
        tree_store::sync(root, &mut data)?;
        synced = true;
    }
    let from = locate(root, &data, source)?;
    let node = get_node(&data, source).ok_or_else(|| FmError::NotFound { rel_parts: source.to_vec() })?;
    rules::check_source(source, &from, node)?;
    let to = locate(root, &data, destination)?;
    rules::check_destination(destination, &to)?;
    rules::check_not_inside(source, destination)?;
    let (name, parent) = split(source)?;
    if !copy && parent == destination {
        return Err(rules::refuse(source, format!("the folder is already in {}", destination.join("/"))));
    }
    // Folders made on disk inside the source count toward the depth like the ones already in the tree.
    if let Some(node) = get_node_mut(&mut data, source) {
        synced |= tree_store::sync_subtree(root, source, node)?;
    }
    let entry = get_node(&data, source).cloned().ok_or_else(|| FmError::NotFound { rel_parts: source.to_vec() })?;
    let limit = depth::limit(root)?;
    let warnings: Vec<String> = rules::depth_warning(name, &entry, destination, limit, copy).into_iter().collect();
    ensure_free(&to.abs, get_node(&data, destination), name, &[]).map_err(|e| match e {
        FmError::AlreadyExists { .. } => rules::refuse(
            destination,
            format!("a folder named {name} already exists in {}", destination.join("/")),
        ),
        other => other,
    })?;

    let new_rel = child_rel(destination, name);
    let new_abs = to.abs.join(name);
    // A folder that lands past the limit is not in the tree, so its destination is what gets selected.
    let select = if new_rel.len() > limit { destination.to_vec() } else { new_rel.clone() };
    if copy {
        return copy::copy_folder(root, data, (&from.abs, &new_abs), (&new_rel, select), entry, (synced, warnings));
    }
    let before = subtrees(&data, &[source.to_vec()]);
    let mut journal = Journal::new();
    journal.rename(&from.abs, &new_abs)?;
    delete_key(&mut data, parent, name);
    if let Err(e) = insert(root, &mut data, &new_rel, entry, limit) {
        return Err(journal.abort("move folder", e));
    }
    persist(root, &data, journal, "move folder")?;
    let m = finish(root, &data, synced, Some(select), warnings, vec![source.to_vec(), new_rel]);
    Ok(m.replacing(before))
}

// Runs once the folder is on disk at `rel`, so folders that were past the limit at the old place are picked up.
fn insert(root: &Path, data: &mut Mapping, rel: &[String], mut entry: Mapping, limit: usize) -> R<()> {
    let (name, destination) = split(rel)?;
    if rel.len() > limit {
        return Ok(());
    }
    rules::prune(&mut entry, rel.len(), limit);
    tree_store::sync_subtree(root, rel, &mut entry)?;
    let slot = get_node_mut(data, destination).ok_or_else(|| FmError::NotFound { rel_parts: destination.to_vec() })?;
    slot.insert(name.clone().into(), entry.into());
    Ok(())
}
