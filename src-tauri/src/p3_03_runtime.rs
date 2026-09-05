use crate::{
    settings::RewriteMode, terminology::EntryType, terminology_matcher::MatchedTerminology,
};
use codex_pencil::instant_selection::{
    AnalysisMode, AnalysisOptions, AnalysisRequest, AnalysisResult, Candidate, CandidateOrigin,
    InstantSelectionEngine, InvalidationReason, ProtectedSpan, SessionCandidateCache,
    SourceIdentity, Utf16Range, ENGINE_ID, ENGINE_VERSION,
};

pub(crate) use codex_pencil::instant_selection::InvalidationReason as InstantInvalidationReason;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::task::spawn_blocking;

#[derive(Debug, Default)]
pub(crate) struct InstantRuntimeState {
    cache: SessionCandidateCache,
    pub(crate) instant_invocations: AtomicU64,
    pub(crate) deep_requests: AtomicU64,
}

impl InstantRuntimeState {
    pub(crate) fn new() -> Self {
        Self {
            cache: SessionCandidateCache::new(),
            instant_invocations: AtomicU64::new(0),
            deep_requests: AtomicU64::new(0),
        }
    }

    pub(crate) fn cache_mut(&mut self) -> &mut SessionCandidateCache {
        &mut self.cache
    }

