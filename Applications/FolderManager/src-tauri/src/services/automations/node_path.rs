// Resolves `@/` headings: each segment by full name, title or `*` wildcard, and a missing last one becomes a folder the run creates.
use std::collections::HashMap;
use std::path::Path;

use crate::domain::config::NewKind;
use crate::domain::error::{FmError, R};
use crate::domain::names::editable_part;
use crate::infra::fs_safe::join_rel;
use crate::infra::scan::list_tracked_subdirs;
use crate::services::mutations;

pub const ABS_PREFIX: &str = "@/";

pub struct NodePath {
    pub rel: Vec<String>,
    // The last segment does not exist yet; it stays as written until the run creates it.
    pub create: bool,
}

fn key(s: &str) -> String {
    s.trim().to_lowercase().replace([' ', '_'], "-")
}

// `*` stands for any run of characters, so `*.Teachnology` and `PJ.*_Kestrel` match whatever id the folder has.
fn glob(pattern: &str, name: &str) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = (pattern.to_lowercase().chars().collect(), name.to_lowercase().chars().collect());
    let (mut pi, mut ni, mut star, mut mark) = (0, 0, None, 0);
    while ni < n.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if pi < p.len() && p[pi] == n[ni] {
            pi += 1;
            ni += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

pub fn segment_matches(folder: &str, segment: &str) -> bool {
    if segment.contains('*') {
        return glob(segment, folder);
    }
    folder.eq_ignore_ascii_case(segment) || key(editable_part(folder)) == key(segment)
}

// What a missing segment creates: a tracker before the number (`PJ.*_Kestrel`, `PJ.006_Kestrel`) makes a worknode, anything else a node.
pub fn new_folder(segment: &str) -> (NewKind, String) {
    let id = |s: &str| s == "*" || (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    if let Some((tracker, rest)) = segment.split_once('.') {
        if let Some((number, title)) = rest.split_once('_') {
            if !tracker.is_empty() && tracker.bytes().all(|b| b.is_ascii_alphabetic()) && id(number) {
                return (NewKind::Worknode, title.trim().to_string());
            }
        }
        if id(tracker) {
            return (NewKind::Node, rest.trim().to_string());
        }
    }
    (NewKind::Node, segment.trim().to_string())
}

pub fn resolve(root: &Path, heading: &str) -> R<Option<NodePath>> {
    let Some(path) = heading.strip_prefix(ABS_PREFIX) else { return Ok(None) };
    let segments: Vec<&str> = path.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    let mut rel: Vec<String> = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        let found = list_tracked_subdirs(&join_rel(root, &rel))?.into_iter().find(|n| segment_matches(n, segment));
        match found {
            Some(name) => rel.push(name),
            None if i + 1 == segments.len() && !rel.is_empty() && !new_folder(segment).1.is_empty() => {
                rel.push((*segment).to_string());
                return Ok(Some(NodePath { rel, create: true }));
            }
            None => return Ok(None),
        }
    }
    Ok((!rel.is_empty()).then_some(NodePath { rel, create: false }))
}

// Creates the planned folder once per run, and reuses one that appeared since the dry run; `made` keeps what this run created.
pub fn ensure(root: &Path, target: &[String], made: &mut HashMap<Vec<String>, Vec<String>>) -> R<Vec<String>> {
    if let Some(done) = made.get(target) {
        return Ok(done.clone());
    }
    let (parent, last) = target.split_at(target.len().saturating_sub(1));
    let last = last.first().ok_or_else(|| FmError::Internal { message: "empty dictionary target".to_string() })?;
    if let Some(existing) = list_tracked_subdirs(&join_rel(root, parent))?.into_iter().find(|n| segment_matches(n, last)) {
        let rel = [parent, std::slice::from_ref(&existing)].concat();
        made.insert(target.to_vec(), rel.clone());
        return Ok(rel);
    }
    let (kind, title) = new_folder(last);
    let created = mutations::add_node_named(root, parent, kind, Some(&title))?.select.ok_or_else(|| FmError::Internal { message: format!("{title} was not created") })?;
    made.insert(target.to_vec(), created.clone());
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_any_id_and_titles_match_without_it() {
        assert!(segment_matches("03.Teachnology", "*.Teachnology"));
        assert!(segment_matches("PJ.006_Kestrel", "PJ.*_Kestrel"));
        assert!(segment_matches("03.Teachnology", "teachnology"));
        assert!(!segment_matches("03.Teachnology", "*.Science"));
        assert!(glob("*", "anything") && glob("a*c", "abbbc") && !glob("a*c", "abd"));
    }

    #[test]
    fn a_missing_segment_reads_as_a_worknode_or_a_node() {
        assert!(matches!(new_folder("PJ.*_Kestrel"), (NewKind::Worknode, t) if t == "Kestrel"));
        assert!(matches!(new_folder("PJ.006_Kestrel"), (NewKind::Worknode, t) if t == "Kestrel"));
        assert!(matches!(new_folder("*.Physics"), (NewKind::Node, t) if t == "Physics"));
        assert!(matches!(new_folder("Physics"), (NewKind::Node, t) if t == "Physics"));
    }

    #[test]
    fn only_the_last_segment_may_be_missing_and_the_run_creates_it_once() {
        let dir = crate::test_support::work("node-path");
        let found = resolve(dir.path(), "@/*.WORK/A").unwrap().unwrap();
        assert_eq!((found.rel, found.create), (vec!["100.WORK".to_string(), "01.A".to_string()], false));
        let planned = resolve(dir.path(), "@/100.WORK/PJ.*_Kestrel").unwrap().unwrap();
        assert!(planned.create);
        assert!(resolve(dir.path(), "@/100.WORK/Missing/Deeper").unwrap().is_none());
        let mut made = HashMap::new();
        let created = ensure(dir.path(), &planned.rel, &mut made).unwrap();
        assert!(created[1].ends_with("_Kestrel") && dir.path().join(created.join("/")).is_dir());
        assert_eq!(ensure(dir.path(), &planned.rel, &mut made).unwrap(), created);
    }
}
