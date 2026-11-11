// Lists the system Downloads folder as tree rows, so the root can browse a folder that lives outside the storage.
use std::path::{Path, PathBuf};

use crate::domain::error::R;
use crate::infra::fs_safe::{dir_entries, io_err, resolve};
use crate::services::tree::tree_node::{stamps_of, NodeActions, TreeNode};

// ':' cannot appear in a Windows file name, so no storage folder can share this first part.
pub const ROOT: &str = "::Downloads";

// Only names under Downloads are accepted, each validated, so a caller can never name a path outside it.
pub fn path_of(base: &Path, rel: &[String]) -> R<PathBuf> {
    resolve(base, rel)
}

pub fn list(base: &Path, rel: &[String]) -> R<Vec<TreeNode>> {
    let dir = path_of(base, rel)?;
    let mut rows = Vec::new();
    for entry in dir_entries(&dir, "read the Downloads folder")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }
        let kind = entry.file_type().map_err(|e| io_err("read the Downloads folder", &dir, &e))?;
        if is_system_hidden(&entry) {
            continue;
        }
        rows.push(node(rel, name, entry.path(), !kind.is_dir()));
    }
    rows.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(rows)
}

#[cfg(windows)]
fn is_system_hidden(entry: &std::fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN_OR_SYSTEM: u32 = 0x2 | 0x4;
    entry.metadata().map(|m| m.file_attributes() & HIDDEN_OR_SYSTEM != 0).unwrap_or(false)
}

#[cfg(not(windows))]
fn is_system_hidden(_entry: &std::fs::DirEntry) -> bool {
    false
}

fn node(parent: &[String], name: String, abs: PathBuf, file: bool) -> TreeNode {
    let mut rel_parts = vec![ROOT.to_string()];
    rel_parts.extend_from_slice(parent);
    rel_parts.push(name.clone());
    TreeNode {
        stamps: stamps_of(&abs, file),
        abs_path: abs.to_string_lossy().into_owned(),
        depth: rel_parts.len() - 1,
        rel_parts,
        ghost: false,
        in_yaml: false,
        node_type: None,
        status: None,
        status_raw: None,
        locked: false,
        reserved: false,
        linked: false,
        link_path: None,
        flags: Vec::new(),
        // Folders are listed when opened, so each one is offered as expandable without reading it.
        has_children: !file,
        is_git: false,
        fixed_prefix: String::new(),
        editable_name: name.clone(),
        favorite: false,
        important: false,
        hidden: false,
        file,
        info: None,
        children: None,
        // Only opening and copying the path apply; every change goes through the storage.
        actions: NodeActions {
            rename: false,
            change_type: false,
            add_next: false,
            add_child: false,
            add_folder: false,
            move_up: false,
            move_down: false,
            delete: false,
            copy_path: true,
            open: true,
            favorite: false,
            important: false,
            edit_details: false,
            edit_flags: false,
            hide_link: false,
            hide: false,
        },
        name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{rel, TempDir};

    #[test]
    fn folders_come_first_and_every_row_sits_under_the_downloads_root() {
        let dir = TempDir::new("downloads-list");
        dir.mkdir("b-folder");
        dir.mkdir(".hidden-folder");
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        std::fs::write(dir.path().join("desktop.ini"), "x").unwrap();
        let rows = list(dir.path(), &[]).unwrap();
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["b-folder", "a.txt"]);
        assert_eq!(rows[0].rel_parts, rel(&[ROOT, "b-folder"]));
        assert!(rows[0].has_children && !rows[0].file && rows[1].file);
        assert!(!rows[0].actions.rename && !rows[0].actions.delete && rows[0].actions.open);
    }

    #[test]
    fn a_subfolder_lists_below_its_own_path() {
        let dir = TempDir::new("downloads-sub");
        dir.mkdir("pics");
        std::fs::write(dir.path().join("pics").join("cat.png"), "x").unwrap();
        let rows = list(dir.path(), &rel(&["pics"])).unwrap();
        assert_eq!(rows[0].rel_parts, rel(&[ROOT, "pics", "cat.png"]));
    }

    #[test]
    fn paths_that_climb_out_or_name_separators_are_refused() {
        let dir = TempDir::new("downloads-escape");
        assert!(list(dir.path(), &rel(&[".."])).is_err());
        assert!(list(dir.path(), &rel(&["a/b"])).is_err());
        assert!(list(dir.path(), &rel(&["C:"])).is_err());
    }
}
