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
    operation: Option<Arc<ProviderOperation>>,
    next_operation: u64,
    statuses: Vec<ProviderStatus>,
    last_self_tests: Vec<SelfTestRecord>,
}

pub(crate) struct ProviderOperation {
    pub(crate) id: u64,
    pub(crate) kind: ProviderKind,
    pub(crate) binding: String,
    pub(crate) capture_binding: Option<(
        crate::capture_session::SessionToken,
        crate::capture_session::BoundRewriteIntent,
    )>,
    cancel: Arc<AtomicBool>,
    cli_slot: Arc<Mutex<Option<ActiveCliProcess>>>,
    completed: AtomicBool,
}

impl ProviderOperation {
    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub(crate) fn is_complete(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self {
            active: ProviderKind::Codex,
            busy: None,
            operation: None,
            next_operation: 0,
            statuses: ProviderKind::ALL
                .iter()
                .map(|kind| {
                    ProviderStatus::unavailable(*kind, "Status not probed yet.", "Refresh status.")
                })
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

    pub(crate) async fn refresh(manager: &Mutex<Self>, codex: &CodexClientCache) {
        let operation = {
            let mut guard = manager.lock().await;
            let kind = guard.active;
            match guard.reserve(kind, "probe".into(), true) {
                Ok(op) => op,
                Err(_) => return,
            }
        };
        let statuses = vec![
            probe_codex(codex, operation.cancel.clone()).await,
            antigravity::probe_cancel(operation.cancel.clone()).await,
            claude::probe_cancel(operation.cancel.clone()).await,
        ];
        let mut guard = manager.lock().await;
        if guard.finish(&operation) && !operation.cancel.load(Ordering::SeqCst) {
            guard.statuses = statuses;
        }
    }

    pub(crate) fn cancel_handle(&self) -> Option<Arc<ProviderOperation>> {
        self.operation.clone()
    }

    pub(crate) fn reserve(
        &mut self,
        kind: ProviderKind,
        binding: String,
        self_test: bool,
    ) -> Result<Arc<ProviderOperation>, ProviderError> {
        self.reserve_bound(
            kind,
            binding,
            self_test,
            None,
            Arc::new(AtomicBool::new(false)),
        )
    }
    pub(crate) fn reserve_capture(
        &mut self,
        kind: ProviderKind,
        token: crate::capture_session::SessionToken,
        intent: crate::capture_session::BoundRewriteIntent,
    ) -> Result<Arc<ProviderOperation>, ProviderError> {
        self.reserve_bound(
            kind,
            format!("{}:{}", token.session_id, token.generation),
            false,
            Some((token, intent)),
            Arc::new(AtomicBool::new(false)),
        )
    }

    pub(crate) fn reserve_with_cancel(
        &mut self,
        kind: ProviderKind,
        binding: String,
        cancel: Arc<AtomicBool>,
    ) -> Result<Arc<ProviderOperation>, ProviderError> {
        self.reserve_bound(kind, binding, false, None, cancel)
    }

    fn reserve_bound(
        &mut self,
        kind: ProviderKind,
        binding: String,
        self_test: bool,
        capture_binding: Option<(
            crate::capture_session::SessionToken,
            crate::capture_session::BoundRewriteIntent,
        )>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Arc<ProviderOperation>, ProviderError> {
        if !self_test && kind != self.active {
            return Err(ProviderError::SilentFallbackRejected);
        }
        if self.busy.is_some() {
            return Err(ProviderError::Busy);
        }
        self.next_operation = self
            .next_operation
            .checked_add(1)
            .ok_or_else(|| ProviderError::Faulted("provider_operation_id_exhausted".into()))?;
        let operation = Arc::new(ProviderOperation {
            id: self.next_operation,
            kind,
            binding,
            capture_binding,
            cancel,
            cli_slot: Arc::new(Mutex::new(None)),
            completed: AtomicBool::new(false),
        });
        self.busy = Some(kind);
        self.operation = Some(operation.clone());
        Ok(operation)
    }

    pub(crate) fn finish(&mut self, operation: &ProviderOperation) -> bool {
        let current = self.operation.as_ref().is_some_and(|op| {
            op.id == operation.id && op.binding == operation.binding && op.kind == operation.kind
        });
        if current {
            self.busy = None;
            self.operation = None;
            for status in &mut self.statuses {
                if matches!(
                    status.state,
                    ProviderLifecycleState::Busy | ProviderLifecycleState::Cancelling
                ) && status.available
                    && status.reason.is_none()
                {
                    status.state = ProviderLifecycleState::Ready;
                }
            }
        }
        operation.completed.store(true, Ordering::SeqCst);
        current
    }

    pub(crate) async fn execute(
        operation: &ProviderOperation,
        selected_text: &str,
        intent: RewriteIntent,
        terminology: &[TerminologyConstraint],
        codex: &CodexClientCache,
    ) -> Result<RewriteResult, ProviderError> {
        super::executor::execute(
            operation.kind,
            operation.cancel.clone(),
            operation.cli_slot.clone(),
            selected_text,
            intent,
            terminology,
            codex,
        )
        .await
    }

    pub(crate) async fn rewrite(
        manager: &Mutex<Self>,
        kind: ProviderKind,
        binding: String,
        selected_text: &str,
        intent: RewriteIntent,
        terminology: &[TerminologyConstraint],
        codex: &CodexClientCache,
    ) -> Result<RewriteResult, ProviderError> {
        let operation = manager.lock().await.reserve(kind, binding, false)?;
        let result = Self::execute(&operation, selected_text, intent, terminology, codex).await;
        if !manager.lock().await.finish(&operation) {
            return Err(ProviderError::Cancelled);
        }
        result
    }

    pub(crate) async fn run_self_test(
        manager: &Mutex<Self>,
        kind: ProviderKind,
        intent: RewriteIntent,
        codex: &CodexClientCache,
    ) -> SelfTestRecord {
        let started = std::time::Instant::now();
        let started_utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".into());
        let reservation = manager.lock().await.reserve(kind, "self-test".into(), true);
        let operation_id = reservation.as_ref().ok().map(|op| op.id);
        let result = match reservation {
            Ok(operation) => {
                let result = Self::execute(&operation, SELF_TEST_SOURCE, intent, &[], codex).await;
                if manager.lock().await.finish(&operation) {
                    result
                } else {
                    Err(ProviderError::Cancelled)
                }
            }
            Err(error) => Err(error),
        };
        let record = SelfTestRecord {
            kind,
            started_utc,
            duration_ms: started.elapsed().as_millis() as u64,
            classification: match &result {
                Ok(value) if value.provider_used == kind => SelfTestClassification::Success,
                Ok(_) => SelfTestClassification::ProductFailure,
                Err(ProviderError::SignedOut(_) | ProviderError::AuthRequired) => {
                    SelfTestClassification::SignedOut
                }
                Err(ProviderError::Unavailable(_)) => SelfTestClassification::Unavailable,
                Err(ProviderError::Cancelled) => SelfTestClassification::Cancelled,
                Err(
                    ProviderError::Faulted(_)
                    | ProviderError::NonzeroExit(_)
                    | ProviderError::EmptyResponse
                    | ProviderError::RateLimited
                    | ProviderError::TimedOut
                    | ProviderError::NetworkFailure
                    | ProviderError::CliUsage
                    | ProviderError::ExternalService
                    | ProviderError::InputTooLarge,
                ) => SelfTestClassification::ExternalFailure,
                Err(_) => SelfTestClassification::ProductFailure,
            },
            error_code: result.err().map(|error| error.code().to_string()),
        };
        let mut guard = manager.lock().await;
        if operation_id != Some(guard.next_operation) {
            return record;
        }
        guard
            .last_self_tests
            .retain(|existing| existing.kind != kind);
        guard.last_self_tests.push(record.clone());
        record
    }
}

async fn probe_codex(codex: &CodexClientCache, cancel: Arc<AtomicBool>) -> ProviderStatus {
    match crate::codex_binary::resolve_codex_executable() {
        None => ProviderStatus::unavailable(
            ProviderKind::Codex,
            "Codex CLI was not found on PATH.",
            "Install Codex CLI and ensure `codex` is available, then Refresh status.",
        ),
        Some(path) => {
            let version = super::cli::run_version_cancel(&path, cancel.clone())
                .await
                .ok();
            let auth = match codex.current_healthy().await {
                Some(client) => {
                    tokio::select! {
                        biased;
                        _ = super::cli::cancellation(&cancel) => { let _ = codex.shutdown_checked().await; None },
                        result = client.auth_status() => result.ok(),
                    }
                }
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
        let record = runtime.block_on(ProviderManager::run_self_test(
            &Mutex::new(manager),
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
        let error = runtime.block_on(ProviderManager::rewrite(
            &Mutex::new(manager),
            ProviderKind::Claude,
            "unit-test".into(),
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
        let manager = Mutex::new(manager);
        let record = ProviderManager::run_self_test(
            &manager,
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
        assert!(manager.lock().await.snapshot().busy_kind.is_none());
    }
}
