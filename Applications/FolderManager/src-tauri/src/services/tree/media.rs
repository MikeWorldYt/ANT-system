// Screenshots and recordings have one home: a global folder at the root and a support folder in each main category.
use std::fs;
use std::path::Path;

use crate::domain::config::MEDIA_NODES;
use crate::domain::error::R;
use crate::domain::names::{editable_part, is_main_category, support_kind};
use crate::infra::fs_safe::io_err;
use crate::infra::scan::list_subdirs;

fn make(dir: &Path) -> R<()> {
    fs::create_dir(dir).map_err(|e| io_err("create folder", dir, &e))
}

// A category that already has its support folder, under any title, keeps it.
pub fn ensure(root: &Path) -> R<()> {
    let top = list_subdirs(root)?;
    for (word, kind, global) in MEDIA_NODES {
        if !top.iter().any(|name| name.eq_ignore_ascii_case(global)) {
            make(&root.join(global))?;
        }
        for category in top.iter().filter(|name| is_main_category(name)) {
            let dir = root.join(category);
            if !list_subdirs(&dir)?.iter().any(|name| support_kind(name) == Some(*kind)) {
                make(&dir.join(format!("{word}{}", editable_part(category))))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn every_category_gets_its_folders_once() {
        let dir = TempDir::new("media-ensure");
        dir.mkdir("200.WORK/REC.Meetings");
        dir.mkdir("Loose");
        ensure(dir.path()).unwrap();
        ensure(dir.path()).unwrap();
        for path in ["ANT.SCREENSHOTS", "ANT.RECORDINGS", "200.WORK/SS.WORK", "200.WORK/REC.Meetings"] {
            assert!(dir.path().join(path).is_dir(), "{path}");
        }
        assert!(!dir.path().join("200.WORK/REC.WORK").exists());
        assert!(!dir.path().join("Loose/SS.Loose").exists());
    }
}
