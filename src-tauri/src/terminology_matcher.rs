use crate::{
    settings::RewriteMode,
    terminology::{
        normalize_match_key, normalize_nfc, EntryStatus, EntryType, LanguageScope,
        TerminologyEntry, TerminologyError, TerminologyStoreV1, GLOBAL_PROFILE_ID,
    },
};
use serde::Serialize;
use std::{cmp::Ordering, collections::HashSet};

const MAX_MATCHES: usize = 50;
const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_CONFLICTS: usize = 50;
const MAX_CONFLICT_ENTRY_IDS: usize = 50;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchContext {
    pub(crate) mode: RewriteMode,
    pub(crate) target_language: Option<LanguageScope>,
    pub(crate) source_language: Option<LanguageScope>,
    pub(crate) active_profile_id: String,
}

impl MatchContext {
    pub(crate) fn new(
        mode: RewriteMode,
        target_language: Option<LanguageScope>,
        source_language: Option<LanguageScope>,
        active_profile_id: String,
    ) -> Self {
        Self {
            mode,
            target_language,
            source_language,
            active_profile_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MatchedTerminology {
    pub(crate) entry_id: String,
    pub(crate) entry_type: EntryType,
    pub(crate) matched_text: String,
    pub(crate) preferred_text: Option<String>,
    pub(crate) span_start: usize,
    pub(crate) span_end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TerminologyConstraint {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) entry_type: EntryType,
    pub(crate) source_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preferred_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TerminologyConflict {
    pub(crate) entry_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MatchResult {
    pub(crate) matches: Vec<MatchedTerminology>,
    pub(crate) conflicts: Vec<TerminologyConflict>,
    pub(crate) identical_duplicates: usize,
    pub(crate) truncated: bool,
}

#[derive(Clone)]
struct Candidate<'a> {
    entry: &'a TerminologyEntry,
    matched_text: String,
    normalized_key: String,
    span_start: usize,
    span_end: usize,
    active_profile_rank: u8,
    language_rank: u8,
    matched_length: usize,
}

pub(crate) fn match_terminology(
    store: &TerminologyStoreV1,
    selected_text: &str,
    context: &MatchContext,
) -> MatchResult {
    let active_enabled = store.enabled_profile(&context.active_profile_id).is_some();
    let mut candidates = Vec::new();

    for entry in &store.entries {
        let Some((profile_rank, language_rank)) = eligibility(entry, context, active_enabled)
        else {
            continue;
        };
        let surfaces = std::iter::once(entry.source_text.as_str())
            .chain(entry.aliases.iter().map(String::as_str));
        for surface in surfaces {
            let Ok(key) = normalize_match_key(surface, entry.case_sensitive) else {
                continue;
            };
            let Ok(source) = SourceView::new(selected_text, entry.case_sensitive) else {
                continue;
            };
            for (span_start, span_end) in source.find_whole_phrase(&key) {
                candidates.push(Candidate {
                    entry,
                    matched_text: surface.to_string(),
                    normalized_key: key.clone(),
                    span_start,
                    span_end,
                    active_profile_rank: profile_rank,
                    language_rank,
                    matched_length: span_end.saturating_sub(span_start),
                });
            }
        }
    }

    candidates.sort_by(compare_candidates);
    let mut result = MatchResult::default();
    collect_conflicts(&candidates, &mut result);
    let mut selected_spans = Vec::<(usize, usize)>::new();
    let mut selected_entry_ids = Vec::<String>::new();

    for candidate in candidates {
        if selected_entry_ids
            .iter()
            .any(|entry_id| entry_id == &candidate.entry.id)
            || selected_spans
                .iter()
                .any(|span| spans_overlap(*span, (candidate.span_start, candidate.span_end)))
        {
            continue;
        }
        if result.matches.len() >= MAX_MATCHES {
            result.truncated = true;
            continue;
        }
        let matched = MatchedTerminology {
            entry_id: candidate.entry.id.clone(),
            entry_type: candidate.entry.entry_type,
            matched_text: candidate.matched_text,
            preferred_text: candidate.entry.preferred_text.clone(),
            span_start: candidate.span_start,
            span_end: candidate.span_end,
        };
        let mut tentative = result.matches.clone();
        tentative.push(matched.clone());
        if constraints_size(&tentative).is_none() {
            result.truncated = true;
            continue;
        }
        selected_spans.push((candidate.span_start, candidate.span_end));
        selected_entry_ids.push(candidate.entry.id.clone());
        result.matches.push(matched);
    }

    result.matches.sort_by(|left, right| {
        left.span_start
            .cmp(&right.span_start)
            .then_with(|| left.entry_id.cmp(&right.entry_id))
    });
    result
}

#[cfg(test)]
pub(crate) fn request_constraints_json(result: &MatchResult) -> Result<String, TerminologyError> {
    let constraints = constraints(&result.matches);
    let json = serde_json::to_string(&constraints).map_err(|_| TerminologyError::InvalidEntry)?;
    if json.len() > MAX_REQUEST_BYTES {
        return Err(TerminologyError::EntryLimit);
    }
    Ok(json)
}

pub(crate) fn constraints(matches: &[MatchedTerminology]) -> Vec<TerminologyConstraint> {
    matches
        .iter()
        .map(|matched| TerminologyConstraint {
            id: matched.entry_id.clone(),
            entry_type: matched.entry_type,
            source_text: matched.matched_text.clone(),
            preferred_text: matched.preferred_text.clone(),
        })
        .collect()
}

fn constraints_size(matches: &[MatchedTerminology]) -> Option<usize> {
    serde_json::to_vec(&constraints(matches))
        .ok()
        .map(|value| value.len())
        .filter(|size| *size <= MAX_REQUEST_BYTES)
}

fn eligibility(
    entry: &TerminologyEntry,
    context: &MatchContext,
    active_enabled: bool,
) -> Option<(u8, u8)> {
    if entry.status != EntryStatus::Approved {
        return None;
    }
    let active_profile_rank = if entry.profile_id == context.active_profile_id && active_enabled {
        1
    } else if entry.profile_id == GLOBAL_PROFILE_ID {
        0
    } else {
        return None;
    };

    let source_rank = language_rank(entry.source_language, context.source_language)?;
    let target_rank = match entry.entry_type {
        EntryType::Protected => 0,
        EntryType::Translation => {
            if context.mode != RewriteMode::Translate {
                return None;
            }
            language_rank(entry.target_language, context.target_language)?
        }
        EntryType::Preferred if context.mode == RewriteMode::Translate => {
            language_rank(entry.target_language, context.target_language)?
        }
        EntryType::Preferred => {
            if entry.target_language == LanguageScope::Any
                || context.source_language == Some(entry.target_language)
                || context.source_language.is_none()
            {
                u8::from(entry.target_language != LanguageScope::Any)
            } else {
                return None;
            }
        }
    };
    Some((active_profile_rank, source_rank + target_rank))
}

fn language_rank(configured: LanguageScope, actual: Option<LanguageScope>) -> Option<u8> {
    if configured == LanguageScope::Any {
        Some(0)
    } else if actual == Some(configured) {
        Some(1)
    } else if actual.is_none() {
        Some(0)
    } else {
        None
    }
}

fn compare_candidates(left: &Candidate<'_>, right: &Candidate<'_>) -> Ordering {
    right
        .active_profile_rank
        .cmp(&left.active_profile_rank)
        .then_with(|| right.language_rank.cmp(&left.language_rank))
        .then_with(|| right.matched_length.cmp(&left.matched_length))
        .then_with(|| right.entry.priority.cmp(&left.entry.priority))
        .then_with(|| right.entry.updated_at_ms.cmp(&left.entry.updated_at_ms))
        .then_with(|| left.entry.id.cmp(&right.entry.id))
        .then_with(|| left.span_start.cmp(&right.span_start))
}

fn collect_conflicts(candidates: &[Candidate<'_>], result: &mut MatchResult) {
    let mut processed_keys = HashSet::new();
    for candidate in candidates {
        if !processed_keys.insert(candidate.normalized_key.clone()) {
            continue;
        }
        let mut entries = candidates
            .iter()
            .filter(|other| other.normalized_key == candidate.normalized_key)
            .map(|other| other.entry)
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        entries.dedup_by(|left, right| left.id == right.id);
        if entries.len() < 2 {
            continue;
        }

        let mut semantic_representatives: Vec<&TerminologyEntry> = Vec::new();
        for entry in &entries {
            if semantic_representatives
                .iter()
                .any(|representative| same_semantics(representative, entry))
            {
                result.identical_duplicates += 1;
            } else {
                semantic_representatives.push(*entry);
            }
        }
        if semantic_representatives.len() > 1 {
            if result.conflicts.len() >= MAX_CONFLICTS {
                result.truncated = true;
                continue;
            }
            let mut entry_ids = entries
                .iter()
                .map(|entry| entry.id.clone())
                .collect::<Vec<_>>();
            if entry_ids.len() > MAX_CONFLICT_ENTRY_IDS {
                entry_ids.truncate(MAX_CONFLICT_ENTRY_IDS);
                result.truncated = true;
            }
            result.conflicts.push(TerminologyConflict { entry_ids });
        }
    }
}

fn same_semantics(left: &TerminologyEntry, right: &TerminologyEntry) -> bool {
    left.entry_type == right.entry_type
        && left.preferred_text == right.preferred_text
        && left.source_language == right.source_language
        && left.target_language == right.target_language
        && left.case_sensitive == right.case_sensitive
}

fn spans_overlap(left: (usize, usize), right: (usize, usize)) -> bool {
    left.0 < right.1 && right.0 < left.1
}

struct SourceView {
    text: String,
    map: Vec<usize>,
    base_chars: Vec<char>,
}

impl SourceView {
    fn new(value: &str, case_sensitive: bool) -> Result<Self, TerminologyError> {
        let nfc = normalize_nfc(value)?;
        let mut base_chars = Vec::new();
        let mut previous_whitespace = false;
        for character in nfc.chars() {
            if character.is_whitespace() {
                if !previous_whitespace {
                    base_chars.push(' ');
                }
                previous_whitespace = true;
            } else {
                base_chars.push(character);
                previous_whitespace = false;
            }
        }

        let mut text = String::new();
        let mut map = Vec::new();
        for (index, character) in base_chars.iter().copied().enumerate() {
            if case_sensitive {
                text.push(character);
                map.push(index);
            } else {
                for lowered in character.to_lowercase() {
                    text.push(lowered);
                    map.push(index);
                }
            }
        }
        Ok(Self {
            text,
            map,
            base_chars,
        })
    }

    fn find_whole_phrase(&self, key: &str) -> Vec<(usize, usize)> {
        if key.is_empty() {
            return Vec::new();
        }
        let key_chars = key.chars().collect::<Vec<_>>();
        let mut matches = Vec::new();
        for (byte_index, _) in self.text.match_indices(key) {
            let char_start = self.text[..byte_index].chars().count();
            let char_end = char_start + key_chars.len();
            let Some(&span_start) = self.map.get(char_start) else {
                continue;
            };
            let Some(&last) = self.map.get(char_end.saturating_sub(1)) else {
                continue;
            };
            let span_end = last + 1;
            if boundary_ok(
                &self.base_chars,
                span_start,
                span_end,
                key_chars.first().copied(),
                key_chars.last().copied(),
            ) {
                matches.push((span_start, span_end));
            }
        }
        matches
    }
}

fn boundary_ok(
    source: &[char],
    start: usize,
    end: usize,
    first: Option<char>,
    last: Option<char>,
) -> bool {
    let left_ok = !first.is_some_and(is_ascii_word)
        || start == 0
        || !source.get(start - 1).copied().is_some_and(is_ascii_word);
    let right_ok = !last.is_some_and(is_ascii_word)
        || end >= source.len()
        || !source.get(end).copied().is_some_and(is_ascii_word);
    left_ok && right_ok
}

fn is_ascii_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}
