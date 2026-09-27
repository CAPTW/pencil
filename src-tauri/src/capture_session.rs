use crate::{active_turn::ActiveTurn, translation::RewriteIntent};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SessionToken {
    pub(crate) session_id: String,
    pub(crate) generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TerminologyIntent {
    pub(crate) enabled: bool,
    pub(crate) use_approved_terminology: bool,
    pub(crate) suggest_terminology: bool,
    pub(crate) active_profile_id: String,
    pub(crate) store_revision: u64,
    pub(crate) matched_entry_ids: Vec<String>,
}

impl TerminologyIntent {
    #[cfg(test)]
    fn disabled() -> Self {
        Self {
            enabled: false,
            use_approved_terminology: false,
            suggest_terminology: false,
            active_profile_id: crate::terminology::GENERAL_PROFILE_ID.to_string(),
            store_revision: 0,
            matched_entry_ids: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundRewriteIntent {
    rewrite: RewriteIntent,
    terminology: TerminologyIntent,
}

impl BoundRewriteIntent {
    pub(crate) fn new(rewrite: RewriteIntent, terminology: TerminologyIntent) -> Self {
        Self {
            rewrite,
            terminology,
        }
    }

    #[cfg(test)]
    pub(crate) fn without_terminology(rewrite: RewriteIntent) -> Self {
        Self::new(rewrite, TerminologyIntent::disabled())
    }

    pub(crate) fn rewrite(&self) -> RewriteIntent {
        self.rewrite
    }

    pub(crate) fn terminology(&self) -> &TerminologyIntent {
        &self.terminology
    }
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

/// Exact state of a standard Edit control observed by the qualified native
/// reader. Apply uses it only to recognise a native capture, which is always
/// Copy-only. Only a hash of the full field text is retained, never the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeEditBinding {
    pub(crate) edit: isize,
    pub(crate) pid: u32,
    /// UTF-16 offsets of the captured selection.
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text_units: usize,
    /// SHA-256 of the full field text as UTF-16LE.
    pub(crate) text_sha256: String,
    pub(crate) read_only: bool,
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

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InstantDraftProof {
    session_id: String,
    generation: u64,
    source: String,
    candidate: String,
    draft_revision: u64,
    user_edited: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ApplyContext {
    pub(crate) token: SessionToken,
    pub(crate) target: WindowTarget,
    pub(crate) native_edit: Option<NativeEditBinding>,
}

#[derive(Clone, Debug)]
struct CaptureSession {
    token: SessionToken,
    selected_text: String,
    target: WindowTarget,
    lifecycle: CaptureLifecycle,
    native_edit: Option<NativeEditBinding>,
    rewrite_intent: Option<BoundRewriteIntent>,
    active_turn: Option<ActiveTurn>,
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
    ) -> Result<SessionToken, SessionError> {
        self.capture_bound(session_id, selected_text, target, None)
    }

    /// Captures a selection read by the qualified native Edit reader. Only such
    /// sessions carry the editor binding; Apply for them is always Copy-only.
    pub(crate) fn capture_native(
        &mut self,
        session_id: String,
        selected_text: String,
        target: WindowTarget,
        binding: NativeEditBinding,
    ) -> Result<SessionToken, SessionError> {
        if binding.pid != target.pid {
            return Err(SessionError::InvalidState);
        }
        self.capture_bound(session_id, selected_text, target, Some(binding))
    }

    fn capture_bound(
        &mut self,
        session_id: String,
        selected_text: String,
        target: WindowTarget,
        native_edit: Option<NativeEditBinding>,
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
            native_edit,
            rewrite_intent: None,
            active_turn: None,
            _created_at: Instant::now(),
        });
        Ok(token)
    }

    #[cfg(test)]
    pub(crate) fn begin_rewrite(&mut self, token: &SessionToken) -> Result<String, SessionError> {
        self.begin_rewrite_for(token, RewriteIntent::grammar())
    }

    #[cfg(test)]
    pub(crate) fn begin_rewrite_for(
        &mut self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<String, SessionError> {
        self.begin_rewrite_bound(token, BoundRewriteIntent::without_terminology(intent))
    }

    pub(crate) fn captured_source(&self, token: &SessionToken) -> Result<String, SessionError> {
        let current = self.current(token)?;
        Self::require_state(current, CaptureLifecycle::Captured)?;
        Ok(current.selected_text.clone())
    }

    pub(crate) fn validate_instant_draft(
        &self, token: &SessionToken, candidate: &str, replacement: &str,
        proof: Option<&InstantDraftProof>,
    ) -> Result<(), SessionError> {
        let source = self.captured_source(token)?;
        let proof = proof.ok_or(SessionError::InvalidState)?;
        if proof.session_id != token.session_id || proof.generation != token.generation ||
            proof.source != source || proof.candidate != candidate || replacement.is_empty() ||
            proof.draft_revision > 9_007_199_254_740_991 ||
            (proof.user_edited && proof.draft_revision == 0) ||
            (!proof.user_edited && candidate != replacement) {
            return Err(SessionError::InvalidState);
        }
        Ok(())
    }

    pub(crate) fn begin_rewrite_bound(
        &mut self,
        token: &SessionToken,
        intent: BoundRewriteIntent,
    ) -> Result<String, SessionError> {
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Captured)?;
        current.rewrite_intent = Some(intent);
        current.active_turn = None;
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

    #[cfg(test)]
    pub(crate) fn finish_rewrite_success_for(
        &mut self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_intent(token, intent)?;
        self.transition(token, CaptureLifecycle::Rewriting, CaptureLifecycle::Ready)
    }

    pub(crate) fn finish_rewrite_success_bound(
        &mut self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_bound_intent(token, intent)?;
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Rewriting)?;
        current.active_turn = None;
        current.lifecycle = CaptureLifecycle::Ready;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn finish_rewrite_failure(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        let intent = self.current_intent(token)?;
        self.finish_rewrite_failure_for(token, intent)
    }

    #[cfg(test)]
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

    pub(crate) fn finish_rewrite_failure_bound(
        &mut self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_bound_intent(token, intent)?;
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Rewriting)?;
        current.active_turn = None;
        current.lifecycle = CaptureLifecycle::Captured;
        Ok(())
    }

    pub(crate) fn bind_active_turn(
        &mut self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
        active_turn: ActiveTurn,
    ) -> Result<(), SessionError> {
        self.require_bound_intent(token, intent)?;
        let current = self.current_mut(token)?;
        Self::require_state(current, CaptureLifecycle::Rewriting)?;
        if current.active_turn.is_some() {
            return Err(SessionError::InvalidState);
        }
        current.active_turn = Some(active_turn);
        Ok(())
    }

    pub(crate) fn validate_rewriting_bound_intent(
        &self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_bound_intent(token, intent)?;
        Self::require_state(self.current(token)?, CaptureLifecycle::Rewriting)
    }

    pub(crate) fn invalidate_intent(&mut self, next: RewriteIntent) -> Result<(), SessionError> {
        self.invalidate_intent_with_active_turn(next).map(|_| ())
    }

    pub(crate) fn invalidate_intent_with_active_turn(
        &mut self,
        next: RewriteIntent,
    ) -> Result<Option<ActiveTurn>, SessionError> {
        let Some(current) = self.current.as_mut() else {
            return Ok(None);
        };
        if current
            .rewrite_intent
            .as_ref()
            .is_some_and(|intent| intent.rewrite == next)
        {
            return Ok(None);
        }
        match current.lifecycle {
            CaptureLifecycle::Captured | CaptureLifecycle::Rewriting | CaptureLifecycle::Ready => {
                current.lifecycle = CaptureLifecycle::Captured;
                current.rewrite_intent = None;
                Ok(current.active_turn.take())
            }
            CaptureLifecycle::Completed | CaptureLifecycle::Cancelled => Ok(None),
            CaptureLifecycle::Applying => Err(SessionError::InvalidState),
        }
    }

    pub(crate) fn invalidate_terminology_intent(&mut self) {
        let _ = self.invalidate_terminology_intent_with_active_turn();
    }

    pub(crate) fn invalidate_terminology_intent_with_active_turn(&mut self) -> Option<ActiveTurn> {
        let Some(current) = self.current.as_mut() else {
            return None;
        };
        if matches!(
            current.lifecycle,
            CaptureLifecycle::Captured | CaptureLifecycle::Rewriting | CaptureLifecycle::Ready
        ) {
            current.lifecycle = CaptureLifecycle::Captured;
            current.rewrite_intent = None;
            return current.active_turn.take();
        }
        None
    }

    pub(crate) fn validate_ready_intent(
        &self,
        token: &SessionToken,
        intent: RewriteIntent,
    ) -> Result<(), SessionError> {
        let current = self.current(token)?;
        if current
            .rewrite_intent
            .as_ref()
            .is_none_or(|bound| bound.rewrite != intent)
        {
            return Err(SessionError::StaleIntent);
        }
        Self::require_state(current, CaptureLifecycle::Ready)
    }

    pub(crate) fn validate_ready_bound_intent(
        &self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<(), SessionError> {
        self.require_bound_intent(token, intent)?;
        Self::require_state(self.current(token)?, CaptureLifecycle::Ready)
    }

    #[cfg(test)]
    pub(crate) fn ready_matched_entry_ids(
        &self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<Vec<String>, SessionError> {
        self.validate_ready_bound_intent(token, intent)?;
        Ok(intent.terminology.matched_entry_ids.clone())
    }

    pub(crate) fn ready_bound_intent_for(
        &self,
        token: &SessionToken,
        rewrite: RewriteIntent,
    ) -> Result<BoundRewriteIntent, SessionError> {
        let current = self.current(token)?;
        Self::require_state(current, CaptureLifecycle::Ready)?;
        let bound = current
            .rewrite_intent
            .as_ref()
            .ok_or(SessionError::StaleIntent)?;
        if bound.rewrite != rewrite {
            return Err(SessionError::StaleIntent);
        }
        Ok(bound.clone())
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
            native_edit: current.native_edit.clone(),
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

    /// Nothing was delivered (for example the clipboard write failed): the
    /// session returns to Ready so the user can try again.
    pub(crate) fn finish_apply_failure(
        &mut self,
        token: &SessionToken,
    ) -> Result<(), SessionError> {
        self.transition(token, CaptureLifecycle::Applying, CaptureLifecycle::Ready)
    }

    pub(crate) fn cancel(&mut self, token: &SessionToken) -> Result<(), SessionError> {
        self.cancel_with_active_turn(token).map(|_| ())
    }

    pub(crate) fn cancel_with_active_turn(
        &mut self,
        token: &SessionToken,
    ) -> Result<Option<ActiveTurn>, SessionError> {
        let current = self.current_mut(token)?;
        match current.lifecycle {
            CaptureLifecycle::Completed | CaptureLifecycle::Cancelled => {
                Err(SessionError::InvalidState)
            }
            _ => {
                current.lifecycle = CaptureLifecycle::Cancelled;
                Ok(current.active_turn.take())
            }
        }
    }

    pub(crate) fn cancel_active_with_turn(&mut self) -> Option<ActiveTurn> {
        if let Some(current) = self.current.as_mut() {
            if !matches!(
                current.lifecycle,
                CaptureLifecycle::Completed | CaptureLifecycle::Cancelled
            ) {
                current.lifecycle = CaptureLifecycle::Cancelled;
                return current.active_turn.take();
            }
        }
        None
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

    #[cfg(test)]
    fn current_intent(&self, token: &SessionToken) -> Result<RewriteIntent, SessionError> {
        self.current(token)?
            .rewrite_intent
            .as_ref()
            .map(BoundRewriteIntent::rewrite)
            .ok_or(SessionError::StaleIntent)
    }

    #[cfg(test)]
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

    fn require_bound_intent(
        &self,
        token: &SessionToken,
        intent: &BoundRewriteIntent,
    ) -> Result<(), SessionError> {
        if self.current(token)?.rewrite_intent.as_ref() == Some(intent) {
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

#[cfg(test)]
mod mission_draft_tests {
    use super::*;
    #[test]
    fn edited_instant_keeps_capture_source_candidate_and_revision_binding() {
        let mut store = CaptureSessionStore::default();
        let token = store.capture("mission".into(), "source".into(), WindowTarget::new(1, 2)).unwrap();
        let mut proof = InstantDraftProof { session_id: token.session_id.clone(), generation: token.generation,
            source: "source".into(), candidate: "candidate".into(), draft_revision: 1, user_edited: true };
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_ok());
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", None).is_err());
        proof.source = "other source".into();
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_err());
        proof.source = "source".into(); proof.generation += 1;
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_err());
        proof.generation = token.generation; proof.draft_revision = 0;
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_err());
        proof.draft_revision = 1; proof.user_edited = false;
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_err());
        proof.user_edited = true;
        assert!(store.validate_instant_draft(&token, "different candidate", "my edit", Some(&proof)).is_err());
        store.cancel(&token).unwrap();
        assert!(store.validate_instant_draft(&token, "candidate", "my edit", Some(&proof)).is_err());
    }
}
