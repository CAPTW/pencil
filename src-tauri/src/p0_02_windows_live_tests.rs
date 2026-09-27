use crate::apply_current_terminology_bound;
use crate::apply_safety::{
    apply_current_session, ApplyFailureReason, ApplyFallbackReason, ApplyOutcome, ApplyPlatform,
    WaitStage,
};
use crate::capture_session::{BoundRewriteIntent, TerminologyIntent};
use crate::capture_session::{CaptureSessionStore, SessionToken, WindowTarget};
use crate::clipboard;
use crate::settings::{RewriteMode, TerminologySettings};
use crate::translation::{
    format_translation, RewriteIntent, TranslationApplyFormat, TranslationTargetLanguage,
};
use crate::windows_apply::{
    foreground_window_handle, request_foreground_window, wait_for_stage, window_is_valid,
    window_process_id,
};
use crate::process_job::{resume_suspended_primary, ProcessJob};
use crate::windows_target::{capture_foreground_target, WindowsForegroundTargetPlatform};
use std::cell::RefCell;
use std::ffi::c_void;
use std::io::{BufRead, BufReader};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{GetLastError, SetLastError, HWND};
use windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, IsWindowEnabled, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetGUIThreadInfo, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    SendMessageW, SetWindowTextW, ShowWindow, ES_READONLY, GUITHREADINFO, GWL_STYLE, SW_HIDE,
    SW_SHOW, WM_CLOSE,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_SUSPENDED: u32 = 0x0000_0004;
const EDITOR_CLEANUP_BUDGET: Duration = Duration::from_secs(5);
const CF_UNICODETEXT: u32 = 13;
const EM_GETMODIFY: u32 = 0x00B8;
const EM_SETSEL: u32 = 0x00B1;
const EM_SETMODIFY: u32 = 0x00B9;
const WM_PASTE: u32 = 0x0302;
const WM_PROBE_EVENT_COUNT: u32 = 0x8001;
const WM_PROBE_EVENT_CODE: u32 = 0x8002;
const WM_PROBE_EVENT_MILLIS: u32 = 0x8003;
const WM_PROBE_RESET: u32 = 0x8004;
const WM_PROBE_FINAL_EQUAL: u32 = 0x8005;
const PROBE_REPETITIONS: usize = 3;
const EXPECTED_SOURCE: &str = "synthetic-target-start";
const EXPECTED_REPLACEMENT: &str = "synthetic-target-replaced";

#[link(name = "user32")]
extern "system" {
    fn GetThreadDesktop(dw_thread_id: u32) -> *mut c_void;
    fn GetUserObjectInformationW(
        object: *mut c_void,
        index: i32,
        value: *mut c_void,
        length: u32,
        needed: *mut u32,
    ) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntegrityRelation {
    Lower,
    Equal,
    Higher,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HelperReady {
    handles: [isize; 4],
    same_session: Option<bool>,
    same_input_desktop: Option<bool>,
    sender_integrity: IntegrityRelation,
}

/// Content-free result of tearing down one task-owned synthetic editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EditorCleanup {
    editor_pid: u32,
    process_exited: bool,
    job_processes_remaining: Option<u32>,
    windows_destroyed: bool,
}

impl EditorCleanup {
    fn complete(&self) -> bool {
        self.process_exited && self.job_processes_remaining == Some(0) && self.windows_destroyed
    }
}

thread_local! {
    // Filled by harness teardown, including teardown during a failing test's unwind.
    pub(crate) static EDITOR_CLEANUPS: RefCell<Vec<EditorCleanup>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn take_editor_cleanups() -> Vec<EditorCleanup> {
    EDITOR_CLEANUPS.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

pub(crate) fn assert_editor_cleanups_complete(expected: usize) -> Vec<EditorCleanup> {
    let cleanups = take_editor_cleanups();
    eprintln!("OWNED_EDITOR_CLEANUP {cleanups:?}");
    assert_eq!(cleanups.len(), expected, "every owned editor must report teardown");
    assert!(cleanups.iter().all(EditorCleanup::complete), "owned editor residue");
    cleanups
}

/// The synthetic editor and every descendant (for example the C# compiler that
/// Add-Type starts) run inside a kill-on-close Job. The process is created
/// suspended so nothing can run before it is owned.
pub(crate) struct OwnedEditorProcess {
    pub(crate) child: Child,
    job: Option<ProcessJob>,
    cleanup: Option<EditorCleanup>,
}

impl OwnedEditorProcess {
    pub(crate) fn spawn(command: &mut Command) -> Result<Self, &'static str> {
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
        let mut child = command.spawn().map_err(|_| "window_harness_spawn_failed")?;
        let job = match ProcessJob::assign_process_handle(child.as_raw_handle().cast()) {
            Ok(job) => job,
            Err(_) => {
                let _ = child.kill();
                let process_exited = child.wait().is_ok();
                // A suspended, unowned process cannot prove descendant cleanup.
                let cleanup = EditorCleanup {
                    editor_pid: child.id(),
                    process_exited,
                    job_processes_remaining: None,
                    windows_destroyed: true,
                };
                EDITOR_CLEANUPS.with(|log| log.borrow_mut().push(cleanup));
                return Err("window_harness_job_assignment_failed");
            }
        };
        let mut owned = Self {
            child,
            job: Some(job),
            cleanup: None,
        };
        if resume_suspended_primary(owned.child.id()).is_err() {
            let cleanup = owned.shutdown(&[]);
            EDITOR_CLEANUPS.with(|log| log.borrow_mut().push(cleanup));
            return Err("window_harness_resume_failed");
        }
        Ok(owned)
    }

    pub(crate) fn shutdown(&mut self, windows: &[isize]) -> EditorCleanup {
        if let Some(cleanup) = self.cleanup {
            return cleanup;
        }
        if let Some(job) = &self.job {
            let _ = job.terminate();
        }
        let _ = self.child.kill();
        let deadline = Instant::now() + EDITOR_CLEANUP_BUDGET;
        let mut process_exited = false;
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                process_exited = true;
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let mut job_processes_remaining = None;
        if let Some(job) = &self.job {
            while Instant::now() < deadline {
                job_processes_remaining = job.active_processes().ok();
                if job_processes_remaining == Some(0) {
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
        let windows_destroyed = wait_until(
            deadline.saturating_duration_since(Instant::now()),
            || windows.iter().all(|hwnd| !window_is_valid(*hwnd)),
        );
        let cleanup = EditorCleanup {
            editor_pid: self.child.id(),
            process_exited,
            job_processes_remaining,
            windows_destroyed,
        };
        // Closing the kill-on-close handle is the final backstop for descendants.
        self.job = None;
        self.cleanup = Some(cleanup);
        cleanup
    }
}

impl Drop for OwnedEditorProcess {
    fn drop(&mut self) {
        if self.cleanup.is_none() {
            let cleanup = self.shutdown(&[]);
            EDITOR_CLEANUPS.with(|log| log.borrow_mut().push(cleanup));
        }
    }
}

/// Content-free CI receipt for one owned-desktop test. It is written only into
/// the task-owned evidence directory and binds the outcome to the exact source
/// commit and workflow run. FAIL is recorded unless the test reaches `pass`.
pub(crate) struct CloudTestReceipt {
    test: &'static str,
    completed_checks: Vec<String>,
    cleanups: Vec<EditorCleanup>,
    passed: bool,
}

thread_local! {
    /// Last panic on this test thread, for the receipt written during unwinding.
    static CLOUD_TEST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Keeps the default panic output and also records the location and message so
/// a FAIL receipt names its cause even when the step log is not retrievable.
/// Owned-desktop tests handle synthetic text only.
/// Receipts carry no text values: every quoted string in a panic message (for
/// example `assert_eq!` operands) is replaced before it is recorded.
fn without_quoted_values(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut chars = message.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            out.push(c);
            continue;
        }
        out.push_str("\"<value>\"");
        let mut escaped = false;
        for inner in chars.by_ref() {
            if escaped {
                escaped = false;
            } else if inner == '\\' {
                escaped = true;
            } else if inner == '"' {
                break;
            }
        }
    }
    out
}

#[test]
fn receipt_failures_drop_quoted_values() {
    assert_eq!(
        without_quoted_values(r#"x.rs:1: assertion failed\n  left: "hello \"q\" world"\n right: "synthetic""#),
        r#"x.rs:1: assertion failed\n  left: "<value>"\n right: "<value>""#
    );
    assert_eq!(without_quoted_values("r1: outcome=Applied state_changes=4"), "r1: outcome=Applied state_changes=4");
}

fn record_cloud_test_panics() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .map(|message| (*message).to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            let location = info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_default();
            let summary: String = without_quoted_values(&format!("{location}: {message}")).chars().take(1200).collect();
            CLOUD_TEST_PANIC.with(|slot| *slot.borrow_mut() = Some(summary));
            previous(info);
        }));
    });
}

