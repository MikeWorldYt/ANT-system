// Places dropped files and plain folders, each under a free name; unlike workboards they never touch the tree.
use std::fs;
use std::path::{Path, PathBuf};

use super::transfer::{copy_new, copy_whole, crosses_devices, move_across, remove_original};
use super::{Placement, ALREADY_INSIDE};
use crate::domain::config::{is_hidden_dir, BACKUP_FOLDER, SETTINGS_FOLDER};
use crate::domain::error::FmError;
use crate::infra::fs_safe::{io_err, Journal};
use crate::infra::naming::free_name;

pub(super) struct Place<'a> {
    pub(super) storage: PathBuf,
    pub(super) dest: &'a Path,
    pub(super) dest_real: PathBuf,
    pub(super) tree_names: Vec<&'a str>,
    pub(super) copy: bool,
    pub(super) recycle: &'a dyn Fn(&Path) -> Result<(), String>,
}

pub(super) enum Outcome {
    // A placed file can still carry a note, as when a cross-drive move could not remove the original.
    Placed(String, Option<String>),
    Skipped(String),
}

// Moved or copied whole under a free name; a folder already in the storage is moved inside the app instead.
pub(super) fn place_folder(
    storage: &Path,
    dest: &Path,
    src: &Path,
    placement: Placement,
    recycle: &dyn Fn(&Path) -> Result<(), String>,
    journal: &mut Journal,
) -> Result<(String, Option<String>), String> {
    let real = fs::canonicalize(src).map_err(|e| io_err("resolve path", src, &e).to_string())?;
    if belongs_to_app(storage, &real) {
        return Err("the folder belongs to Folder Manager".to_string());
    }
    if placement == Placement::Move && real.starts_with(storage) {
        return Err(ALREADY_INSIDE.to_string());
    }
    let dest_real = fs::canonicalize(dest).map_err(|e| io_err("resolve path", dest, &e).to_string())?;
    if dest_real.starts_with(&real) {
        return Err("a folder cannot be placed inside itself".to_string());
    }
    let name = real.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("the path has no file name")?;
    let free = free_name(dest, &name, &[]).map_err(|e| e.to_string())?;
    let to = dest.join(&free);
    if placement == Placement::Copy {
        return copy_whole(&real, &to).map(|()| (free, None));
    }
    match journal.rename(&real, &to) {
        Ok(()) => Ok((free, None)),
        Err(FmError::Io { os_code: Some(code), .. }) if crosses_devices(code) => move_across(&real, &to, recycle).map(|note| (free, note)),
        Err(e) => Err(e.to_string()),
    }
}

impl Place<'_> {
    pub(super) fn one(&self, journal: &mut Journal, src: &Path) -> Result<Outcome, String> {
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
                let note = remove_original(&real, &to, self.recycle);
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
