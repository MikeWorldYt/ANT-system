// Core files and FolderManager: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{
    CORE_API_URL, DB_NAME, FOLDER_MANAGER_FALLBACK_TAG, FOLDER_MANAGER_NAME, FOLDER_MANAGER_TAG_PREFIX, RELEASES_API_URL,
    RELEASE_DOWNLOAD_URL, RETIRED_TREE_NAME,
};
use crate::fetch::{core_file_list, download, download_text, latest_release_tag, MAX_EXE_BYTES, MAX_TEXT_BYTES};
use crate::fs_ops::write_file_atomic;
use crate::Level;

pub fn optional_downloads(root: &Path, core: &Path, progress: &mut dyn FnMut(Level, &str)) {
    progress(Level::Step, "Downloading ANT Core and utility files ...");
    if let Err(e) = core_files(core, progress) {
        progress(Level::Warn, &format!("⚠ Core files download failed: {e}"));
    }
    folder_manager(root, progress);
}

// The template database is already in place from the essential step, and the retired Tree.yaml is not copied.
fn core_files(core: &Path, progress: &mut dyn FnMut(Level, &str)) -> Result<(), String> {
    for (name, url) in core_file_list(&download_text(CORE_API_URL)?)? {
        if name == DB_NAME || name == RETIRED_TREE_NAME {
            continue;
        }
        let dest = core.join(&name);
        download(&url, MAX_TEXT_BYTES, 60).and_then(|b| write_file_atomic(&dest, &b))?;
        progress(Level::Ok, &format!("✔ {} (downloaded)", dest.display()));
    }
    Ok(())
}

fn latest_folder_manager_tag() -> Option<String> {
    let listing = download_text(RELEASES_API_URL).ok()?;
    latest_release_tag(&listing, FOLDER_MANAGER_TAG_PREFIX)
}

// The newest FolderManager release asset goes into the storage root.
fn folder_manager(root: &Path, progress: &mut dyn FnMut(Level, &str)) {
    let tag = latest_folder_manager_tag().unwrap_or_else(|| {
        progress(Level::Warn, &format!("⚠ Could not find the latest FolderManager release; using {FOLDER_MANAGER_FALLBACK_TAG}"));
        FOLDER_MANAGER_FALLBACK_TAG.to_string()
    });
    let url = format!("{RELEASE_DOWNLOAD_URL}/{tag}/{FOLDER_MANAGER_NAME}");
    let exe = root.join(FOLDER_MANAGER_NAME);
    let fetched = download(&url, MAX_EXE_BYTES, 300).and_then(|b| write_file_atomic(&exe, &b)).and_then(|()| make_executable(&exe));
    match fetched {
        Ok(()) => progress(Level::Ok, &format!("✔ {} (downloaded)", exe.display())),
        Err(e) => progress(Level::Warn, &format!("⚠ FolderManager download failed: {e} (asset {FOLDER_MANAGER_NAME} of {tag})")),
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("Could not make {} executable: {e}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}
