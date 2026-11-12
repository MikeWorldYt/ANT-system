// The dry run of a review folder's automations, and the run of the rows the user kept from it.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::dict_match::{classify, renamed};
use super::dictionary;
use super::reduce_name::{dated_name, new_name, undated_name, unique_name};
use crate::domain::error::R;
use crate::domain::project_group::{self, Tier};
use crate::infra::fs_safe::{dir_entries, join_rel};
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
}

fn folder_files(root: &Path, folder: &[String]) -> R<Vec<String>> {
    let dir = join_rel(root, folder);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in dir_entries(&dir, "list folder")? {
        let entry = entry?;
        // "~$report.docx" and other "~" files are temporary copies an app holds open; no automation touches them.
        if entry.path().is_file() && !entry.file_name().to_string_lossy().starts_with('~') {
            names.push(entry.file_name().to_string_lossy().into_owned());
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
}

// A file whose creation time cannot be read keeps its name rather than a made-up date.
fn created_secs(path: &Path) -> Option<i64> {
    let created = std::fs::metadata(path).and_then(|m| m.created()).ok()?;
    created.duration_since(std::time::UNIX_EPOCH).ok().and_then(|d| i64::try_from(d.as_secs()).ok())
}

// `only` narrows the rows to the picked files; the sort still groups against every file of the folder.
pub fn plan(root: &Path, folder: &[String], only: &[String], picks: Picks) -> R<Vec<PlannedFile>> {
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
            PlannedFile { file: file.clone(), name, project: None, sure: false, folder: None, target: None }
        })
        .collect();
    if picks.group {
        group_by_dictionary(root, folder, &mut rows)?;
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

fn group_by_dictionary(root: &Path, folder: &[String], rows: &mut [PlannedFile]) -> R<()> {
    let entries = dictionary::load(root, folder)?;
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
    }
    Ok(())
}

// Renamed where it is, then dropped into the other node's review folder, which runs that folder's own automations on it.
fn send_to_node(root: &Path, folder: &[String], row: &PlannedFile) -> R<()> {
    let Some(target) = &row.target else { return Ok(()) };
    if row.name != row.file {
        mutations::rename_file(root, &[folder, std::slice::from_ref(&row.file)].concat(), &row.name)?;
    }
    let path = join_rel(root, folder).join(&row.name);
    let no_bin = |_: &Path| Err("a dictionary move never replaces anything".to_string());
    mutations::drop_files(root, target, &[path], false, &[], &no_bin)?;
    Ok(())
}

pub fn apply(root: &Path, folder: &[String], rows: &[PlannedFile]) -> R<usize> {
    let mut changed = 0;
    let moves: Vec<(String, String, String)> = rows.iter().filter(|r| r.target.is_none()).filter_map(|r| r.folder.clone().map(|sub| (r.file.clone(), sub, r.name.clone()))).collect();
    if !moves.is_empty() {
        mutations::move_into_folders(root, folder, &moves)?;
        changed += moves.len();
    }
    for row in rows.iter().filter(|r| r.target.is_some()) {
        send_to_node(root, folder, row)?;
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
