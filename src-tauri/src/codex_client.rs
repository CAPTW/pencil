use crate::{codex_binary::resolve_supported_codex, settings::RewriteMode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fmt,
    future::Future,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    process::Command,
    sync::{broadcast, mpsc, oneshot, Mutex},
    time::timeout,
};

const CONNECTION_NEW: u8 = 0;
const CONNECTION_INITIALIZING: u8 = 1;
const CONNECTION_READY: u8 = 2;
const CONNECTION_DEAD: u8 = 3;
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

type PendingSender = oneshot::Sender<Result<Value, ProtocolError>>;
type PendingRequests = Arc<Mutex<HashMap<u64, PendingSender>>>;

struct OutgoingMessage {
    value: Value,
    written: Option<oneshot::Sender<Result<(), ProtocolError>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProtocolError {
    NotReady,
    AlreadyInitialized,
    Timeout(String),
    WriteTimeout,
    InvalidJson,
    StdoutClosed,
    StdinClosed,
    ChildExited,
    ChildWaitFailed,
    ChannelClosed,
    SerializationFailed,
    RequestIdExhausted,
    Remote(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReady => write!(formatter, "Codex app-server connection is not initialized."),
            Self::AlreadyInitialized => write!(
                formatter,
                "Codex app-server connection is already initialized."
            ),
            Self::Timeout(method) => write!(formatter, "Codex request timed out: {method}"),
            Self::WriteTimeout => write!(formatter, "Timed out writing to Codex app-server."),
            Self::InvalidJson => write!(
                formatter,
                "Codex app-server returned invalid protocol JSON."
            ),
            Self::StdoutClosed => write!(formatter, "Codex app-server closed stdout unexpectedly."),
            Self::StdinClosed => write!(formatter, "Codex app-server stdin is no longer writable."),
            Self::ChildExited => write!(formatter, "Codex app-server exited unexpectedly."),
            Self::ChildWaitFailed => {
                write!(formatter, "Could not monitor the Codex app-server process.")
            }
            Self::ChannelClosed => write!(formatter, "Codex app-server request channel is closed."),
            Self::SerializationFailed => {
                write!(formatter, "Could not serialize a Codex protocol message.")
            }
            Self::RequestIdExhausted => write!(formatter, "Codex request id counter exhausted."),
            Self::Remote(message) => write!(
                formatter,
                "Codex app-server rejected the request: {message}"
            ),
        }
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Clone)]
struct TransportFailure {
    pending: PendingRequests,
    notifications: broadcast::Sender<Value>,
    state: Arc<AtomicU8>,
}

impl TransportFailure {
    async fn fail(&self, error: ProtocolError) {
        if self.state.swap(CONNECTION_DEAD, Ordering::SeqCst) == CONNECTION_DEAD {
            return;
        }

        fail_all_pending(&self.pending, error.clone()).await;
        let _ = self.notifications.send(json!({
            "method": "codex/process/exited",
            "params": {
                "message": error.to_string()
            }
        }));
    }
}

pub struct CodexClient {
    tx: mpsc::Sender<OutgoingMessage>,
    pending: PendingRequests,
    notifications: broadcast::Sender<Value>,
    next_id: AtomicU64,
    state: Arc<AtomicU8>,
    failure: TransportFailure,
    _shutdown: ChildShutdown,
}

pub struct CodexClientCache {
    client: Mutex<Option<Arc<CodexClient>>>,
}

impl Default for CodexClientCache {
    fn default() -> Self {
        Self {
            client: Mutex::new(None),
        }
    }
}

impl CodexClientCache {
    pub async fn get(&self) -> Result<Arc<CodexClient>, String> {
        self.get_or_connect_with(CodexClient::connect).await
    }

    async fn get_or_connect_with<F, Fut>(&self, connect: F) -> Result<Arc<CodexClient>, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<CodexClient, String>>,
    {
        let mut guard = self.client.lock().await;
        if let Some(client) = guard.as_ref() {
            if client.is_healthy() {
                return Ok(client.clone());
            }
        }

        *guard = None;
        let client = Arc::new(connect().await?);
        *guard = Some(client.clone());
        Ok(client)
    }
}

struct ChildShutdown {
    tx: StdMutex<Option<oneshot::Sender<()>>>,
}

impl ChildShutdown {
    fn detached() -> Self {
        Self {
            tx: StdMutex::new(None),
        }
    }
}

