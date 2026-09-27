use crate::apply_safety::ApplyPlatform;
use crate::clipboard;

#[cfg(windows)]
pub(crate) fn window_is_valid(hwnd: isize) -> bool {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindow;

    unsafe { IsWindow(hwnd as HWND) != 0 }
}

#[cfg(not(windows))]
pub(crate) fn window_is_valid(_hwnd: isize) -> bool {
    false
}

#[cfg(windows)]
pub(crate) fn window_process_id(hwnd: isize) -> Option<u32> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    let mut pid = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd as HWND, &mut pid) };
    if thread_id == 0 || pid == 0 {
        None
    } else {
        Some(pid)
    }
}

#[cfg(not(windows))]
pub(crate) fn window_process_id(_hwnd: isize) -> Option<u32> {
    None
}

/// Brings a task-owned test window forward (live tests only; the product never
/// changes the foreground window of another application).
#[cfg(all(test, windows))]
pub(crate) fn request_foreground_window(hwnd: isize) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetActiveWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    let hwnd = hwnd as HWND;
    unsafe {
        let current_thread = GetCurrentThreadId();
        let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let foreground = GetForegroundWindow();
        let foreground_thread = if foreground.is_null() {
            0
        } else {
            GetWindowThreadProcessId(foreground, std::ptr::null_mut())
        };
        let attached_to_foreground = current_thread != 0
            && foreground_thread != 0
            && current_thread != foreground_thread
            && AttachThreadInput(current_thread, foreground_thread, 1) != 0;
        let attached_to_target = current_thread != 0
            && target_thread != 0
            && current_thread != target_thread
            && target_thread != foreground_thread
            && AttachThreadInput(current_thread, target_thread, 1) != 0;

        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetActiveWindow(hwnd);

        if attached_to_target {
            let _ = AttachThreadInput(current_thread, target_thread, 0);
        }
        if attached_to_foreground {
            let _ = AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
}


#[cfg(windows)]
pub(crate) fn foreground_window_handle() -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe { GetForegroundWindow() as isize }
}

#[cfg(not(windows))]
pub(crate) fn foreground_window_handle() -> isize {
    0
}

/// Copy-only result delivery: checks the captured window and writes the clipboard.
#[derive(Clone, Default)]
pub(crate) struct WindowsApplyPlatform;

impl WindowsApplyPlatform {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl ApplyPlatform for WindowsApplyPlatform {
    fn is_window(&mut self, hwnd: isize) -> bool {
        window_is_valid(hwnd)
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
        window_process_id(hwnd)
    }

    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()> {
        clipboard::write_clipboard_text(text).map_err(|_| ())
    }
}

#[cfg(not(windows))]
impl ApplyPlatform for WindowsApplyPlatform {
    fn is_window(&mut self, _hwnd: isize) -> bool {
        false
    }

    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        None
    }

    fn write_clipboard_text(&mut self, _text: &str) -> Result<(), ()> {
        Err(())
    }
}
