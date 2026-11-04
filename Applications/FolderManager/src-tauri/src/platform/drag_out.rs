// Drag out to Explorer: the OLE drag on Windows, a refusal on other systems.
#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(not(windows))]
use crate::error::{FmError, R};

const DROPEFFECT_MOVE: u32 = 2;
// Only a move: Explorer would otherwise default to copy across drives and leave the original behind.
pub(crate) const ALLOWED_EFFECTS: u32 = DROPEFFECT_MOVE;

#[cfg(windows)]
pub(crate) use crate::platform::windows::drag_out::drag_folders;

#[cfg(not(windows))]
pub(crate) fn drag_folders(_paths: &[PathBuf]) -> R<u32> {
    Err(FmError::Internal { message: "Drag out is only available on Windows for now".to_string() })
}