impl Drop for ChildShutdown {
    fn drop(&mut self) {
        if let Ok(mut tx) = self.tx.lock() {
            if let Some(tx) = tx.take() {
                let _ = tx.send(());
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub logged_in: bool,
    pub account_label: Option<String>,
    pub auth_mode: Option<String>,
    pub requires_openai_auth: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLogin {
    pub login_id: String,
    pub verification_url: String,
    pub user_code: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RewriteEdit {
    pub before: String,
    pub after: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteResult {
    pub replacement: String,
    pub changed: bool,
    pub summary: String,
    pub edits: Vec<RewriteEdit>,
    pub confidence: f64,
    pub mode: RewriteMode,
}

impl CodexClient {
    pub async fn connect() -> Result<Self, String> {
        let resolved = resolve_supported_codex()?;
        let mut command = Command::new(&resolved);

        // Keep Codex app-server on stdio only. The default transport for
        // `codex app-server` is stdio://, which is local to this child process.
        // Do not change this app to a ws:// listener or any non-local network
        // transport; selected text must not be exposed over a socket server.
        command
            .arg("app-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start local Codex app-server. Install Codex CLI and ensure `codex` is on PATH. Details: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Could not open Codex app-server stdin.".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Could not open Codex app-server stdout.".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Could not open Codex app-server stderr.".to_string())?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (client, failure) = Self::from_io(
            stdout,
            stdin,
            ChildShutdown {
                tx: StdMutex::new(Some(shutdown_tx)),
            },
        );
        spawn_stderr_drain(stderr);
        spawn_child_watcher(child, shutdown_rx, failure);

        client
            .perform_handshake()
            .await
            .map_err(|error| error.to_string())?;
        Ok(client)
    }

    fn from_io<R, W>(stdout: R, stdin: W, shutdown: ChildShutdown) -> (Self, TransportFailure)
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<OutgoingMessage>(64);
        let (notifications, _) = broadcast::channel::<Value>(256);
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let state = Arc::new(AtomicU8::new(CONNECTION_NEW));
        let failure = TransportFailure {
            pending: pending.clone(),
            notifications: notifications.clone(),
            state: state.clone(),
        };

        spawn_stdin_writer(stdin, rx, failure.clone());
        spawn_stdout_reader(
            stdout,
            pending.clone(),
            notifications.clone(),
            tx.clone(),
            failure.clone(),
        );

        let client = Self {
            tx,
            pending,
            notifications,
            next_id: AtomicU64::new(1),
            state,
            failure: failure.clone(),
            _shutdown: shutdown,
        };

        (client, failure)
    }

    pub fn is_healthy(&self) -> bool {
        self.state.load(Ordering::SeqCst) == CONNECTION_READY
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.notifications.subscribe()
    }

    pub async fn auth_status(&self) -> Result<AuthStatus, String> {
        let result = self
            .request(
                "account/read",
                json!({
                    "refreshToken": false
                }),
                Duration::from_secs(30),
            )
            .await?;

        let account = result
            .get("account")
            .ok_or_else(|| "Codex account response was missing `account`.".to_string())?;
        let requires_openai_auth = result
            .get("requiresOpenaiAuth")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                "Codex account response was missing `requiresOpenaiAuth`.".to_string()
            })?;

        Ok(AuthStatus {
            logged_in: !account.is_null(),
            account_label: account_label(account),
            auth_mode: account
                .get("type")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            requires_openai_auth,
        })
    }

    pub async fn start_device_login(&self) -> Result<DeviceLogin, String> {
        let result = self
            .request(
                "account/login/start",
                device_login_params(),
                Duration::from_secs(30),
            )
            .await?;

        if result.get("type").and_then(Value::as_str) != Some("chatgptDeviceCode") {
            return Err("Codex did not return a device-code login response.".to_string());
        }

        Ok(DeviceLogin {
            login_id: required_string(&result, "loginId")?,
            verification_url: required_string(&result, "verificationUrl")?,
            user_code: required_string(&result, "userCode")?,
        })
    }

    pub async fn cancel_login(&self, login_id: String) -> Result<(), String> {
        let result = self
            .request(
                "account/login/cancel",
                json!({
                    "loginId": login_id
                }),
                Duration::from_secs(15),
            )
            .await?;
        match result.get("status").and_then(Value::as_str) {
            Some("canceled" | "notFound") => Ok(()),
            _ => Err("Codex returned an invalid login-cancellation response.".to_string()),
        }
    }

    pub async fn rewrite(
        &self,
        selected_text: &str,
        mode: RewriteMode,
    ) -> Result<RewriteResult, String> {
        let thread = self
            .request(
                "thread/start",
                json!({
                    "ephemeral": true,
                    "approvalPolicy": "never",
                    "sandbox": "read-only",
                    "threadSource": "codex-pencil",
                    "baseInstructions": "You are Codex Pencil, a compact writing assistant. Do not use tools. Do not ask follow-up questions.",
                    "developerInstructions": "Return only strict JSON matching the requested schema. Never include Markdown fences, commentary, or the original text unless it is the replacement.",
                    "personality": "pragmatic"
                }),
                Duration::from_secs(45),
            )
            .await?;

        let thread_id = thread
            .get("thread")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Codex did not return a thread id.".to_string())?
            .to_string();

        let mut notifications = self.notifications.subscribe();
        let prompt = rewrite_prompt(selected_text, mode);
        let turn = self
            .request(
                "turn/start",
                rewrite_turn_params(&thread_id, &prompt),
                Duration::from_secs(30),
            )
            .await?;

        let turn_id = turn
            .get("turn")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Codex did not return a turn id.".to_string())?
            .to_string();

        let final_text = self
            .wait_for_turn(&mut notifications, &thread_id, &turn_id)
            .await?;
        parse_rewrite_result(&final_text, mode)
    }

    async fn perform_handshake(&self) -> Result<(), ProtocolError> {
        match self.state.compare_exchange(
            CONNECTION_NEW,
            CONNECTION_INITIALIZING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {}
            Err(CONNECTION_DEAD) => return Err(ProtocolError::NotReady),
            Err(_) => return Err(ProtocolError::AlreadyInitialized),
        }

        let handshake = async {
            self.request_protocol_inner(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "codex-pencil",
                        "title": "Codex Pencil",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
                Duration::from_secs(30),
                false,
            )
            .await?;

            self.send_notification("initialized", json!({})).await
        }
        .await;

        match handshake {
            Ok(()) => self
                .state
                .compare_exchange(
                    CONNECTION_INITIALIZING,
                    CONNECTION_READY,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .map(|_| ())
                .map_err(|_| ProtocolError::ChannelClosed),
            Err(error) => {
                self.failure.fail(error.clone()).await;
                Err(error)
            }
        }
    }

    async fn request(&self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        self.request_protocol(method, params, wait)
            .await
            .map_err(|error| error.to_string())
    }

    async fn request_protocol(
        &self,
        method: &str,
        params: Value,
        wait: Duration,
    ) -> Result<Value, ProtocolError> {
        self.request_protocol_inner(method, params, wait, true)
            .await
    }

    async fn request_protocol_inner(
        &self,
        method: &str,
        params: Value,
        wait: Duration,
        require_ready: bool,
    ) -> Result<Value, ProtocolError> {
        let state = self.state.load(Ordering::SeqCst);
        if (require_ready && state != CONNECTION_READY)
            || (!require_ready && state != CONNECTION_INITIALIZING)
        {
            return Err(ProtocolError::NotReady);
        }

        let id = self.next_request_id()?;
        let (tx, rx) = oneshot::channel();

        self.pending.lock().await.insert(id, tx);

        let message = json!({
            "id": id,
            "method": method,
            "params": params
        });

        if let Err(error) = self.send_outgoing(message).await {
            self.pending.lock().await.remove(&id);
            return Err(error);
        }

        match timeout(wait, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(ProtocolError::ChannelClosed),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(ProtocolError::Timeout(method.to_string()))
            }
        }
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<(), ProtocolError> {
        self.send_outgoing(json!({
            "method": method,
            "params": params
        }))
        .await
    }

    async fn send_outgoing(&self, value: Value) -> Result<(), ProtocolError> {
        let (written_tx, written_rx) = oneshot::channel();
        let outgoing = OutgoingMessage {
            value,
            written: Some(written_tx),
        };

        match timeout(WRITE_TIMEOUT, self.tx.send(outgoing)).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                self.failure.fail(ProtocolError::ChannelClosed).await;
                return Err(ProtocolError::ChannelClosed);
            }
            Err(_) => {
                self.failure.fail(ProtocolError::WriteTimeout).await;
                return Err(ProtocolError::WriteTimeout);
            }
        }

        match timeout(WRITE_TIMEOUT, written_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.failure.fail(ProtocolError::ChannelClosed).await;
                Err(ProtocolError::ChannelClosed)
            }
            Err(_) => {
                self.failure.fail(ProtocolError::WriteTimeout).await;
                Err(ProtocolError::WriteTimeout)
            }
        }
    }

    fn next_request_id(&self) -> Result<u64, ProtocolError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if id == u64::MAX {
            return Err(ProtocolError::RequestIdExhausted);
        }
        Ok(id)
    }

