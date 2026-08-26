use codex_pencil::instant_selection::{
    build_candidate, sha256_hex, utf16_len, utf16_range_to_byte_range, AnalysisMode,
    AnalysisOptions, AnalysisRequest, Candidate, CandidateOrigin, ConfidenceBand, EngineErrorCode,
    InstantSelectionEngine, InvalidationReason, ProtectedSpan, ReconciliationMachine,
    ReconciliationState, SessionCandidateCache, SourceIdentity, Suggestion, SuggestionKind,
    Utf16Range, ValidationOutcome, ENGINE_ID, ENGINE_VERSION, MAX_SUGGESTIONS, RETAINED_RULES,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::Instant,
};

fn identity_for(source: &str, session_id: &str) -> SourceIdentity {
    SourceIdentity::from_source(
        session_id,
        7,
        11,
        13,
        AnalysisMode::Correction,
        ENGINE_ID,
        ENGINE_VERSION,
        source,
    )
}

fn request_for(source: &str, session_id: &str) -> AnalysisRequest {
    AnalysisRequest {
        source: source.to_string(),
        identity: identity_for(source, session_id),
        mode: AnalysisMode::Correction,
        protected_spans: Vec::new(),
        options: AnalysisOptions::default(),
    }
}

fn range_of(source: &str, needle: &str) -> Utf16Range {
    let byte_start = source
        .find(needle)
        .expect("synthetic test needle must exist");
    let start_utf16 = source[..byte_start].encode_utf16().count();
    Utf16Range::new(start_utf16, start_utf16 + needle.encode_utf16().count())
}

fn suggestion(
    source: &str,
    identity: &SourceIdentity,
    needle: &str,
    replacement: &str,
    kind: SuggestionKind,
    rule_code: &str,
) -> Suggestion {
    Suggestion::new(
        identity.clone(),
        kind,
        range_of(source, needle),
        replacement,
        "LOCAL_CORRECTION",
        rule_code,
        ConfidenceBand::High,
    )
}

fn candidate(
    identity: &SourceIdentity,
    origin: CandidateOrigin,
    id: &str,
    text: &str,
) -> Candidate {
    Candidate::new(identity.clone(), origin, id, text)
}

fn rule_ids(source: &str) -> BTreeSet<String> {
    let result = InstantSelectionEngine::new()
        .analyze(&request_for(source, "rule-test"))
        .expect("synthetic rule analysis must succeed");
    result
        .suggestions
        .iter()
        .map(|item| item.rule_code.clone())
        .collect()
}

#[test]
fn engine_descriptor_is_stable_offline_nonpersistent_and_not_runtime_wired() {
    let descriptor = InstantSelectionEngine::descriptor();
    assert_eq!(descriptor.id, "deterministic-rule-engine");
    assert_eq!(descriptor.version, "0.1.0");
    assert_eq!(descriptor.mode, AnalysisMode::Correction);
    assert!(!descriptor.network_used);
    assert!(!descriptor.persistent_cache_used);
    assert!(!descriptor.runtime_wired);
    assert_eq!(descriptor.max_suggestions, MAX_SUGGESTIONS);
    assert!((1..=128).contains(&MAX_SUGGESTIONS));
}

#[test]
fn correction_only_mode_rejects_noncorrection_requests() {
    let source = "A bounded synthetic sentence.";
    let mut request = request_for(source, "mode-test");
    request.mode = AnalysisMode::Natural;
    request.identity = SourceIdentity::from_source(
        "mode-test",
        7,
        11,
        13,
        AnalysisMode::Natural,
        ENGINE_ID,
        ENGINE_VERSION,
        source,
    );
    let error = InstantSelectionEngine::new()
        .analyze(&request)
        .expect_err("non-correction mode must fail closed");
    assert_eq!(error.code(), EngineErrorCode::UnsupportedMode);
}

