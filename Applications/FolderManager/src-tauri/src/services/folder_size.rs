// Totals a folder's file sizes for the Details view, which shows them without waiting on the walk.
use std::path::Path;

use crate::infra::fs_safe::is_link;

// Links and junctions lead elsewhere and unreadable entries are left out, so the sum is what the folder itself holds.
pub fn total(dir: &Path) -> u64 {
    let mut sum = 0;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if is_link(&meta) {
                continue;
            }
            if meta.is_dir() {
                pending.push(entry.path());
            } else {
                sum += meta.len();
            }
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn sums_nested_files() {
        let dir = TempDir::new("folder-size");
        dir.mkdir("a/b");
        std::fs::write(dir.path().join("x.bin"), [0u8; 10]).unwrap();
        std::fs::write(dir.path().join("a/y.bin"), [0u8; 20]).unwrap();
        std::fs::write(dir.path().join("a/b/z.bin"), [0u8; 30]).unwrap();
        assert_eq!(total(dir.path()), 60);
        assert_eq!(total(&dir.path().join("missing")), 0);
    }
}
