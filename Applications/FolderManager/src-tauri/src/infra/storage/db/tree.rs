// Turns the tree mapping into node rows and back; a save writes only the rows that changed, so ids stay put.
use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_yaml::{Mapping, Value};

use super::{db_err, Store};
use crate::domain::tree_types::META_KEYS;
use crate::domain::error::R;

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

    // Writes only the subtrees at `rels` and drops their stored rows that `data` no longer holds; no paths, or the root, mean the whole tree.
    pub fn replace_paths(&mut self, data: &Mapping, rels: &[Vec<String>]) -> R<()> {
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let tx = self.conn.transaction().map_err(fail)?;
        if rels.is_empty() || rels.iter().any(Vec::is_empty) {
            write_tree(&tx, data).map_err(fail)?;
        } else {
            write_paths(&tx, data, rels).map_err(fail)?;
        }
        tx.commit().map_err(fail)
    }

    // The path down to `rel` plus everything under it, so a level opens without reading the other branches.
    pub fn read_level(&self, rel: &[String]) -> R<Mapping> {
        self.level_tree(rel).map_err(|e| db_err("read ant.db", &self.path, &e))
    }

    // Each path with its ancestors and no children; the entries are whole, so a lookup reads the same fields as in the full tree.
    pub fn read_paths(&self, rels: &[Vec<String>]) -> R<Mapping> {
        self.paths_tree(rels).map_err(|e| db_err("read ant.db", &self.path, &e))
    }

    fn level_tree(&self, rel: &[String]) -> rusqlite::Result<Mapping> {
        let chain = self.chain(rel)?;
        let last = chain.last().map_or(0, |n| n.id);
        let mut inner = Mapping::new();
        if chain.len() == rel.len() {
            attach_children(&mut inner, last, &self.subtree(last)?);
        }
        for node in chain.iter().rev() {
            let mut entry = entry_of(&node.row);
            entry.extend(inner);
            inner = Mapping::new();
            inner.insert(Value::String(node.name.clone()), Value::Mapping(entry));
        }
        let mut root = self.root_mapping()?;
        root.extend(inner);
        Ok(root)
    }

    fn paths_tree(&self, rels: &[Vec<String>]) -> rusqlite::Result<Mapping> {
        let mut root = self.root_mapping()?;
        for rel in rels {
            let mut cursor = &mut root;
            for node in self.chain(rel)? {
                if !cursor.contains_key(node.name.as_str()) {
                    cursor.insert(Value::String(node.name.clone()), Value::Mapping(entry_of(&node.row)));
                }
                let Some(Value::Mapping(next)) = cursor.get_mut(node.name.as_str()) else { break };
                cursor = next;
            }
        }
        Ok(root)
    }

    fn root_mapping(&self) -> rusqlite::Result<Mapping> {
        let root_extra: Option<String> =
            self.conn.query_row("SELECT value FROM meta WHERE key = ?1", [ROOT_EXTRA], |r| r.get(0)).optional()?;
        let mut root = Mapping::new();
        merge_extra(&mut root, root_extra.as_deref());
        Ok(root)
    }

    // The stored nodes along `rel`, stopping at the first name the db does not hold.
    fn chain(&self, rel: &[String]) -> rusqlite::Result<Vec<Node>> {
        let mut stmt = self.conn.prepare_cached("SELECT id, name, type, status, extra, uid, flags FROM nodes WHERE parent_id = ?1 AND name = ?2")?;
        let mut chain = Vec::with_capacity(rel.len());
        let mut parent = 0;
        for part in rel {
            let node = stmt
                .query_row(params![parent, part], |r| Ok(Node { id: r.get(0)?, name: r.get(1)?, row: row_at(r, 2)? }))
                .optional()?;
            let Some(node) = node else { break };
            parent = node.id;
            chain.push(node);
        }
        Ok(chain)
    }

    // UNION rather than UNION ALL, so a corrupt parent loop ends instead of recursing forever.
    fn subtree(&self, id: i64) -> rusqlite::Result<HashMap<i64, Vec<Node>>> {
        let mut by_parent: HashMap<i64, Vec<Node>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE sub(id) AS (SELECT id FROM nodes WHERE parent_id = ?1 UNION SELECT n.id FROM nodes n JOIN sub ON n.parent_id = sub.id)              SELECT n.id, n.parent_id, n.name, n.type, n.status, n.extra, n.uid, n.flags FROM nodes n JOIN sub ON n.id = sub.id ORDER BY n.id",
        )?;
        let rows = stmt.query_map([id], |r| {
            let node = Node { id: r.get(0)?, name: r.get(2)?, row: row_at(r, 3)? };
            Ok((r.get::<_, i64>(1)?, node))
        })?;
        for row in rows {
            let (parent, node) = row?;
            by_parent.entry(parent).or_default().push(node);
        }
        Ok(by_parent)
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
        let mut root = self.root_mapping()?;
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
        let mut child = entry_of(&child_node.row);
        attach_children(&mut child, child_node.id, by_parent);
        node.insert(Value::String(child_node.name.clone()), Value::Mapping(child));
    }
}

