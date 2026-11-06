// Zip backups of the ANT data (ant.db, the .ant files and the legacy 999.Settings), never of the user's own files.
mod archive;
mod latest;
pub mod schedule;
mod stage;
pub(crate) mod stamp;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::domain::config::{BACKUP_FOLDER, SETTINGS_FOLDER};
use crate::services::debug_log::valid_stamp;
use crate::domain::validation::valid_backup_folder;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{dir_entries, io_err, is_link, rename_no_replace, StorageLock};
use crate::infra::storage::db::Store;
use crate::infra::storage::recovery::{db_state, set_aside, DbState};

pub use latest::{newest_default, restore_latest, LatestBackup};

const SUFFIX: &str = "_ANT-fm-backup";
pub const DEFAULT_FREQUENCY: &str = "day";

#[derive(Serialize)]
pub struct BackupEntry {
    name: String,
    path: String,
}

fn fail(message: String) -> FmError {
    FmError::Internal { message }
}

pub fn settings(root: &Path) -> R<HashMap<String, String>> {
    Store::open_existing(root)?.map_or_else(|| Ok(HashMap::new()), |store| store.settings())
}

fn default_dir(root: &Path) -> PathBuf {
    root.join(BACKUP_FOLDER)
}

// Backups an older storage made beside its settings stay listed and restorable until the upgrade moves them.
fn legacy_dir(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(stage::LEGACY_BACKUP_DIR)
}

// Where backups are read from: the chosen folder, or both default ones.
fn read_dirs(root: &Path, settings: &HashMap<String, String>) -> R<Vec<PathBuf>> {
    let dir = backup_dir(root, settings)?;
    Ok(if dir == default_dir(root) { vec![dir, legacy_dir(root)] } else { vec![dir] })
}

// The setting is checked again because ant.db can be edited outside the app.
fn backup_dir(root: &Path, settings: &HashMap<String, String>) -> R<PathBuf> {
    match settings.get("backupFolder").map(String::as_str) {
        None | Some("") => Ok(default_dir(root)),
        Some(folder) if valid_backup_folder(folder) => Ok(PathBuf::from(folder)),
        Some(folder) => Err(fail(format!("Invalid backup folder {folder}"))),
    }
}

pub fn valid_name(name: &str) -> bool {
    let Some(base) = name.strip_suffix(".zip") else { return false };
    let Some((stamp, rest)) = base.split_once(SUFFIX) else { return false };
    let numbered = rest.strip_prefix('_').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    valid_stamp(stamp) && (rest.is_empty() || numbered)
}

fn free_zip(dir: &Path, stamp: &str) -> PathBuf {
    let first = dir.join(format!("{stamp}{SUFFIX}.zip"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stamp}{SUFFIX}_{n}.zip"))).find(|p| !p.exists()).unwrap_or(first)
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

// The staging folder is removed on success and on error alike.
fn staged<T>(staging: &Path, f: impl FnOnce() -> R<T>) -> R<T> {
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|e| io_err("delete folder", staging, &e))?;
    }
    let result = f();
    if let Err(e) = fs::remove_dir_all(staging) {
        if e.kind() != std::io::ErrorKind::NotFound && result.is_ok() {
            return Err(io_err("delete folder", staging, &e));
        }
    }
    result
}

fn remove_part(part: &Path) -> R<()> {
    match fs::remove_file(part) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io_err("delete file", part, &e)),
        _ => Ok(()),
    }
}

// Only a finished zip gets a name the backup lists accept, so a killed run can never become the newest backup.
fn write_zip(zip: &Path, compress: impl FnOnce(&Path) -> R<()>) -> R<()> {
    let mut part = zip.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    remove_part(&part)?;
    let written = compress(&part).and_then(|()| rename_no_replace("rename file", &part, zip));
    if let Err(e) = remove_part(&part) {
        eprintln!("Could not remove {}: {e}", part.display());
    }
    written
}

pub fn create(root: &Path, stamp: &str) -> R<PathBuf> {
    if !valid_stamp(stamp) {
        return Err(fail(format!("Invalid backup stamp {stamp}")));
    }
    let dir = backup_dir(root, &settings(root)?)?;
    fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
    let skip = fs::canonicalize(&dir).map_err(|e| io_err("resolve folder", &dir, &e))?;
    let zip = free_zip(&dir, stamp);
    let staging = dir.join(format!(".{stamp}_staging"));
    staged(&staging, || {
        stage::stage(root, &staging, &skip)?;
        archive::compress(&staging, &zip)
    })?;
    Store::open_or_create(root)?.set_setting("backupLast", &now().to_string())?;
    Ok(zip)
}

