use crate::{
    apply_current_terminology_bound,
    apply_safety::{ApplyOutcome, ApplyPlatform, WaitStage},
    capture_session::{BoundRewriteIntent, CaptureSessionStore, TerminologyIntent, WindowTarget},
    codex_client::{parse_rewrite_result, rewrite_prompt_with_terminology},
    settings::{decode_settings, RewriteMode, SettingsRecoveryCode},
    terminology::{
        EntryMatchMode, EntryStatus, EntryType, LanguageScope, TerminologyEntryDraft,
        TerminologyError, TerminologyStoreV1, GENERAL_PROFILE_ID, GLOBAL_PROFILE_ID,
    },
    terminology_import_export::{
        apply_import_plan, dry_run_import, export_csv, export_json, ImportFormat,
    },
    terminology_matcher::{constraints, match_terminology, request_constraints_json, MatchContext},
    terminology_service::{EntryQuery, EntrySort, TerminologyRuntime, TerminologyRuntimeSnapshot},
    terminology_store::{
        load_store_from_path, save_store_to_path, save_store_to_path_with_failure,
        StoreFailurePoint, StoreRecovery,
    },
    terminology_validation::{validate_result, SuggestionReason, WarningCode},
    translation::{RewriteIntent, TranslationTargetLanguage},
};
use serde_json::Value;
use std::fs;

fn draft(
    profile_id: &str,
    entry_type: EntryType,
    status: EntryStatus,
    source: &str,
    preferred: Option<&str>,
) -> TerminologyEntryDraft {
    TerminologyEntryDraft {
        profile_id: profile_id.to_string(),
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

#[test]
fn new_store_has_only_reserved_profiles_and_serializes_no_history_fields() {
    let store = TerminologyStoreV1::new(1_000);

    assert_eq!(store.schema_version, 1);
    assert_eq!(store.revision, 0);
    assert_eq!(store.profiles.len(), 2);
    assert!(store
        .profiles
        .iter()
        .any(|profile| profile.id == GLOBAL_PROFILE_ID && profile.enabled));
    assert!(store
        .profiles
        .iter()
        .any(|profile| profile.id == GENERAL_PROFILE_ID && profile.enabled));
    assert!(store.entries.is_empty());

    let serialized = serde_json::to_string(&store).expect("store should serialize");
    for forbidden in [
        "selectedText",
        "replacement",
        "rawModelOutput",
        "clipboard",
        "prompt",
        "history",
        "account",
        "auth",
        "hwnd",
        "pid",
    ] {
        assert!(!serialized
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase()));
    }
}

#[test]
fn profile_mutations_are_revisioned_and_reserved_or_active_profiles_fail_closed() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_profile("profile-domain".to_string(), "  Domain  ".to_string(), 20)
        .expect("profile should be added");
    assert_eq!(store.revision, 1);
    assert_eq!(
        store
            .profiles
            .iter()
            .find(|profile| profile.id == "profile-domain")
            .map(|profile| profile.name.as_str()),
        Some("Domain")
    );

    assert_eq!(
        store.add_profile("profile-duplicate".to_string(), "domain".to_string(), 21),
        Err(TerminologyError::DuplicateProfileName)
    );
    assert_eq!(store.revision, 1);

    store
        .rename_profile("profile-domain", "Legal".to_string(), 22)
        .expect("ordinary profile should be renamed");
    assert_eq!(store.revision, 2);
    assert_eq!(
        store
            .profiles
            .iter()
            .find(|profile| profile.id == "profile-domain")
            .map(|profile| (profile.name.as_str(), profile.updated_at_ms)),
        Some(("Legal", 22))
    );
    store
        .set_profile_enabled("profile-domain", false, GENERAL_PROFILE_ID, 23)
        .expect("inactive profile should be disabled");
    assert_eq!(store.revision, 3);
    assert!(
        !store
            .profiles
            .iter()
            .find(|profile| profile.id == "profile-domain")
            .expect("profile should remain present")
            .enabled
    );
    store
        .set_profile_enabled("profile-domain", true, GENERAL_PROFILE_ID, 24)
        .expect("profile should be re-enabled");
    assert_eq!(store.revision, 4);
    assert!(
        store
            .profiles
            .iter()
            .find(|profile| profile.id == "profile-domain")
            .expect("profile should remain present")
            .enabled
    );

    assert_eq!(
        store.rename_profile(GLOBAL_PROFILE_ID, "Changed".to_string(), 25),
        Err(TerminologyError::ReservedProfile)
    );
    assert_eq!(
        store.set_profile_enabled(GENERAL_PROFILE_ID, false, GENERAL_PROFILE_ID, 26),
        Err(TerminologyError::ActiveProfileRequired)
    );
    assert_eq!(store.revision, 4);
}

#[test]
fn entry_validation_crud_and_explicit_approval_keep_suggestions_inert() {
    let mut store = TerminologyStoreV1::new(10);
    let mut suggested = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Suggested,
        "  synthetic source  ",
        Some(" synthetic target "),
    );
    suggested.aliases = vec![" synthetic alias ".to_string()];
    store
        .add_entry("entry-suggested".to_string(), suggested, 20)
        .expect("suggestion should save inertly");
    assert_eq!(store.revision, 1);
    let stored = store
        .entries
        .iter()
        .find(|entry| entry.id == "entry-suggested")
        .expect("entry should exist");
    assert_eq!(stored.source_text, "synthetic source");
    assert_eq!(stored.preferred_text.as_deref(), Some("synthetic target"));
    assert_eq!(stored.status, EntryStatus::Suggested);

    let context = MatchContext::new(
        RewriteMode::Translate,
        Some(LanguageScope::En),
        Some(LanguageScope::En),
        GENERAL_PROFILE_ID.to_string(),
    );
    assert!(match_terminology(&store, "synthetic source", &context)
        .matches
        .is_empty());

    store
        .set_entry_status("entry-suggested", EntryStatus::Approved, 30)
        .expect("explicit approval should succeed");
    assert_eq!(store.revision, 2);
    assert_eq!(
        match_terminology(&store, "synthetic source", &context)
            .matches
            .len(),
        1
    );

    store
        .set_entry_status("entry-suggested", EntryStatus::Disabled, 40)
        .expect("disable should succeed");
    assert!(match_terminology(&store, "synthetic source", &context)
        .matches
        .is_empty());
    store
        .delete_entry("entry-suggested")
        .expect("delete should succeed");
    assert!(store.entries.is_empty());
    assert_eq!(store.revision, 4);
}

