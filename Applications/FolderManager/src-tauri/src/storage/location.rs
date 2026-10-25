// Where the Storage is looked for at launch and which typed paths are accepted as a new Storage.
use std::path::{Path, PathBuf};

use crate::ant_core::names::find_root;
use crate::error::{FmError, R};

// A remembered path that is gone is kept as the start so the health check can report it.
pub fn launch_start(arg: Option<PathBuf>, remembered: Option<PathBuf>, cwd: PathBuf, exe_dir: Option<PathBuf>) -> PathBuf {
    let explicit = arg.filter(|a| !a.to_string_lossy().starts_with("--"));
    explicit.or(remembered).unwrap_or_else(|| match exe_dir {
        Some(dir) if find_root(&cwd).is_none() => dir,
        _ => cwd,
    })
}

fn invalid(text: &str, reason: &str) -> FmError {
    FmError::InvalidName { name: text.to_string(), reason: reason.to_string() }
}

pub fn clean_path(text: &str) -> R<PathBuf> {
    let trimmed = text.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    if trimmed.is_empty() {
        return Err(invalid(text, "the path is empty"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(invalid(text, "the path has control characters"));
    }
    std::path::absolute(trimmed).map_err(|e| invalid(text, &e.to_string()))
}

pub fn storage_root(path: &Path) -> R<PathBuf> {
    let text = path.display().to_string();
    if !path.is_dir() {
        return Err(invalid(&text, "the folder does not exist"));
    }
    find_root(path).ok_or_else(|| FmError::Internal { message: "That folder is not an ANT Storage: it has no 999.Settings folder.".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn the_launch_start_follows_the_resolution_order() {
        let storage = TempDir::new("start-storage");
        storage.mkdir("999.Settings");
        let plain = TempDir::new("start-plain");
        let (s, p) = (storage.path().to_path_buf(), plain.path().to_path_buf());
        let arg = Some(PathBuf::from("C:\\Arg"));
        let mem = Some(PathBuf::from("C:\\Gone"));
        let exe = Some(PathBuf::from("C:\\Exe"));
        let cases = [
            (arg.clone(), mem.clone(), p.clone(), exe.clone(), "C:\\Arg"),
            (Some(PathBuf::from("--flag")), mem.clone(), p.clone(), exe.clone(), "C:\\Gone"),
            (None, mem.clone(), s.clone(), exe.clone(), "C:\\Gone"),
            (None, None, s.clone(), exe.clone(), s.to_str().unwrap()),
            (None, None, p.clone(), exe.clone(), "C:\\Exe"),
            (None, None, p.clone(), None, p.to_str().unwrap()),
            (Some(PathBuf::from("--sync")), None, s.clone(), exe, s.to_str().unwrap()),
        ];
        for (arg, mem, cwd, exe, want) in cases {
            assert_eq!(launch_start(arg, mem, cwd, exe), PathBuf::from(want));
        }
    }

    #[test]
    fn typed_paths_are_cleaned_and_checked() {
        assert!(clean_path("  ").is_err());
        assert!(clean_path("\"\"").is_err());
        assert!(clean_path("C:\\a\u{7}b").is_err());
        let dir = TempDir::new("typed-path");
        let quoted = format!(" \"{}\" ", dir.path().display());
        assert_eq!(clean_path(&quoted).unwrap(), dir.path());
        assert!(clean_path("rel").unwrap().is_absolute());
        assert!(storage_root(&dir.path().join("missing")).is_err());
        let not_storage = storage_root(dir.path()).unwrap_err().to_string();
        assert!(not_storage.contains("no 999.Settings folder"), "{not_storage}");
        dir.mkdir("999.Settings");
        assert_eq!(storage_root(dir.path()).unwrap(), dir.path());
    }
}
