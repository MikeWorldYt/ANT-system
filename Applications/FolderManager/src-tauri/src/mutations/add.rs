// A new folder is created with its tree entry, 98 review folder and .ant marker as one journaled step.
use std::fs;
use std::path::Path;

use serde_yaml::Mapping;

use super::{child_rel, ensure_free, finish, guard_frozen, load_synced, locate, persist, register_ids, Mutation};
use crate::ant_core::names::{editable_part, list_subdirs, next_available_id};
use crate::ant_core::tree_types::child_entries;
use crate::config::{NewKind, DATA_DIR, MAX_DEPTH};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, Journal, StorageLock};
use crate::storage::db::hide;
use crate::storage::info::{write_info, OWNER_KEYS};
use crate::tree::tree_store::{get_node, get_node_mut};

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
        Err(e) => warnings.push(format!("Could not create {rev_name}: {e}")),
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
        warnings.push(format!("Could not set up {DATA_DIR}: {e}"));
    }
    register_ids(root, &mut data, &mut warnings);
    Ok(finish(root, &data, synced, Some(child_rel(parent, &new_name)), warnings))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

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
}
