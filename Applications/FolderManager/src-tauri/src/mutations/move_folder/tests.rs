use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

use super::*;
use crate::ant_core::search::parse_query;
use crate::index::footprint::{plan, scan, Copies};
use crate::index::review_index::rebuild_with;
use crate::index::{reindex_paths, search_index, ReviewIndexEvent};
use crate::storage::db::Store;
use crate::test_support::{names, rel, storage, yaml, TempDir};

const TREE: &str = "100.WORK:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2, 98.Plain: {type: REV-L2}}\n  \
    02.Data: {type: NODE-L2, flags: data, 98.Data: {type: REV-L2}}\n  03.Target: {type: NODE-L2, 98.Target: {type: REV-L2}}\n  \
    04.Ghost: {type: NODE-L2}\n  PJ.001_Web: {type: WKB-L2}\n  PJ.002_App: {type: WKB-L2}\n  \
    PJ.003_Big: {type: WKB-L2, 01.Part: {type: NODE-L3}}\n200.HOME:\n  type: NODE-L1\n  01.Garden: {type: NODE-L2, 98.Garden: {type: REV-L2}}\n";
const DIRS: &[&str] = &[
    "998.Review", "100.WORK/98.WORK", "100.WORK/90.WORK","100.WORK/01.Plain/98.Plain", "100.WORK/02.Data/98.Data", "100.WORK/02.Data/.ant",
    "100.WORK/03.Target/98.Target", "100.WORK/PJ.001_Web", "100.WORK/PJ.002_App", "100.WORK/PJ.003_Big/01.Part",
    "200.HOME/98.HOME", "200.HOME/01.Garden/98.Garden",
];

#[cfg(windows)]
fn fake_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    for (target, link) in pairs {
        let mut file = link.as_os_str().to_owned();
        file.push(".lnk");
        fs::write(PathBuf::from(file), target.to_string_lossy().as_bytes()).unwrap();
    }
    Vec::new()
}

#[cfg(not(windows))]
fn fake_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    crate::ant_core::shortcuts::create_shortcuts(pairs)
}

// Startup state: synced yaml with ids, a full search index and a fully built 998.Review.
fn ready(tag: &str) -> TempDir {
    let dir = storage(tag, DIRS, TREE);
    fs::write(dir.path().join("100.WORK/02.Data/.ant/dragpath.json"), "{\"id\": \"DATA00000000001\"}\n").unwrap();
    fs::write(dir.path().join("100.WORK/02.Data/notes.txt"), "keep").unwrap();
    let mut data = yaml(dir.path());
    tree_store::sync(dir.path(), &mut data).unwrap();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    tree_store::save(dir.path(), &data).unwrap();
    search_index::rebuild(dir.path(), &data).unwrap();
    assert!(rebuild_with(dir.path(), &data, fake_create).is_empty());
    dir
}

fn uid(dir: &TempDir, parts: &[&str]) -> Option<String> {
    get_node(&yaml(dir.path()), &rel(parts)).and_then(|n| n.get("uid")).and_then(Value::as_str).map(str::to_string)
}

fn tree_on_disk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir(dir) {
        out.push(entry.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
    }
    out.sort();
    out
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(walkdir(&path));
        }
        out.push(path);
    }
    out
}

fn links(dir: &TempDir) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = fs::read_dir(dir.path().join("998.Review"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| !p.is_dir())
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), fs::read_link(&p).or_else(|_| fs::read_to_string(&p).map(PathBuf::from)).unwrap().to_string_lossy().into_owned()))
        .collect();
    out.sort();
    out
}

