use super::*;
use crate::test_support::TempDir;

const STAMP: &str = "26-10-24-09-05";

fn storage_in(dir: &TempDir, name: &str) -> PathBuf {
    let root = dir.path().join(name);
    fs::create_dir_all(root.join("999.Settings")).unwrap();
    root
}

#[test]
fn the_log_holds_only_the_two_names() {
    let text = log_text("Old", "New");
    assert_eq!(text, "# Storage renamed\n\n| | |\n| --- | --- |\n| Old name | Old |\n| New name | New |\n");
    assert_eq!(parse_log(&text), Some(("Old".to_string(), "New".to_string())));
}

#[test]
fn garbled_or_missing_rows_do_not_parse() {
    assert_eq!(parse_log("| New name | New |\n"), None);
    assert_eq!(parse_log("| Old name | Old |\n"), None);
    assert_eq!(parse_log("| Old name | a/b |\n| New name | New |\n"), None);
    assert_eq!(parse_log("| Old name |  |\n| New name | New |\n"), None);
    assert_eq!(parse_log("| Old name | Old\n| New name | New |\n"), None);
    assert_eq!(parse_log("garbage"), None);
}

#[test]
fn the_newest_log_wins_and_a_clash_gets_a_suffix() {
    let dir = TempDir::new("rename-logs");
    let root = storage_in(&dir, "B");
    let first = write_log(&root, "26-10-24-09-05", "A", "B").unwrap();
    assert!(first.ends_with(format!("{STAMP}{SUFFIX}.md")));
    let second = write_log(&root, STAMP, "X", "B").unwrap();
    assert!(second.ends_with(format!("{STAMP}{SUFFIX}_2.md")));
    write_log(&root, "26-10-23-09-05", "Y", "B").unwrap();
    fs::write(logs_dir(&root).join("26-10-24-09-05_ANT-fm.md"), "").unwrap();
    assert_eq!(newest_log(&logs_dir(&root)), Some(second));
    assert_eq!(status(&root).map(|s| s.old), Some("X".to_string()));
}

#[test]
fn only_a_matching_name_with_a_free_old_name_is_revertible() {
    let dir = TempDir::new("rename-status");
    let root = storage_in(&dir, "B");
    assert_eq!(status(&root), None);
    write_log(&root, STAMP, "A", "B").unwrap();
    assert_eq!(status(&root), Some(RenameStatus { old: "A".into(), new: "B".into(), revertible: true }));
    fs::create_dir(dir.path().join("A")).unwrap();
    assert!(!status(&root).unwrap().revertible);
    write_log(&root, "26-10-25-09-05", "B", "C").unwrap();
    assert!(!status(&root).unwrap().revertible);
}

#[test]
fn bad_names_and_stamps_are_refused() {
    let dir = TempDir::new("rename-refuse");
    let root = storage_in(&dir, "A");
    fs::create_dir(dir.path().join("Taken")).unwrap();
    assert!(rename_root(&root, "A").is_err());
    assert!(rename_root(&root, "Taken").is_err());
    assert!(rename_root(&root, "x/y").is_err());
    assert!(rename_root(&root, "..").is_err());
    assert!(check_stamp("nope").is_err());
    assert!(root.is_dir());
}

#[test]
fn rename_and_revert_round_trip() {
    let dir = TempDir::new("rename-trip");
    let storage = crate::commands::Storage::resolve(storage_in(&dir, "A"));
    let renamed = crate::commands::rename_flow(&storage, STAMP, |_| Ok("B".to_string())).unwrap();
    assert_eq!(renamed, dir.path().join("B"));
    assert_eq!(storage.root().unwrap(), renamed);
    assert!(!dir.path().join("A").exists());
    let back = crate::commands::rename_flow(&storage, STAMP, crate::commands::revert_target).unwrap();
    assert_eq!(back, dir.path().join("A"));
    assert_eq!(status(&back), Some(RenameStatus { old: "B".into(), new: "A".into(), revertible: true }));
    assert!(crate::commands::rename_flow(&storage, "bad", |_| Ok("C".to_string())).is_err());
    let again = crate::commands::rename_flow(&storage, STAMP, crate::commands::revert_target).unwrap();
    assert_eq!(again, dir.path().join("B"));
}
