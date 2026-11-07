// Sends a deleted folder or file to the Recycle Bin from where it sits, so a restore puts it back in place.
use std::path::{Component, Path, PathBuf, Prefix};

// SHFileOperationW reports failed or cancelled recycles; flags 0x4454 (ALLOWUNDO|NOCONFIRMATION|WANTNUKEWARNING|NOERRORUI|SILENT) leave only the permanent-delete warning.
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

const RECYCLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

// The shell and DriveInfo refuse \\?\ paths, and without the prefix Windows rewrites a name like "notes." into what may be another file.
fn shell_path(path: &Path) -> Result<PathBuf, String> {
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else { return Ok(path.to_path_buf()) };
    let mut plain = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(host, share) => {
            let mut root = std::ffi::OsString::from(r"\\");
            root.push(host);
            root.push(r"\");
            root.push(share);
            PathBuf::from(root)
        }
        _ => return Ok(path.to_path_buf()),
    };
    plain.extend(parts);
    match std::path::absolute(&plain) {
        Ok(full) if full == plain => Ok(plain),
        _ => Err(format!("{} cannot be sent to the Recycle Bin", path.display())),
    }
}

pub(crate) fn send_to_recycle_bin(path: &Path) -> Result<(), String> {
    let path = shell_path(path)?;
    let env = [("ANT_RECYCLE_PATH", path.as_os_str()), ("ANT_RECYCLE_TYPE", std::ffi::OsStr::new(RECYCLE_TYPE))];
    crate::platform::windows::powershell::run_powershell(RECYCLE_SCRIPT, &env, None, RECYCLE_TIMEOUT).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(path: &str) -> String {
        shell_path(Path::new(path)).unwrap().display().to_string()
    }

    #[test]
    fn the_shell_gets_a_path_without_the_verbatim_prefix() {
        assert_eq!(plain(r"\\?\D:\Work\notes.txt"), r"D:\Work\notes.txt");
        assert_eq!(plain(r"\\?\D:\"), r"D:\");
        assert_eq!(plain(r"\\?\UNC\host\share\Work\notes.txt"), r"\\host\share\Work\notes.txt");
        assert_eq!(plain(r"\\host\share\Work\notes.txt"), r"\\host\share\Work\notes.txt");
        assert_eq!(plain(r"D:\Work\notes.txt"), r"D:\Work\notes.txt");
    }

    #[test]
    fn a_name_the_shell_would_rewrite_is_refused() {
        for path in [r"\\?\D:\Work\notes.", r"\\?\D:\Work\notes ", r"\\?\D:\Work\..\notes.txt"] {
            assert!(shell_path(Path::new(path)).is_err(), "{path}");
        }
    }
}
