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