impl CloudTestReceipt {
    pub(crate) fn new(test: &'static str) -> Self {
        record_cloud_test_panics();
        CLOUD_TEST_PANIC.with(|slot| slot.borrow_mut().take());
        take_editor_cleanups();
        Self {
            test,
            completed_checks: Vec::new(),
            cleanups: Vec::new(),
            passed: false,
        }
    }

    pub(crate) fn check(&mut self, name: impl Into<String>) {
        let name = name.into();
        eprintln!("CLOUD_CHECK_PASS {} {name}", self.test);
        self.completed_checks.push(name);
    }

    pub(crate) fn record_cleanups(&mut self, cleanups: Vec<EditorCleanup>) {
        self.cleanups.extend(cleanups);
    }

    pub(crate) fn pass(&mut self) {
        self.passed = true;
    }
}

impl Drop for CloudTestReceipt {
    fn drop(&mut self) {
        self.cleanups.extend(take_editor_cleanups());
        let status = if self.passed
            && !thread::panicking()
            && self.cleanups.iter().all(EditorCleanup::complete)
        {
            "PASS"
        } else {
            "FAIL"
        };
        let failure = CLOUD_TEST_PANIC.with(|slot| slot.borrow_mut().take());
        eprintln!("CLOUD_TEST_RESULT {} {status}", self.test);
        eprintln!("CLOUD_TEST_CLEANUPS {} {:?}", self.test, self.cleanups);
        if let Some(failure) = &failure {
            eprintln!("CLOUD_TEST_FAILURE {} {failure}", self.test);
        }
        let Some(root) = std::env::var_os("GRAMMAR_EVIDENCE") else {
            return;
        };
        let env = |key: &str| std::env::var(key).unwrap_or_default();
        let cleanups = self
            .cleanups
            .iter()
            .map(|cleanup| {
                serde_json::json!({
                    "editor_pid": cleanup.editor_pid,
                    "process_exited": cleanup.process_exited,
                    "job_processes_remaining": cleanup.job_processes_remaining,
                    "windows_destroyed": cleanup.windows_destroyed,
                })
            })
            .collect::<Vec<_>>();
        let receipt = serde_json::json!({
            "schema": "grammar-owned-desktop-receipt/v1",
            "classification": "SYNTHETIC_OWNED_WINDOWS_DESKTOP",
            "test": self.test,
            "status": status,
            "source_sha": env("GITHUB_SHA"),
            "repository": env("GITHUB_REPOSITORY"),
            "ref": env("GITHUB_REF"),
            "run_id": env("GITHUB_RUN_ID"),
            "run_attempt": env("GITHUB_RUN_ATTEMPT"),
            "completed_checks": self.completed_checks,
            "last_completed_check": self.completed_checks.last(),
            "failure": failure,
            "editor_cleanups": cleanups,
            "not_qualified": [
                "production toolbar and global shortcut",
                "physical keyboard and IME",
                "live Provider inference",
            ],
        });
        let directory = std::path::Path::new(&root).join("native");
        let _ = std::fs::create_dir_all(&directory);
        if let Ok(bytes) = serde_json::to_vec_pretty(&receipt) {
            let _ = std::fs::write(directory.join(format!("{}.json", self.test)), bytes);
        }
    }
}

struct WindowHarness {
    editor: OwnedEditorProcess,
    target_window: isize,
    target_textbox: isize,
    widget_window: isize,
    widget_textbox: isize,
    same_session: Option<bool>,
    same_input_desktop: Option<bool>,
    sender_integrity: IntegrityRelation,
}

impl WindowHarness {
    fn spawn() -> Result<Self, &'static str> {
        Self::spawn_with(EXPECTED_SOURCE, EXPECTED_REPLACEMENT)
    }

    fn spawn_with(source: &str, expected_replacement: &str) -> Result<Self, &'static str> {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms.dll,System.Drawing.dll -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public sealed class ProbeEvent
{
    public readonly int Code;
    public readonly long Millis;

    public ProbeEvent(int code, long millis)
    {
        Code = code;
        Millis = millis;
    }
}

public sealed class ProbeTelemetry
{
    private readonly Stopwatch clock = Stopwatch.StartNew();
    private readonly List<ProbeEvent> events = new List<ProbeEvent>();
    private bool firstEqualRecorded;

    public void Reset()
    {
        events.Clear();
        firstEqualRecorded = false;
        clock.Restart();
    }

    public void Record(int code)
    {
        if (events.Count < 128)
        {
            events.Add(new ProbeEvent(code, clock.ElapsedMilliseconds));
        }
    }

    public void RecordFirstEqual()
    {
        if (!firstEqualRecorded)
        {
            firstEqualRecorded = true;
            Record(9);
        }
    }

    public int Count { get { return events.Count; } }
    public int CodeAt(int index) { return index >= 0 && index < events.Count ? events[index].Code : 0; }
    public long MillisAt(int index) { return index >= 0 && index < events.Count ? events[index].Millis : -1; }
}

public sealed class InstrumentedTextBox : TextBox
{
    private const int WM_KEYDOWN = 0x0100;
    private const int WM_KEYUP = 0x0101;
    private const int WM_CHAR = 0x0102;
    private const int WM_PASTE = 0x0302;
    private const int VK_CONTROL = 0x11;
    private const int VK_LCONTROL = 0xA2;
    private const int VK_RCONTROL = 0xA3;
    private const int VK_V = 0x56;
    private readonly ProbeTelemetry telemetry;
    private readonly string expectedReplacement;

    public InstrumentedTextBox(ProbeTelemetry telemetry, string expectedReplacement)
    {
        this.telemetry = telemetry;
        this.expectedReplacement = expectedReplacement;
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg == WM_KEYDOWN || message.Msg == WM_KEYUP)
        {
            int key = unchecked((int)message.WParam.ToInt64());
            if (message.Msg == WM_KEYDOWN && (key == VK_CONTROL || key == VK_LCONTROL || key == VK_RCONTROL)) telemetry.Record(1);
            if (message.Msg == WM_KEYUP && (key == VK_CONTROL || key == VK_LCONTROL || key == VK_RCONTROL)) telemetry.Record(2);
            if (message.Msg == WM_KEYDOWN && key == VK_V) telemetry.Record(3);
            if (message.Msg == WM_KEYUP && key == VK_V) telemetry.Record(4);
        }
        if (message.Msg == WM_CHAR) telemetry.Record(5);
        if (message.Msg == WM_PASTE) telemetry.Record(6);

        base.WndProc(ref message);
    }

    protected override void OnTextChanged(EventArgs args)
    {
        base.OnTextChanged(args);
        if (Text == expectedReplacement) telemetry.RecordFirstEqual();
    }
}

public sealed class FocusEditorForm : Form
{
    public readonly TextBox Editor;
    private const int WM_COMMAND = 0x0111;
    private const int EN_CHANGE = 0x0300;
    private const int EN_UPDATE = 0x0400;
    private const int WM_PROBE_EVENT_COUNT = 0x8001;
    private const int WM_PROBE_EVENT_CODE = 0x8002;
    private const int WM_PROBE_EVENT_MILLIS = 0x8003;
    private const int WM_PROBE_RESET = 0x8004;
    private const int WM_PROBE_FINAL_EQUAL = 0x8005;
    private readonly ProbeTelemetry telemetry;
    private readonly string expectedReplacement;

    public FocusEditorForm(string title, string text, string expectedReplacement, int left)
    {
        telemetry = new ProbeTelemetry();
        this.expectedReplacement = expectedReplacement;
        Text = title;
        StartPosition = FormStartPosition.Manual;
        Location = new Point(left, 80);
        Size = new Size(420, 180);
        Editor = new InstrumentedTextBox(telemetry, expectedReplacement) {
            Multiline = true,
            Dock = DockStyle.Fill,
            Text = text,
            AcceptsReturn = true,
            AcceptsTab = true
        };
        Controls.Add(Editor);
    }

