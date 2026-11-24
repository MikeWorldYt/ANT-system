use super::plan::IdKey;
use super::*;
use crate::test_support::*;

fn targets(pairs: &[(&str, &str)]) -> Vec<IdTarget> {
    pairs.iter().map(|(name, id)| IdTarget { name: name.to_string(), new_id: id.to_string(), new_tracker: None }).collect()
}

fn order(dir: &Path, parent: &[&str], pairs: &[(&str, &str)]) -> R<Mutation> {
    reorder_ids(dir, &rel(parent), &targets(pairs))
}

const WORKNODES: &str = "100.WORK:\n  type: NODE-L1\n  01.A: {type: NODE-L2}\n  02.B: {type: NODE-L2}\n  PJ.001_Web: {type: WKB-L2}\n  \
    PJ.1008_Big: {type: WKB-L2}\n  00.Outcomes: {type: OUT}\n  REV.Work: {type: REV-L2}\n";
const WORKNODE_DIRS: &[&str] = &["100.WORK/01.A", "100.WORK/02.B", "100.WORK/PJ.001_Web", "100.WORK/PJ.1008_Big", "100.WORK/00.Outcomes", "100.WORK/REV.Work"];

#[test]
fn a_rotation_gives_every_child_its_new_id() {
    let dir = work("reorder-rotate");
    let m = order(dir.path(), &["100.WORK"], &[("03.C", "01"), ("01.A", "02"), ("02.B", "03")]).unwrap();
    assert!(m.warnings.is_empty());
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.C", "02.A", "03.B", "04.Wkb", "98.Rev"]);
    assert_eq!(yaml_type(dir.path(), &["100.WORK", "02.A", "01.Keep"]), "NODE-L3");
    assert!(dir.path().join("100.WORK/02.A/01.Keep").is_dir());
    assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "01.A"])).is_none());
    assert!(["01.A", "02.B", "03.C", "01.C", "02.A", "03.B"].iter().all(|n| m.touched.contains(&rel(&["100.WORK", n]))));
}

#[test]
fn the_same_ids_change_nothing() {
    let dir = work("reorder-same");
    let m = order(dir.path(), &["100.WORK"], &[("01.A", "1"), ("02.B", "02"), ("03.C", "03")]).unwrap();
    assert!(m.before.is_empty());
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
}

#[test]
fn typed_ids_are_padded_and_free() {
    let dir = work("reorder-typed");
    order(dir.path(), &["100.WORK"], &[("01.A", "7"), ("02.B", "12"), ("03.C", "3")]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["03.C", "04.Wkb", "07.A", "12.B", "98.Rev"]);
}

#[test]
fn bad_ids_are_refused_before_anything_moves() {
    let dir = work("reorder-bad");
    let w = |pairs: &[(&str, &str)]| order(dir.path(), &["100.WORK"], pairs);
    let invalid = |r: R<Mutation>| matches!(r, Err(FmError::InvalidName { .. }));
    let taken = |r: R<Mutation>| matches!(r, Err(FmError::AlreadyExists { .. }));
    assert!(invalid(w(&[("01.A", "1a"), ("02.B", "02"), ("03.C", "03")])));
    assert!(invalid(w(&[("01.A", ""), ("02.B", "02"), ("03.C", "03")])));
    assert!(invalid(w(&[("01.A", "00"), ("02.B", "02"), ("03.C", "03")])));
    assert!(invalid(w(&[("01.A", "100"), ("02.B", "02"), ("03.C", "03")])));
    assert!(taken(w(&[("01.A", "2"), ("02.B", "02"), ("03.C", "03")])));
    assert!(taken(w(&[("01.A", "04"), ("02.B", "02"), ("03.C", "03")])), "04.Wkb holds 04");
    assert!(taken(w(&[("01.A", "98"), ("02.B", "02"), ("03.C", "03")])), "98.Rev holds 98");
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
}

