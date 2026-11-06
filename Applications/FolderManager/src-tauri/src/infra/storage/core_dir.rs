// Where the Storage root keeps its own files: the root .ant/, with the 999.Settings/00.Core of storages not upgraded yet read as a fallback.
use std::path::{Path, PathBuf};

use crate::domain::config::{CORE_FOLDER, DATA_DIR, SETTINGS_FOLDER};

pub fn data_dir(root: &Path) -> PathBuf {
    root.join(DATA_DIR)
}

pub fn legacy_core_dir(root: &Path) -> PathBuf {
    root.join(SETTINGS_FOLDER).join(CORE_FOLDER)
}

// The file in .ant/ when it is there, else the legacy copy; a write always goes to the .ant/ path this returns when neither exists.
pub fn root_file(root: &Path, name: &str) -> PathBuf {
    let current = data_dir(root).join(name);
    let legacy = legacy_core_dir(root).join(name);
    if !current.exists() && legacy.exists() { legacy } else { current }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn a_root_file_is_read_from_ant_first_and_from_the_legacy_core_after() {
        let dir = TempDir::new("core-dir");
        let (root, name) = (dir.path(), "x.txt");
        assert_eq!(root_file(root, name), root.join(".ant").join(name));
        dir.mkdir("999.Settings/00.Core");
        std::fs::write(root.join("999.Settings/00.Core/x.txt"), "").unwrap();
        assert_eq!(root_file(root, name), legacy_core_dir(root).join(name));
        dir.mkdir(".ant");
        std::fs::write(root.join(".ant/x.txt"), "").unwrap();
        assert_eq!(root_file(root, name), root.join(".ant").join(name));
    }
}
