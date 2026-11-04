// A Ctrl-drag copies the folder with its contents, then gives every id inside the copy its own tail.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use super::insert;
use crate::ant_core::tree_types::META_KEYS;
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, redirects};
use crate::index::footprint;
use crate::mutations::{finish, register_ids, Mutation};
use crate::tree::depth;
use crate::tree::tree_store;

pub(super) fn copy_folder(
    root: &Path,
    mut data: Mapping,
    (from, to): (&Path, &Path),
    (rel, select): (&[String], Vec<String>),
    mut entry: Mapping,
    (synced, mut warnings): (bool, Vec<String>),
) -> R<Mutation> {
    fs::create_dir(to).map_err(|e| io_err("create folder", to, &e))?;
    if let Err(e) = copy_contents(from, to, &mut warnings) {
        return Err(undo(to, e));
    }
    strip_uids(&mut entry);
    let placed = depth::limit(root)
        .and_then(|limit| insert(root, &mut data, rel, entry, limit))
        .and_then(|()| footprint::tail_copy(root, &mut data, rel))
        .and_then(|()| tree_store::save(root, &data));
    if let Err(e) = placed {
        return Err(undo(to, e));
    }
    // Folders of the copy that had no .ant folder get theirs, with a fresh id, as after add_node.
    let moved = register_ids(root, &mut data, &mut warnings);
    Ok(finish(root, &data, synced || moved, Some(select), warnings, vec![rel.to_vec()]))
}

// The copy's tree entry starts without ids; tail_copy then stores the tailed id of every folder that has one.
fn strip_uids(node: &mut Mapping) {
    node.remove("uid");
    for (key, child) in node.iter_mut() {
        if let (Some(name), Value::Mapping(child)) = (key.as_str(), child) {
            if !META_KEYS.contains(&name) {
                strip_uids(child);
            }
        }
    }
}

// Links and junctions are skipped, not followed, so a copy never pulls in folders from outside the storage.
fn copy_contents(from: &Path, to: &Path, warnings: &mut Vec<String>) -> R<()> {
    for entry in fs::read_dir(from).map_err(|e| io_err("list folder", from, &e))? {
        let path = entry.map_err(|e| io_err("list folder", from, &e))?.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
        let Some(name) = path.file_name() else { continue };
        let target = to.join(name);
        if redirects(&path, &meta)? {
            warnings.push(format!("{} is a link and was not copied", path.display()));
        } else if meta.is_dir() {
            fs::create_dir(&target).map_err(|e| io_err("create folder", &target, &e))?;
            copy_contents(&path, &target, warnings)?;
        } else {
            fs::copy(&path, &target).map_err(|e| io_err("copy file", &path, &e))?;
        }
    }
    Ok(())
}

// The copy's folder was created by this copy, so removing it whole can only take what the copy made.
fn undo(to: &Path, cause: FmError) -> FmError {
    let op = "copy folder".to_string();
    match fs::remove_dir_all(to) {
        Ok(()) => FmError::RolledBack { op, cause: cause.to_string() },
        Err(e) => FmError::Inconsistent { op, cause: cause.to_string(), rollback_errors: vec![io_err("delete folder", to, &e).to_string()] },
    }
}
