// SQLite store of the tree and its search fields under <root>/.ant; the only copy of the tree.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Statement, Transaction};
use serde_yaml::{Mapping, Value};

use crate::ant_core::tree_types::META_KEYS;
use crate::config::{DATA_DIR, DB_FILENAME};
use crate::error::{FmError, R};
use crate::fs_safe::io_err;
use crate::search::Query;

const SCHEMA_VERSION: i64 = 1;
const ROOT_EXTRA: &str = "root_extra";
const FIELDS_HASH: &str = "fields_hash";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    depth INTEGER NOT NULL,
    type TEXT,
    status TEXT,
    extra TEXT,
    UNIQUE (parent_id, name)
);
CREATE INDEX IF NOT EXISTS nodes_name ON nodes (name COLLATE NOCASE);
CREATE TABLE IF NOT EXISTS node_fields (
    path TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (path, key)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS footprints (id TEXT PRIMARY KEY, path TEXT NOT NULL) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS node_fields_key_value ON node_fields (key, value COLLATE NOCASE);
";

pub fn db_path(root: &Path) -> PathBuf {
    root.join(DATA_DIR).join(DB_FILENAME)
}

// FNV-1a: std's hashers are not stable across Rust versions and the value is stored on disk.
pub fn hash(text: &str) -> String {
    let h = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
    format!("{h:016x}")
}

pub struct Store {
    conn: Connection,
    path: PathBuf,
}

#[derive(Debug, serde::Serialize)]
pub struct Suggestion {
    pub value: String,
    pub label: String,
    pub hint: String,
}

// One node as the search sees it: its path and every field a query can filter on.
pub struct Doc {
    pub path: String,
    pub fields: Vec<(String, String)>,
}

struct Row {
    id: i64,
    name: String,
    kind: Option<String>,
    status: Option<String>,
    extra: Option<String>,
}

impl Store {
    pub fn open_existing(root: &Path) -> R<Option<Store>> {
        let path = db_path(root);
        if !path.is_file() {
            return Ok(None);
        }
        Self::open(path).map(Some)
    }

    pub fn open_or_create(root: &Path) -> R<Store> {
        let dir = root.join(DATA_DIR);
        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
            hide(&dir);
        }
        Self::open(db_path(root))
    }

    fn open(path: PathBuf) -> R<Store> {
        let fail = |e: rusqlite::Error| db_err("open ant.db", &path, &e);
        let conn = Connection::open(&path).map_err(fail)?;
        conn.busy_timeout(Duration::from_secs(5)).map_err(fail)?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).map_err(fail)?;
        if version > SCHEMA_VERSION {
            return Err(FmError::Internal { message: "ant.db was written by a newer version of the app".to_string() });
        }
        if version < SCHEMA_VERSION {
            conn.execute_batch(SCHEMA).map_err(fail)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION).map_err(fail)?;
        }
        Ok(Store { conn, path })
    }

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

    pub fn footprints(&self) -> R<HashMap<String, String>> {
        let fail = |e: rusqlite::Error| db_err("read ant.db", &self.path, &e);
        let mut stmt = self.conn.prepare("SELECT id, path FROM footprints").map_err(fail)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(fail)?;
        rows.map(|r| r.map_err(fail)).collect()
    }

    // Settings share the meta table under a prefix, so they need no schema change.
    pub fn settings(&self) -> R<HashMap<String, String>> {
        let fail = |e: rusqlite::Error| db_err("read ant.db", &self.path, &e);
        let mut stmt = self.conn.prepare("SELECT substr(key, 9), value FROM meta WHERE key LIKE 'setting.%'").map_err(fail)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(fail)?;
        rows.map(|r| r.map_err(fail)).collect()
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> R<()> {
        self.conn
            .execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![format!("setting.{key}"), value])
            .map(|_| ())
            .map_err(|e| db_err("write ant.db", &self.path, &e))
    }

    pub fn replace_footprints(&mut self, footprints: &HashMap<String, String>) -> R<()> {
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let tx = self.conn.transaction().map_err(fail)?;
        tx.execute("DELETE FROM footprints", []).map_err(fail)?;
        for (id, folder) in footprints {
            tx.execute("INSERT INTO footprints (id, path) VALUES (?1, ?2)", params![id, folder]).map_err(fail)?;
        }
        tx.commit().map_err(fail)
    }

    // Skips the write when nothing changed, so a window-focus refresh costs one read of the info files.
    pub fn replace_fields(&mut self, docs: &[Doc]) -> R<()> {
        let digest = hash(&docs.iter().flat_map(|d| d.fields.iter().map(move |(k, v)| format!("{}\u{1f}{k}\u{1f}{v}\n", d.path))).collect::<String>());
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let current: Option<String> =
            self.conn.query_row("SELECT value FROM meta WHERE key = ?1", [FIELDS_HASH], |r| r.get(0)).optional().map_err(fail)?;
        if current.as_deref() == Some(digest.as_str()) {
            return Ok(());
        }
        let tx = self.conn.transaction().map_err(fail)?;
        tx.execute("DELETE FROM node_fields", []).map_err(fail)?;
        {
            let mut insert = tx.prepare("INSERT OR REPLACE INTO node_fields (path, key, value) VALUES (?1, ?2, ?3)").map_err(fail)?;
            for doc in docs {
                for (key, value) in &doc.fields {
                    insert.execute(params![doc.path, key, value]).map_err(fail)?;
                }
            }
        }
        tx.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![FIELDS_HASH, digest]).map_err(fail)?;
        tx.commit().map_err(fail)
    }

    // Filters are substring matches, case-insensitive for ASCII; the scope is a path prefix.
    pub fn search(&self, query: &Query) -> R<Vec<Vec<String>>> {
        let mut sql = String::from("SELECT path FROM node_fields WHERE key = 'name'");
        let mut args: Vec<String> = Vec::new();
        if let Some(scope) = &query.scope {
            // A leading slash marks a scope picked from the suggestions: that exact path and what is under it.
            let exact = scope.starts_with('/');
            let scope = like_escape(scope.trim_matches('/'));
            if exact || scope.contains('/') {
                sql.push_str(" AND (path LIKE ? ESCAPE '\\' OR path LIKE ? ESCAPE '\\')");
                args.push(scope.clone());
                args.push(format!("{scope}/%"));
            } else {
                // A bare scope starts a folder name, with or without its number: "200.WORK", "WORK", "wo" or "acade".
                sql.push_str(
                    " AND EXISTS (SELECT 1 FROM node_fields s WHERE s.key = 'name' AND (s.value LIKE ? ESCAPE '\\' OR s.value LIKE ? ESCAPE '\\')\
                     AND (node_fields.path = s.path OR substr(node_fields.path, 1, length(s.path) + 1) = s.path || '/'))",
                );
                args.push(format!("{scope}%"));
                args.push(format!("%.{scope}%"));
            }
        }
        for (key, value) in &query.filters {
            sql.push_str(" AND EXISTS (SELECT 1 FROM node_fields f WHERE f.path = node_fields.path AND f.key = ? AND f.value LIKE ? ESCAPE '\\')");
            args.push(key.clone());
            // "=value" is a picked value and must match whole; plain text matches anywhere inside.
            args.push(match value.strip_prefix('=') {
                Some(whole) => like_escape(whole),
                None => format!("%{}%", like_escape(value)),
            });
        }
        for word in &query.text {
            sql.push_str(" AND value LIKE ? ESCAPE '\\'");
            args.push(format!("%{}%", like_escape(word)));
        }
        sql.push_str(" ORDER BY NOT EXISTS (SELECT 1 FROM node_fields f WHERE f.path = node_fields.path AND f.key = 'favorite' AND lower(f.value) = 'true'), path");
        let fail = |e: rusqlite::Error| db_err("search ant.db", &self.path, &e);
        let mut stmt = self.conn.prepare(&sql).map_err(fail)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(args.iter()), |r| r.get::<_, String>(0)).map_err(fail)?;
        rows.map(|r| r.map(|p| p.split('/').map(str::to_string).collect()).map_err(fail)).collect()
    }

    // Completions for `key:` while typing; `in` completes folders, any other key completes its known values.
    pub fn suggest(&self, key: &str, prefix: &str) -> R<Vec<Suggestion>> {
        let fail = |e: rusqlite::Error| db_err("search ant.db", &self.path, &e);
        let like = like_escape(prefix);
        if key == "in" {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT path, value FROM node_fields WHERE key = 'name' \
                     AND (value LIKE ?1 || '%' ESCAPE '\\' OR value LIKE '%.' || ?1 || '%' ESCAPE '\\') ORDER BY path LIMIT 8",
                )
                .map_err(fail)?;
            let rows = stmt.query_map([&like], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(fail)?;
            return rows
                .map(|r| {
                    let (path, name) = r.map_err(fail)?;
                    let hint = path.rsplit_once('/').map(|(parent, _)| parent.replace('/', " › ")).unwrap_or_default();
                    Ok(Suggestion { value: path, label: name, hint })
                })
                .collect();
        }
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT value FROM node_fields WHERE key = ?1 AND value LIKE ?2 || '%' ESCAPE '\\' ORDER BY value LIMIT 8")
            .map_err(fail)?;
        let rows = stmt.query_map([key, like.as_str()], |r| r.get::<_, String>(0)).map_err(fail)?;
        rows.map(|r| r.map(|v| Suggestion { value: v.clone(), label: v, hint: String::new() }).map_err(fail)).collect()
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

fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

// A mapping of already-parsed values always serialises.
fn to_text(m: &Mapping) -> String {
    serde_yaml::to_string(&Value::Mapping(m.clone())).unwrap_or_default()
}

fn db_err(op: &str, path: &Path, e: &rusqlite::Error) -> FmError {
    FmError::Io { op: op.to_string(), path: path.display().to_string(), message: e.to_string(), os_code: None }
}

// A failed attrib only leaves the folder visible, so it is reported on stderr and the save goes on.
pub(crate) fn hide(dir: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let hidden = std::process::Command::new("attrib").arg("+h").arg(dir).creation_flags(CREATE_NO_WINDOW).status();
    match hidden {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("Could not hide {}: attrib exited with {status}", dir.display()),
        Err(e) => eprintln!("Could not hide {}: {e}", dir.display()),
    }
}
