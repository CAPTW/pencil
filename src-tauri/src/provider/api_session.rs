//! Session-only API setup. No disk, environment or credential-store access.
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

#[derive(Deserialize)]
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
    pub fn is_busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn status(&self) -> Status {
        Status {
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
    pub fn configure(&mut self, setup: Setup) -> Result<Status, &'static str> {
        if setup.expected_epoch != self.epoch.to_string() {
            return Err("api_setup_stale");
        }
        // Any current replacement attempt revokes the old key/consent, including invalid edits.
        self.clear();
        let config = ApiConfig::new(&setup.endpoint, &setup.model)
            .map_err(|_| "api_invalid_configuration")?;
        validate_key(&setup.key).map_err(|_| "api_invalid_key")?;
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
