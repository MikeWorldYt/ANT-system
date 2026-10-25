// The Recycle Bin is reached outside the storage lock, so staged folders are tracked until it takes them.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::ant_core::names::list_subdirs;
use crate::config::TRASH_DIR;
use crate::error::R;
use crate::fs_safe::{io_err, redirects, same_name, validate_component};

// A folder already staged in .ant-trash and dropped from the tree; recycling it needs no storage lock.
pub struct PendingRecycle {
    pub(super) root: PathBuf,
    pub(super) staged: PathBuf,
    pub(super) name: String,
}

// Staged folders still on their way to the Recycle Bin; anything else in .ant-trash was restored or kept.
pub(super) static IN_FLIGHT: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

struct InFlight(PathBuf);

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).retain(|p| *p != self.0);
    }
}

// The Recycle Bin restores a staged folder into .ant-trash, where it is hidden, so refresh moves it to the storage root under its own name.
pub fn recover_trashed(root: &Path) -> R<Vec<String>> {
    let trash = root.join(TRASH_DIR);
    let entries = match fs::read_dir(&trash) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io_err("list folder", &trash, &e)),
    };
    let mut recovered = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| io_err("list folder", &trash, &e))?.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| io_err("inspect path", &path, &e))?;
        let in_flight = IN_FLIGHT.lock().unwrap_or_else(|p| p.into_inner()).contains(&path);
        if in_flight || !meta.is_dir() || redirects(&path, &meta)? {
            continue;
        }
        let Some(original) = path.file_name().and_then(|n| n.to_str()).and_then(original_name) else { continue };
        let taken = list_subdirs(root)?;
        let free = (0..100)
            .map(|n| if n == 0 { original.to_string() } else { format!("{original}_{n}") })
            .find(|c| !taken.iter().any(|t| same_name(t, c)) && validate_component(c).is_ok());
        let Some(target) = free else { continue };
        fs::rename(&path, root.join(&target)).map_err(|e| io_err("restore folder", &path, &e))?;
        recovered.push(target);
    }
    remove_trash_if_empty(root, &mut Vec::new());
    Ok(recovered)
}

