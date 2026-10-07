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
                if request.reply.is_closed() || request.cancel.load(Ordering::SeqCst) {
                    continue;
                }
                let cancellation = request.cancel.clone();
                let result = {
                    let operation =
                        engine.recommend(request.binding, cancellation.clone(), request.deadline);
                    tokio::pin!(operation);
                    tokio::select! {
                        biased;
                        _ = revoked(&closing) => {
                            // Do not drop the operation: it owns actual process cleanup.
                            cancellation.store(true, Ordering::SeqCst);
                            operation.await
                        },
                        result = &mut operation => result,
                    }
                };
                let result = if closing.load(Ordering::SeqCst) {
                    Err("gemma_service_closed".into())
                } else {
                    result
                };
                let _ = request.reply.send(result);
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
    pub async fn recommend(
        &self,
        binding: Binding,
        deadline: Instant,
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
mod tests {
    use super::*;
    #[tokio::test]
    async fn bounded_queue_and_dropped_waiter_revoke_request() {
        let (queue, mut receiver) = mpsc::channel(1);
        let client = Client { queue };
        let binding = Binding {
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
