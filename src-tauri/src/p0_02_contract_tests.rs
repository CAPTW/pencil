use crate::apply_safety::{
    apply_current_session, ApplyFailureReason, ApplyFallbackReason, ApplyOutcome, ApplyPlatform,
    WaitStage,
};
use crate::capture_session::{CaptureSessionStore, SessionError, SessionToken, WindowTarget};
use crate::windows_target::{
    capture_before_widget_focus, capture_foreground_target, ForegroundTargetPlatform,
    TargetCaptureError,
};
use crate::SelectionCapturedEvent;
use std::sync::{Arc as StdArc, Mutex as StdMutex};

fn capture(
    store: &mut CaptureSessionStore,
    session_id: &str,
    hwnd: isize,
    pid: u32,
) -> SessionToken {
    store
        .capture(
            session_id.to_string(),
            format!("synthetic-source-{session_id}"),
            WindowTarget::new(hwnd, pid),
            Some(format!("synthetic-prior-{session_id}")),
            Some(41),
        )
        .unwrap()
}

fn ready_session() -> (CaptureSessionStore, SessionToken) {
    let mut store = CaptureSessionStore::default();
    let token = capture(&mut store, "ready", 101, 2001);
    store.begin_rewrite(&token).unwrap();
    store.finish_rewrite_success(&token).unwrap();
    (store, token)
}

#[test]
fn new_capture_supersedes_the_old_session_and_advances_generation() {
    let mut store = CaptureSessionStore::default();
    let first = capture(&mut store, "first", 101, 2001);
    let second = capture(&mut store, "second", 102, 2002);

    assert_eq!(first.generation, 1);
    assert_eq!(second.generation, 2);
    assert_eq!(store.begin_rewrite(&first), Err(SessionError::StaleSession));
    assert!(store.begin_rewrite(&second).is_ok());
}

#[test]
fn generation_mismatch_is_rejected_without_consuming_the_current_capture() {
    let mut store = CaptureSessionStore::default();
    let current = capture(&mut store, "current", 101, 2001);
    let mismatched = SessionToken {
        session_id: current.session_id.clone(),
        generation: current.generation + 1,
    };

    assert_eq!(
        store.begin_rewrite(&mismatched),
        Err(SessionError::StaleSession)
    );
    assert!(store.begin_rewrite(&current).is_ok());
    assert_eq!(
        store.begin_rewrite(&current),
        Err(SessionError::InvalidState)
    );
}

#[test]
fn rewrite_completion_after_a_new_capture_is_discarded() {
    let mut store = CaptureSessionStore::default();
    let old = capture(&mut store, "old", 101, 2001);
    store.begin_rewrite(&old).unwrap();

    let current = capture(&mut store, "current", 102, 2002);

    assert_eq!(
        store.finish_rewrite_success(&old),
        Err(SessionError::StaleSession)
    );
    assert!(store.begin_rewrite(&current).is_ok());
}

#[test]
fn current_rewrite_failure_returns_to_captured_for_retry() {
    let mut store = CaptureSessionStore::default();
    let token = capture(&mut store, "retry", 101, 2001);

    store.begin_rewrite(&token).unwrap();
    store.finish_rewrite_failure(&token).unwrap();

    assert!(store.begin_rewrite(&token).is_ok());
}

#[test]
fn only_ready_can_apply_and_completed_or_applying_cannot_apply_again() {
    let (mut store, token) = ready_session();

    assert!(store.begin_apply(&token).is_ok());
    assert_eq!(store.begin_apply(&token), Err(SessionError::InvalidState));
    store.finish_apply_success(&token).unwrap();
    assert_eq!(store.begin_apply(&token), Err(SessionError::InvalidState));
}

#[test]
fn cancel_or_dismiss_invalidates_apply() {
    let (mut store, token) = ready_session();

    store.cancel(&token).unwrap();

    assert_eq!(store.begin_apply(&token), Err(SessionError::InvalidState));
}

#[test]
fn clipboard_write_failure_before_paste_returns_applying_to_ready() {
    let (mut store, token) = ready_session();

    store.begin_apply(&token).unwrap();
    store.finish_apply_before_paste_failure(&token).unwrap();

    assert!(store.begin_apply(&token).is_ok());
}

