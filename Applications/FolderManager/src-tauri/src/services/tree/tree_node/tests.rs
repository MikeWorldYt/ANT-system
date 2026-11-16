use super::*;
use crate::test_support::{rel, storage, TempDir};

fn names(nodes: &[TreeNode]) -> Vec<&str> {
    nodes.iter().map(|n| n.name.as_str()).collect()
}

fn fixture() -> (TempDir, Mapping) {
    let dir = TempDir::new("tree");
    for d in [
        "01.Alpha/01.Sub", "01.Alpha/98.Hidden", "02.Beta", "03.Gamma", "04.Wkb",
        "998.Review", "999.Settings/00.Core",
    ] {
        dir.mkdir(d);
    }
    let data: Mapping = serde_yaml::from_str(
        "01.Alpha:\n  type: NODE-L1\n  02.Ghost:\n    type: NODE-L2\n  03.GhostRev:\n    type: REV-L2\n\
             02.Beta:\n  type: NODE-L1\n\
             03.Gamma:\n  type: NODE-L1\n\
             04.Wkb:\n  type: WKB\n  status: In_Progress\n\
             99.Gone:\n  type: NODE-L1\n\
             998.Review:\n  type: REV\n\
             999.Settings:\n  type: NODE-L1\n",
    )
    .unwrap();
    (dir, data)
}

fn find<'a>(nodes: &'a [TreeNode], name: &str) -> &'a TreeNode {
    nodes.iter().find(|n| n.name == name).unwrap()
}

#[test]
fn build_tree_merges_disk_and_yaml_sorted_with_ghosts() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, true).unwrap();
    assert!(tree.yaml_changed);
    assert_eq!(
        names(&tree.nodes),
        ["01.Alpha", "02.Beta", "03.Gamma", "04.Wkb", "99.Gone", "998.Review"]
    );
    let gone = find(&tree.nodes, "99.Gone");
    assert!(gone.ghost && gone.in_yaml && !gone.has_children && gone.children.is_none());
    let alpha = find(&tree.nodes, "01.Alpha");
    assert!(!alpha.ghost && alpha.has_children && alpha.depth == 0);
    let nt = alpha.node_type.as_ref().unwrap();
    assert_eq!((nt.base, nt.level), (TypeBase::Node, Some(1)));
}

#[test]
fn the_90s_and_the_last_roots_close_the_list_after_workboards() {
    let dir = TempDir::new("tail");
    for d in ["01.A", "98.Review", "90.Resources", "PJ.001_Wb", "Loose"] {
        dir.mkdir(&format!("01.Top/{d}"));
    }
    let data: Mapping = serde_yaml::from_str(
        "01.Top:
  type: NODE-L1
  01.A: {type: NODE-L2}
  98.Review: {type: REV-L1}
  90.Resources: {type: RES}
  PJ.001_Wb: {type: WKB-L2}
",
    )
    .unwrap();
    let kids = build_children(dir.path(), &data, &rel(&["01.Top"])).unwrap();
    assert_eq!(names(&kids), ["01.A", "Loose", "PJ.001_Wb", "90.Resources", "98.Review"]);
}

#[test]
fn the_word_prefixed_support_folders_close_the_list_and_91_to_99_are_plain_nodes() {
    let dir = TempDir::new("tail-words");
    for d in ["01.A", "91.Plain", "99.Last", "REV.Top", "RES.Resources", "TPL.Templates", "PJ.001_Wb", "Loose"] {
        dir.mkdir(&format!("01.Top/{d}"));
    }
    let data: Mapping = serde_yaml::from_str(
        "01.Top:
  type: NODE-L1
  01.A: {type: NODE-L2}
  91.Plain: {type: NODE-L2}
  99.Last: {type: NODE-L2}
  REV.Top: {type: REV-L1}
  RES.Resources: {type: RES}
  TPL.Templates: {type: TPL}
  PJ.001_Wb: {type: WKB-L2}
",
    )
    .unwrap();
    let kids = build_children(dir.path(), &data, &rel(&["01.Top"])).unwrap();
    assert_eq!(names(&kids), ["01.A", "91.Plain", "99.Last", "Loose", "PJ.001_Wb", "RES.Resources", "REV.Top", "TPL.Templates"]);
}

