// Drops checked on a throwaway storage, with the files coming from a folder outside it.
use std::fs;

use super::*;
use crate::test_support::{names, rel, storage, TempDir};

const TREE: &str = "100.PERSONAL:
  type: NODE-L1
  98.PERSONAL: {type: REV-L2}
  03.Toki:
    type: NODE-L2
    98.Toki: {type: REV-L2}
  04.Data:
    type: NODE-L2
    flags: data
  05.Bare:
    type: NODE-L2
  06.Ghost:
    type: NODE-L2
  PJ.001_Board:
    type: WKB-L2
";
const DIRS: &[&str] = &[
    "100.PERSONAL/98.PERSONAL",
    "100.PERSONAL/03.Toki/98.Toki",
    "100.PERSONAL/04.Data",
    "100.PERSONAL/05.Bare",
    "100.PERSONAL/PJ.001_Board",
];

fn fixture(tag: &str) -> (TempDir, TempDir) {
    let outside = TempDir::new(&format!("{tag}-src"));
    fs::write(outside.path().join("report.pdf"), "new").unwrap();
    fs::write(outside.path().join("notes"), "n").unwrap();
    (storage(tag, DIRS, TREE), outside)
}

fn drop_at(dir: &TempDir, src: &TempDir, target: &[&str], files: &[&str], copy: bool) -> R<DropOutcome> {
    let paths: Vec<PathBuf> = files.iter().map(|f| src.path().join(f)).collect();
    drop_files(dir.path(), &rel(target), &paths, copy)
}

fn placed(name: &str, folder: &[&str], copied: bool) -> PlacedFile {
    PlacedFile { name: name.to_string(), folder: rel(folder), copied }
}

fn read(dir: &TempDir, rel: &str) -> String {
    fs::read_to_string(dir.path().join(rel)).unwrap()
}

#[test]
fn a_move_lands_in_the_destination_and_removes_the_source() {
    let (dir, src) = fixture("drop-move");
    let out = drop_at(&dir, &src, &["100.PERSONAL", "04.Data"], &["report.pdf", "notes"], false).unwrap();
    assert!(out.mutation.warnings.is_empty(), "{:?}", out.mutation.warnings);
    assert_eq!(out.placed, [placed("report.pdf", &["100.PERSONAL", "04.Data"], false), placed("notes", &["100.PERSONAL", "04.Data"], false)]);
    assert_eq!(read(&dir, "100.PERSONAL/04.Data/report.pdf"), "new");
    assert!(dir.path().join("100.PERSONAL/04.Data/notes").is_file());
    assert!(names(src.path()).is_empty());
}

#[test]
fn a_copy_lands_with_a_numbered_name_and_keeps_the_source() {
    let (dir, src) = fixture("drop-copy");
    fs::write(dir.path().join("100.PERSONAL/03.Toki/98.Toki/report.pdf"), "old").unwrap();
    let out = drop_at(&dir, &src, &["100.PERSONAL", "03.Toki"], &["report.pdf"], true).unwrap();
    assert!(out.mutation.warnings.is_empty(), "{:?}", out.mutation.warnings);
    assert_eq!(out.placed, [placed("report (2).pdf", &["100.PERSONAL", "03.Toki", "98.Toki"], true)]);
    assert_eq!(read(&dir, "100.PERSONAL/03.Toki/98.Toki/report.pdf"), "old");
    assert_eq!(read(&dir, "100.PERSONAL/03.Toki/98.Toki/report (2).pdf"), "new");
    assert!(src.path().join("report.pdf").is_file());
    let out = drop_at(&dir, &src, &["100.PERSONAL", "03.Toki"], &["report.pdf"], false).unwrap();
    assert_eq!(out.placed, [placed("report (3).pdf", &["100.PERSONAL", "03.Toki", "98.Toki"], false)]);
    assert!(dir.path().join("100.PERSONAL/03.Toki/98.Toki/report (3).pdf").is_file());
    assert!(!src.path().join("report.pdf").exists());
}