#[test]
fn zero_or_partial_input_cancels_the_session_and_prevents_retry() {
    let (mut store, token) = ready_session();

    store.begin_apply(&token).unwrap();
    store.finish_apply_uncertain_input(&token).unwrap();

    assert_eq!(store.begin_apply(&token), Err(SessionError::InvalidState));
}

#[derive(Debug)]
struct FakeApplyPlatform {
    verified_selection: bool,
    target: isize,
    target_exists: bool,
    target_pid: Option<u32>,
    foreground: isize,
    activation_succeeds: bool,
    hide_succeeds: bool,
    clipboard_text: Option<String>,
    clipboard_sequence: u32,
    write_fails: bool,
    paste_events_sent: u32,
    paste_calls: usize,
    clipboard_writes: usize,
    widget_shows: usize,
    external_change_at: Option<WaitStage>,
    events: Vec<&'static str>,
}

impl Default for FakeApplyPlatform {
    fn default() -> Self {
        Self {
            verified_selection: true,
            target: 101,
            target_exists: true,
            target_pid: Some(2001),
            foreground: 909,
            activation_succeeds: true,
            hide_succeeds: true,
            clipboard_text: Some("synthetic-prior-ready".to_string()),
            clipboard_sequence: 41,
            write_fails: false,
            paste_events_sent: 4,
            paste_calls: 0,
            clipboard_writes: 0,
            widget_shows: 0,
            external_change_at: None,
            events: Vec::new(),
        }
    }
}

impl ApplyPlatform for FakeApplyPlatform {
    // Synthetic state-machine fixture only, not Windows editor acceptance.
    fn has_verified_selection_authority(&mut self) -> bool { self.verified_selection }

    fn is_window(&mut self, hwnd: isize) -> bool {
        self.events.push("is_window");
        hwnd == self.target && self.target_exists
    }

    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        self.events.push("window_pid");
        self.target_pid
    }

    fn hide_widget(&mut self) -> Result<(), ()> {
        self.events.push("hide_widget");
        if self.hide_succeeds {
            Ok(())
        } else {
            Err(())
        }
    }

    fn show_widget(&mut self) {
        self.events.push("show_widget");
        self.widget_shows += 1;
    }

    fn request_foreground(&mut self, hwnd: isize) {
        self.events.push("request_foreground");
        if self.activation_succeeds {
            self.foreground = hwnd;
        }
    }

    fn foreground_window(&mut self) -> isize {
        self.events.push("foreground_window");
        self.foreground
    }

    fn clipboard_sequence(&mut self) -> u32 {
        self.events.push("clipboard_sequence");
        self.clipboard_sequence
    }

    fn read_clipboard_text(&mut self) -> Option<String> {
        self.events.push("read_clipboard_text");
        self.clipboard_text.clone()
    }

    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()> {
        self.events.push("write_clipboard_text");
        if self.write_fails {
            return Err(());
        }
        self.clipboard_text = Some(text.to_string());
        self.clipboard_sequence = self.clipboard_sequence.wrapping_add(1);
        self.clipboard_writes += 1;
        Ok(())
    }

    fn send_paste(&mut self) -> u32 {
        self.events.push("send_paste");
        self.paste_calls += 1;
        self.paste_events_sent
    }

    fn wait(&mut self, stage: WaitStage) {
        match stage {
            WaitStage::AfterActivation => self.events.push("wait_after_activation"),
            WaitStage::BeforePaste => self.events.push("wait_before_paste"),
            WaitStage::AfterPaste => self.events.push("wait_after_paste"),
        }
        if self.external_change_at == Some(stage) {
            self.clipboard_text = Some("synthetic-external-change".to_string());
            self.clipboard_sequence = self.clipboard_sequence.wrapping_add(1);
        }
    }
}

fn event_index(events: &[&str], wanted: &str) -> usize {
    events
        .iter()
        .position(|event| *event == wanted)
        .unwrap_or(usize::MAX)
}

#[test]
fn verified_apply_orders_hide_activation_clipboard_and_one_paste() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform::default();

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(outcome, ApplyOutcome::Applied);
    assert_eq!(platform.paste_calls, 1);
    assert!(
        event_index(&platform.events, "hide_widget")
            < event_index(&platform.events, "request_foreground")
    );
    assert!(
        event_index(&platform.events, "foreground_window")
            < event_index(&platform.events, "write_clipboard_text")
    );
    assert!(
        event_index(&platform.events, "write_clipboard_text")
            < event_index(&platform.events, "send_paste")
    );
}

