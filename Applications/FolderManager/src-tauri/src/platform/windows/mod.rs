// Windows-only implementations behind the cross-platform wrappers of platform.
pub mod clipboard_read;
pub mod clipboard_write;
pub mod drag_out;
pub mod powershell;
pub mod recycle;
#[cfg(not(test))]
pub mod tray;
