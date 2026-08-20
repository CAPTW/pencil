use std::{mem::size_of, ptr};

use tokio::process::Child;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    },
};

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

impl ProcessJob {
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
