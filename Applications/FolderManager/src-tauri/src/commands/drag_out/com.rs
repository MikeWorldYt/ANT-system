// OLE drag source for folders: the data object hands Explorer a CF_HDROP list, the drop source follows the mouse.
use std::mem::ManuallyDrop;
use std::path::PathBuf;

use windows::Win32::Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, DRAGDROP_S_USEDEFAULTCURSORS, DV_E_FORMATETC, E_NOTIMPL, E_OUTOFMEMORY, HGLOBAL, OLE_E_ADVISENOTSUPPORTED, S_OK};
use windows::Win32::System::Com::{IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumSTATDATA, DATADIR_GET, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL};
use windows::Win32::System::Ole::{DoDragDrop, IDropSource, IDropSource_Impl, CF_HDROP, DROPEFFECT, DROPEFFECT_NONE};
use windows::Win32::System::SystemServices::{MODIFIERKEYS_FLAGS, MK_LBUTTON};
use windows::Win32::UI::Shell::SHCreateStdEnumFmtEtc;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_SWAPBUTTON};
use windows_core::{implement, Error, Ref, Result, BOOL, HRESULT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};

use super::ALLOWED_EFFECTS;
use crate::commands::clipboard::{hdrop_bytes, Global};
use crate::error::{FmError, R};

fn hdrop_format() -> FORMATETC {
    FORMATETC { cfFormat: CF_HDROP.0, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}

fn is_hdrop(format: *const FORMATETC) -> bool {
    // SAFETY: OLE passes either null or a FORMATETC that lives for the call.
    let Some(f) = (unsafe { format.as_ref() }) else { return false };
    f.cfFormat == CF_HDROP.0 && f.dwAspect == DVASPECT_CONTENT.0 && f.tymed & TYMED_HGLOBAL.0 as u32 != 0
}

#[implement(IDataObject)]
struct FolderData {
    paths: Vec<PathBuf>,
}

impl IDataObject_Impl for FolderData_Impl {
    fn GetData(&self, format: *const FORMATETC) -> Result<STGMEDIUM> {
        if !is_hdrop(format) {
            return Err(DV_E_FORMATETC.into());
        }
        let block = Global::with(&hdrop_bytes(&self.paths)).map_err(|e| Error::new(E_OUTOFMEMORY, e.to_string()))?;
        // Each call hands out a fresh block, since the receiver frees it with ReleaseStgMedium.
        let hglobal = HGLOBAL(block.into_raw());
        Ok(STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: hglobal }, pUnkForRelease: ManuallyDrop::new(None) })
    }

    // Filling a caller's medium is optional for a drag source; Explorer asks GetData.
    fn GetDataHere(&self, _format: *const FORMATETC, _medium: *mut STGMEDIUM) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn QueryGetData(&self, format: *const FORMATETC) -> HRESULT {
        if is_hdrop(format) { S_OK } else { DV_E_FORMATETC }
    }

    // The single format has no canonical variant to report.
    fn GetCanonicalFormatEtc(&self, _format_in: *const FORMATETC, _format_out: *mut FORMATETC) -> HRESULT {
        E_NOTIMPL
    }

    // Targets may store drop feedback here; the folder list never changes, so it is refused.
    fn SetData(&self, _format: *const FORMATETC, _medium: *const STGMEDIUM, _release: BOOL) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn EnumFormatEtc(&self, direction: u32) -> Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 {
            return Err(E_NOTIMPL.into());
        }
        // SAFETY: the slice outlives the call; the shell copies the formats into its enumerator.
        unsafe { SHCreateStdEnumFmtEtc(&[hdrop_format()]) }
    }

    // The data is fixed for the whole drag, so there is nothing to advise about.
    fn DAdvise(&self, _format: *const FORMATETC, _advf: u32, _sink: Ref<IAdviseSink>) -> Result<u32> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn DUnadvise(&self, _connection: u32) -> Result<()> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(OLE_E_ADVISENOTSUPPORTED.into())
    }
}

#[implement(IDropSource)]
struct FolderSource;

impl IDropSource_Impl for FolderSource_Impl {
    fn QueryContinueDrag(&self, escape: BOOL, keys: MODIFIERKEYS_FLAGS) -> HRESULT {
        if escape.as_bool() {
            DRAGDROP_S_CANCEL
        } else if keys.0 & MK_LBUTTON.0 == 0 {
            DRAGDROP_S_DROP
        } else {
            S_OK
        }
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

// A button released before the drag starts would drop the folder back onto this window.
fn primary_button_down() -> bool {
    // SAFETY: both calls only read system and input state.
    unsafe {
        let key = if GetSystemMetrics(SM_SWAPBUTTON) != 0 { VK_RBUTTON } else { VK_LBUTTON };
        GetAsyncKeyState(i32::from(key)) < 0
    }
}

// Must run on the UI thread, where tao called OleInitialize when it created the window.
pub fn drag_folders(paths: &[PathBuf]) -> R<u32> {
    if !primary_button_down() {
        return Ok(DROPEFFECT_NONE.0);
    }
    let data: IDataObject = FolderData { paths: paths.to_vec() }.into();
    let source: IDropSource = FolderSource.into();
    let mut effect = DROPEFFECT_NONE;
    // SAFETY: both objects live until DoDragDrop returns, and the caller is on the OLE-initialized UI thread.
    let result = unsafe { DoDragDrop(&data, &source, DROPEFFECT(ALLOWED_EFFECTS), &mut effect) };
    if result == DRAGDROP_S_CANCEL {
        return Ok(DROPEFFECT_NONE.0);
    }
    if result != DRAGDROP_S_DROP {
        return Err(FmError::Internal { message: format!("Could not drag the folder: {}", Error::from_hresult(result).message()) });
    }
    Ok(effect.0)
}
