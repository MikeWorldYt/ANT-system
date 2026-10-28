// Side table next to the tree: user settings kept in the meta table.
use std::collections::HashMap;
use std::path::Path;

use rusqlite::params;

use super::{db_err, Store};
use crate::error::R;

impl Store {
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

    // VACUUM INTO writes a consistent copy even while the app keeps writing; the path is a quoted SQL literal.
    pub fn vacuum_into(&self, dest: &Path) -> R<()> {
        let target = dest.to_string_lossy().replace('\'', "''");
        self.conn.execute_batch(&format!("VACUUM INTO '{target}'")).map_err(|e| db_err("back up ant.db", &self.path, &e))
    }

    pub fn reset_settings(&mut self) -> R<()> {
        self.conn
            .execute("DELETE FROM meta WHERE key LIKE 'setting.%'", [])
            .map(|_| ())
            .map_err(|e| db_err("write ant.db", &self.path, &e))
    }

}
