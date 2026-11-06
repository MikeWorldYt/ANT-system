use std::fs;

use super::*;
use crate::domain::config::{DATA_DIR, DB_FILENAME};
use crate::infra::storage::db::db_path;
use crate::infra::fs_safe::LOCK_FILE;
use crate::test_support::{academic, names, remove_db, TempDir};

const STAMP: &str = "26-10-24-09-05";

fn staged_copy(tag: &str) -> (TempDir, TempDir) {
    let dir = academic(tag);
    dir.mkdir("100.ACADEMIC/01.Math/.ant/debug");
    dir.mkdir("100.ACADEMIC/01.Math/98.Math/.ant");
    dir.mkdir("999.Settings/03.Backup/old/.ant");
    fs::write(dir.path().join("100.ACADEMIC/01.Math/.ant/info.json"), "math").unwrap();
    fs::write(dir.path().join("100.ACADEMIC/01.Math/.ant/debug/log.log"), "x").unwrap();
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

fn wal(root: &Path) -> PathBuf {
    db_path(root).with_file_name(format!("{DB_FILENAME}-wal"))
}

#[test]
fn a_failing_copy_leaves_the_wal_and_the_db_in_place() {
    let dir = academic("restore-copy");
    let root = dir.path();
    fs::write(wal(root), "committed pages").unwrap();
    let before = fs::read(db_path(root)).unwrap();
    let empty = TempDir::new("restore-copy-stage");
    assert!(stage::apply(root, empty.path()).is_err());
    assert_eq!(fs::read_to_string(wal(root)).unwrap(), "committed pages");
    assert_eq!(fs::read(db_path(root)).unwrap(), before);
}

#[test]
fn a_damaged_db_is_refused_before_the_wal_goes() {
    let dir = academic("restore-damaged");
    let root = dir.path();
    fs::write(wal(root), "committed pages").unwrap();
    let before = fs::read(db_path(root)).unwrap();
    let bad = TempDir::new("restore-damaged-stage");
    bad.mkdir(DATA_DIR);
    fs::write(bad.path().join(DATA_DIR).join(DB_FILENAME), "not a database ".repeat(512)).unwrap();
    assert!(stage::apply(root, bad.path()).is_err());
    assert_eq!(fs::read_to_string(wal(root)).unwrap(), "committed pages");
    assert_eq!(fs::read(db_path(root)).unwrap(), before);
    assert!(!names(&root.join(DATA_DIR)).iter().any(|n| n.contains("restore")));
}

#[test]
fn a_blocked_rename_puts_the_wal_back() {
    let (dir, out) = staged_copy("restore-blocked");
    let root = dir.path();
    remove_db(root);
    fs::create_dir(db_path(root)).unwrap();
    fs::write(wal(root), "committed pages").unwrap();
    assert!(stage::apply(root, out.path()).is_err());
    assert_eq!(fs::read_to_string(wal(root)).unwrap(), "committed pages");
    assert!(!names(&root.join(DATA_DIR)).iter().any(|n| n.contains("restore")));
}

#[test]
fn a_finished_restore_leaves_no_old_wal() {
    let (dir, out) = staged_copy("restore-wal");
    let root = dir.path();
    fs::write(wal(root), "stale pages").unwrap();
    stage::apply(root, out.path()).unwrap();
    assert!(!wal(root).exists());
    assert!(!names(&root.join(DATA_DIR)).iter().any(|n| n.contains("restore")));
    assert!(settings(root).is_ok());
}

#[test]
fn a_restore_that_fails_after_the_safety_backup_names_it() {
    let dir = academic("restore-safety");
    let root = dir.path();
    let bad = TempDir::new("restore-safety-stage");
    bad.mkdir(DATA_DIR);
    fs::write(bad.path().join(DATA_DIR).join(DB_FILENAME), "not a database ".repeat(512)).unwrap();
    dir.mkdir("backup");
    let name = format!("{STAMP}{SUFFIX}.zip");
    archive::compress(bad.path(), &root.join("backup").join(&name)).unwrap();
    let err = restore_from(root, &root.join("backup"), &name, "26-10-24-09-06").unwrap_err().to_string();
    assert!(err.contains("26-10-24-09-06_ANT-fm-backup.zip"), "{err}");
    assert!(settings(root).is_ok());
}

const SHARE: &str = r"\\ant-test.invalid\share";

fn folder_setting(folder: &str) -> HashMap<String, String> {
    HashMap::from([("backupFolder".to_string(), folder.to_string())])
}

#[cfg(windows)]
#[test]
fn a_share_from_a_foreign_db_is_refused_before_anything_touches_it() {
    let dir = academic("backup-foreign");
    let root = dir.path();
    let refused = chosen_dir(root, &folder_setting(SHARE), |_| Ok(false)).unwrap_err().to_string();
    assert!(refused.contains("not set on this computer"), "{refused}");
    let mut store = Store::open_or_create(root).unwrap();
    store.set_setting("backupFolder", SHARE).unwrap();
    store.set_setting("backupAuto", "true").unwrap();
    drop(store);
    for err in [create(root, STAMP).unwrap_err(), if_due(root, STAMP).unwrap_err()] {
        assert!(err.to_string().contains("not set on this computer"), "{err}");
    }
    assert!(list(root).is_err());
    assert!(!settings(root).unwrap().contains_key("backupLast"));
}

#[cfg(windows)]
#[test]
fn a_share_listed_on_this_machine_and_a_local_folder_keep_working() {
    let dir = TempDir::new("backup-confirmed");
    let file = dir.path().join("data/list.txt");
    assert!(!confirmed::listed(&file, SHARE).unwrap());
    confirmed::add(&file, SHARE).unwrap();
    confirmed::add(&file, SHARE).unwrap();
    confirmed::add(&file, r"\\other.invalid\share").unwrap();
    assert!(confirmed::listed(&file, SHARE).unwrap());
    assert!(!confirmed::listed(&file, r"\\ant-test.invalid\share\sub").unwrap());
    assert_eq!(fs::read_to_string(&file).unwrap().lines().count(), 2);
    let is_confirmed = |folder: &str| confirmed::listed(&file, folder);
    assert_eq!(chosen_dir(dir.path(), &folder_setting(SHARE), is_confirmed).unwrap(), PathBuf::from(SHARE));
    let unasked = |_: &str| -> R<bool> { panic!("a local folder needs no confirmation") };
    assert_eq!(chosen_dir(dir.path(), &folder_setting(r"D:\Backups"), unasked).unwrap(), PathBuf::from(r"D:\Backups"));
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
fn new_backups_use_the_root_backup_folder_and_the_staging_skips_it_with_the_lock() {
    let dir = academic("backup-root-dir");
    let root = dir.path();
    assert_eq!(default_dir(root), root.join("backup"));
    dir.mkdir("backup/old/.ant");
    fs::write(root.join("backup/old/.ant/info.json"), "old").unwrap();
    fs::write(root.join(DATA_DIR).join(LOCK_FILE), "1").unwrap();
    fs::write(root.join(DATA_DIR).join("lnk-favorites.json"), "[]").unwrap();
    let out = TempDir::new("backup-root-dir-stage");
    stage::stage(root, out.path(), &fs::canonicalize(root.join("backup")).unwrap()).unwrap();
    assert!(out.path().join(DATA_DIR).join("lnk-favorites.json").is_file());
    assert!(!out.path().join(DATA_DIR).join(LOCK_FILE).exists() && !out.path().join("backup").exists());
    fs::write(root.join("backup/26-10-26-09-05_ANT-fm-backup.zip"), "").unwrap();
    dir.mkdir("999.Settings/03.Backup");
    fs::write(root.join("999.Settings/03.Backup/26-10-27-09-05_ANT-fm-backup.zip"), "").unwrap();
    let listed: Vec<String> = list(root).unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(listed, ["26-10-27-09-05_ANT-fm-backup.zip", "26-10-26-09-05_ANT-fm-backup.zip"]);
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
fn a_zip_cut_short_never_takes_its_final_name_and_a_stale_part_does_not_block_the_next() {
    let dir = TempDir::new("backup-part");
    let zip = dir.path().join(format!("{STAMP}{SUFFIX}.zip"));
    let cut = write_zip(&zip, |part| {
        fs::write(part, "half").unwrap();
        Err(fail("disk full".to_string()))
    });
    assert!(cut.is_err());
    assert_eq!(names(dir.path()), Vec::<String>::new());
    let stale = dir.path().join(format!("{STAMP}{SUFFIX}.zip.part"));
    fs::write(&stale, "killed").unwrap();
    assert!(entries(dir.path()).unwrap().is_empty());
    write_zip(&zip, |part| {
        assert_eq!(part, stale);
        assert!(!part.exists());
        fs::write(part, "whole").map_err(|e| fail(e.to_string()))
    })
    .unwrap();
    assert_eq!(fs::read_to_string(&zip).unwrap(), "whole");
    assert_eq!(names(dir.path()), [format!("{STAMP}{SUFFIX}.zip")]);
}

#[test]
fn a_truncated_newest_zip_makes_restore_latest_use_the_previous_one() {
    let dir = academic("backup-truncated");
    let root = dir.path();
    Store::open_or_create(root).unwrap().set_setting("backupFrequency", "week").unwrap();
    let good = create(root, STAMP).unwrap();
    let bytes = fs::read(&good).unwrap();
    fs::write(good.with_file_name("26-10-25-09-05_ANT-fm-backup.zip"), &bytes[..bytes.len() / 2]).unwrap();
    remove_db(root);
    restore_latest(root, "26-10-26-09-05").unwrap();
    assert_eq!(settings(root).unwrap().get("backupFrequency").map(String::as_str), Some("week"));
    assert_eq!(names(&root.join("backup")).len(), 2);
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
