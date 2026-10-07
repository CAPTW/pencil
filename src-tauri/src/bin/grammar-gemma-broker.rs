//! Explicit launcher-owned candidate. No installer registration or auto discovery.
use codex_pencil::local_gemma::{broker::Admission, peer::ImagePin, service::Service, GemmaEngine};
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    os::windows::fs::OpenOptionsExt,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::Instant;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    // Explicit paths/hashes are supplied by the trusted launcher, never a document.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err("gemma_broker_args: runtime runtime-sha model model-sha host host-sha".into());
    }
    let started = Instant::now();
    let startup_deadline = started + Duration::from_secs(180);
    // Fixed phase names/times only: no paths, document text, endpoint or token.
    eprintln!("gemma_boot phase=artifact_pins ms=0");
    let active = Arc::new(AtomicBool::new(true));
    let closing = active.clone();
    // Any control byte or EOF revokes permission. Thread never reads documents.
    std::thread::spawn(move || {
        let mut byte = [0];
        let _ = std::io::stdin().read(&mut byte);
        closing.store(false, Ordering::SeqCst);
    });
    let _runtime = ImagePin::verify(Path::new(&args[0]), &args[1])?;
    let host = ImagePin::verify(Path::new(&args[4]), &args[5])?;
    eprintln!("gemma_boot phase=model_hash ms={}", started.elapsed().as_millis());
    let model = Path::new(&args[2]);
    if !model.is_absolute()
        || args[3].len() != 64
        || !args[3].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("gemma_model_config".into());
    }
    // Streaming hash, retained deny-write/delete handle; no multi-GB allocation.
    let mut model_file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(model)
        .map_err(|_| "gemma_model_open")?;
    if model_file
        .metadata()
        .map_err(|_| "gemma_model_metadata")?
        .len()
        > 32 * 1024 * 1024 * 1024
    {
        return Err("gemma_model_limit".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        if !active.load(Ordering::SeqCst) || Instant::now() >= startup_deadline {
            return Err("gemma_broker_startup_revoked_or_deadline".into());
        }
        let n = model_file
            .read(&mut buffer)
            .map_err(|_| "gemma_model_read")?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != args[3].to_ascii_lowercase() {
        return Err("gemma_model_hash".into());
    }
    if !active.load(Ordering::SeqCst) {
        return Err("gemma_broker_revoked".into());
    }
    eprintln!("gemma_boot phase=model_verified ms={}", started.elapsed().as_millis());
    let engine = GemmaEngine::start_cancellable(
        Path::new(&args[0]), model, startup_deadline, active.clone(),
    ).await?;
    let (service, client) = Service::own(engine);
    let result = async {
        if !active.load(Ordering::SeqCst) {
            return Err("gemma_broker_revoked".into());
        }
        let admission = Admission::create(host)?;
        writeln!(
            std::io::stdout(),
            "{}",
            serde_json::json!({"version":1,"name":admission.name(),"pid":std::process::id()})
        )
        .map_err(|_| "gemma_broker_stdout")?;
        std::io::stdout()
            .flush()
            .map_err(|_| "gemma_broker_stdout")?;
        admission.run(client, active).await
    }
    .await;
    service.close().await?;
    result?;
    writeln!(std::io::stdout(), "{{\"cleanup\":\"complete\"}}")
        .map_err(|_| "gemma_broker_stdout")?;
    Ok(())
}
