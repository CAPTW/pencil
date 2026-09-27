use super::{
    cli::{resolve_named_executable, run_args, run_writing, ActiveCliProcess},
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

const CLAUDE_NATIVE_SETUP: &str =
    "Install the native Claude Code (claude.exe) or set CODEX_PENCIL_CLAUDE_BIN to it, then Refresh status.";

pub(crate) fn resolve_claude() -> Option<PathBuf> {
    resolve_named_executable(
        std::env::var("CODEX_PENCIL_CLAUDE_BIN").ok().as_deref(),
        // The native claude.exe first: an npm claude.cmd cannot run requests.
        &["claude.exe", "claude.cmd", "claude"],
    )
}

pub(crate) async fn probe() -> ProviderStatus {
    probe_cancel(Arc::new(AtomicBool::new(false))).await
}

pub(crate) async fn probe_cancel(cancel: Arc<AtomicBool>) -> ProviderStatus {
    let Some(path) = resolve_claude() else {
        return ProviderStatus::unavailable(
            ProviderKind::Claude,
            "Claude Code CLI was not found on PATH.",
            "Install Claude Code and ensure `claude` is available, then Refresh status.",
        );
    };
    if let Some(reason) = super::cli::batch_launcher_reason(&path, "Claude Code") {
        return ProviderStatus::unavailable(ProviderKind::Claude, reason, CLAUDE_NATIVE_SETUP);
    }
    let version = super::cli::run_version_cancel(&path, cancel.clone())
        .await
        .ok();
    let auth = super::cli::run_args_cancel(
        &path,
        &["auth".to_string(), "status".to_string()],
        Duration::from_secs(12),
        cancel,
    )
    .await;
    match auth {
        Ok(captured) if captured.exit_code == Some(0) => {
            let logged_in = serde_json::from_str::<Value>(captured.stdout.trim())
                .ok()
                .and_then(|value| value.get("loggedIn").and_then(Value::as_bool))
                .unwrap_or(true);
            if logged_in {
                ProviderStatus {
                    kind: ProviderKind::Claude,
                    display_name: ProviderKind::Claude.display_name(),
                    state: ProviderLifecycleState::Ready,
                    available: true,
                    executable_path: Some(path.display().to_string()),
                    version,
                    account_label: Some("Claude account".to_string()),
                    reason: None,
                    setup_requirement: None,
                    capabilities: ProviderCapabilities {
                        writing: true,
                        official_sign_in: true,
                        official_sign_out: true,
                        cancellation: true,
                    },
                }
            } else {
                signed_out(path, version)
            }
        }
        Ok(_) => signed_out(path, version),
        Err(error) => ProviderStatus {
            kind: ProviderKind::Claude,
            display_name: ProviderKind::Claude.display_name(),
            state: ProviderLifecycleState::Faulted,
            available: true,
            executable_path: Some(path.display().to_string()),
            version,
            account_label: None,
            reason: Some(error),
            setup_requirement: Some(
                "Retry Refresh status after Claude Code is responding.".to_string(),
            ),
            capabilities: ProviderCapabilities {
                writing: true,
                official_sign_in: true,
                official_sign_out: true,
                cancellation: true,
            },
        },
    }
}

fn signed_out(path: PathBuf, version: Option<String>) -> ProviderStatus {
    ProviderStatus {
        kind: ProviderKind::Claude,
        display_name: ProviderKind::Claude.display_name(),
        state: ProviderLifecycleState::SignedOut,
        available: true,
        executable_path: Some(path.display().to_string()),
        version,
        account_label: None,
        reason: Some("Claude Code is installed but not signed in.".to_string()),
        setup_requirement: Some("Use Sign in to open the official Claude login.".to_string()),
        capabilities: ProviderCapabilities {
            writing: true,
            official_sign_in: true,
            official_sign_out: true,
            cancellation: true,
        },
    }
}

pub(crate) async fn start_login() -> Result<(), String> {
    let path =
        resolve_claude().ok_or_else(|| "Claude Code CLI was not found on PATH.".to_string())?;
    let _ = std::process::Command::new(&path)
        .args(["auth", "login"])
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start Claude login: {error}"))?;
    Ok(())
}

pub(crate) async fn sign_out() -> Result<(), String> {
    let path =
        resolve_claude().ok_or_else(|| "Claude Code CLI was not found on PATH.".to_string())?;
    let captured = run_args(
        &path,
        &["auth".to_string(), "logout".to_string()],
        Duration::from_secs(20),
    )
    .await?;
    if captured.exit_code.unwrap_or(1) == 0 {
        Ok(())
    } else {
        Err("Claude sign-out did not complete.".to_string())
    }
}

pub(crate) async fn rewrite(
    selected_text: &str,
    intent: RewriteIntent,
    terminology: &[TerminologyConstraint],
    cancel: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<ActiveCliProcess>>>,
) -> Result<RewriteResult, ProviderError> {
    let path = resolve_claude().ok_or_else(|| {
        ProviderError::Unavailable("Claude Code CLI was not found on PATH.".to_string())
    })?;
    if let Some(reason) = super::cli::batch_launcher_reason(&path, "Claude Code") {
        return Err(ProviderError::Unavailable(format!("{reason} {CLAUDE_NATIVE_SETUP}")));
    }
    let prompt = rewrite_prompt_with_terminology(selected_text, intent, terminology)
        .map_err(|error| ProviderError::Faulted(error))?;
    let args = vec![
        "-p".to_string(),
        prompt,
        "--output-format".to_string(),
        "json".to_string(),
        "--permission-mode".to_string(),
        "dontAsk".to_string(),
        "--no-session-persistence".to_string(),
        "--bare".to_string(),
        "--disallowedTools".to_string(),
        "*".to_string(),
        "--disable-slash-commands".to_string(),
    ];
    if super::cli::command_line_too_long(&path, &args) {
        return Err(ProviderError::InputTooLarge);
    }
    let captured = run_writing(&path, &args, Duration::from_secs(120), cancel.clone(), slot)
        .await
        .map_err(ProviderError::Faulted)?;
    if captured.cancelled || cancel.load(Ordering::SeqCst) {
        return Err(ProviderError::Cancelled);
    }
    classify_claude_output(captured.exit_code, &captured.stdout, &captured.stderr, intent.mode())
}

/// A successful run is judged only by its JSON result: the rewritten text may
/// itself mention logging in. Sign-in wording counts only in a failed run or
/// in an is_error result.
fn classify_claude_output(
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    mode: RewriteMode,
) -> Result<RewriteResult, ProviderError> {
    if exit_code != Some(0) {
        let combined = format!("{stdout} {stderr}").to_lowercase();
        if combined.contains("not logged in")
            || combined.contains("please run /login")
            || combined.contains("authentication")
        {
            return Err(ProviderError::SignedOut(
                "Claude Code is signed out.".to_string(),
            ));
        }
        if combined.contains("unknown option") {
            return Err(ProviderError::CliUsage);
        }
        return Err(ProviderError::NonzeroExit(exit_code.unwrap_or(1)));
    }
    parse_claude_result(stdout, mode)
}

fn parse_claude_result(stdout: &str, mode: RewriteMode) -> Result<RewriteResult, ProviderError> {
    let value: Value =
        serde_json::from_str(stdout.trim()).map_err(|_| ProviderError::MalformedOutput)?;
    if value.get("is_error").and_then(Value::as_bool) == Some(true) {
        let message = value
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("Claude request failed")
            .to_lowercase();
        if message.contains("not logged in") || message.contains("/login") {
            return Err(ProviderError::SignedOut(
                "Claude Code is signed out.".to_string(),
            ));
        }
        return Err(ProviderError::Faulted(
            value
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or("Claude request failed")
                .to_string(),
        ));
    }
    let text = value
        .get("result")
        .and_then(Value::as_str)
        .ok_or(ProviderError::MalformedOutput)?;
    parse_rewrite_payload(text, mode)
}

pub(crate) fn parse_rewrite_payload(
    text: &str,
    mode: RewriteMode,
) -> Result<RewriteResult, ProviderError> {
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
            .unwrap_or("Updated by Claude")
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
        provider_used: ProviderKind::Claude,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_result_without_leaking_provider_switch() {
        let result = parse_rewrite_payload(
            "{\"replacement\":\"Hello.\",\"changed\":true,\"summary\":\"fixed\",\"confidence\":0.8}",
            RewriteMode::Grammar,
        )
        .unwrap();
        assert_eq!(result.replacement, "Hello.");
        assert_eq!(result.provider_used, ProviderKind::Claude);
        assert_ne!(result.provider_used, ProviderKind::Codex);
    }

    #[test]
    fn rejects_empty_replacement() {
        assert!(parse_rewrite_payload(
            "{\"replacement\":\"\",\"changed\":false}",
            RewriteMode::Grammar
        )
        .is_err());
    }

    #[test]
    fn parses_fenced_json_payload() {
        let result = parse_rewrite_payload(
            "```json\n{\"replacement\":\"Hi.\",\"changed\":true,\"summary\":\"ok\",\"confidence\":0.7}\n```",
            RewriteMode::Grammar,
        )
        .unwrap();
        assert_eq!(result.replacement, "Hi.");
    }

    #[test]
    fn login_required_json_is_signed_out() {
        let stdout =
            r#"{"type":"result","is_error":true,"result":"Not logged in · Please run /login"}"#;
        assert!(matches!(
            parse_claude_result(stdout, RewriteMode::Grammar),
            Err(ProviderError::SignedOut(_))
        ));
        // The same answer with a failing exit code.
        assert!(matches!(
            classify_claude_output(Some(1), stdout, "", RewriteMode::Grammar),
            Err(ProviderError::SignedOut(_))
        ));
        assert!(matches!(
            classify_claude_output(Some(1), "", "Error: Not logged in", RewriteMode::Grammar),
            Err(ProviderError::SignedOut(_))
        ));
    }

    // Pre-use review: an npm-installed claude.cmd showed Ready but failed every
    // request at spawn (Rust refuses line breaks in batch-file arguments).
    #[test]
    fn batch_launchers_are_unavailable_and_native_executables_are_not() {
        use std::path::Path;
        for launcher in ["C:\\npm\\claude.cmd", "C:\\npm\\claude.CMD", "C:\\tools\\agy.bat"] {
            let reason = super::super::cli::batch_launcher_reason(Path::new(launcher), "CLI").unwrap();
            assert!(reason.contains("multi-line request"), "{reason}");
        }
        for native in ["C:\\bin\\claude.exe", "C:\\bin\\claude"] {
            assert!(super::super::cli::batch_launcher_reason(Path::new(native), "CLI").is_none());
        }
    }

    // Pre-use review: a successful rewrite of text that mentions logging in
    // used to be reported as signed out.
    #[test]
    fn successful_result_mentioning_login_is_not_signed_out() {
        let payload = r#"{\"replacement\":\"Please run /login when you are not logged in.\",\"changed\":true,\"summary\":\"ok\",\"confidence\":0.9}"#;
        let stdout = format!(r#"{{"type":"result","is_error":false,"result":"{payload}"}}"#);
        let result = classify_claude_output(Some(0), &stdout, "", RewriteMode::Grammar).unwrap();
        assert_eq!(result.replacement, "Please run /login when you are not logged in.");
        assert!(matches!(
            classify_claude_output(Some(2), "", "error: unknown option '--bare'", RewriteMode::Grammar),
            Err(ProviderError::CliUsage)
        ));
        assert!(matches!(
            classify_claude_output(Some(3), "", "", RewriteMode::Grammar),
            Err(ProviderError::NonzeroExit(3))
        ));
    }
}
