// Places files dropped from Explorer into a storage folder, moved by default or copied when Ctrl is held.
mod plain;
mod target;
#[cfg(test)]
mod tests;
mod transfer;
mod workboard;

pub(super) use transfer::copy_tree;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{exactly_on_disk, finish, load_synced_at, locate, register_ids, scope, sync_branch_drift, sync_loaded_at, Drift, Mutation};
use crate::domain::error::{FmError, R};
use crate::domain::names::is_workboard_name;
use crate::domain::tree_types::{child_entries, type_base};
use crate::infra::fs_safe::{io_err, resolve, Journal, StorageLock};
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::{self, get_node};
use crate::services::tree::{depth, review_links};
use plain::{place_folder, Outcome, Place};
use target::destination;
use workboard::{Workboard, WorkboardDrop};

const ALREADY_INSIDE: &str = "it is already in the storage; drag it inside the app to move it";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    Copy,
    Move,
    // Only for sources the app resolved to folders the tree does not track, which may sit inside the storage.
    MoveUntracked,
}

#[derive(Debug, Serialize)]
pub struct DropOutcome {
    pub mutation: Mutation,
    pub placed: Vec<PlacedFile>,
    pub conflicts: Vec<WorkboardConflict>,
}

// A workboard that is already in the storage under the same id; nothing is placed until the user chooses to replace it.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkboardConflict {
    pub name: String,
    pub source: PathBuf,
    pub existing: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PlacedFile {
    pub name: String,
    pub folder: Vec<String>,
    pub copied: bool,
}

// `replace` lists the workboards the user agreed to swap for the one already in the storage.
pub fn drop_files(
    root: &Path,
    target: &[String],
    paths: &[PathBuf],
    copy: bool,
    replace: &[PathBuf],
    recycle: &dyn Fn(&Path) -> Result<(), String>,
) -> R<DropOutcome> {
    place_all(root, target, paths, if copy { Placement::Copy } else { Placement::Move }, replace, recycle)
}

// The app resolved these sources to folders the tree does not track, the one case where a folder may move from inside the storage.
pub fn move_in(root: &Path, target: &[String], paths: &[PathBuf], copy: bool, recycle: &dyn Fn(&Path) -> Result<(), String>) -> R<DropOutcome> {
    place_all(root, target, paths, if copy { Placement::Copy } else { Placement::MoveUntracked }, &[], recycle)
}

