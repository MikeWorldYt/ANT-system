// Edits only the [mcp_servers.ant-system] table of Codex's config.toml as text, since there is no TOML crate and the user's file must stay byte-identical elsewhere.
use std::fs;
use std::path::{Path, PathBuf};

use super::shared::{home_dir, io_err, on_path, read_text, same_exe, write_checked};
use super::{exe_path, internal, AiClient, SERVER_NAME};
use crate::domain::error::{FmError, R};

const FILE: &str = "config.toml";
const TABLE: &str = "mcp_servers";

pub(crate) fn config_dir(codex_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    codex_home.map(Path::to_path_buf).or_else(|| home.map(|h| h.join(".codex")))
}

fn current_dir() -> Option<PathBuf> {
    let codex_home = std::env::var_os("CODEX_HOME").map(PathBuf::from).filter(|p| p.is_absolute());
    config_dir(codex_home.as_deref(), home_dir().as_deref())
}

// The dotted key path of a table header, without quotes or spaces; None for any other line.
fn header(line: &str) -> Option<String> {
    let t = line.split('#').next()?.trim();
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect())
}

fn key_path(line: &str) -> String {
    line.split('=').next().unwrap_or("").chars().filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'').collect()
}

fn is_ours(path: &str) -> bool {
    let own = format!("{TABLE}.{SERVER_NAME}");
    path == own || path.strip_prefix(own.as_str()).is_some_and(|rest| rest.starts_with('.'))
}

// Inline or dotted forms of mcp_servers cannot be edited as a table block without risking an invalid file.
fn uses_other_form(text: &str) -> bool {
    let mut current = String::new();
    for line in text.lines() {
        if let Some(h) = header(line) {
            current = h;
            continue;
        }
        let key = key_path(line);
        if line.trim().is_empty() || line.trim_start().starts_with('#') || !line.contains('=') { continue; }
        let root_form = current.is_empty() && (key == TABLE || key.starts_with(&format!("{TABLE}.")));
        let own_in_parent = current == TABLE && (key == SERVER_NAME || key.starts_with(&format!("{SERVER_NAME}.")));
        if root_form || own_in_parent { return true; }
    }
    false
}

fn guard(text: &str) -> Result<(), &'static str> {
    if uses_other_form(text) { Err("defines mcp_servers in a form Folder Manager cannot edit") } else { Ok(()) }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn unquote(value: &str) -> Option<String> {
    let v = value.trim();
    if let Some(lit) = v.strip_prefix('\'') { return lit.split('\'').next().map(str::to_string); }
    let body = v.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => out.push(chars.next()?),
            c => out.push(c),
        }
    }
    None
}

// Line ranges (start, end exclusive) of our table and its sub-tables, trailing blank lines left out.
fn blocks(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, line) in lines.iter().enumerate() {
        let Some(h) = header(line) else { continue };
        if let Some(s) = start.take() { out.push((s, trim_blank(lines, s, i))); }
        if is_ours(&h) { start = Some(i); }
    }
    if let Some(s) = start { out.push((s, trim_blank(lines, s, lines.len()))); }
    out
}

fn trim_blank(lines: &[&str], from: usize, mut end: usize) -> usize {
    while end > from + 1 && lines[end - 1].trim().is_empty() { end -= 1; }
    end
}

pub(crate) fn entry_command(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    blocks(&lines).into_iter().find_map(|(s, e)| {
        if header(lines[s])? != format!("{TABLE}.{SERVER_NAME}") { return None; }
        lines[s + 1..e].iter().find_map(|l| (key_path(l) == "command").then(|| l.split_once('=').and_then(|(_, v)| unquote(v))).flatten())
    })
}

pub(crate) fn without_entry(text: &str) -> Result<String, &'static str> {
    guard(text)?;
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let found = blocks(&lines);
    if found.is_empty() { return Ok(text.to_string()); }
    let mut kept: Vec<&str> = Vec::new();
    let mut next = 0;
    for (s, e) in found {
        kept.extend(&lines[next..s]);
        next = e;
    }
    kept.extend(&lines[next..]);
    // The blank line connect added before the table goes with it when nothing follows.
    while kept.last().is_some_and(|l| l.trim().is_empty()) && lines[next..].iter().all(|l| l.trim().is_empty()) { kept.pop(); }
    Ok(kept.concat())
}

pub(crate) fn with_entry(text: &str, exe: &str) -> Result<String, &'static str> {
    let mut out = without_entry(text)?;
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    if !out.is_empty() {
        if !out.ends_with('\n') { out.push_str(nl); }
        out.push_str(nl);
    }
    out.push_str(&format!("[{TABLE}.{SERVER_NAME}]{nl}command = {}{nl}args = [\"--mcp\"]{nl}", quote(exe)));
    Ok(out)
}

fn refused(file: &Path, reason: &str) -> FmError {
    internal(format!("{} {reason}; it was left untouched", file.display()))
}

fn apply(dir: &Path, edit: impl FnOnce(&str) -> Result<String, &'static str>) -> R<()> {
    let file = dir.join(FILE);
    let read = read_text(&file)?;
    let next = edit(read.as_deref().unwrap_or("")).map_err(|reason| refused(&file, reason))?;
    if read.as_deref().unwrap_or("") == next { return Ok(()); }
    fs::create_dir_all(dir).map_err(|e| io_err("create", dir, e))?;
    write_checked(&file, read.as_deref(), &next)
}

pub(crate) fn connect_in(dir: &Path, exe: &str) -> R<()> {
    apply(dir, |text| with_entry(text, exe))
}

pub(crate) fn disconnect_in(dir: &Path) -> R<()> {
    apply(dir, without_entry)
}

pub(crate) fn is_connected_in(dir: &Path, exe: &str) -> R<bool> {
    let text = read_text(&dir.join(FILE))?.unwrap_or_default();
    Ok(entry_command(&text).is_some_and(|c| same_exe(&c, exe)))
}

pub(super) fn status() -> R<AiClient> {
    let dir = current_dir();
    let present = dir.as_ref().filter(|d| d.is_dir());
    let installed = present.is_some() || on_path(&["codex.exe", "codex.cmd"]);
    let connected = match (present, exe_path()) {
        (Some(dir), Ok(exe)) => is_connected_in(dir, &exe).unwrap_or(false),
        _ => false,
    };
    Ok(AiClient { id: "codex", name: "Codex", installed, connected, config_path: dir.map(|d| d.join(FILE).display().to_string()), restart_needed: false })
}

fn required_dir() -> R<PathBuf> {
    current_dir().ok_or_else(|| internal("Could not locate the Codex config folder"))
}

pub(super) fn connect() -> R<AiClient> {
    connect_in(&required_dir()?, &exe_path()?)?;
    status()
}

pub(super) fn disconnect() -> R<AiClient> {
    disconnect_in(&required_dir()?)?;
    status()
}