    protected override void WndProc(ref Message message)
    {
        if (message.Msg == WM_PROBE_EVENT_COUNT)
        {
            message.Result = new IntPtr(telemetry.Count);
            return;
        }
        if (message.Msg == WM_PROBE_EVENT_CODE)
        {
            message.Result = new IntPtr(telemetry.CodeAt(message.WParam.ToInt32()));
            return;
        }
        if (message.Msg == WM_PROBE_EVENT_MILLIS)
        {
            message.Result = new IntPtr(telemetry.MillisAt(message.WParam.ToInt32()));
            return;
        }
        if (message.Msg == WM_PROBE_RESET)
        {
            telemetry.Reset();
            message.Result = new IntPtr(1);
            return;
        }
        if (message.Msg == WM_PROBE_FINAL_EQUAL)
        {
            message.Result = new IntPtr(Editor.Text == expectedReplacement ? 1 : 0);
            return;
        }
        if (message.Msg == WM_COMMAND && message.LParam == Editor.Handle)
        {
            int notification = (int)((message.WParam.ToInt64() >> 16) & 0xffff);
            if (notification == EN_UPDATE) telemetry.Record(7);
            if (notification == EN_CHANGE) telemetry.Record(8);
        }
        base.WndProc(ref message);
    }

    protected override void OnActivated(EventArgs args)
    {
        base.OnActivated(args);
        Editor.Focus();
        Editor.SelectAll();
    }
}

public static class FocusEditorHarness
{
    private const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
    private const uint TOKEN_QUERY = 0x0008;
    private const int TokenIntegrityLevel = 25;
    private const int UOI_NAME = 2;

    [StructLayout(LayoutKind.Sequential)]
    private struct SID_AND_ATTRIBUTES
    {
        public IntPtr Sid;
        public uint Attributes;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TOKEN_MANDATORY_LABEL
    {
        public SID_AND_ATTRIBUTES Label;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inherit, uint processId);
    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll")]
    private static extern bool ProcessIdToSessionId(uint processId, out uint sessionId);
    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")]
    private static extern IntPtr GetThreadDesktop(uint threadId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool GetUserObjectInformationW(IntPtr handle, int index, IntPtr value, uint length, out uint needed);
    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern bool GetTokenInformation(IntPtr token, int infoClass, IntPtr info, uint length, out uint needed);
    [DllImport("advapi32.dll")]
    private static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);
    [DllImport("advapi32.dll")]
    private static extern IntPtr GetSidSubAuthority(IntPtr sid, uint index);

    private static string BoolState(bool? value)
    {
        return !value.HasValue ? "U" : (value.Value ? "1" : "0");
    }

    private static bool? SameSession(uint senderPid)
    {
        uint senderSession;
        uint targetSession;
        if (!ProcessIdToSessionId(senderPid, out senderSession) ||
            !ProcessIdToSessionId((uint)Process.GetCurrentProcess().Id, out targetSession)) return null;
        return senderSession == targetSession;
    }

    private static string DesktopName(uint threadId)
    {
        IntPtr desktop = GetThreadDesktop(threadId);
        if (desktop == IntPtr.Zero) return null;
        uint needed;
        GetUserObjectInformationW(desktop, UOI_NAME, IntPtr.Zero, 0, out needed);
        if (needed == 0) return null;
        IntPtr buffer = Marshal.AllocHGlobal((int)needed);
        try
        {
            if (!GetUserObjectInformationW(desktop, UOI_NAME, buffer, needed, out needed)) return null;
            return Marshal.PtrToStringUni(buffer);
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    private static bool? SameDesktop(string senderDesktop)
    {
        string target = DesktopName(GetCurrentThreadId());
        if (String.IsNullOrEmpty(senderDesktop) || target == null) return null;
        return String.Equals(senderDesktop, target, StringComparison.Ordinal);
    }

    private static int IntegrityRid(uint processId)
    {
        IntPtr process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, processId);
        if (process == IntPtr.Zero) return -1;
        IntPtr token = IntPtr.Zero;
        try
        {
            if (!OpenProcessToken(process, TOKEN_QUERY, out token)) return -1;
            uint needed;
            GetTokenInformation(token, TokenIntegrityLevel, IntPtr.Zero, 0, out needed);
            if (needed == 0) return -1;
            IntPtr buffer = Marshal.AllocHGlobal((int)needed);
            try
            {
                if (!GetTokenInformation(token, TokenIntegrityLevel, buffer, needed, out needed)) return -1;
                TOKEN_MANDATORY_LABEL label = (TOKEN_MANDATORY_LABEL)Marshal.PtrToStructure(buffer, typeof(TOKEN_MANDATORY_LABEL));
                IntPtr countPointer = GetSidSubAuthorityCount(label.Label.Sid);
                if (countPointer == IntPtr.Zero) return -1;
                byte count = Marshal.ReadByte(countPointer);
                if (count == 0) return -1;
                IntPtr ridPointer = GetSidSubAuthority(label.Label.Sid, (uint)(count - 1));
                return ridPointer == IntPtr.Zero ? -1 : Marshal.ReadInt32(ridPointer);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }
        finally
        {
            if (token != IntPtr.Zero) CloseHandle(token);
            CloseHandle(process);
        }
    }

    private static string IntegrityRelation(uint senderPid)
    {
        int sender = IntegrityRid(senderPid);
        int target = IntegrityRid((uint)Process.GetCurrentProcess().Id);
        if (sender < 0 || target < 0) return "U";
        return sender < target ? "L" : (sender > target ? "H" : "E");
    }

    public static void Run()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        uint senderPid;
        string senderDesktop = Environment.GetEnvironmentVariable("CODEX_PENCIL_TEST_SENDER_DESKTOP");
        if (!UInt32.TryParse(Environment.GetEnvironmentVariable("CODEX_PENCIL_TEST_SENDER_PID"), out senderPid)) senderPid = 0;
        string source = Environment.GetEnvironmentVariable("CODEX_PENCIL_TEST_SOURCE") ?? "synthetic-target-start";
        string replacement = Environment.GetEnvironmentVariable("CODEX_PENCIL_TEST_REPLACEMENT") ?? "synthetic-target-replaced";
        var widget = new FocusEditorForm("Synthetic Widget Guard", "synthetic-widget-guard", "synthetic-widget-guard", 80);
        var target = new FocusEditorForm("Synthetic Apply Target", source, replacement, 540);
        widget.Show();
        target.Show();
        target.Activate();
        target.Editor.Focus();
        target.Editor.SelectAll();
        Application.DoEvents();
        Console.Out.WriteLine(
            "READY:{0}:{1}:{2}:{3}:{4}:{5}:{6}",
            target.Handle.ToInt64(),
            target.Editor.Handle.ToInt64(),
            widget.Handle.ToInt64(),
            widget.Editor.Handle.ToInt64(),
            BoolState(senderPid == 0 ? (bool?)null : SameSession(senderPid)),
            BoolState(String.IsNullOrEmpty(senderDesktop) ? (bool?)null : SameDesktop(senderDesktop)),
            senderPid == 0 ? "U" : IntegrityRelation(senderPid));
        Console.Out.Flush();
        Application.Run();
    }
}
"@
[FocusEditorHarness]::Run()
"#;
        let sender_desktop = current_input_desktop_name().unwrap_or_default();
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-STA", "-Command", script])
            .env(
                "CODEX_PENCIL_TEST_SENDER_PID",
                std::process::id().to_string(),
            )
            .env("CODEX_PENCIL_TEST_SENDER_DESKTOP", sender_desktop)
            .env("CODEX_PENCIL_TEST_SOURCE", source)
            .env("CODEX_PENCIL_TEST_REPLACEMENT", expected_replacement)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut editor = OwnedEditorProcess::spawn(&mut command)?;
        let stdout = editor
            .child
            .stdout
            .take()
            .ok_or("window_harness_pipe_failed")?;
        let (sender, receiver) = mpsc::sync_channel(1);
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(handles) = parse_ready_line(&line) {
                    let _ = sender.send(handles);
                    return;
                }
            }
        });
        let handles = receiver
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "window_harness_start_timeout")?;
        Ok(Self {
            editor,
            target_window: handles.handles[0],
            target_textbox: handles.handles[1],
            widget_window: handles.handles[2],
            widget_textbox: handles.handles[3],
            same_session: handles.same_session,
            same_input_desktop: handles.same_input_desktop,
            sender_integrity: handles.sender_integrity,
        })
    }
}

impl Drop for WindowHarness {
    fn drop(&mut self) {
        let cleanup = self
            .editor
            .shutdown(&[self.target_window, self.widget_window]);
        EDITOR_CLEANUPS.with(|log| log.borrow_mut().push(cleanup));
    }
}

pub(crate) fn current_input_desktop_name() -> Option<String> {
    const UOI_NAME: i32 = 2;
    let thread_id = unsafe { GetCurrentThreadId() };
    let desktop = unsafe { GetThreadDesktop(thread_id) };
    if desktop.is_null() {
        return None;
    }

    let mut needed = 0;
    unsafe {
        GetUserObjectInformationW(desktop, UOI_NAME, std::ptr::null_mut(), 0, &mut needed);
    }
    if needed < 2 {
        return None;
    }

    let mut buffer = vec![0u16; (needed as usize).div_ceil(2)];
    if unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return None;
    }
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..length]).ok()
}

