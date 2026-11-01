// Walks the folders on disk for the search's slow pass, matching them the way the database search matches its rows.
use std::path::Path;

use crate::ant_core::names::list_subdirs;
use crate::ant_core::search::{canonical_key, Query};
use crate::error::R;
use crate::storage::info::read_info;

// Past this many matches the walk stops, so a broad search cannot grow the list without bound.
const CAP: usize = 300;

pub fn search_disk(root: &Path, query: &Query) -> R<Vec<Vec<String>>> {
    let mut found = Vec::new();
    walk(root, &mut Vec::new(), query, &mut found)?;
    Ok(found)
}

fn walk(dir: &Path, parts: &mut Vec<String>, query: &Query, found: &mut Vec<Vec<String>>) -> R<()> {
    for name in list_subdirs(dir)? {
        if found.len() >= CAP {
            break;
        }
        let folder = dir.join(&name);
        parts.push(name);
        if matches(&folder, parts, query) {
            found.push(parts.clone());
        }
        walk(&folder, parts, query, found)?;
        parts.pop();
    }
    Ok(())
}

fn matches(folder: &Path, parts: &[String], query: &Query) -> bool {
    let name = parts.last().map_or("", String::as_str);
    let in_scope = match &query.scope {
        Some(scope) => scope_matches(parts, scope),
        None => true,
    };
    in_scope && query.text.iter().all(|word| contains(name, word)) && (query.filters.is_empty() || filters_match(folder, &query.filters))
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
    use crate::ant_core::search::parse_query;
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
}
