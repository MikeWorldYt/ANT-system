use super::*;
use crate::infra::storage::info::{field_of, read_info};
use crate::test_support::{rel, storage, yaml, yaml_type};
use crate::services::tree::tree_store::get_node;
use serde_yaml::Value;

const TREE: &str = "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2, Deep: {type: NODE-L3}}\n";

#[test]
fn a_node_below_the_root_takes_the_next_free_id_of_its_level() {
    let dir = storage("resolve-node", &["100.ACADEMIC/Physics", "100.ACADEMIC/01.Math/Deep"], TREE);
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Physics"]), Resolution::Node).unwrap();
    assert!(dir.path().join("100.ACADEMIC/02.Physics").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Physics"]), "NODE-L2");
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "Deep"]), Resolution::Node).unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Math/01.Deep").is_dir());
    let data = yaml(dir.path());
    let deep = get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "01.Deep"])).unwrap();
    assert_eq!(deep.get("flags").and_then(Value::as_str), Some(DATA_FLAG));
}

#[test]
fn a_workboard_or_track_turned_into_a_node_keeps_one_id() {
    let tree = "100.ACADEMIC:\n  type: NODE-L1\n  01.New-Node: {type: WKB-L2}\n  PJ.1007_Prueba-7: {type: NODE-L2}\n";
    let dir = storage("resolve-node-id", &["100.ACADEMIC/01.New-Node", "100.ACADEMIC/PJ.1007_Prueba-7"], tree);
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.New-Node"]), Resolution::Node).unwrap();
    assert!(dir.path().join("100.ACADEMIC/02.New-Node").is_dir());
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "PJ.1007_Prueba-7"]), Resolution::Node).unwrap();
    assert!(dir.path().join("100.ACADEMIC/01.Prueba-7").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.New-Node"]), "NODE-L2");
    assert!(crate::services::tree::root_check::check_root(dir.path()).unwrap().invalid.is_empty());
}

#[test]
fn a_root_folder_becomes_the_next_main_category() {
    let dir = storage("resolve-root", &["Marcos"], "");
    resolve_invalid(dir.path(), &rel(&["Marcos"]), Resolution::Node).unwrap();
    assert!(dir.path().join("100.Marcos").is_dir());
}

#[test]
fn a_folder_becomes_a_workboard_with_its_info_file() {
    let dir = storage("resolve-wkb", &["100.ACADEMIC/Thesis"], "100.ACADEMIC:\n  type: NODE-L1\n");
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Thesis"]), Resolution::Workboard { tracker: "PJ".to_string() }).unwrap();
    let folder = dir.path().join("100.ACADEMIC/PJ.001_Thesis");
    assert!(folder.is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.001_Thesis"]), "WKB-L2");
    let info = read_info(&folder).unwrap();
    assert_eq!(field_of(&info, &["status"]), Some("in-progress"));
}

#[test]
fn a_workboard_with_a_bad_name_continues_the_siblings_and_keeps_its_title() {
    let tree = "100.ACADEMIC:\n  type: NODE-L1\n  01.New-Node: {type: WKB-L2}\n  PJ.001_Fine: {type: WKB-L2}\n  PJ.1007_Prueba-7: {type: NODE-L2}\n";
    let dir = storage("resolve-track", &["100.ACADEMIC/01.New-Node", "100.ACADEMIC/PJ.001_Fine", "100.ACADEMIC/PJ.1007_Prueba-7"], tree);
    let pj = Resolution::Workboard { tracker: "PJ".to_string() };
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.New-Node"]), pj).unwrap();
    let folder = dir.path().join("100.ACADEMIC/PJ.1008_New-Node");
    assert!(folder.is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.1008_New-Node"]), "WKB-L2");
    assert_eq!(field_of(&read_info(&folder).unwrap(), &["status"]), Some("in-progress"));
    let pj = Resolution::Workboard { tracker: "PJ".to_string() };
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "PJ.1007_Prueba-7"]), pj).unwrap();
    assert!(dir.path().join("100.ACADEMIC/PJ.1007_Prueba-7").is_dir());
}

#[test]
fn a_tracker_in_the_wrong_case_is_normalized_and_its_hash_is_dropped() {
    let old = "Pj.154_Google-Labs_Agent-9f0d172a";
    let tree = format!("100.ACADEMIC:\n  type: NODE-L1\n  {old}: {{type: NODE-L2}}\n");
    let dir = storage("resolve-case", &[&format!("100.ACADEMIC/{old}")], &tree);
    let pj = || Resolution::Workboard { tracker: "Pj".to_string() };
    let row = Pending { rel_parts: rel(&["100.ACADEMIC", old]), resolution: pj() };
    assert_eq!(preview_names(dir.path(), &[row]).unwrap(), ["PJ.154_Google-Labs-Agent"]);
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", old]), pj()).unwrap();
    assert!(dir.path().join("100.ACADEMIC/PJ.154_Google-Labs-Agent").is_dir());
    assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "PJ.154_Google-Labs-Agent"]), "WKB-L2");
}

#[test]
fn a_number_another_workboard_uses_gives_way_to_the_next_one() {
    let tree = "100.ACADEMIC:\n  type: NODE-L1\n  PJ.154_Other: {type: WKB-L2}\n  Pj.154_Mine: {type: NODE-L2}\n";
    let dir = storage("resolve-taken", &["100.ACADEMIC/PJ.154_Other", "100.ACADEMIC/Pj.154_Mine"], tree);
    resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Pj.154_Mine"]), Resolution::Workboard { tracker: "PJ".to_string() }).unwrap();
    assert!(dir.path().join("100.ACADEMIC/PJ.155_Mine").is_dir());
}

#[test]
fn the_preview_gives_each_row_its_own_id_in_the_order_the_apply_takes() {
    let dir = storage("resolve-preview", &["100.ACADEMIC/Prueba 1", "100.ACADEMIC/Prueba 2", "100.ACADEMIC/Thesis"], "100.ACADEMIC:\n  type: NODE-L1\n");
    let rows = [
        Pending { rel_parts: rel(&["100.ACADEMIC", "Prueba 1"]), resolution: Resolution::Node },
        Pending { rel_parts: rel(&["100.ACADEMIC", "Prueba 2"]), resolution: Resolution::Node },
        Pending { rel_parts: rel(&["100.ACADEMIC", "Thesis"]), resolution: Resolution::Workboard { tracker: "PJ".to_string() } },
    ];
    assert_eq!(preview_names(dir.path(), &rows).unwrap(), ["01.Prueba 1", "02.Prueba 2", "PJ.001_Thesis"]);
    assert!(dir.path().join("100.ACADEMIC/Prueba 1").is_dir());
}

#[test]
fn identified_folders_and_root_workboards_are_refused() {
    let dir = storage("resolve-refuse", &["100.ACADEMIC/01.Math", "100.ACADEMIC/Thesis"], TREE);
    assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), Resolution::Node).is_err());
    assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC"]), Resolution::Node).is_err());
    assert!(resolve_invalid(dir.path(), &rel(&["100.ACADEMIC", "Thesis"]), Resolution::Workboard { tracker: "XX".to_string() }).is_err());
    assert!(resolve_invalid(dir.path(), &rel(&["Thesis"]), Resolution::Workboard { tracker: "PJ".to_string() }).is_err());
    assert!(dir.path().join("100.ACADEMIC/Thesis").is_dir());
}
