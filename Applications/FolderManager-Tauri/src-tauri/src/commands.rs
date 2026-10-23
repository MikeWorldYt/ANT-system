// Tauri command layer: runs the pure core under the storage lock off the UI thread.
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use serde_yaml::Mapping;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{EditableType, WkbStatus, CORE_FOLDER, MAX_DEPTH, SETTINGS_FOLDER};
use crate::edit_actions::{self, Mutation};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, validate_component, StorageLock};
use crate::move_delete::{self, DeletePreview, MoveDirection, PendingRecycle};
use crate::review_index;
use crate::tree_node::{build_children, build_tree, TreeNode, TreeSnapshot};
use crate::db::Store;
use crate::search::parse_query;
use crate::search_index;
use crate::yaml_store::{self, yaml_path};

const SKIP_FILE: &str = ".skip_version";
const REVIEW_EVENT: &str = "fm://review-index";

pub struct Storage {
    pub start: PathBuf,
    pub root: Option<PathBuf>,
    pub lock: Mutex<()>,
}

impl Storage {
    pub fn resolve(start: PathBuf) -> Self {
        let root = crate::names::find_root(&start);
        Self { start, root, lock: Mutex::new(()) }
    }

    pub fn root(&self) -> R<&Path> {
        self.root
            .as_deref()
            .ok_or_else(|| FmError::StorageNotFound { start: self.start.display().to_string() })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    root: String,
    yaml_path: String,
    app_version: String,
    max_depth: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewIndexEvent {
    phase: &'static str,
    failures: Vec<String>,
}

type Emit<'a> = &'a dyn Fn(&ReviewIndexEvent) -> Result<(), String>;

pub fn locked<T>(storage: &Storage, f: impl FnOnce(&Path) -> R<T>) -> R<T> {
    let root = storage.root()?;
    let _guard = storage.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(root)
}

// The closure runs with the storage lock held and receives the resolved root.
pub async fn run_locked<T, F>(app: AppHandle, f: F) -> R<T>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> R<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || locked(&app.state::<Storage>(), f))
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}

pub fn snapshot(root: &Path) -> R<TreeSnapshot> {
    let mut data = yaml_store::load(root)?;
    let mut changed = yaml_store::sync(root, &mut data)?;
    if changed {
        // Reloading under the file lock keeps this save from overwriting another process's edit.
        let _lock = StorageLock::acquire(root)?;
        data = yaml_store::load(root)?;
        changed = yaml_store::sync(root, &mut data)?;
        if changed {
            yaml_store::save(root, &data)?;
        }
    }
    let mut tree = build_tree(root, &data, changed)?;
    fill_children(root, &data, &mut tree.nodes)?;
    search_index::rebuild(root, &data)?;
    Ok(tree)
}

fn fill_children(root: &Path, data: &serde_yaml::Mapping, nodes: &mut [TreeNode]) -> R<()> {
    for node in nodes {
        if node.has_children && node.depth + 1 < MAX_DEPTH {
            let mut kids = build_children(root, data, &node.rel_parts)?;
            fill_children(root, data, &mut kids)?;
            node.children = Some(kids);
        }
    }
    Ok(())
}

fn review_emitter(app: &AppHandle) -> impl Fn(&ReviewIndexEvent) -> Result<(), String> + '_ {
    move |event| app.emit(REVIEW_EVENT, event).map_err(|e| e.to_string())
}

