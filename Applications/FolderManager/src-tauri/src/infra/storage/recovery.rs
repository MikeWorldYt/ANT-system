// Tells a missing ant.db from an unreadable one and moves a damaged one aside so a fresh one can take its place.
use std::fs;
use std::path::Path;

use rusqlite::{Connection, ErrorCode};

use super::db::{db_path, Store, BUSY_TIMEOUT};
use crate::domain::config::DB_FILENAME;
use crate::services::debug_log::valid_stamp;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::io_err;

#[derive(Debug, PartialEq)]
pub enum DbState {
    Healthy,
    Missing,
    Unreadable,
}

pub fn db_state(root: &Path) -> DbState {
    match Store::open_existing(root) {
        Ok(Some(_)) => DbState::Healthy,
        Ok(None) => DbState::Missing,
        Err(_) => DbState::Unreadable,
    }
}

fn check_stamp(stamp: &str) -> R<()> {
    if valid_stamp(stamp) {
        return Ok(());
    }
    Err(FmError::Internal { message: format!("Invalid stamp {stamp}") })
}

pub fn set_aside(root: &Path, stamp: &str) -> R<()> {
    check_stamp(stamp)?;
    let db = db_path(root);
    for part in ["", "-wal", "-shm"] {
        let from = db.with_file_name(format!("{DB_FILENAME}{part}"));
        if !from.is_file() {
            continue;
        }
        let to = db.with_file_name(format!("{DB_FILENAME}.corrupt-{stamp}{part}"));
        fs::rename(&from, &to).map_err(|e| io_err("rename file", &from, &e))?;
    }
    Ok(())
}

fn passes_quick_check(path: &Path) -> rusqlite::Result<bool> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    let first: String = conn.query_row("PRAGMA quick_check(1)", [], |r| r.get(0))?;
    Ok(first == "ok")
}

fn is_damage(e: &rusqlite::Error) -> bool {
    matches!(e.sqlite_error_code(), Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt))
}

fn kept(cause: impl std::fmt::Display) -> FmError {
    FmError::Internal { message: format!("ant.db was left in place: {cause}") }
}

// ant.db is the only copy of the tree, so only proof of damage moves it; a lock or a newer version is reported.
pub fn prepare_scan(root: &Path, stamp: &str) -> R<()> {
    check_stamp(stamp)?;
    let path = db_path(root);
    if !path.is_file() {
        return Ok(());
    }
    match passes_quick_check(&path) {
        Ok(true) => Store::open_existing(root).map(|_| ()).map_err(kept),
        Ok(false) => set_aside(root, stamp),
        Err(e) if is_damage(&e) => set_aside(root, stamp),
        Err(e) => Err(kept(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::DATA_DIR;
    use crate::test_support::{names, TempDir};

    const STAMP: &str = "26-10-24-09-05";
    const PAGE: usize = 4096;

    fn set_aside_files(dir: &TempDir) -> Vec<String> {
        names(&dir.path().join(DATA_DIR)).into_iter().filter(|name| name.contains(".corrupt-")).collect()
    }

    fn internal_message(err: FmError) -> String {
        match err {
            FmError::Internal { message } => message,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_db_from_a_newer_version_stays_in_place_and_the_scan_says_why() {
        let dir = TempDir::new("prepare-newer");
        Store::open_or_create(dir.path()).unwrap();
        Connection::open(db_path(dir.path())).unwrap().pragma_update(None, "user_version", 99).unwrap();
        let message = internal_message(prepare_scan(dir.path(), STAMP).unwrap_err());
        assert!(message.contains("newer version"), "{message}");
        assert!(db_path(dir.path()).is_file());
        assert_eq!(set_aside_files(&dir), Vec::<String>::new());
        let version: i64 = Connection::open(db_path(dir.path())).unwrap().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 99);
    }

    // Windows refuses to rename a file SQLite holds open, so the kind of error shows whether a set-aside was tried.
    #[test]
    fn a_locked_db_stays_in_place_and_the_scan_says_why() {
        let dir = TempDir::new("prepare-locked");
        Store::open_or_create(dir.path()).unwrap();
        let holder = Connection::open(db_path(dir.path())).unwrap();
        holder.execute_batch("PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;").unwrap();
        let message = internal_message(prepare_scan(dir.path(), STAMP).unwrap_err());
        assert!(message.contains("locked"), "{message}");
        holder.execute_batch("COMMIT;").unwrap();
        drop(holder);
        assert!(db_path(dir.path()).is_file());
        assert_eq!(set_aside_files(&dir), Vec::<String>::new());
        assert_eq!(db_state(dir.path()), DbState::Healthy);
    }

    #[test]
    fn a_db_that_opens_but_fails_the_check_is_moved_aside() {
        let dir = TempDir::new("prepare-damaged");
        Store::open_or_create(dir.path()).unwrap();
        let mut bytes = fs::read(db_path(dir.path())).unwrap();
        assert!(bytes.len() > PAGE, "{}", bytes.len());
        bytes[PAGE..].fill(0xA5);
        fs::write(db_path(dir.path()), bytes).unwrap();
        prepare_scan(dir.path(), STAMP).unwrap();
        assert!(!db_path(dir.path()).exists());
        assert_eq!(set_aside_files(&dir), vec![format!("ant.db.corrupt-{STAMP}")]);
    }

    #[test]
    fn a_garbage_db_is_moved_aside_before_a_scan() {
        let dir = TempDir::new("prepare-scan");
        assert_eq!(db_state(dir.path()), DbState::Missing);
        prepare_scan(dir.path(), STAMP).unwrap();
        assert!(!dir.path().join(DATA_DIR).exists());
        dir.mkdir(DATA_DIR);
        fs::write(db_path(dir.path()), "not a database ".repeat(512)).unwrap();
        assert_eq!(db_state(dir.path()), DbState::Unreadable);
        assert!(prepare_scan(dir.path(), "../bad").is_err());
        prepare_scan(dir.path(), STAMP).unwrap();
        let data = dir.path().join(DATA_DIR);
        assert!(!db_path(dir.path()).exists());
        assert!(data.join(format!("ant.db.corrupt-{STAMP}")).is_file());
        Store::open_or_create(dir.path()).unwrap();
        prepare_scan(dir.path(), STAMP).unwrap();
        assert_eq!(db_state(dir.path()), DbState::Healthy);
    }
}
