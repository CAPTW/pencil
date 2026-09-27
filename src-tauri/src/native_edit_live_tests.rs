//! Owned-desktop qualification of standard Edit capture (step 2) and verified
//! native Apply (step 3). Every editor is a task-owned synthetic process whose
//! native `Edit` controls count text-bearing messages sent from other threads,
//! so "never read" is observed at the target rather than inferred.

use crate::apply_safety::{
    apply_current_session, ApplyFallbackReason, ApplyOutcome, ApplyPlatform, WaitStage,
};
use crate::capture_session::{CaptureSessionStore, NativeEditBinding, SessionToken, WindowTarget};
use crate::clipboard::{self, ClipboardCapture};
use crate::native_edit::NativeApplyResult;
use crate::p0_02_windows_live_tests::{
    assert_editor_cleanups_complete, focused_window, require_cloud_owned_desktop, wait_until,
    ClipboardTextGuard, CloudTestReceipt, OwnedEditorProcess, EDITOR_CLEANUPS,
};
use crate::windows_apply::{
    apply_native_edit_win32, foreground_window_handle, request_foreground_window, wait_for_stage,
    window_is_valid, window_process_id,
};
use crate::windows_target::{capture_foreground_target, WindowsForegroundTargetPlatform};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, IsWindowUnicode, PostMessageW, SendMessageTimeoutW, SendMessageW, GWL_STYLE,
    SMTO_ABORTIFHUNG, WM_CLOSE,
};

const WM_SETTEXT: u32 = 0x000C;
const WM_GETTEXT: u32 = 0x000D;
const WM_GETTEXTLENGTH: u32 = 0x000E;
const WM_CHAR: u32 = 0x0102;
const EM_SETSEL: u32 = 0x00B1;
const EM_GETMODIFY: u32 = 0x00B8;
const EM_SETMODIFY: u32 = 0x00B9;
const EM_CANUNDO: u32 = 0x00C6;
const EM_UNDO: u32 = 0x00C7;
const ES_READONLY: u32 = 0x0800;
const HARNESS_QUERY: u32 = 0x8101;
const HARNESS_RESET: u32 = 0x8102;
const HARNESS_FOCUS: u32 = 0x8103;
const HARNESS_ARM_SELECTION_RACE: u32 = 0x8104;
const HARNESS_ARM_APP_EDIT: u32 = 0x8105;
const HARNESS_APP_TIMER: u32 = 0x8106;
// Application edit modes of the harness.
const APP_EDIT_AFTER_SELECTION_READ: isize = 1;
const APP_EDIT_BEFORE_SETTEXT: isize = 2;

// Counter kinds recorded by the harness for messages sent from another thread.
const KIND_GETTEXT: usize = 0;
const KIND_GETTEXTLENGTH: usize = 1;
const KIND_GETSEL: usize = 2;
const KIND_REPLACESEL: usize = 3;
const KIND_LOCK: usize = 4;
const KIND_UNLOCK: usize = 5;
const KIND_SETTEXT: usize = 6;
const KIND_UNDO: usize = 7;
const KIND_SETSEL: usize = 8;
const KIND_SETMODIFY: usize = 9;
/// EN_CHANGE the application received while another process's message ran.
const KIND_FOREIGN_CHANGE_NOTICE: usize = 10;
/// Edits made by the application itself.
const KIND_APP_EDITS: usize = 11;
/// Messages that change the editor's text, selection, lock or flags.
const STATE_CHANGING_KINDS: [usize; 7] =
    [KIND_REPLACESEL, KIND_LOCK, KIND_UNLOCK, KIND_SETTEXT, KIND_UNDO, KIND_SETSEL, KIND_SETMODIFY];

// Editor indexes in the harness.
const PLAIN: usize = 0;
const PASSWORD: usize = 1;
const READ_ONLY: usize = 2;
const ANSI: usize = 3;
const WINFORMS: usize = 4;
const OTHER: usize = 5;

const SAMPLE: &str = "첫 줄 synthetic\r\n한국어 선택 😀 끝\r\nthird line";
const RACE_ITERATIONS: u64 = 12;

const HARNESS_SCRIPT: &str = include_str!("../tests/native_edit_harness.ps1");

/// Task-owned synthetic process with real `Edit` controls: 0 multiline,
/// 1 password, 2 read-only, 3 ANSI, 4 WinForms superclass, and 5 in a second
/// top-level window.
struct NativeEditHarness {
    editor: OwnedEditorProcess,
    form: isize,
    other_form: isize,
    edits: [isize; 6],
}

