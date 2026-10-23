// Turns the tree mapping into node rows and back; extra keys travel as YAML text.
use std::collections::HashMap;

use rusqlite::{params, OptionalExtension, Statement, Transaction};
use serde_yaml::{Mapping, Value};

use super::{db_err, Store};
use crate::ant_core::tree_types::META_KEYS;
use crate::error::R;

const ROOT_EXTRA: &str = "root_extra";

struct Row {
    id: i64,
    name: String,
    kind: Option<String>,
    status: Option<String>,
    extra: Option<String>,
}

impl Store {
    pub fn read(&self) -> R<Mapping> {
        self.read_tree().map_err(|e| db_err("read ant.db", &self.path, &e))
    }

    pub fn replace(&mut self, data: &Mapping) -> R<()> {
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let tx = self.conn.transaction().map_err(fail)?;
        write_tree(&tx, data).map_err(fail)?;
        tx.commit().map_err(fail)
    }

    fn read_tree(&self) -> rusqlite::Result<Mapping> {
        let mut by_parent: HashMap<i64, Vec<Row>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT id, parent_id, name, type, status, extra FROM nodes ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            let row = Row { id: r.get(0)?, name: r.get(2)?, kind: r.get(3)?, status: r.get(4)?, extra: r.get(5)? };
            Ok((r.get::<_, i64>(1)?, row))
        })?;
        for row in rows {
            let (parent, row) = row?;
            by_parent.entry(parent).or_default().push(row);
        }
        let root_extra: Option<String> =
            self.conn.query_row("SELECT value FROM meta WHERE key = ?1", [ROOT_EXTRA], |r| r.get(0)).optional()?;
        let mut root = Mapping::new();
        merge_extra(&mut root, root_extra.as_deref());
        attach_children(&mut root, 0, &by_parent);
        Ok(root)
    }
}

fn attach_children(node: &mut Mapping, id: i64, by_parent: &HashMap<i64, Vec<Row>>) {
    for row in by_parent.get(&id).into_iter().flatten() {
        let mut child = Mapping::new();
        merge_extra(&mut child, row.extra.as_deref());
        if let Some(t) = &row.kind {
            child.insert("type".into(), t.as_str().into());
        }
        if let Some(s) = &row.status {
            child.insert("status".into(), s.as_str().into());
        }
        attach_children(&mut child, row.id, by_parent);
        node.insert(Value::String(row.name.clone()), Value::Mapping(child));
    }
}

// The text was written by `write_tree`; a hand-edited db that no longer parses loses those extra entries.
fn merge_extra(node: &mut Mapping, extra: Option<&str>) {
    if let Some(Value::Mapping(entries)) = extra.and_then(|t| serde_yaml::from_str(t).ok()) {
        node.extend(entries);
    }
}

fn write_tree(tx: &Transaction, data: &Mapping) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM nodes", [])?;
    let root_extra: Mapping = data.iter().filter(|(k, v)| !is_child(k, v)).map(|(k, v)| (k.clone(), v.clone())).collect();
    if root_extra.is_empty() {
        tx.execute("DELETE FROM meta WHERE key = ?1", [ROOT_EXTRA])?;
    } else {
        tx.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![ROOT_EXTRA, to_text(&root_extra)])?;
    }
    let mut insert = tx.prepare(
        "INSERT INTO nodes (id, parent_id, name, depth, type, status, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    insert_level(&mut insert, data, 0, 0, &mut 0)
}

// Same rule as `child_entries`: a mapping under a non-meta string key is a child node.
fn is_child(key: &Value, value: &Value) -> bool {
    key.as_str().is_some_and(|k| !META_KEYS.contains(&k)) && value.is_mapping()
}

fn insert_level(insert: &mut Statement, node: &Mapping, parent: i64, depth: i64, last_id: &mut i64) -> rusqlite::Result<()> {
    for (key, value) in node {
        let (Some(name), Value::Mapping(child)) = (key.as_str(), value) else { continue };
        if !is_child(key, value) {
            continue;
        }
        *last_id += 1;
        let id = *last_id;
        let text = |field: &str| child.get(field).and_then(Value::as_str).map(str::to_string);
        let extra: Mapping = child
            .iter()
            .filter(|(k, v)| !is_child(k, v) && !(v.is_string() && k.as_str().is_some_and(|k| META_KEYS.contains(&k))))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let extra = (!extra.is_empty()).then(|| to_text(&extra));
        insert.execute(params![id, parent, name, depth, text("type"), text("status"), extra])?;
        insert_level(insert, child, id, depth + 1, last_id)?;
    }
    Ok(())
}

// A mapping of already-parsed values always serialises.
fn to_text(m: &Mapping) -> String {
    serde_yaml::to_string(&Value::Mapping(m.clone())).unwrap_or_default()
}
