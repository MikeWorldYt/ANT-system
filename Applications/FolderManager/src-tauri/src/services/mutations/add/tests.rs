use super::*;
use crate::test_support::*;

#[test]
fn add_node_takes_lowest_free_id_and_creates_review_folder() {
    let dir = academic("add");
    let m = add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
    assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.New-Node"])));
    assert!(dir.path().join("100.ACADEMIC/04.New-Node/REV.New-Node").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node"]), "NODE-L2");
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node", "REV.New-Node"]), "REV-L2");

    let deep = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
    let p = ["100.ACADEMIC", "01.Math", "02.New-Node"];
    assert_eq!(deep.select, Some(rel(&p)));
    assert!(dir.path().join("100.ACADEMIC/01.Math/02.New-Node/REV.New-Node").is_dir());
    assert_eq!(yaml_type(dir.path(), &p), "NODE-L3");
    assert!(get_node(&yaml(dir.path()), &rel(&p)).unwrap().get("REV.New-Node").is_none());
}

#[test]
fn add_next_goes_beside_a_level_two_folder_and_marks_it_with_a_dot_ant() {
    let dir = academic("add-next");
    let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Node).unwrap();
    assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.New-Node"])));
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    let id = std::fs::read_to_string(dir.path().join("100.ACADEMIC/04.New-Node/.ant/dragpath.json")).unwrap();
    assert!(id.contains("\"id\""), "{id}");
    let main = add_next(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
    let expected = rel(&["200.NEW-CATEGORY"]);
    assert_eq!(main.select, Some(expected.clone()));
    assert_eq!(yaml_type(dir.path(), &["200.NEW-CATEGORY"]), "NODE-L1");
    assert!(dir.path().join("200.NEW-CATEGORY/REV.NEW-CATEGORY").is_dir());
    let main_node = main.touched_nodes()[0];
    assert_eq!(main_node.rel_parts, expected);
    assert!(main_node.flags.iter().any(|flag| flag == "main"));
}

#[test]
fn a_new_worknode_takes_the_next_tracker_and_starts_its_info_json() {
    let dir = academic("add-wkb");
    let p = rel(&["100.ACADEMIC", "01.Math"]);
    let first = add_next(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Worknode).unwrap();
    assert_eq!(first.select, Some(child_rel(&p, "PJ.001_New-Worknode")));
    let second = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Worknode).unwrap();
    assert_eq!(second.select, Some(rel(&["100.ACADEMIC", "PJ.001_New-Worknode"])));
    let wkb = dir.path().join("100.ACADEMIC/01.Math/PJ.001_New-Worknode");
    assert_eq!(names(&wkb), [".ant"], "no 98 folder inside a worknode");
    let info = std::fs::read_to_string(wkb.join(".ant/info.json")).unwrap();
    assert!(info.contains("\"status\": \"in-progress\"") && info.contains("\"owner\": \"unknown\""), "{info}");
    assert!(info.contains("\"breadcrumb\": \"01.Math\"") && info.contains("\"created\": \""), "{info}");
    assert!(wkb.join(".ant/dragpath.json").is_file());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "PJ.001_New-Worknode"]), "WKB-L3");
}

#[test]
fn a_worknode_counts_only_its_siblings_and_those_made_on_disk() {
    let dir = academic("add-wkb-outside");
    dir.mkdir("100.ACADEMIC/01.Math/PJ.180_Elsewhere");
    dir.mkdir("100.ACADEMIC/PJ.014_Outside");
    let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Worknode).unwrap();
    assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "PJ.015_New-Worknode"])));
}

#[test]
fn trackers_keep_counting_past_three_digits() {
    let dir = academic("add-wkb-1000");
    std::fs::create_dir(dir.path().join("100.ACADEMIC/PJ.999_Full")).unwrap();
    let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Worknode).unwrap();
    assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "PJ.1000_New-Worknode"])));
    let again = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Worknode).unwrap();
    assert_eq!(again.select, Some(rel(&["100.ACADEMIC", "PJ.1001_New-Worknode"])));
}

