use super::{SourceIdentity, Utf16Range};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const ENGINE_ID: &str = "deterministic-rule-engine";
pub const ENGINE_VERSION: &str = "0.1.0";
pub const MAX_SUGGESTIONS: usize = 64;
pub const MAX_PROTECTED_SPANS: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisMode {
    Correction,
    Natural,
    Professional,
    Concise,
    Summary,
    Translation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageHint {
    Korean,
    English,
    Mixed,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisOptions {
    pub source_language_hint: Option<LanguageHint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineDescriptor {
    pub id: &'static str,
    pub version: &'static str,
    pub mode: AnalysisMode,
    pub network_used: bool,
    pub persistent_cache_used: bool,
    pub runtime_wired: bool,
    pub max_suggestions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineErrorCode {
    UnsupportedMode,
    SourceIdentityMismatch,
    SourceLimitExceeded,
    InvalidUtf16Range,
    SurrogateSplit,
    InvalidProtectedSpan,
    TooManyProtectedSpans,
    OverlappingProtectedSpans,
    OverlappingSuggestions,
    DuplicateSuggestion,
    NoOpSuggestion,
    EmptyReplacement,
    TooManySuggestions,
    CandidateSizeExceeded,
    CacheIdentityMismatch,
    IllegalTransition,
    StaleIdentity,
    InvalidCandidateOrigin,
}

impl EngineErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedMode => "unsupported_mode",
            Self::SourceIdentityMismatch => "source_identity_mismatch",
            Self::SourceLimitExceeded => "source_content_limit_exceeded",
            Self::InvalidUtf16Range => "invalid_utf16_range",
            Self::SurrogateSplit => "surrogate_pair_split",
            Self::InvalidProtectedSpan => "invalid_protected_span",
            Self::TooManyProtectedSpans => "too_many_protected_spans",
            Self::OverlappingProtectedSpans => "overlapping_protected_spans",
            Self::OverlappingSuggestions => "overlapping_suggestions",
            Self::DuplicateSuggestion => "duplicate_suggestion",
            Self::NoOpSuggestion => "no_op_suggestion",
            Self::EmptyReplacement => "empty_replacement",
            Self::TooManySuggestions => "too_many_suggestions",
            Self::CandidateSizeExceeded => "candidate_content_limit_exceeded",
            Self::CacheIdentityMismatch => "cache_identity_mismatch",
            Self::IllegalTransition => "illegal_reconciliation_transition",
            Self::StaleIdentity => "stale_source_identity",
            Self::InvalidCandidateOrigin => "invalid_candidate_origin",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineError {
    code: EngineErrorCode,
}

impl EngineError {
    pub(crate) const fn new(code: EngineErrorCode) -> Self {
        Self { code }
    }

    pub const fn code(&self) -> EngineErrorCode {
        self.code
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.as_str())
    }
}

impl std::error::Error for EngineError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedSpan {
    pub range: Utf16Range,
    pub label_code: String,
}

impl ProtectedSpan {
    pub fn new(range: Utf16Range, label_code: impl Into<String>) -> Self {
        Self {
            range,
            label_code: label_code.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisRequest {
    pub source: String,
    pub identity: SourceIdentity,
    pub mode: AnalysisMode,
    pub protected_spans: Vec<ProtectedSpan>,
    pub options: AnalysisOptions,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionKind {
    Spelling,
    Spacing,
    BasicGrammar,
    Punctuation,
}

impl SuggestionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Spelling => "spelling",
            Self::Spacing => "spacing",
            Self::BasicGrammar => "basic_grammar",
            Self::Punctuation => "punctuation",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceBand {
    High,
    Medium,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Suggestion {
    pub suggestion_id: String,
    pub source_identity: SourceIdentity,
    pub kind: SuggestionKind,
    pub range: Utf16Range,
    pub replacement: String,
    pub message_code: String,
    pub rule_code: String,
    pub confidence_band: ConfidenceBand,
    pub engine_id: String,
    pub engine_version: String,
}

impl Suggestion {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_identity: SourceIdentity,
        kind: SuggestionKind,
        range: Utf16Range,
        replacement: impl Into<String>,
        message_code: impl Into<String>,
        rule_code: impl Into<String>,
        confidence_band: ConfidenceBand,
    ) -> Self {
        let rule_code = rule_code.into();
        Self {
            suggestion_id: format!("{}:{}:{}", rule_code, range.start_utf16, range.end_utf16),
            source_identity,
            kind,
            range,
            replacement: replacement.into(),
            message_code: message_code.into(),
            rule_code,
            confidence_band,
            engine_id: ENGINE_ID.to_string(),
            engine_version: ENGINE_VERSION.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateOrigin {
    Instant,
    Deep,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Candidate {
    pub source_identity: SourceIdentity,
    pub origin: CandidateOrigin,
    pub candidate_id: String,
    pub text: String,
}

impl Candidate {
    pub fn new(
        source_identity: SourceIdentity,
        origin: CandidateOrigin,
        candidate_id: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            source_identity,
            origin,
            candidate_id: candidate_id.into(),
            text: text.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcome {
    Accepted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CategoryCount {
    pub kind: SuggestionKind,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisDiagnostics {
    pub engine_id: String,
    pub engine_version: String,
    pub input_utf16_length: usize,
    pub input_scalar_count: usize,
    pub input_byte_count: usize,
    pub suggestion_count: usize,
    pub suppressed_suggestion_count: usize,
    pub rule_ids: Vec<String>,
    pub category_counts: Vec<CategoryCount>,
    pub validation_outcome: ValidationOutcome,
    pub network_used: bool,
    pub persistent_cache_used: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisResult {
    pub source_identity: SourceIdentity,
    pub suggestions: Vec<Suggestion>,
    pub candidate: Candidate,
    pub diagnostics: AnalysisDiagnostics,
    pub validation_outcome: ValidationOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleDescriptor {
    pub id: &'static str,
    pub kind: SuggestionKind,
    pub false_positive_boundary: &'static str,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationState {
    Captured,
    InstantAnalyzing,
    InstantReady,
    DeepAnalyzing,
    DeepReady,
    UserEdited,
    Applied,
    Cancelled,
    Stale,
}
