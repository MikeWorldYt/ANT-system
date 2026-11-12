// Core files and FolderManager: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{
    CORE_BASE_URL, CORE_FILES, FOLDER_MANAGER_ASSET, FOLDER_MANAGER_FALLBACK_TAG, FOLDER_MANAGER_NAME, FOLDER_MANAGER_TAG_PREFIX, RELEASES_API_URL,
    RELEASE_DOWNLOAD_URL,
};
use crate::fetch::{download, download_text, latest_release_tag, MAX_EXE_BYTES, MAX_TEXT_BYTES};
use crate::fs_ops::write_file_atomic;
use crate::Level;

pub fn optional_downloads(root: &Path, ant: &Path, folder_manager: bool, progress: &mut dyn FnMut(Level, &str)) {
    progress(Level::Step, "Downloading ANT Core and utility files ...");
    if let Err(e) = core_files(ant, progress) {
        progress(Level::Warn, &format!("⚠ Core files download failed: {e}"));
    }
    if folder_manager {
        download_folder_manager(root, progress);
    }
}

fn core_files(ant: &Path, progress: &mut dyn FnMut(Level, &str)) -> Result<(), String> {
    for name in CORE_FILES {
        let dest = ant.join(name);
        download(&format!("{CORE_BASE_URL}/{name}"), MAX_TEXT_BYTES, 60).and_then(|b| write_file_atomic(&dest, &b))?;
        progress(Level::Ok, &format!("✔ {} (downloaded)", dest.display()));
    }
    Ok(())
}

fn latest_folder_manager_tag() -> Option<String> {
    let listing = download_text(RELEASES_API_URL).ok()?;
    latest_release_tag(&listing, FOLDER_MANAGER_TAG_PREFIX)
}

// The newest FolderManager release asset goes into the storage root.
fn download_folder_manager(root: &Path, progress: &mut dyn FnMut(Level, &str)) {
    let tag = latest_folder_manager_tag().unwrap_or_else(|| {
        progress(Level::Warn, &format!("⚠ Could not find the latest FolderManager release; using {FOLDER_MANAGER_FALLBACK_TAG}"));
        FOLDER_MANAGER_FALLBACK_TAG.to_string()
    });
    let url = format!("{RELEASE_DOWNLOAD_URL}/{tag}/{FOLDER_MANAGER_ASSET}");
    let exe = root.join(FOLDER_MANAGER_NAME);
    let fetched = download(&url, MAX_EXE_BYTES, 300).and_then(|b| install_folder_manager(&exe, &b)).and_then(|()| make_executable(&exe));
    match fetched {
        Ok(()) => progress(Level::Ok, &format!("✔ {} (downloaded)", exe.display())),
        Err(e) => progress(Level::Warn, &format!("⚠ FolderManager download failed: {e} (asset {FOLDER_MANAGER_ASSET} of {tag})")),
    }
}

#[cfg(windows)]
fn install_folder_manager(exe: &Path, asset: &[u8]) -> Result<(), String> {
    write_file_atomic(exe, asset)
}

#[cfg(not(windows))]
fn install_folder_manager(exe: &Path, asset: &[u8]) -> Result<(), String> {
    let archive = exe.with_extension("tar.gz");
    write_file_atomic(&archive, asset)?;
    let unpacked = std::process::Command::new("tar").arg("-xzOf").arg(&archive).arg(crate::config::FOLDER_MANAGER_MEMBER).output();
    let removed = std::fs::remove_file(&archive);
    let unpacked = unpacked.map_err(|e| format!("Could not run tar: {e}"))?;
    if !unpacked.status.success() {
        return Err(format!("Could not unpack {FOLDER_MANAGER_ASSET}: {}", String::from_utf8_lossy(&unpacked.stderr).trim()));
    }
    removed.map_err(|e| format!("Could not remove {}: {e}", archive.display()))?;
    write_file_atomic(exe, &unpacked.stdout)
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