    async fn wait_for_turn(
        &self,
        notifications: &mut broadcast::Receiver<Value>,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<String, String> {
        let mut latest_agent_message = String::new();
        let wait = async {
            loop {
                let notification = notifications
                    .recv()
                    .await
                    .map_err(|error| format!("Codex notification stream closed: {error}"))?;

                let method = notification
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let params = notification.get("params").unwrap_or(&Value::Null);

                if method == "codex/process/exited" {
                    return Err(params
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Codex app-server exited unexpectedly.")
                        .to_string());
                }

                if params.get("threadId").and_then(Value::as_str) != Some(thread_id) {
                    continue;
                }

                match method {
                    "item/agentMessage/delta" => {
                        if params.get("turnId").and_then(Value::as_str) == Some(turn_id) {
                            if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                                latest_agent_message.push_str(delta);
                            }
                        }
                    }
                    "item/completed" => {
                        if params.get("turnId").and_then(Value::as_str) == Some(turn_id) {
                            if let Some(text) =
                                agent_message_text(params.get("item").unwrap_or(&Value::Null))
                            {
                                latest_agent_message = text;
                            }
                        }
                    }
                    "turn/completed" => {
                        let turn = params.get("turn").unwrap_or(&Value::Null);
                        if turn.get("id").and_then(Value::as_str) != Some(turn_id) {
                            continue;
                        }

                        if turn.get("status").and_then(Value::as_str) == Some("failed") {
                            return Err(turn
                                .get("error")
                                .and_then(|error| error.get("message"))
                                .and_then(Value::as_str)
                                .unwrap_or("Codex rewrite failed.")
                                .to_string());
                        }

                        if let Some(text) = final_agent_message_from_turn(turn) {
                            return Ok(text);
                        }

                        if !latest_agent_message.trim().is_empty() {
                            return Ok(latest_agent_message);
                        }

                        return Err("Codex completed without a replacement.".to_string());
                    }
                    "error" => {
                        if params.get("turnId").and_then(Value::as_str) == Some(turn_id) {
                            return Err(params
                                .get("error")
                                .and_then(|error| error.get("message"))
                                .and_then(Value::as_str)
                                .unwrap_or("Codex returned an error.")
                                .to_string());
                        }
                    }
                    _ => {}
                }
            }
        };

        timeout(Duration::from_secs(180), wait)
            .await
            .map_err(|_| "Codex rewrite timed out.".to_string())?
    }
}

