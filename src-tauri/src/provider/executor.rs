//! Shared production dispatch and consent-bound headless facade. No startup probes.
use super::{
    antigravity, claude,
    cli::ActiveCliProcess,
    manager::ProviderManager,
    types::{ProviderError, ProviderKind},
};
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

/// Caller-owned cancellation remains effective while admission, execution and
/// teardown are awaited. Never replace this with aborting the rewrite future.
#[derive(Clone, Default)]
pub struct DeepCancellation {
    flag: Arc<AtomicBool>,
}

impl DeepCancellation {
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

pub struct DeepRequest {
    pub provider: ProviderKind,
    pub document_epoch: String,
    pub document_revision: u64,
    pub source: String,
    pub explicit_consent: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct DeepResult {
    pub provider: ProviderKind,
    pub document_epoch: String,
    pub document_revision: u64,
    pub source_sha256: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeepError {
    code: &'static str,
}
impl DeepError {
    pub fn code(&self) -> &'static str {
        self.code
    }
    fn provider(error: ProviderError) -> Self {
        Self { code: error.code() }
    }
}
impl std::fmt::Display for DeepError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code)
    }
}
impl std::error::Error for DeepError {}

/// One headless operation at a time. All process implementations and admission
/// semantics are shared with the desktop; no private copied runtime or probes.
#[derive(Default)]
pub struct DeepRuntime {
    manager: Mutex<ProviderManager>,
    codex: CodexClientCache,
    cleanup_blocked: AtomicBool,
}

struct AdmissionGuard<'a> {
    runtime: &'a DeepRuntime,
    cancellation: DeepCancellation,
    completed: bool,
}
impl Drop for AdmissionGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.cancellation.cancel();
            // Dropping a future cannot prove process/workspace teardown. Never
            // allow a replacement operation to accumulate unowned work.
            self.runtime.cleanup_blocked.store(true, Ordering::SeqCst);
        }
    }
}

impl DeepRuntime {
    pub async fn rewrite(
        &self,
        request: DeepRequest,
        cancellation: DeepCancellation,
    ) -> Result<DeepResult, DeepError> {
        if !request.explicit_consent {
            return Err(DeepError {
                code: "deep_consent_required",
            });
        }
        if request.document_epoch.is_empty()
            || request.document_epoch.len() > 128
            || request.document_revision == 0
            || request.source.is_empty()
            || request.source.encode_utf16().count() > 8192
        {
            return Err(DeepError {
                code: "deep_invalid_request",
            });
        }
        if cancellation.is_cancelled() {
            return Err(DeepError {
                code: "rewrite_interrupted",
            });
        }
        // The choice remains available without fallback, but this adapter cannot
        // yet prove Antigravity suppresses persistent document history.
        if request.provider == ProviderKind::Antigravity {
            return Err(DeepError {
                code: "provider_privacy_unqualified",
            });
        }
        let intent = RewriteIntent::new_with_auto_reference(
            crate::settings::RewriteMode::Grammar,
            None,
            None,
        )
        .map_err(|_| DeepError {
            code: "deep_invalid_request",
        })?;
        let source_sha256 = crate::instant_selection::sha256_hex(request.source.as_bytes());
        let operation = {
            let mut manager = self.manager.lock().await;
            if self.cleanup_blocked.load(Ordering::SeqCst) || super::cli::cleanup_status().is_err()
            {
                return Err(DeepError {
                    code: "deep_cleanup_unresolved",
                });
            }
            manager
                .set_active(request.provider)
                .map_err(DeepError::provider)?;
            manager
                .reserve_with_cancel(
                    request.provider,
                    format!(
                        "{}:{}:{}",
                        request.document_epoch, request.document_revision, source_sha256
                    ),
                    cancellation.flag.clone(),
                )
                .map_err(DeepError::provider)?
        };
        let mut guard = AdmissionGuard {
            runtime: self,
            cancellation: cancellation.clone(),
            completed: false,
        };
        let result =
            ProviderManager::execute(&operation, &request.source, intent, &[], &self.codex).await;
        // Headless ports do not retain successful Codex servers between requests.
        // Always check teardown on success/error/cancel before releasing admission.
        let cleanup = self.codex.shutdown_checked().await;
        let cleanup_failed = cleanup.is_err() || super::cli::cleanup_status().is_err();
        if cleanup_failed {
            self.cleanup_blocked.store(true, Ordering::SeqCst);
        }
        let current = self.manager.lock().await.finish(&operation);
        guard.completed = true;
        if cleanup_failed {
            return Err(DeepError {
                code: "deep_cleanup_unresolved",
            });
        }
        if !current || cancellation.is_cancelled() {
            return Err(DeepError {
                code: "rewrite_interrupted",
            });
        }
        let result = result.map_err(DeepError::provider)?;
        if result.provider_used != request.provider {
            return Err(DeepError {
                code: "provider_silent_fallback_rejected",
            });
        }
        if result.replacement.encode_utf16().count() > 8192 {
            return Err(DeepError {
                code: "deep_output_limit",
            });
        }
        Ok(DeepResult {
            provider: request.provider,
            document_epoch: request.document_epoch,
            document_revision: request.document_revision,
            source_sha256,
            replacement: result.replacement,
        })
    }

