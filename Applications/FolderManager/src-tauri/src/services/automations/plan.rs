// The dry run of a review folder's automations, and the run of the rows the user kept from it.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::dict_match::{classify, renamed};
use super::dictionary;
use super::reduce_name::{new_name, unique_name};
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
}

fn folder_files(root: &Path, folder: &[String]) -> R<Vec<String>> {
    let dir = join_rel(root, folder);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in dir_entries(&dir, "list folder")? {
        let entry = entry?;
        if entry.path().is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

// `only` narrows the rows to the picked files; the sort still groups against every file of the folder.
pub fn plan(root: &Path, folder: &[String], only: &[String], reduce: bool, sort: bool, group: bool) -> R<Vec<PlannedFile>> {
    let files = folder_files(root, folder)?;
    let mut taken: HashSet<String> = files.iter().map(|f| f.to_lowercase()).collect();
    let mut rows: Vec<PlannedFile> = files
        .iter()
        .map(|file| {
            let mut name = file.clone();
            if reduce {
                let reduced = new_name(file);
                if reduced != *file {
                    taken.remove(&file.to_lowercase());
                    name = unique_name(&reduced, &taken);
                    taken.insert(name.to_lowercase());
                }
            }
            PlannedFile { file: file.clone(), name, project: None, sure: false, folder: None }
        })
        .collect();
    if group {
        group_by_dictionary(root, folder, &mut rows)?;
    }
    if sort {
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
    for row in rows.iter_mut() {
        let Some(entry) = classify(&entries, &row.file).filter(|e| e.group) else { continue };
        let sub = entry.folder.clone();
        if !taken.contains_key(&sub) {
            let here = folder_files(root, &[folder, std::slice::from_ref(&sub)].concat())?;
            taken.insert(sub.clone(), here.iter().map(|f| f.to_lowercase()).collect());
        }
        let names = taken.get_mut(&sub).expect("inserted above");
        let wanted = renamed(entry, &row.name);
        row.name = unique_name(&wanted, names);
        names.insert(row.name.to_lowercase());
        row.folder = Some(sub);
    }
    Ok(())
}

pub fn apply(root: &Path, folder: &[String], rows: &[PlannedFile]) -> R<usize> {
    let mut changed = 0;
    let moves: Vec<(String, String, String)> = rows.iter().filter_map(|r| r.folder.clone().map(|sub| (r.file.clone(), sub, r.name.clone()))).collect();
    if !moves.is_empty() {
        mutations::move_into_folders(root, folder, &moves)?;
        changed += moves.len();
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
