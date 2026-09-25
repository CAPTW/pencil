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

/// Generic and custom controls have no trustworthy secure-field contract here.
fn admitted_edit(class: &[u16], style: i32, enabled: bool, visible: bool) -> bool {
    class == [69, 100, 105, 116] && style & 0x20 == 0 && enabled && visible
}

#[cfg(windows)]
fn admitted_native_edit(edit: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
    unsafe {
        let mut class = [0u16; 32];
        let count = GetClassNameW(edit, class.as_mut_ptr(), class.len() as i32);
        count > 0 && admitted_edit(&class[..count as usize], GetWindowLongW(edit, GWL_STYLE),
            IsWindowEnabled(edit) != 0, IsWindowVisible(edit) != 0)
    }
}

#[cfg(windows)]
pub(crate) fn read_supported_selection() -> Result<String, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    // Candidate containment: metadata denial is tested, but positive capture and
    // sensitivity authority have not been qualified in an isolated input desktop.
    // Keep the implementation for that acceptance step; never silently read an
    // unqualified native field in a personal-use candidate.
    const NATIVE_CAPTURE_QUALIFIED: bool = false;
    if !NATIVE_CAPTURE_QUALIFIED {
        return Err("native_capture_not_qualified: use the explicitly enabled Chromium adapter; native selection capture is withheld pending isolated editor acceptance".into());
    }
    let denied = || "unsupported_or_sensitive_editor: native capture supports visible non-password standard Edit controls only".to_string();
    unsafe {
        let foreground = GetForegroundWindow();
        let thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if thread == 0 || GetGUIThreadInfo(thread, &mut info) == 0 || info.hwndFocus.is_null() {
            return Err(denied());
        }
        let edit = info.hwndFocus;
        let admitted = || admitted_native_edit(edit);
        if !admitted() { return Err(denied()); }
        // Each cross-process message has a bounded wait. No input injection or
        // clipboard access occurs, including on an unsupported target.
        let message = |msg, wparam, lparam| -> Result<usize, String> {
            let mut result = 0usize;
            if SendMessageTimeoutW(edit, msg, wparam, lparam,
                SMTO_ABORTIFHUNG | SMTO_BLOCK, 100, &mut result) == 0 { return Err(denied()); }
            Ok(result)
        };
        let range = message(0x00B0, 0, 0)?; // EM_GETSEL: bounded to 16-bit offsets
        let start = range & 0xffff;
        let end = range >> 16;
        let length = message(WM_GETTEXTLENGTH, 0, 0)?;
        if start >= end || end > length || length >= 65535 || !admitted() { return Err(denied()); }
        let mut text = vec![0u16; length + 1];
        let read = message(WM_GETTEXT, text.len(), text.as_mut_ptr() as isize)?;
        let mut current: GUITHREADINFO = std::mem::zeroed();
        current.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if !admitted() || GetForegroundWindow() != foreground ||
            GetGUIThreadInfo(thread, &mut current) == 0 || current.hwndFocus != edit ||
            message(0x00B0, 0, 0)? != range || read != length {
            return Err(denied());
        }
        String::from_utf16(&text[start..end]).map_err(|_| denied())
    }
}

#[cfg(not(windows))]
pub(crate) fn read_supported_selection() -> Result<String, String> {
    Err("unsupported_or_sensitive_editor".to_string())
}

#[cfg(test)]
mod mission_secure_field_tests {
    use super::*;
    #[test]
    fn only_explicit_supported_non_password_control_is_admitted() {
        let edit: Vec<u16> = "Edit".encode_utf16().collect();
        assert!(admitted_edit(&edit, 0, true, true));
        assert!(!admitted_edit(&edit, 0x20, true, true));
        assert!(!admitted_edit(&edit, 0, false, true));
        assert!(!admitted_edit(&edit, 0, true, false));
        assert!(!admitted_edit(&"RichEdit20W".encode_utf16().collect::<Vec<_>>(), 0, true, true));
        assert!(!admitted_edit(&[], 0, true, true));
    }
    #[cfg(windows)]
    #[test]
    fn mission_native_message_only_edit_is_denied_without_foreground_or_clipboard() {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        // A message-only HWND is invisible, never activated, and owned by this
        // test thread. This is negative native guard evidence, not capture acceptance.
        let class: Vec<u16> = "Edit\0".encode_utf16().collect();
        unsafe {
            for style in [0u32, 0x20u32] {
                let edit = CreateWindowExW(0, class.as_ptr(), std::ptr::null(), style,
                    0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null());
                assert!(!edit.is_null());
                let admitted = admitted_native_edit(edit);
                let destroyed = DestroyWindow(edit);
                assert_eq!(destroyed, 1);
                assert!(!admitted);
            }
        }
    }

}