#[test]
fn add_node_refuses_depth_ghost_locked_and_reserved_parents() {
    let dir = academic("add-refuse");
    let ghost = add_node(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), NewKind::Node);
    assert!(matches!(ghost, Err(FmError::GhostNotAllowed { .. })));
    let locked = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), NewKind::Node);
    assert!(matches!(locked, Err(FmError::Locked { .. })));
    assert!(matches!(add_node(dir.path(), &rel(&["999.Settings"]), NewKind::Node), Err(FmError::Reserved { .. })));
}

#[test]
fn a_raised_depth_limit_moves_the_last_level_down() {
    let dir = academic("add-deeper");
    crate::infra::storage::db::Store::open_or_create(dir.path()).unwrap().set_setting(depth::DEPTH_KEY, "4").unwrap();
    add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "02.New-Node", "REV.New-Node"]), "REV-L3");
    let m = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Node).unwrap();
    let l4 = rel(&["100.ACADEMIC", "01.Math", "01.Algebra", "01.New-Node"]);
    assert_eq!(m.select, Some(l4.clone()));
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "01.Algebra", "01.New-Node"]), "NODE-L4");
    assert!(dir.path().join("100.ACADEMIC/01.Math/01.Algebra/01.New-Node/REV.New-Node").is_dir());
    assert!(get_node(&yaml(dir.path()), &l4).unwrap().get("REV.New-Node").is_none());
    let past = add_node(dir.path(), &l4, NewKind::Node).unwrap();
    assert_eq!(past.select, Some(rel(&["100.ACADEMIC", "01.Math", "01.Algebra", "01.New-Node", "New folder"])));
    assert!(get_node(&yaml(dir.path()), &l4).unwrap().get("New folder").is_none());
}

#[test]
fn add_node_at_root_creates_a_main_category() {
    let dir = academic("add-root");
    let m = add_node(dir.path(), &[], NewKind::Node).unwrap();
    let created = m.select.expect("the new category is selected");
    assert_eq!(created.len(), 1);
    assert!(dir.path().join(&created[0]).is_dir());
}

#[cfg(windows)]
#[test]
fn add_node_removes_created_folders_when_yaml_save_fails() {
    let dir = academic("add-rollback");
    let _held = break_tree_save(dir.path());
    let err = add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap_err();
    assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
    assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math", "02.Bio"]);
}

#[test]
fn a_given_name_is_cleaned_like_a_rename_and_keeps_the_id() {
    let dir = academic("add-named");
    let m = add_node_named(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node, Some("  linear algebra ")).unwrap();
    assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.Linear-algebra"])));
    assert!(dir.path().join("100.ACADEMIC/04.Linear-algebra/REV.Linear-algebra").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.Linear-algebra", "REV.Linear-algebra"]), "REV-L2");
    let main = add_next_named(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node, Some("home stuff")).unwrap();
    assert_eq!(main.select, Some(rel(&["200.HOME-STUFF"])));
    let wkb = add_next_named(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Worknode, Some("fabi web")).unwrap();
    assert_eq!(wkb.select, Some(rel(&["100.ACADEMIC", "PJ.001_Fabi-web"])));
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.001_Fabi-web"]), "WKB-L2");
    assert!(add_node_named(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node, None).unwrap().select.is_some());
}

#[test]
fn a_given_name_that_is_empty_or_invalid_creates_nothing() {
    let dir = academic("add-named-bad");
    let parent = rel(&["100.ACADEMIC"]);
    for bad in ["", "   ", " /:* ", "..", "x\u{1}"] {
        let err = add_node_named(dir.path(), &parent, NewKind::Node, Some(bad));
        assert!(matches!(err, Err(FmError::InvalidName { .. })), "{bad:?}: {err:?}");
        let wkb = add_node_named(dir.path(), &parent, NewKind::Worknode, Some(bad));
        assert!(matches!(wkb, Err(FmError::InvalidName { .. })), "{bad:?}: {wkb:?}");
    }
    assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math", "02.Bio"]);
}

#[test]
fn a_given_name_still_meets_the_parent_rules() {
    let dir = academic("add-named-rules");
    let locked = add_node_named(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), NewKind::Node, Some("X"));
    assert!(matches!(locked, Err(FmError::Locked { .. })));
    let reserved = add_node_named(dir.path(), &rel(&["999.Settings"]), NewKind::Node, Some("X"));
    assert!(matches!(reserved, Err(FmError::Reserved { .. })));
    let at_root = add_node_named(dir.path(), &[], NewKind::Worknode, Some("X"));
    assert!(matches!(at_root, Err(FmError::InvalidPath { .. })));
}

