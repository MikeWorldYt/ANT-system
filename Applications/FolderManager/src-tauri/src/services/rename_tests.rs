use std::fs;

use super::*;
use crate::infra::storage::rename::{status, RenameStatus};
use crate::test_support::TempDir;

const STAMP: &str = "26-10-24-09-05";

fn storage_in(dir: &TempDir, name: &str) -> PathBuf {
    let root = dir.path().join(name);
    fs::create_dir_all(root.join("999.Settings")).unwrap();
    root
}

#[test]
fn rename_and_revert_round_trip() {
    let dir = TempDir::new("rename-trip");
    let storage = Storage::resolve(storage_in(&dir, "A"));
    let renamed = rename_flow(&storage, STAMP, |_| Ok("B".to_string())).unwrap();
    assert_eq!(renamed, dir.path().join("B"));
    assert_eq!(storage.root().unwrap(), renamed);
    assert!(!dir.path().join("A").exists());
    let back = rename_flow(&storage, STAMP, revert_target).unwrap();
    assert_eq!(back, dir.path().join("A"));
    assert_eq!(status(&back), Some(RenameStatus { old: "B".into(), new: "A".into(), revertible: true }));
    assert!(rename_flow(&storage, "bad", |_| Ok("C".to_string())).is_err());
    let again = rename_flow(&storage, STAMP, revert_target).unwrap();
    assert_eq!(again, dir.path().join("B"));
}
