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

pub(crate) fn gemma_configured() -> bool {
    std::env::var_os("GRAMMAR_GEMMA_PIPE").is_some()
        || std::env::var_os("GRAMMAR_GEMMA_BROKER_SHA256").is_some()
}

pub(crate) fn local_identity(session: &str, generation: u64, terminology: u64, source: &str, gemma: bool) -> SourceIdentity {
    SourceIdentity::from_source(session, generation, 1, terminology, AnalysisMode::Correction,
        if gemma { "gemma-local" } else { ENGINE_ID },
        if gemma { "12b-q4_0-v1" } else { ENGINE_VERSION }, source)
}

// A conservative contiguous edit envelope must not touch a protected term.
// Separate edits surrounding a protected term may therefore be refused.
fn validate_local_replacement(source: &str, replacement: &str, spans: &[ProtectedSpan]) -> Result<(), String> {
    if !crate::rewrite_preserves_line_structure(source, replacement) || replacement.trim().is_empty() || replacement.len() > 8192 || replacement.contains('\0') {
        return Err("gemma_invalid_replacement".into());
    }
    let prefix: usize = source.chars().zip(replacement.chars()).take_while(|(a,b)| a==b).map(|(c,_)|c.len_utf8()).sum();
    let suffix: usize = source[prefix..].chars().rev().zip(replacement[prefix..].chars().rev())
        .take_while(|(a,b)|a==b).map(|(c,_)|c.len_utf8()).sum();
    for span in spans {
        let range = codex_pencil::instant_selection::utf16_range_to_byte_range(source, span.range)
            .map_err(|_| "gemma_invalid_protected_span")?;
        if source != replacement && prefix <= range.end && source.len()-suffix >= range.start {
            return Err("gemma_protected_edit".into());
        }
    }
    Ok(())
}

