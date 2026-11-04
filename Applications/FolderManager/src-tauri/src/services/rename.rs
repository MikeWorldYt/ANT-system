// Renaming the open storage's root folder: the flow both the rename and its undo go through.
use std::path::{Path, PathBuf};

use crate::app::storage_session::Storage;
use crate::error::{FmError, R};
use crate::storage::rename;

// Only the in-process lock is held: an open StorageLock file inside the folder would block its own rename.
pub fn rename_flow(storage: &Storage, stamp: &str, target: impl FnOnce(&Path) -> R<String>) -> R<PathBuf> {
    rename::check_stamp(stamp)?;
    let _guard = storage.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let root = storage.root()?;
    let new_name = target(&root)?;
    let new_root = rename::rename_root(&root, &new_name)?;
    storage.set_root(new_root.clone());
    // The folder already has its new name, so a missing log must not report the rename as failed.
    if let Err(e) = rename::write_log(&new_root, stamp, &rename::folder_name(&root), &new_name) {
        eprintln!("Could not write the Storage rename log: {e}");
    }
    Ok(new_root)
}

pub fn revert_target(root: &Path) -> R<String> {
    match rename::status(root) {
        Some(status) if status.revertible => Ok(status.old),
        _ => Err(FmError::Internal { message: "The last Storage rename cannot be undone".to_string() }),
    }
}

#[cfg(test)]
#[path = "rename_tests.rs"]
mod tests;
