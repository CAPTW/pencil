//! Step 5: repetition and resource-accumulation checks for the bounded Provider
//! runtime, plus a loaded reproduction of the historical 750ms timeout. Only the
//! task-owned synthetic fixture runs; no Provider, network or account is used.
use super::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessHandleCount};

const ROUNDS: usize = 8;
const REQUEST_BUDGET: Duration = Duration::from_millis(750);
/// Request budget plus the single shared teardown budget.
const REQUEST_PLUS_CLEANUP_MS: u128 = 5750;
const CONTENTION_ROUNDS: usize = 20;
const CONTENDERS: usize = 8;
const LOADED_RUNS: usize = 6;
const HANDLE_GROWTH_LIMIT: u32 = 64;
const THREAD_GROWTH_LIMIT: u32 = 8;
const PRIVATE_GROWTH_LIMIT: usize = 32 * 1024 * 1024;

// Documented Toolhelp and PSAPI ABI, declared locally to keep the feature set frozen.
#[repr(C)]
struct ProcessEntry {
    size: u32,
    usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    threads: u32,
    parent_process_id: u32,
    priority_class_base: i32,
    flags: u32,
    exe_file: [u16; 260],
}

#[repr(C)]
#[derive(Default)]
struct MemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set: usize,
    working_set: usize,
    quota_peak_paged_pool: usize,
    quota_paged_pool: usize,
    quota_peak_nonpaged_pool: usize,
    quota_nonpaged_pool: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry) -> i32;
    fn K32GetProcessMemoryInfo(process: *mut c_void, counters: *mut MemoryCounters, size: u32) -> i32;
}

/// Content-free resource counters of this test process.
#[derive(Clone, Copy, Debug, serde::Serialize)]
struct Resources {
    handles: u32,
    threads: u32,
    private_bytes: usize,
    child_processes: u32,
    runtime_roots: usize,
}

fn runtime_roots() -> usize {
    std::fs::read_dir(std::env::temp_dir().join("codex-pencil-runtime-v1"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("client-"))
                .count()
        })
        .unwrap_or(0)
}

fn resources() -> Resources {
    let me = std::process::id();
    let process = unsafe { GetCurrentProcess() };
    let mut handles = 0;
    unsafe {
        GetProcessHandleCount(process, &mut handles);
    }
    let mut memory = MemoryCounters {
        cb: std::mem::size_of::<MemoryCounters>() as u32,
        ..Default::default()
    };
    unsafe {
        K32GetProcessMemoryInfo(process, &mut memory, memory.cb);
    }
    let (mut threads, mut child_processes) = (0, 0);
    let snapshot = unsafe { CreateToolhelp32Snapshot(2, 0) }; // TH32CS_SNAPPROCESS
    if snapshot != INVALID_HANDLE_VALUE {
        let mut entry: ProcessEntry = unsafe { std::mem::zeroed() };
        entry.size = std::mem::size_of::<ProcessEntry>() as u32;
        let mut available = unsafe { Process32FirstW(snapshot, &mut entry) };
        while available != 0 {
            if entry.process_id == me {
                threads = entry.threads;
            }
            if entry.parent_process_id == me {
                child_processes += 1;
            }
            entry.size = std::mem::size_of::<ProcessEntry>() as u32;
            available = unsafe { Process32NextW(snapshot, &mut entry) };
        }
        unsafe {
            CloseHandle(snapshot);
        }
    }
    Resources {
        handles,
        threads,
        private_bytes: memory.private_usage,
        child_processes,
        runtime_roots: runtime_roots(),
    }
}

/// Lets idle blocking-pool threads (10s keep-alive) retire before measuring,
/// then keeps the lower of two samples taken two seconds apart.
async fn settled_resources() -> Resources {
    tokio::time::sleep(Duration::from_secs(12)).await;
    let first = resources();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let second = resources();
    if (second.threads, second.handles) <= (first.threads, first.handles) {
        second
    } else {
        first
    }
}

