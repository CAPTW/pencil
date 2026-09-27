use crate::apply_safety::{
    apply_current_session, ApplyFailureReason, ApplyFallbackReason, ApplyOutcome, ApplyPlatform,
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
fn clipboard_write_failure_returns_applying_to_ready() {
    let (mut store, token) = ready_session();

    store.begin_apply(&token).unwrap();
    store.finish_apply_failure(&token).unwrap();

    assert!(store.begin_apply(&token).is_ok());
}

/// Synthetic state-machine fixture: the platform can only check the captured
/// window and write the clipboard.
#[derive(Debug)]
struct FakeApplyPlatform {
    target: isize,
    target_exists: bool,
    target_pid: Option<u32>,
    clipboard_text: Option<String>,
    write_fails: bool,
    clipboard_writes: usize,
    events: Vec<&'static str>,
}

impl Default for FakeApplyPlatform {
    fn default() -> Self {
        Self {
            target: 101,
            target_exists: true,
            target_pid: Some(2001),
            clipboard_text: Some("synthetic-prior-ready".to_string()),
            write_fails: false,
            clipboard_writes: 0,
            events: Vec::new(),
        }
    }
}

impl ApplyPlatform for FakeApplyPlatform {
    fn is_window(&mut self, hwnd: isize) -> bool {
        self.events.push("is_window");
        hwnd == self.target && self.target_exists
    }

    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        self.events.push("window_pid");
        self.target_pid
    }

    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()> {
        self.events.push("write_clipboard_text");
        if self.write_fails {
            return Err(());
        }
        self.clipboard_text = Some(text.to_string());
        self.clipboard_writes += 1;
        Ok(())
    }
}

#[test]
fn copy_checks_the_window_then_writes_only_the_clipboard_and_completes() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform::default();

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetSelectionUnverified,
        }
    );
    assert_eq!(platform.events, ["is_window", "window_pid", "write_clipboard_text"]);
    assert_eq!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    );
    assert_eq!(store.begin_apply(&token), Err(SessionError::InvalidState));
}

#[test]
fn missing_target_still_copies_with_its_reason() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        target_exists: false,
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetMissing,
        }
    );
    assert_eq!(platform.clipboard_writes, 1);
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    ));
}

#[test]
fn pid_mismatch_still_copies_with_its_reason() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        target_pid: Some(9999),
        ..FakeApplyPlatform::default()
    };

    let outcome = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        &mut platform,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetProcessChanged,
        }
    );
    assert_eq!(platform.clipboard_writes, 1);
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
        &mut platform,
    );

    assert_eq!(outcome, ApplyOutcome::RejectedStale);
    assert_eq!(platform.clipboard_writes, 0);
    assert!(platform.events.is_empty());
}

#[test]
fn replacement_write_failure_allows_safe_retry() {
    let (mut store, token) = ready_session();
    let mut platform = FakeApplyPlatform {
        write_fails: true,
        ..FakeApplyPlatform::default()
    };

    let first = apply_current_session(
        &mut store,
        &token,
        "synthetic-approved-replacement",
        &mut platform,
    );

    assert_eq!(
        first,
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        }
    );

    platform.write_fails = false;
    assert_eq!(
        apply_current_session(
            &mut store,
            &token,
            "synthetic-approved-replacement",
            &mut platform,
        ),
        ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetSelectionUnverified,
        }
    );
    assert_eq!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    );
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
        &mut platform,
    );

    assert!(matches!(outcome, ApplyOutcome::CopiedFallback { .. }));
    assert!(matches!(
        platform.clipboard_text.as_deref(),
        Some("synthetic-approved-replacement")
    ));
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
    let mut platform = FakeApplyPlatform::default();
    let outcome = apply_current_session(&mut store, &token, "edited draft", &mut platform);
    assert_eq!(outcome, ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionUnverified });
    assert_eq!(platform.clipboard_text.as_deref(), Some("edited draft"));
    assert_eq!(platform.events, ["is_window", "window_pid", "write_clipboard_text"]);
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
    let mut platform = FakeApplyPlatform::default();
    assert_eq!(apply_current_session(&mut store, &token, "user final draft", &mut platform),
        ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetSelectionUnverified });
    assert_eq!(platform.clipboard_text.as_deref(), Some("user final draft"));
    assert!(store.begin_apply(&token).is_err());
}

