// A new folder is created with its tree entry, 98 review folder and .ant marker as one journaled step.
use std::fs;
use std::path::Path;

use serde_yaml::Mapping;

use super::{child_rel, ensure_free, finish, guard_frozen, load_synced, load_synced_at, locate, persist, register_ids, Mutation};
use crate::ant_core::names::{editable_part, list_subdirs, next_available_id, next_main_category_id};
use crate::ant_core::tree_types::child_entries;
use crate::backup::stamp::local_date;
use crate::config::{NewKind, DATA_DIR, DATA_FLAG};
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, Journal, StorageLock};
use crate::storage::db::hide;
use crate::storage::info::{write_info, OWNER_KEYS};
use crate::tree::depth;
use crate::tree::tree_store::{get_node, get_node_mut};

// A selected Level 1 node creates a sibling category at the Storage root.
pub fn add_next(root: &Path, rel: &[String], kind: NewKind) -> R<Mutation> {
    if rel.is_empty() {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the storage root is not a node".to_string() });
    }
    if rel.len() == 1 && kind == NewKind::Workboard {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "workboards cannot be created at the storage root".to_string() });
    }
    let parent = if rel.len() == 1 { &[][..] } else { &rel[..rel.len() - 1] };
    add_node(root, parent, kind)
}

pub fn add_node(root: &Path, parent: &[String], kind: NewKind) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    // A tracker must be unique across the Storage, so a workboard counts every branch on disk.
    let (mut data, synced) = if kind == NewKind::Workboard { load_synced(root)? } else { load_synced_at(root, parent)? };
    if parent.is_empty() && kind == NewKind::Workboard {
        return Err(FmError::InvalidPath { rel_parts: parent.to_vec(), reason: "workboards cannot be created at the storage root".to_string() });
    }
    let target = if parent.is_empty() {
        super::Target { abs: root.to_path_buf(), ghost: false, node_type: String::new() }
    } else {
        locate(root, &data, parent)?
    };
    guard_frozen(parent, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "add node".to_string() });
    }
    let limit = depth::limit(root)?;
    let child_depth = parent.len();
    if child_depth >= limit {
        return Err(FmError::DepthLimit);
    }
    let parent_yaml =
        get_node(&data, parent).ok_or_else(|| FmError::NotFound { rel_parts: parent.to_vec() })?;
    // Ghost ids count as taken so a new node never inherits a missing folder's record.
    let mut taken = list_subdirs(&target.abs)?;
    taken.extend(child_entries(parent_yaml).map(|(k, _)| k.to_string()));
    let workboard = kind == NewKind::Workboard;
    let new_name = if parent.is_empty() {
        format!("{:03}.NEW-CATEGORY", next_main_category_id(&taken))
    } else if workboard {
        format!("PJ.{:03}_New-Workboard", next_tracker(&data, "PJ"))
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
    if !workboard && level >= 3 {
        entry.insert("flags".into(), DATA_FLAG.into());
    }
    let mut warnings = Vec::new();
    let rev_name = format!("98.{}", editable_part(&new_name));
    // A workboard keeps its review shortcuts out of its own folder.
    match if workboard { Ok(()) } else { journal.mkdir(&new_abs.join(&rev_name)) } {
        // The deepest level keeps its 98 folder on disk only, as the tree stops at the depth limit.
        Ok(()) if level < limit && !workboard => {
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
            return Err(journal.abort("add node", cause));
        }
    }
    persist(root, &data, journal, "add node")?;
    // The folder exists now, so a failure here is only reported.
    if let Err(e) = mark_folder(&new_abs, workboard, parent.last().map_or("", String::as_str)) {
        warnings.push(format!("Could not set up {DATA_DIR}: {e}"));
    }
    let moved = register_ids(root, &mut data, &mut warnings);
    let new_rel = child_rel(parent, &new_name);
    Ok(finish(root, &data, synced || moved, Some(new_rel.clone()), warnings, vec![new_rel]))
}

// Next free id of a tracker (3 digits, more once they run out) after every workboard of that tracker in the tree, missing folders included.
pub(super) fn next_tracker(data: &Mapping, tracker: &str) -> u32 {
    fn highest(node: &Mapping, prefix: &str) -> u32 {
        child_entries(node)
            .map(|(name, child)| {
                let own = name.strip_prefix(prefix).map(|r| r.split(|c: char| !c.is_ascii_digit()).next().unwrap_or("")).and_then(|d| d.parse().ok()).unwrap_or(0);
                own.max(highest(child, prefix))
            })
            .max()
            .unwrap_or(0)
    }
    highest(data, &format!("{tracker}.")) + 1
}

