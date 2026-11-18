// Points at the lines of a dictionary the app would read differently from what the user meant, as the editor window shows them.
use std::collections::HashSet;

use regex::RegexBuilder;
use serde::Serialize;

use crate::infra::fs_safe::validate_component;

const KEYS: [&str; 8] = ["term", "topics", "only", "exclude", "ext", "pattern", "prefix", "suffix"];
const NODE_PREFIXES: [&str; 2] = ["NODE^.", "NODE."];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub line: usize,
    pub level: Level,
    pub message: String,
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

// "ter" reads as a slip of "term", while a word far from every key gets no guess.
fn unknown_key(key: &str) -> String {
    let lower = key.to_lowercase();
    match KEYS.iter().map(|k| (distance(&lower, k), k)).min().filter(|(d, _)| *d <= 2) {
        Some((_, k)) => format!("Unknown key `{key}`: did you mean `{k}`?"),
        None => format!("Unknown key `{key}`: use term, topics, only, exclude, ext or pattern"),
    }
}

struct Open {
    line: usize,
    name: String,
    matchers: bool,
    // Inside the entry's `### files`, where each `- ` line lists whole file names.
    files: bool,
}

fn close(open: Option<Open>, issues: &mut Vec<Issue>) {
    if let Some(entry) = open.filter(|e| !e.matchers) {
        issues.push(Issue { line: entry.line, level: Level::Warning, message: format!("`{}` has no term, topics, only, ext or pattern, so it catches nothing", entry.name) });
    }
}

pub fn lint(text: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut names = HashSet::new();
    let (mut titled, mut comment, mut open): (bool, Option<usize>, Option<Open>) = (false, None, None);
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let mut push = |level: Level, message: String| issues.push(Issue { line: line_no, level, message });
        // `%%` toggles a comment block; text between two of them is never read.
        let mut line = String::new();
        for (k, part) in raw.split("%%").enumerate() {
            if k > 0 {
                comment = if comment.is_some() { None } else { Some(line_no) };
            }
            if comment.is_none() {
                line.push_str(part);
            }
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if super::dictionary::is_section(line) {
            match open.as_mut() {
                Some(entry) => entry.files = true,
                None => issues.push(Issue { line: line_no, level: Level::Error, message: "`### files` belongs inside an entry: put a `## heading` above it".to_string() }),
            }
            continue;
        }
        if let Some(entry) = open.as_mut().filter(|e| e.files) {
            if let Some(names) = line.strip_prefix("- ") {
                for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    if validate_component(name).is_err() {
                        issues.push(Issue { line: line_no, level: Level::Error, message: format!("`{name}` cannot be a file name") });
                    }
                }
                entry.matchers = true;
                continue;
            }
        }
        if let Some(heading) = line.strip_prefix('#') {
            let name = heading.trim_start_matches('#').trim();
            if !titled && !heading.starts_with('#') {
                titled = true;
                continue;
            }
            close(open.take(), &mut issues);
            let mut push = |level: Level, message: String| issues.push(Issue { line: line_no, level, message });
            if !heading.starts_with(['#', ' ']) {
                push(Level::Error, "A heading needs a space after its #".to_string());
            }
            if name.is_empty() {
                push(Level::Error, "This heading has no name".to_string());
                continue;
            }
            if NODE_PREFIXES.iter().any(|p| name.len() >= p.len() && name[..p.len()].eq_ignore_ascii_case(p) && name[p.len()..].trim().is_empty()) {
                push(Level::Error, "A NODE heading needs the node name after the dot".to_string());
            }
            if name.starts_with("@/") && name[2..].trim_matches(['/', ' ']).is_empty() {
                push(Level::Error, "An @/ heading needs the node path from the storage root, like @/500.HOBBIES/08.Art".to_string());
            }
            let path = name.starts_with("@/") || name.starts_with("./") || name.starts_with("../");
            if !name.to_ascii_uppercase().starts_with("NODE") && !path && validate_component(name).is_err() {
                push(Level::Error, format!("`{name}` cannot be a folder name, so the entry is ignored"));
            }
            if !names.insert(name.to_lowercase()) {
                push(Level::Error, format!("`{name}` is already an entry; the second one replaces nothing and is confusing"));
            }
            open = Some(Open { line: line_no, name: name.to_string(), matchers: false, files: false });
            continue;
        }
        let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) else {
            if line.starts_with('-') || line.starts_with('*') {
                push(Level::Error, "A list item needs a space after its dash: `- term: ...`".to_string());
            } else {
                push(Level::Warning, "Not part of the format: write a `## heading` or a `- key: values` line, or wrap notes in %% %%".to_string());
            }
            continue;
        };
        let Some((key, value)) = item.split_once(':') else {
            push(Level::Error, "Expected `key: values` here".to_string());
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        // Above the first entry only cuts and affixes go: `term` and `pattern` text leaves every name, `prefix` and `suffix` join it, and none groups.
        if open.is_none() && matches!(key, "term" | "pattern" | "prefix" | "suffix") {
            if key == "pattern" && RegexBuilder::new(value).case_insensitive(true).build().is_err() {
                push(Level::Error, "The pattern does not compile".to_string());
            }
            continue;
        }
        let Some(entry) = open.as_mut() else {
            let reason = if KEYS.contains(&key) { format!("Above the first entry only `term`, `pattern`, `prefix` and `suffix` go, for every name; `{key}` needs a `## heading` above it") } else { "This line is outside any entry: put a `## heading` above it".to_string() };
            push(Level::Error, reason);
            continue;
        };
        if !KEYS.contains(&key) {
            push(Level::Error, unknown_key(key));
            continue;
        }
        if value.split(',').all(|v| v.trim().is_empty()) {
            push(Level::Warning, format!("`{key}` has no values"));
            continue;
        }
        if key == "pattern" {
            if let Err(e) = RegexBuilder::new(value).case_insensitive(true).build() {
                let first = e.to_string().lines().last().unwrap_or("invalid pattern").trim().trim_start_matches("error:").trim().to_string();
                push(Level::Error, format!("The pattern does not compile: {first}"));
                continue;
            }
        }
        if !matches!(key, "exclude" | "prefix" | "suffix") {
            entry.matchers = true;
        }
    }
    close(open, &mut issues);
    if let Some(start) = comment {
        issues.push(Issue { line: start, level: Level::Error, message: "This %% comment is never closed, so everything after it is ignored".to_string() });
    }
    issues.sort_by_key(|i| i.line);
    issues
}
