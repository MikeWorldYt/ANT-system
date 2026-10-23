// Name and path checks that keep every change inside the storage and valid on Windows.
use std::fs;
use std::path::{Path, PathBuf};

use super::io_err;
use crate::config::is_hidden_dir;
use crate::error::{FmError, R};

pub(crate) const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
// NTFS limits a name to 255 UTF-16 code units, not bytes.
pub(crate) const MAX_NAME_UNITS: usize = 255;
const RESERVED_DEVICES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn validate_component(name: &str) -> R<()> {
    let reject = |reason: &str| FmError::InvalidName {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    if name.is_empty() {
        return Err(reject("empty name"));
    }
    if name == "." || name == ".." {
        return Err(reject("dot segment"));
    }
    if name.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(reject("forbidden character"));
    }
    if name.chars().any(char::is_control) {
        return Err(reject("control character"));
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(reject("trailing dot or space"));
    }
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED_DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem)) {
        return Err(reject("reserved Windows device name"));
    }
    if name.encode_utf16().count() > MAX_NAME_UNITS {
        return Err(reject("longer than 255 characters"));
    }
    Ok(())
}

pub fn resolve(root: &Path, rel: &[String]) -> R<PathBuf> {
    let invalid = |reason: String| FmError::InvalidPath { rel_parts: rel.to_vec(), reason };
    for part in rel {
        validate_component(part).map_err(|e| match e {
            FmError::InvalidName { name, reason } => invalid(format!("{name}: {reason}")),
            other => other,
        })?;
        // The staging folder holds deleted folders and is never a node, whatever its case.
        if is_hidden_dir(part) {
            return Err(invalid(format!("{part}: reserved folder")));
        }
    }
    let canonical_root = fs::canonicalize(root).map_err(|e| io_err("resolve storage", root, &e))?;
    let mut existing = root.to_path_buf();
    for part in rel {
        let next = existing.join(part);
        match fs::symlink_metadata(&next) {
            Ok(meta) if redirects(&next, &meta)? => {
                return Err(invalid(format!("{part} is a link or junction")));
            }
            Ok(_) => existing = next,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(io_err("inspect path", &next, &e)),
        }
    }
    let canonical = fs::canonicalize(&existing).map_err(|e| io_err("resolve path", &existing, &e))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(invalid("outside the storage".to_string()));
    }
    Ok(join_rel(root, rel))
}

pub(crate) fn join_rel(root: &Path, rel: &[String]) -> PathBuf {
    rel.iter().fold(root.to_path_buf(), |p, part| p.join(part))
}

// Junctions and cloud placeholders are reparse points without being symlinks to std.
pub(crate) fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_type().is_symlink() || meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

// Cloud placeholders (OneDrive) are reparse points that stay in place; links and junctions lead elsewhere.
pub(crate) fn redirects(path: &Path, meta: &fs::Metadata) -> R<bool> {
    if !is_link(meta) {
        return Ok(false);
    }
    if meta.file_type().is_symlink() {
        return Ok(true);
    }
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { return Ok(true) };
    let real = fs::canonicalize(path).map_err(|e| io_err("resolve path", path, &e))?;
    let parent = fs::canonicalize(parent).map_err(|e| io_err("resolve path", parent, &e))?;
    Ok(!same_name(&real.to_string_lossy(), &parent.join(name).to_string_lossy()))
}

