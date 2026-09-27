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
use tokio::{io::AsyncReadExt, process::Command, sync::Mutex};

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

pub(crate) fn executable_candidates(name: &str) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        directories.push(current);
    }
    if let Some(path) = std::env::var_os("PATH") {
        directories.extend(std::env::split_paths(&path));
    }
    let extensions: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .map(str::to_owned)
        .collect();
    let mut found = Vec::new();
    for directory in directories {
        let plain = directory.join(name);
        if plain.is_file() {
            found.push(plain);
        }
        if Path::new(name).extension().is_none() {
            for extension in &extensions {
                let path = directory.join(format!("{name}{extension}"));
                if path.is_file() {
                    found.push(path);
                }
            }
        }
    }
    found
}

fn which(name: &str) -> Option<PathBuf> {
    executable_candidates(name).into_iter().next()
}

/// Rust refuses to pass an argument with a line break to a .cmd or .bat
/// launcher (an injection guard: "batch file arguments are invalid"), and every
/// writing request has line breaks. Such a launcher answers the status probe
/// but can never run a request, so it is reported as unavailable up front
/// instead of failing every request at spawn.
pub(crate) fn batch_launcher_reason(path: &Path, cli: &str) -> Option<String> {
    let batch = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat"));
    batch.then(|| {
        format!(
            "{cli} was found only as a batch launcher ({}), which cannot receive a multi-line request on Windows.",
            path.file_name().and_then(|name| name.to_str()).unwrap_or("script")
        )
    })
}

static CLEANUP_BLOCKED: AtomicBool = AtomicBool::new(false);
pub(crate) fn cleanup_status() -> Result<(), &'static str> {
    if CLEANUP_BLOCKED.load(Ordering::SeqCst) {
        Err("provider_cleanup_unresolved_restart_required")
    } else {
        Ok(())
    }
}

pub(crate) const CLEANUP_BUDGET: Duration = Duration::from_secs(5);

pub(crate) const STDOUT_LIMIT: usize = 16 * 1024 * 1024;
pub(crate) const STDERR_LIMIT: usize = 1024 * 1024;
pub(crate) const PROBE_LIMIT: usize = 64 * 1024;

pub(crate) async fn run_version(path: &Path) -> Result<String, String> {
    run_version_cancel(path, Arc::new(AtomicBool::new(false))).await
}

pub(crate) async fn run_version_cancel(
    path: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let output =
        run_args_cancel(path, &["--version".into()], Duration::from_secs(8), cancel).await?;
    if output.cancelled {
        return Err("provider_cancelled".into());
    }
    if output.exit_code != Some(0) {
        return Err("provider_version_failed".into());
    }
    let line = output.stdout.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        Err("provider_version_unavailable".into())
    } else {
        Ok(line.into())
    }
}

pub(crate) async fn run_args(
    path: &Path,
    args: &[String],
    limit: Duration,
) -> Result<CapturedProcess, String> {
    run_args_cancel(path, args, limit, Arc::new(AtomicBool::new(false))).await
}

pub(crate) async fn run_args_cancel(
    path: &Path,
    args: &[String],
    limit: Duration,
    cancel: Arc<AtomicBool>,
) -> Result<CapturedProcess, String> {
    run_bounded(path, args, limit, cancel, PROBE_LIMIT, PROBE_LIMIT).await
}

pub(crate) fn version_sync(path: &Path) -> Result<String, String> {
    let path = path.to_owned();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "provider_probe_runtime_failed".to_string())?;
        runtime.block_on(run_version(&path))
    })
    .join()
    .map_err(|_| "provider_probe_thread_failed".to_string())?
}

const WINDOWS_COMMAND_LINE_LIMIT: usize = 32_000;

pub(crate) fn command_line_too_long(path: &Path, args: &[String]) -> bool {
    let mut size = quoted_arg(&path.to_string_lossy()).len();
    for arg in args {
        size = size.saturating_add(1).saturating_add(quoted_arg(arg).len());
        if size > WINDOWS_COMMAND_LINE_LIMIT {
            return true;
        }
    }
    false
}