impl NativeEditHarness {
    fn spawn() -> Self {
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-STA", "-Command", HARNESS_SCRIPT])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut editor = OwnedEditorProcess::spawn(&mut command)
            .unwrap_or_else(|error| panic!("native edit harness: {error}"));
        let stdout = editor.child.stdout.take().expect("native edit harness stdout");
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(ready) = line.strip_prefix("NATIVE_READY:") {
                    let handles = ready
                        .split(':')
                        .map(|value| value.parse::<isize>())
                        .collect::<Result<Vec<_>, _>>();
                    let _ = sender.send(handles.ok());
                    return;
                }
            }
        });
        let handles = receiver
            .recv_timeout(Duration::from_secs(60))
            .ok()
            .flatten()
            .filter(|handles| handles.len() == 8)
            .unwrap_or_else(|| panic!("native edit harness did not become ready"));
        Self {
            editor,
            form: handles[0],
            edits: [handles[1], handles[2], handles[3], handles[4], handles[5], handles[7]],
            other_form: handles[6],
        }
    }

    fn edit(&self, index: usize) -> isize {
        self.edits[index]
    }

    fn form_of(&self, index: usize) -> isize {
        if index == OTHER {
            self.other_form
        } else {
            self.form
        }
    }

    fn counter(&self, index: usize, kind: usize) -> usize {
        let value = unsafe { SendMessageW(self.form as HWND, HARNESS_QUERY, index * 16 + kind, 0) };
        usize::try_from(value).unwrap_or(usize::MAX)
    }

    /// Text-bearing messages the control received from another thread.
    fn text_messages(&self, index: usize) -> usize {
        [KIND_GETTEXT, KIND_GETTEXTLENGTH, KIND_GETSEL]
            .into_iter()
            .map(|kind| self.counter(index, kind))
            .sum()
    }

    fn reset_counters(&self) {
        assert_eq!(unsafe { SendMessageW(self.form as HWND, HARNESS_RESET, 0, 0) }, 1);
    }

    fn set_text(&self, index: usize, text: &str) {
        let wide = text.encode_utf16().chain([0]).collect::<Vec<_>>();
        unsafe {
            SendMessageW(self.edit(index) as HWND, WM_SETTEXT, 0, wide.as_ptr() as isize);
        }
    }

    fn text(&self, index: usize) -> String {
        let hwnd = self.edit(index) as HWND;
        let length = unsafe { SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0) } as usize;
        let mut buffer = vec![0u16; length + 1];
        let copied =
            unsafe { SendMessageW(hwnd, WM_GETTEXT, buffer.len(), buffer.as_mut_ptr() as isize) };
        String::from_utf16(&buffer[..copied as usize]).expect("synthetic text is valid UTF-16")
    }

    fn select(&self, index: usize, start: usize, end: usize) {
        unsafe {
            SendMessageW(self.edit(index) as HWND, EM_SETSEL, start, end as isize);
        }
    }

    fn style(&self, index: usize) -> u32 {
        unsafe { GetWindowLongW(self.edit(index) as HWND, GWL_STYLE) as u32 }
    }

    fn selection(&self, index: usize) -> (usize, usize) {
        const EM_GETSEL: u32 = 0x00B0;
        let packed = unsafe { SendMessageW(self.edit(index) as HWND, EM_GETSEL, 0, 0) } as usize;
        (packed & 0xFFFF, (packed >> 16) & 0xFFFF)
    }

    /// Makes `index` the focused control of its foreground top-level window.
    fn focus(&self, index: usize) {
        let form = self.form_of(index);
        let edit = self.edit(index);
        for _ in 0..4 {
            unsafe {
                SendMessageW(form as HWND, HARNESS_FOCUS, index, 0);
            }
            request_foreground_window(form);
            if wait_until(Duration::from_secs(1), || {
                foreground_window_handle() == form && focused_window(form) == Some(edit)
            }) {
                return;
            }
        }
        panic!("synthetic editor {index} could not be focused");
    }

    /// Arms the harness to move the selection right after Apply's locked EM_SETSEL.
    fn arm_selection_race(&self, index: usize) {
        assert_eq!(
            unsafe { SendMessageW(self.form as HWND, HARNESS_ARM_SELECTION_RACE, index, 0) },
            1
        );
    }

    /// As `arm_selection_race`, moving the selection to `[start, end)`.
    fn arm_selection_race_to(&self, index: usize, start: usize, end: usize) {
        let target = (start | (end << 16)) as isize;
        assert_eq!(
            unsafe { SendMessageW(self.form as HWND, HARNESS_ARM_SELECTION_RACE, index, target) },
            1
        );
    }

    /// Arms one edit by the application itself inside its own message
    /// handling; a timer in the application makes it anyway if the triggering
    /// message never arrives. Mode 0 disarms.
    fn arm_app_edit(&self, index: usize, mode: isize) {
        assert_eq!(unsafe { SendMessageW(self.form as HWND, HARNESS_ARM_APP_EDIT, index, mode) }, 1);
    }

    /// Starts (interval > 0) or stops an application timer that toggles one word.
    fn app_timer(&self, index: usize, interval_ms: isize) {
        assert_eq!(unsafe { SendMessageW(self.form as HWND, HARNESS_APP_TIMER, index, interval_ms) }, 1);
    }

    fn disarm_races(&self) {
        // An index no editor has (the harness reads wParam as a 32-bit value).
        unsafe {
            SendMessageW(self.form as HWND, HARNESS_ARM_SELECTION_RACE, 0xFFFF, 0);
        }
        self.arm_app_edit(PLAIN, 0);
        self.app_timer(PLAIN, 0);
    }

    fn state_changes(&self, index: usize) -> usize {
        STATE_CHANGING_KINDS.iter().map(|&kind| self.counter(index, kind)).sum()
    }

    fn close_other_form(&self) {
        unsafe {
            PostMessageW(self.other_form as HWND, WM_CLOSE, 0, 0);
        }
        assert!(wait_until(Duration::from_secs(5), || !window_is_valid(self.other_form)));
    }
}

