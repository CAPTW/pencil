#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod codex_binary;
mod codex_client;
mod prerequisites;
mod settings;

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

struct AppState {
    codex: CodexClientCache,
    capture: Mutex<Option<CapturedSelection>>,
    startup_notices: Mutex<Vec<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            codex: CodexClientCache::default(),
            capture: Mutex::new(None),
            startup_notices: Mutex::new(Vec::new()),
        }
    }
}

#[derive(Clone, Debug)]
struct CapturedSelection {
    selected_text: String,
    previous_clipboard: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectionCapturedEvent {
    session_id: String,
    char_count: usize,
    cursor: CursorPayload,
}

#[derive(Clone, Debug, Serialize)]
struct CursorPayload {
    x: i32,
    y: i32,
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
async fn start_device_login(app: AppHandle, state: State<'_, AppState>) -> Result<DeviceLogin, String> {
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
async fn rewrite_selected_text(mode: RewriteMode, state: State<'_, AppState>) -> Result<RewriteResult, String> {
    let selected_text = {
        let capture = state.capture.lock().await;
        capture
            .as_ref()
            .map(|capture| capture.selected_text.clone())
            .ok_or_else(|| "Press Ctrl+Shift+G after selecting text first.".to_string())?
    };

    ensure_codex(&state).await?.rewrite(&selected_text, mode).await
}

#[tauri::command]
async fn apply_replacement(
    replacement: String,
    restore_clipboard: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let previous_clipboard = {
        let capture = state.capture.lock().await;
        capture.as_ref().and_then(|capture| capture.previous_clipboard.clone())
    };

    clipboard::paste_replacement(&replacement, previous_clipboard.as_deref(), restore_clipboard).await?;

    {
        let mut capture = state.capture.lock().await;
        *capture = None;
    }

    hide_main_window(&app)?;
    Ok(())
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
async fn dismiss_window(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut capture = state.capture.lock().await;
        *capture = None;
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
                    *capture = None;
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
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    let hotkey = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyG);
    let handler_hotkey = hotkey.clone();

    app.handle().plugin(
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

            let method = notification.get("method").and_then(serde_json::Value::as_str).unwrap_or_default();
            let params = notification.get("params").unwrap_or(&serde_json::Value::Null);

            match method {
                "account/login/completed" => {
                    let event_login_id = params
                        .get("loginId")
                        .and_then(serde_json::Value::as_str)
                        .map(ToOwned::to_owned);
                    if event_login_id.as_deref().is_some_and(|value| value != login_id) {
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
    let capture = clipboard::capture_selected_text().await?;
    let char_count = capture.selected_text.chars().count();
    let cursor = capture.cursor.clone();
    let session_id = Uuid::new_v4().to_string();

    {
        let state = app.state::<AppState>();
        let mut slot = state.capture.lock().await;
        *slot = Some(CapturedSelection {
            selected_text: capture.selected_text,
            previous_clipboard: capture.previous_text,
        });
    }

    show_main_window(&app, Some(&cursor))?;
    app.emit(
        "selection-captured",
        SelectionCapturedEvent {
            session_id,
            char_count,
            cursor: CursorPayload {
                x: cursor.x,
                y: cursor.y,
            },
        },
    )
    .map_err(|error| format!("Could not notify window: {error}"))?;

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

    window.show().map_err(|error| format!("Could not show window: {error}"))?;
    window.set_focus().map_err(|error| format!("Could not focus window: {error}"))?;
    Ok(())
}

fn hide_main_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Codex Pencil window is missing.".to_string())?;
    window.hide().map_err(|error| format!("Could not hide window: {error}"))
}
