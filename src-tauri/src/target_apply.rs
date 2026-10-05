//! Desktop target-side Apply owner. All transport methods run on a blocking
//! worker. Cancellation only sets the native flag; it never waits for that worker.
use crate::capture_session::SessionToken;
use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DirectOutcome {
    Applied,
    Rejected,
    Indeterminate,
}

trait Api: Send + Sync {
    fn create(&self, field: isize) -> Result<u64, &'static str>;
    fn capture(&self, cookie: u64) -> Result<String, &'static str>;
    fn apply(&self, cookie: u64, text: &str) -> DirectOutcome;
    fn cancel(&self, cookie: u64);
    fn close(&self, cookie: u64) -> bool;
}
struct Slot {
    api: Arc<dyn Api>,
    cookie: u64,
    token: Option<SessionToken>,
}
#[derive(Default)]
struct Registry {
    epoch: u64,
    slot: Option<Slot>,
}
#[derive(Default)]
pub(crate) struct TargetApplyRuntime {
    operation: Mutex<()>,
    registry: Mutex<Registry>,
}
pub(crate) struct TargetCapture {
    pub text: String,
    cookie: u64,
    epoch: u64,
}
impl TargetApplyRuntime {
    /// Record before queuing a capture, so Dismiss can revoke queued work too.
    pub fn epoch(&self) -> Result<u64, &'static str> {
        Ok(self
            .registry
            .lock()
            .map_err(|_| "native_apply_state_unavailable")?
            .epoch)
    }
    pub fn capture(&self, field: isize, epoch: u64) -> Result<TargetCapture, &'static str> {
        self.capture_with(field, epoch, load_api()?)
    }
    pub fn capture_browser(&self, window: isize, epoch: u64) -> Result<TargetCapture, &'static str> {
        let api=load_browser_api().map_err(|_| "browser_review_unavailable")?;
        self.capture_with(window, epoch, api).map_err(|error| match error {
            "native_capture_unavailable" | "native_apply_unavailable" => "browser_review_unavailable",
            other => other,
        })
    }
    pub fn capture_writer(&self, window: isize, epoch: u64) -> Result<TargetCapture, &'static str> {
        let api = load_writer_api().map_err(|_| "writer_review_unavailable")?;
        self.capture_with(window, epoch, api).map_err(|error| match error {
            "native_capture_unavailable" | "native_apply_unavailable" => "writer_review_unavailable",
            other => other,
        })
    }
    fn capture_with(
        &self,
        field: isize,
        epoch: u64,
        api: Arc<dyn Api>,
    ) -> Result<TargetCapture, &'static str> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "native_apply_state_unavailable")?;
        {
            let state = self
                .registry
                .lock()
                .map_err(|_| "native_apply_state_unavailable")?;
            if state.epoch != epoch || epoch == u64::MAX {
                return Err("native_capture_cancelled");
            }
        }
        self.close_locked()?;
        let cookie = {
            let mut state = self
                .registry
                .lock()
                .map_err(|_| "native_apply_state_unavailable")?;
            if state.epoch != epoch {
                return Err("native_capture_cancelled");
            }
            let cookie = api.create(field)?; // Allocation only, no target I/O.
            state.slot = Some(Slot {
                api: api.clone(),
                cookie,
                token: None,
            });
            cookie
        };
        let result = api.capture(cookie);
        let current = self
            .registry
            .lock()
            .map_err(|_| "native_apply_state_unavailable")?
            .epoch
            == epoch;
        match result {
            Ok(text) if current => Ok(TargetCapture {
                text,
                cookie,
                epoch,
            }),
            result => {
                api.cancel(cookie);
                let _ = self.close_locked(); // On failure the slot stays owned.
                Err(result.err().unwrap_or("native_capture_cancelled"))
            }
        }
    }
    pub fn bind(&self, capture: &TargetCapture, token: SessionToken) -> Result<(), &'static str> {
        let mut state = self
            .registry
            .lock()
            .map_err(|_| "native_apply_state_unavailable")?;
        if state.epoch != capture.epoch {
            return Err("native_capture_cancelled");
        }
        let slot = state
            .slot
            .as_mut()
            .filter(|slot| slot.cookie == capture.cookie && slot.token.is_none())
            .ok_or("native_capture_cancelled")?;
        slot.token = Some(token);
        Ok(())
    }
    pub fn supports(&self, token: &SessionToken) -> bool {
        self.registry.lock().ok().is_some_and(|state| {
            state
                .slot
                .as_ref()
                .is_some_and(|slot| slot.token.as_ref() == Some(token))
        })
    }
    pub fn apply(&self, token: &SessionToken, text: &str) -> DirectOutcome {
        let Ok(_operation) = self.operation.lock() else {
            return DirectOutcome::Rejected;
        };
        let (api, cookie) = {
            let Ok(mut state) = self.registry.lock() else {
                return DirectOutcome::Rejected;
            };
            let Some(slot) = state
                .slot
                .as_mut()
                .filter(|slot| slot.token.as_ref() == Some(token))
            else {
                return DirectOutcome::Rejected;
            };
            slot.token = None; // One explicit command, consumed before target I/O.
            (slot.api.clone(), slot.cookie)
        };
        let result = api.apply(cookie, text);
        // This physical result must survive a concurrently ended UI session.
        // A cleanup failure does not turn an applied/uncertain result into rejection.
        let _ = self.close_locked();
        result
    }
    pub fn cancel(&self) {
        if let Ok(mut state) = self.registry.lock() {
            // Exhaustion is fail closed: no epoch can be reused after overflow.
            state.epoch = state.epoch.checked_add(1).unwrap_or(u64::MAX);
            if let Some(slot) = state.slot.as_mut() {
                slot.token = None;
                slot.api.cancel(slot.cookie);
            }
        }
    }
    pub fn cleanup(&self) -> Result<(), &'static str> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "native_apply_state_unavailable")?;
        self.close_locked()
    }
    fn close_locked(&self) -> Result<(), &'static str> {
        let owner = {
            let state = self
                .registry
                .lock()
                .map_err(|_| "native_apply_state_unavailable")?;
            state
                .slot
                .as_ref()
                .map(|slot| (slot.api.clone(), slot.cookie))
        };
        if let Some((api, cookie)) = owner {
            api.cancel(cookie);
            if !api.close(cookie) {
                return Err("native_apply_cleanup_pending");
            }
            self.registry
                .lock()
                .map_err(|_| "native_apply_state_unavailable")?
                .slot = None;
        }
        Ok(())
    }
}
impl Drop for TargetApplyRuntime {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(windows)]
fn load_api() -> Result<Arc<dyn Api>, &'static str> {
    // The module stays loaded for the broker process lifetime. An outstanding
    // native slot can outlive a caller timeout; unloading would invalidate it.
    static API: OnceLock<Result<Arc<dyn Api>, &'static str>> = OnceLock::new();
    API.get_or_init(|| unsafe { WindowsApi::load(0).map(|api| Arc::new(api) as Arc<dyn Api>) })
        .clone()
}
#[cfg(windows)]
fn load_browser_api() -> Result<Arc<dyn Api>, &'static str> {
    static API: OnceLock<Result<Arc<dyn Api>, &'static str>> = OnceLock::new();
    API.get_or_init(|| unsafe { WindowsApi::load(1).map(|api| Arc::new(api) as Arc<dyn Api>) })
        .clone()
}
#[cfg(windows)]
fn load_writer_api() -> Result<Arc<dyn Api>, &'static str> {
    static API: OnceLock<Result<Arc<dyn Api>, &'static str>> = OnceLock::new();
    API.get_or_init(|| unsafe { WindowsApi::load(2).map(|api| Arc::new(api) as Arc<dyn Api>) }).clone()
}
#[cfg(not(windows))]
fn load_writer_api() -> Result<Arc<dyn Api>, &'static str> { Err("writer_review_unavailable") }
#[cfg(not(windows))]
fn load_browser_api() -> Result<Arc<dyn Api>, &'static str> {
    Err("browser_review_unavailable")
}
#[cfg(not(windows))]
fn load_api() -> Result<Arc<dyn Api>, &'static str> {
    Err("native_apply_unavailable")
}

