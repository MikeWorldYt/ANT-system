// A level-1 folder outside the category format moves into the review folder (98.*) of a category.
use std::path::Path;

use super::{insert, rules};
use crate::ant_core::names::{is_main_category, is_valid_root_name};
use crate::ant_core::tree_types::type_base;
use crate::config::is_hidden_dir;
use crate::error::{FmError, R};
use crate::fs_safe::{Journal, StorageLock};
use crate::mutations::{child_rel, ensure_free, finish, load_synced_at, locate, persist, subtrees, Mutation};
use crate::tree::depth;
use crate::tree::tree_store::{self, delete_key, get_node, get_node_mut, infer_type};

pub fn move_to_review(root: &Path, name: &str, destination: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let source = vec![name.to_string()];
    if is_hidden_dir(name) || is_valid_root_name(name) {
        return Err(rules::refuse(&source, "only a level-1 folder outside the category format can be moved to a review folder"));
    }
    let (mut data, mut synced) = load_synced_at(root, destination)?;
    let from = locate(root, &data, &source)?;
    if from.ghost {
        return Err(FmError::GhostNotAllowed { op: "move to review".to_string() });
    }
    let in_category = matches!(destination, [category, _] if is_main_category(category));
    let to = locate(root, &data, destination)?;
    rules::check_destination(destination, &to)?;
    if !in_category || type_base(&to.node_type) != "REV" {
        return Err(rules::refuse(destination, "the destination must be the review folder (98.*) of a category"));
    }
    ensure_free(&to.abs, get_node(&data, destination), name, &[]).map_err(|e| match e {
        FmError::AlreadyExists { .. } => rules::refuse(
            destination,
            format!("a folder named {name} already exists in {}", destination.join("/")),
        ),
        other => other,
    })?;

    if let Some(node) = get_node_mut(&mut data, &source) {
        synced |= tree_store::sync_subtree(root, &source, node)?;
    }
    let mut entry = get_node(&data, &source).cloned().unwrap_or_default();
    // The level-1 type was inferred for the root, so it is inferred again for the level it lands on.
    entry.insert("type".into(), infer_type(name, destination.len()).into());
    let limit = depth::limit(root)?;
    let warnings: Vec<String> = rules::depth_warning(name, &entry, destination, limit, false).into_iter().collect();
    let new_rel = child_rel(destination, name);
    let select = if new_rel.len() > limit { destination.to_vec() } else { new_rel.clone() };

    let before = subtrees(&data, &[source.clone()]);
    let mut journal = Journal::new();
    journal.rename(&from.abs, &to.abs.join(name))?;
    delete_key(&mut data, &[], name);
    if let Err(e) = insert(root, &mut data, &new_rel, entry, limit) {
        return Err(journal.abort("move to review", e));
    }
    persist(root, &data, journal, "move to review")?;
    let m = finish(root, &data, synced, Some(select), warnings, vec![source, new_rel]);
    Ok(m.replacing(before))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{names, rel, storage, yaml, yaml_type, TempDir};

    const TREE: &str = "100.WORK:\n  type: NODE-L1\n  01.Plain: {type: NODE-L2}\n  98.WORK: {type: REV}\n\
        200.HOME:\n  type: NODE-L1\n  98.HOME: {type: REV}\n";
    const DIRS: &[&str] = &["998.Review", "100.WORK/01.Plain", "100.WORK/98.WORK/Marcos", "200.HOME/98.HOME", "Marcos/01.Inner", "Stray/98.Stray"];

    fn ready(tag: &str) -> TempDir {
        let dir = storage(tag, DIRS, TREE);
        fs::write(dir.path().join("Marcos/notes.txt"), "keep").unwrap();
        dir
    }

    fn refused(dir: &TempDir, name: &str, destination: &[&str]) -> FmError {
        let (disk, tree) = (names(dir.path()), yaml(dir.path()));
        let err = move_to_review(dir.path(), name, &rel(destination)).unwrap_err();
        assert_eq!(disk, names(dir.path()), "{err:?}");
        assert_eq!(tree, yaml(dir.path()), "{err:?}");
        err
    }

    fn reason(err: FmError) -> String {
        match err {
            FmError::InvalidPath { reason, .. } => reason,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn refusals_leave_the_disk_and_the_tree_unchanged() {
        let dir = ready("mtr-refuse");
        let home = ["200.HOME", "98.HOME"];
        assert!(reason(refused(&dir, "100.WORK", &home)).contains("outside the category format"));
        assert!(reason(refused(&dir, "998.Review", &home)).contains("outside the category format"));
        assert!(reason(refused(&dir, ".ant", &home)).contains("outside the category format"));
        assert!(reason(refused(&dir, "Marcos", &["100.WORK", "01.Plain"])).contains("review folder (98.*) of a category"));
        assert!(reason(refused(&dir, "Marcos", &["Stray", "98.Stray"])).contains("review folder (98.*) of a category"));
        assert!(matches!(refused(&dir, "Marcos", &["200.HOME", "98.Missing"]), FmError::NotFound { .. }));
        assert!(matches!(refused(&dir, "Gone", &home), FmError::NotFound { .. }));
        assert!(matches!(refused(&dir, "Marcos", &["998.Review"]), FmError::Reserved { .. }));
        assert_eq!(reason(refused(&dir, "Marcos", &["100.WORK", "98.WORK"])), "a folder named Marcos already exists in 100.WORK/98.WORK");
    }

    #[test]
    fn a_stray_folder_moves_into_a_category_review_folder_with_its_contents() {
        let dir = ready("mtr-move");
        let m = move_to_review(dir.path(), "Marcos", &rel(&["200.HOME", "98.HOME"])).unwrap();
        let moved = ["200.HOME", "98.HOME", "Marcos"];
        assert_eq!(m.select, Some(rel(&moved)));
        assert!(!dir.path().join("Marcos").exists());
        assert_eq!(fs::read_to_string(dir.path().join("200.HOME/98.HOME/Marcos/notes.txt")).unwrap(), "keep");
        assert!(dir.path().join("200.HOME/98.HOME/Marcos/01.Inner").is_dir());
        assert!(get_node(&yaml(dir.path()), &rel(&["Marcos"])).is_none());
        assert_eq!(yaml_type(dir.path(), &moved), "NODE-L3");
    }
}
