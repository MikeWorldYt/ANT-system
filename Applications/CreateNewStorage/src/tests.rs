// End-to-end runs of create() and storage() in temp folders, offline only.
use std::path::PathBuf;

use crate::create::storage;
use crate::test_dir::TempDir;
use crate::tree_types::TreeNode;
use crate::{create, CreateOptions, Level};

pub(crate) const TEMPLATE: &[u8] = include_bytes!("../testdata/template.md");

fn opts(root: PathBuf, allow_existing: bool) -> CreateOptions {
    CreateOptions { root, template: Some(TEMPLATE.to_vec()), allow_existing, download: false, folder_manager: false }
}

pub(crate) fn node(name: &str, kind: &str, children: Vec<TreeNode>) -> TreeNode {
    TreeNode { name: name.to_string(), kind: kind.to_string(), fields: Vec::new(), children }
}

#[test]
fn creates_the_tree_and_keeps_the_template() {
    let tmp = TempDir::new("create");
    let root = tmp.path().join("Store");
    let mut log = Vec::new();
    let tree = [
        node("100.PERSONAL", "NODE-L1", vec![node("01.Docs", "NODE-L2", vec![]), node("98.PERSONAL", "REV-L2", vec![])]),
        node("ANT.RECORDINGS", "REC-L1", vec![]),
    ];
    storage(&opts(root.clone(), false), TEMPLATE, &tree, &mut |l, m| log.push((l, m.to_string()))).unwrap();
    for dir in ["100.PERSONAL/01.Docs", "100.PERSONAL/98.PERSONAL", "ANT.RECORDINGS", ".ant"] {
        assert!(root.join(dir).is_dir(), "{dir}");
    }
    assert_eq!(std::fs::read(root.join(".ant/template.md")).unwrap(), TEMPLATE);
    assert!(!log.iter().any(|(l, _)| *l == Level::Error));
}

#[test]
fn the_starter_template_builds_its_folders() {
    let tmp = TempDir::new("starter");
    let root = tmp.path().join("Store");
    create(&opts(root.clone(), false), &mut |_, _| {}).unwrap();
    for dir in ["100.PERSONAL/01.Documents/REV.Documents", "200.WORK/03.Projects/PJ.001_Example-Project", "ANT.RECORDINGS", "ANT.SCREENSHOTS", ".ant"] {
        assert!(root.join(dir).is_dir(), "{dir}");
    }
    for cat in ["100.PERSONAL", "200.WORK", "300.STUDY", "400.HOBBIES"] {
        let tag = cat.split_once('.').unwrap().1;
        for dir in [format!("REC.{tag}"), "RES.Resources".to_string(), format!("REV.{tag}"), format!("SS.{tag}")] {
            assert!(root.join(cat).join(&dir).is_dir(), "{cat}/{dir}");
        }
    }
    assert!(root.join("100.PERSONAL/01.Documents/01.Identity/REV.Identity").is_dir() && !root.join("200.WORK/03.Projects/PJ.001_Example-Project/REV.Example-Project").exists());
    assert!(!root.join("900.Resources").exists() && !root.join("999.Settings").exists() && !root.join("backup").exists());
    assert!(!root.join("998.Review").exists() && !root.join("200.WORK/03.Projects/PJ.001_Example-Project/.ant").exists());
}

#[test]
fn failure_rolls_back_only_this_runs_folders() {
    let tmp = TempDir::new("fail");
    let root = tmp.path().join("Store");
    std::fs::create_dir_all(root.join("keep")).unwrap();
    let long = "x".repeat(300);
    let tree = [node("100.A", "NODE", vec![node("01.B", "NODE", vec![])]), node(&long, "NODE", vec![])];
    let err = storage(&opts(root.clone(), true), TEMPLATE, &tree, &mut |_, _| {}).unwrap_err();
    assert!(err.contains("Could not create"), "{err}");
    assert!(root.join("keep").is_dir());
    assert!(!root.join("100.A").exists());
}

#[test]
fn existing_root_needs_permission_and_a_bad_template_touches_nothing() {
    let tmp = TempDir::new("exists");
    let err = create(&opts(tmp.path().to_path_buf(), false), &mut |_, _| {}).unwrap_err();
    assert!(err.contains("already exists"));
    let root = tmp.path().join("New");
    let broken = CreateOptions { root: root.clone(), template: Some(b"not a template".to_vec()), allow_existing: false, download: false, folder_manager: false };
    assert!(create(&broken, &mut |_, _| {}).is_err());
    assert!(!root.exists());
}

#[test]
fn entries_with_fields_get_an_info_file() {
    let tmp = TempDir::new("info-tree");
    let root = tmp.path().join("Store");
    let mut project = node("PJ.001_A", "WKB", vec![]);
    project.fields = [("owner", "Ren"), ("status", "done"), ("created", "2025-05-16")].map(|(k, v)| (k.to_string(), v.to_string())).to_vec();
    let tree = [node("300.PROJECTS", "WS-L1", vec![project])];
    storage(&opts(root.clone(), false), TEMPLATE, &tree, &mut |_, _| {}).unwrap();
    let info = std::fs::read_to_string(root.join("300.PROJECTS/PJ.001_A/.ant/info.json")).unwrap();
    assert!(info.contains("\"created\": \"25-05-16\"") && info.contains("\"owner\": \"Ren\""));
    assert!(!root.join("300.PROJECTS/.ant").exists());
}
