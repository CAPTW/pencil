use crate::settings::RewriteMode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::{broadcast, mpsc, oneshot, Mutex},
    time::timeout,
};

pub struct CodexClient {
    tx: mpsc::Sender<Value>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    notifications: broadcast::Sender<Value>,
    next_id: AtomicU64,
    _shutdown: ChildShutdown,
}

struct ChildShutdown {
    tx: StdMutex<Option<oneshot::Sender<()>>>,
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
#[serde(rename_all = "camelCase")]
pub struct RewriteEdit {
    pub before: String,
    pub after: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteResult {
    pub replacement: String,
    pub summary: String,
    pub edits: Vec<RewriteEdit>,
    pub confidence: f64,
    pub mode: RewriteMode,
}

impl CodexClient {
    pub async fn connect() -> Result<Self, String> {
        let mut command = codex_command().ok_or_else(missing_codex_message)?;

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

        let (tx, mut rx) = mpsc::channel::<Value>(64);
        let (notifications, _) = broadcast::channel::<Value>(256);
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(message) = rx.recv().await {
                let Ok(line) = serde_json::to_string(&message) else {
                    continue;
                };

                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        spawn_stdout_reader(stdout, pending.clone(), notifications.clone(), tx.clone());
        spawn_stderr_drain(stderr);
        spawn_child_watcher(child, shutdown_rx, pending.clone(), notifications.clone());

        let client = Self {
            tx,
            pending,
            notifications,
            next_id: AtomicU64::new(1),
            _shutdown: ChildShutdown {
                tx: StdMutex::new(Some(shutdown_tx)),
            },
        };

        client.initialize().await?;
        Ok(client)
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

        let account = result.get("account");
        let requires_openai_auth = result
            .get("requiresOpenaiAuth")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        Ok(AuthStatus {
            logged_in: account.is_some_and(|value| !value.is_null()),
            account_label: account.and_then(account_label),
            auth_mode: account
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            requires_openai_auth,
        })
    }

    pub async fn start_device_login(&self) -> Result<DeviceLogin, String> {
        let result = self
            .request(
                "account/login/start",
                json!({
                    "type": "chatgptDeviceCode"
                }),
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
        let _ = self
            .request(
                "account/login/cancel",
                json!({
                    "loginId": login_id
                }),
                Duration::from_secs(15),
            )
            .await?;
        Ok(())
    }

    pub async fn rewrite(&self, selected_text: &str, mode: RewriteMode) -> Result<RewriteResult, String> {
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
        let turn = self
            .request(
                "turn/start",
                json!({
                    "threadId": thread_id,
                    "input": [
                        {
                            "type": "text",
                            "text": rewrite_prompt(selected_text, mode),
                            "text_elements": []
                        }
                    ],
                    "approvalPolicy": "never",
                    "outputSchema": rewrite_output_schema(),
                    "responsesapiClientMetadata": {
                        "codex_pencil_action": "rewrite",
                        "codex_pencil_mode": mode.label()
                    }
                }),
                Duration::from_secs(30),
            )
            .await?;

        let turn_id = turn
            .get("turn")
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| "Codex did not return a turn id.".to_string())?
            .to_string();

        let final_text = self.wait_for_turn(&mut notifications, &thread_id, &turn_id).await?;
        parse_rewrite_result(&final_text, mode)
    }

    async fn initialize(&self) -> Result<(), String> {
        let _ = self
            .request(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "codex-pencil",
                        "title": "Codex Pencil",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "experimentalApi": true,
                        "requestAttestation": false,
                        "optOutNotificationMethods": [
                            "command/exec/outputDelta",
                            "process/outputDelta",
                            "item/commandExecution/outputDelta"
                        ]
                    }
                }),
                Duration::from_secs(30),
            )
            .await?;
        Ok(())
    }

