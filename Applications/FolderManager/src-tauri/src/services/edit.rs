// The whole edit flow without the app: a mutation plus what must follow it, for any caller that holds the storage.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use crate::domain::error::R;
use crate::platform::shortcuts::create_shortcuts;
use crate::services::index::after_mutation::{with_index, with_search_index};
use crate::services::index::{review_index, Emit};
use crate::services::mutations::{self, Mutation};

// For mutations that can add, move or drop 998.Review links.
pub(crate) fn indexed(root: &Path, emit: Emit, f: impl FnOnce(&Path) -> R<Mutation>) -> R<Mutation> {
    with_index(root, f(root), emit, create_shortcuts, review_index::rebuild)
}

pub(crate) fn search_indexed(root: &Path, f: impl FnOnce(&Path) -> R<Mutation>) -> R<Mutation> {
    with_search_index(root, f(root))
}

pub(crate) fn delete_node(
    root: &Path,
    rel: &[String],
    flags: (bool, bool),
    emit: Emit,
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> R<Mutation> {
    let m = mutations::delete_node(root, rel, flags.0, flags.1, recycle)?;
    with_index(root, Ok(m), emit, create, rebuild)
}