/// Fixture PIDs recorded by the child that are still running (none expected).
fn active_fixture_pids(dir: &Path) -> Vec<u32> {
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    ["root.pid", "descendant.pid"]
        .into_iter()
        .filter_map(|name| std::fs::read_to_string(dir.join(name)).ok()?.parse::<u32>().ok())
        .filter(|pid| {
            let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, *pid) };
            if handle.is_null() {
                return false;
            }
            let mut code = 0;
            let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
            unsafe {
                CloseHandle(handle);
            }
            ok != 0 && code == 259 // STILL_ACTIVE
        })
        .collect()
}

struct Case {
    mode: &'static str,
    expected: Result<bool, &'static str>, // Ok(cancelled) or the error code
    cancel_after_spawn: bool,
}

const CASES: &[Case] = &[
    Case { mode: "success", expected: Ok(false), cancel_after_spawn: false },
    Case { mode: "slow", expected: Ok(true), cancel_after_spawn: true },
    Case { mode: "pipe-holder", expected: Err("provider_timeout"), cancel_after_spawn: false },
    Case { mode: "early-descendant", expected: Err("provider_timeout"), cancel_after_spawn: false },
    Case { mode: "cleanup-lock", expected: Err("provider_timeout"), cancel_after_spawn: false },
    Case { mode: "stdout-flood", expected: Err("provider_output_limit"), cancel_after_spawn: false },
    Case { mode: "stderr-flood", expected: Err("provider_output_limit"), cancel_after_spawn: false },
    Case { mode: "dual-flood", expected: Err("provider_output_limit"), cancel_after_spawn: false },
    Case { mode: "invalid-utf8", expected: Err("provider_invalid_utf8"), cancel_after_spawn: false },
];

#[derive(serde::Serialize)]
struct RunRecord {
    mode: &'static str,
    outcome: String,
    elapsed_ms: u128,
    active_pids_after: usize,
    residual_roots_after: usize,
    reader_drain_or_drop: bool,
    job_zero: bool,
    filesystem_cleanup: bool,
}

fn phase_seen(phases: &[provider::cli::RuntimePhase], name: &str) -> bool {
    phases.iter().any(|phase| phase.phase == name)
}

fn phase_span_ms(phases: &[provider::cli::RuntimePhase], begin: &str, end: &str) -> Option<f64> {
    let start = phases.iter().find(|phase| phase.phase == begin)?.elapsed_us;
    let stop = phases.iter().find(|phase| phase.phase == end)?.elapsed_us;
    Some(stop.saturating_sub(start) as f64 / 1000.0)
}

async fn run_case(case: &Case) -> (RunRecord, Vec<provider::cli::RuntimePhase>) {
    let (exe, dir) = fixture(case.mode);
    let cancel = Arc::new(AtomicBool::new(false));
    let canceller = case.cancel_after_spawn.then(|| {
        let cancel = cancel.clone();
        let dir = dir.clone();
        tokio::spawn(async move {
            let started = Instant::now();
            while !dir.join("root.pid").exists() && started.elapsed() < Duration::from_secs(3) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            cancel.store(true, Ordering::SeqCst);
        })
    });
    let start = Instant::now();
    let (result, phases) = provider::cli::diagnose_run(provider::cli::run_bounded(
        &exe,
        &[],
        REQUEST_BUDGET,
        cancel,
        65536,
        65536,
    ))
    .await;
    let elapsed_ms = start.elapsed().as_millis();
    if let Some(canceller) = canceller {
        let _ = canceller.await;
    }
    let outcome = match &result {
        Ok(captured) if captured.cancelled => "cancelled".to_string(),
        Ok(_) => "completed".to_string(),
        Err(error) => error.clone(),
    };
    let record = RunRecord {
        mode: case.mode,
        outcome,
        elapsed_ms,
        active_pids_after: active_fixture_pids(&dir).len(),
        residual_roots_after: runtime_roots(),
        reader_drain_or_drop: phase_seen(&phases, "io_complete")
            || phase_seen(&phases, "timeout_during_io")
            || phase_seen(&phases, "cancel_during_io")
            || (phase_seen(&phases, "stdout_read_end") && phase_seen(&phases, "stderr_read_end")),
        job_zero: phase_seen(&phases, "cleanup_job_zero"),
        filesystem_cleanup: phase_seen(&phases, "cleanup_filesystem_end"),
    };
    (record, phases)
}

