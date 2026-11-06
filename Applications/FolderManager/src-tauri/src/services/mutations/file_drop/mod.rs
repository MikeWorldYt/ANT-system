// Places files dropped from Explorer into a storage folder, moved by default or copied when Ctrl is held.
mod target;
#[cfg(test)]
mod tests;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::recycle::move_to_bin;
use super::{child_rel, exactly_on_disk, finish, load_synced_at, locate, register_ids, scope, sync_branch_drift, sync_loaded_at, Drift, Mutation};
use crate::domain::names::{compose_name, next_track, is_workboard_name, name_parts};
use crate::domain::tree_types::{child_entries, type_base};
use crate::domain::config::{is_hidden_dir, BACKUP_FOLDER, SETTINGS_FOLDER};
use crate::infra::naming::free_name;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, join_rel, redirects, resolve, Journal, StorageLock};
use crate::infra::footprint_file;
use crate::services::index::footprint;
use crate::services::tree::{depth, review_links};
use crate::services::tree::tree_node::is_reserved;

use crate::services::tree::tree_store::{self, delete_key, get_node};
use serde_yaml::{Mapping, Value};
use target::destination;

const ALREADY_INSIDE: &str = "it is already in the storage; drag it inside the app to move it";

struct Place<'a> {
    storage: PathBuf,
    dest: &'a Path,
    dest_real: PathBuf,
    tree_names: Vec<&'a str>,
    copy: bool,
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

enum Outcome {
    // A placed file can still carry a note, as when a cross-drive move could not remove the original.
    Placed(String, Option<String>),
    Skipped(String),
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
    place_all(root, target, paths, copy, replace, recycle, false)
}

// The app resolved these sources to folders the tree does not track, the one case where a folder may come from inside the storage.
pub fn move_in(root: &Path, target: &[String], paths: &[PathBuf], copy: bool, recycle: &dyn Fn(&Path) -> Result<(), String>) -> R<DropOutcome> {
    place_all(root, target, paths, copy, &[], recycle, true)
}

