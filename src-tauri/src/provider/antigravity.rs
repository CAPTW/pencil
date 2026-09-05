use super::{
    cli::{resolve_named_executable, run_version, run_writing, ActiveCliProcess},
    types::{ProviderCapabilities, ProviderError, ProviderKind, ProviderLifecycleState, ProviderStatus},
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
    let Some(path) = resolve_agy() else {
        return ProviderStatus::unavailable(
            ProviderKind::Antigravity,
            "Antigravity CLI (`agy`) was not found on PATH.",
            "Install Antigravity CLI from https://antigravity.google/docs/cli/install/ then Refresh status.",
        );
    };
    let version = run_version(&path).await.ok();
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
    let captured = run_writing(&path, &args, Duration::from_secs(130), cancel.clone(), slot)
        .await
        .map_err(ProviderError::Faulted)?;
    if captured.cancelled || cancel.load(Ordering::SeqCst) {
        return Err(ProviderError::Cancelled);
    }
    if captured.exit_code != Some(0) {
        let combined = format!("{} {}", captured.stdout, captured.stderr).to_lowercase();
        if combined.contains("authentication required") || combined.contains("not already authenticated") {
            return Err(ProviderError::SignedOut(
                "Google Antigravity CLI is not signed in. Run `agy` once in a terminal to complete official Google sign-in.".to_string(),
            ));
        }
        if let Ok(value) = serde_json::from_str::<Value>(captured.stdout.trim()) {
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                if error.to_lowercase().contains("authentication") {
                    return Err(ProviderError::SignedOut(error.to_string()));
                }
            }
        }
        return Err(ProviderError::NonzeroExit(captured.exit_code.unwrap_or(1)));
    }
    parse_agy_result(&captured.stdout, intent.mode())
}

fn parse_agy_result(stdout: &str, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    let value: Value = serde_json::from_str(stdout.trim()).map_err(|_| ProviderError::MalformedOutput)?;
    if value.get("status").and_then(Value::as_str) != Some("SUCCESS") {
        let error = value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Antigravity request failed");
        if error.to_lowercase().contains("authentication") {
            return Err(ProviderError::SignedOut(error.to_string()));
        }
        return Err(ProviderError::Faulted(error.to_string()));
    }
    let text = value
        .get("response")
        .and_then(Value::as_str)
        .ok_or(ProviderError::MalformedOutput)?;
    parse_payload(text, mode)
}

fn parse_payload(text: &str, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    let trimmed = text.trim();
    let json_slice = if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[start..=end]
    } else {
        return Err(ProviderError::MalformedOutput);
    };
    let value: Value = serde_json::from_str(json_slice).map_err(|_| ProviderError::MalformedOutput)?;
    let replacement = value
        .get("replacement")
        .and_then(Value::as_str)
        .ok_or(ProviderError::MalformedOutput)?
        .to_string();
    if replacement.is_empty() {
        return Err(ProviderError::MalformedOutput);
    }
    Ok(RewriteResult {
        replacement,
        changed: value.get("changed").and_then(Value::as_bool).unwrap_or(true),
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
        confidence: value.get("confidence").and_then(Value::as_f64).unwrap_or(0.5),
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
}
