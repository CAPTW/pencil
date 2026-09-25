//! Consent-bound native messaging actor. Local Instant and explicit selected-Provider Deep; no history.
#[path = "grammar_chromium/cleanup.rs"]
mod cleanup;
use codex_pencil::instant_selection::{
    AnalysisMode, AnalysisOptions, AnalysisRequest, InstantSelectionEngine, SourceIdentity,
    ENGINE_ID, ENGINE_VERSION,
};
use codex_pencil::provider::{
    executor::{DeepCancellation, DeepRequest, DeepRuntime},
    ProviderKind,
};
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    io::{self, Read, Write},
    pin::Pin,
    sync::{mpsc, Arc},
    time::Duration,
};
use tokio::sync::{mpsc as channel, oneshot};

const WRITE_BUDGET: Duration = Duration::from_secs(3);
type DeepFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
type CleanupFuture = Pin<Box<dyn Future<Output = bool> + Send>>;

const MAX_FRAME: usize = 128 * 1024;
const MAX_RESPONSE: usize = 256 * 1024;
const MAX_TEXT: usize = 8192;
const CONFIG_NAME: &str = "grammar-chromium-host.origin.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginConfig {
    origin: String,
    #[serde(default)]
    installation_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    version: u32,
    op: String,
    id: String,
    epoch: String,
    revision: u64,
    text: String,
    #[serde(default)]
    provider: Option<ProviderKind>,
    #[serde(default)]
    consent: Option<bool>,
    #[serde(default)]
    cleanup_token: Option<String>,
    #[serde(default)]
    cleanup_ticket: Option<cleanup::Ticket>,
}

#[derive(Serialize)]
struct Suggestion {
    start: usize,
    end: usize,
    replacement: String,
    rule: String,
    message: String,
}

#[derive(Serialize)]
struct Response {
    id: String,
    epoch: String,
    revision: u64,
    suggestions: Vec<Suggestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<ProviderKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cleanup_complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cleanup_ticket: Option<cleanup::Ticket>,
}

fn valid_origin(origin: &str) -> bool {
    origin
        .strip_prefix("chrome-extension://")
        .and_then(|value| value.strip_suffix('/'))
        .is_some_and(|id| id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c)))
}

fn authorized_origin(origin: &str, config: &[u8]) -> bool {
    valid_origin(origin)
        && serde_json::from_slice::<OriginConfig>(config).is_ok_and(|value| value.origin == origin)
}

// Distinguish clean port disconnect from truncated headers/bodies. Allocate only after bounds check.
fn read_frame(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0; 4];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    reader.read_exact(&mut header[1..])?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid_frame"));
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_frame(writer: &mut impl Write, response: &Response) -> io::Result<()> {
    let body = serde_json::to_vec(response).map_err(io::Error::other)?;
    if body.len() > MAX_RESPONSE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "response_limit"));
    }
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}

fn parse_request(body: &[u8]) -> Result<Request, &'static str> {
    let request: Request = serde_json::from_slice(body).map_err(|_| "invalid_request")?;
    if request.version != 1
        || request.id.is_empty()
        || request.id.len() > 128
        || request.epoch.is_empty()
        || request.epoch.len() > 128
        || request.revision == 0
        || request.revision > 9_007_199_254_740_991
        || request.text.encode_utf16().count() > MAX_TEXT
    {
        return Err("invalid_request");
    }
    Ok(request)
}

fn response_for(request: &Request, error: Option<&'static str>) -> Response {
    Response {
        id: request.id.clone(),
        epoch: request.epoch.clone(),
        revision: request.revision,
        suggestions: Vec::new(),
        error,
        replacement: None,
        source_sha256: None,
        provider: None,
        cleanup_complete: None,
        cleanup_ticket: request.cleanup_ticket.clone(),
    }
}