#[test]
fn a_list_that_is_not_the_listed_children_is_refused() {
    let dir = work("reorder-mismatch");
    let stale = |r: R<Mutation>| matches!(r, Err(FmError::Stale { .. }));
    assert!(stale(order(dir.path(), &["100.WORK"], &[("02.B", "01"), ("01.A", "02")])));
    assert!(stale(order(dir.path(), &["100.WORK"], &[("02.B", "01"), ("01.A", "02"), ("01.A", "03")])));
    assert!(stale(order(dir.path(), &["100.WORK"], &[("04.Wkb", "05"), ("02.B", "01"), ("01.A", "02"), ("03.C", "03")])));
    assert!(stale(order(dir.path(), &["100.WORK"], &[("98.Rev", "05"), ("02.B", "01"), ("01.A", "02")])));
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
}

#[test]
fn a_hidden_folder_is_left_out_and_its_id_stays_taken() {
    let dir = storage(
        "reorder-hidden",
        &["100.WORK/01.A", "100.WORK/02.B", "100.WORK/03.C"],
        "100.WORK:\n  type: NODE-L1\n  01.A: {type: NODE-L2}\n  02.B: {type: NODE-L2, flags: hidden}\n  03.C: {type: NODE-L2}\n",
    );
    let plan = reorder_plan(dir.path(), &rel(&["100.WORK"])).unwrap();
    assert_eq!(plan.nodes, ["01.A", "03.C"]);
    assert!(plan.taken.contains(&IdKey { tracker: String::new(), number: 2 }));
    assert!(matches!(order(dir.path(), &["100.WORK"], &[("03.C", "02"), ("01.A", "03")]), Err(FmError::AlreadyExists { .. })));
    order(dir.path(), &["100.WORK"], &[("03.C", "01"), ("01.A", "03")]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.C", "02.B", "03.A"]);
}

#[test]
fn worknodes_take_new_numbers_with_their_tracker_and_title() {
    let dir = storage("reorder-wkb", WORKNODE_DIRS, WORKNODES);
    let plan = reorder_plan(dir.path(), &rel(&["100.WORK"])).unwrap();
    assert_eq!(plan.nodes, ["01.A", "02.B"]);
    assert_eq!(plan.worknodes, ["PJ.001_Web", "PJ.1008_Big"]);
    fn all<'a>(web: &'a str, big: &'a str) -> [(&'a str, &'a str); 4] {
        [("01.A", "01"), ("02.B", "02"), ("PJ.001_Web", web), ("PJ.1008_Big", big)]
    }
    assert!(matches!(order(dir.path(), &["100.WORK"], &all("0", "1008")), Err(FmError::InvalidName { .. })));
    assert!(matches!(order(dir.path(), &["100.WORK"], &all("8", "008")), Err(FmError::AlreadyExists { .. })));
    order(dir.path(), &["100.WORK"], &all("5", "1234")).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["00.Outcomes", "01.A", "02.B", "PJ.005_Web", "PJ.1234_Big", "REV.Work"]);
    assert_eq!(yaml_type(dir.path(), &["100.WORK", "PJ.005_Web"]), "WKB-L2");
    order(dir.path(), &["100.WORK"], &[("01.A", "01"), ("02.B", "02"), ("PJ.005_Web", "1008"), ("PJ.1234_Big", "1")]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["00.Outcomes", "01.A", "02.B", "PJ.001_Big", "PJ.1008_Web", "REV.Work"]);
}