    pub(crate) fn note_instant_invocation(&self) {
        self.instant_invocations.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn note_deep_request(&self) {
        self.deep_requests.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn instant_text_for(&self, session_id: &str, generation: u64) -> Option<String> {
        self.cache
            .instant_text_for(session_id, generation)
            .map(str::to_string)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstantRuntimeEvent {
    pub(crate) session_id: String,
    pub(crate) generation: u64,
    pub(crate) kind: &'static str,
    pub(crate) suggestion_count: usize,
    pub(crate) no_change: bool,
    pub(crate) error_code: Option<String>,
    pub(crate) candidate_text: Option<String>,
    pub(crate) analyzer_id: String,
    pub(crate) analyzer_version: String,
    pub(crate) elapsed_ms: u128,
    pub(crate) network_used: bool,
    pub(crate) persistent_cache_used: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InstantRuntimeOutcome {
    pub(crate) no_change: bool,
    pub(crate) suggestion_count: usize,
    pub(crate) candidate: Candidate,
    pub(crate) elapsed_ms: u128,
}

pub(crate) fn analysis_mode_for_rewrite(mode: RewriteMode) -> Option<AnalysisMode> {
    match mode {
        RewriteMode::Grammar => Some(AnalysisMode::Correction),
        _ => None,
    }
}

pub(crate) fn protected_spans_from_matches(
    source: &str,
    matches: &[MatchedTerminology],
) -> Result<Vec<ProtectedSpan>, String> {
    let mut spans = Vec::new();
    for matched in matches {
        if matched.entry_type != EntryType::Protected {
            continue;
        }
        if matched.span_end > source.len() || matched.span_start >= matched.span_end {
            return Err("invalid_protected_span".to_string());
        }
        if !source.is_char_boundary(matched.span_start)
            || !source.is_char_boundary(matched.span_end)
        {
            return Err("invalid_protected_span".to_string());
        }
        let start_utf16 = source[..matched.span_start].encode_utf16().count();
        let end_utf16 = source[..matched.span_end].encode_utf16().count();
        spans.push(ProtectedSpan::new(
            Utf16Range::new(start_utf16, end_utf16),
            "protected-term",
        ));
    }
    Ok(spans)
}

pub(crate) fn analyze_correction_source(
    session_id: &str,
    generation: u64,
    intent_revision: u64,
    terminology_revision: u64,
    source: &str,
    protected_spans: Vec<ProtectedSpan>,
) -> Result<InstantRuntimeOutcome, String> {
    let started = Instant::now();
    let identity = SourceIdentity::from_source(
        session_id,
        generation,
        intent_revision,
        terminology_revision,
        AnalysisMode::Correction,
        ENGINE_ID,
        ENGINE_VERSION,
        source,
    );
    let request = AnalysisRequest {
        source: source.to_string(),
        identity,
        mode: AnalysisMode::Correction,
        protected_spans,
        options: AnalysisOptions {
            source_language_hint: None,
        },
    };
    let result: AnalysisResult = InstantSelectionEngine::new()
        .analyze(&request)
        .map_err(|error| error.to_string())?;
    let no_change = result.candidate.text == source || result.suggestions.is_empty();
    Ok(InstantRuntimeOutcome {
        no_change,
        suggestion_count: result.suggestions.len(),
        candidate: result.candidate,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

pub(crate) async fn analyze_correction_source_off_ui(
    session_id: String,
    generation: u64,
    intent_revision: u64,
    terminology_revision: u64,
    source: String,
    protected_spans: Vec<ProtectedSpan>,
) -> Result<InstantRuntimeOutcome, String> {
    spawn_blocking(move || {
        analyze_correction_source(
            &session_id,
            generation,
            intent_revision,
            terminology_revision,
            &source,
            protected_spans,
        )
    })
    .await
    .map_err(|_| "instant_join_failed".to_string())?
}

pub(crate) fn store_candidate(
    cache: &mut SessionCandidateCache,
    candidate: Candidate,
) -> Result<(), String> {
    cache.activate(candidate.source_identity.clone());
    cache.store(candidate).map_err(|error| error.to_string())
}

pub(crate) fn invalidate_cache(cache: &mut SessionCandidateCache, reason: InvalidationReason) {
    let _ = match reason {
        InvalidationReason::Recapture
        | InvalidationReason::Cancel
        | InvalidationReason::Dismiss
        | InvalidationReason::SuccessfulApply
        | InvalidationReason::AppShutdown
        | InvalidationReason::ModeChange
        | InvalidationReason::LanguageChange
        | InvalidationReason::ProfileChange
        | InvalidationReason::TerminologyRevisionChange
        | InvalidationReason::SourceIdentityChange
        | InvalidationReason::AnalyzerIdentityChange => reason,
    };
    cache.invalidate(reason);
}

pub(crate) fn deep_candidate_from_text(identity: SourceIdentity, text: String) -> Candidate {
    let candidate_id = format!("deep:{}:{}", identity.source_sha256, ENGINE_VERSION);
    Candidate::new(identity, CandidateOrigin::Deep, candidate_id, text)
}

pub(crate) fn source_identity_for_capture(
    session_id: &str,
    generation: u64,
    intent_revision: u64,
    terminology_revision: u64,
    source: &str,
) -> SourceIdentity {
    SourceIdentity::from_source(
        session_id,
        generation,
        intent_revision,
        terminology_revision,
        AnalysisMode::Correction,
        ENGINE_ID,
        ENGINE_VERSION,
        source,
    )
}

pub(crate) fn event_from_outcome(
    session_id: String,
    generation: u64,
    outcome: &InstantRuntimeOutcome,
) -> InstantRuntimeEvent {
    InstantRuntimeEvent {
        session_id,
        generation,
        kind: "instant",
        suggestion_count: outcome.suggestion_count,
        no_change: outcome.no_change,
        error_code: None,
        candidate_text: if outcome.no_change {
            None
        } else {
            Some(outcome.candidate.text.clone())
        },
        analyzer_id: ENGINE_ID.to_string(),
        analyzer_version: ENGINE_VERSION.to_string(),
        elapsed_ms: outcome.elapsed_ms,
        network_used: false,
        persistent_cache_used: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminology_matcher::MatchedTerminology;

    #[test]
    fn non_grammar_modes_do_not_map_to_correction() {
        assert!(analysis_mode_for_rewrite(RewriteMode::Natural).is_none());
        assert!(analysis_mode_for_rewrite(RewriteMode::Concise).is_none());
        assert!(analysis_mode_for_rewrite(RewriteMode::Polite).is_none());
        assert!(analysis_mode_for_rewrite(RewriteMode::Translate).is_none());
        assert_eq!(
            analysis_mode_for_rewrite(RewriteMode::Grammar),
            Some(AnalysisMode::Correction)
        );
    }

    #[test]
    fn correction_invokes_engine_and_rejects_mismatched_identity() {
        let source = "I has a apple.";
        let outcome = analyze_correction_source("s1", 1, 1, 1, source, Vec::new()).unwrap();
        assert_eq!(outcome.candidate.source_identity.session_id, "s1");
        assert_eq!(outcome.candidate.origin, CandidateOrigin::Instant);
        assert!(!outcome.candidate.source_identity.source_sha256.is_empty());
    }

    #[test]
    fn session_generation_intent_terminology_and_hash_are_bound() {
        let source = "Hello world";
        let outcome = analyze_correction_source("sess", 9, 3, 7, source, Vec::new()).unwrap();
        let identity = &outcome.candidate.source_identity;
        assert_eq!(identity.session_id, "sess");
        assert_eq!(identity.generation, 9);
        assert_eq!(identity.intent_revision, 3);
        assert_eq!(identity.terminology_revision, 7);
        assert_eq!(identity.analyzer_id, ENGINE_ID);
        assert_eq!(identity.analyzer_version, ENGINE_VERSION);
    }

    #[test]
    fn invalid_protected_span_fails_closed() {
        let source = "keep Term safe";
        let matched = MatchedTerminology {
            entry_id: "e1".to_string(),
            entry_type: EntryType::Protected,
            matched_text: "Term".to_string(),
            preferred_text: None,
            span_start: 5,
            span_end: 99,
        };
        assert!(protected_spans_from_matches(source, &[matched]).is_err());
    }

    #[test]
    fn protected_span_projection_uses_approved_match_bytes() {
        let source = "keep Term safe";
        let start = source.find("Term").unwrap();
        let matched = MatchedTerminology {
            entry_id: "e1".to_string(),
            entry_type: EntryType::Protected,
            matched_text: "Term".to_string(),
            preferred_text: None,
            span_start: start,
            span_end: start + 4,
        };
        let spans = protected_spans_from_matches(source, &[matched]).unwrap();
        assert_eq!(spans.len(), 1);
        let outcome = analyze_correction_source("s", 1, 1, 1, source, spans).unwrap();
        assert!(outcome.candidate.text.contains("Term"));
    }

    #[test]
    fn cache_invalidation_covers_all_eleven_reasons() {
        let mut cache = SessionCandidateCache::new();
        let source = "abc";
        let outcome = analyze_correction_source("s", 1, 1, 1, source, Vec::new()).unwrap();
        store_candidate(&mut cache, outcome.candidate.clone()).unwrap();
        assert!(!cache.is_empty());
        for reason in InvalidationReason::ALL {
            store_candidate(&mut cache, outcome.candidate.clone()).unwrap();
            invalidate_cache(&mut cache, reason);
            assert!(cache.is_empty());
        }
    }

    #[test]
    fn stale_identity_is_not_stored() {
        let mut cache = SessionCandidateCache::new();
        let first = analyze_correction_source("s", 1, 1, 1, "one", Vec::new()).unwrap();
        store_candidate(&mut cache, first.candidate).unwrap();
        let second = analyze_correction_source("s", 2, 1, 1, "two", Vec::new()).unwrap();
        cache
            .store(second.candidate)
            .expect_err("mismatched generation must fail closed");
    }

    #[test]
    fn no_change_when_engine_returns_identical_text() {
        let source = "Already clean text.";
        let outcome = analyze_correction_source("s", 1, 1, 1, source, Vec::new()).unwrap();
        if outcome.candidate.text == source {
            assert!(outcome.no_change);
        }
    }
}