#[test]
fn protected_entries_canonicalize_preferred_text_and_duplicate_aliases_are_rejected() {
    let mut store = TerminologyStoreV1::new(10);
    let protected = draft(
        GLOBAL_PROFILE_ID,
        EntryType::Protected,
        EntryStatus::Approved,
        "CargoFixture",
        Some("must be removed"),
    );
    store
        .add_entry("entry-protected".to_string(), protected, 20)
        .expect("protected entry should canonicalize");
    assert_eq!(store.entries[0].preferred_text, None);
    assert!(store.entries[0].case_sensitive);

    let mut duplicate_alias = draft(
        GENERAL_PROFILE_ID,
        EntryType::Preferred,
        EntryStatus::Approved,
        "term",
        Some("preferred"),
    );
    duplicate_alias.aliases = vec!["Alias".to_string(), " alias ".to_string()];
    assert_eq!(
        store.add_entry("entry-duplicate-alias".to_string(), duplicate_alias, 30),
        Err(TerminologyError::DuplicateAlias)
    );
}

#[test]
fn deserialized_store_rejects_noncanonical_outer_term_whitespace() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_entry(
            "entry-canonical".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "canonical fixture",
                Some("canonical preference"),
            ),
            20,
        )
        .expect("canonical entry should save");
    store.entries[0].source_text = " padded fixture ".to_string();

    assert_eq!(store.validate(), Err(TerminologyError::InvalidTerm));
}

#[test]
fn deserialized_store_rejects_profile_entry_id_collisions() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_entry(
            "entry-collision".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "collision fixture",
                Some("collision preference"),
            ),
            20,
        )
        .expect("entry fixture should save");
    store.entries[0].id = GENERAL_PROFILE_ID.to_string();

    assert_eq!(store.validate(), Err(TerminologyError::DuplicateId));
}

#[test]
fn matcher_normalizes_nfc_aliases_boundaries_case_and_korean_suffixes() {
    let mut store = TerminologyStoreV1::new(10);
    let mut cafe = draft(
        GENERAL_PROFILE_ID,
        EntryType::Preferred,
        EntryStatus::Approved,
        "Café term",
        Some("preferred café"),
    );
    cafe.aliases = vec!["cafe alias".to_string()];
    store
        .add_entry("entry-cafe".to_string(), cafe, 20)
        .expect("entry should save");
    store
        .add_entry(
            "entry-cat".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "cat",
                None,
            ),
            21,
        )
        .expect("entry should save");
    store
        .add_entry(
            "entry-korean".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "선박",
                None,
            ),
            22,
        )
        .expect("entry should save");

    let context = MatchContext::new(
        RewriteMode::Grammar,
        None,
        Some(LanguageScope::En),
        GENERAL_PROFILE_ID.to_string(),
    );
    let result = match_terminology(
        &store,
        "Cafe\u{301} term, CAFE ALIAS, concatenate, cat, 선박은 안전하다.",
        &context,
    );
    let ids = result
        .matches
        .iter()
        .map(|matched| matched.entry_id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"entry-cafe"));
    assert!(ids.contains(&"entry-cat"));
    assert!(ids.contains(&"entry-korean"));
    assert_eq!(ids.iter().filter(|id| **id == "entry-cat").count(), 1);
}

#[test]
fn matcher_uses_active_profile_language_longest_priority_and_stable_overlap_precedence() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_profile("profile-domain".to_string(), "Domain".to_string(), 11)
        .expect("profile should save");

    let mut global = draft(
        GLOBAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "surface effect",
        Some("global value"),
    );
    global.target_language = LanguageScope::Any;
    global.priority = 1_000;
    store
        .add_entry("entry-global".to_string(), global, 20)
        .expect("entry should save");

    let mut active = draft(
        "profile-domain",
        EntryType::Translation,
        EntryStatus::Approved,
        "surface effect",
        Some("active exact value"),
    );
    active.target_language = LanguageScope::En;
    active.priority = 10;
    store
        .add_entry("entry-active".to_string(), active, 21)
        .expect("entry should save");

    let mut longer = draft(
        "profile-domain",
        EntryType::Translation,
        EntryStatus::Approved,
        "free surface effect",
        Some("long value"),
    );
    longer.target_language = LanguageScope::En;
    store
        .add_entry("entry-long".to_string(), longer, 22)
        .expect("entry should save");

    let context = MatchContext::new(
        RewriteMode::Translate,
        Some(LanguageScope::En),
        Some(LanguageScope::Ko),
        "profile-domain".to_string(),
    );
    let result = match_terminology(&store, "free surface effect", &context);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].entry_id, "entry-long");

    let result = match_terminology(&store, "surface effect", &context);
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].entry_id, "entry-active");
}

#[test]
fn matcher_keeps_global_active_and_excludes_a_disabled_selected_profile() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_profile("profile-disabled".to_string(), "Disabled".to_string(), 11)
        .expect("profile should save");
    store
        .add_entry(
            "entry-global-only".to_string(),
            draft(
                GLOBAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "global fixture",
                None,
            ),
            20,
        )
        .expect("global entry should save");
    store
        .add_entry(
            "entry-disabled-profile".to_string(),
            draft(
                "profile-disabled",
                EntryType::Protected,
                EntryStatus::Approved,
                "profile fixture",
                None,
            ),
            21,
        )
        .expect("profile entry should save");
    store
        .set_profile_enabled("profile-disabled", false, GENERAL_PROFILE_ID, 22)
        .expect("non-active profile should disable");
    let result = match_terminology(
        &store,
        "global fixture profile fixture",
        &MatchContext::new(
            RewriteMode::Grammar,
            None,
            Some(LanguageScope::En),
            "profile-disabled".to_string(),
        ),
    );

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].entry_id, "entry-global-only");
}