#[test]
fn main_nodes_and_workboards_follow_the_drop_rules() {
    let (dir, src) = fixture("drop-rules");
    drop_at(&dir, &src, &["100.PERSONAL"], &["report.pdf"], true).unwrap();
    assert!(dir.path().join("100.PERSONAL/98.PERSONAL/report.pdf").is_file());
    drop_at(&dir, &src, &["100.PERSONAL", "PJ.001_Board"], &["report.pdf"], true).unwrap();
    assert!(dir.path().join("100.PERSONAL/PJ.001_Board/report.pdf").is_file());
    drop_at(&dir, &src, &["100.PERSONAL", "03.Toki", "98.Toki"], &["notes"], true).unwrap();
    assert!(dir.path().join("100.PERSONAL/03.Toki/98.Toki/notes").is_file());
}

#[test]
fn a_refused_target_changes_nothing_on_disk() {
    let (dir, src) = fixture("drop-refused");
    let before = names(&dir.path().join("100.PERSONAL"));
    let refused = [
        &["100.PERSONAL", "05.Bare"][..],
        &["100.PERSONAL", "06.Ghost"],
        &["999.Settings"],
        &[],
    ];
    for target in refused {
        assert!(drop_at(&dir, &src, target, &["report.pdf"], false).is_err(), "{target:?}");
    }
    assert_eq!(names(&dir.path().join("100.PERSONAL")), before);
    assert!(names(&dir.path().join("100.PERSONAL/05.Bare")).is_empty());
    assert_eq!(names(src.path()), ["notes", "report.pdf"]);
}

#[test]
fn folders_missing_files_and_app_files_are_skipped_and_reported() {
    let (dir, src) = fixture("drop-skip");
    src.mkdir("Folder");
    let lock_like = dir.path().join("999.Settings/00.Core/keep.txt");
    fs::write(&lock_like, "app").unwrap();
    let paths = [src.path().join("Folder"), src.path().join("gone.txt"), lock_like.clone(), src.path().join("notes")];
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), &paths, false).unwrap();
    let m = &out.mutation;
    assert_eq!(out.placed, [placed("notes", &["100.PERSONAL", "04.Data"], false)]);
    assert_eq!(m.warnings.len(), 3, "{:?}", m.warnings);
    assert!(m.warnings[0].contains("folders cannot be dropped"), "{:?}", m.warnings);
    assert!(m.warnings[2].contains("belongs to Folder Manager"), "{:?}", m.warnings);
    assert!(src.path().join("Folder").is_dir() && lock_like.is_file());
    assert_eq!(names(&dir.path().join("100.PERSONAL/04.Data")), ["notes"]);
}

#[test]
fn a_file_moved_onto_its_own_folder_stays_put() {
    let (dir, _src) = fixture("drop-same");
    let own = dir.path().join("100.PERSONAL/04.Data/a.txt");
    fs::write(&own, "a").unwrap();
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), &[own.clone()], false).unwrap();
    assert!(out.placed.is_empty());
    let m = &out.mutation;
    assert!(m.warnings[0].contains("already in this folder"), "{:?}", m.warnings);
    assert_eq!(names(&dir.path().join("100.PERSONAL/04.Data")), ["a.txt"]);
}

#[test]
fn the_tree_gets_the_destination_level_back() {
    let (dir, src) = fixture("drop-tree");
    let mut data = crate::test_support::yaml(dir.path());
    tree_store::sync(dir.path(), &mut data).unwrap();
    tree_store::save(dir.path(), &data).unwrap();
    let m = drop_at(&dir, &src, &["100.PERSONAL", "03.Toki"], &["notes"], false).unwrap().mutation;
    assert!(m.tree.is_none());
    let review = rel(&["100.PERSONAL", "03.Toki", "98.Toki"]);
    assert_eq!(m.touched, [review.clone()]);
    assert_eq!(m.levels[0].parent, rel(&["100.PERSONAL", "03.Toki"]));
    assert!(m.levels[0].children.iter().any(|n| n.rel_parts == review));
}
