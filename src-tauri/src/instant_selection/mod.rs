mod cache;
mod candidate;
mod engine;
mod identity;
mod reconciliation;
mod rules;
mod types;
mod utf16;

pub use cache::{InvalidationReason, SessionCandidateCache};
pub use candidate::build_candidate;
pub use engine::InstantSelectionEngine;
pub use identity::{sha256_hex, SourceIdentity};
pub use reconciliation::ReconciliationMachine;
pub use rules::RETAINED_RULES;
pub use types::{
    AnalysisDiagnostics, AnalysisMode, AnalysisOptions, AnalysisRequest, AnalysisResult, Candidate,
    CandidateOrigin, CategoryCount, ConfidenceBand, EngineDescriptor, EngineError, EngineErrorCode,
    LanguageHint, ProtectedSpan, ReconciliationState, RuleDescriptor, Suggestion, SuggestionKind,
    ValidationOutcome, ENGINE_ID, ENGINE_VERSION, MAX_PROTECTED_SPANS, MAX_SUGGESTIONS,
};
pub use utf16::{utf16_len, utf16_range_to_byte_range, Utf16Range};
