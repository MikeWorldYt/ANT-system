// Walks the folders on disk for the search's slow pass, matching them the way the database search matches its rows.
use std::path::Path;

use serde_yaml::Mapping;

use crate::infra::fs_safe::io_err;
use crate::infra::scan::list_subdirs;
use crate::domain::search::{canonical_key, Query};
use crate::domain::tree_types::flags_in;
use crate::domain::config::HIDDEN_FLAG;
use crate::domain::error::R;
use crate::infra::storage::info::read_info;
use crate::services::tree::tree_store::{self, get_node};

// Past this many matches the walk stops, so a broad search cannot grow the list without bound.
const CAP: usize = 300;

#[derive(Debug, serde::Serialize)]
pub struct FileHit {
    pub folder: Vec<String>,
    pub name: String,
}

pub fn search_disk(root: &Path, query: &Query) -> R<Vec<Vec<String>>> {
    let data = tree_store::load(root)?;
    let mut found = Vec::new();
    visit(root, &mut Vec::new(), &data, &mut |folder, parts| {
        if matches(folder, parts, query) {
            found.push(parts.to_vec());
        }
        found.len() >= CAP
    })?;
    Ok(found)
}

// A file search needs words: filters and scope alone only narrow which folders are read.
pub fn search_files(root: &Path, query: &Query) -> R<Vec<FileHit>> {
    let mut found = Vec::new();
    if query.text.is_empty() {
        return Ok(found);
    }
    let data = tree_store::load(root)?;
    let mut failure = None;
    visit(root, &mut Vec::new(), &data, &mut |folder, parts| {
        if in_query(folder, parts, query) {
            match list_files(folder) {
                Ok(names) => found.extend(
                    names.into_iter().filter(|name| query.text.iter().all(|word| contains(name, word))).map(|name| FileHit { folder: parts.to_vec(), name }),
                ),
                Err(e) => failure = Some(e),
            }
        }
        failure.is_some() || found.len() >= CAP
    })?;
    match failure {
        Some(e) => Err(e),
        None => {
            found.truncate(CAP);
            Ok(found)
        }
    }
}

// Visits every folder that is not hidden, parents first; `stop` returning true ends the walk.
fn visit(dir: &Path, parts: &mut Vec<String>, data: &Mapping, stop: &mut dyn FnMut(&Path, &[String]) -> bool) -> R<bool> {
    for name in list_subdirs(dir)? {
        let folder = dir.join(&name);
        parts.push(name);
        // A hidden folder is neither matched nor read.
        if get_node(data, parts).is_some_and(|node| flags_in(node).iter().any(|f| f == HIDDEN_FLAG)) {
            parts.pop();
            continue;
        }
        let done = stop(&folder, parts) || visit(&folder, parts, data, stop)?;
        parts.pop();
        if done {
            return Ok(true);
        }
    }
    Ok(false)
}

// Dot files are the system's own (.ant, thumbnails), so they never show up as results.
fn list_files(dir: &Path) -> R<Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(Vec::new()),
        Err(e) => return Err(io_err("list folder", dir, &e)),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| io_err("list folder", dir, &e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && entry.path().is_file() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn matches(folder: &Path, parts: &[String], query: &Query) -> bool {
    let name = parts.last().map_or("", String::as_str);
    in_query(folder, parts, query) && query.text.iter().all(|word| contains(name, word))
}

// The folder's place and info decide whether it, or the files inside it, can match.
fn in_query(folder: &Path, parts: &[String], query: &Query) -> bool {
    let in_scope = match &query.scope {
        Some(scope) => scope_matches(parts, scope),
        None => true,
    };
    in_scope && (query.filters.is_empty() || filters_match(folder, &query.filters))
}

// A scope is a folder name or number at any level; with a slash, or a leading one, it is a path from the storage root.
fn scope_matches(parts: &[String], scope: &str) -> bool {
    let trimmed = scope.trim_matches('/');
    if scope.starts_with('/') || trimmed.contains('/') {
        let path = parts.join("/");
        return path.eq_ignore_ascii_case(trimmed) || starts(&path, &format!("{trimmed}/"));
    }
    parts.iter().any(|part| starts(part, trimmed) || contains(part, &format!(".{trimmed}")))
}

// Filters live in the folder's info.json, the only place a folder off the database can hold them.
fn filters_match(folder: &Path, filters: &[(String, String)]) -> bool {
    let Some(info) = read_info(folder) else { return false };
    filters.iter().all(|(key, wanted)| info.fields.iter().any(|field| canonical_key(&field.key) == *key && value_matches(&field.value, wanted)))
}

// A value written as "=text" is a picked value and must match whole; plain text matches anywhere inside.
fn value_matches(field: &str, wanted: &str) -> bool {
    match wanted.strip_prefix('=') {
        Some(whole) => field.eq_ignore_ascii_case(whole),
        None => contains(field, wanted),
    }
}

fn starts(text: &str, prefix: &str) -> bool {
    text.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase())
}

