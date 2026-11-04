// Loading and saving the tree around a mutation, with new folder ids registered on the way.
use std::collections::BTreeSet;
use std::path::Path;

use serde_yaml::Mapping;

use crate::ant_core::tree_types::child_entries;
use crate::domain::model::Copies;
use crate::error::R;
use crate::fs_safe::Journal;
use crate::index::footprint;
use crate::tree::tree_store;

// New .ant folders get their id now; true means a folder moved outside the app was re-keyed, which is not a local change.
pub(crate) fn register_ids(root: &Path, data: &mut Mapping, warnings: &mut Vec<String>) -> bool {
    let shape = node_paths(data);
    match footprint::plan(root, data, Copies::Ignore).map(|plan| plan.changed) {
        Ok(true) => {
            if let Err(e) = tree_store::save(root, data) {
                warnings.push(format!("Could not save moved folders: {e}"));
            }
            node_paths(data) != shape
        }
        Ok(false) => false,
        Err(e) => {
            warnings.push(format!("Could not assign folder ids: {e}"));
            false
        }
    }
}

fn node_paths(data: &Mapping) -> BTreeSet<Vec<String>> {
    fn walk(node: &Mapping, rel: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
        for (name, child) in child_entries(node) {
            rel.push(name.to_string());
            out.insert(rel.clone());
            walk(child, rel, out);
            rel.pop();
        }
    }
    let mut out = BTreeSet::new();
    walk(data, &mut Vec::new(), &mut out);
    out
}

// Ordinary mutations list only the levels down to `dir`; drift found there re-syncs everything, as the change is then not local.
pub(crate) fn load_synced_at(root: &Path, dir: &[String]) -> R<(Mapping, bool)> {
    let mut data = tree_store::load(root)?;
    let changed = tree_store::sync_branch(root, &mut data, dir)?;
    if changed {
        tree_store::sync(root, &mut data)?;
    }
    Ok((data, changed))
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
