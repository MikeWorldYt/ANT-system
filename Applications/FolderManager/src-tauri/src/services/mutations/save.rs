// Loading and saving the tree around a mutation, with new folder ids registered on the way.
use std::collections::BTreeSet;
use std::path::Path;

use serde_yaml::Mapping;

use crate::domain::tree_types::child_entries;
use crate::domain::model::Copies;
use crate::domain::error::R;
use crate::infra::fs_safe::Journal;
use crate::services::index::footprint;
use crate::services::tree::tree_store;

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

// What a sync found out of date on disk; a folder list is a known scope, `unscoped` means rows may differ anywhere.
#[derive(Debug, Default)]
pub(crate) struct Drift {
    paths: Vec<Vec<String>>,
    unscoped: bool,
}

impl Drift {
    pub(crate) fn unscoped(on: bool) -> Self {
        Self { paths: Vec::new(), unscoped: on }
    }

    pub(crate) fn any(&self) -> bool {
        self.unscoped || !self.paths.is_empty()
    }

    pub(crate) fn absorb(&mut self, other: Drift) {
        self.unscoped |= other.unscoped;
        self.paths.extend(other.paths);
    }

    pub(crate) fn or_unscoped(mut self, on: bool) -> Self {
        self.unscoped |= on;
        self
    }
}

// Ordinary mutations list only the levels down to `dir`; the folders that sync changed are saved with the mutation, and drift elsewhere is left to the watcher.
pub(crate) fn load_synced_at(root: &Path, dir: &[String]) -> R<(Mapping, Drift)> {
    sync_loaded_at(root, dir, tree_store::load(root)?)
}

// For a caller that already holds the tree, so it is not read twice.
pub(crate) fn sync_loaded_at(root: &Path, dir: &[String], mut data: Mapping) -> R<(Mapping, Drift)> {
    let drift = sync_branch_drift(root, &mut data, dir)?;
    Ok((data, drift))
}

pub(crate) fn sync_branch_drift(root: &Path, data: &mut Mapping, dir: &[String]) -> R<Drift> {
    let (renamed, paths) = tree_store::sync_branch_paths(root, data, dir)?;
    Ok(Drift { paths, unscoped: renamed })
}

// The touched paths plus the folders the sync changed, so they are saved and patched too; unscoped drift empties it (the whole tree).
pub(crate) fn scope(drift: &Drift, mut touched: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if drift.unscoped {
        return Vec::new();
    }
    for path in &drift.paths {
        if !touched.contains(path) {
            touched.push(path.clone());
        }
    }
    touched
}

pub(crate) fn persist(root: &Path, data: &Mapping, journal: Journal, op: &str, touched: &[Vec<String>]) -> R<()> {
    let saved = if touched.is_empty() { tree_store::save(root, data) } else { tree_store::save_paths(root, data, touched) };
    match saved {
        Ok(()) => {
            journal.commit();
            Ok(())
        }
        Err(e) => Err(journal.abort(op, e)),
    }
}
