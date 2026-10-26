// End-to-end runs of create() in temp folders, offline only.
use crate::test_dir::TempDir;
use crate::{create, CreateOptions, Level};

const TREE: &str = "100.PERSONAL:\n  type: NODE-L1\n  01.Docs:\n    type: NODE-L2\n  98.PERSONAL:\n    type: REV-L2\n999.Settings:\n  type: CFG\n  01.Extra:\n    type: NODE\n";

fn opts(root: std::path::PathBuf, tree: &str, allow_existing: bool) -> CreateOptions {
    CreateOptions { root, tree_yaml: Some(tree.to_string()), allow_existing, download: false }
}

#[test]
fn creates_template_and_tree_yaml() {
    let tmp = TempDir::new("create");
    let root = tmp.path().join("Store");
    let mut log = Vec::new();
    create(&opts(root.clone(), TREE, false), &mut |l, m| log.push((l, m.to_string()))).unwrap();
    for dir in ["100.PERSONAL/01.Docs", "100.PERSONAL/98.PERSONAL", "998.Review", "999.Settings/01.Extra"] {
        assert!(root.join(dir).is_dir(), "{dir}");
    }
    assert_eq!(std::fs::read_to_string(root.join("999.Settings/00.Core/Tree.yaml")).unwrap(), TREE);
    assert!(!log.iter().any(|(l, _)| *l == Level::Error));
}

#[test]
fn failure_rolls_back_only_this_runs_folders() {
    let tmp = TempDir::new("fail");
    let root = tmp.path().join("Store");
    std::fs::create_dir_all(root.join("keep")).unwrap();
    let long = "x".repeat(300);
    let tree = format!("100.A:\n  type: NODE\n  01.B:\n    type: NODE\n{long}:\n  type: NODE\n");
    let err = create(&opts(root.clone(), &tree, true), &mut |_, _| {}).unwrap_err();
    assert!(err.contains("Could not create"), "{err}");
    assert!(root.join("keep").is_dir());
    assert!(!root.join("100.A").exists() && !root.join("998.Review").exists() && !root.join("999.Settings").exists());
}

#[test]
fn existing_root_needs_permission_and_bad_tree_touches_nothing() {
    let tmp = TempDir::new("exists");
    let err = create(&opts(tmp.path().to_path_buf(), TREE, false), &mut |_, _| {}).unwrap_err();
    assert!(err.contains("already exists"));
    let root = tmp.path().join("New");
    assert!(create(&opts(root.clone(), "- not a mapping\n", false), &mut |_, _| {}).is_err());
    assert!(!root.exists());
}

#[test]
fn entries_with_fields_get_an_info_file() {
    let tmp = TempDir::new("info-tree");
    let root = tmp.path().join("Store");
    let tree = "300.PROJECTS:\n  type: WS-L1\n  PJ.001_A:\n    owner: Ren\n    type: WKB\n    status: done\n    created: 2025-05-16\n";
    create(&opts(root.clone(), tree, false), &mut |_, _| {}).unwrap();
    let info = std::fs::read_to_string(root.join("300.PROJECTS/PJ.001_A/.ant/info.json")).unwrap();
    assert!(info.contains("\"created\": \"25-05-16\"") && info.contains("\"owner\": \"Ren\""));
    assert!(!root.join("300.PROJECTS/.ant").exists());
}
