// Win32 read of the CF_HDROP file list and the "Preferred DropEffect" Explorer puts beside it.
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::thread::sleep;
use std::time::Duration;

use windows_sys::w;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows_sys::Win32::System::Ole::CF_HDROP;
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};

use super::ClipboardFiles;
use crate::error::{FmError, R};

// Another program can hold the clipboard for a moment, so opening it is retried briefly.
const OPEN_TRIES: u32 = 10;
const OPEN_WAIT: Duration = Duration::from_millis(30);

pub(super) struct OpenClipboardGuard;

impl OpenClipboardGuard {
    pub(super) fn acquire() -> R<Self> {
        for _ in 0..OPEN_TRIES {
            // SAFETY: a null owner opens the clipboard for this thread until CloseClipboard.
            if unsafe { OpenClipboard(null_mut()) } != 0 {
                return Ok(Self);
            }
            sleep(OPEN_WAIT);
        }
        Err(failure("The clipboard is in use by another program; try again"))
    }
}

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: the guard exists only after OpenClipboard succeeded on this thread.
        if unsafe { CloseClipboard() } == 0 {
            eprintln!("Could not close the clipboard: {}", std::io::Error::last_os_error());
        }
    }
}

pub fn read() -> R<ClipboardFiles> {
    let _open = OpenClipboardGuard::acquire()?;
    Ok(ClipboardFiles { paths: file_list()?, effect: drop_effect()? })
}

fn file_list() -> R<Vec<PathBuf>> {
    // SAFETY: the clipboard is open; the handle stays owned by the clipboard and is only read.
    let drop: HDROP = unsafe { GetClipboardData(u32::from(CF_HDROP)) };
    if drop.is_null() {
        return Ok(Vec::new());
    }
    // SAFETY: index u32::MAX with a null buffer asks only for the file count.
    let count = unsafe { DragQueryFileW(drop, u32::MAX, null_mut(), 0) };
    (0..count).map(|i| path_at(drop, i)).collect()
}

fn path_at(drop: HDROP, index: u32) -> R<PathBuf> {
    // SAFETY: a null buffer asks only for the length, without the terminating null.
    let len = unsafe { DragQueryFileW(drop, index, null_mut(), 0) };
    let mut buf = vec![0u16; len as usize + 1];
    // SAFETY: buf holds len characters plus the terminating null, as passed in cch.
    let got = unsafe { DragQueryFileW(drop, index, buf.as_mut_ptr(), len + 1) };
    if len == 0 || got != len {
        return Err(failure(&format!("Could not read file {} from the clipboard", index + 1)));
    }
    buf.truncate(got as usize);
    Ok(PathBuf::from(OsString::from_wide(&buf)))
}

pub(super) fn drop_effect_format() -> R<u32> {
    // SAFETY: w! yields a static null-terminated UTF-16 string.
    let format = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
    if format == 0 {
        return Err(failure(&format!("Could not read the clipboard format: {}", std::io::Error::last_os_error())));
    }
    Ok(format)
}

fn drop_effect() -> R<Option<u32>> {
    let format = drop_effect_format()?;
    // SAFETY: the clipboard is open; the handle stays owned by the clipboard and is only read.
    let data = unsafe { GetClipboardData(format) };
    // SAFETY: data is a non-null global memory handle from the clipboard.
    if data.is_null() || unsafe { GlobalSize(data) } < size_of::<u32>() {
        return Ok(None);
    }
    // SAFETY: data is a valid global memory handle holding at least one DWORD.
    let ptr = unsafe { GlobalLock(data) }.cast::<u32>();
    if ptr.is_null() {
        return Err(failure(&format!("Could not read the clipboard drop effect: {}", std::io::Error::last_os_error())));
    }
    // SAFETY: ptr points to at least four locked bytes; global memory carries no alignment promise for the reader.
    let effect = unsafe { ptr.read_unaligned() };
    // SAFETY: balances the GlobalLock above. A zero result only means the lock count reached zero.
    unsafe { GlobalUnlock(data) };
    Ok(Some(effect))
}

pub(super) fn failure(message: &str) -> FmError {
    FmError::Internal { message: message.to_string() }
}