impl Drop for NativeEditHarness {
    fn drop(&mut self) {
        let cleanup = self.editor.shutdown(&[self.form, self.other_form]);
        EDITOR_CLEANUPS.with(|log| log.borrow_mut().push(cleanup));
    }
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

fn utf16_range(haystack: &str, needle: &str) -> (usize, usize) {
    let byte = haystack.find(needle).expect("synthetic needle");
    let start = units(&haystack[..byte]);
    (start, start + units(needle))
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("test runtime")
}

/// The production hotkey capture path after the shortcut fires: foreground
/// target capture, then the bounded standard Edit reader.
fn production_capture() -> Result<(WindowTarget, ClipboardCapture), String> {
    let mut platform = WindowsForegroundTargetPlatform;
    let target = capture_foreground_target(&mut platform, -1, std::process::id())
        .map_err(|error| format!("{error:?}"))?;
    let capture = runtime().block_on(clipboard::capture_selected_text_for(target))?;
    Ok((target, capture))
}

fn capture_ok() -> (WindowTarget, ClipboardCapture) {
    production_capture().unwrap_or_else(|error| panic!("supported Edit capture failed: {error}"))
}

fn capture_denied() -> String {
    match production_capture() {
        Ok((_, capture)) => panic!("capture unexpectedly returned {} units", units(&capture.selected_text)),
        Err(error) => error,
    }
}

#[test]
#[ignore = "mutates only an explicitly opted-in GitHub-hosted synthetic desktop"]
fn cloud_owned_desktop_native_capture() {
    let mut receipt = CloudTestReceipt::new("cloud_owned_desktop_native_capture");
    require_cloud_owned_desktop();
    receipt.check("owned_desktop_opt_in");
    assert!(clipboard::write_clipboard_text("synthetic-native-prior").is_ok());
    let _clipboard_guard = ClipboardTextGuard::capture().expect("synthetic clipboard roundtrip");
    let clipboard_sequence = clipboard::clipboard_sequence_number();
    let harness = NativeEditHarness::spawn();
    assert_eq!(unsafe { IsWindowUnicode(harness.edit(ANSI) as HWND) }, 0);
    assert_ne!(unsafe { IsWindowUnicode(harness.edit(PLAIN) as HWND) }, 0);
    receipt.check("owned_native_editor_ready_with_unicode_and_ansi_edits");

    harness.set_text(PLAIN, SAMPLE);
    let selected = "한국어 선택 😀 끝\r\nthird";
    let (start, end) = utf16_range(SAMPLE, selected);
    harness.select(PLAIN, start, end);
    harness.focus(PLAIN);
    let (target, capture) = capture_ok();
    assert_eq!(capture.selected_text, selected);
    let binding = capture.native_edit.expect("native capture binds its editor");
    assert_eq!(binding.edit, harness.edit(PLAIN));
    assert_eq!((binding.start, binding.end, binding.text_units), (start, end, units(SAMPLE)));
    assert_eq!(binding.pid, target.pid);
    assert!(!binding.read_only);
    receipt.check("korean_crlf_surrogate_exact_selection");

    harness.select(PLAIN, 0, units(SAMPLE));
    assert_eq!(capture_ok().1.selected_text, SAMPLE);
    receipt.check("whole_field_with_crlf_exact");

    for needle in ["첫 줄", "third line", "😀", "\r\n"] {
        let (start, end) = utf16_range(SAMPLE, needle);
        harness.select(PLAIN, start, end);
        assert_eq!(capture_ok().1.selected_text, needle);
    }
    receipt.check("reselection_returns_only_current_selection");

    harness.select(PLAIN, 3, 3);
    harness.reset_counters();
    let denied = capture_denied();
    assert!(denied.starts_with("native_no_selection"), "{denied}");
    assert_eq!(harness.counter(PLAIN, KIND_GETTEXT), 0);
    receipt.check("empty_selection_denied_without_text_read");

    let (emoji, _) = utf16_range(SAMPLE, "😀");
    harness.select(PLAIN, emoji, emoji + 1);
    let denied = capture_denied();
    assert!(denied.starts_with("native_selection_splits_character"), "{denied}");
    receipt.check("split_surrogate_selection_denied");

    for (index, text, prefix, check) in [
        (PASSWORD, "synthetic-secret-value", "native_sensitive_editor", "password_edit_denied_with_zero_text_messages"),
        (WINFORMS, "synthetic superclassed editor", "native_unsupported_editor", "superclassed_edit_denied_with_zero_text_messages"),
        (ANSI, "synthetic ansi editor", "native_unsupported_editor", "ansi_edit_denied_with_zero_text_messages"),
    ] {
        harness.set_text(index, text);
        harness.select(index, 0, units(text));
        harness.focus(index);
        harness.reset_counters();
        let denied = capture_denied();
        assert!(denied.starts_with(prefix), "{check}: {denied}");
        assert_eq!(harness.text_messages(index), 0, "{check}");
        receipt.check(check);
    }

    harness.set_text(READ_ONLY, "synthetic read-only field");
    harness.select(READ_ONLY, 10, 19);
    harness.focus(READ_ONLY);
    let capture = capture_ok().1;
    assert_eq!(capture.selected_text, "read-only");
    assert!(capture.native_edit.expect("read-only binding").read_only);
    receipt.check("read_only_field_captured_and_marked_copy_only");

    harness.select(PLAIN, 0, 3);
    harness.focus(PLAIN);
    let mut platform = WindowsForegroundTargetPlatform;
    let target = capture_foreground_target(&mut platform, -1, std::process::id())
        .expect("foreground target");
    harness.set_text(OTHER, "synthetic other editor");
    harness.select(OTHER, 0, 9);
    harness.focus(OTHER);
    harness.reset_counters();
    let switched = runtime().block_on(clipboard::capture_selected_text_for(target));
    assert!(matches!(&switched, Err(error) if error.starts_with("capture_target_changed")));
    assert_eq!(harness.text_messages(PLAIN) + harness.text_messages(OTHER), 0);
    receipt.check("window_switch_denied_without_text_read");

    assert_eq!(clipboard::clipboard_sequence_number(), clipboard_sequence);
    receipt.check("capture_never_touched_clipboard");

    drop(harness);
    receipt.record_cleanups(assert_editor_cleanups_complete(1));
    receipt.check("owned_editor_cleanup");
    receipt.pass();
}

/// Mirrors the production Windows platform; there is no widget in this harness.
#[derive(Default)]
struct NativeLivePlatform {
    clipboard_writes: usize,
    native_results: Vec<NativeApplyResult>,
}

impl ApplyPlatform for NativeLivePlatform {
    fn apply_native_edit(
        &mut self,
        top_level: isize,
        pid: u32,
        binding: &NativeEditBinding,
        replacement: &str,
    ) -> NativeApplyResult {
        let result = apply_native_edit_win32(top_level, pid, binding, replacement);
        self.native_results.push(result);
        result
    }

