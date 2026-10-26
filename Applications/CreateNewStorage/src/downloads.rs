// Core files and FolderManager: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{ANT_DIR, CORE_API_URL, DB_NAME};
use crate::fetch::{core_file_list, download, download_text, MAX_TEXT_BYTES};
use crate::fs_ops::write_file_atomic;
use crate::Level;

pub fn optional_downloads(root: &Path, core: &Path, progress: &mut dyn FnMut(Level, &str)) {
    progress(Level::Step, "Downloading ANT Core and utility files ...");
    #[cfg(windows)]
    {
        let exe = root.join(crate::config::FOLDER_MANAGER_NAME);
        let fetched = download(crate::config::FOLDER_MANAGER_URL, crate::fetch::MAX_EXE_BYTES, 300).and_then(|b| write_file_atomic(&exe, &b));
        match fetched {
            Ok(()) => progress(Level::Ok, &format!("✔ {} (downloaded)", exe.display())),
            Err(e) => progress(Level::Warn, &format!("⚠ FolderManager download failed: {e}")),
        }
    }
    if let Err(e) = core_files(root, core, progress) {
        progress(Level::Warn, &format!("⚠ Core files download failed: {e}"));
    }
}

fn core_files(root: &Path, core: &Path, progress: &mut dyn FnMut(Level, &str)) -> Result<(), String> {
    for (name, url) in core_file_list(&download_text(CORE_API_URL)?)? {
        // The database is not a setting: the app reads it from .ant/, next to the storage's own state.
        let dest = if name == DB_NAME { root.join(ANT_DIR).join(&name) } else { core.join(&name) };
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
        }
        download(&url, MAX_TEXT_BYTES, 60).and_then(|b| write_file_atomic(&dest, &b))?;
        progress(Level::Ok, &format!("✔ {} (downloaded)", dest.display()));
    }
    Ok(())
}
