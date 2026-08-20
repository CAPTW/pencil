use crate::clipboard::{capture_selected_text, clipboard_sequence_number};
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, IsClipboardFormatAvailable,
    OpenClipboard,
};

const CF_UNICODETEXT: u32 = 13;

enum OriginalClipboard {
    Empty,
    Text(String),
}

impl OriginalClipboard {
    fn capture_supported_only() -> Result<Self, &'static str> {
        let count = unsafe { CountClipboardFormats() };
        let has_text = unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) } != 0;
        if count == 0 {
            return Ok(Self::Empty);
        }
        if !has_text {
            return Err("live_clipboard_precondition_unsupported");
        }
        Clipboard::new()
            .ok()
            .and_then(|mut clipboard| clipboard.get_text().ok())
            .map(Self::Text)
            .ok_or("live_clipboard_precondition_unreadable")
    }

    fn restore(&self) {
        match self {
            Self::Empty => clear_clipboard(),
            Self::Text(text) => {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text.clone());
                }
            }
        }
    }
}

struct ClipboardRestore(OriginalClipboard);

impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        self.0.restore();
    }
}

fn clear_clipboard() {
    let opened = unsafe { OpenClipboard(std::ptr::null_mut()) } != 0;
    if opened {
        unsafe {
            EmptyClipboard();
            CloseClipboard();
        }
    }
}

#[tokio::test]
#[ignore = "requires exclusive access to the interactive Windows clipboard"]
async fn p1_03_windows_live_image_only_clipboard_aborts_before_mutation() {
    let restore = ClipboardRestore(
        OriginalClipboard::capture_supported_only()
            .unwrap_or_else(|_| panic!("live clipboard precondition is not safely restorable")),
    );
    let pixels = vec![17u8, 34, 51, 255];
    let mut clipboard =
        Clipboard::new().unwrap_or_else(|_| panic!("interactive clipboard is unavailable"));
    clipboard
        .set_image(ImageData {
            width: 1,
            height: 1,
            bytes: Cow::Owned(pixels.clone()),
        })
        .unwrap_or_else(|_| panic!("synthetic image clipboard fixture could not be installed"));
    drop(clipboard);

    let sequence_before = clipboard_sequence_number();
    let formats_before = unsafe { CountClipboardFormats() };
    assert!(formats_before > 0);
    assert_eq!(unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) }, 0);

    let result = capture_selected_text().await;
    assert!(matches!(result, Err(error) if error == "unsupported_non_text_clipboard"));
    assert_eq!(clipboard_sequence_number(), sequence_before);
    assert_eq!(unsafe { CountClipboardFormats() }, formats_before);

    let image = Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_image().ok())
        .unwrap_or_else(|| panic!("synthetic image clipboard fixture was not preserved"));
    assert_eq!(image.width, 1);
    assert_eq!(image.height, 1);
    assert!(image.bytes.as_ref() == pixels.as_slice());

    drop(restore);
}