#[cfg(windows)]
struct WindowsApi {
    max_units: usize,
    create: unsafe extern "system" fn(isize, *mut u64) -> u32,
    capture: unsafe extern "system" fn(u64, *mut u16, u32, *mut u32) -> u32,
    apply: unsafe extern "system" fn(u64, *const u16, u32, *mut u32) -> u32,
    cancel: unsafe extern "system" fn(u64) -> u32,
    close: unsafe extern "system" fn(u64, u32) -> u32,
}
#[cfg(windows)]
impl WindowsApi {
    unsafe fn load(kind: u8) -> Result<Self, &'static str> {
        use windows_sys::Win32::System::LibraryLoader::*;
        let path = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.join("target-apply.dll")))
            .ok_or("native_apply_unavailable")?;
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let module = LoadLibraryExW(
            path.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if module.is_null() {
            return Err("native_apply_unavailable");
        }
        macro_rules! symbol {
            ($name:literal,$browser:literal,$writer:literal,$ty:ty) => {{
                let name = match kind { 1 => concat!($browser, "\0"), 2 => concat!($writer, "\0"), _ => concat!($name, "\0") };
                match GetProcAddress(module, name.as_ptr()) {
                    Some(function) => {
                        std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(function)
                    }
                    None => {
                        windows_sys::Win32::Foundation::FreeLibrary(module);
                        return Err("native_apply_incompatible");
                    }
                }
            }};
        }
        Ok(Self {
            max_units: if kind != 0 { 8192 } else { 65534 },
            create: symbol!(
                "GrammarTargetCreate",
                "GrammarBrowserClientCreate", "GrammarWriterClientCreate",
                unsafe extern "system" fn(isize, *mut u64) -> u32
            ),
            capture: symbol!(
                "GrammarTargetCapture",
                "GrammarBrowserClientCapture", "GrammarWriterClientCapture",
                unsafe extern "system" fn(u64, *mut u16, u32, *mut u32) -> u32
            ),
            apply: symbol!(
                "GrammarTargetApply",
                "GrammarBrowserClientApply", "GrammarWriterClientApply",
                unsafe extern "system" fn(u64, *const u16, u32, *mut u32) -> u32
            ),
            cancel: symbol!("GrammarTargetCancel", "GrammarBrowserClientCancel", "GrammarWriterClientCancel", unsafe extern "system" fn(u64) -> u32),
            close: symbol!(
                "GrammarTargetClose",
                "GrammarBrowserClientClose", "GrammarWriterClientClose",
                unsafe extern "system" fn(u64, u32) -> u32
            ),
        })
    }
}
#[cfg(windows)]
impl Api for WindowsApi {
    fn create(&self, field: isize) -> Result<u64, &'static str> {
        let mut cookie = 0;
        if unsafe { (self.create)(field, &mut cookie) } == 0 && cookie != 0 {
            Ok(cookie)
        } else {
            Err("native_apply_unavailable")
        }
    }
    fn capture(&self, cookie: u64) -> Result<String, &'static str> {
        let mut text = vec![0u16; self.max_units + 1];
        let mut units = 0;
        let code =
            unsafe { (self.capture)(cookie, text.as_mut_ptr(), text.len() as u32, &mut units) };
        let result = if code == 0 && units > 0 && units as usize <= self.max_units {
            String::from_utf16(&text[..units as usize]).map_err(|_| "native_capture_invalid_text")
        } else {
            Err("native_capture_unavailable")
        };
        for unit in &mut text {
            unsafe {
                std::ptr::write_volatile(unit, 0);
            }
        }
        result
    }
    fn apply(&self, cookie: u64, text: &str) -> DirectOutcome {
        let mut text: Vec<u16> = text.encode_utf16().collect();
        if text.is_empty() || text.len() > self.max_units || text.contains(&0) {
            return DirectOutcome::Rejected;
        }
        let mut outcome = 4;
        let code = unsafe { (self.apply)(cookie, text.as_ptr(), text.len() as u32, &mut outcome) };
        for unit in &mut text {
            unsafe {
                std::ptr::write_volatile(unit, 0);
            }
        }
        match (code, outcome) {
            (0, 3) => DirectOutcome::Applied,
            (0, 0) => DirectOutcome::Rejected,
            _ => DirectOutcome::Indeterminate,
        }
    }
    fn cancel(&self, cookie: u64) {
        unsafe {
            (self.cancel)(cookie);
        }
    }
    fn close(&self, cookie: u64) -> bool {
        unsafe { (self.close)(cookie, 5000) == 0 }
    }
}

