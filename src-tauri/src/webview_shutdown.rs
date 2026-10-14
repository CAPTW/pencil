//! Close our WebView controllers while the UI event loop is still running.
//! Tauri's process exit does not wait for the WebView2 browser to exit.
use tauri::Manager;

#[cfg(windows)]
struct BrowserHandle(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
unsafe impl Send for BrowserHandle {}
#[cfg(windows)]
impl Drop for BrowserHandle {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0); }
    }
}

/// Only handles obtained from our controllers are observed. Never terminate a
/// process by name or enumerate other applications' WebView processes.
pub async fn close(app: &tauri::AppHandle) -> Result<(), &'static str> {
    #[cfg(windows)]
    {
        use std::time::{Duration, Instant};
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut receivers = Vec::new();
        for (_, window) in app.webview_windows() {
            let (send, receive) = tokio::sync::oneshot::channel();
            window.with_webview(move |view| {
                let result = (|| unsafe {
                    let controller = view.controller();
                    let core = controller.CoreWebView2().map_err(|_| "webview_core")?;
                    let mut pid = 0;
                    core.BrowserProcessId(&mut pid).map_err(|_| "webview_pid")?;
                    let raw = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
                    if raw.is_null() { return Err("webview_process_handle"); }
                    let handle = BrowserHandle(raw);
                    controller.Close().map_err(|_| "webview_close")?;
                    Ok(handle)
                })();
                let _ = send.send(result);
            }).map_err(|_| "webview_dispatch")?;
            receivers.push(receive);
        }
        let mut handles = Vec::new();
        for receive in receivers {
            handles.push(tokio::time::timeout_at(deadline.into(), receive).await
                .map_err(|_| "webview_close_timeout")?
                .map_err(|_| "webview_close_disconnected")??);
        }
        tauri::async_runtime::spawn_blocking(move || {
            for handle in handles { wait(&handle, deadline)?; }
            Ok(())
        }).await.map_err(|_| "webview_wait_task")?
    }
    #[cfg(not(windows))]
    {
        for (_, window) in app.webview_windows() {
            window.destroy().map_err(|_| "webview_destroy")?;
        }
        Ok(())
    }
}

#[cfg(windows)]
fn wait(handle: &BrowserHandle, deadline: std::time::Instant) -> Result<(), &'static str> {
    use windows_sys::Win32::{Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject};
    let millis = deadline.saturating_duration_since(std::time::Instant::now()).as_millis();
    let outcome = unsafe { WaitForSingleObject(handle.0, millis.min(u32::MAX as u128 - 1) as u32) };
    if outcome == WAIT_OBJECT_0 { Ok(()) } else { Err("webview_exit_unconfirmed") }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{os::windows::process::CommandExt, process::Command, time::{Duration, Instant}};
    use windows_sys::Win32::System::Threading::{GetCurrentProcessId, OpenProcess, PROCESS_SYNCHRONIZE};

    #[test]
    fn live_process_times_out_without_termination() {
        let handle = BrowserHandle(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, GetCurrentProcessId()) });
        assert!(!handle.0.is_null());
        assert_eq!(wait(&handle, Instant::now() + Duration::from_millis(20)), Err("webview_exit_unconfirmed"));
    }

    #[test]
    fn owned_process_exit_is_observed_with_same_deadline() {
        let mut child = Command::new("cmd.exe").args(["/d", "/c", "exit", "0"])
            .creation_flags(0x08000000).spawn().unwrap();
        let handle = BrowserHandle(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, child.id()) });
        assert!(!handle.0.is_null());
        assert_eq!(wait(&handle, Instant::now() + Duration::from_secs(5)), Ok(()));
        assert!(child.wait().unwrap().success());
        assert_eq!(wait(&handle, Instant::now()), Ok(()));
    }
}
