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

// A moved node takes the lowest free slot of its new level, so the ids stay in sequence; other names are kept as they are.
pub fn with_next_slot(name: &str, siblings: &[String]) -> String {
    if has_node_id(name) {
        format!("{:02}.{}", next_available_id(siblings), &name[3..])
    } else {
        name.to_string()
    }
}

// A workboard whose track another sibling already uses takes the lowest free number of that tracker, at the same width.
pub fn free_track(name: &str, siblings: &[String]) -> String {
    let head = name.split('_').next().unwrap_or(name);
    let Some(dot) = head.find('.') else { return name.to_string() };
    let letters = &head[..=dot];
    let used: Vec<u32> = siblings.iter().filter_map(|s| track_number(s.split('_').next().unwrap_or(s), letters)).collect();
    if !track_number(head, letters).is_some_and(|n| used.contains(&n)) {
        return name.to_string();
    }
    let width = head.len() - letters.len();
    let free = (1..).find(|n| !used.contains(n)).unwrap_or(1);
    format!("{letters}{free:0width$}{}", &name[head.len()..])
}

// The number after a tracker's letters ("PJ.") when the head is that tracker, else None.
fn track_number(head: &str, letters: &str) -> Option<u32> {
    let same_letters = head.get(..letters.len())?.eq_ignore_ascii_case(letters);
    let number = head.get(letters.len()..)?;
    (same_letters && !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())).then(|| number.parse().ok()).flatten()
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

// A tracker and its number ("PJ.001", "PJ.1007_Name"); the title after the number is free.
pub fn is_track_name(name: &str) -> bool {
    WKB_IDENTIFIERS.iter().any(|tracker| name.strip_prefix(tracker).is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit())))
}

// The title left after a folder's id: "PJ.001_Name" and "01.Name" both give "Name".
pub fn title_of(name: &str) -> &str {
    match WKB_IDENTIFIERS.iter().find_map(|tracker| name.strip_prefix(tracker)) {
        Some(rest) => rest.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('_'),
        None if has_node_id(name) => &name[3..],
        None => name,
    }
}

// The text after a tracker such as "PJ." in any letter case, or None when the name does not start with it.
fn after_tracker<'a>(name: &'a str, tracker: &str) -> Option<&'a str> {
    name.get(..tracker.len()).filter(|head| head.eq_ignore_ascii_case(tracker)).map(|_| &name[tracker.len()..])
}

// The number a name already carries for that tracker in any letter case ("Pj.154_Name" with "PJ." gives 154).
pub fn tracker_number(name: &str, tracker: &str) -> Option<u32> {
    let rest = after_tracker(name, tracker)?;
    rest[..rest.bytes().take_while(|b| b.is_ascii_digit()).count()].parse().ok()
}

// A trailing hash is a separator and 6 to 40 hex digits that mix letters and digits, so "Plan-2024" and "Face-decade" stay.
fn without_hash_suffix(title: &str) -> &str {
    let Some((head, tail)) = title.rsplit_once('-') else { return title };
    let hex = (6..=40).contains(&tail.len()) && tail.bytes().all(|b| b.is_ascii_hexdigit());
    let mixed = tail.bytes().any(|b| b.is_ascii_digit()) && tail.bytes().any(|b| b.is_ascii_alphabetic());
    if hex && mixed && !head.is_empty() { head } else { title }
}

// The title a folder keeps when it becomes a workboard: no tracker or number in any letter case, no trailing hash.
pub fn workboard_title(name: &str) -> String {
    let rest = match WKB_IDENTIFIERS.iter().find_map(|tracker| after_tracker(name, tracker)) {
        Some(rest) => rest.trim_start_matches(|c: char| c.is_ascii_digit()).trim_start_matches('_'),
        None => title_of(name),
    };
    let dashed = rest.replace('_', "-");
    without_hash_suffix(&dashed).to_string()
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
    fn a_taken_track_takes_the_lowest_free_number_of_its_tracker() {
        let siblings: Vec<String> = ["PJ.001_Fabi-web", "PJ.009_Phospo", "PJ.010_Cangrebot", "PR.002_Other"].map(String::from).to_vec();
        assert_eq!(free_track("PJ.009_Test", &siblings), "PJ.002_Test");
        assert_eq!(free_track("PJ.005_Free", &siblings), "PJ.005_Free");
        assert_eq!(free_track("PJ.009", &siblings), "PJ.002");
        assert_eq!(free_track("PR.002_Mine", &siblings), "PR.001_Mine");
        assert_eq!(free_track("PJ.0009_Wide", &["PJ.0009_X".to_string()]), "PJ.0001_Wide");
    }

    #[test]
    fn track_names_and_their_titles() {
        assert!(is_track_name("PJ.1007_Prueba-7") && is_track_name("PR.001") && !is_track_name("PJ.Prueba"));
        assert_eq!(title_of("PJ.1007_Prueba-7"), "Prueba-7");
        assert_eq!(title_of("01.New-Node"), "New-Node");
        assert_eq!(title_of("Prueba 4"), "Prueba 4");
        assert_eq!(title_of("PJ.001"), "");
    }

    #[test]
    fn a_workboard_title_drops_the_tracker_the_number_and_a_trailing_hash() {
        assert_eq!(workboard_title("Pj.154_Google-Labs_Agent-9f0d172a"), "Google-Labs-Agent");
        assert_eq!(workboard_title("01.New-Node"), "New-Node");
        assert_eq!(workboard_title("Thesis"), "Thesis");
        assert_eq!(workboard_title("Plan-2024"), "Plan-2024");
        assert_eq!(workboard_title("Plan-123456"), "Plan-123456");
        assert_eq!(workboard_title("Face-decade"), "Face-decade");
        assert_eq!(tracker_number("Pj.154_X", "PJ."), Some(154));
        assert_eq!(tracker_number("PR.7_X", "PJ."), None);
        assert_eq!(tracker_number("Pj.Name", "PJ."), None);
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