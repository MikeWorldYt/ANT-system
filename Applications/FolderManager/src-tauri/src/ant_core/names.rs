// Folder-name rules: numeric slot ids, editable name parts and sanitizing of user input.
use std::path::{Path, PathBuf};

use crate::ant_core::tree_types::type_base;
use crate::config::{is_hidden_dir, SETTINGS_FOLDER, WKB_IDENTIFIERS};
use crate::error::R;
use crate::fs_safe::{io_err, FORBIDDEN};

const FIRST_SLOT: u32 = 1;
const LAST_SLOT: u32 = 89;
const FIRST_MAIN_ID: u32 = 100;
const LAST_MAIN_ID: u32 = 800;
const MAIN_ID_STEP: u32 = 100;
const FIRST_SYSTEM_ID: u32 = 900;
const LAST_SYSTEM_ID: u32 = 999;

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

pub fn next_main_category_id(siblings: &[String]) -> u32 {
    (FIRST_MAIN_ID..=LAST_MAIN_ID)
        .step_by(MAIN_ID_STEP as usize)
        .find(|id| !siblings.iter().any(|name| numeric_id(name) == Some(*id)))
        .unwrap_or(LAST_MAIN_ID)
}

pub fn is_main_category(name: &str) -> bool {
    numeric_id(name).is_some_and(|id| (FIRST_MAIN_ID..=LAST_MAIN_ID).contains(&id) && id % MAIN_ID_STEP == 0)
}

// The 9xx range at the root belongs to the app (998.Review, 999.Settings).
pub fn is_system_root_name(name: &str) -> bool {
    numeric_id(name).is_some_and(|id| (FIRST_SYSTEM_ID..=LAST_SYSTEM_ID).contains(&id))
}

pub fn is_valid_root_name(name: &str) -> bool {
    is_main_category(name) || is_system_root_name(name)
}

// Nodes below the root open with a two-digit id ("01.Math"); "00" is not one.
pub fn has_node_id(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b'.' && &name[..2] != "00"
}

pub fn is_workboard_name(name: &str) -> bool {
    WKB_IDENTIFIERS.iter().any(|tracker| name.starts_with(tracker))
}

// The root takes only category or app ids; deeper folders take a node id or a workboard tracker.
pub fn has_valid_id(rel: &[String]) -> bool {
    match rel {
        [name] => is_valid_root_name(name),
        [.., name] => has_node_id(name) || is_workboard_name(name),
        [] => false,
    }
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

// Splits a name into the part the user cannot edit and the rest: "03." + "Toki", "PJ.001_" + "Name".
pub fn name_parts(name: &str, workboard: bool) -> (String, String) {
    let (sep, whole_is_fixed) = if workboard { ('_', true) } else { ('.', false) };
    match name.split_once(sep) {
        Some((fixed, rest)) => (format!("{fixed}{sep}"), rest.to_string()),
        None if whole_is_fixed => (format!("{name}{sep}"), String::new()),
        None => (String::new(), name.to_string()),
    }
}

// A workboard without a name goes back to its bare tracker, "PJ.001".
pub fn compose_name(fixed: &str, editable: &str, workboard: bool) -> String {
    if workboard && editable.is_empty() {
        return fixed.trim_end_matches('_').to_string();
    }
    format!("{fixed}{editable}")
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
        if !is_hidden_dir(&name) && entry.path().is_dir() {
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
    fn root_names_need_a_hundred_step_category_or_the_system_range() {
        for valid in ["100.ACADEMIC", "200.WORK", "800.X", "998.Review", "999.Settings", "900.Sys"] {
            assert!(is_valid_root_name(valid), "{valid}");
        }
        for invalid in ["Marcos", "New Folder", "574.Notes", "000.Zero", "050.Half", "100", "1000.Big", "01.Math"] {
            assert!(!is_valid_root_name(invalid), "{invalid}");
        }
        assert!(is_main_category("300.HOME"));
        assert!(!is_main_category("998.Review"));
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
    fn workboard_names_keep_the_tracker_and_edit_only_the_name() {
        assert_eq!(name_parts("PJ.001_Fabi-web", true), ("PJ.001_".to_string(), "Fabi-web".to_string()));
        assert_eq!(name_parts("PJ.001", true), ("PJ.001_".to_string(), String::new()));
        assert_eq!(name_parts("PJ.145_(25-11-14) Plans_DOC", true).1, "(25-11-14) Plans_DOC");
        assert_eq!(compose_name("PJ.001_", "Name", true), "PJ.001_Name");
        assert_eq!(compose_name("PJ.001_", "", true), "PJ.001");
    }

    #[test]
    fn editable_part_and_build_new_name_keep_prefix() {
        assert_eq!(editable_part("01.Math"), "Math");
        assert_eq!(editable_part("Math"), "Math");
        assert_eq!(editable_part("01."), "");
        assert_eq!(name_parts("01.Math", false), ("01.".to_string(), "Math".to_string()));
        assert_eq!(name_parts("Math", false), (String::new(), "Math".to_string()));
        assert_eq!(compose_name("01.", "Physics", false), "01.Physics");
    }
}
