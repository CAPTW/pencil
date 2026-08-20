use std::future::Future;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

const UNSUPPORTED_NON_TEXT_CLIPBOARD: &str = "unsupported_non_text_clipboard";
const MODIFIER_RELEASE_ERROR: &str = "shortcut_modifiers_still_pressed";
const MODIFIER_RELEASE_CHECKS: usize = 81;
const MODIFIER_RELEASE_POLL: Duration = Duration::from_millis(20);

#[derive(Clone, Debug)]
pub struct CursorPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug)]
pub struct ClipboardCapture {
    pub selected_text: String,
    pub previous_text: Option<String>,
    pub owned_sequence: Option<u32>,
    pub cursor: CursorPoint,
}

pub async fn capture_selected_text() -> Result<ClipboardCapture, String> {
    wait_for_capture_modifiers_released().await?;
    let previous_text = supported_text_snapshot_before_capture()?;
    let cursor = current_cursor_position()?;
    let sentinel = format!("__CODEX_PENCIL_SENTINEL_{}__", Uuid::new_v4());

    // This is intentionally a one-shot clipboard workflow. The app never
    // subscribes to clipboard changes or stores clipboard history.
    write_clipboard_text(&sentinel)?;
    let sentinel_sequence = clipboard_sequence_number();
    if let Err(error) = send_copy_shortcut() {
        let _ = restore_clipboard_after_capture(previous_text.as_deref(), sentinel_sequence);
        return Err(error);
    }

    let mut copied = String::new();
    let mut observed_sequence = sentinel_sequence;
    for _ in 0..8 {
        sleep(Duration::from_millis(55)).await;
        let sequence_before_read = clipboard_sequence_number();
        if let Ok(candidate) = read_clipboard_text() {
            let sequence_after_read = clipboard_sequence_number();
            if sequence_before_read == sequence_after_read {
                copied = candidate;
                observed_sequence = sequence_after_read;
                if copied != sentinel {
                    break;
                }
            }
        }
    }

    let owned_sequence =
        restore_clipboard_after_capture(previous_text.as_deref(), observed_sequence)?;

    if copied == sentinel || copied.trim().is_empty() {
        return Err(
            "No text selected. Select text in another app, then press Ctrl+Shift+G.".to_string(),
        );
    }

    Ok(ClipboardCapture {
        selected_text: copied,
        previous_text,
        owned_sequence,
        cursor,
    })
}

async fn wait_for_capture_modifiers_released() -> Result<(), String> {
    wait_for_modifier_release_with(
        any_user_modifier_down,
        || sleep(MODIFIER_RELEASE_POLL),
        MODIFIER_RELEASE_CHECKS,
    )
    .await
    .map_err(str::to_string)
}

async fn wait_for_modifier_release_with<C, P, F>(
    mut any_modifier_down: C,
    mut pause: P,
    max_checks: usize,
) -> Result<(), &'static str>
where
    C: FnMut() -> bool,
    P: FnMut() -> F,
    F: Future<Output = ()>,
{
    for check in 0..max_checks {
        if !any_modifier_down() {
            return Ok(());
        }
        if check + 1 < max_checks {
            pause().await;
        }
    }
    Err(MODIFIER_RELEASE_ERROR)
}

#[cfg(windows)]
fn any_user_modifier_down() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };

    [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
        .into_iter()
        .any(|key| unsafe { GetAsyncKeyState(key as i32) as u16 & 0x8000 != 0 })
}

#[cfg(not(windows))]
fn any_user_modifier_down() -> bool {
    false
}

#[cfg(windows)]
fn supported_text_snapshot_before_capture() -> Result<Option<String>, String> {
    use windows_sys::Win32::System::DataExchange::{
        CountClipboardFormats, IsClipboardFormatAvailable,
    };
    const CF_UNICODETEXT: u32 = 13;

    let format_count = unsafe { CountClipboardFormats() };
    let unicode_text_available = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) } != 0;
    classify_supported_clipboard(format_count, unicode_text_available, || {
        read_clipboard_text().ok()
    })
}

#[cfg(not(windows))]
fn supported_text_snapshot_before_capture() -> Result<Option<String>, String> {
    Ok(read_clipboard_text().ok())
}

fn classify_supported_clipboard<F>(
    format_count: i32,
    unicode_text_available: bool,
    read_text: F,
) -> Result<Option<String>, String>
where
    F: FnOnce() -> Option<String>,
{
    if unicode_text_available {
        return read_text()
            .map(Some)
            .ok_or_else(|| "clipboard_text_unreadable".to_string());
    }
    if format_count > 0 {
        return Err(UNSUPPORTED_NON_TEXT_CLIPBOARD.to_string());
    }
    Ok(None)
}

