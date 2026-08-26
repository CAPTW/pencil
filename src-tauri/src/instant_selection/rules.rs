use super::{
    utf16::byte_range_to_utf16_range, ConfidenceBand, RuleDescriptor, SuggestionKind, Utf16Range,
};

pub static RETAINED_RULES: &[RuleDescriptor] = &[
    RuleDescriptor {
        id: "KO_SPACING_CONFIRM",
        kind: SuggestionKind::Spacing,
        false_positive_boundary: "exact Korean predicate phrase with a lexical end boundary",
    },
    RuleDescriptor {
        id: "KO_TYPO_FINAL",
        kind: SuggestionKind::Spelling,
        false_positive_boundary: "complete Korean token only; longer synthetic tokens are excluded",
    },
    RuleDescriptor {
        id: "KO_TENSE_AGREEMENT",
        kind: SuggestionKind::BasicGrammar,
        false_positive_boundary: "same-sentence completed review context is required",
    },
    RuleDescriptor {
        id: "EN_SPELLING_SEPARATE",
        kind: SuggestionKind::Spelling,
        false_positive_boundary: "ASCII whole-word typo only",
    },
    RuleDescriptor {
        id: "EN_SUBJECT_VERB_RESULTS",
        kind: SuggestionKind::BasicGrammar,
        false_positive_boundary: "exact plural subject and adjacent singular verb only",
    },
    RuleDescriptor {
        id: "MIXED_EN_SUCCESSFUL",
        kind: SuggestionKind::Spelling,
        false_positive_boundary: "ASCII whole-word typo in any language context",
    },
    RuleDescriptor {
        id: "KO_SPACING_STABLE",
        kind: SuggestionKind::Spacing,
        false_positive_boundary: "exact adverbial construction only",
    },
    RuleDescriptor {
        id: "KO_SPACING_MODIFY",
        kind: SuggestionKind::Spacing,
        false_positive_boundary: "predicate construction with a lexical end boundary",
    },
];

#[derive(Clone, Debug)]
pub(crate) struct ProposedEdit {
    pub(crate) range: Utf16Range,
    pub(crate) replacement: &'static str,
    pub(crate) kind: SuggestionKind,
    pub(crate) rule_code: &'static str,
    pub(crate) message_code: &'static str,
    pub(crate) confidence_band: ConfidenceBand,
    pub(crate) priority: u8,
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn whole_word(source: &str, start: usize, end: usize) -> bool {
    let before = source[..start].chars().next_back();
    let after = source[end..].chars().next();
    !before.is_some_and(is_word_character) && !after.is_some_and(is_word_character)
}

fn lexical_end(source: &str, end: usize) -> bool {
    !source[end..].chars().next().is_some_and(is_word_character)
}

#[allow(clippy::too_many_arguments)]
fn push_literal_matches(
    source: &str,
    needle: &'static str,
    replacement: &'static str,
    kind: SuggestionKind,
    rule_code: &'static str,
    message_code: &'static str,
    priority: u8,
    require_whole_word: bool,
    require_lexical_end: bool,
    output: &mut Vec<ProposedEdit>,
) {
    for (start, _) in source.match_indices(needle) {
        let end = start + needle.len();
        if require_whole_word && !whole_word(source, start, end) {
            continue;
        }
        if require_lexical_end && !lexical_end(source, end) {
            continue;
        }
        if let Ok(range) = byte_range_to_utf16_range(source, start, end) {
            output.push(ProposedEdit {
                range,
                replacement,
                kind,
                rule_code,
                message_code,
                confidence_band: ConfidenceBand::High,
                priority,
            });
        }
    }
}

fn sentence_context_before(source: &str, byte_start: usize) -> &str {
    let prefix = &source[..byte_start];
    let boundary = prefix
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            matches!(character, '.' | '!' | '?' | '\n' | '\r')
                .then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    &prefix[boundary..]
}

fn push_tense_matches(source: &str, output: &mut Vec<ProposedEdit>) {
    const NEEDLE: &str = "작성한다";
    for (start, _) in source.match_indices(NEEDLE) {
        let end = start + NEEDLE.len();
        if !whole_word(source, start, end)
            || !sentence_context_before(source, start).contains("검토했고")
        {
            continue;
        }
        if let Ok(range) = byte_range_to_utf16_range(source, start, end) {
            output.push(ProposedEdit {
                range,
                replacement: "작성했다",
                kind: SuggestionKind::BasicGrammar,
                rule_code: "KO_TENSE_AGREEMENT",
                message_code: "LOCAL_TENSE_AGREEMENT",
                confidence_band: ConfidenceBand::High,
                priority: 2,
            });
        }
    }
}

fn push_results_agreement(source: &str, output: &mut Vec<ProposedEdit>) {
    const PHRASE: &str = "results is";
    for (start, _) in source.match_indices(PHRASE) {
        let end = start + PHRASE.len();
        if !whole_word(source, start, end) {
            continue;
        }
        let verb_start = end - 2;
        if let Ok(range) = byte_range_to_utf16_range(source, verb_start, end) {
            output.push(ProposedEdit {
                range,
                replacement: "are",
                kind: SuggestionKind::BasicGrammar,
                rule_code: "EN_SUBJECT_VERB_RESULTS",
                message_code: "LOCAL_SUBJECT_VERB_AGREEMENT",
                confidence_band: ConfidenceBand::High,
                priority: 4,
            });
        }
    }
}

pub(crate) fn collect_proposals(source: &str) -> Vec<ProposedEdit> {
    let mut output = Vec::new();
    push_literal_matches(
        source,
        "확인 해",
        "확인해",
        SuggestionKind::Spacing,
        "KO_SPACING_CONFIRM",
        "LOCAL_KO_SPACING",
        0,
        false,
        true,
        &mut output,
    );
    push_literal_matches(
        source,
        "명확합니댜",
        "명확합니다",
        SuggestionKind::Spelling,
        "KO_TYPO_FINAL",
        "LOCAL_KO_SPELLING",
        1,
        true,
        false,
        &mut output,
    );
    push_tense_matches(source, &mut output);
    push_literal_matches(
        source,
        "seperate",
        "separate",
        SuggestionKind::Spelling,
        "EN_SPELLING_SEPARATE",
        "LOCAL_EN_SPELLING",
        3,
        true,
        false,
        &mut output,
    );
    push_results_agreement(source, &mut output);
    push_literal_matches(
        source,
        "sucessful",
        "successful",
        SuggestionKind::Spelling,
        "MIXED_EN_SUCCESSFUL",
        "LOCAL_EN_SPELLING",
        5,
        true,
        false,
        &mut output,
    );
    push_literal_matches(
        source,
        "안정 적으로",
        "안정적으로",
        SuggestionKind::Spacing,
        "KO_SPACING_STABLE",
        "LOCAL_KO_SPACING",
        6,
        false,
        true,
        &mut output,
    );
    push_literal_matches(
        source,
        "수정 할",
        "수정할",
        SuggestionKind::Spacing,
        "KO_SPACING_MODIFY",
        "LOCAL_KO_SPACING",
        7,
        false,
        true,
        &mut output,
    );
    output
}
