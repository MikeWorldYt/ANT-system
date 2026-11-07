// The Recycle Bin the delete commands pass to the mutations: the Windows shell's, or the desktop trash elsewhere.
#[cfg(not(windows))]
use std::path::Path;

#[cfg(windows)]
pub(crate) use crate::platform::windows::recycle::send_to_recycle_bin;

#[cfg(not(windows))]
pub(crate) fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    super::unix_trash::send(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::mutations::{delete_file, delete_node};
    use crate::test_support::*;
    use std::fs;

    #[cfg(windows)]
    #[test]
    #[ignore = "starts PowerShell"]
    fn recycle_reports_a_missing_folder_as_an_error() {
        let dir = work("recycle-missing");
        let err = send_to_recycle_bin(&dir.path().join("does-not-exist")).unwrap_err();
        assert!(err.contains("PowerShell failed") && err.contains("does-not-exist"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "sends a temp file to the real Recycle Bin through PowerShell"]
    fn delete_sends_the_file_to_the_recycle_bin() {
        let dir = work("delete-real-file");
        let file = dir.path().join("100.WORK/loose.txt");
        fs::write(&file, "x").unwrap();
        let m = delete_file(dir.path(), &rel(&["100.WORK", "loose.txt"]), send_to_recycle_bin).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        assert!(!file.exists());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "sends a temp folder to the real Recycle Bin through PowerShell"]
    fn delete_sends_the_folder_to_the_recycle_bin() {
        let dir = work("delete-real");
        let m = delete_node(dir.path(), &rel(&["100.WORK", "02.B"]), false, false, send_to_recycle_bin).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        assert!(!dir.path().join("100.WORK/02.B").exists());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "sends a temp file to the real Recycle Bin through PowerShell"]
    fn a_canonical_path_reaches_the_recycle_bin() {
        let dir = work("recycle-canonical");
        let file = dir.path().join("100.WORK/loose.txt");
        fs::write(&file, "x").unwrap();
        let canonical = fs::canonicalize(&file).unwrap();
        assert!(canonical.to_string_lossy().starts_with(r"\\?\"), "{}", canonical.display());
        send_to_recycle_bin(&canonical).unwrap();
        assert!(!file.exists());
    }
}
