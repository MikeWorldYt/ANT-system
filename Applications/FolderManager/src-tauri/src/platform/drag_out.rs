// Drag out to Explorer: the OLE drag on Windows, a refusal on other systems.
#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(not(windows))]
use crate::domain::error::{FmError, R};

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;
// Folders are only moved: Explorer would otherwise default to copy across drives and leave the original behind.
pub(crate) const ALLOWED_EFFECTS: u32 = DROPEFFECT_MOVE;
// Apps such as chats, browsers and editors only take a copy, so files also offer it.
pub(crate) const FILE_EFFECTS: u32 = DROPEFFECT_COPY | DROPEFFECT_MOVE;

#[cfg(windows)]
pub(crate) use crate::platform::windows::drag_out::drag_folders;

#[cfg(not(windows))]
pub(crate) fn drag_folders(_paths: &[PathBuf], _effects: u32) -> R<u32> {
    Err(FmError::Internal { message: "Drag out is only available on Windows for now".to_string() })
}
