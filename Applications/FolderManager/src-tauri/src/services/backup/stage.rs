// What a backup holds and how it goes back: the walks never follow links and skip the backup folder itself.
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::domain::config::{is_hidden_dir, BACKUP_FOLDER, CORE_FOLDER, DATA_DIR, DB_FILENAME, SETTINGS_FOLDER};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{dir_entries, io_err, is_link, Journal, LOCK_FILE};
use crate::infra::storage::db::{db_path, hide, Store};
use crate::services::tree::depth;

pub(super) const LEGACY_BACKUP_DIR: &str = "03.Backup";

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

fn entries(dir: &Path) -> R<Vec<Entry>> {
    let mut out = Vec::new();
    for entry in dir_entries(dir, "read folder")? {
        let entry = entry?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("read folder", &path, &e))?;
        if !is_link(&meta) {
            out.push(Entry { name: entry.file_name().to_string_lossy().into_owned(), path, is_dir: meta.is_dir() });
        }
    }
    Ok(out)
}

fn real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !is_link(&m))
}

fn is_lock(rel: &Path) -> bool {
    rel == Path::new(DATA_DIR).join(LOCK_FILE) || rel == Path::new(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE)
}

// Scripts and programs never travel in a backup, so a crafted zip cannot plant one in the storage's data folders.
const RUNNABLE: [&str; 23] = ["bat", "cmd", "com", "exe", "msi", "msp", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "hta", "scr", "pif", "cpl", "msc", "lnk", "url", "reg", "dll"];

fn runnable(rel: &Path) -> bool {
    rel.extension().and_then(|e| e.to_str()).is_some_and(|e| RUNNABLE.iter().any(|r| e.eq_ignore_ascii_case(r)))
}

fn rel(base: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base).map(Path::to_path_buf).unwrap_or_default()
}

// The root's own ant.db and its journals are copied through SQLite, never as raw files.
fn tree_data(root: &Path, dir: &Path, depth: usize, limit: usize, skip: &Path, out: &mut Vec<PathBuf>) -> R<()> {
    let data = dir.join(DATA_DIR);
    if real_dir(&data) {
        for e in entries(&data)? {
            if !e.is_dir && !(depth == 0 && e.name.starts_with(DB_FILENAME)) && !is_lock(&rel(root, &e.path)) {
                out.push(rel(root, &e.path));
            }
        }
    }
    if depth > limit {
        return Ok(());
    }
    for e in entries(dir)? {
        let app_owned = depth == 0 && (e.name.eq_ignore_ascii_case(SETTINGS_FOLDER) || e.name.eq_ignore_ascii_case(BACKUP_FOLDER));
        if e.is_dir && !is_hidden_dir(&e.name) && !app_owned && e.path != skip {
            tree_data(root, &e.path, depth + 1, limit, skip, out)?;
        }
    }
    Ok(())
}

fn all_files(base: &Path, dir: &Path, skip: &[&Path], out: &mut Vec<PathBuf>) -> R<()> {
    for e in entries(dir)? {
        if skip.contains(&e.path.as_path()) {
            continue;
        }
        if e.is_dir {
            all_files(base, &e.path, skip, out)?;
        } else if !is_lock(&rel(base, &e.path)) {
            out.push(rel(base, &e.path));
        }
    }
    Ok(())
}

fn copy(from: &Path, to: &Path) -> R<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err("create folder", parent, &e))?;
    }
    fs::copy(from, to).map(|_| ()).map_err(|e| io_err("copy file", to, &e))
}

// `skip` is the canonical backup folder, so earlier backups and the staging folder stay out.
pub fn stage(root: &Path, stage: &Path, skip: &Path) -> R<()> {
    let base = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    let db = stage.join(DATA_DIR).join(DB_FILENAME);
    fs::create_dir_all(stage.join(DATA_DIR)).map_err(|e| io_err("create folder", stage, &e))?;
    Store::open_or_create(root)?.vacuum_into(&db)?;
    let mut files = Vec::new();
    tree_data(&base, &base, 0, depth::limit(root)?, skip, &mut files)?;
    let settings = base.join(SETTINGS_FOLDER);
    if real_dir(&settings) {
        // Older storages keep their backups beside the settings; they never go into a new backup.
        all_files(&base, &settings, &[skip, &settings.join(LEGACY_BACKUP_DIR)], &mut files)?;
    }
    files.iter().filter(|f| !runnable(f)).try_for_each(|f| copy(&base.join(f), &stage.join(f)))
}

pub fn has_db(stage: &Path) -> bool {
    stage.join(DATA_DIR).join(DB_FILENAME).is_file()
}

pub fn check_staged_db(stage: &Path) -> R<()> {
    check_db(&stage.join(DATA_DIR).join(DB_FILENAME))
}

fn check_db(path: &Path) -> R<()> {
    let refused = |cause: String| FmError::Internal { message: format!("The backup's ant.db is not usable: {cause}") };
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| refused(e.to_string()))?;
    let first: String = conn.query_row("PRAGMA quick_check(1)", [], |r| r.get(0)).map_err(|e| refused(e.to_string()))?;
    if first == "ok" {
        return Ok(());
    }
    Err(refused(first))
}