#[test]
fn missing_target_uses_copy_fallback_and_sends_no_paste() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        target_exists: false,
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMissing,
        }
    );
    assert_eq!(platform.paste_calls, 0);
    assert_eq!(platform.clipboard_writes, 1);
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    ));
}

#[test]
fn pid_mismatch_uses_copy_fallback_and_sends_no_paste() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        target_pid: Some(9999),
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetProcessChanged,
        }
    );
    assert_eq!(platform.paste_calls, 0);
}

#[test]
fn focus_failure_uses_copy_fallback_sends_no_paste_and_reshows_widget() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        activation_succeeds: false,
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetNotForeground,
        }
    );
    assert_eq!(platform.paste_calls, 0);
    assert_eq!(platform.widget_shows, 1);
}

#[test]
fn stale_apply_makes_zero_clipboard_or_input_changes() {
    let (mut store, stale) = ready_session();
    let _current = capture(&mut store, "new-current", 102, 2002);
    let mut platform = FakeApplyPlatform::default();

    let outcome = apply_current_session(
        &mut store,
        &stale,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(outcome, ApplyOutcome::RejectedStale);
    assert_eq!(platform.clipboard_writes, 0);
    assert_eq!(platform.paste_calls, 0);
    assert!(platform.events.is_empty());
}

#[test]
fn replacement_write_failure_sends_no_paste_and_allows_safe_retry() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        write_fails: true,
        ..FakeApplyPlatform::default()
    };

    let first = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(
        first,
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        }
    );
    assert_eq!(platform.paste_calls, 0);

    platform.write_fails = false;
    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
}

#[test]
fn newer_supported_text_before_apply_becomes_the_restore_candidate() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        clipboard_text: Some("synthetic-newer-before-apply".to_string()),
        clipboard_sequence: 42,
        ..FakeApplyPlatform::default()
    };

    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-newer-before-apply")
    ));
}

#[test]
fn newer_supported_text_after_replacement_is_not_overwritten_by_restore() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        external_change_at: Some(WaitStage::AfterPaste),
        ..FakeApplyPlatform::default()
    };

    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-external-change")
    ));
}

#[test]
fn app_owned_capture_sequence_allows_supported_text_restore() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform::default();

    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-prior-ready")
    ));
}

#[test]
fn unowned_capture_sequence_uses_current_supported_text_for_restore() {
    let mut store = CaptureSessionStore::default();
    let token = store
        .capture(
            "unowned".to_string(),
            "synthetic-source-unowned".to_string(),
            WindowTarget::new(101, 2001),
            Some("synthetic-old-prior".to_string()),
            None,
        )
        .unwrap();
    store.begin_rewrite(&token).unwrap();
    store.finish_rewrite_success(&token).unwrap();
    let mut platform = FakeApplyPlatform {
        clipboard_text: Some("synthetic-current-supported".to_string()),
        clipboard_sequence: 41,
        ..FakeApplyPlatform::default()
    };

    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Applied
    );
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-current-supported")
    ));
}

#[test]
fn copy_fallback_intentionally_leaves_replacement_in_clipboard() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        target_exists: false,
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert!(matches!(outcome, ApplyOutcome::CopiedFallback { .. }));
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    ));
}

#[test]
fn partial_send_input_is_failed_and_the_session_cannot_auto_retry() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        paste_events_sent: 2,
        ..FakeApplyPlatform::default()
    };

    let first = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        true,
        &mut platform,
    );

    assert_eq!(
        first,
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::InputInjectionFailed,
        }
    );
    let paste_calls = platform.paste_calls;
    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            true,
            &mut platform,
        ),
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::InvalidSessionState,
        }
    );
    assert_eq!(platform.paste_calls, paste_calls);
}

#[derive(Debug)]
struct FakeForegroundTargetPlatform {
    foreground: isize,
    pid: Option<u32>,
    events: StdArc<StdMutex<Vec<&'static str>>>,
}

impl ForegroundTargetPlatform for FakeForegroundTargetPlatform {
    fn foreground_window(&mut self) -> isize {
        self.events.lock().unwrap().push("capture_foreground");
        self.foreground
    }

    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        self.events.lock().unwrap().push("capture_pid");
        self.pid
    }
}

