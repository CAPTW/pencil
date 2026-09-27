//! Step 7: what the selected synthetic Provider actually receives at the process
//! boundary through the production manager and executor, and that failures and
//! cancellation never fall back to another Provider or replay the request.
use super::*;
use crate::settings::RewriteMode;
use crate::terminology::{
    EntryMatchMode, EntryStatus, EntryType, LanguageScope, TerminologyEntryDraft,
    TerminologyStoreV1, GENERAL_PROFILE_ID,
};
use crate::terminology_matcher::{constraints, match_terminology, MatchContext};

const SOURCE: &str = "Synthetic CargoMax note about the fixture tank, a suggested phrase and a disabled phrase.";

fn entry(
    entry_type: EntryType,
    status: EntryStatus,
    source: &str,
    preferred: Option<&str>,
) -> TerminologyEntryDraft {
    TerminologyEntryDraft {
        profile_id: GENERAL_PROFILE_ID.to_string(),
        entry_type,
        status,
        source_text: source.to_string(),
        preferred_text: preferred.map(str::to_string),
        source_language: LanguageScope::Any,
        target_language: LanguageScope::Any,
        aliases: Vec::new(),
        match_mode: EntryMatchMode::WholePhrase,
        case_sensitive: false,
        priority: 100,
        usage_count: 0,
        occurrence_count: 0,
        note: None,
    }
}

/// Production matching over a store that also holds entries that must never be sent.
fn request_constraints() -> Vec<crate::terminology_matcher::TerminologyConstraint> {
    let mut store = TerminologyStoreV1::new(1);
    for (id, entry_type, status, source, preferred) in [
        ("matched-protected", EntryType::Protected, EntryStatus::Approved, "CargoMax", None),
        ("matched-preferred", EntryType::Preferred, EntryStatus::Approved, "fixture tank", Some("FIXTURE_PREFERRED_TANK")),
        ("unmatched-approved", EntryType::Preferred, EntryStatus::Approved, "absent phrase", Some("UNMATCHED_SENTINEL")),
        ("matched-suggested", EntryType::Preferred, EntryStatus::Suggested, "suggested phrase", Some("SUGGESTED_SENTINEL")),
        ("matched-disabled", EntryType::Preferred, EntryStatus::Disabled, "disabled phrase", Some("DISABLED_SENTINEL")),
    ] {
        store
            .add_entry(id.to_string(), entry(entry_type, status, source, preferred), 2)
            .expect("synthetic entry");
    }
    let matches = match_terminology(
        &store,
        SOURCE,
        &MatchContext::new(RewriteMode::Grammar, None, Some(LanguageScope::En), GENERAL_PROFILE_ID.to_string()),
    );
    constraints(&matches.matches)
}

fn spawns(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("spawns.log"))
        .map(|log| log.lines().filter(|line| !line.is_empty()).count())
        .unwrap_or(0)
}

fn recorded_argv(dir: &Path) -> Vec<String> {
    serde_json::from_slice(&std::fs::read(dir.join("argv.json")).expect("fixture argv"))
        .expect("fixture argv is JSON")
}

/// Every Provider binary resolves to the same synthetic fixture, so any
/// fallback or replay would appear as an extra recorded spawn.
fn route_all_providers(exe: &Path) {
    for key in ["CODEX_PENCIL_CLAUDE_BIN", "CODEX_PENCIL_CODEX_BIN", "CODEX_PENCIL_AGY_BIN"] {
        std::env::set_var(key, exe);
    }
}

/// Removes the routes even when an assertion unwinds, so later serial tests
/// never inherit them.
struct ProviderRoutes;

