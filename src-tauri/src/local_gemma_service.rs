//! One resident engine shared by bounded in-process document clients.
//! IPC/admission across native-host processes is intentionally not activated here.
use super::{Binding, GemmaEngine, Recommendation};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{timeout_at, Instant},
};

struct Request {
    binding: Binding,
    supersedable: bool,
    superseded: Option<Arc<AtomicBool>>,
    cancel: Arc<AtomicBool>,
    deadline: Instant,
    reply: oneshot::Sender<Result<Recommendation, String>>,
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct Client {
    queue: mpsc::Sender<Request>,
}
pub struct Service {
    shutdown: Arc<AtomicBool>,
    task: JoinHandle<Result<(), String>>,
}
impl Service {
    /// The integrator owns startup and manifest admission; clients cannot spawn engines.
    pub fn own(mut engine: GemmaEngine) -> (Self, Client) {
        let (queue, mut receiver) = mpsc::channel::<Request>(4);
        let shutdown = Arc::new(AtomicBool::new(false));
        let closing = shutdown.clone();
        let task = tokio::spawn(async move {
            loop {
                let request = tokio::select! {
                    biased;
                    _ = revoked(&closing) => break,
                    request = receiver.recv() => match request { Some(r) => r, None => break },
                };
                if request.reply.is_closed() || request.cancel.load(Ordering::SeqCst)
                    || request.superseded.as_ref().is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                    let _ = request.reply.send(Err("gemma_cancelled_before_admission".into()));
                    continue;
                }
                let cancellation = request.cancel.clone();
                let controlled = request.superseded.is_some();
                let soft = request.superseded.unwrap_or_else(|| cancellation.clone());
                let result = {
                    let operation = async {
                        if request.supersedable || controlled {
                            engine
                                .recommend_supersedable(
                                    request.binding,
                                    soft,
                                    request.deadline,
                                )
                                .await
                        } else {
                            engine
                                .recommend(request.binding, cancellation.clone(), request.deadline)
                                .await
                        }
                    };
                    tokio::pin!(operation);
                    tokio::select! {
                        biased;
                        _ = revoked(&closing) => {
                            // Leaving this scope drops only the request future.
                            // The owner explicitly stops the actual engine immediately below.
                            cancellation.store(true, Ordering::SeqCst);
                            Err("gemma_service_closed".into())
                        },
                        _ = revoked(&cancellation), if controlled => Err("gemma_cancelled".into()),
                        result = &mut operation => result,
                    }
                };
                let result = if closing.load(Ordering::SeqCst)
                    || (controlled && cancellation.load(Ordering::SeqCst)) {
                    engine.stop().await?;
                    Err(if closing.load(Ordering::SeqCst) {
                        "gemma_service_closed"
                    } else { "gemma_cancelled" }.into())
                } else {
                    result
                };
                let _ = request.reply.send(result);
                // A stopped model is terminal for this owner. Close the queue so
                // admission/launcher observes failure instead of advertising readiness.
                if !engine.ready { break; }
            }
            receiver.close();
            while let Ok(request) = receiver.try_recv() {
                let _ = request.reply.send(Err("gemma_service_closed".into()));
            }
            engine.stop().await
        });
        (Self { shutdown, task }, Client { queue })
    }
    /// Completion is acknowledged only after engine.stop verifies actual cleanup.
    pub async fn close(self) -> Result<(), String> {
        self.shutdown.store(true, Ordering::SeqCst);
        // Take the task out without aborting it if the caller drops this close future.
        let mut this = self;
        (&mut this.task)
            .await
            .map_err(|_| "gemma_service_task_failed".to_string())?
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        // Detached owner task retains the engine until its bounded cleanup completes.
        self.shutdown.store(true, Ordering::SeqCst);
    }
}
async fn revoked(flag: &AtomicBool) {
    while !flag.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}