// Every created folder gets a .ant folder, which is what lets its id follow it around; workboards also start their info.json.
pub(super) fn mark_folder(folder: &Path, workboard: bool, parent: &str) -> R<()> {
    if workboard {
        return write_info(
            folder,
            &[
                (&["status"], "status", Some("in-progress".into())),
                (&OWNER_KEYS, "owner", Some("unknown".into())),
                (&["created"], "created", Some(local_date().into())),
                (&["breadcrumb"], "breadcrumb", Some(parent.into())),
            ],
        );
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
    fn add_node_takes_lowest_free_id_and_creates_review_folder() {
        let dir = academic("add");
        let m = add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "04.New-Node"])));
        assert!(dir.path().join("100.ACADEMIC/04.New-Node/98.New-Node").is_dir());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node"]), "NODE-L2");
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "04.New-Node", "98.New-Node"]), "REV-L2");

        let deep = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
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
        let main = add_next(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap();
        let expected = rel(&["200.NEW-CATEGORY"]);
        assert_eq!(main.select, Some(expected.clone()));
        assert_eq!(yaml_type(dir.path(), &["200.NEW-CATEGORY"]), "NODE-L1");
        assert!(dir.path().join("200.NEW-CATEGORY/98.NEW-CATEGORY").is_dir());
        let main_node = main.touched_nodes()[0];
        assert_eq!(main_node.rel_parts, expected);
        assert!(main_node.flags.iter().any(|flag| flag == "main"));
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
        assert!(info.contains("\"breadcrumb\": \"01.Math\"") && info.contains("\"created\": \""), "{info}");
        assert!(wkb.join(".ant/dragpath.json").is_file());
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "PJ.001_New-Workboard"]), "WKB-L3");
    }

    #[test]
    fn a_workboard_made_outside_the_app_in_another_branch_keeps_its_tracker() {
        let dir = academic("add-wkb-outside");
        let mut data = yaml(dir.path());
        crate::tree::tree_store::sync(dir.path(), &mut data).unwrap();
        crate::tree::tree_store::save(dir.path(), &data).unwrap();
        dir.mkdir("100.ACADEMIC/01.Math/PJ.007_Outside");
        let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Workboard).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "PJ.008_New-Workboard"])));
    }

    #[test]
    fn trackers_keep_counting_past_three_digits() {
        let dir = academic("add-wkb-1000");
        let data = serde_yaml::from_str("PJ.999_Full: {type: WKB-L1}
PJ.1000_Next: {type: WKB-L1}
").unwrap();
        assert_eq!(next_tracker(&data, "PJ"), 1001);
        std::fs::create_dir(dir.path().join("100.ACADEMIC/PJ.999_Full")).unwrap();
        let m = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Workboard).unwrap();
        assert_eq!(m.select, Some(rel(&["100.ACADEMIC", "PJ.1000_New-Workboard"])));
        let again = add_next(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), NewKind::Workboard).unwrap();
        assert_eq!(again.select, Some(rel(&["100.ACADEMIC", "PJ.1001_New-Workboard"])));
    }

    #[test]
    fn add_node_refuses_depth_ghost_locked_and_reserved_parents() {
        let dir = academic("add-refuse");
        let too_deep = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Node);
        assert!(matches!(too_deep, Err(FmError::DepthLimit)));
        let ghost = add_node(dir.path(), &rel(&["100.ACADEMIC", "03.Ghost"]), NewKind::Node);
        assert!(matches!(ghost, Err(FmError::GhostNotAllowed { .. })));
        let locked = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "98.Math"]), NewKind::Node);
        assert!(matches!(locked, Err(FmError::Locked { .. })));
        assert!(matches!(add_node(dir.path(), &rel(&["999.Settings"]), NewKind::Node), Err(FmError::Reserved { .. })));
    }

    #[test]
    fn a_raised_depth_limit_moves_the_last_level_down() {
        let dir = academic("add-deeper");
        crate::storage::db::Store::open_or_create(dir.path()).unwrap().set_setting(depth::DEPTH_KEY, "4").unwrap();
        add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math"]), NewKind::Node).unwrap();
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "02.New-Node", "98.New-Node"]), "REV-L3");
        let m = add_node(dir.path(), &rel(&["100.ACADEMIC", "01.Math", "01.Algebra"]), NewKind::Node).unwrap();
        let l4 = rel(&["100.ACADEMIC", "01.Math", "01.Algebra", "01.New-Node"]);
        assert_eq!(m.select, Some(l4.clone()));
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "01.Math", "01.Algebra", "01.New-Node"]), "NODE-L4");
        assert!(dir.path().join("100.ACADEMIC/01.Math/01.Algebra/01.New-Node/98.New-Node").is_dir());
        assert!(get_node(&yaml(dir.path()), &l4).unwrap().get("98.New-Node").is_none());
        assert!(matches!(add_node(dir.path(), &l4, NewKind::Node), Err(FmError::DepthLimit)));
    }

    #[test]
    fn add_node_at_root_creates_a_main_category() {
        let dir = academic("add-root");
        let m = add_node(dir.path(), &[], NewKind::Node).unwrap();
        let created = m.select.expect("the new category is selected");
        assert_eq!(created.len(), 1);
        assert!(dir.path().join(&created[0]).is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn add_node_removes_created_folders_when_yaml_save_fails() {
        let dir = academic("add-rollback");
        let _held = break_tree_save(dir.path());
        let err = add_node(dir.path(), &rel(&["100.ACADEMIC"]), NewKind::Node).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.ACADEMIC")), ["01.Math", "02.Bio"]);
    }
}
