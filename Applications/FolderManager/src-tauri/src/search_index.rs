// Flattens every node and its info.json fields into the db so searches never touch the folders.
use std::path::Path;

use serde_yaml::Mapping;

use crate::config::MAX_DEPTH;
use crate::db::{Doc, Store};
use crate::error::R;
use crate::search::canonical_key;
use crate::tree_node::build_children;

pub fn rebuild(root: &Path, data: &Mapping) -> R<()> {
    let mut docs = Vec::new();
    collect(root, data, &[], &mut docs)?;
    Store::open_or_create(root)?.replace_fields(&docs)
}

fn collect(root: &Path, data: &Mapping, parent: &[String], docs: &mut Vec<Doc>) -> R<()> {
    for node in build_children(root, data, parent)? {
        if node.reserved {
            continue;
        }
        let mut fields = vec![("name".to_string(), node.name.clone())];
        if let Some(t) = &node.node_type {
            fields.push(("type".to_string(), t.raw.clone()));
        }
        if let Some(status) = &node.status_raw {
            fields.push(("status".to_string(), status.clone()));
        }
        // info.json comes last so its keys win over the yaml ones of the same name.
        for field in node.info.iter().flat_map(|i| &i.fields) {
            fields.push((canonical_key(&field.key), field.value.clone()));
        }
        docs.push(Doc { path: node.rel_parts.join("/"), fields });
        if node.has_children && node.depth + 1 < MAX_DEPTH {
            collect(root, data, &node.rel_parts, docs)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::search::parse_query;
    use crate::tree_store::test_support::TempDir;

    fn storage() -> TempDir {
        let dir = TempDir::new("search-index");
        dir.mkdir("200.WORK/PJ.001_Fabi-web/.ant");
        dir.mkdir("200.WORK/PJ.002_Other");
        dir.mkdir("300.HOME/PJ.003_Mine/.ant");
        fs::write(dir.path().join("200.WORK/PJ.001_Fabi-web/.ant/info.json"), r#"{"owner": "Mike", "status": "hold"}"#).unwrap();
        fs::write(dir.path().join("300.HOME/PJ.003_Mine/.ant/info.json"), r#"{"holder": "Ana"}"#).unwrap();
        dir
    }

    const YAML: &str = "200.WORK:\n  type: NODE-L1\n  PJ.001_Fabi-web: {type: WKB-L2, status: in-progress}\n  PJ.002_Other: {type: WKB-L2, status: in-progress}\n\
        300.HOME:\n  type: NODE-L1\n  PJ.003_Mine: {type: WKB-L2, status: done}\n";

    fn run(dir: &TempDir, query: &str) -> Vec<String> {
        let store = Store::open_or_create(dir.path()).unwrap();
        store.search(&parse_query(query)).unwrap().iter().map(|p| p.join("/")).collect()
    }

    #[test]
    fn filters_by_scope_holder_and_status() {
        let dir = storage();
        let data: Mapping = serde_yaml::from_str(YAML).unwrap();
        rebuild(dir.path(), &data).unwrap();
        assert_eq!(run(&dir, "in:200.WORK client:Mike"), ["200.WORK/PJ.001_Fabi-web"]);
        assert_eq!(run(&dir, "client:ana"), ["300.HOME/PJ.003_Mine"]);
        assert_eq!(run(&dir, "in:200.WORK status: in-progress"), ["200.WORK/PJ.002_Other"]);
        assert_eq!(run(&dir, "in:200.WORK status:hold"), ["200.WORK/PJ.001_Fabi-web"]);
        assert_eq!(run(&dir, "other"), ["200.WORK/PJ.002_Other"]);
        assert_eq!(run(&dir, "in:work client:mike"), ["200.WORK/PJ.001_Fabi-web"]);
        assert_eq!(run(&dir, "in:WORK other"), ["200.WORK/PJ.002_Other"]);
        assert_eq!(run(&dir, "in:200.WORK/PJ.002_Other"), ["200.WORK/PJ.002_Other"]);
        assert!(run(&dir, "in:HOME client:mike").is_empty());
        assert_eq!(run(&dir, "in:ho client:ana"), ["300.HOME/PJ.003_Mine"]);
        assert_eq!(run(&dir, "in:20 client:mike"), ["200.WORK/PJ.001_Fabi-web"]);
        assert_eq!(run(&dir, "in:/200.WORK status:=hold"), ["200.WORK/PJ.001_Fabi-web"]);
        assert!(run(&dir, "status:=hol").is_empty());
        assert_eq!(run(&dir, "in:/200.WORK/PJ.002_Other"), ["200.WORK/PJ.002_Other"]);
        assert!(run(&dir, "in:/300 client:mike").is_empty());
    }

    #[test]
    fn suggestions_complete_folders_and_values() {
        let dir = storage();
        let data: Mapping = serde_yaml::from_str(YAML).unwrap();
        rebuild(dir.path(), &data).unwrap();
        let store = Store::open_or_create(dir.path()).unwrap();
        let folders = store.suggest("in", "wo").unwrap();
        assert_eq!(folders.first().map(|s| (s.value.as_str(), s.label.as_str())), Some(("200.WORK", "200.WORK")));
        let nested = store.suggest("in", "PJ.002").unwrap();
        assert_eq!(nested.first().map(|s| (s.value.as_str(), s.hint.as_str())), Some(("200.WORK/PJ.002_Other", "200.WORK")));
        let owners: Vec<String> = store.suggest("owner", "").unwrap().into_iter().map(|s| s.value).collect();
        assert_eq!(owners, ["Ana", "Mike"]);
    }

    #[test]
    fn a_changed_info_file_is_picked_up_by_the_next_rebuild() {
        let dir = storage();
        let data: Mapping = serde_yaml::from_str(YAML).unwrap();
        rebuild(dir.path(), &data).unwrap();
        fs::write(dir.path().join("200.WORK/PJ.001_Fabi-web/.ant/info.json"), r#"{"holder": "Zed"}"#).unwrap();
        rebuild(dir.path(), &data).unwrap();
        assert!(run(&dir, "client:Mike").is_empty());
        assert_eq!(run(&dir, "client:Zed"), ["200.WORK/PJ.001_Fabi-web"]);
    }
}
