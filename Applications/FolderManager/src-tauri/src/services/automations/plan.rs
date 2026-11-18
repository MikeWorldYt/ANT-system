// The dry run of a review folder's automations, and the run of the rows the user kept from it.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::dict_match::{classify, cut_globals, renamed};
use super::dictionary;
use super::reduce_name::{affixed, dated_name, new_name, unbracketed_name, undated_name, unique_name};
use crate::domain::error::R;
use crate::domain::project_group::{self, Tier};
use crate::infra::fs_safe::{dir_entries, join_rel, validate_component};
use crate::services::mutations::{self, ProjectPlan};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub file: String,
    pub name: String,
    pub project: Option<String>,
    #[serde(default)]
    pub sure: bool,
    // The dictionary's subfolder the file moves into; such a file is left out of the project sort.
    #[serde(default)]
    pub folder: Option<String>,
    // The review folder of another node the file is sent to, when its dictionary entry names a node.
    #[serde(default)]
    pub target: Option<Vec<String>>,
    // The target's last folder is created by the run; the batch marks it as new.
    #[serde(default)]
    pub create: bool,
    // The dictionary heading that placed the file, so a drop on its group can pin more files to it.
    #[serde(default)]
    pub entry: Option<String>,
}

fn folder_files(root: &Path, folder: &[String]) -> R<Vec<String>> {
    let dir = join_rel(root, folder);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in dir_entries(&dir, "list folder")? {
        let entry = entry?;
        // "~$report.docx" and other "~" files are temporary copies an app holds open; hidden, system and dot files ("desktop.ini", ".gitignore") are kept out of sight for a reason. No automation touches them.
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_file() && !name.starts_with('~') && !crate::services::downloads::is_concealed(&entry) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Picks {
    pub reduce: bool,
    pub sort: bool,
    pub group: bool,
    pub add_date: bool,
    pub remove_date: bool,
    pub remove_symbols: bool,
}

// A file whose creation time cannot be read keeps its name rather than a made-up date.
fn created_secs(path: &Path) -> Option<i64> {
    let created = std::fs::metadata(path).and_then(|m| m.created()).ok()?;
    created.duration_since(std::time::UNIX_EPOCH).ok().and_then(|d| i64::try_from(d.as_secs()).ok())
}

// A rename after the grouping; names stay unique in each destination.
fn retitle(root: &Path, folder: &[String], rows: &mut [PlannedFile], change: impl Fn(&PlannedFile) -> String) -> R<()> {
    let mut taken: HashMap<Option<String>, HashSet<String>> = HashMap::new();
    for row in rows.iter() {
        if row.folder.is_some() && !taken.contains_key(&row.folder) {
            let dir = row.target.clone().unwrap_or_else(|| [folder, std::slice::from_ref(row.folder.as_ref().expect("checked above"))].concat());
            taken.insert(row.folder.clone(), folder_files(root, &dir)?.iter().map(|f| f.to_lowercase()).collect());
        }
        taken.entry(row.folder.clone()).or_default().insert(row.name.to_lowercase());
    }
    for row in rows.iter_mut() {
        let wanted = change(row);
        if wanted == row.name {
            continue;
        }
        let names = taken.entry(row.folder.clone()).or_default();
        names.remove(&row.name.to_lowercase());
        row.name = unique_name(&wanted, names);
        names.insert(row.name.to_lowercase());
    }
    Ok(())
}

// `only` narrows the rows to the picked files; the sort still groups against every file of the folder.
pub fn plan(root: &Path, folder: &[String], only: &[String], picks: Picks) -> R<Vec<PlannedFile>> {
    plan_with(root, folder, only, picks, dictionary::load(root, folder)?, &dictionary::load_globals(root, folder)?)
}

// The dictionary comes in already read, since the Downloads folder keeps its own outside the storage.
pub(super) fn plan_with(root: &Path, folder: &[String], only: &[String], picks: Picks, entries: Vec<dictionary::Entry>, globals: &dictionary::Globals) -> R<Vec<PlannedFile>> {
    let files = folder_files(root, folder)?;
    let dir = join_rel(root, folder);
    let mut taken: HashSet<String> = files.iter().map(|f| f.to_lowercase()).collect();
    let mut rows: Vec<PlannedFile> = files
        .iter()
        .map(|file| {
            let mut wanted = file.clone();
            if picks.reduce {
                wanted = new_name(&wanted);
            }
            // Removing wins over adding, so ticking both leaves names without a date.
            if picks.remove_date {
                wanted = undated_name(&wanted);
            } else if picks.add_date {
                wanted = dated_name(&wanted, created_secs(&dir.join(file)));
            }
            let mut name = file.clone();
            if wanted != *file {
                taken.remove(&file.to_lowercase());
                name = unique_name(&wanted, &taken);
                taken.insert(name.to_lowercase());
            }
            PlannedFile { file: file.clone(), name, project: None, sure: false, folder: None, target: None, create: false, entry: None }
        })
        .collect();
    if picks.group {
        group_by_dictionary(root, folder, &entries, &mut rows)?;
        // The global cuts come after the grouping, which read the names whole, so they never change where a file goes.
        if !globals.is_empty() {
            retitle(root, folder, &mut rows, |r| cut_globals(globals, &r.name))?;
        }
        // An entry's own prefix or suffix wins over the dictionary's, which reaches every file of the folder.
        let affix = |r: &PlannedFile| {
            let entry = r.entry.as_ref().and_then(|f| entries.iter().find(|e| e.folder == *f));
            let pick = |own: Option<&String>, all: &String| own.filter(|v| !v.is_empty()).unwrap_or(all).clone();
            affixed(&r.name, &pick(entry.map(|e| &e.prefix), &globals.prefix), &pick(entry.map(|e| &e.suffix), &globals.suffix))
        };
        retitle(root, folder, &mut rows, affix)?;
    }
    // Last of the renames, so the dictionary still reads the "(date)" and "[owner]" it places the topic after.
    if picks.remove_symbols {
        retitle(root, folder, &mut rows, |r| unbracketed_name(&r.name))?;
    }
    if picks.sort {
        let loose: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].folder.is_none()).collect();
        let names: Vec<String> = loose.iter().map(|&i| rows[i].name.clone()).collect();
        for g in project_group::group(&names) {
            for &m in &g.members {
                rows[loose[m]].project = Some(format!("PJ_{}", g.title));
                rows[loose[m]].sure = g.tier == Tier::Sure;
            }
        }
    }
    if !only.is_empty() {
        rows.retain(|r| only.contains(&r.file));
    }
    Ok(rows)
}