#[test]
fn disk_only_linked_dirs_are_hidden_but_other_disk_only_dirs_show() {
    let (dir, data) = fixture();
    let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
    assert_eq!(names(&kids), ["01.Sub", "02.Ghost", "03.GhostRev"]);
    let sub = find(&kids, "01.Sub");
    assert!(!sub.in_yaml && !sub.ghost && sub.node_type.is_none());
    assert!(find(&kids, "02.Ghost").ghost);
    assert_eq!(find(&kids, "02.Ghost").depth, 1);

    let data2: Mapping = serde_yaml::from_str("01.Alpha:\n  98.Hidden:\n    type: REV-L2\n").unwrap();
    let kids = build_children(dir.path(), &data2, &rel(&["01.Alpha"])).unwrap();
    assert!(names(&kids).contains(&"98.Hidden"));
}

#[test]
fn status_is_parsed_case_insensitively_for_wkb_only() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let wkb = find(&tree.nodes, "04.Wkb");
    assert_eq!(wkb.status, Some(WkbStatus::InProgress));
    assert_eq!(wkb.status_raw.as_deref(), Some("In_Progress"));
    assert!(find(&tree.nodes, "02.Beta").status.is_none());
}

#[test]
fn locked_ghost_can_be_deleted_but_not_renamed_or_extended() {
    let (dir, data) = fixture();
    let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
    let ghost_rev = find(&kids, "03.GhostRev");
    assert!(ghost_rev.locked && ghost_rev.ghost);
    let a = &ghost_rev.actions;
    assert!(a.delete && !a.rename && !a.change_type && !a.copy_path && !a.add_child);
    assert!(!a.move_up && !a.move_down);
}

#[test]
fn ghost_node_can_retype_and_delete_but_not_rename() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let a = &find(&tree.nodes, "99.Gone").actions;
    assert!(!a.rename && a.change_type && a.delete && !a.add_next && !a.copy_path);
    assert!(!a.move_up && !a.move_down);
}

#[test]
fn locked_real_folder_cannot_be_deleted() {
    let (dir, data) = fixture();
    dir.mkdir("01.Alpha/98.Real");
    let mut data = data;
    let alpha = crate::services::tree::tree_store::get_node_mut(&mut data, &rel(&["01.Alpha"])).unwrap();
    alpha.insert("98.Real".into(), serde_yaml::from_str("type: REV-L2").unwrap());
    let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
    let a = &find(&kids, "98.Real").actions;
    assert!(!a.delete && !a.rename && !a.change_type && !a.add_child && a.copy_path);
}

#[test]
fn system_root_folders_stay_out_of_the_tree_but_nested_ones_show() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    assert!(!names(&tree.nodes).contains(&"999.Settings"));
    assert!(find(&tree.nodes, "998.Review").reserved);
    let nested: Mapping = serde_yaml::from_str("01.Alpha:\n  999.Settings:\n    type: NODE-L2\n").unwrap();
    dir.mkdir("01.Alpha/999.Settings");
    let kids = build_children(dir.path(), &nested, &rel(&["01.Alpha"])).unwrap();
    assert!(!find(&kids, "999.Settings").reserved);
}

#[test]
fn review_links_are_leaf_folders_that_open_their_owner() {
    let (dir, data) = fixture();
    dir.mkdir("02.Beta/98.Beta");
    std::fs::write(dir.path().join("998.Review/02.Beta.lnk"), "").unwrap();
    let kids = build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap();
    let link = find(&kids, "02.Beta");
    assert!(link.linked && !link.ghost && !link.has_children && link.node_type.is_none());
    assert_eq!(std::path::PathBuf::from(&link.abs_path), dir.path().join("02.Beta/98.Beta"));
    let a = &link.actions;
    assert!(a.open && a.copy_path && a.favorite && !a.rename && !a.delete && !a.move_up && !a.move_down);
    assert!(!link.favorite);
    dir.mkdir(".ant");
    std::fs::write(dir.path().join(".ant/lnk-favorites.json"), "[\"02.Beta\"]\n").unwrap();
    let kids = build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap();
    assert!(find(&kids, "02.Beta").favorite);
}

