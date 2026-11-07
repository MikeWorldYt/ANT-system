// A workboard's deliverable moves to its sibling Outcomes folder, and a link is left in the workboard to find it again.
use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

use super::file_drop::copy_tree;
use super::{child_rel, editable_target, finish, scope, guard_frozen, load_synced_at, locate, parent_of, Mutation};
use crate::domain::config::is_hidden_dir;
use crate::domain::names::title_of;
use crate::infra::scan::list_subdirs;
use crate::platform::shortcuts::create_shortcuts;
use crate::domain::tree_types::type_base;
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{dir_entries, io_err, join_rel, resolve, validate_component, Journal, StorageLock};
use crate::infra::storage::info::{field_of, read_info, write_info};
use crate::services::tree::tree_store::{self, delete_key, get_type};

const OUTPUT_KEY: &str = "output";
const LINK_EXTENSION: &str = ".lnk";

pub fn set_output(root: &Path, rel: &[String], output: Option<&str>) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = editable_target(root, &data, rel, "edit")?;
    if type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards have an output".to_string() });
    }
    let value = match output {
        Some(raw) => {
            let path = clean_output(rel, raw)?;
            if !inside_workboard(&target.abs, &path)?.exists() {
                return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: format!("{path} is not inside this workboard") });
            }
            Some(Value::String(path))
        }
        None => None,
    };
    write_info(&target.abs, &[(&[OUTPUT_KEY][..], OUTPUT_KEY, value)])?;
    Ok(finish(root, &data, false, Some(rel.to_vec()), Vec::new(), scope(&synced, vec![rel.to_vec()])))
}

// Moves the deliverable into `(date)_Name` inside the Outcomes folder beside the workboard, and leaves `(date)_Name.lnk` behind.
pub fn deliver(root: &Path, rel: &[String], date: &str, all: bool) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost || type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards deliver an output".to_string() });
    }
    check_date(date)?;
    let outcomes = outcomes_beside(root, &data, rel)?;
    let sources = if all { children_of(&target.abs)? } else { output_sources(rel, &target.abs)? };
    let title = title_of(rel.last().map_or("", String::as_str));
    let (dest, name) = free_name(&outcomes, &format!("({date})_{title}"));
    fs::create_dir(&dest).map_err(|e| io_err("create outcome", &dest, &e))?;
    let mut journal = Journal::new();
    for src in &sources {
        let Some(file) = src.file_name() else { continue };
        if let Err(error) = journal.rename(src, &dest.join(file)) {
            return Err(journal.abort("deliver output", error));
        }
        if src.parent() == Some(target.abs.as_path()) {
            if let Some(file) = file.to_str() {
                delete_key(&mut data, rel, file);
            }
        }
    }
    journal.commit();
    tree_store::save(root, &data)?;
    // An output folder keeps its place, so the link waits inside it where the deliverable was.
    let link = link_dir(rel, &target.abs, all).join(&name);
    let warnings: Vec<String> = create_shortcuts(&[(dest, link)]);
    Ok(finish(root, &data, true, Some(rel.to_vec()), warnings, scope(&synced, vec![rel.to_vec()])))
}

// Brings back a copy of the deliverable for a workboard that is no longer Done, and removes its link; the Outcomes folder keeps it.
pub fn restore(root: &Path, rel: &[String]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, parent_of(rel))?;
    let target = locate(root, &data, rel)?;
    guard_frozen(rel, &target.node_type)?;
    if target.ghost || type_base(&target.node_type) != "WKB" {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "only workboards restore an output".to_string() });
    }
    let outcomes = outcomes_beside(root, &data, rel)?;
    let mut warnings = Vec::new();
    let mut found = delivery_links(&target.abs, &outcomes)?.into_iter().map(|l| (target.abs.clone(), l)).collect::<Vec<_>>();
    let output_dir = link_dir(rel, &target.abs, false);
    if output_dir != target.abs && output_dir.is_dir() {
        found.extend(delivery_links(&output_dir, &outcomes)?.into_iter().map(|l| (output_dir.clone(), l)));
    }
    for (dir, link) in found {
        let stem = link.trim_end_matches(LINK_EXTENSION);
        for file in children_of(&outcomes.join(stem))? {
            let Some(name) = file.file_name() else { continue };
            let to = dir.join(name);
            if to.exists() {
                warnings.push(format!("{} already exists, the delivered copy was not restored", name.to_string_lossy()));
                continue;
            }
            if file.is_dir() {
                copy_tree(&file, &to).map_err(|message| FmError::Internal { message })?;
            } else {
                fs::copy(&file, &to).map_err(|e| io_err("restore file", &to, &e))?;
            }
        }
        let link_path = dir.join(&link);
        fs::remove_file(&link_path).map_err(|e| io_err("remove link", &link_path, &e))?;
    }
    tree_store::sync_branch(root, &mut data, rel)?;
    tree_store::save(root, &data)?;
    Ok(finish(root, &data, true, Some(rel.to_vec()), warnings, scope(&synced, vec![rel.to_vec()])))
}

