// Tells a missing ant.db from an unreadable one and moves an unreadable one aside so a fresh one can take its place.
use std::fs;
use std::path::Path;

use super::db::{db_path, Store};
use crate::config::DB_FILENAME;
use crate::debug_log::valid_stamp;
use crate::error::{FmError, R};
use crate::fs_safe::io_err;

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

pub fn prepare_scan(root: &Path, stamp: &str) -> R<()> {
    check_stamp(stamp)?;
    if db_state(root) == DbState::Unreadable {
        set_aside(root, stamp)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DATA_DIR;
    use crate::test_support::TempDir;

    const STAMP: &str = "26-10-24-09-05";

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
