// Creates the 998.Review shortcuts: .lnk files through the Windows shell, symlinks on other systems.
use std::path::PathBuf;

// Pairs are (target folder, link path without extension); Windows gets `.lnk` files, other systems symlinks.
pub fn create_shortcuts(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    if pairs.is_empty() {
        return Vec::new();
    }
    platform_create(pairs)
}

#[cfg(windows)]
use crate::platform::windows::shortcuts::platform_create;

#[cfg(not(windows))]
fn platform_create(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    let mut failures = Vec::new();
    for (target, link) in pairs {
        if std::fs::symlink_metadata(link).is_ok() {
            continue;
        }
        if let Err(e) = std::os::unix::fs::symlink(target, link) {
            failures.push(format!("Shortcut not created: {}: {e}", link.display()));
        }
    }
    failures
}
