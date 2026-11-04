// Intake rules checked on a throwaway storage with one node of each kind.
use std::fs;
use std::path::Path;

use super::*;
use crate::test_support::{storage, yaml, TempDir};

const TREE: &str = "100.PERSONAL:
  type: NODE-L1
  03.Toki:
    type: NODE-L2
    98.Toki: {type: REV-L2}
    01.Sub: {type: NODE-L3, flags: data}
  98.PERSONAL: {type: REV-L2}
  04.Data:
    type: NODE-L2
    flags: data
  05.Bare:
    type: NODE-L2
  06.Ghost:
    type: NODE-L2
  PJ.Board:
    type: WKB
";
const DIRS: &[&str] = &[
    "100.PERSONAL/.ant",
    "100.PERSONAL/03.Toki/98.Toki",
    "100.PERSONAL/03.Toki/01.Sub",
    "100.PERSONAL/03.Toki/.ant",
    "100.PERSONAL/98.PERSONAL",
    "100.PERSONAL/04.Data",
    "100.PERSONAL/05.Bare/.ant",
    "100.PERSONAL/PJ.Board",
];

fn fixture(tag: &str) -> TempDir {
    storage(tag, DIRS, TREE)
}

fn put(dir: &TempDir, rel: &str, text: &str) {
    fs::write(dir.path().join(rel), text).unwrap();
}

