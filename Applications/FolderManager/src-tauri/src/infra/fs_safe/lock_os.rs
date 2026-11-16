// Platform side of the storage lock: an owner handle others cannot delete, its release, and whether a PID still runs.
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[cfg(windows)]
pub(super) fn create_owned(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
    use windows_sys::Win32::Storage::FileSystem::{DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};
    // Without FILE_SHARE_DELETE no other process, older builds and Explorer included, can delete the file while this handle is open.
    OpenOptions::new().write(true).create_new(true).access_mode(GENERIC_READ | GENERIC_WRITE | DELETE).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(path)
}

#[cfg(not(windows))]
pub(super) fn create_owned(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).write(true).create_new(true).open(path)
}

// Only this handle can delete the file, so it names this owner's file with no path to race on.
#[cfg(windows)]
pub(super) fn release(file: &File, _path: &Path, _stamp: &str) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{FileDispositionInfo, SetFileInformationByHandle, FILE_DISPOSITION_INFO};
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: the handle stays open for the call, and the pointer and size describe `info`.
    let ok = unsafe { SetFileInformationByHandle(file.as_raw_handle(), FileDispositionInfo, (&raw const info).cast(), size_of::<FILE_DISPOSITION_INFO>() as u32) };
    if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

// Only a file still holding this owner's stamp is removed, while the flock keeps any Refresh from taking it.
#[cfg(not(windows))]
pub(super) fn release(_file: &File, path: &Path, stamp: &str) -> io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(text) if text == stamp => std::fs::remove_file(path),
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(windows)]
pub(super) fn pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the handle is checked before use and closed before returning.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return io::Error::last_os_error().raw_os_error() == Some(ERROR_ACCESS_DENIED as i32);
        }
        let mut code = 0;
        let alive = GetExitCodeProcess(handle, &mut code) == 0 || code == STILL_ACTIVE as u32;
        CloseHandle(handle);
        alive
    }
}

// `kill -0` sends no signal; when it cannot run, the owner is taken as alive so its lock is kept.
#[cfg(not(windows))]
pub(super) fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill").args(["-0", &pid.to_string()]).stderr(std::process::Stdio::null()).status().map_or(true, |s| s.success())
}
