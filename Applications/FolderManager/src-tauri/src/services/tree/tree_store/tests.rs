use crate::test_support::TempDir;
use super::*;
use crate::test_support::rel;

fn parse(s: &str) -> Mapping {
    serde_yaml::from_str(s).unwrap()
}

#[test]
fn load_without_a_db_gives_empty_mapping() {
    let dir = TempDir::new("load-missing");
    assert!(load(dir.path()).unwrap().is_empty());
    assert!(!dir.path().join(".ant/ant.db").exists());
}

#[test]
fn a_level_load_holds_the_path_and_what_lies_under_it_only() {
    let dir = TempDir::new("load-level");
    let data = sort_mapping(&parse(RICH));
    let mut data = data;
    data.extend(parse("deep:
  type: NODE-L1
  mid:
    type: NODE-L2
    leaf: {type: X}
  side: {type: NODE-L2}
other:
  type: NODE-L1
  kid: {type: Y}
"));
    save(dir.path(), &data).unwrap();
    let full = load(dir.path()).unwrap();
    let level = load_level(dir.path(), &rel(&["deep", "mid"])).unwrap();
    assert_eq!(get_node(&level, &rel(&["deep", "mid"])), get_node(&full, &rel(&["deep", "mid"])));
    assert_eq!(get_type(&level, &rel(&["deep"])), "NODE-L1");
    assert!(get_node(&level, &rel(&["deep", "side"])).is_none());
    assert!(get_node(&level, &rel(&["other"])).is_none());
    assert_eq!(load_level(dir.path(), &[]).unwrap(), full);
    let missing = load_level(dir.path(), &rel(&["deep", "nope"])).unwrap();
    assert_eq!(get_type(&missing, &rel(&["deep"])), "NODE-L1");
    assert!(get_node(&missing, &rel(&["deep", "nope"])).is_none());
}

#[test]
fn a_path_load_holds_each_chain_without_children() {
    let dir = TempDir::new("load-paths");
    let data = parse("a:
  type: NODE-L1
  flags: hidden
  b:
    type: NODE-L2
    c: {type: X}
  d: {type: Y}
z: {type: NODE-L1}
");
    save(dir.path(), &data).unwrap();
    let part = load_paths(dir.path(), &[rel(&["a", "b"]), rel(&["a", "d"]), rel(&["a", "gone"])]).unwrap();
    assert_eq!(get_type(&part, &rel(&["a", "b"])), "NODE-L2");
    assert_eq!(get_type(&part, &rel(&["a", "d"])), "Y");
    assert!(get_node(&part, &rel(&["a"])).unwrap().get("flags").is_some());
    assert!(get_node(&part, &rel(&["a", "b", "c"])).is_none());
    assert!(get_node(&part, &rel(&["a", "gone"])).is_none());
    assert!(get_node(&part, &rel(&["z"])).is_none());
}

#[test]
fn save_then_load_round_trips_with_sorted_keys() {
    let dir = TempDir::new("roundtrip");
    let data = parse("zeta:\n  type: NODE-L1\n  b: {type: X}\n  a: {type: Y}\nalpha:\n  type: REV\n");
    save(dir.path(), &data).unwrap();
    let loaded = load(dir.path()).unwrap();
    let order: Vec<_> = loaded.keys().filter_map(Value::as_str).collect();
    assert_eq!(order, ["alpha", "zeta"]);
    let children: Vec<_> = get_node(&loaded, &rel(&["zeta"])).unwrap().keys().filter_map(Value::as_str).collect();
    assert_eq!(children, ["type", "a", "b"]);
    assert_eq!(get_type(&loaded, &rel(&["zeta", "a"])), "Y");
    assert_eq!(get_type(&loaded, &rel(&["alpha"])), "REV");
    assert!(!dir.path().join("999.Settings").exists());
}

#[test]
fn sync_adds_new_dirs_keeps_ghosts_and_settles() {
    let dir = TempDir::new("sync");
    dir.mkdir("01.Math/PJ.Demo");
    dir.mkdir("01.Math/PJ.Demo/deep/deeper");
    dir.mkdir("90.Resources");
    dir.mkdir(".ant-trash/x");
    let mut data = parse("ZZ.Ghost:\n  type: NODE-L1\n01.Math: null\n");
    assert!(sync(dir.path(), &mut data).unwrap());
    assert_eq!(get_type(&data, &rel(&["ZZ.Ghost"])), "NODE-L1");
    assert_eq!(get_type(&data, &rel(&["01.Math"])), "NODE-L1");
    assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo"])), "WKB");
    assert_eq!(get_type(&data, &rel(&["90.Resources"])), "RES");
    assert_eq!(get_type(&data, &rel(&["01.Math", "PJ.Demo", "deep"])), "NODE-L3");
    assert!(get_node(&data, &rel(&["01.Math", "PJ.Demo", "deep", "deeper"])).is_none());
    assert!(data.get(".ant-trash").is_none());
    assert!(!sync(dir.path(), &mut data).unwrap());
}

fn settle(root: &Path, data: &mut Mapping) -> bool {
    sync(root, data).unwrap() | reconcile_portfolios_all(root, data).unwrap()
}

#[test]
fn a_portfolio_follows_its_workboards_on_disk() {
    let dir = TempDir::new("portfolio-follow");
    dir.mkdir("100.A/01.Plan/PJ.001_Thesis");
    let mut data = parse("");
    settle(dir.path(), &mut data);
    assert!(dir.path().join("100.A/01.Plan/00.Outcomes").is_dir());
    assert_eq!(get_type(&data, &rel(&["100.A", "01.Plan", "00.Outcomes"])), "OUT");
    std::fs::remove_dir_all(dir.path().join("100.A/01.Plan/PJ.001_Thesis")).unwrap();
    assert!(settle(dir.path(), &mut data));
    assert!(!dir.path().join("100.A/01.Plan/00.Outcomes").exists());
    assert!(get_node(&data, &rel(&["100.A", "01.Plan", "00.Outcomes"])).is_none());
}

#[test]
fn a_portfolio_with_content_stays_once_its_workboards_are_gone() {
    let dir = TempDir::new("portfolio-content");
    dir.mkdir("100.A/01.Plan/00.Porfolio/01.Notes");
    let mut data = parse("");
    settle(dir.path(), &mut data);
    assert_eq!(get_type(&data, &rel(&["100.A", "01.Plan", "00.Porfolio"])), "OUT");
    assert!(!dir.path().join("100.A/01.Plan/00.Outcomes").exists());
    assert!(dir.path().join("100.A/01.Plan/00.Porfolio/01.Notes").is_dir());
}

#[test]
fn a_legacy_00_node_becomes_a_portfolio_without_a_rename() {
    let dir = TempDir::new("portfolio-legacy");
    dir.mkdir("100.A/00.Porfolio/01.Notes");
    let mut data = parse("100.A:\n  type: NODE-L1\n  00.Porfolio: {type: NODE-L2}\n");
    assert!(settle(dir.path(), &mut data));
    assert_eq!(get_type(&data, &rel(&["100.A", "00.Porfolio"])), "OUT-L2");
    assert!(dir.path().join("100.A/00.Porfolio").is_dir());
}

#[test]
fn the_system_folders_get_no_portfolio() {
    let dir = TempDir::new("portfolio-system");
    dir.mkdir("999.Settings/00.Core");
    dir.mkdir("999.Settings/PJ.001_Inside");
    let mut data = parse("");
    settle(dir.path(), &mut data);
    assert!(!dir.path().join("999.Settings/00.Outcomes").exists());
    assert!(dir.path().join("999.Settings/00.Core").is_dir());
}

#[test]
fn the_hidden_key_in_dragpath_and_the_database_agree_after_a_sync() {
    let dir = TempDir::new("dragpath-hidden");
    dir.mkdir("100.A/02.Bio/.ant");
    dir.mkdir("100.A/03.Art/.ant");
    std::fs::write(dir.path().join("100.A/02.Bio/.ant/dragpath.json"), "{\"id\": \"bio-2\", \"hidden\": true}\n").unwrap();
    std::fs::write(dir.path().join("100.A/03.Art/.ant/dragpath.json"), "{\"id\": \"art-3\"}\n").unwrap();
    let mut data = parse("100.A:\n  type: NODE-L1\n  03.Art:\n    type: NODE-L2\n    flags: hidden\n");
    settle(dir.path(), &mut data);
    assert!(flags_in(get_node(&data, &rel(&["100.A", "02.Bio"])).unwrap()).contains(&HIDDEN_FLAG.to_string()));
    assert!(!flags_in(get_node(&data, &rel(&["100.A", "03.Art"])).unwrap()).is_empty());
    assert!(std::fs::read_to_string(dir.path().join("100.A/03.Art/.ant/dragpath.json")).unwrap().contains("\"hidden\": true"));
}

#[test]
fn a_legacy_portf_type_is_read_and_rewritten_as_out() {
    let mut data = parse("100.A:\n  type: NODE-L1\n  02.B: {type: PORTF-L2}\n  00.P: {type: PORTF}\n");
    assert!(rename_legacy_types(&mut data));
    assert_eq!(get_type(&data, &rel(&["100.A", "02.B"])), "OUT-L2");
    assert_eq!(get_type(&data, &rel(&["100.A", "00.P"])), "OUT");
    assert!(!rename_legacy_types(&mut data));
}

#[test]
fn sync_branch_lists_only_the_levels_down_to_the_folder() {
    let dir = TempDir::new("sync-branch");
    dir.mkdir("01.Math/01.Alg/01.Deep");
    dir.mkdir("02.Bio/01.Cell");
    let mut data = parse("01.Math: {type: NODE-L1}\n");
    assert!(sync_branch(dir.path(), &mut data, &rel(&["01.Math"])).unwrap());
    assert_eq!(get_type(&data, &rel(&["02.Bio"])), "NODE-L1");
    assert_eq!(get_type(&data, &rel(&["01.Math", "01.Alg"])), "NODE-L2");
    assert!(get_node(&data, &rel(&["02.Bio", "01.Cell"])).is_none());
    assert!(get_node(&data, &rel(&["01.Math", "01.Alg", "01.Deep"])).is_none());
    assert!(!sync_branch(dir.path(), &mut data, &rel(&["01.Math"])).unwrap());
}

#[test]
fn infer_type_follows_prefixes_and_depth() {
    assert_eq!(infer_type("PJ.Thesis", 1), "WKB");
    assert_eq!(infer_type("PR.X", 0), "WKB");
    assert_eq!(infer_type("HW.X", 2), "WKB");
    assert_eq!(infer_type("90.Resources", 1), "RES");
    assert_eq!(infer_type("97.Templates", 1), "TPL");
    assert_eq!(infer_type("98.Review", 2), "REV");
    assert_eq!(infer_type("RES.Brand-Assets", 1), "RES");
    assert_eq!(infer_type("tpl.Video-Project", 1), "TPL");
    assert_eq!(infer_type("REV.Math", 2), "REV");
    assert_eq!(infer_type("99.Misc", 1), "NODE-L2");
    assert_eq!(infer_type("00.Porfolio", 1), "OUT");
    assert_eq!(infer_type("01.Math", 0), "NODE-L1");
    assert_eq!(infer_type("a/b/02.X", 2), "NODE-L3");
}

#[test]
fn locked_types_are_rev_res_tpl_cfg() {
    for t in ["REV", "REV-L3", "RES", "TPL-L1", "CFG"] {
        assert!(is_locked_type(t), "{t}");
    }
    for t in ["NODE-L1", "WKB", "OUT", "", "REVX"] {
        assert!(!is_locked_type(t), "{t}");
    }
}

#[test]
fn rename_and_delete_key_act_on_the_parent_mapping() {
    let mut data = parse("p:\n  a: {type: X}\n  b: {type: Y}\n");
    assert!(rename_key(&mut data, &rel(&["p"]), "a", "c"));
    assert_eq!(get_type(&data, &rel(&["p", "c"])), "X");
    assert!(get_node(&data, &rel(&["p", "a"])).is_none());
    assert!(!rename_key(&mut data, &rel(&["p"]), "nope", "d"));
    assert!(!rename_key(&mut data, &rel(&["missing"]), "a", "d"));
    assert!(delete_key(&mut data, &rel(&["p"]), "b"));
    assert!(!delete_key(&mut data, &rel(&["p"]), "b"));
}

#[test]
fn rename_and_delete_key_never_touch_meta_values() {
    let mut data = parse("p:\n  type: NODE-L1\n  status: done\n  a: {type: X}\n");
    assert!(!delete_key(&mut data, &rel(&["p"]), "type"));
    assert!(!rename_key(&mut data, &rel(&["p"]), "status", "x"));
    assert!(!rename_key(&mut data, &rel(&["p"]), "a", "type"));
    assert_eq!(get_type(&data, &rel(&["p"])), "NODE-L1");
    assert_eq!(get_type(&data, &rel(&["p", "a"])), "X");
    assert!(get_node(&data, &rel(&["p"])).unwrap().get("status").is_some());
}

const RICH: &str = "version: 3\nzeta:\n  type: NODE-L1\n  status: done\n  note: hello\n  a: {type: Y}\n  b: {}\nalpha:\n  type: REV\n";

#[cfg(windows)]
#[test]
fn save_creates_a_hidden_db() {
    use std::os::windows::fs::MetadataExt;
    let dir = TempDir::new("db-created");
    save(dir.path(), &parse(RICH)).unwrap();
    let data_dir = dir.path().join(".ant");
    assert!(data_dir.join("ant.db").is_file());
    assert_ne!(std::fs::metadata(&data_dir).unwrap().file_attributes() & 0x2, 0, ".ant is not hidden");
}

#[test]
fn db_round_trips_extra_keys_and_root_entries() {
    let dir = TempDir::new("db-roundtrip");
    let data = sort_mapping(&parse(RICH));
    save(dir.path(), &data).unwrap();
    assert_eq!(load(dir.path()).unwrap(), data);
}

#[cfg(windows)]
#[test]
fn a_failed_db_write_leaves_the_tree_unchanged() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = TempDir::new("db-failed-write");
    let before = parse("one: {type: NODE-L1}\n");
    save(dir.path(), &before).unwrap();
    let _held = std::fs::OpenOptions::new().read(true).share_mode(0x1).open(crate::infra::storage::db::db_path(dir.path())).unwrap();
    assert!(save(dir.path(), &parse("two: {type: NODE-L1}\n")).is_err());
    assert_eq!(load(dir.path()).unwrap(), before);
}

#[test]
fn only_a_new_storage_gives_level_three_nodes_the_data_flag() {
    let dir = TempDir::new("fresh-l3");
    dir.mkdir("100.A/01.B/01.C");
    let mut data = Mapping::new();
    sync(dir.path(), &mut data).unwrap();
    assert_eq!(get_node(&data, &rel(&["100.A", "01.B", "01.C"])).unwrap().get("flags").and_then(Value::as_str), Some("data"));
    assert!(get_node(&data, &rel(&["100.A", "01.B"])).unwrap().get("flags").is_none());
    save(dir.path(), &data).unwrap();
    dir.mkdir("100.A/01.B/02.D");
    sync(dir.path(), &mut data).unwrap();
    assert!(get_node(&data, &rel(&["100.A", "01.B", "02.D"])).unwrap().get("flags").is_none());
}

const SCOPED: &str = "100.A:
  type: NODE-L1
  01.B:
    type: NODE-L2
    flags: data
    status: done
    01.C: {type: NODE-L3, note: [x, y]}
  02.D: {type: NODE-L2}
200.E:
  type: NODE-L1
  01.F: {type: NODE-L2}
";

fn dump(root: &Path) -> String {
    serde_yaml::to_string(&sort_mapping(&load(root).unwrap())).unwrap()
}

// A full save and a path save of the same edit must leave the same rows behind.
fn assert_same_as_full_save(tag: &str, edit: impl Fn(&mut Mapping), touched: &[&[&str]]) {
    let (full, scoped) = (TempDir::new(&format!("{tag}-full")), TempDir::new(&format!("{tag}-scoped")));
    let mut data = parse(SCOPED);
    save(full.path(), &data).unwrap();
    save(scoped.path(), &data).unwrap();
    edit(&mut data);
    save(full.path(), &data).unwrap();
    let touched: Vec<Vec<String>> = touched.iter().map(|t| rel(t)).collect();
    save_paths(scoped.path(), &data, &touched).unwrap();
    assert_eq!(dump(scoped.path()), dump(full.path()));
    assert_eq!(dump(full.path()), serde_yaml::to_string(&sort_mapping(&data)).unwrap());
}

#[test]
fn a_path_save_of_a_rename_matches_a_full_save() {
    assert_same_as_full_save("ps-rename", |d| { rename_key(d, &rel(&["100.A"]), "01.B", "01.Z"); }, &[&["100.A", "01.B"], &["100.A", "01.Z"]]);
}

#[test]
fn a_path_save_of_a_move_matches_a_full_save() {
    let edit = |d: &mut Mapping| {
        let entry = get_node(d, &rel(&["100.A", "01.B"])).unwrap().clone();
        delete_key(d, &rel(&["100.A"]), "01.B");
        get_node_mut(d, &rel(&["200.E"])).unwrap().insert("02.B".into(), entry.into());
    };
    assert_same_as_full_save("ps-move", edit, &[&["100.A", "01.B"], &["200.E", "02.B"]]);
}

#[test]
fn a_path_save_of_a_delete_matches_a_full_save() {
    assert_same_as_full_save("ps-delete", |d| { delete_key(d, &rel(&["100.A"]), "01.B"); }, &[&["100.A", "01.B"]]);
}

#[test]
fn a_path_save_of_a_flag_change_matches_a_full_save() {
    assert_same_as_full_save("ps-flag", |d| set_flag(get_node_mut(d, &rel(&["100.A", "02.D"])).unwrap(), "hidden", true), &[&["100.A", "02.D"]]);
}

#[test]
fn a_path_save_of_an_added_node_matches_a_full_save() {
    let edit = |d: &mut Mapping| {
        get_node_mut(d, &rel(&["200.E", "01.F"])).unwrap().insert("01.G".into(), parse("type: NODE-L3
98.G: {type: REV-L3}
").into());
    };
    assert_same_as_full_save("ps-add", edit, &[&["200.E", "01.F", "01.G"]]);
}

#[test]
fn a_path_save_creates_missing_ancestors_and_leaves_other_branches_alone() {
    let dir = TempDir::new("ps-ancestors");
    save(dir.path(), &parse("100.A: {type: NODE-L1}
")).unwrap();
    let mut data = parse(SCOPED);
    save_paths(dir.path(), &data, &[rel(&["200.E", "01.F"])]).unwrap();
    let loaded = load(dir.path()).unwrap();
    assert_eq!(get_type(&loaded, &rel(&["200.E", "01.F"])), "NODE-L2");
    assert_eq!(get_type(&loaded, &rel(&["200.E"])), "NODE-L1");
    assert!(get_node(&loaded, &rel(&["100.A", "01.B"])).is_none());
    data.clear();
    save_paths(dir.path(), &data, &[rel(&["200.E"])]).unwrap();
    assert!(get_node(&load(dir.path()).unwrap(), &rel(&["200.E"])).is_none());
}
