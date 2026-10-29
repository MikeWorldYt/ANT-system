// The stored tree and the snapshot the UI loads, kept in step with the folders on disk.
pub mod depth;
pub mod tree_node;
pub mod tree_store;

use std::path::Path;

use crate::error::R;
use crate::fs_safe::StorageLock;
use crate::index::footprint::{self, Copies};
use crate::index::search_index;
use crate::mutations;
use tree_node::{build_tree, fill_children, TreeSnapshot};

pub fn snapshot(root: &Path, mode: Copies) -> R<TreeSnapshot> {
    if root.join(crate::config::TRASH_DIR).is_dir() {
        let _lock = StorageLock::acquire(root)?;
        mutations::recover_trashed(root)?;
    }
    let mut data = tree_store::load(root)?;
    let mut changed = tree_store::sync(root, &mut data)?;
    let mut copies = Vec::new();
    if changed || !footprint::scan(root, &data)?.is_clean() {
        // Reloading under the file lock keeps this save from overwriting another process's edit.
        let _lock = StorageLock::acquire(root)?;
        data = tree_store::load(root)?;
        changed = tree_store::sync(root, &mut data)?;
        let plan = footprint::plan(root, &mut data, mode)?;
        if plan.changed {
            tree_store::sync(root, &mut data)?;
            changed = true;
        }
        copies = plan.copies;
        if changed {
            tree_store::save(root, &data)?;
        }
    }
    let mut tree = build_tree(root, &data, changed)?;
    tree.copies = copies;
    fill_children(root, &data, &mut tree.nodes)?;
    search_index::rebuild_from_nodes(root, &tree.nodes)?;
    Ok(tree)
}
