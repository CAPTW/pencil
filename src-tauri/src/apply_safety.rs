use crate::capture_session::{ApplyContext, CaptureSessionStore, SessionError, SessionToken};
use serde::Serialize;

const ACTIVATION_ATTEMPTS: usize = 3;
const COMPLETE_PASTE_INPUT_COUNT: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WaitStage {
    AfterActivation,
    BeforePaste,
    AfterPaste,
}

pub(crate) trait ApplyPlatform {
    fn is_window(&mut self, hwnd: isize) -> bool;
    fn window_pid(&mut self, hwnd: isize) -> Option<u32>;
    fn hide_widget(&mut self) -> Result<(), ()>;
    fn show_widget(&mut self);
    fn request_foreground(&mut self, hwnd: isize);
    fn foreground_window(&mut self) -> isize;
    fn clipboard_sequence(&mut self) -> u32;
    fn read_clipboard_text(&mut self) -> Option<String>;
    fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()>;
    fn send_paste(&mut self) -> u32;
    fn wait(&mut self, stage: WaitStage);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyFallbackReason {
    TargetMissing,
    TargetProcessChanged,
    TargetNotForeground,
    TargetChangedBeforePaste,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApplyFailureReason {
    EmptyReplacement,
    InvalidSessionState,
    WidgetHideFailed,
    ClipboardWriteFailed,
    ClipboardOwnershipLost,
    InputInjectionFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum ApplyOutcome {
    Applied,
    CopiedFallback { reason: ApplyFallbackReason },
    RejectedStale,
    Failed { reason: ApplyFailureReason },
}

pub(crate) fn apply_current_session<P: ApplyPlatform>(
    store: &mut CaptureSessionStore,
    token: &SessionToken,
    replacement: &str,
    restore_clipboard: bool,
    platform: &mut P,
) -> ApplyOutcome {
    if replacement.is_empty() {
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::EmptyReplacement,
        };
    }

    let context = match store.begin_apply(token) {
        Ok(context) => context,
        Err(SessionError::StaleSession) => return ApplyOutcome::RejectedStale,
        Err(SessionError::InvalidState | SessionError::GenerationExhausted) => {
            return ApplyOutcome::Failed {
                reason: ApplyFailureReason::InvalidSessionState,
            };
        }
    };

    let outcome = execute_apply(&context, replacement, restore_clipboard, platform);
    let transition = match outcome {
        ApplyOutcome::Applied | ApplyOutcome::CopiedFallback { .. } => {
            store.finish_apply_success(&context.token)
        }
        ApplyOutcome::Failed {
            reason: ApplyFailureReason::InputInjectionFailed,
        } => store.finish_apply_uncertain_input(&context.token),
        ApplyOutcome::Failed { .. } => store.finish_apply_before_paste_failure(&context.token),
        ApplyOutcome::RejectedStale => Err(SessionError::InvalidState),
    };

    if transition.is_err() {
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::InvalidSessionState,
        };
    }

    outcome
}

fn execute_apply<P: ApplyPlatform>(
    context: &ApplyContext,
    replacement: &str,
    restore_clipboard: bool,
    platform: &mut P,
) -> ApplyOutcome {
    if !platform.is_window(context.target.hwnd) {
        return copy_fallback(
            platform,
            replacement,
            ApplyFallbackReason::TargetMissing,
            false,
        );
    }
    if platform.window_pid(context.target.hwnd) != Some(context.target.pid) {
        return copy_fallback(
            platform,
            replacement,
            ApplyFallbackReason::TargetProcessChanged,
            false,
        );
    }

    if platform.hide_widget().is_err() {
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::WidgetHideFailed,
        };
    }

    let mut target_is_foreground = false;
    for _ in 0..ACTIVATION_ATTEMPTS {
        platform.request_foreground(context.target.hwnd);
        platform.wait(WaitStage::AfterActivation);
        if platform.foreground_window() == context.target.hwnd {
            target_is_foreground = true;
            break;
        }
    }
    if !target_is_foreground {
        return copy_fallback(
            platform,
            replacement,
            ApplyFallbackReason::TargetNotForeground,
            true,
        );
    }

    if !platform.is_window(context.target.hwnd) {
        return copy_fallback(
            platform,
            replacement,
            ApplyFallbackReason::TargetMissing,
            true,
        );
    }
    if platform.window_pid(context.target.hwnd) != Some(context.target.pid) {
        return copy_fallback(
            platform,
            replacement,
            ApplyFallbackReason::TargetProcessChanged,
            true,
        );
    }

    let restore_candidate = if restore_clipboard {
        if context.capture_clipboard_sequence == Some(platform.clipboard_sequence()) {
            context.previous_clipboard_text.clone()
        } else {
            platform.read_clipboard_text()
        }
    } else {
        None
    };

    if platform.write_clipboard_text(replacement).is_err() {
        platform.show_widget();
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        };
    }
    let replacement_sequence = platform.clipboard_sequence();

    platform.wait(WaitStage::BeforePaste);
    if platform.clipboard_sequence() != replacement_sequence {
        platform.show_widget();
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardOwnershipLost,
        };
    }
    if !platform.is_window(context.target.hwnd)
        || platform.window_pid(context.target.hwnd) != Some(context.target.pid)
        || platform.foreground_window() != context.target.hwnd
    {
        platform.show_widget();
        return ApplyOutcome::CopiedFallback {
            reason: ApplyFallbackReason::TargetChangedBeforePaste,
        };
    }

    let sent = platform.send_paste();
    platform.wait(WaitStage::AfterPaste);

    if restore_clipboard && platform.clipboard_sequence() == replacement_sequence {
        if let Some(candidate) = restore_candidate.as_deref() {
            let _ = platform.write_clipboard_text(candidate);
        }
    }

    if sent != COMPLETE_PASTE_INPUT_COUNT {
        platform.show_widget();
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::InputInjectionFailed,
        };
    }

    ApplyOutcome::Applied
}

fn copy_fallback<P: ApplyPlatform>(
    platform: &mut P,
    replacement: &str,
    reason: ApplyFallbackReason,
    widget_was_hidden: bool,
) -> ApplyOutcome {
    if platform.write_clipboard_text(replacement).is_err() {
        if widget_was_hidden {
            platform.show_widget();
        }
        return ApplyOutcome::Failed {
            reason: ApplyFailureReason::ClipboardWriteFailed,
        };
    }
    if widget_was_hidden {
        platform.show_widget();
    }
    ApplyOutcome::CopiedFallback { reason }
}