fn place_all(
    root: &Path,
    target: &[String],
    paths: &[PathBuf],
    placement: Placement,
    replace: &[PathBuf],
    recycle: &dyn Fn(&Path) -> Result<(), String>,
) -> R<DropOutcome> {
    let copy = placement == Placement::Copy;
    let _lock = StorageLock::acquire(root)?;
    // Only a 998.Review link needs the tree before the sync, and then the same load serves both.
    let loaded = if review_links::is_link_path(target) { Some(tree_store::load(root)?) } else { None };
    let owner = match &loaded {
        Some(data) => review_links::linked_owner(root, data, target)?,
        None => None,
    };
    let target = owner.as_deref().unwrap_or(target);
    let (mut data, mut synced) = match loaded {
        Some(data) => sync_loaded_at(root, target, data)?,
        None => load_synced_at(root, target)?,
    };
    if locate(root, &data, target)?.ghost {
        return Err(FmError::GhostNotAllowed { op: "drop files into".to_string() });
    }
    let node = get_node(&data, target);
    // Past the depth limit the database tracks nothing, so the folder itself takes what is dropped on it.
    let past_limit = node.is_none() && !target.is_empty() && target.len() >= depth::limit(root)?;
    // A 998.Review link stands for its owner's review folder, so its files go there whatever the owner's type.
    let dest_rel = match (&owner, node) {
        (Some(owner), _) => review_links::rev_path(root, owner).split('/').map(str::to_string).collect(),
        (None, Some(node)) => destination(target, node)?,
        (None, None) if past_limit && !is_reserved(&target[..1]) => target.to_vec(),
        (None, None) => return Err(FmError::NotFound { rel_parts: target.to_vec() }),
    };
    let dest = resolve(root, &dest_rel)?;
    if !exactly_on_disk(root, &dest_rel)? {
        return Err(FmError::NotFound { rel_parts: dest_rel });
    }
    if synced.any() {
        tree_store::save(root, &data)?;
    }
    let storage = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    let source_folders: Vec<Vec<String>> = if copy { Vec::new() } else { paths.iter().filter_map(|src| folder_of(&storage, src)).collect() };
    let mut journal = Journal::new();
    let mut warnings = Vec::new();
    let mut placed = Vec::new();
    let mut conflicts = Vec::new();
    // Workboards go first, since placing one changes the tree that the files are placed by.
    let (folders, files): (Vec<&PathBuf>, Vec<&PathBuf>) = paths.iter().partition(|src| is_folder(src));
    let (folders, plain): (Vec<&PathBuf>, Vec<&PathBuf>) = folders.into_iter().partition(|src| src.file_name().is_some_and(|n| is_workboard_name(&n.to_string_lossy())));
    let mut moved_workboard = false;
    if !folders.is_empty() && !matches!(type_base(&tree_store::get_type(&data, target)), "NODE" | "WKB") {
        return Err(FmError::InvalidPath { rel_parts: target.to_vec(), reason: "workboards go into a node or another workboard".to_string() });
    }
    // A workboard sits directly in the node it was dropped on, unlike files, which go to its review folder.
    let node_dir = resolve(root, target)?;
    let boards = WorkboardDrop { root, dest_rel: target, dest: &node_dir, copy, recycle };
    for src in folders {
        match boards.place(&mut data, src, replace.contains(src), &mut journal) {
            Ok(Workboard::Placed(name, note)) => {
                moved_workboard = true;
                if let Some(note) = note {
                    warnings.push(format!("{}: {note}", src.display()));
                }
                placed.push(PlacedFile { name, folder: target.to_vec(), copied: false });
            }
            Ok(Workboard::Taken(conflict)) => conflicts.push(conflict),
            Err(why) => warnings.push(format!("{}: not placed, {why}", src.display())),
        }
    }
    // Any other folder lands as it is; the format validation then asks what it becomes.
    for src in plain {
        match place_folder(&storage, &node_dir, src, placement, recycle, &mut journal) {
            Ok((name, note)) => {
                if let Some(note) = note {
                    warnings.push(format!("{}: {note}", src.display()));
                }
                placed.push(PlacedFile { name, folder: target.to_vec(), copied: copy });
            }
            Err(why) => warnings.push(format!("{}: not placed, {why}", src.display())),
        }
    }
    if moved_workboard {
        synced.absorb(sync_branch_drift(root, &mut data, target)?);
        synced.absorb(Drift::unscoped(register_ids(root, &mut data, &mut warnings)));
        tree_store::save(root, &data)?;
    }
    let place = Place {
        storage,
        dest: &dest,
        dest_real: fs::canonicalize(&dest).map_err(|e| io_err("resolve path", &dest, &e))?,
        tree_names: get_node(&data, &dest_rel).map(|n| child_entries(n).map(|(k, _)| k).collect()).unwrap_or_default(),
        copy,
        recycle,
    };
    for src in files {
        match place.one(&mut journal, src) {
            Ok(Outcome::Placed(name, note)) => {
                if let Some(note) = note {
                    warnings.push(format!("{}: {note}", src.display()));
                }
                placed.push(PlacedFile { name, folder: dest_rel.clone(), copied: copy });
            }
            Ok(Outcome::Skipped(note)) => warnings.push(format!("{}: {note}", src.display())),
            Err(why) => warnings.push(format!("{}: not placed, {why}", src.display())),
        }
    }
    // Each file stands alone: one that cannot be placed is reported and never undoes the others.
    journal.commit();
    let mut touched = vec![dest_rel];
    if !touched.contains(&target.to_vec()) {
        touched.push(target.to_vec());
    }
    for folder in source_folders {
        if !touched.contains(&folder) {
            touched.push(folder);
        }
    }
    let mutation = finish(root, &data, false, None, warnings, scope(&synced, touched));
    Ok(DropOutcome { mutation, placed, conflicts })
}

fn is_folder(src: &Path) -> bool {
    fs::symlink_metadata(src).is_ok_and(|meta| meta.is_dir())
}

// The storage folder a file sits in before it moves, so that folder's listing refreshes as well.
fn folder_of(storage: &Path, src: &Path) -> Option<Vec<String>> {
    let real = fs::canonicalize(src).ok()?;
    let inner = real.parent()?.strip_prefix(storage).ok()?;
    Some(inner.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect())
}