#[test]
fn source_identity_hash_and_every_identity_field_are_exact() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let source = "자료를 확인 해 주세요.";
    let mut request = request_for(source, "identity-a");
    request.identity.source_sha256 = "0".repeat(64);
    let error = InstantSelectionEngine::new()
        .analyze(&request)
        .expect_err("caller-provided hash must be recomputed");
    assert_eq!(error.code(), EngineErrorCode::SourceIdentityMismatch);

    let baseline = identity_for(source, "identity-a");
    let variants = [
        SourceIdentity {
            session_id: "identity-b".into(),
            ..baseline.clone()
        },
        SourceIdentity {
            generation: 8,
            ..baseline.clone()
        },
        SourceIdentity {
            intent_revision: 12,
            ..baseline.clone()
        },
        SourceIdentity {
            terminology_revision: 14,
            ..baseline.clone()
        },
        SourceIdentity {
            source_sha256: "1".repeat(64),
            ..baseline.clone()
        },
        SourceIdentity {
            mode: AnalysisMode::Natural,
            ..baseline.clone()
        },
        SourceIdentity {
            analyzer_id: "other-engine".into(),
            ..baseline.clone()
        },
        SourceIdentity {
            analyzer_version: "9.9.9".into(),
            ..baseline.clone()
        },
    ];
    assert!(variants.iter().all(|variant| variant != &baseline));
}

#[test]
fn oversized_source_is_rejected_without_truncation() {
    let source = "가".repeat(12_001);
    let error = InstantSelectionEngine::new()
        .analyze(&request_for(&source, "oversized"))
        .expect_err("12,001 scalars must fail closed");
    assert_eq!(error.code(), EngineErrorCode::SourceLimitExceeded);
}

#[test]
fn utf16_boundaries_cover_bmp_supplementary_combining_and_line_endings() {
    let source = "가A😀𐐷e\u{301}\r\n나\nB";
    assert_eq!(utf16_len(source), source.encode_utf16().count());

    for needle in ["가", "A", "😀", "𐐷", "e\u{301}", "\r\n", "나", "\nB"] {
        let range = range_of(source, needle);
        let bytes = utf16_range_to_byte_range(source, range)
            .expect("valid scalar-aligned UTF-16 range must convert");
        assert_eq!(&source[bytes], needle);
    }

    let emoji = range_of(source, "😀");
    let split = Utf16Range::new(emoji.start_utf16 + 1, emoji.end_utf16);
    let error = utf16_range_to_byte_range(source, split)
        .expect_err("a surrogate-pair split must be rejected");
    assert_eq!(error.code(), EngineErrorCode::SurrogateSplit);

    for invalid in [
        Utf16Range::new(3, 3),
        Utf16Range::new(9_999, 10_000),
        Utf16Range::new(4, 2),
    ] {
        assert_eq!(
            utf16_range_to_byte_range(source, invalid)
                .expect_err("invalid UTF-16 range must fail")
                .code(),
            EngineErrorCode::InvalidUtf16Range
        );
    }
}

#[test]
fn suggestion_validation_rejects_overlap_duplicates_noops_and_stale_identity() {
    let source = "abcd";
    let identity = identity_for(source, "suggestion-validation");
    let left = suggestion(
        source,
        &identity,
        "ab",
        "A",
        SuggestionKind::Spelling,
        "TEST_LEFT",
    );
    let overlapping = Suggestion::new(
        identity.clone(),
        SuggestionKind::Spacing,
        Utf16Range::new(1, 3),
        "B",
        "LOCAL_CORRECTION",
        "TEST_OVERLAP",
        ConfidenceBand::High,
    );
    assert_eq!(
        build_candidate(source, &identity, &[left.clone(), overlapping])
            .expect_err("overlap must be rejected")
            .code(),
        EngineErrorCode::OverlappingSuggestions
    );
    assert_eq!(
        build_candidate(source, &identity, &[left.clone(), left.clone()])
            .expect_err("duplicates must be rejected")
            .code(),
        EngineErrorCode::DuplicateSuggestion
    );

    let noop = suggestion(
        source,
        &identity,
        "ab",
        "ab",
        SuggestionKind::Spelling,
        "TEST_NOOP",
    );
    assert_eq!(
        build_candidate(source, &identity, &[noop])
            .expect_err("no-op replacement must be rejected")
            .code(),
        EngineErrorCode::NoOpSuggestion
    );

    let stale_identity = identity_for(source, "other-session");
    let stale = suggestion(
        source,
        &stale_identity,
        "ab",
        "A",
        SuggestionKind::Spelling,
        "TEST_STALE",
    );
    assert_eq!(
        build_candidate(source, &identity, &[stale])
            .expect_err("stale suggestion identity must be rejected")
            .code(),
        EngineErrorCode::SourceIdentityMismatch
    );
}

