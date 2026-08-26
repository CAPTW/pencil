use super::{
    Candidate, CandidateOrigin, EngineError, EngineErrorCode, ReconciliationState, SourceIdentity,
};

#[derive(Clone, Debug)]
pub struct ReconciliationMachine {
    identity: SourceIdentity,
    state: ReconciliationState,
    instant: Option<Candidate>,
    deep: Option<Candidate>,
    active_origin: Option<CandidateOrigin>,
    draft: String,
    draft_revision: u64,
    dirty: bool,
    recoverable_user_draft: Option<String>,
    deep_pending: bool,
}

impl ReconciliationMachine {
    pub fn new(identity: SourceIdentity, source: impl Into<String>) -> Self {
        Self {
            identity,
            state: ReconciliationState::Captured,
            instant: None,
            deep: None,
            active_origin: None,
            draft: source.into(),
            draft_revision: 0,
            dirty: false,
            recoverable_user_draft: None,
            deep_pending: false,
        }
    }

    pub const fn state(&self) -> ReconciliationState {
        self.state
    }

    pub fn draft(&self) -> &str {
        &self.draft
    }

    pub const fn draft_revision(&self) -> u64 {
        self.draft_revision
    }

    pub fn recoverable_user_draft(&self) -> Option<&str> {
        self.recoverable_user_draft.as_deref()
    }

    pub fn alternate(&self, origin: CandidateOrigin) -> Option<&Candidate> {
        match origin {
            CandidateOrigin::Instant => self.instant.as_ref(),
            CandidateOrigin::Deep => self.deep.as_ref(),
        }
    }

    pub fn begin_instant(&mut self, identity: &SourceIdentity) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        if self.state != ReconciliationState::Captured {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        self.state = ReconciliationState::InstantAnalyzing;
        Ok(())
    }

    pub fn complete_instant(
        &mut self,
        identity: &SourceIdentity,
        candidate: Candidate,
    ) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        self.require_candidate(&candidate, CandidateOrigin::Instant)?;
        if self.state != ReconciliationState::InstantAnalyzing {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        self.draft = candidate.text.clone();
        self.instant = Some(candidate);
        self.active_origin = Some(CandidateOrigin::Instant);
        self.draft_revision = self.draft_revision.saturating_add(1);
        self.dirty = false;
        self.state = ReconciliationState::InstantReady;
        Ok(())
    }

    pub fn begin_deep(&mut self, identity: &SourceIdentity) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        match self.state {
            ReconciliationState::Captured
            | ReconciliationState::InstantReady
            | ReconciliationState::DeepReady => {
                self.state = ReconciliationState::DeepAnalyzing;
                self.deep_pending = true;
                Ok(())
            }
            ReconciliationState::UserEdited => {
                self.deep_pending = true;
                Ok(())
            }
            _ => Err(EngineError::new(EngineErrorCode::IllegalTransition)),
        }
    }

    pub fn complete_deep(
        &mut self,
        identity: &SourceIdentity,
        candidate: Candidate,
    ) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        self.require_candidate(&candidate, CandidateOrigin::Deep)?;
        if !self.deep_pending
            || !matches!(
                self.state,
                ReconciliationState::DeepAnalyzing | ReconciliationState::UserEdited
            )
        {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        self.deep = Some(candidate);
        self.deep_pending = false;
        if self.state != ReconciliationState::UserEdited {
            self.state = ReconciliationState::DeepReady;
        }
        Ok(())
    }

    pub fn edit_draft(&mut self, draft: impl Into<String>) -> Result<(), EngineError> {
        if !matches!(
            self.state,
            ReconciliationState::InstantReady
                | ReconciliationState::DeepReady
                | ReconciliationState::DeepAnalyzing
                | ReconciliationState::UserEdited
        ) {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        self.draft = draft.into();
        self.draft_revision = self.draft_revision.saturating_add(1);
        self.dirty = true;
        self.state = ReconciliationState::UserEdited;
        Ok(())
    }

    pub fn switch_candidate(
        &mut self,
        identity: &SourceIdentity,
        origin: CandidateOrigin,
    ) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        if !matches!(
            self.state,
            ReconciliationState::InstantReady
                | ReconciliationState::DeepReady
                | ReconciliationState::UserEdited
        ) {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        let selected = match origin {
            CandidateOrigin::Instant => self.instant.as_ref(),
            CandidateOrigin::Deep => self.deep.as_ref(),
        }
        .ok_or_else(|| EngineError::new(EngineErrorCode::IllegalTransition))?;
        if self.dirty && self.recoverable_user_draft.is_none() {
            self.recoverable_user_draft = Some(self.draft.clone());
        }
        self.draft = selected.text.clone();
        self.active_origin = Some(origin);
        self.draft_revision = self.draft_revision.saturating_add(1);
        self.dirty = false;
        self.state = match origin {
            CandidateOrigin::Instant => ReconciliationState::InstantReady,
            CandidateOrigin::Deep => ReconciliationState::DeepReady,
        };
        Ok(())
    }

    pub fn restore_user_draft(&mut self) -> Result<(), EngineError> {
        let Some(draft) = self.recoverable_user_draft.take() else {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        };
        self.draft = draft;
        self.draft_revision = self.draft_revision.saturating_add(1);
        self.dirty = true;
        self.state = ReconciliationState::UserEdited;
        Ok(())
    }

    pub fn apply(&mut self, identity: &SourceIdentity) -> Result<(), EngineError> {
        self.require_identity(identity)?;
        if !matches!(
            self.state,
            ReconciliationState::InstantReady
                | ReconciliationState::DeepReady
                | ReconciliationState::UserEdited
        ) {
            return Err(EngineError::new(EngineErrorCode::IllegalTransition));
        }
        self.clear_candidates();
        self.state = ReconciliationState::Applied;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.clear_candidates();
        self.state = ReconciliationState::Cancelled;
    }

    pub fn mark_stale(&mut self) {
        self.clear_candidates();
        self.state = ReconciliationState::Stale;
    }

    fn require_identity(&mut self, identity: &SourceIdentity) -> Result<(), EngineError> {
        if &self.identity != identity {
            self.mark_stale();
            return Err(EngineError::new(EngineErrorCode::StaleIdentity));
        }
        Ok(())
    }

    fn require_candidate(
        &mut self,
        candidate: &Candidate,
        origin: CandidateOrigin,
    ) -> Result<(), EngineError> {
        if candidate.source_identity != self.identity {
            self.mark_stale();
            return Err(EngineError::new(EngineErrorCode::StaleIdentity));
        }
        if candidate.origin != origin {
            return Err(EngineError::new(EngineErrorCode::InvalidCandidateOrigin));
        }
        Ok(())
    }

    fn clear_candidates(&mut self) {
        self.instant = None;
        self.deep = None;
        self.active_origin = None;
        self.deep_pending = false;
    }
}