fn place_all(
    root: &Path,
    target: &[String],
    paths: &[PathBuf],
    copy: bool,
    replace: &[PathBuf],
    recycle: &dyn Fn(&Path) -> Result<(), String>,
    untracked_inside: bool,
) -> R<DropOutcome> {
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
    for src in folders {
        match place_workboard(root, &mut data, target, &node_dir, src, copy, replace.contains(src), recycle, &mut journal) {
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
        match place_folder(&storage, &node_dir, src, copy, untracked_inside, &mut journal) {
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

// Moved or copied whole under a free name; a folder already in the storage is moved inside the app instead.
fn place_folder(storage: &Path, dest: &Path, src: &Path, copy: bool, untracked_inside: bool, journal: &mut Journal) -> Result<(String, Option<String>), String> {
    let real = fs::canonicalize(src).map_err(|e| io_err("resolve path", src, &e).to_string())?;
    if belongs_to_app(storage, &real) {
        return Err("the folder belongs to Folder Manager".to_string());
    }
    if !untracked_inside && real.starts_with(storage) {
        return Err(ALREADY_INSIDE.to_string());
    }
    let dest_real = fs::canonicalize(dest).map_err(|e| io_err("resolve path", dest, &e).to_string())?;
    if dest_real.starts_with(&real) {
        return Err("a folder cannot be placed inside itself".to_string());
    }
    let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
    let free = free_name(dest, &name, &[]).map_err(|e| e.to_string())?;
    let to = dest.join(&free);
    if copy {
        return copy_whole(&real, &to).map(|()| (free, None));
    }
    match journal.rename(&real, &to) {
        Ok(()) => Ok((free, None)),
        Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => move_across(&real, &to).map(|note| (free, note)),
        Err(e) => Err(e.to_string()),
    }
}

enum Workboard {
    // The note is set when the original could not be removed after a copy across drives.
    Placed(String, Option<String>),
    Taken(WorkboardConflict),
}

fn is_folder(src: &Path) -> bool {
    fs::symlink_metadata(src).is_ok_and(|meta| meta.is_dir())
}

// A workboard moves in with its folder and id; a track its new folder already uses is renumbered.
fn place_workboard(
    root: &Path,
    data: &mut Mapping,
    dest_rel: &[String],
    dest: &Path,
    src: &Path,
    copy: bool,
    replace: bool,
    recycle: &dyn Fn(&Path) -> Result<(), String>,
    journal: &mut Journal,
) -> Result<Workboard, String> {
    let real = fs::canonicalize(src).map_err(|e| io_err("resolve path", src, &e).to_string())?;
    let storage = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e).to_string())?;
    let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
    if !is_workboard_name(&name) {
        return Err("folders cannot be dropped yet, only files and workboards".to_string());
    }
    if copy {
        return copy_workboard(root, data, dest_rel, dest, &real, &name);
    }
    if real.starts_with(&storage) {
        return Err(ALREADY_INSIDE.to_string());
    }
    // A replacement keeps the track of the workboard it replaces, so the number a project is known by stays.
    let mut replaced_track = None;
    if let Some(owner) = footprint_file::read_id(&real).and_then(|id| owner_of(data, &id)) {
        if !replace {
            return Ok(Workboard::Taken(WorkboardConflict { name, source: src.to_path_buf(), existing: owner }));
        }
        move_to_bin(&join_rel(root, &owner), recycle).map_err(|e| e.to_string())?;
        if let Some((last, parent)) = owner.split_last() {
            delete_key(data, parent, last);
            replaced_track = Some(name_parts(last, true).0);
        }
    }
    let siblings: Vec<String> = get_node(data, dest_rel).map(|n| child_entries(n).map(|(k, _)| k.to_string()).collect()).unwrap_or_default();
    let free = match replaced_track {
        Some(track) => compose_name(&track, &name_parts(&name, true).1, true),
        None => next_track(&name, &siblings),
    };
    let to = dest.join(&free);
    if to.exists() {
        return Err(format!("a folder named {free} already exists"));
    }
    match journal.rename(&real, &to) {
        Ok(()) => Ok(Workboard::Placed(free, None)),
        Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => {
            let note = move_across(&real, &to)?;
            Ok(Workboard::Placed(free, note))
        }
        Err(e) => Err(e.to_string()),
    }
}

// A copy takes the next track of its folder and its own id, so the original and the copy never share either.
fn copy_workboard(root: &Path, data: &mut Mapping, dest_rel: &[String], dest: &Path, real: &Path, name: &str) -> Result<Workboard, String> {
    let dest_real = fs::canonicalize(dest).map_err(|e| io_err("resolve path", dest, &e).to_string())?;
    if dest_real.starts_with(real) {
        return Err("a workboard cannot be copied into itself".to_string());
    }
    let siblings: Vec<String> = get_node(data, dest_rel).map(|n| child_entries(n).map(|(k, _)| k.to_string()).collect()).unwrap_or_default();
    let free = next_track(name, &siblings);
    let to = dest.join(&free);
    if to.exists() {
        return Err(format!("a folder named {free} already exists"));
    }
    copy_whole(real, &to)?;
    let new_rel = child_rel(dest_rel, &free);
    tree_store::sync_branch(root, data, dest_rel).map_err(|e| e.to_string())?;
    footprint::tail_copy(root, data, &new_rel).map_err(|e| e.to_string())?;
    Ok(Workboard::Placed(free, None))
}

// Across drives a folder is copied and then removed from its place, as files are; links inside stop the copy before it starts.
fn move_across(from: &Path, to: &Path) -> Result<Option<String>, String> {
    copy_whole(from, to)?;
    Ok(fs::remove_dir_all(from).err().map(|e| format!("copied as {}, but the original could not be removed: {e}", to.display())))
}

// A folder copied in part is removed again; `to` is created first, so that removal only ever takes what this copy made.
fn copy_whole(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir(to).map_err(|e| io_err("create folder", to, &e).to_string())?;
    if let Err(why) = copy_contents(from, to) {
        return match fs::remove_dir_all(to) {
            Ok(()) => Err(why),
            Err(r) => Err(format!("{why}; the partial copy {} could not be removed: {r}", to.display())),
        };
    }
    Ok(())
}

pub(super) fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir(to).map_err(|e| io_err("create folder", to, &e).to_string())?;
    copy_contents(from, to)
}

fn copy_contents(from: &Path, to: &Path) -> Result<(), String> {
    for entry in fs::read_dir(from).map_err(|e| io_err("list folder", from, &e).to_string())? {
        let entry = entry.map_err(|e| io_err("list folder", from, &e).to_string())?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e).to_string())?;
        if redirects(&path, &meta).map_err(|e| e.to_string())? {
            return Err(format!("{} is a link, which cannot be moved across drives", path.display()));
        }
        let copy_to = to.join(entry.file_name());
        if meta.is_dir() {
            copy_tree(&path, &copy_to)?;
        } else {
            fs::copy(&path, &copy_to).map_err(|e| io_err("copy file", &path, &e).to_string())?;
        }
    }
    Ok(())
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

