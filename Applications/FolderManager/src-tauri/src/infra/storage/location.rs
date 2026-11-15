// Where the Storage is looked for at launch and which typed paths are accepted as a new Storage.
use std::path::{Path, PathBuf};

use crate::infra::scan::find_root;
use crate::domain::error::{FmError, R};

// A portable copy inside a Storage opens it before the remembered one; a remembered path that is gone is kept so the picker can report it.
pub fn launch_start(arg: Option<PathBuf>, remembered: Option<PathBuf>, cwd: PathBuf, exe_dir: Option<PathBuf>) -> PathBuf {
    let explicit = arg.filter(|a| !a.to_string_lossy().starts_with("--"));
    let portable = exe_dir.clone().filter(|d| find_root(d).is_some());
    explicit.or(portable).or(remembered).unwrap_or_else(|| match exe_dir {
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
    find_root(path).ok_or_else(|| FmError::Internal { message: "That folder is not an ANT Storage: it has no .ant folder.".to_string() })
}

pub fn check_name(name: &str) -> R<&str> {
    let name = name.trim();
    let bad = name.is_empty() || name.ends_with('.') || name.chars().any(|c| c.is_control() || r#"<>:"/\|?*"#.contains(c));
    if bad {
        return Err(invalid(name, "use a folder name without < > : \" / \\ | ? * and not ending in a dot"));
    }
    Ok(name)
}

// An empty Storage is only its folder and .ant/; the scan that follows builds the database.
pub fn create(parent: &Path, name: &str) -> R<PathBuf> {
    let name = check_name(name)?;
    if !parent.is_dir() {
        return Err(invalid(&parent.display().to_string(), "the parent folder does not exist"));
    }
    let root = parent.join(name);
    if root.exists() {
        return Err(invalid(name, "a folder with that name already exists there"));
    }
    std::fs::create_dir_all(root.join(crate::domain::config::DATA_DIR)).map_err(|e| crate::infra::fs_safe::io_err("create the Storage", &root, &e))?;
    Ok(root)
}

// Rename and move of a listed Storage: a plain rename, so a move to another drive is refused instead of copied.
pub fn relocate(from: &Path, to: &Path) -> R<()> {
    if storage_root(from)? != from {
        return Err(invalid(&from.display().to_string(), "the folder is not a Storage root"));
    }
    if to.exists() {
        return Err(invalid(&to.display().to_string(), "a folder with that name already exists there"));
    }
    let same_drive = from.components().next() == to.components().next();
    if !same_drive {
        return Err(invalid(&to.display().to_string(), "moving a Storage to another drive is not supported"));
    }
    std::fs::rename(from, to).map_err(|e| crate::infra::fs_safe::io_err("move the Storage", from, &e))
}

// Windows users only pick a drive; the system drive maps to the home folder, where the app can read and write.
#[cfg(windows)]
pub fn locations(home: Option<PathBuf>) -> Vec<(String, PathBuf)> {
    let system = home.as_ref().and_then(|h| h.components().next()).map(|c| c.as_os_str().to_string_lossy().to_uppercase());
    (b'A'..=b'Z')
        .map(|l| format!("{}:", l as char))
        .filter(|d| Path::new(&format!("{d}\\")).is_dir())
        .map(|d| match &home {
            Some(h) if system.as_deref() == Some(d.as_str()) => (format!("{d} (your user folder)"), h.clone()),
            _ => (d.clone(), PathBuf::from(format!("{d}\\"))),
        })
        .collect()
}

#[cfg(not(windows))]
pub fn locations(_home: Option<PathBuf>) -> Vec<(String, PathBuf)> {
    Vec::new()
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
            (None, mem.clone(), p.clone(), Some(s.clone()), s.to_str().unwrap()),
            (arg.clone(), mem.clone(), p.clone(), Some(s.clone()), "C:\\Arg"),
        ];
        for (arg, mem, cwd, exe, want) in cases {
            assert_eq!(launch_start(arg, mem, cwd, exe), PathBuf::from(want));
        }
    }

    #[test]
    fn an_empty_storage_is_created_once() {
        let dir = TempDir::new("create-storage");
        assert!(create(dir.path(), "a/b").is_err());
        assert!(create(dir.path(), " ").is_err());
        assert!(create(&dir.path().join("missing"), "S").is_err());
        let root = create(dir.path(), " My Storage ").unwrap();
        assert_eq!(find_root(&root).as_deref(), Some(root.as_path()));
        assert!(create(dir.path(), "My Storage").is_err());
    }

    #[test]
    fn a_storage_is_renamed_on_the_same_drive_only() {
        let dir = TempDir::new("relocate");
        let root = create(dir.path(), "S").unwrap();
        assert!(relocate(&dir.path().join("missing"), &dir.path().join("T")).is_err());
        dir.mkdir("Taken");
        assert!(relocate(&root, &dir.path().join("Taken")).is_err());
        relocate(&root, &dir.path().join("T")).unwrap();
        assert!(find_root(&dir.path().join("T")).is_some());
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
        assert!(not_storage.contains("no .ant folder"), "{not_storage}");
        dir.mkdir("999.Settings");
        assert_eq!(storage_root(dir.path()).unwrap(), dir.path());
    }
}