// The index updates the command layer runs, each compared with its full rebuild.
fn assert_indexes_like_full_rebuild(dir: &TempDir, m: &Mutation) {
    assert!(m.tree.is_none(), "{:?}", m.warnings);
    search_index::update_paths(dir.path(), &m.touched_nodes(), &m.touched).unwrap();
    let failures = reindex_paths(dir.path(), &|_: &ReviewIndexEvent| Ok(()), &m.before, &m.touched, fake_create, |_, _| panic!("no fallback"));
    assert!(failures.is_empty(), "{failures:?}");
    let state = |d: &TempDir| {
        let store = Store::open_or_create(d.path()).unwrap();
        let found: Vec<_> = ["", "type:WKB", "type:NODE"].iter().map(|q| store.search(&parse_query(q)).unwrap()).collect();
        (store.field_rows().unwrap(), found, links(d))
    };
    let incremental = state(dir);
    let mut data = yaml(dir.path());
    assert!(!tree_store::sync(dir.path(), &mut data).unwrap(), "the saved tree already matches the disk");
    assert!(scan(dir.path(), &data).unwrap().is_clean());
    search_index::rebuild(dir.path(), &data).unwrap();
    assert!(rebuild_with(dir.path(), &data, fake_create).is_empty());
    assert_eq!(incremental, state(dir));
}

#[test]
fn a_data_node_moves_into_a_node_with_its_uid_type_and_contents() {
    let dir = ready("mf-data");
    let m = move_folder(dir.path(), &rel(&["100.WORK", "02.Data"]), &rel(&["200.HOME", "01.Garden"]), false).unwrap();
    let moved = ["200.HOME", "01.Garden", "02.Data"];
    assert_eq!(m.select, Some(rel(&moved)));
    assert_eq!(m.levels.iter().map(|l| l.parent.clone()).collect::<Vec<_>>(), [rel(&["100.WORK"]), rel(&["200.HOME", "01.Garden"])]);
    assert!(!dir.path().join("100.WORK/02.Data").exists());
    assert_eq!(fs::read_to_string(dir.path().join("200.HOME/01.Garden/02.Data/notes.txt")).unwrap(), "keep");
    assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "02.Data"])).is_none());
    let node = get_node(&yaml(dir.path()), &rel(&moved)).unwrap().clone();
    assert_eq!(node.get("type").and_then(Value::as_str), Some("NODE-L2"));
    assert_eq!(node.get("flags").and_then(Value::as_str), Some("data"));
    assert_eq!(uid(&dir, &moved).as_deref(), Some("DATA00000000001"));
    assert!(node.get("98.Data").is_none(), "past the depth limit the 98 folder stays on disk only");
    assert_indexes_like_full_rebuild(&dir, &m);
}

#[test]
fn a_workboard_moves_into_a_workboard_and_into_a_data_node() {
    let dir = ready("mf-wkb");
    let web = uid(&dir, &["100.WORK", "PJ.001_Web"]).unwrap();
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.001_Web"]), &rel(&["100.WORK", "PJ.002_App"]), false).unwrap();
    assert_eq!(uid(&dir, &["100.WORK", "PJ.002_App", "PJ.001_Web"]), Some(web));
    assert_eq!(names(&dir.path().join("100.WORK/PJ.002_App")), [".ant", "PJ.001_Web"]);
    assert_indexes_like_full_rebuild(&dir, &m);
    let app = uid(&dir, &["100.WORK", "PJ.002_App"]).unwrap();
    let back = move_folder(dir.path(), &rel(&["100.WORK", "PJ.002_App", "PJ.001_Web"]), &rel(&["100.WORK"]), false).unwrap();
    assert_indexes_like_full_rebuild(&dir, &back);
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.002_App"]), &rel(&["100.WORK", "02.Data"]), false).unwrap();
    assert_eq!(uid(&dir, &["100.WORK", "02.Data", "PJ.002_App"]), Some(app));
    assert_eq!(get_type_of(&dir, &["100.WORK", "02.Data", "PJ.002_App"]), "WKB-L2");
    assert_indexes_like_full_rebuild(&dir, &m);
}

fn get_type_of(dir: &TempDir, parts: &[&str]) -> String {
    tree_store::get_type(&yaml(dir.path()), &rel(parts))
}

fn refused(dir: &TempDir, source: &[&str], destination: &[&str], copy: bool) -> FmError {
    let (disk, tree) = (tree_on_disk(dir.path()), yaml(dir.path()));
    let err = move_folder(dir.path(), &rel(source), &rel(destination), copy).unwrap_err();
    assert_eq!(disk, tree_on_disk(dir.path()), "{err:?}");
    assert_eq!(tree, yaml(dir.path()), "{err:?}");
    err
}

