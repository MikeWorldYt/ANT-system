// Decides what happens when a run would write into an existing folder or over existing files.
use std::path::{Path, PathBuf};

use crate::config::{ANT_DIR, FOLDER_MANAGER_NAME, TEMPLATE_NAME};
use crate::create::CreateOptions;
use crate::info::info_path;
use crate::tree_types::TreeNode;
use crate::Level;

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Proceed,
    Announce,
    Ask,
    Refuse,
}

pub fn decide(root_exists: bool, allow_existing: bool, overwrites: usize, can_ask: bool) -> Decision {
    match (root_exists, allow_existing, overwrites > 0, can_ask) {
        (false, ..) | (_, true, false, _) => Decision::Proceed,
        (_, true, true, _) => Decision::Announce,
        (_, false, _, true) => Decision::Ask,
        (_, false, _, false) => Decision::Refuse,
    }
}

pub fn refusal(root: &Path) -> String {
    format!("{} already exists; pass --yes to create inside it", root.display())
}

fn info_files(nodes: &[TreeNode], base: &Path, out: &mut Vec<PathBuf>) {
    for node in nodes {
        let folder = base.join(&node.name);
        out.extend(info_path(&folder, &node.fields));
        if node.base_kind() != "REV" {
            info_files(&node.children, &folder, out);
        }
    }
}

// Every file this run would write that is already on disk.
pub fn existing_targets(opts: &CreateOptions, tree: &[TreeNode]) -> Vec<PathBuf> {
    let mut files = vec![opts.root.join(ANT_DIR).join(TEMPLATE_NAME)];
    info_files(tree, &opts.root, &mut files);
    if opts.download && opts.folder_manager {
        files.push(opts.root.join(FOLDER_MANAGER_NAME));
    }
    files.retain(|f| f.exists());
    files
}

pub fn gate(opts: &CreateOptions, tree: &[TreeNode], progress: &mut dyn FnMut(Level, &str)) -> Result<(), String> {
    let files = existing_targets(opts, tree);
    match decide(opts.root.exists(), opts.allow_existing, files.len(), opts.confirm.is_some()) {
        Decision::Proceed => Ok(()),
        Decision::Refuse => Err(refusal(&opts.root)),
        Decision::Announce => {
            files.iter().for_each(|f| progress(Level::Warn, &format!("⚠ Overwriting {} (the old copy is restored if the run fails)", f.display())));
            Ok(())
        }
        Decision::Ask => match opts.confirm.is_some_and(|ask| ask(&opts.root, &files)) {
            true => Ok(()),
            false => Err("Cancelled; nothing was changed".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_dir::TempDir;
    use crate::tests::{node, TEMPLATE};

    fn opts(root: PathBuf, allow_existing: bool, confirm: Option<crate::Confirm>) -> CreateOptions {
        CreateOptions { root, template: Some(TEMPLATE.to_vec()), allow_existing, download: false, folder_manager: false, confirm }
    }

    #[test]
    fn decisions_follow_the_flags() {
        assert_eq!(decide(false, false, 0, false), Decision::Proceed);
        assert_eq!(decide(true, true, 0, false), Decision::Proceed);
        assert_eq!(decide(true, true, 2, false), Decision::Announce);
        assert_eq!(decide(true, false, 2, true), Decision::Ask);
        assert_eq!(decide(true, false, 0, true), Decision::Ask);
        assert_eq!(decide(true, false, 2, false), Decision::Refuse);
    }

    #[test]
    fn existing_targets_lists_only_files_already_on_disk() {
        let tmp = TempDir::new("targets");
        let mut project = node("PJ.001_A", "WKB", vec![]);
        project.fields = vec![("owner".to_string(), "Ren".to_string())];
        let tree = [node("300.PROJECTS", "WS-L1", vec![project])];
        let o = opts(tmp.path().to_path_buf(), true, None);
        assert!(existing_targets(&o, &tree).is_empty());
        std::fs::create_dir_all(tmp.path().join(".ant")).unwrap();
        std::fs::write(tmp.path().join(".ant/template.md"), b"old").unwrap();
        std::fs::create_dir_all(tmp.path().join("300.PROJECTS/PJ.001_A/.ant")).unwrap();
        std::fs::write(tmp.path().join("300.PROJECTS/PJ.001_A/.ant/info.json"), b"{}").unwrap();
        assert_eq!(existing_targets(&o, &tree).len(), 2);
    }

    #[test]
    fn the_gate_asks_announces_or_refuses_without_real_input() {
        let tmp = TempDir::new("gate");
        std::fs::create_dir_all(tmp.path().join(".ant")).unwrap();
        std::fs::write(tmp.path().join(".ant/template.md"), b"old").unwrap();
        let mut quiet = |_: Level, _: &str| panic!("asking must not print");
        let root = tmp.path().to_path_buf();
        assert!(gate(&opts(root.clone(), false, Some(|_, files| files.len() == 1)), &[], &mut quiet).is_ok());
        assert_eq!(gate(&opts(root.clone(), false, Some(|_, _| false)), &[], &mut quiet).unwrap_err(), "Cancelled; nothing was changed");
        assert!(gate(&opts(root.clone(), false, None), &[], &mut quiet).unwrap_err().contains("already exists"));
        let mut log = Vec::new();
        assert!(gate(&opts(root, true, None), &[], &mut |l, m| log.push((l, m.to_string()))).is_ok());
        assert!(log.len() == 1 && log[0].0 == Level::Warn && log[0].1.contains("template.md"));
    }
}
