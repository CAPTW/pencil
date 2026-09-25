use crate::{
    active_turn::ActiveTurn,
    capture_session::{BoundRewriteIntent, CaptureSessionStore, WindowTarget},
    codex_client::parse_rewrite_result,
    rewrite_preserves_line_structure,
    settings::{
        decode_settings, load_from_path, save_to_path, AppSettings, RewriteMode,
        SettingsRecoveryCode, CLOUD_PROCESSING_DISCLOSURE_VERSION,
    },
    terminology::TerminologyStoreV1,
    terminology_store::{load_store_from_path, save_store_to_path, StoreRecovery},
    translation::RewriteIntent,
    DeviceLoginResponse,
};
use serde_json::Value;
use std::fs;

#[test]
fn writing_turn_contract_requires_owned_cwd_and_explicit_read_only_policy() {
    let source = include_str!("codex_client.rs");
    assert!(source.contains("runtime_cwd"));
    assert!(source.contains("\"sandboxPolicy\""));
    assert!(source.contains("\"networkAccess\": false"));
}

#[test]
fn app_server_child_is_strictly_started_without_tool_or_persistence_surfaces() {
    let source = include_str!("codex_client.rs");
    for required in [
        "shell_tool",
        "multi_agent",
        "computer_use",
        "browser_use",
        "image_generation",
        "mcp_servers={}",
        "web_search=\\\"disabled\\\"",
        "history.persistence=\\\"none\\\"",
        "analytics.enabled=false",
        "otel.log_user_prompt=false",
        "project_doc_max_bytes=0",
        "--strict-config",
    ] {
        assert!(source.contains(required));
    }
}

#[test]
fn app_server_uses_a_dedicated_config_free_home_and_owned_process_tree() {
    let client = include_str!("codex_client.rs");
    let home = include_str!("codex_home.rs");
    let job = include_str!("process_job.rs");

    assert!(client.contains("CodexHome::prepare"));
    assert!(client.contains(".env(\"CODEX_HOME\", codex_home.path())"));
    assert!(home.contains("codex_home_configuration_not_isolated"));
    assert!(home.contains(".codex-pencil-home-owner-v1"));
    assert!(job.contains("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE"));
    assert!(job.contains("AssignProcessToJobObject"));
}

#[test]
fn oversized_model_replacement_is_rejected_before_ready() {
    let replacement = "가".repeat(24_001);
    let payload = serde_json::json!({
        "replacement": replacement,
        "changed": true,
        "summary": "synthetic",
        "confidence": 0.5
    })
    .to_string();

    assert!(parse_rewrite_result(&payload, RewriteMode::Grammar).is_err());
}

#[test]
fn model_rewrite_must_preserve_logical_line_structure_before_ready() {
    assert!(rewrite_preserves_line_structure(
        "first\r\nsecond\n\nthird\r",
        "updated\nsecond updated\r\n\r\nthird updated\n",
    ));
    assert!(!rewrite_preserves_line_structure(
        "selected paragraph\r\n",
        "rewritten paragraph",
    ));
    assert!(!rewrite_preserves_line_structure(
        "first\n\nthird",
        "first rewritten\nthird rewritten",
    ));
}

