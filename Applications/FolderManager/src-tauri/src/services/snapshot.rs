// The snapshot the UI loads: the stored tree synced with disk, ids assigned and the search index rebuilt.
use std::path::Path;

use crate::domain::model::Copies;
use crate::domain::error::R;
use crate::infra::fs_safe::StorageLock;
use crate::services::index::footprint;
use crate::services::index::search_index;
use crate::services::tree::tree_node::{build_tree, fill_children, TreeSnapshot};
use crate::services::tree::tree_store;

pub fn snapshot(root: &Path, mode: Copies) -> R<TreeSnapshot> {
    let mut data = tree_store::load(root)?;
    let mut changed = tree_store::sync(root, &mut data)?;
    changed |= tree_store::reconcile_portfolios_all(root, &mut data)?;
    let mut copies = Vec::new();
    let mut warnings = Vec::new();
    if changed || !footprint::scan(root, &data)?.is_clean() {
        // Reloading under the file lock keeps this save from overwriting another process's edit.
        let _lock = StorageLock::acquire(root)?;
        data = tree_store::load(root)?;
        changed |= tree_store::sync(root, &mut data)?;
        changed |= tree_store::reconcile_portfolios_all(root, &mut data)?;
        let plan = footprint::plan(root, &mut data, mode)?;
        if plan.changed {
            tree_store::sync(root, &mut data)?;
            tree_store::reconcile_portfolios_all(root, &mut data)?;
            changed = true;
        }
        copies = plan.copies;
        warnings = plan.warnings;
        if changed {
            tree_store::save(root, &data)?;
        }
    }
    let mut tree = build_tree(root, &data, changed)?;
    tree.copies = copies;
    tree.warnings = warnings;
    fill_children(root, &data, &mut tree.nodes)?;
    search_index::rebuild_from_nodes(root, &tree.nodes)?;
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, TempDir};
    use crate::services::tree::depth;
    use crate::services::tree::tree_node::TreeNode;
    use crate::services::tree::tree_store::get_node;

    #[test]
    fn a_raised_depth_limit_loads_and_opens_the_next_level() {
        let dir = TempDir::new("raised-depth");
        dir.mkdir("01.A/01.B/01.C/01.D/01.E");
        crate::infra::storage::db::Store::open_or_create(dir.path()).unwrap().set_setting(depth::DEPTH_KEY, "4").unwrap();
        let tree = snapshot(dir.path(), Copies::Tail).unwrap();
        let child = |n: &TreeNode| n.children.as_ref().map(|k| k[0].clone()).unwrap();
        let l3 = child(&child(&tree.nodes[0]));
        assert!(l3.has_children && l3.actions.change_type && l3.actions.add_next);
        let l4 = child(&l3);
        assert_eq!((l4.name.as_str(), l4.depth), ("01.D", 3));
        assert!(l4.has_children && l4.actions.change_type && l4.actions.add_next);
        let data = tree_store::load(dir.path()).unwrap();
        assert_eq!(tree_store::get_type(&data, &rel(&["01.A", "01.B", "01.C", "01.D"])), "NODE-L4");
        assert!(get_node(&data, &rel(&["01.A", "01.B", "01.C", "01.D", "01.E"])).is_none());
    }
}