impl Client {
    pub async fn closed(&self) {
        self.queue.closed().await;
    }
    pub async fn recommend(
        &self,
        binding: Binding,
        deadline: Instant,
    ) -> Result<Recommendation, String> {
        self.request(binding, deadline, false, None).await
    }
    /// Only use for source supersession while permission remains granted.
    /// Disable/revocation must retain the hard-cancellation/close path.
    pub async fn recommend_supersedable(
        &self,
        binding: Binding,
        deadline: Instant,
    ) -> Result<Recommendation, String> {
        self.request(binding, deadline, true, None).await
    }
    /// Soft supersession retains the waiter until idle is confirmed. Dropping
    /// this future (EOF/revocation/deadline) still requests physical hard cleanup,
    /// including when soft cancellation is already draining.
    pub async fn recommend_controlled(
        &self,
        binding: Binding,
        deadline: Instant,
        superseded: Arc<AtomicBool>,
    ) -> Result<Recommendation, String> {
        self.request(binding, deadline, false, Some(superseded)).await
    }
    async fn request(
        &self,
        binding: Binding,
        deadline: Instant,
        supersedable: bool,
        superseded: Option<Arc<AtomicBool>>,
    ) -> Result<Recommendation, String> {
        if Instant::now() >= deadline {
            return Err("gemma_deadline".into());
        }
        let cancellation = Arc::new(AtomicBool::new(false));
        let _guard = CancelOnDrop(cancellation.clone());
        let (reply, result) = oneshot::channel();
        self.queue
            .try_send(Request {
                binding,
                supersedable,
                superseded,
                cancel: cancellation,
                deadline,
                reply,
            })
            .map_err(|e| {
                match e {
                    mpsc::error::TrySendError::Full(_) => "gemma_busy",
                    mpsc::error::TrySendError::Closed(_) => "gemma_service_closed",
                }
                .to_string()
            })?;
        let result = timeout_at(deadline, result)
            .await
            .map_err(|_| "gemma_deadline".to_string())?
            .map_err(|_| "gemma_service_closed".to_string())?;
        if Instant::now() >= deadline {
            Err("gemma_deadline".into())
        } else {
            result
        }
    }
}

#[cfg(test)]
pub(super) fn controlled_test_client() -> (
    Client,
    oneshot::Receiver<(Arc<AtomicBool>, Arc<AtomicBool>)>,
    oneshot::Sender<Result<Recommendation, String>>,
) {
    let (queue, mut receiver) = mpsc::channel::<Request>(1);
    let (observe, observed) = oneshot::channel();
    let (complete, completed) = oneshot::channel();
    tokio::spawn(async move {
        let request = receiver.recv().await.unwrap();
        let _ = observe.send((request.superseded.unwrap(), request.cancel));
        if let Ok(result) = completed.await { let _ = request.reply.send(result); }
    });
    (Client { queue }, observed, complete)
}

#[cfg(test)]
pub(super) fn closed_test_client() -> Client {
    let (queue, receiver) = mpsc::channel(1);
    drop(receiver);
    Client { queue }
}

#[cfg(test)]
pub(super) fn rejecting_test_client() -> Client {
    let (queue, mut receiver) = mpsc::channel::<Request>(4);
    tokio::spawn(async move {
        while let Some(request) = receiver.recv().await {
            let _ = request.reply.send(Err("synthetic_rejection".into()));
        }
    });
    Client { queue }
}

#[cfg(test)]
pub(super) fn closable_test_client() -> (Client, oneshot::Sender<()>) {
    let (queue, receiver) = mpsc::channel::<Request>(1);
    let (close, closed) = oneshot::channel();
    tokio::spawn(async move { let _ = closed.await; drop(receiver); });
    (Client { queue }, close)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn controlled_deadline_remains_hard_after_soft_supersession() {
        let (client, observed, complete) = controlled_test_client();
        let soft = Arc::new(AtomicBool::new(false));
        let signal = soft.clone();
        let waiter = tokio::spawn(async move {
            client.recommend_controlled(Binding {
                intent: crate::local_gemma::Intent::Grammar, epoch: "owned".into(),
                revision: 1, source: "synthetic".into(),
            }, Instant::now() + std::time::Duration::from_millis(100), signal).await
        });
        let (owner_soft, owner_hard) = observed.await.unwrap();
        soft.store(true, Ordering::SeqCst);
        assert!(owner_soft.load(Ordering::SeqCst));
        assert!(!owner_hard.load(Ordering::SeqCst));
        assert_eq!(waiter.await.unwrap().unwrap_err(), "gemma_deadline");
        assert!(owner_hard.load(Ordering::SeqCst));
        drop(complete);
    }
    #[tokio::test]
    async fn bounded_queue_and_dropped_waiter_revoke_request() {
        let (queue, mut receiver) = mpsc::channel(1);
        let client = Client { queue };
        let binding = Binding {
            intent: crate::local_gemma::Intent::Grammar,
            epoch: "owned".into(),
            revision: 1,
            source: "text".into(),
        };
        let c = client.clone();
        let b = binding.clone();
        let waiting = tokio::spawn(async move {
            c.recommend(b, Instant::now() + std::time::Duration::from_secs(10))
                .await
        });
        tokio::task::yield_now().await;
        assert_eq!(
            client
                .recommend(binding, Instant::now() + std::time::Duration::from_secs(1))
                .await
                .unwrap_err(),
            "gemma_busy"
        );
        let request = receiver.recv().await.unwrap();
        waiting.abort();
        let _ = waiting.await;
        assert!(request.cancel.load(Ordering::SeqCst));
        assert!(request.reply.is_closed());
    }
}