fn analyze(body: &[u8]) -> Result<Response, &'static str> {
    let request = parse_request(body)?;
    if request.op != "analyze"
        || request.provider.is_some()
        || request.consent.is_some()
        || request.cleanup_token.is_some()
        || request.cleanup_ticket.is_some()
    {
        return Err("invalid_request");
    }
    let mut response = response_for(&request, None);
    let identity = SourceIdentity::from_source(
        &response.epoch,
        request.revision,
        0,
        0,
        AnalysisMode::Correction,
        ENGINE_ID,
        ENGINE_VERSION,
        &request.text,
    );
    let result = InstantSelectionEngine::new()
        .analyze(&AnalysisRequest {
            source: request.text,
            identity,
            mode: AnalysisMode::Correction,
            protected_spans: vec![],
            options: AnalysisOptions::default(),
        })
        .map_err(|_| "analysis_unavailable")?;
    response.suggestions = result
        .suggestions
        .into_iter()
        .map(|item| Suggestion {
            start: item.range.start_utf16,
            end: item.range.end_utf16,
            replacement: item.replacement,
            rule: item.rule_code,
            message: item.message_code,
        })
        .collect();
    Ok(response)
}

// The service seam permits synthetic actor tests without any executable discovery.
trait DeepService {
    fn start(&self, request: Request, cancellation: DeepCancellation) -> DeepFuture;
    fn shutdown(&self) -> CleanupFuture;
    fn admit(&self, _: &Request) -> Result<(), &'static str> {
        Ok(())
    }
    fn control(&self, request: &Request) -> Response {
        response_for(request, Some("cleanup_unavailable"))
    }
}
struct ProviderService(Arc<DeepRuntime>, Option<cleanup::Store>);
impl DeepService for ProviderService {
    fn admit(&self, request: &Request) -> Result<(), &'static str> {
        if request.cleanup_token.is_some() {
            return Err("cleanup_invalid_ticket");
        }
        self.1.as_ref().ok_or("cleanup_unavailable")?.start(
            request
                .cleanup_ticket
                .as_ref()
                .ok_or("cleanup_invalid_ticket")?,
        )
    }
    fn control(&self, request: &Request) -> Response {
        let mut response = response_for(request, None);
        let result = (|| -> Result<(), &'static str> {
            let store = self.1.as_ref().ok_or("cleanup_unavailable")?;
            match request.op.as_str() {
                "cleanup-reserve" | "cleanup-find" if request.cleanup_ticket.is_none() => {
                    let token = request
                        .cleanup_token
                        .as_deref()
                        .ok_or("cleanup_invalid_ticket")?;
                    response.cleanup_ticket = Some(if request.op == "cleanup-find" {
                        store.find(token)?
                    } else {
                        store.reserve(token)?
                    });
                }
                "cleanup-query" | "cleanup-ack" if request.cleanup_token.is_none() => {
                    let ticket = request
                        .cleanup_ticket
                        .as_ref()
                        .ok_or("cleanup_invalid_ticket")?;
                    if request.op == "cleanup-query" {
                        store.query(ticket)?
                    } else {
                        store.ack(ticket)?
                    };
                    response.cleanup_complete = Some(true);
                }
                _ => return Err("cleanup_invalid_ticket"),
            }
            Ok(())
        })();
        if let Err(error) = result {
            response.error = Some(error);
            response.cleanup_complete = Some(false);
        }
        response
    }
    fn start(&self, request: Request, cancellation: DeepCancellation) -> DeepFuture {
        let runtime = self.0.clone();
        let store = self.1.clone();
        let ticket = request.cleanup_ticket.clone();
        let mut failed = response_for(&request, Some("deep_cleanup_unresolved"));
        failed.cleanup_complete = Some(false);
        Box::pin(async move {
            // Keep the actor responsive while production filesystem/spawn work
            // synchronously occupies a runtime worker. Retain and join the task;
            // cancellation is the owned token, never JoinHandle::abort/drop.
            let task = tokio::spawn(async move {
                let mut response = response_for(&request, None);
                let result = runtime
                    .rewrite(
                        DeepRequest {
                            provider: request.provider.expect("actor validates provider"),
                            document_epoch: request.epoch,
                            document_revision: request.revision,
                            source: request.text,
                            explicit_consent: request.consent == Some(true),
                        },
                        cancellation,
                    )
                    .await;
                let cleanup = runtime.shutdown().await;
                response.cleanup_complete = Some(cleanup.is_ok());
                if let Err(error) = cleanup {
                    response.error = Some(error.code());
                    return response;
                }
                if store
                    .as_ref()
                    .zip(ticket.as_ref())
                    .is_none_or(|(store, ticket)| store.complete(ticket).is_err())
                {
                    response.cleanup_complete = Some(false);
                    response.error = Some("deep_cleanup_unresolved");
                    return response;
                }
                match result {
                    Ok(value) => {
                        response.replacement = Some(value.replacement);
                        response.source_sha256 = Some(value.source_sha256);
                        response.provider = Some(value.provider);
                    }
                    Err(error) => response.error = Some(error.code()),
                }
                response
            });
            task.await.unwrap_or(failed)
        })
    }
    fn shutdown(&self) -> CleanupFuture {
        let runtime = self.0.clone();
        Box::pin(async move { runtime.shutdown().await.is_ok() })
    }
}

