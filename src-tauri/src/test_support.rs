// Throwaway storages and tree readers shared by the unit tests of every module.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_yaml::Mapping;

use crate::config::TRASH_DIR;
use crate::error::R;
use crate::mutations::{delete_node, recycle_staged_with, Mutation};
use crate::tree::tree_store::{get_type, load, save};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("fm-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn mkdir(&self, rel: &str) {
        std::fs::create_dir_all(self.0.join(rel)).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn rel(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

pub fn storage(tag: &str, dirs: &[&str], yaml: &str) -> TempDir {
    let dir = TempDir::new(tag);
    dir.mkdir("999.Settings/00.Core");
    for d in dirs {
        dir.mkdir(d);
    }
    save(dir.path(), &serde_yaml::from_str::<Mapping>(yaml).unwrap()).unwrap();
    dir
}

pub fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

pub fn yaml(dir: &Path) -> Mapping {
    load(dir).unwrap()
}

pub fn yaml_type(dir: &Path, parts: &[&str]) -> String {
    get_type(&yaml(dir), &rel(parts))
}

// Holding ant.db open for read only keeps loads working but makes the save fail after the disk changes.
#[cfg(windows)]
pub fn break_tree_save(dir: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 0x1;
    fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(crate::storage::db::db_path(dir)).unwrap()
}

const ACADEMIC: &str = "100.ACADEMIC:\n  type: NODE-L1\n  01.Math:\n    type: NODE-L2\n    status: done\n    \
    98.Math: {type: REV-L2}\n    90.Math: {type: RES}\n    01.Algebra: {type: NODE-L3}\n  \
    02.Bio:\n    type: NODE-L2\n  02.Gone:\n    type: NODE-L2\n  03.Ghost:\n    type: NODE-L2\n";
const ACADEMIC_DIRS: &[&str] = &[
    "100.ACADEMIC/01.Math/98.Math",
    "100.ACADEMIC/01.Math/90.Math",
    "100.ACADEMIC/01.Math/01.Algebra",
    "100.ACADEMIC/02.Bio",
];

pub fn academic(tag: &str) -> TempDir {
    storage(tag, ACADEMIC_DIRS, ACADEMIC)
}

const NODES: &str = "100.WORK:\n  type: NODE-L1\n  01.A:\n    type: NODE-L2\n    01.Keep: {type: NODE-L3}\n  \
    02.B:\n    type: NODE-L2\n  03.C:\n    type: NODE-L2\n  04.Wkb:\n    type: WKB\n  \
    05.Ghost:\n    type: NODE-L2\n  98.Rev:\n    type: REV-L2\n";
const NODE_DIRS: &[&str] =
    &["100.WORK/01.A/01.Keep", "100.WORK/02.B", "100.WORK/03.C", "100.WORK/04.Wkb", "100.WORK/98.Rev"];

pub fn work(tag: &str) -> TempDir {
    storage(tag, NODE_DIRS, NODES)
}

pub fn ok_recycle(path: &Path) -> Result<(), String> {
    assert!(path.parent().unwrap().ends_with(TRASH_DIR), "{}", path.display());
    fs::remove_dir_all(path).map_err(|e| e.to_string())
}

// What the command layer does: delete under the lock, then recycle after releasing it.
pub fn delete_with(
    root: &Path,
    rel: &[String],
    expect_ghost: bool,
    confirm: bool,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> R<Mutation> {
    let (mut m, pending) = delete_node(root, rel, expect_ghost, confirm)?;
    if let Some(pending) = pending {
        m.warnings.extend(recycle_staged_with(pending, recycle));
    }
    Ok(m)
}
