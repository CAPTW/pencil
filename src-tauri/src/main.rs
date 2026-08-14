#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apply_safety;
mod capture_session;
mod clipboard;
mod codex_binary;
mod codex_client;
mod prerequisites;
mod settings;
mod shortcut;
mod translation;
mod windows_apply;
mod windows_target;

#[cfg(test)]
mod p0_02_contract_tests;

#[cfg(all(test, windows))]
mod p0_02_windows_live_tests;

#[cfg(test)]
mod p1_01_contract_tests;

#[cfg(all(test, windows))]
mod p1_01_windows_live_tests;

use apply_safety::{apply_current_session, ApplyFailureReason, ApplyOutcome};
use capture_session::{CaptureSessionStore, SessionError, SessionToken};
use clipboard::CursorPoint;
use codex_client::{AuthStatus, CodexClient, CodexClientCache, DeviceLogin, RewriteResult};
use prerequisites::PrerequisiteReport;
use serde::Serialize;
use settings::{AppSettings, RewriteMode, SettingsRecoveryCode};
use shortcut::{
    dispatch_shortcut_trigger, PrimaryShortcut, ShortcutCandidate, ShortcutEventState,
    ShortcutManager, ShortcutPersistence, ShortcutRegistrar, ShortcutRegistrarError,
    ShortcutStartupStatus, ShortcutTriggerGate, ShortcutUpdateStatus,
};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{
    menu::{Menu, MenuItem},
    plugin::TauriPlugin,
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
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
    shortcut_trigger: StdMutex<ShortcutTriggerGate>,
    startup_notices: StdMutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            codex: CodexClientCache::default(),
            capture: Mutex::new(CaptureSessionStore::default()),
            configuration: StdMutex::new(ConfigurationState::default()),
            shortcut_trigger: StdMutex::new(ShortcutTriggerGate::default()),
            startup_notices: StdMutex::new(Vec::new()),
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

#[tauri::command]
async fn auth_status(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    ensure_codex(&state).await?.auth_status().await
}

#[tauri::command]
async fn start_device_login(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DeviceLogin, String> {
    let client = ensure_codex(&state).await?;
    let login = client.start_device_login().await?;
    spawn_auth_notification_bridge(app, client, login.login_id.clone());
    Ok(login)
}

#[tauri::command]
async fn cancel_device_login(login_id: String, state: State<'_, AppState>) -> Result<(), String> {
    ensure_codex(&state).await?.cancel_login(login_id).await
}

#[tauri::command]
async fn rewrite_selected_text(
    session_id: String,
    generation: u64,
    mode: RewriteMode,
    target_language: Option<TranslationTargetLanguage>,
    state: State<'_, AppState>,
) -> Result<RewriteResult, String> {
    let token = SessionToken {
        session_id,
        generation,
    };
    let intent = RewriteIntent::new(mode, target_language).map_err(str::to_string)?;
    {
        let configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        if configuration
            .settings
            .rewrite_intent()
            .map_err(str::to_string)?
            != intent
        {
            return Err("stale_rewrite_intent".to_string());
        }
    }
    let selected_text = {
        let mut capture = state.capture.lock().await;
        capture
            .begin_rewrite_for(&token, intent)
            .map_err(|error| error.code().to_string())?
    };

    let rewrite = match ensure_codex(&state).await {
        Ok(client) => client.rewrite(&selected_text, intent).await,
        Err(error) => Err(error),
    };

    let mut capture = state.capture.lock().await;
    match rewrite {
        Ok(result) => {
            capture
                .finish_rewrite_success_for(&token, intent)
                .map_err(|error| error.code().to_string())?;
            Ok(result)
        }
        Err(error) => match capture.finish_rewrite_failure_for(&token, intent) {
            Ok(()) => Err(error),
            Err(session_error) => Err(session_error.code().to_string()),
        },
    }
}

#[tauri::command]
async fn apply_replacement(
    session_id: String,
    generation: u64,
    replacement: String,
    mode: RewriteMode,
    target_language: Option<TranslationTargetLanguage>,
    restore_clipboard: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyOutcome, String> {
    let token = SessionToken {
        session_id,
        generation,
    };
    let intent = RewriteIntent::new(mode, target_language).map_err(str::to_string)?;
    let (current_intent, apply_format) = {
        let configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        (
            configuration
                .settings
                .rewrite_intent()
                .map_err(str::to_string)?,
            configuration.settings.translation.apply_format,
        )
    };
    if current_intent != intent {
        return Ok(ApplyOutcome::RejectedStale);
    }
    let mut capture = state.capture.lock().await;
    if let Err(error) = capture.validate_ready_intent(&token, intent) {
        return Ok(match error {
            SessionError::StaleSession | SessionError::StaleIntent => ApplyOutcome::RejectedStale,
            SessionError::InvalidState | SessionError::GenerationExhausted => {
                ApplyOutcome::Failed {
                    reason: ApplyFailureReason::InvalidSessionState,
                }
            }
        });
    }
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
    let mut platform = WindowsApplyPlatform::new(app);
    Ok(apply_current_session(
        &mut capture,
        &token,
        &final_replacement,
        restore_clipboard,
        &mut platform,
    ))
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
async fn save_settings(
    app: AppHandle,
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    settings.validate().map_err(str::to_string)?;
    let (previous_intent, next_intent) = {
        let mut configuration = state
            .configuration
            .lock()
            .map_err(|_| "configuration_unavailable".to_string())?;
        if settings.shortcut != configuration.settings.shortcut {
            return Err("shortcut_update_requires_transaction".to_string());
        }
        let previous_intent = configuration
            .settings
            .rewrite_intent()
            .map_err(str::to_string)?;
        let next_intent = settings.rewrite_intent().map_err(str::to_string)?;
        settings::save(&app, &settings)?;
        configuration.settings = settings.clone();
        configuration.recovery = None;
        (previous_intent, next_intent)
    };

    if previous_intent != next_intent {
        state
            .capture
            .lock()
            .await
            .invalidate_intent(next_intent)
            .map_err(|error| error.code().to_string())?;
    }
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
    {
        let mut capture = state.capture.lock().await;
        match (session_id, generation) {
            (Some(session_id), Some(generation)) => capture
                .cancel(&SessionToken {
                    session_id,
                    generation,
                })
                .map_err(|error| error.code().to_string())?,
            (None, None) if !capture.has_active() => {}
            _ => return Err("session_token_required".to_string()),
        }
    }
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

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(global_shortcut_plugin())
        .invoke_handler(tauri::generate_handler![
            apply_replacement,
            auth_status,
            cancel_device_login,
            check_prerequisites,
            dismiss_window,
            load_settings,
            reset_primary_shortcut,
            rewrite_selected_text,
            save_settings,
            start_device_login,
            take_startup_notices,
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
                    let mut capture = state.capture.lock().await;
                    capture.cancel_active();
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Codex Pencil");
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
                state.capture.lock().await.cancel_active();
            });
        }
        "account" => {
            let _ = show_main_window(app, None);
        }
        "settings" => {
            let _ = show_main_window(app, None);
            let _ = app.emit("open-settings", ());
        }
        "quit" => app.exit(0),
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
    let loaded = match settings::load_state(&app_handle) {
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
        state.capture.lock().await.cancel_active();
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
        let _ = state.capture.lock().await.cancel(&token);
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
