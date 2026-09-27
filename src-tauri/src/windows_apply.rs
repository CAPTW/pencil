use crate::apply_safety::{ApplyPlatform, WaitStage};
use crate::clipboard;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const ACTIVATION_WAIT: Duration = Duration::from_millis(50);
const PRE_PASTE_WAIT: Duration = Duration::from_millis(40);
const POST_PASTE_WAIT: Duration = Duration::from_millis(160);

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

#[cfg(windows)]
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

#[cfg(not(windows))]
pub(crate) fn request_foreground_window(_hwnd: isize) {}

#[cfg(windows)]
pub(crate) fn foreground_window_handle() -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe { GetForegroundWindow() as isize }
}

#[cfg(not(windows))]
pub(crate) fn foreground_window_handle() -> isize {
    0
}

pub(crate) fn wait_for_stage(stage: WaitStage) {
    let duration = match stage {
        WaitStage::AfterActivation => ACTIVATION_WAIT,
        WaitStage::BeforePaste => PRE_PASTE_WAIT,
        WaitStage::AfterPaste => POST_PASTE_WAIT,
    };
    thread::sleep(duration);
}

#[derive(Clone)]
pub(crate) struct WindowsApplyPlatform {
    app: AppHandle,
}

impl WindowsApplyPlatform {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

/// Production native Apply: the verified standard Edit protocol in `native_edit`.
#[cfg(windows)]
pub(crate) fn apply_native_edit_win32(
    top_level: isize,
    pid: u32,
    binding: &crate::capture_session::NativeEditBinding,
    replacement: &str,
) -> crate::native_edit::NativeApplyResult {
    crate::native_edit::apply_to_captured_edit(
        &mut crate::native_edit::Win32EditPort::default(),
        top_level,
        pid,
        binding,
        replacement,
    )
}

#[cfg(windows)]
impl ApplyPlatform for WindowsApplyPlatform {
    fn apply_native_edit(
        &mut self,
        top_level: isize,
        pid: u32,
        binding: &crate::capture_session::NativeEditBinding,
        replacement: &str,
    ) -> crate::native_edit::NativeApplyResult {
        apply_native_edit_win32(top_level, pid, binding, replacement)
    }

    fn is_window(&mut self, hwnd: isize) -> bool {
        window_is_valid(hwnd)
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
        window_process_id(hwnd)
    }

    fn hide_widget(&mut self) -> Result<(), ()> {
        let window = self.app.get_webview_window("main").ok_or(())?;
        window.hide().map_err(|_| ())
    }

    fn show_widget(&mut self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn request_foreground(&mut self, hwnd: isize) {
        request_foreground_window(hwnd);
    }

    fn foreground_window(&mut self) -> isize {
        foreground_window_handle()
    }

    fn clipboard_sequence(&mut self) -> u32 {
        clipboard::clipboard_sequence_number()
    }

    fn read_clipboard_text(&mut self) -> Option<String> {
        clipboard::read_clipboard_text().ok()
    }

    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()> {
        clipboard::write_clipboard_text(text).map_err(|_| ())
    }

    fn send_paste(&mut self) -> u32 {
        clipboard::send_paste_shortcut_count()
    }

    fn wait(&mut self, stage: WaitStage) {
        wait_for_stage(stage);
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

    fn hide_widget(&mut self) -> Result<(), ()> {
        Err(())
    }

    fn show_widget(&mut self) {}

    fn request_foreground(&mut self, _hwnd: isize) {}

    fn foreground_window(&mut self) -> isize {
        0
    }

    fn clipboard_sequence(&mut self) -> u32 {
        0
    }

    fn read_clipboard_text(&mut self) -> Option<String> {
        None
    }

    fn write_clipboard_text(&mut self, _text: &str) -> Result<(), ()> {
        Err(())
    }

    fn send_paste(&mut self) -> u32 {
        0
    }

    fn wait(&mut self, _stage: WaitStage) {}
}