/// Admission reads only window metadata. The target service independently
/// checks input scope/protection before any document text read.
fn direct_control_candidate(class_name: &str, style: u32) -> bool {
    class_name.eq_ignore_ascii_case("RichEdit50W")
        || (class_name.eq_ignore_ascii_case("Edit") && (style & 0x0004 == 0 || style & 0x0040 != 0))
}

#[cfg(windows)]
pub(crate) fn is_browser_window(target: crate::capture_session::WindowTarget) -> Result<bool, String> {
    use crate::native_edit::{EditPort, Win32EditPort};
    let mut port = Win32EditPort::default();
    if port.foreground_window() != target.hwnd || port.window_pid(target.hwnd) != Some(target.pid) {
        return Err("capture_target_changed".into());
    }
    Ok(port.class_name(target.hwnd).and_then(|name| String::from_utf16(&name).ok())
        .is_some_and(|name| name == "Chrome_WidgetWin_1"))
}
#[cfg(not(windows))]
pub(crate) fn is_browser_window(_: crate::capture_session::WindowTarget) -> Result<bool, String> { Ok(false) }

#[cfg(windows)]
pub(crate) fn is_writer_window(target: crate::capture_session::WindowTarget) -> Result<bool, String> {
    use crate::native_edit::{EditPort, Win32EditPort};
    let mut port = Win32EditPort::default();
    if port.foreground_window() != target.hwnd || port.window_pid(target.hwnd) != Some(target.pid) {
        return Err("capture_target_changed".into());
    }
    // Metadata routing only. Native admission independently pins executable
    // identity; the opted-in target core checks actual Writer document eligibility.
    Ok(port.class_name(target.hwnd).and_then(|name| String::from_utf16(&name).ok())
        .is_some_and(|name| name == "SALFRAME"))
}
#[cfg(not(windows))]
pub(crate) fn is_writer_window(_: crate::capture_session::WindowTarget) -> Result<bool, String> { Ok(false) }

