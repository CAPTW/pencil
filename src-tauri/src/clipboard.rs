use tokio::time::{sleep, Duration};
use uuid::Uuid;

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
    let cursor = current_cursor_position()?;
    let previous_text = read_clipboard_text().ok();
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