fn parse_ready_line(line: &str) -> Option<HelperReady> {
    let mut parts = line.strip_prefix("READY:")?.split(':');
    let handles = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    let same_session = parse_optional_bool(parts.next()?)?;
    let same_input_desktop = parse_optional_bool(parts.next()?)?;
    let sender_integrity = match parts.next()? {
        "L" => IntegrityRelation::Lower,
        "E" => IntegrityRelation::Equal,
        "H" => IntegrityRelation::Higher,
        "U" => IntegrityRelation::Unavailable,
        _ => return None,
    };
    if parts.next().is_some() {
        None
    } else {
        Some(HelperReady {
            handles,
            same_session,
            same_input_desktop,
            sender_integrity,
        })
    }
}

fn parse_optional_bool(value: &str) -> Option<Option<bool>> {
    match value {
        "1" => Some(Some(true)),
        "0" => Some(Some(false)),
        "U" => Some(None),
        _ => None,
    }
}

pub(crate) struct ClipboardTextGuard {
    original: String,
}

impl ClipboardTextGuard {
    pub(crate) fn capture() -> Result<Self, &'static str> {
        clipboard::read_clipboard_text()
            .map(|original| Self { original })
            .map_err(|_| "live_test_requires_supported_text_clipboard")
    }
}

impl Drop for ClipboardTextGuard {
    fn drop(&mut self) {
        let _ = clipboard::write_clipboard_text(&self.original);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ModifierState {
    control: bool,
    shift: bool,
    alt: bool,
    left_windows: bool,
    right_windows: bool,
    v: bool,
}

impl ModifierState {
    fn any_pressed(self) -> bool {
        self.control || self.shift || self.alt || self.left_windows || self.right_windows || self.v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InjectionObservation {
    target_is_foreground: bool,
    expected_control_has_focus: bool,
    nonempty_selection: bool,
    modifiers: ModifierState,
    unicode_text_available: bool,
    clipboard_matches_expected: bool,
    replacement_sequence: u32,
    requested_count: u32,
    returned_count: u32,
    last_error: u32,
}

struct LiveApplyPlatform {
    widget_window: isize,
    target_window: isize,
    target_textbox: isize,
    paste_calls: usize,
    clipboard_writes: usize,
    started_at: Instant,
    injection: Option<InjectionObservation>,
    restore_started_millis: Option<u128>,
    restore_completed_millis: Option<u128>,
}

impl LiveApplyPlatform {
    fn new(widget_window: isize, target_window: isize, target_textbox: isize) -> Self {
        Self {
            widget_window,
            target_window,
            target_textbox,
            paste_calls: 0,
            clipboard_writes: 0,
            started_at: Instant::now(),
            injection: None,
            restore_started_millis: None,
            restore_completed_millis: None,
        }
    }
}

impl ApplyPlatform for LiveApplyPlatform {
    fn is_window(&mut self, hwnd: isize) -> bool {
        window_is_valid(hwnd)
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
        window_process_id(hwnd)
    }

    fn hide_widget(&mut self) -> Result<(), ()> {
        unsafe {
            ShowWindow(self.widget_window as HWND, SW_HIDE);
        }
        if unsafe { IsWindowVisible(self.widget_window as HWND) } == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    fn show_widget(&mut self) {
        unsafe {
            ShowWindow(self.widget_window as HWND, SW_SHOW);
        }
        request_foreground_window(self.widget_window);
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
        let is_restore = self.paste_calls > 0;
        if is_restore {
            self.restore_started_millis = Some(self.started_at.elapsed().as_millis());
        }
        clipboard::write_clipboard_text(text).map_err(|_| ())?;
        self.clipboard_writes += 1;
        if is_restore {
            self.restore_completed_millis = Some(self.started_at.elapsed().as_millis());
        }
        Ok(())
    }

    fn send_paste(&mut self) -> u32 {
        self.paste_calls += 1;
        let target_is_foreground = foreground_window_handle() == self.target_window;
        let expected_control_has_focus =
            focused_window(self.target_window) == Some(self.target_textbox);
        let modifiers = modifier_state();
        let unicode_text_available = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 };
        let clipboard_matches_expected =
            clipboard::read_clipboard_text().ok().as_deref() == Some(EXPECTED_REPLACEMENT);
        let replacement_sequence = clipboard::clipboard_sequence_number();
        unsafe {
            SetLastError(0);
        }
        let returned_count = clipboard::send_paste_shortcut_count();
        let last_error = unsafe { GetLastError() };
        self.injection = Some(InjectionObservation {
            target_is_foreground,
            expected_control_has_focus,
            nonempty_selection: selection_is_nonempty(self.target_textbox),
            modifiers,
            unicode_text_available,
            clipboard_matches_expected,
            replacement_sequence,
            requested_count: 4,
            returned_count,
            last_error,
        });
        returned_count
    }

    fn wait(&mut self, stage: WaitStage) {
        wait_for_stage(stage);
    }
}

fn key_is_pressed(virtual_key: u16) -> bool {
    unsafe { (GetAsyncKeyState(virtual_key as i32) as u16 & 0x8000) != 0 }
}

fn modifier_state() -> ModifierState {
    ModifierState {
        control: key_is_pressed(VK_CONTROL),
        shift: key_is_pressed(VK_SHIFT),
        alt: key_is_pressed(VK_MENU),
        left_windows: key_is_pressed(VK_LWIN),
        right_windows: key_is_pressed(VK_RWIN),
        v: key_is_pressed(VK_V),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ControlState {
    top_level_created: bool,
    edit_created: bool,
    edit_enabled: bool,
    edit_writable: bool,
    edit_visible: bool,
    edit_not_read_only: bool,
    exact_selection: bool,
}

fn control_state(top_level: isize, edit: isize) -> ControlState {
    let top_level_created = unsafe { IsWindow(top_level as HWND) != 0 };
    let edit_created = unsafe { IsWindow(edit as HWND) != 0 };
    let edit_enabled = edit_created && unsafe { IsWindowEnabled(edit as HWND) != 0 };
    let edit_visible = edit_created && unsafe { IsWindowVisible(edit as HWND) != 0 };
    let style = if edit_created {
        unsafe { GetWindowLongPtrW(edit as HWND, GWL_STYLE) }
    } else {
        0
    };
    let edit_not_read_only = edit_created && (style & ES_READONLY as isize) == 0;
    ControlState {
        top_level_created,
        edit_created,
        edit_enabled,
        edit_writable: edit_enabled && edit_not_read_only,
        edit_visible,
        edit_not_read_only,
        exact_selection: selection_is_exact(edit, EXPECTED_SOURCE.encode_utf16().count()),
    }
}

fn selection_is_exact(hwnd: isize, expected_length: usize) -> bool {
    let packed = unsafe { SendMessageW(hwnd as HWND, 0x00B0, 0, 0) } as usize;
    let start = packed & 0xffff;
    let end = (packed >> 16) & 0xffff;
    start == 0 && end == expected_length
}

fn selection_is_nonempty(hwnd: isize) -> bool {
    let packed = unsafe { SendMessageW(hwnd as HWND, 0x00B0, 0, 0) } as usize;
    let start = packed & 0xffff;
    let end = (packed >> 16) & 0xffff;
    end > start
}

fn reset_probe_state(harness: &WindowHarness) -> bool {
    unsafe {
        SendMessageW(harness.target_textbox as HWND, EM_SETMODIFY, 0, 0);
        let target_reset = SendMessageW(harness.target_window as HWND, WM_PROBE_RESET, 0, 0);
        let widget_reset = SendMessageW(harness.widget_window as HWND, WM_PROBE_RESET, 0, 0);
        target_reset == 1 && widget_reset == 1
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MessageEvent {
    code: u8,
    millis: u64,
}

fn probe_events(window: isize) -> Vec<MessageEvent> {
    let count = unsafe { SendMessageW(window as HWND, WM_PROBE_EVENT_COUNT, 0, 0) };
    let bounded_count = usize::try_from(count).unwrap_or_default().min(128);
    (0..bounded_count)
        .filter_map(|index| {
            let code = unsafe { SendMessageW(window as HWND, WM_PROBE_EVENT_CODE, index, 0) };
            let millis = unsafe { SendMessageW(window as HWND, WM_PROBE_EVENT_MILLIS, index, 0) };
            Some(MessageEvent {
                code: u8::try_from(code).ok()?,
                millis: u64::try_from(millis).ok()?,
            })
        })
        .collect()
}

fn event_count(events: &[MessageEvent], code: u8) -> usize {
    events.iter().filter(|event| event.code == code).count()
}

fn edit_is_modified(hwnd: isize) -> bool {
    unsafe { SendMessageW(hwnd as HWND, EM_GETMODIFY, 0, 0) != 0 }
}

fn set_text(hwnd: isize, text: &str) -> bool {
    let mut wide = text.encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    unsafe { SetWindowTextW(hwnd as HWND, wide.as_ptr()) != 0 }
}

fn select_all(hwnd: isize) {
    unsafe {
        SendMessageW(hwnd as HWND, EM_SETSEL, 0, -1);
    }
}

fn content_equals_expected(window: isize) -> bool {
    unsafe { SendMessageW(window as HWND, WM_PROBE_FINAL_EQUAL, 0, 0) == 1 }
}

pub(crate) fn wait_until(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    condition()
}

pub(crate) fn focused_window(hwnd: isize) -> Option<isize> {
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd as HWND, std::ptr::null_mut()) };
    if thread_id == 0 {
        return None;
    }
    let mut info: GUITHREADINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if unsafe { GetGUIThreadInfo(thread_id, &mut info) } == 0 {
        None
    } else {
        Some(info.hwndFocus as isize)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeKind {
    CurrentCandidate,
    RestoreSuppressed,
    DirectWindowPaste,
    IsolatedSendInput,
}

#[derive(Debug)]
struct ProbeEvidence {
    kind: ProbeKind,
    run: usize,
    control: ControlState,
    same_session: Option<bool>,
    same_input_desktop: Option<bool>,
    sender_integrity: IntegrityRelation,
    injection: Option<InjectionObservation>,
    target_events: Vec<MessageEvent>,
    widget_edit_created: bool,
    widget_event_count: usize,
    target_ctrl_down_count: usize,
    target_ctrl_up_count: usize,
    target_v_down_count: usize,
    target_v_up_count: usize,
    target_char_count: usize,
    target_paste_count: usize,
    target_en_update_count: usize,
    target_en_change_count: usize,
    edit_modified: bool,
    restore_started_millis: Option<u128>,
    restore_completed_millis: Option<u128>,
    first_equal_millis: Option<u64>,
    final_content_equal: bool,
    widget_content_unchanged: bool,
    outcome: &'static str,
}

fn outcome_code(outcome: &ApplyOutcome) -> &'static str {
    match outcome {
        ApplyOutcome::Applied => "applied",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetSelectionUnverified,
        } => "copied_selection_unverified",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMissing,
        } => "copied_fallback_target_missing",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetProcessChanged,
        } => "copied_fallback_target_process_changed",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetNotForeground,
        } => "copied_fallback_target_not_foreground",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetChangedBeforePaste,
        } => "copied_fallback_target_changed_before_paste",
        ApplyOutcome::RejectedStale => "rejected_stale",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::EmptyReplacement,
        } => "failed_empty_replacement",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::InvalidSessionState,
        } => "failed_invalid_session_state",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::WidgetHideFailed,
        } => "failed_widget_hide",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        } => "failed_clipboard_write",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardOwnershipLost,
        } => "failed_clipboard_ownership_lost",
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::InputInjectionFailed,
        } => "failed_input_injection",
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMutationDisabled,
        } => "copied_fallback_target_mutation_disabled",
    }
}

