// Rename, retype and add nodes, keeping disk folders, linked 90./98. folders and the tree in step.
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::{child_entries, type_base, type_level, META_KEYS};
use crate::config::{EditableType, NewKind, WkbStatus, is_hidden_dir, DATA_DIR, LINKED_PREFIXES, MAX_DEPTH};
use crate::db::hide;
use crate::footprint;
use crate::error::{FmError, R};
use crate::info::{field_of, read_info, write_info, OWNER_KEYS};
use crate::fs_safe::{
    error_text, io_err, join_rel, resolve, same_name, validate_component, Journal, StorageLock,
};
use crate::names::{
    compose_name, editable_part, name_parts, list_subdirs, next_available_id, sanitize_train_case,
    sanitize_upper,
};
use crate::tree_node::{build_tree, is_reserved, TreeSnapshot};
use crate::tree_store::{self, get_node, get_node_mut, get_type, is_locked_type, rename_key};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub tree: TreeSnapshot,
    pub select: Option<Vec<String>>,
    pub warnings: Vec<String>,
}

pub(crate) struct Target {
    pub abs: PathBuf,
    pub ghost: bool,
    pub node_type: String,
}

pub fn rename_node(root: &Path, rel: &[String], new_editable: &str) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "rename".to_string() });
    }
    let (old_name, parent) = split(rel)?;
    let workboard = type_base(&target.node_type) == "WKB";
    let editable =
        if parent.is_empty() { sanitize_upper(new_editable) } else { sanitize_train_case(new_editable) };
    if editable.is_empty() && !workboard {
        return Err(FmError::InvalidName {
            name: new_editable.to_string(),
            reason: "name cannot be empty".to_string(),
        });
    }
    let (fixed, _) = name_parts(old_name, workboard);
    let new_name = compose_name(&fixed, &editable, workboard);
    validate_component(&new_name)?;
    let new_rel = child_rel(parent, &new_name);
    if new_name == *old_name {
        if synced {
            tree_store::save(root, &data)?;
        }
        return Ok(finish(root, &data, synced, Some(new_rel), Vec::new()));
    }
    let parent_abs = join_rel(root, parent);
    ensure_free(&parent_abs, get_node(&data, parent), &new_name, &[old_name.as_str()])?;

    let new_abs = parent_abs.join(&new_name);
    let mut journal = Journal::new();
    journal.rename(&target.abs, &new_abs)?;
    rename_key(&mut data, parent, old_name, &new_name);
    let warnings = cascade_linked(&mut journal, &mut data, &new_abs, &new_rel, old_name, &new_name);
    if let Some(node) = get_node_mut(&mut data, &new_rel) {
        if type_base(node.get("type").and_then(Value::as_str).unwrap_or("")) != "WKB" {
            node.remove("status");
        }
    }
    persist(root, &data, journal, "rename")?;
    Ok(finish(root, &data, synced, Some(new_rel), warnings))
}

pub fn set_node_type(
    root: &Path,
    rel: &[String],
    base: EditableType,
    status: Option<WkbStatus>,
) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    let base_name = match base {
        EditableType::Node => "NODE",
        EditableType::Wkb => "WKB",
        EditableType::Portf => "PORTF",
    };
    let new_type = match type_level(&target.node_type) {
        Some(level) => format!("{base_name}-{level}"),
        None => base_name.to_string(),
    };
    let node = get_node_mut(&mut data, rel)
        .ok_or_else(|| FmError::NotFound { rel_parts: rel.to_vec() })?;
    node.insert("type".into(), new_type.into());
    match (base, status) {
        (EditableType::Wkb, Some(s)) => {
            node.insert("status".into(), status_text(s).into());
        }
        (EditableType::Wkb, None) => {}
        _ => {
            node.remove("status");
        }
    }
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new()))
}

