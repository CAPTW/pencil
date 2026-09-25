use std::{mem::size_of, ptr};

use tokio::process::{Child, Command};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
    System::Threading::{
        GetProcessIdOfThread, OpenThread, ResumeThread, CREATE_SUSPENDED,
        THREAD_QUERY_LIMITED_INFORMATION, THREAD_SUSPEND_RESUME,
    },
};

// Toolhelp's documented ABI, declared locally to preserve the frozen feature set.
#[repr(C)]
struct ThreadEntry {
    size: u32,
    usage: u32,
    thread_id: u32,
    process_id: u32,
    base_priority: i32,
    delta_priority: i32,
    flags: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateToolhelp32Snapshot(flags: u32, process: u32) -> HANDLE;
    fn Thread32First(snapshot: HANDLE, entry: *mut ThreadEntry) -> i32;
    fn Thread32Next(snapshot: HANDLE, entry: *mut ThreadEntry) -> i32;
}

struct OwnedHandle(HANDLE);
#[cfg(test)]
static TEST_SPAWN_PID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn resume_owned_primary(child: &Child) -> Result<(), String> {
    let pid = child.id().ok_or("provider_process_id_unavailable")?;
    let snapshot = unsafe { CreateToolhelp32Snapshot(4, 0) }; // TH32CS_SNAPTHREAD
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(last_error("provider_thread_snapshot_failed"));
    }
    let snapshot = OwnedHandle(snapshot);
    let mut entry: ThreadEntry = unsafe { std::mem::zeroed() };
    entry.size = size_of::<ThreadEntry>() as u32;
    let mut found = None;
    let mut available = unsafe { Thread32First(snapshot.0, &mut entry) };
    while available != 0 {
        if entry.process_id == pid {
            if found.replace(entry.thread_id).is_some() {
                return Err("provider_ambiguous_suspended_threads".into());
            }
        }
        entry.size = size_of::<ThreadEntry>() as u32;
        available = unsafe { Thread32Next(snapshot.0, &mut entry) };
    }
    if unsafe { GetLastError() } != 18 {
        return Err(last_error("provider_thread_enumeration_failed"));
    }
    let tid = found.ok_or("provider_primary_thread_missing")?;
    let thread = unsafe {
        OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_QUERY_LIMITED_INFORMATION,
            0,
            tid,
        )
    };
    if thread.is_null() {
        return Err(last_error("provider_primary_thread_open_failed"));
    }
    let thread = OwnedHandle(thread);
    if unsafe { GetProcessIdOfThread(thread.0) } != pid {
        return Err("provider_primary_thread_owner_mismatch".into());
    }
    if unsafe { ResumeThread(thread.0) } != 1 {
        return Err("provider_primary_thread_resume_failed".into());
    }
    Ok(())
}

/// Owns a Windows Job Object containing one Codex launcher and every descendant
/// it creates. The exact supported CLI may resolve to an npm `.cmd` wrapper, so
/// terminating only the immediate child can otherwise orphan Node/native Codex
/// descendants and leave the isolated runtime directory locked.
pub(crate) struct ProcessJob {
    handle: HANDLE,
}

// Windows kernel handles may be used and closed from a different thread than
// the one that created them. `ProcessJob` retains sole ownership of the handle.
unsafe impl Send for ProcessJob {}
// Shared operations only query or terminate the owned kernel Job.
unsafe impl Sync for ProcessJob {}

impl ProcessJob {
    pub(crate) async fn spawn(command: &mut Command) -> Result<(Child, Self), String> {
        Self::spawn_before(
            command,
            tokio::time::Instant::now() + std::time::Duration::from_secs(3),
        )
        .await
    }

    pub(crate) async fn spawn_before(
        command: &mut Command,
        deadline: tokio::time::Instant,
    ) -> Result<(Child, Self), String> {
        Self::spawn_checked(command, false, deadline).await
    }