#[test]
fn line_structure_rejection_resets_the_bound_session_for_a_safe_retry() {
    let mut store = CaptureSessionStore::default();
    let token = store
        .capture(
            "line-structure-session".to_string(),
            "selected paragraph\r\n".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("synthetic capture must succeed");
    let intent = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store
        .begin_rewrite_bound(&token, intent.clone())
        .expect("rewrite must start");

    assert!(!rewrite_preserves_line_structure(
        "selected paragraph\r\n",
        "rewritten paragraph",
    ));
    store
        .finish_rewrite_failure_bound(&token, &intent)
        .expect("line-structure rejection must return to Captured");
    store
        .begin_rewrite_bound(&token, intent)
        .expect("the same current capture must be retryable after rejection");
}

#[test]
fn production_line_structure_guard_runs_before_ready_transition() {
    let source = include_str!("main.rs");
    let rewrite_branch = source
        .split_once("match rewrite {")
        .map(|(_, tail)| tail)
        .expect("production rewrite result branch must exist");
    let guard = rewrite_branch
        .find("if !rewrite_preserves_line_structure")
        .expect("line-structure guard must exist");
    let rejected = rewrite_branch
        .find("finish_rewrite_failure_bound")
        .expect("line-structure rejection must reset the bound session");
    let ready = rewrite_branch
        .find("finish_rewrite_success_bound")
        .expect("successful rewrite must still enter Ready");

    assert!(guard < rejected);
    assert!(rejected < ready);
    assert!(rewrite_branch.contains("rewrite_line_structure_changed"));
}

#[test]
fn settings_contract_contains_versioned_cloud_processing_acknowledgement() {
    let source = include_str!("settings.rs");
    assert!(source.contains("cloud_processing_acknowledgement_version"));
    assert!(source.contains("CLOUD_PROCESSING_DISCLOSURE_VERSION"));
}

#[test]
fn migrated_settings_require_disclosure_again_and_acknowledgement_round_trips_alone() {
    let legacy = decode_settings(
        r#"{"schemaVersion":3,"mode":"natural","restoreClipboard":false,"autoRewrite":false,"shortcut":{"primary":{"modifiers":["CTRL","SHIFT"],"key":"H","display":"Ctrl+Shift+H"}},"translation":{"sourceLanguage":"auto","targetLanguage":"ja","applyFormat":"translation_only"},"terminology":{"enabled":true,"activeProfileId":"general","useApprovedTerminology":true,"suggestTerminology":true,"autoSaveSuggestions":false}}"#,
    );
    assert_eq!(legacy.recovery, Some(SettingsRecoveryCode::Migrated));
    assert_eq!(legacy.settings.cloud_processing_acknowledgement_version, 0);
    assert_eq!(legacy.settings.mode, RewriteMode::Natural);
    assert!(!legacy.settings.auto_rewrite);

    let mut acknowledged = AppSettings::default();
    acknowledged.cloud_processing_acknowledgement_version = CLOUD_PROCESSING_DISCLOSURE_VERSION;
    let encoded = serde_json::to_string(&acknowledged)
        .expect("synthetic acknowledged settings should serialize");
    let decoded = decode_settings(&encoded);
    assert_eq!(decoded.recovery, None);
    assert_eq!(decoded.settings, acknowledged);

    let mut unsupported = acknowledged;
    unsupported.cloud_processing_acknowledgement_version = CLOUD_PROCESSING_DISCLOSURE_VERSION + 1;
    assert_eq!(
        unsupported.validate(),
        Err("cloud_processing_acknowledgement_unsupported")
    );
}

#[test]
fn settings_and_terminology_backups_recover_independently_in_one_owned_app_data_root() {
    let root = std::env::temp_dir().join(format!(
        "codex-pencil-p1-03-independent-recovery-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("owned app-data fixture root should be created");
    let settings_path = root.join("settings.json");
    let terminology_path = root.join("terminology.v1.json");

    let mut prior_settings = AppSettings::default();
    prior_settings.mode = RewriteMode::Natural;
    let mut newer_settings = prior_settings.clone();
    newer_settings.mode = RewriteMode::Polite;
    save_to_path(&settings_path, &prior_settings).expect("prior settings should save");
    save_to_path(&settings_path, &newer_settings).expect("newer settings should save");

    let prior_terminology = TerminologyStoreV1::new(100);
    let mut newer_terminology = prior_terminology.clone();
    newer_terminology
        .add_profile(
            "recovery-probe".to_string(),
            "Recovery probe".to_string(),
            200,
        )
        .expect("newer terminology should mutate");
    save_store_to_path(&terminology_path, &prior_terminology)
        .expect("prior terminology should save");
    save_store_to_path(&terminology_path, &newer_terminology)
        .expect("newer terminology should save");

    fs::write(&settings_path, b"{synthetic-corrupt-settings")
        .expect("settings main corruption fixture should write");
    fs::write(&terminology_path, b"{synthetic-corrupt-terminology")
        .expect("terminology main corruption fixture should write");

    let recovered_settings =
        load_from_path(&settings_path).expect("settings backup should recover");
    let recovered_terminology =
        load_store_from_path(&terminology_path).expect("terminology backup should recover");
    assert_eq!(
        recovered_settings.recovery,
        Some(SettingsRecoveryCode::BackupRecovered)
    );
    assert_eq!(recovered_settings.settings, prior_settings);
    assert_eq!(
        recovered_terminology.recovery,
        Some(StoreRecovery::BackupRecovered)
    );
    assert_eq!(recovered_terminology.store, prior_terminology);

    fs::remove_dir_all(root).expect("owned app-data fixture root should be removed");
}

#[test]
fn backend_disclosure_and_source_limits_precede_client_or_thread_creation() {
    let source = include_str!("main.rs");
    let start = source
        .find("async fn rewrite_selected_text")
        .expect("rewrite command must exist");
    let end = source[start..]
        .find("fn terminology_environment_matches")
        .map(|offset| start + offset)
        .expect("rewrite command boundary must exist");
    let rewrite = &source[start..end];
    let disclosure = rewrite
        .find("cloud_processing_disclosure_required")
        .expect("backend disclosure gate must exist");
    let source_limit = rewrite
        .find("ContentLimitKind::Source")
        .expect("backend source limit must exist");
    let client = rewrite
        .find("ProviderManager::execute")
        .expect("reserved Provider execution must exist");
    assert!(disclosure < source_limit);
    assert!(source_limit < client);
}

#[test]
fn final_apply_limit_precedes_every_clipboard_or_input_platform_action() {
    let source = include_str!("main.rs");
    let start = source
        .find("async fn apply_replacement")
        .expect("apply command must exist");
    let end = source[start..]
        .find("fn load_settings")
        .map(|offset| start + offset)
        .expect("apply command boundary must exist");
    let apply = &source[start..end];
    let initial_limit = apply
        .find("ContentLimitKind::FinalApply")
        .expect("user-edited Apply input must have a final limit");
    let platform = apply
        .find("WindowsApplyPlatform::new")
        .expect("Apply platform must exist");
    assert!(initial_limit < platform);
    assert_eq!(apply.matches("ContentLimitKind::FinalApply").count(), 2);
}

#[test]
fn device_login_response_exposes_no_url_to_frontend() {
    let response = DeviceLoginResponse {
        login_id: "synthetic-login".to_string(),
        user_code: "synthetic-code".to_string(),
    };
    let serialized = serde_json::to_value(response).expect("synthetic response should serialize");
    assert_eq!(serialized.as_object().map(serde_json::Map::len), Some(2));
    assert!(serialized.get("verificationUrl").is_none());
    assert!(serialized.get("url").is_none());
}

#[test]
fn frontend_never_opens_an_app_server_supplied_url_directly() {
    let source = include_str!("../../src/App.tsx");
    assert!(!source.contains("openUrl(next.verificationUrl)"));
    assert!(!source.contains("openUrl(deviceLogin.verificationUrl)"));
    assert!(source.contains("open_device_login_page"));
}

#[test]
fn production_csp_is_non_null_and_remote_navigation_is_not_enabled() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json"))
        .expect("tracked Tauri configuration must be JSON");
    let csp = config.pointer("/app/security/csp");
    assert!(csp.is_some_and(|value| value.is_string()));
    let csp = csp.and_then(Value::as_str).unwrap_or_default();
    assert!(!csp.contains("https:"));
    assert!(!csp.contains('*'));
    assert!(csp.contains("connect-src ipc: http://ipc.localhost"));
    assert_eq!(csp.matches("http:").count(), 1);
}

#[test]
fn supported_capture_does_not_mutate_or_read_clipboard() {
    let source = include_str!("clipboard.rs");
    let capture = &source[..source.find("async fn wait_for_capture_modifiers_released").unwrap()];
    assert!(capture.contains("read_supported_selection()?"));
    assert!(!capture.contains("write_clipboard_text("));
    assert!(!capture.contains("read_clipboard_text("));
    assert!(!capture.contains("send_copy_shortcut"));
    // Actual metadata/Win32 denial tests live in windows_target::mission_secure_field_tests.
}

#[test]
fn active_turn_is_bound_to_the_exact_rewriting_session_and_taken_once_on_cancel() {
    let mut store = CaptureSessionStore::default();
    let token = store
        .capture(
            "session-interrupt".to_string(),
            "synthetic-source".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("synthetic capture must succeed");
    let intent = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store
        .begin_rewrite_bound(&token, intent.clone())
        .expect("rewrite must start");
    let active = ActiveTurn::synthetic("one");

    store
        .bind_active_turn(&token, &intent, active.clone())
        .expect("exact rewriting intent must bind");
    assert_eq!(store.cancel_with_active_turn(&token), Ok(Some(active)));
    assert!(store.cancel_with_active_turn(&token).is_err());
    assert!(store.finish_rewrite_success_bound(&token, &intent).is_err());
}

#[test]
fn recapture_takes_the_old_active_turn_and_stale_completion_cannot_become_ready() {
    let mut store = CaptureSessionStore::default();
    let first = store
        .capture(
            "session-old".to_string(),
            "synthetic-old".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("first capture must succeed");
    let intent = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    store
        .begin_rewrite_bound(&first, intent.clone())
        .expect("first rewrite must start");
    let active = ActiveTurn::synthetic("old");
    store
        .bind_active_turn(&first, &intent, active.clone())
        .expect("old turn must bind");

    assert_eq!(store.cancel_active_with_turn(), Some(active));
    let second = store
        .capture(
            "session-new".to_string(),
            "synthetic-new".to_string(),
            WindowTarget::new(303, 404),
            None,
            None,
        )
        .expect("new capture must succeed");
    assert_ne!(first, second);
    assert!(store.finish_rewrite_success_bound(&first, &intent).is_err());
}

#[test]
fn every_app_exit_request_uses_the_single_guarded_interrupt_path() {
    let source = include_str!("main.rs");
    assert!(source.contains("tauri::RunEvent::ExitRequested"));
    assert!(source.contains("shutdown_started.swap(true, Ordering::SeqCst)"));
    let exit_hook = source
        .find("tauri::RunEvent::ExitRequested")
        .expect("exit hook must exist");
    let tail = &source[exit_hook..];
    assert!(tail.contains("cancel_active_with_turn"));
    assert!(tail.contains("interrupt_active_turn"));
    assert!(tail.contains("app.exit(0)"));
}

#[test]
fn mode_and_terminology_revision_changes_take_the_active_turn_once() {
    let mut mode_store = CaptureSessionStore::default();
    let mode_token = mode_store
        .capture(
            "mode-session".to_string(),
            "synthetic-source".to_string(),
            WindowTarget::new(101, 202),
            None,
            None,
        )
        .expect("mode fixture capture must succeed");
    let grammar = BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    mode_store
        .begin_rewrite_bound(&mode_token, grammar.clone())
        .expect("mode fixture rewrite must start");
    let mode_turn = ActiveTurn::synthetic("mode");
    mode_store
        .bind_active_turn(&mode_token, &grammar, mode_turn.clone())
        .expect("mode fixture turn must bind");
    assert_eq!(
        mode_store
            .invalidate_intent_with_active_turn(
                RewriteIntent::new(RewriteMode::Natural, None)
                    .expect("natural mode must form a valid intent"),
            )
            .expect("mode invalidation must be valid"),
        Some(mode_turn)
    );
    assert!(mode_store
        .finish_rewrite_success_bound(&mode_token, &grammar)
        .is_err());

    let mut terminology_store = CaptureSessionStore::default();
    let terminology_token = terminology_store
        .capture(
            "terminology-session".to_string(),
            "synthetic-source".to_string(),
            WindowTarget::new(303, 404),
            None,
            None,
        )
        .expect("terminology fixture capture must succeed");
    terminology_store
        .begin_rewrite_bound(&terminology_token, grammar.clone())
        .expect("terminology fixture rewrite must start");
    let terminology_turn = ActiveTurn::synthetic("terminology");
    terminology_store
        .bind_active_turn(&terminology_token, &grammar, terminology_turn.clone())
        .expect("terminology fixture turn must bind");
    assert_eq!(
        terminology_store.invalidate_terminology_intent_with_active_turn(),
        Some(terminology_turn)
    );
    assert_eq!(
        terminology_store.invalidate_terminology_intent_with_active_turn(),
        None
    );
}
