//! Dedicated native-host review actor. No Provider API or document history.
//! The MV3 worker owns documentId/epoch/ticket binding; desktop owns explicit Accept.
use super::{channel, io, Output, Request};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Debug)]
pub(super) enum Poll {
    Idle,
    Closed,
    Capture,
    Apply(String),
}
trait Api: Send + Sync {
    fn create(&self) -> Result<u64, ()>;
    fn poll(&self, cookie: u64) -> Result<Poll, ()>;
    fn respond(&self, cookie: u64, status: u32, text: &str) -> Result<(), ()>;
    fn close(&self, cookie: u64) -> Result<(), ()>;
}
#[derive(Serialize)]
struct Command<'a> {
    version: u32,
    op: &'static str,
    id: &'a str,
    sequence: u64,
    ticket: Option<&'a str>,
    text: Option<&'a str>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    version: u32,
    op: String,
    id: String,
    sequence: u64,
    status: u32,
    text: String,
    ticket: Option<String>,
}
fn valid_ticket(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
pub(super) fn admitted(request: &Request) -> bool {
    request.version == 1
        && request.op == "review-listen"
        && valid_ticket(&request.id)
        && request.epoch == "desktop-review"
        && request.revision == 1
        && request.text.is_empty()
        && request.provider.is_none()
        && request.consent.is_none()
        && request.cleanup_token.is_none()
        && request.cleanup_ticket.is_none()
}
fn reply(body: &[u8], id: &str, sequence: u64, ticket: Option<&str>) -> Result<Reply, ()> {
    let value: Reply = serde_json::from_slice(body).map_err(|_| ())?;
    if value.version != 1
        || value.op != "review-result"
        || value.id != id
        || value.sequence != sequence
        || value.text.encode_utf16().count() > 8192
        || value.text.contains('\0')
    {
        return Err(());
    }
    let valid = match ticket {
        None => {
            (value.status == 0 && value.text.is_empty() && value.ticket.is_none())
                || (value.status == 2
                    && !value.text.is_empty()
                    && value.ticket.as_deref().is_some_and(valid_ticket))
        }
        Some(expected) => {
            [0, 3, 4].contains(&value.status)
                && value.text.is_empty()
                && value.ticket.as_deref() == Some(expected)
        }
    };
    if !valid {
        return Err(());
    }
    Ok(value)
}
pub(super) async fn run(
    input: channel::Receiver<io::Result<Vec<u8>>>,
    output: &Output,
    id: &str,
) -> Result<(), ()> {
    run_with(input, output, id, load()?).await
}
async fn run_with(
    mut input: channel::Receiver<io::Result<Vec<u8>>>,
    output: &Output,
    id: &str,
    api: Arc<dyn Api>,
) -> Result<(), ()> {
    let create = api.clone();
    let cookie = tokio::task::spawn_blocking(move || create.create())
        .await
        .map_err(|_| ())??;
    let result = async {
        output
            .send(&Command {
                version: 1,
                op: "review-ready",
                id,
                sequence: 0,
                ticket: None,
                text: None,
            })
            .await?;
        let mut sequence = 0;
        let mut pending = false;
        let mut ticket: Option<String> = None;
        loop {
            // Exactly one native call at a time. No timeout abandons a blocking
            // DLL call; the transport itself bounds its pipe/header/body waits.
            let polling = api.clone();
            let event = tokio::task::spawn_blocking(move || polling.poll(cookie))
                .await
                .map_err(|_| ())??;
            match event {
                Poll::Closed => return Ok(()),
                Poll::Idle => {}
                Poll::Capture | Poll::Apply(_) => {
                    if pending {
                        return Err(());
                    }
                    sequence += 1;
                    pending = true;
                    let (op, text) = match &event {
                        Poll::Capture if sequence == 1 => ("review-capture", None),
                        Poll::Apply(text) if sequence == 2 && ticket.is_some() => {
                            ("review-apply", Some(text.as_str()))
                        }
                        _ => return Err(()),
                    };
                    output
                        .send(&Command {
                            version: 1,
                            op,
                            id,
                            sequence,
                            ticket: ticket.as_deref(),
                            text,
                        })
                        .await?;
                }
            }
            match input.try_recv() {
                Ok(Ok(body)) => {
                    if !pending {
                        return Err(());
                    }
                    let result = reply(&body, id, sequence, ticket.as_deref())?;
                    let response_api = api.clone();
                    let status = result.status;
                    let source = result.text;
                    tokio::task::spawn_blocking(move || {
                        response_api.respond(cookie, status, &source)
                    })
                    .await
                    .map_err(|_| ())??;
                    pending = false;
                    if sequence == 1 {
                        ticket = result.ticket;
                    }
                }
                Ok(Err(_)) => return Err(()),
                Err(channel::error::TryRecvError::Disconnected) => return Ok(()),
                Err(channel::error::TryRecvError::Empty) => {}
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    .await;
    let cleanup = tokio::task::spawn_blocking(move || api.close(cookie))
        .await
        .map_err(|_| ())?;
    result.and(cleanup)
}
#[cfg(windows)]
fn load() -> Result<Arc<dyn Api>, ()> {
    use std::sync::OnceLock;
    static API: OnceLock<Result<Arc<dyn Api>, ()>> = OnceLock::new();
    API.get_or_init(|| unsafe { WindowsApi::load().map(|api| Arc::new(api) as Arc<dyn Api>) })
        .clone()
}
#[cfg(not(windows))]
fn load() -> Result<Arc<dyn Api>, ()> {
    Err(())
}
#[cfg(windows)]
struct WindowsApi {
    create: unsafe extern "system" fn(*mut u64) -> u32,
    poll: unsafe extern "system" fn(
        u64,
        *mut u32,
        *mut u32,
        *mut u64,
        *mut u16,
        u32,
        *mut u32,
    ) -> u32,
    respond: unsafe extern "system" fn(u64, u32, *const u16, u32) -> u32,
    close: unsafe extern "system" fn(u64) -> u32,
}
#[cfg(windows)]
impl WindowsApi {
    unsafe fn load() -> Result<Self, ()> {
        use windows_sys::Win32::System::LibraryLoader::*;
        let path = std::env::current_exe()
            .map_err(|_| ())?
            .with_file_name("target-apply.dll");
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let module = LoadLibraryExW(
            path.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if module.is_null() {
            return Err(());
        }
        // Retain the library for this native-host process lifetime. No callback
        // can outlive a DLL unload, including an exceptional cleanup failure.
        macro_rules! symbol {
            ($name:literal,$ty:ty) => {{
                match GetProcAddress(module, concat!($name, "\0").as_ptr()) {
                    Some(p) => std::mem::transmute::<unsafe extern "system" fn() -> isize, $ty>(p),
                    None => return Err(()),
                }
            }};
        }
        let version = symbol!(
            "GrammarBrowserServerVersion",
            unsafe extern "system" fn() -> u32
        );
        if version() != 1 {
            return Err(());
        }
        Ok(Self {
            create: symbol!(
                "GrammarBrowserServerCreate",
                unsafe extern "system" fn(*mut u64) -> u32
            ),
            poll: symbol!(
                "GrammarBrowserServerPoll",
                unsafe extern "system" fn(
                    u64,
                    *mut u32,
                    *mut u32,
                    *mut u64,
                    *mut u16,
                    u32,
                    *mut u32,
                ) -> u32
            ),
            respond: symbol!(
                "GrammarBrowserServerRespond",
                unsafe extern "system" fn(u64, u32, *const u16, u32) -> u32
            ),
            close: symbol!(
                "GrammarBrowserServerClose",
                unsafe extern "system" fn(u64) -> u32
            ),
        })
    }
}
#[cfg(windows)]
impl Api for WindowsApi {
    fn create(&self) -> Result<u64, ()> {
        let mut id = 0;
        if unsafe { (self.create)(&mut id) } == 0 && id != 0 {
            Ok(id)
        } else {
            Err(())
        }
    }
    fn poll(&self, cookie: u64) -> Result<Poll, ()> {
        let (mut state, mut command, mut capture, mut units) = (2, 0, 0, 0);
        let mut buffer = vec![0u16; 8193];
        let code = unsafe {
            (self.poll)(
                cookie,
                &mut state,
                &mut command,
                &mut capture,
                buffer.as_mut_ptr(),
                8193,
                &mut units,
            )
        };
        let result = if code != 0 || units > 8192 {
            Err(())
        } else {
            match (state, command, capture, units) {
                (0, 0, 0, 0) => Ok(Poll::Idle),
                (2, 0, 0, 0) => Ok(Poll::Closed),
                (1, 1, 0, 0) => Ok(Poll::Capture),
                (1, 2, 1, 1..=8192) => String::from_utf16(&buffer[..units as usize])
                    .map(Poll::Apply)
                    .map_err(|_| ()),
                _ => Err(()),
            }
        };
        for unit in &mut buffer {
            unsafe {
                std::ptr::write_volatile(unit, 0);
            }
        }
        result
    }
    fn respond(&self, cookie: u64, status: u32, text: &str) -> Result<(), ()> {
        let mut units: Vec<u16> = text.encode_utf16().collect();
        if units.len() > 8192 {
            return Err(());
        }
        let code = unsafe { (self.respond)(cookie, status, units.as_ptr(), units.len() as u32) };
        for unit in &mut units {
            unsafe {
                std::ptr::write_volatile(unit, 0);
            }
        }
        if code == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
    fn close(&self, cookie: u64) -> Result<(), ()> {
        if unsafe { (self.close)(cookie) } == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "11111111-1111-4111-8111-111111111111";
    const TICKET: &str = "22222222-2222-4222-8222-222222222222";
    fn value(status: u32, text: &str, ticket: Option<&str>) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"version":1,"op":"review-result","id":ID,"sequence":1,"status":status,"text":text,"ticket":ticket})).unwrap()
    }
    #[test]
    fn listener_requires_empty_first_protocol_and_no_provider_fields() {
        let good = serde_json::json!({"version":1,"op":"review-listen","id":ID,"epoch":"desktop-review","revision":1,"text":""});
        assert!(admitted(&serde_json::from_value(good.clone()).unwrap()));
        for (key, value) in [
            ("text", serde_json::json!("source")),
            ("provider", serde_json::json!("codex")),
            ("consent", serde_json::json!(true)),
            ("epoch", serde_json::json!("other")),
            ("revision", serde_json::json!(2)),
            ("cleanup_token", serde_json::json!(ID)),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(!admitted(&serde_json::from_value(bad).unwrap()));
        }
    }
    #[test]
    fn exact_capture_and_outcome_shapes() {
        assert!(reply(&value(2, "source", Some(TICKET)), ID, 1, None).is_ok());
        assert!(reply(&value(0, "", None), ID, 1, None).is_ok());
        for status in [0, 3, 4] {
            assert!(reply(&value(status, "", Some(TICKET)), ID, 1, Some(TICKET)).is_ok());
        }
    }
    #[test]
    fn stale_or_unsolicited_results_cannot_advance() {
        let bytes = value(2, "source", Some(TICKET));
        assert!(reply(&bytes, ID, 2, None).is_err());
        assert!(reply(&bytes, TICKET, 1, None).is_err());
        assert!(reply(&value(3, "", None), ID, 1, None).is_err());
        assert!(reply(&value(3, "", Some(ID)), ID, 1, Some(TICKET)).is_err());
    }
    #[test]
    fn oversize_nul_unknown_fields_and_bad_ticket_rejected() {
        for text in ["x".repeat(8193), "x\0y".into()] {
            assert!(reply(&value(2, &text, Some(TICKET)), ID, 1, None).is_err());
        }
        assert!(reply(&value(2, "source", Some("invalid")), ID, 1, None).is_err());
        let mut v: serde_json::Value =
            serde_json::from_slice(&value(2, "source", Some(TICKET))).unwrap();
        v["provider"] = "codex".into();
        assert!(reply(&serde_json::to_vec(&v).unwrap(), ID, 1, None).is_err());
    }
}
