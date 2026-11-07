// Folder-name rules: numeric slot ids, editable name parts and sanitizing of user input.
#[cfg(test)]
mod tests;

use crate::domain::config::{SUPPORT_NODES, WKB_IDENTIFIERS};
use crate::domain::tree_types::type_base;

pub(crate) const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

const FIRST_SLOT: u32 = 1;
const LAST_SLOT: u32 = 99;
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

// A moved node takes the lowest free slot of its new level and a workboard the next track; other names are kept as they are.
pub fn with_next_slot(name: &str, siblings: &[String]) -> String {
    if let Some((tracker, title)) = untracked_parts(name) {
        format!("{tracker}{:03}_{title}", next_track_number(tracker, siblings))
    } else if has_node_id(name) {
        format!("{:02}.{}", next_available_id(siblings), &name[3..])
    } else if is_track_name(name) {
        next_track(name, siblings)
    } else {
        name.to_string()
    }
}

// Each folder counts its own workboards, so a tracker's next number follows the highest one among the siblings.
pub fn next_track_number(letters: &str, siblings: &[String]) -> u32 {
    siblings.iter().filter_map(|s| track_number(s.split('_').next().unwrap_or(s), letters)).max().unwrap_or(0) + 1
}

// A workboard entering a folder continues that folder's count, at the same width.
pub fn next_track(name: &str, siblings: &[String]) -> String {
    let head = name.split('_').next().unwrap_or(name);
    let Some(dot) = head.find('.') else { return name.to_string() };
    let letters = &head[..=dot];
    let width = head.len() - letters.len();
    format!("{letters}{:0width$}{}", next_track_number(letters, siblings), &name[head.len()..])
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

// The type a support folder gives ("REV", "RES", "TPL"): the word prefix in any case, or the numeric one of older storages.
pub fn support_kind(name: &str) -> Option<&'static str> {
    SUPPORT_NODES.iter().find_map(|(word, legacy, kind)| {
        let is_word = name.get(..word.len()).is_some_and(|head| head.eq_ignore_ascii_case(word));
        (is_word || name.starts_with(legacy)).then_some(*kind)
    })
}

// A support folder still on its numeric id waits for the upgrade to rename it.
pub fn is_legacy_support(name: &str) -> bool {
    SUPPORT_NODES.iter().any(|(_, legacy, _)| name.starts_with(legacy))
}

// Nodes below the root open with a two-digit id ("01.Math"); "00" is not one, nor a numeric support folder.
pub fn has_node_id(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b'.' && &name[..2] != "00" && !is_legacy_support(name)
}

pub fn is_workboard_name(name: &str) -> bool {
    WKB_IDENTIFIERS.iter().any(|tracker| name.starts_with(tracker))
}

// A workboard still in a review folder ("PJ_Name") has its tracker but no number until it enters the storage.
pub fn untracked_parts(name: &str) -> Option<(&'static str, &str)> {
    WKB_IDENTIFIERS.iter().find_map(|tracker| name.strip_prefix(tracker.trim_end_matches('.')).and_then(|rest| rest.strip_prefix('_')).map(|title| (*tracker, title)))
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
        [.., name] => has_node_id(name) || is_workboard_name(name) || support_kind(name).is_some(),
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