fn spawn_stdin_writer<W>(
    mut stdin: W,
    mut rx: mpsc::Receiver<OutgoingMessage>,
    failure: TransportFailure,
) where
    W: AsyncWrite + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(outgoing) = rx.recv().await {
            let mut line = match serde_json::to_vec(&outgoing.value) {
                Ok(line) => line,
                Err(_) => {
                    if let Some(written) = outgoing.written {
                        let _ = written.send(Err(ProtocolError::SerializationFailed));
                    }
                    failure.fail(ProtocolError::SerializationFailed).await;
                    return;
                }
            };
            line.push(b'\n');

            if stdin.write_all(&line).await.is_err() || stdin.flush().await.is_err() {
                if let Some(written) = outgoing.written {
                    let _ = written.send(Err(ProtocolError::StdinClosed));
                }
                failure.fail(ProtocolError::StdinClosed).await;
                return;
            }

            if let Some(written) = outgoing.written {
                let _ = written.send(Ok(()));
            }
        }
    });
}

fn spawn_stdout_reader<R>(
    stdout: R,
    pending: PendingRequests,
    notifications: broadcast::Sender<Value>,
    tx: mpsc::Sender<OutgoingMessage>,
    failure: TransportFailure,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            let line = match lines.next_line().await {
                Ok(Some(line)) => line,
                Ok(None) | Err(_) => {
                    failure.fail(ProtocolError::StdoutClosed).await;
                    return;
                }
            };

            let message = match serde_json::from_str::<Value>(&line) {
                Ok(message) => message,
                Err(_) => {
                    failure.fail(ProtocolError::InvalidJson).await;
                    return;
                }
            };

            if is_server_request(&message) {
                let response = json!({
                    "id": message.get("id").cloned().unwrap_or(Value::Null),
                    "error": {
                        "code": -32601,
                        "message": "Codex Pencil does not support server-initiated requests."
                    }
                });
                let _ = tx
                    .send(OutgoingMessage {
                        value: response,
                        written: None,
                    })
                    .await;
                continue;
            }

            if let Some(id) = message.get("id").and_then(Value::as_u64) {
                let sender = pending.lock().await.remove(&id);
                if let Some(sender) = sender {
                    let result = if let Some(error) = message.get("error") {
                        Err(ProtocolError::Remote(error_message(error)))
                    } else {
                        Ok(message.get("result").cloned().unwrap_or(Value::Null))
                    };
                    let _ = sender.send(result);
                }
                continue;
            }

            if message.get("method").is_some() {
                let _ = notifications.send(message);
            }
        }
    });
}

fn spawn_stderr_drain(stderr: tokio::process::ChildStderr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        // Intentionally discard stderr. Codex diagnostics should not be surfaced
        // here because prompts may contain selected user text.
        while matches!(lines.next_line().await, Ok(Some(_))) {}
    });
}

fn spawn_child_watcher(
    mut child: tokio::process::Child,
    mut shutdown: oneshot::Receiver<()>,
    failure: TransportFailure,
) {
    tokio::spawn(async move {
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    failure.fail(ProtocolError::ChildExited).await;
                    return;
                }
                Ok(None) => {}
                Err(_) => {
                    failure.fail(ProtocolError::ChildWaitFailed).await;
                    return;
                }
            }

            tokio::select! {
                _ = &mut shutdown => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return;
                }
                _ = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
        }
    });
}

async fn fail_all_pending(pending: &PendingRequests, error: ProtocolError) {
    let mut pending = pending.lock().await;
    let requests = std::mem::take(&mut *pending);
    for (_, sender) in requests {
        let _ = sender.send(Err(error.clone()));
    }
}

fn is_server_request(message: &Value) -> bool {
    message.get("method").is_some()
        && message.get("id").is_some()
        && message.get("result").is_none()
        && message.get("error").is_none()
}

fn error_message(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| error.to_string())
}

