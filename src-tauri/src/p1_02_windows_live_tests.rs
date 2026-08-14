use crate::{
    p0_02_windows_live_tests::run_p1_02_live_target_bound_terminology_apply,
    settings::{load_from_path, save_to_path, AppSettings, RewriteMode},
    terminology::{
        EntryMatchMode, EntryStatus, EntryType, LanguageScope, TerminologyEntryDraft,
        TerminologyStoreV1, GENERAL_PROFILE_ID, GLOBAL_PROFILE_ID,
    },
    terminology_import_export::{export_csv, export_json, ImportFormat},
    terminology_matcher::{match_terminology, MatchContext},
    terminology_service::{
        EntryQuery, EntrySort, RuntimeRecoveryCode, TerminologyRuntime, TerminologyRuntimeSnapshot,
    },
    terminology_validation::{
        validate_result, SuggestionReason, TerminologySuggestion, WarningCode,
    },
};
use std::fs;

fn live_draft(
    profile_id: &str,
    entry_type: EntryType,
    status: EntryStatus,
    source: &str,
    preferred: Option<&str>,
    source_language: LanguageScope,
    target_language: LanguageScope,
) -> TerminologyEntryDraft {
    TerminologyEntryDraft {
        profile_id: profile_id.to_string(),
        entry_type,
        status,
        source_text: source.to_string(),
        preferred_text: preferred.map(str::to_string),
        source_language,
        target_language,
        aliases: Vec::new(),
        match_mode: EntryMatchMode::WholePhrase,
        case_sensitive: false,
        priority: 100,
        usage_count: 0,
        occurrence_count: 0,
        note: None,
    }
}

