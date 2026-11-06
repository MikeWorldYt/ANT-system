// Drops checked on a throwaway storage, with the files coming from a folder outside it.
use std::fs;

use super::*;
use crate::test_support::{names, rel, storage, yaml, TempDir};

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
    drop_files(dir.path(), &rel(target), &paths, copy, &[], &never_recycles)
}

fn placed(name: &str, folder: &[&str], copied: bool) -> PlacedFile {
    PlacedFile { name: name.to_string(), folder: rel(folder), copied }
}

fn never_recycles(_: &Path) -> Result<(), String> {
    Err("the tests have no Recycle Bin".to_string())
}

// A workboard folder outside the storage, carrying the id a .ant/dragpath.json would hold; the TempDir keeps it alive.
fn outside_workboard(tag: &str, name: &str, id: &str) -> (TempDir, PathBuf) {
    let root = TempDir::new(tag);
    let dir = root.path().join(name);
    fs::create_dir_all(dir.join(".ant")).unwrap();
    fs::write(dir.join(".ant/dragpath.json"), format!("{{\"id\": \"{id}\"}}\n")).unwrap();
    fs::write(dir.join("brief.txt"), "brief").unwrap();
    (root, dir)
}

#[test]
fn a_workboard_from_outside_moves_in_and_takes_the_next_track() {
    let (dir, _src) = fixture("drop-wkb");
    let (_outside, incoming) = outside_workboard("drop-wkb-in", "PJ.001_Incoming", "NEWID00000000001");
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL"]), std::slice::from_ref(&incoming), false, &[], &never_recycles).unwrap();
    assert!(out.mutation.warnings.is_empty(), "{:?}", out.mutation.warnings);
    assert_eq!(out.placed, [placed("PJ.002_Incoming", &["100.PERSONAL"], false)]);
    assert_eq!(read(&dir, "100.PERSONAL/PJ.002_Incoming/brief.txt"), "brief");
    assert!(!incoming.exists());
    assert!(out.mutation.tree.is_some(), "a placed workboard is a yaml change, so the tree is read again");
}

#[test]
fn a_workboard_already_in_the_storage_is_reported_by_its_id_and_not_placed() {
    let (dir, _src) = fixture("drop-wkb-conflict");
    let mut data = yaml(dir.path());
    tree_store::get_node_mut(&mut data, &rel(&["100.PERSONAL", "PJ.001_Board"])).unwrap().insert("uid".into(), "EXISTINGID000001".into());
    tree_store::save(dir.path(), &data).unwrap();
    let (_outside, incoming) = outside_workboard("drop-wkb-dup", "PJ.009_Test", "EXISTINGID000001");
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL"]), std::slice::from_ref(&incoming), false, &[], &never_recycles).unwrap();
    assert!(out.placed.is_empty());
    assert_eq!(out.conflicts.len(), 1, "{:?}", out.conflicts);
    assert_eq!(out.conflicts[0].existing, rel(&["100.PERSONAL", "PJ.001_Board"]));
    assert!(incoming.exists() && dir.path().join("100.PERSONAL/PJ.001_Board").is_dir());
}

#[test]
fn replacing_a_workboard_sends_the_existing_one_to_the_bin_and_places_the_new_one() {
    let (dir, _src) = fixture("drop-wkb-replace");
    let mut data = yaml(dir.path());
    tree_store::get_node_mut(&mut data, &rel(&["100.PERSONAL", "PJ.001_Board"])).unwrap().insert("uid".into(), "EXISTINGID000002".into());
    tree_store::save(dir.path(), &data).unwrap();
    let (_outside, incoming) = outside_workboard("drop-wkb-swap", "PJ.007_Other", "EXISTINGID000002");
    let sent = std::cell::RefCell::new(Vec::new());
    let bin = |p: &Path| {
        sent.borrow_mut().push(p.to_path_buf());
        fs::remove_dir_all(p).map_err(|e| e.to_string())
    };
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL"]), std::slice::from_ref(&incoming), false, std::slice::from_ref(&incoming), &bin).unwrap();
    assert_eq!(sent.borrow().as_slice(), [dir.path().join("100.PERSONAL/PJ.001_Board")]);
    assert_eq!(out.placed, [placed("PJ.001_Other", &["100.PERSONAL"], false)]);
    assert!(out.conflicts.is_empty());
    assert_eq!(read(&dir, "100.PERSONAL/PJ.001_Other/brief.txt"), "brief");
}

