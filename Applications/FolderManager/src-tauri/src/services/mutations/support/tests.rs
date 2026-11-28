use super::*;
use crate::test_support::*;

fn summary(rows: &[SupportRow]) -> Vec<(&str, &str, bool)> {
    rows.iter().map(|r| (r.kind, r.name.as_str(), r.exists)).collect()
}

#[test]
fn each_level_lists_only_the_kinds_it_may_hold() {
    let dir = academic("support-rows");
    let at = |parts: &[&str]| support_rows(dir.path(), &rel(parts)).unwrap();
    let category = at(&["100.ACADEMIC"]).unwrap();
    assert_eq!(
        summary(&category),
        [("REV", "REV.ACADEMIC", false), ("RES", "RES.Resources", false), ("TPL", "TPL.ACADEMIC", false), ("SS", "SS.ACADEMIC", false), ("REC", "REC.ACADEMIC", false)]
    );
    let math = at(&["100.ACADEMIC", "01.Math"]).unwrap();
    assert_eq!(summary(&math), [("REV", "98.Math", true), ("RES", "90.Math", true), ("TPL", "TPL.Math", false), ("SS", "SS.Math", false), ("REC", "REC.Math", false)]);
    assert_eq!(summary(&at(&["100.ACADEMIC", "01.Math", "01.Algebra"]).unwrap()), [("REV", "REV.Algebra", false)]);
}

#[test]
fn worknodes_support_folders_and_ghosts_have_no_rows() {
    let dir = work("support-none");
    for parts in [&["100.WORK", "04.Wkb"][..], &["100.WORK", "98.Rev"], &["100.WORK", "05.Ghost"], &["998.Review"]] {
        assert_eq!(support_rows(dir.path(), &rel(parts)).unwrap(), None, "{parts:?}");
    }
    assert!(support_rows(dir.path(), &[]).unwrap().is_none());
}

#[test]
fn adding_makes_the_folder_and_its_tree_entry() {
    let dir = academic("support-add");
    let math = rel(&["100.ACADEMIC", "01.Math"]);
    let m = add_support(dir.path(), &math, "TPL").unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Math/TPL.Math").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "TPL.Math"]), "TPL-L2");
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert!(matches!(add_support(dir.path(), &math, "TPL"), Err(FmError::AlreadyExists { .. })));
    assert!(matches!(add_support(dir.path(), &math, "RES"), Err(FmError::AlreadyExists { .. })), "a legacy 90. folder already is the resources");
    add_support(dir.path(), &rel(&["100.ACADEMIC"]), "SS").unwrap();
    assert!(dir.path().join("100.ACADEMIC/SS.ACADEMIC").is_dir());
}

#[test]
fn a_kind_the_level_does_not_allow_is_refused_and_nothing_is_made() {
    let dir = academic("support-refuse");
    let before = names(&dir.path().join("100.ACADEMIC/01.Math"));
    for (parts, kind) in [(&["100.ACADEMIC", "01.Math", "01.Algebra"][..], "SS"), (&["100.ACADEMIC", "01.Math", "01.Algebra"], "RES"), (&["100.ACADEMIC", "01.Math"], "CFG")] {
        assert!(matches!(add_support(dir.path(), &rel(parts), kind), Err(FmError::InvalidPath { .. })), "{parts:?} {kind}");
    }
    assert_eq!(names(&dir.path().join("100.ACADEMIC/01.Math")), before);
    let work = work("support-refuse-locked");
    assert!(matches!(add_support(work.path(), &rel(&["100.WORK", "04.Wkb"]), "REV"), Err(FmError::InvalidPath { .. })));
    assert!(matches!(add_support(work.path(), &rel(&["100.WORK", "98.Rev"]), "REV"), Err(FmError::Locked { .. })));
}

#[test]
fn the_deepest_level_keeps_its_review_folder_on_disk_only() {
    let dir = academic("support-deepest");
    let algebra = rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]);
    add_support(dir.path(), &algebra, "REV").unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Math/01.Algebra/REV.Algebra").is_dir());
    assert!(get_node(&yaml(dir.path()), &algebra).unwrap().get("REV.Algebra").is_none());
    remove_support(dir.path(), &algebra, "REV.Algebra", ok_recycle).unwrap();
    assert!(!dir.path().join("100.ACADEMIC/01.Math/01.Algebra/REV.Algebra").exists());
}

#[test]
fn only_an_empty_folder_is_quit_and_its_dot_ant_marker_does_not_count() {
    let dir = academic("support-quit");
    let math = rel(&["100.ACADEMIC", "01.Math"]);
    dir.mkdir("100.ACADEMIC/01.Math/98.Math/.ant");
    let rows = support_rows(dir.path(), &math).unwrap().unwrap();
    assert!(rows.iter().all(|r| r.empty));
    std::fs::write(dir.path().join("100.ACADEMIC/01.Math/90.Math/notes.txt"), "keep me").unwrap();
    let rows = support_rows(dir.path(), &math).unwrap().unwrap();
    assert!(!rows.iter().find(|r| r.name == "90.Math").unwrap().empty);

    let refused = remove_support(dir.path(), &math, "90.Math", |_| panic!("a full folder must never reach the bin"));
    assert!(matches!(refused, Err(FmError::InvalidPath { .. })), "{refused:?}");
    assert!(dir.path().join("100.ACADEMIC/01.Math/90.Math/notes.txt").is_file());

    let failed = remove_support(dir.path(), &math, "98.Math", |_| Err("bin unavailable".to_string()));
    assert!(matches!(failed, Err(FmError::Io { .. })), "{failed:?}");
    assert!(dir.path().join("100.ACADEMIC/01.Math/98.Math").is_dir());

    remove_support(dir.path(), &math, "98.Math", ok_recycle).unwrap();
    assert!(!dir.path().join("100.ACADEMIC/01.Math/98.Math").exists());
    assert!(get_node(&yaml(dir.path()), &math).unwrap().get("98.Math").is_none());
}

#[test]
fn quitting_names_only_a_support_folder_that_exists() {
    let dir = academic("support-quit-names");
    let math = rel(&["100.ACADEMIC", "01.Math"]);
    assert!(matches!(remove_support(dir.path(), &math, "01.Algebra", ok_recycle), Err(FmError::InvalidPath { .. })));
    assert!(matches!(remove_support(dir.path(), &math, "TPL.Math", ok_recycle), Err(FmError::NotFound { .. })));
    assert!(matches!(remove_support(dir.path(), &math, "SS.Math", ok_recycle), Err(FmError::NotFound { .. })));
    assert!(dir.path().join("100.ACADEMIC/01.Math/01.Algebra").is_dir());
}
