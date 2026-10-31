// Data derived from the tree after each change: search fields, 998.Review shortcuts and folder ids.
pub mod footprint;
pub mod node_names;
pub mod review_folders;
pub mod review_index;
pub mod review_update;
pub mod search_index;

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml::Mapping;

use crate::tree::tree_store;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewIndexEvent {
    pub(crate) phase: &'static str,
    pub(crate) failures: Vec<String>,
}

pub(crate) type Emit<'a> = &'a dyn Fn(&ReviewIndexEvent) -> Result<(), String>;

pub(crate) fn reindex(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> Vec<String> {
    report(emit, || match tree_store::load(root) {
        Ok(data) => rebuild(root, &data),
        Err(e) => vec![e.to_string()],
    })
}

// Updates only the touched branches and falls back to `rebuild` when that update cannot be trusted.
pub(crate) fn reindex_paths(
    root: &Path,
    emit: Emit,
    before: &[(Vec<String>, Mapping)],
    touched: &[Vec<String>],
    create: impl FnOnce(&[(PathBuf, PathBuf)]) -> Vec<String>,
    rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>,
) -> Vec<String> {
    report(emit, || {
        let data = match tree_store::load(root) {
            Ok(data) => data,
            Err(e) => return vec![e.to_string()],
        };
        let quick = match review_index::review_dir(root) {
            Ok(Some(dir)) => review_update::update_with(root, &dir, &data, before, touched, create),
            // The full rebuild owns the report for a missing or linked 998.Review, and does no work then.
            _ => return rebuild(root, &data),
        };
        match quick {
            Ok(failures) => failures,
            Err(e) => {
                let mut failures = vec![format!("the quick update failed, so it was rebuilt in full. {e}")];
                failures.extend(rebuild(root, &data));
                failures
            }
        }
    })
}

// Failures are returned instead of raised: the mutation behind them is already committed.
fn report(emit: Emit, work: impl FnOnce() -> Vec<String>) -> Vec<String> {
    let mut notes = Vec::new();
    let mut send = |phase, failures: Vec<String>| {
        if let Err(why) = emit(&ReviewIndexEvent { phase, failures }) {
            notes.push(format!("Could not report the review index progress: {why}"));
        }
    };
    send("started", Vec::new());
    let failures = work();
    send("done", failures.clone());
    let mut all = failures;
    all.extend(notes);
    all
}