#[test]
fn past_the_depth_limit_a_given_name_is_the_folder_name_and_never_clashes() {
    let dir = academic("add-named-past");
    let leaf = rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]);
    let m = add_node_named(dir.path(), &leaf, NewKind::Node, Some("Notes")).unwrap();
    assert_eq!(m.select, Some(child_rel(&leaf, "Notes")));
    let clash = add_node_named(dir.path(), &leaf, NewKind::Node, Some("notes"));
    assert!(matches!(clash, Err(FmError::AlreadyExists { .. })), "{clash:?}");
    let wkb = add_node_named(dir.path(), &leaf, NewKind::Worknode, Some("sprint one")).unwrap();
    assert_eq!(wkb.select, Some(child_rel(&leaf, "PJ.001_Sprint-one")));
    assert!(matches!(add_node_named(dir.path(), &leaf, NewKind::Node, Some("status")), Err(FmError::InvalidName { .. })));
}

#[test]
fn the_outcomes_template_names_and_fills_a_new_worknode() {
    let dir = storage("add-template", &["100.WORK/00.Outcomes", "ANT.TEMPLATES/PR.Video-Short/Footage", "ANT.TEMPLATES/PR.Video-Short/.ant"], "100.WORK:\n  type: NODE-L1\n  00.Outcomes: {type: OUT-L2}\n");
    std::fs::write(dir.path().join("ANT.TEMPLATES/PR.Video-Short/edit.prproj"), "base").unwrap();
    crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultTemplate", Some("PR.Video-Short")).unwrap();
    let m = add_node(dir.path(), &rel(&["100.WORK"]), NewKind::Worknode).unwrap();
    assert_eq!(m.select, Some(rel(&["100.WORK", "PR.001_New-Worknode"])));
    let wkb = dir.path().join("100.WORK/PR.001_New-Worknode");
    assert!(wkb.join("Footage").is_dir());
    assert_eq!(std::fs::read_to_string(wkb.join("edit.prproj")).unwrap(), "base");
    assert!(wkb.join(".ant/info.json").is_file(), "the worknode keeps its own marker");
    let missing = crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultTemplate", Some("PJ.Gone"));
    assert!(matches!(missing, Err(FmError::InvalidPath { .. })));
}

#[test]
fn a_new_template_is_named_by_its_identifier_and_keeps_its_description_apart() {
    let dir = storage("new-template", &["100.WORK"], "100.WORK:\n  type: NODE-L1\n");
    let rel_parts = crate::services::mutations::create_template(dir.path(), "PR", "video short", Some("Edits under a minute")).unwrap();
    assert_eq!(rel_parts, rel(&["ANT.TEMPLATES", "PR.Video-short"]));
    let info = std::fs::read_to_string(dir.path().join("ANT.TEMPLATES/PR.Video-short/.ant/info.json")).unwrap();
    assert!(info.contains("Edits under a minute"), "{info}");
    assert!(matches!(crate::services::mutations::create_template(dir.path(), "PR.", "Video short", None), Err(FmError::AlreadyExists { .. })));
    assert!(matches!(crate::services::mutations::create_template(dir.path(), "ZZ", "x", None), Err(FmError::InvalidName { .. })));
}