fn prepare_probe_harness() -> (WindowHarness, WindowTarget, ControlState) {
    let harness = WindowHarness::spawn().unwrap_or_else(|_| panic!("probe helper did not start"));
    assert!(set_text(harness.target_textbox, EXPECTED_SOURCE));
    select_all(harness.target_textbox);
    unsafe {
        SendMessageW(harness.target_textbox as HWND, EM_SETMODIFY, 0, 0);
    }
    assert!(activate_target_for_setup(&harness));

    let control = control_state(harness.target_window, harness.target_textbox);
    let mut capture_platform = WindowsForegroundTargetPlatform;
    let target = capture_foreground_target(
        &mut capture_platform,
        harness.widget_window,
        std::process::id(),
    )
    .unwrap_or_else(|_| panic!("probe target capture failed"));

    request_foreground_window(harness.widget_window);
    assert!(wait_until(Duration::from_secs(2), || {
        foreground_window_handle() == harness.widget_window
    }));
    assert!(reset_probe_state(&harness));
    (harness, target, control)
}

fn activate_target_for_setup(harness: &WindowHarness) -> bool {
    for _ in 0..3 {
        request_foreground_window(harness.target_window);
        if wait_until(Duration::from_secs(2), || {
            foreground_window_handle() == harness.target_window
                && focused_window(harness.target_window) == Some(harness.target_textbox)
        }) {
            return true;
        }
    }
    false
}

fn ready_store(target: WindowTarget, run: usize) -> (CaptureSessionStore, SessionToken) {
    assert!(clipboard::write_clipboard_text("synthetic-live-prior").is_ok());
    let capture_sequence = clipboard::clipboard_sequence_number();
    let mut store = CaptureSessionStore::default();
    let token = store
        .capture(
            format!("synthetic-probe-session-{run}"),
            EXPECTED_SOURCE.to_string(),
            target,
            Some("synthetic-live-prior".to_string()),
            Some(capture_sequence),
        )
        .unwrap_or_else(|_| panic!("probe session capture failed"));
    assert!(store.begin_rewrite(&token).is_ok());
    assert!(store.finish_rewrite_success(&token).is_ok());
    (store, token)
}

fn current_injection_observation(
    harness: &WindowHarness,
    requested_count: u32,
    returned_count: u32,
    last_error: u32,
) -> InjectionObservation {
    InjectionObservation {
        target_is_foreground: foreground_window_handle() == harness.target_window,
        expected_control_has_focus: focused_window(harness.target_window)
            == Some(harness.target_textbox),
        nonempty_selection: selection_is_nonempty(harness.target_textbox),
        modifiers: modifier_state(),
        unicode_text_available: unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 },
        clipboard_matches_expected: clipboard::read_clipboard_text().ok().as_deref()
            == Some(EXPECTED_REPLACEMENT),
        replacement_sequence: clipboard::clipboard_sequence_number(),
        requested_count,
        returned_count,
        last_error,
    }
}

fn hide_widget_and_activate_target(harness: &WindowHarness) -> bool {
    unsafe {
        ShowWindow(harness.widget_window as HWND, SW_HIDE);
    }
    if unsafe { IsWindowVisible(harness.widget_window as HWND) } != 0 {
        return false;
    }
    for _ in 0..3 {
        request_foreground_window(harness.target_window);
        wait_for_stage(WaitStage::AfterActivation);
        if foreground_window_handle() == harness.target_window
            && focused_window(harness.target_window) == Some(harness.target_textbox)
        {
            return true;
        }
    }
    false
}

fn finish_probe_evidence(
    kind: ProbeKind,
    run: usize,
    harness: &WindowHarness,
    control: ControlState,
    injection: Option<InjectionObservation>,
    restore_started_millis: Option<u128>,
    restore_completed_millis: Option<u128>,
    outcome: &'static str,
) -> ProbeEvidence {
    let final_content_equal = wait_until(Duration::from_secs(2), || {
        content_equals_expected(harness.target_window)
    });
    let target_events = probe_events(harness.target_window);
    let widget_events = probe_events(harness.widget_window);
    let first_equal_millis = target_events
        .iter()
        .find(|event| event.code == 9)
        .map(|event| event.millis);
    ProbeEvidence {
        kind,
        run,
        control,
        same_session: harness.same_session,
        same_input_desktop: harness.same_input_desktop,
        sender_integrity: harness.sender_integrity,
        injection,
        widget_edit_created: unsafe { IsWindow(harness.widget_textbox as HWND) != 0 },
        widget_event_count: widget_events.len(),
        target_ctrl_down_count: event_count(&target_events, 1),
        target_ctrl_up_count: event_count(&target_events, 2),
        target_v_down_count: event_count(&target_events, 3),
        target_v_up_count: event_count(&target_events, 4),
        target_char_count: event_count(&target_events, 5),
        target_paste_count: event_count(&target_events, 6),
        target_en_update_count: event_count(&target_events, 7),
        target_en_change_count: event_count(&target_events, 8),
        edit_modified: edit_is_modified(harness.target_textbox),
        restore_started_millis,
        restore_completed_millis,
        first_equal_millis,
        final_content_equal,
        widget_content_unchanged: content_equals_expected(harness.widget_window),
        outcome,
        target_events,
    }
}

