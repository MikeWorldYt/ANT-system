// Builds the storage on disk and undoes this run's folders when an essential step fails.
use std::path::{Path, PathBuf};

use serde_yaml::Mapping;

use crate::config::{CORE_DIR, REVIEW_DIR, SETTINGS_DIR, TREE_YAML_NAME, TREE_YAML_URL};
use crate::fs_ops::{make_dirs_tracked, rollback_created, write_file_atomic};
use crate::plan::parse_tree;
use crate::shortcuts::{create_shortcuts, review_shortcut_name};
use crate::tree_types::{child_entries, node_type};
use crate::{downloads, Level};

pub struct CreateOptions {
    pub root: PathBuf,
    pub tree_yaml: Option<String>,
    pub allow_existing: bool,
    pub download: bool,
}

pub type Progress<'a> = &'a mut dyn FnMut(Level, &str);

pub fn create(opts: &CreateOptions, progress: Progress) -> Result<(), String> {
    if opts.root.exists() && !opts.allow_existing {
        return Err(format!("{} already exists; creating inside it needs explicit permission", opts.root.display()));
    }
    let (raw, tree) = load_tree(opts)?;
    let mut created = Vec::new();
    match build(opts, &raw, &tree, &mut created, progress) {
        Ok(()) => Ok(()),
        Err(e) => {
            progress(Level::Error, &format!("✖ {e}"));
            progress(Level::Step, "Applying roll-back...");
            let left = rollback_created(&mut created, &mut |r| match r {
                Ok(p) => progress(Level::Ok, &format!("◄◄ {} (Deleted)", p.display())),
                Err((p, why)) => progress(Level::Error, &format!("✖ {} (Not deleted: {why})", p.display())),
            });
            let leftover: Vec<String> = left.iter().map(|p| p.display().to_string()).collect();
            match leftover.is_empty() {
                true => Err(e),
                false => Err(format!("{e}; these folders could not be removed: {}", leftover.join(", "))),
            }
        }
    }
}

// Nothing touches the disk until Tree.yaml is fetched and valid.
fn load_tree(opts: &CreateOptions) -> Result<(String, Mapping), String> {
    let raw = match (&opts.tree_yaml, opts.download) {
        (Some(text), _) => text.clone(),
        (None, true) => crate::fetch::download_text(TREE_YAML_URL).map_err(|e| format!("Failed to download essential file 'Tree.yaml': {e}"))?,
        (None, false) => return Err("Tree.yaml is required when downloads are disabled".to_string()),
    };
    let tree = parse_tree(&raw)?;
    Ok((raw, tree))
}

fn build(opts: &CreateOptions, raw: &str, tree: &Mapping, created: &mut Vec<PathBuf>, progress: Progress) -> Result<(), String> {
    let root = &opts.root;
    let review = root.join(REVIEW_DIR);
    let core = root.join(SETTINGS_DIR).join(CORE_DIR);
    progress(Level::Step, "Generating root directory ...");
    make_dirs_tracked(root, created)?;
    progress(Level::Ok, &format!("✔ {}", root.display()));
    progress(Level::Step, "Generating ANT Essentials ...");
    for dir in [&review, &core] {
        make_dirs_tracked(dir, created)?;
        progress(Level::Ok, &format!("✔ {}", dir.display()));
    }
    let tree_dest = core.join(TREE_YAML_NAME);
    write_file_atomic(&tree_dest, raw.as_bytes())?;
    progress(Level::Ok, &format!("✔ {}", tree_dest.display()));

    progress(Level::Step, "Generating an ANT Storage Template ...");
    let mut links = Vec::new();
    make_tree(tree, root, &review, &[], created, &mut links, progress)?;
    for failure in create_shortcuts(&links) {
        progress(Level::Warn, &format!("⚠ {failure}"));
    }
    if opts.download {
        downloads::optional_downloads(root, &core, progress);
    }
    progress(Level::Ok, "Congrats, ANT Storage successfully created!");
    Ok(())
}

fn make_tree(
    m: &Mapping,
    base: &Path,
    review: &Path,
    context: &[String],
    created: &mut Vec<PathBuf>,
    links: &mut Vec<(PathBuf, PathBuf)>,
    progress: Progress,
) -> Result<(), String> {
    for (name, child) in child_entries(m) {
        let folder = base.join(name);
        let kind = node_type(child);
        // CFG folders are the essentials made above; only their children come from the template.
        if kind != "CFG" {
            make_dirs_tracked(&folder, created)?;
            progress(Level::Ok, &format!("✔ {}", folder.display()));
        }
        if kind == "REV" {
            match review_shortcut_name(context, name) {
                Ok(link) => {
                    progress(Level::Info, &format!("  ⤤ shortcut → {link}"));
                    links.push((folder, review.join(link)));
                }
                Err(e) => progress(Level::Warn, &format!("  ⚠ {e}")),
            }
            continue;
        }
        let next: Vec<String> = context.iter().cloned().chain([name.to_string()]).collect();
        make_tree(child, &folder, review, &next, created, links, progress)?;
    }
    Ok(())
}
