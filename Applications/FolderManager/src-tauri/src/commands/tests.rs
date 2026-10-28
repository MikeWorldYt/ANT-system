// Command-layer flows checked without a running app: index rebuilds, events, locks and the recycle hand-off.
use std::cell::RefCell;

use serde_yaml::Mapping;

use super::edit::{delete_flow, with_index, with_search_index};
use super::tree::refresh_flow;
use super::update::{skip_path, skipped_for};
use super::*;
use crate::ant_core::search::parse_query;
use crate::config::{EditableType, NewKind, WkbStatus, TRASH_DIR};
use crate::mutations::{self, recycle_staged_with, MoveDirection, Mutation};
use crate::fs_safe::StorageLock;
use crate::index::reindex;
use crate::storage::db::Store;
use crate::test_support::{rel, storage, yaml};
use crate::tree::tree_store::get_node;
use crate::test_support::TempDir;

const TREE: &str = "100.WORK:\n  type: NODE-L1\n  01.A: {type: NODE-L2}\n  02.B: {type: NODE-L2}\n  03.Ghost: {type: NODE-L2}\n";

fn work(tag: &str) -> TempDir {
    storage(tag, &["100.WORK/01.A", "100.WORK/02.B"], TREE)
}

struct Probe {
    events: RefCell<Vec<ReviewIndexEvent>>,
    seen_yaml: RefCell<Vec<Mapping>>,
}

impl Probe {
    fn new() -> Self {
        Self { events: RefCell::new(Vec::new()), seen_yaml: RefCell::new(Vec::new()) }
    }

    fn emit(&self) -> impl Fn(&ReviewIndexEvent) -> Result<(), String> + '_ {
        |e| {
            self.events.borrow_mut().push(e.clone());
            Ok(())
        }
    }

    fn rebuild(&self, failures: &[&str]) -> impl FnOnce(&Path, &Mapping) -> Vec<String> + '_ {
        let failures: Vec<String> = failures.iter().map(|s| s.to_string()).collect();
        move |_, data| {
            self.seen_yaml.borrow_mut().push(data.clone());
            failures
        }
    }

    fn phases(&self) -> Vec<&'static str> {
        self.events.borrow().iter().map(|e| e.phase).collect()
    }
}

fn ok_recycle(staged: &Path) -> Result<(), String> {
    std::fs::remove_dir_all(staged).map_err(|e| e.to_string())
}

fn delete(
    storage: &Storage,
    probe: &Probe,
    parts: &[&str],
    ghost: bool,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> R<Mutation> {
    let emit = probe.emit();
    delete_flow(storage, &rel(parts), (ghost, true), &emit, probe.rebuild(&[]), |p| recycle_staged_with(p, recycle))
}

#[test]
fn rename_rebuilds_with_the_new_yaml_and_merges_failures_into_warnings() {
    let dir = work("cmd-rename");
    let probe = Probe::new();
    let result = mutations::rename_node(dir.path(), &rel(&["100.WORK", "01.A"]), "Alpha");
    let emit = probe.emit();
    let m = with_index(dir.path(), result, &emit, probe.rebuild(&["no powershell"])).unwrap();
    assert_eq!(m.warnings, ["Review index: no powershell"]);
    assert_eq!(m.select, Some(rel(&["100.WORK", "01.Alpha"])));
    assert_eq!(probe.phases(), ["started", "done"]);
    assert_eq!(probe.events.borrow()[1].failures, ["no powershell"]);
    let seen = probe.seen_yaml.borrow();
    assert!(get_node(&seen[0], &rel(&["100.WORK", "01.Alpha"])).is_some());
    assert!(get_node(&seen[0], &rel(&["100.WORK", "01.A"])).is_none());
}

#[test]
fn a_refused_mutation_neither_rebuilds_nor_emits() {
    let dir = work("cmd-refused");
    let probe = Probe::new();
    let result = mutations::rename_node(dir.path(), &rel(&["100.WORK", "03.Ghost"]), "X");
    let emit = probe.emit();
    let err = with_index(dir.path(), result, &emit, probe.rebuild(&[])).unwrap_err();
    assert!(matches!(err, FmError::GhostNotAllowed { .. }));
    assert!(probe.events.borrow().is_empty() && probe.seen_yaml.borrow().is_empty());
}

#[test]
fn move_and_add_node_rebuild_too() {
    let dir = work("cmd-move-add");
    let probe = Probe::new();
    let emit = probe.emit();
    let moved = mutations::move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down);
    with_index(dir.path(), moved, &emit, probe.rebuild(&[])).unwrap();
    let added = mutations::add_node(dir.path(), &rel(&["100.WORK"]), NewKind::Node);
    let m = with_index(dir.path(), added, &emit, probe.rebuild(&[])).unwrap();
    assert!(m.warnings.is_empty());
    assert_eq!(probe.phases(), ["started", "done", "started", "done"]);
    let seen = probe.seen_yaml.borrow();
    assert!(get_node(&seen[1], &rel(&["100.WORK", "04.New-Node", "98.New-Node"])).is_some());
}

#[test]
fn a_failed_event_and_an_unreadable_db_become_warnings() {
    let dir = work("cmd-emit-fail");
    let boom = |_: &ReviewIndexEvent| Err::<(), _>("window closed".to_string());
    let m = mutations::set_node_type(dir.path(), &rel(&["100.WORK", "02.B"]), EditableType::Portf, None);
    let m = with_index(dir.path(), m, &boom, |_, _| Vec::new()).unwrap();
    assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
    assert!(m.warnings[0].contains("window closed"));

    std::fs::write(crate::storage::db::db_path(dir.path()), "not a database, just text that sqlite cannot open").unwrap();
    let probe = Probe::new();
    let emit = probe.emit();
    let failures = reindex(dir.path(), &emit, |_, _| panic!("no yaml, no rebuild"));
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(probe.phases(), ["started", "done"]);
}