struct WriteRequest {
    bytes: Vec<u8>,
    done: oneshot::Sender<io::Result<()>>,
}
struct Output {
    sender: mpsc::SyncSender<WriteRequest>,
    budget: Duration,
}
impl Output {
    async fn send(&self, response: &Response) -> Result<(), ()> {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, response).map_err(|_| ())?;
        let (done, receipt) = oneshot::channel();
        self.sender
            .try_send(WriteRequest { bytes, done })
            .map_err(|_| ())?;
        tokio::time::timeout(self.budget, receipt)
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?
            .map_err(|_| ())
    }
}
struct ActiveDeep {
    id: String,
    epoch: String,
    revision: u64,
    cancel: DeepCancellation,
    future: DeepFuture,
}

async fn run_actor<S: DeepService>(
    mut input: channel::Receiver<io::Result<Vec<u8>>>,
    output: &Output,
    service: &S,
) -> Result<(), ()> {
    let mut active: Option<ActiveDeep> = None;
    let result = loop {
        enum Event {
            Input(Option<io::Result<Vec<u8>>>),
            Done(Response),
        }
        let event = if let Some(current) = active.as_mut() {
            tokio::select! {
                request = input.recv() => Event::Input(request),
                response = current.future.as_mut() => Event::Done(response),
            }
        } else {
            Event::Input(input.recv().await)
        };
        let body = match event {
            Event::Done(response) => {
                active = None; // Future resolved after checked cleanup; never aborted.
                let unresolved = response.cleanup_complete != Some(true);
                if output.send(&response).await.is_err() || unresolved {
                    break Err(());
                }
                continue;
            }
            Event::Input(None) => break Ok(()),
            Event::Input(Some(Err(_))) => break Err(()),
            Event::Input(Some(Ok(body))) => body,
        };
        let request = match parse_request(&body) {
            Ok(request) => request,
            Err(_) => break Err(()),
        };
        if request.op.starts_with("cleanup-") {
            if !request.text.is_empty() || request.provider.is_some() || request.consent.is_some() {
                break Err(());
            }
            if output.send(&service.control(&request)).await.is_err() {
                break Err(());
            }
            continue;
        }
        if request.op == "cancel" {
            if request.text.is_empty()
                && request.provider.is_none()
                && request.consent.is_none()
                && active.as_ref().is_some_and(|current| {
                    current.id == request.id
                        && current.epoch == request.epoch
                        && current.revision == request.revision
                })
            {
                active.as_ref().unwrap().cancel.cancel();
                continue; // The original request replies only AFTER actual cleanup.
            }
            // Late/nonmatching cancellation is a no-op: never poison a newer request.
            continue;
        }
        if active.is_some() {
            if output
                .send(&response_for(&request, Some("deep_busy")))
                .await
                .is_err()
            {
                break Err(());
            }
            continue;
        }
        match request.op.as_str() {
            "deep"
                if request.consent != Some(true)
                    || request.provider.is_none()
                    || request.text.is_empty() =>
            {
                // No Provider API or discovery occurs before this boundary.
                if output
                    .send(&response_for(&request, Some("deep_consent_required")))
                    .await
                    .is_err()
                {
                    break Err(());
                }
            }
            "deep" => {
                if let Err(error) = service.admit(&request) {
                    if output
                        .send(&response_for(&request, Some(error)))
                        .await
                        .is_err()
                    {
                        break Err(());
                    }
                    continue;
                }
                let cancel = DeepCancellation::default();
                active = Some(ActiveDeep {
                    id: request.id.clone(),
                    epoch: request.epoch.clone(),
                    revision: request.revision,
                    future: service.start(request, cancel.clone()),
                    cancel,
                });
            }
            "analyze" => {
                let response = match analyze(&body) {
                    Ok(response) => response,
                    Err(_) => break Err(()),
                };
                if output.send(&response).await.is_err() {
                    break Err(());
                }
            }
            _ => {
                if output
                    .send(&response_for(&request, Some("unsupported_op")))
                    .await
                    .is_err()
                {
                    break Err(());
                }
            }
        }
    };
    // EOF, framing failure and output failure all cancel the owned operation,
    // retain its future, and await process/workspace cleanup before host exit.
    if let Some(current) = active {
        current.cancel.cancel();
        let response = current.future.await;
        if response.cleanup_complete != Some(true) {
            return Err(());
        }
    }
    if !service.shutdown().await {
        return Err(());
    }
    result
}

