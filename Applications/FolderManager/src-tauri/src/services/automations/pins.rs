// Pins a file to a dictionary entry by writing its name under the entry's `### files`, keeping the rest of the file as the user wrote it.
use std::fs;
use std::path::{Path, PathBuf};

use super::dictionary::{is_section, parse, DICTIONARY_FILE, FILES_HEADING};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, join_rel, validate_component, write_atomic, StorageLock};

fn entry_name(line: &str) -> Option<&str> {
    let t = line.trim();
    (t.starts_with("##") && !is_section(t)).then(|| t.trim_start_matches('#').trim())
}

fn is_files_heading(line: &str) -> bool {
    is_section(line.trim())
}

// Takes the name out of every `### files` list; a list left empty goes with its heading and the blank line above it.
fn drop_name(lines: &mut Vec<String>, file: &str) {
    let mut in_files = false;
    lines.retain_mut(|line| {
        if entry_name(line).is_some() {
            in_files = false;
        } else if is_files_heading(line) {
            in_files = true;
        } else if let (true, Some(items)) = (in_files, line.trim().strip_prefix("- ")) {
            let kept: Vec<&str> = items.split(',').map(str::trim).filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case(file)).collect();
            if kept.is_empty() {
                return false;
            }
            *line = format!("- {}", kept.join(", "));
        }
        true
    });
    let mut i = 0;
    while i < lines.len() {
        let listed = lines.iter().skip(i + 1).find(|l| !l.trim().is_empty()).is_some_and(|l| l.trim().starts_with("- "));
        if is_files_heading(&lines[i]) && !listed {
            lines.remove(i);
            if i > 0 && lines[i - 1].trim().is_empty() {
                lines.remove(i - 1);
                i -= 1;
            }
            continue;
        }
        i += 1;
    }
}

fn joined(lines: &[String], text: &str) -> String {
    let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
    lines.join(ending) + ending
}

pub fn unpin_text(text: &str, file: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    drop_name(&mut lines, file);
    joined(&lines, text)
}

// The name leaves every other `### files` list first, so a file is pinned to one entry only.
pub fn pin_text(text: &str, entry: &str, file: &str) -> R<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    drop_name(&mut lines, file);
    let start = lines.iter().position(|l| entry_name(l).is_some_and(|n| n.eq_ignore_ascii_case(entry))).ok_or_else(|| FmError::Internal { message: format!("The dictionary has no entry {entry}") })?;
    let end = lines.iter().enumerate().skip(start + 1).find(|(_, l)| entry_name(l).is_some() || l.trim().starts_with("# ")).map_or(lines.len(), |(i, _)| i);
    match (start + 1..end).find(|&i| is_files_heading(&lines[i])) {
        Some(heading) => match (heading + 1..end).find(|&i| lines[i].trim().starts_with("- ")) {
            Some(first) => lines[first] = format!("{}, {file}", lines[first].trim_end()),
            None => lines.insert(heading + 1, format!("- {file}")),
        },
        None => {
            let mut at = end;
            while at > start + 1 && lines[at - 1].trim().is_empty() {
                at -= 1;
            }
            lines.splice(at..at, [String::new(), format!("### {FILES_HEADING}"), format!("- {file}")]);
        }
    }
    Ok(joined(&lines, text))
}

// The dictionary that holds the entry, the closest one first, since a closer entry replaces a farther one of the same name.
pub fn storage_dictionary(root: &Path, folder: &[String], entry: &str) -> R<PathBuf> {
    (0..=folder.len())
        .rev()
        .map(|end| join_rel(root, &folder[..end]).join(".ant").join(DICTIONARY_FILE))
        .find(|path| fs::read_to_string(path).is_ok_and(|text| parse(&text).iter().any(|s| s.folder.eq_ignore_ascii_case(entry))))
        .ok_or_else(|| FmError::Internal { message: format!("No dictionary above this folder has the entry {entry}") })
}

// No entry unpins: the name leaves every list and the rules place the file again.
pub fn pin(path: &Path, entry: Option<&str>, file: &str) -> R<()> {
    validate_component(file)?;
    let text = fs::read_to_string(path).map_err(|e| io_err("read the dictionary", path, &e))?;
    let next = match entry {
        Some(entry) => pin_text(&text, entry, file)?,
        None => unpin_text(&text, file),
    };
    if next != text {
        write_atomic(path, next.as_bytes())?;
    }
    Ok(())
}

pub fn pin_in_storage(root: &Path, folder: &[String], entry: Option<&str>, file: &str) -> R<()> {
    let _lock = StorageLock::acquire(root)?;
    match entry {
        Some(entry) => pin(&storage_dictionary(root, folder, entry)?, Some(entry), file),
        None => (0..=folder.len()).map(|end| join_rel(root, &folder[..end]).join(".ant").join(DICTIONARY_FILE)).filter(|p| p.is_file()).try_for_each(|p| pin(&p, None, file)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# D\n\n## A\n- term: x\n\n### files\n- one.jpg, two.png\n\n## B\n- term: y\n\n## C\n- term: z\n";

    #[test]
    fn a_pin_moves_the_name_between_entries_and_keeps_the_rest() {
        let moved = pin_text(TEXT, "B", "one.jpg").unwrap();
        assert!(moved.contains("## A\n- term: x\n\n### files\n- two.png\n"));
        assert!(moved.contains("## B\n- term: y\n\n### files\n- one.jpg\n"));
        let again = pin_text(&moved, "B", "three.pdf").unwrap();
        assert!(again.contains("### files\n- one.jpg, three.pdf\n"));
        let emptied = pin_text(TEXT, "C", "two.png").unwrap();
        let emptied = pin_text(&emptied, "C", "one.jpg").unwrap();
        assert!(emptied.contains("## A\n- term: x\n\n## B"));
        let freed = unpin_text(&again, "one.jpg");
        assert!(freed.contains("### files\n- three.pdf\n") && !freed.contains("one.jpg"));
        assert!(pin_text(TEXT, "Missing", "a.jpg").is_err());
    }

    #[test]
    fn pinned_names_are_read_back_by_the_parser() {
        let specs = parse(&pin_text(TEXT, "C", "photo.jpg").unwrap());
        assert_eq!(specs.iter().find(|s| s.folder == "C").unwrap().files, vec!["photo.jpg".to_string()]);
        assert_eq!(specs.iter().find(|s| s.folder == "A").unwrap().files, vec!["one.jpg".to_string(), "two.png".to_string()]);
    }
}
