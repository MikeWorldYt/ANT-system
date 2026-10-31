// Sorts files dropped into the Storage tree: Node files go to their review folder, while data and special nodes keep them.
mod rules;
#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use crate::ant_core::names::list_subdirs;
use crate::ant_core::tree_types::child_entries;
use crate::config::DATA_DIR;
use crate::error::R;
use crate::fs_safe::{io_err, join_rel, Journal, StorageLock};
use crate::index::review_folders;
use crate::mutations::{child_rel, free_name, load_synced};
use crate::storage::db::hide;
use rules::{is_root_exempt_file, is_skipped_file, rule, Rule};
pub(crate) use rules::{node_type, review_child};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeReport {
    pub moved: Vec<Moved>,
    pub loose: Vec<LooseFile>,
    pub waiting: Vec<WaitingNode>,
    pub warnings: Vec<String>,
}

// A node the app has never marked, with loose files in it: its files move only once the user says so.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingNode {
    pub node: String,
    pub rel_parts: Vec<String>,
    pub files: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    pub node: String,
    pub name: String,
    pub count: u32,
    pub review: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LooseFile {
    pub node: String,
    pub file: String,
}

// Intake moves files, never folders; the only folder it makes is a missing 98.* review folder, so every node has one to take files.
pub fn run(root: &Path) -> R<IntakeReport> {
    review_folders::restore(root)?;
    let _lock = StorageLock::acquire(root)?;
    let (data, _) = load_synced(root)?;
    let mut report = IntakeReport::default();
    walk(root, &data, &[], &mut report)?;
    Ok(report)
}

fn walk(dir: &Path, node: &Mapping, rel: &[String], report: &mut IntakeReport) -> R<()> {
    let subdirs = list_subdirs(dir)?;
    match rule(rel, node) {
        Rule::Root => report_root_loose(dir, report)?,
        Rule::Skip => return Ok(()),
        Rule::Keep => {}
        Rule::Loose => report_loose(dir, rel, report)?,
        Rule::Review(review) => {
            let files = waiting_files(dir, rel, node)?;
            if files > 0 {
                report.waiting.push(WaitingNode { node: rel.join("/"), rel_parts: rel.to_vec(), files });
            } else {
                let review = review.filter(|r| subdirs.iter().any(|d| d == r));
                move_to_review(dir, rel, review, report)?;
            }
        }
    }
    // Tree entries missing on disk are ghosts and hold no files.
    for (name, child) in child_entries(node).filter(|(name, _)| subdirs.iter().any(|d| d == name)) {
        walk(&dir.join(name), child, &child_rel(rel, name), report)?;
    }
    Ok(())
}

// Zero unless the node is one the app has not marked yet and has loose files; footprint leaves such a node unmarked until the user answers.
pub(crate) fn waiting_files(dir: &Path, rel: &[String], node: &Mapping) -> R<usize> {
    if !matches!(rule(rel, node), Rule::Review(_)) || dir.join(DATA_DIR).exists() {
        return Ok(0);
    }
    Ok(direct_files(dir)?.len())
}

// The user let intake take a node's loose files; the marker folder lets footprint track the node like the others.
pub fn acknowledge(root: &Path, rel: &[String]) -> R<()> {
    let _lock = StorageLock::acquire(root)?;
    let dir = join_rel(root, rel).join(DATA_DIR);
    if !dir.is_dir() {
        fs::create_dir(&dir).map_err(|e| io_err("create info folder", &dir, &e))?;
        hide(&dir);
    }
    Ok(())
}

fn report_root_loose(dir: &Path, report: &mut IntakeReport) -> R<()> {
    for file in direct_files(dir)? {
        let file = file.to_string_lossy().into_owned();
        if !is_root_exempt_file(&file) {
            report.loose.push(LooseFile { node: String::new(), file });
        }
    }
    Ok(())
}

fn report_loose(dir: &Path, rel: &[String], report: &mut IntakeReport) -> R<()> {
    let node = rel.join("/");
    for file in direct_files(dir)? {
        let file = file.to_string_lossy().into_owned();
        report.loose.push(LooseFile { node: node.clone(), file });
    }
    Ok(())
}

fn move_to_review(dir: &Path, rel: &[String], review: Option<&str>, report: &mut IntakeReport) -> R<()> {
    let files = direct_files(dir)?;
    if files.is_empty() {
        return Ok(());
    }
    let node = rel.join("/");
    let Some(review) = review else {
        report.warnings.push(format!("{node}: no review folder, {} file(s) left in place", files.len()));
        return Ok(());
    };
    let target = dir.join(review);
    let mut journal = Journal::new();
    let mut count = 0;
    for file in files {
        let shown = file.to_string_lossy().into_owned();
        let landing = match free_name(&target, &shown, &[]) {
            Ok(name) => name,
            Err(e) => {
                report.warnings.push(format!("{node}/{shown}: not moved, {e}"));
                continue;
            }
        };
        match journal.rename(&dir.join(&file), &target.join(&landing)) {
            Ok(()) => count += 1,
            // A file still being copied in stays locked, so it is reported and the other files still move.
            Err(e) => report.warnings.push(format!("{node}/{shown}: not moved, {e}")),
        }
    }
    journal.commit();
    if count > 0 {
        let name = rel.last().cloned().unwrap_or_default();
        report.moved.push(Moved { node, name, count, review: review.to_string() });
    }
    Ok(())
}

fn direct_files(dir: &Path) -> R<Vec<OsString>> {
    let entries = fs::read_dir(dir).map_err(|e| io_err("list folder", dir, &e))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        let kind = entry.file_type().map_err(|e| io_err("inspect path", &entry.path(), &e))?;
        let name = entry.file_name();
        if kind.is_file() && !is_skipped_file(&name.to_string_lossy()) {
            files.push(name);
        }
    }
    files.sort();
    Ok(files)
}