#[test]
fn a_workboard_copy_keeps_the_original_and_takes_a_track_and_an_id_of_its_own() {
    let (dir, _src) = fixture("drop-wkb-copy");
    let (_outside, incoming) = outside_workboard("drop-wkb-copy-in", "PJ.001_Board", "COPYID0000000001");
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL"]), std::slice::from_ref(&incoming), true, &[], &never_recycles).unwrap();
    assert!(out.mutation.warnings.is_empty(), "{:?}", out.mutation.warnings);
    assert_eq!(out.placed, [placed("PJ.002_Board", &["100.PERSONAL"], false)]);
    assert!(incoming.join("brief.txt").is_file() && dir.path().join("100.PERSONAL/PJ.002_Board/brief.txt").is_file());
    let uid = tree_store::get_node(&yaml(dir.path()), &rel(&["100.PERSONAL", "PJ.002_Board"])).and_then(|n| n.get("uid")).and_then(Value::as_str).map(str::to_string);
    assert!(uid.as_deref().is_some_and(|id| id != "COPYID0000000001"), "{uid:?}");
}

#[test]
fn a_folder_moved_across_drives_is_copied_whole_and_then_removed() {
    let outside = TempDir::new("across-src");
    let from = outside.path().join("PJ.050_X");
    fs::create_dir_all(from.join("sub")).unwrap();
    fs::write(from.join("sub/a.txt"), "a").unwrap();
    let other = TempDir::new("across-dst");
    let to = other.path().join("PJ.050_X");
    assert_eq!(move_across(&from, &to).unwrap(), None);
    assert_eq!(fs::read_to_string(to.join("sub/a.txt")).unwrap(), "a");
    assert!(!from.exists());
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
fn a_plain_folder_lands_whole_while_missing_and_app_files_are_reported() {
    let (dir, src) = fixture("drop-skip");
    src.mkdir("Folder");
    let lock_like = dir.path().join("999.Settings/00.Core/keep.txt");
    fs::write(&lock_like, "app").unwrap();
    let paths = [src.path().join("Folder"), src.path().join("gone.txt"), lock_like.clone(), src.path().join("notes")];
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), &paths, false, &[], &never_recycles).unwrap();
    let m = &out.mutation;
    assert_eq!(out.placed, [placed("Folder", &["100.PERSONAL", "04.Data"], false), placed("notes", &["100.PERSONAL", "04.Data"], false)]);
    assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
    assert!(m.warnings[1].contains("belongs to Folder Manager"), "{:?}", m.warnings);
    assert!(!src.path().join("Folder").exists() && lock_like.is_file());
    assert_eq!(names(&dir.path().join("100.PERSONAL/04.Data")), ["Folder", "notes"]);
}

#[test]
fn a_file_moved_onto_its_own_folder_stays_put() {
    let (dir, _src) = fixture("drop-same");
    let own = dir.path().join("100.PERSONAL/04.Data/a.txt");
    fs::write(&own, "a").unwrap();
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), &[own.clone()], false, &[], &never_recycles).unwrap();
    assert!(out.placed.is_empty());
    let m = &out.mutation;
    assert!(m.warnings[0].contains("already in this folder"), "{:?}", m.warnings);
    assert_eq!(names(&dir.path().join("100.PERSONAL/04.Data")), ["a.txt"]);
}

#[test]
fn a_drop_on_a_review_link_lands_in_the_folder_it_opens() {
    let (dir, src) = fixture("drop-link");
    dir.mkdir("998.Review");
    fs::write(dir.path().join("998.Review/103.00.Toki.lnk"), "").unwrap();
    let out = drop_at(&dir, &src, &["998.Review", "103.00.Toki"], &["notes"], false).unwrap();
    assert_eq!(out.placed, [placed("notes", &["100.PERSONAL", "03.Toki", "98.Toki"], false)]);
    assert!(dir.path().join("100.PERSONAL/03.Toki/98.Toki/notes").is_file());
    assert!(drop_at(&dir, &src, &["998.Review", "999.Gone"], &["report.pdf"], false).is_err());
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

#[test]
fn a_file_already_in_the_storage_moves_to_another_folder_and_a_copy_keeps_the_original() {
    let (dir, _src) = fixture("drop-internal");
    let original = dir.path().join("100.PERSONAL/03.Toki/98.Toki/inside.txt");
    fs::write(&original, "i").unwrap();
    let out = drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), std::slice::from_ref(&original), false, &[], &never_recycles).unwrap();
    assert!(!original.exists());
    let landed = dir.path().join(out.placed[0].folder.join("/")).join("inside.txt");
    assert_eq!(fs::read_to_string(&landed).unwrap(), "i");
    assert!(out.mutation.touched.contains(&rel(&["100.PERSONAL", "03.Toki", "98.Toki"])), "{:?}", out.mutation.touched);
    fs::write(&original, "again").unwrap();
    drop_files(dir.path(), &rel(&["100.PERSONAL", "04.Data"]), std::slice::from_ref(&original), true, &[], &never_recycles).unwrap();
    assert!(original.exists());
}
