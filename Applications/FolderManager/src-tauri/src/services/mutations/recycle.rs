// Hands an item to the Recycle Bin function the caller passes in, turning its failure into an FmError.
use std::path::Path;

// A failed recycle leaves the item where it is, so the caller changes nothing else.
pub(crate) fn move_to_bin(path: &Path, recycle: impl FnOnce(&Path) -> Result<(), String>) -> crate::error::R<()> {
    recycle(path).map_err(|message| crate::error::FmError::Io {
        op: "send to the Recycle Bin".to_string(),
        path: path.display().to_string(),
        message,
        os_code: None,
    })
}
