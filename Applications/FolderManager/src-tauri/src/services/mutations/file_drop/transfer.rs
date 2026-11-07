// Takes a dropped path across drives: a copy that cleans up after itself, and a recycle of the original only while it still matches that copy.
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use crate::services::mutations::recycle::move_to_bin;
use crate::infra::fs_safe::{io_err, redirects};

// FAT drives round modified times to 2 seconds. Only an original newer than its copy counts, since a copy made off Windows gets its own time.
const TIME_SLACK: Duration = Duration::from_secs(2);

// Across drives a folder is copied and its original then recycled, as files are; links inside stop the copy before it starts.
pub(super) fn move_across(from: &Path, to: &Path, recycle: &dyn Fn(&Path) -> Result<(), String>) -> Result<Option<String>, String> {
    copy_whole(from, to)?;
    Ok(remove_original(from, to, recycle))
}

// What reached the original after the copy walked it exists nowhere else, and a drive with no Recycle Bin would make the removal final, so either keeps it.
pub(super) fn remove_original(from: &Path, to: &Path, recycle: &dyn Fn(&Path) -> Result<(), String>) -> Option<String> {
    let removed = matches_copy(from, to).and_then(|()| move_to_bin(from, recycle).map_err(|e| e.to_string()));
    removed.err().map(|why| format!("copied as {}, but the original could not be removed: {why}", to.display()))
}

// Only the original is walked: something the copy holds beyond it risks nothing when the original goes.
fn matches_copy(from: &Path, to: &Path) -> Result<(), String> {
    let inspect = |path: &Path| fs::symlink_metadata(path).map_err(|e| io_err("inspect path", path, &e).to_string());
    let (original, copy) = (inspect(from)?, inspect(to)?);
    let changed = || format!("{} changed during the copy", from.display());
    if redirects(from, &original).map_err(|e| e.to_string())? {
        return Err(changed());
    }
    if original.is_dir() {
        if !copy.is_dir() {
            return Err(changed());
        }
        for entry in fs::read_dir(from).map_err(|e| io_err("list folder", from, &e).to_string())? {
            let entry = entry.map_err(|e| io_err("list folder", from, &e).to_string())?;
            matches_copy(&entry.path(), &to.join(entry.file_name()))?;
        }
        return Ok(());
    }
    let modified = |meta: &fs::Metadata, path: &Path| meta.modified().map_err(|e| io_err("inspect path", path, &e).to_string());
    if copy.is_file() && original.len() == copy.len() && modified(&original, from)? <= modified(&copy, to)? + TIME_SLACK {
        Ok(())
    } else {
        Err(changed())
    }
}

// A folder copied in part is removed again; `to` is created first, so that removal only ever takes what this copy made.
pub(super) fn copy_whole(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir(to).map_err(|e| io_err("create folder", to, &e).to_string())?;
    if let Err(why) = copy_contents(from, to) {
        return match fs::remove_dir_all(to) {
            Ok(()) => Err(why),
            Err(r) => Err(format!("{why}; the partial copy {} could not be removed: {r}", to.display())),
        };
    }
    Ok(())
}

pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
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

// fs::copy alone would overwrite a file that appeared after the free name was picked; creating it first reserves the name.
pub(super) fn copy_new(from: &Path, to: &Path) -> Result<(), String> {
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
pub(super) fn crosses_devices(os_code: i32) -> bool {
    std::io::Error::from_raw_os_error(os_code).kind() == ErrorKind::CrossesDevices
}