    /// The owner first cancels and joins any pending rewrite. Busy is explicitly
    /// not shutdown success; there is no detached cleanup task.
    pub async fn shutdown(&self) -> Result<(), DeepError> {
        let manager = self.manager.lock().await;
        if manager.cancel_handle().is_some() {
            return Err(DeepError { code: "deep_busy" });
        }
        if self.codex.shutdown_checked().await.is_err() {
            self.cleanup_blocked.store(true, Ordering::SeqCst);
        }
        if self.cleanup_blocked.load(Ordering::SeqCst) || super::cli::cleanup_status().is_err() {
            return Err(DeepError {
                code: "deep_cleanup_unresolved",
            });
        }
        Ok(())
    }
}

pub(crate) async fn execute(
    kind: ProviderKind,
    cancel: Arc<AtomicBool>,
    cli_slot: Arc<Mutex<Option<ActiveCliProcess>>>,
    selected_text: &str,
    intent: RewriteIntent,
    terminology: &[TerminologyConstraint],
    codex: &CodexClientCache,
) -> Result<RewriteResult, ProviderError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(ProviderError::Cancelled);
    }
    let result = match kind {
        ProviderKind::Codex => {
            rewrite_codex(codex, selected_text, intent, terminology, cancel.clone()).await
        }
        ProviderKind::Claude => {
            claude::rewrite(
                selected_text,
                intent,
                terminology,
                cancel.clone(),
                cli_slot.clone(),
            )
            .await
        }
        ProviderKind::Antigravity => {
            antigravity::rewrite(
                selected_text,
                intent,
                terminology,
                cancel.clone(),
                cli_slot.clone(),
            )
            .await
        }
    };
    if cancel.load(Ordering::SeqCst) && result.is_ok() {
        return Err(ProviderError::Cancelled);
    }
    result.map(|mut value| {
        value.provider_used = kind;
        value
    })
}

async fn rewrite_codex(
    cache: &CodexClientCache,
    selected_text: &str,
    intent: RewriteIntent,
    terminology: &[TerminologyConstraint],
    cancel: Arc<AtomicBool>,
) -> Result<RewriteResult, ProviderError> {
    let client = cache.get_cancel(cancel.clone()).await.map_err(|error| {
        if cancel.load(Ordering::SeqCst) && !error.contains("cleanup") {
            return ProviderError::Cancelled;
        }
        if error.to_lowercase().contains("auth") || error.to_lowercase().contains("login") {
            ProviderError::SignedOut(error)
        } else {
            ProviderError::Faulted(error)
        }
    })?;
    let result = tokio::select! {
        biased;
        _ = super::cli::cancellation(&cancel) => Err("rewrite_interrupted".to_string()),
        result = client.rewrite_with_terminology(selected_text, intent, terminology) => result,
    };
    if cancel.load(Ordering::SeqCst) {
        cache
            .shutdown_checked()
            .await
            .map_err(ProviderError::Faulted)?;
        return Err(ProviderError::Cancelled);
    }
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            // A failed turn/start or response wait may still have dispatched
            // work. Reap its exact cached server before releasing admission.
            cache
                .shutdown_checked()
                .await
                .map_err(|cleanup| ProviderError::Faulted(format!("{error};cleanup={cleanup}")))?;
            Err(if error == "rewrite_interrupted" {
                ProviderError::Cancelled
            } else {
                ProviderError::Faulted(error)
            })
        }
    }
}