#[test]
fn delete_rebuilds_before_the_recycle_and_releases_the_lock_for_it() {
    let dir = work("cmd-delete");
    let storage = Storage::resolve(dir.path().to_path_buf());
    let probe = Probe::new();
    let emit = probe.emit();
    let rel_b = rel(&["100.WORK", "02.B"]);
    let flow = delete_flow(&storage, &rel_b, (false, false), &emit, probe.rebuild(&["stub"]), |p| {
        assert_eq!(probe.phases(), ["started", "done"]);
        assert!(storage.lock.try_lock().is_ok(), "mutex still held during recycle");
        assert!(StorageLock::acquire(dir.path()).is_ok(), "lock file still held during recycle");
        recycle_staged_with(p, |_| Err("bin unavailable".to_string()))
    });
    let m = flow.unwrap();
    assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
    assert_eq!(m.warnings[0], "Review index: stub");
    assert!(m.warnings[1].contains("bin unavailable"));
    assert!(get_node(&probe.seen_yaml.borrow()[0], &rel_b).is_none());
    assert_eq!(std::fs::read_dir(dir.path().join(TRASH_DIR)).unwrap().count(), 1);
}

#[test]
fn delete_recycles_cleanly_and_ghost_deletes_skip_the_recycle() {
    let dir = work("cmd-delete-ok");
    let storage = Storage::resolve(dir.path().to_path_buf());
    let probe = Probe::new();
    let m = delete(&storage, &probe, &["100.WORK", "02.B"], false, ok_recycle).unwrap();
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert!(!dir.path().join(TRASH_DIR).exists());
    let ghost = delete(&storage, &probe, &["100.WORK", "03.Ghost"], true, |_| panic!("no disk delete")).unwrap();
    assert!(ghost.warnings.is_empty());
    assert_eq!(probe.phases().len(), 4);
    let stale = delete(&storage, &probe, &["100.WORK", "01.A"], true, ok_recycle);
    assert!(matches!(stale, Err(FmError::Stale { .. })));
    assert_eq!(probe.phases().len(), 4);
}

#[test]
fn commands_release_both_locks_and_fail_without_a_storage() {
    let dir = work("cmd-locks");
    let storage = Storage::resolve(dir.path().to_path_buf());
    locked(&storage, |root| mutations::add_node(root, &rel(&["100.WORK"]), NewKind::Node)).unwrap();
    locked(&storage, |root| mutations::rename_node(root, &rel(&["100.WORK", "02.B"]), "Bee")).unwrap();
    assert!(storage.lock.try_lock().is_ok());
    assert!(StorageLock::acquire(dir.path()).is_ok());
    let none = Storage { start: RwLock::new(PathBuf::from("nowhere")), root: RwLock::new(None), lock: Mutex::new(()) };
    assert!(matches!(locked(&none, |_| Ok(())), Err(FmError::StorageNotFound { .. })));
}

#[test]
fn refresh_syncs_new_folders_then_rebuilds_the_index() {
    let dir = work("cmd-refresh");
    dir.mkdir("100.WORK/05.Fresh");
    let probe = Probe::new();
    let emit = probe.emit();
    let tree = refresh_flow(dir.path(), &emit, probe.rebuild(&["stub"])).unwrap();
    assert!(tree.yaml_changed);
    assert_eq!(probe.phases(), ["started", "done"]);
    assert_eq!(probe.events.borrow()[1].failures, ["stub"]);
    assert!(get_node(&probe.seen_yaml.borrow()[0], &rel(&["100.WORK", "05.Fresh"])).is_some());
    assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "05.Fresh"])).is_some());
}

#[test]
fn details_and_type_edits_reach_the_search_index() {
    let dir = work("cmd-search");
    let p = rel(&["100.WORK", "01.A"]);
    let search = |q: &str| Store::open_or_create(dir.path()).unwrap().search(&parse_query(q)).unwrap();
    with_search_index(dir.path(), mutations::set_node_type(dir.path(), &p, EditableType::Wkb, None)).unwrap();
    assert_eq!(search("type:WKB-L2"), [p.clone()]);
    let m = with_search_index(dir.path(), mutations::set_details(dir.path(), &p, None, Some("Zed"))).unwrap();
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    assert_eq!(search("client:Zed"), [p]);
}

#[test]
fn a_manual_update_check_ignores_the_skipped_version() {
    let dir = work("cmd-skip");
    std::fs::write(skip_path(dir.path()), " 9.9.9 \n").unwrap();
    assert_eq!(skipped_for(dir.path(), false).unwrap(), "9.9.9");
    assert_eq!(skipped_for(dir.path(), true).unwrap(), "");
}

#[test]
fn command_arguments_deserialize_from_the_frontend_values() {
    assert_eq!(serde_yaml::from_str::<MoveDirection>("up").unwrap(), MoveDirection::Up);
    assert_eq!(serde_yaml::from_str::<EditableType>("WKB").unwrap(), EditableType::Wkb);
    assert_eq!(serde_yaml::from_str::<WkbStatus>("for-review").unwrap(), WkbStatus::ForReview);
}
