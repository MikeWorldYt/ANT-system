// The automations on the system Downloads folder: it lives outside the storage, so its dictionary sits in the temp folder and the run renames and moves on disk without the storage's tree or lock.
use std::fs;
use std::path::{Path, PathBuf};

use std::collections::HashMap;

use super::dictionary::{compile, node_name, parse, parse_globals, render, Globals, DICTIONARY_FILE};
use super::node_path;
use super::plan::{plan_with, Picks, PlannedFile};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, resolve, validate_component, write_atomic};
use crate::services::mutations;

pub fn dictionary_path() -> PathBuf {
    std::env::temp_dir().join("ANT").join("automations").join("downloads").join(DICTIONARY_FILE)
}

pub fn read_text() -> R<String> {
    let path = dictionary_path();
    if !path.is_file() {
        return Ok(render("Downloads", &[]));
    }
    fs::read_to_string(&path).map_err(|e| io_err("read the dictionary", &path, &e))
}

pub fn write_text(text: &str) -> R<()> {
    let path = dictionary_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| io_err("create the dictionary folder", dir, &e))?;
    }
    write_atomic(&path, text.as_bytes())
}

// Every folder below Downloads shares the one dictionary; `NODE.` headings are relative to a storage folder, so only `@/` ones reach a node from here.
fn entries(storage: &Path) -> R<(Vec<super::dictionary::Entry>, Globals)> {
    let path = dictionary_path();
    if !path.is_file() {
        return Ok((Vec::new(), Globals::default()));
    }
    let text = fs::read_to_string(&path).map_err(|e| io_err("read the dictionary", &path, &e))?;
    let globals = parse_globals(&text, &path)?;
    let mut entries = Vec::new();
    for mut entry in parse(&text).iter().map(|spec| compile(spec, &path)).collect::<R<Vec<_>>>()? {
        if node_name(&entry.folder).is_some() {
            continue;
        }
        if entry.folder.starts_with(node_path::ABS_PREFIX) {
            let path = node_path::resolve(storage, &entry.folder)?;
            entry.group = path.is_some();
            if let Some(path) = path {
                (entry.node, entry.create) = (Some(path.rel), path.create);
            }
        }
        entries.push(entry);
    }
    Ok((entries, globals))
}

pub fn plan(storage: &Path, base: &Path, sub: &[String], only: &[String], picks: Picks) -> R<Vec<PlannedFile>> {
    resolve(base, sub)?;
    let (entries, globals) = entries(storage)?;
    plan_with(base, sub, only, picks, entries, &globals)
}

fn move_to(from: &Path, to: &Path) -> R<()> {
    // A change of case names the same file on Windows, so only another file blocks the move.
    let same = from.to_string_lossy().eq_ignore_ascii_case(&to.to_string_lossy());
    if to.exists() && !same {
        return Err(FmError::InvalidName { name: to.display().to_string(), reason: "a file with this name is already there".to_string() });
    }
    fs::rename(from, to).map_err(|e| io_err("move the file", from, &e))
}

// Renamed in Downloads first, then dropped into the node, which files it into its review folder and runs that folder's automations.
fn send_to_node(storage: &Path, dir: &Path, row: &PlannedFile, target: &[String], made: &mut HashMap<Vec<String>, Vec<String>>) -> R<()> {
    let target = if row.create { node_path::ensure(storage, target, made)? } else { target.to_vec() };
    if row.name != row.file {
        move_to(&dir.join(&row.file), &dir.join(&row.name))?;
    }
    let no_bin = |_: &Path| Err("a dictionary move never replaces anything".to_string());
    mutations::drop_files(storage, &target, &[dir.join(&row.name)], false, &[], &no_bin)?;
    Ok(())
}

pub fn apply(storage: &Path, base: &Path, sub: &[String], rows: &[PlannedFile]) -> R<usize> {
    let dir = resolve(base, sub)?;
    let mut changed = 0;
    let mut made = HashMap::new();
    for row in rows {
        validate_component(&row.file)?;
        validate_component(&row.name)?;
        let into = row.folder.as_ref().or(row.project.as_ref());
        if let Some(folder) = into {
            validate_component(folder)?;
        }
        if let Some(target) = &row.target {
            send_to_node(storage, &dir, row, target, &mut made)?;
            changed += 1;
            continue;
        }
        if into.is_none() && row.name == row.file {
            continue;
        }
        let dest = match into {
            Some(folder) => {
                let target = dir.join(folder);
                fs::create_dir_all(&target).map_err(|e| io_err("create the folder", &target, &e))?;
                target.join(&row.name)
            }
            None => dir.join(&row.name),
        };
        move_to(&dir.join(&row.file), &dest)?;
        changed += 1;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn renames_and_moves_stay_inside_downloads_and_never_overwrite() {
        let dir = TempDir::new("downloads-auto");
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("b.txt"), "b").unwrap();
        fs::write(dir.path().join("c.txt"), "c").unwrap();
        let row = |file: &str, name: &str, folder: Option<&str>| PlannedFile { file: file.into(), name: name.into(), project: None, sure: false, folder: folder.map(str::to_string), target: None, create: false, entry: None };
        let storage = dir.path();
        assert_eq!(apply(storage, dir.path(), &[], &[row("a.txt", "A.txt", None), row("b.txt", "b.txt", Some("Docs"))]).unwrap(), 2);
        assert!(dir.path().join("A.txt").is_file() && dir.path().join("Docs/b.txt").is_file());
        assert!(apply(storage, dir.path(), &[], &[row("c.txt", "A.txt", None)]).is_err());
        assert!(apply(storage, dir.path(), &[], &[row("c.txt", "../x.txt", None)]).is_err());
        assert!(apply(storage, dir.path(), &["..".into()], &[]).is_err());
    }
}