#[test]
#[ignore = "requires bounded Windows local-store acceptance"]
fn p1_02_windows_live_store_profile_suggestion_and_import_restart_acceptance() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-02-live-store-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("owned live root should be created");
    let path = root.join("terminology.v1.json");
    let mut runtime = TerminologyRuntime::open(path.clone(), 1_000);
    let initial = runtime.store().expect("fresh runtime should be ready");
    assert_eq!(initial.profiles.len(), 2);
    assert!(initial
        .profiles
        .iter()
        .any(|profile| profile.id == GLOBAL_PROFILE_ID));
    assert!(initial
        .profiles
        .iter()
        .any(|profile| profile.id == GENERAL_PROFILE_ID));

    runtime
        .add_profile("maritime".to_string(), "Maritime".to_string(), 1_010)
        .expect("maritime profile should persist");
    runtime
        .validate_active_profile("maritime")
        .expect("maritime profile should be selectable");
    let settings_path = root.join("settings.json");
    let mut persisted_settings = AppSettings::default();
    persisted_settings.terminology.active_profile_id = "maritime".to_string();
    save_to_path(&settings_path, &persisted_settings)
        .expect("active terminology profile should persist in settings");
    assert_eq!(
        load_from_path(&settings_path)
            .expect("settings should reopen")
            .settings
            .terminology
            .active_profile_id,
        "maritime"
    );
    runtime
        .add_entry(
            "entry-live-translation".to_string(),
            live_draft(
                "maritime",
                EntryType::Translation,
                EntryStatus::Approved,
                "자유수면효과",
                Some("free surface effect"),
                LanguageScope::Ko,
                LanguageScope::En,
            ),
            1_020,
        )
        .expect("translation should persist");
    runtime
        .add_entry(
            "entry-live-protected".to_string(),
            live_draft(
                GLOBAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "CargoMax",
                None,
                LanguageScope::Any,
                LanguageScope::Any,
            ),
            1_030,
        )
        .expect("protected entry should persist");
    runtime
        .add_entry(
            "entry-live-preferred".to_string(),
            live_draft(
                "maritime",
                EntryType::Preferred,
                EntryStatus::Approved,
                "실시하다",
                Some("수행하다"),
                LanguageScope::Ko,
                LanguageScope::Ko,
            ),
            1_040,
        )
        .expect("preferred entry should persist");
    let mut edited = runtime
        .store()
        .expect("runtime should be ready")
        .entries
        .iter()
        .find(|entry| entry.id == "entry-live-preferred")
        .map(|entry| {
            live_draft(
                &entry.profile_id,
                entry.entry_type,
                entry.status,
                &entry.source_text,
                entry.preferred_text.as_deref(),
                entry.source_language,
                entry.target_language,
            )
        })
        .expect("preferred entry should exist");
    edited.priority = 600;
    runtime
        .update_entry("entry-live-preferred", edited, 1_050)
        .expect("entry edit should persist");
    runtime
        .set_entry_status("entry-live-preferred", EntryStatus::Disabled, 1_060)
        .expect("disable should persist");
    assert_eq!(
        runtime
            .query_entries(&EntryQuery {
                query: Some("수행".to_string()),
                profile_id: Some("maritime".to_string()),
                entry_type: Some(EntryType::Preferred),
                status: Some(EntryStatus::Disabled),
                source_language: Some(LanguageScope::Ko),
                target_language: Some(LanguageScope::Ko),
                sort: EntrySort::Priority,
            })
            .expect("offline query should succeed")
            .len(),
        1
    );
    runtime
        .set_entry_status("entry-live-preferred", EntryStatus::Approved, 1_070)
        .expect("re-enable should persist");

    let suggestion = TerminologySuggestion {
        entry_type: EntryType::Preferred,
        source_text: "synthetic candidate".to_string(),
        preferred_text: "synthetic preference".to_string(),
        source_language: LanguageScope::En,
        target_language: LanguageScope::En,
        reason: SuggestionReason::PreferredExpression,
    };
    let before_suggestion_count = runtime
        .store()
        .expect("runtime should be ready")
        .entries
        .len();
    assert_eq!(
        runtime
            .store()
            .expect("runtime should be ready")
            .entries
            .len(),
        before_suggestion_count
    );
    runtime
        .save_suggestion(
            "entry-live-suggestion".to_string(),
            "maritime".to_string(),
            suggestion,
            1_080,
        )
        .expect("explicit save should persist suggestion");
    let grammar_context = MatchContext::new(
        RewriteMode::Grammar,
        None,
        Some(LanguageScope::En),
        "maritime".to_string(),
    );
    assert!(match_terminology(
        runtime.store().expect("runtime should be ready"),
        "synthetic candidate",
        &grammar_context,
    )
    .matches
    .is_empty());
    runtime
        .set_entry_status("entry-live-suggestion", EntryStatus::Approved, 1_090)
        .expect("explicit approval should persist");
    assert_eq!(
        match_terminology(
            runtime.store().expect("runtime should be ready"),
            "synthetic candidate",
            &grammar_context,
        )
        .matches
        .len(),
        1
    );

    let json = runtime
        .export(ImportFormat::Json)
        .expect("JSON should export");
    let csv = runtime
        .export(ImportFormat::Csv)
        .expect("CSV should export");
    assert_eq!(
        json,
        export_json(runtime.store().expect("runtime should be ready"))
            .expect("JSON should be deterministic")
    );
    assert_eq!(
        csv,
        export_csv(runtime.store().expect("runtime should be ready"))
            .expect("CSV should be deterministic")
    );
    let preview = runtime
        .dry_run_import(ImportFormat::Csv, &csv, 1_100)
        .expect("CSV duplicate dry run should succeed");
    assert_eq!(preview.report.new_entries, 0);
    assert!(preview.report.identical_duplicates >= 4);
    runtime
        .apply_import(&preview.plan_id, 1_101)
        .expect("duplicate-only plan should be safe");

    let mut incoming = TerminologyStoreV1::new(1_110);
    incoming
        .add_profile("maritime".to_string(), "Maritime".to_string(), 1_111)
        .expect("incoming matching profile should save");
    incoming
        .add_profile(
            "profile-live-import".to_string(),
            "Live Import".to_string(),
            1_112,
        )
        .expect("incoming new profile should save");
    incoming
        .add_entry(
            "entry-live-import-conflict".to_string(),
            live_draft(
                "maritime",
                EntryType::Translation,
                EntryStatus::Approved,
                "자유수면효과",
                Some("conflicting synthetic preference"),
                LanguageScope::Ko,
                LanguageScope::En,
            ),
            1_113,
        )
        .expect("incoming semantic conflict should be valid alone");
    incoming
        .add_entry(
            "entry-live-imported".to_string(),
            live_draft(
                "profile-live-import",
                EntryType::Preferred,
                EntryStatus::Approved,
                "imported synthetic fixture",
                Some("imported preferred fixture"),
                LanguageScope::En,
                LanguageScope::En,
            ),
            1_114,
        )
        .expect("incoming non-conflicting entry should save");
    let revision_before_import = runtime.store().expect("runtime should be ready").revision;
    let import_preview = runtime
        .dry_run_import(
            ImportFormat::Json,
            &export_json(&incoming).expect("incoming JSON should export"),
            1_120,
        )
        .expect("JSON conflict-safe dry run should succeed");
    assert_eq!(
        runtime.store().expect("dry run must not mutate").revision,
        revision_before_import
    );
    assert_eq!(import_preview.report.new_profiles, 1);
    assert_eq!(import_preview.report.new_entries, 1);
    assert_eq!(import_preview.report.semantic_key_conflicts, 1);
    runtime
        .apply_import(&import_preview.plan_id, 1_121)
        .expect("valid non-conflicting import rows should persist");

    let reopened = TerminologyRuntime::open(path.clone(), 1_200);
    assert!(reopened
        .store()
        .expect("restart should reload store")
        .profiles
        .iter()
        .any(|profile| profile.id == "maritime"));
    assert!(reopened
        .store()
        .expect("restart should reload imported data")
        .entries
        .iter()
        .any(|entry| entry.id == "entry-live-imported"));
    assert!(!reopened
        .store()
        .expect("restart should keep conflicts skipped")
        .entries
        .iter()
        .any(|entry| entry.id == "entry-live-import-conflict"));
    drop(reopened);
    fs::write(&path, b"{synthetic-corrupt-main")
        .expect("owned main should be corrupted for recovery test");
    let recovered = TerminologyRuntime::open(path.clone(), 1_300);
    assert!(matches!(
        recovered.snapshot(),
        TerminologyRuntimeSnapshot::Ready {
            recovery: Some(RuntimeRecoveryCode::BackupRecovered),
            ..
        }
    ));
    drop(recovered);
    fs::remove_dir_all(root).expect("owned live root should be removed");
}

