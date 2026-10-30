// Win32 write of one folder as CF_HDROP plus the "Preferred DropEffect", so Explorer can paste or move it.
use std::io::Error;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows_sys::Win32::System::DataExchange::{EmptyClipboard, SetClipboardData};
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_HDROP;
use windows_sys::Win32::UI::Shell::DROPFILES;

use super::win32::{drop_effect_format, failure, OpenClipboardGuard};
use crate::error::R;

// Owns a moveable global block until the clipboard or a drop target takes it.
pub(crate) struct Global(HGLOBAL);

impl Global {
    pub(crate) fn with(bytes: &[u8]) -> R<Self> {
        // SAFETY: a plain allocation; a null result is checked right after.
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
        if handle.is_null() {
            return Err(failure(&format!("Could not allocate clipboard memory: {}", Error::last_os_error())));
        }
        let block = Self(handle);
        // SAFETY: handle is a live moveable block of bytes.len() bytes.
        let ptr = unsafe { GlobalLock(handle) }.cast::<u8>();
        if ptr.is_null() {
            return Err(failure(&format!("Could not fill clipboard memory: {}", Error::last_os_error())));
        }
        // SAFETY: ptr holds bytes.len() locked bytes of a fresh block, so the ranges cannot overlap.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        // SAFETY: balances the GlobalLock above. A zero result only means the lock count reached zero.
        unsafe { GlobalUnlock(handle) };
        Ok(block)
    }

    fn hand_to_clipboard(self, format: u32) -> R<()> {
        // SAFETY: the clipboard is open and emptied on this thread; the handle is an unlocked moveable block.
        if unsafe { SetClipboardData(format, self.0) }.is_null() {
            return Err(failure(&format!("Could not write to the clipboard: {}", Error::last_os_error())));
        }
        // The system owns the block once SetClipboardData succeeds, so it must not be freed here.
        std::mem::forget(self);
        Ok(())
    }

    // The receiver frees the block, as a drop target does with ReleaseStgMedium.
    pub(crate) fn into_raw(self) -> HGLOBAL {
        let handle = self.0;
        std::mem::forget(self);
        handle
    }
}

impl Drop for Global {
    fn drop(&mut self) {
        // SAFETY: the block was allocated by GlobalAlloc and never handed to the clipboard.
        if !unsafe { GlobalFree(self.0) }.is_null() {
            eprintln!("Could not free clipboard memory: {}", Error::last_os_error());
        }
    }
}

// DROPFILES fields in order: pFiles, pt.x, pt.y, fNC, fWide; each path ends with a null and the list with an extra one.
pub(crate) fn hdrop_bytes(paths: &[PathBuf]) -> Vec<u8> {
    let header = [size_of::<DROPFILES>() as u32, 0, 0, 0, 1];
    let mut bytes: Vec<u8> = header.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let units = paths.iter().flat_map(|p| p.as_os_str().encode_wide().chain([0])).chain([0]);
    for unit in units {
        bytes.extend_from_slice(&unit.to_ne_bytes());
    }
    bytes
}

pub fn write(path: &Path, effect: u32) -> R<()> {
    let format = drop_effect_format()?;
    let files = Global::with(&hdrop_bytes(&[path.to_path_buf()]))?;
    let effect = Global::with(&effect.to_ne_bytes())?;
    let _open = OpenClipboardGuard::acquire()?;
    // SAFETY: the clipboard is open on this thread.
    if unsafe { EmptyClipboard() } == 0 {
        return Err(failure(&format!("Could not clear the clipboard: {}", Error::last_os_error())));
    }
    files.hand_to_clipboard(u32::from(CF_HDROP))?;
    effect.hand_to_clipboard(format)
}