// Status and owner live in the folder's info.json; a value that is missing there is written as "unknown".
pub fn set_details(root: &Path, rel: &[String], status: Option<WkbStatus>, owner: Option<&str>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "edit".to_string() });
    }
    if type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards have status and owner".to_string() });
    }
    let current = read_info(&target.abs);
    let kept = |keys: &[&str]| current.as_ref().and_then(|i| field_of(i, keys)).map(str::to_string);
    let yaml_status = get_node(&data, rel).and_then(|n| n.get("status")).and_then(Value::as_str).map(str::to_string);
    let status = status
        .map(|s| status_text(s).to_string())
        .or_else(|| kept(&["status"]))
        .or(yaml_status)
        .unwrap_or_else(|| "unknown".to_string());
    let owner = match owner.map(str::trim) {
        Some(o) if !o.is_empty() => o.chars().filter(|c| !c.is_control()).take(100).collect(),
        _ => kept(&OWNER_KEYS).unwrap_or_else(|| "unknown".to_string()),
    };
    write_info(&target.abs, &[(&["status"], "status", Some(status.into())), (&OWNER_KEYS, "owner", Some(owner.into()))])?;
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), Vec::new()))
}

// Level 1 has its own creation rules, so the new folder always goes next to a level 2 or deeper one.
pub fn add_next(root: &Path, rel: &[String], kind: NewKind) -> R<Mutation> {
    match rel.split_last() {
        Some((_, parent)) if !parent.is_empty() => add_child(root, parent, kind),
        _ => Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "level 1 folders are not created from the tree".to_string() }),
    }
}

pub fn add_child(root: &Path, parent: &[String], kind: NewKind) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, parent)?;
    guard_frozen(parent, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "add child".to_string() });
    }
    let child_depth = parent.len();
    if child_depth >= MAX_DEPTH {
        return Err(FmError::DepthLimit);
    }
    let parent_yaml =
        get_node(&data, parent).ok_or_else(|| FmError::NotFound { rel_parts: parent.to_vec() })?;
    // Ghost ids count as taken so a new node never inherits a missing folder's record.
    let mut taken = list_subdirs(&target.abs)?;
    taken.extend(child_entries(parent_yaml).map(|(k, _)| k.to_string()));
    let workboard = kind == NewKind::Workboard;
    let new_name = if workboard {
        format!("PJ.{:03}_New-Workboard", next_tracker(&data))
    } else {
        format!("{:02}.New-Node", next_available_id(&taken))
    };
    ensure_free(&target.abs, Some(parent_yaml), &new_name, &[])?;

    let new_abs = target.abs.join(&new_name);
    let mut journal = Journal::new();
    journal.mkdir(&new_abs)?;
    let level = child_depth + 1;
    let mut entry = Mapping::new();
    entry.insert("type".into(), format!("{}-L{level}", if workboard { "WKB" } else { "NODE" }).into());
    let mut warnings = Vec::new();
    let rev_name = format!("98.{}", editable_part(&new_name));
    // A workboard keeps its review shortcuts out of its own folder.
    match if workboard { Ok(()) } else { journal.mkdir(&new_abs.join(&rev_name)) } {
        // The deepest level keeps its 98 folder on disk only, as the tree stops at MAX_DEPTH.
        Ok(()) if level < MAX_DEPTH && !workboard => {
            let mut rev = Mapping::new();
            rev.insert("type".into(), format!("REV-L{level}").into());
            entry.insert(rev_name.into(), rev.into());
        }
        Ok(()) => {}
        Err(e) => warnings.push(format!("Could not create {rev_name}: {}", error_text(&e))),
    }
    match get_node_mut(&mut data, parent) {
        Some(node) => {
            node.insert(new_name.clone().into(), entry.into());
        }
        None => {
            let cause = FmError::NotFound { rel_parts: parent.to_vec() };
            return Err(journal.abort("add child", cause));
        }
    }
    persist(root, &data, journal, "add child")?;
    // The folder exists now, so a failure here is only reported.
    if let Err(e) = mark_folder(&new_abs, workboard) {
        warnings.push(format!("Could not set up {DATA_DIR}: {}", error_text(&e)));
    }
    register_ids(root, &mut data, &mut warnings);
    Ok(finish(root, &data, synced, Some(child_rel(parent, &new_name)), warnings))
}

// Folders that just got a .ant folder receive their id now instead of at the next refresh.
fn register_ids(root: &Path, data: &mut Mapping, warnings: &mut Vec<String>) {
    match footprint::plan(root, data) {
        Ok(applied) => {
            // The db keeps the old paths if this save fails, so the next run migrates again.
            let saved = if applied.changed { tree_store::save(root, data) } else { Ok(()) };
            if let Err(e) = saved.and_then(|()| applied.commit(root)) {
                warnings.push(format!("Could not save moved folders: {}", error_text(&e)));
            }
        }
        Err(e) => warnings.push(format!("Could not assign folder ids: {}", error_text(&e))),
    }
}