pub fn if_due(root: &Path, stamp: &str) -> R<Option<PathBuf>> {
    let s = settings(root)?;
    let auto = s.get("backupAuto").is_some_and(|v| v == "true");
    let frequency = s.get("backupFrequency").map_or(DEFAULT_FREQUENCY, String::as_str);
    let last = s.get("backupLast").and_then(|v| v.parse().ok());
    if !schedule::is_due(auto, frequency, last, now()) {
        return Ok(None);
    }
    create(root, stamp).map(Some)
}

pub fn list(root: &Path) -> R<Vec<BackupEntry>> {
    entries_in(&read_dirs(root, &settings(root)?)?)
}

fn entries_in(dirs: &[PathBuf]) -> R<Vec<BackupEntry>> {
    let mut out = Vec::new();
    for dir in dirs {
        out.extend(entries(dir)?);
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

fn entries(dir: &Path) -> R<Vec<BackupEntry>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in dir_entries(dir, "read folder")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let file = fs::symlink_metadata(entry.path()).is_ok_and(|m| m.is_file() && !is_link(&m));
        if file && valid_name(&name) {
            out.push(BackupEntry { name, path: entry.path().to_string_lossy().into_owned() });
        }
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

fn backup_zip(dir: &Path, name: &str) -> R<PathBuf> {
    let zip = dir.join(name);
    let file = fs::symlink_metadata(&zip).is_ok_and(|m| m.is_file() && !is_link(&m));
    if !valid_name(name) || !file {
        return Err(fail(format!("{name} is not a backup of this storage")));
    }
    Ok(zip)
}

// A safety backup comes first so the restore itself can be undone.
pub fn restore(root: &Path, name: &str, stamp: &str) -> R<()> {
    if !valid_stamp(stamp) {
        return Err(fail(format!("Invalid backup stamp {stamp}")));
    }
    let dirs = read_dirs(root, &settings(root)?)?;
    let dir = dirs.iter().find(|d| d.join(name).is_file()).unwrap_or(&dirs[0]);
    restore_from(root, dir, name, stamp)
}

fn unpack(zip: &Path, staging: &Path, name: &str) -> R<()> {
    archive::expand(zip, staging)?;
    if stage::has_db(staging) {
        return Ok(());
    }
    Err(fail(format!("{name} holds no ant.db")))
}

// Without a readable ant.db there is nothing worth a safety backup, and an unreadable one is kept aside instead.
fn install(root: &Path, staging: &Path, stamp: &str) -> R<()> {
    let safety = match db_state(root) {
        DbState::Healthy => Some(create(root, stamp)?),
        DbState::Unreadable => {
            set_aside(root, stamp)?;
            None
        }
        DbState::Missing => None,
    };
    let _lock = StorageLock::acquire(root)?;
    let applied = stage::apply(root, staging);
    // The restored ant.db predates the safety backup, so its last-backup time is renewed.
    let restored = applied.and_then(|()| Store::open_or_create(root)?.set_setting("backupLast", &now().to_string()));
    match (restored, safety) {
        (Err(e), Some(zip)) => {
            let name = zip.file_name().unwrap_or_default().to_string_lossy();
            Err(fail(format!("{e}. The storage as it was before the restore is in the backup {name}")))
        }
        (result, _) => result,
    }
}

// The inner error means the zip is unusable and the storage is untouched, so another backup can still be tried.
fn try_restore(root: &Path, dir: &Path, name: &str, stamp: &str) -> R<R<()>> {
    let zip = backup_zip(dir, name)?;
    let staging = dir.join(format!(".{stamp}_restore"));
    staged(&staging, || match unpack(&zip, &staging, name) {
        Ok(()) => install(root, &staging, stamp).map(Ok),
        Err(e) => Ok(Err(e)),
    })
}

fn restore_from(root: &Path, dir: &Path, name: &str, stamp: &str) -> R<()> {
    try_restore(root, dir, name, stamp)?
}

pub fn run_headless(start: Option<OsString>) -> i32 {
    let Some(root) = start.map(PathBuf::from).and_then(|p| crate::infra::scan::find_root(&p)) else {
        eprintln!("--backup needs the path of an ANT storage");
        return 1;
    };
    match if_due(&root, &stamp::local_stamp()) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Backup failed: {e}");
            1
        }
    }
}
