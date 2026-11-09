// Sorts the loose files of a review folder into untracked workboards, and can put the last sort back.
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

use super::add::mark_workboard;
use super::{child_rel, ensure_free, finish, load_synced_at, locate, persist, scope, Mutation};
use crate::domain::config::{DATA_DIR, OWNER_KEYS};
use crate::domain::error::{FmError, R};
use crate::domain::names::untracked_parts;
use crate::domain::project_group::{group, Tier};
use crate::domain::tree_types::type_base;
use crate::infra::fs_safe::{io_err, join_rel, validate_component, write_atomic, Journal, StorageLock};
use crate::infra::storage::info::write_info;
use crate::services::tree::depth;
use crate::services::tree::tree_store::{self, delete_key, get_node, get_node_mut};

const UNDO_FILENAME: &str = "last-project-sort.json";
const OWNER_UNKNOWN: &str = "unknown";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroup {
    pub name: String,
    pub files: Vec<String>,
    pub tier: Tier,
    pub owner: Option<String>,
    pub owner_conflict: bool,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPlan {
    pub name: String,
    pub files: Vec<String>,
    pub owner: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LastSort {
    folder: Vec<String>,
    projects: Vec<ProjectPlan>,
}

fn undo_file(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(UNDO_FILENAME)
}

fn check_in_review(data: &Mapping, folder: &[String]) -> R<()> {
    let reviewed = (1..=folder.len()).any(|end| get_node(data, &folder[..end]).and_then(|n| n.get("type")).and_then(Value::as_str).is_some_and(|t| type_base(t) == "REV"));
    if reviewed {
        return Ok(());
    }
    Err(FmError::InvalidPath { rel_parts: folder.to_vec(), reason: "only a review folder (REV.*) can be sorted into projects".to_string() })
}

pub fn preview_project_groups(root: &Path, folder: &[String]) -> R<Vec<ProjectGroup>> {
    let dir = join_rel(root, folder);
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| io_err("list folder", &dir, &e))? {
        let entry = entry.map_err(|e| io_err("list folder", &dir, &e))?;
        if entry.path().is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(group(&names)
        .into_iter()
        .map(|g| ProjectGroup {
            name: format!("PJ_{}", g.title),
            files: g.members.iter().map(|&m| names[m].clone()).collect(),
            tier: g.tier,
            owner: g.owner,
            owner_conflict: g.owner_conflict,
            created: g.created,
        })
        .collect())
}

fn free_name(dir: &Path, node: Option<&Mapping>, wanted: &str) -> R<String> {
    let mut name = wanted.to_string();
    for n in 2.. {
        match ensure_free(dir, node, &name, &[]) {
            Err(FmError::AlreadyExists { .. }) => name = format!("{wanted}-{n}"),
            other => return other.map(|()| name),
        }
    }
    unreachable!("the counter never ends")
}

fn fill_project(dir: &Path, plan: &ProjectPlan, parent: &str) -> R<()> {
    let owner = plan.owner.as_deref().unwrap_or(OWNER_UNKNOWN);
    let mut updates: Vec<(&[&str], &str, Option<Value>)> = vec![(&OWNER_KEYS, "owner", Some(owner.into()))];
    if let Some(created) = &plan.created {
        updates.push((&["created"], "created", Some(created.as_str().into())));
    }
    write_info(dir, &updates)?;
    mark_workboard(dir, parent, "in-progress")
}

pub fn apply_project_groups(root: &Path, folder: &[String], plans: &[ProjectPlan]) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, synced) = load_synced_at(root, folder)?;
    check_in_review(&data, folder)?;
    let to = locate(root, &data, folder)?;
    if to.ghost {
        return Err(FmError::GhostNotAllowed { op: "sort into projects".to_string() });
    }
    let limit = depth::limit(root)?;
    let mut journal = Journal::new();
    let mut placed = Vec::new();
    let mut made = Vec::new();
    for plan in plans {
        let attempt = (|| {
            if untracked_parts(&plan.name).is_none() {
                return Err(FmError::InvalidName { name: plan.name.clone(), reason: "a project in a review folder has no track number".to_string() });
            }
            validate_component(&plan.name)?;
            let name = free_name(&to.abs, get_node(&data, folder), &plan.name)?;
            let dir = to.abs.join(&name);
            journal.mkdir(&dir)?;
            for file in &plan.files {
                validate_component(file)?;
                journal.rename(&to.abs.join(file), &dir.join(file))?;
            }
            fill_project(&dir, plan, &folder.join("/"))?;
            Ok(name)
        })();
        let name = match attempt {
            Ok(name) => name,
            Err(e) => return Err(journal.abort("sort into projects", e)),
        };
        let rel = child_rel(folder, &name);
        if rel.len() <= limit {
            let mut entry = Mapping::new();
            entry.insert("type".into(), "WKB".into());
            if let Err(e) = tree_store::sync_subtree(root, &rel, &mut entry) {
                return Err(journal.abort("sort into projects", e));
            }
            let slot = get_node_mut(&mut data, folder).ok_or_else(|| FmError::NotFound { rel_parts: folder.to_vec() });
            match slot {
                Ok(slot) => slot.insert(name.clone().into(), entry.into()),
                Err(e) => return Err(journal.abort("sort into projects", e)),
            };
        }
        placed.push(ProjectPlan { name, ..plan.clone() });
        made.push(rel);
    }
    let mut touched = vec![folder.to_vec()];
    touched.extend(made);
    persist(root, &data, journal, "sort into projects", &scope(&synced, touched.clone()))?;
    let last = LastSort { folder: folder.to_vec(), projects: placed };
    let mut warnings = Vec::new();
    let text = serde_yaml::to_string(&last).map_err(|e| e.to_string()).and_then(|text| write_atomic(&undo_file(root), text.as_bytes()).map_err(|e| e.to_string()));
    if let Err(e) = text {
        warnings.push(format!("The sort cannot be undone: {e}"));
    }
    Ok(finish(root, &data, false, Some(folder.to_vec()), warnings, scope(&synced, touched)))
}