// Failures are returned instead of raised: the mutation behind them is already committed.
fn reindex(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> Vec<String> {
    let mut notes = Vec::new();
    let mut send = |phase, failures: Vec<String>| {
        if let Err(why) = emit(&ReviewIndexEvent { phase, failures }) {
            notes.push(format!("Could not report the review index progress: {why}"));
        }
    };
    send("started", Vec::new());
    let failures = match yaml_store::load(root) {
        Ok(data) => rebuild(root, &data),
        Err(e) => vec![e.to_string()],
    };
    send("done", failures.clone());
    let mut all = failures;
    all.extend(notes);
    all
}

fn with_index(
    root: &Path,
    result: R<Mutation>,
    emit: Emit,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> R<Mutation> {
    let mut m = result?;
    m.warnings.extend(reindex(root, emit, rebuild).into_iter().map(|f| format!("Review index: {f}")));
    let indexed = yaml_store::load(root).and_then(|data| search_index::rebuild(root, &data));
    if let Err(e) = indexed {
        m.warnings.push(format!("Search index: {e}"));
    }
    Ok(m)
}

fn refresh_flow(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> R<TreeSnapshot> {
    let tree = snapshot(root)?;
    reindex(root, emit, rebuild);
    Ok(tree)
}

// The recycle runs after the storage lock is released so a slow Recycle Bin never blocks other commands.
fn delete_flow(
    storage: &Storage,
    rel: &[String],
    flags: (bool, bool),
    emit: Emit,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
    recycle: impl FnOnce(PendingRecycle) -> Vec<String>,
) -> R<Mutation> {
    let (mut m, pending) = locked(storage, |root| {
        let (m, pending) = move_delete::delete_node(root, rel, flags.0, flags.1)?;
        Ok((with_index(root, Ok(m), emit, rebuild)?, pending))
    })?;
    if let Some(pending) = pending {
        m.warnings.extend(recycle(pending));
    }
    Ok(m)
}

fn skip_path(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER).join(SKIP_FILE)
}

#[tauri::command]
pub async fn get_storage(app: AppHandle) -> R<StorageInfo> {
    run_locked(app, |root| {
        Ok(StorageInfo {
            root: root.to_string_lossy().into_owned(),
            yaml_path: yaml_path(root).to_string_lossy().into_owned(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            max_depth: MAX_DEPTH,
        })
    })
    .await
}

#[tauri::command]
pub async fn load_tree(app: AppHandle) -> R<TreeSnapshot> {
    run_locked(app, snapshot).await
}

#[tauri::command]
pub async fn refresh(app: AppHandle) -> R<TreeSnapshot> {
    let emitter = app.clone();
    run_locked(app, move |root| refresh_flow(root, &review_emitter(&emitter), review_index::rebuild)).await
}

#[tauri::command]
pub async fn list_children(app: AppHandle, rel_parts: Vec<String>) -> R<Vec<TreeNode>> {
    run_locked(app, move |root| {
        rel_parts.iter().try_for_each(|part| validate_component(part))?;
        let data = yaml_store::load(root)?;
        let mut kids = build_children(root, &data, &rel_parts)?;
        fill_children(root, &data, &mut kids)?;
        Ok(kids)
    })
    .await
}

#[tauri::command]
pub async fn search_nodes(app: AppHandle, query: String) -> R<Vec<Vec<String>>> {
    run_locked(app, move |root| {
        let store = Store::open_or_create(root)?;
        store.search(&parse_query(&query))
    })
    .await
}

#[tauri::command]
pub async fn rename_node(app: AppHandle, rel_parts: Vec<String>, new_editable: String) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = edit_actions::rename_node(root, &rel_parts, &new_editable);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn set_node_type(
    app: AppHandle,
    rel_parts: Vec<String>,
    type_base: EditableType,
    status: Option<WkbStatus>,
) -> R<Mutation> {
    run_locked(app, move |root| edit_actions::set_node_type(root, &rel_parts, type_base, status)).await
}

#[tauri::command]
pub async fn add_child(app: AppHandle, parent_rel_parts: Vec<String>) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = edit_actions::add_child(root, &parent_rel_parts);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn move_node(app: AppHandle, rel_parts: Vec<String>, direction: MoveDirection) -> R<Mutation> {
    let emitter = app.clone();
    run_locked(app, move |root| {
        let result = move_delete::move_node(root, &rel_parts, direction);
        with_index(root, result, &review_emitter(&emitter), review_index::rebuild)
    })
    .await
}

#[tauri::command]
pub async fn inspect_delete(app: AppHandle, rel_parts: Vec<String>) -> R<DeletePreview> {
    run_locked(app, move |root| move_delete::inspect_delete(root, &rel_parts)).await
}

#[tauri::command]
pub async fn delete_node(
    app: AppHandle,
    rel_parts: Vec<String>,
    expect_ghost: bool,
    confirm_non_empty: bool,
) -> R<Mutation> {
    tauri::async_runtime::spawn_blocking(move || {
        let flags = (expect_ghost, confirm_non_empty);
        let emit = review_emitter(&app);
        let storage = app.state::<Storage>();
        delete_flow(&storage, &rel_parts, flags, &emit, review_index::rebuild, move_delete::recycle_staged)
    })
    .await
    .map_err(|e| FmError::Internal { message: e.to_string() })?
}

#[tauri::command]
pub async fn get_skipped_version(app: AppHandle) -> R<String> {
    run_locked(app, |root| {
        let path = skip_path(root);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text.trim().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(io_err("read skipped version", &path, &e)),
        }
    })
    .await
}

#[tauri::command]
pub async fn skip_version(app: AppHandle, version: String) -> R<()> {
    run_locked(app, move |root| {
        let path = skip_path(root);
        std::fs::write(&path, version.trim()).map_err(|e| io_err("save skipped version", &path, &e))
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::config::TRASH_DIR;
    use crate::edit_actions::test_support::{rel, storage, yaml};
    use crate::move_delete::recycle_staged_with;
    use crate::yaml_store::get_node;
    use crate::yaml_store::test_support::TempDir;

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
        let result = edit_actions::rename_node(dir.path(), &rel(&["100.WORK", "01.A"]), "Alpha");
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
        let result = edit_actions::rename_node(dir.path(), &rel(&["100.WORK", "03.Ghost"]), "X");
        let emit = probe.emit();
        let err = with_index(dir.path(), result, &emit, probe.rebuild(&[])).unwrap_err();
        assert!(matches!(err, FmError::GhostNotAllowed { .. }));
        assert!(probe.events.borrow().is_empty() && probe.seen_yaml.borrow().is_empty());
    }

    #[test]
    fn move_and_add_child_rebuild_too() {
        let dir = work("cmd-move-add");
        let probe = Probe::new();
        let emit = probe.emit();
        let moved = move_delete::move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down);
        with_index(dir.path(), moved, &emit, probe.rebuild(&[])).unwrap();
        let added = edit_actions::add_child(dir.path(), &rel(&["100.WORK"]));
        let m = with_index(dir.path(), added, &emit, probe.rebuild(&[])).unwrap();
        assert!(m.warnings.is_empty());
        assert_eq!(probe.phases(), ["started", "done", "started", "done"]);
        let seen = probe.seen_yaml.borrow();
        assert!(get_node(&seen[1], &rel(&["100.WORK", "04.New-Node", "98.New-Node"])).is_some());
    }

    #[test]
    fn a_failed_event_and_an_unreadable_yaml_become_warnings() {
        let dir = work("cmd-emit-fail");
        let boom = |_: &ReviewIndexEvent| Err::<(), _>("window closed".to_string());
        let m = edit_actions::set_node_type(dir.path(), &rel(&["100.WORK", "02.B"]), EditableType::Portf, None);
        let m = with_index(dir.path(), m, &boom, |_, _| Vec::new()).unwrap();
        assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
        assert!(m.warnings[0].contains("window closed"));

        std::fs::write(yaml_path(dir.path()), "100.WORK: [unclosed").unwrap();
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
        locked(&storage, |root| edit_actions::add_child(root, &rel(&["100.WORK"]))).unwrap();
        locked(&storage, |root| edit_actions::rename_node(root, &rel(&["100.WORK", "02.B"]), "Bee")).unwrap();
        assert!(storage.lock.try_lock().is_ok());
        assert!(StorageLock::acquire(dir.path()).is_ok());
        let none = Storage { start: PathBuf::from("nowhere"), root: None, lock: Mutex::new(()) };
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
    fn command_arguments_deserialize_from_the_frontend_values() {
        assert_eq!(serde_yaml::from_str::<MoveDirection>("up").unwrap(), MoveDirection::Up);
        assert_eq!(serde_yaml::from_str::<EditableType>("WKB").unwrap(), EditableType::Wkb);
        assert_eq!(serde_yaml::from_str::<WkbStatus>("for-review").unwrap(), WkbStatus::ForReview);
    }
}