    async fn spawn_checked(
        command: &mut Command,
        inject_assignment_failure: bool,
        deadline: tokio::time::Instant,
    ) -> Result<(Child, Self), String> {
        if tokio::time::Instant::now() >= deadline {
            return Err("provider_spawn_deadline".into());
        }
        command.creation_flags(CREATE_SUSPENDED).kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|_| "provider_spawn_failed".to_string())?;
        #[cfg(test)]
        TEST_SPAWN_PID.store(child.id().unwrap_or(0), std::sync::atomic::Ordering::SeqCst);
        let assignment = if inject_assignment_failure {
            Err("provider_job_assignment_injected".to_string())
        } else {
            Self::assign(&child)
        };
        match assignment {
            Ok(job) => {
                if let Err(error) = resume_owned_primary(&child) {
                    let _ = job.terminate();
                    let _ = child.start_kill();
                    if !matches!(
                        tokio::time::timeout_at(deadline, child.wait()).await,
                        Ok(Ok(_))
                    ) {
                        return Err(format!("{error};cleanup=provider_spawn_cleanup_timeout"));
                    }
                    return Err(error);
                }
                Ok((child, job))
            }
            Err(error) => {
                let _ = child.start_kill();
                if !matches!(
                    tokio::time::timeout_at(deadline, child.wait()).await,
                    Ok(Ok(_))
                ) {
                    return Err(format!("{error};cleanup=provider_spawn_cleanup_timeout"));
                }
                Err(error)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn test_spawn_pid() -> u32 {
        TEST_SPAWN_PID.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) async fn spawn_assignment_failure(
        command: &mut Command,
    ) -> Result<(Child, Self), String> {
        Self::spawn_checked(
            command,
            true,
            tokio::time::Instant::now() + std::time::Duration::from_secs(3),
        )
        .await
    }

    pub(crate) fn active_processes(&self) -> Result<u32, String> {
        let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            QueryInformationJobObject(
                self.handle,
                JobObjectBasicAccountingInformation,
                (&raw mut info).cast(),
                size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(last_error("provider_job_query_failed"));
        }
        Ok(info.ActiveProcesses)
    }

    pub(crate) fn assign(child: &Child) -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(last_error("app_server_job_create_failed"));
        }

        let job = Self { handle };
        let mut information: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        let configured = unsafe {
            SetInformationJobObject(
                job.handle,
                JobObjectExtendedLimitInformation,
                (&raw const information).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if configured == 0 {
            return Err(last_error("app_server_job_configure_failed"));
        }

        let Some(process_handle) = child.raw_handle() else {
            return Err("app_server_process_handle_unavailable".to_string());
        };
        let assigned = unsafe { AssignProcessToJobObject(job.handle, process_handle.cast()) };
        if assigned == 0 {
            return Err(last_error("app_server_job_assignment_failed"));
        }

        Ok(job)
    }

    pub(crate) fn terminate(&self) -> Result<(), String> {
        if unsafe { TerminateJobObject(self.handle, 1) } == 0 {
            return Err(last_error("app_server_job_termination_failed"));
        }
        Ok(())
    }
}

impl Drop for ProcessJob {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}

fn last_error(prefix: &str) -> String {
    let code = match std::io::Error::last_os_error().raw_os_error() {
        Some(code) => code,
        None => -1,
    };
    format!("{prefix}:os_error={code}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, process::Stdio, time::Duration};
    use tokio::process::Command;

    #[tokio::test]
    async fn terminating_job_releases_descendant_runtime_directory() {
        let root =
            std::env::temp_dir().join(format!("codex-pencil-job-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).expect("synthetic test directory must be created");

        let mut command = Command::new("cmd.exe");
        command
            .args(["/D", "/S", "/C", "ping -n 60 127.0.0.1 >nul"])
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn().expect("synthetic process tree must start");
        let job = ProcessJob::assign(&child).expect("synthetic process tree must join its job");

        tokio::time::sleep(Duration::from_millis(250)).await;
        job.terminate().expect("job termination must succeed");
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("synthetic launcher termination must be bounded")
            .expect("synthetic launcher must be waitable");
        drop(child);
        drop(job);

        fs::remove_dir_all(&root).expect("terminated descendants must release their runtime cwd");
        assert!(!root.exists());
    }
}