    fn is_window(&mut self, hwnd: isize) -> bool {
        window_is_valid(hwnd)
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
        window_process_id(hwnd)
    }

    fn hide_widget(&mut self) -> Result<(), ()> {
        Ok(())
    }

    fn show_widget(&mut self) {}

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
        self.clipboard_writes += 1;
        clipboard::write_clipboard_text(text).map_err(|_| ())
    }

    fn send_paste(&mut self) -> u32 {
        panic!("native Apply never injects paste input");
    }

    fn wait(&mut self, stage: WaitStage) {
        wait_for_stage(stage);
    }
}

/// Captures the current selection and drives the session to `Ready`.
fn ready_native_session() -> (CaptureSessionStore, SessionToken, WindowTarget) {
    let (target, capture) = capture_ok();
    let binding = capture.native_edit.expect("native binding");
    let mut store = CaptureSessionStore::default();
    let token = store
        .capture_native(
            "synthetic-native-apply".to_string(),
            capture.selected_text,
            target,
            binding,
        )
        .expect("native session");
    assert!(store.begin_rewrite(&token).is_ok());
    assert!(store.finish_rewrite_success(&token).is_ok());
    (store, token, target)
}

fn prepare(harness: &NativeEditHarness, index: usize, text: &str, selected: &str) {
    harness.set_text(index, text);
    let (start, end) = utf16_range(text, selected);
    harness.select(index, start, end);
    harness.focus(index);
}