fn run_apply_probe(kind: ProbeKind, run: usize, restore_clipboard: bool) -> ProbeEvidence {
    let (harness, target, control) = prepare_probe_harness();
    let (mut store, token) = ready_store(target, run);
    let mut platform = LiveApplyPlatform::new(
        harness.widget_window,
        harness.target_window,
        harness.target_textbox,
    );
    let outcome = apply_current_session(
        &mut store,
        &token,
        EXPECTED_REPLACEMENT,
        restore_clipboard,
        &mut platform,
    );
    finish_probe_evidence(
        kind,
        run,
        &harness,
        control,
        platform.injection,
        platform.restore_started_millis,
        platform.restore_completed_millis,
        outcome_code(&outcome),
    )
}

fn run_direct_window_paste_probe(run: usize) -> ProbeEvidence {
    let (harness, _target, control) = prepare_probe_harness();
    assert!(hide_widget_and_activate_target(&harness));
    assert!(clipboard::write_clipboard_text(EXPECTED_REPLACEMENT).is_ok());
    wait_for_stage(WaitStage::BeforePaste);
    let injection = current_injection_observation(&harness, 0, 0, 0);
    unsafe {
        SendMessageW(harness.target_textbox as HWND, WM_PASTE, 0, 0);
    }
    finish_probe_evidence(
        ProbeKind::DirectWindowPaste,
        run,
        &harness,
        control,
        Some(injection),
        None,
        None,
        "reference_only",
    )
}

fn run_isolated_send_input_probe(run: usize) -> ProbeEvidence {
    let (harness, _target, control) = prepare_probe_harness();
    // CI-only fault injection proves a failure with a live owned editor still
    // tears it down, records FAIL and leaves the independent checks running.
    if run == 2 && std::env::var("GRAMMAR_OWNED_DESKTOP_FAULT").as_deref() == Ok("input_environment") {
        panic!("GRAMMAR_OWNED_DESKTOP_FAULT: deliberate input_environment failure with a live owned editor");
    }
    assert!(hide_widget_and_activate_target(&harness));
    assert!(clipboard::write_clipboard_text(EXPECTED_REPLACEMENT).is_ok());
    let pre = current_injection_observation(&harness, 4, 0, 0);
    unsafe {
        SetLastError(0);
    }
    let returned_count = clipboard::send_paste_shortcut_count();
    let last_error = unsafe { GetLastError() };
    let injection = InjectionObservation {
        returned_count,
        last_error,
        ..pre
    };
    wait_for_stage(WaitStage::AfterPaste);
    finish_probe_evidence(
        ProbeKind::IsolatedSendInput,
        run,
        &harness,
        control,
        Some(injection),
        None,
        None,
        "reference_only",
    )
}

fn assert_common_probe_preconditions(evidence: &ProbeEvidence) {
    assert!((1..=PROBE_REPETITIONS).contains(&evidence.run));
    assert!(evidence.control.top_level_created);
    assert!(evidence.control.edit_created);
    assert!(evidence.control.edit_enabled);
    assert!(evidence.control.edit_writable);
    assert!(evidence.control.edit_visible);
    assert!(evidence.control.edit_not_read_only);
    assert!(evidence.control.exact_selection);
    assert_eq!(evidence.same_session, Some(true));
    assert_eq!(evidence.same_input_desktop, Some(true));
    assert_eq!(evidence.sender_integrity, IntegrityRelation::Equal);
    if let Some(injection) = evidence.injection {
        assert!(injection.target_is_foreground);
        assert!(injection.expected_control_has_focus);
        assert!(injection.nonempty_selection);
        assert!(!injection.modifiers.any_pressed());
        assert!(injection.unicode_text_available);
        assert!(injection.clipboard_matches_expected);
        assert_ne!(injection.replacement_sequence, 0);
    }
    assert!(evidence.widget_content_unchanged);
    assert!(evidence.widget_edit_created);
    assert_eq!(evidence.widget_event_count, 0);
    assert!(evidence.final_content_equal);
    assert!(evidence.edit_modified);
    assert!(evidence.first_equal_millis.is_some());
    assert!(evidence.target_en_update_count >= 1);
    assert!(evidence.target_en_change_count >= 1);
}

fn event_position(events: &[MessageEvent], code: u8) -> usize {
    events
        .iter()
        .position(|event| event.code == code)
        .unwrap_or(usize::MAX)
}

fn assert_send_input_delivery(evidence: &ProbeEvidence) {
    let injection = evidence
        .injection
        .unwrap_or_else(|| panic!("SendInput evidence missing"));
    assert_eq!(injection.requested_count, 4);
    assert_eq!(injection.returned_count, 4);
    assert_eq!(injection.last_error, 0);
    assert_eq!(evidence.target_ctrl_down_count, 1);
    assert_eq!(evidence.target_ctrl_up_count, 1);
    assert_eq!(evidence.target_v_down_count, 1);
    assert_eq!(evidence.target_v_up_count, 1);
    assert_eq!(evidence.target_paste_count, 1);
    assert!(evidence.target_char_count >= 1);
    let ctrl_down = event_position(&evidence.target_events, 1);
    let v_down = event_position(&evidence.target_events, 3);
    let paste = event_position(&evidence.target_events, 6);
    let first_equal = event_position(&evidence.target_events, 9);
    let v_up = event_position(&evidence.target_events, 4);
    let ctrl_up = event_position(&evidence.target_events, 2);
    assert!(ctrl_down < v_down);
    assert!(v_down < paste);
    assert!(paste < first_equal);
    assert!(first_equal < v_up);
    assert!(v_up < ctrl_up);
}

/// Owner-authorized CI refs (governance contract 1.2.0 and 1.3.0 amendments).
const CLOUD_OWNED_DESKTOP_REFS: [&str; 2] = [
    "refs/heads/codex/grammar-autonomous-r1",
    "refs/heads/claude/eloquent-faraday-hh62qc",
];

/// Checks the dedicated CI opt-in before any desktop, clipboard or process use.
pub(crate) fn require_cloud_owned_desktop() {
    for (key, expected) in [
        ("GITHUB_ACTIONS", "true"),
        ("GITHUB_REPOSITORY", "CAPTW/pencil"),
        ("RUNNER_ENVIRONMENT", "github-hosted"),
        ("GRAMMAR_OWNED_DESKTOP_TEST", "1"),
    ] {
        assert_eq!(std::env::var(key).as_deref(), Ok(expected), "{key}");
    }
    let git_ref = std::env::var("GITHUB_REF").unwrap_or_default();
    assert!(CLOUD_OWNED_DESKTOP_REFS.contains(&git_ref.as_str()), "GITHUB_REF");
}

// The input-environment and Copy-only checks are separate tests (and separate
// CI steps) so a failure in one never prevents or masks the other.
#[test]
#[ignore = "mutates only an explicitly opted-in GitHub-hosted synthetic desktop"]
fn cloud_owned_desktop_input_environment() {
    let mut receipt = CloudTestReceipt::new("cloud_owned_desktop_input_environment");
    require_cloud_owned_desktop();
    receipt.check("owned_desktop_opt_in");
    assert!(clipboard::write_clipboard_text("synthetic-cloud-prior").is_ok());
    let _clipboard_guard = ClipboardTextGuard::capture().expect("synthetic clipboard roundtrip");
    receipt.check("synthetic_clipboard_roundtrip");
    for run in 1..=PROBE_REPETITIONS {
        let evidence = run_isolated_send_input_probe(run);
        eprintln!("CLOUD_OWNED_INPUT_EVIDENCE {evidence:?}");
        receipt.record_cleanups(assert_editor_cleanups_complete(1));
        receipt.check(format!("run{run}_owned_editor_cleanup"));
        assert_common_probe_preconditions(&evidence);
        receipt.check(format!("run{run}_same_session_desktop_integrity_focus"));
        assert_send_input_delivery(&evidence);
        receipt.check(format!("run{run}_sendinput_delivered_and_readback"));
    }
    receipt.pass();
}

