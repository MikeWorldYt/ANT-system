// The per-node action rules: what the UI may offer on a folder, a file or a review link.
use serde::Serialize;

use super::TreeNode;
use crate::domain::names::{is_movable_node, numeric_id};
use crate::domain::tree_types::type_base;
use crate::services::tree::tree_store::is_locked_type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeActions {
    pub rename: bool,
    pub change_type: bool,
    pub add_next: bool,
    pub add_child: bool,
    pub add_folder: bool,
    pub move_up: bool,
    pub move_down: bool,
    pub delete: bool,
    pub copy_path: bool,
    pub open: bool,
    pub favorite: bool,
    pub important: bool,
    pub edit_details: bool,
    pub edit_flags: bool,
    pub hide_link: bool,
    pub hide: bool,
}

// Files carry no tree entry, so only what acts on the disk applies.
pub(super) fn file_actions() -> NodeActions {
    NodeActions {
        rename: true,
        change_type: false,
        add_next: false,
        add_child: false,
        add_folder: false,
        move_up: false,
        move_down: false,
        delete: true,
        copy_path: true,
        open: true,
        favorite: false,
        important: false,
        edit_details: false,
        edit_flags: false,
        hide_link: false,
        hide: false,
    }
}

// A review link only opens its owner folder; files reach the owner through the drop target instead.
pub(super) fn link_actions() -> NodeActions {
    NodeActions {
        rename: false,
        change_type: false,
        add_next: false,
        add_child: false,
        add_folder: false,
        move_up: false,
        move_down: false,
        delete: false,
        copy_path: true,
        open: true,
        favorite: true,
        important: false,
        edit_details: false,
        edit_flags: false,
        hide_link: true,
        hide: false,
    }
}

// The only place that decides what a node allows; move flags are filled in by `mark_moves`.
pub(super) fn compute_actions(depth: usize, limit: usize, ghost: bool, raw_type: &str, reserved: bool, info_broken: bool) -> NodeActions {
    let locked = is_locked_type(raw_type);
    let frozen = locked || reserved;
    // The tree stops at the depth limit, so deeper folders have no entry to retype or to add a sibling under.
    let in_tree = depth < limit;
    let outcome = type_base(raw_type) == "OUT";
    NodeActions {
        rename: !ghost && !frozen,
        change_type: !frozen && in_tree && type_base(raw_type) != "OUT",
        // Main categories can create another Level 1 category; deeper nodes create siblings as usual.
        add_next: !ghost && !reserved && in_tree,
        // Mirrors add_node: the new child sits one level below this node and must stay inside the limit.
        add_child: !ghost && !frozen && depth + 1 < limit,
        // Past the limit a child is a plain folder or a workboard the database does not track.
        add_folder: !ghost && !frozen && depth + 1 >= limit,
        move_up: false,
        move_down: false,
        delete: !reserved && (ghost || !locked) && !outcome,
        copy_path: !ghost,
        open: !ghost,
        // A broken info.json is never overwritten, so neither edit can be saved into it.
        favorite: !reserved && !ghost && !info_broken && !outcome,
        important: !frozen && !ghost && !info_broken && type_base(raw_type) == "WKB",
        edit_details: !frozen && !ghost && !info_broken && type_base(raw_type) == "WKB",
        edit_flags: !frozen && !ghost && depth > 0 && type_base(raw_type) == "NODE",
        hide_link: false,
        hide: !frozen && !ghost && depth > 0,
    }
}

// Moves swap ids with the nearest movable sibling, so the ends of the movable list cannot go further.
pub(super) fn mark_moves(nodes: &mut [TreeNode]) {
    let mut movable: Vec<(u32, usize)> = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| is_movable(n))
        .filter_map(|(i, n)| numeric_id(&n.name).map(|id| (id, i)))
        .collect();
    movable.sort();
    let last = movable.len().saturating_sub(1);
    for (pos, (_, idx)) in movable.iter().enumerate() {
        nodes[*idx].actions.move_up = pos > 0;
        nodes[*idx].actions.move_down = pos < last;
    }
}

fn is_movable(node: &TreeNode) -> bool {
    let raw = node.node_type.as_ref().map_or("", |t| t.raw.as_str());
    !node.file && !node.ghost && !node.locked && !node.reserved && !node.linked && is_movable_node(&node.name, raw)
}