#[test]
fn matcher_prefers_exact_language_then_stable_id_and_reports_conflict() {
    let mut store = TerminologyStoreV1::new(10);
    let mut any = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "direction fixture",
        Some("any preference"),
    );
    any.target_language = LanguageScope::Any;
    any.priority = 1_000;
    store
        .add_entry("entry-any".to_string(), any, 20)
        .expect("any-language entry should save");
    let mut exact_b = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "direction fixture",
        Some("exact preference b"),
    );
    exact_b.target_language = LanguageScope::En;
    exact_b.priority = 10;
    store
        .add_entry("entry-exact-b".to_string(), exact_b, 20)
        .expect("exact-language entry should save");
    let mut exact_a = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "direction fixture",
        Some("exact preference a"),
    );
    exact_a.target_language = LanguageScope::En;
    exact_a.priority = 10;
    store
        .add_entry("entry-exact-a".to_string(), exact_a, 20)
        .expect("stable-ID entry should save");
    let result = match_terminology(
        &store,
        "direction fixture",
        &MatchContext::new(
            RewriteMode::Translate,
            Some(LanguageScope::En),
            Some(LanguageScope::En),
            GENERAL_PROFILE_ID.to_string(),
        ),
    );

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].entry_id, "entry-exact-a");
    assert_eq!(result.conflicts.len(), 1);
    assert!(result.conflicts[0]
        .entry_ids
        .iter()
        .any(|entry_id| entry_id == "entry-any"));
}

#[test]
fn matcher_enforces_the_serialized_sixteen_kibibyte_bound_before_fifty_entries() {
    let mut store = TerminologyStoreV1::new(10);
    let mut selected = Vec::new();
    for index in 0..30 {
        let source = format!("fixture-{index:02}-{}", "s".repeat(220));
        let preferred = format!("preferred-{index:02}-{}", "p".repeat(490));
        store
            .add_entry(
                format!("entry-byte-bound-{index:02}"),
                draft(
                    GENERAL_PROFILE_ID,
                    EntryType::Preferred,
                    EntryStatus::Approved,
                    &source,
                    Some(&preferred),
                ),
                20 + index,
            )
            .expect("byte-bound entry should save");
        selected.push(source);
    }
    let result = match_terminology(
        &store,
        &selected.join(" "),
        &MatchContext::new(
            RewriteMode::Grammar,
            None,
            Some(LanguageScope::En),
            GENERAL_PROFILE_ID.to_string(),
        ),
    );

    assert!(result.matches.len() < 30);
    assert!(result.truncated);
    assert!(
        request_constraints_json(&result)
            .expect("bounded constraints should serialize")
            .len()
            <= 16 * 1024
    );
}

#[test]
fn matcher_bounds_conflict_identifiers_without_expanding_the_request_subset() {
    let mut store = TerminologyStoreV1::new(10);
    for index in 0..60 {
        store
            .add_entry(
                format!("entry-conflict-bound-{index:02}"),
                draft(
                    GENERAL_PROFILE_ID,
                    EntryType::Preferred,
                    EntryStatus::Approved,
                    "shared conflict fixture",
                    Some(&format!("preference-{index:02}")),
                ),
                20,
            )
            .expect("conflict-bound entry should save");
    }
    let result = match_terminology(
        &store,
        "shared conflict fixture",
        &MatchContext::new(
            RewriteMode::Grammar,
            None,
            Some(LanguageScope::En),
            GENERAL_PROFILE_ID.to_string(),
        ),
    );

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.conflicts.len(), 1);
    assert_eq!(result.conflicts[0].entry_ids.len(), 50);
    assert!(result.truncated);
}

#[test]
fn matcher_bounds_request_subset_and_excludes_unmatched_suggested_and_disabled_entries() {
    let mut store = TerminologyStoreV1::new(10);
    for index in 0..55 {
        let source = format!("fixture{index:02}");
        let mut entry = draft(
            GENERAL_PROFILE_ID,
            EntryType::Preferred,
            EntryStatus::Approved,
            &source,
            Some("bounded preferred value"),
        );
        entry.priority = 1_000 - index;
        store
            .add_entry(format!("entry-{index:02}"), entry, 100 + index as u64)
            .expect("bounded fixture should save");
    }
    store
        .add_entry(
            "entry-unmatched-sentinel".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "UNMATCHED_SENTINEL",
                None,
            ),
            500,
        )
        .expect("sentinel should save");
    store
        .add_entry(
            "entry-suggested".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Suggested,
                "suggested-sentinel",
                Some("ignored"),
            ),
            501,
        )
        .expect("suggested should save");
    store
        .add_entry(
            "entry-disabled".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Disabled,
                "disabled-sentinel",
                Some("ignored"),
            ),
            502,
        )
        .expect("disabled should save");

    let selected = (0..55)
        .map(|index| format!("fixture{index:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    let context = MatchContext::new(
        RewriteMode::Grammar,
        None,
        Some(LanguageScope::En),
        GENERAL_PROFILE_ID.to_string(),
    );
    let result = match_terminology(&store, &selected, &context);
    assert_eq!(result.matches.len(), 50);
    assert!(result.truncated);

    let request = request_constraints_json(&result).expect("request subset should serialize");
    assert!(request.len() <= 16 * 1024);
    let parsed: Value = serde_json::from_str(&request).expect("request JSON should parse");
    assert_eq!(parsed.as_array().map(Vec::len), Some(50));
    let ids = result
        .matches
        .iter()
        .map(|matched| matched.entry_id.as_str())
        .collect::<Vec<_>>();
    assert!(!ids.contains(&"entry-unmatched-sentinel"));
    assert!(!ids.contains(&"entry-suggested"));
    assert!(!ids.contains(&"entry-disabled"));
}