fn flush_posted_input(harness: &NativeEditHarness) {
    // Posted input drains before the target idles; a sent message then syncs.
    thread::sleep(Duration::from_millis(150));
    let mut result = 0usize;
    unsafe {
        SendMessageTimeoutW(
            harness.edit(PLAIN) as HWND,
            WM_GETTEXTLENGTH,
            0,
            0,
            SMTO_ABORTIFHUNG,
            2000,
            &mut result,
        );
    }
}

/// Small deterministic generator for race timing; no external randomness.
fn jitter(iteration: u64) -> Duration {
    let mixed = iteration.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
    Duration::from_micros((mixed >> 33) % 6_000)
}

#[test]
#[ignore = "mutates only an explicitly opted-in GitHub-hosted synthetic desktop"]
fn cloud_owned_desktop_native_apply() {
    let mut receipt = CloudTestReceipt::new("cloud_owned_desktop_native_apply");
    require_cloud_owned_desktop();
    receipt.check("owned_desktop_opt_in");
    assert!(clipboard::write_clipboard_text("synthetic-native-prior").is_ok());
    let _clipboard_guard = ClipboardTextGuard::capture().expect("synthetic clipboard roundtrip");
    let harness = NativeEditHarness::spawn();
    receipt.check("owned_native_editor_ready");

    let selected = "한국어 선택 😀";
    let replacement = "교정된 문장 ✅";
    prepare(&harness, PLAIN, SAMPLE, selected);
    let (mut store, token, _) = ready_native_session();
    let clipboard_sequence = clipboard::clipboard_sequence_number();
    harness.reset_counters();
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, replacement, false, &mut platform),
        ApplyOutcome::Applied
    );
    assert_eq!(harness.counter(PLAIN, KIND_LOCK), 1);
    assert_eq!(harness.counter(PLAIN, KIND_REPLACESEL), 1);
    assert!(harness.counter(PLAIN, KIND_UNLOCK) >= 1);
    assert_eq!(harness.style(PLAIN) & ES_READONLY, 0);
    assert_eq!(harness.text(PLAIN), SAMPLE.replacen(selected, replacement, 1));
    assert_eq!(platform.clipboard_writes, 0);
    assert_eq!(clipboard::clipboard_sequence_number(), clipboard_sequence);
    receipt.check("verified_apply_exact_reread_lock_released_clipboard_untouched");
    assert_ne!(unsafe { SendMessageW(harness.edit(PLAIN) as HWND, EM_CANUNDO, 0, 0) }, 0);
    unsafe {
        SendMessageW(harness.edit(PLAIN) as HWND, EM_UNDO, 0, 0);
    }
    assert_eq!(harness.text(PLAIN), SAMPLE);
    receipt.check("apply_is_undoable_by_user");

    // Cursor change after capture: no mutation, Copy-only.
    prepare(&harness, PLAIN, SAMPLE, "third");
    let (mut store, token, _) = ready_native_session();
    harness.select(PLAIN, 0, 2);
    harness.reset_counters();
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, "REPLACED", false, &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionChanged }
    );
    assert_eq!(harness.counter(PLAIN, KIND_REPLACESEL), 0);
    assert_eq!(harness.style(PLAIN) & ES_READONLY, 0);
    assert_eq!(harness.text(PLAIN), SAMPLE);
    assert_eq!(clipboard::read_clipboard_text().ok().as_deref(), Some("REPLACED"));
    receipt.check("cursor_change_zero_mutation_copy_only");

    // Source change after capture: no mutation, Copy-only.
    prepare(&harness, PLAIN, SAMPLE, "third");
    let (mut store, token, _) = ready_native_session();
    let edited = SAMPLE.replace("synthetic", "user-edited");
    harness.set_text(PLAIN, &edited);
    harness.reset_counters();
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, "REPLACED", false, &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSourceChanged }
    );
    assert_eq!(harness.counter(PLAIN, KIND_REPLACESEL), 0);
    assert_eq!(harness.text(PLAIN), edited);
    receipt.check("source_change_zero_mutation_copy_only");

    // Another window in the foreground: the bound editor is still the only target.
    prepare(&harness, PLAIN, SAMPLE, "third");
    let (mut store, token, _) = ready_native_session();
    harness.set_text(OTHER, "synthetic other editor third");
    harness.select(OTHER, 0, 9);
    harness.focus(OTHER);
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, "REPLACED", false, &mut platform),
        ApplyOutcome::Applied
    );
    assert_eq!(harness.text(PLAIN), SAMPLE.replacen("third", "REPLACED", 1));
    assert_eq!(harness.text(OTHER), "synthetic other editor third");
    receipt.check("foreground_switch_mutates_only_bound_editor");

    // Read-only editors are never locked or mutated.
    prepare(&harness, READ_ONLY, "synthetic read-only field", "read-only");
    let (mut store, token, _) = ready_native_session();
    harness.reset_counters();
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, "REPLACED", false, &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetEditorChanged }
    );
    assert_eq!(harness.counter(READ_ONLY, KIND_LOCK), 0);
    assert_eq!(harness.text(READ_ONLY), "synthetic read-only field");
    receipt.check("read_only_editor_copy_only_without_lock");

    // Typing races: posted characters must never be overwritten or displaced.
    let marker = "⟦교정⟧";
    let (mut applied, mut refused) = (0, 0);
    for iteration in 0..RACE_ITERATIONS {
        prepare(&harness, PLAIN, SAMPLE, "third");
        let (mut store, token, _) = ready_native_session();
        let edit = harness.edit(PLAIN);
        let stop = Arc::new(AtomicBool::new(false));
        let early = iteration % 2 == 0;
        let typist = {
            let stop = Arc::clone(&stop);
            let delay = if early { Duration::ZERO } else { jitter(iteration) };
            thread::spawn(move || {
                thread::sleep(delay);
                let started = Instant::now();
                while !stop.load(Ordering::SeqCst) && started.elapsed() < Duration::from_millis(40) {
                    unsafe {
                        PostMessageW(edit as HWND, WM_CHAR, 'x' as usize, 0);
                    }
                    thread::sleep(Duration::from_micros(300));
                }
            })
        };
        if early {
            // Let posted keystrokes reach the editor before Apply takes its lock.
            thread::sleep(Duration::from_millis(3));
        }
        let mut platform = NativeLivePlatform::default();
        let outcome = apply_current_session(&mut store, &token, marker, false, &mut platform);
        thread::sleep(Duration::from_millis(45));
        stop.store(true, Ordering::SeqCst);
        typist.join().expect("typist thread");
        flush_posted_input(&harness);
        let text = harness.text(PLAIN);
        match outcome {
            ApplyOutcome::Applied => {
                applied += 1;
                let (start, _) = utf16_range(SAMPLE, "third");
                let head = SAMPLE.encode_utf16().take(start).collect::<Vec<_>>();
                let wide = text.encode_utf16().collect::<Vec<_>>();
                let tail = SAMPLE.encode_utf16().skip(start + units("third")).collect::<Vec<_>>();
                assert!(
                    wide.len() >= head.len() + tail.len() && wide.starts_with(&head) && wide.ends_with(&tail),
                    "{iteration}: {text:?}"
                );
                let body = String::from_utf16(&wide[head.len()..wide.len() - tail.len()]).expect("valid body");
                let typed = body.strip_prefix(marker).unwrap_or_else(|| panic!("{iteration}: {text:?}"));
                assert!(typed.chars().all(|character| character == 'x'), "{iteration}: {text:?}");
            }
            ApplyOutcome::CopiedFallback { .. } => {
                refused += 1;
                assert!(!text.contains(marker), "{iteration}: {text:?}");
            }
            other => panic!("{iteration}: unexpected typing-race outcome {other:?}"),
        }
        assert_eq!(harness.style(PLAIN) & ES_READONLY, 0);
    }
    receipt.check(format!("typing_race_{RACE_ITERATIONS}_iterations_applied{applied}_refused{refused}_no_misplacement"));

    // Selection races: the replacement lands only on the verified range or the
    // verified original is restored under the lock.
    let (mut applied, mut refused, mut restored) = (0, 0, 0);
    for iteration in 0..RACE_ITERATIONS {
        prepare(&harness, PLAIN, SAMPLE, "third");
        let (mut store, token, _) = ready_native_session();
        let edit = harness.edit(PLAIN);
        let stop = Arc::new(AtomicBool::new(false));
        let mover = {
            let stop = Arc::clone(&stop);
            let delay = jitter(iteration + 100);
            thread::spawn(move || {
                thread::sleep(delay);
                let started = Instant::now();
                while !stop.load(Ordering::SeqCst) && started.elapsed() < Duration::from_millis(40) {
                    let mut result = 0usize;
                    unsafe {
                        SendMessageTimeoutW(edit as HWND, EM_SETSEL, 0, 1, SMTO_ABORTIFHUNG, 200, &mut result);
                    }
                }
            })
        };
        harness.reset_counters();
        let mut platform = NativeLivePlatform::default();
        let outcome = apply_current_session(&mut store, &token, marker, false, &mut platform);
        stop.store(true, Ordering::SeqCst);
        mover.join().expect("selection mover thread");
        let text = harness.text(PLAIN);
        match outcome {
            ApplyOutcome::Applied => {
                applied += 1;
                assert_eq!(text, SAMPLE.replacen("third", marker, 1), "{iteration}");
            }
            ApplyOutcome::CopiedFallback { reason } => {
                refused += 1;
                if platform.native_results.last()
                    == Some(&NativeApplyResult::Refused(crate::native_edit::NativeApplyRefusal::SelectionChanged))
                    && reason == ApplyFallbackReason::TargetSelectionChanged
                {
                    restored += usize::from(harness.counter(PLAIN, KIND_REPLACESEL) > 0);
                }
                assert_eq!(text, SAMPLE, "{iteration}: fallback left a mutation");
            }
            other => panic!("{iteration}: unexpected selection-race outcome {other:?}"),
        }
        assert_eq!(harness.style(PLAIN) & ES_READONLY, 0);
    }
    receipt.check(format!(
        "selection_race_{RACE_ITERATIONS}_iterations_applied{applied}_refused{refused}_restored{restored}_no_misplacement"
    ));

    // Deterministic selection race between the locked EM_SETSEL and EM_REPLACESEL:
    // the displaced replacement must be replaced by the verified original under
    // the lock (a real Edit control refuses EM_UNDO while read-only), with the
    // modification flag and the user's moved selection restored, and reported.
    for modified in [0usize, 1] {
        prepare(&harness, PLAIN, SAMPLE, "third");
        let (mut store, token, _) = ready_native_session();
        let edit = harness.edit(PLAIN) as HWND;
        unsafe {
            SendMessageW(edit, EM_SETMODIFY, modified, 0);
        }
        harness.arm_selection_race(PLAIN);
        harness.reset_counters();
        let mut platform = NativeLivePlatform::default();
        assert_eq!(
            apply_current_session(&mut store, &token, marker, false, &mut platform),
            ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionChanged },
            "modified={modified}"
        );
        assert_eq!(harness.counter(PLAIN, KIND_REPLACESEL), 1);
        assert_eq!(harness.counter(PLAIN, KIND_SETTEXT), 1);
        assert_eq!(harness.counter(PLAIN, KIND_UNDO), 0);
        assert_eq!(harness.style(PLAIN) & ES_READONLY, 0);
        assert_eq!(harness.text(PLAIN), SAMPLE);
        assert_eq!(usize::from(unsafe { SendMessageW(edit, EM_GETMODIFY, 0, 0) } != 0), modified);
        assert_eq!(harness.selection(PLAIN), (0, 1), "the moved selection is kept");
        assert_eq!(clipboard::read_clipboard_text().ok().as_deref(), Some(marker));
    }
    receipt.check("injected_selection_race_restored_under_lock_modify_flag_and_selection_kept_copy_only");

    // Target closed after capture: Copy-only without mutation.
    prepare(&harness, OTHER, "synthetic other editor third", "third");
    let (mut store, token, _) = ready_native_session();
    harness.close_other_form();
    let mut platform = NativeLivePlatform::default();
    assert_eq!(
        apply_current_session(&mut store, &token, "REPLACED", false, &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetMissing }
    );
    assert!(platform.native_results.is_empty());
    receipt.check("closed_target_copy_only");

    drop(harness);
    receipt.record_cleanups(assert_editor_cleanups_complete(1));
    receipt.check("owned_editor_cleanup");
    receipt.pass();
}

