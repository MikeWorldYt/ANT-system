// SQLite store of the tree and its search fields under <root>/.ant; the only copy of the tree.
mod lists;
mod meta;
mod query;
mod tree;

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, TransactionBehavior};

use crate::domain::config::{DATA_DIR, DB_FILENAME};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::io_err;

pub(crate) use query::{Doc, Suggestion};

const SCHEMA_VERSION: i64 = 2;
pub(super) const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

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
    uid TEXT,
    flags TEXT,
    UNIQUE (parent_id, name)
);
CREATE INDEX IF NOT EXISTS nodes_name ON nodes (name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS nodes_uid ON nodes (uid);
CREATE TABLE IF NOT EXISTS node_fields (
    path TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (path, key)
) WITHOUT ROWID;
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
        let mut conn = Connection::open(&path).map_err(fail)?;
        conn.busy_timeout(BUSY_TIMEOUT).map_err(fail)?;
        // WAL lets the search connection read while a change is being written.
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(())).map_err(fail)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(fail)?;
        if version > SCHEMA_VERSION {
            return Err(FmError::NewerDatabase { path: path.display().to_string() });
        }
        match version {
            0 => {
                conn.execute_batch(SCHEMA).map_err(fail)?;
                conn.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(fail)?;
            }
            1 => {
                let copy = pre_migration_copy(&conn, &path)?;
                if let Err(e) = migrate_v1(&mut conn) {
                    drop(conn);
                    // A busy db is another process migrating it, so its file is never replaced under it.
                    if !matches!(e.sqlite_error_code(), Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)) {
                        put_back_copy(&copy, &path)?;
                    }
                    return Err(fail(e));
                }
            }
            _ => {}
        }
        drop_yaml_hash(&conn).map_err(fail)?;
        Ok(Store { conn, path })
    }
}

// The copy stays beside ant.db after the migration, as the only way back to the v1 tree.
fn pre_migration_copy(conn: &Connection, path: &Path) -> R<PathBuf> {
    let copy = path.with_file_name(format!("{DB_FILENAME}.pre-v{SCHEMA_VERSION}"));
    match std::fs::remove_file(&copy) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io_err("delete file", &copy, &e)),
        _ => {}
    }
    let target = copy.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{target}'")).map_err(|e| db_err("copy ant.db before its migration", path, &e))?;
    Ok(copy)
}

// The copy holds every committed page, so the journals beside ant.db go with the file they belong to; the copy itself stays.
fn put_back_copy(copy: &Path, path: &Path) -> R<()> {
    let tmp = path.with_file_name(format!(".{DB_FILENAME}.migrate.tmp"));
    std::fs::copy(copy, &tmp).map_err(|e| io_err("copy file", &tmp, &e))?;
    for journal in ["-wal", "-shm"] {
        let file = path.with_file_name(format!("{DB_FILENAME}{journal}"));
        match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(io_err("delete file", &file, &e)),
            _ => {}
        }
    }
    std::fs::rename(&tmp, path).map_err(|e| io_err("replace file", path, &e))
}

// v1 kept folder ids in a footprints table; they move onto their nodes so one row carries the whole folder.
fn migrate_v1(conn: &mut Connection) -> rusqlite::Result<()> {
    // Immediate takes the write lock before the check below, so a second process waits and finds the work done.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if !has_uid_column(&tx)? {
        tx.execute_batch(
            "ALTER TABLE nodes ADD COLUMN uid TEXT;
             ALTER TABLE nodes ADD COLUMN flags TEXT;
             CREATE INDEX IF NOT EXISTS nodes_uid ON nodes (uid);",
        )?;
        let footprints: Vec<(String, String)> = {
            let mut stmt = tx.prepare("SELECT id, path FROM footprints")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        tree::adopt_uids(&tx, &footprints)?;
        tx.execute_batch("DROP TABLE footprints;")?;
    }
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()
}

// Builds that mirrored Tree.yaml kept its hash here; nothing reads it now, and the write happens only while the row exists.
fn drop_yaml_hash(conn: &Connection) -> rusqlite::Result<()> {
    let stale: bool = conn.query_row("SELECT EXISTS (SELECT 1 FROM meta WHERE key = 'yaml_hash')", [], |r| r.get(0))?;
    if stale {
        conn.execute("DELETE FROM meta WHERE key = 'yaml_hash'", [])?;
    }
    Ok(())
}

// Earlier builds wrote user_version after this commit, so a crash between them left a migrated db on version 1.
fn has_uid_column(conn: &Connection) -> rusqlite::Result<bool> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM pragma_table_info('nodes') WHERE name = 'uid')", [], |r| r.get(0))
}

fn db_err(op: &str, path: &Path, e: &rusqlite::Error) -> FmError {
    FmError::Io { op: op.to_string(), path: path.display().to_string(), message: e.to_string(), os_code: None }
}

#[cfg(windows)]
pub(crate) use crate::platform::windows::hidden_attr::hide;