#[test]
fn local_result_validation_warns_without_mutating_noncompliant_output() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_entry(
            "entry-protected".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "CargoFixture",
                None,
            ),
            20,
        )
        .expect("protected entry should save");
    let mut translated = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "source fixture",
        Some("required target fixture"),
    );
    translated.target_language = LanguageScope::En;
    store
        .add_entry("entry-translation".to_string(), translated, 21)
        .expect("translation entry should save");
    let context = MatchContext::new(
        RewriteMode::Translate,
        Some(LanguageScope::En),
        Some(LanguageScope::En),
        GENERAL_PROFILE_ID.to_string(),
    );
    let matches = match_terminology(&store, "CargoFixture source fixture", &context);

    let warnings = validate_result("noncompliant synthetic result", &matches, &[]);
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::ProtectedMissing));
    assert!(warnings
        .iter()
        .any(|warning| warning.code == WarningCode::PreferredMissing));
    assert_eq!(
        "noncompliant synthetic result",
        "noncompliant synthetic result"
    );
}

#[test]
fn schema_two_settings_migrate_to_fixed_false_terminology_defaults() {
    let loaded = decode_settings(
        r#"{"schemaVersion":2,"mode":"translate","restoreClipboard":false,"autoRewrite":false,"shortcut":{"primary":{"modifiers":["CTRL","SHIFT"],"key":"H","display":"Ctrl+Shift+H"}},"translation":{"sourceLanguage":"auto","targetLanguage":"ja","applyFormat":"source_with_translation"}}"#,
    );

    assert_eq!(loaded.recovery, Some(SettingsRecoveryCode::Migrated));
    assert_eq!(loaded.settings.schema_version, 4);
    assert_eq!(loaded.settings.cloud_processing_acknowledgement_version, 0);
    assert_eq!(loaded.settings.shortcut.primary.key, "H");
    assert_eq!(
        loaded.settings.translation.target_language,
        TranslationTargetLanguage::Ja
    );
    assert!(loaded.settings.terminology.enabled);
    assert_eq!(
        loaded.settings.terminology.active_profile_id,
        GENERAL_PROFILE_ID
    );
    assert!(loaded.settings.terminology.use_approved_terminology);
    assert!(loaded.settings.terminology.suggest_terminology);
    assert!(!loaded.settings.terminology.auto_save_suggestions);
}

#[test]
fn invalid_true_auto_save_recovers_to_false_without_enabling_automatic_persistence() {
    let loaded = decode_settings(
        r#"{"schemaVersion":3,"mode":"grammar","restoreClipboard":true,"autoRewrite":true,"shortcut":{"primary":{"modifiers":["CTRL","SHIFT"],"key":"G","display":"Ctrl+Shift+G"}},"translation":{"sourceLanguage":"auto","targetLanguage":"en","applyFormat":"translation_only"},"terminology":{"enabled":true,"activeProfileId":"general","useApprovedTerminology":true,"suggestTerminology":true,"autoSaveSuggestions":true}}"#,
    );

    assert_eq!(loaded.recovery, Some(SettingsRecoveryCode::InvalidFields));
    assert!(!loaded.settings.terminology.auto_save_suggestions);
}

