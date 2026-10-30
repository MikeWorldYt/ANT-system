// Event-driven Storage watching: folder changes on disk update only the affected part of the stored tree.
mod apply;
mod filter;
mod guard;
mod gui;
mod headless;
mod targets;
mod tray;
mod watcher;

use std::path::{Path, PathBuf};

use crate::debug_log;
use crate::error::R;

pub use gui::GuiWatch;
pub use headless::run_headless;
pub use watcher::watch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub paths: usize,
    pub full: bool,
    pub skipped: usize,
    pub saved: bool,
}

pub fn apply_batch(root: &Path, changed: &[PathBuf]) -> R<ApplyReport> {
    apply::apply(root, changed, None)
}

// Logging must never stop the watcher, so a failed write only reaches stderr.
fn log(level: &str, message: &str) {
    if let Err(e) = debug_log::line(level, message) {
        eprintln!("Debug log: {e}");
    }
}