#[test]
fn protected_ranges_are_validated_and_intersections_are_suppressed() {
    let source = "Codex Pencil 문장을 확인 해 주세요.";
    let mut request = request_for(source, "protected");
    request.protected_spans = vec![ProtectedSpan::new(
        range_of(source, "확인 해"),
        "approved-term",
    )];
    let result = InstantSelectionEngine::new()
        .analyze(&request)
        .expect("valid protected span must analyze");
    assert!(result.suggestions.is_empty());
    assert_eq!(result.candidate.text, source);
    assert_eq!(result.diagnostics.suppressed_suggestion_count, 1);

    request.protected_spans = vec![ProtectedSpan::new(
        Utf16Range::new(1, 1),
        "invalid-protected",
    )];
    assert_eq!(
        InstantSelectionEngine::new()
            .analyze(&request)
            .expect_err("zero-length protected span must fail")
            .code(),
        EngineErrorCode::InvalidProtectedSpan
    );
}

#[test]
fn candidate_construction_applies_adjacent_ranges_descending_and_preserves_structure() {
    let source = "  abcd\r\nnext\n";
    let identity = identity_for(source, "candidate");
    let left = suggestion(
        source,
        &identity,
        "ab",
        "A",
        SuggestionKind::Spelling,
        "TEST_LEFT",
    );
    let right = suggestion(
        source,
        &identity,
        "cd",
        "B",
        SuggestionKind::Spacing,
        "TEST_RIGHT",
    );
    assert_eq!(
        build_candidate(source, &identity, &[left, right]).expect("adjacent ranges are valid"),
        "  AB\r\nnext\n"
    );
    assert_eq!(
        build_candidate(source, &identity, &[]).expect("zero suggestions must be valid"),
        source
    );
}

#[test]
fn retained_rules_have_stable_inventory_false_positive_boundaries_and_coverage() {
    assert_eq!(RETAINED_RULES.len(), 8);
    let unique = RETAINED_RULES
        .iter()
        .map(|rule| rule.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), RETAINED_RULES.len());
    assert!(RETAINED_RULES
        .iter()
        .all(|rule| { !rule.id.is_empty() && !rule.false_positive_boundary.is_empty() }));

    let positive_cases: [(&str, [&str; 2]); 8] = [
        (
            "KO_SPACING_CONFIRM",
            ["서류를 확인 해 주세요.", "내용을 확인 해 보세요."],
        ),
        (
            "KO_TYPO_FINAL",
            ["판단은 명확합니댜.", "요건이 명확합니댜!"],
        ),
        (
            "KO_TENSE_AGREEMENT",
            [
                "팀이 검토했고 결론을 작성한다.",
                "위원이 검토했고 보고서를 작성한다.",
            ],
        ),
        (
            "EN_SPELLING_SEPARATE",
            ["Keep a seperate record.", "Use the seperate folder."],
        ),
        (
            "EN_SUBJECT_VERB_RESULTS",
            ["The results is clear.", "Audit results is final."],
        ),
        (
            "MIXED_EN_SUCCESSFUL",
            ["배포는 sucessful 상태입니다.", "The run was sucessful."],
        ),
        (
            "KO_SPACING_STABLE",
            [
                "서비스가 안정 적으로 동작한다.",
                "장비는 안정 적으로 유지된다.",
            ],
        ),
        (
            "KO_SPACING_MODIFY",
            ["문장을 수정 할 수 있다.", "값을 수정 할 필요가 있다."],
        ),
    ];
    for (rule, cases) in positive_cases {
        for source in cases {
            assert!(
                rule_ids(source).contains(rule),
                "positive rule {rule} must match"
            );
        }
    }

    let negative_cases: [(&str, [&str; 2]); 8] = [
        (
            "KO_SPACING_CONFIRM",
            ["서류를 확인해 주세요.", "확인하여 제출하세요."],
        ),
        (
            "KO_TYPO_FINAL",
            ["판단은 명확합니다.", "명확합니댜아는 합성 토큰이다."],
        ),
        (
            "KO_TENSE_AGREEMENT",
            [
                "팀이 내일 결론을 작성한다.",
                "팀이 검토했고 결론을 작성했다.",
            ],
        ),
        (
            "EN_SPELLING_SEPARATE",
            [
                "Keep a separate record.",
                "The word seperately is outside this rule.",
            ],
        ),
        (
            "EN_SUBJECT_VERB_RESULTS",
            ["The result is clear.", "The results are clear."],
        ),
        (
            "MIXED_EN_SUCCESSFUL",
            [
                "배포는 successful 상태입니다.",
                "The token sucessfully is outside this rule.",
            ],
        ),
        (
            "KO_SPACING_STABLE",
            [
                "서비스가 안정적으로 동작한다.",
                "안정 적 조건은 별도 명사구다.",
            ],
        ),
        (
            "KO_SPACING_MODIFY",
            ["문장을 수정할 수 있다.", "수정 할당량은 별도 명사구다."],
        ),
    ];
    for (rule, cases) in negative_cases {
        for source in cases {
            assert!(
                !rule_ids(source).contains(rule),
                "negative rule {rule} must not match"
            );
        }
    }
}

