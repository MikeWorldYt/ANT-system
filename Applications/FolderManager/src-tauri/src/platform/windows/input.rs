// Keyboard state the drop events do not carry.
// Tauri's drop event carries no modifier keys, so the Ctrl state is read when the command arrives.
pub(crate) fn ctrl_held() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
    // SAFETY: GetAsyncKeyState only reads the keyboard state; the high bit is set while the key is down.
    unsafe { GetAsyncKeyState(i32::from(VK_CONTROL)) < 0 }
}