fn reason(err: FmError) -> String {
    match err {
        FmError::InvalidPath { reason, .. } => reason,
        other => panic!("{other:?}"),
    }
}

#[test]
fn refusals_leave_the_disk_and_the_tree_unchanged() {
    let dir = ready("mf-refuse");
    let target = ["100.WORK", "03.Target"];
    assert!(reason(refused(&dir, &["100.WORK", "01.Plain"], &target, false)).contains("no data flag"));
    assert!(matches!(refused(&dir, &["998.Review"], &target, false), FmError::Reserved { .. }));
    assert!(matches!(refused(&dir, &["100.WORK", "01.Plain", "98.Plain"], &target, false), FmError::Locked { .. }));
    assert!(matches!(refused(&dir, &["100.WORK", "04.Ghost"], &target, false), FmError::GhostNotAllowed { .. }));
    let web = ["100.WORK", "PJ.001_Web"];
    assert!(matches!(refused(&dir, &web, &["100.WORK", "90.WORK"], false), FmError::Locked { .. }));
    assert!(matches!(refused(&dir, &web, &["998.Review"], false), FmError::Reserved { .. }));
    assert!(matches!(refused(&dir, &web, &["999.Settings", "00.Core"], false), FmError::Reserved { .. }));
    assert!(matches!(refused(&dir, &web, &["100.WORK", "04.Ghost"], false), FmError::GhostNotAllowed { .. }));
    assert!(reason(refused(&dir, &web, &web, false)).contains("into itself"));
    let big = ["100.WORK", "PJ.003_Big"];
    assert!(reason(refused(&dir, &big, &["100.WORK", "PJ.003_Big", "01.Part"], true)).contains("into itself"));
    assert!(reason(refused(&dir, &web, &["100.WORK"], false)).contains("already in 100.WORK"));
    assert_eq!(reason(refused(&dir, &web, &["100.WORK"], true)), "a folder named PJ.001_Web already exists in 100.WORK");
    dir.mkdir("100.WORK/03.Target/PJ.001_Web");
    assert_eq!(reason(refused(&dir, &web, &target, false)), "a folder named PJ.001_Web already exists in 100.WORK/03.Target");
}

#[test]
fn a_review_folder_takes_a_data_node_and_a_workboard_but_998_review_does_not() {
    let dir = ready("mf-review");
    let review = ["100.WORK", "98.WORK"];
    let m = move_folder(dir.path(), &rel(&["100.WORK", "02.Data"]), &rel(&review), false).unwrap();
    assert_eq!(fs::read_to_string(dir.path().join("100.WORK/98.WORK/02.Data/notes.txt")).unwrap(), "keep");
    assert_eq!(uid(&dir, &["100.WORK", "98.WORK", "02.Data"]).as_deref(), Some("DATA00000000001"));
    assert_indexes_like_full_rebuild(&dir, &m);
    let web = uid(&dir, &["100.WORK", "PJ.001_Web"]);
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.001_Web"]), &rel(&review), false).unwrap();
    assert_eq!(uid(&dir, &["100.WORK", "98.WORK", "PJ.001_Web"]), web);
    assert_indexes_like_full_rebuild(&dir, &m);
    let copied = move_folder(dir.path(), &rel(&["100.WORK", "PJ.002_App"]), &rel(&review), true).unwrap();
    assert!(dir.path().join("100.WORK/98.WORK/PJ.002_App/.ant").is_dir());
    assert_indexes_like_full_rebuild(&dir, &copied);
    assert_eq!(reason(refused(&dir, &["100.WORK", "PJ.002_App"], &review, true)), "a folder named PJ.002_App already exists in 100.WORK/98.WORK");
    assert!(matches!(refused(&dir, &["100.WORK", "PJ.002_App"], &["998.Review"], false), FmError::Reserved { .. }));
}

