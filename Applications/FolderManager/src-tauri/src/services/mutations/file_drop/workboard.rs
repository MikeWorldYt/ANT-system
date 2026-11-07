// Places dropped workboards, which bring their folder and id into the tree and can replace the one holding that id.
use std::fs;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::services::mutations::recycle::move_to_bin;
use crate::services::mutations::{child_rel, guard_frozen};
use super::transfer::{copy_whole, crosses_devices, move_across, remove_original};
use super::{WorkboardConflict, ALREADY_INSIDE};
use crate::domain::error::FmError;
use crate::domain::names::{compose_name, is_workboard_name, name_parts, next_track};
use crate::domain::tree_types::{child_entries, type_base};
use crate::infra::footprint_file;
use crate::infra::fs_safe::{io_err, join_rel, same_name, Journal};
use crate::infra::naming::free_name;
use crate::services::index::footprint;
use crate::services::tree::tree_store::{self, delete_key, get_node};

pub(super) enum Workboard {
    // The note is set when the original could not be removed after a copy across drives.
    Placed(String, Option<String>),
    Taken(WorkboardConflict),
}

// What every workboard of one drop shares; `dest` is the folder on disk behind `dest_rel`.
pub(super) struct WorkboardDrop<'a> {
    pub(super) root: &'a Path,
    pub(super) dest_rel: &'a [String],
    pub(super) dest: &'a Path,
    pub(super) copy: bool,
    pub(super) recycle: &'a dyn Fn(&Path) -> Result<(), String>,
}

impl WorkboardDrop<'_> {
    // A workboard moves in with its folder and id; a track its new folder already uses is renumbered.
    pub(super) fn place(&self, data: &mut Mapping, src: &Path, replace: bool, journal: &mut Journal) -> Result<Workboard, String> {
        let real = fs::canonicalize(src).map_err(|e| io_err("resolve path", src, &e).to_string())?;
        let storage = fs::canonicalize(self.root).map_err(|e| io_err("resolve storage", self.root, &e).to_string())?;
        let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
        if !is_workboard_name(&name) {
            return Err("folders cannot be dropped yet, only files and workboards".to_string());
        }
        if self.copy {
            return self.copy_in(data, &real, &name);
        }
        if real.starts_with(&storage) {
            return Err(ALREADY_INSIDE.to_string());
        }
        if let Some(owner) = footprint_file::read_id(&real).and_then(|id| owner_of(data, &id)) {
            if !replace {
                return Ok(Workboard::Taken(WorkboardConflict { name, source: src.to_path_buf(), existing: owner }));
            }
            return self.replace(data, &real, &name, &owner, journal);
        }
        let siblings: Vec<String> = get_node(data, self.dest_rel).map(|n| child_entries(n).map(|(k, _)| k.to_string()).collect()).unwrap_or_default();
        let free = next_track(&name, &siblings);
        let to = self.dest.join(&free);
        if to.exists() {
            return Err(format!("a folder named {free} already exists"));
        }
        match journal.rename(&real, &to) {
            Ok(()) => Ok(Workboard::Placed(free, None)),
            Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => {
                let note = move_across(&real, &to, self.recycle)?;
                Ok(Workboard::Placed(free, note))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    // The old workboard goes to the bin only once the new one waits beside its final name, so a refused replace changes nothing.
    fn replace(&self, data: &mut Mapping, real: &Path, name: &str, owner: &[String], journal: &mut Journal) -> Result<Workboard, String> {
        let (last, parent) = owner.split_last().ok_or("the workboard has no place in the tree")?;
        let kind = tree_store::get_type(data, owner);
        if type_base(&kind) != "WKB" {
            return Err("only a workboard can be replaced".to_string());
        }
        guard_frozen(owner, &kind).map_err(|e| e.to_string())?;
        let old = join_rel(self.root, owner);
        let inner = fs::canonicalize(self.dest).map_err(|e| io_err("resolve path", self.dest, &e).to_string())?;
        if inner.starts_with(fs::canonicalize(&old).map_err(|e| io_err("resolve path", &old, &e).to_string())?) {
            return Err("the drop target is inside the workboard being replaced".to_string());
        }
        // A replacement keeps the track of the workboard it replaces, so the number a project is known by stays.
        let free = compose_name(&name_parts(last, true).0, &name_parts(name, true).1, true);
        let to = self.dest.join(&free);
        if !(parent == self.dest_rel && same_name(last, &free)) && fs::symlink_metadata(&to).is_ok() {
            return Err(format!("a folder named {free} already exists"));
        }
        let temp = self.dest.join(free_name(self.dest, &format!("{free}.replacing"), &[]).map_err(|e| e.to_string())?);
        let renamed = match journal.rename(real, &temp) {
            Ok(()) => true,
            Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => {
                copy_whole(real, &temp)?;
                false
            }
            Err(e) => return Err(e.to_string()),
        };
        // Across drives the original is still in place, so only the copy has to go.
        let put_back = |journal: &mut Journal, why: String| {
            let undone = if renamed { journal.rename(&temp, real).map_err(|e| e.to_string()) } else { fs::remove_dir_all(&temp).map_err(|e| e.to_string()) };
            match undone {
                Ok(()) => why,
                Err(r) => format!("{why}; the new workboard stays as {}: {r}", temp.display()),
            }
        };
        if let Err(e) = move_to_bin(&old, self.recycle) {
            return Err(put_back(journal, e.to_string()));
        }
        delete_key(data, parent, last);
        if let Err(e) = journal.rename(&temp, &to) {
            return Err(put_back(journal, format!("{e}; the old workboard is in the Recycle Bin")));
        }
        Ok(Workboard::Placed(free, if renamed { None } else { remove_original(real, &to, self.recycle) }))
    }

    // A copy takes the next track of its folder and its own id, so the original and the copy never share either.
    fn copy_in(&self, data: &mut Mapping, real: &Path, name: &str) -> Result<Workboard, String> {
        let dest_real = fs::canonicalize(self.dest).map_err(|e| io_err("resolve path", self.dest, &e).to_string())?;
        if dest_real.starts_with(real) {
            return Err("a workboard cannot be copied into itself".to_string());
        }
        let siblings: Vec<String> = get_node(data, self.dest_rel).map(|n| child_entries(n).map(|(k, _)| k.to_string()).collect()).unwrap_or_default();
        let free = next_track(name, &siblings);
        let to = self.dest.join(&free);
        if to.exists() {
            return Err(format!("a folder named {free} already exists"));
        }
        copy_whole(real, &to)?;
        let new_rel = child_rel(self.dest_rel, &free);
        tree_store::sync_branch(self.root, data, self.dest_rel).map_err(|e| e.to_string())?;
        footprint::tail_copy(self.root, data, &new_rel).map_err(|e| e.to_string())?;
        Ok(Workboard::Placed(free, None))
    }
}

// The tree path of the node whose uid is `id`, if the storage already holds it.
fn owner_of(data: &Mapping, id: &str) -> Option<Vec<String>> {
    fn walk(node: &Mapping, path: &mut Vec<String>, id: &str) -> Option<Vec<String>> {
        for (name, child) in child_entries(node) {
            path.push(name.to_string());
            if child.get("uid").and_then(Value::as_str) == Some(id) {
                return Some(path.clone());
            }
            if let Some(found) = walk(child, path, id) {
                return Some(found);
            }
            path.pop();
        }
        None
    }
    walk(data, &mut Vec::new(), id)
}