fn has(dir: &TempDir, rel: &str) -> bool {
    dir.path().join(rel).is_file()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn a_file_in_a_flagless_node_moves_into_its_review_child() {
    let dir = fixture("intake-move");
    put(&dir, "100.PERSONAL/03.Toki/notes.txt", "n");
    put(&dir, "100.PERSONAL/03.Toki/draft.md", "d");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/03.Toki/98.Toki/notes.txt") && has(&dir, "100.PERSONAL/03.Toki/98.Toki/draft.md"));
    assert!(!has(&dir, "100.PERSONAL/03.Toki/notes.txt"));
    assert_eq!(report.moved.len(), 1, "{report:?}");
    let moved = &report.moved[0];
    assert_eq!((moved.node.as_str(), moved.name.as_str()), ("100.PERSONAL/03.Toki", "03.Toki"));
    assert_eq!((moved.count, moved.review.as_str()), (2, "98.Toki"));
    assert!(report.loose.is_empty() && report.warnings.is_empty(), "{report:?}");
}

#[test]
fn files_in_data_nodes_and_workboards_stay() {
    let dir = fixture("intake-keep");
    put(&dir, "100.PERSONAL/04.Data/table.csv", "t");
    put(&dir, "100.PERSONAL/03.Toki/01.Sub/raw.bin", "r");
    put(&dir, "100.PERSONAL/PJ.Board/plan.txt", "p");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/04.Data/table.csv"));
    assert!(has(&dir, "100.PERSONAL/03.Toki/01.Sub/raw.bin"));
    assert!(has(&dir, "100.PERSONAL/PJ.Board/plan.txt"));
    assert!(report.moved.is_empty() && report.loose.is_empty() && report.warnings.is_empty(), "{report:?}");
}

#[test]
fn a_file_in_a_main_node_moves_into_its_review_child() {
    let dir = fixture("intake-loose");
    put(&dir, "100.PERSONAL/stray.pdf", "s");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/98.PERSONAL/stray.pdf"));
    assert!(!has(&dir, "100.PERSONAL/stray.pdf"));
    assert_eq!(report.moved.len(), 1, "{report:?}");
    assert_eq!((report.moved[0].node.as_str(), report.moved[0].name.as_str()), ("100.PERSONAL", "100.PERSONAL"));
    assert_eq!((report.moved[0].count, report.moved[0].review.as_str()), (1, "98.PERSONAL"));
    assert!(report.loose.is_empty() && report.warnings.is_empty(), "{report:?}");
}

#[test]
fn folder_manager_files_at_root_are_exempt_but_other_root_files_are_reported() {
    let dir = fixture("intake-exempt");
    put(&dir, "FolderManager.exe", "x");
    put(&dir, "FolderManager_notas.txt", "x");
    put(&dir, "100.PERSONAL/desktop.ini", "x");
    put(&dir, "100.PERSONAL/.hidden", "x");
    put(&dir, "New Text Document.txt", "x");
    put(&dir, "999.Settings/00.Core/core.txt", "x");
    put(&dir, "100.PERSONAL/03.Toki/Thumbs.db", "x");
    let report = run(dir.path()).unwrap();
    assert!(report.moved.is_empty() && report.warnings.is_empty(), "{report:?}");
    assert_eq!(report.loose.len(), 1, "{report:?}");
    assert_eq!((report.loose[0].node.as_str(), report.loose[0].file.as_str()), ("", "New Text Document.txt"));
    assert!(has(&dir, "FolderManager.exe") && has(&dir, "FolderManager_notas.txt"));
    assert!(has(&dir, "100.PERSONAL/03.Toki/Thumbs.db"));
}

#[test]
fn a_name_collision_lands_with_a_number_and_keeps_the_existing_file() {
    let dir = fixture("intake-collision");
    put(&dir, "100.PERSONAL/03.Toki/same.txt", "new");
    put(&dir, "100.PERSONAL/03.Toki/98.Toki/same.txt", "old");
    let report = run(dir.path()).unwrap();
    assert_eq!(read(&dir.path().join("100.PERSONAL/03.Toki/98.Toki/same.txt")), "old");
    assert_eq!(read(&dir.path().join("100.PERSONAL/03.Toki/98.Toki/same (2).txt")), "new");
    assert!(!has(&dir, "100.PERSONAL/03.Toki/same.txt"));
    assert_eq!(report.moved.len(), 1, "{report:?}");
    assert!(report.warnings.is_empty(), "{report:?}");
}

#[test]
fn a_node_without_a_review_child_gets_one_and_its_files_move_into_it() {
    let dir = fixture("intake-no-review");
    put(&dir, "100.PERSONAL/05.Bare/lost.txt", "l");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/05.Bare/98.Bare/lost.txt"));
    assert!(!has(&dir, "100.PERSONAL/05.Bare/lost.txt"));
    assert_eq!(report.moved.len(), 1, "{report:?}");
    assert!(report.warnings.is_empty(), "{report:?}");
}

#[test]
fn a_node_the_app_has_never_marked_waits_until_the_user_acknowledges_it() {
    let dir = fixture("intake-waiting");
    dir.mkdir("100.PERSONAL/07.Fresh");
    put(&dir, "100.PERSONAL/07.Fresh/loose.txt", "f");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/07.Fresh/loose.txt"));
    assert_eq!(report.waiting.len(), 1, "{report:?}");
    assert_eq!(report.waiting[0].node, "100.PERSONAL/07.Fresh");
    assert!(report.moved.is_empty(), "{report:?}");
    acknowledge(dir.path(), &["100.PERSONAL".to_string(), "07.Fresh".to_string()]).unwrap();
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/07.Fresh/98.Fresh/loose.txt"));
    assert!(report.waiting.is_empty() && report.moved.len() == 1, "{report:?}");
}

#[test]
fn a_review_child_missing_on_disk_is_recreated_and_takes_the_files() {
    let dir = fixture("intake-ghost-review");
    fs::remove_dir(dir.path().join("100.PERSONAL/03.Toki/98.Toki")).unwrap();
    put(&dir, "100.PERSONAL/03.Toki/notes.txt", "n");
    let report = run(dir.path()).unwrap();
    assert!(has(&dir, "100.PERSONAL/03.Toki/98.Toki/notes.txt"));
    assert_eq!(report.moved.len(), 1, "{report:?}");
    assert!(report.warnings.is_empty(), "{report:?}");
}

#[test]
fn subfolders_are_never_moved_and_a_second_run_changes_nothing() {
    let dir = fixture("intake-subfolder");
    dir.mkdir("100.PERSONAL/03.Toki/loose-folder");
    run(dir.path()).unwrap();
    let before = yaml(dir.path());
    let report = run(dir.path()).unwrap();
    assert!(dir.path().join("100.PERSONAL/03.Toki/loose-folder").is_dir());
    assert!(report.moved.is_empty() && report.warnings.is_empty(), "{report:?}");
    assert_eq!(yaml(dir.path()), before);
}
