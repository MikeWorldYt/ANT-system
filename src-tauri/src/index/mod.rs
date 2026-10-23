// Data derived from the tree after each change: search fields, 998.Review shortcuts and folder ids.
pub mod footprint;
pub mod review_index;
pub mod search_index;

use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use crate::tree::tree_store;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReviewIndexEvent {
    pub(crate) phase: &'static str,
    pub(crate) failures: Vec<String>,
}

pub(crate) type Emit<'a> = &'a dyn Fn(&ReviewIndexEvent) -> Result<(), String>;

// Failures are returned instead of raised: the mutation behind them is already committed.
pub(crate) fn reindex(root: &Path, emit: Emit, rebuild: impl FnOnce(&Path, &Mapping) -> Vec<String>) -> Vec<String> {
    let mut notes = Vec::new();
    let mut send = |phase, failures: Vec<String>| {
        if let Err(why) = emit(&ReviewIndexEvent { phase, failures }) {
            notes.push(format!("Could not report the review index progress: {why}"));
        }
    };
    send("started", Vec::new());
    let failures = match tree_store::load(root) {
        Ok(data) => rebuild(root, &data),
        Err(e) => vec![e.to_string()],
    };
    send("done", failures.clone());
    let mut all = failures;
    all.extend(notes);
    all
}
