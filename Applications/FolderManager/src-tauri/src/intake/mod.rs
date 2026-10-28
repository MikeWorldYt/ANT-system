// Sorts files dropped into the Storage tree: Node files go to their review folder, main-folder files are reported.
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
use crate::error::{FmError, R};
use crate::fs_safe::{io_err, Journal, StorageLock};
use crate::mutations::{child_rel, load_synced};
use rules::{is_exempt, is_skipped_file, rule, Rule};

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeReport {
    pub moved: Vec<Moved>,
    pub loose: Vec<LooseFile>,
    pub warnings: Vec<String>,
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

// The tree is only read: intake moves files, never folders, so nothing in it changes.
pub fn run(root: &Path) -> R<IntakeReport> {
    let _lock = StorageLock::acquire(root)?;
    let (data, _) = load_synced(root)?;
    let mut report = IntakeReport::default();
    walk(root, &data, &[], &mut report)?;
    Ok(report)
}

fn walk(dir: &Path, node: &Mapping, rel: &[String], report: &mut IntakeReport) -> R<()> {
    let subdirs = list_subdirs(dir)?;
    match rule(rel, node) {
        Rule::Skip => return Ok(()),
        Rule::Keep => {}
        Rule::Loose => report_loose(dir, rel, report)?,
        Rule::Review(review) => {
            let review = review.filter(|r| subdirs.iter().any(|d| d == r));
            move_to_review(dir, rel, review, report)?;
        }
    }
    // Tree entries missing on disk are ghosts and hold no files.
    for (name, child) in child_entries(node).filter(|(name, _)| subdirs.iter().any(|d| d == name)) {
        walk(&dir.join(name), child, &child_rel(rel, name), report)?;
    }
    Ok(())
}

fn report_loose(dir: &Path, rel: &[String], report: &mut IntakeReport) -> R<()> {
    let node = rel.join("/");
    for file in direct_files(dir)? {
        let file = file.to_string_lossy().into_owned();
        if !is_exempt(&file) {
            report.loose.push(LooseFile { node: node.clone(), file });
        }
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
        let shown = file.to_string_lossy();
        match journal.rename(&dir.join(&file), &target.join(&file)) {
            Ok(()) => count += 1,
            Err(FmError::AlreadyExists { .. }) => {
                report.warnings.push(format!("{node}/{shown}: {review} already has a file with this name, left in place"));
            }
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