#[test]
fn a_node_and_its_outcomes_folder_share_the_same_defaults() {
    let dir = storage("outcome-defaults", &["100.WORK/00.Outcomes", "100.WORK/01.Plain"], "100.WORK:
  type: NODE-L1
  00.Outcomes: {type: OUT-L2}
  01.Plain: {type: NODE-L2}
");
    let outcomes = rel(&["100.WORK", "00.Outcomes"]);
    crate::services::mutations::set_default(dir.path(), &outcomes, "defaultOwner", Some("Ana")).unwrap();
    let from_node = crate::services::mutations::outcome_defaults(dir.path(), &rel(&["100.WORK"])).unwrap().unwrap();
    assert_eq!((from_node.outcomes.clone(), from_node.owner.as_str(), from_node.status.as_str()), (outcomes.clone(), "Ana", ""));
    assert_eq!(crate::services::mutations::outcome_defaults(dir.path(), &outcomes).unwrap(), Some(from_node));
    assert_eq!(crate::services::mutations::outcome_defaults(dir.path(), &rel(&["100.WORK", "01.Plain"])).unwrap(), None);
}

fn output_fixture(tag: &str) -> crate::test_support::TempDir {
    let dir = storage(tag, &["100.WORK/00.Outcomes", "ANT.TEMPLATES/PR.Base/Footage", "ANT.TEMPLATES/PR.Base/.ant"], "100.WORK:\n  type: NODE-L1\n  00.Outcomes: {type: OUT-L2}\n");
    std::fs::write(dir.path().join("ANT.TEMPLATES/PR.Base/Footage/clip.txt"), "raw").unwrap();
    dir
}

fn new_worknode_with_output(dir: &std::path::Path, output: &str) -> std::path::PathBuf {
    crate::services::mutations::set_default(dir, &rel(&["100.WORK", "00.Outcomes"]), "defaultOutput", Some(output)).unwrap();
    add_node(dir, &rel(&["100.WORK"]), NewKind::Worknode).unwrap();
    dir.join("100.WORK/PJ.001_New-Worknode")
}

#[test]
fn an_output_preset_is_saved_with_one_leading_slash_and_creates_its_folder() {
    let dir = output_fixture("output-folder");
    let wkb = new_worknode_with_output(dir.path(), " render\\ ");
    assert!(wkb.join("render").is_dir());
    let saved = crate::services::mutations::outcome_defaults(dir.path(), &rel(&["100.WORK"])).unwrap().unwrap();
    assert_eq!(saved.output, "/render");
    assert!(std::fs::read_to_string(wkb.join(".ant/info.json")).unwrap().contains("\"output\": \"render\""));
    crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultOutput", Some("  ")).unwrap();
    assert_eq!(crate::services::mutations::outcome_defaults(dir.path(), &rel(&["100.WORK"])).unwrap().unwrap().output, "");
}

#[test]
fn a_nested_output_preset_creates_every_folder() {
    let dir = output_fixture("output-nested");
    let wkb = new_worknode_with_output(dir.path(), "//dist\\build//");
    assert!(wkb.join("dist/build").is_dir());
}

#[test]
fn an_output_preset_naming_a_file_does_not_create_it() {
    let dir = output_fixture("output-file");
    let wkb = new_worknode_with_output(dir.path(), "/executable.exe");
    assert!(!wkb.join("executable.exe").exists());
    assert_eq!(names(&wkb), [".ant"]);
}

#[test]
fn an_output_preset_naming_a_file_creates_its_parent_folders() {
    let dir = output_fixture("output-file-parent");
    let wkb = new_worknode_with_output(dir.path(), "/dist/app.exe");
    assert!(wkb.join("dist").is_dir());
    assert!(!wkb.join("dist/app.exe").exists());
}

#[test]
fn an_output_preset_keeps_the_template_folder_it_names() {
    let dir = output_fixture("output-template");
    crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultTemplate", Some("PR.Base")).unwrap();
    crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultOutput", Some("/Footage")).unwrap();
    add_node(dir.path(), &rel(&["100.WORK"]), NewKind::Worknode).unwrap();
    assert_eq!(std::fs::read_to_string(dir.path().join("100.WORK/PR.001_New-Worknode/Footage/clip.txt")).unwrap(), "raw");
}

#[test]
fn an_output_preset_cannot_leave_the_worknode() {
    let dir = output_fixture("output-escape");
    for bad in ["/../outside", "..\\x", "/a/../b", "C:/x"] {
        let denied = crate::services::mutations::set_default(dir.path(), &rel(&["100.WORK", "00.Outcomes"]), "defaultOutput", Some(bad));
        assert!(matches!(denied, Err(FmError::InvalidPath { .. })), "{bad}: {denied:?}");
    }
}