struct Swap {
    tmp: PathBuf,
    target: PathBuf,
}

fn remove_tmp(tmp: &Path) {
    match fs::remove_file(tmp) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => eprintln!("Could not remove {}: {e}", tmp.display()),
        _ => {}
    }
}

// Only a regular file is ever replaced, so a folder or link in the way stops the restore before anything moved.
fn copy_beside(from: &Path, target: &Path, tag: &str, swaps: &mut Vec<Swap>) -> R<()> {
    if fs::symlink_metadata(target).is_ok_and(|m| !m.is_file() || is_link(&m)) {
        return Err(FmError::Internal { message: format!("{} is not a file, so the backup cannot replace it", target.display()) });
    }
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let tmp = target.with_file_name(format!("{name}.restore-new-{tag}"));
    swaps.push(Swap { tmp: tmp.clone(), target: target.to_path_buf() });
    fs::copy(from, &tmp).map(|_| ()).map_err(|e| io_err("copy file", &tmp, &e))
}

fn make_dirs(dir: &Path, made: &mut Journal) -> R<()> {
    if real_dir(dir) {
        return Ok(());
    }
    if let Some(parent) = dir.parent() {
        make_dirs(parent, made)?;
    }
    made.mkdir(dir)
}

// Folders that are gone since the backup are skipped, so a restore never recreates the tree.
fn destination(root: &Path, file: &Path, made: &mut Journal) -> R<Option<PathBuf>> {
    if file.starts_with(SETTINGS_FOLDER) {
        make_dirs(root.join(file).parent().unwrap_or(root), made)?;
        return Ok(Some(root.join(file)));
    }
    let Some(data) = file.parent().filter(|p| p.file_name().is_some_and(|n| n == DATA_DIR)) else { return Ok(None) };
    let folder = root.join(data.parent().unwrap_or(Path::new("")));
    if !real_dir(&folder) {
        return Ok(None);
    }
    let data_dir = folder.join(DATA_DIR);
    if !data_dir.is_dir() {
        made.mkdir(&data_dir)?;
        hide(&data_dir);
    }
    Ok(Some(root.join(file)))
}

// The first phase copies every file beside its target and checks the db, so a failure there leaves the storage as it was.
fn prepare(root: &Path, stage: &Path, tag: &str, made: &mut Journal, swaps: &mut Vec<Swap>) -> R<()> {
    let mut files = Vec::new();
    all_files(stage, stage, &[], &mut files)?;
    let db = Path::new(DATA_DIR).join(DB_FILENAME);
    copy_beside(&stage.join(&db), &db_path(root), tag, swaps)?;
    check_db(&swaps[0].tmp)?;
    for file in files.iter().filter(|f| **f != db && !is_lock(f) && !runnable(f)) {
        if let Some(target) = destination(root, file, made)? {
            copy_beside(&stage.join(file), &target, tag, swaps)?;
        }
    }
    Ok(())
}

// SQLite would replay an old WAL over the new db, so the journals step aside with it.
fn swap(journal: &mut Journal, s: &Swap, tag: &str, old: &mut Vec<PathBuf>) -> R<()> {
    let name = s.target.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let journals: &[&str] = if name == DB_FILENAME { &["-wal", "-shm", ""] } else { &[""] };
    for suffix in journals {
        let path = s.target.with_file_name(format!("{name}{suffix}"));
        if fs::symlink_metadata(&path).is_ok() {
            let kept = s.target.with_file_name(format!("{name}{suffix}.restore-old-{tag}"));
            journal.rename(&path, &kept)?;
            old.push(kept);
        }
    }
    journal.rename(&s.tmp, &s.target)
}

// The second phase only renames, and a failure renames everything back.
fn commit(swaps: &[Swap], tag: &str) -> R<()> {
    let mut journal = Journal::new();
    let mut old = Vec::new();
    if let Err(e) = swaps.iter().try_for_each(|s| swap(&mut journal, s, tag, &mut old)) {
        return Err(journal.abort("restore backup", e));
    }
    journal.commit();
    old.iter().for_each(|kept| remove_tmp(kept));
    Ok(())
}

fn restore_tag() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}

pub fn apply(root: &Path, stage: &Path) -> R<()> {
    apply_tagged(root, stage, &restore_tag())
}

pub(super) fn apply_tagged(root: &Path, stage: &Path, tag: &str) -> R<()> {
    let mut made = Journal::new();
    let mut swaps = Vec::new();
    let result = prepare(root, stage, tag, &mut made, &mut swaps).and_then(|()| commit(&swaps, tag));
    swaps.iter().for_each(|s| remove_tmp(&s.tmp));
    if result.is_err() {
        made.rollback().iter().for_each(|e| eprintln!("Could not undo a restore folder: {e}"));
    } else {
        made.commit();
    }
    result
}