fn quoted_arg(value: &str) -> String {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| matches!(byte, b' ' | b'\t' | b'"'))
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

pub(crate) async fn run_writing(
    path: &Path,
    args: &[String],
    limit: Duration,
    cancel: Arc<AtomicBool>,
    slot: Arc<Mutex<Option<ActiveCliProcess>>>,
) -> Result<CapturedProcess, String> {
    *slot.lock().await = Some(ActiveCliProcess {
        cancel: cancel.clone(),
    });
    let result = run_bounded(path, args, limit, cancel, STDOUT_LIMIT, STDERR_LIMIT).await;
    *slot.lock().await = None;
    result
}

pub(crate) async fn read_bounded<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader
            .read(&mut buffer)
            .await
            .map_err(|_| "provider_output_read_failed".to_string())?;
        if count == 0 {
            return Ok(output);
        }
        if count > limit.saturating_sub(output.len()) {
            return Err("provider_output_limit".into());
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

pub(crate) async fn cancellation(cancel: &AtomicBool) {
    while !cancel.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(test)]
#[derive(Clone, serde::Serialize)]
pub(crate) struct RuntimePhase {
    pub(crate) phase: &'static str,
    pub(crate) elapsed_us: u128,
    pub(crate) pid: Option<u32>,
}

#[cfg(test)]
tokio::task_local! {
    static RUNTIME_PHASES: (std::time::Instant, std::sync::Arc<std::sync::Mutex<Vec<RuntimePhase>>>);
}

// Content-free and allocation-free in non-test builds. A trace belongs to one
// scoped future, so parallel requests cannot steal or mix each other's events.
fn record_phase(phase: &'static str, pid: Option<u32>) {
    #[cfg(test)]
    let _ = RUNTIME_PHASES.try_with(|(started, phases)| {
        phases.lock().unwrap().push(RuntimePhase {
            phase,
            elapsed_us: started.elapsed().as_micros(),
            pid,
        });
    });
    #[cfg(not(test))]
    let _ = (phase, pid);
}

#[cfg(test)]
pub(crate) async fn diagnose_run<F: std::future::Future>(
    future: F,
) -> (F::Output, Vec<RuntimePhase>) {
    let phases = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let result = RUNTIME_PHASES
        .scope((std::time::Instant::now(), phases.clone()), future)
        .await;
    let receipt = phases.lock().unwrap().clone();
    (result, receipt)
}

fn request_expired(now: tokio::time::Instant, deadline: tokio::time::Instant) -> bool {
    now >= deadline
}

fn teardown_deadline(
    now: tokio::time::Instant,
    overall_deadline: tokio::time::Instant,
) -> tokio::time::Instant {
    overall_deadline.min(now + CLEANUP_BUDGET)
}

pub(crate) async fn run_bounded(
    path: &Path,
    args: &[String],
    limit: Duration,
    cancel: Arc<AtomicBool>,
    stdout_limit: usize,
    stderr_limit: usize,
) -> Result<CapturedProcess, String> {
    if limit.is_zero() {
        return Err("provider_lifetime_budget_too_small".into());
    }
    if CLEANUP_BLOCKED.load(Ordering::SeqCst) {
        return Err("provider_cleanup_unresolved_restart_required".into());
    }
    record_phase("request_start", None);
    let execution_deadline = tokio::time::Instant::now() + limit;
    let deadline = execution_deadline + CLEANUP_BUDGET;
    if cancel.load(Ordering::SeqCst) {
        return Ok(CapturedProcess {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            cancelled: true,
        });
    }
    record_phase("workspace_create_begin", None);
    let workspace_result = RuntimeWorkspace::create();
    record_phase(
        if workspace_result.is_ok() {
            "workspace_create_end"
        } else {
            "workspace_create_failed"
        },
        None,
    );
    let workspace = workspace_result?;
    if request_expired(tokio::time::Instant::now(), execution_deadline)
        || cancel.load(Ordering::SeqCst)
    {
        record_phase(
            if cancel.load(Ordering::SeqCst) {
                "cancel_before_spawn"
            } else {
                "timeout_before_spawn"
            },
            None,
        );
        record_phase("cleanup_filesystem_begin", None);
        let cleanup = workspace
            .close_before(teardown_deadline(tokio::time::Instant::now(), deadline).into_std());
        record_phase(
            if cleanup.is_ok() {
                "cleanup_filesystem_end"
            } else {
                "cleanup_filesystem_failed"
            },
            None,
        );
        cleanup.map_err(|error| {
            CLEANUP_BLOCKED.store(true, Ordering::SeqCst);
            error
        })?;
        return if cancel.load(Ordering::SeqCst) {
            Ok(CapturedProcess {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                cancelled: true,
            })
        } else {
            Err("provider_timeout".into())
        };
    }
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
    record_phase("spawn_job_begin", None);
    let (mut child, job) =
        match crate::process_job::ProcessJob::spawn_before(&mut command, deadline).await {
            Ok(value) => {
                record_phase("spawn_job_end", value.0.id());
                value
            }
            Err(error) => {
                record_phase("spawn_job_failed", None);
                record_phase("cleanup_filesystem_begin", None);
                let cleanup = workspace.close_before(deadline.into_std());
                record_phase(
                    if cleanup.is_ok() {
                        "cleanup_filesystem_end"
                    } else {
                        "cleanup_filesystem_failed"
                    },
                    None,
                );
                if let Err(cleanup) = cleanup {
                    CLEANUP_BLOCKED.store(true, Ordering::SeqCst);
                    return Err(format!("{error};cleanup={cleanup}"));
                }
                if error.contains("cleanup") {
                    CLEANUP_BLOCKED.store(true, Ordering::SeqCst);
                }
                return Err(error);
            }
        };
    let stdout = child.stdout.take().ok_or("provider_stdout_missing")?;
    let stderr = child.stderr.take().ok_or("provider_stderr_missing")?;
    record_phase("io_begin", None);
    let outcome = {
        let io = async {
            let wait = async {
                let result = child
                    .wait()
                    .await
                    .map_err(|_| "provider_wait_failed".to_string());
                record_phase("child_wait_end", None);
                result
            };
            let out = async {
                let result = read_bounded(stdout, stdout_limit).await;
                record_phase("stdout_read_end", None);
                result
            };
            let err = async {
                let result = read_bounded(stderr, stderr_limit).await;
                record_phase("stderr_read_end", None);
                result
            };
            let (out, err, status) = tokio::try_join!(out, err, wait)?;
            Ok::<_, String>(CapturedProcess {
                exit_code: status.code(),
                stdout: String::from_utf8(out).map_err(|_| "provider_invalid_utf8".to_string())?,
                stderr: String::from_utf8(err).map_err(|_| "provider_invalid_utf8".to_string())?,
                cancelled: false,
            })
        };
        tokio::select! {
            biased;
            _ = cancellation(&cancel) => { record_phase("cancel_during_io", None); Ok(CapturedProcess { exit_code: None, stdout: String::new(), stderr: String::new(), cancelled: true }) },
            result = tokio::time::timeout_at(execution_deadline, io) => match result {
                Ok(value) => { record_phase("io_complete", None); value },
                Err(_) => { record_phase("timeout_during_io", None); Err("provider_timeout".into()) },
            },
        }
    }; // All pipe futures are dropped here, including on overflow/cancel/timeout.
       // One teardown deadline starts once, after normal completion/cancel/timeout.
       // No retry, reader, process, or filesystem phase restarts this budget.
    record_phase("cleanup_begin", None);
    let cleanup_deadline = teardown_deadline(tokio::time::Instant::now(), deadline);
    let cleanup = async {
        job.terminate()?;
        let _ = child.start_kill();
        tokio::time::timeout_at(cleanup_deadline, child.wait())
            .await
            .map_err(|_| "provider_cleanup_timeout".to_string())?
            .map_err(|_| "provider_reap_failed".to_string())?;
        while job.active_processes()? != 0 {
            if tokio::time::Instant::now() >= cleanup_deadline {
                return Err("provider_subtree_cleanup_timeout".into());
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        record_phase("cleanup_job_zero", None);
        Ok::<(), String>(())
    }
    .await;
    drop(child);
    drop(job);
    record_phase("cleanup_filesystem_begin", None);
    let workspace_cleanup = workspace.close_before(cleanup_deadline.into_std());
    record_phase(
        if workspace_cleanup.is_ok() {
            "cleanup_filesystem_end"
        } else {
            "cleanup_filesystem_failed"
        },
        None,
    );
    let cleanup = cleanup.and(workspace_cleanup);
    record_phase("cleanup_end", None);
    if let Err(cleanup_error) = cleanup {
        CLEANUP_BLOCKED.store(true, Ordering::SeqCst);
        let cause = match &outcome {
            Err(error) => error.as_str(),
            Ok(value) if value.cancelled => "provider_cancelled",
            _ => "provider_completed",
        };
        return Err(format!("{cause};cleanup={cleanup_error}"));
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_budget_expires_at_boundary_without_deadline_restart() {
        let start = tokio::time::Instant::now();
        let deadline = start + Duration::from_millis(750);
        assert!(!request_expired(
            deadline - Duration::from_nanos(1),
            deadline
        ));
        assert!(request_expired(deadline, deadline));
        assert!(request_expired(
            deadline + Duration::from_millis(1187),
            deadline
        ));
        let overall = deadline + CLEANUP_BUDGET;
        let late_cleanup_start = deadline + Duration::from_secs(2);
        assert_eq!(teardown_deadline(late_cleanup_start, overall), overall);
    }

    #[test]
    fn overdue_workspace_creation_cannot_renew_cleanup_budget() {
        let start = tokio::time::Instant::now();
        let request_deadline = start + Duration::from_millis(750);
        let overall = request_deadline + CLEANUP_BUDGET;
        let overdue_create_end = overall + Duration::from_millis(500);
        assert!(request_expired(overdue_create_end, request_deadline));
        let cleanup = teardown_deadline(overdue_create_end, overall);
        assert_eq!(cleanup, overall);
        assert!(cleanup < overdue_create_end);
    }

    #[tokio::test]
    async fn bounded_reader_accepts_exact_limit_and_rejects_one_extra() {
        assert_eq!(read_bounded(&b"1234"[..], 4).await.unwrap(), b"1234");
        assert_eq!(
            read_bounded(&b"12345"[..], 4).await.unwrap_err(),
            "provider_output_limit"
        );
    }

    #[tokio::test]
    #[ignore = "synthetic Windows process; task-owned TEMP required"]
    async fn native_pipe_timeout_and_cancel_cleanup() {
        let cmd = Path::new("C:/Windows/System32/cmd.exe");
        let begin = std::time::Instant::now();
        let result = run_bounded(
            cmd,
            &["/D".into(), "/C".into(), "ping -n 60 127.0.0.1 >nul".into()],
            Duration::from_millis(150),
            Arc::new(AtomicBool::new(false)),
            1024,
            1024,
        )
        .await;
        assert_eq!(result.err().as_deref(), Some("provider_timeout"));
        assert!(begin.elapsed() < Duration::from_millis(5150));
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            signal.store(true, Ordering::SeqCst);
        });
        let result = run_bounded(
            cmd,
            &["/D".into(), "/C".into(), "ping -n 60 127.0.0.1 >nul".into()],
            Duration::from_secs(30),
            cancel,
            1024,
            1024,
        )
        .await
        .unwrap();
        task.await.unwrap();
        assert!(result.cancelled);
        assert!(!CLEANUP_BLOCKED.load(Ordering::SeqCst));
    }

    #[test]
    fn command_line_limit_rejects_oversized_argument() {
        let path = PathBuf::from("agy.exe");
        assert!(!command_line_too_long(
            &path,
            &["-p".to_string(), "short".to_string()]
        ));
        assert!(command_line_too_long(&path, &["x".repeat(40_000)]));
    }
}
