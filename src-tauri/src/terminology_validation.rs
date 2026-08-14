use crate::{
    terminology::{
        EntryMatchMode, EntryStatus, EntryType, LanguageScope, TerminologyEntryDraft,
        TerminologyError,
    },
    terminology_matcher::MatchResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WarningCode {
    ProtectedMissing,
    PreferredMissing,
    TerminologyUsageUnverified,
    TerminologyMatchTruncated,
    TerminologyConflict,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologyWarning {
    pub(crate) code: WarningCode,
    pub(crate) entry_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SuggestionReason {
    TranslationCandidate,
    PreferredExpression,
    RepeatedPair,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologySuggestion {
    #[serde(rename = "type")]
    pub(crate) entry_type: EntryType,
    pub(crate) source_text: String,
    pub(crate) preferred_text: String,
    pub(crate) source_language: LanguageScope,
    pub(crate) target_language: LanguageScope,
    pub(crate) reason: SuggestionReason,
}

impl TerminologySuggestion {
    pub(crate) fn validate(&self) -> Result<(), TerminologyError> {
        if !matches!(
            self.entry_type,
            EntryType::Translation | EntryType::Preferred
        ) || self.source_text.trim().is_empty()
            || self.preferred_text.trim().is_empty()
            || self.source_text != self.source_text.trim()
            || self.preferred_text != self.preferred_text.trim()
            || self.source_text.chars().count() > 256
            || self.preferred_text.chars().count() > 512
            || self.source_text.contains(['\r', '\n'])
            || self.preferred_text.contains(['\r', '\n'])
        {
            return Err(TerminologyError::InvalidEntry);
        }
        Ok(())
    }

    pub(crate) fn into_suggested_draft(self, profile_id: String) -> TerminologyEntryDraft {
        TerminologyEntryDraft {
            profile_id,
            entry_type: self.entry_type,
            status: EntryStatus::Suggested,
            source_text: self.source_text,
            preferred_text: Some(self.preferred_text),
            source_language: self.source_language,
            target_language: self.target_language,
            aliases: Vec::new(),
            match_mode: EntryMatchMode::WholePhrase,
            case_sensitive: false,
            priority: 100,
            usage_count: 0,
            occurrence_count: 1,
            note: None,
        }
    }
}

pub(crate) fn validate_suggestions(
    suggestions: Vec<TerminologySuggestion>,
) -> Result<Vec<TerminologySuggestion>, TerminologyError> {
    if suggestions.len() > 5 {
        return Err(TerminologyError::EntryLimit);
    }
    for suggestion in &suggestions {
        suggestion.validate()?;
    }
    Ok(suggestions)
}

pub(crate) fn validate_result(
    replacement: &str,
    matches: &MatchResult,
    used_terminology_ids: &[String],
) -> Vec<TerminologyWarning> {
    let used = used_terminology_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut warnings = Vec::new();

    for matched in &matches.matches {
        match matched.entry_type {
            EntryType::Protected if !replacement.contains(&matched.matched_text) => {
                warnings.push(TerminologyWarning {
                    code: WarningCode::ProtectedMissing,
                    entry_ids: vec![matched.entry_id.clone()],
                });
            }
            EntryType::Translation => {
                if matched
                    .preferred_text
                    .as_deref()
                    .is_some_and(|preferred| !replacement.contains(preferred))
                {
                    warnings.push(TerminologyWarning {
                        code: WarningCode::PreferredMissing,
                        entry_ids: vec![matched.entry_id.clone()],
                    });
                }
            }
            EntryType::Preferred if used.contains(matched.entry_id.as_str()) => {
                if matched
                    .preferred_text
                    .as_deref()
                    .is_some_and(|preferred| !replacement.contains(preferred))
                {
                    warnings.push(TerminologyWarning {
                        code: WarningCode::PreferredMissing,
                        entry_ids: vec![matched.entry_id.clone()],
                    });
                }
            }
            EntryType::Preferred => {
                warnings.push(TerminologyWarning {
                    code: WarningCode::TerminologyUsageUnverified,
                    entry_ids: vec![matched.entry_id.clone()],
                });
            }
            EntryType::Protected => {}
        }
    }

    if matches.truncated {
        warnings.push(TerminologyWarning {
            code: WarningCode::TerminologyMatchTruncated,
            entry_ids: Vec::new(),
        });
    }
    for conflict in &matches.conflicts {
        warnings.push(TerminologyWarning {
            code: WarningCode::TerminologyConflict,
            entry_ids: conflict.entry_ids.clone(),
        });
    }
    warnings
}