#[tokio::test]
async fn foreground_target_is_captured_and_committed_before_widget_focus() {
    let events = StdArc::new(StdMutex::new(Vec::new()));
    let mut platform = FakeForegroundTargetPlatform {
        foreground: 101,
        pid: Some(2001),
        events: events.clone(),
    };
    let prepare_events = events.clone();
    let focus_events = events.clone();

    let (_, prepared) = capture_before_widget_focus(
        &mut platform,
        909,
        9009,
        move |target| async move {
            prepare_events.lock().unwrap().push("commit_session");
            Ok::<_, String>(target)
        },
        move |_| {
            focus_events.lock().unwrap().push("focus_widget");
            Ok::<_, String>(())
        },
    )
    .await
    .unwrap();

    assert_eq!(prepared, WindowTarget::new(101, 2001));
    assert_eq!(
        events.lock().unwrap().as_slice(),
        [
            "capture_foreground",
            "capture_pid",
            "commit_session",
            "focus_widget"
        ]
    );
}

#[test]
fn zero_widget_or_own_process_target_is_rejected() {
    let events = StdArc::new(StdMutex::new(Vec::new()));

    let mut zero = FakeForegroundTargetPlatform {
        foreground: 0,
        pid: None,
        events: events.clone(),
    };
    assert_eq!(
        capture_foreground_target(&mut zero, 909, 9009),
        Err(TargetCaptureError::MissingForegroundWindow)
    );

    let mut widget = FakeForegroundTargetPlatform {
        foreground: 909,
        pid: Some(9009),
        events: events.clone(),
    };
    assert_eq!(
        capture_foreground_target(&mut widget, 909, 9009),
        Err(TargetCaptureError::OwnWindow)
    );

    let mut own_process = FakeForegroundTargetPlatform {
        foreground: 101,
        pid: Some(9009),
        events,
    };
    assert_eq!(
        capture_foreground_target(&mut own_process, 909, 9009),
        Err(TargetCaptureError::OwnProcess)
    );
}

#[test]
fn selection_event_exposes_only_token_and_selected_text() {
    let payload = SelectionCapturedEvent {
        session_id: "synthetic-session".to_string(),
        generation: 7,
        selected_text: "synthetic-selected-source".to_string(),
    };

    let value = serde_json::to_value(payload).unwrap();
    let object = value.as_object().unwrap();
    let mut keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();

    assert_eq!(keys, ["generation", "selectedText", "sessionId"]);
    assert!(object.get("hwnd").is_none());
    assert!(object.get("pid").is_none());
}

#[test]
fn mission_unverified_selection_is_copy_only_without_focus_or_paste() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform { verified_selection: false, ..FakeApplyPlatform::default() };
    let outcome = apply_current_session(&mut store, &token, "edited draft", true, &mut platform);
    assert_eq!(outcome, ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionUnverified });
    assert_eq!(platform.clipboard_text.as_deref(), Some("edited draft"));
    assert_eq!(platform.paste_calls, 0);
    assert!(!platform.events.contains(&"hide_widget"));
    assert!(!platform.events.contains(&"request_foreground"));
    assert!(!platform.events.contains(&"read_clipboard_text"));
}

#[test]
fn mission_edited_instant_proof_to_explicit_copy_fallback_keeps_source_binding() {
    use crate::capture_session::{BoundRewriteIntent, InstantDraftProof};
    use crate::translation::RewriteIntent;
    let mut store = CaptureSessionStore::default();
    let token = capture(&mut store, "edited", 101, 2001);
    let proof: InstantDraftProof = serde_json::from_value(serde_json::json!({
        "sessionId": token.session_id, "generation": token.generation,
        "source": "synthetic-source-edited", "candidate": "local Instant", "draftRevision": 2, "userEdited": true
    })).unwrap();
    store.validate_instant_draft(&token, "local Instant", "user final draft", Some(&proof)).unwrap();
    let bound = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store.begin_rewrite_bound(&token, bound.clone()).unwrap();
    store.finish_rewrite_success_bound(&token, &bound).unwrap();
    assert_eq!(store.ready_source_for(&token, RewriteIntent::grammar()).unwrap(), "synthetic-source-edited");
    let mut platform = FakeApplyPlatform { verified_selection: false, ..FakeApplyPlatform::default() };
    assert_eq!(apply_current_session(&mut store, &token, "user final draft", true, &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionUnverified });
    assert_eq!(platform.clipboard_text.as_deref(), Some("user final draft"));
    assert_eq!(platform.paste_calls, 0);
    assert!(store.begin_apply(&token).is_err());
}