// A dot-folder is already hidden on Unix.
#[cfg(not(windows))]
pub(crate) fn hide(_dir: &Path) {}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::TempDir;

    const V1_SHAPE: &str = "DROP INDEX nodes_uid;
        ALTER TABLE nodes DROP COLUMN uid;
        ALTER TABLE nodes DROP COLUMN flags;
        INSERT INTO nodes (id, parent_id, name, depth) VALUES (1, 0, 'A', 0);";
    const FOOTPRINTS: &str = "CREATE TABLE footprints (id TEXT PRIMARY KEY, path TEXT NOT NULL);
        INSERT INTO footprints VALUES ('kept-uid', 'A');";

    fn v1_db(dir: &TempDir, batches: &[&str]) {
        dir.mkdir(DATA_DIR);
        let conn = Connection::open(db_path(dir.path())).unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        for batch in batches {
            conn.execute_batch(batch).unwrap();
        }
        conn.pragma_update(None, "user_version", 1).unwrap();
    }

    fn version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap()
    }

    fn uid_of_a(conn: &Connection) -> String {
        conn.query_row("SELECT uid FROM nodes WHERE name = 'A'", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn the_old_tree_yaml_hash_is_dropped_on_open() {
        let dir = TempDir::new("db-yaml-hash");
        drop(Store::open_or_create(dir.path()).unwrap());
        Connection::open(db_path(dir.path())).unwrap().execute("INSERT INTO meta VALUES ('yaml_hash', '88cd30f2a1046524')", []).unwrap();
        let store = Store::open_existing(dir.path()).unwrap().unwrap();
        let left: i64 = store.conn.query_row("SELECT COUNT(*) FROM meta WHERE key = 'yaml_hash'", [], |r| r.get(0)).unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn a_v1_db_moves_its_ids_onto_the_nodes_and_ends_on_the_current_version() {
        let dir = TempDir::new("db-migrate");
        v1_db(&dir, &[V1_SHAPE, FOOTPRINTS]);
        let store = Store::open_existing(dir.path()).unwrap().unwrap();
        assert_eq!(version(&store.conn), SCHEMA_VERSION);
        assert_eq!(uid_of_a(&store.conn), "kept-uid");
        assert!(store.conn.prepare("SELECT id FROM footprints").is_err());
    }

    #[test]
    fn a_db_migrated_before_its_version_was_written_opens_on_the_current_version() {
        let dir = TempDir::new("db-stranded");
        v1_db(&dir, &["INSERT INTO nodes (id, parent_id, name, depth, uid) VALUES (1, 0, 'A', 0, 'kept-uid');"]);
        let store = Store::open_existing(dir.path()).unwrap().unwrap();
        assert_eq!(version(&store.conn), SCHEMA_VERSION);
        assert_eq!(uid_of_a(&store.conn), "kept-uid");
    }

    #[test]
    fn a_failed_migration_leaves_the_db_on_its_version() {
        let dir = TempDir::new("db-failed-migration");
        v1_db(&dir, &[V1_SHAPE]);
        assert!(Store::open_existing(dir.path()).is_err());
        let conn = Connection::open(db_path(dir.path())).unwrap();
        assert_eq!(version(&conn), 1);
        assert!(conn.prepare("SELECT uid FROM nodes").is_err());
        assert!(pre_v2(&dir).is_file());
    }

    fn pre_v2(dir: &TempDir) -> PathBuf {
        db_path(dir.path()).with_file_name(format!("{DB_FILENAME}.pre-v2"))
    }

    #[test]
    fn a_v1_db_is_copied_before_its_migration() {
        let dir = TempDir::new("db-pre-migration");
        v1_db(&dir, &[V1_SHAPE, FOOTPRINTS]);
        drop(Store::open_existing(dir.path()).unwrap().unwrap());
        let copy = Connection::open(pre_v2(&dir)).unwrap();
        assert_eq!(version(&copy), 1);
        let ids: String = copy.query_row("SELECT id FROM footprints", [], |r| r.get(0)).unwrap();
        assert_eq!(ids, "kept-uid");
    }

    #[test]
    fn the_copy_goes_back_over_a_db_left_changed() {
        let dir = TempDir::new("db-put-back");
        v1_db(&dir, &[V1_SHAPE, FOOTPRINTS]);
        let path = db_path(dir.path());
        let copy = pre_migration_copy(&Connection::open(&path).unwrap(), &path).unwrap();
        Connection::open(&path).unwrap().execute_batch("DROP TABLE footprints;").unwrap();
        fs::write(path.with_file_name(format!("{DB_FILENAME}-wal")), "stale pages").unwrap();
        put_back_copy(&copy, &path).unwrap();
        let conn = Connection::open(&path).unwrap();
        assert_eq!(version(&conn), 1);
        assert!(conn.prepare("SELECT id FROM footprints").is_ok());
        assert!(copy.is_file());
    }

    #[test]
    fn a_newer_db_gives_its_own_error_kind() {
        let dir = TempDir::new("db-newer");
        drop(Store::open_or_create(dir.path()).unwrap());
        Connection::open(db_path(dir.path())).unwrap().pragma_update(None, "user_version", 99).unwrap();
        let err = Store::open_existing(dir.path()).err().unwrap();
        assert!(matches!(&err, FmError::NewerDatabase { path } if *path == db_path(dir.path()).display().to_string()), "{err:?}");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "newerDatabase");
        assert_eq!(json["path"], db_path(dir.path()).display().to_string());
    }
}