/// Observable result of one Apply against a synthetic editor that the
/// application itself edits (review findings R1/R2). Content-free.
struct RaceObservation {
    outcome: ApplyOutcome,
    state_changes: usize,
    foreign_change_notices: usize,
    app_edits: usize,
    final_matches: bool,
    clipboard_has_result: bool,
}

impl RaceObservation {
    fn safe(&self, expected_app_edits: Option<usize>) -> bool {
        matches!(self.outcome, ApplyOutcome::CopiedFallback { .. })
            && self.state_changes == 0
            && self.foreign_change_notices == 0
            && expected_app_edits.map_or(self.app_edits > 0, |expected| self.app_edits == expected)
            && self.final_matches
            && self.clipboard_has_result
    }

    fn describe(&self, name: &str) -> String {
        format!(
            "{name}: outcome={:?} state_changes={} foreign_change_notices={} app_edits={} final_is_application_text={} clipboard_has_result={}",
            self.outcome,
            self.state_changes,
            self.foreign_change_notices,
            self.app_edits,
            self.final_matches,
            self.clipboard_has_result
        )
    }
}

/// Captures "world" in "hello world", arms `arm`, applies "fixed" through the
/// production Apply entry, lets any application timer finish and observes.
fn apply_against_application(
    harness: &NativeEditHarness,
    arm: impl FnOnce(&NativeEditHarness),
    settle: Duration,
    final_ok: impl Fn(&str) -> bool,
) -> RaceObservation {
    harness.disarm_races();
    prepare(harness, PLAIN, "hello world", "world");
    let (mut store, token, _) = ready_native_session();
    harness.reset_counters();
    arm(harness);
    let mut platform = NativeLivePlatform::default();
    let outcome = apply_current_session(&mut store, &token, "fixed", false, &mut platform);
    thread::sleep(settle);
    harness.app_timer(PLAIN, 0);
    thread::sleep(Duration::from_millis(100));
    let observation = RaceObservation {
        outcome,
        state_changes: harness.state_changes(PLAIN),
        foreign_change_notices: harness.counter(PLAIN, KIND_FOREIGN_CHANGE_NOTICE),
        app_edits: harness.counter(PLAIN, KIND_APP_EDITS),
        final_matches: final_ok(&harness.text(PLAIN)),
        clipboard_has_result: clipboard::read_clipboard_text().ok().as_deref() == Some("fixed"),
    };
    harness.disarm_races();
    observation
}

