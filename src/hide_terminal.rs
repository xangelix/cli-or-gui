#[cfg(target_os = "windows")]
pub fn hide_console_window() {
    use windows::Win32::{
        Foundation::HWND,
        System::Console::GetConsoleWindow,
        UI::WindowsAndMessaging::{SW_HIDE, ShowWindow},
    };

    unsafe {
        let console_window = GetConsoleWindow();
        if !console_window.0.is_null() {
            ShowWindow(HWND(console_window.0.cast()), SW_HIDE).unwrap();
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[allow(clippy::missing_const_for_fn)]
pub fn hide_console_window() {
    // No-op on non-Windows platforms.
    // Linux/macOS GUI bundles do not automatically open terminal windows,
    // and running directly from a shell should not hide the shell window.
}