#[test]
fn an_empty_outcome_still_opens_and_cannot_be_favorited_or_deleted() {
    let dir = TempDir::new("tree-outcome");
    dir.mkdir("100.WORK/02.Beta/00.Outcomes");
    let mut data = Mapping::new();
    crate::services::tree::tree_store::sync(dir.path(), &mut data).unwrap();
    let kids = build_children(dir.path(), &data, &rel(&["100.WORK", "02.Beta"])).unwrap();
    let out = find(&kids, "00.Outcomes");
    assert!(out.has_children);
    assert!(!out.actions.favorite && !out.actions.delete && out.actions.open);
}

#[test]
fn the_review_root_stays_a_container_when_no_link_is_listed() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    assert!(find(&tree.nodes, "998.Review").has_children);
}

#[test]
fn reserved_check_ignores_ascii_case() {
    for name in ["999.SETTINGS", "999.settings", "998.REVIEW", "998.review"] {
        assert!(is_reserved(&rel(&[name])), "{name}");
    }
    assert!(!is_reserved(&rel(&["01.A", "998.review"])));
}

#[test]
fn review_folders_always_open_and_list_their_files() {
    let dir = storage("review-files", &["100.A/98.A", "100.A/98.B"], "100.A:\n  98.A:\n    type: REV-L2\n  98.B:\n    type: REV-L2\n");
    std::fs::write(dir.path().join("100.A/98.A/notes.txt"), "x").unwrap();
    std::fs::write(dir.path().join("100.A/98.A/.hidden"), "x").unwrap();
    std::fs::write(dir.path().join("100.A/98.A/desktop.ini"), "x").unwrap();
    let data = crate::test_support::yaml(dir.path());
    let folders = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
    let review = folders.iter().find(|n| n.name == "98.A").unwrap();
    assert!(review.has_children);
    let files = build_children(dir.path(), &data, &review.rel_parts).unwrap();
    assert_eq!(names(&files), ["notes.txt"]);
    assert!(files[0].file && !files[0].has_children && files[0].actions.rename && !files[0].actions.favorite);
    let empty = folders.iter().find(|n| n.name == "98.B").unwrap();
    assert!(empty.has_children);
    assert!(build_children(dir.path(), &data, &empty.rel_parts).unwrap().is_empty());
}

#[test]
fn folders_past_a_review_limit_show_for_browsing_without_database_actions() {
    let dir = storage("review-browse", &["100.A/01.B/98.B/New folder/Inside"], "100.A:\n  01.B:\n    type: NODE-L2\n    98.B: {type: REV-L3}\n");
    std::fs::write(dir.path().join("100.A/01.B/98.B/notes.txt"), "x").unwrap();
    let data = crate::test_support::yaml(dir.path());
    let kids = build_children(dir.path(), &data, &rel(&["100.A", "01.B", "98.B"])).unwrap();
    assert_eq!(names(&kids), ["New folder", "notes.txt"]);
    let folder = find(&kids, "New folder");
    assert!(folder.has_children && folder.node_type.is_none() && !folder.file);
    assert_eq!(names(&build_children(dir.path(), &data, &folder.rel_parts).unwrap()), ["Inside"]);
}

#[test]
fn every_folder_lists_its_files_but_reserved_folders_do_not() {
    let dir = storage("files-anywhere", &["100.A/02.B"], "100.A:\n  type: NODE-L1\n  02.B: {type: NODE-L2}\n");
    std::fs::write(dir.path().join("100.A/02.B/photo.png"), "x").unwrap();
    let data = crate::test_support::yaml(dir.path());
    let kids = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
    let node = kids.iter().find(|n| n.name == "02.B").unwrap();
    assert!(node.has_children);
    assert_eq!(names(&build_children(dir.path(), &data, &node.rel_parts).unwrap()), ["photo.png"]);
    assert!(build_children(dir.path(), &data, &rel(&["998.Review"])).unwrap().iter().all(|n| !n.file));
}

