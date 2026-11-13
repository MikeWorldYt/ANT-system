// Reduces the names of a set of files, each kept unique among the other names of its folder.
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use super::reduce_name::{new_name, unique_name};
use crate::domain::error::R;
use crate::infra::fs_safe::{dir_entries, join_rel};
use crate::services::mutations;

pub fn reduce_files(root: &Path, files: &[Vec<String>]) -> R<usize> {
    let mut by_folder: BTreeMap<&[String], Vec<&String>> = BTreeMap::new();
    for rel in files {
        if let Some((name, folder)) = rel.split_last() {
            by_folder.entry(folder).or_default().push(name);
        }
    }
    let mut renamed = 0;
    for (folder, names) in by_folder {
        let dir = join_rel(root, folder);
        let mut taken = HashSet::new();
        for entry in dir_entries(&dir, "read the folder")? {
            taken.insert(entry?.file_name().to_string_lossy().to_lowercase());
        }
        for name in names.into_iter().filter(|n| !n.starts_with('~')) {
            let reduced = new_name(name);
            if reduced == *name {
                continue;
            }
            taken.remove(&name.to_lowercase());
            let target = unique_name(&reduced, &taken);
            let mut rel = folder.to_vec();
            rel.push(name.clone());
            mutations::rename_file(root, &rel, &target)?;
            taken.insert(target.to_lowercase());
            renamed += 1;
        }
    }
    Ok(renamed)
}
