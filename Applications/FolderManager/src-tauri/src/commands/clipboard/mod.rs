// Files on the Windows clipboard, read for paste and written for copy and cut, with their cut or copy effect.
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod win32;
#[cfg(windows)]
mod win32_write;

use std::path::PathBuf;

use crate::ant_core::tree_types::type_base;
use crate::config::DATA_FLAG;
use crate::error::{FmError, R};
use crate::tree::tree_node::is_reserved;

// Explorer writes this "Preferred DropEffect" value on Ctrl+X; Ctrl+C writes copy or link.
const DROPEFFECT_MOVE: u32 = 2;
const DROPEFFECT_COPY: u32 = 1;
const CUT_REFUSED: &str = "Only data folders and workboards can be cut";

pub struct ClipboardFiles {
    pub paths: Vec<PathBuf>,
    pub effect: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Paste {
    pub paths: Vec<PathBuf>,
    pub copy: bool,
}

pub fn paste_plan(files: ClipboardFiles) -> R<Paste> {
    if files.paths.is_empty() {
        return Err(FmError::ClipboardEmpty);
    }
    Ok(Paste { paths: files.paths, copy: files.effect != Some(DROPEFFECT_MOVE) })
}

pub fn drop_effect(cut: bool) -> u32 {
    if cut { DROPEFFECT_MOVE } else { DROPEFFECT_COPY }
}

// Explorer moves a cut folder out of the storage, so only data folders and workboards may leave that way.
pub fn folder_effect(rel: &[String], node_type: &str, flags: &[String], ghost: bool, cut: bool) -> R<u32> {
    if is_reserved(rel) {
        return Err(FmError::Reserved { rel_parts: rel.to_vec() });
    }
    if ghost {
        return Err(FmError::GhostNotAllowed { op: if cut { "cut" } else { "copy" }.to_string() });
    }
    let cuttable = type_base(node_type) == "WKB" || flags.iter().any(|f| f == DATA_FLAG);
    if cut && !cuttable {
        return Err(FmError::InvalidPath { rel_parts: rel.to_vec(), reason: CUT_REFUSED.to_string() });
    }
    Ok(drop_effect(cut))
}

#[cfg(windows)]
pub use win32::read;
#[cfg(windows)]
pub use win32_write::write;
#[cfg(windows)]
pub(crate) use win32_write::{hdrop_bytes, Global};

#[cfg(not(windows))]
pub fn read() -> R<ClipboardFiles> {
    Err(FmError::Internal { message: "Paste is only available on Windows for now".to_string() })
}

#[cfg(not(windows))]
pub fn write(_path: &std::path::Path, _effect: u32) -> R<()> {
    Err(FmError::Internal { message: "Copy and cut are only available on Windows for now".to_string() })
}
