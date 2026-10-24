// Temporary folders where a downloaded zip is expanded and checked before it becomes an extension.
use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::fail;
use super::manifest::has_manifest;
use super::store::extensions_dir;
use super::tools;
use crate::error::R;
use crate::fs_safe::{io_err, is_link};

const STAGING_DIR: &str = ".staging";
pub const ARCHIVE_FILE: &str = "archive.zip";
const FILES_DIR: &str = "files";
const MAX_AGE: Duration = Duration::from_secs(3600);

static COUNTER: AtomicU64 = AtomicU64::new(0);

// RandomState is seeded from the OS, so this needs no crate and tokens differ between runs.
pub fn new_token() -> String {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u128(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
    hasher.write_u32(std::process::id());
    format!("{:016x}", hasher.finish())
}

pub fn valid_token(token: &str) -> bool {
    token.len() == 16 && token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn expired(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified).is_ok_and(|age| age > MAX_AGE)
}

fn staging_root(root: &Path) -> PathBuf {
    extensions_dir(root).join(STAGING_DIR)
}

pub fn clear_old(root: &Path) -> R<()> {
    let dir = staging_root(root);
    if !dir.is_dir() {
        return Ok(());
    }
    let now = SystemTime::now();
    for entry in fs::read_dir(&dir).map_err(|e| io_err("read folder", &dir, &e))? {
        let entry = entry.map_err(|e| io_err("read folder", &dir, &e))?;
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| io_err("inspect path", &entry.path(), &e))?;
        let old = meta.modified().is_ok_and(|m| expired(m, now));
        if old && meta.is_dir() && !is_link(&meta) && valid_token(&entry.file_name().to_string_lossy()) {
            fs::remove_dir_all(entry.path()).map_err(|e| io_err("delete folder", &entry.path(), &e))?;
        }
    }
    Ok(())
}

// The cleanup error is logged because the error that caused it matters more to the caller.
pub fn discard(dir: &Path) {
    if let Err(e) = fs::remove_dir_all(dir) {
        if e.kind() != std::io::ErrorKind::NotFound {
            if let Err(log) = crate::debug_log::line("error", &io_err("delete folder", dir, &e).to_string()) {
                eprintln!("{log}");
            }
        }
    }
}

// .NET and unzip already refuse entries outside the target; this checks the result anyway.
pub fn check_tree(dir: &Path, inside: &Path) -> R<()> {
    for entry in fs::read_dir(dir).map_err(|e| io_err("read folder", dir, &e))? {
        let path = entry.map_err(|e| io_err("read folder", dir, &e))?.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
        let real = fs::canonicalize(&path).map_err(|e| io_err("resolve path", &path, &e))?;
        if is_link(&meta) || !real.starts_with(inside) {
            return Err(fail("The archive holds links or paths outside its folder".to_string()));
        }
        if meta.is_dir() {
            check_tree(&path, inside)?;
        }
    }
    Ok(())
}

pub fn create(root: &Path, bytes: &[u8]) -> R<(String, PathBuf)> {
    let token = new_token();
    let dir = staging_root(root).join(&token);
    fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
    let filled = (|| {
        let zip = dir.join(ARCHIVE_FILE);
        fs::write(&zip, bytes).map_err(|e| io_err("write file", &zip, &e))?;
        let files = dir.join(FILES_DIR);
        tools::expand(&zip, &files)?;
        let inside = fs::canonicalize(&files).map_err(|e| io_err("resolve path", &files, &e))?;
        check_tree(&files, &inside)
    })();
    if let Err(e) = filled {
        discard(&dir);
        return Err(e);
    }
    Ok((token, dir))
}

pub fn open(root: &Path, token: &str) -> R<PathBuf> {
    let dir = staging_root(root).join(token);
    if !valid_token(token) || !fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir() && !is_link(&m)) {
        return Err(fail("The preview expired; find the repository again".to_string()));
    }
    Ok(dir)
}

// Some(None) is the archive root itself, Some(Some(name)) its single top folder.
pub fn pick_root(entries: &[(String, bool)], root_has_manifest: bool) -> Option<Option<String>> {
    if root_has_manifest {
        return Some(None);
    }
    match entries {
        [(name, true)] => Some(Some(name.clone())),
        _ => None,
    }
}

pub fn manifest_dir(staged: &Path) -> R<PathBuf> {
    let files = staged.join(FILES_DIR);
    let mut entries = Vec::new();
    for entry in fs::read_dir(&files).map_err(|e| io_err("read folder", &files, &e))? {
        let entry = entry.map_err(|e| io_err("read folder", &files, &e))?;
        let is_dir = fs::symlink_metadata(entry.path()).is_ok_and(|m| m.is_dir() && !is_link(&m));
        entries.push((entry.file_name().to_string_lossy().into_owned(), is_dir));
    }
    let missing = || fail("No manifest.json at the root or in the single top folder".to_string());
    let dir = match pick_root(&entries, has_manifest(&files)).ok_or_else(missing)? {
        None => files,
        Some(name) => files.join(name),
    };
    if !has_manifest(&dir) {
        return Err(missing());
    }
    Ok(dir)
}