fn run_stdio(service: ProviderService) -> Result<(), ()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|_| ())?;
    let (input_tx, input_rx) = channel::channel(2);
    let reader = std::thread::Builder::new()
        .name("grammar-native-input".into())
        .spawn(move || {
            let mut stdin = io::stdin().lock();
            loop {
                match read_frame(&mut stdin) {
                    Ok(Some(body)) => {
                        if input_tx.blocking_send(Ok(body)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = input_tx.blocking_send(Err(error));
                        break;
                    }
                }
            }
        })
        .map_err(|_| ())?;
    let (writer_tx, writer_rx) = mpsc::sync_channel::<WriteRequest>(1);
    let writer = std::thread::Builder::new()
        .name("grammar-native-output".into())
        .spawn(move || {
            let mut stdout = io::stdout().lock();
            while let Ok(message) = writer_rx.recv() {
                let result = stdout
                    .write_all(&message.bytes)
                    .and_then(|_| stdout.flush());
                let failed = result.is_err();
                let _ = message.done.send(result);
                if failed {
                    break;
                }
            }
        })
        .map_err(|_| ())?;
    let output = Output {
        sender: writer_tx,
        budget: WRITE_BUDGET,
    };
    let result = runtime.block_on(run_actor(input_rx, &output, &service));
    drop(output);
    // Reader/writer are bounded, process-lifetime std threads. A browser holding
    // a pipe open cannot block provider cleanup or host exit. Join completed
    // threads; process exit closes any remaining blocked OS pipe operation.
    if reader.is_finished() {
        let _ = reader.join();
    }
    if writer.is_finished() {
        let _ = writer.join();
    }
    result
}

