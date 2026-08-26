use super::{
    utf16_range_to_byte_range, AnalysisMode, EngineError, EngineErrorCode, SourceIdentity,
    Suggestion, ENGINE_ID, ENGINE_VERSION, MAX_SUGGESTIONS,
};
use crate::content_limits::{validate_text_limit, ContentLimitKind};
use std::collections::BTreeSet;

pub fn build_candidate(
    source: &str,
    identity: &SourceIdentity,
    suggestions: &[Suggestion],
) -> Result<String, EngineError> {
    identity.validate(source, AnalysisMode::Correction, ENGINE_ID, ENGINE_VERSION)?;
    if suggestions.len() > MAX_SUGGESTIONS {
        return Err(EngineError::new(EngineErrorCode::TooManySuggestions));
    }

    let mut ordered = suggestions.to_vec();
    ordered.sort_by(|left, right| {
        left.range
            .start_utf16
            .cmp(&right.range.start_utf16)
            .then(left.range.end_utf16.cmp(&right.range.end_utf16))
            .then(left.rule_code.cmp(&right.rule_code))
            .then(left.suggestion_id.cmp(&right.suggestion_id))
    });

    let mut identities = BTreeSet::new();
    let mut previous_end = 0usize;
    for (index, suggestion) in ordered.iter().enumerate() {
        if suggestion.source_identity != *identity
            || suggestion.engine_id != ENGINE_ID
            || suggestion.engine_version != ENGINE_VERSION
        {
            return Err(EngineError::new(EngineErrorCode::SourceIdentityMismatch));
        }
        let byte_range = utf16_range_to_byte_range(source, suggestion.range)?;
        let semantic_key = (
            suggestion.range.start_utf16,
            suggestion.range.end_utf16,
            suggestion.replacement.clone(),
        );
        if !identities.insert(semantic_key) {
            return Err(EngineError::new(EngineErrorCode::DuplicateSuggestion));
        }
        if index > 0 && suggestion.range.start_utf16 < previous_end {
            return Err(EngineError::new(EngineErrorCode::OverlappingSuggestions));
        }
        previous_end = suggestion.range.end_utf16;
        if suggestion.replacement.is_empty() {
            return Err(EngineError::new(EngineErrorCode::EmptyReplacement));
        }
        if source[byte_range.clone()] == suggestion.replacement {
            return Err(EngineError::new(EngineErrorCode::NoOpSuggestion));
        }
    }

    let mut candidate = source.to_string();
    for suggestion in ordered.iter().rev() {
        let byte_range = utf16_range_to_byte_range(source, suggestion.range)?;
        candidate.replace_range(byte_range, &suggestion.replacement);
    }
    validate_text_limit(ContentLimitKind::FinalApply, &candidate)
        .map_err(|_| EngineError::new(EngineErrorCode::CandidateSizeExceeded))?;
    Ok(candidate)
}