#[test]
#[ignore = "requires an interactive Windows desktop and target-bound SendInput acceptance"]
fn p1_02_windows_live_matched_validation_and_target_bound_apply_acceptance() {
    let mut store = TerminologyStoreV1::new(2_000);
    store
        .add_profile("maritime".to_string(), "Maritime".to_string(), 2_001)
        .expect("profile should save");
    store
        .add_entry(
            "entry-live-translation".to_string(),
            live_draft(
                "maritime",
                EntryType::Translation,
                EntryStatus::Approved,
                "자유수면효과",
                Some("free surface effect"),
                LanguageScope::Ko,
                LanguageScope::En,
            ),
            2_010,
        )
        .expect("translation should save");
    store
        .add_entry(
            "entry-live-protected".to_string(),
            live_draft(
                GLOBAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "CargoMax",
                None,
                LanguageScope::Any,
                LanguageScope::Any,
            ),
            2_020,
        )
        .expect("protected entry should save");
    for (id, status, source) in [
        (
            "entry-live-unmatched",
            EntryStatus::Approved,
            "unmatched sentinel",
        ),
        (
            "entry-live-suggested",
            EntryStatus::Suggested,
            "suggested sentinel",
        ),
        (
            "entry-live-disabled",
            EntryStatus::Disabled,
            "disabled sentinel",
        ),
    ] {
        store
            .add_entry(
                id.to_string(),
                live_draft(
                    "maritime",
                    EntryType::Preferred,
                    status,
                    source,
                    Some("inert fixture"),
                    LanguageScope::Any,
                    LanguageScope::Any,
                ),
                2_030,
            )
            .expect("inert entry should save");
    }
    let context = MatchContext::new(
        RewriteMode::Translate,
        Some(LanguageScope::En),
        Some(LanguageScope::Ko),
        "maritime".to_string(),
    );
    let matched = match_terminology(&store, "자유수면효과 CargoMax", &context);
    assert_eq!(matched.matches.len(), 2);
    assert!(validate_result(
        "free surface effect CargoMax",
        &matched,
        &[
            "entry-live-translation".to_string(),
            "entry-live-protected".to_string(),
        ]
    )
    .is_empty());
    let warnings = validate_result("nonpreferred deterministic output", &matched, &[]);
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::ProtectedMissing));
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::PreferredMissing));

    run_p1_02_live_target_bound_terminology_apply();
}