#[cfg(test)]
mod clipboard_policy_tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::future::ready;

    #[test]
    fn empty_clipboard_needs_no_snapshot_or_mutation() {
        let read = Cell::new(false);
        let result = classify_supported_clipboard(0, false, || {
            read.set(true);
            None
        });
        assert_eq!(result, Ok(None));
        assert!(!read.get());
    }

    #[test]
    fn readable_unicode_text_is_the_supported_restore_snapshot_even_when_mixed() {
        let result = classify_supported_clipboard(4, true, || Some("synthetic text".to_string()));
        assert_eq!(result, Ok(Some("synthetic text".to_string())));
    }

    #[test]
    fn unreadable_advertised_text_and_pure_non_text_both_fail_closed() {
        assert_eq!(
            classify_supported_clipboard(2, true, || None),
            Err("clipboard_text_unreadable".to_string())
        );
        assert_eq!(
            classify_supported_clipboard(1, false, || {
                panic!("pure non-text classification must not attempt a text read")
            }),
            Err(UNSUPPORTED_NON_TEXT_CLIPBOARD.to_string())
        );
    }

    #[tokio::test]
    async fn capture_waits_for_physical_modifiers_before_copy_injection() {
        let samples = RefCell::new(VecDeque::from([true, true, false]));
        let waits = Cell::new(0usize);
        let copy_injections = Cell::new(0usize);

        wait_for_modifier_release_with(
            || samples.borrow_mut().pop_front().unwrap_or(false),
            || {
                waits.set(waits.get() + 1);
                ready(())
            },
            4,
        )
        .await
        .expect("the bounded synthetic modifier sequence should release");
        copy_injections.set(copy_injections.get() + 1);

        assert_eq!(waits.get(), 2);
        assert_eq!(copy_injections.get(), 1);
    }

    #[tokio::test]
    async fn held_modifiers_fail_closed_before_copy_injection() {
        let waits = Cell::new(0usize);
        let copy_injections = Cell::new(0usize);

        let result = wait_for_modifier_release_with(
            || true,
            || {
                waits.set(waits.get() + 1);
                ready(())
            },
            3,
        )
        .await;
        if result.is_ok() {
            copy_injections.set(copy_injections.get() + 1);
        }

        assert_eq!(result, Err("shortcut_modifiers_still_pressed"));
        assert_eq!(waits.get(), 2);
        assert_eq!(copy_injections.get(), 0);
    }
}

pub(crate) fn read_clipboard_text() -> Result<String, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Clipboard unavailable: {error}"))?;
    clipboard
        .get_text()
        .map_err(|error| format!("Clipboard does not contain text: {error}"))
}

pub(crate) fn write_clipboard_text(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Clipboard unavailable: {error}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|error| format!("Could not write clipboard: {error}"))
}

fn restore_clipboard_after_capture(
    previous_text: Option<&str>,
    observed_sequence: u32,
) -> Result<Option<u32>, String> {
    if clipboard_sequence_number() != observed_sequence {
        return Ok(None);
    }

    if let Some(text) = previous_text {
        write_clipboard_text(text)?;
    } else {
        // arboard exposes text clipboard contents only. If the previous
        // clipboard was an image/file/rich format, full-fidelity restore is not
        // practical here; clear our sentinel/selection text instead.
        write_clipboard_text("")?;
    }
    Ok(Some(clipboard_sequence_number()))
}

#[cfg(windows)]
pub(crate) fn clipboard_sequence_number() -> u32 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

    unsafe { GetClipboardSequenceNumber() }
}

#[cfg(not(windows))]
pub(crate) fn clipboard_sequence_number() -> u32 {
    0
}

#[cfg(windows)]
fn current_cursor_position() -> Result<CursorPoint, String> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        return Err("Could not read cursor position.".to_string());
    }

    Ok(CursorPoint {
        x: point.x,
        y: point.y,
    })
}

#[cfg(not(windows))]
fn current_cursor_position() -> Result<CursorPoint, String> {
    Ok(CursorPoint { x: 80, y: 80 })
}

#[cfg(windows)]
fn send_copy_shortcut() -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_C;
    if send_ctrl_key(VK_C as u16) == 4 {
        Ok(())
    } else {
        Err("Could not send keyboard shortcut.".to_string())
    }
}

#[cfg(not(windows))]
fn send_copy_shortcut() -> Result<(), String> {
    Err("Clipboard capture is implemented for Windows.".to_string())
}

#[cfg(windows)]
pub(crate) fn send_paste_shortcut_count() -> u32 {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_V;
    send_ctrl_key(VK_V as u16)
}

#[cfg(not(windows))]
pub(crate) fn send_paste_shortcut_count() -> u32 {
    0
}

#[cfg(windows)]
fn send_ctrl_key(key: u16) -> u32 {
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };

    fn keyboard_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let mut inputs = [
        keyboard_input(VK_CONTROL as u16, 0),
        keyboard_input(key, 0),
        keyboard_input(key, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL as u16, KEYEVENTF_KEYUP),
    ];

    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            size_of::<INPUT>() as i32,
        )
    }
}
