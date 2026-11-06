use super::*;

#[test]
fn numeric_id_reads_leading_ascii_digits() {
    assert_eq!(numeric_id("01.Math"), Some(1));
    assert_eq!(numeric_id("90.RES"), Some(90));
    assert_eq!(numeric_id("007.x"), Some(7));
    assert_eq!(numeric_id("0.x"), Some(0));
    assert_eq!(numeric_id("12"), None);
    assert_eq!(numeric_id("a1.x"), None);
    assert_eq!(numeric_id(".x"), None);
    assert_eq!(numeric_id("\u{0663}.x"), None);
}

#[test]
fn next_available_id_fills_gaps_within_slots() {
    let siblings = vec!["01.A".to_string(), "03.B".to_string(), "90.RES".to_string()];
    assert_eq!(next_available_id(&siblings), 2);
    let outside = vec!["00.A".to_string(), "90.R".to_string()];
    assert_eq!(next_available_id(&outside), 1);
    let empty: Vec<String> = Vec::new();
    assert_eq!(next_available_id(&empty), 1);
}

#[test]
fn next_available_id_falls_back_when_all_slots_full() {
    let full: Vec<String> = (1..=99).map(|i| format!("{i:02}.X")).collect();
    assert_eq!(next_available_id(&full), 99);
}

#[test]
fn the_slots_skip_the_numbers_a_legacy_support_folder_still_uses() {
    let siblings: Vec<String> = (1..=89).map(|i| format!("{i:02}.X")).chain(["90.Res".to_string(), "97.Tpl".to_string(), "98.Rev".to_string()]).collect();
    assert_eq!(next_available_id(&siblings), 91);
    let more: Vec<String> = siblings.into_iter().chain((91..=96).map(|i| format!("{i}.X"))).collect();
    assert_eq!(next_available_id(&more), 99);
}

#[test]
fn support_folders_are_known_by_their_word_prefix_or_the_legacy_number() {
    for (name, kind) in [("REV.Math", "REV"), ("rev.Math", "REV"), ("RES.Brand-Assets", "RES"), ("TPL.Video-Project", "TPL"), ("98.Math", "REV"), ("90.Res", "RES"), ("97.Tpl", "TPL")] {
        assert_eq!(support_kind(name), Some(kind), "{name}");
    }
    for name in ["01.Math", "99.Misc", "91.Misc", "PJ.001_Web", "Review", "REVIEW.x", "00.Outcomes"] {
        assert_eq!(support_kind(name), None, "{name}");
    }
    assert!(is_legacy_support("98.Math") && !is_legacy_support("REV.Math"));
}

#[test]
fn numbers_from_01_to_99_are_node_ids_except_the_legacy_support_ones_and_00() {
    for id in ["01.A", "55.A", "91.A", "96.A", "99.A"] {
        assert!(has_node_id(id), "{id}");
    }
    for not_id in ["00.Outcomes", "90.Res", "97.Tpl", "98.Rev", "REV.A", "RES.A", "TPL.A"] {
        assert!(!has_node_id(not_id), "{not_id}");
    }
    assert!(has_valid_id(&["100.A".to_string(), "REV.A".to_string()]) && has_valid_id(&["100.A".to_string(), "98.A".to_string()]));
}

#[test]
fn root_names_need_a_hundred_step_category_or_the_system_range() {
    for valid in ["100.ACADEMIC", "200.WORK", "800.X", "998.Review", "999.Settings", "900.Sys"] {
        assert!(is_valid_root_name(valid), "{valid}");
    }
    for invalid in ["Marcos", "New Folder", "574.Notes", "000.Zero", "050.Half", "100", "1000.Big", "01.Math"] {
        assert!(!is_valid_root_name(invalid), "{invalid}");
    }
    assert!(is_main_category("300.HOME"));
    assert!(!is_main_category("998.Review"));
}

