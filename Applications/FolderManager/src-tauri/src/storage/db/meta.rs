// Side tables next to the tree: user settings and the folder-id footprints.
use std::collections::HashMap;

use rusqlite::params;

use super::{db_err, Store};
use crate::error::R;

impl Store {
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
}