fn case_failures(case: &Case, record: &RunRecord) -> Vec<String> {
    let mut failures = Vec::new();
    let expected = match case.expected {
        Ok(true) => "cancelled",
        Ok(false) => "completed",
        Err(code) => code,
    };
    if record.outcome != expected {
        failures.push(format!("{}: expected {expected}, got {}", case.mode, record.outcome));
    }
    if record.elapsed_ms >= REQUEST_PLUS_CLEANUP_MS {
        failures.push(format!("{}: request plus cleanup budget exceeded", case.mode));
    }
    if record.active_pids_after != 0 {
        failures.push(format!("{}: owned fixture process still active", case.mode));
    }
    if record.residual_roots_after != 0 {
        failures.push(format!("{}: residual runtime workspace", case.mode));
    }
    if !record.job_zero || !record.filesystem_cleanup || !record.reader_drain_or_drop {
        failures.push(format!("{}: teardown phase evidence missing", case.mode));
    }
    failures
}

async fn reservation_contention() -> Vec<String> {
    let mut failures = Vec::new();
    let state = Arc::new(AppState::default());
    for round in 0..CONTENTION_ROUNDS {
        let attempts = (0..CONTENDERS)
            .map(|index| {
                let state = state.clone();
                tokio::spawn(async move {
                    state
                        .providers
                        .lock()
                        .await
                        .reserve(ProviderKind::Claude, format!("contender-{index}"), true)
                })
            })
            .collect::<Vec<_>>();
        let mut granted = Vec::new();
        let mut busy = 0;
        for attempt in attempts {
            match attempt.await.expect("contender task") {
                Ok(operation) => granted.push(operation),
                Err(provider::types::ProviderError::Busy) => busy += 1,
                Err(other) => failures.push(format!("round {round}: unexpected {other:?}")),
            }
        }
        if granted.len() != 1 || busy != CONTENDERS - 1 {
            failures.push(format!("round {round}: granted {} busy {busy}", granted.len()));
        }
        let cancelled = cancel_cli_operation(&state).await;
        if cancelled.as_ref().map(|operation| operation.id) != granted.first().map(|operation| operation.id) {
            failures.push(format!("round {round}: cancel targeted the wrong reservation"));
        }
        for operation in &granted {
            if !state.providers.lock().await.finish(operation) {
                failures.push(format!("round {round}: finish rejected the owner"));
            }
        }
        if state.providers.lock().await.snapshot().busy_kind.is_some() {
            failures.push(format!("round {round}: manager stayed busy"));
        }
    }
    failures
}

