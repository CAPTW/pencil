use crate::shortcut::{
    dispatch_shortcut_trigger, PrimaryShortcut, ShortcutCandidate, ShortcutEventState,
    ShortcutManager, ShortcutPersistence, ShortcutRegistrar, ShortcutRegistrarError,
    ShortcutStartupStatus, ShortcutTriggerGate, ShortcutUpdateStatus,
};
use crate::{create_tray, handle_tray_menu_action};
use std::collections::HashMap;
use std::mem::{size_of, MaybeUninit};
use std::ptr::null_mut;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{Listener, Manager};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, SendInput, UnregisterHotKey, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_REMOVE, WM_HOTKEY};

const FIRST_HOTKEY_ID: i32 = 0x5101;
const CONFLICT_HOTKEY_ID: i32 = 0x6201;
const LIVE_WAIT: Duration = Duration::from_secs(2);
const ABSENT_WAIT: Duration = Duration::from_millis(350);

struct LiveShortcutRegistrar {
    next_id: i32,
    registrations: HashMap<String, i32>,
}

impl LiveShortcutRegistrar {
    fn new() -> Self {
        Self {
            next_id: FIRST_HOTKEY_ID,
            registrations: HashMap::new(),
        }
    }

    fn id_for(&self, shortcut: &PrimaryShortcut) -> Option<i32> {
        self.registrations
            .get(&shortcut.registration_string())
            .copied()
    }
}

impl ShortcutRegistrar for LiveShortcutRegistrar {
    fn register(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError> {
        let registration = shortcut.registration_string();
        if self.registrations.contains_key(&registration) {
            return Ok(());
        }

        let (modifiers, virtual_key) = hotkey_parts(shortcut)?;
        let id = self.next_id;
        self.next_id += 1;
        let registered = unsafe {
            RegisterHotKey(
                null_mut(),
                id,
                modifiers | MOD_NOREPEAT,
                u32::from(virtual_key),
            )
        };
        if registered == 0 {
            return Err(
                if std::io::Error::last_os_error().raw_os_error() == Some(1409) {
                    ShortcutRegistrarError::Conflict
                } else {
                    ShortcutRegistrarError::Failed
                },
            );
        }

        self.registrations.insert(registration, id);
        Ok(())
    }

    fn unregister(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ShortcutRegistrarError> {
        let Some(id) = self.registrations.remove(&shortcut.registration_string()) else {
            return Err(ShortcutRegistrarError::Failed);
        };
        if unsafe { UnregisterHotKey(null_mut(), id) } == 0 {
            return Err(ShortcutRegistrarError::Failed);
        }
        Ok(())
    }
}

impl Drop for LiveShortcutRegistrar {
    fn drop(&mut self) {
        for id in self.registrations.values().copied().collect::<Vec<_>>() {
            unsafe {
                UnregisterHotKey(null_mut(), id);
            }
        }
        self.registrations.clear();
    }
}

#[derive(Default)]
struct MemoryPersistence {
    saved: Option<PrimaryShortcut>,
}

impl ShortcutPersistence for MemoryPersistence {
    fn persist(&mut self, shortcut: &PrimaryShortcut) -> Result<(), ()> {
        self.saved = Some(shortcut.clone());
        Ok(())
    }
}

struct ConflictGuard {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ConflictGuard {
    fn hold(shortcut: PrimaryShortcut) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let result = hotkey_parts(&shortcut).and_then(|(modifiers, virtual_key)| {
                if unsafe {
                    RegisterHotKey(
                        null_mut(),
                        CONFLICT_HOTKEY_ID,
                        modifiers | MOD_NOREPEAT,
                        u32::from(virtual_key),
                    )
                } == 0
                {
                    Err(ShortcutRegistrarError::Failed)
                } else {
                    Ok(())
                }
            });
            let registered = result.is_ok();
            let _ = ready_tx.send(result);
            if registered {
                let _ = stop_rx.recv_timeout(Duration::from_secs(15));
                unsafe {
                    UnregisterHotKey(null_mut(), CONFLICT_HOTKEY_ID);
                }
            }
        });

        match ready_rx.recv_timeout(LIVE_WAIT) {
            Ok(Ok(())) => Ok(Self {
                stop: Some(stop_tx),
                thread: Some(handle),
            }),
            Ok(Err(_)) => {
                let _ = handle.join();
                Err("conflict_candidate_registration_failed".to_string())
            }
            Err(_) => {
                let _ = stop_tx.send(());
                let _ = handle.join();
                Err("conflict_helper_timeout".to_string())
            }
        }
    }
}