// A favorite is a flag in the folder's own info.json, so it travels with the folder and is searchable.
pub fn set_favorite(root: &Path, rel: &[String], on: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced(root)?;
    let target = locate(root, &data, rel)?;
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "favorite".to_string() });
    }
    write_info(&target.abs, &[(&["favorite"], "favorite", on.then_some(Value::Bool(true)))])?;
    let mut warnings = Vec::new();
    register_ids(root, &mut data, &mut warnings);
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, synced, Some(rel.to_vec()), warnings))
}

// Next free 3-digit tracker after every PJ workboard in the tree, missing folders included.
fn next_tracker(data: &Mapping) -> u32 {
    fn highest(node: &Mapping) -> u32 {
        child_entries(node)
            .map(|(name, child)| {
                let own = name.strip_prefix("PJ.").and_then(|r| r.get(..3)).and_then(|d| d.parse().ok()).unwrap_or(0);
                own.max(highest(child))
            })
            .max()
            .unwrap_or(0)
    }
    highest(data) + 1
}

// Every created folder gets a .ant folder, which is what lets its id follow it around; workboards also start their info.json.
fn mark_folder(folder: &Path, workboard: bool) -> R<()> {
    if workboard {
        return write_info(folder, &[(&["status"], "status", Some("in-progress".into())), (&OWNER_KEYS, "owner", Some("unknown".into()))]);
    }
    let dir = folder.join(DATA_DIR);
    fs::create_dir(&dir).map_err(|e| io_err("create info folder", &dir, &e))?;
    hide(&dir);
    Ok(())
}

// Linked folders follow best-effort: a clash or failure becomes a warning and its yaml key stays.
fn cascade_linked(
    journal: &mut Journal,
    data: &mut Mapping,
    node_abs: &Path,
    node_rel: &[String],
    old_name: &str,
    new_name: &str,
) -> Vec<String> {
    let (old_ed, new_ed) = (editable_part(old_name), editable_part(new_name));
    if old_ed == new_ed {
        return Vec::new();
    }
    let existing = match list_subdirs(node_abs) {
        Ok(names) => names,
        Err(e) => return vec![format!("Linked folders not renamed: {}", error_text(&e))],
    };
    let mut warnings = Vec::new();
    for prefix in LINKED_PREFIXES {
        let wanted = format!("{prefix}.{old_ed}");
        let Some(child_old) = existing.iter().find(|n| same_name(n, &wanted)) else { continue };
        let child_new = format!("{prefix}.{new_ed}");
        let renamed = ensure_free(node_abs, get_node(data, node_rel), &child_new, &[child_old.as_str()])
            .and_then(|_| journal.rename(&node_abs.join(child_old), &node_abs.join(&child_new)));
        match renamed {
            Ok(()) => {
                rename_key(data, node_rel, child_old, &child_new);
            }
            Err(e) => warnings.push(format!("Could not rename {child_old}: {}", error_text(&e))),
        }
    }
    warnings
}

fn status_text(status: WkbStatus) -> &'static str {
    match status {
        WkbStatus::InProgress => "in-progress",
        WkbStatus::ForReview => "for-review",
        WkbStatus::Hold => "hold",
        WkbStatus::Done => "done",
    }
}

// Folders added on disk since the last refresh get their yaml entry before any check runs.
pub(crate) fn load_synced(root: &Path) -> R<(Mapping, bool)> {
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync(root, &mut data)?;
    Ok((data, changed))
}

pub(crate) fn split(rel: &[String]) -> R<(&String, &[String])> {
    rel.split_last().ok_or_else(|| FmError::InvalidPath {
        rel_parts: Vec::new(),
        reason: "the storage root is not a node".to_string(),
    })
}

pub(crate) fn child_rel(parent: &[String], name: &str) -> Vec<String> {
    let mut rel = parent.to_vec();
    rel.push(name.to_string());
    rel
}

