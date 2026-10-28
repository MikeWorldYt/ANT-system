// Turns the tree mapping into node rows and back; a save writes only the rows that changed, so ids stay put.
use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_yaml::{Mapping, Value};

use super::{db_err, Store};
use crate::ant_core::tree_types::META_KEYS;
use crate::error::R;

const ROOT_EXTRA: &str = "root_extra";

// The columns a node keeps besides its id, parent and name.
#[derive(Clone, PartialEq)]
struct Row {
    kind: Option<String>,
    status: Option<String>,
    extra: Option<String>,
    uid: Option<String>,
    flags: Option<String>,
}

struct Node {
    id: i64,
    name: String,
    row: Row,
}

struct Stored {
    id: i64,
    row: Row,
}

struct Want {
    id: i64,
    parent_id: i64,
    depth: i64,
    path: String,
    name: String,
    row: Row,
}

impl Store {
    pub fn read(&self) -> R<Mapping> {
        self.read_tree().map_err(|e| db_err("read ant.db", &self.path, &e))
    }

    // Writes only the rows that differ from what the db holds.
    pub fn replace(&mut self, data: &Mapping) -> R<()> {
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let tx = self.conn.transaction().map_err(fail)?;
        write_tree(&tx, data).map_err(fail)?;
        tx.commit().map_err(fail)
    }

    fn read_tree(&self) -> rusqlite::Result<Mapping> {
        let mut by_parent: HashMap<i64, Vec<Node>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT id, parent_id, name, type, status, extra, uid, flags FROM nodes ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            let node = Node { id: r.get(0)?, name: r.get(2)?, row: row_at(r, 3)? };
            Ok((r.get::<_, i64>(1)?, node))
        })?;
        for row in rows {
            let (parent, node) = row?;
            by_parent.entry(parent).or_default().push(node);
        }
        let root_extra: Option<String> =
            self.conn.query_row("SELECT value FROM meta WHERE key = ?1", [ROOT_EXTRA], |r| r.get(0)).optional()?;
        let mut root = Mapping::new();
        merge_extra(&mut root, root_extra.as_deref());
        attach_children(&mut root, 0, &by_parent);
        Ok(root)
    }
}

// Reads the node columns that start at `first`, in the order the select lists them.
fn row_at(r: &rusqlite::Row, first: usize) -> rusqlite::Result<Row> {
    Ok(Row { kind: r.get(first)?, status: r.get(first + 1)?, extra: r.get(first + 2)?, uid: r.get(first + 3)?, flags: r.get(first + 4)? })
}

fn attach_children(node: &mut Mapping, id: i64, by_parent: &HashMap<i64, Vec<Node>>) {
    for child_node in by_parent.get(&id).into_iter().flatten() {
        let mut child = Mapping::new();
        merge_extra(&mut child, child_node.row.extra.as_deref());
        let fields = [("type", &child_node.row.kind), ("status", &child_node.row.status), ("uid", &child_node.row.uid), ("flags", &child_node.row.flags)];
        for (key, value) in fields {
            if let Some(text) = value {
                child.insert(key.into(), text.as_str().into());
            }
        }
        attach_children(&mut child, child_node.id, by_parent);
        node.insert(Value::String(child_node.name.clone()), Value::Mapping(child));
    }
}

// The text was written by `write_tree`; a hand-edited db that no longer parses loses those extra entries.
fn merge_extra(node: &mut Mapping, extra: Option<&str>) {
    if let Some(Value::Mapping(entries)) = extra.and_then(|t| serde_yaml::from_str(t).ok()) {
        node.extend(entries);
    }
}

