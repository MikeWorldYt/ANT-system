// Picks a free name for a dropped file, numbering duplicates the way Explorer does.
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::{FmError, R};
use crate::fs_safe::{io_err, same_name, validate_component};

const MAX_SUFFIX: u32 = 9_999;

pub(super) fn numbered(name: &str, n: u32) -> String {
    // A leading dot starts a hidden name, not an extension.
    match name.rfind('.').filter(|&i| i > 0) {
        Some(i) => format!("{} ({n}){}", &name[..i], &name[i..]),
        None => format!("{name} ({n})"),
    }
}

// Tree entries count as taken so a file never claims the name of a folder that is only missing on disk.
pub(crate) fn free_name(dir: &Path, name: &str, tree_names: &[&str]) -> R<String> {
    for n in 1..=MAX_SUFFIX {
        let candidate = if n == 1 { name.to_string() } else { numbered(name, n) };
        validate_component(&candidate)?;
        if !tree_names.iter().any(|t| same_name(t, &candidate)) && is_free(&dir.join(&candidate))? {
            return Ok(candidate);
        }
    }
    Err(FmError::AlreadyExists { name: name.to_string() })
}

fn is_free(path: &Path) -> R<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(true),
        Err(e) => Err(io_err("inspect path", path, &e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_number_goes_before_the_last_extension() {
        assert_eq!(numbered("report.pdf", 2), "report (2).pdf");
        assert_eq!(numbered("archive.tar.gz", 3), "archive.tar (3).gz");
        assert_eq!(numbered("README", 2), "README (2)");
        assert_eq!(numbered(".gitignore", 2), ".gitignore (2)");
    }

    #[test]
    fn the_first_free_number_on_disk_wins() {
        let dir = TempDir::new("drop-naming");
        assert_eq!(free_name(dir.path(), "report.pdf", &[]).unwrap(), "report.pdf");
        for taken in ["report.pdf", "report (2).pdf", "report (3).pdf", "notes"] {
            fs::write(dir.path().join(taken), "x").unwrap();
        }
        dir.mkdir("report (4).pdf");
        assert_eq!(free_name(dir.path(), "report.pdf", &[]).unwrap(), "report (5).pdf");
        assert_eq!(free_name(dir.path(), "notes", &[]).unwrap(), "notes (2)");
        assert_eq!(free_name(dir.path(), "notes", &["Notes (2)"]).unwrap(), "notes (3)");
    }
}
