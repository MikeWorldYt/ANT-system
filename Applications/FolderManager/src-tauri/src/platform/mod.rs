// OS-specific edges: shell shortcuts, the clipboard, drag out, the Recycle Bin, OS randomness and the Windows tray.
pub mod clipboard;
pub mod drag_out;
pub mod random;
pub mod recycle;
pub mod shortcuts;
#[cfg(not(windows))]
mod unix_trash;
#[cfg(windows)]
pub mod windows;
