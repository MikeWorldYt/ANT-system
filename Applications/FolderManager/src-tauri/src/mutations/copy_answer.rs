// Applies the user's answer for a folder that a refresh marked as a copy of another one.
use std::path::Path;

use serde::Deserialize;

use super::locate;
use crate::error::{FmError, R};
use crate::fs_safe::StorageLock;
use crate::index::footprint;
use crate::tree::tree_store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyAnswer {
    Copy,
    Independent,
}

// A copy keeps its tailed id, so only an independent project changes anything.
pub fn answer_copy(root: &Path, rel: &[String], answer: CopyAnswer) -> R<()> {
    if answer == CopyAnswer::Copy {
        return Ok(());
    }
    let _lock = StorageLock::acquire(root)?;
    let mut data = tree_store::load(root)?;
    if locate(root, &data, rel)?.ghost {
        return Err(FmError::GhostNotAllowed { op: "detach".to_string() });
    }
    footprint::detach(root, &mut data, rel)?;
    tree_store::save(root, &data)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_yaml::{Mapping, Value};

    use super::*;
    use crate::index::footprint::{plan, Copies};
    use crate::test_support::{rel, TempDir};
    use crate::tree::tree_store::{get_node, sync};

    const ORIGINAL: &str = "100.A/PJ.001";
    const COPY: &str = "100.A/PJ.002";

    fn read_footprint(dir: &TempDir, folder: &str) -> String {
        fs::read_to_string(dir.path().join(folder).join(".ant/dragpath.json")).unwrap()
    }

    fn id_of(dir: &TempDir, folder: &str) -> String {
        let value: Value = serde_yaml::from_str(&read_footprint(dir, folder)).unwrap();
        value.get("id").and_then(Value::as_str).unwrap().to_string()
    }

    fn stored_uid(dir: &TempDir, folder: &str) -> Option<String> {
        let data = tree_store::load(dir.path()).unwrap();
        let parts: Vec<&str> = folder.split('/').collect();
        get_node(&data, &rel(&parts)).and_then(|n| n.get("uid")).and_then(Value::as_str).map(str::to_string)
    }

    // A refresh has tailed the copy and saved the tree, which is where the question is asked.
    fn tailed_copy(name: &str) -> TempDir {
        let dir = TempDir::new(name);
        dir.mkdir(&format!("{ORIGINAL}/.ant"));
        let mut data: Mapping = serde_yaml::from_str("100.A:\n  type: NODE-L1\n  PJ.001: {type: WKB-L2, status: hold}\n").unwrap();
        sync(dir.path(), &mut data).unwrap();
        plan(dir.path(), &mut data, Copies::Tail).unwrap();
        let id = id_of(&dir, ORIGINAL);
        dir.mkdir(&format!("{COPY}/.ant"));
        fs::write(dir.path().join(COPY).join(".ant/dragpath.json"), format!("{{\"id\": \"{id}\", \"flags\": [\"urgent\"]}}\n")).unwrap();
        sync(dir.path(), &mut data).unwrap();
        let result = plan(dir.path(), &mut data, Copies::Tail).unwrap();
        assert_eq!(result.copies.len(), 1);
        tree_store::save(dir.path(), &data).unwrap();
        assert_eq!(id_of(&dir, COPY), format!("{id}-2"));
        dir
    }

    #[test]
    fn a_copy_answer_keeps_the_tailed_id() {
        let dir = tailed_copy("answer-copy");
        let tail = id_of(&dir, COPY);
        answer_copy(dir.path(), &rel(&["100.A", "PJ.002"]), CopyAnswer::Copy).unwrap();
        assert_eq!(id_of(&dir, COPY), tail);
        assert_eq!(stored_uid(&dir, COPY), Some(tail));
    }

    #[test]
    fn an_independent_answer_gets_a_fresh_id_without_the_tail() {
        let dir = tailed_copy("answer-independent");
        let tail = id_of(&dir, COPY);
        answer_copy(dir.path(), &rel(&["100.A", "PJ.002"]), CopyAnswer::Independent).unwrap();
        let fresh = id_of(&dir, COPY);
        assert!(!fresh.contains('-') && fresh.len() == 15 && fresh != tail && fresh != id_of(&dir, ORIGINAL));
        assert_eq!(stored_uid(&dir, COPY), Some(fresh));
        assert!(read_footprint(&dir, COPY).contains("\"flags\": [\"urgent\"]"));
        let again = answer_copy(dir.path(), &rel(&["100.A", "PJ.002"]), CopyAnswer::Independent);
        assert!(matches!(again, Err(FmError::InvalidPath { .. })));
    }

    #[test]
    fn a_dismissed_question_keeps_the_tail_and_is_not_asked_again() {
        let dir = tailed_copy("answer-dismissed");
        let tail = id_of(&dir, COPY);
        let tree = crate::tree::snapshot(dir.path(), Copies::Tail).unwrap();
        assert!(tree.copies.is_empty());
        assert_eq!(id_of(&dir, COPY), tail);
        assert_eq!(stored_uid(&dir, COPY), Some(tail));
    }
}
