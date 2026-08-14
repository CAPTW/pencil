use crate::translation::RewriteIntent;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionToken {
    pub(crate) session_id: String,
    pub(crate) generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowTarget {
    pub(crate) hwnd: isize,
    pub(crate) pid: u32,
}

impl WindowTarget {
    pub(crate) fn new(hwnd: isize, pid: u32) -> Self {
        Self { hwnd, pid }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureLifecycle {
    Captured,
    Rewriting,
    Ready,
    Applying,
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SessionError {
    StaleSession,
    StaleIntent,
    InvalidState,
    GenerationExhausted,
}

impl SessionError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::StaleSession => "stale_session",
            Self::StaleIntent => "stale_rewrite_intent",
            Self::InvalidState => "invalid_session_state",
            Self::GenerationExhausted => "session_generation_exhausted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApplyContext {
    pub(crate) token: SessionToken,
    pub(crate) target: WindowTarget,
    pub(crate) previous_clipboard_text: Option<String>,
    pub(crate) capture_clipboard_sequence: Option<u32>,
}

#[derive(Clone, Debug)]
struct CaptureSession {
    token: SessionToken,
    selected_text: String,
    target: WindowTarget,
    lifecycle: CaptureLifecycle,
    previous_clipboard_text: Option<String>,
    capture_clipboard_sequence: Option<u32>,
    rewrite_intent: Option<RewriteIntent>,
    _created_at: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct CaptureSessionStore {
    current: Option<CaptureSession>,
    generation: u64,
}

impl CaptureSessionStore {
    pub(crate) fn capture(
        &mut self,
        session_id: String,
        selected_text: String,
        target: WindowTarget,
        previous_clipboard_text: Option<String>,
        capture_clipboard_sequence: Option<u32>,
    ) -> Result<SessionToken, SessionError> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(SessionError::GenerationExhausted)?;

        if let Some(previous) = self.current.as_mut() {
            previous.lifecycle = CaptureLifecycle::Cancelled;
        }

        self.generation = generation;
        let token = SessionToken {
            session_id,
            generation,
        };
        self.current = Some(CaptureSession {
            token: token.clone(),
            selected_text,
            target,
            lifecycle: CaptureLifecycle::Captured,
            previous_clipboard_text,
            capture_clipboard_sequence,
            rewrite_intent: None,
            _created_at: Instant::now(),
        });
        Ok(token)
    }

    #[cfg(test)]
    pub(crate) fn begin_rewrite(&mut self, token: &SessionToken) -> Result<String, SessionError> {
        self.begin_rewrite_for(token, RewriteIntent::grammar())
    }

    pub(crate) fn begin_rewrite_for(
        &mut self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<String, SessionError> {
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Captured)?;
        current.rewrite_intent = Some(intent);
        current.lifecycle = CaptureLifecycle::Rewriting;
        Ok(current.selected_text.clone())
    }

    #[cfg(test)]
    pub(crate) fn finish_rewrite_success(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        let intent = self.current_intent(token)?;
        self.finish_rewrite_success_for(token, intent)
    }

    pub(crate) fn finish_rewrite_success_for(
        &mut self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_intent(token, intent)?;
        self.transition(token, CaptureLifecycle::Rewriting, CaptureLifecycle::Ready)
    }

    #[cfg(test)]
    pub(crate) fn finish_rewrite_failure(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        let intent = self.current_intent(token)?;
        self.finish_rewrite_failure_for(token, intent)
    }

    pub(crate) fn finish_rewrite_failure_for(
        &mut self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_intent(token, intent)?;
        self.transition(
            token,
            CaptureLifecycle::Rewriting,
            CaptureLifecycle::Captured,
        )
    }

    pub(crate) fn invalidate_intent(&mut self, next: RewriteIntent) -> Result<(), SessionError> {
        let Some(current) = self.current.as_mut() else {
            return Ok(());
        };
        if current.rewrite_intent == Some(next) {
            return Ok(());
        }
        match current.lifecycle {
            CaptureLifecycle::Captured | CaptureLifecycle::Rewriting | CaptureLifecycle::Ready => {
                current.lifecycle = CaptureLifecycle::Captured;
                current.rewrite_intent = None;
                Ok(())
            }
            CaptureLifecycle::Completed | CaptureLifecycle::Cancelled => Ok(()),
            CaptureLifecycle::Applying => Err(SessionError::InvalidState),
        }
    }

    pub(crate) fn validate_ready_intent(
        &self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        let current = self.current(token)?;
        if current.rewrite_intent != Some(intent) {
            return Err(SessionError::StaleIntent);
        }
        Self::require_state(current, CaptureLifecycle::Ready)
    }

    pub(crate) fn ready_source_for(
        &self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<String, SessionError> {
        self.validate_ready_intent(token, intent)?;
        Ok(self.current(token)?.selected_text.clone())
    }

    pub(crate) fn begin_apply(
        &mut self,
        token: &SessionToken,
    ) -> Result<ApplyContext, SessionError> {
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Ready)?;
        current.lifecycle = CaptureLifecycle::Applying;
        Ok(ApplyContext {
            token: current.token.clone(),
            target: current.target,
            previous_clipboard_text: current.previous_clipboard_text.clone(),
            capture_clipboard_sequence: current.capture_clipboard_sequence,
        })
    }

    pub(crate) fn finish_apply_success(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        self.transition(
            token,
            CaptureLifecycle::Applying,
            CaptureLifecycle::Completed,
        )
    }

    pub(crate) fn finish_apply_before_paste_failure(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        self.transition(token, CaptureLifecycle::Applying, CaptureLifecycle::Ready)
    }

    pub(crate) fn finish_apply_uncertain_input(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        self.transition(
            token,
            CaptureLifecycle::Applying,
            CaptureLifecycle::Cancelled,
        )
    }

    pub(crate) fn cancel(&mut self, token: &SessionToken) -> Result<(), SessionError> {
        let current = self.current_mut(token)?;
        match current.lifecycle {
            CaptureLifecycle::Completed | CaptureLifecycle::Cancelled => {
                Err(SessionError::InvalidState)
            }
            _ => {
                current.lifecycle = CaptureLifecycle::Cancelled;
                Ok(())
            }
        }
    }

    pub(crate) fn cancel_active(&mut self) {
        if let Some(current) = self.current.as_mut() {
            if !matches!(
                current.lifecycle,
                CaptureLifecycle::Completed | CaptureLifecycle::Cancelled
            ) {
                current.lifecycle = CaptureLifecycle::Cancelled;
            }
        }
    }

    pub(crate) fn has_active(&self) -> bool {
        self.current.as_ref().is_some_and(|current| {
            !matches!(
                current.lifecycle,
                CaptureLifecycle::Completed | CaptureLifecycle::Cancelled
            )
        })
    }

    fn transition(
        &mut self,
        token: &SessionToken,
        expected: CaptureLifecycle,
        next: CaptureLifecycle,
    ) -> Result<(), SessionError> {
        let current = self.current_mut(token)?;
        Self::require_state(current, expected)?;
        current.lifecycle = next;
        Ok(())
    }

    fn current_mut(&mut self, token: &SessionToken) -> Result<&mut CaptureSession, SessionError> {
        let current = self.current.as_mut().ok_or(SessionError::StaleSession)?;
        if current.token != *token {
            return Err(SessionError::StaleSession);
        }
        Ok(current)
    }

    fn current(&self, token: &SessionToken) -> Result<&CaptureSession, SessionError> {
        let current = self.current.as_ref().ok_or(SessionError::StaleSession)?;
        if current.token != *token {
            return Err(SessionError::StaleSession);
        }
        Ok(current)
    }

    fn current_intent(&self, token: &SessionToken) -> Result<RewriteIntent, SessionError> {
        self.current(token)?
            .rewrite_intent
            .ok_or(SessionError::StaleIntent)
    }

    fn require_intent(
        &self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        if self.current_intent(token)? == intent {
            Ok(())
        } else {
            Err(SessionError::StaleIntent)
        }
    }

    fn require_state(
        current: &CaptureSession,
        expected: CaptureLifecycle,
    ) -> Result<(), SessionError> {
        if current.lifecycle == expected {
            Ok(())
        } else {
            Err(SessionError::InvalidState)
        }
    }
}