pub fn can_undo_project_sort(root: &Path) -> bool {
    undo_file(root).is_file()
}

pub fn undo_project_sort(root: &Path) -> R<Mutation> {
    let _lock = StorageLock::acquire(root)?;
    let path = undo_file(root);
    let text = fs::read_to_string(&path).map_err(|e| io_err("read the last sort", &path, &e))?;
    let last: LastSort = serde_yaml::from_str(&text).map_err(|e| FmError::Internal { message: format!("{UNDO_FILENAME}: {e}") })?;
    let (mut data, synced) = load_synced_at(root, &last.folder)?;
    let folder = join_rel(root, &last.folder);
    let mut journal = Journal::new();
    let mut warnings = Vec::new();
    let mut emptied = Vec::new();
    for project in &last.projects {
        let dir = folder.join(&project.name);
        for file in &project.files {
            if !dir.join(file).is_file() {
                warnings.push(format!("{file} is no longer in {}", project.name));
                continue;
            }
            if let Err(e) = journal.rename(&dir.join(file), &folder.join(file)) {
                return Err(journal.abort("undo the sort", e));
            }
        }
        emptied.push((project.name.as_str(), dir));
    }
    let mut touched = vec![last.folder.clone()];
    for (name, dir) in emptied {
        let others = fs::read_dir(&dir).map_err(|e| io_err("list folder", &dir, &e))?.flatten().any(|e| e.file_name() != DATA_DIR);
        if others {
            warnings.push(format!("{name} kept the files added after the sort"));
            continue;
        }
        fs::remove_dir_all(dir.join(DATA_DIR)).map_err(|e| io_err("remove folder", &dir, &e))?;
        fs::remove_dir(&dir).map_err(|e| io_err("remove folder", &dir, &e))?;
        delete_key(&mut data, &last.folder, name);
        touched.push(child_rel(&last.folder, name));
    }
    persist(root, &data, journal, "undo the sort", &scope(&synced, touched.clone()))?;
    fs::remove_file(&path).map_err(|e| io_err("remove the last sort", &path, &e))?;
    Ok(finish(root, &data, false, Some(last.folder.clone()), warnings, scope(&synced, touched)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;
    use crate::test_support::{rel, storage, TempDir};

    const TREE: &str = "100.WORK:\n  type: NODE-L1\n  98.WORK: {type: REV}\n  01.Plain: {type: NODE-L2}\n";
    const REVIEW: &[&str] = &["100.WORK", "98.WORK"];

    fn ready(tag: &str) -> TempDir {
        let dir = storage(tag, &["998.Review", "100.WORK/98.WORK", "100.WORK/01.Plain"], TREE);
        for file in ["Alpha Beta Gamma v1.pdf", "Alpha Beta Gamma v2.pdf", "notes.md"] {
            fs::write(dir.path().join("100.WORK/98.WORK").join(file), "x").unwrap();
        }
        dir
    }

    fn plan() -> ProjectPlan {
        ProjectPlan { name: "PJ_Alpha-Beta-Gamma".into(), files: vec!["Alpha Beta Gamma v1.pdf".into(), "Alpha Beta Gamma v2.pdf".into()], owner: Some("Ana".into()), created: Some("25-10-29".into()) }
    }

    #[test]
    fn the_preview_moves_nothing_and_leaves_lone_files_out() {
        let dir = ready("pg-preview");
        let groups = preview_project_groups(dir.path(), &rel(REVIEW)).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        assert!(dir.path().join("100.WORK/98.WORK/notes.md").is_file());
        assert!(dir.path().join("100.WORK/98.WORK/Alpha Beta Gamma v1.pdf").is_file());
    }

    #[test]
    fn applying_makes_a_workboard_and_undo_puts_everything_back() {
        let dir = ready("pg-apply");
        apply_project_groups(dir.path(), &rel(REVIEW), &[plan()]).unwrap();
        let board = dir.path().join("100.WORK/98.WORK/PJ_Alpha-Beta-Gamma");
        assert!(board.join("Alpha Beta Gamma v2.pdf").is_file());
        let info = fs::read_to_string(board.join(DATA_DIR).join("info.json")).unwrap();
        assert!(info.contains("Ana") && info.contains("25-10-29"), "{info}");
        assert!(can_undo_project_sort(dir.path()));
        undo_project_sort(dir.path()).unwrap();
        assert!(!board.exists());
        assert!(dir.path().join("100.WORK/98.WORK/Alpha Beta Gamma v2.pdf").is_file());
        assert!(!can_undo_project_sort(dir.path()));
    }

    #[test]
    fn a_missing_file_rolls_the_whole_sort_back() {
        let dir = ready("pg-rollback");
        let mut bad = plan();
        bad.files.push("gone.pdf".into());
        assert!(apply_project_groups(dir.path(), &rel(REVIEW), &[bad]).is_err());
        assert!(dir.path().join("100.WORK/98.WORK/Alpha Beta Gamma v1.pdf").is_file());
    }

    #[test]
    fn only_review_folders_are_sorted() {
        let dir = ready("pg-refuse");
        assert!(apply_project_groups(dir.path(), &rel(&["100.WORK", "01.Plain"]), &[plan()]).is_err());
    }

    fn contents(dir: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(at) = stack.pop() {
            for path in fs::read_dir(&at).unwrap().map(|e| e.unwrap().path()) {
                let key = path.strip_prefix(dir).unwrap().to_path_buf();
                if path.is_dir() {
                    out.insert(key, None);
                    stack.push(path);
                } else {
                    out.insert(key, Some(fs::read(&path).unwrap()));
                }
            }
        }
        out
    }

    // Few words and ids, so the preview finds projects to sort in most cases.
    fn file_name() -> impl Strategy<Value = String> {
        let words = prop::collection::vec(prop::sample::select(vec!["Alpha", "beta", "Gamma", "pool", "plans", "summer"]), 1..4);
        (words, prop::option::of(1040u32..1045), prop::option::of(2u32..4), prop::sample::select(vec!["pdf", "png", "docx"])).prop_map(|(words, id, copy, ext)| {
            format!("{}{}{}.{ext}", words.join(" "), id.map(|n| format!(" {n}")).unwrap_or_default(), copy.map(|n| format!(" ({n})")).unwrap_or_default())
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]

        #[test]
        fn apply_then_undo_leaves_the_review_folder_as_it_was(files in prop::collection::btree_map(file_name(), prop::collection::vec(any::<u8>(), 0..64), 1..10)) {
            let dir = ready("pg-prop");
            let review = dir.path().join("100.WORK/98.WORK");
            for (name, bytes) in &files {
                fs::write(review.join(name), bytes).unwrap();
            }
            let before = contents(&review);
            let plans: Vec<ProjectPlan> = preview_project_groups(dir.path(), &rel(REVIEW)).unwrap().into_iter().map(|g| ProjectPlan { name: g.name, files: g.files, owner: g.owner, created: g.created }).collect();
            apply_project_groups(dir.path(), &rel(REVIEW), &plans).unwrap();
            undo_project_sort(dir.path()).unwrap();
            prop_assert_eq!(contents(&review), before);
        }
    }

    #[test]
    fn a_taken_name_gets_a_suffix() {
        let dir = ready("pg-free");
        dir.mkdir("100.WORK/98.WORK/PJ_Alpha-Beta-Gamma");
        apply_project_groups(dir.path(), &rel(REVIEW), &[plan()]).unwrap();
        assert!(dir.path().join("100.WORK/98.WORK/PJ_Alpha-Beta-Gamma-2/Alpha Beta Gamma v1.pdf").is_file());
    }
}
