// What the Properties window shows: an item's own facts at once, and a folder's totals from a walk that reports as it goes.
use std::fs::Metadata;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::domain::error::R;
use crate::infra::fs_safe::{io_err, is_link};

#[derive(Serialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub size: u64,
    pub on_disk: u64,
    pub files: u64,
    pub folders: u64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    pub name: String,
    pub folder: bool,
    pub kind: String,
    pub location: String,
    pub size: Option<u64>,
    pub on_disk: Option<u64>,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
    pub read_only: bool,
    pub hidden: bool,
}

pub fn on_disk(len: u64, cluster: u64) -> u64 {
    len.div_ceil(cluster.max(1)) * cluster.max(1)
}

// Links and junctions lead elsewhere and unreadable entries are left out, as in the folder sizes of the Details view.
// `tick` sees the running totals now and then and answers false to stop, as when the window was closed.
pub fn walk(dir: &Path, cluster: u64, tick: &mut dyn FnMut(&Totals) -> bool) -> Option<Totals> {
    let mut sum = Totals::default();
    let mut pending = vec![dir.to_path_buf()];
    let mut seen = 0u32;
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if is_link(&meta) {
                continue;
            }
            if meta.is_dir() {
                sum.folders += 1;
                pending.push(entry.path());
            } else {
                sum.files += 1;
                sum.size += meta.len();
                sum.on_disk += on_disk(meta.len(), cluster);
            }
            seen += 1;
            if seen % 256 == 0 && !tick(&sum) {
                return None;
            }
        }
    }
    Some(sum)
}

fn millis(time: std::io::Result<SystemTime>) -> Option<i64> {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64)
}

// The verbatim prefix of a canonical Windows path is noise to the reader.
fn plain(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(text)
}

#[cfg(windows)]
fn is_hidden(_path: &Path, meta: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x2 != 0
}

#[cfg(not(windows))]
fn is_hidden(path: &Path, _meta: &Metadata) -> bool {
    path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))
}

fn kind_of(path: &Path, folder: bool, system_root: bool) -> String {
    if folder {
        return if system_root { "System Folder" } else { "File folder" }.to_string();
    }
    match path.extension().map(|e| e.to_string_lossy().to_uppercase()) {
        Some(ext) if !ext.is_empty() => format!("{ext} File"),
        _ => "File".to_string(),
    }
}

// A file's size is known now; a folder's comes from `walk`.
pub fn facts(path: &Path, system_root: bool, cluster: u64) -> R<Facts> {
    let meta = std::fs::metadata(path).map_err(|e| io_err("read properties of", path, &e))?;
    let folder = meta.is_dir();
    Ok(Facts {
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| plain(path)),
        folder,
        kind: kind_of(path, folder, system_root),
        location: path.parent().map(plain).unwrap_or_default(),
        size: (!folder).then(|| meta.len()),
        on_disk: (!folder).then(|| on_disk(meta.len(), cluster)),
        created: millis(meta.created()),
        modified: millis(meta.modified()),
        accessed: millis(meta.accessed()),
        read_only: meta.permissions().readonly(),
        hidden: is_hidden(path, &meta),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn rounds_up_to_the_cluster() {
        assert_eq!(on_disk(0, 4096), 0);
        assert_eq!(on_disk(1, 4096), 4096);
        assert_eq!(on_disk(4096, 4096), 4096);
        assert_eq!(on_disk(4097, 4096), 8192);
    }

    #[test]
    fn walk_counts_nested_items() {
        let dir = TempDir::new("properties");
        dir.mkdir("a/b");
        std::fs::write(dir.path().join("x.bin"), [0u8; 10]).unwrap();
        std::fs::write(dir.path().join("a/y.bin"), [0u8; 5000]).unwrap();
        std::fs::write(dir.path().join("a/b/z.bin"), [0u8; 30]).unwrap();
        let totals = walk(dir.path(), 4096, &mut |_| true).unwrap();
        assert_eq!(totals, Totals { size: 5040, on_disk: 4096 + 8192 + 4096, files: 3, folders: 2 });
        assert_eq!(walk(&dir.path().join("missing"), 4096, &mut |_| true), Some(Totals::default()));
    }

    #[test]
    fn walk_stops_when_the_tick_refuses() {
        let dir = TempDir::new("properties-stop");
        for n in 0..300 {
            std::fs::write(dir.path().join(format!("{n}.bin")), [0u8; 1]).unwrap();
        }
        assert_eq!(walk(dir.path(), 4096, &mut |_| false), None);
    }

    #[test]
    fn facts_describe_a_file_and_a_folder() {
        let dir = TempDir::new("properties-facts");
        dir.mkdir("sub");
        std::fs::write(dir.path().join("pic.png"), [0u8; 10]).unwrap();
        let file = facts(&dir.path().join("pic.png"), false, 4096).unwrap();
        assert_eq!((file.name.as_str(), file.kind.as_str(), file.size, file.on_disk), ("pic.png", "PNG File", Some(10), Some(4096)));
        let folder = facts(&dir.path().join("sub"), false, 4096).unwrap();
        assert_eq!((folder.kind.as_str(), folder.size), ("File folder", None));
        assert_eq!(facts(dir.path(), true, 4096).unwrap().kind, "System Folder");
        assert!(facts(&dir.path().join("missing"), false, 4096).is_err());
    }
}
