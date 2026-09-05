use super::{
    antigravity, claude,
    cli::ActiveCliProcess,
    types::{
        ProviderError, ProviderKind, ProviderLifecycleState, ProviderSnapshot, ProviderStatus,
        SelfTestRecord,
    },
};
use crate::diagnostics::{SelfTestClassification, SELF_TEST_SOURCE};
use crate::{
    codex_client::{CodexClientCache, RewriteResult},
    terminology_matcher::TerminologyConstraint,
    translation::RewriteIntent,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;

pub(crate) struct ProviderManager {
    active: ProviderKind,
    busy: Option<ProviderKind>,
    cancel: Arc<AtomicBool>,
    cli_slot: Arc<Mutex<Option<ActiveCliProcess>>>,
    statuses: Vec<ProviderStatus>,
    last_self_tests: Vec<SelfTestRecord>,
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self {
            active: ProviderKind::Codex,
            busy: None,
            cancel: Arc::new(AtomicBool::new(false)),
            cli_slot: Arc::new(Mutex::new(None)),
            statuses: ProviderKind::ALL
                .iter()
                .map(|kind| ProviderStatus::unavailable(*kind, "Status not probed yet.", "Refresh status."))
                .collect(),
            last_self_tests: Vec::new(),
        }
    }
}

impl ProviderManager {
    pub(crate) fn set_active(&mut self, kind: ProviderKind) -> Result<(), ProviderError> {
        if self.busy.is_some() {
            return Err(ProviderError::Busy);
        }
        self.active = kind;
        Ok(())
    }

    pub(crate) fn active(&self) -> ProviderKind {
        self.active
    }

    pub(crate) fn snapshot(&self) -> ProviderSnapshot {
        ProviderSnapshot {
            active: self.active,
            busy_kind: self.busy,
            statuses: self.statuses.clone(),
            last_self_tests: self.last_self_tests.clone(),
        }
    }

    pub(crate) fn last_self_tests(&self) -> &[SelfTestRecord] {
        &self.last_self_tests
    }

    pub(crate) async fn refresh(&mut self, codex: &CodexClientCache) {
        let mut statuses = Vec::new();
        statuses.push(probe_codex(codex).await);
        statuses.push(antigravity::probe().await);
        statuses.push(claude::probe().await);
        if let Some(busy) = self.busy {
            for status in &mut statuses {
                if status.kind == busy {
                    status.state = if self.cancel.load(Ordering::SeqCst) {
                        ProviderLifecycleState::Cancelling
                    } else {
                        ProviderLifecycleState::Busy
                    };
                }
            }
        }
        self.statuses = statuses;
    }

    pub(crate) async fn cancel_active(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(process) = self.cli_slot.lock().await.as_ref() {
            process.request_cancel();
        }
        if self.busy.is_some() {
            for status in &mut self.statuses {
                if Some(status.kind) == self.busy {
                    status.state = ProviderLifecycleState::Cancelling;
                }
            }
        }
    }

    pub(crate) fn clear_busy(&mut self) {
        self.busy = None;
        self.cancel.store(false, Ordering::SeqCst);
        for status in &mut self.statuses {
            if matches!(
                status.state,
                ProviderLifecycleState::Busy | ProviderLifecycleState::Cancelling
            ) {
                if status.available && status.reason.is_none() {
                    status.state = ProviderLifecycleState::Ready;
                }
            }
        }
    }

