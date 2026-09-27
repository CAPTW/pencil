use crate::capture_session::{ApplyContext, CaptureSessionStore, SessionError, SessionToken};
use serde::Serialize;

/// Delivers the approved result of the current capture (the desktop Copy
/// button). Grammar never changes text inside another application's editor: a
/// standard Edit belongs to that application, which can change its own text
/// between any two messages Grammar sends, and no message sequence verifies a
/// range and replaces it atomically (review findings R1/R2). The result is
/// copied to the clipboard and the user pastes it. The platform can only check
/// the captured window and write the clipboard; it has no way to send input,
/// change focus or message the captured editor.
pub(crate) trait ApplyPlatform {
    fn is_window(&mut self, hwnd: isize) -> bool;
    fn window_pid(&mut self, hwnd: isize) -> Option<u32>;
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyFallbackReason {
    /// A capture without the native reader's binding (none in production).
    TargetSelectionUnverified,
    TargetMissing,
    TargetProcessChanged,
    /// Grammar never changes text inside another application's native editor.
    TargetMutationDisabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyFailureReason {
    EmptyReplacement,
    InvalidSessionState,
    /// The capture is still usable, but the Instant draft no longer matches
    /// it (the mode or dictionary changed after the draft was made).
    DraftOutdated,
    ClipboardWriteFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum ApplyOutcome {
    CopiedFallback { reason: ApplyFallbackReason },
    RejectedStale,
    Failed { reason: ApplyFailureReason },
}

pub(crate) fn apply_current_session<P: ApplyPlatform>(
    store: &mut CaptureSessionStore,
    token: &SessionToken,
    replacement: &str,
    platform: &mut P,
) -> ApplyOutcome {
    if replacement.is_empty() {
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::EmptyReplacement,
        };
    }

    let context = match store.begin_apply(token) {
        Ok(context) => context,
        Err(SessionError::StaleSession | SessionError::StaleIntent) => {
            return ApplyOutcome::RejectedStale
        }
        Err(SessionError::InvalidState | SessionError::GenerationExhausted) => {
            return ApplyOutcome::Failed {
                reason: ApplyFailureReason::InvalidSessionState,
            };
        }
    };

    let outcome = execute_apply(&context, replacement, platform);
    let transition = match outcome {
        ApplyOutcome::CopiedFallback { .. } => store.finish_apply_success(&context.token),
        ApplyOutcome::Failed { .. } => store.finish_apply_failure(&context.token),
        ApplyOutcome::RejectedStale => Err(SessionError::InvalidState),
    };

    if transition.is_err() {
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::InvalidSessionState,
        };
    }

    outcome
}

/// Copy-only: the window checks only choose the reason reported with the copy.
fn execute_apply<P: ApplyPlatform>(
    context: &ApplyContext,
    replacement: &str,
    platform: &mut P,
) -> ApplyOutcome {
    let reason = if !platform.is_window(context.target.hwnd) {
        ApplyFallbackReason::TargetMissing
    } else if platform.window_pid(context.target.hwnd) != Some(context.target.pid) {
        ApplyFallbackReason::TargetProcessChanged
    } else if context.native_edit.is_some() {
        ApplyFallbackReason::TargetMutationDisabled
    } else {
        ApplyFallbackReason::TargetSelectionUnverified
    };
    match platform.write_clipboard_text(replacement) {
        Ok(()) => ApplyOutcome::CopiedFallback { reason },
        Err(()) => ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        },
    }
}
