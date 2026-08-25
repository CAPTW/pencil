#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod active_turn;
mod apply_safety;
mod capture_session;
mod clipboard;
mod codex_binary;
mod codex_client;
mod codex_home;
mod content_limits;
mod device_login;
mod prerequisites;
#[cfg(windows)]
mod process_job;
mod runtime_isolation;
mod settings;
mod shortcut;
mod terminology;
mod terminology_import_export;
mod terminology_matcher;
mod terminology_service;
mod terminology_store;
mod terminology_validation;
mod translation;
mod windows_apply;
mod windows_target;

#[cfg(test)]
mod p0_02_contract_tests;

#[cfg(all(test, windows))]
mod p0_02_windows_live_tests;

#[cfg(test)]
mod p1_01_contract_tests;

#[cfg(test)]
mod p1_02_contract_tests;

#[cfg(test)]
mod p1_03_contract_tests;

#[cfg(all(test, windows))]
mod p1_02_windows_live_tests;

#[cfg(all(test, windows))]
mod p1_01_windows_live_tests;

#[cfg(all(test, windows))]
mod p1_03_windows_live_tests;

use active_turn::ActiveTurn;
use apply_safety::{apply_current_session, ApplyFailureReason, ApplyOutcome, ApplyPlatform};
use capture_session::{
    BoundRewriteIntent, CaptureSessionStore, SessionError, SessionToken, TerminologyIntent,
};
use clipboard::CursorPoint;
use codex_client::{AuthStatus, CodexClient, CodexClientCache, RewriteResult};
use content_limits::{validate_text_limit, ContentLimitKind};
use device_login::ValidatedDeviceLoginUrl;
use prerequisites::PrerequisiteReport;
use serde::Serialize;
use settings::{AppSettings, RewriteMode, SettingsRecoveryCode, TerminologySettings};
use shortcut::{
    dispatch_shortcut_trigger, PrimaryShortcut, ShortcutCandidate, ShortcutEventState,
    ShortcutManager, ShortcutPersistence, ShortcutRegistrar, ShortcutRegistrarError,
    ShortcutStartupStatus, ShortcutTriggerGate, ShortcutUpdateStatus,
};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex as StdMutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    plugin::TauriPlugin,
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use terminology::{
    infer_source_language, target_scope_for_mode, EntryStatus, TerminologyEntryDraft,
};
use terminology_import_export::{ImportFormat, ImportReport};
use terminology_matcher::{constraints, match_terminology, MatchContext, MatchResult};
use terminology_service::{
    EntryQuery, ImportPlanPreview, TerminologyRuntime, TerminologyRuntimeSnapshot,
};
use terminology_store::now_ms;
use terminology_validation::{
    validate_result, TerminologySuggestion, TerminologyWarning, WarningCode,
};
use tokio::sync::Mutex;
use translation::{format_translation, RewriteIntent, TranslationTargetLanguage};
use uuid::Uuid;
use windows_apply::WindowsApplyPlatform;
use windows_target::{capture_before_widget_focus, WindowsForegroundTargetPlatform};

struct ConfigurationState {
    settings: AppSettings,
    shortcut: ShortcutManager,
    recovery: Option<SettingsRecoveryCode>,
}

impl Default for ConfigurationState {
    fn default() -> Self {
        let settings = AppSettings::default();
        Self {
            shortcut: ShortcutManager::new(settings.shortcut.primary.clone()),
            settings,
            recovery: None,
        }
    }
}