fn write_tree(tx: &Transaction, data: &Mapping) -> rusqlite::Result<()> {
    write_root_extra(tx, data)?;
    let (stored, max_id) = stored_by_path(tx)?;
    let mut wanted = Vec::new();
    let mut next_id = max_id;
    collect(data, 0, "", 0, &stored, &mut next_id, &mut wanted);

    let keep: HashSet<&str> = wanted.iter().map(|w| w.path.as_str()).collect();
    let mut delete = tx.prepare("DELETE FROM nodes WHERE id = ?1")?;
    for (path, old) in &stored {
        if !keep.contains(path.as_str()) {
            delete.execute([old.id])?;
        }
    }
    let mut insert = tx.prepare("INSERT INTO nodes (id, parent_id, name, depth, type, status, extra, uid, flags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?;
    let mut update = tx.prepare("UPDATE nodes SET type = ?1, status = ?2, extra = ?3, uid = ?4, flags = ?5 WHERE id = ?6")?;
    for w in &wanted {
        match stored.get(&w.path) {
            Some(old) if old.row == w.row => {}
            Some(old) => {
                update.execute(params![w.row.kind, w.row.status, w.row.extra, w.row.uid, w.row.flags, old.id])?;
            }
            None => {
                insert.execute(params![w.id, w.parent_id, w.name, w.depth, w.row.kind, w.row.status, w.row.extra, w.row.uid, w.row.flags])?;
            }
        }
    }
    Ok(())
}

fn write_root_extra(tx: &Transaction, data: &Mapping) -> rusqlite::Result<()> {
    let root_extra: Mapping = data.iter().filter(|(k, v)| !is_child(k, v)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let desired = (!root_extra.is_empty()).then(|| to_text(&root_extra));
    let current: Option<String> = tx.query_row("SELECT value FROM meta WHERE key = ?1", [ROOT_EXTRA], |r| r.get(0)).optional()?;
    if current == desired {
        return Ok(());
    }
    match desired {
        None => {
            tx.execute("DELETE FROM meta WHERE key = ?1", [ROOT_EXTRA])?;
        }
        Some(text) => {
            tx.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![ROOT_EXTRA, text])?;
        }
    }
    Ok(())
}

// Every stored node keyed by its path, plus the highest id, so new rows never reuse one.
fn stored_by_path(conn: &Connection) -> rusqlite::Result<(HashMap<String, Stored>, i64)> {
    let mut stmt = conn.prepare("SELECT id, parent_id, name, type, status, extra, uid, flags FROM nodes")?;
    let rows: Vec<(i64, i64, String, Row)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, row_at(r, 3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let names: HashMap<i64, (i64, String)> = rows.iter().map(|(id, parent, name, _)| (*id, (*parent, name.clone()))).collect();
    let max_id = rows.iter().map(|(id, ..)| *id).max().unwrap_or(0);
    let stored = rows
        .into_iter()
        .map(|(id, _, _, row)| (path_of(id, &names), Stored { id, row }))
        .collect();
    Ok((stored, max_id))
}

// Walks up the parent links; bounded by the row count so a corrupt loop cannot hang the app.
fn path_of(id: i64, names: &HashMap<i64, (i64, String)>) -> String {
    let mut parts = Vec::new();
    let mut current = id;
    for _ in 0..names.len() {
        let Some((parent, name)) = names.get(&current) else { break };
        parts.push(name.as_str());
        current = *parent;
    }
    parts.reverse();
    parts.join("/")
}

fn collect(node: &Mapping, parent_id: i64, parent: &str, depth: i64, stored: &HashMap<String, Stored>, next_id: &mut i64, out: &mut Vec<Want>) {
    for (key, value) in node {
        let (Some(name), Value::Mapping(child)) = (key.as_str(), value) else { continue };
        if !is_child(key, value) {
            continue;
        }
        let path = if parent.is_empty() { name.to_string() } else { format!("{parent}/{name}") };
        let id = match stored.get(&path) {
            Some(old) => old.id,
            None => {
                *next_id += 1;
                *next_id
            }
        };
        let text = |field: &str| child.get(field).and_then(Value::as_str).map(str::to_string);
        let extra: Mapping = child
            .iter()
            .filter(|(k, v)| !is_child(k, v) && !(v.is_string() && k.as_str().is_some_and(|k| META_KEYS.contains(&k))))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let row = Row {
            kind: text("type"),
            status: text("status"),
            extra: (!extra.is_empty()).then(|| to_text(&extra)),
            uid: text("uid"),
            flags: text("flags"),
        };
        out.push(Want { id, parent_id, depth, path: path.clone(), name: name.to_string(), row });
        collect(child, id, &path, depth + 1, stored, next_id, out);
    }
}

// Same rule as `child_entries`: a mapping under a non-meta string key is a child node.
fn is_child(key: &Value, value: &Value) -> bool {
    key.as_str().is_some_and(|k| !META_KEYS.contains(&k)) && value.is_mapping()
}

// A mapping of already-parsed values always serialises.
fn to_text(m: &Mapping) -> String {
    serde_yaml::to_string(&Value::Mapping(m.clone())).unwrap_or_default()
}

// Gives the stored nodes at these paths their uid, used when a db predates the uid column.
pub(super) fn adopt_uids(conn: &Connection, footprints: &[(String, String)]) -> rusqlite::Result<()> {
    let (stored, _) = stored_by_path(conn)?;
    for (uid, path) in footprints {
        if let Some(node) = stored.get(path) {
            conn.execute("UPDATE nodes SET uid = ?1 WHERE id = ?2", params![uid, node.id])?;
        }
    }
    Ok(())
}
