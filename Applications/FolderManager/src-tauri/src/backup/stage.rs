// What a backup holds and how it goes back: the walks never follow links and skip the backup folder itself.
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{is_hidden_dir, CORE_FOLDER, DATA_DIR, DB_FILENAME, SETTINGS_FOLDER};
use crate::error::R;
use crate::fs_safe::{io_err, is_link, LOCK_FILE};
use crate::storage::db::{db_path, hide, Store};
use crate::tree::depth;

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

fn entries(dir: &Path) -> R<Vec<Entry>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_err("read folder", dir, &e))? {
        let entry = entry.map_err(|e| io_err("read folder", dir, &e))?;
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
    rel == Path::new(SETTINGS_FOLDER).join(CORE_FOLDER).join(LOCK_FILE)
}

fn rel(base: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base).map(Path::to_path_buf).unwrap_or_default()
}

// The root's own ant.db and its journals are copied through SQLite, never as raw files.
fn tree_data(root: &Path, dir: &Path, depth: usize, limit: usize, skip: &Path, out: &mut Vec<PathBuf>) -> R<()> {
    let data = dir.join(DATA_DIR);
    if real_dir(&data) {
        for e in entries(&data)? {
            if !e.is_dir && !(depth == 0 && e.name.starts_with(DB_FILENAME)) {
                out.push(rel(root, &e.path));
            }
        }
    }
    if depth > limit {
        return Ok(());
    }
    for e in entries(dir)? {
        let settings = depth == 0 && e.name.eq_ignore_ascii_case(SETTINGS_FOLDER);
        if e.is_dir && !is_hidden_dir(&e.name) && !settings && e.path != skip {
            tree_data(root, &e.path, depth + 1, limit, skip, out)?;
        }
    }
    Ok(())
}

fn all_files(base: &Path, dir: &Path, skip: &Path, out: &mut Vec<PathBuf>) -> R<()> {
    for e in entries(dir)? {
        if e.path == skip {
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
        all_files(&base, &settings, skip, &mut files)?;
    }
    files.iter().try_for_each(|f| copy(&base.join(f), &stage.join(f)))
}

pub fn has_db(stage: &Path) -> bool {
    stage.join(DATA_DIR).join(DB_FILENAME).is_file()
}

fn replace_db(root: &Path, from: &Path) -> R<()> {
    let target = db_path(root);
    for stale in ["-wal", "-shm"] {
        let path = target.with_file_name(format!("{DB_FILENAME}{stale}"));
        match fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io_err("delete file", &path, &e)),
            _ => {}
        }
    }
    let tmp = target.with_file_name(format!(".{DB_FILENAME}.restore.tmp"));
    copy(from, &tmp)?;
    fs::rename(&tmp, &target).map_err(|e| io_err("replace file", &target, &e))
}

// Folders that are gone since the backup are skipped, so a restore never recreates the tree.
pub fn apply(root: &Path, stage: &Path) -> R<()> {
    let mut files = Vec::new();
    all_files(stage, stage, Path::new(""), &mut files)?;
    let db = Path::new(DATA_DIR).join(DB_FILENAME);
    replace_db(root, &stage.join(&db))?;
    for file in files.iter().filter(|f| **f != db && !is_lock(f)) {
        if file.starts_with(SETTINGS_FOLDER) {
            copy(&stage.join(file), &root.join(file))?;
            continue;
        }
        let Some(data) = file.parent().filter(|p| p.file_name().is_some_and(|n| n == DATA_DIR)) else { continue };
        let folder = root.join(data.parent().unwrap_or(Path::new("")));
        if !real_dir(&folder) {
            continue;
        }
        let data_dir = folder.join(DATA_DIR);
        if !data_dir.is_dir() {
            fs::create_dir_all(&data_dir).map_err(|e| io_err("create folder", &data_dir, &e))?;
            hide(&data_dir);
        }
        copy(&stage.join(file), &root.join(file))?;
    }
    Ok(())
}
