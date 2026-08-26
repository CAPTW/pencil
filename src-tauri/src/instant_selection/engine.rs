use super::{
    build_candidate, rules::collect_proposals, utf16_len, utf16_range_to_byte_range,
    AnalysisDiagnostics, AnalysisMode, AnalysisRequest, AnalysisResult, Candidate, CandidateOrigin,
    CategoryCount, EngineDescriptor, EngineError, EngineErrorCode, ProtectedSpan, Suggestion,
    SuggestionKind, ValidationOutcome, ENGINE_ID, ENGINE_VERSION, MAX_PROTECTED_SPANS,
    MAX_SUGGESTIONS,
};
use crate::content_limits::{validate_text_limit, ContentLimitKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default)]
pub struct InstantSelectionEngine;

impl InstantSelectionEngine {
    pub const fn new() -> Self {
        Self
    }

    pub const fn descriptor() -> EngineDescriptor {
        EngineDescriptor {
            id: ENGINE_ID,
            version: ENGINE_VERSION,
            mode: AnalysisMode::Correction,
            network_used: false,
            persistent_cache_used: false,
            runtime_wired: false,
            max_suggestions: MAX_SUGGESTIONS,
        }
    }

    pub fn analyze(&self, request: &AnalysisRequest) -> Result<AnalysisResult, EngineError> {
        if request.mode != AnalysisMode::Correction {
            return Err(EngineError::new(EngineErrorCode::UnsupportedMode));
        }
        validate_text_limit(ContentLimitKind::Source, &request.source)
            .map_err(|_| EngineError::new(EngineErrorCode::SourceLimitExceeded))?;
        request
            .identity
            .validate(&request.source, request.mode, ENGINE_ID, ENGINE_VERSION)?;
        let protected = validate_protected_spans(&request.source, &request.protected_spans)?;
        let _language_hint = request.options.source_language_hint;

        let mut proposals = collect_proposals(&request.source);
        proposals.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then(left.range.start_utf16.cmp(&right.range.start_utf16))
                .then(left.range.end_utf16.cmp(&right.range.end_utf16))
                .then(left.rule_code.cmp(right.rule_code))
        });

        let mut accepted = Vec::new();
        let mut suppressed_suggestion_count = 0usize;
        for proposal in proposals {
            if protected
                .iter()
                .any(|span| proposal.range.intersects(span.range))
            {
                suppressed_suggestion_count = suppressed_suggestion_count.saturating_add(1);
                continue;
            }
            if accepted
                .iter()
                .any(|existing: &Suggestion| proposal.range.intersects(existing.range))
            {
                continue;
            }
            if accepted.len() == MAX_SUGGESTIONS {
                break;
            }
            accepted.push(Suggestion::new(
                request.identity.clone(),
                proposal.kind,
                proposal.range,
                proposal.replacement,
                proposal.message_code,
                proposal.rule_code,
                proposal.confidence_band,
            ));
        }
        accepted.sort_by(|left, right| {
            left.range
                .start_utf16
                .cmp(&right.range.start_utf16)
                .then(left.range.end_utf16.cmp(&right.range.end_utf16))
                .then(left.rule_code.cmp(&right.rule_code))
                .then(left.suggestion_id.cmp(&right.suggestion_id))
        });

        let candidate_text = build_candidate(&request.source, &request.identity, &accepted)?;
        let candidate = Candidate::new(
            request.identity.clone(),
            CandidateOrigin::Instant,
            format!(
                "instant:{}:{}",
                request.identity.source_sha256, ENGINE_VERSION
            ),
            candidate_text,
        );

        let rule_ids = accepted
            .iter()
            .map(|item| item.rule_code.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut counts = BTreeMap::<SuggestionKind, usize>::new();
        for suggestion in &accepted {
            let count = counts.entry(suggestion.kind).or_default();
            *count = count.saturating_add(1);
        }
        let category_counts = counts
            .into_iter()
            .map(|(kind, count)| CategoryCount { kind, count })
            .collect();
        let validation_outcome = ValidationOutcome::Accepted;
        let diagnostics = AnalysisDiagnostics {
            engine_id: ENGINE_ID.to_string(),
            engine_version: ENGINE_VERSION.to_string(),
            input_utf16_length: utf16_len(&request.source),
            input_scalar_count: request.source.chars().count(),
            input_byte_count: request.source.len(),
            suggestion_count: accepted.len(),
            suppressed_suggestion_count,
            rule_ids,
            category_counts,
            validation_outcome,
            network_used: false,
            persistent_cache_used: false,
        };

        Ok(AnalysisResult {
            source_identity: request.identity.clone(),
            suggestions: accepted,
            candidate,
            diagnostics,
            validation_outcome,
        })
    }
}

fn valid_label_code(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn validate_protected_spans(
    source: &str,
    spans: &[ProtectedSpan],
) -> Result<Vec<ProtectedSpan>, EngineError> {
    if spans.len() > MAX_PROTECTED_SPANS {
        return Err(EngineError::new(EngineErrorCode::TooManyProtectedSpans));
    }
    let mut ordered = spans.to_vec();
    ordered.sort_by(|left, right| {
        left.range
            .start_utf16
            .cmp(&right.range.start_utf16)
            .then(left.range.end_utf16.cmp(&right.range.end_utf16))
            .then(left.label_code.cmp(&right.label_code))
    });
    let mut previous_end = 0usize;
    for (index, span) in ordered.iter().enumerate() {
        if !valid_label_code(&span.label_code) {
            return Err(EngineError::new(EngineErrorCode::InvalidProtectedSpan));
        }
        if let Err(error) = utf16_range_to_byte_range(source, span.range) {
            return Err(match error.code() {
                EngineErrorCode::SurrogateSplit => error,
                _ => EngineError::new(EngineErrorCode::InvalidProtectedSpan),
            });
        }
        if index > 0 && span.range.start_utf16 < previous_end {
            return Err(EngineError::new(EngineErrorCode::OverlappingProtectedSpans));
        }
        previous_end = span.range.end_utf16;
    }
    Ok(ordered)
}