#[tokio::test]
#[ignore = "task-owned native fixture and TEMP required, serial invocation"]
async fn native_runtime_endurance() {
    let evidence = PathBuf::from(std::env::var("MISSION_EVIDENCE").expect("task-owned evidence"));
    let mut failures = Vec::new();
    let mut records = Vec::new();

    // Warm-up round: lazily created runtime state is not a leak.
    for case in CASES {
        let (record, _) = run_case(case).await;
        failures.extend(case_failures(case, &record));
    }
    let baseline = settled_resources().await;
    let mut trend = vec![baseline];
    for round in 0..ROUNDS {
        for case in CASES {
            let (record, _) = run_case(case).await;
            failures.extend(case_failures(case, &record).into_iter().map(|failure| format!("round {round}: {failure}")));
            records.push(record);
        }
        trend.push(resources());
    }
    failures.extend(reservation_contention().await);

    // Loaded reproduction of the historical 1937ms failure: the unchanged 750ms
    // success request while every CPU is saturated. A timeout here is recorded
    // as a reproduction, never as a pass; cleanup must stay complete either way.
    let stop = Arc::new(AtomicBool::new(false));
    let spinners = (0..std::thread::available_parallelism().map_or(4, |n| n.get()) * 2)
        .map(|_| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut value = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    value = std::hint::black_box(value.wrapping_mul(6364136223846793005).wrapping_add(1));
                }
            })
        })
        .collect::<Vec<_>>();
    let mut loaded = Vec::new();
    for _ in 0..LOADED_RUNS {
        let (record, phases) = run_case(&CASES[0]).await;
        let spans = serde_json::json!({
            "workspace_ms": phase_span_ms(&phases, "workspace_create_begin", "workspace_create_end"),
            "spawn_job_ms": phase_span_ms(&phases, "spawn_job_begin", "spawn_job_end"),
            "io_ms": phase_span_ms(&phases, "io_begin", "io_complete")
                .or_else(|| phase_span_ms(&phases, "io_begin", "timeout_during_io")),
            "cleanup_ms": phase_span_ms(&phases, "cleanup_begin", "cleanup_end"),
            "timeout_phase": phases.iter().find(|phase| phase.phase.starts_with("timeout_")).map(|phase| phase.phase),
        });
        if record.elapsed_ms >= REQUEST_PLUS_CLEANUP_MS || record.active_pids_after != 0 || record.residual_roots_after != 0 {
            failures.push(format!("loaded run: incomplete cleanup ({})", record.outcome));
        }
        loaded.push(serde_json::json!({"record": record, "phases": spans}));
    }
    stop.store(true, Ordering::Relaxed);
    for spinner in spinners {
        let _ = spinner.join();
    }
    let reproduced = loaded
        .iter()
        .filter(|run| run["record"]["outcome"] == "provider_timeout")
        .count();

    let after = settled_resources().await;
    if after.child_processes != 0 || after.runtime_roots != 0 {
        failures.push(format!("residual children {} / roots {}", after.child_processes, after.runtime_roots));
    }
    if after.handles > baseline.handles + HANDLE_GROWTH_LIMIT {
        failures.push(format!("handle growth {} -> {}", baseline.handles, after.handles));
    }
    if after.threads > baseline.threads + THREAD_GROWTH_LIMIT {
        failures.push(format!("thread growth {} -> {}", baseline.threads, after.threads));
    }
    if after.private_bytes > baseline.private_bytes + PRIVATE_GROWTH_LIMIT {
        failures.push(format!("private memory growth {} -> {}", baseline.private_bytes, after.private_bytes));
    }

    let classification = if !failures.is_empty() {
        "FAIL"
    } else if reproduced > 0 {
        "PASS_WITH_LOADED_TIMEOUT_REPRODUCED"
    } else {
        "PASS_LOADED_TIMEOUT_NOT_REPRODUCED"
    };
    let receipt = serde_json::json!({
        "classification": classification,
        "provider_live": false,
        "rounds": ROUNDS,
        "cases_per_round": CASES.len(),
        "request_budget_ms": REQUEST_BUDGET.as_millis(),
        "request_plus_cleanup_ms": REQUEST_PLUS_CLEANUP_MS,
        "reservation_contention": {"rounds": CONTENTION_ROUNDS, "contenders": CONTENDERS},
        "resources": {"baseline": baseline, "after": after, "trend": trend},
        "loaded_reproduction": {
            "historical_failure": "final-checkpoint 750ms success fixture returned provider_timeout after 1937ms during a concurrent release build",
            "spinning_threads": std::thread::available_parallelism().map_or(4, |n| n.get()) * 2,
            "runs": loaded,
            "timeouts_reproduced": reproduced,
        },
        "runs": records,
        "failures": failures,
    });
    std::fs::write(
        evidence.join("endurance-receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    eprintln!("ENDURANCE_RESOURCES baseline={baseline:?} after={after:?}");
    eprintln!("ENDURANCE_LOADED timeouts_reproduced={reproduced}/{LOADED_RUNS} runs={}", serde_json::to_string(&receipt["loaded_reproduction"]["runs"]).unwrap());
    eprintln!("ENDURANCE_RESULT {classification} runs={} failures={}", records.len(), failures.len());
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
