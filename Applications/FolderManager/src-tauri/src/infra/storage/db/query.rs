// The search index: per-node fields, the filtered search over them and typing suggestions.
use rusqlite::{params, OptionalExtension};

use super::{db_err, hash, Store};
use crate::error::R;
use crate::ant_core::search::Query;

const FIELDS_HASH: &str = "fields_hash";

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

impl Store {
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

    // Drops each prefix path with everything under it, then writes the docs, in one transaction.
    pub fn update_paths(&mut self, prefixes: &[String], docs: &[Doc]) -> R<()> {
        let path = self.path.clone();
        let fail = |e: rusqlite::Error| db_err("write ant.db", &path, &e);
        let tx = self.conn.transaction().map_err(fail)?;
        {
            // "p/" up to "p0" is exactly what lies under p, as '0' sorts right after '/', so the primary key serves it.
            let mut drop = tx
                .prepare("DELETE FROM node_fields WHERE path = ?1 OR (path >= ?1 || '/' AND path < ?1 || '0')")
                .map_err(fail)?;
            for prefix in prefixes {
                drop.execute([prefix]).map_err(fail)?;
            }
            let mut insert = tx.prepare("INSERT OR REPLACE INTO node_fields (path, key, value) VALUES (?1, ?2, ?3)").map_err(fail)?;
            for doc in docs {
                for (key, value) in &doc.fields {
                    insert.execute(params![doc.path, key, value]).map_err(fail)?;
                }
            }
        }
        // The stored digest no longer describes the rows, so the next full rebuild must not skip its write.
        tx.execute("DELETE FROM meta WHERE key = ?1", [FIELDS_HASH]).map_err(fail)?;
        tx.commit().map_err(fail)
    }

    #[cfg(test)]
    pub fn field_rows(&self) -> R<Vec<(String, String, String)>> {
        let fail = |e: rusqlite::Error| db_err("search ant.db", &self.path, &e);
        let mut stmt = self.conn.prepare("SELECT path, key, value FROM node_fields ORDER BY path, key").map_err(fail)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(fail)?;
        rows.map(|r| r.map_err(fail)).collect()
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
}

fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
