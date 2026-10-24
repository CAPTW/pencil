//! API runtime with optional profile-scoped Windows secure persistence.
//! Request tickets govern setup changes; capture binding remains the caller's job.
use super::{
    executor::DeepCancellation,
    http_api::{validate_key, ApiConfig},
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Setup {
    pub expected_epoch: String,
    pub endpoint: String,
    pub model: String,
    pub key: String,
    pub consent: bool,
}
impl Drop for Setup {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.key.zeroize();
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub epoch: String,
    pub configured: bool,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub consent: bool,
    pub request_enabled: bool,
    pub storage_error: Option<&'static str>,
}
struct Prepared {
    config: ApiConfig,
    endpoint: String,
    model: String,
    key: Zeroizing<String>,
    consent: bool,
    epoch: Uuid,
}
pub(crate) struct ApiSession {
    store: Option<Box<dyn super::api_credentials::CredentialStore>>,
    storage_error: Option<&'static str>,
    epoch: Uuid,
    setup: Option<Arc<Prepared>>,
    active: Option<Arc<RequestTicket>>,
    binding: Option<(
        crate::capture_session::SessionToken,
        crate::capture_session::BoundRewriteIntent,
        u64,
    )>,
}
impl Default for ApiSession {
    fn default() -> Self {
        Self {
            store: None,
            storage_error: None,
            epoch: Uuid::new_v4(),
            setup: None,
            active: None,
            binding: None,
        }
    }
}
// Neither this object nor Prepared derives Debug/Serialize: no key in status/errors.
pub(crate) struct RequestTicket {
    started: AtomicBool,
    setup: Arc<Prepared>,
    id: Uuid,
    pub cancel: DeepCancellation,
}
impl ApiSession {
    pub fn attach_store(&mut self, store: Box<dyn super::api_credentials::CredentialStore>) {
        self.clear();
        self.store = None;
        let restored = (|| {
            let Some(bytes) = store.read()? else {
                return Ok(None);
            };
            let mut saved: Setup =
                serde_json::from_slice(&bytes).map_err(|_| "api_secure_storage_invalid")?;
            saved.expected_epoch = self.epoch.to_string();
            self.configure(saved)?;
            Ok::<_, &'static str>(Some(()))
        })();
        self.storage_error = restored.err();
        self.store = Some(store);
    }
    pub fn saved_key(&self, endpoint: &str) -> Result<Zeroizing<String>, &'static str> {
        let saved = self
            .setup
            .as_ref()
            .filter(|s| s.endpoint == endpoint)
            .ok_or("api_invalid_key")?;
        Ok(Zeroizing::new(saved.key.to_string()))
    }
    pub fn forget(&mut self) -> Result<Status, &'static str> {
        self.clear();
        let result = self.store.as_ref().map_or(Ok(()), |store| store.delete());
        self.storage_error = result.err();
        result.map(|_| self.status())
    }
    pub fn revoke(&mut self) -> Result<Status, &'static str> {
        let Some(saved) = &self.setup else {
            return self.forget();
        };
        self.configure(Setup {
            expected_epoch: self.epoch.to_string(),
            endpoint: saved.endpoint.clone(),
            model: saved.model.clone(),
            key: saved.key.to_string(),
            consent: false,
        })
    }
    pub fn ensure_store(&self) -> Result<(), &'static str> {
        if self.store.is_some() {
            Ok(())
        } else {
            Err("api_secure_storage_unavailable")
        }
    }
    pub fn is_busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn status(&self) -> Status {
        Status {
            storage_error: self.storage_error,
            epoch: self.epoch.to_string(),
            configured: self.setup.is_some(),
            endpoint: self.setup.as_ref().map(|s| s.endpoint.clone()),
            model: self.setup.as_ref().map(|s| s.model.clone()),
            consent: self.setup.as_ref().is_some_and(|s| s.consent),
            request_enabled: self.setup.as_ref().is_some_and(|s| s.consent),
        }
    }
    pub fn clear(&mut self) -> Status {
        if let Some(active) = &self.active {
            active.cancel.cancel();
        }
        self.setup = None;
        self.epoch = Uuid::new_v4();
        self.status()
    }
    pub fn configure(&mut self, mut setup: Setup) -> Result<Status, &'static str> {
        if setup.expected_epoch != self.epoch.to_string() {
            return Err("api_setup_stale");
        }
        if setup.key.is_empty() {
            match self.saved_key(&setup.endpoint) {
                Ok(key) => setup.key = key.to_string(),
                Err(error) => {
                    self.forget()?;
                    return Err(error);
                }
            }
        }
        // Any current replacement attempt revokes the old key/consent, including invalid edits.
        self.forget()?;
        let config = ApiConfig::new(&setup.endpoint, &setup.model)
            .map_err(|_| "api_invalid_configuration")?;
        validate_key(&setup.key).map_err(|_| "api_invalid_key")?;
        if let Some(store) = &self.store {
            let bytes = Zeroizing::new(
                serde_json::to_vec(&setup).map_err(|_| "api_secure_storage_invalid")?,
            );
            if let Err(error) = store.write(&bytes) {
                self.storage_error = Some(error);
                return Err(error);
            }
        }
        self.setup = Some(Arc::new(Prepared {
            config,
            endpoint: setup.endpoint.clone(),
            model: setup.model.clone(),
            key: Zeroizing::new(setup.key.clone()),
            consent: setup.consent,
            epoch: self.epoch,
        }));
        Ok(self.status())
    }
    pub fn begin(&mut self, explicit_consent: bool) -> Result<Arc<RequestTicket>, &'static str> {
        if self.active.is_some() {
            return Err("api_busy");
        }
        let setup = self.setup.as_ref().ok_or("api_not_configured")?;
        if !setup.consent || !explicit_consent {
            return Err("api_consent_required");
        }
        let ticket = Arc::new(RequestTicket {
            started: AtomicBool::new(false),
            setup: setup.clone(),
            id: Uuid::new_v4(),
            cancel: DeepCancellation::default(),
        });
        self.active = Some(ticket.clone());
        Ok(ticket)
    }
    #[cfg(test)]
    pub(crate) fn configure_fixture(&mut self, endpoint: &str) -> String {
        self.clear();
        self.setup = Some(Arc::new(Prepared {
            config: ApiConfig::fixture(endpoint),
            endpoint: endpoint.into(),
            model: "fixture".into(),
            key: Zeroizing::new("dummy-key".into()),
            consent: true,
            epoch: self.epoch,
        }));
        self.epoch.to_string()
    }
    pub fn begin_bound(
        &mut self,
        epoch: &str,
        consent: bool,
        token: crate::capture_session::SessionToken,
        intent: crate::capture_session::BoundRewriteIntent,
        attempt: u64,
    ) -> Result<Arc<RequestTicket>, &'static str> {
        if epoch != self.epoch.to_string() {
            return Err("api_setup_stale");
        }
        let ticket = self.begin(consent)?;
        self.binding = Some((token, intent, attempt));
        Ok(ticket)
    }
    pub fn cancel_stale(&self, capture: &crate::capture_session::CaptureSessionStore) {
        if let Some(active) = &self.active {
            let current = self
                .binding
                .as_ref()
                .is_some_and(|(token, intent, attempt)| {
                    capture.owns_rewrite(token, *attempt)
                        && capture
                            .validate_rewriting_bound_intent(token, intent)
                            .is_ok()
                });
            if !current {
                active.cancel.cancel();
            }
        }
    }
    pub fn finish(&mut self, ticket: &RequestTicket) -> bool {
        let current = self.active.as_ref().is_some_and(|a| a.id == ticket.id)
            && self
                .setup
                .as_ref()
                .is_some_and(|s| s.epoch == ticket.setup.epoch);
        if self.active.as_ref().is_some_and(|a| a.id == ticket.id) {
            self.active = None;
            self.binding = None;
        }
        current && !ticket.cancel.is_cancelled()
    }
}
impl Drop for ApiSession {
    fn drop(&mut self) {
        self.clear();
    }
}
impl RequestTicket {
    pub async fn complete(&self, prompt: &str) -> Result<String, super::http_api::ApiError> {
        if self.started.swap(true, Ordering::SeqCst) {
            return Err(super::http_api::ApiError::RequestAlreadyUsed);
        }
        super::http_api::complete(
            &self.setup.config,
            &self.setup.key,
            prompt,
            true,
            &self.cancel,
        )
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup(s: &ApiSession) -> Setup {
        Setup {
            expected_epoch: s.epoch.to_string(),
            endpoint: "https://api.example.test/v1/chat/completions".into(),
            model: "synthetic-model".into(),
            key: "dummy-secret".into(),
            consent: true,
        }
    }
    #[derive(Clone, Default)]
    struct MemoryStore {
        bytes: Arc<std::sync::Mutex<Option<Vec<u8>>>>,
        fail: Arc<std::sync::atomic::AtomicBool>,
        fail_write: Arc<std::sync::atomic::AtomicBool>,
    }
    impl super::super::api_credentials::CredentialStore for MemoryStore {
        fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, &'static str> {
            if self.fail.load(Ordering::SeqCst) {
                return Err("api_secure_storage_unavailable");
            }
            Ok(self.bytes.lock().unwrap().clone().map(Zeroizing::new))
        }
        fn write(&self, bytes: &[u8]) -> Result<(), &'static str> {
            if self.fail.load(Ordering::SeqCst) || self.fail_write.load(Ordering::SeqCst) {
                return Err("api_secure_storage_write_failed");
            }
            *self.bytes.lock().unwrap() = Some(bytes.to_vec());
            Ok(())
        }
        fn delete(&self) -> Result<(), &'static str> {
            if self.fail.load(Ordering::SeqCst) {
                return Err("api_secure_storage_delete_failed");
            }
            *self.bytes.lock().unwrap() = None;
            Ok(())
        }
    }
    #[test]
    fn restore_revoke_delete_and_same_endpoint_key_reuse() {
        let vault = MemoryStore::default();
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault.clone()));
        s.configure(setup(&s)).unwrap();
        drop(s);
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault.clone()));
        assert!(s.status().consent);
        let mut changed = setup(&s);
        changed.key.clear();
        changed.model = "new-model".into();
        s.configure(changed).unwrap();
        let active = s.begin(true).unwrap();
        let stale = setup(&s);
        s.revoke().unwrap();
        assert!(active.cancel.is_cancelled());
        assert!(!s.finish(&active));
        assert!(s.configure(stale).is_err());
        drop(s);
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault.clone()));
        assert_eq!(s.status().model.as_deref(), Some("new-model"));
        assert!(s.status().configured);
        assert!(!s.status().consent);
        assert!(s.begin(true).is_err());
        assert!(s
            .saved_key("https://other.test/v1/chat/completions")
            .is_err());
        s.forget().unwrap();
        drop(s);
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault));
        assert!(!s.status().configured);
    }
    #[test]
    fn unavailable_or_corrupt_vault_never_silently_enables_requests() {
        let vault = MemoryStore::default();
        *vault.bytes.lock().unwrap() = Some(b"invalid".to_vec());
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault.clone()));
        assert!(s.status().storage_error.is_some());
        assert!(!s.status().configured);
        s.configure(setup(&s)).unwrap();
        let ticket = s.begin(true).unwrap();
        vault.fail.store(true, Ordering::SeqCst);
        assert!(s.revoke().is_err());
        assert!(ticket.cancel.is_cancelled());
        assert!(!s.status().request_enabled);
        assert!(s.forget().is_err());
        assert!(s.status().storage_error.is_some());
        let mut restart = ApiSession::default();
        restart.attach_store(Box::new(vault));
        assert!(!restart.status().configured);
        assert!(restart.status().storage_error.is_some());
    }
    #[test]
    fn failed_write_or_cross_endpoint_reuse_never_leaves_previous_grant() {
        let vault = MemoryStore::default();
        let mut s = ApiSession::default();
        s.attach_store(Box::new(vault.clone()));
        s.configure(setup(&s)).unwrap();
        let mut switched = setup(&s);
        switched.key.clear();
        switched.endpoint = "https://other.test/v1/chat/completions".into();
        assert!(s.configure(switched).is_err());
        assert!(!s.status().configured);
        assert!(vault.bytes.lock().unwrap().is_none());
        s.configure(setup(&s)).unwrap();
        vault.fail_write.store(true, Ordering::SeqCst);
        assert!(s.configure(setup(&s)).is_err());
        assert!(!s.status().request_enabled);
        assert!(vault.bytes.lock().unwrap().is_none());
        let mut restart = ApiSession::default();
        restart.attach_store(Box::new(vault));
        assert!(!restart.status().configured);
    }
    #[cfg(windows)]
    #[test]
    fn windows_secure_storage_synthetic_process_restart_and_removal() {
        use super::super::api_credentials::{CredentialStore, WindowsStore};
        let path = std::env::temp_dir().join(format!("pencil-vault-fixture-{}", Uuid::new_v4()));
        struct Cleanup(WindowsStore);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete();
            }
        }
        let cleanup = Cleanup(WindowsStore::for_profile(&path));
        assert!(cleanup.0.read().unwrap().is_none());
        assert!(!path.exists());
        assert!(cleanup.0.write(&vec![0u8; 2561]).is_err());
        assert!(cleanup.0.read().unwrap().is_none());
        let mut s = ApiSession::default();
        s.attach_store(Box::new(WindowsStore::for_profile(&path)));
        s.configure(setup(&s)).unwrap();
        drop(s);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "provider::api_session::tests::windows_secure_storage_child",
                "--ignored",
            ])
            .env("PENCIL_SYNTHETIC_VAULT_TEST", &path)
            .output()
            .unwrap();
        assert!(result.status.success(), "synthetic child failed");
        let mut s = ApiSession::default();
        s.attach_store(Box::new(WindowsStore::for_profile(&path)));
        assert!(s.status().configured);
        assert!(!s.status().consent);
        assert!(s.begin(true).is_err());
        s.forget().unwrap();
        assert!(cleanup.0.read().unwrap().is_none());
        assert!(
            !path.exists(),
            "secure setup must not create ordinary profile files"
        );
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "invoked only by unique synthetic vault parent"]
    fn windows_secure_storage_child() {
        let path = std::path::PathBuf::from(
            std::env::var_os("PENCIL_SYNTHETIC_VAULT_TEST").expect("parent required"),
        );
        assert!(path.starts_with(std::env::temp_dir()));
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("pencil-vault-fixture-"));
        let mut s = ApiSession::default();
        s.attach_store(Box::new(
            super::super::api_credentials::WindowsStore::for_profile(&path),
        ));
        assert!(s.status().configured && s.status().consent);
        assert_eq!(s.status().model.as_deref(), Some("synthetic-model"));
        assert!(!serde_json::to_string(&s.status())
            .unwrap()
            .contains("dummy-secret"));
        s.revoke().unwrap();
    }
    #[test]
    fn stale_ui_save_cannot_restore_revoked_key_or_permission() {
        let mut s = ApiSession::default();
        let stale = setup(&s);
        s.clear();
        assert!(matches!(s.configure(stale), Err("api_setup_stale")));
        assert!(!s.status().configured);
        assert!(!s.status().consent);
    }
    #[test]
    fn status_never_contains_key_and_requires_setup_consent() {
        let mut s = ApiSession::default();
        let status = s.configure(setup(&s)).unwrap();
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("dummy-secret"));
        assert!(status.request_enabled);
        assert!(status.configured && status.consent);
        assert!(!ApiSession::default().status().configured);
    }
    #[test]
    fn both_setup_and_per_request_consent_required() {
        let mut s = ApiSession::default();
        let mut input = setup(&s);
        input.consent = false;
        s.configure(input).unwrap();
        assert!(s.begin(true).is_err());
        s.configure(setup(&s)).unwrap();
        assert!(s.begin(false).is_err());
        assert!(s.begin(true).is_ok());
    }
    #[test]
    fn replacement_revokes_old_operation_without_releasing_new_slot() {
        let mut s = ApiSession::default();
        s.configure(setup(&s)).unwrap();
        let old = s.begin(true).unwrap();
        let mut input = setup(&s);
        input.model = "different-model".into();
        s.configure(input).unwrap();
        assert!(old.cancel.is_cancelled());
        assert!(s.begin(true).is_err());
        assert!(!s.finish(&old));
        let next = s.begin(true).unwrap();
        assert!(!s.finish(&old));
        assert!(s.begin(true).is_err());
        assert!(s.finish(&next));
        assert!(!s.finish(&next));
    }
    #[test]
    fn invalid_replacement_cannot_fall_back_to_old_credentials() {
        let mut s = ApiSession::default();
        s.configure(setup(&s)).unwrap();
        let old = s.begin(true).unwrap();
        let mut input = setup(&s);
        input.endpoint = "http://bad.test/chat/completions".into();
        assert!(s.configure(input).is_err());
        assert!(old.cancel.is_cancelled());
        assert!(!s.status().configured);
        assert!(!s.finish(&old));
    }
    #[test]
    fn clear_and_drop_cancel_inflight_and_forget_configuration() {
        let mut s = ApiSession::default();
        s.configure(setup(&s)).unwrap();
        let old = s.begin(true).unwrap();
        s.clear();
        assert!(old.cancel.is_cancelled());
        assert!(s.begin(true).is_err());
        assert!(!s.finish(&old));
        s.configure(setup(&s)).unwrap();
        let next = s.begin(true).unwrap();
        drop(s);
        assert!(next.cancel.is_cancelled());
    }
    #[tokio::test]
    async fn revoked_ticket_cannot_send() {
        let mut s = ApiSession::default();
        s.configure(setup(&s)).unwrap();
        let old = s.begin(true).unwrap();
        s.clear();
        assert_eq!(
            old.complete("synthetic").await,
            Err(super::super::http_api::ApiError::Cancelled)
        );
        assert_eq!(
            old.complete("synthetic").await,
            Err(super::super::http_api::ApiError::RequestAlreadyUsed)
        );
    }
}