// The output path is kept relative to the workboard, with forward slashes and no leading slash.
fn clean_output(rel: &[String], raw: &str) -> R<String> {
    let path = raw.trim().replace('\\', "/");
    let path = path.trim_matches('/').to_string();
    let inside = !path.is_empty() && path.split('/').all(|part| validate_component(part).is_ok() && !is_hidden_dir(part));
    if !inside {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the output must be a path inside the workboard".to_string() });
    }
    Ok(path)
}

// The names alone cannot show a link or junction on the way out of the workboard.
fn inside_workboard(workboard: &Path, output: &str) -> R<PathBuf> {
    let parts: Vec<String> = output.split('/').map(str::to_string).collect();
    resolve(workboard, &parts)
}

// A workboard can arrive with a foreign `info.json`, so a value that fails the checks reads as no output.
fn stored_output(rel: &[String], workboard: &Path) -> Option<PathBuf> {
    let info = read_info(workboard)?;
    let output = clean_output(rel, field_of(&info, &[OUTPUT_KEY])?).ok()?;
    inside_workboard(workboard, &output).ok()
}

fn check_date(date: &str) -> R<()> {
    let shaped = (date.len() == 8 || date.len() == 10) && date.chars().all(|c| c.is_ascii_digit() || c == '-');
    if !shaped {
        return Err(FmError::InvalidName { name: date.to_string(), reason: "the date must read yy-mm-dd".to_string() });
    }
    Ok(())
}

// A file is moved as it is; a folder gives up its contents, not the folder itself.
fn output_sources(rel: &[String], workboard: &Path) -> R<Vec<PathBuf>> {
    let path = stored_output(rel, workboard).ok_or_else(|| FmError::InvalidPath {
        rel_parts: rel.to_vec(),
        reason: "no output is set for this workboard".to_string(),
    })?;
    if !path.exists() {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "the output is no longer there".to_string() });
    }
    if path.is_dir() {
        children_of(&path)
    } else {
        Ok(vec![path])
    }
}

// The output folder when one is set and still a folder, else the workboard itself.
fn link_dir(rel: &[String], workboard: &Path, all: bool) -> PathBuf {
    match stored_output(rel, workboard) {
        Some(dir) if !all && dir.is_dir() => dir,
        _ => workboard.to_path_buf(),
    }
}

// The Outcomes folder is the Outcomes-typed sibling of the workboard; a level without one has nothing to deliver to.
fn outcomes_beside(root: &Path, data: &serde_yaml::Mapping, rel: &[String]) -> R<PathBuf> {
    let parent = parent_of(rel);
    let parent_abs = join_rel(root, parent);
    for name in list_subdirs(&parent_abs)? {
        let sibling = child_rel(parent, &name);
        if type_base(&get_type(data, &sibling)) == "OUT" {
            return Ok(parent_abs.join(name));
        }
    }
    Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: "no Outcomes folder beside this workboard".to_string() })
}

// `(date)_Name` gets a numbered suffix when a delivery with that name already exists.
fn free_name(outcomes: &Path, base: &str) -> (PathBuf, String) {
    let mut name = base.to_string();
    let mut n = 2;
    while outcomes.join(&name).exists() {
        name = format!("{base} ({n})");
        n += 1;
    }
    (outcomes.join(&name), name)
}

// The app's own folders stay with the folder they describe: `.ant` holds its status, owner and lasting id.
fn children_of(dir: &Path) -> R<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).map_err(|e| io_err("list folder", dir, &e))?;
    let paths = entries.map(|e| e.map(|e| e.path()).map_err(|e| io_err("list folder", dir, &e))).collect::<R<Vec<PathBuf>>>()?;
    Ok(paths.into_iter().filter(|p| !p.file_name().is_some_and(|n| is_hidden_dir(&n.to_string_lossy()))).collect())
}

// Links in the workboard that name a delivery folder in the Outcomes folder.
fn delivery_links(workboard: &Path, outcomes: &Path) -> R<Vec<String>> {
    let mut links = Vec::new();
    for entry in dir_entries(workboard, "list folder")? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(LINK_EXTENSION).map(str::to_string) else { continue };
        if outcomes.join(&stem).is_dir() {
            links.push(name);
        }
    }
    Ok(links)
}

#[cfg(test)]
#[path = "outputs_tests.rs"]
mod tests;