struct AppState {
    codex: CodexClientCache,
    capture: Mutex<CaptureSessionStore>,
    configuration: StdMutex<ConfigurationState>,
    pending_device_login: StdMutex<Option<PendingDeviceLogin>>,
    terminology: StdMutex<TerminologyRuntime>,
    shortcut_trigger: StdMutex<ShortcutTriggerGate>,
    startup_notices: StdMutex<Vec<String>>,
    shutdown_started: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            codex: CodexClientCache::default(),
            capture: Mutex::new(CaptureSessionStore::default()),
            configuration: StdMutex::new(ConfigurationState::default()),
            pending_device_login: StdMutex::new(None),
            terminology: StdMutex::new(TerminologyRuntime::default()),
            shortcut_trigger: StdMutex::new(ShortcutTriggerGate::default()),
            startup_notices: StdMutex::new(Vec::new()),
            shutdown_started: AtomicBool::new(false),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutUpdateResponse {
    status: ShortcutUpdateStatus,
    active: PrimaryShortcut,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectionCapturedEvent {
    session_id: String,
    generation: u64,
    selected_text: String,
}

#[derive(Clone, Debug, Serialize)]
struct CaptureErrorEvent {
    message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginCompletedEvent {
    login_id: Option<String>,
    success: bool,
    error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct AuthChangedEvent {}

#[derive(Clone, Debug, Serialize)]
struct CodexProcessEvent {
    message: String,
}

#[derive(Clone)]
struct PendingDeviceLogin {
    login_id: String,
    url: ValidatedDeviceLoginUrl,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceLoginResponse {
    login_id: String,
    user_code: String,
}

#[tauri::command]
async fn auth_status(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    ensure_codex(&state).await?.auth_status().await
}

#[tauri::command]
async fn start_device_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DeviceLoginResponse, String> {
    let client = ensure_codex(&state).await?;
    let login = client.start_device_login().await?;
    let validated_url = match ValidatedDeviceLoginUrl::parse(&login.verification_url) {
        Ok(url) => url,
        Err(code) => {
            let _ = client.cancel_login(login.login_id).await;
            return Err(code.to_string());
        }
    };
    *state
        .pending_device_login
        .lock()
        .map_err(|_| "device_login_state_unavailable".to_string())? = Some(PendingDeviceLogin {
        login_id: login.login_id.clone(),
        url: validated_url,
    });
    spawn_auth_notification_bridge(app, client, login.login_id.clone());
    Ok(DeviceLoginResponse {
        login_id: login.login_id,
        user_code: login.user_code,
    })
}

#[tauri::command]
async fn cancel_device_login(login_id: String, state: State<'_, AppState>) -> Result<(), String> {
    ensure_codex(&state)
        .await?
        .cancel_login(login_id.clone())
        .await?;
    let mut pending = state
        .pending_device_login
        .lock()
        .map_err(|_| "device_login_state_unavailable".to_string())?;
    if pending
        .as_ref()
        .is_some_and(|current| current.login_id == login_id)
    {
        *pending = None;
    }
    Ok(())
}

#[tauri::command]
fn open_device_login_page(
    login_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let url = {
        let pending = state
            .pending_device_login
            .lock()
            .map_err(|_| "device_login_state_unavailable".to_string())?;
        let pending = pending
            .as_ref()
            .filter(|pending| pending.login_id == login_id)
            .ok_or_else(|| "device_login_stale".to_string())?;
        pending.url.clone()
    };
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| "device_login_open_failed".to_string())
}

#[tauri::command]
async fn rewrite_selected_text(
    session_id: String,
    generation: u64,
    mode: RewriteMode,
    target_language: Option<TranslationTargetLanguage>,
    auto_reference_language: Option<TranslationTargetLanguage>,
    state: State<'_, AppState>,
) -> Result<RewriteResult, String> {
    let token = SessionToken {
        session_id,
        generation,
    };
    let intent =
        RewriteIntent::new_with_auto_reference(mode, target_language, auto_reference_language)
            .map_err(str::to_string)?;
    let disclosure_version = state
        .configuration
        .lock()
        .map_err(|_| "configuration_unavailable".to_string())?
        .settings
        .cloud_processing_acknowledgement_version;
    if disclosure_version < settings::CLOUD_PROCESSING_DISCLOSURE_VERSION {
        return Err("cloud_processing_disclosure_required".to_string());
    }
    let (selected_text, bound_intent, match_result, request_constraints) = {
        let mut capture = state.capture.lock().await;
        let selected_text = capture
            .captured_source(&token)
            .map_err(|error| error.code().to_string())?;
        validate_text_limit(ContentLimitKind::Source, &selected_text)
            .map_err(|error| error.to_string())?;
        let settings = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?
            .settings
            .clone();
        if settings.rewrite_intent().map_err(str::to_string)? != intent {
            return Err("stale_rewrite_intent".to_string());
        }

        let terminology = state
            .terminology
            .lock()
            .map_err(|_| "terminology_store_unavailable".to_string())?;
        let (store_revision, match_result) =
            if settings.terminology.enabled && settings.terminology.use_approved_terminology {
                terminology
                    .validate_active_profile(&settings.terminology.active_profile_id)
                    .map_err(|error| error.code().to_string())?;
                let store = terminology
                    .store()
                    .map_err(|error| error.code().to_string())?;
                let context = MatchContext::new(
                    mode,
                    target_scope_for_mode(mode, target_language),
                    infer_source_language(&selected_text),
                    settings.terminology.active_profile_id.clone(),
                );
                (
                    store.revision,
                    match_terminology(store, &selected_text, &context),
                )
            } else {
                (
                    terminology.store().map(|store| store.revision).unwrap_or(0),
                    MatchResult::default(),
                )
            };
        let matched_entry_ids = match_result
            .matches
            .iter()
            .map(|matched| matched.entry_id.clone())
            .collect::<Vec<_>>();
        let request_constraints = constraints(&match_result.matches);
        let bound_intent = BoundRewriteIntent::new(
            intent,
            TerminologyIntent {
                enabled: settings.terminology.enabled,
                use_approved_terminology: settings.terminology.use_approved_terminology,
                suggest_terminology: settings.terminology.suggest_terminology,
                active_profile_id: settings.terminology.active_profile_id,
                store_revision,
                matched_entry_ids,
            },
        );
        capture
            .begin_rewrite_bound(&token, bound_intent.clone())
            .map_err(|error| error.code().to_string())?;
        (
            selected_text,
            bound_intent,
            match_result,
            request_constraints,
        )
    };

    let rewrite = match ensure_codex(&state).await {
        Ok(client) => match client
            .prepare_rewrite_with_terminology(&selected_text, intent, &request_constraints)
            .await
        {
            Ok(prepared) => {
                let still_current = state
                    .capture
                    .lock()
                    .await
                    .validate_rewriting_bound_intent(&token, &bound_intent);
                if let Err(error) = still_current {
                    return Err(error.code().to_string());
                }
                match client.start_prepared_rewrite(prepared).await {
                    Ok(pending) => {
                        let active_turn = pending.active_turn();
                        let bound = state.capture.lock().await.bind_active_turn(
                            &token,
                            &bound_intent,
                            active_turn.clone(),
                        );
                        if let Err(error) = bound {
                            let _ = client.interrupt_turn(&active_turn).await;
                            Err(error.code().to_string())
                        } else {
                            client.complete_rewrite(pending).await
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };

    let mut capture = state.capture.lock().await;
    match rewrite {
        Ok(mut result) => {
            if !terminology_environment_matches(&state, &bound_intent)? {
                capture.invalidate_terminology_intent();
                return Err("stale_rewrite_intent".to_string());
            }
            if !rewrite_preserves_line_structure(&selected_text, &result.replacement) {
                return match capture.finish_rewrite_failure_bound(&token, &bound_intent) {
                    Ok(()) => Err("rewrite_line_structure_changed".to_string()),
                    Err(error) => Err(error.code().to_string()),
                };
            }
            capture
                .finish_rewrite_success_bound(&token, &bound_intent)
                .map_err(|error| error.code().to_string())?;
            let matched_ids = match_result
                .matches
                .iter()
                .map(|matched| matched.entry_id.as_str())
                .collect::<HashSet<_>>();
            let reported_unknown = result
                .used_terminology_ids
                .iter()
                .any(|id| !matched_ids.contains(id.as_str()));
            result.terminology_warnings = validate_result(
                &result.replacement,
                &match_result,
                &result.used_terminology_ids,
            );
            if reported_unknown {
                result.terminology_warnings.push(TerminologyWarning {
                    code: WarningCode::TerminologyUsageUnverified,
                    entry_ids: Vec::new(),
                });
            }
            result.terminology_match_count = match_result.matches.len();
            if !bound_intent.terminology().enabled
                || !bound_intent.terminology().suggest_terminology
            {
                result.terminology_suggestions.clear();
            }
            Ok(result)
        }
        Err(error) => match capture.finish_rewrite_failure_bound(&token, &bound_intent) {
            Ok(()) => Err(error),
            Err(session_error) => Err(session_error.code().to_string()),
        },
    }
}

pub(crate) fn rewrite_preserves_line_structure(source: &str, replacement: &str) -> bool {
    logical_line_break_runs(source) == logical_line_break_runs(replacement)
}

fn logical_line_break_runs(value: &str) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut current_run = 0usize;
    let mut characters = value.chars().peekable();

    while let Some(character) = characters.next() {
        let is_line_break = match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                true
            }
            '\n' | '\u{2028}' | '\u{2029}' => true,
            _ => false,
        };

        if is_line_break {
            current_run += 1;
        } else if current_run > 0 {
            runs.push(current_run);
            current_run = 0;
        }
    }

    if current_run > 0 {
        runs.push(current_run);
    }
    runs
}

fn terminology_environment_matches(
    state: &State<'_, AppState>,
    bound: &BoundRewriteIntent,
) -> Result<bool, String> {
    let settings = state
        .configuration
        .lock()
        .map_err(|_| "configuration_unavailable".to_string())?
        .settings
        .clone();
    if settings.rewrite_intent().map_err(str::to_string)? != bound.rewrite()
        || settings.terminology.enabled != bound.terminology().enabled
        || settings.terminology.use_approved_terminology
            != bound.terminology().use_approved_terminology
        || settings.terminology.suggest_terminology != bound.terminology().suggest_terminology
        || settings.terminology.active_profile_id != bound.terminology().active_profile_id
    {
        return Ok(false);
    }
    let terminology = state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())?;
    Ok(terminology
        .store()
        .map(|store| store.revision == bound.terminology().store_revision)
        .unwrap_or(!bound.terminology().enabled))
}

pub(crate) fn apply_current_terminology_bound<P: ApplyPlatform>(
    capture: &mut CaptureSessionStore,
    token: &SessionToken,
    bound: &BoundRewriteIntent,
    rewrite: RewriteIntent,
    settings: &TerminologySettings,
    current_store_revision: Option<u64>,
    replacement: &str,
    restore_clipboard: bool,
    platform: &mut P,
) -> ApplyOutcome {
    if capture.validate_ready_bound_intent(token, bound).is_err()
        || bound.rewrite() != rewrite
        || bound.terminology().enabled != settings.enabled
        || bound.terminology().use_approved_terminology != settings.use_approved_terminology
        || bound.terminology().suggest_terminology != settings.suggest_terminology
        || bound.terminology().active_profile_id != settings.active_profile_id
        || current_store_revision
            .map(|revision| revision != bound.terminology().store_revision)
            .unwrap_or(bound.terminology().enabled || bound.terminology().store_revision != 0)
    {
        return ApplyOutcome::RejectedStale;
    }
    apply_current_session(capture, token, replacement, restore_clipboard, platform)
}

#[tauri::command]
async fn apply_replacement(
    session_id: String,
    generation: u64,
    replacement: String,
    mode: RewriteMode,
    target_language: Option<TranslationTargetLanguage>,
    auto_reference_language: Option<TranslationTargetLanguage>,
    restore_clipboard: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyOutcome, String> {
    validate_text_limit(ContentLimitKind::FinalApply, &replacement)
        .map_err(|error| error.to_string())?;
    let token = SessionToken {
        session_id,
        generation,
    };
    let intent =
        RewriteIntent::new_with_auto_reference(mode, target_language, auto_reference_language)
            .map_err(str::to_string)?;
    let mut capture = state.capture.lock().await;
    let bound_intent = match capture.ready_bound_intent_for(&token, intent) {
        Ok(bound) => bound,
        Err(error) => {
            return Ok(match error {
                SessionError::StaleSession | SessionError::StaleIntent => {
                    ApplyOutcome::RejectedStale
                }
                SessionError::InvalidState | SessionError::GenerationExhausted => {
                    ApplyOutcome::Failed {
                        reason: ApplyFailureReason::InvalidSessionState,
                    }
                }
            })
        }
    };
    let configuration = state
        .configuration
        .lock()
        .map_err(|_| "configuration_unavailable".to_string())?;
    let current_intent = configuration
        .settings
        .rewrite_intent()
        .map_err(str::to_string)?;
    let apply_format = configuration.settings.translation.apply_format;
    let mut terminology = state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())?;
    let current_store_revision = terminology.store().ok().map(|store| store.revision);
    let final_replacement = if intent.is_translation() {
        let source = capture
            .ready_source_for(&token, intent)
            .map_err(|error| error.code().to_string())?;
        match format_translation(&source, &replacement, apply_format) {
            Ok(formatted) => formatted,
            Err("empty_translation") => {
                return Ok(ApplyOutcome::Failed {
                    reason: ApplyFailureReason::EmptyReplacement,
                })
            }
            Err(_) => {
                return Ok(ApplyOutcome::Failed {
                    reason: ApplyFailureReason::InvalidSessionState,
                })
            }
        }
    } else {
        replacement
    };
    validate_text_limit(ContentLimitKind::FinalApply, &final_replacement)
        .map_err(|error| error.to_string())?;
    let mut platform = WindowsApplyPlatform::new(app);
    let outcome = apply_current_terminology_bound(
        &mut capture,
        &token,
        &bound_intent,
        current_intent,
        &configuration.settings.terminology,
        current_store_revision,
        &final_replacement,
        restore_clipboard,
        &mut platform,
    );
    if matches!(
        outcome,
        ApplyOutcome::Applied | ApplyOutcome::CopiedFallback { .. }
    ) {
        let _ =
            terminology.increment_usage(&bound_intent.terminology().matched_entry_ids, now_ms());
    }
    Ok(outcome)
}

#[tauri::command]
fn load_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .configuration
        .lock()
        .map(|configuration| configuration.settings.clone())
        .map_err(|_| "configuration_unavailable".to_string())
}

#[tauri::command]
fn terminology_state(state: State<'_, AppState>) -> Result<TerminologyRuntimeSnapshot, String> {
    state
        .terminology
        .lock()
        .map(|terminology| terminology.snapshot())
        .map_err(|_| "terminology_store_unavailable".to_string())
}

#[tauri::command]
fn query_terminology_entries(
    query: EntryQuery,
    state: State<'_, AppState>,
) -> Result<Vec<terminology::TerminologyEntry>, String> {
    state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())?
        .query_entries(&query)
        .map_err(|error| error.code().to_string())
}

#[tauri::command]
async fn add_terminology_profile(
    name: String,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.add_profile(Uuid::new_v4().to_string(), name, now_ms())
    })
    .await
}

#[tauri::command]
async fn rename_terminology_profile(
    profile_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.rename_profile(&profile_id, name, now_ms())
    })
    .await
}