#[test]
fn at_least_twenty_four_synthetic_off_corpus_cases_cover_every_retained_rule() {
    let cases = [
        ("교사는 답안을 확인 해 즉시 반환했다.", "KO_SPACING_CONFIRM"),
        ("운영자는 상태를 확인 해 기록했다.", "KO_SPACING_CONFIRM"),
        ("검토자는 수치를 확인 해 승인했다.", "KO_SPACING_CONFIRM"),
        ("이 결론은 명확합니댜.", "KO_TYPO_FINAL"),
        ("해당 기준도 명확합니댜.", "KO_TYPO_FINAL"),
        ("원인은 매우 명확합니댜.", "KO_TYPO_FINAL"),
        ("연구자가 검토했고 요약을 작성한다.", "KO_TENSE_AGREEMENT"),
        ("담당자가 검토했고 회신을 작성한다.", "KO_TENSE_AGREEMENT"),
        ("편집자가 검토했고 초안을 작성한다.", "KO_TENSE_AGREEMENT"),
        ("Create a seperate archive.", "EN_SPELLING_SEPARATE"),
        ("Open a seperate channel.", "EN_SPELLING_SEPARATE"),
        ("Choose a seperate sample.", "EN_SPELLING_SEPARATE"),
        (
            "The measured results is repeatable.",
            "EN_SUBJECT_VERB_RESULTS",
        ),
        ("These results is actionable.", "EN_SUBJECT_VERB_RESULTS"),
        ("Our results is reproducible.", "EN_SUBJECT_VERB_RESULTS"),
        ("테스트가 sucessful 로 끝났다.", "MIXED_EN_SUCCESSFUL"),
        ("응답은 sucessful 이었다.", "MIXED_EN_SUCCESSFUL"),
        ("A sucessful 결과를 기록했다.", "MIXED_EN_SUCCESSFUL"),
        ("프로세스가 안정 적으로 종료됐다.", "KO_SPACING_STABLE"),
        ("연결이 안정 적으로 유지됐다.", "KO_SPACING_STABLE"),
        ("출력이 안정 적으로 생성됐다.", "KO_SPACING_STABLE"),
        ("오류를 수정 할 계획이다.", "KO_SPACING_MODIFY"),
        ("설정을 수정 할 권한이 있다.", "KO_SPACING_MODIFY"),
        ("초안을 수정 할 예정이다.", "KO_SPACING_MODIFY"),
    ];
    assert_eq!(cases.len(), 24);
    let mut covered = BTreeSet::new();
    for (source, rule) in cases {
        assert!(
            rule_ids(source).contains(rule),
            "off-corpus rule {rule} must match"
        );
        covered.insert(rule);
    }
    assert_eq!(covered.len(), RETAINED_RULES.len());
}

