//! Opt-in OpenAI-compatible chat-completions transport foundation.
//! Desktop API sessions bind
//! consent, source/revision, matched terms and the writing-result contract.
//! No persistence, credential discovery, automatic retry or engine fallback.
use super::executor::DeepCancellation;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Url,
};
use serde_json::{json, Value};
use std::time::Duration;

const MAX_REQUEST: usize = 128 * 1024;
const MAX_RESPONSE: usize = 256 * 1024;
const DEADLINE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiError {
    RequestAlreadyUsed,
    Configuration,
    ConsentRequired,
    InvalidKey,
    InputLimit,
    Cancelled,
    Timeout,
    Transport,
    HttpStatus(u16),
    OutputLimit,
    InvalidResponse,
    IncompleteResponse,
}

/// Immutable destination/model. A different destination requires a new config.
/// Full Chat Completions endpoint or the exact Anthropic Messages endpoint.
pub struct ApiConfig {
    endpoint: Url,
    model: String,
    anthropic: bool,
    openrouter: bool,
    google: bool,
}
impl ApiConfig {
    #[cfg(test)]
    pub(crate) fn fixture(endpoint: &str) -> Self {
        Self::validate(endpoint, "fixture", true).unwrap()
    }
    pub fn new(endpoint: &str, model: &str) -> Result<Self, ApiError> {
        Self::validate(endpoint, model, false)
    }
    fn validate(endpoint: &str, model: &str, fixture: bool) -> Result<Self, ApiError> {
        if endpoint.len() > 2048
            || endpoint.chars().any(char::is_whitespace)
            || model.is_empty()
            || model.len() > 256
            || model.trim() != model
            || model.chars().any(char::is_control)
        {
            return Err(ApiError::Configuration);
        }
        let url = Url::parse(endpoint).map_err(|_| ApiError::Configuration)?;
        let owned_fixture =
            cfg!(test) && fixture && url.scheme() == "http" && url.host_str() == Some("127.0.0.1");
        let anthropic = url.as_str() == "https://api.anthropic.com/v1/messages";
        let openrouter = url.host_str() == Some("openrouter.ai");
        let google = url.host_str() == Some("generativelanguage.googleapis.com");
        if openrouter
            && (model == "openrouter/auto"
                || model.starts_with("@")
                || model.contains(":")
                || !model.contains("/"))
        {
            return Err(ApiError::Configuration);
        }
        if (!owned_fixture && url.scheme() != "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || (!anthropic && !url.path().ends_with("/chat/completions"))
        {
            return Err(ApiError::Configuration);
        }
        Ok(Self {
            endpoint: url,
            model: model.to_owned(),
            anthropic,
            openrouter,
            google,
        })
    }
}

/// Borrow the caller's in-memory key for this single explicit request. Neither
/// key nor request/response bodies are logged, serialized to disk or in errors.
/// Cancel ends local HTTP waiting; it cannot promise the remote server stopped
/// computing or charging for a request it already received.
pub async fn complete(
    config: &ApiConfig,
    key: &str,
    prompt: &str,
    explicit_consent: bool,
    cancellation: &DeepCancellation,
) -> Result<String, ApiError> {
    run(
        config,
        key,
        prompt,
        explicit_consent,
        cancellation,
        DEADLINE,
    )
    .await
}
async fn run(
    config: &ApiConfig,
    key: &str,
    prompt: &str,
    consent: bool,
    cancel: &DeepCancellation,
    deadline: Duration,
) -> Result<String, ApiError> {
    if !consent {
        return Err(ApiError::ConsentRequired);
    }
    if cancel.is_cancelled() {
        return Err(ApiError::Cancelled);
    }
    if prompt.trim().is_empty() || prompt.len() > MAX_REQUEST {
        return Err(ApiError::InputLimit);
    }
    validate_key(key)?;
    let credential = zeroize::Zeroizing::new(if config.anthropic {
        key.to_owned()
    } else {
        format!("Bearer {key}")
    });
    let mut authorization = HeaderValue::from_str(&credential).map_err(|_| ApiError::InvalidKey)?;
    authorization.set_sensitive(true);
    let body =
        serde_json::to_vec(&request_body(config, prompt)).map_err(|_| ApiError::InputLimit)?;
    if body.len() > MAX_REQUEST {
        return Err(ApiError::InputLimit);
    }
    let operation = async {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .connect_timeout(Duration::from_secs(10))
            .timeout(deadline)
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|_| ApiError::Transport)?;
        let request = client.post(config.endpoint.clone());
        let request = if config.anthropic {
            request
                .header("x-api-key", authorization)
                .header("anthropic-version", "2023-06-01")
        } else {
            request.header(AUTHORIZATION, authorization)
        };
        let mut response = request
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(http_error)?;
        if !response.status().is_success() {
            return Err(ApiError::HttpStatus(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE as u64)
        {
            return Err(ApiError::OutputLimit);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(http_error)? {
            if chunk.len() > MAX_RESPONSE.saturating_sub(bytes.len()) {
                return Err(ApiError::OutputLimit);
            }
            bytes.extend_from_slice(&chunk);
        }
        if config.anthropic {
            parse_anthropic_response(&bytes)
        } else {
            parse_response(&bytes)
        }
    };
    tokio::select! {biased;
        _=async {loop {if cancel.is_cancelled(){break;}tokio::time::sleep(Duration::from_millis(10)).await;}} => Err(ApiError::Cancelled),
        result=tokio::time::timeout(deadline,operation) => {
            if cancel.is_cancelled(){Err(ApiError::Cancelled)} else {result.map_err(|_|ApiError::Timeout)?}
        }
    }
}
fn request_body(config: &ApiConfig, prompt: &str) -> Value {
    let mut body =
        json!({"model":config.model,"messages":[{"role":"user","content":prompt}],"stream":false});
    if config.anthropic {
        body["max_tokens"] = json!(4096);
    } else if !config.google {
        body["store"] = json!(false);
    }
    if config.openrouter {
        body["provider"] = json!({"allow_fallbacks":false,"data_collection":"deny","zdr":true});
    }
    body
}
fn parse_anthropic_response(bytes: &[u8]) -> Result<String, ApiError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ApiError::InvalidResponse)?;
    if value["type"] != "message" || value["role"] != "assistant" {
        return Err(ApiError::InvalidResponse);
    }
    if value["stop_reason"] != "end_turn" {
        return Err(ApiError::IncompleteResponse);
    }
    let blocks = value["content"]
        .as_array()
        .ok_or(ApiError::InvalidResponse)?;
    let mut output = String::new();
    for block in blocks {
        if block["type"] != "text" {
            return Err(ApiError::InvalidResponse);
        }
        output.push_str(block["text"].as_str().ok_or(ApiError::InvalidResponse)?);
    }
    if output.trim().is_empty() {
        return Err(ApiError::InvalidResponse);
    }
    Ok(output)
}
pub(crate) fn validate_key(key: &str) -> Result<(), ApiError> {
    if key.is_empty() || key.len() > 4096 || !key.bytes().all(|c| (33..=126).contains(&c)) {
        return Err(ApiError::InvalidKey);
    }
    Ok(())
}
fn http_error(error: reqwest::Error) -> ApiError {
    if error.is_timeout() {
        ApiError::Timeout
    } else {
        ApiError::Transport
    }
}
fn parse_response(bytes: &[u8]) -> Result<String, ApiError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ApiError::InvalidResponse)?;
    let choices = value["choices"]
        .as_array()
        .ok_or(ApiError::InvalidResponse)?;
    if choices.len() != 1 {
        return Err(ApiError::InvalidResponse);
    }
    let choice = &choices[0];
    let message = &choice["message"];
    if choice["finish_reason"] != "stop" {
        return Err(ApiError::IncompleteResponse);
    }
    if !message["tool_calls"].is_null()
        || !message["function_call"].is_null()
        || !message["refusal"].is_null()
    {
        return Err(ApiError::InvalidResponse);
    }
    let content = message["content"]
        .as_str()
        .ok_or(ApiError::InvalidResponse)?;
    if content.trim().is_empty() {
        return Err(ApiError::InvalidResponse);
    }
    Ok(content.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    const REPLY: &str =
        r#"{"choices":[{"finish_reason":"stop","message":{"content":"synthetic correction"}}]}"#;
    async fn fixture(
        status: &str,
        body: &str,
        extra: &str,
        delay: Duration,
    ) -> (
        ApiConfig,
        tokio::task::JoinHandle<Vec<u8>>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = ApiConfig::validate(
            &format!(
                "http://{}/v1/chat/completions",
                listener.local_addr().unwrap()
            ),
            "fixture-model",
            true,
        )
        .unwrap();
        let length = if extra.starts_with("Transfer-Encoding:") {
            String::new()
        } else {
            format!("Content-Length: {}\r\n", body.len())
        };
        let reply = format!("HTTP/1.1 {status}\r\n{length}Connection: close\r\n{extra}\r\n{body}");
        let (sent, received) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let Ok(Ok((mut socket, _))) =
                tokio::time::timeout(Duration::from_secs(2), listener.accept()).await
            else {
                return Vec::new();
            };
            let mut data = Vec::new();
            let mut byte = [0];
            while !data.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).await.unwrap();
                data.push(byte[0]);
                assert!(data.len() < 8192);
            }
            let headers = String::from_utf8_lossy(&data).to_lowercase();
            let length = headers
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            data.extend(body);
            let _ = sent.send(());
            tokio::time::sleep(delay).await;
            let _ = socket.write_all(reply.as_bytes()).await;
            data
        });
        (config, task, received)
    }
    #[test]
    fn provider_configuration_and_payloads() {
        for (endpoint, model) in [
            ("https://api.anthropic.com/v1/messages", "claude-test"),
            (
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
                "gemini-test",
            ),
            (
                "https://openrouter.ai/api/v1/chat/completions",
                "vendor/model",
            ),
            ("https://api.x.ai/v1/chat/completions", "grok-test"),
        ] {
            let config = ApiConfig::new(endpoint, model).unwrap();
            let body = request_body(&config, "synthetic source");
            assert_eq!(body["model"], model);
            assert_eq!(body["messages"][0]["content"], "synthetic source");
            assert!(body["tools"].is_null());
            if config.anthropic {
                assert_eq!(body["max_tokens"], 4096);
            }
            if config.google {
                assert!(body["store"].is_null());
            }
            if config.openrouter {
                assert_eq!(body["provider"]["allow_fallbacks"], false);
                assert_eq!(body["provider"]["zdr"], true);
            }
        }
        for model in ["openrouter/auto", "@preset", "vendor/model:nitro", "auto"] {
            assert!(
                ApiConfig::new("https://openrouter.ai/api/v1/chat/completions", model).is_err()
            );
        }
        assert!(ApiConfig::new("https://evil.test/v1/messages", "m").is_err());
    }
    #[tokio::test]
    async fn anthropic_wire_contract_and_incomplete_responses() {
        for reason in ["end_turn", "max_tokens", "tool_use", "refusal"] {
            let reply = json!({"type":"message","role":"assistant","stop_reason":reason,
                "content":[{"type":"text","text":"synthetic correction"}]})
            .to_string();
            let (mut config, server, _) = fixture("200 OK", &reply, "", Duration::ZERO).await;
            config.anthropic = true;
            config.endpoint.set_path("/v1/messages");
            let result = complete(
                &config,
                "synthetic-key",
                "synthetic",
                true,
                &DeepCancellation::default(),
            )
            .await;
            if reason == "end_turn" {
                assert_eq!(result.unwrap(), "synthetic correction");
            } else {
                assert_eq!(result, Err(ApiError::IncompleteResponse));
            }
            let request = String::from_utf8(server.await.unwrap()).unwrap();
            assert!(request.starts_with("POST /v1/messages HTTP/1.1"));
            let headers = request.split_once("\r\n\r\n").unwrap().0.to_lowercase();
            assert!(headers.contains("x-api-key: synthetic-key"));
            assert!(headers.contains("anthropic-version: 2023-06-01"));
            assert!(!headers.contains("authorization:"));
        }
        let invalid = json!({"type":"message","role":"assistant","stop_reason":"end_turn",
            "content":[{"type":"tool_use","text":"never execute"}]})
        .to_string();
        assert_eq!(
            parse_anthropic_response(invalid.as_bytes()),
            Err(ApiError::InvalidResponse)
        );
    }
    #[test]
    fn destination_is_explicit_https_without_embedded_credentials() {
        assert!(ApiConfig::new("https://api.example.test/v1/chat/completions", "model").is_ok());
        for url in [
            "http://127.0.0.1/v1/chat/completions",
            "https://user:secret@example.test/chat/completions",
            "https://example.test/chat/completions?key=secret",
            "https://example.test/chat/completions#fragment",
            "https://example.test/responses",
        ] {
            assert!(matches!(
                ApiConfig::new(url, "m"),
                Err(ApiError::Configuration)
            ));
        }
    }
    #[tokio::test]
    async fn no_consent_or_cancel_sends_nothing() {
        let (config, server, _received) = fixture("200 OK", REPLY, "", Duration::ZERO).await;
        let cancel = DeepCancellation::default();
        assert_eq!(
            complete(&config, "dummy", "synthetic", false, &cancel).await,
            Err(ApiError::ConsentRequired)
        );
        cancel.cancel();
        assert_eq!(
            complete(&config, "dummy", "synthetic", true, &cancel).await,
            Err(ApiError::Cancelled)
        );
        assert!(!server.is_finished());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }
    #[tokio::test]
    async fn one_request_preserves_destination_and_content() {
        let (config, server, _received) = fixture("200 OK", REPLY, "", Duration::ZERO).await;
        assert_eq!(
            complete(
                &config,
                "synthetic-key",
                "synthetic source",
                true,
                &DeepCancellation::default()
            )
            .await
            .unwrap(),
            "synthetic correction"
        );
        let request = server.await.unwrap();
        let request = String::from_utf8(request).unwrap();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer synthetic-key"));
        let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], false);
        assert_eq!(body["model"], "fixture-model");
        assert_eq!(body["messages"][0]["content"], "synthetic source");
    }
    #[tokio::test]
    async fn redirects_and_rate_limits_are_not_replayed() {
        for status in ["302 Found", "429 Too Many Requests"] {
            let (config, server, _received) = fixture(
                status,
                "not logged",
                "Location: http://127.0.0.1:1/leak\r\n",
                Duration::ZERO,
            )
            .await;
            let result = complete(
                &config,
                "dummy",
                "synthetic",
                true,
                &DeepCancellation::default(),
            )
            .await;
            assert_eq!(
                result,
                Err(ApiError::HttpStatus(status[..3].parse().unwrap()))
            );
            server.await.unwrap();
        }
    }
    #[tokio::test]
    async fn response_limit_and_timeout() {
        let (config, server, _received) =
            fixture("200 OK", &"x".repeat(MAX_RESPONSE + 1), "", Duration::ZERO).await;
        assert_eq!(
            complete(
                &config,
                "dummy",
                "synthetic",
                true,
                &DeepCancellation::default()
            )
            .await,
            Err(ApiError::OutputLimit)
        );
        server.await.unwrap();
        let (config, server, _received) =
            fixture("200 OK", REPLY, "", Duration::from_millis(300)).await;
        assert_eq!(
            run(
                &config,
                "dummy",
                "synthetic",
                true,
                &DeepCancellation::default(),
                Duration::from_millis(80)
            )
            .await,
            Err(ApiError::Timeout)
        );
        server.await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_during_http_wait() {
        let (config, server, _received) =
            fixture("200 OK", REPLY, "", Duration::from_millis(300)).await;
        let cancel = DeepCancellation::default();
        let signal = cancel.clone();
        let pending =
            tokio::spawn(
                async move { complete(&config, "dummy", "synthetic", true, &signal).await },
            );
        tokio::time::timeout(Duration::from_secs(2), _received)
            .await
            .unwrap()
            .unwrap();
        cancel.cancel();
        assert_eq!(pending.await.unwrap(), Err(ApiError::Cancelled));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_key_and_escaped_body_limit_never_connect() {
        let (config, server, _received) = fixture("200 OK", REPLY, "", Duration::ZERO).await;
        for key in ["", "key\r\nInjected: header", "white space"] {
            assert_eq!(
                complete(
                    &config,
                    key,
                    "synthetic",
                    true,
                    &DeepCancellation::default()
                )
                .await,
                Err(ApiError::InvalidKey)
            );
        }
        assert_eq!(
            complete(
                &config,
                "dummy",
                &"\n".repeat(MAX_REQUEST / 2 + 1),
                true,
                &DeepCancellation::default()
            )
            .await,
            Err(ApiError::InputLimit)
        );
        assert!(!server.is_finished());
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }
    #[tokio::test]
    async fn chunked_body_cannot_bypass_response_limit() {
        let body = format!(
            "{:x}\r\n{}\r\n0\r\n\r\n",
            MAX_RESPONSE + 1,
            "x".repeat(MAX_RESPONSE + 1)
        );
        let (config, server, _received) = fixture(
            "200 OK",
            &body,
            "Transfer-Encoding: chunked\r\n",
            Duration::ZERO,
        )
        .await;
        assert_eq!(
            complete(
                &config,
                "dummy",
                "synthetic",
                true,
                &DeepCancellation::default()
            )
            .await,
            Err(ApiError::OutputLimit)
        );
        server.await.unwrap();
    }
    #[test]
    fn truncated_or_tool_results_are_never_suggestions() {
        assert_eq!(
            parse_response(REPLY.replace("stop", "length").as_bytes()),
            Err(ApiError::IncompleteResponse)
        );
        for body in [
            "{}",
            r#"{"choices":[]}"#,
            r#"{"choices":[{"finish_reason":"stop","message":{"content":"text","tool_calls":[]}}]}"#,
        ] {
            assert_eq!(
                parse_response(body.as_bytes()),
                Err(ApiError::InvalidResponse)
            );
        }
    }
}