#[test]
fn a_worknode_takes_its_linked_review_folder_along() {
    let dir = storage("reorder-wkb-linked", &["100.WORK/PJ.001_Web/REV.001_Web", "100.WORK/PJ.002_App"], "100.WORK:\n  type: NODE-L1\n  PJ.001_Web: {type: WKB-L2}\n  PJ.002_App: {type: WKB-L2}\n");
    order(dir.path(), &["100.WORK"], &[("PJ.001_Web", "2"), ("PJ.002_App", "1")]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK/PJ.002_Web")), ["REV.002_Web"]);
    assert!(dir.path().join("100.WORK/PJ.001_App").is_dir());
}

fn retrack(name: &str, id: &str, tracker: &str) -> IdTarget {
    IdTarget { name: name.to_string(), new_id: id.to_string(), new_tracker: Some(tracker.to_string()) }
}

#[test]
fn a_worknode_changes_its_tracker_and_keeps_its_linked_folder() {
    let dir = storage("reorder-tracker", &["100.WORK/PJ.001_Web/REV.001_Web", "100.WORK/HW.001_Essay"], "100.WORK:
  type: NODE-L1
  PJ.001_Web: {type: WKB-L2}
  HW.001_Essay: {type: WKB-L2}
");
    let essay = IdTarget { name: "HW.001_Essay".to_string(), new_id: "1".to_string(), new_tracker: None };
    let clash = reorder_ids(dir.path(), &rel(&["100.WORK"]), &[retrack("PJ.001_Web", "1", "HW."), essay.clone()]);
    assert!(matches!(clash, Err(FmError::AlreadyExists { .. })), "HW.001 is held");
    let unknown = reorder_ids(dir.path(), &rel(&["100.WORK"]), &[retrack("PJ.001_Web", "1", "XX."), essay.clone()]);
    assert!(matches!(unknown, Err(FmError::InvalidName { .. })));
    reorder_ids(dir.path(), &rel(&["100.WORK"]), &[retrack("PJ.001_Web", "1", "TK."), essay.clone()]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["HW.001_Essay", "TK.001_Web"]);
    assert_eq!(names(&dir.path().join("100.WORK/TK.001_Web")), ["REV.001_Web"]);
    assert_eq!(yaml_type(dir.path(), &["100.WORK", "TK.001_Web"]), "WKB-L2");
}

#[test]
fn a_worknode_changes_tracker_and_number_together() {
    let dir = storage("reorder-tracker-number", &["100.WORK/PJ.001_Web/REV.001_Web", "100.WORK/01.A"], "100.WORK:
  type: NODE-L1
  01.A: {type: NODE-L2}
  PJ.001_Web: {type: WKB-L2}
");
    let node = IdTarget { name: "01.A".to_string(), new_id: "01".to_string(), new_tracker: Some("PJ.".to_string()) };
    let refused = reorder_ids(dir.path(), &rel(&["100.WORK"]), &[node, retrack("PJ.001_Web", "3", "HW.")]);
    assert!(matches!(refused, Err(FmError::InvalidName { .. })), "a node takes no tracker");
    let node = IdTarget { name: "01.A".to_string(), new_id: "01".to_string(), new_tracker: None };
    reorder_ids(dir.path(), &rel(&["100.WORK"]), &[node, retrack("PJ.001_Web", "3", "HW.")]).unwrap();
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "HW.003_Web"]);
    assert_eq!(names(&dir.path().join("100.WORK/HW.003_Web")), ["REV.003_Web"]);
}

#[test]
fn root_categories_take_main_category_ids_only() {
    let dir = storage(
        "reorder-root",
        &["100.WORK/01.A", "300.HOME", "500.PLAY"],
        "100.WORK:\n  type: NODE-L1\n  01.A: {type: NODE-L2}\n300.HOME: {type: NODE-L1}\n500.PLAY: {type: NODE-L1}\n",
    );
    assert!(matches!(order(dir.path(), &[], &[("500.PLAY", "150"), ("100.WORK", "300"), ("300.HOME", "500")]), Err(FmError::InvalidName { .. })));
    order(dir.path(), &[], &[("500.PLAY", "100"), ("100.WORK", "300"), ("300.HOME", "500")]).unwrap();
    assert!(["100.PLAY", "300.WORK", "500.HOME"].iter().all(|n| dir.path().join(n).is_dir()));
    assert!(["100.WORK", "300.HOME", "500.PLAY"].iter().all(|n| !dir.path().join(n).exists()));
    assert_eq!(yaml_type(dir.path(), &["300.WORK", "01.A"]), "NODE-L2");
}

#[cfg(windows)]
#[test]
fn a_failed_save_puts_every_folder_back() {
    let dir = work("reorder-yaml");
    let _held = break_tree_save(dir.path());
    let err = order(dir.path(), &["100.WORK"], &[("03.C", "01"), ("01.A", "02"), ("02.B", "03")]).unwrap_err();
    assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
    assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
}
