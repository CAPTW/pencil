#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apply_safety;
mod capture_session;
mod clipboard;
mod codex_binary;
mod codex_client;
mod prerequisites;
mod settings;
mod windows_apply;
mod windows_target;

#[cfg(test)]
mod p0_02_contract_tests;

#[cfg(all(test, windows))]
mod p0_02_windows_live_tests;

use apply_safety::{apply_current_session, ApplyOutcome};
use capture_session::{CaptureSessionStore, SessionToken};
use clipboard::CursorPoint;
use codex_client::{AuthStatus, CodexClient, CodexClientCache, DeviceLogin, RewriteResult};
use prerequisites::PrerequisiteReport;
use serde::Serialize;
use settings::{AppSettings, RewriteMode};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, Position, State, WindowEvent,
};
use tokio::sync::Mutex;
use uuid::Uuid;
use windows_apply::WindowsApplyPlatform;
use windows_target::{capture_before_widget_focus, WindowsForegroundTargetPlatform};

struct AppState {
    codex: CodexClientCache,
    capture: Mutex<CaptureSessionStore>,
    startup_notices: Mutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            codex: CodexClientCache::default(),
            capture: Mutex::new(CaptureSessionStore::default()),
            startup_notices: Mutex::new(Vec::new()),
        }
    }
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
    state: State<'_, AppState>,
) -> Result<RewriteResult, String> {
    let token = SessionToken {
        session_id,
        generation,
    };
    let selected_text = {
        let mut capture = state.capture.lock().await;
        capture
            .begin_rewrite(&token)
            .map_err(|error| error.code().to_string())?
    };

    let rewrite = match ensure_codex(&state).await {
        Ok(client) => client.rewrite(&selected_text, mode).await,
        Err(error) => Err(error),
    };

    let mut capture = state.capture.lock().await;
    match rewrite {
        Ok(result) => {
            capture
                .finish_rewrite_success(&token)
                .map_err(|error| error.code().to_string())?;
            Ok(result)
        }
        Err(error) => match capture.finish_rewrite_failure(&token) {
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
    restore_clipboard: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyOutcome, String> {
    let token = SessionToken {
        session_id,
        generation,
    };
    let mut capture = state.capture.lock().await;
    let mut platform = WindowsApplyPlatform::new(app);
    Ok(apply_current_session(
        &mut capture,
        &token,
        &replacement,
        restore_clipboard,
        &mut platform,
    ))
}

#[tauri::command]
fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    settings::save(&app, &settings)
}

#[tauri::command]
fn check_prerequisites() -> PrerequisiteReport {
    prerequisites::check()
}

#[tauri::command]
async fn take_startup_notices(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut notices = state.startup_notices.lock().await;
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

async fn ensure_codex(state: &State<'_, AppState>) -> Result<Arc<CodexClient>, String> {
    state.codex.get().await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            apply_replacement,
            auth_status,
            cancel_device_login,
            check_prerequisites,
            dismiss_window,
            load_settings,
            rewrite_selected_text,
            save_settings,
            start_device_login,
            take_startup_notices
        ])
        .setup(|app| {
            create_tray(app)?;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            if let Err(message) = register_global_hotkey(app) {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    state.startup_notices.lock().await.push(message);
                    let _ = show_main_window(&app_handle, None);
                });
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
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &account, &quit])?;

    TrayIconBuilder::with_id("main")
        .tooltip("Codex Pencil")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
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
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn register_global_hotkey(app: &mut tauri::App) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let hotkey = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyG);
    let handler_hotkey = hotkey.clone();

    app.handle()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &handler_hotkey && event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(message) = capture_from_hotkey(app.clone()).await {
                                let _ = app.emit("capture-error", CaptureErrorEvent { message });
                                let _ = show_main_window(&app, None);
                            }
                        });
                    }
                })
                .build(),
        )
        .map_err(|error| format!("Could not initialize global shortcut support: {error}"))?;

    app.global_shortcut()
        .register(hotkey)
        .map_err(|error| format!("Ctrl+Shift+G could not be registered. Another app may already be using it. Details: {error}"))?;
    Ok(())
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
