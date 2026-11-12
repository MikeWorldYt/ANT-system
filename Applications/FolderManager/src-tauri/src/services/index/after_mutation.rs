// Brings the indexes up to date after a mutation, whoever ran it.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use super::{reindex, reindex_paths, search_index, Emit};
use crate::domain::error::R;
use crate::services::mutations::Mutation;
use crate::services::tree::tree_store;

pub(crate) fn with_index(
    root: &Path,
    result: R<Mutation>,
    emit: Emit,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> R<Mutation> {
    let mut m = result?;
    // An unknown scope (no touched paths) or a failed reload leaves nothing to update in place, so it is rebuilt in full.
    let failures = if known_scope(&m) {
        reindex_paths(root, emit, &m.before, &m.touched, create, rebuild)
    } else {
        reindex(root, emit, rebuild)
    };
    m.warnings.extend(failures.into_iter().map(|f| format!("Review index: {f}")));
    with_search_index(root, Ok(m))
}

pub(crate) fn with_search_index(root: &Path, result: R<Mutation>) -> R<Mutation> {
    let mut m = result?;
    if known_scope(&m) {
        match search_index::update_paths(root, &m.touched_nodes(), &m.touched) {
            Ok(()) => return Ok(m),
            Err(e) => m.warnings.push(format!("Search index: the quick update failed, so it was rebuilt in full. {e}")),
        }
    }
    // after_commit's empty fallback has no tree, so the tree is indexed instead of wiping the index.
    let indexed = match m.tree.as_ref().filter(|t| !t.nodes.is_empty()) {
        Some(tree) => search_index::rebuild_from_nodes(root, &tree.nodes),
        None => tree_store::load(root).and_then(|data| search_index::rebuild(root, &data)),
    };
    if let Err(e) = indexed {
        m.warnings.push(format!("Search index: {e}"));
    }
    Ok(m)
}

fn known_scope(m: &Mutation) -> bool {
    !m.touched.is_empty() && m.tree.as_ref().is_none_or(|t| !t.nodes.is_empty())
}
