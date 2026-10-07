//! One trusted broker admission. Launcher supplies pinned images and global permit.
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
    host_image: ImagePin,
}
pub struct Grant {
    ticket: Ticket,
    peer: ProcessPin,
    epoch: String,
}
impl Admission {
    pub fn create(host_image: ImagePin) -> Result<Self, String> {
        let name = format!(r"\\.\pipe\Grammar.Gemma.Broker.{}", uuid::Uuid::new_v4());
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .map_err(|_| "gemma_broker_create")?;
        Ok(Self {
            pipe,
            name,
            host_image,
        })
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub async fn serve(
        mut self,
        client: Client,
        active: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<(), String> {
        let operation = async {
            self.pipe
                .connect()
                .await
                .map_err(|_| "gemma_broker_connect")?;
            let mut pid = 0;
            if unsafe { GetNamedPipeClientProcessId(self.pipe.as_raw_handle(), &mut pid) } == 0 {
                return Err("gemma_broker_peer".into());
            }
            let peer = self.host_image.process(pid)?;
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
