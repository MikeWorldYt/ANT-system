// Guide, core files and FolderManager: helpful but not essential, so failures only warn.
use std::path::Path;

use crate::config::{CORE_API_URL, DOCS_URL, README_NAME, README_URL};
use crate::fetch::{core_file_list, download, download_text, MAX_TEXT_BYTES};
use crate::fs_ops::write_file_atomic;
use crate::Level;

pub fn optional_downloads(root: &Path, core: &Path, progress: &mut dyn FnMut(Level, &str)) {
    progress(Level::Step, "Downloading ANT Core and utility files ...");
    let readme = root.join(README_NAME);
    let fetched = download(README_URL, MAX_TEXT_BYTES, 30).and_then(|b| write_file_atomic(&readme, &b));
    match fetched {
        Ok(()) => progress(Level::Ok, &format!("✔ {} (downloaded)", readme.display())),
        Err(e) => {
            let text = format!("# ANT System — Get Started\n\nCould not download the guide automatically.\nPlease visit:\n\n  {DOCS_URL}\n\nError: {e}\n");
            match write_file_atomic(&readme, text.as_bytes()) {
                Ok(()) => progress(Level::Warn, "⚠ Guide download failed. Placeholder created."),
                Err(w) => progress(Level::Warn, &format!("⚠ Guide download failed ({e}) and no placeholder: {w}")),
            }
        }
    }
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

fn core_files(core: &Path, progress: &mut dyn FnMut(Level, &str)) -> Result<(), String> {
    for (name, url) in core_file_list(&download_text(CORE_API_URL)?)? {
        let dest = core.join(&name);
        download(&url, MAX_TEXT_BYTES, 60).and_then(|b| write_file_atomic(&dest, &b))?;
        progress(Level::Ok, &format!("✔ {} (downloaded)", dest.display()));
    }
    Ok(())
}
