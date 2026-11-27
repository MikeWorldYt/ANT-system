// Builds a folder's dictionary with the connected AI client from the names of the files waiting in its review folder, for the editor window and the MCP.
mod agent;
mod runner;
mod scan;

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{dir_entries, io_err, resolve, write_atomic};
use crate::services::automations::dictionary::{compile, parse, parse_globals, DICTIONARY_FILE};
use crate::services::automations::dictionary_lint::{lint, Level};
use crate::services::automations::downloads as auto_downloads;
use crate::services::intake::review_child;
use crate::services::tree::tree_store::{get_node, load_level};

const SCAN_FILE: &str = "shared-words.md";

// The editor opens the dictionary of the node that holds the review folder, so the files are read where they land: its review folder, or the node itself when it takes them.
pub fn review_dir(root: &Path, folder: &[String]) -> R<PathBuf> {
    let data = load_level(root, folder)?;
    let node = get_node(&data, folder).ok_or_else(|| FmError::NotFound { rel_parts: folder.to_vec() })?;
    match review_child(folder, node) {
        Some(review) => resolve(root, &[folder, &[review.to_string()]].concat()),
        None => resolve(root, folder),
    }
}

// Beside the Downloads dictionary for Downloads; any other folder gets its own temp folder, never a file inside the storage.
pub fn work_dir(folder: &[String], downloads: bool) -> PathBuf {
    if downloads {
        if let Some(dir) = auto_downloads::dictionary_path().parent() {
            return dir.to_path_buf();
        }
    }
    let key: String = folder.join("-").chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    std::env::temp_dir().join("ANT").join("automations").join("build").join(if key.is_empty() { "root".to_string() } else { key })
}

// The same files the automations see: no temporary "~" copies, no hidden, system or dot files.
fn file_names(dir: &Path) -> R<Vec<String>> {
    let mut names = Vec::new();
    for entry in dir_entries(dir, "list folder")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_file() && !name.starts_with('~') && !crate::services::downloads::is_concealed(&entry) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

// The AI may still wrap its answer in a fence or a sentence; only the fenced block is the dictionary then.
fn unfenced(raw: &str) -> &str {
    let Some(open) = raw.find("```") else { return raw.trim() };
    let body = raw[open..].split_once('\n').map_or("", |(_, rest)| rest);
    body.find("\n```").map_or(body, |end| &body[..end]).trim()
}

fn checked(raw: &str) -> R<String> {
    let bad = |why: String| FmError::Internal { message: format!("The AI answered with no usable dictionary: {why}") };
    let text = unfenced(raw);
    if text.is_empty() {
        return Err(bad("the answer was empty".to_string()));
    }
    if !text.lines().any(|l| l.starts_with("# ")) {
        return Err(bad("it has no `# ` title line".to_string()));
    }
    if let Some(issue) = lint(text).into_iter().find(|i| i.level == Level::Error) {
        return Err(bad(format!("line {}: {}", issue.line, issue.message)));
    }
    let source = Path::new(DICTIONARY_FILE);
    parse_globals(text, source).map_err(|e| bad(e.to_string()))?;
    for spec in parse(text) {
        compile(&spec, source).map_err(|e| bad(e.to_string()))?;
    }
    Ok(format!("{text}\n"))
}

// Blocks for as long as the AI client works, up to its timeout; callers run it off the UI thread and without the storage lock.
pub fn build(dir: &Path, work: &Path, title: &str, current: &str) -> R<String> {
    let (client, cli) = runner::choose()?;
    let names = file_names(dir)?;
    if names.len() < 2 {
        return Err(FmError::Internal { message: format!("{title} needs at least two files to find the words they share") });
    }
    let found = scan::shared(&names);
    if found.is_empty() {
        return Err(FmError::Internal { message: format!("No two files in {title} share a word, so there is nothing to build a dictionary from") });
    }
    let listing = scan::render(title, names.len(), &found);
    fs::create_dir_all(work).map_err(|e| io_err("create the dictionary build folder", work, &e))?;
    let path = work.join(SCAN_FILE);
    write_atomic(&path, listing.as_bytes())?;
    checked(&agent::ask(client, &cli, work, agent::prompt(title, current, &listing, &path))?)
}
