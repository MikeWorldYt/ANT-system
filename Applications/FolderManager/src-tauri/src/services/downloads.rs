// Lists the system Downloads folder and the other native folders as tree rows, so the root can browse folders that live outside the storage.
use std::path::{Path, PathBuf};

use crate::domain::error::R;
use crate::infra::fs_safe::{dir_entries, io_err, resolve};
use crate::services::tree::tree_node::{stamps_of, NodeActions, TreeNode};

// ':' cannot appear in a Windows file name, so no storage folder can share this first part.
pub const ROOT: &str = "::Downloads";
// The other native folders, in the order they close the root; they browse and change as Downloads does, without its automations.
pub const SYSTEM_ROOTS: [&str; 5] = ["::Documents", "::Pictures", "::Music", "::Videos", "::Desktop"];

pub fn is_root(first: &str) -> bool {
    first == ROOT || SYSTEM_ROOTS.contains(&first)
}

// A path inside any native folder: its root marker, then the names below it.
pub fn split_root(parts: &[String]) -> Option<(&str, &[String])> {
    parts.split_first().filter(|(first, _)| is_root(first)).map(|(first, rest)| (first.as_str(), rest))
}

// The MCP process has no Tauri path resolver, so it asks the OS for the user's Downloads folder itself.
#[cfg(windows)]
pub fn system_base() -> R<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
    let fail = |e: String| crate::domain::error::FmError::Internal { message: format!("Could not find the Downloads folder: {e}") };
    // SAFETY: the returned buffer is read once into an owned string and then freed with the allocator Windows used.
    unsafe {
        let raw = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None).map_err(|e| fail(e.to_string()))?;
        let path = raw.to_string().map_err(|e| fail(e.to_string()));
        CoTaskMemFree(Some(raw.0 as *const _));
        path.map(PathBuf::from)
    }
}

#[cfg(not(windows))]
pub fn system_base() -> R<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads")).ok_or_else(|| crate::domain::error::FmError::Internal { message: "Could not find the Downloads folder: HOME is not set".to_string() })
}

// Only names under Downloads are accepted, each validated, so a caller can never name a path outside it.
pub fn path_of(base: &Path, rel: &[String]) -> R<PathBuf> {
    resolve(base, rel)
}

pub fn list(base: &Path, root: &str, rel: &[String]) -> R<Vec<TreeNode>> {
    let dir = path_of(base, rel)?;
    let mut rows = Vec::new();
    for entry in dir_entries(&dir, "read the folder")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().map_err(|e| io_err("read the folder", &dir, &e))?;
        let concealed = is_concealed(&entry);
        rows.push(TreeNode { concealed, ..node(root, rel, name, entry.path(), !kind.is_dir()) });
    }
    rows.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(rows)
}

// Dot files, desktop.ini and what Windows marks hidden or system are kept out of sight for a reason: the views show them only on request and no automation touches them.
pub(crate) fn is_concealed(entry: &std::fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    name.starts_with('.') || name.eq_ignore_ascii_case("desktop.ini") || is_system_hidden(entry)
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

fn node(root: &str, parent: &[String], name: String, abs: PathBuf, file: bool) -> TreeNode {
    let mut rel_parts = vec![root.to_string()];
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
        concealed: false,
        file,
        info: None,
        children: None,
        // Opening, copying the path and renaming apply; every other change goes through the storage.
        actions: NodeActions {
            rename: true,
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
        let all = list(dir.path(), ROOT, &[]).unwrap();
        assert_eq!(all.iter().filter(|r| r.concealed).map(|r| r.name.as_str()).collect::<Vec<_>>(), [".hidden-folder", "desktop.ini"]);
        let rows: Vec<_> = all.into_iter().filter(|r| !r.concealed).collect();
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["b-folder", "a.txt"]);
        assert_eq!(rows[0].rel_parts, rel(&[ROOT, "b-folder"]));
        assert!(rows[0].has_children && !rows[0].file && rows[1].file);
        assert!(rows[0].actions.rename && !rows[0].actions.delete && rows[0].actions.open);
    }

    #[test]
    fn a_subfolder_lists_below_its_own_path() {
        let dir = TempDir::new("downloads-sub");
        dir.mkdir("pics");
        std::fs::write(dir.path().join("pics").join("cat.png"), "x").unwrap();
        let rows = list(dir.path(), ROOT, &rel(&["pics"])).unwrap();
        assert_eq!(rows[0].rel_parts, rel(&[ROOT, "pics", "cat.png"]));
    }

    #[test]
    fn paths_that_climb_out_or_name_separators_are_refused() {
        let dir = TempDir::new("downloads-escape");
        assert!(list(dir.path(), ROOT, &rel(&[".."])).is_err());
        assert!(list(dir.path(), ROOT, &rel(&["a/b"])).is_err());
        assert!(list(dir.path(), ROOT, &rel(&["C:"])).is_err());
    }

    #[test]
    fn every_native_folder_lists_under_its_own_root_and_only_markers_split() {
        let dir = TempDir::new("system-folder-list");
        std::fs::write(dir.path().join("song.mp3"), "x").unwrap();
        let rows = list(dir.path(), "::Music", &[]).unwrap();
        assert_eq!(rows[0].rel_parts, rel(&["::Music", "song.mp3"]));
        let parts = rel(&["::Desktop", "a", "b.txt"]);
        assert_eq!(split_root(&parts), Some(("::Desktop", &parts[1..])));
        assert_eq!(split_root(&rel(&[ROOT])), Some((ROOT, &[][..])));
        assert!(split_root(&rel(&["Desktop", "a"])).is_none() && split_root(&rel(&["::Other"])).is_none() && split_root(&[]).is_none());
    }
}