// Pre-use review: Cancel on the widget stops only Deep. The capture must stay
// usable (its edited Instant draft can still be copied), and a late end of the
// cancelled request must not end a Deep request the user started afterwards.
#[test]
fn cancel_deep_keeps_the_capture_and_its_instant_draft_usable() {
    use crate::capture_session::{BoundRewriteIntent, InstantDraftProof};
    use crate::translation::RewriteIntent;
    let mut store = CaptureSessionStore::default();
    let token = capture(&mut store, "cancel", 101, 2001);
    let bound = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store.begin_rewrite_bound(&token, bound.clone()).unwrap();
    let first = store.rewrite_attempt(&token).unwrap();
    assert!(store.owns_rewrite(&token, first));

    assert!(store.cancel_rewrite_keep_capture().is_none(), "no Codex turn was bound");
    assert!(store.is_captured(&token), "Cancel returns the capture to Captured");
    assert!(!store.owns_rewrite(&token, first), "the cancelled request no longer owns the capture");
    assert!(store.cancel_rewrite_keep_capture().is_none(), "a second Cancel changes nothing");
    let proof: InstantDraftProof = serde_json::from_value(serde_json::json!({
        "sessionId": token.session_id, "generation": token.generation,
        "source": "synthetic-source-cancel", "candidate": "local Instant", "draftRevision": 1, "userEdited": true
    })).unwrap();
    store.validate_instant_draft(&token, "local Instant", "edited draft", Some(&proof)).unwrap();

    // Deep again: the new request owns the capture, the cancelled one does not.
    store.begin_rewrite_bound(&token, bound.clone()).unwrap();
    let second = store.rewrite_attempt(&token).unwrap();
    assert_ne!(first, second);
    assert!(store.owns_rewrite(&token, second));
    assert!(!store.owns_rewrite(&token, first), "a late end of the cancelled request is ignored");
    store.finish_rewrite_success_bound(&token, &bound).unwrap();
    assert!(!store.owns_rewrite(&token, second), "a finished request no longer owns the capture");
}

#[test]
fn closing_the_widget_still_ends_the_capture() {
    use crate::capture_session::BoundRewriteIntent;
    use crate::translation::RewriteIntent;
    let mut store = CaptureSessionStore::default();
    let token = capture(&mut store, "close", 101, 2001);
    let bound = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store.begin_rewrite_bound(&token, bound).unwrap();
    store.cancel_active_with_turn();
    assert!(!store.is_captured(&token));
    assert!(!store.has_active());
    assert_eq!(store.captured_source(&token), Err(SessionError::InvalidState));
}

// A busy Provider (an earlier request still finishing, a status probe or a
// self-test) used to leave the capture in Rewriting: Deep, and Copy of the
// Instant draft, then failed with invalid_session_state until a new capture.
#[test]
fn busy_provider_reservation_returns_the_capture_to_captured() {
    use crate::capture_session::BoundRewriteIntent;
    use crate::provider::{ProviderKind, ProviderManager};
    use crate::translation::RewriteIntent;
    let mut store = CaptureSessionStore::default();
    let mut providers = ProviderManager::default();
    let earlier = capture(&mut store, "earlier", 101, 2001);
    let bound = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store.begin_rewrite_bound(&earlier, bound.clone()).unwrap();
    let busy = crate::reserve_rewrite(&mut store, &mut providers, ProviderKind::Codex, &earlier, &bound).unwrap();

    let token = capture(&mut store, "busy", 102, 2002);
    store.begin_rewrite_bound(&token, bound.clone()).unwrap();
    let Err(error) = crate::reserve_rewrite(&mut store, &mut providers, ProviderKind::Codex, &token, &bound) else {
        panic!("a busy Provider must refuse the reservation");
    };
    assert_eq!(error, "Another Provider request is still running. Try again in a moment.");
    assert!(store.is_captured(&token), "nothing started, so the capture is Captured again");
    assert_eq!(store.captured_source(&token).unwrap(), "synthetic-source-busy");

    assert!(providers.finish(&busy));
    store.begin_rewrite_bound(&token, bound.clone()).unwrap();
    assert!(crate::reserve_rewrite(&mut store, &mut providers, ProviderKind::Codex, &token, &bound).is_ok(),
        "once the earlier request has finished, Deep can run again");
}
