//! Production runtime seams, synthetic data only. Native tests require isolated TEMP.
use super::*;

mod deep_boundary;
mod endurance;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn operation_cancel_ack_and_stale_finish() {
    let state = AppState::default();
    let old = state
        .providers
        .lock()
        .await
        .reserve(ProviderKind::Codex, "old".into(), true)
        .unwrap();
    let begin = Instant::now();
    let cancelled = cancel_cli_operation(&state).await.unwrap();
    assert!(begin.elapsed() < Duration::from_millis(100));
    assert_eq!(cancelled.id, old.id);
    assert!(state.providers.lock().await.finish(&old));
    let new = state
        .providers
        .lock()
        .await
        .reserve(ProviderKind::Codex, "new".into(), true)
        .unwrap();
    assert!(!state.providers.lock().await.finish(&old));
    old.cancel();
    assert_eq!(
        state.providers.lock().await.cancel_handle().unwrap().id,
        new.id
    );
    assert!(state.providers.lock().await.finish(&new));
}

fn fixture(mode: &str) -> (PathBuf, PathBuf) {
    let exe =
        PathBuf::from(std::env::var("MISSION_FIXTURE_EXE").expect("explicit fixture executable"));
    let root = PathBuf::from(std::env::var("MISSION_EVIDENCE").expect("task-owned evidence"));
    let dir = root.join(format!("{}-{}", mode, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("P01_CASE_DIR", &dir);
    std::env::set_var("P01_FIXTURE_MODE", mode);
    std::env::set_var("P01_FIXTURE_BYTES", "65537");
    (exe, dir)
}

fn assert_exited(dir: &Path) -> usize {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let mut observed_pid_count = 0;
    for name in ["root.pid", "descendant.pid"] {
        if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
            observed_pid_count += 1;
            let pid = text.parse::<u32>().unwrap();
            let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if !handle.is_null() {
                let mut code = 0;
                let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
                unsafe {
                    CloseHandle(handle);
                }
                assert!(ok != 0 && code != 259, "owned PID still active: {pid}");
            }
        }
    }
    observed_pid_count
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_lifecycle_matrix() {
    let mut failures = Vec::new();
    for (mode, expected) in [
        ("success", None),
        ("pipe-holder", Some("provider_timeout")),
        ("stdout-flood", Some("provider_output_limit")),
        ("stderr-flood", Some("provider_output_limit")),
        ("invalid-utf8", Some("provider_invalid_utf8")),
        ("probe-hang", Some("provider_timeout")),
    ] {
        let (exe, dir) = fixture(mode);
        let start = Instant::now();
        let (result, phases) = provider::cli::diagnose_run(provider::cli::run_bounded(
            &exe,
            &[],
            Duration::from_millis(750),
            Arc::new(AtomicBool::new(false)),
            65536,
            65536,
        ))
        .await;
        let elapsed = start.elapsed().as_millis();
        let observed_pid_count = assert_exited(&dir);
        let spawn_observed_pid_count = phases
            .iter()
            .filter(|event| event.phase == "spawn_job_end" && event.pid.is_some())
            .count();
        let base = std::env::temp_dir().join("codex-pencil-runtime-v1");
        let roots: Vec<_> = std::fs::read_dir(&base)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("client-"))
            .collect();
        std::fs::write(dir.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "mode":mode,"elapsed_ms":elapsed,"request_budget_ms":750,"cleanup_budget_ms":5000,
            "error":result.as_ref().err(),"residual_roots":roots.len(),
            "observed_pid_count":observed_pid_count,"fixture_observed_pid_count":observed_pid_count,
            "spawn_observed_pid_count":spawn_observed_pid_count,"owned_pids_exited":if observed_pid_count > 0 { Some(true) } else { None },
            "provider_live":false,"phases":phases
        })).unwrap()).unwrap();
        if result.as_ref().err().map(String::as_str) != expected {
            failures.push(format!(
                "{mode}: expected {expected:?}, got {:?}",
                result.as_ref().err()
            ));
        }
        if elapsed >= 5750 {
            failures.push(format!("{mode}: request plus cleanup budget exceeded"));
        }
        if !roots.is_empty() {
            failures.push(format!("{mode}: residual runtime roots"));
        }
        let expected_pid_count = if mode == "pipe-holder" { 2 } else { 1 };
        if observed_pid_count != expected_pid_count {
            failures.push(format!(
                "{mode}: expected {expected_pid_count} observed PIDs, got {observed_pid_count}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_claude_existing_120s_caller_success() {
    let (exe, dir) = fixture("success");
    std::env::set_var("CODEX_PENCIL_CLAUDE_BIN", exe);
    let state = AppState::default();
    let operation = state
        .providers
        .lock()
        .await
        .reserve(
            ProviderKind::Claude,
            "production-caller-fixture".into(),
            true,
        )
        .unwrap();
    let start = Instant::now();
    // Calls Claude's actual 120-second request policy; this does not replace
    // or change the separate 750ms stress matrix above.
    let (result, phases) = provider::cli::diagnose_run(ProviderManager::execute(
        &operation,
        "Synthetic.",
        RewriteIntent::grammar(),
        &[],
        &state.codex,
    ))
    .await;
    let elapsed = start.elapsed().as_millis();
    assert!(state.providers.lock().await.finish(&operation));
    let observed = assert_exited(&dir);
    let base = std::env::temp_dir().join("codex-pencil-runtime-v1");
    let roots = std::fs::read_dir(&base)
        .unwrap()
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("client-"))
        .count();
    std::fs::write(
        dir.join("caller-receipt.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "classification":"PRODUCTION_CLAUDE_CALLER_SYNTHETIC_CHILD", "request_budget_ms":120000,
            "cleanup_budget_ms":5000,"elapsed_ms":elapsed,"observed_pid_count":observed,
            "residual_roots":roots,"provider_live":false,"phases":phases,
            "error":result.as_ref().err().map(|error| error.code())
        }))
        .unwrap(),
    )
    .unwrap();
    std::env::remove_var("CODEX_PENCIL_CLAUDE_BIN");
    assert_eq!(observed, 1);
    assert_eq!(roots, 0);
    let result = result.unwrap();
    assert_eq!(result.provider_used, ProviderKind::Claude);
    assert_eq!(result.replacement, "Synthetic.");
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_cancel_during_codex_version_probe() {
    let (exe, dir) = fixture("probe-hang");
    std::env::set_var("CODEX_PENCIL_CODEX_BIN", exe);
    let cache = Arc::new(CodexClientCache::default());
    let work_cache = cache.clone();
    let work = tokio::spawn(async move { work_cache.get().await });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !dir.join("root.pid").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let start = Instant::now();
    cache.shutdown_checked().await.unwrap();
    assert!(work.await.unwrap().is_err());
    assert_exited(&dir);
    assert!(start.elapsed() < Duration::from_secs(5));
    std::env::remove_var("CODEX_PENCIL_CODEX_BIN");
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_manager_cancel_releases_busy_after_cleanup() {
    let (exe, dir) = fixture("slow");
    std::env::set_var("CODEX_PENCIL_CLAUDE_BIN", exe);
    let state = Arc::new(AppState::default());
    let worker = state.clone();
    let work = tokio::spawn(async move {
        let operation = worker
            .providers
            .lock()
            .await
            .reserve(ProviderKind::Claude, "fixture".into(), true)
            .unwrap();
        let result = ProviderManager::execute(
            &operation,
            "Synthetic.",
            RewriteIntent::grammar(),
            &[],
            &worker.codex,
        )
        .await;
        assert!(worker.providers.lock().await.finish(&operation));
        result
    });
    tokio::time::timeout(Duration::from_secs(3), async {
        while !dir.join("root.pid").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let start = Instant::now();
    cancel_cli_operation(&state).await.unwrap();
    let dispatch = start.elapsed().as_millis();
    assert!(dispatch <= 100);
    assert!(matches!(
        work.await.unwrap(),
        Err(provider::types::ProviderError::Cancelled)
    ));
    assert_exited(&dir);
    assert!(state.providers.lock().await.snapshot().busy_kind.is_none());
    assert!(start.elapsed() < Duration::from_secs(5));
    std::fs::write(dir.join("cancel-receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "dispatch_ms":dispatch,"completed_cleanup_ms":start.elapsed().as_millis(),"manager_idle":true,"owned_pids_exited":true
    })).unwrap()).unwrap();
    std::env::remove_var("CODEX_PENCIL_CLAUDE_BIN");
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_job_assignment_failure_runs_no_child_code() {
    let (exe, dir) = fixture("success");
    let mut command = tokio::process::Command::new(exe);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    assert!(
        process_job::ProcessJob::spawn_assignment_failure(&mut command)
            .await
            .is_err()
    );
    assert!(!dir.join("root.pid").exists());
    let pid = process_job::ProcessJob::test_spawn_pid();
    std::fs::write(dir.join("root.pid"), pid.to_string()).unwrap();
    assert_exited(&dir);
}

#[test]
#[ignore = "task-owned TEMP required"]
fn native_unreleased_file_reports_cleanup_failure() {
    use std::os::windows::fs::OpenOptionsExt;
    let workspace = runtime_isolation::RuntimeWorkspace::create().unwrap();
    let root = workspace.cwd().parent().unwrap().to_owned();
    let held = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .share_mode(0)
        .open(workspace.cwd().join("synthetic-held-file"))
        .unwrap();
    let start = Instant::now();
    let result = workspace.close_before(start + Duration::from_millis(50));
    assert!(result.is_err());
    assert!(root.exists());
    drop(held);
    assert!(root.starts_with(std::env::temp_dir().join("codex-pencil-runtime-v1")));
    std::fs::remove_dir_all(&root).unwrap();
    assert!(!root.exists());
}

#[tokio::test]
async fn stale_cancel_does_not_target_new_valid_capture() {
    let state = AppState::default();
    let intent = capture_session::BoundRewriteIntent::without_terminology(RewriteIntent::grammar());
    let token = {
        let mut capture = state.capture.lock().await;
        let token = capture
            .capture(
                "current".into(),
                "Synthetic.".into(),
                capture_session::WindowTarget::new(1, 1),
            )
            .unwrap();
        capture.begin_rewrite_bound(&token, intent.clone()).unwrap();
        token
    };
    let op = state
        .providers
        .lock()
        .await
        .reserve_capture(ProviderKind::Codex, token.clone(), intent)
        .unwrap();
    assert!(cancel_cli_operation(&state).await.is_none());
    state.capture.lock().await.cancel_active_with_turn();
    assert_eq!(cancel_cli_operation(&state).await.unwrap().id, op.id);
    assert!(state.providers.lock().await.finish(&op));
}