impl Drop for ProviderRoutes {
    fn drop(&mut self) {
        for key in ["CODEX_PENCIL_CLAUDE_BIN", "CODEX_PENCIL_CODEX_BIN", "CODEX_PENCIL_AGY_BIN"] {
            std::env::remove_var(key);
        }
    }
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_deep_boundary_selected_provider_matched_terms_no_fallback_or_replay() {
    let evidence = PathBuf::from(std::env::var("MISSION_EVIDENCE").expect("task-owned evidence"));
    let request = request_constraints();
    let mut checks = Vec::new();
    let _routes = ProviderRoutes;

    // 1. Success: exactly one spawn of the selected Provider with privacy flags,
    //    carrying the selected text and only matched approved terminology.
    let (exe, dir) = fixture("success");
    route_all_providers(&exe);
    let state = AppState::default();
    state.providers.lock().await.set_active(ProviderKind::Claude).unwrap();
    let result = ProviderManager::rewrite(
        &state.providers,
        ProviderKind::Claude,
        "boundary-success".into(),
        SOURCE,
        RewriteIntent::grammar(),
        &request,
        &state.codex,
    )
    .await
    .expect("synthetic selected Provider succeeds");
    assert_eq!(result.provider_used, ProviderKind::Claude);
    assert_eq!(spawns(&dir), 1);
    let argv = recorded_argv(&dir);
    assert_eq!(argv.first().map(String::as_str), Some("-p"));
    for flag in ["--no-session-persistence", "--bare", "--disable-slash-commands"] {
        assert!(argv.iter().any(|arg| arg == flag), "{flag}");
    }
    assert!(argv.windows(2).any(|pair| pair[0] == "--disallowedTools" && pair[1] == "*"));
    let prompt = &argv[1];
    assert!(prompt.contains("CargoMax") && prompt.contains("FIXTURE_PREFERRED_TANK"));
    for sentinel in ["UNMATCHED_SENTINEL", "SUGGESTED_SENTINEL", "DISABLED_SENTINEL", "absent phrase"] {
        assert!(!prompt.contains(sentinel), "{sentinel} must not be sent");
    }
    assert_eq!(prompt.matches("Synthetic CargoMax note").count(), 1, "selected text sent exactly once");
    assert_exited(&dir);
    checks.push("selected_provider_only_privacy_flags_selected_text_and_matched_approved_terms");

    // 2. A non-selected Provider request is refused before any process starts.
    let refused = ProviderManager::rewrite(
        &state.providers,
        ProviderKind::Codex,
        "boundary-fallback".into(),
        SOURCE,
        RewriteIntent::grammar(),
        &request,
        &state.codex,
    )
    .await;
    assert!(matches!(refused, Err(provider::types::ProviderError::SilentFallbackRejected)));
    assert_eq!(spawns(&dir), 1);
    checks.push("non_selected_provider_refused_without_spawn");

    // 3. A failing request is neither retried nor sent to another Provider.
    let (exe, dir) = fixture("invalid-utf8");
    route_all_providers(&exe);
    let failed = ProviderManager::rewrite(
        &state.providers,
        ProviderKind::Claude,
        "boundary-failure".into(),
        SOURCE,
        RewriteIntent::grammar(),
        &request,
        &state.codex,
    )
    .await;
    assert!(failed.is_err());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(spawns(&dir), 1, "no replay or fallback after a failure");
    assert!(state.providers.lock().await.snapshot().busy_kind.is_none());
    assert_exited(&dir);
    checks.push("failure_not_replayed_or_fallen_back");

    // 4. Cancellation ends the one process; nothing is re-sent afterwards.
    let (exe, dir) = fixture("slow");
    route_all_providers(&exe);
    let shared = Arc::new(state);
    let worker = shared.clone();
    let work = tokio::spawn(async move {
        ProviderManager::rewrite(
            &worker.providers,
            ProviderKind::Claude,
            "boundary-cancel".into(),
            SOURCE,
            RewriteIntent::grammar(),
            &[],
            &worker.codex,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !dir.join("root.pid").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("synthetic Provider started");
    cancel_cli_operation(&shared).await.expect("active operation");
    assert!(matches!(work.await.unwrap(), Err(provider::types::ProviderError::Cancelled)));
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(spawns(&dir), 1, "no replay after cancellation");
    assert_exited(&dir);
    checks.push("cancel_terminates_once_without_replay");

    std::fs::write(
        evidence.join("deep-boundary-receipt.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "classification": "NATIVE_SYNTHETIC_PROVIDER_BOUNDARY",
            "provider_live": false,
            "selected_provider": "claude",
            "constraints_sent": request.len(),
            "checks": checks,
        }))
        .unwrap(),
    )
    .unwrap();
    eprintln!("DEEP_BOUNDARY_PASS {}", checks.join(","));
}