#[tauri::command]
async fn set_terminology_profile_enabled(
    profile_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    let snapshot = {
        let configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        let mut terminology = state
            .terminology
            .lock()
            .map_err(|_| "terminology_store_unavailable".to_string())?;
        terminology
            .set_profile_enabled(
                &profile_id,
                enabled,
                &configuration.settings.terminology.active_profile_id,
                now_ms(),
            )
            .map_err(|error| error.code().to_string())?;
        terminology.snapshot()
    };
    invalidate_terminology_capture(&state).await;
    Ok(snapshot)
}

#[tauri::command]
async fn set_active_terminology_profile(
    profile_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let saved = {
        let mut configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        let terminology = state
            .terminology
            .lock()
            .map_err(|_| "terminology_store_unavailable".to_string())?;
        terminology
            .validate_active_profile(&profile_id)
            .map_err(|error| error.code().to_string())?;
        let mut next = configuration.settings.clone();
        next.terminology.active_profile_id = profile_id;
        next.validate().map_err(str::to_string)?;
        settings::save(&app, &next)?;
        configuration.settings = next.clone();
        configuration.recovery = None;
        next
    };
    invalidate_terminology_capture(&state).await;
    Ok(saved)
}

#[tauri::command]
async fn add_terminology_entry(
    draft: TerminologyEntryDraft,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.add_entry(Uuid::new_v4().to_string(), draft, now_ms())
    })
    .await
}