fn main() {
    let result = (|| -> Result<(), ()> {
        let origin = std::env::args().nth(1).ok_or(())?;
        let path = std::env::current_exe()
            .map_err(|_| ())?
            .with_file_name(CONFIG_NAME);
        let mut config = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| ())?
            .take(4097)
            .read_to_end(&mut config)
            .map_err(|_| ())?;
        if config.len() > 4096 || !authorized_origin(&origin, &config) {
            return Err(());
        }
        let parsed: OriginConfig = serde_json::from_slice(&config).map_err(|_| ())?;
        let store = parsed.installation_id.map(|installation| cleanup::Store {
            directory: std::env::current_exe().unwrap().parent().unwrap().into(),
            installation,
        });
        run_stdio(ProviderService(Arc::new(DeepRuntime::default()), store))
    })();
    // No parser/provider error or document text is printed to stderr.
    std::process::exit(if result.is_ok() { 0 } else { 1 });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(text: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"version":1,"op":"analyze","id":"a","epoch":"b","revision":7,"text":text})).unwrap()
    }
    #[test]
    fn framing_rejects_truncation_and_oversize_and_accepts_eof() {
        assert!(read_frame(&mut &[][..]).unwrap().is_none());
        for data in [
            vec![1],
            vec![1, 0, 0, 0],
            vec![0, 0, 0, 0],
            ((MAX_FRAME + 1) as u32).to_le_bytes().to_vec(),
        ] {
            assert!(read_frame(&mut data.as_slice()).is_err());
        }
    }
    #[test]
    fn exact_single_origin_and_strict_schema() {
        let origin = "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/";
        let config = serde_json::to_vec(&serde_json::json!({"origin":origin})).unwrap();
        assert!(authorized_origin(origin, &config));
        assert!(!authorized_origin(
            "chrome-extension://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb/",
            &config
        ));
        assert!(!valid_origin(
            "chrome-extension://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/extra"
        ));
        assert!(!authorized_origin(origin, br#"{"origin":"*"}"#));
        assert!(analyze(&request(&"x".repeat(MAX_TEXT + 1))).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&request("hello")).unwrap();
        value["command"] = "calc.exe".into();
        assert!(analyze(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn production_engine_offsets_and_streaming() {
        let input = request("😀 seperate");
        let result = analyze(&input).unwrap();
        assert_eq!(result.revision, 7);
        assert!(!result.suggestions.is_empty());
        for item in &result.suggestions {
            assert!(item.start >= 3 && item.end <= 15);
        }
        let mut framed = Vec::new();
        for _ in 0..2 {
            framed.extend_from_slice(&(input.len() as u32).to_le_bytes());
            framed.extend_from_slice(&input);
        }
        let mut output = Vec::new();
        let mut input = framed.as_slice();
        while let Some(body) = read_frame(&mut input).unwrap() {
            write_frame(&mut output, &analyze(&body).unwrap()).unwrap();
        }
        let mut reader = output.as_slice();
        assert!(read_frame(&mut reader).unwrap().is_some());
        assert!(read_frame(&mut reader).unwrap().is_some());
        assert!(read_frame(&mut reader).unwrap().is_none());
    }
    #[test]
    fn instant_entry_never_dispatches_deep() {
        let mut value: serde_json::Value = serde_json::from_slice(&request("hello")).unwrap();
        value["op"] = "deep".into();
        assert!(analyze(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Clone, Default)]
    struct SyntheticService {
        calls: Arc<AtomicUsize>,
        cancelled: Arc<AtomicUsize>,
        shutdowns: Arc<AtomicUsize>,
        cleanup: Arc<tokio::sync::Notify>,
        selected: Arc<std::sync::Mutex<Vec<ProviderKind>>>,
    }
    impl DeepService for SyntheticService {
        fn start(&self, request: Request, cancel: DeepCancellation) -> DeepFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.selected
                .lock()
                .unwrap()
                .push(request.provider.unwrap());
            let this = self.clone();
            Box::pin(async move {
                while !cancel.is_cancelled() {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
                this.cancelled.fetch_add(1, Ordering::SeqCst);
                this.cleanup.notified().await;
                let mut response = response_for(&request, Some("deep_cancelled"));
                response.cleanup_complete = Some(true);
                response
            })
        }
        fn shutdown(&self) -> CleanupFuture {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { true })
        }
    }
    fn actor_output(budget: Duration) -> (Output, mpsc::Receiver<WriteRequest>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        (Output { sender, budget }, receiver)
    }
    async fn receipt(receiver: &mpsc::Receiver<WriteRequest>) -> serde_json::Value {
        let message = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Ok(message) = receiver.try_recv() {
                    break message;
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("actor reply");
        let value = serde_json::from_slice(&message.bytes[4..]).unwrap();
        message.done.send(Ok(())).unwrap();
        value
    }
    async fn observed(value: &AtomicUsize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while value.load(Ordering::SeqCst) == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .unwrap();
    }
    fn deep_request() -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"version":1,"op":"deep","id":"deep1","epoch":"doc","revision":3,
            "text":"synthetic writing","provider":"claude","consent":true}),
        )
        .unwrap()
    }
    fn cancel_request(id: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"version":1,"op":"cancel","id":id,"epoch":"doc","revision":3,"text":""})).unwrap()
    }
    #[tokio::test]
    async fn actor_denies_deep_without_consent_before_service_and_keeps_instant() {
        let service = SyntheticService::default();
        let observed_service = service.clone();
        let (tx, rx) = channel::channel(2);
        let (output, replies) = actor_output(Duration::from_secs(1));
        let actor = tokio::spawn(async move { run_actor(rx, &output, &service).await });
        for consent in [serde_json::Value::Null, serde_json::Value::Bool(false)] {
            let mut value: serde_json::Value = serde_json::from_slice(&deep_request()).unwrap();
            value["consent"] = consent;
            tx.send(Ok(serde_json::to_vec(&value).unwrap()))
                .await
                .unwrap();
            assert_eq!(receipt(&replies).await["error"], "deep_consent_required");
        }
        tx.send(Ok(request("seperate"))).await.unwrap();
        assert!(!receipt(&replies).await["suggestions"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(observed_service.calls.load(Ordering::SeqCst), 0);
        drop(tx);
        assert!(actor.await.unwrap().is_ok());
    }
    #[tokio::test]
    async fn actor_cancel_is_exact_and_acknowledged_only_after_cleanup() {
        let service = SyntheticService::default();
        let observed_service = service.clone();
        let (tx, rx) = channel::channel(2);
        let (output, replies) = actor_output(Duration::from_secs(1));
        let actor = tokio::spawn(async move { run_actor(rx, &output, &service).await });
        tx.send(Ok(deep_request())).await.unwrap();
        observed(&observed_service.calls).await;
        assert_eq!(
            *observed_service.selected.lock().unwrap(),
            vec![ProviderKind::Claude]
        );
        tx.send(Ok(cancel_request("wrong"))).await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(replies.try_recv().is_err());
        assert_eq!(observed_service.cancelled.load(Ordering::SeqCst), 0);
        tx.send(Ok(deep_request())).await.unwrap();
        assert_eq!(receipt(&replies).await["error"], "deep_busy");
        assert_eq!(observed_service.calls.load(Ordering::SeqCst), 1);
        tx.send(Ok(cancel_request("deep1"))).await.unwrap();
        observed(&observed_service.cancelled).await;
        assert!(replies.try_recv().is_err());
        assert!(!actor.is_finished());
        observed_service.cleanup.notify_one();
        let reply = receipt(&replies).await;
        assert_eq!(reply["id"], "deep1");
        assert_eq!(reply["cleanup_complete"], true);
        drop(tx);
        assert!(actor.await.unwrap().is_ok());
    }
    #[tokio::test]
    async fn actor_eof_and_invalid_frame_await_actual_cleanup() {
        for invalid in [false, true] {
            let service = SyntheticService::default();
            let seen = service.clone();
            let (tx, rx) = channel::channel(2);
            let (output, replies) = actor_output(Duration::from_secs(1));
            let actor = tokio::spawn(async move { run_actor(rx, &output, &service).await });
            tx.send(Ok(deep_request())).await.unwrap();
            observed(&seen.calls).await;
            if invalid {
                tx.send(Err(io::Error::new(io::ErrorKind::InvalidData, "synthetic")))
                    .await
                    .unwrap();
            }
            drop(tx);
            observed(&seen.cancelled).await;
            assert!(!actor.is_finished());
            assert!(replies.try_recv().is_err());
            seen.cleanup.notify_one();
            assert_eq!(actor.await.unwrap().is_err(), invalid);
            assert_eq!(seen.shutdowns.load(Ordering::SeqCst), 1);
        }
    }
    #[tokio::test]
    async fn actor_stalled_output_cancels_deep_and_waits_for_cleanup() {
        let service = SyntheticService::default();
        let seen = service.clone();
        let (tx, rx) = channel::channel(2);
        let (output, replies) = actor_output(Duration::from_millis(20));
        let actor = tokio::spawn(async move { run_actor(rx, &output, &service).await });
        tx.send(Ok(deep_request())).await.unwrap();
        observed(&seen.calls).await;
        tx.send(Ok(request("another request"))).await.unwrap();
        // Keep the output receiver alive but never acknowledge the queued write.
        observed(&seen.cancelled).await;
        assert!(!actor.is_finished());
        seen.cleanup.notify_one();
        assert!(actor.await.unwrap().is_err());
        drop(replies);
    }
    struct CompletedService {
        clean: bool,
        missing: bool,
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn actor_services_cancel_while_owned_provider_task_is_synchronously_busy() {
        struct BlockingService(Arc<AtomicUsize>);
        impl DeepService for BlockingService {
            fn start(&self, request: Request, cancellation: DeepCancellation) -> DeepFuture {
                let observed = self.0.clone();
                Box::pin(async move {
                    let task = tokio::spawn(async move {
                        observed.store(1, Ordering::SeqCst);
                        let deadline = std::time::Instant::now() + Duration::from_secs(1);
                        while !cancellation.is_cancelled() && std::time::Instant::now() < deadline {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        assert!(
                            cancellation.is_cancelled(),
                            "actor must poll cancel during provider's synchronous work"
                        );
                        let mut response = response_for(&request, Some("deep_cancelled"));
                        response.cleanup_complete = Some(true);
                        response
                    });
                    task.await.unwrap()
                })
            }
            fn shutdown(&self) -> CleanupFuture {
                Box::pin(async { true })
            }
        }
        let started = Arc::new(AtomicUsize::new(0));
        let service = BlockingService(started.clone());
        let (tx, rx) = channel::channel(2);
        let (output, replies) = actor_output(Duration::from_secs(2));
        let actor = tokio::spawn(async move { run_actor(rx, &output, &service).await });
        tx.send(Ok(deep_request())).await.unwrap();
        observed(&started).await;
        tx.send(Ok(cancel_request("deep1"))).await.unwrap();
        assert_eq!(receipt(&replies).await["cleanup_complete"], true);
        drop(tx);
        assert!(actor.await.unwrap().is_ok());
    }
    impl DeepService for CompletedService {
        fn start(&self, request: Request, _: DeepCancellation) -> DeepFuture {
            let clean = self.clean;
            let missing = self.missing;
            Box::pin(async move {
                let mut response = response_for(
                    &request,
                    if clean {
                        None
                    } else {
                        Some("deep_cleanup_unresolved")
                    },
                );
                response.cleanup_complete = if missing { None } else { Some(clean) };
                if clean {
                    response.provider = request.provider;
                    response.replacement = Some("synthetic replacement".into());
                    response.source_sha256 = Some("a".repeat(64));
                }
                response
            })
        }
        fn shutdown(&self) -> CleanupFuture {
            let clean = self.clean;
            Box::pin(async move { clean })
        }
    }
    #[tokio::test]
    async fn actor_completion_echoes_binding_and_never_claims_failed_cleanup() {
        for (clean, missing) in [(true, false), (false, false), (true, true)] {
            let (tx, rx) = channel::channel(2);
            let (output, replies) = actor_output(Duration::from_secs(1));
            let actor = tokio::spawn(async move {
                run_actor(rx, &output, &CompletedService { clean, missing }).await
            });
            tx.send(Ok(deep_request())).await.unwrap();
            let reply = receipt(&replies).await;
            assert_eq!(reply["id"], "deep1");
            assert_eq!(reply["epoch"], "doc");
            assert_eq!(reply["revision"], 3);
            if missing {
                assert!(reply.get("cleanup_complete").is_none());
            } else {
                assert_eq!(reply["cleanup_complete"], clean);
            }
            if clean {
                assert_eq!(reply["provider"], "claude");
                assert_eq!(reply["replacement"], "synthetic replacement");
                assert_eq!(reply["source_sha256"], "a".repeat(64));
            } else {
                assert_eq!(reply["error"], "deep_cleanup_unresolved");
                assert!(reply.get("replacement").is_none());
            }
            drop(tx);
            assert_eq!(actor.await.unwrap().is_ok(), clean && !missing);
        }
    }
}
