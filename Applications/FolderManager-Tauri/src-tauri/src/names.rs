// Folder-name rules: numeric slot ids, editable name parts and sanitizing of user input.
use std::path::{Path, PathBuf};

use crate::ant_core::tree_types::type_base;
use crate::config::{SETTINGS_FOLDER, TRASH_DIR};
use crate::error::R;
use crate::fs_safe::io_err;

const FIRST_SLOT: u32 = 1;
const LAST_SLOT: u32 = 89;
const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

pub fn numeric_id(name: &str) -> Option<u32> {
    let digits_len = name.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_len == 0 || name.as_bytes().get(digits_len) != Some(&b'.') {
        return None;
    }
    name[..digits_len].parse().ok()
}

pub fn next_available_id(siblings: &[String]) -> u32 {
    let used: Vec<u32> = siblings
        .iter()
        .filter_map(|s| numeric_id(s))
        .filter(|n| (FIRST_SLOT..=LAST_SLOT).contains(n))
        .collect();
    (FIRST_SLOT..=LAST_SLOT)
        .find(|i| !used.contains(i))
        .unwrap_or(LAST_SLOT)
}

pub fn is_movable_node(name: &str, node_type: &str) -> bool {
    type_base(node_type) == "NODE"
        && numeric_id(name).is_some_and(|n| (FIRST_SLOT..=LAST_SLOT).contains(&n))
}

pub fn sanitize_train_case(raw: &str) -> String {
    let cleaned = clean(raw);
    let mut chars = cleaned.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub fn sanitize_upper(raw: &str) -> String {
    clean(raw).to_uppercase()
}

pub fn editable_part(name: &str) -> &str {
    match name.split_once('.') {
        Some((_, rest)) => rest,
        None => name,
    }
}

pub fn build_new_name(original: &str, new_editable: &str) -> String {
    match original.split_once('.') {
        Some((prefix, _)) => format!("{prefix}.{new_editable}"),
        None => new_editable.to_string(),
    }
}

pub fn find_root(start: &Path) -> Option<PathBuf> {
    let abs = std::path::absolute(start).unwrap_or_else(|_| start.to_path_buf());
    if abs.file_name().is_some_and(|n| n == SETTINGS_FOLDER) {
        return abs.parent().map(Path::to_path_buf);
    }
    let parent = abs.parent();
    std::iter::once(Some(abs.as_path()))
        .chain(std::iter::once(parent))
        .flatten()
        .find(|p| p.join(SETTINGS_FOLDER).is_dir())
        .map(Path::to_path_buf)
}

pub fn list_subdirs(dir: &Path) -> R<Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(Vec::new()),
        Err(e) => return Err(io_err("list folder", dir, &e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.eq_ignore_ascii_case(TRASH_DIR) && entry.path().is_dir() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

// Forbidden characters are dropped before separators collapse, as the Python regexes do.
fn clean(raw: &str) -> String {
    let mut out = String::new();
    let mut in_separator = false;
    for c in raw.trim().chars() {
        if FORBIDDEN.contains(&c) {
            continue;
        }
        if c == '-' || c.is_whitespace() {
            if !in_separator {
                out.push('-');
                in_separator = true;
            }
        } else {
            out.push(c);
            in_separator = false;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_id_reads_leading_ascii_digits() {
        assert_eq!(numeric_id("01.Math"), Some(1));
        assert_eq!(numeric_id("90.RES"), Some(90));
        assert_eq!(numeric_id("007.x"), Some(7));
        assert_eq!(numeric_id("0.x"), Some(0));
        assert_eq!(numeric_id("12"), None);
        assert_eq!(numeric_id("a1.x"), None);
        assert_eq!(numeric_id(".x"), None);
        assert_eq!(numeric_id("\u{0663}.x"), None);
    }

    #[test]
    fn next_available_id_fills_gaps_within_slots() {
        let siblings = vec!["01.A".to_string(), "03.B".to_string(), "90.RES".to_string()];
        assert_eq!(next_available_id(&siblings), 2);
        let outside = vec!["00.A".to_string(), "90.R".to_string()];
        assert_eq!(next_available_id(&outside), 1);
        let empty: Vec<String> = Vec::new();
        assert_eq!(next_available_id(&empty), 1);
    }

    #[test]
    fn next_available_id_falls_back_when_all_slots_full() {
        let full: Vec<String> = (1..=89).map(|i| format!("{i:02}.X")).collect();
        assert_eq!(next_available_id(&full), 89);
    }

    #[test]
    fn is_movable_node_needs_node_type_and_slot_range() {
        assert!(is_movable_node("05.X", "NODE-L2"));
        assert!(is_movable_node("89.x", "NODE"));
        assert!(!is_movable_node("05.X", "WKB"));
        assert!(!is_movable_node("90.RES", "NODE"));
        assert!(!is_movable_node("00.Core", "NODE"));
        assert!(!is_movable_node("X", "NODE"));
    }

    #[test]
    fn sanitize_train_case_cleans_and_capitalizes() {
        assert_eq!(sanitize_train_case("  hello   world/ "), "Hello-world");
        assert_eq!(sanitize_train_case("-- a:b--"), "Ab");
        assert_eq!(sanitize_train_case("ñandu"), "Ñandu");
        assert_eq!(sanitize_train_case("  /:*  "), "");
    }

    #[test]
    fn sanitize_upper_cleans_and_uppercases() {
        assert_eq!(sanitize_upper("ab cd"), "AB-CD");
        assert_eq!(sanitize_upper("x?y"), "XY");
    }

    #[test]
    fn editable_part_and_build_new_name_keep_prefix() {
        assert_eq!(editable_part("01.Math"), "Math");
        assert_eq!(editable_part("Math"), "Math");
        assert_eq!(editable_part("01."), "");
        assert_eq!(build_new_name("01.Math", "Physics"), "01.Physics");
        assert_eq!(build_new_name("Math", "Physics"), "Physics");
    }
}
