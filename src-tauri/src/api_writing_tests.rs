//! Synthetic HTTP through the production desktop rewrite implementation, without GUI/native input.
use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
const RESULT: &str = r#"{"replacement":"Corrected text.","changed":true,"summary":"Correction","edits":[],"confidence":0.9,"usedTerminologyIds":[],"terminologySuggestions":[]}"#;
async fn fixture(
    content: &str,
) -> (
    Arc<AppState>,
    SessionToken,
    String,
    oneshot::Receiver<()>,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<String>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let state = Arc::new(AppState::default());
    state
        .configuration
        .lock()
        .unwrap()
        .settings
        .terminology
        .enabled = false;
    let token = state
        .capture
        .lock()
        .await
        .capture(
            "synthetic-document".into(),
            "Incorrect text.".into(),
            capture_session::WindowTarget::new(123, 456),
        )
        .unwrap();
    let epoch = state
        .api_session
        .lock()
        .unwrap()
        .configure_fixture(&endpoint);
    let (sent, received) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let body =
        serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":content}}]})
            .to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut head = Vec::new();
        let mut b = [0];
        while !head.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut b).await.unwrap();
            head.push(b[0]);
            assert!(head.len() < 8192);
        }
        let headers = String::from_utf8(head).unwrap().to_lowercase();
        let n = headers
            .lines()
            .find_map(|l| l.strip_prefix("content-length: "))
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let mut request = vec![0; n];
        socket.read_exact(&mut request).await.unwrap();
        let _ = sent.send(());
        let _ = wait.await;
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(reply.as_bytes()).await;
        String::from_utf8(request).unwrap()
    });
    (state, token, epoch, received, release, server)
}
async fn run(
    state: &AppState,
    token: &SessionToken,
    epoch: &str,
    consent: bool,
) -> Result<RewriteResult, String> {
    rewrite_selected_text_impl(
        token.session_id.clone(),
        token.generation,
        RewriteMode::Grammar,
        None,
        None,
        Some(epoch.into()),
        Some(consent),
        state,
    )
    .await
}
#[tokio::test]
async fn api_uses_captured_source_canonical_prompt_and_ready_state() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    let result = result.unwrap();
    assert_eq!(result.provider_used, provider::types::ResultProvider::Api);
    assert!(state.capture.lock().await.is_ready(&token));
    assert!(!state.api_session.lock().unwrap().is_busy());
    let body: serde_json::Value = serde_json::from_str(&server.await.unwrap()).unwrap();
    let prompt = body["messages"][0]["content"].as_str().unwrap();
    assert_eq!(
        prompt,
        writing_contract::build_canonical_prompt("Incorrect text.", RewriteIntent::grammar(), &[])
            .unwrap()
    );
    assert_eq!(state.providers.lock().await.snapshot().busy_kind, None);
}
#[tokio::test]
async fn no_consent_stale_epoch_and_stale_capture_send_nothing() {
    let (state, token, epoch, _received, release, server) = fixture(RESULT).await;
    assert_eq!(
        run(&state, &token, &epoch, false).await.unwrap_err(),
        "api_consent_required"
    );
    assert!(state.capture.lock().await.is_captured(&token));
    assert_eq!(
        run(&state, &token, "old-epoch", true).await.unwrap_err(),
        "api_setup_stale"
    );
    let wrong = SessionToken {
        generation: token.generation + 1,
        ..token.clone()
    };
    assert!(run(&state, &wrong, &epoch, true).await.is_err());
    assert!(!server.is_finished());
    drop(release);
    server.abort();
    let _ = server.await;
}
#[tokio::test]
async fn revoked_permission_rejects_pending_result_and_keeps_capture() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        state.api_session.lock().unwrap().clear();
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert!(result.is_err());
    assert!(state.capture.lock().await.is_captured(&token));
    server.await.unwrap();
    assert!(!state.api_session.lock().unwrap().is_busy());
}
#[tokio::test]
async fn newer_capture_is_never_replaced_by_old_response() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        let next = state
            .capture
            .lock()
            .await
            .capture(
                "next".into(),
                "New source.".into(),
                capture_session::WindowTarget::new(123, 456),
            )
            .unwrap();
        release.send(()).unwrap();
        next
    };
    let (result, next) = tokio::join!(running, control);
    assert_eq!(result.unwrap_err(), "rewrite_superseded");
    assert_eq!(
        state.capture.lock().await.captured_source(&next).unwrap(),
        "New source."
    );
    server.await.unwrap();
}
#[tokio::test]
async fn invalid_structured_result_restores_instant_capture() {
    let (state, token, epoch, received, release, server) = fixture("raw sensitive echo").await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert_eq!(result.unwrap_err(), "api_result_invalid");
    assert!(state.capture.lock().await.is_captured(&token));
    server.await.unwrap();
}
#[tokio::test]
async fn cancel_retains_capture_and_releases_api_slot() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        state.capture.lock().await.cancel_rewrite_keep_capture();
        cancel_cli_operation(&state).await;
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert_eq!(result.unwrap_err(), "rewrite_cancelled");
    assert!(state.capture.lock().await.is_captured(&token));
    assert!(!state.api_session.lock().unwrap().is_busy());
    server.await.unwrap();
}
#[tokio::test]
async fn changed_intent_cannot_publish_ready_result() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        state.configuration.lock().unwrap().settings.mode = RewriteMode::Natural;
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert_eq!(result.unwrap_err(), "stale_rewrite_intent");
    assert!(state.capture.lock().await.is_captured(&token));
    server.await.unwrap();
}
#[tokio::test]
async fn only_matching_approved_terms_reach_api() {
    use terminology::{EntryMatchMode, EntryType, LanguageScope};
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let root = std::env::temp_dir().join(format!("grammar-api-terms-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let mut runtime = TerminologyRuntime::open(root.join("terms.json"), 1);
    for (id, source, status) in [
        ("approved-match", "Incorrect", EntryStatus::Approved),
        ("unmatched", "unrelated", EntryStatus::Approved),
        ("unapproved", "text", EntryStatus::Suggested),
    ] {
        runtime
            .add_entry(
                id.into(),
                TerminologyEntryDraft {
                    profile_id: terminology::GENERAL_PROFILE_ID.into(),
                    entry_type: EntryType::Preferred,
                    status,
                    source_text: source.into(),
                    preferred_text: Some("Corrected".into()),
                    source_language: LanguageScope::Any,
                    target_language: LanguageScope::Any,
                    aliases: vec![],
                    match_mode: EntryMatchMode::WholePhrase,
                    case_sensitive: false,
                    priority: 100,
                    usage_count: 0,
                    occurrence_count: 0,
                    note: None,
                },
                2,
            )
            .unwrap();
    }
    *state.terminology.lock().unwrap() = runtime;
    state
        .configuration
        .lock()
        .unwrap()
        .settings
        .terminology
        .enabled = true;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert_eq!(result.unwrap().terminology_match_count, 1);
    let request = server.await.unwrap();
    assert!(request.contains("approved-match"));
    assert!(!request.contains("unmatched"));
    assert!(!request.contains("unapproved"));
    *state.terminology.lock().unwrap() = TerminologyRuntime::default();
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn delayed_cancel_does_not_cancel_current_valid_api_binding() {
    let (state, token, epoch, received, release, server) = fixture(RESULT).await;
    let running = run(&state, &token, &epoch, true);
    let control = async {
        received.await.unwrap();
        cancel_cli_operation(&state).await;
        release.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(running, control);
    assert_eq!(
        result.unwrap().provider_used,
        provider::types::ResultProvider::Api
    );
    assert!(!state.api_session.lock().unwrap().is_busy());
    server.await.unwrap();
}