#[tauri::command]
async fn update_terminology_entry(
    entry_id: String,
    draft: TerminologyEntryDraft,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.update_entry(&entry_id, draft, now_ms())
    })
    .await
}

#[tauri::command]
async fn set_terminology_entry_status(
    entry_id: String,
    status: EntryStatus,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.set_entry_status(&entry_id, status, now_ms())
    })
    .await
}

#[tauri::command]
async fn approve_terminology_suggestion(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.set_entry_status(&entry_id, EntryStatus::Approved, now_ms())
    })
    .await
}

#[tauri::command]
async fn save_terminology_suggestion(
    profile_id: String,
    suggestion: TerminologySuggestion,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| {
        runtime.save_suggestion(Uuid::new_v4().to_string(), profile_id, suggestion, now_ms())
    })
    .await
}

#[tauri::command]
async fn delete_terminology_entry(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    mutate_terminology(&state, |runtime| runtime.delete_entry(&entry_id)).await
}

#[tauri::command]
fn export_terminology(format: ImportFormat, state: State<'_, AppState>) -> Result<String, String> {
    state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())?
        .export(format)
        .map_err(|error| error.code().to_string())
}

#[tauri::command]
fn dry_run_terminology_import(
    format: ImportFormat,
    text: String,
    state: State<'_, AppState>,
) -> Result<ImportPlanPreview, String> {
    state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())?
        .dry_run_import(format, &text, now_ms())
        .map_err(|error| error.code().to_string())
}

