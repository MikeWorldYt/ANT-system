use std::fs;

use super::*;
use crate::infra::scan::list_subdirs;
use crate::test_support::{rel, storage, yaml, TempDir};
use crate::services::tree::tree_store::get_node;

fn log_text(dir: &TempDir) -> String {
    let path = logs_dir(dir.path()).join(format!("{}_ANT-fm-renamed.log", local_date()));
    fs::read_to_string(path).unwrap()
}

#[test]
fn letters_in_front_of_an_id_are_trimmed_and_logged() {
    let dir = storage(
        "node-names-trim",
        &["100.ACADEMIC/01.Math/Unit02.Physics", "100.ACADEMIC/Unit03.Root", "100.ACADEMIC/01.Math/Physics"],
        "100.ACADEMIC:\n  type: NODE-L1\n  Unit03.Root: {type: NODE-L2}\n  01.Math: {type: NODE-L2, Unit02.Physics: {type: NODE-L3}, Physics: {type: NODE-L3}}\n",
    );
    tidy(dir.path()).unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Math/02.Physics").is_dir());
    assert!(!dir.path().join("100.ACADEMIC/01.Math/Unit02.Physics").exists());
    assert!(dir.path().join("100.ACADEMIC/03.Root").is_dir());
    assert!(dir.path().join("100.ACADEMIC/01.Math/Physics").is_dir());
    let data = yaml(dir.path());
    assert!(get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "02.Physics"])).is_some());
    assert!(get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "Unit02.Physics"])).is_none());
    assert_eq!(
        log_text(&dir),
        "100.ACADEMIC/01.Math : \"Unit02.Physics\" → \"02.Physics\"\n100.ACADEMIC : \"Unit03.Root\" → \"03.Root\"\n",
    );
}

#[test]
fn a_trim_that_would_clash_is_skipped_and_root_names_are_left_to_the_root_check() {
    let dir = storage(
        "node-names-clash",
        &["100.ACADEMIC/01.Math/04.Chem", "100.ACADEMIC/01.Math/Unit04.Chem", "Unit05.Loose"],
        "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2, 04.Chem: {type: NODE-L3}, Unit04.Chem: {type: NODE-L3}}\n",
    );
    tidy(dir.path()).unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Math/Unit04.Chem").is_dir());
    assert!(dir.path().join("Unit05.Loose").is_dir());
    assert!(!logs_dir(dir.path()).join(format!("{}_ANT-fm-renamed.log", local_date())).exists());
}

#[test]
fn trimmed_needs_letters_then_a_two_digit_id() {
    assert_eq!(trimmed("Unit01.Math").as_deref(), Some("01.Math"));
    assert_eq!(trimmed("Unidad99.X").as_deref(), Some("99.X"));
    assert_eq!(trimmed("01.Math"), None);
    assert_eq!(trimmed("Unit1.Math"), None);
    assert_eq!(trimmed("Unit00.Math"), None);
    assert_eq!(trimmed("Unit01Math"), None);
    assert_eq!(trimmed("Unit.Math"), None);
    assert_eq!(trimmed("01Unit.Math"), None);
}

#[test]
fn workboard_names_take_the_tracker_format_and_keep_legacy_metadata() {
    let (name, meta) = workboard("PJ.111_(23-05MAY-04) Large [c011]_presentation").unwrap();
    assert_eq!(name, "PJ.111_Presentation");
    assert_eq!((meta.created.as_deref(), meta.tag.as_deref(), meta.owner.as_deref()), (Some("23-05-04"), Some("Large"), Some("c011")));
    let (name, meta) = workboard("PJ.145_(25-11-14) Plans_DOC").unwrap();
    assert_eq!(name, "PJ.145_Plans-DOC");
    assert_eq!((meta.created.as_deref(), meta.owner.is_none()), (Some("25-11-14"), true));
    let (name, meta) = workboard("PJ.150_my project").unwrap();
    assert_eq!(name, "PJ.150_My-project");
    assert!(meta.created.is_none() && meta.tag.is_none() && meta.owner.is_none());
    assert_eq!(workboard("PR.010").unwrap().0, "PR.010");
    assert!(workboard("PJ.1_Short").is_none());
    assert!(workboard("PJ.150x").is_none());
}

#[test]
fn system_folders_at_the_root_are_not_renamed_inside() {
    let dir = storage("node-names-system", &["999.Settings/Unit01.Keep", "900.Sys/Unit02.Keep"], "");
    tidy(dir.path()).unwrap();
    assert!(dir.path().join("999.Settings/Unit01.Keep").is_dir());
    assert!(dir.path().join("900.Sys/Unit02.Keep").is_dir());
}

#[test]
fn tree_entries_whose_folder_is_gone_are_left_alone() {
    let dir = storage(
        "node-names-ghosts",
        &["100.ACADEMIC/02.Keep"],
        "100.ACADEMIC:\n  type: NODE-L1\n  PJ.002_New-Workboard: {type: WKB-L2}\n  Unit03.Gone: {type: NODE-L2}\n",
    );
    tidy(dir.path()).unwrap();
    let data = yaml(dir.path());
    assert!(get_node(&data, &rel(&["100.ACADEMIC", "PJ.002_New-Workboard"])).is_some());
    assert!(get_node(&data, &rel(&["100.ACADEMIC", "Unit03.Gone"])).is_some());
    assert!(!logs_dir(dir.path()).join(format!("{}_ANT-fm-renamed.log", local_date())).exists());
}

#[test]
fn workboards_are_renamed_to_the_format_and_get_their_info_json() {
    let dir = storage(
        "node-names-workboards",
        &["100.ACADEMIC/PJ.111_(23-05MAY-04) Large [c011]_presentation", "100.ACADEMIC/PR.010_my project", "100.ACADEMIC/PJ.150_my-project"],
        "100.ACADEMIC:\n  type: NODE-L1\n",
    );
    tidy(dir.path()).unwrap();
    let parent = dir.path().join("100.ACADEMIC");
    let names = list_subdirs(&parent).unwrap();
    assert!(names.contains(&"PJ.111_Presentation".to_string()));
    assert!(names.contains(&"PR.010_My-project".to_string()));
    assert!(names.contains(&"PJ.150_My-project".to_string()));
    let info = read_info(&parent.join("PJ.111_Presentation")).unwrap();
    assert_eq!(field_of(&info, &["owner"]), Some("c011"));
    assert_eq!(field_of(&info, &["created"]), Some("23-05-04"));
    assert_eq!(field_of(&info, &["tag"]), Some("Large"));
    assert_eq!(field_of(&info, &["status"]), Some("in-progress"));
    assert!(log_text(&dir).contains("100.ACADEMIC : \"PJ.111_(23-05MAY-04) Large [c011]_presentation\" → \"PJ.111_Presentation\"\n"));
}
