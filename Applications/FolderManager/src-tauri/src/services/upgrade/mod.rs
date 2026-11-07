// One-time upgrade of storages that still use the numeric support ids (90., 97., 98.) and the 998/999 root folders.
mod apply;
mod extra_reviews;
mod finish;
mod scan;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::domain::config::{REVIEW_FOLDER, SETTINGS_FOLDER};
use crate::domain::error::{FmError, R};
use crate::infra::fs_safe::{io_err, write_atomic};
use crate::infra::storage::core_dir::data_dir;
use crate::infra::storage::db::hide;

pub use apply::apply_support_upgrade;
pub use extra_reviews::{extra_reviews, merge_extra_review, ExtraReview};
pub use finish::finish_support_upgrade;

const MARKER: &str = "support-nodes-v2.done";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportFolder {
    pub rel_parts: Vec<String>,
    pub name: String,
    pub notice: bool,
    pub proposed: String,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportCheck {
    pub needed: bool,
    pub folders: Vec<SupportFolder>,
    pub root: bool,
}

fn marker(root: &Path) -> PathBuf {
    data_dir(root).join(MARKER)
}

pub(super) fn is_done(root: &Path) -> bool {
    marker(root).is_file()
}

fn write_marker(root: &Path) -> R<()> {
    let dir = data_dir(root);
    if !dir.is_dir() {
        fs::create_dir_all(&dir).map_err(|e| io_err("create folder", &dir, &e))?;
        hide(&dir);
    }
    write_atomic(&marker(root), b"done\n")
}

fn has_root_legacy(root: &Path) -> bool {
    [REVIEW_FOLDER, SETTINGS_FOLDER].iter().any(|name| root.join(name).is_dir())
}

// A storage with nothing old gets the marker at once, so the check never runs for it again.
pub fn check_support_upgrade(root: &Path) -> R<SupportCheck> {
    if is_done(root) {
        return Ok(SupportCheck::default());
    }
    let folders = scan::legacy_folders(root)?;
    let root_legacy = has_root_legacy(root);
    if folders.is_empty() && !root_legacy {
        write_marker(root)?;
        return Ok(SupportCheck::default());
    }
    Ok(SupportCheck { needed: true, folders, root: root_legacy })
}

fn internal(message: impl Into<String>) -> FmError {
    FmError::Internal { message: message.into() }
}
