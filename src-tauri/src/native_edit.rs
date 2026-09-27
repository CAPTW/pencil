//! Standard Win32 Edit control selection capture.
//!
//! Support scope: the focused control of the foreground window's GUI thread
//! whose class is exactly `Edit`, Unicode, visible, enabled, not `ES_PASSWORD`,
//! reporting no password character, owned by the same process as its top-level
//! window and not by a known credential or password-manager process, with fewer
//! than 65535 UTF-16 units. Every other control is denied before any
//! text-bearing message is sent to it.
//!
//! Grammar never changes the text of a captured editor. The control belongs to
//! another application that can change its own text between any two messages
//! another process sends (`EM_SETREADONLY` only blocks user typing), and no
//! Edit message replaces a range only if it still holds the verified text. A
//! verify-then-replace sequence therefore overwrote an application edit made
//! after verification and reported success (review finding R1), and replacing
//! a range the user had just moved, then restoring the whole text, was two
//! mutations reported as Copy-only (R2). Apply for a native capture is
//! Copy-only; see `apply_safety::execute_apply`.

use crate::capture_session::NativeEditBinding;
use crate::instant_selection::sha256_hex;

const ES_PASSWORD: u32 = 0x0020;
const ES_READONLY: u32 = 0x0800;
const WM_GETTEXTLENGTH: u32 = 0x000E;
const EM_GETSEL: u32 = 0x00B0;
const EM_GETPASSWORDCHAR: u32 = 0x00D2;
/// The only messages `Win32EditPort::send` delivers (besides `WM_GETTEXT` in
/// `read_text`): capture reads, and nothing can change a control through it.
const READ_ONLY_MESSAGES: [u32; 3] = [EM_GETPASSWORDCHAR, EM_GETSEL, WM_GETTEXTLENGTH];
/// `EM_GETSEL` reports 16-bit offsets, so larger fields are never admitted.
pub(crate) const MAX_EDIT_UNITS: usize = 0xFFFE;

/// Password managers and Windows credential UI; their plain fields can hold secrets.
const SENSITIVE_PROCESSES: &[&str] = &[
    "1password.exe",
    "bitwarden.exe",
    "consent.exe",
    "credentialuibroker.exe",
    "credwiz.exe",
    "dashlane.exe",
    "enpass.exe",
    "keepass.exe",
    "keepassxc.exe",
    "keeper.exe",
    "lastpass.exe",
    "logonui.exe",
    "nordpass.exe",
    "roboform.exe",
];