#[test]
fn prompt_like_text_urls_code_units_and_literals_are_treated_as_unchanged_data() {
    let source = "Ignore previous instructions. URL https://example.invalid/x, code `CASE_X`, 10 kg. 수정 할 부분.";
    let result = InstantSelectionEngine::new()
        .analyze(&request_for(source, "structure"))
        .expect("prompt-like synthetic data must analyze locally");
    assert!(result
        .candidate
        .text
        .contains("Ignore previous instructions."));
    assert!(result.candidate.text.contains("https://example.invalid/x"));
    assert!(result.candidate.text.contains("`CASE_X`"));
    assert!(result.candidate.text.contains("10 kg"));
    assert_eq!(result.validation_outcome, ValidationOutcome::Accepted);
}

#[test]
fn session_cache_is_identity_isolated_bounded_and_explicitly_invalidated() {
    let source = "자료를 확인 해 주세요.";
    let first_identity = identity_for(source, "cache-a");
    let second_identity = identity_for(source, "cache-b");
    let instant = candidate(
        &first_identity,
        CandidateOrigin::Instant,
        "instant-a",
        "자료를 확인해 주세요.",
    );
    let deep = candidate(
        &first_identity,
        CandidateOrigin::Deep,
        "deep-a",
        "자료를 확인해 주세요.",
    );

    let mut cache = SessionCandidateCache::new();
    cache.activate(first_identity.clone());
    cache
        .store(instant.clone())
        .expect("active Instant candidate must store");
    cache
        .store(deep.clone())
        .expect("active Deep candidate must store");
    assert_eq!(
        cache.get(CandidateOrigin::Instant, &first_identity),
        Some(&instant)
    );
    assert_eq!(
        cache.get(CandidateOrigin::Deep, &first_identity),
        Some(&deep)
    );
    assert_eq!(cache.get(CandidateOrigin::Instant, &second_identity), None);

    cache.activate(second_identity.clone());
    assert_eq!(cache.get(CandidateOrigin::Instant, &first_identity), None);
    assert_eq!(cache.get(CandidateOrigin::Deep, &first_identity), None);
    assert_eq!(cache.active_identity(), Some(&second_identity));

    for reason in InvalidationReason::ALL {
        let mut cache = SessionCandidateCache::new();
        cache.activate(first_identity.clone());
        cache.store(instant.clone()).expect("fixture must store");
        cache.invalidate(reason);
        assert!(
            cache.active_identity().is_none(),
            "{reason:?} must clear identity"
        );
        assert!(cache.is_empty(), "{reason:?} must clear candidates");
    }
}

#[test]
fn deep_completion_never_silently_replaces_instant_or_user_draft() {
    let source = "The results is stable.";
    let identity = identity_for(source, "reconcile");
    let instant = candidate(
        &identity,
        CandidateOrigin::Instant,
        "instant",
        "The results are stable.",
    );
    let deep = candidate(
        &identity,
        CandidateOrigin::Deep,
        "deep",
        "The results remain stable.",
    );

    let mut machine = ReconciliationMachine::new(identity.clone(), source);
    machine
        .begin_instant(&identity)
        .expect("Captured -> InstantAnalyzing");
    machine
        .complete_instant(&identity, instant.clone())
        .expect("Instant -> ready");
    assert_eq!(machine.state(), ReconciliationState::InstantReady);
    assert_eq!(machine.draft(), instant.text);

    machine
        .begin_deep(&identity)
        .expect("InstantReady -> DeepAnalyzing");
    machine
        .complete_deep(&identity, deep.clone())
        .expect("Deep completion must be retained");
    assert_eq!(machine.state(), ReconciliationState::DeepReady);
    assert_eq!(machine.draft(), instant.text);
    assert_eq!(machine.alternate(CandidateOrigin::Deep), Some(&deep));

    machine
        .edit_draft("Owner-edited synthetic draft")
        .expect("ready draft may be edited");
    let revision = machine.draft_revision();
    assert_eq!(machine.state(), ReconciliationState::UserEdited);
    machine
        .begin_deep(&identity)
        .expect("UserEdited may request a Deep alternate");
    machine
        .complete_deep(&identity, deep.clone())
        .expect("Deep alternate may complete");
    assert_eq!(machine.state(), ReconciliationState::UserEdited);
    assert_eq!(machine.draft(), "Owner-edited synthetic draft");
    assert_eq!(machine.draft_revision(), revision);

    machine
        .switch_candidate(&identity, CandidateOrigin::Deep)
        .expect("explicit switch is allowed");
    assert_eq!(machine.draft(), deep.text);
    assert_eq!(
        machine.recoverable_user_draft(),
        Some("Owner-edited synthetic draft")
    );
    machine
        .restore_user_draft()
        .expect("saved user draft must be recoverable");
    assert_eq!(machine.draft(), "Owner-edited synthetic draft");
}