fn account_label(account: &Value) -> Option<String> {
    match account.get("type").and_then(Value::as_str) {
        Some("chatgpt") => {
            let email = account
                .get("email")
                .and_then(Value::as_str)
                .unwrap_or("ChatGPT");
            let plan = account
                .get("planType")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            Some(format!("{email} ({plan})"))
        }
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Codex response was missing `{key}`."))
}

fn device_login_params() -> Value {
    json!({
        "type": "chatgptDeviceCode"
    })
}

fn rewrite_turn_params(thread_id: &str, prompt: &str) -> Value {
    json!({
        "threadId": thread_id,
        "input": [
            {
                "type": "text",
                "text": prompt,
                "text_elements": []
            }
        ],
        "approvalPolicy": "never",
        "outputSchema": rewrite_output_schema()
    })
}

pub(crate) fn rewrite_prompt(selected_text: &str, mode: RewriteMode) -> String {
    format!(
        "Rewrite the selected text for Codex Pencil.\n\
         Mode: {mode_label}\n\
         Instruction: {instruction}\n\n\
         Rules:\n\
         - Preserve the original meaning.\n\
         - Do not add new facts, claims, details, or promises.\n\
         - Preserve URLs, code, shell commands, product names, numbers, and email addresses exactly unless translation requires surrounding words to change.\n\
         - Keep the user's language unless the mode is translate_en or translate_ko.\n\
         - Preserve formatting where practical.\n\
         - Return strict JSON only. No Markdown, no prose before or after JSON, no code fences.\n\n\
         Expected JSON shape:\n\
         {{\"replacement\":\"...\",\"changed\":true,\"summary\":\"...\",\"edits\":[{{\"before\":\"...\",\"after\":\"...\",\"reason\":\"...\"}}],\"confidence\":0.0}}\n\n\
         Selected text:\n\
         <selection>\n{selected_text}\n</selection>",
        mode_label = mode.label(),
        instruction = mode.instruction()
    )
}

fn rewrite_output_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["replacement", "changed", "summary", "confidence"],
        "properties": {
            "replacement": {
                "type": "string",
                "description": "The complete non-empty text that should replace the selected text."
            },
            "changed": {
                "type": "boolean",
                "description": "Whether the replacement differs from the selected text."
            },
            "summary": {
                "type": "string",
                "description": "One short plain-language edit summary."
            },
            "edits": {
                "type": "array",
                "maxItems": 8,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["before", "after", "reason"],
                    "properties": {
                        "before": { "type": "string" },
                        "after": { "type": "string" },
                        "reason": { "type": "string" }
                    }
                }
            },
            "confidence": {
                "type": "number",
                "minimum": 0,
                "maximum": 1
            }
        }
    })
}

fn final_agent_message_from_turn(turn: &Value) -> Option<String> {
    let items = turn.get("items")?.as_array()?;
    let mut fallback = None;

    for item in items {
        if let Some(text) = agent_message_text(item) {
            if item.get("phase").and_then(Value::as_str) == Some("final_answer") {
                return Some(text);
            }
            fallback = Some(text);
        }
    }

    fallback
}

fn agent_message_text(item: &Value) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("agentMessage") {
        return None;
    }

    item.get("text")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(ToOwned::to_owned)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StructuredRewriteResult {
    replacement: String,
    changed: bool,
    summary: String,
    confidence: f64,
    #[serde(default)]
    edits: Vec<RewriteEdit>,
}