// A node's own entries, without its children.
fn entry_of(row: &Row) -> Mapping {
    let mut entry = Mapping::new();
    merge_extra(&mut entry, row.extra.as_deref());
    let fields = [("type", &row.kind), ("status", &row.status), ("uid", &row.uid), ("flags", &row.flags)];
    for (key, value) in fields {
        if let Some(text) = value {
            entry.insert(key.into(), text.as_str().into());
        }
    }
    entry
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
    let mut update = tx.prepare("UPDATE nodes SET type = ?1, status = ?2, extra = ?3, uid = ?4, flags = ?5 WHERE id = ?6")?;
    for w in &wanted {
        match stored.get(&w.path) {
            Some(old) if old.row == w.row => {}
            Some(old) => {
                update.execute(params![w.row.kind, w.row.status, w.row.extra, w.row.uid, w.row.flags, old.id])?;
            }
            None => {
                insert_row(tx, w)?;
            }
        }
    }
    Ok(())
}

fn write_paths(tx: &Transaction, data: &Mapping, rels: &[Vec<String>]) -> rusqlite::Result<()> {
    let mut next_id: i64 = tx.query_row("SELECT COALESCE(MAX(id), 0) FROM nodes", [], |r| r.get(0))?;
    for rel in rels {
        write_path(tx, data, rel, &mut next_id)?;
    }
    Ok(())
}

// Missing ancestors are created from `data`, but existing ones are left alone, as only the subtree is this write's concern.
fn write_path(tx: &Transaction, data: &Mapping, rel: &[String], next_id: &mut i64) -> rusqlite::Result<()> {
    let Some((name, parents)) = rel.split_last() else { return Ok(()) };
    let mut parent_id = 0;
    let mut cursor = Some(data);
    for (depth, part) in parents.iter().enumerate() {
        cursor = cursor.and_then(|node| child_of(node, part));
        parent_id = match stored_child(tx, parent_id, part)? {
            Some(node) => node.id,
            None => match cursor {
                Some(entry) => {
                    *next_id += 1;
                    insert_row(tx, &Want { id: *next_id, parent_id, depth: depth as i64, path: String::new(), name: part.clone(), row: row_of(entry) })?;
                    *next_id
                }
                None => return Ok(()),
            },
        };
    }
    let wanted = cursor.and_then(|node| child_of(node, name));
    let stored = stored_child(tx, parent_id, name)?;
    match (wanted, stored) {
        (Some(entry), stored) => write_node(tx, parent_id, parents.len() as i64, name, entry, stored, next_id),
        (None, Some(old)) => delete_subtree(tx, old.id),
        (None, None) => Ok(()),
    }
}

