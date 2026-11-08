// What one tree level lists: disk folders merged with tree entries, review links and files, in their display order.
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use serde_yaml::Mapping;

use super::lists_files;
use crate::domain::config::{is_hidden_dir, HIDDEN_FLAG, LINK_FAVORITES_FILENAME, LINK_HIDDEN_FILENAME, REVIEW_FOLDER};
use crate::domain::error::R;
use crate::domain::names::{is_system_root_name, numeric_id, support_kind};
use crate::domain::tree_types::{child_entries, flags_in, META_KEYS};
use crate::infra::fs_safe::{dir_entries, io_err, join_rel};
use crate::infra::link_list;
use crate::infra::scan::list_subdirs;
use crate::services::tree::review_links;
use crate::services::tree::tree_store::get_node;

pub(super) struct Entry {
    pub(super) name: String,
    pub(super) on_disk: bool,
    pub(super) in_yaml: bool,
    pub(super) link: Option<Vec<String>>,
    pub(super) favorite: bool,
    pub(super) hidden: bool,
    pub(super) file: bool,
}

// A folder on disk with no database entry, and not a review link.
pub(super) fn is_unmanaged(entry: &Entry) -> bool {
    entry.on_disk && !entry.in_yaml && entry.link.is_none() && !entry.file
}

pub(super) fn level_entries(root: &Path, data: &Mapping, parent: &[String]) -> R<Vec<Entry>> {
    let disk_dir = join_rel(root, parent);
    let disk = if disk_dir.is_dir() { list_subdirs(&disk_dir)? } else { Vec::new() };
    let yaml: Vec<&str> = get_node(data, parent)
        .map(|m| child_entries(m).map(|(k, _)| k).collect())
        .unwrap_or_default();
    // A hidden folder keeps its tree entry but is not listed, so nothing below it is read either.
    let hidden_here: BTreeSet<&str> = get_node(data, parent)
        .map(|m| child_entries(m).filter(|(_, c)| flags_in(c).iter().any(|f| f == HIDDEN_FLAG)).map(|(k, _)| k).collect())
        .unwrap_or_default();
    // A disk folder named like a meta key has no tree entry of its own, and the trash is never a node.
    let names: BTreeSet<&str> = disk
        .iter()
        .map(String::as_str)
        .filter(|d| !META_KEYS.contains(d))
        .chain(yaml.iter().copied().filter(|k| !is_hidden_dir(k)))
        .filter(|name| !hidden_here.contains(name))
        .filter(|name| !(parent.is_empty() && is_hidden_system_root(name)))
        .collect();
    let link_entries: Vec<Entry> = if matches!(parent, [top] if top.eq_ignore_ascii_case(REVIEW_FOLDER)) {
        let starred = link_list::read(root, LINK_FAVORITES_FILENAME)?;
        let hidden_links = link_list::read(root, LINK_HIDDEN_FILENAME)?;
        let hidden = review_links::hidden_categories(root);
        let favorites_shown = review_links::favorites_shown(root);
        let links = review_links::links(root, data)?;
        // A starred link brings its category's top link along, so the group keeps its heading under a hidden category.
        let starred_categories: Vec<String> = links
            .iter()
            .filter(|(name, _)| favorites_shown && starred.contains(name))
            .map(|(_, owner)| owner[0].clone())
            .collect();
        links
            .into_iter()
            .filter(|(name, owner)| {
                let kept_as_favorite = favorites_shown && starred.contains(name);
                let kept_as_heading = owner.len() == 1 && starred_categories.contains(&owner[0]);
                (kept_as_favorite || kept_as_heading || !hidden.contains(&owner[0])) && !names.contains(name.as_str())
            })
            .map(|(name, owner)| Entry { favorite: starred.contains(&name), hidden: hidden_links.contains(&name), name, on_disk: true, in_yaml: false, link: Some(owner), file: false })
            .collect()
    } else {
        Vec::new()
    };

    // The root holds only the folders and the app itself, whose files are not the user's to browse.
    let file_entries: Vec<Entry> = if !parent.is_empty() && lists_files(parent) {
        disk_files(&disk_dir)?
            .into_iter()
            .map(|name| Entry { name, on_disk: true, in_yaml: false, link: None, favorite: false, hidden: false, file: true })
            .collect()
    } else {
        Vec::new()
    };
    let mut entries: Vec<Entry> = names
        .into_iter()
        .map(|name| Entry {
            name: name.to_string(),
            on_disk: disk.iter().any(|d| d == name),
            in_yaml: yaml.contains(&name),
            link: None,
            favorite: false,
            hidden: false,
            file: false,
        })
        .filter(|e| !(e.on_disk && !e.in_yaml && is_linked_folder(&e.name)))
        .chain(link_entries)
        .chain(file_entries)
        .collect();
    entries.sort_by_cached_key(|e| (tail_rank(&e.name), natural_key(&e.name)));
    Ok(entries)
}

// Compares the first run of digits as a number, so PJ.999 comes before PJ.1000.
pub(super) fn natural_key(name: &str) -> (String, u64, String) {
    let start = name.find(|c: char| c.is_ascii_digit()).unwrap_or(name.len());
    let digits = name[start..].bytes().take_while(u8::is_ascii_digit).count();
    let number = name[start..start + digits].parse().unwrap_or(0);
    (name[..start].to_string(), number, name[start + digits..].to_string())
}

// Support folders (REV., RES., TPL. and the numeric ids older storages use) and the 998/999 root ones always close a list, after workboards and unnumbered folders too.
fn is_tail(name: &str) -> bool {
    support_kind(name).is_some() || numeric_id(name).is_some_and(|n| n == 998 || n == 999)
}

// Inside the tail, support folders keep a fixed order so every level reads the same way.
const SUPPORT_ORDER: [&str; 5] = ["REC", "SS", "RES", "REV", "TPL"];

fn tail_rank(name: &str) -> usize {
    if !is_tail(name) {
        return 0;
    }
    support_kind(name).and_then(|kind| SUPPORT_ORDER.iter().position(|k| *k == kind)).map_or(SUPPORT_ORDER.len() + 1, |at| at + 1)
}

fn is_linked_folder(name: &str) -> bool {
    matches!(support_kind(name), Some("REV" | "RES" | "SS" | "REC"))
}

// Review folders also list their files; hidden files and the Windows desktop.ini stay out.
fn disk_files(dir: &Path) -> R<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in dir_entries(dir, "read the folder")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") || is_running_app(dir, &entry.file_name()) {
            continue;
        }
        if entry.file_type().map_err(|e| io_err("read the folder", dir, &e))?.is_file() {
            files.push(name);
        }
    }
    Ok(files)
}

// The app can sit inside the storage it manages; listing its own executable would show the app inside itself.
fn is_running_app(dir: &Path, name: &OsStr) -> bool {
    let Ok(exe) = std::env::current_exe() else { return false };
    exe.file_name().is_some_and(|exe_name| exe_name.eq_ignore_ascii_case(name))
        && exe.parent().is_some_and(|parent| fs::canonicalize(parent).ok() == fs::canonicalize(dir).ok())
}

// The 9xx root folders are the app's; 998.Review stays listed because its links are what the user browses.
fn is_hidden_system_root(name: &str) -> bool {
    is_system_root_name(name) && !name.eq_ignore_ascii_case(REVIEW_FOLDER)
}