// Windows compares names case-insensitively, so this decides what counts as the same folder.
pub(crate) fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree_store::test_support::TempDir;

    fn rel(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn resolve_joins_valid_parts_even_when_missing() {
        let dir = TempDir::new("resolve-ok");
        dir.mkdir("01.A");
        let got = resolve(dir.path(), &rel(&["01.A", "02.Missing"])).unwrap();
        assert_eq!(got, dir.path().join("01.A").join("02.Missing"));
        assert_eq!(resolve(dir.path(), &[]).unwrap(), dir.path());
    }

    #[test]
    fn resolve_refuses_traversal_and_absolute_parts() {
        let dir = TempDir::new("resolve-bad");
        let cases: [&[&str]; 5] = [&["..", "x"], &["01.A", ".."], &["a/b"], &["C:\\Windows"], &["\\\\server"]];
        for bad in cases {
            let err = resolve(dir.path(), &rel(bad)).unwrap_err();
            assert!(matches!(err, FmError::InvalidPath { .. }), "{bad:?} gave {err:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn resolve_refuses_junctions_and_symlinks() {
        let dir = TempDir::new("resolve-link");
        let outside = TempDir::new("resolve-outside");
        let link = dir.path().join("01.Link");
        let made = std::os::windows::fs::symlink_dir(outside.path(), &link).is_ok()
            || std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(outside.path())
                .output()
                .is_ok_and(|o| o.status.success());
        if !made {
            eprintln!("skipped: could not create a symlink or junction");
            return;
        }
        let err = resolve(dir.path(), &rel(&["01.Link", "x"])).unwrap_err();
        assert!(matches!(err, FmError::InvalidPath { .. }), "{err:?}");
        fs::remove_dir(&link).unwrap();
    }

    #[test]
    fn accepts_ordinary_names() {
        assert!(validate_component("01.Math").is_ok());
        assert!(validate_component("a.b").is_ok());
        assert!(validate_component("COM10").is_ok());
        assert!(validate_component("CONSOLE").is_ok());
        assert!(validate_component("LPT0").is_ok());
        assert!(validate_component(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn rejects_empty_and_dot_segments() {
        assert!(validate_component("").is_err());
        assert!(validate_component(".").is_err());
        assert!(validate_component("..").is_err());
    }

    #[test]
    fn rejects_forbidden_and_control_characters() {
        for name in ["a/b", "a\\b", "a:b", "a*", "a?", "a\"", "a<", "a>", "a|", "a\0b", "a\tb"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert!(validate_component("name.").is_err());
        assert!(validate_component("name ").is_err());
    }

    #[test]
    fn rejects_device_names_in_any_case_and_with_extension() {
        for name in ["CON", "con", "Nul.txt", "COM1", "lpt9.log", "aux.tar.gz"] {
            assert!(validate_component(name).is_err(), "{name:?} should be rejected");
        }
    }

    #[test]
    fn name_limit_counts_utf16_units() {
        assert!(validate_component(&"a".repeat(256)).is_err());
        assert!(validate_component(&"é".repeat(255)).is_ok());
        assert!(validate_component(&"\u{1F600}".repeat(127)).is_ok());
        assert!(validate_component(&"\u{1F600}".repeat(128)).is_err());
    }

    #[test]
    fn resolve_refuses_the_trash_folder_in_any_case_and_position() {
        let dir = TempDir::new("resolve-trash");
        dir.mkdir(".ant-trash/x");
        for bad in [&[".ant-trash"][..], &[".ANT-Trash", "x"], &["01.A", ".ant-trash"]] {
            let err = resolve(dir.path(), &rel(bad)).unwrap_err();
            assert!(matches!(err, FmError::InvalidPath { .. }), "{bad:?} gave {err:?}");
        }
    }

    #[test]
    fn plain_folders_do_not_redirect() {
        let dir = TempDir::new("redirect-plain");
        dir.mkdir("a");
        let meta = fs::symlink_metadata(dir.path().join("a")).unwrap();
        assert!(!redirects(&dir.path().join("a"), &meta).unwrap());
    }

    #[cfg(windows)]
    #[test]
    fn junctions_redirect() {
        let dir = TempDir::new("redirect-junction");
        let outside = TempDir::new("redirect-outside");
        let link = dir.path().join("j");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .is_ok_and(|o| o.status.success());
        if !made {
            eprintln!("skipped: could not create a junction");
            return;
        }
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(redirects(&link, &meta).unwrap());
        fs::remove_dir(&link).unwrap();
    }
}