    async fn request(&self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        let id = self.next_request_id()?;
        let (tx, rx) = oneshot::channel();

        self.pending.lock().await.insert(id, tx);

        let message = json!({
            "id": id,
            "method": method,
            "params": params
        });

        if self.tx.send(message).await.is_err() {
            self.pending.lock().await.remove(&id);
            return Err("Codex app-server is not accepting requests. It may have exited unexpectedly.".to_string());
        }

        match timeout(wait, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("Codex app-server closed the request channel.".to_string()),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(format!("Codex request timed out: {method}"))
            }
        }
    }

    fn next_request_id(&self) -> Result<u64, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if id == u64::MAX {
            return Err("Codex request id counter exhausted.".to_string());
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

                let method = notification.get("method").and_then(Value::as_str).unwrap_or_default();
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
                            if let Some(text) = agent_message_text(params.get("item").unwrap_or(&Value::Null)) {
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

fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    notifications: broadcast::Sender<Value>,
    tx: mpsc::Sender<Value>,
) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let message = match serde_json::from_str::<Value>(&line) {
                Ok(message) => message,
                Err(_) => {
                    fail_all_pending(&pending, "Codex app-server returned invalid protocol JSON.").await;
                    let _ = notifications.send(json!({
                        "method": "codex/process/exited",
                        "params": {
                            "message": "Codex app-server returned invalid protocol JSON."
                        }
                    }));
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
                let _ = tx.send(response).await;
                continue;
            }

            if let Some(id) = message.get("id").and_then(Value::as_u64) {
                let sender = pending.lock().await.remove(&id);
                if let Some(sender) = sender {
                    let result = if let Some(error) = message.get("error") {
                        Err(error_message(error))
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

        fail_all_pending(&pending, "Codex app-server exited unexpectedly.").await;
        let _ = notifications.send(json!({
            "method": "codex/process/exited",
            "params": {
                "message": "Codex app-server exited unexpectedly."
            }
        }));
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
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    notifications: broadcast::Sender<Value>,
) {
    tokio::spawn(async move {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let message = format!("Codex app-server exited with status {status}.");
                    fail_all_pending(&pending, &message).await;
                    let _ = notifications.send(json!({
                        "method": "codex/process/exited",
                        "params": { "message": message }
                    }));
                    return;
                }
                Ok(None) => {}
                Err(error) => {
                    let message = format!("Could not wait for Codex app-server: {error}");
                    fail_all_pending(&pending, &message).await;
                    let _ = notifications.send(json!({
                        "method": "codex/process/exited",
                        "params": { "message": message }
                    }));
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

async fn fail_all_pending(
    pending: &Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    message: &str,
) {
    let mut pending = pending.lock().await;
    let requests = std::mem::take(&mut *pending);
    for (_, sender) in requests {
        let _ = sender.send(Err(message.to_string()));
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
            let email = account.get("email").and_then(Value::as_str).unwrap_or("ChatGPT");
            let plan = account.get("planType").and_then(Value::as_str).unwrap_or("unknown");
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
         {{\"replacement\":\"...\",\"summary\":\"...\",\"edits\":[{{\"before\":\"...\",\"after\":\"...\",\"reason\":\"...\"}}],\"confidence\":0.0}}\n\n\
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
        "required": ["replacement", "summary", "edits", "confidence"],
        "properties": {
            "replacement": {
                "type": "string",
                "description": "The complete non-empty text that should replace the selected text."
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

pub(crate) fn parse_rewrite_result(text: &str, mode: RewriteMode) -> Result<RewriteResult, String> {
    let value = parse_json_object(text).map_err(|error| format!("Codex returned invalid JSON: {error}"))?;
    let replacement = value
        .get("replacement")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Codex JSON did not include a non-empty replacement.".to_string())?;

    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Rewrote text in {} mode.", mode.label()));

    let edits = value
        .get("edits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(parse_edit)
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let confidence = value
        .get("confidence")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    Ok(RewriteResult {
        replacement,
        summary,
        edits,
        confidence,
        mode,
    })
}

fn parse_edit(value: &Value) -> Option<RewriteEdit> {
    Some(RewriteEdit {
        before: value.get("before")?.as_str()?.trim().to_string(),
        after: value.get("after")?.as_str()?.trim().to_string(),
        reason: value.get("reason")?.as_str()?.trim().to_string(),
    })
}

fn parse_json_object(text: &str) -> Result<Value, serde_json::Error> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }

    let without_fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);

    if let Ok(value) = serde_json::from_str(without_fence) {
        return Ok(value);
    }

    if let Some(candidate) = extract_first_json_object(trimmed) {
        return serde_json::from_str(candidate);
    }

    serde_json::from_str(trimmed)
}

fn extract_first_json_object(text: &str) -> Option<&str> {
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in text.char_indices() {
        if start.is_none() {
            if ch == '{' {
                start = Some(index);
                depth = 1;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let object_start = start?;
                    return text.get(object_start..=index);
                }
            }
            _ => {}
        }
    }

    None
}

fn codex_command() -> Option<Command> {
    if let Ok(path) = env::var("CODEX_PENCIL_CODEX_BIN") {
        if !path.trim().is_empty() {
            return Some(Command::new(path));
        }
    }

    #[cfg(windows)]
    {
        for candidate in ["codex.cmd", "codex.exe"] {
            if let Some(path) = where_first(candidate) {
                return Some(Command::new(path));
            }
        }
    }

    #[cfg(not(windows))]
    {
        Some(Command::new("codex"))
    }

    #[cfg(windows)]
    None
}

fn missing_codex_message() -> String {
    "Codex CLI is required at runtime. Install it with `npm install -g @openai/codex` and ensure `codex` is available on PATH.".to_string()
}

#[cfg(windows)]
fn where_first(name: &str) -> Option<String> {
    let output = std::process::Command::new("where").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strict_rewrite_json() {
        let result = parse_rewrite_result(
            r#"{"replacement":"Hello.","summary":"Fixed punctuation.","edits":[{"before":"Hello","after":"Hello.","reason":"Added punctuation"}],"confidence":0.82}"#,
            RewriteMode::Grammar,
        )
        .unwrap();

        assert_eq!(result.replacement, "Hello.");
        assert_eq!(result.summary, "Fixed punctuation.");
        assert_eq!(result.edits.len(), 1);
        assert_eq!(result.confidence, 0.82);
    }

    #[test]
    fn extracts_first_json_object_from_wrapped_output() {
        let result = parse_rewrite_result(
            "Here is the JSON: {\"replacement\":\"Done\",\"summary\":\"Updated tone\",\"edits\":[],\"confidence\":1.4} trailing text",
            RewriteMode::Natural,
        )
        .unwrap();

        assert_eq!(result.replacement, "Done");
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn rejects_empty_replacement() {
        let result = parse_rewrite_result(
            r#"{"replacement":"   ","summary":"No change","edits":[],"confidence":0.5}"#,
            RewriteMode::Grammar,
        );

        assert!(result.is_err());
    }

    #[test]
    fn defaults_malformed_edits_to_empty() {
        let result = parse_rewrite_result(
            r#"{"replacement":"Hi","summary":"Shortened","edits":[{"before":4}],"confidence":-0.1}"#,
            RewriteMode::Concise,
        )
        .unwrap();

        assert!(result.edits.is_empty());
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn prompt_keeps_language_unless_translation() {
        let prompt = rewrite_prompt("안녕하세요", RewriteMode::Polite);
        assert!(prompt.contains("Keep the user's language unless the mode is translate_en or translate_ko."));
        assert!(prompt.contains("Preserve URLs, code, shell commands, product names, numbers, and email addresses"));
    }
}
