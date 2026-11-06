use super::*;
use crate::infra::footprint_file::read_flags;
use crate::test_support::{rel, TempDir};
use crate::services::tree::tree_store::sync;

fn storage() -> (TempDir, Mapping) {
    let dir = TempDir::new("footprint");
    dir.mkdir("100.A/PJ.001/.ant");
    let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  PJ.001: {type: WKB-L2, status: hold}\n").unwrap();
    sync(dir.path(), &mut data).unwrap();
    (dir, data)
}

#[test]
fn ids_are_fifteen_characters_from_the_alphabet_and_avoid_taken_ones() {
    let mut taken = HashSet::new();
    for _ in 0..500 {
        let id = new_id(&taken);
        assert_eq!(id.len(), ID_LEN);
        assert!(id.bytes().all(|b| ALPHABET.contains(&b)));
        assert!(taken.insert(id));
    }
}

#[test]
fn a_folder_with_a_dot_ant_gets_one_id_and_keeps_it() {
    let (dir, mut data) = storage();
    assert!(!scan(dir.path(), &data).unwrap().is_clean());
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    let first = read_id(&dir.path().join("100.A/PJ.001")).unwrap();
    assert_eq!(first.len(), ID_LEN);
    assert!(scan(dir.path(), &data).unwrap().is_clean());
    assert!(!plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    assert_eq!(read_id(&dir.path().join("100.A/PJ.001")).unwrap(), first);
}

#[test]
fn a_node_with_loose_files_gets_no_marker_until_it_is_acknowledged() {
    let dir = TempDir::new("footprint-waiting");
    dir.mkdir("100.A/01.Plain");
    fs::write(dir.path().join("100.A/01.Plain/loose.txt"), "l").unwrap();
    let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2}\n").unwrap();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert!(!dir.path().join("100.A/01.Plain/.ant").exists());
    assert!(scan(dir.path(), &data).unwrap().is_clean());
    fs::create_dir(dir.path().join("100.A/01.Plain/.ant")).unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    assert!(read_id(&dir.path().join("100.A/01.Plain")).is_some());
}

#[test]
fn a_refresh_marks_data_nodes_too() {
    let dir = TempDir::new("footprint-marks");
    dir.mkdir("100.A/01.Plain");
    dir.mkdir("100.A/02.Data");
    let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2}\n  02.Data: {type: NODE-L2, flags: data}\n").unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    assert!(read_id(&dir.path().join("100.A/01.Plain")).is_some());
    assert!(read_id(&dir.path().join("100.A/02.Data")).is_some());
    assert!(scan(dir.path(), &data).unwrap().is_clean());
}

#[test]
fn a_folder_without_dot_ant_is_left_alone() {
    let dir = TempDir::new("footprint-plain");
    dir.mkdir("100.A/PJ.001");
    let mut data = Mapping::new();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert!(!dir.path().join("100.A/PJ.001/.ant").exists());
}

#[test]
fn a_folder_moved_outside_the_app_keeps_its_type_and_status() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/PJ.009")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    let moved = get_node(&data, &rel(&["100.A", "PJ.009"])).unwrap();
    assert_eq!(moved.get("status").and_then(Value::as_str), Some("hold"));
    assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_none());
    assert!(scan(dir.path(), &data).unwrap().is_clean());
}

fn copy_footprint(dir: &TempDir, from: &str, to: &str) {
    dir.mkdir(&format!("{to}/.ant"));
    fs::copy(dir.path().join(from).join(".ant/dragpath.json"), dir.path().join(to).join(".ant/dragpath.json")).unwrap();
}

fn uid(data: &Mapping, parts: &[&str]) -> Option<String> {
    get_node(data, &rel(parts)).and_then(|n| n.get("uid")).and_then(Value::as_str).map(str::to_string)
}

#[test]
fn a_copied_folder_is_not_taken_for_a_move() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
    sync(dir.path(), &mut data).unwrap();
    let result = plan(dir.path(), &mut data, Copies::Ignore).unwrap();
    assert!(!result.changed && result.copies.is_empty());
    assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_some() && get_node(&data, &rel(&["100.A", "PJ.002"])).is_some());
    assert_eq!(read_id(&dir.path().join("100.A/PJ.002")), read_id(&dir.path().join("100.A/PJ.001")));
}

#[test]
fn the_known_folder_keeps_the_id_and_each_copy_gets_a_tail_once() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    let id = read_id(&dir.path().join("100.A/PJ.001")).unwrap();
    fs::write(dir.path().join("100.A/PJ.001/.ant/dragpath.json"), format!("{{\"id\": \"{id}\", \"flags\": [\"urgent\"]}}\n")).unwrap();
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.000");
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
    sync(dir.path(), &mut data).unwrap();
    let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert!(result.changed);
    let original = rel(&["100.A", "PJ.001"]);
    let expected = vec![
        Copied { rel_parts: rel(&["100.A", "PJ.000"]), original: original.clone() },
        Copied { rel_parts: rel(&["100.A", "PJ.002"]), original: original.clone() },
    ];
    assert_eq!(result.copies, expected);
    assert_eq!(read_id(&dir.path().join("100.A/PJ.001")), Some(id.clone()));
    let tails = [read_id(&dir.path().join("100.A/PJ.000")).unwrap(), read_id(&dir.path().join("100.A/PJ.002")).unwrap()];
    assert_eq!(tails.iter().filter(|t| t.starts_with(&format!("{id}-")) && t.len() == ID_LEN + 2).count(), 2);
    assert_ne!(tails[0], tails[1]);
    assert_eq!(uid(&data, &["100.A", "PJ.002"]), Some(tails[1].clone()));
    assert_eq!(read_flags(&dir.path().join("100.A/PJ.002")), ["urgent"]);
    assert!(scan(dir.path(), &data).unwrap().is_clean());
    let again = plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert!(!again.changed && again.copies.is_empty());
}

