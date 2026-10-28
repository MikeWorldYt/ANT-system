// A move swaps the numeric ids of two siblings, so both renames go through one journal.
use std::path::Path;

use serde::Deserialize;

use super::{child_rel, ensure_free, finish, guard_frozen, load_synced_at, locate, parent_of, persist, split, subtrees, Mutation};
use crate::ant_core::names::{editable_part, is_movable_node, list_subdirs, numeric_id};
use crate::error::{FmError, R};
use crate::fs_safe::{join_rel, same_name, validate_component, Journal, StorageLock};
use crate::tree::tree_store::{get_node, get_node_mut, get_type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoveDirection {
    Up,
    Down,
}

pub fn move_node(root: &Path, rel: &[String], dir: MoveDirection) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost {
        return Err(FmError::GhostNotAllowed { op: "move".to_string() });
    }
    let (name, parent) = split(rel)?;
    if !is_movable_node(name, &target.node_type) {
        return Err(FmError::NotMovable);
    }
    let parent_abs = join_rel(root, parent);
    let mut movable: Vec<(u32, String)> = list_subdirs(&parent_abs)?
        .into_iter()
        .filter(|s| is_movable_node(s, &get_type(&data, &child_rel(parent, s))))
        .filter_map(|s| numeric_id(&s).map(|id| (id, s)))
        .collect();
    movable.sort();
    let idx = movable.iter().position(|(_, s)| s == name).ok_or(FmError::NotMovable)?;
    let swap_idx = match dir {
        MoveDirection::Up => idx.checked_sub(1),
        MoveDirection::Down => Some(idx + 1).filter(|i| *i < movable.len()),
    }
    .ok_or(FmError::NotMovable)?;
    let node_id = movable[idx].0;
    let (swap_id, swap_name) = movable[swap_idx].clone();
    if swap_id == node_id {
        return Err(FmError::NotMovable);
    }

    let node_new = format!("{swap_id:02}.{}", editable_part(name));
    let swap_new = format!("{node_id:02}.{}", editable_part(&swap_name));
    // Equal editable parts would make both names collide, so the node keeps a trailing "_" as Python did.
    let collision = same_name(&node_new, &swap_name);
    let node_final = if collision { format!("{node_new}_") } else { node_new };
    validate_component(&node_final)?;
    validate_component(&swap_new)?;
    let pair = [name.as_str(), swap_name.as_str()];
    ensure_free(&parent_abs, get_node(&data, parent), &node_final, &pair)?;
    ensure_free(&parent_abs, get_node(&data, parent), &swap_new, &pair)?;

    let before = subtrees(&data, &[rel.to_vec(), child_rel(parent, &swap_name)]);
    let mut journal = Journal::new();
    journal.rename(&parent_abs.join(name), &parent_abs.join(&node_final))?;
    if let Err(e) = journal.rename(&parent_abs.join(&swap_name), &parent_abs.join(&swap_new)) {
        return Err(journal.abort("move", e));
    }
    if let Some(siblings) = get_node_mut(&mut data, parent) {
        let node_entry = siblings.remove(name.as_str());
        let swap_entry = siblings.remove(swap_name.as_str());
        if let Some(entry) = node_entry {
            siblings.insert(node_final.clone().into(), entry);
        }
        if let Some(entry) = swap_entry {
            siblings.insert(swap_new.clone().into(), entry);
        }
    }
    persist(root, &data, journal, "move")?;
    let warnings = if collision {
        vec![format!("{name} became {node_final} because {swap_name} has the same name")]
    } else {
        Vec::new()
    };
    let node_rel = child_rel(parent, &node_final);
    let touched = vec![rel.to_vec(), child_rel(parent, &swap_name), node_rel.clone(), child_rel(parent, &swap_new)];
    Ok(finish(root, &data, synced, Some(node_rel), warnings, touched).replacing(before))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::*;

    #[test]
    fn move_down_swaps_ids_with_the_next_movable_sibling() {
        let dir = work("move-down");
        let m = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap();
        assert_eq!(m.select, Some(rel(&["100.WORK", "02.A"])));
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.B", "02.A", "03.C", "04.Wkb", "98.Rev"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "02.A", "01.Keep"]), "NODE-L3");
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "01.B"]), "NODE-L2");
        assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "01.A"])).is_none());
        let up = move_node(dir.path(), &rel(&["100.WORK", "03.C"]), MoveDirection::Up).unwrap();
        assert_eq!(up.select, Some(rel(&["100.WORK", "02.C"])));
        assert!(dir.path().join("100.WORK/03.A").is_dir());
    }

    #[test]
    fn move_refuses_ends_ghosts_and_non_node_types() {
        let dir = work("move-refuse");
        let w = |p: &str, d| move_node(dir.path(), &rel(&["100.WORK", p]), d);
        assert!(matches!(w("01.A", MoveDirection::Up), Err(FmError::NotMovable)));
        assert!(matches!(w("03.C", MoveDirection::Down), Err(FmError::NotMovable)));
        assert!(matches!(w("04.Wkb", MoveDirection::Up), Err(FmError::NotMovable)));
        assert!(matches!(w("05.Ghost", MoveDirection::Up), Err(FmError::GhostNotAllowed { .. })));
        assert!(matches!(w("98.Rev", MoveDirection::Up), Err(FmError::Locked { .. })));
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
    }

    #[test]
    fn move_with_equal_names_uses_the_underscore_name() {
        let dir = storage(
            "move-clash",
            &["100.WORK/05.Ships", "100.WORK/06.ships"],
            "100.WORK:\n  05.Ships: {type: NODE-L2, x: {type: NODE-L3}}\n  06.ships: {type: NODE-L2}\n",
        );
        let m = move_node(dir.path(), &rel(&["100.WORK", "05.Ships"]), MoveDirection::Down).unwrap();
        assert_eq!(m.select, Some(rel(&["100.WORK", "06.Ships_"])));
        assert_eq!(m.warnings.len(), 1);
        assert_eq!(names(&dir.path().join("100.WORK")), ["05.ships", "06.Ships_"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "06.Ships_", "x"]), "NODE-L3");
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "05.ships"]), "NODE-L2");
    }

    #[cfg(windows)]
    #[test]
    fn move_rolls_back_when_yaml_save_fails() {
        let dir = work("move-yaml");
        let _held = break_tree_save(dir.path());
        let err = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap_err();
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
    }

    // A handle without FILE_SHARE_DELETE on the sibling makes the second rename fail mid-move.
    #[cfg(windows)]
    #[test]
    fn move_undoes_the_first_rename_when_the_second_fails() {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_SHARE_READ_WRITE: u32 = 0x1 | 0x2;
        let dir = work("move-second");
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(dir.path().join("100.WORK/02.B"))
            .unwrap();
        let err = move_node(dir.path(), &rel(&["100.WORK", "01.A"]), MoveDirection::Down).unwrap_err();
        drop(held);
        assert!(matches!(err, FmError::RolledBack { .. }), "{err:?}");
        assert_eq!(names(&dir.path().join("100.WORK")), ["01.A", "02.B", "03.C", "04.Wkb", "98.Rev"]);
        assert_eq!(yaml_type(dir.path(), &["100.WORK", "01.A"]), "NODE-L2");
    }
}