fn group_by_dictionary(root: &Path, folder: &[String], entries: &[dictionary::Entry], rows: &mut [PlannedFile]) -> R<()> {
    if entries.is_empty() {
        return Ok(());
    }
    // Names already in each subfolder, so a moved file never lands on one.
    let mut taken: HashMap<String, HashSet<String>> = HashMap::new();
    let mut here: HashSet<String> = rows.iter().map(|r| r.name.to_lowercase()).chain(folder_files(root, folder)?.iter().map(|f| f.to_lowercase())).collect();
    for row in rows.iter_mut() {
        let Some(entry) = classify(&entries, &row.file) else { continue };
        // "Do not group" still tidies the name, but the file stays in this folder.
        if !entry.group {
            if entry.folder.eq_ignore_ascii_case(dictionary::NO_GROUP) {
                let wanted = renamed(entry, &row.name);
                if wanted != row.name {
                    here.remove(&row.name.to_lowercase());
                    row.name = unique_name(&wanted, &here);
                    here.insert(row.name.to_lowercase());
                }
            }
            continue;
        }
        // A heading like ".." is no folder; the file stays and the lint flags it.
        if entry.node.is_none() && validate_component(&entry.folder).is_err() {
            continue;
        }
        let (sub, dir) = match &entry.node {
            Some(review) => (review.join("/"), review.clone()),
            None => (entry.folder.clone(), [folder, std::slice::from_ref(&entry.folder)].concat()),
        };
        if !taken.contains_key(&sub) {
            let here = folder_files(root, &dir)?;
            taken.insert(sub.clone(), here.iter().map(|f| f.to_lowercase()).collect());
        }
        let names = taken.get_mut(&sub).expect("inserted above");
        let wanted = renamed(entry, &row.name);
        row.name = unique_name(&wanted, names);
        names.insert(row.name.to_lowercase());
        row.folder = Some(sub);
        row.target = entry.node.clone();
        row.create = entry.create;
        row.entry = Some(entry.folder.clone());
    }
    Ok(())
}

// Renamed where it is, then dropped into the other node's review folder, which runs that folder's own automations on it.
fn send_to_node(root: &Path, folder: &[String], row: &PlannedFile, made: &mut HashMap<Vec<String>, Vec<String>>) -> R<()> {
    let Some(target) = &row.target else { return Ok(()) };
    let target = if row.create { super::node_path::ensure(root, target, made)? } else { target.clone() };
    if row.name != row.file {
        mutations::rename_file(root, &[folder, std::slice::from_ref(&row.file)].concat(), &row.name)?;
    }
    let path = join_rel(root, folder).join(&row.name);
    let no_bin = |_: &Path| Err("a dictionary move never replaces anything".to_string());
    mutations::drop_files(root, &target, &[path], false, &[], &no_bin)?;
    Ok(())
}

pub fn apply(root: &Path, folder: &[String], rows: &[PlannedFile]) -> R<usize> {
    let mut changed = 0;
    let moves: Vec<(String, String, String)> = rows.iter().filter(|r| r.target.is_none()).filter_map(|r| r.folder.clone().map(|sub| (r.file.clone(), sub, r.name.clone()))).collect();
    if !moves.is_empty() {
        mutations::move_into_folders(root, folder, &moves)?;
        changed += moves.len();
    }
    let mut made = HashMap::new();
    for row in rows.iter().filter(|r| r.target.is_some()) {
        send_to_node(root, folder, row, &mut made)?;
        changed += 1;
    }
    for row in rows.iter().filter(|r| r.folder.is_none() && r.name != r.file) {
        mutations::rename_file(root, &[folder, std::slice::from_ref(&row.file)].concat(), &row.name)?;
        changed += 1;
    }
    let mut picked: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for row in rows {
        if let (Some(project), None) = (&row.project, &row.folder) {
            picked.entry(project).or_default().push(row.name.clone());
        }
    }
    if picked.is_empty() {
        return Ok(changed);
    }
    // Owner and date come from the grouping of the folder as it now reads, after the renames.
    let plans: Vec<ProjectPlan> = mutations::preview_project_groups(root, folder)?
        .into_iter()
        .filter_map(|g| picked.remove(g.name.as_str()).map(|files| ProjectPlan { name: g.name, files, owner: g.owner, created: g.created }))
        .collect();
    changed += plans.iter().map(|p| p.files.len()).sum::<usize>();
    if !plans.is_empty() {
        mutations::apply_project_groups(root, folder, &plans)?;
    }
    Ok(changed)
}
