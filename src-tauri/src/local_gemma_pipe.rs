//! Owned per-client named-pipe transport. No discovery, file-based tokens or activation.
//! The trusted launcher supplies the actual child PID and document permission.
use super::{service::Client, Binding, Recommendation};
use serde::{Deserialize, Serialize};
use std::{
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    time::{timeout_at, Instant},
};
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, GetNamedPipeServerProcessId};
const LIMIT: usize = 32768;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ticket {
    pub name: String,
    pub owner_pid: u32,
}
pub struct Endpoint {
    pipe: NamedPipeServer,
    ticket: Ticket,
}
pub struct Permit {
    pub client_pid: u32,
    pub epoch: String,
    pub active: Arc<AtomicBool>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    intent: super::Intent,
    epoch: String,
    revision: u64,
    text: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    #[serde(default)]
    superseded: bool,
    intent: super::Intent,
    epoch: String,
    revision: u64,
    replacement: String,
}
pub(super) async fn read_frame<T: AsyncRead + Unpin>(stream: &mut T) -> Result<Vec<u8>, String> {
    let size = stream.read_u32_le().await.map_err(|_| "gemma_pipe_read")? as usize;
    if size == 0 || size > LIMIT {
        return Err("gemma_pipe_frame_limit".into());
    }
    let mut data = vec![0; size];
    stream
        .read_exact(&mut data)
        .await
        .map_err(|_| "gemma_pipe_read")?;
    Ok(data)
}
pub(super) async fn write_frame<T: AsyncWrite + Unpin>(
    stream: &mut T,
    data: &[u8],
) -> Result<(), String> {
    if data.is_empty() || data.len() > LIMIT {
        return Err("gemma_pipe_frame_limit".into());
    }
    stream
        .write_u32_le(data.len() as u32)
        .await
        .map_err(|_| "gemma_pipe_write")?;
    stream
        .write_all(data)
        .await
        .map_err(|_| "gemma_pipe_write")?;
    Ok(())
}
impl Endpoint {
    pub fn create() -> Result<Self, String> {
        let name = format!(r"\\.\pipe\Grammar.Gemma.{}", uuid::Uuid::new_v4());
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .map_err(|_| "gemma_pipe_create")?;
        Ok(Self {
            pipe,
            ticket: Ticket {
                name,
                owner_pid: std::process::id(),
            },
        })
    }
    pub fn ticket(&self) -> Ticket {
        self.ticket.clone()
    }
    /// Single request, one authorized child, no client-supplied process identity.
    pub async fn serve(
        mut self,
        permit: Permit,
        client: Client,
        deadline: Instant,
    ) -> Result<(), String> {
        if !permit.active.load(Ordering::SeqCst) {
            return Err("gemma_pipe_revoked".into());
        }
        let operation = async {
            self.pipe
                .connect()
                .await
                .map_err(|_| "gemma_pipe_connect")?;
            let mut pid = 0;
            if unsafe { GetNamedPipeClientProcessId(self.pipe.as_raw_handle(), &mut pid) } == 0
                || pid != permit.client_pid
            {
                return Err("gemma_pipe_peer".into());
            }
            let request: Request = serde_json::from_slice(&read_frame(&mut self.pipe).await?)
                .map_err(|_| "gemma_pipe_request")?;
            let binding = Binding {
                intent: request.intent,
                epoch: request.epoch,
                revision: request.revision,
                source: request.text,
            };
            if binding.epoch != permit.epoch || !super::valid(&binding) {
                return Err("gemma_pipe_scope".into());
            }
            let superseded = Arc::new(AtomicBool::new(false));
            let result = {
                let operation = client.recommend_controlled(binding.clone(), deadline, superseded.clone());
                tokio::pin!(operation);
                loop {
                    tokio::select! {
                        biased;
                        signal=self.pipe.read_u8()=> {
                            if !matches!(signal, Ok(2)) || superseded.swap(true, Ordering::SeqCst) {
                                return Err("gemma_pipe_disconnected".into());
                            }
                        },
                        result=&mut operation=>break result,
                    }
                }
            };
            let discarded = superseded.load(Ordering::SeqCst);
            let replacement = match result {
                Ok(result) if result.matches(&binding) => result.replacement,
                Err(error) if discarded && matches!(error.as_str(),
                    "gemma_superseded_idle_confirmed" | "gemma_superseded_after_completion"
                    | "gemma_cancelled_before_admission") => String::new(),
                Err(error) => return Err(error),
                _ => return Err("gemma_pipe_binding".into()),
            };
            if !permit.active.load(Ordering::SeqCst) {
                return Err("gemma_pipe_revoked".into());
            }
            let reply = serde_json::to_vec(&Reply {
                superseded: discarded,
                intent: binding.intent,
                epoch: binding.epoch,
                revision: binding.revision,
                replacement: if discarded { String::new() } else { replacement },
            })
            .map_err(|_| "gemma_pipe_reply")?;
            write_frame(&mut self.pipe, &reply).await?;
            let mut ack = self.pipe.read_u8().await.map_err(|_| "gemma_pipe_ack")?;
            // Supersession may cross an already completed reply on the wire.
            if ack == 2 && !discarded {
                ack = self.pipe.read_u8().await.map_err(|_| "gemma_pipe_ack")?;
            }
            if ack != 1 {
                return Err("gemma_pipe_ack".into());
            }
            Ok(())
        };
        tokio::select! {
            biased;
            _=async{while permit.active.load(Ordering::SeqCst){tokio::time::sleep(std::time::Duration::from_millis(5)).await;}}=>Err("gemma_pipe_revoked".into()),
            result=timeout_at(deadline,operation)=>result.unwrap_or_else(|_|Err("gemma_pipe_deadline".into())),
        }
    }
}
pub async fn recommend(
    ticket: &Ticket,
    binding: Binding,
    deadline: Instant,
) -> Result<Recommendation, String> {
    recommend_controlled(ticket, binding, deadline, Arc::new(AtomicBool::new(false))).await
}
pub async fn recommend_controlled(
    ticket: &Ticket,
    binding: Binding,
    deadline: Instant,
    superseded: Arc<AtomicBool>,
) -> Result<Recommendation, String> {
    if !super::valid(&binding) {
        return Err("gemma_pipe_scope".into());
    }
    timeout_at(deadline, async {
        let mut pipe = ClientOptions::new()
            .open(&ticket.name)
            .map_err(|_| "gemma_pipe_open")?;
        let mut pid = 0;
        // Verify the OS endpoint owner BEFORE writing any document bytes.
        if unsafe { GetNamedPipeServerProcessId(pipe.as_raw_handle(), &mut pid) } == 0
            || pid != ticket.owner_pid
        {
            return Err("gemma_pipe_peer".into());
        }
        let data = serde_json::to_vec(&Request {
            intent: binding.intent.clone(),
            epoch: binding.epoch.clone(),
            revision: binding.revision,
            text: binding.source.clone(),
        })
        .map_err(|_| "gemma_pipe_request")?;
        write_frame(&mut pipe, &data).await?;
        let bytes = {
            let (mut reader, mut writer) = tokio::io::split(&mut pipe);
            let response = read_frame(&mut reader);
            tokio::pin!(response);
            tokio::select! {
                biased;
                _=async { while !superseded.load(Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }} => {
                    writer.write_u8(2).await.map_err(|_| "gemma_pipe_write")?;
                    response.await?
                },
                bytes=&mut response=>bytes?,
            }
        };
        let reply: Reply = serde_json::from_slice(&bytes)
            .map_err(|_| "gemma_pipe_reply")?;
        if reply.epoch != binding.epoch
            || reply.intent != binding.intent
            || reply.revision != binding.revision
            || reply.replacement.len() > 8192
            || (!reply.superseded && reply.replacement.trim().is_empty())
            || (reply.superseded && (!superseded.load(Ordering::SeqCst) || !reply.replacement.is_empty()))
            || reply.replacement.contains('\0')
        {
            return Err("gemma_pipe_reply_binding".into());
        }
        if Instant::now() >= deadline {
            return Err("gemma_pipe_deadline".into());
        }
        pipe.write_u8(1).await.map_err(|_| "gemma_pipe_ack")?;
        if reply.superseded || superseded.load(Ordering::SeqCst) {
            return Err("gemma_superseded".into());
        }
        Ok(Recommendation {
            binding,
            replacement: reply.replacement,
        })
    })
    .await
    .unwrap_or_else(|_| Err("gemma_pipe_deadline".into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn actual_pipe_soft_supersession_waits_for_owner_and_eof_stays_hard() {
        for mode in ["complete", "disconnect", "revoke"] {
            let endpoint = Endpoint::create().unwrap();
            let ticket = endpoint.ticket();
            let (client, observed, complete) = super::super::service::controlled_test_client();
            let deadline = Instant::now() + std::time::Duration::from_secs(3);
            let active = Arc::new(AtomicBool::new(true));
            let server = tokio::spawn(endpoint.serve(Permit {
                client_pid: std::process::id(), epoch: "owned".into(),
                active: active.clone(),
            }, client, deadline));
            let soft = Arc::new(AtomicBool::new(false));
            let signal = soft.clone();
            let waiter = tokio::spawn(async move {
                recommend_controlled(&ticket, Binding {
                    intent: super::super::Intent::Grammar, epoch: "owned".into(),
                    revision: 1, source: "synthetic".into(),
                }, deadline, signal).await
            });
            let (owner_soft, owner_hard) = observed.await.unwrap();
            soft.store(true, Ordering::SeqCst);
            timeout_at(deadline, async {
                while !owner_soft.load(Ordering::SeqCst) { tokio::task::yield_now().await; }
            }).await.unwrap();
            assert!(!owner_hard.load(Ordering::SeqCst));
            assert!(!waiter.is_finished(), "signal is not idle acknowledgement");
            if mode != "complete" {
                if mode == "disconnect" {
                    waiter.abort(); let _ = waiter.await;
                } else {
                    active.store(false, Ordering::SeqCst);
                    assert!(waiter.await.unwrap().is_err());
                }
                assert!(server.await.unwrap().is_err());
                assert!(owner_hard.load(Ordering::SeqCst), "EOF must override soft draining");
                drop(complete);
            } else {
                complete.send(Err("gemma_superseded_idle_confirmed".into())).unwrap();
                assert_eq!(waiter.await.unwrap().unwrap_err(), "gemma_superseded");
                server.await.unwrap().unwrap();
            }
        }
    }
    #[test]
    fn wire_intent_is_required_and_strict() {
        for intent in [serde_json::json!({"mode":"translate"}),
            serde_json::json!({"mode":"translate","target":"unknown"}),
            serde_json::json!({"mode":"grammar","target":"en"}),
            serde_json::json!({"mode":"paraphrase","instructions":"override"})] {
            assert!(serde_json::from_value::<Request>(serde_json::json!({"epoch":"d","revision":1,"text":"s","intent":intent})).is_err());
        }
        assert!(serde_json::from_value::<Request>(serde_json::json!({"epoch":"d","revision":1,"text":"s"})).is_err());
    }

    #[tokio::test]
    async fn actual_pipe_rejects_response_for_another_translation_language() {
        let mut endpoint = Endpoint::create().unwrap();
        let ticket = endpoint.ticket();
        let server = tokio::spawn(async move {
            endpoint.pipe.connect().await.unwrap();
            let request: Request = serde_json::from_slice(&read_frame(&mut endpoint.pipe).await.unwrap()).unwrap();
            assert_eq!(request.intent, super::super::Intent::Translate { target: super::super::Language::English });
            let reply = Reply { superseded: false, epoch: request.epoch, revision: request.revision,
                intent: super::super::Intent::Translate { target: super::super::Language::Korean }, replacement: "wrong language".into() };
            write_frame(&mut endpoint.pipe, &serde_json::to_vec(&reply).unwrap()).await.unwrap();
        });
        let result = recommend(&ticket, Binding { epoch: "doc".into(), revision: 1, source: "synthetic".into(),
            intent: super::super::Intent::Translate { target: super::super::Language::English } },
            Instant::now() + std::time::Duration::from_secs(2)).await;
        server.await.unwrap();
        assert!(matches!(result, Err(error) if error == "gemma_pipe_reply_binding"));
    }
    #[tokio::test]
    #[ignore = "owned child launched by GPU pipe trial"]
    async fn gpu_pipe_child() {
        use std::io::Read;
        let mut go = [0];
        std::io::stdin().read_exact(&mut go).unwrap();
        assert_eq!(go[0], 1);
        let ticket = Ticket {
            name: std::env::var("GRAMMAR_PIPE_TEST_NAME").unwrap(),
            owner_pid: std::env::var("GRAMMAR_PIPE_TEST_PID")
                .unwrap()
                .parse()
                .unwrap(),
        };
        let before = Instant::now();
        let result = recommend(
            &ticket,
            Binding {
                intent: crate::local_gemma::Intent::Grammar,
                epoch: "owned-gpu".into(),
                revision: 1,
                source: "She have two books.".into(),
            },
            before + std::time::Duration::from_secs(1),
        )
        .await;
        let receipt = std::env::var("GRAMMAR_PIPE_TEST_RECEIPT").unwrap();
        assert!(!std::path::Path::new(&receipt).exists());
        std::fs::write(receipt,serde_json::to_vec_pretty(&serde_json::json!({"ms":before.elapsed().as_millis(),"replacement":result.as_ref().ok().map(|r|&r.replacement),"error":result.as_ref().err()})).unwrap()).unwrap();
        assert_eq!(result.unwrap().replacement, "She has two books.");
    }
    #[tokio::test]
    #[ignore = "explicit owned cross-process GPU pipe trial"]
    async fn owned_gpu_cross_process() {
        assert_eq!(
            std::env::var("GRAMMAR_GEMMA_TRIAL").as_deref(),
            Ok("owned-synthetic")
        );
        let root = std::path::Path::new("D:/dev/grammar-evidence/grammar-gemma-20261007");
        let receipt = root.join("pipe-child-r1.json");
        assert!(!receipt.exists());
        let engine = super::super::GemmaEngine::start(
            &root.join("runtime/llama-server.exe"),
            &root.join("gemma-4-12b-it-qat-q4_0.gguf"),
            Instant::now() + std::time::Duration::from_secs(120),
        )
        .await
        .unwrap();
        let runtime = engine
            .workspace
            .as_ref()
            .unwrap()
            .cwd()
            .parent()
            .unwrap()
            .to_owned();
        let (owner, client) = super::super::service::Service::own(engine);
        let endpoint = Endpoint::create().unwrap();
        let ticket = endpoint.ticket();
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "local_gemma::pipe::tests::gpu_pipe_child",
            ])
            .env("GRAMMAR_PIPE_TEST_NAME", &ticket.name)
            .env("GRAMMAR_PIPE_TEST_PID", ticket.owner_pid.to_string())
            .env("GRAMMAR_PIPE_TEST_RECEIPT", &receipt)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        let serve = tokio::spawn(endpoint.serve(
            Permit {
                client_pid: pid,
                epoch: "owned-gpu".into(),
                active: Arc::new(AtomicBool::new(true)),
            },
            client,
            Instant::now() + std::time::Duration::from_secs(1),
        ));
        child.stdin.take().unwrap().write_u8(1).await.unwrap();
        let served = serve.await.unwrap();
        let exited = timeout_at(
            Instant::now() + std::time::Duration::from_secs(2),
            child.wait(),
        )
        .await;
        let closed = owner.close().await;
        std::fs::write(root.join("pipe-owner-r1.json"),serde_json::to_vec_pretty(&serde_json::json!({"served":served,"childExited":exited.as_ref().is_ok_and(|r|r.as_ref().is_ok_and(|s|s.success())),"cleanup":closed,"runtimeRemoved":!runtime.exists()})).unwrap()).unwrap();
        assert!(
            served.is_ok()
                && exited.unwrap().unwrap().success()
                && closed.is_ok()
                && !runtime.exists()
        );
    }
    #[tokio::test]
    async fn unauthorized_client_scope_and_revocation_fail_before_service() {
        for mode in ["pid", "epoch", "revoked"] {
            let endpoint = Endpoint::create().unwrap();
            let ticket = endpoint.ticket();
            let active = Arc::new(AtomicBool::new(mode != "revoked"));
            let permit = Permit {
                client_pid: if mode == "pid" { 0 } else { std::process::id() },
                epoch: "allowed".into(),
                active,
            };
            let serving = tokio::spawn(endpoint.serve(
                permit,
                super::super::service::closed_test_client(),
                Instant::now() + std::time::Duration::from_secs(1),
            ));
            if mode != "revoked" {
                let result = recommend(
                    &ticket,
                    Binding {
                        intent: crate::local_gemma::Intent::Grammar,
                        epoch: "wrong".into(),
                        revision: 1,
                        source: "synthetic".into(),
                    },
                    Instant::now() + std::time::Duration::from_secs(1),
                )
                .await;
                assert!(result.is_err());
            }
            assert_eq!(
                serving.await.unwrap().unwrap_err(),
                match mode {
                    "pid" => "gemma_pipe_peer",
                    "epoch" => "gemma_pipe_scope",
                    _ => "gemma_pipe_revoked",
                }
            );
        }
    }
    #[tokio::test]
    async fn wrong_server_pid_sends_no_document() {
        let mut endpoint = Endpoint::create().unwrap();
        let mut ticket = endpoint.ticket();
        ticket.owner_pid = 0;
        let client = tokio::spawn(async move {
            recommend(
                &ticket,
                Binding {
                    intent: crate::local_gemma::Intent::Grammar,
                    epoch: "e".into(),
                    revision: 1,
                    source: "synthetic".into(),
                },
                Instant::now() + std::time::Duration::from_secs(1),
            )
            .await
        });
        endpoint.pipe.connect().await.unwrap();
        assert_eq!(client.await.unwrap().unwrap_err(), "gemma_pipe_peer");
        let mut byte = [0];
        let read = endpoint.pipe.read(&mut byte).await;
        assert!(matches!(read, Ok(0)) || read.is_err());
    }
    #[tokio::test]
    async fn oversized_header_rejected_before_body() {
        let (mut sender, mut receiver) = tokio::io::duplex(4);
        sender.write_u32_le((LIMIT + 1) as u32).await.unwrap();
        assert_eq!(
            read_frame(&mut receiver).await.unwrap_err(),
            "gemma_pipe_frame_limit"
        );
    }
}
