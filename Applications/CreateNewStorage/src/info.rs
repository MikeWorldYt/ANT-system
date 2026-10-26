// Writes the people and dates a Tree.yaml entry carries into the folder's .ant/info.json.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use crate::config::{ANT_DIR, INFO_NAME};
use crate::fs_ops::{make_dirs_tracked, write_file_atomic};
use crate::{Level, Progress};

// Same order the file is written in.
const FIELDS: [&str; 3] = ["owner", "created", "status"];

pub fn write_info(folder: &Path, entry: &Mapping, created: &mut Vec<PathBuf>, progress: Progress) -> Result<(), String> {
    let pairs: Vec<(&str, String)> = FIELDS
        .iter()
        .filter_map(|key| entry.get(*key).and_then(|v| v.as_str()).map(|v| (*key, short_date(key, v))))
        .collect();
    if pairs.is_empty() {
        return Ok(());
    }
    let dir = folder.join(ANT_DIR);
    make_dirs_tracked(&dir, created)?;
    let dest = dir.join(INFO_NAME);
    write_file_atomic(&dest, render(&pairs).as_bytes())?;
    progress(Level::Info, &format!("  ✔ {}", dest.display()));
    Ok(())
}

// 2024-06-01 is stored as 24-06-01, the short form the app reads.
fn short_date(key: &str, value: &str) -> String {
    let b = value.as_bytes();
    let dated = key == "created" && b.len() == 10 && b.iter().enumerate().all(|(i, c)| if i == 4 || i == 7 { *c == b'-' } else { c.is_ascii_digit() });
    if dated { value[2..].to_string() } else { value.to_string() }
}

fn render(pairs: &[(&str, String)]) -> String {
    let lines: Vec<String> = pairs.iter().map(|(k, v)| format!("  \"{k}\": {}", quote(v))).collect();
    format!("{{\n{}\n}}\n", lines.join(",\n"))
}

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_fields_with_a_short_created_date() {
        let entry: Mapping = serde_yaml::from_str("owner: Mike Diaz\ntype: WKB\nstatus: in-progress\ncreated: 2024-06-01\n").unwrap();
        let mut made = Vec::new();
        let tmp = crate::test_dir::TempDir::new("info");
        write_info(tmp.path(), &entry, &mut made, &mut |_, _| {}).unwrap();
        let text = std::fs::read_to_string(tmp.path().join(".ant/info.json")).unwrap();
        assert_eq!(text, "{\n  \"owner\": \"Mike Diaz\",\n  \"created\": \"24-06-01\",\n  \"status\": \"in-progress\"\n}\n");
    }

    #[test]
    fn entries_without_fields_write_nothing_and_quotes_are_escaped() {
        let tmp = crate::test_dir::TempDir::new("info-none");
        let plain: Mapping = serde_yaml::from_str("type: NODE-L1\n").unwrap();
        write_info(tmp.path(), &plain, &mut Vec::new(), &mut |_, _| {}).unwrap();
        assert!(!tmp.path().join(".ant").exists());
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }
}