// Inverse of `trash_name`: drops "<stamp>-" and the optional "<n>-" counter.
fn original_name(staged: &str) -> Option<&str> {
    let (stamp, rest) = staged.split_once('-')?;
    if stamp.is_empty() || !stamp.bytes().all(|b| b.is_ascii_digit() || b.is_ascii_lowercase()) {
        return None;
    }
    let after_counter = rest.split_once('-').filter(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    Some(after_counter.map_or(rest, |(_, name)| name)).filter(|n| !n.is_empty())
}

// Returns warnings for the Mutation; on any doubt the staged copy stays in .ant-trash.
pub fn recycle_staged(pending: PendingRecycle) -> Vec<String> {
    recycle_staged_with(pending, send_to_recycle_bin)
}

pub(crate) fn recycle_staged_with(
    pending: PendingRecycle,
    recycle: impl FnOnce(&Path) -> Result<(), String>,
) -> Vec<String> {
    let PendingRecycle { root, staged, name } = pending;
    let _in_flight = InFlight(staged.clone());
    let kept =|why: &str| {
        vec![format!("{name} left the tree but was not sent to the Recycle Bin; it is kept in {}: {why}", staged.display())]
    };
    if let Err(why) = check_staged(&root, &staged) {
        return kept(&why);
    }
    match recycle(&staged) {
        Err(why) => kept(&why),
        Ok(()) if fs::symlink_metadata(&staged).is_ok() => kept("the folder is still there"),
        Ok(()) => {
            let mut warnings = Vec::new();
            remove_trash_if_empty(&root, &mut warnings);
            warnings
        }
    }
}

// The folder sat unlocked since staging, so it must still be a plain folder directly inside .ant-trash.
fn check_staged(root: &Path, staged: &Path) -> Result<(), String> {
    if staged.parent() != Some(root.join(TRASH_DIR).as_path()) {
        return Err("it is not inside the staging folder".to_string());
    }
    let meta = fs::symlink_metadata(staged).map_err(|e| e.to_string())?;
    match redirects(staged, &meta) {
        Ok(false) if meta.is_dir() => Ok(()),
        Ok(_) => Err("it is no longer a plain folder".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn remove_trash_if_empty(root: &Path, warnings: &mut Vec<String>) {
    let trash = root.join(TRASH_DIR);
    let empty = fs::read_dir(&trash).map(|mut entries| entries.next().is_none());
    match empty {
        Ok(true) => {
            if let Err(e) = fs::remove_dir(&trash) {
                warnings.push(format!("Could not remove {}: {e}", trash.display()));
            }
        }
        Ok(false) => {}
        Err(e) => warnings.push(format!("Could not inspect {}: {e}", trash.display())),
    }
}

// SHFileOperationW reports failed or cancelled recycles; flags 0x4454 (ALLOWUNDO|NOCONFIRMATION|WANTNUKEWARNING|NOERRORUI|SILENT) leave only the permanent-delete warning.
#[cfg(windows)]
const RECYCLE_TYPE: &str = r#"
using System;
using System.Runtime.InteropServices;
public static class AntRecycleBin {
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct SHFILEOPSTRUCTW {
        public IntPtr hwnd;
        public uint wFunc;
        public string pFrom;
        public string pTo;
        public ushort fFlags;
        [MarshalAs(UnmanagedType.Bool)] public bool fAnyOperationsAborted;
        public IntPtr hNameMappings;
        public string lpszProgressTitle;
    }
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHFileOperationW(ref SHFILEOPSTRUCTW op);
    public static string Send(string path) {
        if (IntPtr.Size != 8) { return "only 64-bit PowerShell is supported"; }
        SHFILEOPSTRUCTW op = new SHFILEOPSTRUCTW();
        op.wFunc = 3;
        op.pFrom = path + "\0";
        op.fFlags = 0x4454;
        int code = SHFileOperationW(ref op);
        if (code != 0) { return "SHFileOperation failed with code 0x" + code.ToString("X"); }
        if (op.fAnyOperationsAborted) { return "the operation was cancelled"; }
        return "";
    }
}
"#;

// Removable and network drives have no Recycle Bin, where Windows would delete without asking.
#[cfg(windows)]
const RECYCLE_SCRIPT: &str = "$ErrorActionPreference = 'Stop'; \
    $out = [IO.StreamWriter]::new([Console]::OpenStandardOutput(), [Text.UTF8Encoding]::new($false)); \
    try { \
        $path = $env:ANT_RECYCLE_PATH; \
        if ([IO.DriveInfo]::new([IO.Path]::GetPathRoot($path)).DriveType -ne 'Fixed') { throw 'This drive has no Recycle Bin' }; \
        if (-not (Test-Path -LiteralPath $path -PathType Container)) { throw ('The folder is missing: ' + $path) }; \
        Add-Type -TypeDefinition $env:ANT_RECYCLE_TYPE; \
        $failure = [AntRecycleBin]::Send($path); \
        if ($failure) { throw $failure }; \
        if (Test-Path -LiteralPath $path) { throw 'The folder is still there' }; \
        exit 0 \
    } catch { $out.WriteLine($_.Exception.Message); $out.Flush(); exit 1 }";

#[cfg(windows)]
const RECYCLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

#[cfg(windows)]
fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    let env = [("ANT_RECYCLE_PATH", path.as_os_str()), ("ANT_RECYCLE_TYPE", std::ffi::OsStr::new(RECYCLE_TYPE))];
    crate::fs_safe::run_powershell(RECYCLE_SCRIPT, &env, None, RECYCLE_TIMEOUT).map(|_| ())
}

#[cfg(not(windows))]
fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    super::unix_trash::send(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_safe::StorageLock;
    use crate::mutations::delete::trash_name;
    use crate::mutations::delete_node;
    use crate::test_support::*;
    use crate::tree::tree_store::get_node;

    #[test]
    fn recycle_runs_without_the_storage_lock_after_the_yaml_commit() {
        let dir = work("recycle-unlocked");
        let p = rel(&["100.WORK", "02.B"]);
        let m = delete_with(dir.path(), &p, false, false, |staged| {
            assert!(get_node(&yaml(dir.path()), &rel(&["100.WORK", "02.B"])).is_none());
            drop(StorageLock::acquire(dir.path()).map_err(|e| e.to_string())?);
            ok_recycle(staged)
        })
        .unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    }

    #[test]
    fn recycle_that_leaves_the_folder_in_place_keeps_it_with_a_warning() {
        let dir = work("recycle-noop");
        let m = delete_with(dir.path(), &rel(&["100.WORK", "02.B"]), false, false, |_| Ok(())).unwrap();
        assert_eq!(m.warnings.len(), 1, "{:?}", m.warnings);
        assert!(m.warnings[0].contains("still there"), "{:?}", m.warnings);
        assert_eq!(names(&dir.path().join(TRASH_DIR)).len(), 1);
    }

    #[test]
    fn recycle_refuses_a_path_outside_the_staging_folder() {
        let dir = work("recycle-outside");
        let staged = dir.path().join("100.WORK/02.B");
        let pending = PendingRecycle { root: dir.path().to_path_buf(), staged, name: "02.B".to_string() };
        let warnings = recycle_staged_with(pending, |_| panic!("must not recycle"));
        assert_eq!(warnings.len(), 1);
        assert!(dir.path().join("100.WORK/02.B").is_dir());
    }

    #[test]
    fn original_name_inverts_trash_name() {
        for (name, n) in [("01.A", 0), ("01.A", 3), ("02.Plan-b", 0)] {
            assert_eq!(original_name(&trash_name("lz4k9", n, name)), Some(name));
        }
        assert_eq!(original_name("nodash"), None);
        assert_eq!(original_name("Bad!-01.A"), None);
    }

    #[test]
    fn restored_folders_come_back_to_the_root_but_in_flight_ones_stay() {
        let dir = work("recover");
        let p = rel(&["100.WORK", "02.B"]);
        fs::write(dir.path().join("100.WORK/02.B/n.txt"), "x").unwrap();
        let (_, pending) = delete_node(dir.path(), &p, false, true).unwrap();
        assert!(recover_trashed(dir.path()).unwrap().is_empty());
        assert_eq!(names(&dir.path().join(TRASH_DIR)).len(), 1);
        // Simulates the Recycle Bin restoring the folder under its staged name.
        let staged = names(&dir.path().join(TRASH_DIR)).remove(0);
        drop(recycle_staged_with(pending.unwrap(), |_| Ok(())));
        fs::create_dir(dir.path().join("02.B")).unwrap();
        assert_eq!(recover_trashed(dir.path()).unwrap(), ["02.B_1"]);
        assert!(staged.ends_with("-02.B"));
        assert_eq!(fs::read_to_string(dir.path().join("02.B_1/n.txt")).unwrap(), "x");
        assert!(!dir.path().join(TRASH_DIR).exists());
    }

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
    #[ignore = "sends a temp folder to the real Recycle Bin through PowerShell"]
    fn delete_sends_the_staged_folder_to_the_recycle_bin() {
        let dir = work("delete-real");
        let (m, pending) = delete_node(dir.path(), &rel(&["100.WORK", "02.B"]), false, false).unwrap();
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        let warnings = recycle_staged(pending.unwrap());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(!dir.path().join(TRASH_DIR).exists());
    }
}