pub(crate) async fn analyze_gemma_source(identity: SourceIdentity, source: String, spans: Vec<ProtectedSpan>) -> Result<InstantRuntimeOutcome, String> {
    use codex_pencil::local_gemma::{broker::Connection, peer::ImagePin, Binding, Intent};
    let started = Instant::now();
    // One operational budget, separate from the 1s recommendation performance target.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let name = std::env::var("GRAMMAR_GEMMA_PIPE").map_err(|_| "gemma_launcher_config")?;
    let hash = std::env::var("GRAMMAR_GEMMA_BROKER_SHA256").map_err(|_| "gemma_launcher_config")?;
    let image = std::env::current_exe().map_err(|_| "gemma_app_path")?
        .parent().ok_or("gemma_app_path")?.join("grammar-gemma-broker.exe");
    // The blocking pin keeps its permit even if this async caller is cancelled.
    // Repeated recapture cannot accumulate unbounded background hash readers.
    static PIN_GATE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    let permit = PIN_GATE.try_acquire().map_err(|_| "gemma_image_busy")?;
    let broker_image = tokio::time::timeout_at(deadline, spawn_blocking(move || {
        let _permit = permit;
        ImagePin::verify(&image, &hash)
    })).await.map_err(|_| "gemma_image_deadline")?
        .map_err(|_| "gemma_image_join")??;
    let connection = Connection { name, broker_image };
    let binding = Binding {intent:Intent::Grammar,epoch:identity.session_id.clone(),revision:identity.generation,source:source.clone()};
    let result = connection.acquire(&binding.epoch, deadline).await?.recommend(binding.clone(), deadline).await?;
    if !result.matches(&binding) { return Err("gemma_source_mismatch".into()); }
    validate_local_replacement(&source, &result.replacement, &spans)?;
    let no_change = result.replacement == source;
    let candidate = Candidate::new(identity, CandidateOrigin::Instant, "gemma-grammar", result.replacement);
    Ok(InstantRuntimeOutcome {no_change,suggestion_count:usize::from(!no_change),candidate,elapsed_ms:started.elapsed().as_millis()})
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
        analyzer_id: outcome.candidate.source_identity.analyzer_id.clone(),
        analyzer_version: outcome.candidate.source_identity.analyzer_version.clone(),
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
    fn gemma_protected_terms_and_unicode_boundaries_fail_closed() {
        let source = "😀 Term has error.";
        let protected = vec![ProtectedSpan::new(Utf16Range::new(3, 7), "term")];
        assert!(validate_local_replacement(source, "😀 Term has errors.", &protected).is_ok());
        assert!(validate_local_replacement(source, "😀 Word has error.", &protected).is_err());
        assert!(validate_local_replacement(source, "", &protected).is_err());
        assert!(validate_local_replacement(source, source, &[ProtectedSpan::new(Utf16Range::new(1,2), "split")]).is_err());
    }

    #[test]
    fn gemma_identity_is_distinct_and_invalidated_result_cannot_reactivate() {
        let identity = local_identity("capture", 2, 3, "I has a pen.", true);
        assert_eq!(identity.analyzer_id, "gemma-local");
        let mut cache = SessionCandidateCache::new();
        cache.activate(identity.clone());
        cache.invalidate(InstantInvalidationReason::ModeChange);
        assert!(cache.store(Candidate::new(identity, CandidateOrigin::Instant, "gemma", "I have a pen.")).is_err());
    }

    #[tokio::test]
    #[ignore = "explicit owned real-model desktop analysis trial"]
    async fn owned_desktop_gemma_analysis() {
        use tokio::io::{AsyncBufReadExt, BufReader};
        assert_eq!(std::env::var("GRAMMAR_GEMMA_TRIAL").as_deref(), Ok("owned-synthetic"));
        let root = std::path::Path::new("D:/dev/grammar-evidence/grammar-launcher-20261008");
        let app = std::env::current_exe().unwrap();
        let broker = app.parent().unwrap().join("grammar-gemma-broker.exe");
        let hash = |p: &std::path::Path| codex_pencil::instant_selection::sha256_hex(&std::fs::read(p).unwrap());
        let host = root.join("grammar-chromium-host.exe");
        let model = std::env::var_os("GRAMMAR_GEMMA_DESKTOP_TRIAL_MODEL").map(std::path::PathBuf::from)
            .unwrap_or_else(||root.join("local/gemma-4-12b-it-qat-q4_0.gguf"));
        let report = std::path::PathBuf::from(std::env::var("GRAMMAR_GEMMA_DESKTOP_TRIAL_RECEIPT").unwrap());
        assert!(!report.exists());
        let diagnostics = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(report.with_extension("broker.log")).unwrap();
        let startup = Instant::now();
        let mut child = tokio::process::Command::new(&broker)
            .creation_flags(0x08000000)
            .args([root.join("local/runtime/llama-server.exe").to_str().unwrap(),
                "a812655fed77b9f98d918e7aa0b1552403b76dacbeb9fa24c17dac9ed3d08ec1",
                model.to_str().unwrap(),
                "93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b",
                host.to_str().unwrap(), &hash(&host),
                "785601b8f32ba6ab10374f8e9ead5d8deadd56e70940883ef65608dbfbb80860",
                app.to_str().unwrap(), &hash(&app)])
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::from(diagnostics)).kill_on_drop(true).spawn().unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        let readiness = tokio::time::timeout(std::time::Duration::from_secs(190), reader.read_line(&mut line)).await;
        if !matches!(readiness, Ok(Ok(n)) if n > 0) {
            drop(child.stdin.take());
            let exited = tokio::time::timeout(std::time::Duration::from_secs(12), child.wait()).await;
            let forced = exited.is_err();
            if forced { child.kill().await.unwrap(); }
            std::fs::write(&report, serde_json::to_vec_pretty(&serde_json::json!({
                "scope":"REAL_MODEL_DESKTOP_STARTUP_FAILED", "userRequest":"NOT_RUN",
                "startupWarmup":"INSPECT_PHASE_DIAGNOSTICS",
                "readiness":false,"forcedBrokerTermination":forced,"diagnostics":report.with_extension("broker.log")
            })).unwrap()).unwrap();
            panic!("Owned broker never became ready; inspect the preserved startup receipt and diagnostics");
        }
        let startup_ms = startup.elapsed().as_millis();
        let ready: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(ready["pid"].as_u64().unwrap(), child.id().unwrap() as u64);
        std::env::set_var("GRAMMAR_GEMMA_PIPE", ready["name"].as_str().unwrap());
        std::env::set_var("GRAMMAR_GEMMA_BROKER_SHA256", hash(&broker));
        let mut rows=Vec::new();
        for (i,(source,expected)) in [("She go to school every day.","She goes to school every day."),
            ("They has two books.","They have two books."),("She go to school every day.","She goes to school every day.")].iter().enumerate() {
            let started = Instant::now();
            let result = analyze_gemma_source(local_identity("owned-desktop",i as u64+1,1,source,true), (*source).into(), vec![]).await;
            let elapsed=started.elapsed().as_millis();
            let passed=result.as_ref().is_ok_and(|r|r.candidate.text==*expected);
            rows.push(serde_json::json!({"index":i,"elapsedMs":elapsed,"targetMet":elapsed<=1000,"qualityPassed":passed,
                "result":result.as_ref().map(|r| &r.candidate.text).map_err(|e|e.as_str())}));
            if result.is_err() { break; }
        }
        std::env::remove_var("GRAMMAR_GEMMA_PIPE");
        std::env::remove_var("GRAMMAR_GEMMA_BROKER_SHA256");
        drop(child.stdin.take());
        line.clear();
        tokio::time::timeout(std::time::Duration::from_secs(15), reader.read_line(&mut line)).await.unwrap().unwrap();
        let exit = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await.unwrap().unwrap();
        std::fs::write(&report, serde_json::to_vec_pretty(&serde_json::json!({
            "scope":"REAL_MODEL_DESKTOP_PRODUCTION_ANALYZER_NOT_WIDGET_APPLY",
            "startupMs":startup_ms,"modelPath":model,"targetMs":1000,"rows":rows,
            "cleanup":line.trim(),"exitSuccess":exit.success(),"brokerSha256":hash(&broker)
        })).unwrap()).unwrap();
        assert!(exit.success());
        assert_eq!(line.trim(), r#"{"cleanup":"complete"}"#);
        assert_eq!(rows.len(),3);
        assert!(rows.iter().all(|r|r["qualityPassed"]==true));
    }

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