#[test]
fn reconciliation_rejects_illegal_stale_and_terminal_transitions() {
    let source = "The results is stable.";
    let identity = identity_for(source, "state-a");
    let stale = identity_for(source, "state-b");
    let instant = candidate(
        &identity,
        CandidateOrigin::Instant,
        "instant",
        "The results are stable.",
    );

    let mut machine = ReconciliationMachine::new(identity.clone(), source);
    assert_eq!(
        machine
            .complete_instant(&identity, instant.clone())
            .expect_err("completion before analysis is illegal")
            .code(),
        EngineErrorCode::IllegalTransition
    );
    assert_eq!(
        machine
            .begin_instant(&stale)
            .expect_err("stale identity is illegal")
            .code(),
        EngineErrorCode::StaleIdentity
    );
    assert_eq!(machine.state(), ReconciliationState::Stale);

    for terminal in [
        ReconciliationState::Applied,
        ReconciliationState::Cancelled,
        ReconciliationState::Stale,
    ] {
        let mut terminal_machine = ReconciliationMachine::new(identity.clone(), source);
        match terminal {
            ReconciliationState::Applied => {
                terminal_machine
                    .begin_instant(&identity)
                    .expect("fixture transition");
                terminal_machine
                    .complete_instant(&identity, instant.clone())
                    .expect("fixture ready");
                terminal_machine.apply(&identity).expect("explicit Apply");
            }
            ReconciliationState::Cancelled => terminal_machine.cancel(),
            ReconciliationState::Stale => terminal_machine.mark_stale(),
            _ => unreachable!(),
        }
        assert_eq!(terminal_machine.state(), terminal);
        assert_eq!(
            terminal_machine
                .begin_instant(&identity)
                .expect_err("terminal state must reject analysis")
                .code(),
            EngineErrorCode::IllegalTransition
        );
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkProtectedSpan {
    start_utf16: usize,
    end_utf16: usize,
    label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BenchmarkCase {
    case_id: String,
    source: String,
    source_sha256: String,
    protected_spans_utf16: Vec<BenchmarkProtectedSpan>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PredictionEngine {
    id: String,
    version: String,
    kind: String,
    cold_start_ms: f64,
    peak_rss_mi_b: f64,
    artifact_bytes: u64,
    network_used: bool,
    persistent_cache_used: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PredictionSuggestion {
    suggestion_id: String,
    start_utf16: usize,
    end_utf16: usize,
    replacement: String,
    kind: String,
    rule_code: String,
    engine_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PredictionCase {
    case_id: String,
    source_sha256: String,
    latency_ms: f64,
    suggestions: Vec<PredictionSuggestion>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Predictions {
    schema_version: u32,
    engine: PredictionEngine,
    cases: Vec<PredictionCase>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticSuggestion {
    suggestion_id: String,
    rule_id: String,
    category: String,
    start_utf16: usize,
    end_utf16: usize,
    replacement: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SemanticCase {
    case_id: String,
    source_identity: SourceIdentity,
    accepted_suggestions: Vec<SemanticSuggestion>,
    final_candidate: String,
    validation_outcome: ValidationOutcome,
}

fn required_env_path(name: &str) -> PathBuf {
    PathBuf::from(env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn write_new(path: &PathBuf, bytes: &[u8]) {
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap_or_else(|error| panic!("could not create benchmark output: {error}"));
    output
        .write_all(bytes)
        .unwrap_or_else(|error| panic!("could not write benchmark output: {error}"));
}

#[test]
#[ignore = "external product-engine benchmark writer"]
fn product_benchmark_writer_uses_the_same_rust_analysis_api() {
    let corpus_path = required_env_path("P3_02_CORPUS_PATH");
    let predictions_path = required_env_path("P3_02_PREDICTIONS_PATH");
    let semantic_path = required_env_path("P3_02_SEMANTIC_PATH");
    let artifact_bytes = env::var("P3_02_ARTIFACT_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let corpus_body = fs::read_to_string(corpus_path).expect("synthetic corpus must be readable");
    let corpus = corpus_body
        .lines()
        .map(|line| serde_json::from_str::<BenchmarkCase>(line).expect("corpus case must parse"))
        .collect::<Vec<_>>();
    assert_eq!(corpus.len(), 120);

    let cold_start = Instant::now();
    let engine = InstantSelectionEngine::new();
    let mut predictions = Vec::with_capacity(corpus.len());
    let mut semantics = Vec::with_capacity(corpus.len());
    let mut measured_cold_ms = 0.0;

    for (index, item) in corpus.into_iter().enumerate() {
        let identity = SourceIdentity::from_source(
            format!("benchmark-{}", item.case_id),
            1,
            1,
            1,
            AnalysisMode::Correction,
            ENGINE_ID,
            ENGINE_VERSION,
            &item.source,
        );
        assert_eq!(identity.source_sha256, item.source_sha256);
        let request = AnalysisRequest {
            source: item.source,
            identity,
            mode: AnalysisMode::Correction,
            protected_spans: item
                .protected_spans_utf16
                .into_iter()
                .map(|span| {
                    ProtectedSpan::new(
                        Utf16Range::new(span.start_utf16, span.end_utf16),
                        span.label,
                    )
                })
                .collect(),
            options: AnalysisOptions::default(),
        };
        let started = Instant::now();
        let result = engine
            .analyze(&request)
            .expect("product engine analysis must succeed");
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        if index == 0 {
            measured_cold_ms = cold_start.elapsed().as_secs_f64() * 1000.0;
        }

        let prediction_suggestions = result
            .suggestions
            .iter()
            .map(|suggestion| PredictionSuggestion {
                suggestion_id: suggestion.suggestion_id.clone(),
                start_utf16: suggestion.range.start_utf16,
                end_utf16: suggestion.range.end_utf16,
                replacement: suggestion.replacement.clone(),
                kind: suggestion.kind.as_str().to_string(),
                rule_code: suggestion.rule_code.clone(),
                engine_version: suggestion.engine_version.clone(),
            })
            .collect();
        let semantic_suggestions = result
            .suggestions
            .iter()
            .map(|suggestion| SemanticSuggestion {
                suggestion_id: suggestion.suggestion_id.clone(),
                rule_id: suggestion.rule_code.clone(),
                category: suggestion.kind.as_str().to_string(),
                start_utf16: suggestion.range.start_utf16,
                end_utf16: suggestion.range.end_utf16,
                replacement: suggestion.replacement.clone(),
            })
            .collect();
        predictions.push(PredictionCase {
            case_id: item.case_id.clone(),
            source_sha256: request.identity.source_sha256.clone(),
            latency_ms,
            suggestions: prediction_suggestions,
        });
        semantics.push(SemanticCase {
            case_id: item.case_id,
            source_identity: request.identity,
            accepted_suggestions: semantic_suggestions,
            final_candidate: result.candidate.text,
            validation_outcome: result.validation_outcome,
        });
    }

    let predictions_document = Predictions {
        schema_version: 1,
        engine: PredictionEngine {
            id: ENGINE_ID.to_string(),
            version: ENGINE_VERSION.to_string(),
            kind: "PRODUCT_RUST_DETERMINISTIC_RULE_ENGINE".to_string(),
            cold_start_ms: measured_cold_ms,
            peak_rss_mi_b: 0.0,
            artifact_bytes,
            network_used: false,
            persistent_cache_used: false,
        },
        cases: predictions,
    };
    let predictions_bytes = serde_json::to_vec_pretty(&predictions_document)
        .expect("predictions serialization must succeed");
    let semantic_bytes =
        serde_json::to_vec(&semantics).expect("semantic serialization must succeed");
    write_new(&predictions_path, &predictions_bytes);
    write_new(&semantic_path, &semantic_bytes);
    println!(
        "PRODUCT_ENGINE_BENCHMARK_WRITER_OK cases=120 semantic_sha256={}",
        sha256_hex(&semantic_bytes)
    );
}