#[test]
fn store_writer_creates_backup_and_recovers_corrupt_main_without_exposing_content() {
    let root =
        std::env::temp_dir().join(format!("codex-pencil-p1-02-store-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("fixture root should be created");
    let path = root.join("terminology.v1.json");
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_entry(
            "entry-storage".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "storage fixture",
                Some("stored preference"),
            ),
            20,
        )
        .expect("entry should save");
    save_store_to_path(&path, &store).expect("store should persist");
    let backup = path.with_file_name("terminology.v1.json.bak");
    assert!(backup.exists());
    let previous = store.clone();
    store
        .add_profile("profile-newer".to_string(), "Newer".to_string(), 30)
        .expect("newer store should mutate");
    save_store_to_path(&path, &store).expect("newer store should persist");

    fs::write(&path, b"{synthetic-corrupt-main").expect("corruption fixture should write");
    let recovered = load_store_from_path(&path).expect("backup should recover");
    assert_eq!(recovered.recovery, Some(StoreRecovery::BackupRecovered));
    assert_eq!(recovered.store.revision, previous.revision);
    assert!(load_store_from_path(&path).is_ok());

    fs::remove_dir_all(root).expect("fixture root should be removed");
}

#[test]
fn every_injected_store_failure_preserves_a_valid_prior_or_recovery_file() {
    for failure in [
        StoreFailurePoint::TempWrite,
        StoreFailurePoint::TempSync,
        StoreFailurePoint::BackupStage,
        StoreFailurePoint::PromoteRename,
        StoreFailurePoint::Rollback,
    ] {
        let root = std::env::temp_dir().join(format!(
            "codex-pencil-p1-02-failure-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("fixture root should be created");
        let path = root.join("terminology.v1.json");
        let prior = TerminologyStoreV1::new(10);
        save_store_to_path(&path, &prior).expect("prior store should save");
        let mut next = prior.clone();
        next.add_profile("profile-next".to_string(), "Next".to_string(), 20)
            .expect("next store should mutate");

        assert!(save_store_to_path_with_failure(&path, &next, failure).is_err());
        let loaded = load_store_from_path(&path).expect("some valid recovery source must remain");
        assert!(loaded.store == prior || loaded.store == next);
        fs::remove_dir_all(root).expect("fixture root should be removed");
    }
}

#[test]
fn json_and_csv_round_trip_and_exports_exclude_private_runtime_fields() {
    let mut store = TerminologyStoreV1::new(10);
    let mut entry = draft(
        GENERAL_PROFILE_ID,
        EntryType::Translation,
        EntryStatus::Approved,
        "쉼표, 따옴표 \"fixture\"",
        Some("日本語 fixture, 中文 fixture"),
    );
    entry.aliases = vec!["별칭 fixture".to_string()];
    entry.note = Some("local note".to_string());
    store
        .add_entry("entry-export".to_string(), entry, 20)
        .expect("entry should save");

    let json = export_json(&store).expect("JSON export should succeed");
    let csv = export_csv(&store).expect("CSV export should succeed");
    assert!(csv.contains("\r\n"));
    for exported in [&json, &csv] {
        let lowered = exported.to_ascii_lowercase();
        for forbidden in [
            "selected_text",
            "replacement",
            "raw_model_output",
            "clipboard",
            "prompt_history",
            "account",
            "auth_token",
            "hwnd",
        ] {
            assert!(!lowered.contains(forbidden));
        }
    }

    for (format, text) in [(ImportFormat::Json, json), (ImportFormat::Csv, csv)] {
        let empty = TerminologyStoreV1::new(30);
        let plan = dry_run_import(&empty, format, &text, "plan-roundtrip".to_string(), 40)
            .expect("dry run should parse");
        assert_eq!(empty.revision, 0);
        assert_eq!(plan.report.new_entries, 1);
        let imported = apply_import_plan(&empty, &plan, 41).expect("plan should apply");
        assert_eq!(imported.entries.len(), 1);
        assert_eq!(
            imported.entries[0].source_text,
            store.entries[0].source_text
        );
    }
}

#[test]
fn import_dry_run_reports_duplicates_conflicts_and_rejects_stale_apply() {
    let mut store = TerminologyStoreV1::new(10);
    store
        .add_entry(
            "entry-existing".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "existing fixture",
                Some("preferred fixture"),
            ),
            20,
        )
        .expect("entry should save");
    let exported = export_json(&store).expect("export should succeed");
    let plan = dry_run_import(
        &store,
        ImportFormat::Json,
        &exported,
        "plan-duplicate".to_string(),
        30,
    )
    .expect("dry run should succeed");
    assert_eq!(plan.report.identical_duplicates, 1);
    assert_eq!(plan.report.new_entries, 0);
    assert_eq!(store.revision, 1);

    let mut changed = store.clone();
    changed
        .add_profile("profile-changed".to_string(), "Changed".to_string(), 31)
        .expect("store should change");
    assert_eq!(
        apply_import_plan(&changed, &plan, 32),
        Err(TerminologyError::StaleImportPlan)
    );
}

#[test]
fn csv_import_rejects_unknown_enum_malformed_quotes_and_oversized_input() {
    let mut exported = TerminologyStoreV1::new(10);
    exported
        .add_entry(
            "entry-csv-validation".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "csv validation fixture",
                Some("csv preferred fixture"),
            ),
            20,
        )
        .expect("CSV fixture should save");
    let csv = export_csv(&exported).expect("CSV fixture should export");
    let unknown_enum = csv.replacen(",preferred,approved,", ",unsupported,approved,", 1);
    let current = TerminologyStoreV1::new(30);

    assert_eq!(
        dry_run_import(
            &current,
            ImportFormat::Csv,
            &unknown_enum,
            "plan-unknown-enum".to_string(),
            40,
        ),
        Err(TerminologyError::ImportInvalid)
    );
    assert_eq!(
        dry_run_import(
            &current,
            ImportFormat::Csv,
            "profile_id,profile_name\r\n\"unterminated",
            "plan-malformed-csv".to_string(),
            40,
        ),
        Err(TerminologyError::ImportInvalid)
    );
    let bare_quote = csv.replacen("csv validation fixture", "csv\"validation fixture", 1);
    assert_eq!(
        dry_run_import(
            &current,
            ImportFormat::Csv,
            &bare_quote,
            "plan-bare-quote".to_string(),
            40,
        ),
        Err(TerminologyError::ImportInvalid)
    );
    let oversized = "x".repeat(2 * 1024 * 1024 + 1);
    assert_eq!(
        dry_run_import(
            &current,
            ImportFormat::Json,
            &oversized,
            "plan-oversized".to_string(),
            40,
        ),
        Err(TerminologyError::ImportTooLarge)
    );
}

#[test]
fn import_reports_semantic_conflicts_adds_new_profiles_and_expires_plans() {
    let mut current = TerminologyStoreV1::new(10);
    current
        .add_entry(
            "entry-existing-key".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "semantic fixture",
                Some("first preference"),
            ),
            20,
        )
        .expect("current entry should save");
    let mut incoming = TerminologyStoreV1::new(30);
    incoming
        .add_profile("profile-imported".to_string(), "Imported".to_string(), 31)
        .expect("incoming profile should save");
    incoming
        .add_entry(
            "entry-conflicting-key".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "semantic fixture",
                Some("second preference"),
            ),
            32,
        )
        .expect("incoming conflict should be valid alone");
    incoming
        .add_entry(
            "entry-new-profile".to_string(),
            draft(
                "profile-imported",
                EntryType::Translation,
                EntryStatus::Approved,
                "new profile fixture",
                Some("new translated fixture"),
            ),
            33,
        )
        .expect("incoming entry should save");
    let incoming_json = export_json(&incoming).expect("incoming JSON should export");
    let plan = dry_run_import(
        &current,
        ImportFormat::Json,
        &incoming_json,
        "plan-conflict-new-profile".to_string(),
        40,
    )
    .expect("conflict-safe dry run should succeed");

    assert_eq!(plan.report.semantic_key_conflicts, 1);
    assert_eq!(plan.report.new_profiles, 1);
    assert_eq!(plan.report.new_entries, 1);
    assert_eq!(current.entries.len(), 1);
    assert_eq!(
        apply_import_plan(&current, &plan, plan.expires_at_ms + 1),
        Err(TerminologyError::ImportPlanExpired)
    );
    let applied = apply_import_plan(&current, &plan, plan.expires_at_ms)
        .expect("unexpired non-conflicting data should apply");
    assert!(applied
        .profiles
        .iter()
        .any(|profile| profile.id == "profile-imported"));
    assert!(applied
        .entries
        .iter()
        .any(|entry| entry.id == "entry-new-profile"));
    assert!(!applied
        .entries
        .iter()
        .any(|entry| entry.id == "entry-conflicting-key"));
}

