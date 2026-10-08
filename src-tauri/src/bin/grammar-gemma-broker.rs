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
    let _model_file=pin_model(model,&args[3],&active,startup_deadline)?;
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

struct ModelPinTimings {
    started: Instant, read_us:u128, hash_us:u128, bytes:u64, verified:bool,
}
impl ModelPinTimings {
    fn new()->Self {Self{started:Instant::now(),read_us:0,hash_us:0,bytes:0,verified:false}}
}
impl Drop for ModelPinTimings {
    fn drop(&mut self) {
        if std::env::var("GRAMMAR_GEMMA_TIMING").as_deref()==Ok("1") {
            eprintln!("gemma_model_pin elapsed_ms={} read_us={} hash_us={} bytes={} verified={}",
                self.started.elapsed().as_millis(),self.read_us,self.hash_us,self.bytes,self.verified);
        }
    }
}
// Returned handle retains the original deny-write/delete protection through engine lifetime.
fn pin_model(model:&Path,expected:&str,active:&AtomicBool,deadline:Instant)->Result<std::fs::File,String> {
    if !model.is_absolute()
        || expected.len() != 64
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
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
    let mut timings=ModelPinTimings::new();
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        if !active.load(Ordering::SeqCst) || Instant::now() >= deadline {
            return Err("gemma_broker_startup_revoked_or_deadline".into());
        }
        let before=Instant::now();
        let n = model_file
            .read(&mut buffer)
            .map_err(|_| "gemma_model_read")?;
        timings.read_us+=before.elapsed().as_micros();
        if n == 0 {
            break;
        }
        let before=Instant::now();
        hash.update(&buffer[..n]);
        timings.hash_us+=before.elapsed().as_micros();
        timings.bytes+=n as u64;
    }
    if format!("{:x}", hash.finalize()) != expected.to_ascii_lowercase() {
        return Err("gemma_model_hash".into());
    }
    if !active.load(Ordering::SeqCst) {
        return Err("gemma_broker_revoked".into());
    }
    timings.verified=true;
    Ok(model_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_pin_rejects_wrong_hash_revocation_and_deadline_and_retains_lock() {
        let path=std::env::temp_dir().join(format!("grammar-pin-{}.bin",uuid::Uuid::new_v4()));
        let mut created=OpenOptions::new().write(true).create_new(true).open(&path).unwrap();
        created.write_all(b"owned synthetic pin").unwrap();drop(created);
        let expected=format!("{:x}",Sha256::digest(b"owned synthetic pin"));
        let active=AtomicBool::new(true);
        assert_eq!(pin_model(&path,&"0".repeat(64),&active,Instant::now()+Duration::from_secs(5)).err().as_deref(),Some("gemma_model_hash"));
        assert_eq!(pin_model(&path,&expected,&active,Instant::now()).err().as_deref(),Some("gemma_broker_startup_revoked_or_deadline"));
        active.store(false,Ordering::SeqCst);
        assert_eq!(pin_model(&path,&expected,&active,Instant::now()+Duration::from_secs(5)).err().as_deref(),Some("gemma_broker_startup_revoked_or_deadline"));
        active.store(true,Ordering::SeqCst);
        let pin=pin_model(&path,&expected,&active,Instant::now()+Duration::from_secs(5)).unwrap();
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::remove_file(&path).is_err());
        drop(pin);std::fs::remove_file(path).unwrap();
    }
    #[test]
    #[ignore="owned model hash profile only; no GPU or inference"]
    fn owned_model_pin_profile() {
        assert_eq!(std::env::var("GRAMMAR_GEMMA_TRIAL").as_deref(),Ok("owned-synthetic"));
        let root=Path::new("D:/dev/grammar-evidence/grammar-gemma-20261007");
        let label=std::env::var("GRAMMAR_GEMMA_REWRITE_LABEL").unwrap();
        assert!(!label.is_empty()&&label.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'));
        let receipt=root.join(format!("model-pin-{label}.json"));assert!(!receipt.exists());
        let storage=std::env::var("GRAMMAR_GEMMA_MODEL_STORAGE").unwrap_or_else(|_|"evidence".into());
        let model_root=match storage.as_str() {"evidence"=>root,"ssd"=>Path::new("C:/Users/USER/.codex/grammar-owned-models/20261008-io-trial"),_=>panic!("unknown owned storage")};
        let mut rows=Vec::new();
        for (name,sha) in [
            ("gemma-4-12b-it-qat-q4_0.gguf","93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b"),
            ("gemma-4-E4B_q4_0-it.gguf","676c35070db6dbe52f93e9c864ee0fba4eddea94b9c875d9cb10daff453fbaee")] {
            let before=Instant::now();
            let result=pin_model(&model_root.join(name),sha,&AtomicBool::new(true),before+Duration::from_secs(180));
            rows.push(serde_json::json!({"storage":storage,"model":name,"sha256":sha,"elapsedMs":before.elapsed().as_secs_f64()*1000.,"verified":result.is_ok(),"error":result.as_ref().err()}));
            std::fs::write(&receipt,serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
            assert!(result.is_ok());drop(result);
        }
    }
}