#[cfg(windows)]
pub(crate) fn focused_field(
    target: crate::capture_session::WindowTarget,
) -> Result<Option<isize>, String> {
    use crate::native_edit::{is_sensitive_process, EditPort, Win32EditPort};
    let mut port = Win32EditPort::default();
    if port.foreground_window() != target.hwnd || port.window_pid(target.hwnd) != Some(target.pid) {
        return Err("capture_target_changed".into());
    }
    let Some(field) = port.focused_control(target.hwnd) else {
        return Ok(None);
    };
    let style = port.style(field);
    if !port
        .class_name(field)
        .and_then(|name| String::from_utf16(&name).ok())
        .is_some_and(|name| direct_control_candidate(&name, style))
    {
        return Ok(None);
    }
    if port.root_window(field) != Some(target.hwnd)
        || port.window_pid(field) != Some(target.pid)
        || !port.is_unicode(field)
        || !port.is_visible(field)
        || !port.is_enabled(field)
        || style & (0x20 | 0x800) != 0
    {
        return Err("native_sensitive_or_unavailable_editor".into());
    }
    let image = port
        .process_image_name(target.pid)
        .ok_or("native_target_unavailable")?;
    if is_sensitive_process(&image) {
        return Err("native_sensitive_editor".into());
    }
    Ok(Some(field))
}
#[cfg(not(windows))]
pub(crate) fn focused_field(
    _: crate::capture_session::WindowTarget,
) -> Result<Option<isize>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    #[test]
    fn direct_control_admission_requires_multiline_autovscroll_and_rejects_class_guesses() {
        assert!(direct_control_candidate("Edit", 0));
        assert!(!direct_control_candidate("Edit", 0x0004));
        assert!(direct_control_candidate("Edit", 0x0044));
        assert!(direct_control_candidate("RichEdit50W", 0x0004));
        for name in ["_WwG", "EXCEL7", "HwpCtrl", "Chrome_RenderWidgetHostHWND", "DOCX", "Text"] {
            assert!(!direct_control_candidate(name, 0));
        }
    }
    #[derive(Default)]
    struct Fake {
        creates: AtomicU64,
        applies: AtomicU64,
        closes: AtomicU64,
        cancelled: AtomicBool,
        cleanup_pending: AtomicBool,
    }
    impl Api for Fake {
        fn create(&self, _: isize) -> Result<u64, &'static str> {
            self.cancelled.store(false, Ordering::SeqCst);
            Ok(self.creates.fetch_add(1, Ordering::SeqCst) + 1)
        }
        fn capture(&self, _: u64) -> Result<String, &'static str> {
            Ok("selected".into())
        }
        fn apply(&self, _: u64, _: &str) -> DirectOutcome {
            self.applies.fetch_add(1, Ordering::SeqCst);
            DirectOutcome::Applied
        }
        fn cancel(&self, _: u64) {
            self.cancelled.store(true, Ordering::SeqCst);
        }
        fn close(&self, _: u64) -> bool {
            self.closes.fetch_add(1, Ordering::SeqCst);
            !self.cleanup_pending.load(Ordering::SeqCst)
        }
    }
    fn token(generation: u64) -> SessionToken {
        SessionToken {
            session_id: "owned".into(),
            generation,
        }
    }
    fn captured(runtime: &TargetApplyRuntime, api: &Arc<Fake>, generation: u64) {
        let c = runtime
            .capture_with(1, runtime.epoch().unwrap(), api.clone())
            .unwrap();
        runtime.bind(&c, token(generation)).unwrap();
    }
    #[test]
    fn target_apply_cancellation_does_not_relabel_committed_physical_result() {
        struct InFlight {
            entered: std::sync::Barrier,
            finish: std::sync::Barrier,
        }
        impl Api for InFlight {
            fn create(&self, _: isize) -> Result<u64, &'static str> {
                Ok(1)
            }
            fn capture(&self, _: u64) -> Result<String, &'static str> {
                Ok("selected".into())
            }
            fn apply(&self, _: u64, _: &str) -> DirectOutcome {
                self.entered.wait();
                self.finish.wait();
                DirectOutcome::Applied
            }
            fn cancel(&self, _: u64) {}
            fn close(&self, _: u64) -> bool {
                true
            }
        }
        let api = Arc::new(InFlight {
            entered: std::sync::Barrier::new(2),
            finish: std::sync::Barrier::new(2),
        });
        let runtime = Arc::new(TargetApplyRuntime::default());
        let c = runtime.capture_with(1, 0, api.clone()).unwrap();
        runtime.bind(&c, token(1)).unwrap();
        let worker = runtime.clone();
        let job = std::thread::spawn(move || worker.apply(&token(1), "draft"));
        api.entered.wait();
        runtime.cancel();
        api.finish.wait();
        assert_eq!(job.join().unwrap(), DirectOutcome::Applied);
        assert!(!runtime.supports(&token(1)));
    }
    #[test]
    fn target_apply_wrong_token_and_replay_never_reach_native() {
        let runtime = TargetApplyRuntime::default();
        let api = Arc::new(Fake::default());
        captured(&runtime, &api, 1);
        assert_eq!(
            runtime.apply(&token(2), "edited draft"),
            DirectOutcome::Rejected
        );
        assert_eq!(api.applies.load(Ordering::SeqCst), 0);
        assert_eq!(
            runtime.apply(&token(1), "edited draft"),
            DirectOutcome::Applied
        );
        assert_eq!(
            runtime.apply(&token(1), "edited draft"),
            DirectOutcome::Rejected
        );
        assert_eq!(api.applies.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn target_apply_cancel_revokes_and_old_queued_capture_cannot_close_new_owner() {
        let runtime = TargetApplyRuntime::default();
        let api = Arc::new(Fake::default());
        let old_epoch = runtime.epoch().unwrap();
        captured(&runtime, &api, 1);
        runtime.cancel();
        assert!(!runtime.supports(&token(1)));
        assert_eq!(runtime.apply(&token(1), "draft"), DirectOutcome::Rejected);
        captured(&runtime, &api, 2);
        let closes = api.closes.load(Ordering::SeqCst);
        assert!(runtime.capture_with(1, old_epoch, api.clone()).is_err());
        assert_eq!(api.closes.load(Ordering::SeqCst), closes);
        assert!(runtime.supports(&token(2)));
    }
    #[test]
    fn target_apply_cleanup_pending_retains_slot_and_blocks_new_allocation() {
        let runtime = TargetApplyRuntime::default();
        let api = Arc::new(Fake::default());
        captured(&runtime, &api, 1);
        api.cleanup_pending.store(true, Ordering::SeqCst);
        assert_eq!(runtime.apply(&token(1), "draft"), DirectOutcome::Applied);
        assert_eq!(runtime.cleanup(), Err("native_apply_cleanup_pending"));
        assert!(runtime
            .capture_with(1, runtime.epoch().unwrap(), api.clone())
            .is_err());
        assert_eq!(api.creates.load(Ordering::SeqCst), 1);
        api.cleanup_pending.store(false, Ordering::SeqCst);
        assert!(runtime.cleanup().is_ok());
        captured(&runtime, &api, 2);
        assert_eq!(api.creates.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn target_apply_cancel_between_capture_and_bind_does_not_publish_authority() {
        let runtime = TargetApplyRuntime::default();
        let api = Arc::new(Fake::default());
        let c = runtime
            .capture_with(1, runtime.epoch().unwrap(), api.clone())
            .unwrap();
        runtime.cancel();
        assert!(runtime.bind(&c, token(1)).is_err());
        assert!(!runtime.supports(&token(1)));
        assert_eq!(api.applies.load(Ordering::SeqCst), 0);
    }
}
