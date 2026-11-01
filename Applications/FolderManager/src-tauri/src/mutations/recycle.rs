// Sends a deleted folder or file to the Recycle Bin from where it sits, so a restore puts it back in place.
use std::path::Path;

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
        if (-not (Test-Path -LiteralPath $path)) { throw ('The item is missing: ' + $path) }; \
        Add-Type -TypeDefinition $env:ANT_RECYCLE_TYPE; \
        $failure = [AntRecycleBin]::Send($path); \
        if ($failure) { throw $failure }; \
        if (Test-Path -LiteralPath $path) { throw 'The folder is still there' }; \
        exit 0 \
    } catch { $out.WriteLine($_.Exception.Message); $out.Flush(); exit 1 }";

#[cfg(windows)]
const RECYCLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

// A failed recycle leaves the item where it is, so the caller changes nothing else.
pub(crate) fn move_to_bin(path: &Path, recycle: impl FnOnce(&Path) -> Result<(), String>) -> crate::error::R<()> {
    recycle(path).map_err(|message| crate::error::FmError::Io {
        op: "send to the Recycle Bin".to_string(),
        path: path.display().to_string(),
        message,
        os_code: None,
    })
}

#[cfg(windows)]
pub(crate) fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    let env = [("ANT_RECYCLE_PATH", path.as_os_str()), ("ANT_RECYCLE_TYPE", std::ffi::OsStr::new(RECYCLE_TYPE))];
    crate::fs_safe::run_powershell(RECYCLE_SCRIPT, &env, None, RECYCLE_TIMEOUT).map(|_| ())
}

#[cfg(not(windows))]
pub(crate) fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    super::unix_trash::send(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutations::{delete_file, delete_node};
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
}