pub(crate) fn parse_rewrite_result(text: &str, mode: RewriteMode) -> Result<RewriteResult, String> {
    let structured = serde_json::from_str::<StructuredRewriteResult>(text)
        .map_err(|error| format!("Codex returned invalid structured JSON: {error}"))?;

    if structured.replacement.is_empty() {
        return Err("Codex JSON included an empty replacement.".to_string());
    }
    if !structured.confidence.is_finite() || !(0.0..=1.0).contains(&structured.confidence) {
        return Err("Codex JSON included confidence outside 0 through 1.".to_string());
    }
    if structured.edits.len() > 8 {
        return Err("Codex JSON included more than 8 edit details.".to_string());
    }

    Ok(RewriteResult {
        replacement: structured.replacement,
        changed: structured.changed,
        summary: structured.summary,
        edits: structured.edits,
        confidence: structured.confidence,
        mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{
        duplex, split, AsyncBufReadExt, AsyncWriteExt, BufReader, Lines, ReadHalf, WriteHalf,
    };

    struct FakeAppServer {
        lines: Lines<BufReader<ReadHalf<tokio::io::DuplexStream>>>,
        writer: WriteHalf<tokio::io::DuplexStream>,
    }

    fn fake_transport() -> (CodexClient, FakeAppServer, TransportFailure) {
        let (client_stream, server_stream) = duplex(16 * 1024);
        let (client_stdout, client_stdin) = split(client_stream);
        let (server_stdin, server_stdout) = split(server_stream);
        let (client, failure) =
            CodexClient::from_io(client_stdout, client_stdin, ChildShutdown::detached());

        (
            client,
            FakeAppServer {
                lines: BufReader::new(server_stdin).lines(),
                writer: server_stdout,
            },
            failure,
        )
    }

    impl FakeAppServer {
        async fn receive(&mut self) -> Value {
            let line = timeout(Duration::from_secs(1), self.lines.next_line())
                .await
                .expect("fake app-server timed out waiting for a client message")
                .expect("fake app-server could not read a client message")
                .expect("client stdout closed before the expected message");
            serde_json::from_str(&line).expect("client emitted malformed JSON")
        }

        async fn send(&mut self, message: Value) {
            let mut line = serde_json::to_vec(&message).expect("fixture response must serialize");
            line.push(b'\n');
            self.writer
                .write_all(&line)
                .await
                .expect("fixture response must be writable");
            self.writer
                .flush()
                .await
                .expect("fixture response must flush");
        }

        async fn send_raw(&mut self, line: &[u8]) {
            self.writer
                .write_all(line)
                .await
                .expect("fixture bytes must be writable");
            self.writer.flush().await.expect("fixture bytes must flush");
        }

        async fn close_stdout(&mut self) {
            self.writer
                .shutdown()
                .await
                .expect("fixture stdout must close");
        }

        async fn expect_no_message(&mut self) {
            assert!(timeout(Duration::from_millis(50), self.lines.next_line())
                .await
                .is_err());
        }
    }

    async fn complete_handshake(client: &CodexClient, server: &mut FakeAppServer) -> Vec<Value> {
        let server_flow = async {
            let initialize = server.receive().await;
            let id = initialize
                .get("id")
                .and_then(Value::as_u64)
                .expect("initialize must have a numeric id");
            server
                .send(json!({
                    "id": id,
                    "result": {
                        "codexHome": "C:\\codex-fixture",
                        "platformFamily": "windows",
                        "platformOs": "windows",
                        "userAgent": "codex-fixture"
                    }
                }))
                .await;
            let initialized = server.receive().await;
            vec![initialize, initialized]
        };

        let (handshake, messages) = tokio::join!(client.perform_handshake(), server_flow);
        handshake.expect("handshake must succeed against the fixture");
        messages
    }

    #[tokio::test]
    async fn handshake_waits_for_initialize_response_then_sends_initialized_before_account_read() {
        let (client, mut server, _failure) = fake_transport();
        let messages = complete_handshake(&client, &mut server).await;

        assert_eq!(
            messages[0].get("method").and_then(Value::as_str),
            Some("initialize")
        );
        assert!(messages[0].get("id").is_some());
        assert!(messages[0]
            .pointer("/params/capabilities/experimentalApi")
            .is_none());
        assert_eq!(
            messages[1].get("method").and_then(Value::as_str),
            Some("initialized")
        );
        assert!(messages[1].get("id").is_none());

        let server_flow = async {
            let request = server.receive().await;
            assert_eq!(
                request.get("method").and_then(Value::as_str),
                Some("account/read")
            );
            assert_eq!(
                request.pointer("/params/refreshToken"),
                Some(&Value::Bool(false))
            );
            let id = request
                .get("id")
                .and_then(Value::as_u64)
                .expect("account/read must have a numeric id");
            server
                .send(json!({
                    "id": id,
                    "result": { "account": null, "requiresOpenaiAuth": true }
                }))
                .await;
        };
        let (status, ()) = tokio::join!(client.auth_status(), server_flow);
        let status = status.expect("account/read must parse");
        assert!(!status.logged_in);
        assert!(status.requires_openai_auth);
    }

    #[tokio::test]
    async fn rejects_requests_before_handshake_without_writing_them() {
        let (client, mut server, _failure) = fake_transport();

        let error = client
            .request_protocol(
                "account/read",
                json!({ "refreshToken": false }),
                Duration::from_secs(1),
            )
            .await
            .expect_err("pre-handshake request must fail");

        assert_eq!(error, ProtocolError::NotReady);
        assert!(client.pending.lock().await.is_empty());
        server.expect_no_message().await;
    }

    #[tokio::test]
    async fn repeated_initialize_is_rejected_without_a_second_wire_request() {
        let (client, mut server, _failure) = fake_transport();
        complete_handshake(&client, &mut server).await;

        let error = client
            .perform_handshake()
            .await
            .expect_err("a connection can initialize only once");

        assert_eq!(error, ProtocolError::AlreadyInitialized);
        server.expect_no_message().await;
    }

    #[tokio::test]
    async fn malformed_json_fails_initialize_and_cleans_pending_requests() {
        let (client, mut server, _failure) = fake_transport();
        let server_flow = async {
            let request = server.receive().await;
            assert_eq!(
                request.get("method").and_then(Value::as_str),
                Some("initialize")
            );
            server.send_raw(b"{malformed-json}\n").await;
        };

        let (result, ()) = tokio::join!(client.perform_handshake(), server_flow);

        assert_eq!(
            result.expect_err("malformed JSON must fail the handshake"),
            ProtocolError::InvalidJson
        );
        assert!(!client.is_healthy());
        assert!(client.pending.lock().await.is_empty());
    }

    #[tokio::test]
    async fn request_timeout_removes_the_pending_request() {
        let (client, mut server, _failure) = fake_transport();
        complete_handshake(&client, &mut server).await;

        let server_flow = async {
            let request = server.receive().await;
            assert_eq!(
                request.get("method").and_then(Value::as_str),
                Some("account/read")
            );
        };
        let request = client.request_protocol(
            "account/read",
            json!({ "refreshToken": false }),
            Duration::from_millis(25),
        );
        let (result, ()) = tokio::join!(request, server_flow);

        assert_eq!(
            result.expect_err("unanswered request must time out"),
            ProtocolError::Timeout("account/read".to_string())
        );
        assert!(client.pending.lock().await.is_empty());
    }

    #[tokio::test]
    async fn stdout_closure_invalidates_client_and_fails_pending_request() {
        let (client, mut server, _failure) = fake_transport();
        complete_handshake(&client, &mut server).await;

        let server_flow = async {
            let request = server.receive().await;
            assert_eq!(
                request.get("method").and_then(Value::as_str),
                Some("account/read")
            );
            server.close_stdout().await;
        };
        let request = client.request_protocol(
            "account/read",
            json!({ "refreshToken": false }),
            Duration::from_secs(1),
        );
        let (result, ()) = tokio::join!(request, server_flow);

        assert_eq!(
            result.expect_err("stdout closure must fail pending RPCs"),
            ProtocolError::StdoutClosed
        );
        assert!(!client.is_healthy());
        assert!(client.pending.lock().await.is_empty());
    }

    #[tokio::test]
    async fn child_exit_invalidates_client_and_fails_pending_request_with_typed_error() {
        let (client, mut server, failure) = fake_transport();
        complete_handshake(&client, &mut server).await;

        let server_flow = async {
            let request = server.receive().await;
            assert_eq!(
                request.get("method").and_then(Value::as_str),
                Some("account/read")
            );
            failure.fail(ProtocolError::ChildExited).await;
        };
        let request = client.request_protocol(
            "account/read",
            json!({ "refreshToken": false }),
            Duration::from_secs(1),
        );
        let (result, ()) = tokio::join!(request, server_flow);

        assert_eq!(
            result.expect_err("child exit must fail pending RPCs"),
            ProtocolError::ChildExited
        );
        assert!(!client.is_healthy());
        assert!(client.pending.lock().await.is_empty());
    }

    #[tokio::test]
    async fn auth_notifications_are_forwarded_after_handshake() {
        let (client, mut server, _failure) = fake_transport();
        complete_handshake(&client, &mut server).await;
        let mut notifications = client.subscribe();

        server
            .send(json!({
                "method": "account/updated",
                "params": { "authMode": "chatgpt", "planType": "plus" }
            }))
            .await;
        let account_updated = timeout(Duration::from_secs(1), notifications.recv())
            .await
            .expect("account/updated notification must arrive")
            .expect("notification channel must stay open");

        server
            .send(json!({
                "method": "account/login/completed",
                "params": { "loginId": null, "success": true, "error": null }
            }))
            .await;
        let login_completed = timeout(Duration::from_secs(1), notifications.recv())
            .await
            .expect("account/login/completed notification must arrive")
            .expect("notification channel must stay open");

        assert_eq!(
            account_updated.get("method").and_then(Value::as_str),
            Some("account/updated")
        );
        assert_eq!(
            login_completed.get("method").and_then(Value::as_str),
            Some("account/login/completed")
        );
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires the exact locally installed Codex CLI"]
    async fn live_supported_app_server_completes_handshake_and_account_read() {
        let client = CodexClient::connect()
            .await
            .expect("exact supported Codex app-server must complete the handshake");

        client
            .auth_status()
            .await
            .expect("account/read must succeed without exposing the response payload");
        assert!(client.is_healthy());
    }

    #[tokio::test]
    async fn cache_reconnects_once_after_child_invalidation_then_reuses_the_new_client() {
        let cache = CodexClientCache::default();
        let (first_source, mut first_server, first_failure) = fake_transport();
        complete_handshake(&first_source, &mut first_server).await;
        let first = cache
            .get_or_connect_with(|| async { Ok(first_source) })
            .await
            .expect("first connection must be cached");

        first_failure.fail(ProtocolError::ChildExited).await;
        assert!(!first.is_healthy());

        let (second_source, mut second_server, _second_failure) = fake_transport();
        complete_handshake(&second_source, &mut second_server).await;
        let reconnects = AtomicU64::new(0);
        let second = cache
            .get_or_connect_with(|| async {
                reconnects.fetch_add(1, Ordering::SeqCst);
                Ok(second_source)
            })
            .await
            .expect("next request must reconnect once");
        let reused = cache
            .get_or_connect_with(|| async {
                reconnects.fetch_add(1, Ordering::SeqCst);
                Err("healthy cached client must not reconnect".to_string())
            })
            .await
            .expect("healthy replacement must be reused");

        assert_eq!(reconnects.load(Ordering::SeqCst), 1);
        assert!(Arc::ptr_eq(&second, &reused));
    }

    #[tokio::test]
    async fn structured_rewrite_round_trip_uses_stable_requests_and_strict_result_contract() {
        let (client, mut server, _failure) = fake_transport();
        complete_handshake(&client, &mut server).await;

        let server_flow = async {
            let thread_request = server.receive().await;
            assert_eq!(
                thread_request.get("method").and_then(Value::as_str),
                Some("thread/start")
            );
            let thread_request_id = thread_request
                .get("id")
                .and_then(Value::as_u64)
                .expect("thread/start must have a numeric id");
            server
                .send(json!({
                    "id": thread_request_id,
                    "result": { "thread": { "id": "thread-fixture" } }
                }))
                .await;

            let turn_request = server.receive().await;
            assert_eq!(
                turn_request.get("method").and_then(Value::as_str),
                Some("turn/start")
            );
            assert!(turn_request
                .pointer("/params/responsesapiClientMetadata")
                .is_none());
            assert_eq!(
                turn_request.pointer("/params/outputSchema/additionalProperties"),
                Some(&Value::Bool(false))
            );
            let turn_request_id = turn_request
                .get("id")
                .and_then(Value::as_u64)
                .expect("turn/start must have a numeric id");
            server
                .send(json!({
                    "id": turn_request_id,
                    "result": { "turn": { "id": "turn-fixture" } }
                }))
                .await;
            server
                .send(json!({
                    "method": "turn/completed",
                    "params": {
                        "threadId": "thread-fixture",
                        "turn": {
                            "id": "turn-fixture",
                            "status": "completed",
                            "items": [{
                                "type": "agentMessage",
                                "phase": "final_answer",
                                "text": "{\"replacement\":\" revised fixture \",\"changed\":true,\"summary\":\"Adjusted grammar.\",\"confidence\":0.91}"
                            }]
                        }
                    }
                }))
                .await;
        };

        let (result, ()) = tokio::join!(
            client.rewrite("synthetic fixture input", RewriteMode::Grammar),
            server_flow
        );
        let result = result.expect("structured rewrite round trip must succeed");

        assert_eq!(result.replacement, " revised fixture ");
        assert!(result.changed);
        assert_eq!(result.summary, "Adjusted grammar.");
        assert_eq!(result.confidence, 0.91);
    }

    #[test]
    fn parses_strict_rewrite_json_without_trimming_replacement() {
        let result = parse_rewrite_result(
            r#"{"replacement":" \nHello.\n  ","changed":true,"summary":"Fixed punctuation.","edits":[{"before":"Hello","after":"Hello.","reason":"Added punctuation"}],"confidence":0.82}"#,
            RewriteMode::Grammar,
        )
        .unwrap();

        assert_eq!(result.replacement, " \nHello.\n  ");
        assert!(result.changed);
        assert_eq!(result.summary, "Fixed punctuation.");
        assert_eq!(result.edits.len(), 1);
        assert_eq!(result.confidence, 0.82);
    }

    #[test]
    fn rejects_wrapped_output_instead_of_extracting_json() {
        let result = parse_rewrite_result(
            "Here is the JSON: {\"replacement\":\"Done\",\"changed\":true,\"summary\":\"Updated tone\",\"confidence\":0.9}",
            RewriteMode::Natural,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_replacement() {
        let result = parse_rewrite_result(
            r#"{"replacement":"","changed":false,"summary":"No change","confidence":0.5}"#,
            RewriteMode::Grammar,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_or_malformed_required_fields_instead_of_defaulting() {
        let result = parse_rewrite_result(
            r#"{"replacement":"Hi","summary":"Shortened","confidence":"high"}"#,
            RewriteMode::Concise,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_out_of_range_confidence_and_unknown_properties() {
        let out_of_range = parse_rewrite_result(
            r#"{"replacement":"Hi","changed":true,"summary":"Shortened","confidence":1.1}"#,
            RewriteMode::Concise,
        );
        let unknown_property = parse_rewrite_result(
            r#"{"replacement":"Hi","changed":true,"summary":"Shortened","confidence":0.8,"extra":true}"#,
            RewriteMode::Concise,
        );

        assert!(out_of_range.is_err());
        assert!(unknown_property.is_err());
    }

    #[test]
    fn rewrite_schema_requires_stable_contract_and_forbids_additional_properties() {
        let schema = rewrite_output_schema();
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .expect("schema must declare required fields");

        for field in ["replacement", "changed", "summary", "confidence"] {
            assert!(required.contains(&Value::String(field.to_string())));
        }
        assert_eq!(
            schema.get("additionalProperties"),
            Some(&Value::Bool(false))
        );
        assert_eq!(
            schema.pointer("/properties/confidence/minimum"),
            Some(&json!(0))
        );
        assert_eq!(
            schema.pointer("/properties/confidence/maximum"),
            Some(&json!(1))
        );
    }

    #[test]
    fn device_login_uses_schema_stable_chatgpt_managed_variant() {
        assert_eq!(
            device_login_params(),
            json!({ "type": "chatgptDeviceCode" })
        );
    }

    #[test]
    fn turn_start_omits_undocumented_metadata_field() {
        let params = rewrite_turn_params("thread-fixture", "prompt-fixture");

        assert!(params.get("responsesapiClientMetadata").is_none());
        assert_eq!(
            params.get("threadId").and_then(Value::as_str),
            Some("thread-fixture")
        );
        assert!(params.get("outputSchema").is_some());
    }

    #[test]
    fn prompt_keeps_language_unless_translation() {
        let prompt = rewrite_prompt("안녕하세요", RewriteMode::Polite);
        assert!(prompt
            .contains("Keep the user's language unless the mode is translate_en or translate_ko."));
        assert!(prompt.contains(
            "Preserve URLs, code, shell commands, product names, numbers, and email addresses"
        ));
    }
}
