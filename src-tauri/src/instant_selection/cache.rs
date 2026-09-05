use super::{Candidate, CandidateOrigin, EngineError, EngineErrorCode, SourceIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidationReason {
    Recapture,
    Cancel,
    Dismiss,
    SuccessfulApply,
    AppShutdown,
    ModeChange,
    LanguageChange,
    ProfileChange,
    TerminologyRevisionChange,
    SourceIdentityChange,
    AnalyzerIdentityChange,
}

impl InvalidationReason {
    pub const ALL: [Self; 11] = [
        Self::Recapture,
        Self::Cancel,
        Self::Dismiss,
        Self::SuccessfulApply,
        Self::AppShutdown,
        Self::ModeChange,
        Self::LanguageChange,
        Self::ProfileChange,
        Self::TerminologyRevisionChange,
        Self::SourceIdentityChange,
        Self::AnalyzerIdentityChange,
    ];
}

#[derive(Clone, Debug, Default)]
pub struct SessionCandidateCache {
    active_identity: Option<SourceIdentity>,
    instant: Option<Candidate>,
    deep: Option<Candidate>,
}

impl SessionCandidateCache {
    pub const fn new() -> Self {
        Self {
            active_identity: None,
            instant: None,
            deep: None,
        }
    }

    pub fn activate(&mut self, identity: SourceIdentity) {
        if self.active_identity.as_ref() != Some(&identity) {
            self.instant = None;
            self.deep = None;
        }
        self.active_identity = Some(identity);
    }

    pub fn store(&mut self, candidate: Candidate) -> Result<(), EngineError> {
        if self.active_identity.as_ref() != Some(&candidate.source_identity) {
            return Err(EngineError::new(EngineErrorCode::CacheIdentityMismatch));
        }
        match candidate.origin {
            CandidateOrigin::Instant => self.instant = Some(candidate),
            CandidateOrigin::Deep => self.deep = Some(candidate),
        }
        Ok(())
    }

    pub fn get(&self, origin: CandidateOrigin, identity: &SourceIdentity) -> Option<&Candidate> {
        if self.active_identity.as_ref() != Some(identity) {
            return None;
        }
        match origin {
            CandidateOrigin::Instant => self.instant.as_ref(),
            CandidateOrigin::Deep => self.deep.as_ref(),
        }
    }

    pub fn instant_text_for(&self, session_id: &str, generation: u64) -> Option<&str> {
        let identity = self.active_identity.as_ref()?;
        if identity.session_id == session_id && identity.generation == generation {
            self.instant.as_ref().map(|candidate| candidate.text.as_str())
        } else {
            None
        }
    }

    pub fn active_identity(&self) -> Option<&SourceIdentity> {
        self.active_identity.as_ref()
    }

    pub fn invalidate(&mut self, _reason: InvalidationReason) {
        self.active_identity = None;
        self.instant = None;
        self.deep = None;
    }

    pub fn is_empty(&self) -> bool {
        self.instant.is_none() && self.deep.is_none()
    }
}