#[tauri::command]
async fn apply_terminology_import(
    plan_id: String,
    state: State<'_, AppState>,
) -> Result<ImportReport, String> {
    let report = {
        let mut runtime = state
            .terminology
            .lock()
            .map_err(|_| "terminology_store_unavailable".to_string())?;
        runtime
            .apply_import(&plan_id, now_ms())
            .map_err(|error| error.code().to_string())?
    };
    invalidate_terminology_capture(&state).await;
    Ok(report)
}

#[tauri::command]
async fn reset_terminology_store(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    let snapshot = mutate_terminology(&state, |runtime| runtime.reset(now_ms())).await?;
    let mut configuration = state
        .configuration
        .lock()
        .map_err(|_| "configuration_unavailable".to_string())?;
    let mut next = configuration.settings.clone();
    next.terminology.active_profile_id = terminology::GENERAL_PROFILE_ID.to_string();
    settings::save(&app, &next)?;
    configuration.settings = next;
    configuration.recovery = None;
    Ok(snapshot)
}

async fn mutate_terminology(
    state: &State<'_, AppState>,
    mutation: impl FnOnce(&mut TerminologyRuntime) -> Result<(), terminology::TerminologyError>,
) -> Result<TerminologyRuntimeSnapshot, String> {
    let snapshot = {
        let mut terminology = state
            .terminology
            .lock()
            .map_err(|_| "terminology_store_unavailable".to_string())?;
        mutation(&mut terminology).map_err(|error| error.code().to_string())?;
        terminology.snapshot()
    };
    invalidate_terminology_capture(state).await;
    Ok(snapshot)
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    settings.validate().map_err(str::to_string)?;
    let (previous_intent, next_intent, terminology_changed) = {
        let mut configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        if settings.terminology.enabled {
            state
                .terminology
                .lock()
                .map_err(|_| "terminology_store_unavailable".to_string())?
                .validate_active_profile(&settings.terminology.active_profile_id)
                .map_err(|error| error.code().to_string())?;
        }
        if settings.shortcut != configuration.settings.shortcut {
            return Err("shortcut_update_requires_transaction".to_string());
        }
        let previous_intent = configuration
            .settings
            .rewrite_intent()
            .map_err(str::to_string)?;
        let next_intent = settings.rewrite_intent().map_err(str::to_string)?;
        let terminology_changed = configuration.settings.terminology != settings.terminology;
        settings::save(&app, &settings)?;
        configuration.settings = settings.clone();
        configuration.recovery = None;
        (previous_intent, next_intent, terminology_changed)
    };

    let active_turn = if previous_intent != next_intent || terminology_changed {
        let mut capture = state.capture.lock().await;
        let mut active_turn = if previous_intent != next_intent {
            capture
                .invalidate_intent_with_active_turn(next_intent)
                .map_err(|error| error.code().to_string())?
        } else {
            None
        };
        if terminology_changed {
            active_turn =
                active_turn.or_else(|| capture.invalidate_terminology_intent_with_active_turn());
        }
        active_turn
    } else {
        None
    };
    interrupt_active_turn(state.inner(), active_turn).await;
    Ok(settings)
}