#[test]
#[ignore = "mutates only an explicitly opted-in GitHub-hosted synthetic desktop"]
fn cloud_owned_desktop_copy_only_boundary() {
    let mut receipt = CloudTestReceipt::new("cloud_owned_desktop_copy_only_boundary");
    require_cloud_owned_desktop();
    receipt.check("owned_desktop_opt_in");
    assert!(clipboard::write_clipboard_text("synthetic-cloud-prior").is_ok());
    let _clipboard_guard = ClipboardTextGuard::capture().expect("synthetic clipboard roundtrip");
    receipt.check("synthetic_clipboard_roundtrip");
    // The legacy Apply acceptance expects mutation and predates the fail-closed
    // selection gate. Verify today's production boundary without bypassing it.
    let (harness, target) = prepare_translation_harness(EXPECTED_SOURCE, EXPECTED_SOURCE);
    receipt.check("owned_editor_ready");
    let (mut store, token) = ready_store(target, 1);
    let mut platform = LiveApplyPlatform::new(
        harness.widget_window,
        harness.target_window,
        harness.target_textbox,
    );
    assert_eq!(
        apply_current_session(&mut store, &token, EXPECTED_REPLACEMENT, false, &mut platform),
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetSelectionUnverified
        },
    );
    receipt.check("copy_only_outcome");
    assert_eq!(platform.paste_calls, 0);
    receipt.check("no_paste_input");
    assert_eq!(
        clipboard::read_clipboard_text().ok().as_deref(),
        Some(EXPECTED_REPLACEMENT)
    );
    receipt.check("replacement_on_clipboard");
    assert!(content_equals_expected(harness.target_window));
    assert!(content_equals_expected(harness.widget_window));
    receipt.check("editor_readback_unchanged");
    drop(harness);
    receipt.record_cleanups(assert_editor_cleanups_complete(1));
    receipt.check("owned_editor_cleanup");
    receipt.pass();
}

#[test]
#[ignore = "requires an interactive Windows desktop"]
fn windows_live_sendinput_root_cause_probes() {
    let clipboard_guard = ClipboardTextGuard::capture();
    assert!(clipboard_guard.is_ok());
    let _clipboard_guard = clipboard_guard.ok();

    let mut evidence = Vec::with_capacity(PROBE_REPETITIONS * 4);
    for run in 1..=PROBE_REPETITIONS {
        evidence.push(run_apply_probe(ProbeKind::CurrentCandidate, run, true));
        evidence.push(run_apply_probe(ProbeKind::RestoreSuppressed, run, false));
        evidence.push(run_direct_window_paste_probe(run));
        evidence.push(run_isolated_send_input_probe(run));
    }

    for item in &evidence {
        eprintln!("P0_02B_CONTENT_FREE_EVIDENCE {item:?}");
        assert_common_probe_preconditions(item);
        match item.kind {
            ProbeKind::CurrentCandidate => {
                assert_send_input_delivery(item);
                assert_eq!(item.outcome, "applied");
                assert!(item.restore_started_millis.is_some());
                assert!(item.restore_completed_millis.is_some());
                assert!(item.restore_started_millis <= item.restore_completed_millis);
            }
            ProbeKind::RestoreSuppressed => {
                assert_send_input_delivery(item);
                assert_eq!(item.outcome, "applied");
                assert!(item.restore_started_millis.is_none());
                assert!(item.restore_completed_millis.is_none());
            }
            ProbeKind::DirectWindowPaste => {
                let injection = item
                    .injection
                    .unwrap_or_else(|| panic!("WM_PASTE reference evidence missing"));
                assert_eq!(injection.requested_count, 0);
                assert_eq!(injection.returned_count, 0);
                assert_eq!(item.target_ctrl_down_count, 0);
                assert_eq!(item.target_ctrl_up_count, 0);
                assert_eq!(item.target_v_down_count, 0);
                assert_eq!(item.target_v_up_count, 0);
                assert_eq!(item.target_paste_count, 1);
                assert!(
                    event_position(&item.target_events, 6) < event_position(&item.target_events, 9)
                );
            }
            ProbeKind::IsolatedSendInput => assert_send_input_delivery(item),
        }
    }
}

#[test]
#[ignore = "requires an interactive Windows desktop"]
fn windows_live_target_bound_apply_acceptance() {
    let clipboard_guard = ClipboardTextGuard::capture();
    assert!(clipboard_guard.is_ok());
    let _clipboard_guard = clipboard_guard.ok();
    let (harness, target, control) = prepare_probe_harness();
    assert!(control.top_level_created);
    assert!(control.edit_created);
    assert!(control.edit_enabled);
    assert!(control.edit_writable);
    assert!(control.edit_visible);
    assert!(control.edit_not_read_only);
    assert!(control.exact_selection);
    assert_eq!(harness.same_session, Some(true));
    assert_eq!(harness.same_input_desktop, Some(true));
    assert_eq!(harness.sender_integrity, IntegrityRelation::Equal);

    assert!(clipboard::write_clipboard_text("synthetic-live-prior").is_ok());
    let capture_sequence = clipboard::clipboard_sequence_number();
    let mut store = CaptureSessionStore::default();
    let first = store
        .capture(
            "synthetic-live-session-1".to_string(),
            "synthetic-target-start".to_string(),
            target,
            Some("synthetic-live-prior".to_string()),
            Some(capture_sequence),
        )
        .unwrap_or_else(|_| unreachable!());
    assert!(store.begin_rewrite(&first).is_ok());
    assert!(store.finish_rewrite_success(&first).is_ok());
    let mut platform = LiveApplyPlatform::new(
        harness.widget_window,
        harness.target_window,
        harness.target_textbox,
    );

    let success = apply_current_session(
        &mut store,
        &first,
        "synthetic-target-replaced",
        false,
        &mut platform,
    );
    assert_eq!(success, ApplyOutcome::Applied);
    assert_eq!(platform.paste_calls, 1);
    assert!(
        foreground_window_handle() == harness.target_window,
        "synthetic target lost foreground during accepted input"
    );
    assert!(
        focused_window(harness.target_window) == Some(harness.target_textbox),
        "synthetic target did not own keyboard focus after activation"
    );
    let target_was_replaced = wait_until(Duration::from_secs(2), || {
        content_equals_expected(harness.target_window)
    });
    assert!(content_equals_expected(harness.widget_window));

    let second = store
        .capture(
            "synthetic-live-session-2".to_string(),
            "synthetic-second-source".to_string(),
            target,
            Some("synthetic-live-prior".to_string()),
            Some(clipboard::clipboard_sequence_number()),
        )
        .unwrap_or_else(|_| unreachable!());
    assert!(store.begin_rewrite(&second).is_ok());
    assert!(store.finish_rewrite_success(&second).is_ok());
    let writes_before_stale = platform.clipboard_writes;
    let paste_before_stale = platform.paste_calls;
    assert_eq!(
        apply_current_session(
            &mut store,
            &first,
            "synthetic-stale-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::RejectedStale
    );
    assert_eq!(platform.clipboard_writes, writes_before_stale);
    assert_eq!(platform.paste_calls, paste_before_stale);

    unsafe {
        SendMessageW(harness.target_window as HWND, WM_CLOSE, 0, 0);
    }
    assert!(wait_until(Duration::from_secs(2), || {
        !window_is_valid(harness.target_window)
    }));
    let paste_before_fallback = platform.paste_calls;
    assert_eq!(
        apply_current_session(
            &mut store,
            &second,
            "synthetic-manual-paste",
            true,
            &mut platform,
        ),
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMissing,
        }
    );
    assert_eq!(platform.paste_calls, paste_before_fallback);
    assert!(matches!(
        clipboard::read_clipboard_text().ok().as_deref(),
        Some("synthetic-manual-paste")
    ));
    assert!(content_equals_expected(harness.widget_window));
    assert!(
        target_was_replaced,
        "SendInput was accepted but the synthetic target did not consume the paste"
    );
}

fn prepare_translation_harness(source: &str, expected: &str) -> (WindowHarness, WindowTarget) {
    let harness = WindowHarness::spawn_with(source, expected)
        .unwrap_or_else(|_| panic!("translation helper did not start"));
    assert!(activate_target_for_setup(&harness));
    let mut capture_platform = WindowsForegroundTargetPlatform;
    let target = capture_foreground_target(
        &mut capture_platform,
        harness.widget_window,
        std::process::id(),
    )
    .unwrap_or_else(|_| panic!("translation target capture failed"));
    request_foreground_window(harness.widget_window);
    assert!(wait_until(Duration::from_secs(2), || {
        foreground_window_handle() == harness.widget_window
    }));
    assert!(reset_probe_state(&harness));
    (harness, target)
}

