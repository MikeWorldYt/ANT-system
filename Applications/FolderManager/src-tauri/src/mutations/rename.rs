// A rename keeps the fixed id prefix and carries the linked 90./98. folders along with the node.
use std::path::Path;

use serde_yaml::{Mapping, Value};

use super::{child_rel, ensure_free, finish, guard_frozen, load_synced, locate, persist, split, Mutation};
use crate::ant_core::names::{compose_name, editable_part, list_subdirs, name_parts, sanitize_train_case, sanitize_upper};
use crate::ant_core::tree_types::type_base;
use crate::config::LINKED_PREFIXES;
use crate::error::{FmError, R};
use crate::fs_safe::{join_rel, same_name, validate_component, Journal, StorageLock};
use crate::tree::tree_store::{self, get_node, get_node_mut, rename_key};

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
        return Ok(finish(root, &data, synced, Some(new_rel.clone()), Vec::new()).touching(vec![new_rel]));
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
    Ok(finish(root, &data, synced, Some(new_rel.clone()), warnings).touching(vec![rel.to_vec(), new_rel]))
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
        Err(e) => return vec![format!("Linked folders not renamed: {e}")],
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
            Err(e) => warnings.push(format!("Could not rename {child_old}: {e}")),
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

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
    fn noop_rename_reports_and_saves_the_sync() {
        let dir = academic("rename-noop");
        dir.mkdir("100.ACADEMIC/05.Fresh");
        let m = rename_node(dir.path(), &rel(&["100.ACADEMIC", "02.Bio"]), "bio").unwrap();
        assert!(m.tree.yaml_changed);
        assert_eq!(yaml_type(dir.path(), &["100.ACADEMIC", "05.Fresh"]), "NODE-L2");
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
