// Windows-only implementations behind the cross-platform wrappers of platform.
pub mod clipboard_read;
pub mod clipboard_write;
pub mod curl;
pub mod drag_out;
pub mod file_manager;
pub mod hidden_attr;
pub mod input;
pub mod powershell;
pub mod recycle;
pub mod review_links;
pub mod schtasks;
pub mod shortcuts;
pub mod system;
#[cfg(not(test))]
pub mod tray;
pub mod updater;
pub mod url_handler;
