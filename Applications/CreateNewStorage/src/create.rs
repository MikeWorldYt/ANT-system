// Builds the storage on disk and undoes this run's folders when an essential step fails.
use std::path::{Path, PathBuf};

use crate::config::{ANT_DIR, TEMPLATE_NAME, TEMPLATE_URL};
use crate::fetch::{download, MAX_TEXT_BYTES};
use crate::fs_ops::{make_dirs_tracked, rollback_created, Backups};
use crate::info::write_info;
use crate::overwrite::{decide, gate, refusal, Decision};
use crate::template::tree_from_template;
use crate::tree_types::TreeNode;
use crate::{downloads, Level};

pub struct CreateOptions {
    pub root: PathBuf,
    pub template: Option<Vec<u8>>,
    pub allow_existing: bool,
    pub download: bool,
    pub folder_manager: bool,
    pub confirm: Option<Confirm>,
}

// Asks whether the listed existing files may be overwritten inside the given folder.
pub type Confirm = fn(&Path, &[PathBuf]) -> bool;

pub type Progress<'a> = &'a mut dyn FnMut(Level, &str);

pub fn create(opts: &CreateOptions, progress: Progress) -> Result<(), String> {
    if decide(opts.root.exists(), opts.allow_existing, 0, opts.confirm.is_some()) == Decision::Refuse {
        return Err(refusal(&opts.root));
    }
    let template = load_template(opts)?;
    let tree = tree_from_template(&template)?;
    gate(opts, &tree, progress)?;
    storage(opts, &template, &tree, progress)
}

// Nothing touches the disk until the template is fetched and readable.
pub fn load_template(opts: &CreateOptions) -> Result<Vec<u8>, String> {
    match (&opts.template, opts.download) {
        (Some(bytes), _) => Ok(bytes.clone()),
        (None, true) => download(TEMPLATE_URL, MAX_TEXT_BYTES, 60).map_err(|e| format!("Failed to download essential file 'template.md': {e}")),
        (None, false) => Err("A template.md is required when downloads are disabled".to_string()),
    }
}

pub(crate) fn storage(opts: &CreateOptions, template: &[u8], tree: &[TreeNode], progress: Progress) -> Result<(), String> {
    let mut created = Vec::new();
    let mut backups = Backups::default();
    match build(opts, template, tree, &mut created, &mut backups, progress) {
        Ok(()) => {
            backups.discard(&mut |p, why| progress(Level::Warn, &format!("⚠ {}: {why}", p.display())));
            Ok(())
        }
        Err(e) => {
            progress(Level::Error, &format!("✖ {e}"));
            progress(Level::Step, "Applying roll-back...");
            let left = rollback_created(&mut created, &mut |r| match r {
                Ok(p) => progress(Level::Ok, &format!("◄◄ {} (Deleted)", p.display())),
                Err((p, why)) => progress(Level::Error, &format!("✖ {} (Not deleted: {why})", p.display())),
            });
            let unrestored = backups.restore(&mut |r| match r {
                Ok(p) => progress(Level::Ok, &format!("◄◄ {} (Restored)", p.display())),
                Err((p, why)) => progress(Level::Error, &format!("✖ {} (Not restored: {why})", p.display())),
            });
            let leftover: Vec<String> = left.iter().chain(&unrestored).map(|p| p.display().to_string()).collect();
            match leftover.is_empty() {
                true => Err(e),
                false => Err(format!("{e}; these paths could not be cleaned up: {}", leftover.join(", "))),
            }
        }
    }
}

fn build(opts: &CreateOptions, template: &[u8], tree: &[TreeNode], created: &mut Vec<PathBuf>, backups: &mut Backups, progress: Progress) -> Result<(), String> {
    let root = &opts.root;
    let ant = root.join(ANT_DIR);
    progress(Level::Step, "Generating root directory ...");
    make_dirs_tracked(root, created)?;
    progress(Level::Ok, &format!("✔ {}", root.display()));
    progress(Level::Step, "Generating ANT Essentials ...");
    make_dirs_tracked(&ant, created)?;
    progress(Level::Ok, &format!("✔ {}", ant.display()));
    let template_dest = ant.join(TEMPLATE_NAME);
    backups.write(&template_dest, template)?;
    progress(Level::Ok, &format!("✔ {}", template_dest.display()));

    progress(Level::Step, "Generating an ANT Storage Template ...");
    make_tree(tree, root, created, backups, progress)?;
    if opts.download {
        downloads::optional_downloads(root, opts.folder_manager, backups, progress);
    }
    progress(Level::Ok, "Congrats, ANT Storage successfully created!");
    Ok(())
}

fn make_tree(nodes: &[TreeNode], base: &Path, created: &mut Vec<PathBuf>, backups: &mut Backups, progress: Progress) -> Result<(), String> {
    for node in nodes {
        let folder = base.join(&node.name);
        make_dirs_tracked(&folder, created)?;
        progress(Level::Ok, &format!("✔ {}", folder.display()));
        write_info(&folder, &node.fields, created, backups, progress)?;
        if node.base_kind() == "REV" {
            continue;
        }
        make_tree(&node.children, &folder, created, backups, progress)?;
    }
    Ok(())
}
