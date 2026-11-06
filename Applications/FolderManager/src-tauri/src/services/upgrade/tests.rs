use std::fs;

use super::*;
use crate::test_support::*;

const STAMP: &str = "26-01-01-10-00";

const TREE: &str = "100.A:
  type: NODE-L1
  01.Math:
    type: NODE-L2
    98.Math: {type: REV-L2}
    90.Resources: {type: RES}
    01.Alg:
      type: NODE-L3
    97.Templates: {type: TPL}
";

fn old_storage(tag: &str) -> TempDir {
    storage(tag, &["100.A/01.Math/98.Math/note", "100.A/01.Math/90.Resources", "100.A/01.Math/97.Templates", "100.A/01.Math/01.Alg"], TREE)
}

fn found(dir: &TempDir) -> Vec<String> {
    check_support_upgrade(dir.path()).unwrap().folders.iter().map(|f| f.rel_parts.join("/")).collect()
}

#[test]
fn the_check_lists_numeric_support_folders_inside_the_depth_limit_and_skips_workboards() {
    let dir = old_storage("upgrade-list");
    dir.mkdir("100.A/97.Category");
    dir.mkdir("100.A/01.Math/01.Alg/98.Alg");
    dir.mkdir("100.A/01.Math/REV.Math");
    dir.mkdir("100.A/PJ.001/98.Board");
    let check = check_support_upgrade(dir.path()).unwrap();
    assert!(check.needed && check.root);
    let mut rows: Vec<(String, bool)> = check.folders.iter().map(|f| (f.rel_parts.join("/"), f.notice)).collect();
    rows.sort();
    let expected = [
        ("100.A/01.Math/90.Resources", true),
        ("100.A/01.Math/97.Templates", false),
        ("100.A/01.Math/98.Math", false),
        ("100.A/97.Category", false),
    ];
    assert_eq!(rows, expected.map(|(a, b)| (a.to_string(), b)));
}

