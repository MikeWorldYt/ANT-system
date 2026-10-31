// Brings level 2 and deeper folder names to the "NN." id form, logging each rename; names without an id are left for the user.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::type_base;
use crate::backup::stamp::local_date;
use crate::debug_log::logs_dir;
use crate::error::R;
use crate::fs_safe::{io_err, Journal, StorageLock};
use crate::mutations::load_synced;
use crate::tree::depth;
use crate::tree::tree_store;

struct Rename {
    parent: String,
    old: String,
    new: String,
}

pub fn tidy(root: &Path) -> R<()> {
    let _lock = StorageLock::acquire(root)?;
    let (mut data, _) = load_synced(root)?;
    let limit = depth::limit(root)?;
    let mut journal = Journal::new();
    let mut renames = Vec::new();
    if let Err(e) = walk(root, &mut data, &mut Vec::new(), limit, &mut journal, &mut renames) {
        return Err(journal.abort("rename folders", e));
    }
    if renames.is_empty() {
        return Ok(());
    }
    if let Err(e) = tree_store::save(root, &data) {
        return Err(journal.abort("rename folders", e));
    }
    journal.commit();
    log(root, &renames)
}

// Only nodes are walked: app folders, workboards and their contents keep their own names.
fn walk(dir: &Path, node: &mut Mapping, rel: &mut Vec<String>, limit: usize, journal: &mut Journal, renames: &mut Vec<Rename>) -> R<()> {
    let names: Vec<String> = node.iter().filter(|(_, v)| v.is_mapping()).filter_map(|(k, _)| k.as_str().map(str::to_string)).collect();
    for name in names {
        let is_node = node.get(name.as_str()).and_then(|c| c.get("type")).and_then(Value::as_str).is_some_and(|t| type_base(t) == "NODE");
        if !is_node {
            continue;
        }
        let level = rel.len() + 1;
        let mut current = name.clone();
        if level >= 2 {
            if let Some(new) = trimmed(&name) {
                if !node.contains_key(new.as_str()) && !dir.join(&new).exists() {
                    journal.rename(&dir.join(&name), &dir.join(&new))?;
                    if let Some(entry) = node.remove(name.as_str()) {
                        node.insert(Value::String(new.clone()), entry);
                    }
                    renames.push(Rename { parent: rel.join("/"), old: name.clone(), new: new.clone() });
                    current = new;
                }
            }
        }
        if level < limit {
            if let Some(child) = node.get_mut(current.as_str()).and_then(Value::as_mapping_mut) {
                rel.push(current.clone());
                walk(&dir.join(&current), child, rel, limit, journal, renames)?;
                rel.pop();
            }
        }
    }
    Ok(())
}

// "Unit01.Math" becomes "01.Math": letters in front of a two-digit id are noise.
fn trimmed(name: &str) -> Option<String> {
    if has_id(name) {
        return None;
    }
    let at = name.find(|c: char| !c.is_alphabetic())?;
    let rest = &name[at..];
    (at > 0 && has_id(rest)).then(|| rest.to_string())
}

fn has_id(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b'.' && &name[..2] != "00"
}

fn log(root: &Path, renames: &[Rename]) -> R<()> {
    let dir = logs_dir(root);
    fs::create_dir_all(&dir).map_err(|e| io_err("create log folder", &dir, &e))?;
    let path = dir.join(format!("{}_ANT-fm-renamed.log", local_date()));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| io_err("write rename log", &path, &e))?;
    let text: String = renames.iter().map(|r| format!("{} : \"{}\" → \"{}\"\n", r.parent, r.old, r.new)).collect();
    file.write_all(text.as_bytes()).map_err(|e| io_err("write rename log", &path, &e))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{rel, storage, yaml, TempDir};
    use crate::tree::tree_store::get_node;

    fn log_text(dir: &TempDir) -> String {
        let path = logs_dir(dir.path()).join(format!("{}_ANT-fm-renamed.log", local_date()));
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn letters_in_front_of_an_id_are_trimmed_and_logged() {
        let dir = storage(
            "node-names-trim",
            &["100.ACADEMIC/01.Math/Unit02.Physics", "100.ACADEMIC/Unit03.Root", "100.ACADEMIC/01.Math/Physics"],
            "100.ACADEMIC:\n  type: NODE-L1\n  Unit03.Root: {type: NODE-L2}\n  01.Math: {type: NODE-L2, Unit02.Physics: {type: NODE-L3}, Physics: {type: NODE-L3}}\n",
        );
        tidy(dir.path()).unwrap();
        assert!(dir.path().join("100.ACADEMIC/01.Math/02.Physics").is_dir());
        assert!(!dir.path().join("100.ACADEMIC/01.Math/Unit02.Physics").exists());
        assert!(dir.path().join("100.ACADEMIC/03.Root").is_dir());
        assert!(dir.path().join("100.ACADEMIC/01.Math/Physics").is_dir());
        let data = yaml(dir.path());
        assert!(get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "02.Physics"])).is_some());
        assert!(get_node(&data, &rel(&["100.ACADEMIC", "01.Math", "Unit02.Physics"])).is_none());
        assert_eq!(
            log_text(&dir),
            "100.ACADEMIC/01.Math : \"Unit02.Physics\" → \"02.Physics\"\n100.ACADEMIC : \"Unit03.Root\" → \"03.Root\"\n",
        );
    }

    #[test]
    fn a_trim_that_would_clash_is_skipped_and_root_names_are_left_to_the_root_check() {
        let dir = storage(
            "node-names-clash",
            &["100.ACADEMIC/01.Math/04.Chem", "100.ACADEMIC/01.Math/Unit04.Chem", "Unit05.Loose"],
            "100.ACADEMIC:\n  type: NODE-L1\n  01.Math: {type: NODE-L2, 04.Chem: {type: NODE-L3}, Unit04.Chem: {type: NODE-L3}}\n",
        );
        tidy(dir.path()).unwrap();
        assert!(dir.path().join("100.ACADEMIC/01.Math/Unit04.Chem").is_dir());
        assert!(dir.path().join("Unit05.Loose").is_dir());
        assert!(!logs_dir(dir.path()).join(format!("{}_ANT-fm-renamed.log", local_date())).exists());
    }

    #[test]
    fn trimmed_needs_letters_then_a_two_digit_id() {
        assert_eq!(trimmed("Unit01.Math").as_deref(), Some("01.Math"));
        assert_eq!(trimmed("Unidad99.X").as_deref(), Some("99.X"));
        assert_eq!(trimmed("01.Math"), None);
        assert_eq!(trimmed("Unit1.Math"), None);
        assert_eq!(trimmed("Unit00.Math"), None);
        assert_eq!(trimmed("Unit01Math"), None);
        assert_eq!(trimmed("Unit.Math"), None);
        assert_eq!(trimmed("01Unit.Math"), None);
    }
}
