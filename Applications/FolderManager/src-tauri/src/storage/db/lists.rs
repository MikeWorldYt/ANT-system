// Storage-wide lists behind the virtual root entries, read from the search index; 998.* is left out.
use super::{db_err, Store};
use crate::error::R;

impl Store {
    pub fn favorites(&self) -> R<Vec<Vec<String>>> {
        self.paths_where("favorite", "true")
    }

    // LIKE is case-insensitive for ASCII, so "true" and "True" both count.
    fn paths_where(&self, key: &str, like: &str) -> R<Vec<Vec<String>>> {
        let fail = |e: rusqlite::Error| db_err("search ant.db", &self.path, &e);
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM node_fields WHERE key = ?1 AND value LIKE ?2 AND path NOT LIKE '998.%' ORDER BY path")
            .map_err(fail)?;
        let rows = stmt.query_map([key, like], |r| r.get::<_, String>(0)).map_err(fail)?;
        rows.map(|r| r.map(|p| p.split('/').map(str::to_string).collect()).map_err(fail)).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_yaml::Mapping;

    use super::*;
    use crate::index::search_index;
    use crate::test_support::TempDir;

    #[test]
    fn favorites_cover_the_storage_but_not_the_review_folder() {
        let dir = TempDir::new("lists-favorites");
        for folder in ["100.A/01.Deep/.ant", "200.B/.ant", "998.Review/100.A/.ant"] {
            dir.mkdir(folder);
            fs::write(dir.path().join(folder).join("info.json"), r#"{"favorite": true}"#).unwrap();
        }
        dir.mkdir("200.B/02.Plain");
        let data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  01.Deep: {type: NODE-L2}\n200.B:\n  type: NODE-L1\n  02.Plain: {type: NODE-L2}\n").unwrap();
        search_index::rebuild(dir.path(), &data).unwrap();
        let store = Store::open_or_create(dir.path()).unwrap();
        let found: Vec<String> = store.favorites().unwrap().iter().map(|p| p.join("/")).collect();
        assert_eq!(found, ["100.A/01.Deep", "200.B"]);
    }
}
