// Which children a reorder by id may renumber, the ids their other siblings hold, and how a typed id becomes a name.
use std::path::Path;

use serde::Serialize;
use serde_yaml::Mapping;

use crate::services::mutations::child_rel;
use crate::domain::config::{HIDDEN_FLAG, WKB_IDENTIFIERS};
use crate::domain::error::{FmError, R};
use crate::domain::names::{editable_part, is_main_category, is_movable_node, numeric_id, tracker_number};
use crate::domain::tree_types::{child_entries, flags_in, type_base};
use crate::infra::fs_safe::join_rel;
use crate::infra::scan::list_subdirs;
use crate::services::tree::tree_node::is_reserved;
use crate::services::tree::tree_store::{get_node, get_type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IdKind {
    Node,
    Worknode,
}

// One id in a folder: a node id has no tracker, a worknode number counts per tracker ("PJ.").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct IdKey {
    pub tracker: String,
    pub number: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderPlan {
    pub nodes: Vec<String>,
    pub worknodes: Vec<String>,
    pub taken: Vec<IdKey>,
    pub trackers: Vec<String>,
}

// A worknode name split as tracker, number and the rest: "PJ.", "001", "_Fabi-web".
pub(super) fn track_parts(name: &str) -> Option<(&str, &str, &str)> {
    let tracker = WKB_IDENTIFIERS.iter().find(|t| name.starts_with(*t))?;
    let rest = &name[tracker.len()..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0).then(|| (&name[..tracker.len()], &rest[..digits], &rest[digits..]))
}

// A hidden folder is not listed, so it keeps its id; worknodes need their tracker and number in place.
fn kind_of(data: &Mapping, parent: &[String], name: &str) -> Option<IdKind> {
    let rel = child_rel(parent, name);
    if get_node(data, &rel).is_some_and(|node| flags_in(node).iter().any(|f| f == HIDDEN_FLAG)) {
        return None;
    }
    let node_type = get_type(data, &rel);
    if parent.is_empty() {
        return (is_main_category(name) && type_base(&node_type) == "NODE" && !is_reserved(&rel)).then_some(IdKind::Node);
    }
    if is_movable_node(name, &node_type) {
        return Some(IdKind::Node);
    }
    let worknode = type_base(&node_type) == "WKB" || node_type.is_empty();
    (worknode && track_parts(name).is_some()).then_some(IdKind::Worknode)
}

// Any id a sibling outside the lists holds, in any letter case, so a new id never doubles it.
fn held_id(name: &str) -> Option<IdKey> {
    if let Some(number) = numeric_id(name) {
        return Some(IdKey { tracker: String::new(), number });
    }
    WKB_IDENTIFIERS.iter().find_map(|t| tracker_number(name, t).map(|number| IdKey { tracker: t.to_string(), number }))
}

pub(super) fn key_of(name: &str, kind: IdKind) -> Option<IdKey> {
    match kind {
        IdKind::Node => numeric_id(name).map(|number| IdKey { tracker: String::new(), number }),
        IdKind::Worknode => track_parts(name).and_then(|(t, n, _)| Some(IdKey { tracker: t.to_string(), number: n.parse().ok()? })),
    }
}

pub(super) fn groups(root: &Path, data: &Mapping, parent: &[String]) -> R<ReorderPlan> {
    let disk = list_subdirs(&join_rel(root, parent))?;
    let (mut nodes, mut worknodes, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for name in &disk {
        match kind_of(data, parent, name) {
            Some(IdKind::Node) => nodes.push(name.clone()),
            Some(IdKind::Worknode) => worknodes.push(name.clone()),
            None => others.push(name.as_str()),
        }
    }
    let ghosts: Vec<&str> = get_node(data, parent).map(|m| child_entries(m).map(|(k, _)| k).filter(|k| !disk.iter().any(|d| d == k)).collect()).unwrap_or_default();
    let mut taken: Vec<IdKey> = others.into_iter().chain(ghosts).filter_map(held_id).collect();
    taken.sort();
    taken.dedup();
    let by_number = |kind| move |a: &String, b: &String| key_of(a, kind).map(|k| k.number).cmp(&key_of(b, kind).map(|k| k.number)).then(a.cmp(b));
    nodes.sort_by(by_number(IdKind::Node));
    worknodes.sort_by(by_number(IdKind::Worknode));
    Ok(ReorderPlan { nodes, worknodes, taken, trackers: WKB_IDENTIFIERS.iter().map(|t| t.to_string()).collect() })
}

// Digits only, then padded: a node id to 2 digits, a worknode number to at least 3; a main category keeps its plain number.
pub(super) fn renamed(name: &str, kind: IdKind, at_root: bool, raw: &str, new_tracker: Option<&str>) -> R<(String, IdKey)> {
    let bad = |reason: &str| FmError::InvalidName { name: raw.to_string(), reason: reason.to_string() };
    if kind == IdKind::Node && new_tracker.is_some() {
        return Err(FmError::InvalidName { name: name.to_string(), reason: "only a worknode has a tracker".to_string() });
    }
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("an id takes digits only"));
    }
    let number: u32 = raw.parse().map_err(|_| bad("the id is too long"))?;
    match kind {
        IdKind::Node if at_root => {
            let text = number.to_string();
            if !is_main_category(&format!("{text}.")) {
                return Err(bad("a main category id is 100 to 800 in steps of 100"));
            }
            Ok((format!("{text}.{}", editable_part(name)), IdKey { tracker: String::new(), number }))
        }
        IdKind::Node if number == 0 => Err(bad("00 is reserved for Outcomes")),
        IdKind::Node if number > 99 => Err(bad("a node id goes up to 99")),
        IdKind::Node => Ok((format!("{number:02}.{}", editable_part(name)), IdKey { tracker: String::new(), number })),
        IdKind::Worknode if number == 0 => Err(bad("a worknode number starts at 1")),
        IdKind::Worknode => {
            let (current, _, rest) = track_parts(name).ok_or_else(|| bad("not a worknode name"))?;
            let tracker = match new_tracker {
                Some(t) => *WKB_IDENTIFIERS.iter().find(|known| **known == t).ok_or_else(|| FmError::InvalidName { name: t.to_string(), reason: "not a worknode tracker".to_string() })?,
                None => current,
            };
            Ok((format!("{tracker}{number:03}{rest}"), IdKey { tracker: tracker.to_string(), number }))
        }
    }
}