#[tauri::command]
fn update_primary_shortcut(
    candidate: ShortcutCandidate,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ShortcutUpdateResponse, String> {
    change_primary_shortcut(candidate, &app, &state)
}

#[tauri::command]
fn reset_primary_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ShortcutUpdateResponse, String> {
    change_primary_shortcut(PrimaryShortcut::default().candidate(), &app, &state)
}

#[tauri::command]
fn check_prerequisites() -> PrerequisiteReport {
    prerequisites::check()
}

#[tauri::command]
fn take_startup_notices(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut notices = state
        .startup_notices
        .lock()
        .map_err(|_| "startup_notices_unavailable".to_string())?;
    Ok(std::mem::take(&mut *notices))
}

#[tauri::command]
async fn dismiss_window(
    session_id: Option<String>,
    generation: Option<u64>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let active_turn = {
        let mut capture = state.capture.lock().await;
        match (session_id, generation) {
            (Some(session_id), Some(generation)) => capture
                .cancel_with_active_turn(&SessionToken {
                    session_id,
                    generation,
                })
                .map_err(|error| error.code().to_string())?,
            (None, None) if !capture.has_active() => None,
            _ => return Err("session_token_required".to_string()),
        }
    };
    interrupt_active_turn(&state, active_turn).await;
    hide_main_window(&app)
}

struct TauriShortcutRegistrar {
    app: AppHandle,
}

impl ShortcutRegistrar for TauriShortcutRegistrar {
    fn register(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError> {
        let parsed = shortcut
            .registration_string()
            .parse::<Shortcut>()
            .map_err(|_| ShortcutRegistrarError::Failed)?;
        self.app
            .global_shortcut()
            .register(parsed)
            .map_err(|_| ShortcutRegistrarError::Conflict)
    }

    fn unregister(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError> {
        let parsed = shortcut
            .registration_string()
            .parse::<Shortcut>()
            .map_err(|_| ShortcutRegistrarError::Failed)?;
        self.app
            .global_shortcut()
            .unregister(parsed)
            .map_err(|_| ShortcutRegistrarError::Failed)
    }
}

struct AppShortcutPersistence<'a> {
    app: &'a AppHandle,
    base: AppSettings,
    saved: Option<AppSettings>,
}

impl ShortcutPersistence for AppShortcutPersistence<'_> {
    fn persist(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ()> {
        let mut next = self.base.clone();
        next.shortcut.primary = shortcut.clone();
        settings::save(self.app, &next).map_err(|_| ())?;
        self.saved = Some(next);
        Ok(())
    }
}

fn change_primary_shortcut(
    candidate: ShortcutCandidate,
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> Result<ShortcutUpdateResponse, String> {
    let response = {
        let mut configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        let mut registrar = TauriShortcutRegistrar { app: app.clone() };
        let mut persistence = AppShortcutPersistence {
            app,
            base: configuration.settings.clone(),
            saved: None,
        };
        let outcome = configuration
            .shortcut
            .update(candidate, &mut registrar, &mut persistence);
        if outcome.status == ShortcutUpdateStatus::Applied {
            let saved = persistence
                .saved
                .take()
                .ok_or_else(|| "shortcut_persistence_state_missing".to_string())?;
            configuration.settings = saved;
            configuration.recovery = None;
        }
        ShortcutUpdateResponse {
            status: outcome.status,
            active: configuration.shortcut.active().clone(),
        }
    };

    if matches!(
        response.status,
        ShortcutUpdateStatus::Applied | ShortcutUpdateStatus::Unchanged
    ) {
        if let Ok(mut trigger) = state.shortcut_trigger.lock() {
            let _ = trigger.handle(true, ShortcutEventState::Released);
        }
    }
    Ok(response)
}

async fn ensure_codex(state: &State<'_, AppState>) -> Result<Arc<CodexClient>, String> {
    state.codex.get().await
}

async fn interrupt_active_turn(state: &AppState, active_turn: Option<ActiveTurn>) {
    let Some(active_turn) = active_turn else {
        return;
    };
    let Some(client) = state.codex.current_healthy().await else {
        return;
    };
    let _ = client.interrupt_turn(&active_turn).await;
}

async fn invalidate_terminology_capture(state: &State<'_, AppState>) {
    let active_turn = state
        .capture
        .lock()
        .await
        .invalidate_terminology_intent_with_active_turn();
    interrupt_active_turn(state.inner(), active_turn).await;
}

fn main() {
    let app = tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(global_shortcut_plugin())
        .invoke_handler(tauri::generate_handler![
            add_terminology_entry,
            add_terminology_profile,
            apply_replacement,
            apply_terminology_import,
            approve_terminology_suggestion,
            auth_status,
            cancel_device_login,
            check_prerequisites,
            delete_terminology_entry,
            dismiss_window,
            dry_run_terminology_import,
            export_terminology,
            load_settings,
            open_device_login_page,
            query_terminology_entries,
            rename_terminology_profile,
            reset_primary_shortcut,
            reset_terminology_store,
            rewrite_selected_text,
            save_terminology_suggestion,
            save_settings,
            set_active_terminology_profile,
            set_terminology_entry_status,
            set_terminology_profile_enabled,
            start_device_login,
            take_startup_notices,
            terminology_state,
            update_terminology_entry,
            update_primary_shortcut
        ])
        .setup(|app| {
            create_tray(app)?;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            match initialize_configuration(app) {
                Ok(ShortcutStartupStatus::Failed) | Err(_) => {
                    let _ = show_main_window(app.handle(), None);
                }
                Ok(_) => {}
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle().clone();
                let _ = window.hide();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    let active_turn = state.capture.lock().await.cancel_active_with_turn();
                    interrupt_active_turn(state.inner(), active_turn).await;
                });
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Codex Pencil");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let state = app.state::<AppState>();
            if !state.shutdown_started.swap(true, Ordering::SeqCst) {
                api.prevent_exit();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    let active_turn = state.capture.lock().await.cancel_active_with_turn();
                    interrupt_active_turn(state.inner(), active_turn).await;
                    state.codex.shutdown().await;
                    app.exit(0);
                });
            }
        }
    });
}