#[test]
fn import_profile_identity_conflict_blocks_its_entries_from_existing_profile() {
    let mut current = TerminologyStoreV1::new(10);
    current
        .add_profile(
            "profile-shared-id".to_string(),
            "Current Identity".to_string(),
            11,
        )
        .expect("current profile should save");
    let mut incoming = TerminologyStoreV1::new(20);
    incoming
        .add_profile(
            "profile-shared-id".to_string(),
            "Different Identity".to_string(),
            21,
        )
        .expect("incoming profile should be valid alone");
    incoming
        .add_entry(
            "entry-blocked-profile".to_string(),
            draft(
                "profile-shared-id",
                EntryType::Preferred,
                EntryStatus::Approved,
                "blocked profile fixture",
                Some("blocked preference fixture"),
            ),
            22,
        )
        .expect("incoming entry should be valid alone");
    let plan = dry_run_import(
        &current,
        ImportFormat::Json,
        &export_json(&incoming).expect("incoming JSON should export"),
        "plan-profile-identity-conflict".to_string(),
        30,
    )
    .expect("conflict should be reported without mutation");

    assert_eq!(plan.report.id_conflicts, 1);
    assert_eq!(plan.report.new_entries, 0);
    assert!(plan.report.skipped_rows >= 1);
    assert!(!plan
        .next_store
        .entries
        .iter()
        .any(|entry| entry.id == "entry-blocked-profile"));
}

#[test]
fn csv_semantic_conflict_reports_the_source_row_number() {
    let mut current = TerminologyStoreV1::new(10);
    current
        .add_entry(
            "entry-current-csv-conflict".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "csv conflict fixture",
                Some("current preference"),
            ),
            11,
        )
        .expect("current entry should save");
    let mut incoming = TerminologyStoreV1::new(20);
    incoming
        .add_entry(
            "entry-incoming-csv-conflict".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "csv conflict fixture",
                Some("incoming preference"),
            ),
            21,
        )
        .expect("incoming entry should save");
    let plan = dry_run_import(
        &current,
        ImportFormat::Csv,
        &export_csv(&incoming).expect("incoming CSV should export"),
        "plan-csv-row-conflict".to_string(),
        30,
    )
    .expect("CSV conflict should be reported");

    assert_eq!(plan.report.semantic_key_conflicts, 1);
    assert_eq!(plan.report.conflicts.len(), 1);
    assert_eq!(plan.report.conflicts[0].row_number, Some(2));
}

#[test]
fn runtime_import_persistence_failure_leaves_store_and_plan_recoverable() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-02-import-failure-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("fixture directory should exist");
    let path = root.join("terminology.v1.json");
    let mut runtime = TerminologyRuntime::open(path.clone(), 10);
    let mut incoming = TerminologyStoreV1::new(20);
    incoming
        .add_entry(
            "entry-import-failure".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Preferred,
                EntryStatus::Approved,
                "import failure fixture",
                Some("import preference fixture"),
            ),
            21,
        )
        .expect("incoming entry should save");
    let preview = runtime
        .dry_run_import(
            ImportFormat::Json,
            &export_json(&incoming).expect("incoming JSON should export"),
            30,
        )
        .expect("dry run should succeed");
    let before = runtime.store().expect("runtime should be ready").clone();

    assert_eq!(
        runtime.apply_import_with_failure(&preview.plan_id, 31, StoreFailurePoint::PromoteRename,),
        Err(TerminologyError::StoreIo)
    );
    assert_eq!(
        runtime.store().expect("runtime should remain ready"),
        &before
    );
    assert_eq!(
        runtime
            .apply_import(&preview.plan_id, 32)
            .expect("the retained plan should apply after the transient failure")
            .new_entries,
        1
    );
    assert!(TerminologyRuntime::open(path, 40)
        .store()
        .expect("persisted import should reopen")
        .entries
        .iter()
        .any(|entry| entry.id == "entry-import-failure"));
    fs::remove_dir_all(root).expect("fixture directory should be removed");
}

#[test]
fn unrecoverable_store_requires_explicit_import_or_reset_before_recovery() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-02-explicit-recovery-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("fixture directory should exist");
    let path = root.join("terminology.v1.json");
    let backup = path.with_file_name("terminology.v1.json.bak");
    fs::write(&path, b"{synthetic-invalid-main").expect("invalid main should be written");
    fs::write(&backup, b"{synthetic-invalid-backup").expect("invalid backup should be written");
    let mut runtime = TerminologyRuntime::open(path.clone(), 10);
    assert!(matches!(
        runtime.snapshot(),
        TerminologyRuntimeSnapshot::Unrecoverable { .. }
    ));
    let mut imported = TerminologyStoreV1::new(20);
    imported
        .add_entry(
            "entry-explicit-recovery".to_string(),
            draft(
                GENERAL_PROFILE_ID,
                EntryType::Protected,
                EntryStatus::Approved,
                "explicit recovery fixture",
                None,
            ),
            21,
        )
        .expect("recovery import should be valid");

    let preview = runtime
        .dry_run_import(
            ImportFormat::Json,
            &export_json(&imported).expect("recovery JSON should export"),
            30,
        )
        .expect("explicit dry run should be available while unrecoverable");
    runtime
        .apply_import(&preview.plan_id, 31)
        .expect("explicit import apply should recover the store");
    assert!(runtime
        .store()
        .expect("runtime should recover")
        .entries
        .iter()
        .any(|entry| entry.id == "entry-explicit-recovery"));
    assert!(TerminologyRuntime::open(path, 40).store().is_ok());
    fs::remove_dir_all(root).expect("fixture directory should be removed");
}

