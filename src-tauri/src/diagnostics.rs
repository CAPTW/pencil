use crate::provider::{ProviderKind, ProviderSnapshot};
use crate::settings::{AppSettings, ONBOARDING_VERSION, SETTINGS_SCHEMA_VERSION};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const DIAGNOSTIC_SCHEMA_VERSION: u32 = 1;
pub const SELF_TEST_SOURCE: &str = "This are a synthetic provider connection test.";
const MAX_SANITIZED_LEN: usize = 240;

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SelfTestClassification {
    Success,
    SignedOut,
    Unavailable,
    Cancelled,
    ExternalFailure,
    ProductFailure,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDiagnostic {
    pub kind: ProviderKind,
    pub lifecycle_state: String,
    pub official_client_available: bool,
    pub client_basename: Option<String>,
    pub client_version: Option<String>,
    pub authentication_classification: String,
    pub last_self_test_classification: Option<SelfTestClassification>,
    pub last_typed_error_code: Option<String>,
    pub last_request_duration_ms: Option<u64>,
    pub owned_residual_process_count: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSnapshotV1 {
    pub snapshot_schema_version: u32,
    pub generated_utc: String,
    pub application_version: String,
    pub build_commit: String,
    pub settings_schema_version: u32,
    pub windows_version: String,
    pub architecture: String,
    pub startup_mode: String,
    pub shortcut_registration_status: String,
    pub current_active_provider: ProviderKind,
    pub providers: Vec<ProviderDiagnostic>,
    pub cloud_disclosure_version_acknowledged: u32,
    pub instant_engine_available: bool,
    pub current_app_process_id: u32,
    pub feature_flags: Vec<String>,
    pub diagnostic_id: String,
}

pub fn sanitize_text(input: &str) -> String {
    let mut value = input.replace('\\', "/");
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let normalized = profile.replace('\\', "/");
        if !normalized.is_empty() {
            value = value.replace(&normalized, "<USER_PROFILE>");
            value = value.replace(&profile, "<USER_PROFILE>");
        }
    }
    let lower = value.to_lowercase();
    if lower.contains("bearer ")
        || lower.contains("api_key")
        || lower.contains("api-key")
        || value.contains("sk-")
        || lower.contains("cookie:")
        || value.contains("eyJ")
    {
        return "<REDACTED>".to_string();
    }
    if value.lines().count() > 2 {
        return "<REDACTED_MULTILINE>".to_string();
    }
    if value.chars().count() > MAX_SANITIZED_LEN {
        let truncated: String = value.chars().take(MAX_SANITIZED_LEN).collect();
        return format!("{truncated}…");
    }
    value
}

pub fn basename_only(path: Option<&str>) -> Option<String> {
    path.and_then(|value| {
        std::path::Path::new(value)
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
    })
}

pub fn build_snapshot(
    settings: &AppSettings,
    snapshot: &ProviderSnapshot,
    self_tests: &[crate::provider::SelfTestRecord],
    shortcut_status: &str,
) -> DiagnosticSnapshotV1 {
    let generated = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    DiagnosticSnapshotV1 {
        snapshot_schema_version: DIAGNOSTIC_SCHEMA_VERSION,
        generated_utc: format!("{generated}"),
        application_version: env!("CARGO_PKG_VERSION").to_string(),
        build_commit: option_env!("GRAMMAR_BUILD_COMMIT")
            .unwrap_or("unspecified")
            .to_string(),
        settings_schema_version: SETTINGS_SCHEMA_VERSION,
        windows_version: sanitize_text(&std::env::var("OS").unwrap_or_else(|_| "windows".to_string())),
        architecture: std::env::consts::ARCH.to_string(),
        startup_mode: if settings.start_hidden_to_tray {
            "tray".to_string()
        } else {
            "window".to_string()
        },
        shortcut_registration_status: shortcut_status.to_string(),
        current_active_provider: settings.active_provider,
        providers: snapshot
            .statuses
            .iter()
            .map(|status| {
                let last = self_tests.iter().find(|record| record.kind == status.kind);
                ProviderDiagnostic {
                    kind: status.kind,
                    lifecycle_state: status.state.as_str().to_string(),
                    official_client_available: status.available,
                    client_basename: basename_only(status.executable_path.as_deref()),
                    client_version: status.version.clone(),
                    authentication_classification: status.state.as_str().to_string(),
                    last_self_test_classification: last.map(|record| record.classification.clone()),
                    last_typed_error_code: last.and_then(|record| record.error_code.clone()),
                    last_request_duration_ms: last.map(|record| record.duration_ms),
                    owned_residual_process_count: 0,
                }
            })
            .collect(),
        cloud_disclosure_version_acknowledged: settings.cloud_processing_acknowledgement_version,
        instant_engine_available: true,
        current_app_process_id: std::process::id(),
        feature_flags: vec![
            format!("onboarding_version={ONBOARDING_VERSION}"),
            "p3b_not_implemented".to_string(),
        ],
        diagnostic_id: Uuid::new_v4().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_profile_paths_tokens_and_multiline() {
        let profile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:/Users/test".to_string());
        assert!(sanitize_text(&format!("{profile}/secret.txt")).contains("<USER_PROFILE>"));
        assert_eq!(sanitize_text("Authorization: Bearer abc.def"), "<REDACTED>");
        assert_eq!(sanitize_text("sk-examplekey"), "<REDACTED>");
        assert_eq!(sanitize_text("cookie: session=1"), "<REDACTED>");
        assert_eq!(sanitize_text("line1\nline2\nline3"), "<REDACTED_MULTILINE>");
        assert!(!sanitize_text("provider_busy").contains("This are"));
    }

    #[test]
    fn snapshot_omits_source_results_and_env() {
        let settings = crate::settings::AppSettings::default();
        let snapshot = crate::provider::ProviderSnapshot {
            active: ProviderKind::Codex,
            busy_kind: None,
            statuses: Vec::new(),
            last_self_tests: Vec::new(),
        };
        let json = serde_json::to_string(&build_snapshot(
            &settings,
            &snapshot,
            &[],
            "registered",
        ))
        .unwrap();
        assert!(!json.contains(SELF_TEST_SOURCE));
        assert!(!json.contains("replacement"));
        assert!(!json.contains("USERPROFILE"));
        assert!(!json.contains("selectedText"));
    }
}