fn create_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Codex Pencil", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let account = MenuItem::with_id(app, "account", "Login / Account", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &account, &settings, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main").tooltip("Codex Pencil");
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_tray_menu_action(app, event.id.as_ref()))
        .build(app)?;

    Ok(())
}

fn handle_tray_menu_action(app: &AppHandle, action: &str) {
    match action {
        "show" => {
            let _ = show_main_window(app, None);
        }
        "hide" => {
            let _ = hide_main_window(app);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let active_turn = state.capture.lock().await.cancel_active_with_turn();
                interrupt_active_turn(state.inner(), active_turn).await;
            });
        }
        "account" => {
            let _ = show_main_window(app, None);
        }
        "settings" => {
            let _ = show_main_window(app, None);
            let _ = app.emit("open-settings", ());
        }
        "quit" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let active_turn = state.capture.lock().await.cancel_active_with_turn();
                interrupt_active_turn(state.inner(), active_turn).await;
                app.exit(0);
            });
        }
        _ => {}
    }
}

fn global_shortcut_plugin() -> TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            let state = app.state::<AppState>();
            let is_active = state
                .configuration
                .lock()
                .ok()
                .and_then(|configuration| {
                    configuration
                        .shortcut
                        .active()
                        .registration_string()
                        .parse::<Shortcut>()
                        .ok()
                })
                .is_some_and(|active| &active == shortcut);
            let event_state = if event.state() == ShortcutState::Pressed {
                ShortcutEventState::Pressed
            } else {
                ShortcutEventState::Released
            };
            let should_capture = state.shortcut_trigger.lock().is_ok_and(|mut trigger| {
                dispatch_shortcut_trigger(&mut trigger, is_active, event_state, || {})
            });
            if should_capture {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(message) = capture_from_hotkey(app.clone()).await {
                        let _ = app.emit("capture-error", CaptureErrorEvent { message });
                        let _ = show_main_window(&app, None);
                    }
                });
            }
        })
        .build()
}

fn initialize_configuration(app: &mut tauri::App) -> Result<ShortcutStartupStatus, String> {
    let app_handle = app.handle().clone();
    let mut notices = Vec::new();
    let mut loaded = match settings::load_state(&app_handle) {
        Ok(loaded) => loaded,
        Err(_) => {
            notices.push("settings_recovered_to_defaults".to_string());
            settings::SettingsLoad {
                settings: AppSettings::default(),
                recovery: Some(SettingsRecoveryCode::InvalidFields),
            }
        }
    };
    if let Some(recovery) = loaded.recovery {
        notices.push(
            match recovery {
                SettingsRecoveryCode::Migrated => "settings_migrated",
                SettingsRecoveryCode::InvalidFields => "settings_invalid_fields_recovered",
                SettingsRecoveryCode::BackupRecovered => "settings_backup_recovered",
            }
            .to_string(),
        );
        if matches!(
            recovery,
            SettingsRecoveryCode::Migrated | SettingsRecoveryCode::BackupRecovered
        ) && settings::save(&app_handle, &loaded.settings).is_err()
        {
            notices.push("settings_recovery_persist_failed".to_string());
        }
    }

    let state = app_handle.state::<AppState>();
    let terminology = match terminology_store::terminology_path(&app_handle) {
        Ok(path) => TerminologyRuntime::open(path, now_ms()),
        Err(_) => TerminologyRuntime::default(),
    };
    match terminology.snapshot() {
        TerminologyRuntimeSnapshot::Ready {
            recovery: Some(_), ..
        } => notices.push("terminology_backup_recovered".to_string()),
        TerminologyRuntimeSnapshot::Unrecoverable { .. } => {
            notices.push("terminology_store_unrecoverable".to_string())
        }
        TerminologyRuntimeSnapshot::Uninitialized { .. } => {
            notices.push("terminology_store_uninitialized".to_string())
        }
        TerminologyRuntimeSnapshot::Ready { recovery: None, .. } => {}
    }
    if terminology
        .validate_active_profile(&loaded.settings.terminology.active_profile_id)
        .is_err()
    {
        if let Some(fallback) = terminology.fallback_active_profile_id() {
            loaded.settings.terminology.active_profile_id = fallback;
            if settings::save(&app_handle, &loaded.settings).is_ok() {
                notices.push("terminology_active_profile_recovered".to_string());
            } else {
                notices.push("terminology_active_profile_recovery_failed".to_string());
            }
        }
    }
    *state
        .terminology
        .lock()
        .map_err(|_| "terminology_store_unavailable".to_string())? = terminology;
    let startup = {
        let mut configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        configuration.settings = loaded.settings;
        configuration.recovery = loaded.recovery;
        configuration.shortcut = ShortcutManager::new(PrimaryShortcut::default());
        let saved_shortcut = configuration.settings.shortcut.primary.clone();
        let mut registrar = TauriShortcutRegistrar {
            app: app_handle.clone(),
        };
        let mut persistence = AppShortcutPersistence {
            app: &app_handle,
            base: configuration.settings.clone(),
            saved: None,
        };
        let startup = configuration.shortcut.activate_startup(
            saved_shortcut,
            &mut registrar,
            &mut persistence,
        );
        if let Some(saved) = persistence.saved {
            configuration.settings = saved;
        }
        configuration.settings.shortcut.primary = configuration.shortcut.active().clone();
        if startup == ShortcutStartupStatus::FallbackDefault {
            configuration.recovery = Some(SettingsRecoveryCode::InvalidFields);
        }
        startup
    };
    match startup {
        ShortcutStartupStatus::RegisteredSaved => {}
        ShortcutStartupStatus::FallbackDefault => {
            notices.push("shortcut_startup_fallback".to_string())
        }
        ShortcutStartupStatus::Failed => notices.push("shortcut_startup_failed".to_string()),
    }
    state
        .startup_notices
        .lock()
        .map_err(|_| "startup_notices_unavailable".to_string())?
        .extend(notices);
    Ok(startup)
}