#[cfg(test)]
mod headless_tests {
    use super::*;

    fn request(provider: ProviderKind) -> DeepRequest {
        DeepRequest {
            provider,
            document_epoch: "synthetic-document".into(),
            document_revision: 1,
            source: "synthetic seperate".into(),
            explicit_consent: true,
        }
    }

    #[test]
    fn public_handles_are_send_sync_and_cancel_is_shared() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DeepRuntime>();
        assert_send_sync::<DeepCancellation>();
        let cancel = DeepCancellation::default();
        let cloned = cancel.clone();
        cloned.cancel();
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn consent_and_bounds_reject_before_admission_or_process_work() {
        let runtime = DeepRuntime::default();
        for kind in ProviderKind::ALL {
            let mut denied = request(kind);
            denied.explicit_consent = false;
            assert_eq!(
                runtime
                    .rewrite(denied, DeepCancellation::default())
                    .await
                    .unwrap_err()
                    .code(),
                "deep_consent_required"
            );
            let mut oversized = request(kind);
            oversized.source = "😀".repeat(4097);
            assert_eq!(
                runtime
                    .rewrite(oversized, DeepCancellation::default())
                    .await
                    .unwrap_err()
                    .code(),
                "deep_invalid_request"
            );
            let mut unbound = request(kind);
            unbound.document_revision = 0;
            assert_eq!(
                runtime
                    .rewrite(unbound, DeepCancellation::default())
                    .await
                    .unwrap_err()
                    .code(),
                "deep_invalid_request"
            );
        }
        assert!(runtime.manager.lock().await.cancel_handle().is_none());
        assert!(runtime.codex.current_healthy().await.is_none());
    }

    #[tokio::test]
    async fn already_cancelled_requests_never_resolve_or_probe_a_provider() {
        let runtime = DeepRuntime::default();
        let cancel = DeepCancellation::default();
        cancel.cancel();
        for kind in ProviderKind::ALL {
            assert_eq!(
                runtime
                    .rewrite(request(kind), cancel.clone())
                    .await
                    .unwrap_err()
                    .code(),
                "rewrite_interrupted"
            );
            // Exercise the actual extracted desktop dispatch guard as well.
            let result = execute(
                kind,
                cancel.flag.clone(),
                Arc::new(Mutex::new(None)),
                "synthetic",
                RewriteIntent::grammar(),
                &[],
                &runtime.codex,
            )
            .await;
            assert!(matches!(result, Err(ProviderError::Cancelled)));
        }
        assert!(runtime.manager.lock().await.cancel_handle().is_none());
    }

    #[tokio::test]
    async fn headless_antigravity_privacy_is_unavailable_without_fallback() {
        let runtime = DeepRuntime::default();
        assert_eq!(
            runtime
                .rewrite(
                    request(ProviderKind::Antigravity),
                    DeepCancellation::default()
                )
                .await
                .unwrap_err()
                .code(),
            "provider_privacy_unqualified"
        );
        assert!(runtime.manager.lock().await.cancel_handle().is_none());
        assert!(runtime.codex.current_healthy().await.is_none());
    }

    #[tokio::test]
    async fn abandoned_owner_latches_admission_and_shutdown_unavailable() {
        let runtime = DeepRuntime::default();
        let cancel = DeepCancellation::default();
        drop(AdmissionGuard {
            runtime: &runtime,
            cancellation: cancel.clone(),
            completed: false,
        });
        assert!(cancel.is_cancelled());
        assert_eq!(
            runtime
                .rewrite(request(ProviderKind::Claude), DeepCancellation::default())
                .await
                .unwrap_err()
                .code(),
            "deep_cleanup_unresolved"
        );
        assert_eq!(
            runtime.shutdown().await.unwrap_err().code(),
            "deep_cleanup_unresolved"
        );
        assert!(runtime.manager.lock().await.cancel_handle().is_none());
    }
}
