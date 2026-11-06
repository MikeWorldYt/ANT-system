// Creates the 998.Review shortcuts and reads them back: .lnk files through the Windows shell, symlinks on other systems.
use std::path::PathBuf;

// Pairs are (target folder, link path without extension); Windows gets `.lnk` files, other systems symlinks.
pub fn create_shortcuts(pairs: &[(PathBuf, PathBuf)]) -> Vec<String> {
    if pairs.is_empty() {
        return Vec::new();
    }
    platform_create(pairs)
}

// One target per link, in order; empty when a link cannot be read.
pub fn shortcut_targets(links: &[PathBuf]) -> Result<Vec<String>, String> {
    platform_targets(links)
}

#[cfg(windows)]
use crate::platform::windows::shortcuts::{platform_create, platform_targets};

// Links are symlinks here, so no .lnk file is the app's.
#[cfg(not(windows))]
fn platform_targets(links: &[PathBuf]) -> Result<Vec<String>, String> {
    Ok(vec![String::new(); links.len()])
}

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