impl Drop for ConflictGuard {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn hotkey_parts(shortcut: &PrimaryShortcut) -> Result<(u32, u16), ShortcutRegistrarError> {
    let mut modifiers = 0;
    for modifier in &shortcut.modifiers {
        modifiers |= match modifier.as_str() {
            "CTRL" => MOD_CONTROL,
            "ALT" => MOD_ALT,
            "SHIFT" => MOD_SHIFT,
            "WIN" => MOD_WIN,
            _ => return Err(ShortcutRegistrarError::Failed),
        };
    }

    let bytes = shortcut.key.as_bytes();
    let virtual_key = if bytes.len() == 1 && bytes[0].is_ascii_alphanumeric() {
        u16::from(bytes[0])
    } else {
        return Err(ShortcutRegistrarError::Failed);
    };
    Ok((modifiers, virtual_key))
}

fn keyboard_input(virtual_key: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_chord(shortcut: &PrimaryShortcut) -> Result<(), String> {
    let (_, virtual_key) = hotkey_parts(shortcut).map_err(|_| "unsupported_live_key")?;
    let mut modifier_keys = Vec::new();
    for modifier in &shortcut.modifiers {
        modifier_keys.push(match modifier.as_str() {
            "CTRL" => VK_CONTROL,
            "ALT" => VK_MENU,
            "SHIFT" => VK_SHIFT,
            "WIN" => VK_LWIN,
            _ => return Err("unsupported_live_modifier".to_string()),
        });
    }

    let mut inputs = modifier_keys
        .iter()
        .copied()
        .map(|key| keyboard_input(key, false))
        .collect::<Vec<_>>();
    inputs.push(keyboard_input(virtual_key, false));
    inputs.push(keyboard_input(virtual_key, true));
    inputs.extend(
        modifier_keys
            .iter()
            .rev()
            .copied()
            .map(|key| keyboard_input(key, true)),
    );

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err("send_input_incomplete".to_string());
    }
    Ok(())
}

fn wait_for_hotkey(timeout: Duration) -> Option<i32> {
    let deadline = Instant::now() + timeout;
    loop {
        let mut message = MaybeUninit::<MSG>::zeroed();
        while unsafe {
            PeekMessageW(
                message.as_mut_ptr(),
                null_mut(),
                WM_HOTKEY,
                WM_HOTKEY,
                PM_REMOVE,
            )
        } != 0
        {
            let message = unsafe { message.assume_init() };
            if message.message == WM_HOTKEY {
                return Some(message.wParam as i32);
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn fire_active_once(
    registrar: &LiveShortcutRegistrar,
    shortcut: &PrimaryShortcut,
    gate: &mut ShortcutTriggerGate,
    callback_count: &mut usize,
) -> Result<(), String> {
    let expected_id = registrar
        .id_for(shortcut)
        .ok_or_else(|| "active_shortcut_not_registered".to_string())?;
    send_chord(shortcut)?;
    let actual_id =
        wait_for_hotkey(LIVE_WAIT).ok_or_else(|| "global_shortcut_message_timeout".to_string())?;
    if actual_id != expected_id {
        return Err("unexpected_global_shortcut_id".to_string());
    }
    dispatch_shortcut_trigger(gate, true, ShortcutEventState::Pressed, || {
        *callback_count += 1;
    });
    dispatch_shortcut_trigger(gate, true, ShortcutEventState::Released, || {
        *callback_count += 1;
    });
    Ok(())
}

fn candidate(key: &str) -> ShortcutCandidate {
    ShortcutCandidate {
        modifiers: vec!["CTRL".to_string(), "SHIFT".to_string()],
        key: key.to_string(),
    }
}

#[test]
#[ignore = "requires an interactive Windows desktop and actual global hotkey registration"]
fn p1_01_windows_live_shortcut_registration_conflict_restart_and_reset() {
    let default = PrimaryShortcut::default();
    let mut persistence = MemoryPersistence::default();
    let mut gate = ShortcutTriggerGate::default();
    let mut callback_count = 0;

    let saved_non_default = {
        let mut registrar = LiveShortcutRegistrar::new();
        let mut manager = ShortcutManager::new(default.clone());
        assert_eq!(
            manager.activate_startup(default.clone(), &mut registrar, &mut persistence),
            ShortcutStartupStatus::RegisteredSaved
        );

        fire_active_once(&registrar, manager.active(), &mut gate, &mut callback_count)
            .expect("default global shortcut should dispatch");
        assert_eq!(callback_count, 1);

        let update = manager.update(candidate("H"), &mut registrar, &mut persistence);
        assert_eq!(update.status, ShortcutUpdateStatus::Applied);
        let non_default = manager.active().clone();
        assert_eq!(non_default.display, "Ctrl+Shift+H");

        send_chord(&default).expect("old shortcut input should be deliverable");
        assert_eq!(wait_for_hotkey(ABSENT_WAIT), None);
        fire_active_once(&registrar, &non_default, &mut gate, &mut callback_count)
            .expect("new global shortcut should dispatch");
        assert_eq!(callback_count, 2);

        let conflict = PrimaryShortcut::from_candidate(candidate("J"))
            .expect("synthetic conflict candidate should be valid");
        let _conflict_guard =
            ConflictGuard::hold(conflict).expect("conflict helper should hold the candidate");
        let update = manager.update(candidate("J"), &mut registrar, &mut persistence);
        assert_eq!(update.status, ShortcutUpdateStatus::Conflict);
        assert_eq!(manager.active(), &non_default);
        fire_active_once(&registrar, manager.active(), &mut gate, &mut callback_count)
            .expect("prior shortcut should survive a conflict");
        assert_eq!(callback_count, 3);
        persistence
            .saved
            .clone()
            .expect("successful update should persist the shortcut")
    };

    let mut restarted_registrar = LiveShortcutRegistrar::new();
    let mut restarted_manager = ShortcutManager::new(default.clone());
    assert_eq!(
        restarted_manager.activate_startup(
            saved_non_default,
            &mut restarted_registrar,
            &mut persistence,
        ),
        ShortcutStartupStatus::RegisteredSaved
    );
    fire_active_once(
        &restarted_registrar,
        restarted_manager.active(),
        &mut gate,
        &mut callback_count,
    )
    .expect("persisted shortcut should register after state recreation");
    assert_eq!(callback_count, 4);

    let reset = restarted_manager.update(
        default.candidate(),
        &mut restarted_registrar,
        &mut persistence,
    );
    assert_eq!(reset.status, ShortcutUpdateStatus::Applied);
    assert_eq!(restarted_manager.active(), &default);
    fire_active_once(
        &restarted_registrar,
        restarted_manager.active(),
        &mut gate,
        &mut callback_count,
    )
    .expect("default shortcut should work after reset");
    assert_eq!(callback_count, 5);
}

#[test]
#[ignore = "requires an interactive Windows desktop and a live Tauri tray/window runtime"]
#[allow(deprecated)]
fn p1_01_windows_live_tray_settings_action_shows_focuses_and_emits_settings_mode() {
    let mut app = tauri::Builder::default()
        .any_thread()
        .build(tauri::generate_context!())
        .expect("live Tauri acceptance app should build");
    create_tray(&mut app).expect("production tray should be created");
    let app_handle = app.handle().clone();
    let (settings_tx, settings_rx) = mpsc::sync_channel(1);
    let listener = app_handle.listen("open-settings", move |_| {
        let _ = settings_tx.send(());
    });

    let startup_deadline = Instant::now() + LIVE_WAIT;
    while app_handle.get_webview_window("main").is_none() && Instant::now() < startup_deadline {
        app.run_iteration(|_, _| {});
        thread::sleep(Duration::from_millis(10));
    }
    let Some(window) = app_handle.get_webview_window("main") else {
        app_handle.unlisten(listener);
        drop(app_handle.remove_tray_by_id("main"));
        app.cleanup_before_exit();
        panic!("configured main window should exist");
    };

    handle_tray_menu_action(&app_handle, "settings");
    let deadline = Instant::now() + LIVE_WAIT;
    let mut settings_event_received = false;
    let mut window_visible = false;
    let mut window_focused = false;
    while Instant::now() < deadline {
        app.run_iteration(|_, _| {});
        if settings_rx.try_recv().is_ok() {
            settings_event_received = true;
        }
        window_visible = matches!(window.is_visible(), Ok(true));
        window_focused = matches!(window.is_focused(), Ok(true));
        if settings_event_received && window_visible && window_focused {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    app_handle.unlisten(listener);
    let _ = window.hide();
    drop(app_handle.remove_tray_by_id("main"));
    app.cleanup_before_exit();

    assert!(
        settings_event_received,
        "settings mode event should be emitted"
    );
    assert!(window_visible, "settings action should show the widget");
    assert!(window_focused, "settings action should focus the widget");
}