#[test]
fn workboards_open_with_their_files_and_hold_no_review_folder() {
    let dir = storage("wkb-files", &["100.A/PJ.001_X"], "100.A:\n  PJ.001_X: {type: WKB-L2}\n");
    std::fs::write(dir.path().join("100.A/PJ.001_X/brief.txt"), "x").unwrap();
    let data = crate::test_support::yaml(dir.path());
    let kids = build_children(dir.path(), &data, &rel(&["100.A"])).unwrap();
    let wkb = kids.iter().find(|n| n.name == "PJ.001_X").unwrap();
    assert!(wkb.has_children);
    assert_eq!(names(&build_children(dir.path(), &data, &wkb.rel_parts).unwrap()), ["brief.txt"]);
}

#[test]
fn meta_key_folders_and_trash_records_are_hidden() {
    let (dir, mut data) = fixture();
    dir.mkdir("01.Alpha/type");
    dir.mkdir("01.Alpha/status");
    dir.mkdir(".ant-trash/123-02.Old");
    data.insert(".ant-trash".into(), serde_yaml::from_str("type: NODE-L1").unwrap());
    data.insert(".ANT-TRASH".into(), serde_yaml::from_str("type: NODE-L1").unwrap());
    let tree = build_tree(dir.path(), &data, false).unwrap();
    assert!(!names(&tree.nodes).iter().any(|n| n.eq_ignore_ascii_case(".ant-trash")));
    let kids = build_children(dir.path(), &data, &rel(&["01.Alpha"])).unwrap();
    assert_eq!(names(&kids), ["01.Sub", "02.Ghost", "03.GhostRev"]);
}

#[test]
fn movable_nodes_stop_at_the_ends_of_the_movable_list() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let up_down = |n: &str| {
        let a = &find(&tree.nodes, n).actions;
        (a.move_up, a.move_down)
    };
    assert_eq!(up_down("01.Alpha"), (false, true));
    assert_eq!(up_down("02.Beta"), (true, true));
    assert_eq!(up_down("03.Gamma"), (true, false));
    assert_eq!(up_down("04.Wkb"), (false, false));
}

#[test]
fn a_new_folder_is_only_offered_next_to_level_two_or_deeper() {
    let dir = TempDir::new("depth");
    dir.mkdir("01.A/01.B/01.C");
    let data: Mapping = serde_yaml::from_str(
        "01.A:
  type: NODE-L1
  01.B:
    type: NODE-L2
    01.C:
      type: NODE-L3
",
    )
    .unwrap();
    let top = build_children(dir.path(), &data, &[]).unwrap();
    assert!(top[0].actions.add_next && top[0].actions.add_child);
    let l2 = build_children(dir.path(), &data, &rel(&["01.A"])).unwrap();
    assert!(l2[0].actions.add_next && l2[0].actions.add_child);
    let l3 = build_children(dir.path(), &data, &rel(&["01.A", "01.B"])).unwrap();
    assert!(l3[0].actions.add_next && !l3[0].actions.add_child);
}

#[test]
fn folders_below_the_yaml_depth_cannot_be_retyped_or_extended() {
    let dir = TempDir::new("too-deep");
    dir.mkdir("01.A/01.B/01.C/01.D/01.E");
    let data: Mapping = serde_yaml::from_str(
        "01.A:
  type: NODE-L1
  01.B:
    type: NODE-L2
    01.C:
      type: NODE-L3
",
    )
    .unwrap();
    let l3 = build_children(dir.path(), &data, &rel(&["01.A", "01.B"])).unwrap();
    assert!(l3[0].actions.change_type && l3[0].actions.add_next);
    let l4 = build_children(dir.path(), &data, &rel(&["01.A", "01.B", "01.C"])).unwrap();
    let a = &l4[0].actions;
    assert_eq!(l4[0].depth, crate::domain::config::MAX_DEPTH);
    assert!(!a.change_type && !a.add_next && a.rename && a.copy_path);
    assert!(l4[0].has_children);
}