#[test]
#[ignore = "mutates only an explicitly opted-in GitHub-hosted synthetic desktop"]
fn cloud_owned_desktop_native_apply_against_application_edits() {
    let mut receipt = CloudTestReceipt::new("cloud_owned_desktop_native_apply_against_application_edits");
    require_cloud_owned_desktop();
    receipt.check("owned_desktop_opt_in");
    assert!(clipboard::write_clipboard_text("synthetic-native-prior").is_ok());
    let _clipboard_guard = ClipboardTextGuard::capture().expect("synthetic clipboard roundtrip");
    let harness = NativeEditHarness::spawn();
    receipt.check("owned_native_editor_ready");
    let mut failures = Vec::new();
    let mut record = |name: &str, observation: RaceObservation, expected_app_edits: Option<usize>| {
        let description = observation.describe(name);
        eprintln!("NATIVE_APPLY_RACE {description}");
        if observation.safe(expected_app_edits) {
            receipt.check(name.to_string());
        } else {
            failures.push(description);
        }
    };

    // R1: the application rewrites the text inside its message handling right
    // after another process reads the selection; "hello newer" must survive.
    let observation = apply_against_application(
        &harness,
        |h| h.arm_app_edit(PLAIN, APP_EDIT_AFTER_SELECTION_READ),
        Duration::from_millis(700),
        |text| text == "hello newer",
    );
    record("r1_application_edit_after_verification_survives", observation, Some(1));

    // R2: the selection moves to "hello" between Apply's EM_SETSEL and
    // EM_REPLACESEL; nothing may be replaced, restored or notified.
    let observation = apply_against_application(
        &harness,
        |h| h.arm_selection_race_to(PLAIN, 0, 5),
        Duration::from_millis(300),
        |text| text == "hello world",
    );
    record("r2_moved_selection_causes_no_mutation_or_restore", observation, Some(0));

    // The application edits its text right before a restore would be processed.
    let observation = apply_against_application(
        &harness,
        |h| {
            h.arm_selection_race_to(PLAIN, 0, 5);
            h.arm_app_edit(PLAIN, APP_EDIT_BEFORE_SETTEXT);
        },
        Duration::from_millis(700),
        |text| text == "hello brave world",
    );
    record("application_edit_before_restore_survives", observation, Some(1));

    // An application timer keeps rewriting one word while Apply runs.
    let mut timer_failures = 0;
    for iteration in 0..RACE_ITERATIONS {
        let observation = apply_against_application(
            &harness,
            |h| h.app_timer(PLAIN, 10),
            Duration::from_millis(150),
            |text| text == "hello world" || text == "hello newer",
        );
        if !observation.safe(None) {
            timer_failures += 1;
            record(&format!("application_timer_iteration_{iteration}"), observation, None);
        }
    }
    if timer_failures == 0 {
        receipt.check(format!("application_timer_{RACE_ITERATIONS}_iterations_no_state_change"));
    }

    drop(harness);
    receipt.record_cleanups(assert_editor_cleanups_complete(1));
    receipt.check("owned_editor_cleanup");
    assert!(
        failures.is_empty(),
        "native Apply changed the editor or overwrote the application's own text: {failures:?}"
    );
    receipt.pass();
}