pub(crate) fn locate(root: &Path, data: &Mapping, rel: &[String]) -> R<Target> {
    split(rel)?;
    // A folder named like a meta key would make its parent's type or status look like a node.
    if let Some(meta) = rel.iter().find(|p| META_KEYS.contains(&p.as_str())) {
        return Err(FmError::InvalidName { name: meta.clone(), reason: "reserved name".to_string() });
    }
    let abs = resolve(root, rel)?;
    let mut on_disk = true;
    let mut dir = root.to_path_buf();
    for part in rel {
        if !has_exact_dir(&dir, part)? {
            on_disk = false;
            break;
        }
        dir.push(part);
    }
    if !on_disk && get_node(data, rel).is_none() {
        return Err(FmError::NotFound { rel_parts: rel.to_vec() });
    }
    Ok(Target { abs, ghost: !on_disk, node_type: get_type(data, rel) })
}

// Exact match, because Windows would also open "01.math" for a yaml key "01.Math".
fn has_exact_dir(dir: &Path, name: &str) -> R<bool> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(io_err("list folder", dir, &e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        if entry.file_name() == name {
            let kind = entry.file_type().map_err(|e| io_err("inspect path", &entry.path(), &e))?;
            return Ok(kind.is_dir());
        }
    }
    Ok(false)
}

pub(crate) fn guard_frozen(rel: &[String], node_type: &str) -> R<()> {
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if is_locked_type(node_type) {
        return Err(FmError::Locked { rel_parts: rel.to_vec(), node_type: node_type.to_string() });
    }
    Ok(())
}

