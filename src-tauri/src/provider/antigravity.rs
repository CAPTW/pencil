use super::{
    cli::{resolve_named_executable, run_writing, ActiveCliProcess},
    types::{
        ProviderCapabilities, ProviderError, ProviderKind, ProviderLifecycleState, ProviderStatus,
    },
};
use crate::{
    codex_client::{rewrite_prompt_with_terminology, RewriteEdit, RewriteResult},
    settings::RewriteMode,
    terminology_matcher::TerminologyConstraint,
    translation::RewriteIntent,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;

pub(crate) fn resolve_agy() -> Option<PathBuf> {
    resolve_named_executable(
        std::env::var("CODEX_PENCIL_AGY_BIN").ok().as_deref(),
        &["agy.exe", "agy"],
    )
}

pub(crate) async fn probe() -> ProviderStatus {
    probe_cancel(Arc::new(AtomicBool::new(false))).await
}

pub(crate) async fn probe_cancel(cancel: Arc<AtomicBool>) -> ProviderStatus {
    let Some(path) = resolve_agy() else {
        return ProviderStatus::unavailable(
            ProviderKind::Antigravity,
            "Antigravity CLI (`agy`) was not found on PATH.",
            "Install Antigravity CLI from https://antigravity.google/docs/cli/install/ then Refresh status.",
        );
    };
    if let Some(reason) = super::cli::batch_launcher_reason(&path, "Antigravity CLI") {
        return ProviderStatus::unavailable(
            ProviderKind::Antigravity,
            reason,
            "Use the native agy.exe or set CODEX_PENCIL_AGY_BIN to it, then Refresh status.",
        );
    }
    let version = super::cli::run_version_cancel(&path, cancel.clone())
        .await
        .ok();
    let capabilities = ProviderCapabilities {
        writing: true,
        official_sign_in: false,
        official_sign_out: false,
        cancellation: true,
    };
    ProviderStatus {
        kind: ProviderKind::Antigravity,
        display_name: ProviderKind::Antigravity.display_name(),
        state: ProviderLifecycleState::Ready,
        available: true,
        executable_path: Some(path.display().to_string()),
        version,
        account_label: Some("Google Antigravity CLI".to_string()),
        reason: Some("Sign-in uses the official `agy` keyring session. Complete it once in a terminal if a request reports authentication required.".to_string()),
        setup_requirement: None,
        capabilities,
    }
}

pub(crate) async fn rewrite(
    selected_text: &str,
    intent: RewriteIntent,
    terminology: &[TerminologyConstraint],
    cancel: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<ActiveCliProcess>>>,
) -> Result<RewriteResult, ProviderError> {
    let path = resolve_agy().ok_or_else(|| {
        ProviderError::Unavailable("Antigravity CLI (`agy`) was not found on PATH.".to_string())
    })?;
    if let Some(reason) = super::cli::batch_launcher_reason(&path, "Antigravity CLI") {
        return Err(ProviderError::Unavailable(reason));
    }
    let prompt = rewrite_prompt_with_terminology(selected_text, intent, terminology)
        .map_err(ProviderError::Faulted)?;
    let args = vec![
        "-p".to_string(),
        prompt,
        "--output-format".to_string(),
        "json".to_string(),
        "--disable-slash-commands".to_string(),
        "--print-timeout".to_string(),
        "2m".to_string(),
    ];
    if super::cli::command_line_too_long(&path, &args) {
        return Err(ProviderError::InputTooLarge);
    }
    let captured = run_writing(&path, &args, Duration::from_secs(130), cancel.clone(), slot)
        .await
        .map_err(ProviderError::Faulted)?;
    if captured.cancelled || cancel.load(Ordering::SeqCst) {
        return Err(ProviderError::Cancelled);
    }
    finish_agy_capture(&captured.stdout, captured.exit_code, intent.mode())
}

fn finish_agy_capture(
    stdout: &str,
    exit_code: Option<i32>,
    mode: RewriteMode,
) -> Result<RewriteResult, ProviderError> {
    let parsed = serde_json::from_str::<Value>(stdout.trim()).ok();
    if exit_code == Some(0) {
        let Some(value) = parsed else {
            return Err(ProviderError::MalformedOutput);
        };
        return finish_agy_value(value, mode);
    }
    Err(classify_agy_failure(
        parsed.as_ref(),
        exit_code,
        stdout.trim().is_empty(),
    ))
}

fn classify_agy_failure(
    parsed: Option<&Value>,
    exit_code: Option<i32>,
    stdout_empty: bool,
) -> ProviderError {
    let error = parsed
        .and_then(|value| value.get("error").and_then(Value::as_str))
        .unwrap_or("");
    let lower = error.to_lowercase();
    if lower.contains("authentication")
        || lower.contains("not already authenticated")
        || lower.contains("sign in")
        || lower.contains("signed out")
    {
        return ProviderError::SignedOut(
            "Google Antigravity CLI is not signed in. Run `agy` once in a terminal to complete official Google sign-in.".to_string(),
        );
    }
    if lower.contains("unknown option") || lower.contains("unknown flag") {
        return ProviderError::CliUsage;
    }
    if lower.contains("rate") || lower.contains("quota") || lower.contains("resource exhausted") {
        return ProviderError::RateLimited;
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return ProviderError::TimedOut;
    }
    if lower.contains("network") || lower.contains("connection") || lower.contains("dns") {
        return ProviderError::NetworkFailure;
    }
    if parsed.is_some() {
        return ProviderError::ExternalService;
    }
    if stdout_empty {
        return ProviderError::EmptyResponse;
    }
    ProviderError::NonzeroExit(exit_code.unwrap_or(1))
}

fn parse_agy_result(stdout: &str, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    finish_agy_capture(stdout, Some(0), mode)
}

fn finish_agy_value(value: Value, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    if value.get("status").and_then(Value::as_str) != Some("SUCCESS") {
        return Err(classify_agy_failure(Some(&value), Some(0), false));
    }
    let text = value
        .get("response")
        .and_then(Value::as_str)
        .ok_or(ProviderError::MalformedOutput)?;
    if text.trim().is_empty() {
        return Err(ProviderError::EmptyResponse);
    }
    parse_payload(text, mode)
}

fn parse_payload(text: &str, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    let extracted = crate::writing_contract::extract_json_object(text)
        .map_err(|_| ProviderError::MalformedOutput)?;
    let replacement = crate::writing_contract::replacement_from_payload(&extracted)
        .map_err(|_| ProviderError::MalformedOutput)?;
    let value = extracted.value;
    Ok(RewriteResult {
        replacement,
        changed: value
            .get("changed")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        summary: value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Updated by Google Antigravity")
            .to_string(),
        edits: value
            .get("edits")
            .and_then(Value::as_array)
            .map(|edits| {
                edits
                    .iter()
                    .filter_map(|edit| {
                        Some(RewriteEdit {
                            before: edit.get("before")?.as_str()?.to_string(),
                            after: edit.get("after")?.as_str()?.to_string(),
                            reason: edit.get("reason")?.as_str()?.to_string(),
                        })
                    })
                    .take(8)
                    .collect()
            })
            .unwrap_or_default(),
        confidence: value
            .get("confidence")
            .and_then(Value::as_f64)
            .unwrap_or(0.5),
        mode,
        used_terminology_ids: Vec::new(),
        terminology_suggestions: Vec::new(),
        terminology_match_count: 0,
        terminology_warnings: Vec::new(),
        provider_used: ProviderKind::Antigravity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_success_envelope() {
        let stdout = r#"{"conversation_id":"x","status":"SUCCESS","response":"{\"replacement\":\"Hi.\",\"changed\":true,\"summary\":\"ok\",\"confidence\":0.7}"}"#;
        let result = parse_agy_result(stdout, RewriteMode::Grammar).unwrap();
        assert_eq!(result.replacement, "Hi.");
        assert_eq!(result.provider_used, ProviderKind::Antigravity);
    }

    #[test]
    fn authentication_error_is_signed_out() {
        let stdout = r#"{"status":"ERROR","response":"","error":"authentication required"}"#;
        assert!(matches!(
            parse_agy_result(stdout, RewriteMode::Grammar),
            Err(ProviderError::SignedOut(_))
        ));
    }

    #[test]
    fn structured_nonzero_error_is_external_and_does_not_leak_raw_text() {
        let stdout = r#"{"status":"ERROR","response":"","error":"synthetic-official-detail-should-not-leak"}"#;
        let error = finish_agy_capture(stdout, Some(1), RewriteMode::Grammar).unwrap_err();
        assert!(matches!(error, ProviderError::ExternalService));
        assert_eq!(error.code(), "provider_external_service");
        assert!(!error
            .to_string()
            .contains("synthetic-official-detail-should-not-leak"));
    }

    #[test]
    fn empty_nonzero_stdout_is_empty_response() {
        let error = finish_agy_capture("", Some(1), RewriteMode::Grammar).unwrap_err();
        assert!(matches!(error, ProviderError::EmptyResponse));
    }

    #[test]
    fn dash_dash_source_stays_inside_canonical_prompt() {
        let prompt = rewrite_prompt_with_terminology(
            "--inject option",
            crate::translation::RewriteIntent::grammar(),
            &[],
        )
        .unwrap();
        assert!(prompt.contains("--inject option"));
        assert!(prompt.contains("untrusted data"));
    }

    #[test]
    fn command_line_preflight_rejects_oversized_argv() {
        let path = PathBuf::from("agy.exe");
        let huge = "x".repeat(40_000);
        assert!(super::super::cli::command_line_too_long(&path, &[huge]));
    }

    #[tokio::test]
    #[ignore]
    async fn live_adapter_synthetic_grammar() {
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let slot = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let result = rewrite(
            "These vessel is ready for departure.",
            crate::translation::RewriteIntent::grammar(),
            &[],
            cancel,
            slot,
        )
        .await;
        match result {
            Ok(value) => {
                assert_eq!(value.provider_used, ProviderKind::Antigravity);
                assert!(!value.replacement.is_empty());
            }
            Err(error) => {
                assert_ne!(error.code(), "provider_silent_fallback_rejected");
                assert!(!error.to_string().contains("These vessel"));
            }
        }
    }
}