#[test]
fn a_support_folder_gets_its_word_and_moves_its_tree_entry() {
    let dir = old_storage("upgrade-yes");
    let m = apply_support_upgrade(dir.path(), &rel(&["100.A", "01.Math", "98.Math"])).unwrap();
    assert_eq!(m.select, Some(rel(&["100.A", "01.Math", "REV.Math"])));
    assert_eq!(names(&dir.path().join("100.A/01.Math")), ["01.Alg", "90.Resources", "97.Templates", "REV.Math"]);
    assert!(dir.path().join("100.A/01.Math/REV.Math/note").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.A", "01.Math", "REV.Math"]), "REV-L2");
    assert_eq!(yaml_type(dir.path(), &["100.A", "01.Math", "98.Math"]), "");
    apply_support_upgrade(dir.path(), &rel(&["100.A", "01.Math", "97.Templates"])).unwrap();
    assert_eq!(yaml_type(dir.path(), &["100.A", "01.Math", "TPL.Templates"]), "TPL");
}

#[test]
fn a_taken_word_name_gets_a_numbered_one() {
    let dir = old_storage("upgrade-clash");
    dir.mkdir("100.A/01.Math/REV.Math");
    apply_support_upgrade(dir.path(), &rel(&["100.A", "01.Math", "98.Math"])).unwrap();
    assert!(names(&dir.path().join("100.A/01.Math")).contains(&"REV.Math-2".to_string()));
}

#[test]
fn a_review_named_unlike_its_parent_becomes_a_regular_node() {
    let dir = old_storage("upgrade-no");
    dir.mkdir("100.A/01.Math/02.Other");
    dir.mkdir("100.A/01.Math/98.Gameplays");
    apply_support_upgrade(dir.path(), &rel(&["100.A", "01.Math", "98.Gameplays"])).unwrap();
    assert!(names(&dir.path().join("100.A/01.Math")).contains(&"03.Gameplays".to_string()));
    assert_eq!(yaml_type(dir.path(), &["100.A", "01.Math", "03.Gameplays"]), "NODE-L3");
    assert_eq!(found(&dir).len(), 3);
}

#[test]
fn the_name_decides_what_is_a_support_folder() {
    assert!(scan::is_support("04.Marketing", "98.marketing"));
    assert!(!scan::is_support("04.Video-edition", "98.Gameplays"));
    assert!(scan::is_support("04.Video", "97.Video-templates"));
    assert!(!scan::is_support("04.Video", "97.Clips"));
    assert!(scan::is_support("04.Video", "90.Resources"));
    assert!(!scan::is_support("04.Video", "90.Clips"));
}

#[test]
fn a_folder_that_is_not_a_numeric_support_one_is_refused() {
    let dir = old_storage("upgrade-refuse");
    let err = apply_support_upgrade(dir.path(), &rel(&["100.A", "01.Math", "01.Alg"])).unwrap_err();
    assert!(matches!(err, FmError::InvalidPath { .. }), "{err:?}");
}

#[test]
fn finish_moves_the_root_data_writes_the_marker_and_ends_the_check() {
    let dir = storage("upgrade-finish", &["100.A", "998.Review/.ant", "999.Settings/03.Backup"], "100.A: {type: NODE-L1}\n998.Review: {type: CFG}\n999.Settings: {type: CFG}\n");
    fs::write(dir.path().join("999.Settings/00.Core/.skip_version"), "1.0.0").unwrap();
    fs::write(dir.path().join("999.Settings/00.Core/FM_Updater.bat"), "").unwrap();
    fs::write(dir.path().join("999.Settings/00.Core/.FolderManager.lock"), "1").unwrap();
    fs::write(dir.path().join("999.Settings/03.Backup/old.zip"), "z").unwrap();
    fs::write(dir.path().join("998.Review/100.A.lnk"), "").unwrap();
    fs::write(dir.path().join("998.Review/.ant/info.json"), "{}").unwrap();
    assert!(check_support_upgrade(dir.path()).unwrap().needed);
    let warnings = finish_support_upgrade(dir.path(), STAMP).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let data = dir.path().join(".ant");
    assert!(data.join(".skip_version").is_file() && data.join("FM_Updater.bat").is_file());
    assert!(dir.path().join("backup/old.zip").is_file());
    assert!(!dir.path().join("998.Review").exists() && !dir.path().join("999.Settings").exists());
    assert!(data.join(MARKER).is_file());
    assert!(get_root_keys(dir.path()).iter().all(|k| k != "998.Review" && k != "999.Settings"));
    assert!(!check_support_upgrade(dir.path()).unwrap().needed);
}

fn get_root_keys(root: &std::path::Path) -> Vec<String> {
    yaml(root).keys().filter_map(|k| k.as_str().map(str::to_string)).collect()
}

#[test]
fn finish_keeps_folders_that_hold_user_files_and_says_so() {
    let dir = storage("upgrade-keep", &["998.Review"], "100.A: {type: NODE-L1}\n");
    fs::write(dir.path().join("999.Settings/notes.txt"), "mine").unwrap();
    fs::write(dir.path().join("998.Review/mine.docx"), "mine").unwrap();
    let warnings = finish_support_upgrade(dir.path(), STAMP).unwrap();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(dir.path().join("999.Settings/notes.txt").is_file() && dir.path().join("998.Review/mine.docx").is_file());
    assert!(is_done(dir.path()));
}

#[test]
fn finish_waits_for_every_folder_and_leaves_no_marker_before_that() {
    let dir = old_storage("upgrade-early");
    let err = finish_support_upgrade(dir.path(), STAMP).unwrap_err();
    assert!(matches!(err, FmError::Internal { .. }), "{err:?}");
    assert!(!is_done(dir.path()));
    assert!(dir.path().join("999.Settings").is_dir());
}

const EXTRAS: &str = "100.A:
  type: NODE-L1
  01.Math:
    type: NODE-L2
    98.Math: {type: REV-L3}
    98.Old: {type: REV-L3}
    98.Plain: {type: NODE-L3}
    REV.2024: {type: REV-L3}
";

fn extras_storage(tag: &str) -> TempDir {
    let dir = storage(tag, &["100.A/01.Math/98.Math", "100.A/01.Math/98.Old/sub", "100.A/01.Math/98.Plain", "100.A/01.Math/REV.2024"], EXTRAS);
    fs::write(dir.path().join("100.A/01.Math/98.Math/notes.txt"), "kept").unwrap();
    fs::write(dir.path().join("100.A/01.Math/98.Old/notes.txt"), "old").unwrap();
    dir
}

const OLD: [&str; 3] = ["100.A", "01.Math", "98.Old"];

#[test]
fn only_a_legacy_review_tracked_as_one_is_offered() {
    let dir = extras_storage("extras-list");
    write_marker(dir.path()).unwrap();
    let extras = extra_reviews(dir.path()).unwrap();
    let rows: Vec<(Vec<String>, &str)> = extras.iter().map(|e| (e.rel_parts.clone(), e.keep.as_str())).collect();
    assert_eq!(rows, [(rel(&OLD), "98.Math")]);
    assert!(merge_extra_review(dir.path(), &rel(&["100.A", "01.Math", "REV.2024"])).is_err());
    assert!(merge_extra_review(dir.path(), &rel(&["100.A", "01.Math", "98.Plain"])).is_err());
    assert!(dir.path().join("100.A/01.Math/REV.2024").is_dir() && dir.path().join("100.A/01.Math/98.Plain").is_dir());
}

#[test]
fn nothing_is_offered_or_merged_before_the_upgrade_is_done() {
    let dir = extras_storage("extras-marker");
    assert!(extra_reviews(dir.path()).unwrap().is_empty());
    assert!(merge_extra_review(dir.path(), &rel(&OLD)).is_err());
    assert!(dir.path().join("100.A/01.Math/98.Old/notes.txt").is_file());
}

#[test]
fn a_merge_numbers_collisions_and_removes_only_the_emptied_folder() {
    let dir = extras_storage("extras-merge");
    write_marker(dir.path()).unwrap();
    let m = merge_extra_review(dir.path(), &rel(&OLD)).unwrap();
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    let keep = dir.path().join("100.A/01.Math/98.Math");
    assert_eq!(fs::read_to_string(keep.join("notes.txt")).unwrap(), "kept");
    assert_eq!(fs::read_to_string(keep.join("notes (2).txt")).unwrap(), "old");
    assert!(keep.join("sub").is_dir());
    assert!(!dir.path().join("100.A/01.Math/98.Old").exists());
    assert_eq!(yaml_type(dir.path(), &OLD), "");
    assert!(extra_reviews(dir.path()).unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn a_failed_merge_puts_every_file_back() {
    let dir = extras_storage("extras-rollback");
    write_marker(dir.path()).unwrap();
    let _held = break_tree_save(dir.path());
    assert!(merge_extra_review(dir.path(), &rel(&OLD)).is_err());
    assert_eq!(fs::read_to_string(dir.path().join("100.A/01.Math/98.Old/notes.txt")).unwrap(), "old");
    assert!(dir.path().join("100.A/01.Math/98.Old/sub").is_dir());
    assert!(!dir.path().join("100.A/01.Math/98.Math/notes (2).txt").exists());
}

#[test]
fn a_storage_with_nothing_old_gets_the_marker_at_once() {
    let dir = TempDir::new("upgrade-clean");
    dir.mkdir("100.A/01.Math/REV.Math");
    let check = check_support_upgrade(dir.path()).unwrap();
    assert!(!check.needed && check.folders.is_empty());
    assert!(is_done(dir.path()));
}