pub(crate) fn ensure_free(
    parent_abs: &Path,
    parent_yaml: Option<&Mapping>,
    name: &str,
    ignore: &[&str],
) -> R<()> {
    // A folder named like a meta key would overwrite its parent's type or status in the yaml.
    if META_KEYS.iter().any(|k| same_name(k, name)) || is_hidden_dir(name) {
        return Err(FmError::InvalidName { name: name.to_string(), reason: "reserved name".to_string() });
    }
    let clashes = |other: &str| same_name(other, name) && !ignore.contains(&other);
    let taken = FmError::AlreadyExists { name: name.to_string() };
    if parent_yaml.is_some_and(|m| m.keys().filter_map(Value::as_str).any(clashes)) {
        return Err(taken);
    }
    let entries = fs::read_dir(parent_abs).map_err(|e| io_err("list folder", parent_abs, &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", parent_abs, &e))?;
        if clashes(&entry.file_name().to_string_lossy()) {
            return Err(taken);
        }
    }
    Ok(())
}

pub(crate) fn persist(root: &Path, data: &Mapping, journal: Journal, op: &str) -> R<()> {
    match tree_store::save(root, data) {
        Ok(()) => {
            journal.commit();
            Ok(())
        }
        Err(e) => Err(journal.abort(op, e)),
    }
}

// Runs after the yaml is committed, so it never fails: the change is already on disk.
pub(crate) fn finish(
    root: &Path,
    data: &Mapping,
    yaml_changed: bool,
    select: Option<Vec<String>>,
    warnings: Vec<String>,
) -> Mutation {
    after_commit(root, build_tree(root, data, yaml_changed), select, warnings)
}

// An empty tree with yamlChanged set tells the UI to refresh instead of showing a stale tree.
fn after_commit(
    root: &Path,
    tree: R<TreeSnapshot>,
    select: Option<Vec<String>>,
    mut warnings: Vec<String>,
) -> Mutation {
    let tree = tree.unwrap_or_else(|e| {
        warnings.push(format!("The change was saved but the tree could not be reloaded; refresh. {e}"));
        TreeSnapshot { root: root.to_string_lossy().into_owned(), nodes: Vec::new(), yaml_changed: true }
    });
    Mutation { tree, select, warnings }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::fs;
    use std::path::Path;

    use serde_yaml::Mapping;

    use crate::tree_store::test_support::TempDir;
    use crate::tree_store::{get_type, load, save};

    pub fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    pub fn storage(tag: &str, dirs: &[&str], yaml: &str) -> TempDir {
        let dir = TempDir::new(tag);
        dir.mkdir("999.Settings/00.Core");
        for d in dirs {
            dir.mkdir(d);
        }
        save(dir.path(), &serde_yaml::from_str::<Mapping>(yaml).unwrap()).unwrap();
        dir
    }

    pub fn names(dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        out.sort();
        out
    }

    pub fn yaml(dir: &Path) -> Mapping {
        load(dir).unwrap()
    }

    pub fn yaml_type(dir: &Path, parts: &[&str]) -> String {
        get_type(&yaml(dir), &rel(parts))
    }

    // Holding ant.db open for read only keeps loads working but makes the save fail after the disk changes.
    #[cfg(windows)]
    pub fn break_tree_save(dir: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 0x1;
        fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(crate::db::db_path(dir)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::tree_store::get_node;
    use crate::tree_store::test_support::TempDir;

    const ACADEMIC: &str = "100.ACADEMIC:\n  type: NODE-L1\n  01.Math:\n    type: NODE-L2\n    status: done\n    \
        98.Math: {type: REV-L2}\n    90.Math: {type: RES}\n    01.Algebra: {type: NODE-L3}\n  \
        02.Bio:\n    type: NODE-L2\n  02.Gone:\n    type: NODE-L2\n  03.Ghost:\n    type: NODE-L2\n";
    const ACADEMIC_DIRS: &[&str] = &[
        "100.ACADEMIC/01.Math/98.Math",
        "100.ACADEMIC/01.Math/90.Math",
        "100.ACADEMIC/01.Math/01.Algebra",
        "100.ACADEMIC/02.Bio",
    ];

    fn academic(tag: &str) -> TempDir {
        storage(tag, ACADEMIC_DIRS, ACADEMIC)
    }

    #[test]
    fn rename_moves_folder_yaml_and_linked_children() {
        let dir = academic("rename-ok");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "  linear algebra ").unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "01.Linear-algebra"])));
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        let node = dir.path().join("100.ACADEMIC/01.Linear-algebra");
        assert_eq!(names(&node), ["01.Algebra", "90.Linear-algebra", "98.Linear-algebra"]);
        assert!(!dir.path().join("100.ACADEMIC/01.Math").exists());
        let p = ["100.ACADEMIC", "01.Linear-algebra"];
        assert_eq!(yaml_type(dir.path(), &p), "NODE-L2");
        assert_eq!(yaml_type(dir.path(), &[p[0], p[1], "98.Linear-algebra"]), "REV-L2");
        assert_eq!(yaml_type(dir.path(), &[p[0], p[1], "90.Linear-algebra"]), "RES");
        assert_eq!(yaml_type(dir.path(), &[p[0], p[1], "01.Algebra"]), "NODE-L3");
        let data = yaml(dir.path());
        assert!(get_node(&data, &rel(&["100.ACADEMIC", "01.Math"])).is_none());
        assert!(get_node(&data, &rel(&p)).unwrap().get("status").is_none());
    }

    #[test]
    fn rename_at_root_level_uppercases_and_keeps_prefix() {
        let dir = academic("rename-root");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC"]), "school work").unwrap();
        assert_eq!(m.select, Some(rel(&["100.SCHOOL-WORK"])));
        assert!(dir.path().join("100.SCHOOL-WORK/01.Math").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.SCHOOL-WORK", "01.Math"]), "NODE-L2");
    }

    #[test]
    fn rename_refuses_case_insensitive_collisions_on_disk_and_in_yaml() {
        let dir = academic("rename-clash");
        dir.mkdir("100.ACADEMIC/02.math");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(matches!(rename_node(dir.path(), &p, "Math"), Err(FmError::AlreadyExists { .. })));
        assert!(matches!(rename_node(dir.path(), &p, "gone"), Err(FmError::AlreadyExists { .. })));
        assert!(dir.path().join("100.ACADEMIC/02.Bio").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Bio"]), "NODE-L2");
    }

    #[test]
    fn rename_allows_a_case_only_change() {
        let dir = storage("rename-case", &["100.ACADEMIC/01.math"], "100.ACADEMIC:\n  01.math: {type: NODE-L2}\n");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.math"]), "Math").unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "01.Math"])));
        assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math"]);
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math"]), "NODE-L2");
        assert!(get_node(&yaml(dir.path()), &rel(&["100.ACADEMIC", "01.math"])).is_none());
    }

    #[test]
    fn rename_refuses_ghosts_reserved_locked_and_bad_names() {
        let dir = academic("rename-refuse");
        let ghost = rename_node(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), "X");
        assert!(matches!(ghost, Err(FmError::GhostNotAllowed { .. })));
        dir.mkdir("998.Review");
        for reserved in ["999.Settings", "998.Review"] {
            let err = rename_node(dir.path(), &rel(&[reserved]), "X");
            assert!(matches!(err, Err(FmError::Reserved { .. })), "{reserved}: {err:?}");
        }
        let locked = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), "X");
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        let up = rename_node(dir.path(), &rel(&["100.ACADEMIC", "..", "100.ACADEMIC"]), "X");
        assert!(matches!(up, Err(FmError::InvalidPath { .. })));
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(matches!(rename_node(dir.path(), &p, ".."), Err(FmError::InvalidName { .. })));
        assert!(matches!(rename_node(dir.path(), &p, " /:* "), Err(FmError::InvalidName { .. })));
        assert!(matches!(rename_node(dir.path(), &p, "x\u{1}"), Err(FmError::InvalidName { .. })));
        let missing = rename_node(dir.path(), &rel(&["100.ACADEMIC", "07.Nope"]), "X");
        assert!(matches!(missing, Err(FmError::NotFound { .. })));
        assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math", "02.Bio"]);
    }

    #[test]
    fn rename_cascade_skips_clashing_linked_folder_with_a_warning() {
        let dir = academic("rename-cascade");
        dir.mkdir("100.ACADEMIC/01.Math/98.Physics");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "Physics").unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
        let node = dir.path().join("100.ACADEMIC/01.Physics");
        assert_eq!(names(&node), ["01.Algebra", "90.Physics", "98.Math", "98.Physics"]);
        let p = ["100.ACADEMIC", "01.Physics"];
        assert_eq!(yaml_type(dir.path(), &[p[0], p[1], "98.Math"]), "REV-L2");
        assert_eq!(yaml_type(dir.path(), &[p[0], p[1], "90.Physics"]), "RES");
    }

    #[cfg(windows)]
    #[test]
    fn rename_rolls_back_disk_and_cascade_when_yaml_save_fails() {
        let dir = academic("rename-rollback");
        let _held = break_tree_save(dir.path());
        let before = yaml(dir.path());
        let err = rename_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), "Physics").unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        let node = dir.path().join("100.ACADEMIC/01.Math");
        assert_eq!(names(&node), ["01.Algebra", "90.Math", "98.Math"]);
        assert!(!dir.path().join("100.ACADEMIC/01.Physics").exists());
        assert_eq!(yaml(dir.path()), before);
    }

    #[test]
    fn details_go_to_info_json_and_default_to_the_yaml_status_then_unknown() {
        let dir = academic("details");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        assert!(set_details(dir.path(), &p, None, Some("Ana")).is_err(), "not a workboard yet");
        set_node_type(dir.path(), &p, EditableType::Wkb, Some(WkbStatus::Hold)).unwrap();
        let m = set_details(dir.path(), &p, None, Some(" Ana ")).unwrap();
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(info.contains("\"status\": \"hold\"") && info.contains("\"owner\": \"Ana\""), "{info}");
        assert_eq!(m.select, Some(p.clone()));
        set_details(dir.path(), &p, Some(WkbStatus::Done), Some("  ")).unwrap();
        let info = std::fs::read_to_string(dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json")).unwrap();
        assert!(info.contains("\"status\": \"done\"") && info.contains("\"owner\": \"Ana\""), "{info}");
    }

    #[test]
    fn set_node_type_keeps_level_and_manages_status() {
        let dir = academic("retype");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        set_node_type(dir.path(), &p, EditableType::Wkb, Some(WkbStatus::ForReview)).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Bio"]), "WKB-L2");
        let status = |d: &Path| {
            get_node(&yaml(d), &p).unwrap().get("status").and_then(Value::as_str).map(str::to_string)
        };
        assert_eq!(status(dir.path()).as_deref(), Some("for-review"));
        set_node_type(dir.path(), &p, EditableType::Wkb, None).unwrap();
        assert_eq!(status(dir.path()).as_deref(), Some("for-review"));
        set_node_type(dir.path(), &p, EditableType::Portf, Some(WkbStatus::Done)).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "02.Bio"]), "PORTF-L2");
        assert_eq!(status(dir.path()), None);
    }

    #[test]
    fn set_node_type_allows_ghosts_but_not_locked_or_reserved() {
        let dir = academic("retype-refuse");
        let ghost = rel(&["100.ACADEMIC", "03.Ghost"]);
        set_node_type(dir.path(), &ghost, EditableType::Portf, None).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "03.Ghost"]), "PORTF-L2");
        let locked = set_node_type(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "90.Math"]), EditableType::Node, None);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        let reserved = set_node_type(dir.path(), &rel(&["999.Settings"]), EditableType::Node, None);
        assert!(matches!(reserved, Err(FmError::Reserved { .. })));
    }

    #[test]
    fn add_child_takes_lowest_free_id_and_creates_review_folder() {
        let dir = academic("add");
        let m = add_child(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.New-Node"])));
        assert!(dir.path().join("100.ACADEMIC/04.New-Node/98.New-Node").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node"]), "NODE-L2");
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node", "98.New-Node"]), "REV-L2");

        let deep = add_child(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
        let p = ["100.ACADEMIC", "01.Math", "02.New-Node"];
        assert_eq!(deep.select, Some(rel(&p)));
        assert!(dir.path().join("100.ACADEMIC/01.Math/02.New-Node/98.New-Node").is_dir());
        assert_eq!(yaml_type(dir.path(), &p), "NODE-L3");
        assert!(get_node(&yaml(dir.path()), &rel(&p)).unwrap().get("98.New-Node").is_none());
    }

    #[test]
    fn add_next_goes_beside_a_level_two_folder_and_marks_it_with_a_dot_ant() {
        let dir = academic("add-next");
        let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Node).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.New-Node"])));
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        let id = std::fs::read_to_string(dir.path().join("100.ACADEMIC/04.New-Node/.ant/dragpath.json")).unwrap();
        assert!(id.contains("\"id\""), "{id}");
        assert!(matches!(add_next(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node), Err(FmError::InvalidPath { .. })));
    }

    #[test]
    fn favorite_is_a_flag_in_info_json_that_can_be_removed_again() {
        let dir = academic("favorite");
        let p = rel(&["100.ACADEMIC", "02.Bio"]);
        set_favorite(dir.path(), &p, true).unwrap();
        let file = dir.path().join("100.ACADEMIC/02.Bio/.ant/info.json");
        assert!(std::fs::read_to_string(&file).unwrap().contains("\"favorite\": true"));
        set_favorite(dir.path(), &p, false).unwrap();
        assert!(!std::fs::read_to_string(&file).unwrap().contains("favorite"));
        assert!(matches!(set_favorite(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), true), Err(FmError::GhostNotAllowed { .. })));
    }

    #[test]
    fn a_new_workboard_takes_the_next_tracker_and_starts_its_info_json() {
        let dir = academic("add-wkb");
        let p = rel(&["100.ACADEMIC", "01.Math"]);
        let first = add_next(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Workboard).unwrap();
        assert_eq!(first.select, Some(child_rel(&p, "PJ.001_New-Workboard")));
        let second = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Workboard).unwrap();
        assert_eq!(second.select, Some(rel(&["100.ACADEMIC", "PJ.002_New-Workboard"])));
        let wkb = dir.path().join("100.ACADEMIC/01.Math/PJ.001_New-Workboard");
        assert_eq!(names(&wkb), [".ant"], "no 98 folder inside a workboard");
        let info = std::fs::read_to_string(wkb.join(".ant/info.json")).unwrap();
        assert!(info.contains("\"status\": \"in-progress\"") && info.contains("\"owner\": \"unknown\""), "{info}");
        assert!(wkb.join(".ant/dragpath.json").is_file());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "PJ.001_New-Workboard"]), "WKB-L3");
    }

    #[test]
    fn add_child_refuses_depth_ghost_locked_and_reserved_parents() {
        let dir = academic("add-refuse");
        let too_deep = add_child(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Node);
        assert!(matches!(too_deep, Err(FmError::DepthLimit)));
        let ghost = add_child(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), NewKind::Node);
        assert!(matches!(ghost, Err(FmError::GhostNotAllowed { .. })));
        let locked = add_child(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), NewKind::Node);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        assert!(matches!(add_child(dir.path(), &rel(&["999.Settings"]), NewKind::Node), Err(FmError::Reserved { .. })));
        assert!(matches!(add_child(dir.path(), &[], NewKind::Node), Err(FmError::InvalidPath { .. })));
    }

    #[cfg(windows)]
    #[test]
    fn add_child_removes_created_folders_when_yaml_save_fails() {
        let dir = academic("add-rollback");
        let _held = break_tree_save(dir.path());
        let err = add_child(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math", "02.Bio"]);
    }

    #[test]
    fn meta_key_folders_are_refused_and_the_parent_type_survives() {
        let dir = academic("meta-key");
        dir.mkdir("100.ACADEMIC/type");
        dir.mkdir("100.ACADEMIC/01.Math/status");
        let refused = |r: R<Mutation>| matches!(r, Err(FmError::InvalidName { .. }));
        assert!(refused(rename_node(dir.path(), &rel(&["100.ACADEMIC", "type"]), "X")));
        assert!(refused(set_node_type(dir.path(), &rel(&["100.ACADEMIC", "type"]), EditableType::Wkb, None)));
        assert!(refused(add_child(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "status"]), NewKind::Node)));
        assert!(dir.path().join("100.ACADEMIC/type").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC"]), "NODE-L1");
        let math = get_node(&yaml(dir.path()), &rel(&["100.ACADEMIC", "01.Math"])).unwrap().clone();
        assert_eq!(math.get("status").and_then(Value::as_str), Some("done"));
    }

    #[test]
    fn locate_needs_every_component_with_its_exact_case() {
        let dir = academic("locate-case");
        let data: Mapping = serde_yaml::from_str("100.academic:\n  02.Bio: {type: NODE-L2}\n").unwrap();
        let target = locate(dir.path(), &data, &rel(&["100.academic", "02.Bio"])).unwrap();
        assert!(target.ghost);
        assert!(!locate(dir.path(), &data, &rel(&["100.ACADEMIC", "02.Bio"])).unwrap().ghost);
    }

    #[test]
    fn noop_rename_reports_and_saves_the_sync() {
        let dir = academic("rename-noop");
        dir.mkdir("100.ACADEMIC/05.Fresh");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "bio").unwrap();
        assert!(m.tree.yaml_changed);
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "05.Fresh"]), "NODE-L2");
    }

    #[test]
    fn mutations_refuse_while_another_process_holds_the_lock() {
        let dir = academic("locked-storage");
        let held = StorageLock::acquire(dir.path()).unwrap();
        let err = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap_err();
        assert!(matches!(err, FmError::Io { ref op, .. } if op == "lock storage"), "{err:?}");
        assert!(dir.path().join("100.ACADEMIC/02.Bio").is_dir());
        drop(held);
        rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "Chem").unwrap();
    }

    #[test]
    fn a_tree_failure_after_commit_is_a_warning_not_an_error() {
        let dir = academic("after-commit");
        let failed = Err(FmError::Internal { message: "boom".to_string() });
        let m = after_commit(dir.path(), failed, Some(rel(&["x"])), vec!["w".to_string()]);
        assert!(m.tree.nodes.is_empty() && m.tree.yaml_changed);
        assert_eq!(m.select, Some(rel(&["x"])));
        assert_eq!(m.warnings.len(), 2);
        assert!(m.warnings[1].contains("refresh") && m.warnings[1].contains("boom"), "{:?}", m.warnings);
    }

    #[test]
    fn workboard_rename_edits_only_the_name_after_the_tracker() {
        let dir = storage("wkb-rename", &["100.A/PJ.001"], "100.A:
  type: NODE-L1
  PJ.001:
    type: WKB-L2
");
        let rel_wkb = rel(&["100.A", "PJ.001"]);
        let renamed = rename_node(dir.path(), &rel_wkb, "Fabi-web").unwrap();
        assert_eq!(renamed.select, Some(rel(&["100.A", "PJ.001_Fabi-web"])));
        assert_eq!(names(&dir.path().join("100.A")), ["PJ.001_Fabi-web"]);
        let cleared = rename_node(dir.path(), &rel(&["100.A", "PJ.001_Fabi-web"]), "").unwrap();
        assert_eq!(cleared.select, Some(rel_wkb));
        assert_eq!(names(&dir.path().join("100.A")), ["PJ.001"]);
    }
}