fn contains(text: &str, part: &str) -> bool {
    text.to_ascii_lowercase().contains(&part.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::search::parse_query;
    use crate::test_support::TempDir;

    fn paths(found: Vec<Vec<String>>) -> Vec<String> {
        found.into_iter().map(|parts| parts.join("/")).collect()
    }

    #[test]
    fn disk_folders_match_by_name_scope_and_info_like_the_database() {
        let dir = TempDir::new("disk-search");
        dir.mkdir("200.WORK/06.Test-dt/98.Test-dt/New folder");
        dir.mkdir("200.WORK/06.Test-dt/PJ.013_New-Node/.ant");
        std::fs::write(dir.path().join("200.WORK/06.Test-dt/PJ.013_New-Node/.ant/info.json"), r#"{"status": "hold"}"#).unwrap();
        let found = |q: &str| paths(search_disk(dir.path(), &parse_query(q)).unwrap());
        assert_eq!(found("new"), ["200.WORK/06.Test-dt/98.Test-dt/New folder", "200.WORK/06.Test-dt/PJ.013_New-Node"]);
        assert_eq!(found("in:200.WORK new").len(), 2);
        assert_eq!(found("in:200.WORK/06.Test-dt/98.Test-dt new"), ["200.WORK/06.Test-dt/98.Test-dt/New folder"]);
        assert!(found("in:300.ACADEMIC new").is_empty());
        assert_eq!(found("status:hold"), ["200.WORK/06.Test-dt/PJ.013_New-Node"]);
        assert!(found("status:done").is_empty());
    }

    #[test]
    fn files_are_found_with_their_folder_and_skip_hidden_places() {
        let yaml = "200.WORK:
  type: NODE-L1
  06.Test-dt:
    type: NODE-L2
    98.Test-dt: {type: NODE-L3}
  Secret:
    type: NODE-L2
    flags: hidden
";
        let dir = crate::test_support::storage("disk-files", &["200.WORK/06.Test-dt/98.Test-dt", "200.WORK/06.Test-dt/.ant", "200.WORK/Secret"], yaml);
        let write = |rel: &str| std::fs::write(dir.path().join(rel), "x").unwrap();
        write("200.WORK/06.Test-dt/98.Test-dt/Report Final.pdf");
        write("200.WORK/06.Test-dt/98.Test-dt/.report-hidden");
        write("200.WORK/06.Test-dt/.ant/report.json");
        write("200.WORK/Secret/report.txt");
        write("200.WORK/report-root.txt");
        let found = |q: &str| -> Vec<(String, String)> {
            search_files(dir.path(), &parse_query(q)).unwrap().into_iter().map(|h| (h.folder.join("/"), h.name)).collect()
        };
        assert_eq!(
            found("report"),
            [("200.WORK".to_string(), "report-root.txt".to_string()), ("200.WORK/06.Test-dt/98.Test-dt".to_string(), "Report Final.pdf".to_string())]
        );
        assert_eq!(found("report final").len(), 1);
        assert!(found("in:300.ACADEMIC report").is_empty());
        assert_eq!(found("in:98.Test-dt report").len(), 1);
        assert!(found("status:hold").is_empty());
    }
}