#[test]
fn rewrite_intent_binds_terminology_revision_profile_flags_and_matched_ids() {
    let mut capture = CaptureSessionStore::default();
    let token = capture
        .capture(
            "session-p1-02".to_string(),
            "synthetic source".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("capture should succeed");
    let bound = BoundRewriteIntent::new(
        RewriteIntent::grammar(),
        TerminologyIntent {
            enabled: true,
            use_approved_terminology: true,
            suggest_terminology: true,
            active_profile_id: GENERAL_PROFILE_ID.to_string(),
            store_revision: 7,
            matched_entry_ids: vec!["entry-a".to_string(), "entry-b".to_string()],
        },
    );

    assert_eq!(
        capture
            .begin_rewrite_bound(&token, bound.clone())
            .expect("bound rewrite should start"),
        "synthetic source"
    );
    capture
        .finish_rewrite_success_bound(&token, &bound)
        .expect("same bound intent should become ready");
    assert_eq!(
        capture
            .ready_matched_entry_ids(&token, &bound)
            .expect("matched ids should remain backend-owned"),
        vec!["entry-a".to_string(), "entry-b".to_string()]
    );

    let changed_revision = BoundRewriteIntent::new(
        RewriteIntent::grammar(),
        TerminologyIntent {
            store_revision: 8,
            ..bound.terminology().clone()
        },
    );
    assert!(capture
        .validate_ready_bound_intent(&token, &changed_revision)
        .is_err());
    capture.invalidate_terminology_intent();
    assert!(capture.validate_ready_bound_intent(&token, &bound).is_err());
}

#[test]
fn matched_only_prompt_serializes_constraints_as_untrusted_json_data() {
    let mut store = TerminologyStoreV1::new(1);
    for (id, status, source) in [
        (
            "matched-approved",
            EntryStatus::Approved,
            "approved fixture",
        ),
        (
            "unmatched-approved",
            EntryStatus::Approved,
            "sentinel fixture",
        ),
        (
            "matched-suggested",
            EntryStatus::Suggested,
            "suggested fixture",
        ),
        (
            "matched-disabled",
            EntryStatus::Disabled,
            "disabled fixture",
        ),
    ] {
        store
            .add_entry(
                id.to_string(),
                draft(
                    GENERAL_PROFILE_ID,
                    EntryType::Protected,
                    status,
                    source,
                    None,
                ),
                2,
            )
            .expect("synthetic entry should save");
    }
    let matches = match_terminology(
        &store,
        "approved fixture suggested fixture disabled fixture",
        &MatchContext::new(
            RewriteMode::Grammar,
            None,
            Some(LanguageScope::En),
            GENERAL_PROFILE_ID.to_string(),
        ),
    );
    let prompt = rewrite_prompt_with_terminology(
        "approved fixture suggested fixture disabled fixture",
        RewriteIntent::grammar(),
        &constraints(&matches.matches),
    )
    .expect("bounded prompt should serialize");

    assert_eq!(matches.matches.len(), 1);
    assert!(prompt.contains("matched-approved"));
    assert!(!prompt.contains("unmatched-approved"));
    assert!(!prompt.contains("matched-suggested"));
    assert!(!prompt.contains("matched-disabled"));
    assert!(prompt.contains("untrusted JSON data"));
}

#[test]
fn optional_result_metadata_is_strict_bounded_and_ephemeral() {
    let result = parse_rewrite_result(
        r#"{"replacement":"synthetic replacement","changed":true,"summary":"Synthetic summary.","confidence":0.9,"usedTerminologyIds":["entry-a"],"terminologySuggestions":[{"type":"preferred","sourceText":"candidate fixture","preferredText":"preferred fixture","sourceLanguage":"en","targetLanguage":"en","reason":"preferred_expression"}]}"#,
        RewriteMode::Grammar,
    )
    .expect("bounded metadata should parse");

    assert_eq!(result.used_terminology_ids, vec!["entry-a"]);
    assert_eq!(result.terminology_suggestions.len(), 1);
    assert_eq!(
        result.terminology_suggestions[0].reason,
        SuggestionReason::PreferredExpression
    );
    let six = (0..6)
        .map(|index| format!(r#"{{"type":"preferred","sourceText":"candidate-{index}","preferredText":"preferred-{index}","sourceLanguage":"en","targetLanguage":"en","reason":"preferred_expression"}}"#))
        .collect::<Vec<_>>()
        .join(",");
    let oversized = format!(
        r#"{{"replacement":"synthetic","changed":true,"summary":"Synthetic.","confidence":0.8,"terminologySuggestions":[{six}]}}"#
    );
    assert!(parse_rewrite_result(&oversized, RewriteMode::Grammar).is_err());
    let duplicate_usage = parse_rewrite_result(
        r#"{"replacement":"synthetic","changed":true,"summary":"Synthetic.","confidence":0.8,"usedTerminologyIds":["entry-duplicate","entry-duplicate"]}"#,
        RewriteMode::Grammar,
    );
    assert!(duplicate_usage.is_err());
    let noncanonical_suggestion = parse_rewrite_result(
        r#"{"replacement":"synthetic","changed":true,"summary":"Synthetic.","confidence":0.8,"terminologySuggestions":[{"type":"preferred","sourceText":" candidate ","preferredText":"preferred","sourceLanguage":"en","targetLanguage":"en","reason":"preferred_expression"}]}"#,
        RewriteMode::Grammar,
    );
    assert!(noncanonical_suggestion.is_err());
}

#[test]
fn runtime_mutations_persist_once_and_query_locally_with_deterministic_filters() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-02-runtime-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("fixture directory should exist");
    let path = root.join("terminology.v1.json");
    let mut runtime = TerminologyRuntime::open(path.clone(), 10);
    assert!(matches!(
        runtime.snapshot(),
        TerminologyRuntimeSnapshot::Ready { .. }
    ));

    runtime
        .add_profile("profile-local".to_string(), "Local Profile".to_string(), 20)
        .expect("profile command should persist");
    runtime
        .add_entry(
            "entry-local-a".to_string(),
            draft(
                "profile-local",
                EntryType::Preferred,
                EntryStatus::Approved,
                "Alpha fixture",
                Some("Alpha preferred"),
            ),
            30,
        )
        .expect("approved entry should persist");
    runtime
        .add_entry(
            "entry-local-b".to_string(),
            draft(
                "profile-local",
                EntryType::Translation,
                EntryStatus::Disabled,
                "Beta fixture",
                Some("Beta translated"),
            ),
            40,
        )
        .expect("disabled entry should persist");

    let query = EntryQuery {
        query: Some("alpha".to_string()),
        profile_id: Some("profile-local".to_string()),
        entry_type: Some(EntryType::Preferred),
        status: Some(EntryStatus::Approved),
        source_language: None,
        target_language: None,
        sort: EntrySort::SourceText,
    };
    let entries = runtime
        .query_entries(&query)
        .expect("local query should work");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "entry-local-a");
    assert_eq!(
        runtime.store().expect("runtime should be ready").revision,
        3
    );

    let reopened = TerminologyRuntime::open(path.clone(), 50);
    assert_eq!(
        reopened
            .store()
            .expect("persisted store should reload")
            .revision,
        3
    );
    assert_eq!(
        reopened
            .store()
            .expect("persisted store should reload")
            .entries
            .len(),
        2
    );
    fs::remove_dir_all(root).expect("fixture directory should be removed");
}

#[test]
fn runtime_keeps_suggestions_inert_until_two_explicit_persistence_actions() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-02-suggestion-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("fixture directory should exist");
    let path = root.join("terminology.v1.json");
    let mut runtime = TerminologyRuntime::open(path, 10);
    let suggestion = crate::terminology_validation::TerminologySuggestion {
        entry_type: EntryType::Preferred,
        source_text: "candidate fixture".to_string(),
        preferred_text: "preferred fixture".to_string(),
        source_language: LanguageScope::En,
        target_language: LanguageScope::En,
        reason: SuggestionReason::PreferredExpression,
    };

    assert!(runtime
        .store()
        .expect("runtime should be ready")
        .entries
        .is_empty());
    runtime
        .save_suggestion(
            "entry-suggestion".to_string(),
            GENERAL_PROFILE_ID.to_string(),
            suggestion,
            20,
        )
        .expect("explicit save should persist suggested status");
    let saved = runtime
        .store()
        .expect("runtime should be ready")
        .entries
        .first()
        .expect("suggestion should now exist");
    assert_eq!(saved.status, EntryStatus::Suggested);

    let context = MatchContext::new(
        RewriteMode::Grammar,
        None,
        Some(LanguageScope::En),
        GENERAL_PROFILE_ID.to_string(),
    );
    assert!(match_terminology(
        runtime.store().expect("runtime should be ready"),
        "candidate fixture",
        &context,
    )
    .matches
    .is_empty());
    runtime
        .set_entry_status("entry-suggestion", EntryStatus::Approved, 30)
        .expect("explicit approval should persist");
    assert_eq!(
        match_terminology(
            runtime.store().expect("runtime should be ready"),
            "candidate fixture",
            &context,
        )
        .matches
        .len(),
        1
    );
    fs::remove_dir_all(root).expect("fixture directory should be removed");
}

