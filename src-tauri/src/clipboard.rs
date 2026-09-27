use std::future::Future;
use tokio::time::{sleep, Duration};

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
    pub cursor: CursorPoint,
    /// Set only by the qualified native reader; Apply for such a capture is
    /// always Copy-only and never touches the editor.
    pub(crate) native_edit: Option<crate::capture_session::NativeEditBinding>,
}

const CAPTURE_TARGET_CHANGED: &str =
    "capture_target_changed: The foreground window changed before the selection was read. Try again.";

pub(crate) async fn capture_selected_text_for(target: crate::capture_session::WindowTarget) -> Result<ClipboardCapture, String> {
    wait_for_capture_modifiers_released().await?;
    if crate::windows_apply::foreground_window_handle() != target.hwnd ||
        crate::windows_apply::window_process_id(target.hwnd) != Some(target.pid) {
        return Err(CAPTURE_TARGET_CHANGED.to_string());
    }
    let capture = capture_supported_selection(target.hwnd)?;
    if crate::windows_apply::foreground_window_handle() != target.hwnd ||
        crate::windows_apply::window_process_id(target.hwnd) != Some(target.pid) ||
        capture.native_edit.as_ref().is_some_and(|binding| binding.pid != target.pid) {
        return Err(CAPTURE_TARGET_CHANGED.to_string());
    }
    Ok(capture)
}

pub async fn capture_selected_text() -> Result<ClipboardCapture, String> {
    wait_for_capture_modifiers_released().await?;
    capture_supported_selection(crate::windows_apply::foreground_window_handle())
}

fn capture_supported_selection(top_level: isize) -> Result<ClipboardCapture, String> {
    // Generic Ctrl+C cannot identify password fields or bind the copied text to
    // a focused editor. Only the bounded standard Edit reader is admitted.
    let selection = crate::windows_target::read_supported_selection(top_level)?;
    let cursor = current_cursor_position()?;
    Ok(ClipboardCapture {
        selected_text: selection.text,
        cursor,
        native_edit: Some(selection.binding),
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

#[cfg(test)]
mod clipboard_policy_tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::future::ready;

    #[tokio::test]
    async fn capture_waits_for_physical_modifiers_before_reading() {
        let samples = RefCell::new(VecDeque::from([true, true, false]));
        let waits = Cell::new(0usize);
        let reads = Cell::new(0usize);

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
        reads.set(reads.get() + 1);

        assert_eq!(waits.get(), 2);
        assert_eq!(reads.get(), 1);
    }

    #[tokio::test]
    async fn held_modifiers_fail_closed_before_reading() {
        let waits = Cell::new(0usize);
        let reads = Cell::new(0usize);

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
            reads.set(reads.get() + 1);
        }

        assert_eq!(result, Err("shortcut_modifiers_still_pressed"));
        assert_eq!(waits.get(), 2);
        assert_eq!(reads.get(), 0);
    }
}

/// Tests read back what the product wrote; the product itself never reads the clipboard.
#[cfg(test)]
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

#[cfg(all(test, windows))]
pub(crate) fn clipboard_sequence_number() -> u32 {
    use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;

    unsafe { GetClipboardSequenceNumber() }
}

#[cfg(all(test, not(windows)))]
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
