// SQLite store of the tree and its search fields under <root>/.ant; the only copy of the tree.
mod meta;
mod query;
mod tree;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::config::{DATA_DIR, DB_FILENAME};
use crate::error::{FmError, R};
use crate::fs_safe::io_err;

pub(crate) use query::{Doc, Suggestion};

const SCHEMA_VERSION: i64 = 1;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    depth INTEGER NOT NULL,
    type TEXT,
    status TEXT,
    extra TEXT,
    UNIQUE (parent_id, name)
);
CREATE INDEX IF NOT EXISTS nodes_name ON nodes (name COLLATE NOCASE);
CREATE TABLE IF NOT EXISTS node_fields (
    path TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (path, key)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS footprints (id TEXT PRIMARY KEY, path TEXT NOT NULL) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS node_fields_key_value ON node_fields (key, value COLLATE NOCASE);
";

pub fn db_path(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(DB_FILENAME)
}

// FNV-1a: std's hashers are not stable across Rust versions and the value is stored on disk.
pub fn hash(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")
}

pub struct Store {
    conn: Connection,
    path: PathBuf,
}

impl Store {
    pub fn open_existing(root: &Path) -> R<Option<Store>> {
        let path = db_path(root);
        if !path.is_file() {
            return Ok(None);
        }
        Self::open(path).map(Some)
    }

    pub fn open_or_create(root: &Path) -> R<Store> {
        let dir = root.join(DATA_DIR);
        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
            hide(&dir);
        }
        Self::open(db_path(root))
    }

    fn open(path: PathBuf) -> R<Store> {
        let fail = |e: rusqlite::Error| db_err("open ant.db", &path, &e);
        let conn = Connection::open(&path).map_err(fail)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(fail)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(fail)?;
        if version > SCHEMA_VERSION {
            return Err(FmError::Internal { message: "ant.db was written by a newer version of the app".to_string() });
        }
        if version < SCHEMA_VERSION {
            conn.execute_batch(SCHEMA).map_err(fail)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(fail)?;
        }
        Ok(Store { conn, path })
    }
}

fn db_err(op: &str, path: &Path, e: &rusqlite::Error) -> FmError {
    FmError::Io { op: op.to_string(), path: path.display().to_string(), message: e.to_string(), os_code: None }
}

// A failed attrib only leaves the folder visible, so it is reported on stderr and the save goes on.
#[cfg(windows)]
pub(crate) fn hide(dir: &Path) {
    use std::os::windows::process::CommandExt;
    let hidden = std::process::Command::new("attrib").arg("+h").arg(dir).creation_flags(crate::fs_safe::CREATE_NO_WINDOW).status();
    match hidden {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("Could not hide {}: attrib exited with {status}", dir.display()),
        Err(e) => eprintln!("Could not hide {}: {e}", dir.display()),
    }
}

// A dot-folder is already hidden on Unix.
#[cfg(not(windows))]
pub(crate) fn hide(_dir: &Path) {}
