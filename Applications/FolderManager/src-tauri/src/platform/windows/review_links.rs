// 998.Review links are .lnk files on Windows: their path and their removal.
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub(crate) fn with_lnk(link: &Path) -> PathBuf {
    let mut path = link.as_os_str().to_owned();
    path.push(".lnk");
    PathBuf::from(path)
}

// Mirrors the full rebuild's clearing rule: on Windows any .lnk file is ours.
pub(crate) fn remove_owned(_root: &Path, _review_dir: &Path, link: &Path) -> Result<(), String> {
    let file = with_lnk(link);
    match fs::symlink_metadata(&file) {
        Ok(meta) if meta.is_file() => fs::remove_file(&file).map_err(|e| format!("Could not remove {}: {e}", file.display())),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Could not inspect {}: {e}", file.display())),
    }
}
