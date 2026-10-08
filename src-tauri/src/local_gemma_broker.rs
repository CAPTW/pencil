//! Bounded resident broker admission. Launcher supplies pinned images and global permit.
//! No document contents travel on this bootstrap pipe.
use super::{
    peer::{ImagePin, ProcessPin},
    pipe::{self, Endpoint, Permit, Ticket},
    service::Client,
    Binding, Recommendation,
};
use serde::{Deserialize, Serialize};
use std::{
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    time::{timeout_at, Instant},
};
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hello {
    version: u32,
    epoch: String,
}
#[derive(Clone)]
pub struct Connection {
    pub name: String,
    pub broker_image: ImagePin,
}
pub struct Admission {
    pipe: NamedPipeServer,
    name: String,
    host_images: Vec<ImagePin>,
}
pub struct Grant {
    ticket: Ticket,
    peer: ProcessPin,
    epoch: String,
}
impl Admission {
    /// Keep one listener present while at most four connected peers are served.
    /// Idle listening has no request budget; every accepted peer gets one budget
    /// covering bootstrap and inference. Excess peers are disconnected, never queued.
    pub async fn run(mut self, client: Client, active: Arc<AtomicBool>) -> Result<(), String> {
        let mut tasks = tokio::task::JoinSet::new();
        let result = loop {
            tokio::select! { biased;
                _ = async { while active.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }} => break Ok(()),
                _ = client.closed() => break Err("gemma_service_closed".into()),
                finished = tasks.join_next(), if !tasks.is_empty() => {
                    if finished.is_some_and(|r| r.is_err()) { break Err("gemma_broker_task".into()); }
                },
                connected = self.pipe.connect() => {
                    if connected.is_err() { break Err("gemma_broker_connect".into()); }
                    let next = match ServerOptions::new().reject_remote_clients(true).create(&self.name) {
                        Ok(next) => next,
                        Err(_) => break Err("gemma_broker_renew".into()),
                    };
                    let connected = Admission { pipe: std::mem::replace(&mut self.pipe, next),
                        name: self.name.clone(), host_images: self.host_images.clone() };
                    if tasks.len() < 4 {
                        let client = client.clone();
                        let active = active.clone();
                        // Operational ceiling, not the 1s/3s user-facing performance target.
                        let deadline = Instant::now() + std::time::Duration::from_secs(15);
                        tasks.spawn(connected.serve_connected(client, active, deadline));
                    }
                }
            }
        };
        active.store(false, Ordering::SeqCst);
        drop(self);
        // Revocation closes transports; the service owner must then acknowledge cleanup.
        while tasks.join_next().await.is_some() {}
        result
    }
    pub fn create(host_image: ImagePin) -> Result<Self, String> {
        Self::create_for_clients(vec![host_image])
    }
    pub fn create_for_clients(host_images: Vec<ImagePin>) -> Result<Self, String> {
        if host_images.is_empty() || host_images.len() > 2 {
            return Err("gemma_broker_clients".into());
        }
        let name = format!(r"\\.\pipe\Grammar.Gemma.Broker.{}", uuid::Uuid::new_v4());
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .map_err(|_| "gemma_broker_create")?;
        Ok(Self {
            pipe,
            name,
            host_images,
        })
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub async fn serve(
        self,
        client: Client,
        active: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<(), String> {
        tokio::select! { biased;
            _ = async { while active.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }} => return Err("gemma_broker_revoked".into()),
            connected = timeout_at(deadline, self.pipe.connect()) => {
                connected.map_err(|_| "gemma_broker_deadline")?
                    .map_err(|_| "gemma_broker_connect")?;
            }
        }
        self.serve_connected(client, active, deadline).await
    }
    async fn serve_connected(
        mut self,
        client: Client,
        active: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<(), String> {
        let operation = async {
            let mut pid = 0;
            if unsafe { GetNamedPipeClientProcessId(self.pipe.as_raw_handle(), &mut pid) } == 0 {
                return Err("gemma_broker_peer".into());
            }
            let peer = self.host_images.iter().find_map(|image| image.process(pid).ok())
                .ok_or("gemma_peer_image")?;
            let hello: Hello = serde_json::from_slice(&pipe::read_frame(&mut self.pipe).await?)
                .map_err(|_| "gemma_broker_hello")?;
            if hello.version != 1
                || hello.epoch.is_empty()
                || hello.epoch.len() > 128
                || hello.epoch.contains('\0')
                || !peer.alive()
            {
                return Err("gemma_broker_scope".into());
            }
            let endpoint = Endpoint::create()?;
            let ticket = endpoint.ticket();
            pipe::write_frame(
                &mut self.pipe,
                &serde_json::to_vec(&ticket).map_err(|_| "gemma_broker_ticket")?,
            )
            .await?;
            if self.pipe.read_u8().await.map_err(|_| "gemma_broker_ack")? != 1 {
                return Err("gemma_broker_ack".into());
            }
            endpoint
                .serve(
                    Permit {
                        client_pid: peer.pid(),
                        epoch: hello.epoch,
                        active: active.clone(),
                    },
                    client,
                    deadline,
                )
                .await
            // peer handle remains alive through endpoint completion, preventing PID reuse.
        };
        tokio::select! {biased;
         _=async{while active.load(Ordering::SeqCst){tokio::time::sleep(std::time::Duration::from_millis(5)).await;}}=>Err("gemma_broker_revoked".into()),
         result=timeout_at(deadline,operation)=>result.unwrap_or_else(|_|Err("gemma_broker_deadline".into())),
        }
    }
}
impl Connection {
    pub async fn acquire(&self, epoch: &str, deadline: Instant) -> Result<Grant, String> {
        if !self.name.starts_with(r"\\.\pipe\Grammar.Gemma.Broker.")
            || epoch.is_empty()
            || epoch.len() > 128
        {
            return Err("gemma_broker_scope".into());
        }
        timeout_at(deadline, async {
            let mut socket = ClientOptions::new()
                .open(&self.name)
                .map_err(|_| "gemma_broker_open")?;
            let mut pid = 0;
            if unsafe { GetNamedPipeServerProcessId(socket.as_raw_handle(), &mut pid) } == 0 {
                return Err("gemma_broker_peer".into());
            }
            let peer = self.broker_image.process(pid)?;
            pipe::write_frame(
                &mut socket,
                &serde_json::to_vec(&Hello {
                    version: 1,
                    epoch: epoch.into(),
                })
                .map_err(|_| "gemma_broker_hello")?,
            )
            .await?;
            let ticket: Ticket = serde_json::from_slice(&pipe::read_frame(&mut socket).await?)
                .map_err(|_| "gemma_broker_ticket")?;
            if ticket.owner_pid != peer.pid()
                || !ticket.name.starts_with(r"\\.\pipe\Grammar.Gemma.")
                || !peer.alive()
            {
                return Err("gemma_broker_ticket".into());
            }
            socket.write_u8(1).await.map_err(|_| "gemma_broker_ack")?;
            Ok(Grant {
                ticket,
                peer,
                epoch: epoch.into(),
            })
        })
        .await
        .unwrap_or_else(|_| Err("gemma_broker_deadline".into()))
    }
}
impl Grant {
    pub async fn recommend(
        self,
        binding: Binding,
        deadline: Instant,
    ) -> Result<Recommendation, String> {
        if binding.epoch != self.epoch || !self.peer.alive() {
            return Err("gemma_broker_scope".into());
        }
        pipe::recommend(&self.ticket, binding, deadline).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn own_image() -> ImagePin {
        let path = std::env::current_exe().unwrap();
        let hash = crate::instant_selection::sha256_hex(&std::fs::read(&path).unwrap());
        ImagePin::verify(&path, &hash).unwrap()
    }
    fn fake_image() -> (std::path::PathBuf, ImagePin) {
        let path = std::env::temp_dir().join(format!("grammar-peer-{}.bin", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"owned synthetic image").unwrap();
        let hash = crate::instant_selection::sha256_hex(b"owned synthetic image");
        let pin = ImagePin::verify(&path, &hash).unwrap();
        (path, pin)
    }
    #[tokio::test]
    #[ignore = "explicit owned two-process GPU trial"]
    async fn owned_resident_binary() {
        use tokio::io::{AsyncBufReadExt, BufReader};
        assert_eq!(
            std::env::var("GRAMMAR_GEMMA_TRIAL").as_deref(),
            Ok("owned-synthetic")
        );
        let root = std::path::Path::new("D:/dev/grammar-evidence/grammar-gemma-20261007");
        let run_id = std::env::var("GRAMMAR_GEMMA_RUN_ID").unwrap_or_else(|_| "r1".into());
        assert!(
            !run_id.is_empty()
                && run_id.len() <= 16
                && run_id.bytes().all(|b| b.is_ascii_alphanumeric())
        );
        let receipt = root.join(format!("resident-binary-{run_id}.json"));
        assert!(!receipt.exists());
        let binary = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/grammar-gemma-broker.exe");
        let image_hash = crate::instant_selection::sha256_hex(&std::fs::read(&binary).unwrap());
        let broker_image = ImagePin::verify(&binary, &image_hash).unwrap();
        let host = std::env::current_exe().unwrap();
        let host_hash = crate::instant_selection::sha256_hex(&std::fs::read(&host).unwrap());
        let started = Instant::now();
        let mut child = tokio::process::Command::new(&binary)
            .args([
                root.join("runtime/llama-server.exe").to_str().unwrap(),
                "a812655fed77b9f98d918e7aa0b1552403b76dacbeb9fa24c17dac9ed3d08ec1",
                root.join("gemma-4-12b-it-qat-q4_0.gguf").to_str().unwrap(),
                "93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b",
                host.to_str().unwrap(),
                &host_hash,
                "785601b8f32ba6ab10374f8e9ead5d8deadd56e70940883ef65608dbfbb80860",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let control = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut rows = Vec::new();
        let run: Result<(), String> = async {
            let line = tokio::time::timeout(std::time::Duration::from_secs(190), output.next_line()).await
                .map_err(|_| "startup timeout")?.map_err(|_| "startup read")?.ok_or("startup EOF")?;
            let ready: serde_json::Value = serde_json::from_str(&line).map_err(|_| "ready JSON")?;
            assert_eq!(ready["pid"].as_u64(), child.id().map(u64::from));
            let connection = Connection { name: ready["name"].as_str().ok_or("ready name")?.into(), broker_image };
            rows.push(serde_json::json!({"startupMs":started.elapsed().as_millis()}));
            for (revision, (source, expected)) in [("She have two books.", "She has two books."),
                ("He do not agree.", "He does not agree."), ("She have two books.", "She has two books.")].into_iter().enumerate() {
                let before = Instant::now();
                let deadline = before + std::time::Duration::from_secs(1);
                let result = connection.acquire("owned-binary", deadline).await?
                    .recommend(Binding { intent: crate::local_gemma::Intent::Grammar, epoch:"owned-binary".into(), revision:revision as u64 + 1, source:source.into() }, deadline).await?;
                rows.push(serde_json::json!({"revision":revision+1,"ms":before.elapsed().as_millis(),"replacement":result.replacement}));
                if result.replacement != expected { return Err("quality mismatch".into()); }
            }
            Ok(())
        }.await;
        drop(control);
        let cleanup =
            tokio::time::timeout(std::time::Duration::from_secs(8), output.next_line()).await;
        let exit = tokio::time::timeout(std::time::Duration::from_secs(8), child.wait()).await;
        let cleanup_ok =
            matches!(&cleanup, Ok(Ok(Some(line))) if line == "{\"cleanup\":\"complete\"}");
        let exit_ok = matches!(&exit, Ok(Ok(status)) if status.success());
        std::fs::write(receipt, serde_json::to_vec_pretty(&serde_json::json!({"scope":"OWNED_RESIDENT_BINARY_SYNTHETIC_NOT_UI", "binarySha256":image_hash,"rows":rows,"result":run,"cleanup":cleanup_ok,"exit":exit_ok})).unwrap()).unwrap();
        assert!(run.is_ok(), "{run:?}");
        assert!(cleanup_ok && exit_ok);
    }
    #[tokio::test]
    async fn resident_renews_tickets_and_revokes_idle_peers() {
        let own = own_image();
        let admission = Admission::create(own.clone()).unwrap();
        let connection = Connection {
            name: admission.name(),
            broker_image: own,
        };
        let active = Arc::new(AtomicBool::new(true));
        let server = tokio::spawn(
            admission.run(super::super::service::rejecting_test_client(), active.clone()),
        );
        let mut names = std::collections::HashSet::new();
        for revision in 1..=8 {
            let deadline = Instant::now() + std::time::Duration::from_secs(1);
            let grant = connection.acquire("owned", deadline).await.unwrap();
            assert!(names.insert(grant.ticket.name.clone()));
            assert!(grant
                .recommend(
                    Binding {
                        intent: crate::local_gemma::Intent::Grammar,
                        epoch: "owned".into(),
                        revision,
                        source: "She have books.".into()
                    },
                    deadline
                )
                .await
                .is_err());
        }
        let mut stalled = ClientOptions::new().open(&connection.name).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        active.store(false, Ordering::SeqCst);
        tokio::time::timeout(std::time::Duration::from_millis(250), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(stalled.read_u8().await.is_err());
        assert!(ClientOptions::new().open(&connection.name).is_err());
    }
    #[tokio::test]
    async fn closed_service_revokes_admission_without_waiting_for_another_request() {
        let admission = Admission::create(own_image()).unwrap();
        let name = admission.name();
        let active = Arc::new(AtomicBool::new(true));
        let result = tokio::time::timeout(std::time::Duration::from_millis(250),
            admission.run(super::super::service::closed_test_client(), active.clone())).await.unwrap();
        assert_eq!(result.unwrap_err(), "gemma_service_closed");
        assert!(!active.load(Ordering::SeqCst));
        assert!(ClientOptions::new().open(&name).is_err());
    }

    #[tokio::test]
    async fn service_failure_disconnects_an_already_connected_idle_peer() {
        let admission = Admission::create(own_image()).unwrap();
        let name = admission.name();
        let active = Arc::new(AtomicBool::new(true));
        let (client, close) = super::super::service::closable_test_client();
        let server = tokio::spawn(admission.run(client, active.clone()));
        let mut socket = ClientOptions::new().open(&name).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        close.send(()).unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_millis(250),server).await.unwrap().unwrap();
        assert_eq!(result.unwrap_err(),"gemma_service_closed");
        assert!(!active.load(Ordering::SeqCst));
        assert!(socket.read_u8().await.is_err());
        assert!(ClientOptions::new().open(&name).is_err());
    }

    #[tokio::test]
    async fn resident_caps_stalled_connections() {
        let admission = Admission::create(own_image()).unwrap();
        let name = admission.name();
        let active = Arc::new(AtomicBool::new(true));
        let server = tokio::spawn(
            admission.run(super::super::service::rejecting_test_client(), active.clone()),
        );
        let mut held = Vec::new();
        for _ in 0..4 {
            held.push(ClientOptions::new().open(&name).unwrap());
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let mut excess = ClientOptions::new().open(&name).unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), excess.read_u8())
                .await
                .unwrap()
                .is_err()
        );
        active.store(false, Ordering::SeqCst);
        server.await.unwrap().unwrap();
        for mut socket in held {
            assert!(socket.read_u8().await.is_err());
        }
    }
    #[tokio::test]
    async fn second_pinned_client_is_admitted_without_allowing_arbitrary_images() {
        let (path, fake) = fake_image();
        let own = own_image();
        let admission = Admission::create_for_clients(vec![fake, own.clone()]).unwrap();
        let connection = Connection { name: admission.name(), broker_image: own };
        let server = tokio::spawn(admission.serve(super::super::service::closed_test_client(),
            Arc::new(AtomicBool::new(true)), Instant::now()+std::time::Duration::from_secs(1)));
        let grant = connection.acquire("desktop", Instant::now()+std::time::Duration::from_secs(1)).await.unwrap();
        drop(grant);
        assert!(server.await.unwrap().is_err());
        std::fs::remove_file(path).unwrap();
        assert!(Admission::create_for_clients(vec![]).is_err());
    }

    #[tokio::test]
    async fn wrong_host_image_is_rejected_before_inference() {
        let (path, fake) = fake_image();
        let own = own_image();
        let admission = Admission::create(fake).unwrap();
        let connection = Connection {
            name: admission.name(),
            broker_image: own,
        };
        let server = tokio::spawn(admission.serve(
            super::super::service::closed_test_client(),
            Arc::new(AtomicBool::new(true)),
            Instant::now() + std::time::Duration::from_secs(1),
        ));
        assert!(connection
            .acquire("owned", Instant::now() + std::time::Duration::from_secs(1))
            .await
            .is_err());
        assert_eq!(server.await.unwrap().unwrap_err(), "gemma_peer_image");
        std::fs::remove_file(path).unwrap();
    }
    #[tokio::test]
    async fn wrong_broker_image_receives_no_metadata() {
        let (path, fake) = fake_image();
        let mut admission = Admission::create(own_image()).unwrap();
        let connection = Connection {
            name: admission.name(),
            broker_image: fake,
        };
        let client = tokio::spawn(async move {
            connection
                .acquire("owned", Instant::now() + std::time::Duration::from_secs(1))
                .await
        });
        admission.pipe.connect().await.unwrap();
        assert_eq!(client.await.unwrap().err().unwrap(), "gemma_peer_image");
        let read = admission.pipe.read_u8().await;
        assert!(read.is_err());
        std::fs::remove_file(path).unwrap();
    }
}