fn write_node(tx: &Transaction, parent_id: i64, depth: i64, name: &str, entry: &Mapping, stored: Option<Node>, next_id: &mut i64) -> rusqlite::Result<()> {
    let row = row_of(entry);
    let id = match &stored {
        Some(old) if old.row == row => old.id,
        Some(old) => {
            tx.execute(
                "UPDATE nodes SET type = ?1, status = ?2, extra = ?3, uid = ?4, flags = ?5 WHERE id = ?6",
                params![row.kind, row.status, row.extra, row.uid, row.flags, old.id],
            )?;
            old.id
        }
        None => {
            *next_id += 1;
            insert_row(tx, &Want { id: *next_id, parent_id, depth, path: String::new(), name: name.to_string(), row })?;
            *next_id
        }
    };
    let mut old_children: HashMap<String, Node> = if stored.is_some() { stored_children(tx, id)? } else { HashMap::new() };
    for (key, value) in entry {
        let (Some(child_name), Value::Mapping(child)) = (key.as_str(), value) else { continue };
        if is_child(key, value) {
            write_node(tx, id, depth + 1, child_name, child, old_children.remove(child_name), next_id)?;
        }
    }
    for gone in old_children.into_values() {
        delete_subtree(tx, gone.id)?;
    }
    Ok(())
}

fn child_of<'a>(node: &'a Mapping, name: &str) -> Option<&'a Mapping> {
    match node.get(name) {
        Some(Value::Mapping(child)) if !META_KEYS.contains(&name) => Some(child),
        _ => None,
    }
}

fn insert_row(tx: &Transaction, w: &Want) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO nodes (id, parent_id, name, depth, type, status, extra, uid, flags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![w.id, w.parent_id, w.name, w.depth, w.row.kind, w.row.status, w.row.extra, w.row.uid, w.row.flags],
    )?;
    Ok(())
}

fn stored_child(tx: &Transaction, parent_id: i64, name: &str) -> rusqlite::Result<Option<Node>> {
    tx.prepare_cached("SELECT id, name, type, status, extra, uid, flags FROM nodes WHERE parent_id = ?1 AND name = ?2")?
        .query_row(params![parent_id, name], |r| Ok(Node { id: r.get(0)?, name: r.get(1)?, row: row_at(r, 2)? }))
        .optional()
}

fn stored_children(tx: &Transaction, parent_id: i64) -> rusqlite::Result<HashMap<String, Node>> {
    let mut stmt = tx.prepare_cached("SELECT id, name, type, status, extra, uid, flags FROM nodes WHERE parent_id = ?1")?;
    let rows = stmt.query_map([parent_id], |r| Ok(Node { id: r.get(0)?, name: r.get(1)?, row: row_at(r, 2)? }))?;
    rows.map(|n| n.map(|n| (n.name.clone(), n))).collect()
}

// UNION rather than UNION ALL, so a corrupt parent loop ends instead of recursing forever.
fn delete_subtree(tx: &Transaction, id: i64) -> rusqlite::Result<()> {
    tx.execute(
        "WITH RECURSIVE sub(id) AS (SELECT ?1 UNION SELECT n.id FROM nodes n JOIN sub ON n.parent_id = sub.id) DELETE FROM nodes WHERE id IN (SELECT id FROM sub)",
        [id],
    )?;
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
        out.push(Want { id, parent_id, depth, path: path.clone(), name: name.to_string(), row: row_of(child) });
        collect(child, id, &path, depth + 1, stored, next_id, out);
    }
}

fn row_of(child: &Mapping) -> Row {
    let text = |field: &str| child.get(field).and_then(Value::as_str).map(str::to_string);
    let extra: Mapping = child
        .iter()
        .filter(|(k, v)| !is_child(k, v) && !(v.is_string() && k.as_str().is_some_and(|k| META_KEYS.contains(&k))))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Row { kind: text("type"), status: text("status"), extra: (!extra.is_empty()).then(|| to_text(&extra)), uid: text("uid"), flags: text("flags") }
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