/// Win32 primitives used by the capture reader. Text-bearing calls are bounded
/// cross-process read messages; `None` means the outcome is unknown (timeout,
/// hung or destroyed target, a UIPI denial, or a message that is not a read).
pub(crate) trait EditPort {
    fn foreground_window(&mut self) -> isize;
    fn focused_control(&mut self, top_level: isize) -> Option<isize>;
    fn root_window(&mut self, hwnd: isize) -> Option<isize>;
    fn window_pid(&mut self, hwnd: isize) -> Option<u32>;
    fn class_name(&mut self, hwnd: isize) -> Option<Vec<u16>>;
    fn style(&mut self, hwnd: isize) -> u32;
    fn is_unicode(&mut self, hwnd: isize) -> bool;
    fn is_enabled(&mut self, hwnd: isize) -> bool;
    fn is_visible(&mut self, hwnd: isize) -> bool;
    /// Lower-case executable file name of the process.
    fn process_image_name(&mut self, pid: u32) -> Option<String>;
    fn send(&mut self, hwnd: isize, message: u32, wparam: usize, lparam: isize) -> Option<isize>;
    /// `WM_GETTEXT` of at most `units` UTF-16 units.
    fn read_text(&mut self, hwnd: isize, units: usize) -> Option<Vec<u16>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureDenial {
    NoEditor,
    Unsupported,
    Sensitive,
    NoSelection,
    TooLarge,
    SplitCharacter,
    Changed,
    Unavailable,
}

impl CaptureDenial {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NoEditor | Self::Unsupported => {
                "native_unsupported_editor: Grammar reads only standard Windows Edit fields, such as classic Notepad. Use the Chromium adapter for browser text."
            }
            Self::Sensitive => {
                "native_sensitive_editor: Password and credential fields are never read."
            }
            Self::NoSelection => {
                "native_no_selection: No text selected. Select text in a standard Windows Edit field and try again."
            }
            Self::TooLarge => {
                "native_field_too_large: This field is too large for exact native capture."
            }
            Self::SplitCharacter => {
                "native_selection_splits_character: The selection splits a character. Adjust the selection and try again."
            }
            Self::Changed => {
                "native_capture_changed: The field or selection changed while it was read. Try again."
            }
            Self::Unavailable => {
                "native_capture_unavailable: The editor did not respond safely. Try again."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeSelection {
    pub(crate) text: String,
    pub(crate) binding: NativeEditBinding,
}

pub(crate) fn is_sensitive_process(image_name: &str) -> bool {
    let name = image_name.to_ascii_lowercase();
    SENSITIVE_PROCESSES.contains(&name.as_str())
}

fn is_edit_class(class: &[u16]) -> bool {
    class.len() == 4
        && class
            .iter()
            .zip("edit".encode_utf16())
            .all(|(actual, expected)| {
                u8::try_from(*actual).is_ok_and(|byte| byte.to_ascii_lowercase() as u16 == expected)
            })
}

pub(crate) fn utf16_sha256(text: &[u16]) -> String {
    let bytes = text
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .collect::<Vec<_>>();
    sha256_hex(&bytes)
}

/// Denies unsupported and sensitive controls before any text-bearing message.
fn admit<P: EditPort>(port: &mut P, top_level: isize, edit: isize) -> Result<u32, CaptureDenial> {
    if !port.class_name(edit).is_some_and(|class| is_edit_class(&class)) {
        return Err(CaptureDenial::Unsupported);
    }
    if port.root_window(edit) != Some(top_level) {
        return Err(CaptureDenial::Unsupported);
    }
    let pid = port.window_pid(edit).ok_or(CaptureDenial::Unavailable)?;
    if port.window_pid(top_level) != Some(pid) {
        return Err(CaptureDenial::Unsupported);
    }
    if port.style(edit) & ES_PASSWORD != 0 {
        return Err(CaptureDenial::Sensitive);
    }
    if !port.is_unicode(edit) || !port.is_visible(edit) || !port.is_enabled(edit) {
        return Err(CaptureDenial::Unsupported);
    }
    match port.process_image_name(pid) {
        None => return Err(CaptureDenial::Unavailable),
        Some(name) if is_sensitive_process(&name) => return Err(CaptureDenial::Sensitive),
        Some(_) => {}
    }
    if port
        .send(edit, EM_GETPASSWORDCHAR, 0, 0)
        .ok_or(CaptureDenial::Unavailable)?
        != 0
    {
        return Err(CaptureDenial::Sensitive);
    }
    Ok(pid)
}

fn selection<P: EditPort>(port: &mut P, edit: isize) -> Result<(usize, usize), CaptureDenial> {
    let packed = port
        .send(edit, EM_GETSEL, 0, 0)
        .ok_or(CaptureDenial::Unavailable)? as usize;
    Ok((packed & 0xFFFF, (packed >> 16) & 0xFFFF))
}

fn text_length<P: EditPort>(port: &mut P, edit: isize) -> Result<usize, CaptureDenial> {
    let length = port
        .send(edit, WM_GETTEXTLENGTH, 0, 0)
        .ok_or(CaptureDenial::Unavailable)?;
    usize::try_from(length).map_err(|_| CaptureDenial::Unavailable)
}

fn read_exact<P: EditPort>(port: &mut P, edit: isize, units: usize) -> Option<Vec<u16>> {
    port.read_text(edit, units).filter(|text| text.len() == units)
}

/// Reads the exact selection of the focused standard Edit control of the
/// foreground window `top_level`.
pub(crate) fn read_focused_selection<P: EditPort>(
    port: &mut P,
    top_level: isize,
) -> Result<NativeSelection, CaptureDenial> {
    if top_level == 0 || port.foreground_window() != top_level {
        return Err(CaptureDenial::NoEditor);
    }
    let edit = port
        .focused_control(top_level)
        .filter(|hwnd| *hwnd != 0)
        .ok_or(CaptureDenial::NoEditor)?;
    let pid = admit(port, top_level, edit)?;
    let (start, end) = selection(port, edit)?;
    if start >= end {
        return Err(CaptureDenial::NoSelection);
    }
    let units = text_length(port, edit)?;
    if units > MAX_EDIT_UNITS {
        return Err(CaptureDenial::TooLarge);
    }
    if end > units {
        return Err(CaptureDenial::Changed);
    }
    let text = read_exact(port, edit, units).ok_or(CaptureDenial::Changed)?;
    // Revalidate identity, focus, range and length after the read so a
    // concurrent change can never produce a mixed selection.
    if admit(port, top_level, edit)? != pid
        || port.foreground_window() != top_level
        || port.focused_control(top_level) != Some(edit)
        || selection(port, edit)? != (start, end)
        || text_length(port, edit)? != units
    {
        return Err(CaptureDenial::Changed);
    }
    let selected =
        String::from_utf16(&text[start..end]).map_err(|_| CaptureDenial::SplitCharacter)?;
    Ok(NativeSelection {
        text: selected,
        binding: NativeEditBinding {
            edit,
            pid,
            start,
            end,
            text_units: units,
            text_sha256: utf16_sha256(&text),
            read_only: port.style(edit) & ES_READONLY != 0,
        },
    })
}

#[cfg(windows)]
pub(crate) struct Win32EditPort {
    read_timeout_ms: u32,
}

#[cfg(windows)]
impl Default for Win32EditPort {
    fn default() -> Self {
        Self { read_timeout_ms: 100 }
    }
}

#[cfg(windows)]
impl Win32EditPort {
    fn send_timeout(
        &self,
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
        timeout_ms: u32,
    ) -> Option<isize> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, SMTO_ABORTIFHUNG, SMTO_BLOCK,
        };
        let mut result = 0usize;
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd as HWND,
                message,
                wparam,
                lparam,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                timeout_ms,
                &mut result,
            )
        };
        (sent != 0).then_some(result as isize)
    }
}

