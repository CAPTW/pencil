//! Ephemeral launcher status, never document storage or inference authority.
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot { version:u32, session:String, phase:String, pipe:Option<String>, #[serde(default, rename="retryTicket")] retry_ticket:Option<String> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct Status { pub phase:String, pub can_retry:bool, pub retry_generation:Option<String> }
fn nonce_valid(n:&str)->bool { n.len()==32 && n.bytes().all(|b|b.is_ascii_hexdigit()) }
fn parse(bytes:&[u8],nonce:&str)->Result<Snapshot,String> {
    if !nonce_valid(nonce) || bytes.len()>4096 { return Err("gemma_session_invalid".into()); }
    let state:Snapshot=serde_json::from_slice(bytes).map_err(|_|"gemma_session_invalid")?;
    if state.version!=1 || state.session!=nonce || !matches!(state.phase.as_str(),"starting"|"ready"|"failed") {
        return Err("gemma_session_invalid".into());
    }
    if state.phase=="ready" {
        let pipe=state.pipe.as_deref().ok_or("gemma_session_invalid")?;
        let suffix=pipe.strip_prefix(r"\\.\pipe\Grammar.Gemma.Broker.").ok_or("gemma_session_invalid")?;
        if suffix.len()!=36 || uuid::Uuid::parse_str(suffix).is_err() {
            return Err("gemma_session_invalid".into());
        }
    } else if state.pipe.is_some() { return Err("gemma_session_invalid".into()); }
    if state.retry_ticket.as_ref().is_some_and(|t| state.phase!="failed" || !nonce_valid(t)) { return Err("gemma_session_invalid".into()); }
    Ok(state)
}
// Retain one launcher process handle: a reused PID cannot revive a stale session.
fn launcher_alive()->bool {
    use windows_sys::Win32::{Foundation::{CloseHandle, WAIT_TIMEOUT},System::Threading::{OpenProcess,WaitForSingleObject,PROCESS_SYNCHRONIZE}};
    struct Pin(isize);
    impl Drop for Pin { fn drop(&mut self) { unsafe { CloseHandle(self.0 as _); } } }
    static PIN: std::sync::OnceLock<Option<Pin>> = std::sync::OnceLock::new();
    let pin=PIN.get_or_init(|| {
        let pid=std::env::var("GRAMMAR_GEMMA_LAUNCHER_PID").ok()?.parse::<u32>().ok()?;
        let handle=unsafe{OpenProcess(PROCESS_SYNCHRONIZE,0,pid)};
        if handle.is_null() {None} else {Some(Pin(handle as isize))}
    });
    pin.as_ref().is_some_and(|p|unsafe{WaitForSingleObject(p.0 as _,0)==WAIT_TIMEOUT})
}
fn snapshot()->Result<Snapshot,String> {
    if !launcher_alive() { return Err("gemma_launcher_closed".into()); }
    let nonce=std::env::var("GRAMMAR_GEMMA_SESSION").map_err(|_|"gemma_session_missing")?;
    if !nonce_valid(&nonce) { return Err("gemma_session_invalid".into()); }
    let app=std::env::current_exe().map_err(|_|"gemma_app_path")?;
    let path=app.parent().ok_or("gemma_app_path")?.join(format!("gemma-session-{nonce}.json"));
    let mut bytes=Vec::new();
    std::fs::File::open(path).map_err(|_|"gemma_session_unavailable")?.take(4097).read_to_end(&mut bytes).map_err(|_|"gemma_session_unavailable")?;
    parse(&bytes,&nonce)
}
pub(crate) fn status()->Status {
    match std::env::var_os("GRAMMAR_GEMMA_SESSION") {
        None => Status{phase:"unmanaged".into(),can_retry:false,retry_generation:None},
        Some(_) => match snapshot() {
            Ok(s)=>Status{phase:s.phase,can_retry:s.retry_ticket.is_some(),retry_generation:s.retry_ticket},
            Err(_)=>Status{phase:"failed".into(),can_retry:false,retry_generation:None},
        }
    }
}
// Only a fresh launcher-issued failure ticket can request model preparation.
// No capture, draft, text, endpoint or executable is accepted from the caller.
pub(crate) fn retry()->Result<(),String> {
    let state=snapshot()?;
    let ticket=state.retry_ticket.ok_or("gemma_retry_unavailable")?;
    let app=std::env::current_exe().map_err(|_|"gemma_app_path")?;
    let path=app.parent().ok_or("gemma_app_path")?.join(format!("gemma-session-{}.json.retry",state.session));
    write_retry(&path,&ticket)
}
fn write_retry(path:&std::path::Path,ticket:&str)->Result<(),String> {
    use std::io::Write;
    // create_new prevents duplicate clicks from replacing an outstanding request.
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|_|"gemma_retry_pending")?;
    file.write_all(ticket.as_bytes()).map_err(|_|"gemma_retry_write".into())
}
pub(crate) fn endpoint()->Result<String,String> {
    if std::env::var_os("GRAMMAR_GEMMA_SESSION").is_some() {
        let s=snapshot()?;
        if s.phase!="ready" { return Err("gemma_not_ready".into()); }
        s.pipe.ok_or_else(||"gemma_session_invalid".into())
    } else { std::env::var("GRAMMAR_GEMMA_PIPE").map_err(|_|"gemma_launcher_config".into()) }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn retry_ticket_is_failure_only_and_requests_are_exclusive() {
        let n="a".repeat(32);
        let mut v=serde_json::json!({"version":1,"session":n,"phase":"failed","pipe":null,"retryTicket":n});
        assert!(parse(&serde_json::to_vec(&v).unwrap(),&n).is_ok());
        v["phase"]=serde_json::json!("starting");
        assert!(parse(&serde_json::to_vec(&v).unwrap(),&n).is_err());
        v["phase"]=serde_json::json!("failed");v["retryTicket"]=serde_json::json!("../bad");
        assert!(parse(&serde_json::to_vec(&v).unwrap(),&n).is_err());
        let path=std::env::temp_dir().join(format!("grammar-retry-test-{}",uuid::Uuid::new_v4()));
        write_retry(&path,&n).unwrap();
        assert!(write_retry(&path,&"b".repeat(32)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(),n);
        std::fs::remove_file(path).unwrap();
    }
    #[test] fn session_is_scoped_bounded_and_never_supplies_binary_hash() {
        let n="a".repeat(32);
        for phase in ["starting","failed"] {
            let b=serde_json::to_vec(&serde_json::json!({"version":1,"session":n,"phase":phase,"pipe":null})).unwrap();
            assert_eq!(parse(&b,&n).unwrap().phase,phase);
            assert!(parse(&b,&"b".repeat(32)).is_err());
        }
        let mut v=serde_json::json!({"version":1,"session":n,"phase":"ready","pipe":r"\\.\pipe\Grammar.Gemma.Broker.00000000-0000-0000-0000-000000000001"});
        assert!(parse(&serde_json::to_vec(&v).unwrap(),&n).is_ok());
        let mut bad=v.clone();bad["pipe"]=serde_json::json!(r"\\.\pipe\Grammar.Gemma.Broker.other");
        assert!(parse(&serde_json::to_vec(&bad).unwrap(),&n).is_err());
        bad=v.clone();bad["phase"]=serde_json::json!("starting");
        assert!(parse(&serde_json::to_vec(&bad).unwrap(),&n).is_err());
        v["broker_sha256"]=serde_json::json!("untrusted");
        assert!(parse(&serde_json::to_vec(&v).unwrap(),&n).is_err());
        assert!(parse(&vec![b' ';4097],&n).is_err());
        assert!(parse(b"{}","../outside").is_err());
    }
}