// The storage folder a file sits in before it moves, so that folder's listing refreshes as well.
fn folder_of(storage: &Path, src: &Path) -> Option<Vec<String>> {
    let real = fs::canonicalize(src).ok()?;
    let inner = real.parent()?.strip_prefix(storage).ok()?;
    Some(inner.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect())
}

impl Place<'_> {
    fn one(&self, journal: &mut Journal, src: &Path) -> Result<Outcome, String> {
        let real = self.check_source(src)?;
        if !self.copy && real.parent() == Some(self.dest_real.as_path()) {
            return Ok(Outcome::Skipped("already in this folder".to_string()));
        }
        let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
        let free = free_name(self.dest, &name, &self.tree_names).map_err(|e| e.to_string())?;
        let to = self.dest.join(&free);
        if self.copy {
            return copy_new(&real, &to).map(|()| Outcome::Placed(free, None));
        }
        match journal.rename(&real, &to) {
            Ok(()) => Ok(Outcome::Placed(free, None)),
            Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => {
                copy_new(&real, &to)?;
                let note = fs::remove_file(&real)
                    .err()
                    .map(|e| format!("copied as {free}, but the original could not be removed: {e}"));
                Ok(Outcome::Placed(free, note))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn check_source(&self, src: &Path) -> Result<PathBuf, String> {
        let meta = fs::symlink_metadata(src).map_err(|e| io_err("inspect file", src, &e).to_string())?;
        if meta.is_dir() {
            return Err("folders cannot be dropped yet, only files".to_string());
        }
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err("only regular files can be dropped".to_string());
        }
        let real = fs::canonicalize(src).map_err(|e| io_err("resolve path", src, &e).to_string())?;
        if belongs_to_app(&self.storage, &real) {
            return Err("the file belongs to Folder Manager".to_string());
        }
        Ok(real)
    }
}

// Settings, backups and every .ant folder are the app's own, whichever way they are dragged in.
fn belongs_to_app(storage: &Path, real: &Path) -> bool {
    let Ok(inner) = real.strip_prefix(storage) else { return false };
    let parts: Vec<String> = inner.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    let settings = parts.first().is_some_and(|top| top.eq_ignore_ascii_case(SETTINGS_FOLDER) || top.eq_ignore_ascii_case(BACKUP_FOLDER));
    settings || parts.iter().any(|p| is_hidden_dir(p))
}

// fs::copy alone would overwrite a file that appeared after the free name was picked; creating it first reserves the name.
fn copy_new(from: &Path, to: &Path) -> Result<(), String> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)
        .map_err(|e| io_err("create file", to, &e).to_string())?;
    let Err(e) = fs::copy(from, to) else { return Ok(()) };
    let cause = io_err("copy file", from, &e).to_string();
    match fs::remove_file(to) {
        Ok(()) => Err(cause),
        Err(r) => Err(format!("{cause}; the partial copy {} could not be removed: {r}", to.display())),
    }
}

// A move to another drive cannot be a rename, so it becomes a copy followed by removing the original.
fn crosses_devices(os_code: i32) -> bool {
    std::io::Error::from_raw_os_error(os_code).kind() == ErrorKind::CrossesDevices
}
