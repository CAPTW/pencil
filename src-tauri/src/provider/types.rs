use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    Codex,
    Antigravity,
    Claude,
}

impl ProviderKind {
    pub const ALL: [Self; 3] = [Self::Codex, Self::Antigravity, Self::Claude];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Antigravity => "antigravity",
            Self::Claude => "claude",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Antigravity => "Google Antigravity",
            Self::Claude => "Claude",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "codex" => Some(Self::Codex),
            "antigravity" => Some(Self::Antigravity),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderLifecycleState {
    Unavailable,
    SignedOut,
    Authenticating,
    Ready,
    Busy,
    Cancelling,
    Faulted,
    SignedOutPendingCleanup,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub writing: bool,
    pub official_sign_in: bool,
    pub official_sign_out: bool,
    pub cancellation: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    pub kind: ProviderKind,
    pub display_name: &'static str,
    pub state: ProviderLifecycleState,
    pub available: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub account_label: Option<String>,
    pub reason: Option<String>,
    pub setup_requirement: Option<String>,
    pub capabilities: ProviderCapabilities,
}

impl ProviderStatus {
    pub fn unavailable(kind: ProviderKind, reason: impl Into<String>, setup: impl Into<String>) -> Self {
        Self {
            kind,
            display_name: kind.display_name(),
            state: ProviderLifecycleState::Unavailable,
            available: false,
            executable_path: None,
            version: None,
            account_label: None,
            reason: Some(reason.into()),
            setup_requirement: Some(setup.into()),
            capabilities: ProviderCapabilities {
                writing: false,
                official_sign_in: false,
                official_sign_out: false,
                cancellation: false,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub active: ProviderKind,
    pub busy_kind: Option<ProviderKind>,
    pub statuses: Vec<ProviderStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderError {
    Unavailable(String),
    SignedOut(String),
    Busy,
    Cancelled,
    Faulted(String),
    AuthRequired,
    ContentLimit(String),
    MalformedOutput,
    NonzeroExit(i32),
    SilentFallbackRejected,
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => "provider_unavailable",
            Self::SignedOut(_) => "provider_signed_out",
            Self::Busy => "provider_busy",
            Self::Cancelled => "rewrite_interrupted",
            Self::Faulted(_) => "provider_faulted",
            Self::AuthRequired => "provider_auth_required",
            Self::ContentLimit(_) => "content_limit",
            Self::MalformedOutput => "provider_malformed_output",
            Self::NonzeroExit(_) => "provider_nonzero_exit",
            Self::SilentFallbackRejected => "provider_silent_fallback_rejected",
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) | Self::SignedOut(reason) | Self::Faulted(reason) => {
                write!(formatter, "{reason}")
            }
            Self::Busy => write!(formatter, "A Provider request is already active."),
            Self::Cancelled => write!(formatter, "The Provider request was cancelled."),
            Self::AuthRequired => write!(formatter, "Sign in to the selected Provider before sending a cloud request."),
            Self::ContentLimit(reason) => write!(formatter, "{reason}"),
            Self::MalformedOutput => write!(formatter, "The Provider returned output that could not be used."),
            Self::NonzeroExit(code) => write!(formatter, "The Provider process exited with status {code}."),
            Self::SilentFallbackRejected => {
                write!(formatter, "A failed Provider request cannot be replayed through another Provider.")
            }
        }
    }
}
