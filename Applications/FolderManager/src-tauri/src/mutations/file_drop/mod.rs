// Places files dropped from Explorer into a storage folder, moved by default or copied when Ctrl is held.
mod naming;
mod target;
#[cfg(test)]
mod tests;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{exactly_on_disk, finish, load_synced_at, locate, Mutation};
use crate::ant_core::tree_types::child_entries;
use crate::config::{is_hidden_dir, SETTINGS_FOLDER};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, resolve, Journal, StorageLock};
use crate::tree::tree_store::{self, get_node};
pub(crate) use naming::free_name;
use target::destination;

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

pub fn drop_files(root: &Path, target: &[String], paths: &[PathBuf], copy: bool) -> R<DropOutcome> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced_at(root, target)?;
    if locate(root, &data, target)?.ghost {
        return Err(FmError::GhostNotAllowed { op: "drop files into".to_string() });
    }
    let node = get_node(&data, target).ok_or_else(|| FmError::NotFound { rel_parts: target.to_vec() })?;
    let dest_rel = destination(target, node)?;
    let dest = resolve(root, &dest_rel)?;
    if !exactly_on_disk(root, &dest_rel)? {
        return Err(FmError::NotFound { rel_parts: dest_rel });
    }
    if synced {
        tree_store::save(root, &data)?;
    }
    let place = Place {
        storage: fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?,
        dest: &dest,
        dest_real: fs::canonicalize(&dest).map_err(|e| io_err("resolve path", &dest, &e))?,
        tree_names: get_node(&data, &dest_rel).map(|n| child_entries(n).map(|(k, _)| k).collect()).unwrap_or_default(),
        copy,
    };
    let mut journal = Journal::new();
    let mut warnings = Vec::new();
    let mut placed = Vec::new();
    for src in paths {
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
    let mutation = finish(root, &data, synced, None, warnings, vec![dest_rel]);
    Ok(DropOutcome { mutation, placed })
}

impl Place<'_> {
    fn one(&self, journal: &mut Journal, src: &Path) -> Result<Outcome, String> {
        let real = self.check_source(src)?;
        if !self.copy && real.parent() == Some(self.dest_real.as_path()) {
            return Ok(Outcome::Skipped("already in this folder".to_string()));
        }
        let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
        let free = naming::free_name(self.dest, &name, &self.tree_names).map_err(|e| e.to_string())?;
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
        if let Ok(inner) = real.strip_prefix(&self.storage) {
            let parts: Vec<String> = inner.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
            let settings = parts.first().is_some_and(|top| top.eq_ignore_ascii_case(SETTINGS_FOLDER));
            if settings || parts.iter().any(|p| is_hidden_dir(p)) {
                return Err("the file belongs to Folder Manager".to_string());
            }
        }
        Ok(real)
    }
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