#[test]
fn a_copy_of_a_copy_counts_on_from_the_original_id() {
    let taken: HashSet<String> = ["7K3M9Q2XH43O7Z4", "7K3M9Q2XH43O7Z4-2"].map(str::to_string).into();
    assert_eq!(tailed_id("7K3M9Q2XH43O7Z4", &taken), "7K3M9Q2XH43O7Z4-3");
    assert_eq!(tailed_id("7K3M9Q2XH43O7Z4-2", &taken), "7K3M9Q2XH43O7Z4-3");
}

#[test]
fn a_copy_then_move_carries_the_entry_to_one_folder_and_tails_the_other() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
    fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/PJ.009")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert_eq!(result.copies.len(), 1);
    let keeper = result.copies[0].original.clone();
    assert_eq!(get_node(&data, &keeper).unwrap().get("status").and_then(Value::as_str), Some("hold"));
    assert!(get_node(&data, &rel(&["100.A", "PJ.001"])).is_none());
    assert!(read_id(&join_rel(dir.path(), &result.copies[0].rel_parts)).unwrap().contains('-'));
    assert!(scan(dir.path(), &data).unwrap().is_clean());
}

#[test]
fn a_tailed_copy_is_never_taken_for_the_deleted_original() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
    sync(dir.path(), &mut data).unwrap();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    let tail = read_id(&dir.path().join("100.A/PJ.002")).unwrap();
    fs::remove_dir_all(dir.path().join("100.A/PJ.001")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
    assert!(!result.changed && result.copies.is_empty());
    assert_eq!(get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap().get("status").and_then(Value::as_str), Some("hold"));
    assert_eq!(uid(&data, &["100.A", "PJ.002"]), Some(tail));
}

#[test]
fn a_moved_tailed_copy_is_followed_by_its_own_id() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    copy_footprint(&dir, "100.A/PJ.001", "100.A/PJ.002");
    sync(dir.path(), &mut data).unwrap();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    get_node_mut(&mut data, &rel(&["100.A", "PJ.002"])).unwrap().insert("status".into(), "done".into());
    fs::rename(dir.path().join("100.A/PJ.002"), dir.path().join("100.A/PJ.008")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    assert_eq!(get_node(&data, &rel(&["100.A", "PJ.008"])).unwrap().get("status").and_then(Value::as_str), Some("done"));
    assert_eq!(get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap().get("status").and_then(Value::as_str), Some("hold"));
}

#[test]
fn a_case_only_rename_migrates_the_entry() {
    let (dir, mut data) = storage();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    fs::rename(dir.path().join("100.A/PJ.001"), dir.path().join("100.A/pj.001")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    let moved = get_node(&data, &rel(&["100.A", "pj.001"])).unwrap();
    assert_eq!(moved.get("status").and_then(Value::as_str), Some("hold"));
}

#[test]
fn a_renamed_parent_and_child_keep_the_child_entry() {
    let dir = TempDir::new("footprint-subtree");
    dir.mkdir("100.A/.ant");
    dir.mkdir("100.A/PJ.001/.ant");
    let mut data: Mapping = serde_yaml::from_str("100.A:
  type: NODE-L1
  PJ.001: {type: WKB-L2, status: hold}
").unwrap();
    sync(dir.path(), &mut data).unwrap();
    plan(dir.path(), &mut data, Copies::Tail).unwrap();
    fs::rename(dir.path().join("100.A"), dir.path().join("200.B")).unwrap();
    fs::rename(dir.path().join("200.B/PJ.001"), dir.path().join("200.B/PJ.002")).unwrap();
    sync(dir.path(), &mut data).unwrap();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    let child = get_node(&data, &rel(&["200.B", "PJ.002"])).unwrap();
    assert_eq!(child.get("status").and_then(Value::as_str), Some("hold"));
    assert!(get_node(&data, &rel(&["100.A"])).is_none());
}

#[test]
fn plan_writes_the_uid_into_the_tree() {
    let (dir, mut data) = storage();
    assert!(plan(dir.path(), &mut data, Copies::Tail).unwrap().changed);
    let node = get_node(&data, &rel(&["100.A", "PJ.001"])).unwrap();
    assert_eq!(node.get("uid").and_then(Value::as_str), read_id(&dir.path().join("100.A/PJ.001")).as_deref());
}

#[test]
fn a_folder_that_cannot_take_its_marker_is_reported_and_the_others_are_marked() {
    let dir = TempDir::new("footprint-marker-failure");
    dir.mkdir("100.A/02.Bio");
    let paths = ["100.A/missing".to_string(), "100.A/02.Bio".to_string()];
    let warnings = create_markers(dir.path(), &paths);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("100.A"), "{warnings:?}");
    assert!(dir.path().join("100.A/02.Bio/.ant").is_dir());
}