#[test]
fn is_movable_node_needs_node_type_and_slot_range() {
    assert!(is_movable_node("05.X", "NODE-L2"));
    assert!(is_movable_node("89.x", "NODE") && is_movable_node("99.x", "NODE"));
    assert!(!is_movable_node("05.X", "WKB"));
    assert!(!is_movable_node("90.RES", "RES") && !is_movable_node("100.X", "NODE"));
    assert!(!is_movable_node("00.Core", "NODE"));
    assert!(!is_movable_node("X", "NODE"));
}

#[test]
fn sanitize_train_case_cleans_and_capitalizes() {
    assert_eq!(sanitize_train_case("  hello   world/ "), "Hello-world");
    assert_eq!(sanitize_train_case("-- a:b--"), "Ab");
    assert_eq!(sanitize_train_case("ñandu"), "Ñandu");
    assert_eq!(sanitize_train_case("  /:*  "), "");
}

#[test]
fn sanitize_upper_cleans_and_uppercases() {
    assert_eq!(sanitize_upper("ab cd"), "AB-CD");
    assert_eq!(sanitize_upper("x?y"), "XY");
}

#[test]
fn workboard_names_keep_the_tracker_and_edit_only_the_name() {
    assert_eq!(name_parts("PJ.001_Fabi-web", true), ("PJ.001_".to_string(), "Fabi-web".to_string()));
    assert_eq!(name_parts("PJ.001", true), ("PJ.001_".to_string(), String::new()));
    assert_eq!(name_parts("PJ.145_(25-11-14) Plans_DOC", true).1, "(25-11-14) Plans_DOC");
    assert_eq!(compose_name("PJ.001_", "Name", true), "PJ.001_Name");
    assert_eq!(compose_name("PJ.001_", "", true), "PJ.001");
}

#[test]
fn a_workboard_continues_the_count_of_its_new_siblings() {
    let siblings: Vec<String> = ["PJ.001_Fabi-web", "PJ.048_Phospo", "PR.002_Other"].map(String::from).to_vec();
    assert_eq!(next_track("PJ.026_Test", &siblings), "PJ.049_Test");
    assert_eq!(next_track("PJ.009", &siblings), "PJ.049");
    assert_eq!(next_track("PR.180_Mine", &siblings), "PR.003_Mine");
    assert_eq!(next_track("PJ.180_Alone", &[]), "PJ.001_Alone");
    assert_eq!(with_next_slot("PJ.026_Test", &siblings), "PJ.049_Test");
}

#[test]
fn track_names_and_their_titles() {
    assert!(is_track_name("PJ.1007_Prueba-7") && is_track_name("PR.001") && !is_track_name("PJ.Prueba"));
    assert_eq!(title_of("PJ.1007_Prueba-7"), "Prueba-7");
    assert_eq!(title_of("01.New-Node"), "New-Node");
    assert_eq!(title_of("Prueba 4"), "Prueba 4");
    assert_eq!(title_of("PJ.001"), "");
}

#[test]
fn a_workboard_title_drops_the_tracker_the_number_and_a_trailing_hash() {
    assert_eq!(workboard_title("Pj.154_Google-Labs_Agent-9f0d172a"), "Google-Labs-Agent");
    assert_eq!(workboard_title("01.New-Node"), "New-Node");
    assert_eq!(workboard_title("Thesis"), "Thesis");
    assert_eq!(workboard_title("Plan-2024"), "Plan-2024");
    assert_eq!(workboard_title("Plan-123456"), "Plan-123456");
    assert_eq!(workboard_title("Face-decade"), "Face-decade");
    assert_eq!(tracker_number("Pj.154_X", "PJ."), Some(154));
    assert_eq!(tracker_number("PR.7_X", "PJ."), None);
    assert_eq!(tracker_number("Pj.Name", "PJ."), None);
}

#[test]
fn editable_part_and_build_new_name_keep_prefix() {
    assert_eq!(editable_part("01.Math"), "Math");
    assert_eq!(editable_part("Math"), "Math");
    assert_eq!(editable_part("01."), "");
    assert_eq!(name_parts("01.Math", false), ("01.".to_string(), "Math".to_string()));
    assert_eq!(name_parts("Math", false), (String::new(), "Math".to_string()));
    assert_eq!(compose_name("01.", "Physics", false), "01.Physics");
}