#[cfg(windows)]
impl EditPort for Win32EditPort {
    fn foreground_window(&mut self) -> isize {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize }
    }

    fn focused_control(&mut self, top_level: isize) -> Option<isize> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
        };
        let thread = unsafe { GetWindowThreadProcessId(top_level as HWND, std::ptr::null_mut()) };
        if thread == 0 {
            return None;
        }
        let mut info: GUITHREADINFO = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if unsafe { GetGUIThreadInfo(thread, &mut info) } == 0 || info.hwndFocus.is_null() {
            return None;
        }
        Some(info.hwndFocus as isize)
    }

    fn root_window(&mut self, hwnd: isize) -> Option<isize> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
        let root = unsafe { GetAncestor(hwnd as HWND, GA_ROOT) };
        (!root.is_null()).then_some(root as isize)
    }

    fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
        let mut pid = 0;
        let thread = unsafe { GetWindowThreadProcessId(hwnd as HWND, &mut pid) };
        (thread != 0 && pid != 0).then_some(pid)
    }

    fn class_name(&mut self, hwnd: isize) -> Option<Vec<u16>> {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW;
        let mut class = [0u16; 64];
        let count = unsafe { GetClassNameW(hwnd as HWND, class.as_mut_ptr(), class.len() as i32) };
        (count > 0).then(|| class[..count as usize].to_vec())
    }

    fn style(&mut self, hwnd: isize) -> u32 {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongW, GWL_STYLE};
        unsafe { GetWindowLongW(hwnd as HWND, GWL_STYLE) as u32 }
    }

    fn is_unicode(&mut self, hwnd: isize) -> bool {
        use windows_sys::Win32::Foundation::HWND;
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowUnicode(hwnd as HWND) != 0 }
    }

    fn is_enabled(&mut self, hwnd: isize) -> bool {
        use windows_sys::Win32::Foundation::HWND;
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(hwnd as HWND) != 0 }
    }

    fn is_visible(&mut self, hwnd: isize) -> bool {
        use windows_sys::Win32::Foundation::HWND;
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(hwnd as HWND) != 0 }
    }

    fn process_image_name(&mut self, pid: u32) -> Option<String> {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = unsafe {
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                buffer.as_mut_ptr(),
                &mut length,
            )
        };
        unsafe {
            CloseHandle(process);
        }
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16(&buffer[..length as usize]).ok()?;
        let name = path.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
        (!name.is_empty()).then_some(name)
    }

    fn send(&mut self, hwnd: isize, message: u32, wparam: usize, lparam: isize) -> Option<isize> {
        if !READ_ONLY_MESSAGES.contains(&message) {
            return None;
        }
        self.send_timeout(hwnd, message, wparam, lparam, self.read_timeout_ms)
    }

    fn read_text(&mut self, hwnd: isize, units: usize) -> Option<Vec<u16>> {
        use windows_sys::Win32::UI::WindowsAndMessaging::WM_GETTEXT;
        let mut buffer = vec![0u16; units.checked_add(1)?];
        let copied = self.send_timeout(
            hwnd,
            WM_GETTEXT,
            buffer.len(),
            buffer.as_mut_ptr() as isize,
            self.read_timeout_ms,
        )?;
        let copied = usize::try_from(copied).ok().filter(|copied| *copied <= units)?;
        buffer.truncate(copied);
        Some(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_safety::ApplyFallbackReason;

    // Messages that change an editor's state. Production cannot send them
    // (`Win32EditPort::send` delivers only `READ_ONLY_MESSAGES`); the fake still
    // models them so a test that routes a new path to it records the change.
    const EM_SETSEL: u32 = 0x00B1;
    const EM_SCROLLCARET: u32 = 0x00B7;
    const EM_GETMODIFY: u32 = 0x00B8;
    const EM_SETMODIFY: u32 = 0x00B9;
    const EM_SETREADONLY: u32 = 0x00CF;
    const EM_GETLIMITTEXT: u32 = 0x00D5;
    const EM_UNDO: u32 = 0x00C7;
    const EM_REPLACESEL: u32 = 0x00C2;
    const WM_SETTEXT: u32 = 0x000C;
    const TOP: isize = 0x100;
    const EDIT: isize = 0x200;
    const PID: u32 = 42;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Race {
        None,
        /// The user moves the selection right after any `EM_SETSEL`.
        MoveSelection(usize, usize),
    }

    /// In-memory model of one standard Edit control and its top-level window.
    struct FakeEdit {
        text: Vec<u16>,
        selection: (usize, usize),
        style: u32,
        class: Vec<u16>,
        unicode: bool,
        foreground: isize,
        focus: isize,
        edit_alive: bool,
        edit_pid: u32,
        image: String,
        password_char: isize,
        limit: isize,
        modified: bool,
        undo: Option<(Vec<u16>, (usize, usize))>,
        race: Race,
        refuse_unlock: bool,
        text_reads: usize,
        messages: Vec<(u32, usize)>,
        /// The application's own edit, made right after the next text read.
        change_after_first_read: Option<Vec<u16>>,
        /// The application's own edit, made right before a `WM_SETTEXT` from
        /// another process is processed.
        change_before_restore: Option<Vec<u16>>,
        /// The latest text produced by the application itself.
        app_text: Vec<u16>,
        /// Change notifications (`true` when caused by Grammar) with the text
        /// the application observed at that moment.
        notifications: Vec<(bool, String)>,
    }

    impl FakeEdit {
        fn new(text: &str, selection: (usize, usize)) -> Self {
            Self {
                text: text.encode_utf16().collect(),
                selection,
                style: 0,
                class: "Edit".encode_utf16().collect(),
                unicode: true,
                foreground: TOP,
                focus: EDIT,
                edit_alive: true,
                edit_pid: PID,
                image: "notepad.exe".into(),
                password_char: 0,
                limit: 30_000,
                modified: false,
                undo: None,
                race: Race::None,
                refuse_unlock: false,
                text_reads: 0,
                messages: Vec::new(),
                change_after_first_read: None,
                change_before_restore: None,
                app_text: text.encode_utf16().collect(),
                notifications: Vec::new(),
            }
        }

        /// An edit made by the application itself.
        fn app_edit(&mut self, text: Vec<u16>) {
            self.text = text.clone();
            self.app_text = text;
            self.notifications.push((false, self.text()));
        }

        fn grammar_edit(&mut self) {
            self.notifications.push((true, self.text()));
        }

        /// Makes any application edit still pending, as its own timer would
        /// if Grammar never sent the message that triggers it.
        fn settle(&mut self) {
            for change in [self.change_after_first_read.take(), self.change_before_restore.take()]
                .into_iter()
                .flatten()
            {
                self.app_edit(change);
            }
        }

        /// Messages that change the editor's text, selection, lock or flags.
        fn state_changes(&self) -> Vec<(u32, usize)> {
            self.messages
                .iter()
                .copied()
                .filter(|(message, _)| {
                    matches!(*message, EM_SETREADONLY | EM_SETSEL | EM_SETMODIFY | EM_UNDO | EM_REPLACESEL | WM_SETTEXT)
                })
                .collect()
        }

        fn grammar_notifications(&self) -> Vec<String> {
            self.notifications
                .iter()
                .filter(|(grammar, _)| *grammar)
                .map(|(_, text)| text.clone())
                .collect()
        }

        fn text(&self) -> String {
            String::from_utf16(&self.text).unwrap()
        }

        fn locked(&self) -> bool {
            self.style & ES_READONLY != 0
        }

        fn text_messages(&self) -> usize {
            self.messages
                .iter()
                .filter(|(message, _)| matches!(*message, EM_GETSEL | WM_GETTEXTLENGTH))
                .count()
                + self.text_reads
        }
    }

    impl EditPort for FakeEdit {
        fn foreground_window(&mut self) -> isize {
            self.foreground
        }
        fn focused_control(&mut self, top_level: isize) -> Option<isize> {
            (top_level == TOP).then_some(self.focus)
        }
        fn root_window(&mut self, hwnd: isize) -> Option<isize> {
            (hwnd == EDIT && self.edit_alive).then_some(TOP)
        }
        fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
            match hwnd {
                TOP => Some(PID),
                EDIT if self.edit_alive => Some(self.edit_pid),
                _ => None,
            }
        }
        fn class_name(&mut self, hwnd: isize) -> Option<Vec<u16>> {
            (hwnd == EDIT && self.edit_alive).then(|| self.class.clone())
        }
        fn style(&mut self, _hwnd: isize) -> u32 {
            self.style
        }
        fn is_unicode(&mut self, _hwnd: isize) -> bool {
            self.unicode
        }
        fn is_enabled(&mut self, _hwnd: isize) -> bool {
            true
        }
        fn is_visible(&mut self, _hwnd: isize) -> bool {
            true
        }
        fn process_image_name(&mut self, pid: u32) -> Option<String> {
            (pid == PID).then(|| self.image.clone())
        }
        fn send(&mut self, hwnd: isize, message: u32, wparam: usize, lparam: isize) -> Option<isize> {
            assert_eq!(hwnd, EDIT, "messages go only to the captured edit");
            if !self.edit_alive {
                return None;
            }
            self.messages.push((message, wparam));
            match message {
                EM_GETPASSWORDCHAR => Some(self.password_char),
                EM_GETSEL => Some((self.selection.0 | (self.selection.1 << 16)) as isize),
                WM_GETTEXTLENGTH => Some(self.text.len() as isize),
                EM_GETLIMITTEXT => Some(self.limit),
                EM_GETMODIFY => Some(isize::from(self.modified)),
                EM_SETMODIFY => {
                    self.modified = wparam != 0;
                    Some(1)
                }
                EM_SETSEL => {
                    self.selection = (wparam, lparam as usize);
                    if let Race::MoveSelection(start, end) = self.race {
                        self.selection = (start, end);
                    }
                    Some(1)
                }
                EM_SETREADONLY if wparam == 0 && self.refuse_unlock => Some(0),
                EM_SETREADONLY => {
                    if wparam == 0 {
                        self.style &= !ES_READONLY;
                    } else {
                        self.style |= ES_READONLY;
                    }
                    Some(1)
                }
                // Like a multiline Edit control: no undo while read-only.
                EM_UNDO if self.locked() => Some(0),
                EM_UNDO => {
                    if let Some((text, selection)) = self.undo.take() {
                        self.text = text;
                        self.selection = selection;
                        self.grammar_edit();
                    }
                    Some(1)
                }
                EM_SCROLLCARET => Some(1),
                other => panic!("unexpected message {other:#x}"),
            }
        }
        fn read_text(&mut self, hwnd: isize, units: usize) -> Option<Vec<u16>> {
            assert_eq!(hwnd, EDIT);
            self.text_reads += 1;
            let text = self.text[..units.min(self.text.len())].to_vec();
            if let Some(changed) = self.change_after_first_read.take() {
                self.app_edit(changed);
            }
            Some(text)
        }
    }

    fn units(text: &str) -> usize {
        text.encode_utf16().count()
    }

    fn range_of(haystack: &str, needle: &str) -> (usize, usize) {
        let byte = haystack.find(needle).unwrap();
        let start = units(&haystack[..byte]);
        (start, start + units(needle))
    }

    const SAMPLE: &str = "첫 줄 synthetic\r\n한국어 선택 😀 끝\r\nthird line";

    fn captured(edit: &mut FakeEdit) -> NativeSelection {
        read_focused_selection(edit, TOP).expect("supported synthetic edit is captured")
    }

    #[test]
    fn exact_korean_crlf_and_surrogate_selection_is_returned() {
        let selected = "한국어 선택 😀 끝\r\nthird";
        let mut edit = FakeEdit::new(SAMPLE, range_of(SAMPLE, selected));
        let capture = captured(&mut edit);
        assert_eq!(capture.text, selected);
        assert_eq!(capture.binding.edit, EDIT);
        assert_eq!(capture.binding.pid, PID);
        assert_eq!((capture.binding.start, capture.binding.end), range_of(SAMPLE, selected));
        assert_eq!(capture.binding.text_units, units(SAMPLE));
        assert_eq!(
            capture.binding.text_sha256,
            utf16_sha256(&SAMPLE.encode_utf16().collect::<Vec<_>>())
        );
        assert!(!capture.binding.read_only);
    }

    #[test]
    fn empty_selection_is_reported_without_reading_text() {
        let mut edit = FakeEdit::new(SAMPLE, (4, 4));
        assert_eq!(read_focused_selection(&mut edit, TOP), Err(CaptureDenial::NoSelection));
        assert_eq!(edit.text_reads, 0);
    }

    #[test]
    fn sensitive_and_unsupported_controls_are_denied_before_any_text_message() {
        let cases: [(&str, fn(&mut FakeEdit), CaptureDenial); 6] = [
            ("password style", |e| e.style |= ES_PASSWORD, CaptureDenial::Sensitive),
            ("password manager", |e| e.image = "KeePass.exe".into(), CaptureDenial::Sensitive),
            ("custom class", |e| e.class = "RichEdit20W".encode_utf16().collect(), CaptureDenial::Unsupported),
            ("superclassed edit", |e| e.class = "WindowsForms10.EDIT.app.0.1".encode_utf16().collect(), CaptureDenial::Unsupported),
            ("ANSI window", |e| e.unicode = false, CaptureDenial::Unsupported),
            ("foreign process child", |e| e.edit_pid = PID + 1, CaptureDenial::Unsupported),
        ];
        for (name, mutate, denial) in cases {
            let mut edit = FakeEdit::new(SAMPLE, (0, 3));
            mutate(&mut edit);
            assert_eq!(read_focused_selection(&mut edit, TOP), Err(denial), "{name}");
            assert_eq!(edit.text_messages(), 0, "{name} must not receive text messages");
            assert!(edit.messages.is_empty(), "{name} must receive no message");
        }
        let mut edit = FakeEdit::new(SAMPLE, (0, 3));
        edit.password_char = '*' as isize;
        assert_eq!(read_focused_selection(&mut edit, TOP), Err(CaptureDenial::Sensitive));
        assert_eq!(edit.text_messages(), 0);
    }

    #[test]
    fn window_switch_and_missing_focus_are_denied() {
        let mut edit = FakeEdit::new(SAMPLE, (0, 3));
        edit.foreground = 0x999;
        assert_eq!(read_focused_selection(&mut edit, TOP), Err(CaptureDenial::NoEditor));
        edit.foreground = TOP;
        edit.focus = 0;
        assert_eq!(read_focused_selection(&mut edit, TOP), Err(CaptureDenial::NoEditor));
        assert!(edit.messages.is_empty());
    }

    #[test]
    fn changes_during_read_and_split_characters_are_denied() {
        let mut edit = FakeEdit::new(SAMPLE, (0, 3));
        edit.change_after_first_read = Some("x".repeat(40).encode_utf16().collect());
        assert_eq!(read_focused_selection(&mut edit, TOP), Err(CaptureDenial::Changed));

        let (emoji_start, _) = range_of(SAMPLE, "😀");
        let mut split = FakeEdit::new(SAMPLE, (emoji_start, emoji_start + 1));
        assert_eq!(read_focused_selection(&mut split, TOP), Err(CaptureDenial::SplitCharacter));

        let mut large = FakeEdit::new(&"a".repeat(MAX_EDIT_UNITS + 1), (0, 3));
        assert_eq!(read_focused_selection(&mut large, TOP), Err(CaptureDenial::TooLarge));
        assert_eq!(large.text_reads, 0);
    }

    #[test]
    fn reselection_returns_only_the_current_selection() {
        let mut edit = FakeEdit::new(SAMPLE, range_of(SAMPLE, "첫 줄"));
        assert_eq!(captured(&mut edit).text, "첫 줄");
        edit.selection = range_of(SAMPLE, "third line");
        assert_eq!(captured(&mut edit).text, "third line");
    }

    // ---- Review findings R1/R2 through the production Apply entry ----
    // `EM_SETREADONLY` only blocks user typing; the application itself can
    // still change its text between any two messages another process sends.
    // These tests capture with the production reader and drive
    // `apply_current_session` (the production Apply entry). `ApplyPlatform`
    // has no editor access at all, so the decisive guards are the Copy-only
    // outcome and the exact platform call sequence; the fake editor's state,
    // notifications and application text are checked as well. Red at 4400974,
    // when a platform hook still mutated the editor.

    use crate::apply_safety::{apply_current_session, ApplyOutcome, ApplyPlatform, WaitStage};
    use crate::capture_session::{CaptureSessionStore, SessionToken, WindowTarget};

    /// Records every platform call. The editor is the application's; the
    /// platform exposes no way to change it, and Apply must not need one.
    struct EditorPlatform {
        edit: FakeEdit,
        clipboard: Option<String>,
        pastes: usize,
        calls: Vec<&'static str>,
        selection_authority: bool,
    }

    impl EditorPlatform {
        fn new(edit: FakeEdit) -> Self {
            Self { edit, clipboard: None, pastes: 0, calls: Vec::new(), selection_authority: false }
        }
    }

    impl ApplyPlatform for EditorPlatform {
        fn has_verified_selection_authority(&mut self) -> bool {
            self.calls.push("has_verified_selection_authority");
            self.selection_authority
        }
        fn is_window(&mut self, hwnd: isize) -> bool {
            self.calls.push("is_window");
            hwnd == TOP
        }
        fn window_pid(&mut self, hwnd: isize) -> Option<u32> {
            self.calls.push("window_pid");
            (hwnd == TOP).then_some(PID)
        }
        fn hide_widget(&mut self) -> Result<(), ()> {
            self.calls.push("hide_widget");
            Ok(())
        }
        fn show_widget(&mut self) {
            self.calls.push("show_widget");
        }
        fn request_foreground(&mut self, _hwnd: isize) {
            self.calls.push("request_foreground");
        }
        fn foreground_window(&mut self) -> isize {
            self.calls.push("foreground_window");
            TOP
        }
        fn clipboard_sequence(&mut self) -> u32 {
            self.calls.push("clipboard_sequence");
            0
        }
        fn read_clipboard_text(&mut self) -> Option<String> {
            self.calls.push("read_clipboard_text");
            self.clipboard.clone()
        }
        fn write_clipboard_text(&mut self, text: &str) -> Result<(), ()> {
            self.calls.push("write_clipboard_text");
            self.clipboard = Some(text.to_string());
            Ok(())
        }
        fn send_paste(&mut self) -> u32 {
            self.calls.push("send_paste");
            self.pastes += 1;
            0
        }
        fn wait(&mut self, _stage: WaitStage) {
            self.calls.push("wait");
        }
    }

    /// Captures with the production reader and drives the session to Ready.
    fn ready_session(edit: &mut FakeEdit) -> (CaptureSessionStore, SessionToken) {
        let selection = read_focused_selection(edit, TOP).expect("supported synthetic edit");
        let mut store = CaptureSessionStore::default();
        let token = store
            .capture_native("review".into(), selection.text, WindowTarget::new(TOP, PID), selection.binding)
            .expect("native session");
        assert!(store.begin_rewrite(&token).is_ok());
        assert!(store.finish_rewrite_success(&token).is_ok());
        edit.messages.clear();
        edit.notifications.clear();
        (store, token)
    }

    fn apply_through_production(edit: FakeEdit, store: &mut CaptureSessionStore, token: &SessionToken) -> (ApplyOutcome, EditorPlatform) {
        let mut platform = EditorPlatform::new(edit);
        let outcome = apply_current_session(store, token, "fixed", false, &mut platform);
        platform.edit.settle();
        (outcome, platform)
    }

    fn assert_editor_untouched(outcome: &ApplyOutcome, platform: &EditorPlatform, expected_text: &str) {
        let edit = &platform.edit;
        assert!(
            matches!(outcome, ApplyOutcome::CopiedFallback { .. }),
            "{outcome:?}: Apply must end in Copy-only; state changes {:?}, final text {:?}",
            edit.state_changes(),
            edit.text()
        );
        assert_eq!(edit.state_changes(), vec![], "Grammar changed the editor");
        assert_eq!(edit.grammar_notifications(), Vec::<String>::new(), "the application saw Grammar's intermediate text");
        assert_eq!(edit.text(), expected_text, "the application's own text must survive");
        assert_eq!(edit.text, edit.app_text);
        assert_eq!(platform.clipboard.as_deref(), Some("fixed"), "Copy-only puts the result on the clipboard");
        assert_eq!(platform.pastes, 0);
        assert_eq!(
            outcome,
            &ApplyOutcome::CopiedFallback { reason: ApplyFallbackReason::TargetMutationDisabled }
        );
        // Target identity checks, then the clipboard: no widget, focus or input activity.
        assert_eq!(platform.calls, ["is_window", "window_pid", "write_clipboard_text"]);
    }

    #[test]
    fn native_apply_is_copy_only_and_changes_no_editor_state() {
        let mut edit = FakeEdit::new(SAMPLE, range_of(SAMPLE, "third"));
        let (mut store, token) = ready_session(&mut edit);
        let (outcome, platform) = apply_through_production(edit, &mut store, &token);
        assert_editor_untouched(&outcome, &platform, SAMPLE);
    }

    #[test]
    fn r1_application_edit_after_verification_is_never_overwritten() {
        // "hello world", selection "world"; the application rewrites the text
        // to "hello newer" right after Apply's verification read.
        let mut edit = FakeEdit::new("hello world", (6, 11));
        let (mut store, token) = ready_session(&mut edit);
        edit.change_after_first_read = Some("hello newer".encode_utf16().collect());
        let (outcome, platform) = apply_through_production(edit, &mut store, &token);
        assert_editor_untouched(&outcome, &platform, "hello newer");
    }

    #[test]
    fn r2_selection_moved_before_replace_causes_no_mutation_or_restore() {
        // The selection moves to "hello" between Apply's EM_SETSEL and
        // EM_REPLACESEL. Replacing the wrong range and restoring the whole
        // text afterwards is two mutations, not Copy-only.
        let mut edit = FakeEdit::new("hello world", (6, 11));
        let (mut store, token) = ready_session(&mut edit);
        edit.race = Race::MoveSelection(0, 5);
        let (outcome, platform) = apply_through_production(edit, &mut store, &token);
        assert_editor_untouched(&outcome, &platform, "hello world");
    }

    #[test]
    fn native_capture_stays_copy_only_even_with_selection_authority() {
        // A platform that claimed selection authority would enable the paste
        // path; a native capture must still never reach it.
        let mut edit = FakeEdit::new(SAMPLE, range_of(SAMPLE, "third"));
        let (mut store, token) = ready_session(&mut edit);
        let mut platform = EditorPlatform::new(edit);
        platform.selection_authority = true;
        let outcome = apply_current_session(&mut store, &token, "fixed", false, &mut platform);
        platform.edit.settle();
        assert_editor_untouched(&outcome, &platform, SAMPLE);
    }

    #[cfg(windows)]
    #[test]
    fn win32_port_delivers_only_read_messages() {
        let mut port = Win32EditPort::default();
        // WM_PASTE and WM_CHAR too.
        for message in [WM_SETTEXT, EM_SETSEL, EM_SETREADONLY, EM_UNDO, EM_SETMODIFY, EM_REPLACESEL, 0x0302, 0x0102] {
            assert_eq!(port.send(0, message, 0, 0), None, "message {message:#06x} must never be sent");
        }
    }

    #[test]
    fn application_edit_right_before_a_restore_is_never_overwritten() {
        let mut edit = FakeEdit::new("hello world", (6, 11));
        let (mut store, token) = ready_session(&mut edit);
        edit.race = Race::MoveSelection(0, 5);
        edit.change_before_restore = Some("hello brave world".encode_utf16().collect());
        let (outcome, platform) = apply_through_production(edit, &mut store, &token);
        assert_editor_untouched(&outcome, &platform, "hello brave world");
    }
}