fn spawn_auth_notification_bridge(app: AppHandle, client: Arc<CodexClient>, login_id: String) {
    let mut notifications = client.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            let Ok(notification) = notifications.recv().await else {
                return;
            };

            let method = notification
                .get("method")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let params = notification
                .get("params")
                .unwrap_or(&serde_json::Value::Null);

            match method {
                "account/login/completed" => {
                    let event_login_id = params
                        .get("loginId")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned);
                    if event_login_id
                        .as_deref()
                        .is_some_and(|value| value != login_id)
                    {
                        continue;
                    }

                    let success = params
                        .get("success")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let error = params
                        .get("error")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned);
                    if let Ok(mut pending) = app.state::<AppState>().pending_device_login.lock() {
                        if pending
                            .as_ref()
                            .is_some_and(|pending| pending.login_id == login_id)
                        {
                            *pending = None;
                        }
                    }

                    let _ = app.emit(
                        "login-completed",
                        LoginCompletedEvent {
                            login_id: event_login_id,
                            success,
                            error,
                        },
                    );
                    let _ = app.emit("auth-changed", AuthChangedEvent {});
                    return;
                }
                "account/updated" => {
                    let _ = app.emit("auth-changed", AuthChangedEvent {});
                }
                "codex/process/exited" => {
                    if let Ok(mut pending) = app.state::<AppState>().pending_device_login.lock() {
                        *pending = None;
                    }
                    let message = params
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Codex app-server exited unexpectedly.")
                        .to_string();
                    let _ = app.emit("codex-process-exited", CodexProcessEvent { message });
                    return;
                }
                _ => {}
            }
        }
    });
}

async fn capture_from_hotkey(app: AppHandle) -> Result<(), String> {
    struct PreparedCapture {
        token: SessionToken,
        selected_text: String,
        cursor: CursorPoint,
    }

    {
        let state = app.state::<AppState>();
        let active_turn = state.capture.lock().await.cancel_active_with_turn();
        if active_turn.is_some() {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                interrupt_active_turn(state.inner(), active_turn).await;
            });
        }
    }

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Codex Pencil window is missing.".to_string())?;
    let own_hwnd = window
        .hwnd()
        .map_err(|_| "Codex Pencil window handle is unavailable.".to_string())?
        .0 as isize;
    let mut target_platform = WindowsForegroundTargetPlatform;
    let prepare_app = app.clone();
    let focus_app = app.clone();

    let (_, prepared) = capture_before_widget_focus(
        &mut target_platform,
        own_hwnd,
        std::process::id(),
        move |target| {
            let app = prepare_app.clone();
            async move {
                let capture = clipboard::capture_selected_text().await?;
                let token = {
                    let state = app.state::<AppState>();
                    let mut store = state.capture.lock().await;
                    store
                        .capture(
                            Uuid::new_v4().to_string(),
                            capture.selected_text.clone(),
                            target,
                            capture.previous_text,
                            capture.owned_sequence,
                        )
                        .map_err(|error| error.code().to_string())?
                };
                Ok(PreparedCapture {
                    token,
                    selected_text: capture.selected_text,
                    cursor: capture.cursor,
                })
            }
        },
        move |prepared| show_main_window(&focus_app, Some(&prepared.cursor)),
    )
    .await?;

    let token = prepared.token.clone();
    if let Err(error) = app.emit(
        "selection-captured",
        SelectionCapturedEvent {
            session_id: prepared.token.session_id,
            generation: prepared.token.generation,
            selected_text: prepared.selected_text,
        },
    ) {
        let state = app.state::<AppState>();
        let active_turn = state
            .capture
            .lock()
            .await
            .cancel_with_active_turn(&token)
            .ok()
            .flatten();
        interrupt_active_turn(state.inner(), active_turn).await;
        return Err(format!("Could not notify window: {error}"));
    }

    Ok(())
}

fn show_main_window(app: &AppHandle, cursor: Option<&CursorPoint>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Codex Pencil window is missing.".to_string())?;

    if let Some(cursor) = cursor {
        let x = cursor.x.saturating_add(18).max(0);
        let y = cursor.y.saturating_add(18).max(0);
        window
            .set_position(Position::Physical(PhysicalPosition { x, y }))
            .map_err(|error| format!("Could not position window: {error}"))?;
    }

    window
        .show()
        .map_err(|error| format!("Could not show window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("Could not focus window: {error}"))?;
    Ok(())
}

fn hide_main_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Codex Pencil window is missing.".to_string())?;
    window
        .hide()
        .map_err(|error| format!("Could not hide window: {error}"))
}
