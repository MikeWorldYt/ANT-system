use std::fs;

use super::*;
use crate::config::{DATA_DIR, DB_FILENAME};
use crate::fs_safe::LOCK_FILE;
use crate::test_support::{academic, TempDir};

const STAMP: &str = "26-10-24-09-05";

fn staged_copy(tag: &str) -> (TempDir, TempDir) {
    let dir = academic(tag);
    dir.mkdir("100.ACADEMIC/01.Math/.ant/debug");
    dir.mkdir("100.ACADEMIC/01.Math/98.Math/.ant");
    dir.mkdir("999.Settings/03.Backup/old/.ant");
    fs::write(dir.path().join("100.ACADEMIC/01.Math/.ant/info.json"), "math").unwrap();
    fs::write(dir.path().join("100.ACADEMIC/01.Math/.ant/debug/log.md"), "x").unwrap();
    fs::write(dir.path().join("100.ACADEMIC/01.Math/98.Math/.ant/info.json"), "rev").unwrap();
    fs::write(dir.path().join("999.Settings/03.Backup/old/.ant/info.json"), "old").unwrap();
    fs::write(dir.path().join("999.Settings/00.Core/config.yaml"), "core").unwrap();
    fs::write(dir.path().join("999.Settings/00.Core").join(LOCK_FILE), "1").unwrap();
    let out = TempDir::new(&format!("{tag}-stage"));
    let skip = fs::canonicalize(dir.path().join("999.Settings/03.Backup")).unwrap();
    stage::stage(dir.path(), out.path(), &skip).unwrap();
    (dir, out)
}

#[test]
fn a_backup_holds_only_the_ant_data() {
    let (_dir, out) = staged_copy("backup-stage");
    let s = out.path();
    let conn = rusqlite::Connection::open(s.join(DATA_DIR).join(DB_FILENAME)).unwrap();
    let nodes: i64 = conn.query_row("SELECT count(*) FROM nodes", [], |r| r.get(0)).unwrap();
    assert!(nodes > 0);
    assert_eq!(fs::read_to_string(s.join("100.ACADEMIC/01.Math/.ant/info.json")).unwrap(), "math");
    assert!(s.join("100.ACADEMIC/01.Math/98.Math/.ant/info.json").is_file());
    assert!(s.join("999.Settings/00.Core/config.yaml").is_file());
    assert!(!s.join("100.ACADEMIC/01.Math/.ant/debug").exists());
    assert!(!s.join("999.Settings/03.Backup").exists());
    assert!(!s.join("999.Settings/00.Core").join(LOCK_FILE).exists());
    assert!(!s.join("100.ACADEMIC/01.Math/01.Algebra").exists());
}

#[test]
fn restoring_brings_the_data_back_and_skips_missing_folders() {
    let (dir, out) = staged_copy("backup-restore");
    let root = dir.path();
    Store::open_or_create(root).unwrap().set_setting("hideDone", "true").unwrap();
    fs::write(root.join("100.ACADEMIC/01.Math/.ant/info.json"), "changed").unwrap();
    fs::remove_dir_all(root.join("100.ACADEMIC/01.Math/98.Math")).unwrap();
    fs::write(root.join("999.Settings/00.Core").join(LOCK_FILE), "held").unwrap();
    stage::apply(root, out.path()).unwrap();
    assert!(!settings(root).unwrap().contains_key("hideDone"));
    assert_eq!(fs::read_to_string(root.join("100.ACADEMIC/01.Math/.ant/info.json")).unwrap(), "math");
    assert!(!root.join("100.ACADEMIC/01.Math/98.Math").exists());
    assert_eq!(fs::read_to_string(root.join("999.Settings/00.Core").join(LOCK_FILE)).unwrap(), "held");
    assert!(stage::has_db(out.path()));
    fs::remove_file(out.path().join(DATA_DIR).join(DB_FILENAME)).unwrap();
    assert!(!stage::has_db(out.path()));
}

