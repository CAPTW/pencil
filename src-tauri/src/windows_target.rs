use crate::capture_session::WindowTarget;
use std::future::Future;

pub(crate) trait ForegroundTargetPlatform {
    fn foreground_window(&mut self) -> isize;
    fn window_pid(&mut self, hwnd: isize) -> Option<u32>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetCaptureError {
    MissingForegroundWindow,
    MissingProcess,
    OwnWindow,
    OwnProcess,
}

impl TargetCaptureError {
    fn message(self) -> &'static str {
        match self {
            Self::MissingForegroundWindow | Self::MissingProcess => {
                "No safe foreground target is available. Select text in another app and try again."
            }
            Self::OwnWindow | Self::OwnProcess => {
                "Codex Pencil cannot capture itself. Return to the target app and try again."
            }
        }
    }
}

pub(crate) fn capture_foreground_target<P: ForegroundTargetPlatform>(
    platform: &mut P,
    own_hwnd: isize,
    own_pid: u32,
) -> Result<WindowTarget, TargetCaptureError> {
    let hwnd = platform.foreground_window();
    if hwnd == 0 {
        return Err(TargetCaptureError::MissingForegroundWindow);
    }
    if hwnd == own_hwnd {
        return Err(TargetCaptureError::OwnWindow);
    }

    let pid = platform
        .window_pid(hwnd)
        .filter(|pid| *pid != 0)
        .ok_or(TargetCaptureError::MissingProcess)?;
    if pid == own_pid {
        return Err(TargetCaptureError::OwnProcess);
    }

    Ok(WindowTarget::new(hwnd, pid))
}

pub(crate) async fn capture_before_widget_focus<P, T, Prepare, PrepareFuture, Focus>(
    platform: &mut P,
    own_hwnd: isize,
    own_pid: u32,
    prepare: Prepare,
    focus_widget: Focus,
) -> Result<(WindowTarget, T), String>
where
    P: ForegroundTargetPlatform,
    Prepare: FnOnce(WindowTarget) -> PrepareFuture,
    PrepareFuture: Future<Output = Result<T, String>>,
    Focus: FnOnce(&T) -> Result<(), String>,
{
    let target = capture_foreground_target(platform, own_hwnd, own_pid)
        .map_err(|error| error.message().to_string())?;
    let prepared = prepare(target).await?;
    focus_widget(&prepared)?;
    Ok((target, prepared))
}

#[derive(Debug, Default)]
pub(crate) struct WindowsForegroundTargetPlatform;

#[cfg(windows)]
impl ForegroundTargetPlatform for WindowsForegroundTargetPlatform {
    fn foreground_window(&mut self) -> isize {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        unsafe { GetForegroundWindow() as isize }
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
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
}

#[cfg(not(windows))]
impl ForegroundTargetPlatform for WindowsForegroundTargetPlatform {
    fn foreground_window(&mut self) -> isize {
        0
    }

    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        None
    }
}

/// Reads the exact selection of the focused standard Edit control of the
/// captured foreground window. The admitted scope, sensitive-field denial and
/// re-validation live in `native_edit`; everything else is denied before any
/// text-bearing message and nothing touches input or the clipboard.
#[cfg(windows)]
pub(crate) fn read_supported_selection(
    top_level: isize,
) -> Result<crate::native_edit::NativeSelection, String> {
    crate::native_edit::read_focused_selection(
        &mut crate::native_edit::Win32EditPort::default(),
        top_level,
    )
    .map_err(|denial| denial.message().to_string())
}

#[cfg(not(windows))]
pub(crate) fn read_supported_selection(
    _top_level: isize,
) -> Result<crate::native_edit::NativeSelection, String> {
    Err(crate::native_edit::CaptureDenial::Unsupported.message().to_string())
}

#[cfg(all(test, windows))]
mod mission_secure_field_tests {
    use crate::native_edit::{read_focused_selection, CaptureDenial, EditPort, Win32EditPort};

    #[test]
    fn mission_native_message_only_edit_is_denied_without_text_messages() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        // A message-only HWND is invisible, never activated, and owned by this
        // test thread. This is negative native guard evidence, not capture acceptance.
        let class: Vec<u16> = "Edit\0".encode_utf16().collect();
        unsafe {
            for style in [0u32, 0x20u32] {
                let edit = CreateWindowExW(0, class.as_ptr(), std::ptr::null(), style,
                    0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null());
                assert!(!edit.is_null());
                let mut port = Win32EditPort::default();
                // Not the foreground window, so the reader refuses before any message.
                let denied = read_focused_selection(&mut port, edit as isize);
                assert!(!port.is_visible(edit as isize));
                let destroyed = DestroyWindow(edit);
                assert_eq!(destroyed, 1);
                assert_eq!(denied, Err(CaptureDenial::NoEditor));
            }
        }
    }
}
