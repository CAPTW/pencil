use crate::runtime_isolation::RuntimeWorkspace;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, sync::Mutex, time::timeout};

pub(crate) struct CapturedProcess {
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) cancelled: bool,
}

pub(crate) struct ActiveCliProcess {
    cancel: Arc<AtomicBool>,
}

impl ActiveCliProcess {
    pub(crate) fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

pub(crate) fn resolve_named_executable(explicit: Option<&str>, names: &[&str]) -> Option<PathBuf> {
    if let Some(explicit) = explicit {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Some(path);
        }
    }
    for name in names {
        if let Some(path) = which(name) {
            return Some(path);
        }
    }
    None
}

fn which(name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("where.exe")
        .arg(name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

pub(crate) async fn run_version(path: &Path) -> Result<String, String> {
    let output = timeout(
        Duration::from_secs(8),
        Command::new(path)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| "provider_version_timeout".to_string())?
    .map_err(|error| error.to_string())?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(err
            .lines()
            .next()
            .unwrap_or("provider_version_unavailable")
            .to_string());
    }
    Ok(line.to_string())
}

pub(crate) async fn run_args(
    path: &Path,
    args: &[String],
    limit: Duration,
) -> Result<CapturedProcess, String> {
    let output = timeout(
        limit,
        Command::new(path)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_remove("ANTHROPIC_API_KEY")
            .env_remove("GEMINI_API_KEY")
            .env_remove("GOOGLE_API_KEY")
            .env_remove("OPENAI_API_KEY")
            .output(),
    )
    .await
    .map_err(|_| "provider_probe_timeout".to_string())?
    .map_err(|error| error.to_string())?;
    Ok(CapturedProcess {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        cancelled: false,
    })
}

pub(crate) async fn run_writing(
    path: &Path,
    args: &[String],
    limit: Duration,
    cancel: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<ActiveCliProcess>>>,
) -> Result<CapturedProcess, String> {
    let workspace = RuntimeWorkspace::create()?;
    let mut command = Command::new(path);
    command
        .args(args)
        .current_dir(workspace.cwd())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .env_remove("GOOGLE_API_KEY")
        .env_remove("OPENAI_API_KEY");
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start Provider process: {error}"))?;
    #[cfg(windows)]
    let job = crate::process_job::ProcessJob::assign(&child).ok();
    {
        *slot.lock().await = Some(ActiveCliProcess {
            cancel: cancel.clone(),
        });
    }
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(reader) = stdout_pipe.as_mut() {
            let _ = reader.read_to_end(&mut buf).await;
        }
        buf
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(reader) = stderr_pipe.as_mut() {
            let _ = reader.read_to_end(&mut buf).await;
        }
        buf
    });

    let started = tokio::time::Instant::now();
    let outcome = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.start_kill();
            #[cfg(windows)]
            if let Some(job) = &job {
                let _ = job.terminate();
            }
            let _ = child.wait().await;
            break Ok(CapturedProcess {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                cancelled: true,
            });
        }
        if started.elapsed() >= limit {
            let _ = child.start_kill();
            #[cfg(windows)]
            if let Some(job) = &job {
                let _ = job.terminate();
            }
            let _ = child.wait().await;
            break Err("provider_timeout".to_string());
        }
        match timeout(Duration::from_millis(50), child.wait()).await {
            Ok(Ok(status)) => {
                let stdout =
                    String::from_utf8_lossy(&stdout_task.await.unwrap_or_default()).into_owned();
                let stderr =
                    String::from_utf8_lossy(&stderr_task.await.unwrap_or_default()).into_owned();
                break Ok(CapturedProcess {
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    cancelled: false,
                });
            }
            Ok(Err(error)) => break Err(error.to_string()),
            Err(_) => {}
        }
    };
    *slot.lock().await = None;
    drop(workspace);
    outcome
}