#[test]
fn backup_names_follow_the_pattern() {
    assert!(valid_name("26-10-24-09-05_ANT-fm-backup.zip"));
    assert!(valid_name("26-10-24-09-05_ANT-fm-backup_12.zip"));
    assert!(!valid_name("26-10-24-09-05_ANT-fm-backup"));
    assert!(!valid_name("26-10-24-09-05_ANT-fm-backup_.zip"));
    assert!(!valid_name("26-10-24-09-05_ANT-fm-backup_2x.zip"));
    assert!(!valid_name("../26-10-24-09-05_ANT-fm-backup.zip"));
    assert!(!valid_name("26-10-24-09_ANT-fm-backup.zip"));
}

#[test]
fn a_clashing_name_gets_a_numeric_suffix() {
    let dir = TempDir::new("backup-name");
    let first = free_zip(dir.path(), STAMP);
    assert!(first.ends_with("26-10-24-09-05_ANT-fm-backup.zip"));
    fs::write(&first, "").unwrap();
    let second = free_zip(dir.path(), STAMP);
    assert!(second.ends_with("26-10-24-09-05_ANT-fm-backup_2.zip"));
    fs::write(&second, "").unwrap();
    assert!(free_zip(dir.path(), STAMP).ends_with("26-10-24-09-05_ANT-fm-backup_3.zip"));
}

#[test]
fn listing_shows_matching_zips_newest_first() {
    let dir = academic("backup-list");
    assert!(list(dir.path()).unwrap().is_empty());
    dir.mkdir("999.Settings/03.Backup/26-10-25-00-00_ANT-fm-backup.zip");
    let names = ["26-10-24-09-05_ANT-fm-backup.zip", "26-10-26-09-05_ANT-fm-backup.zip", "26-10-24-09-05_ANT-fm-backup_2.zip", "notes.zip"];
    for name in names {
        fs::write(dir.path().join("999.Settings/03.Backup").join(name), "").unwrap();
    }
    let listed: Vec<String> = list(dir.path()).unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(listed, ["26-10-26-09-05_ANT-fm-backup.zip", "26-10-24-09-05_ANT-fm-backup_2.zip", "26-10-24-09-05_ANT-fm-backup.zip"]);
}

#[test]
fn restore_rejects_names_outside_the_backup_folder() {
    let dir = academic("backup-reject");
    let backups = dir.path().join("999.Settings/03.Backup");
    fs::create_dir_all(&backups).unwrap();
    fs::write(dir.path().join("999.Settings/26-10-24-09-05_ANT-fm-backup.zip"), "").unwrap();
    assert!(backup_zip(&backups, "../26-10-24-09-05_ANT-fm-backup.zip").is_err());
    assert!(backup_zip(&backups, "26-10-24-09-05_ANT-fm-backup.zip").is_err());
    assert!(restore(dir.path(), "..\\x.zip", STAMP).is_err());
    assert!(restore(dir.path(), "26-10-24-09-05_ANT-fm-backup.zip", "bad").is_err());
    fs::write(backups.join("26-10-24-09-05_ANT-fm-backup.zip"), "").unwrap();
    assert!(backup_zip(&backups, "26-10-24-09-05_ANT-fm-backup.zip").is_ok());
}

#[test]
#[ignore = "starts PowerShell or zip"]
fn a_zip_backup_round_trips_and_leaves_a_safety_backup() {
    let dir = academic("backup-zip");
    let zip = create(dir.path(), STAMP).unwrap();
    assert!(zip.is_file());
    assert!(settings(dir.path()).unwrap().contains_key("backupLast"));
    Store::open_or_create(dir.path()).unwrap().set_setting("hideDone", "true").unwrap();
    let name = zip.file_name().unwrap().to_string_lossy().into_owned();
    restore(dir.path(), &name, "26-10-24-09-06").unwrap();
    assert!(!settings(dir.path()).unwrap().contains_key("hideDone"));
    assert_eq!(list(dir.path()).unwrap().len(), 2);
    assert!(!dir.path().join("999.Settings/03.Backup/.26-10-24-09-06_restore").exists());
}