#[test]
#[ignore = "requires an interactive Windows desktop"]
fn p1_01_windows_live_translation_formats_acceptance() {
    let clipboard_guard = ClipboardTextGuard::capture();
    assert!(clipboard_guard.is_ok());
    let _clipboard_guard = clipboard_guard.ok();
    let cases = [
        (
            "synthetic Korean source",
            "synthetic English translation",
            TranslationApplyFormat::TranslationOnly,
        ),
        (
            "synthetic source line",
            "synthetic translated line",
            TranslationApplyFormat::SourceWithTranslation,
        ),
        (
            "synthetic first\r\nsynthetic second",
            "translated first\r\ntranslated second",
            TranslationApplyFormat::SourceWithTranslation,
        ),
    ];
    let intent = RewriteIntent::new(RewriteMode::Translate, Some(TranslationTargetLanguage::En))
        .unwrap_or_else(|_| panic!("translation intent fixture is invalid"));
    let stale_intent =
        RewriteIntent::new(RewriteMode::Translate, Some(TranslationTargetLanguage::Ja))
            .unwrap_or_else(|_| panic!("stale intent fixture is invalid"));

    for repetition in 0..2 {
        for (case_index, (source, translated, format)) in cases.iter().enumerate() {
            let expected = format_translation(source, translated, *format)
                .unwrap_or_else(|_| panic!("translation format fixture failed"));
            let (harness, target) = prepare_translation_harness(source, &expected);
            assert_eq!(harness.same_session, Some(true));
            assert_eq!(harness.same_input_desktop, Some(true));
            assert_eq!(harness.sender_integrity, IntegrityRelation::Equal);
            assert!(clipboard::write_clipboard_text("synthetic-translation-prior").is_ok());
            let capture_sequence = clipboard::clipboard_sequence_number();
            let mut store = CaptureSessionStore::default();
            let token = store
                .capture(
                    format!("synthetic-translation-{repetition}-{case_index}"),
                    (*source).to_string(),
                    target,
                    Some("synthetic-translation-prior".to_string()),
                    Some(capture_sequence),
                )
                .unwrap_or_else(|_| panic!("translation capture fixture failed"));
            assert!(store.begin_rewrite_for(&token, intent).is_ok());
            assert!(store.finish_rewrite_success_for(&token, intent).is_ok());

            let mut platform = LiveApplyPlatform::new(
                harness.widget_window,
                harness.target_window,
                harness.target_textbox,
            );
            assert!(store.invalidate_intent(stale_intent).is_ok());
            assert_eq!(
                store.ready_source_for(&token, intent),
                Err(crate::capture_session::SessionError::StaleIntent)
            );
            assert_eq!(platform.paste_calls, 0);
            assert_eq!(platform.clipboard_writes, 0);
            assert!(store.begin_rewrite_for(&token, intent).is_ok());
            assert!(store.finish_rewrite_success_for(&token, intent).is_ok());

            let backend_source = store
                .ready_source_for(&token, intent)
                .unwrap_or_else(|_| panic!("backend source fixture was unavailable"));
            let final_replacement = format_translation(&backend_source, translated, *format)
                .unwrap_or_else(|_| panic!("backend translation composition failed"));
            assert_eq!(final_replacement, expected);
            assert_eq!(
                apply_current_session(&mut store, &token, &final_replacement, false, &mut platform,),
                ApplyOutcome::Applied
            );
            assert_eq!(platform.paste_calls, 1);
            assert!(wait_until(Duration::from_secs(2), || {
                content_equals_expected(harness.target_window)
            }));
            assert!(content_equals_expected(harness.widget_window));
            assert_eq!(event_count(&probe_events(harness.widget_window), 6), 0);

            let fallback = store
                .capture(
                    format!("synthetic-translation-fallback-{repetition}-{case_index}"),
                    (*source).to_string(),
                    target,
                    Some("synthetic-translation-prior".to_string()),
                    Some(clipboard::clipboard_sequence_number()),
                )
                .unwrap_or_else(|_| panic!("fallback capture fixture failed"));
            assert!(store.begin_rewrite_for(&fallback, intent).is_ok());
            assert!(store.finish_rewrite_success_for(&fallback, intent).is_ok());
            unsafe {
                SendMessageW(harness.target_window as HWND, WM_CLOSE, 0, 0);
            }
            assert!(wait_until(Duration::from_secs(2), || {
                !window_is_valid(harness.target_window)
            }));
            let paste_before_fallback = platform.paste_calls;
            assert_eq!(
                apply_current_session(
                    &mut store,
                    &fallback,
                    &final_replacement,
                    false,
                    &mut platform,
                ),
                ApplyOutcome::CopiedFallback {
                    reason: ApplyFallbackReason::TargetMissing,
                }
            );
            assert_eq!(platform.paste_calls, paste_before_fallback);
            assert_eq!(
                clipboard::read_clipboard_text().ok().as_deref(),
                Some(final_replacement.as_str())
            );
            assert!(content_equals_expected(harness.widget_window));
        }
    }
}

pub(crate) fn run_p1_02_live_target_bound_terminology_apply() {
    let clipboard_guard = ClipboardTextGuard::capture();
    assert!(clipboard_guard.is_ok());
    let _clipboard_guard = clipboard_guard.ok();
    let source = "자유수면효과 CargoMax";
    let replacement = "free surface effect CargoMax";
    let (harness, target) = prepare_translation_harness(source, replacement);
    assert_eq!(harness.same_session, Some(true));
    assert_eq!(harness.same_input_desktop, Some(true));
    assert_eq!(harness.sender_integrity, IntegrityRelation::Equal);

    assert!(clipboard::write_clipboard_text("synthetic-terminology-prior").is_ok());
    let mut store = CaptureSessionStore::default();
    let intent = RewriteIntent::new(RewriteMode::Translate, Some(TranslationTargetLanguage::En))
        .unwrap_or_else(|_| panic!("terminology translation intent fixture is invalid"));
    let settings = TerminologySettings {
        active_profile_id: "maritime".to_string(),
        ..TerminologySettings::default()
    };
    let bound = BoundRewriteIntent::new(
        intent,
        TerminologyIntent {
            enabled: true,
            use_approved_terminology: true,
            suggest_terminology: true,
            active_profile_id: "maritime".to_string(),
            store_revision: 7,
            matched_entry_ids: vec![
                "entry-live-protected".to_string(),
                "entry-live-translation".to_string(),
            ],
        },
    );
    let token = store
        .capture(
            "synthetic-terminology-live-1".to_string(),
            source.to_string(),
            target,
            Some("synthetic-terminology-prior".to_string()),
            Some(clipboard::clipboard_sequence_number()),
        )
        .unwrap_or_else(|_| panic!("terminology capture fixture failed"));
    assert!(store.begin_rewrite_bound(&token, bound.clone()).is_ok());
    assert!(store.finish_rewrite_success_bound(&token, &bound).is_ok());
    let mut platform = LiveApplyPlatform::new(
        harness.widget_window,
        harness.target_window,
        harness.target_textbox,
    );

    assert_eq!(
        apply_current_terminology_bound(
            &mut store,
            &token,
            &bound,
            intent,
            &settings,
            Some(8),
            replacement,
            false,
            &mut platform,
        ),
        ApplyOutcome::RejectedStale
    );
    assert_eq!(platform.clipboard_writes, 0);
    assert_eq!(platform.paste_calls, 0);

    assert_eq!(
        apply_current_terminology_bound(
            &mut store,
            &token,
            &bound,
            intent,
            &settings,
            Some(7),
            replacement,
            false,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
    assert_eq!(platform.paste_calls, 1);
    assert!(wait_until(Duration::from_secs(2), || {
        content_equals_expected(harness.target_window)
    }));
    assert!(content_equals_expected(harness.widget_window));
    assert_eq!(event_count(&probe_events(harness.widget_window), 6), 0);

    let fallback_token = store
        .capture(
            "synthetic-terminology-live-2".to_string(),
            source.to_string(),
            target,
            Some("synthetic-terminology-prior".to_string()),
            Some(clipboard::clipboard_sequence_number()),
        )
        .unwrap_or_else(|_| panic!("fallback terminology capture fixture failed"));
    assert!(store
        .begin_rewrite_bound(&fallback_token, bound.clone())
        .is_ok());
    assert!(store
        .finish_rewrite_success_bound(&fallback_token, &bound)
        .is_ok());
    unsafe {
        SendMessageW(harness.target_window as HWND, WM_CLOSE, 0, 0);
    }
    assert!(wait_until(Duration::from_secs(2), || {
        !window_is_valid(harness.target_window)
    }));
    let paste_before_fallback = platform.paste_calls;
    assert_eq!(
        apply_current_terminology_bound(
            &mut store,
            &fallback_token,
            &bound,
            intent,
            &settings,
            Some(7),
            replacement,
            false,
            &mut platform,
        ),
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMissing,
        }
    );
    assert_eq!(platform.paste_calls, paste_before_fallback);
    assert_eq!(
        clipboard::read_clipboard_text().ok().as_deref(),
        Some(replacement)
    );
    assert!(content_equals_expected(harness.widget_window));
}
