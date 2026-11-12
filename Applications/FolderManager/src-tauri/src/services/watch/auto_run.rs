// Runs the automations a review folder has switched on for the files that just landed in it, whoever put them there.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::filter::{is_own_write, relative};
use super::log;
use crate::domain::config::{AUTO_ADD_DATE_FLAG, AUTO_GROUP_FLAG, AUTO_REDUCE_FLAG, AUTO_REMOVE_DATE_FLAG, AUTO_REMOVE_SYMBOLS_FLAG, AUTO_SORT_FLAG};
use crate::domain::project_group::Tier;
use crate::domain::tree_types::{flags_in, type_base};
use crate::services::automations::{self, reduce_files};
use crate::services::mutations::{self, ProjectPlan};
use crate::services::tree::tree_store;

pub(super) fn run(root: &Path, changed: &[PathBuf]) {
    let mut landed: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for path in changed.iter().filter(|p| p.is_file()) {
        let Some(mut rel) = relative(root, path).filter(|rel| rel.len() > 1 && !is_own_write(rel)) else { continue };
        let Some(name) = rel.pop() else { continue };
        landed.entry(rel).or_default().push(name);
    }
    if landed.is_empty() {
        return;
    }
    let data = match tree_store::load_paths(root, &landed.keys().cloned().collect::<Vec<_>>()) {
        Ok(data) => data,
        Err(e) => return log("error", &format!("Automations: tree not read: {e}")),
    };
    for (folder, files) in landed {
        let Some(node) = tree_store::get_node(&data, &folder) else { continue };
        if type_base(node.get("type").and_then(serde_yaml::Value::as_str).unwrap_or("")) != "REV" {
            continue;
        }
        let flags = flags_in(node);
        if flags.iter().any(|f| f == AUTO_REDUCE_FLAG) {
            reduce(root, &folder, &files);
        }
        let has = |flag: &str| flags.iter().any(|f| f == flag);
        let picks = automations::Picks { add_date: has(AUTO_ADD_DATE_FLAG), remove_date: has(AUTO_REMOVE_DATE_FLAG), remove_symbols: has(AUTO_REMOVE_SYMBOLS_FLAG), ..Default::default() };
        if picks.add_date || picks.remove_date || picks.remove_symbols {
            retouch(root, &folder, &files, picks);
        }
        if flags.iter().any(|f| f == AUTO_GROUP_FLAG) {
            group(root, &folder, &files);
        }
        if flags.iter().any(|f| f == AUTO_SORT_FLAG) {
            sort(root, &folder);
        }
    }
}

// The new names come back as another batch, where they already read as reduced and stay.
fn reduce(root: &Path, folder: &[String], files: &[String]) {
    let rels: Vec<Vec<String>> = files.iter().map(|f| [folder, std::slice::from_ref(f)].concat()).collect();
    if let Err(e) = reduce_files(root, &rels) {
        log("error", &format!("Automations: names in {} not reduced: {e}", folder.join("/")));
    }
}

// Without a preview to confirm them, only the groups the sort is sure of are made.
fn sort(root: &Path, folder: &[String]) {
    let groups = match mutations::preview_project_groups(root, folder) {
        Ok(groups) => groups,
        Err(e) => return log("error", &format!("Automations: {} not sorted: {e}", folder.join("/"))),
    };
    let plans: Vec<ProjectPlan> = groups.into_iter().filter(|g| g.tier == Tier::Sure).map(|g| ProjectPlan { name: g.name, files: g.files, owner: g.owner, created: g.created }).collect();
    if plans.is_empty() {
        return;
    }
    if let Err(e) = mutations::apply_project_groups(root, folder, &plans) {
        log("error", &format!("Automations: {} not sorted: {e}", folder.join("/")));
    }
}

// A file the reduce renamed in this same batch is gone under its old name, so only the names still there are retouched.
fn retouch(root: &Path, folder: &[String], files: &[String], picks: automations::Picks) {
    let result = automations::plan(root, folder, files, picks).and_then(|rows| {
        let renamed: Vec<_> = rows.into_iter().filter(|r| r.name != r.file).collect();
        if renamed.is_empty() { Ok(0) } else { automations::apply(root, folder, &renamed) }
    });
    if let Err(e) = result {
        log("error", &format!("Automations: names in {} not retouched: {e}", folder.join("/")));
    }
}

// A file the reduce renamed in this same batch is gone under its old name, so only the names still there are grouped.
fn group(root: &Path, folder: &[String], files: &[String]) {
    let result = automations::plan(root, folder, files, automations::Picks { group: true, ..Default::default() }).and_then(|rows| {
        let moving: Vec<_> = rows.into_iter().filter(|r| r.folder.is_some()).collect();
        if moving.is_empty() { Ok(0) } else { automations::apply(root, folder, &moving) }
    });
    if let Err(e) = result {
        log("error", &format!("Automations: {} not grouped: {e}", folder.join("/")));
    }
}