#[derive(Default)]
struct NoTouchApplyPlatform {
    calls: usize,
}

impl ApplyPlatform for NoTouchApplyPlatform {
    fn is_window(&mut self, _hwnd: isize) -> bool {
        self.calls += 1;
        true
    }
    fn window_pid(&mut self, _hwnd: isize) -> Option<u32> {
        self.calls += 1;
        Some(202)
    }
    fn hide_widget(&mut self) -> Result<(), ()> {
        self.calls += 1;
        Ok(())
    }
    fn show_widget(&mut self) {
        self.calls += 1;
    }
    fn request_foreground(&mut self, _hwnd: isize) {
        self.calls += 1;
    }
    fn foreground_window(&mut self) -> isize {
        self.calls += 1;
        101
    }
    fn clipboard_sequence(&mut self) -> u32 {
        self.calls += 1;
        1
    }
    fn read_clipboard_text(&mut self) -> Option<String> {
        self.calls += 1;
        None
    }
    fn write_clipboard_text(&mut self, _text: &str) -> Result<(), ()> {
        self.calls += 1;
        Ok(())
    }
    fn send_paste(&mut self) -> u32 {
        self.calls += 1;
        4
    }
    fn wait(&mut self, _stage: WaitStage) {
        self.calls += 1;
    }
}

#[test]
fn changed_terminology_revision_rejects_apply_before_any_platform_or_clipboard_call() {
    let mut capture = CaptureSessionStore::default();
    let token = capture
        .capture(
            "session-stale-terminology".to_string(),
            "synthetic source".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("capture should succeed");
    let settings = crate::settings::TerminologySettings::default();
    let bound = BoundRewriteIntent::new(
        RewriteIntent::grammar(),
        TerminologyIntent {
            enabled: settings.enabled,
            use_approved_terminology: settings.use_approved_terminology,
            suggest_terminology: settings.suggest_terminology,
            active_profile_id: settings.active_profile_id.clone(),
            store_revision: 7,
            matched_entry_ids: vec!["entry-bound".to_string()],
        },
    );
    capture
        .begin_rewrite_bound(&token, bound.clone())
        .expect("rewrite should begin");
    capture
        .finish_rewrite_success_bound(&token, &bound)
        .expect("rewrite should become ready");
    let mut platform = NoTouchApplyPlatform::default();

    let outcome = apply_current_terminology_bound(
        &mut capture,
        &token,
        &bound,
        RewriteIntent::grammar(),
        &settings,
        Some(8),
        "synthetic replacement",
        true,
        &mut platform,
    );

    assert_eq!(outcome, ApplyOutcome::RejectedStale);
    assert_eq!(platform.calls, 0);
}
