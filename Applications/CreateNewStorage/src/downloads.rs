// Core files and FolderManager: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{CORE_API_URL, DB_NAME, RETIRED_TREE_NAME};
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
    if let Err(e) = core_files(core, progress) {
        progress(Level::Warn, &format!("⚠ Core files download failed: {e}"));
    }
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