#[test]
fn fill_children_stops_at_the_last_tree_level() {
    let (dir, data) = fixture();
    dir.mkdir("01.Alpha/01.Sub/01.Deep/01.Deeper");
    let mut tree = build_tree(dir.path(), &data, false).unwrap();
    fill_children(dir.path(), &data, &mut tree.nodes).unwrap();
    let alpha = find(&tree.nodes, "01.Alpha");
    let sub = find(alpha.children.as_ref().unwrap(), "01.Sub");
    let deep = find(sub.children.as_ref().unwrap(), "01.Deep");
    assert!(deep.has_children && deep.children.is_none());
    assert!(find(&tree.nodes, "02.Beta").children.as_ref().is_some_and(Vec::is_empty));
}

#[test]
fn favorite_and_details_follow_what_their_commands_accept() {
    let (dir, data) = fixture();
    dir.mkdir("03.Gamma/.ant");
    std::fs::write(dir.path().join("03.Gamma/.ant/info.json"), "{oops").unwrap();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let acts = |n: &str| {
        let a = &find(&tree.nodes, n).actions;
        (a.favorite, a.edit_details)
    };
    assert!(find(&tree.nodes, "04.Wkb").actions.important && !find(&tree.nodes, "02.Beta").actions.important && !find(&tree.nodes, "03.Gamma").actions.important);
    assert_eq!(acts("04.Wkb"), (true, true));
    assert_eq!(acts("02.Beta"), (true, false));
    assert_eq!(acts("03.Gamma"), (false, false));
    assert_eq!(acts("99.Gone"), (false, false));
    dir.mkdir("04.Wkb/.ant");
    std::fs::write(dir.path().join("04.Wkb/.ant/info.json"), "[1]").unwrap();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let a = &find(&tree.nodes, "04.Wkb").actions;
    assert!(!a.favorite && !a.edit_details && !a.important);
    assert!(!find(&tree.nodes, "02.Beta").actions.edit_flags && !find(&tree.nodes, "04.Wkb").actions.edit_flags);
}

#[test]
fn an_untracked_workboard_past_the_limit_is_a_workboard_with_its_info() {
    let dir = storage("untracked-wkb", &["100.A/01.B/01.C/PJ.013_Logo/.ant", "100.A/01.B/01.C/Plain"], "100.A:
  type: NODE-L1
  01.B:
    type: NODE-L2
    01.C: {type: NODE-L3}
");
    std::fs::write(dir.path().join("100.A/01.B/01.C/PJ.013_Logo/.ant/info.json"), "{\"status\": \"hold\", \"owner\": \"Ana\", \"important\": true}").unwrap();
    let data = crate::test_support::yaml(dir.path());
    let kids = build_children(dir.path(), &data, &rel(&["100.A", "01.B", "01.C"])).unwrap();
    let wkb = find(&kids, "PJ.013_Logo");
    assert!(!wkb.in_yaml && wkb.node_type.as_ref().is_some_and(|t| t.base == TypeBase::Wkb));
    assert_eq!(wkb.status, Some(WkbStatus::Hold));
    assert!(wkb.important && wkb.info.as_ref().and_then(|i| field_of(i, &["owner"])) == Some("Ana"));
    let a = &wkb.actions;
    assert!(a.edit_details && a.important && a.favorite && a.rename && a.delete && !a.change_type && !a.hide && !a.add_next);
    assert!(find(&kids, "Plain").node_type.is_none());
}

#[test]
fn snapshot_serialises_in_camel_case() {
    let (dir, data) = fixture();
    let tree = build_tree(dir.path(), &data, false).unwrap();
    let text = serde_yaml::to_string(&tree).unwrap();
    for key in ["yamlChanged", "relParts", "inYaml", "hasChildren", "absPath", "changeType", "copyPath", "editDetails", "addChild"] {
        assert!(text.contains(key), "{key}");
    }
}
