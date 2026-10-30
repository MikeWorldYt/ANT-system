// Applies one batch of changed paths to ant.db, syncing only the subtrees the paths fall in.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use super::filter::{is_own_write, needs_full, relative};
use super::targets::targets;
use super::{log, ApplyReport};
use crate::error::R;
use crate::fs_safe::StorageLock;
use crate::tree::tree_store;

pub(super) fn apply(root: &Path, changed: &[PathBuf], rescan: Option<&str>) -> R<ApplyReport> {
    let mut rels = BTreeSet::new();
    let mut skipped = 0;
    for path in changed {
        match relative(root, path).filter(|rel| !is_own_write(rel)) {
            Some(rel) => {
                rels.insert(rel);
            }
            None => skipped += 1,
        }
    }
    let rels: Vec<Vec<String>> = rels.into_iter().collect();
    let reason = rescan.map(str::to_string).or_else(|| needs_full(rels.len()).then(|| format!("{} changed paths", rels.len())));
    let mut report = ApplyReport { paths: rels.len(), full: reason.is_some(), skipped, saved: false };
    if let Some(reason) = &reason {
        log("info", &format!("Watch: full sync ({reason})"));
    }
    if rels.is_empty() && !report.full {
        return Ok(report);
    }
    // The unlocked pass keeps a batch that changes nothing from taking the lock or writing ant.db.
    let mut data = tree_store::load(root)?;
    let (dirty, whole) = sync(root, &mut data, &rels, report.full)?;
    report.full = whole;
    if !dirty {
        return Ok(report);
    }
    // Reloading under the file lock keeps this save from overwriting another process's edit.
    let _lock = StorageLock::acquire(root)?;
    let mut data = tree_store::load(root)?;
    sync(root, &mut data, &rels, report.full)?;
    tree_store::save(root, &data)?;
    report.saved = true;
    Ok(report)
}

// A change at the storage root makes its target the root itself, which is a full sync.
fn sync(root: &Path, data: &mut Mapping, rels: &[Vec<String>], full: bool) -> R<(bool, bool)> {
    let nodes = targets(data, rels);
    if full || nodes.iter().any(Vec::is_empty) {
        return Ok((tree_store::sync(root, data)?, true));
    }
    let mut changed = false;
    for rel in &nodes {
        if let Some(node) = tree_store::get_node_mut(data, rel) {
            changed |= tree_store::sync_subtree(root, rel, node)?;
        }
    }
    Ok((changed, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::db_path;
    use crate::test_support::{rel, work, yaml};
    use crate::watch::apply_batch;

    #[test]
    fn a_new_folder_under_a_node_reaches_the_tree_and_ant_db() {
        let dir = work("watch-new");
        dir.mkdir("100.WORK/06.New/01.Deep");
        dir.mkdir("200.Other");
        let report = apply_batch(dir.path(), &[dir.path().join("100.WORK/06.New")]).unwrap();
        assert_eq!(report, ApplyReport { paths: 1, full: false, skipped: 0, saved: true });
        let data = yaml(dir.path());
        assert_eq!(tree_store::get_type(&data, &rel(&["100.WORK", "06.New"])), "NODE-L2");
        assert_eq!(tree_store::get_type(&data, &rel(&["100.WORK", "06.New", "01.Deep"])), "NODE-L3");
        assert!(tree_store::get_node(&data, &rel(&["200.Other"])).is_none());
    }

    #[test]
    fn the_sync_writes_do_not_trigger_another_sync() {
        let dir = work("watch-loop");
        dir.mkdir("100.WORK/06.New");
        apply_batch(dir.path(), &[dir.path().join("100.WORK/06.New")]).unwrap();
        assert!(!apply_batch(dir.path(), &[dir.path().join("100.WORK/06.New")]).unwrap().saved);
        let before = std::fs::metadata(db_path(dir.path())).unwrap().modified().unwrap();
        let own = [".ant/ant.db", ".ant/ant.db-journal", "999.Settings/00.Core/.FolderManager.lock"].map(|p| dir.path().join(p));
        let report = apply_batch(dir.path(), &own).unwrap();
        assert_eq!(report, ApplyReport { paths: 0, full: false, skipped: 3, saved: false });
        assert_eq!(std::fs::metadata(db_path(dir.path())).unwrap().modified().unwrap(), before);
    }

    #[test]
    fn a_root_level_folder_or_a_rescan_syncs_the_whole_tree() {
        let dir = work("watch-full");
        dir.mkdir("200.Other/01.X");
        let report = apply_batch(dir.path(), &[dir.path().join("200.Other")]).unwrap();
        assert!(report.full && report.saved);
        assert_eq!(tree_store::get_type(&yaml(dir.path()), &rel(&["200.Other", "01.X"])), "NODE-L2");
        dir.mkdir("100.WORK/02.B/01.Y");
        assert!(apply(dir.path(), &[], Some("overflow")).unwrap().full);
        assert_eq!(tree_store::get_type(&yaml(dir.path()), &rel(&["100.WORK", "02.B", "01.Y"])), "NODE-L3");
    }
}