    pub(crate) async fn rewrite(
        &mut self,
        kind: ProviderKind,
        selected_text: &str,
        intent: RewriteIntent,
        terminology: &[TerminologyConstraint],
        codex: &CodexClientCache,
    ) -> Result<RewriteResult, ProviderError> {
        if kind != self.active {
            return Err(ProviderError::SilentFallbackRejected);
        }
        if self.busy.is_some() {
            return Err(ProviderError::Busy);
        }
        self.busy = Some(kind);
        self.cancel.store(false, Ordering::SeqCst);
        let result = match kind {
            ProviderKind::Codex => rewrite_codex(codex, selected_text, intent, terminology).await,
            ProviderKind::Claude => {
                claude::rewrite(
                    selected_text,
                    intent,
                    terminology,
                    self.cancel.clone(),
                    self.cli_slot.clone(),
                )
                .await
            }
            ProviderKind::Antigravity => {
                antigravity::rewrite(
                    selected_text,
                    intent,
                    terminology,
                    self.cancel.clone(),
                    self.cli_slot.clone(),
                )
                .await
            }
        };
        self.clear_busy();
        match result {
            Ok(mut value) => {
                value.provider_used = kind;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn run_self_test(
        &mut self,
        kind: ProviderKind,
        intent: RewriteIntent,
        codex: &CodexClientCache,
    ) -> SelfTestRecord {
        let started = std::time::Instant::now();
        let started_utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        if self.busy.is_some() {
            return SelfTestRecord {
                kind,
                classification: SelfTestClassification::ProductFailure,
                started_utc,
                duration_ms: started.elapsed().as_millis() as u64,
                error_code: Some("provider_busy".to_string()),
            };
        }
        self.busy = Some(kind);
        self.cancel.store(false, Ordering::SeqCst);
        let result = match kind {
            ProviderKind::Codex => rewrite_codex(codex, SELF_TEST_SOURCE, intent, &[]).await,
            ProviderKind::Claude => {
                claude::rewrite(
                    SELF_TEST_SOURCE,
                    intent,
                    &[],
                    self.cancel.clone(),
                    self.cli_slot.clone(),
                )
                .await
            }
            ProviderKind::Antigravity => {
                antigravity::rewrite(
                    SELF_TEST_SOURCE,
                    intent,
                    &[],
                    self.cancel.clone(),
                    self.cli_slot.clone(),
                )
                .await
            }
        };
        self.clear_busy();
        let record = SelfTestRecord {
            kind,
            classification: match &result {
                Ok(value) if value.provider_used == kind => SelfTestClassification::Success,
                Ok(_) => SelfTestClassification::ProductFailure,
                Err(ProviderError::SignedOut(_) | ProviderError::AuthRequired) => {
                    SelfTestClassification::SignedOut
                }
                Err(ProviderError::Unavailable(_)) => SelfTestClassification::Unavailable,
                Err(ProviderError::Cancelled) => SelfTestClassification::Cancelled,
                Err(ProviderError::Faulted(_) | ProviderError::NonzeroExit(_)) => {
                    SelfTestClassification::ExternalFailure
                }
                Err(_) => SelfTestClassification::ProductFailure,
            },
            started_utc,
            duration_ms: started.elapsed().as_millis() as u64,
            error_code: result.err().map(|error| error.code().to_string()),
        };
        self.last_self_tests.retain(|existing| existing.kind != kind);
        self.last_self_tests.push(record.clone());
        record
    }
}

async fn probe_codex(codex: &CodexClientCache) -> ProviderStatus {
    match crate::codex_binary::resolve_codex_executable() {
        None => ProviderStatus::unavailable(
            ProviderKind::Codex,
            "Codex CLI was not found on PATH.",
            "Install Codex CLI and ensure `codex` is available, then Refresh status.",
        ),
        Some(path) => {
            let version = crate::codex_binary::read_codex_version(&path).ok();
            let auth = match codex.current_healthy().await {
                Some(client) => client.auth_status().await.ok(),
                None => None,
            };
            if let Some(auth) = auth {
                if auth.logged_in {
                    return ProviderStatus {
                        kind: ProviderKind::Codex,
                        display_name: ProviderKind::Codex.display_name(),
                        state: ProviderLifecycleState::Ready,
                        available: true,
                        executable_path: Some(path.display().to_string()),
                        version,
                        account_label: auth.account_label,
                        reason: None,
                        setup_requirement: None,
                        capabilities: super::types::ProviderCapabilities {
                            writing: true,
                            official_sign_in: true,
                            official_sign_out: false,
                            cancellation: true,
                        },
                    };
                }
                return ProviderStatus {
                    kind: ProviderKind::Codex,
                    display_name: ProviderKind::Codex.display_name(),
                    state: ProviderLifecycleState::SignedOut,
                    available: true,
                    executable_path: Some(path.display().to_string()),
                    version,
                    account_label: None,
                    reason: Some("Codex is installed but not signed in.".to_string()),
                    setup_requirement: Some("Use the official ChatGPT device login.".to_string()),
                    capabilities: super::types::ProviderCapabilities {
                        writing: true,
                        official_sign_in: true,
                        official_sign_out: false,
                        cancellation: true,
                    },
                };
            }
            ProviderStatus {
                kind: ProviderKind::Codex,
                display_name: ProviderKind::Codex.display_name(),
                state: ProviderLifecycleState::SignedOut,
                available: true,
                executable_path: Some(path.display().to_string()),
                version,
                account_label: None,
                reason: Some("Codex authentication has not been confirmed.".to_string()),
                setup_requirement: Some("Use the official ChatGPT device login.".to_string()),
                capabilities: super::types::ProviderCapabilities {
                    writing: true,
                    official_sign_in: true,
                    official_sign_out: false,
                    cancellation: true,
                },
            }
        }
    }
}

async fn rewrite_codex(
    cache: &CodexClientCache,
    selected_text: &str,
    intent: RewriteIntent,
    terminology: &[TerminologyConstraint],
) -> Result<RewriteResult, ProviderError> {
    let client = cache.get().await.map_err(|error| {
        if error.to_lowercase().contains("auth") || error.to_lowercase().contains("login") {
            ProviderError::SignedOut(error)
        } else {
            ProviderError::Faulted(error)
        }
    })?;
    client
        .rewrite_with_terminology(selected_text, intent, terminology)
        .await
        .map_err(|error| {
            if error == "rewrite_interrupted" {
                ProviderError::Cancelled
            } else {
                ProviderError::Faulted(error)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_is_blocked_while_busy() {
        let mut manager = ProviderManager::default();
        manager.busy = Some(ProviderKind::Codex);
        assert!(matches!(
            manager.set_active(ProviderKind::Claude),
            Err(ProviderError::Busy)
        ));
    }

    #[test]
    fn self_test_busy_does_not_start_another_provider() {
        let mut manager = ProviderManager::default();
        manager.busy = Some(ProviderKind::Codex);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cache = CodexClientCache::default();
        let record = runtime.block_on(manager.run_self_test(
            ProviderKind::Claude,
            RewriteIntent::grammar(),
            &cache,
        ));
        assert_eq!(record.kind, ProviderKind::Claude);
        assert_eq!(
            record.classification,
            crate::diagnostics::SelfTestClassification::ProductFailure
        );
        assert_eq!(record.error_code.as_deref(), Some("provider_busy"));
    }

    #[test]
    fn other_provider_cannot_be_used_as_fallback() {
        let mut manager = ProviderManager::default();
        manager.active = ProviderKind::Codex;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cache = CodexClientCache::default();
        let error = runtime.block_on(manager.rewrite(
            ProviderKind::Claude,
            "hi",
            RewriteIntent::grammar(),
            &[],
            &cache,
        ));
        assert!(matches!(error, Err(ProviderError::SilentFallbackRejected)));
    }

    #[tokio::test]
    #[ignore]
    async fn live_self_test_antigravity_is_not_apply_ready() {
        let mut manager = ProviderManager::default();
        let cache = CodexClientCache::default();
        let record = manager
            .run_self_test(
                ProviderKind::Antigravity,
                RewriteIntent::grammar(),
                &cache,
            )
            .await;
        assert_eq!(record.kind, ProviderKind::Antigravity);
        assert_eq!(
            record.classification,
            crate::diagnostics::SelfTestClassification::Success
        );
        assert!(manager.snapshot().busy_kind.is_none());
    }
}
