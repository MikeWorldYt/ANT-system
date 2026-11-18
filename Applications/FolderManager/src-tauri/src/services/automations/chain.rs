// Files a Downloads run drops in a review folder that runs its automations on its own are filed on at once, so the run can tell where they really ended.
use std::path::Path;

use serde::Serialize;

use crate::domain::config::{AUTO_ADD_DATE_FLAG, AUTO_GROUP_FLAG, AUTO_REDUCE_FLAG, AUTO_REMOVE_DATE_FLAG, AUTO_REMOVE_SYMBOLS_FLAG};
use crate::domain::error::R;
use crate::domain::tree_types::{flags_in, runs_automations, type_base};
use crate::services::tree::tree_store;

use super::plan::{apply, plan, Picks};

// What a run changed, and where the files a review folder passed on went next.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyOutcome {
    pub changed: usize,
    pub chained: Vec<Chained>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chained {
    pub name: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
}

pub fn follow(root: &Path, folder: &[String], files: &[String]) -> R<Vec<Chained>> {
    let data = tree_store::load_paths(root, &[folder.to_vec()])?;
    let Some(node) = tree_store::get_node(&data, folder) else { return Ok(Vec::new()) };
    let flags = flags_in(node);
    if !runs_automations(type_base(node.get("type").and_then(serde_yaml::Value::as_str).unwrap_or("")), &flags) {
        return Ok(Vec::new());
    }
    let has = |flag: &str| flags.iter().any(|f| f == flag);
    let picks = Picks { reduce: has(AUTO_REDUCE_FLAG), add_date: has(AUTO_ADD_DATE_FLAG), remove_date: has(AUTO_REMOVE_DATE_FLAG), group: has(AUTO_GROUP_FLAG), remove_symbols: has(AUTO_REMOVE_SYMBOLS_FLAG), ..Picks::default() };
    if !(picks.reduce || picks.add_date || picks.remove_date || picks.group || picks.remove_symbols) {
        return Ok(Vec::new());
    }
    let rows: Vec<_> = plan(root, folder, files, picks)?.into_iter().filter(|r| r.name != r.file || r.folder.is_some()).collect();
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    apply(root, folder, &rows)?;
    // A folder the run created has a name the plan could not know, so the trip ends at its parent.
    Ok(rows.into_iter().filter_map(|r| r.target.map(|t| Chained { name: r.name, from: folder.to_vec(), to: if r.create { t[..t.len().saturating_sub(1)].to_vec() } else { t } })).collect())
}