#[test]
fn a_move_past_the_depth_limit_completes_and_drops_the_deeper_entries_from_the_tree() {
    let dir = ready("mf-overflow");
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.003_Big"]), &rel(&["200.HOME", "01.Garden"]), false).unwrap();
    assert_eq!(m.warnings, ["PJ.003_Big moved; folders below level 3 are not shown in the tree (they stay on disk)"]);
    let moved = ["200.HOME", "01.Garden", "PJ.003_Big"];
    assert_eq!(m.select, Some(rel(&moved)));
    assert!(dir.path().join("200.HOME/01.Garden/PJ.003_Big/01.Part").is_dir());
    assert!(!dir.path().join("100.WORK/PJ.003_Big").exists());
    assert_eq!(get_type_of(&dir, &moved), "WKB-L2");
    assert!(get_node(&yaml(dir.path()), &rel(&moved)).unwrap().get("01.Part").is_none());
    assert_indexes_like_full_rebuild(&dir, &m);
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.001_Web"]), &rel(&["100.WORK", "03.Target", "98.Target"]), false).unwrap();
    assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
    assert_eq!(m.select, Some(rel(&["100.WORK", "03.Target", "98.Target"])));
    assert!(dir.path().join("100.WORK/03.Target/98.Target/PJ.001_Web/.ant").is_dir());
    assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "PJ.001_Web"])).is_none());
    assert_indexes_like_full_rebuild(&dir, &m);
}

#[test]
fn a_copy_past_the_depth_limit_tails_every_id_inside_it() {
    let dir = ready("mf-overflow-copy");
    let read = |p: &str| fs::read_to_string(dir.path().join(p).join(".ant/dragpath.json")).unwrap();
    let (big, part) = (read("100.WORK/PJ.003_Big"), read("100.WORK/PJ.003_Big/01.Part"));
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.003_Big"]), &rel(&["100.WORK", "03.Target", "98.Target"]), true).unwrap();
    assert_eq!(m.warnings, ["PJ.003_Big copied; folders below level 3 are not shown in the tree (they stay on disk)"]);
    let copy = "100.WORK/03.Target/98.Target/PJ.003_Big";
    for (original, copied) in [(&big, read(copy)), (&part, read(&format!("{copy}/01.Part")))] {
        assert_ne!(*original, copied);
        assert!(copied.contains("-2\""), "{copied}");
    }
    assert_eq!(read("100.WORK/PJ.003_Big"), big);
    assert_indexes_like_full_rebuild(&dir, &m);
}

#[test]
fn a_raised_depth_limit_lets_a_deeper_workboard_move() {
    let dir = ready("mf-deeper");
    Store::open_or_create(dir.path()).unwrap().set_setting(depth::DEPTH_KEY, "4").unwrap();
    let m = move_folder(dir.path(), &rel(&["100.WORK", "PJ.003_Big"]), &rel(&["200.HOME", "01.Garden"]), false).unwrap();
    assert_eq!(m.select, Some(rel(&["200.HOME", "01.Garden", "PJ.003_Big"])));
    assert!(dir.path().join("200.HOME/01.Garden/PJ.003_Big/01.Part").is_dir());
}

#[test]
fn a_copy_gets_its_own_tailed_uid_and_the_source_stays() {
    let dir = ready("mf-copy");
    fs::write(dir.path().join("100.WORK/PJ.001_Web/brief.txt"), "brief").unwrap();
    let source = ["100.WORK", "PJ.001_Web"];
    let id = uid(&dir, &source).unwrap();
    let m = move_folder(dir.path(), &rel(&source), &rel(&["100.WORK", "03.Target"]), true).unwrap();
    let copy = ["100.WORK", "03.Target", "PJ.001_Web"];
    assert_eq!(m.select, Some(rel(&copy)));
    assert_eq!(uid(&dir, &source), Some(id.clone()));
    assert_eq!(uid(&dir, &copy), Some(format!("{id}-2")));
    assert!(fs::read_to_string(dir.path().join("100.WORK/03.Target/PJ.001_Web/.ant/dragpath.json")).unwrap().contains(&format!("{id}-2")));
    assert_eq!(fs::read_to_string(dir.path().join("100.WORK/PJ.001_Web/brief.txt")).unwrap(), "brief");
    assert_eq!(fs::read_to_string(dir.path().join("100.WORK/03.Target/PJ.001_Web/brief.txt")).unwrap(), "brief");
    assert_eq!(get_type_of(&dir, &copy), "WKB-L2");
    assert_indexes_like_full_rebuild(&dir, &m);
}
