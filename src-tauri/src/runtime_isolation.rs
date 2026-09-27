use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const BASE_NAME: &str = "codex-pencil-runtime-v1";
const BASE_MARKER: &str = ".codex-pencil-runtime-root-v1";
const SESSION_MARKER: &str = ".codex-pencil-runtime-owner-v1";
const SESSION_PREFIX: &str = "client-";
const STALE_AFTER: Duration = Duration::from_secs(60);
const MAX_STARTUP_CLEANUPS: usize = 32;
const REMOVE_RETRIES: usize = 8;
const CLEANUP_BUDGET: Duration = Duration::from_secs(2);
const REMOVE_RETRY_DELAY: Duration = Duration::from_millis(50);

pub(crate) struct RuntimeWorkspace {
    session_root: PathBuf,
    cwd: PathBuf,
}

impl RuntimeWorkspace {
    pub(crate) fn create() -> Result<Self, String> {
        Self::create_under(&std::env::temp_dir())
    }

    fn create_under(temp_root: &Path) -> Result<Self, String> {
        let base = temp_root.join(BASE_NAME);
        prepare_base(&base)?;
        cleanup_stale_sessions(&base, unix_now(), process_is_alive)?;

        let session_root = base.join(format!("{SESSION_PREFIX}{}", Uuid::new_v4()));
        fs::create_dir(&session_root)
            .map_err(|_| "runtime_session_directory_create_failed".to_string())?;
        let marker = session_root.join(SESSION_MARKER);
        if let Err(error) = write_session_marker(&marker, std::process::id(), unix_now()) {
            let _ = fs::remove_dir(&session_root);
            return Err(error);
        }
        let cwd = session_root.join("cwd");
        if fs::create_dir(&cwd).is_err() {
            let _ = remove_owned_session(&session_root);
            return Err("runtime_cwd_create_failed".to_string());
        }

        Ok(Self { session_root, cwd })
    }

    pub(crate) fn cwd(&self) -> &Path {
        &self.cwd
    }

    // Persistent app-server workspaces have no deadline from creation. The owner
    // supplies the absolute teardown deadline when shutdown actually begins.
    pub(crate) fn close(self) -> Result<(), String> {
        self.close_before(Instant::now() + CLEANUP_BUDGET)
    }

    pub(crate) fn close_before(mut self, deadline: Instant) -> Result<(), String> {
        let result = remove_owned_session_before(&self.session_root, deadline);
        self.session_root.clear();
        self.cwd.clear();
        result
    }
}

impl Drop for RuntimeWorkspace {
    fn drop(&mut self) {
        if !self.session_root.as_os_str().is_empty() {
            let _ = remove_owned_session(&self.session_root);
        }
    }
}

fn prepare_base(base: &Path) -> Result<(), String> {
    fs::create_dir_all(base).map_err(|_| "runtime_root_create_failed".to_string())?;
    let metadata =
        fs::symlink_metadata(base).map_err(|_| "runtime_root_metadata_failed".to_string())?;
    if !metadata.file_type().is_dir() || is_directory_link(&metadata) {
        return Err("runtime_root_not_owned".to_string());
    }
    let marker = base.join(BASE_MARKER);
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&marker)
    {
        Ok(mut file) => file
            .write_all(b"codex-pencil-runtime-root-v1\n")
            .map_err(|_| "runtime_root_marker_write_failed".to_string()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let value = fs::read_to_string(&marker)
                .map_err(|_| "runtime_root_marker_read_failed".to_string())?;
            if value == "codex-pencil-runtime-root-v1\n" {
                Ok(())
            } else {
                Err("runtime_root_not_owned".to_string())
            }
        }
        Err(_) => Err("runtime_root_marker_create_failed".to_string()),
    }
}

fn write_session_marker(path: &Path, pid: u32, created: u64) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "runtime_session_marker_create_failed".to_string())?;
    write!(
        file,
        "codex-pencil-runtime-session-v1\npid={pid}\ncreated={created}\n"
    )
    .map_err(|_| "runtime_session_marker_write_failed".to_string())?;
    file.sync_all()
        .map_err(|_| "runtime_session_marker_sync_failed".to_string())
}

fn read_session_marker(path: &Path) -> Option<(u32, u64)> {
    let text = fs::read_to_string(path).ok()?;
    let mut lines = text.lines();
    if lines.next()? != "codex-pencil-runtime-session-v1" {
        return None;
    }
    let pid = lines.next()?.strip_prefix("pid=")?.parse().ok()?;
    let created = lines.next()?.strip_prefix("created=")?.parse().ok()?;
    if lines.next().is_some() {
        return None;
    }
    Some((pid, created))
}

fn cleanup_stale_sessions<F>(base: &Path, now: u64, mut alive: F) -> Result<(), String>
where
    F: FnMut(u32) -> bool,
{
    let entries = fs::read_dir(base).map_err(|_| "runtime_root_read_failed".to_string())?;
    let mut removed = 0usize;
    let deadline = Instant::now() + Duration::from_millis(250);
    for entry in entries.flatten() {
        if removed >= MAX_STARTUP_CLEANUPS || Instant::now() >= deadline {
            break;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !name.starts_with(SESSION_PREFIX)
            || !metadata.file_type().is_dir()
            || is_directory_link(&metadata)
        {
            continue;
        }
        let Some((pid, created)) = read_session_marker(&path.join(SESSION_MARKER)) else {
            continue;
        };
        let stale = now.saturating_sub(created) >= STALE_AFTER.as_secs();
        if stale && !alive(pid) && remove_owned_session_before(&path, deadline).is_ok() {
            removed += 1;
        }
    }
    Ok(())
}

fn remove_owned_session(path: &Path) -> Result<(), String> {
    remove_owned_session_before(path, Instant::now() + CLEANUP_BUDGET)
}

fn remove_owned_session_before(path: &Path, deadline: Instant) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "runtime_session_metadata_failed".to_string())?;
    if !metadata.file_type().is_dir() || is_directory_link(&metadata) {
        return Err("runtime_session_not_owned".to_string());
    }
    let marker = path.join(SESSION_MARKER);
    let Some((pid, created)) = read_session_marker(&marker) else {
        return Err("runtime_session_not_owned".to_string());
    };

    // A partial remove_dir_all can delete the marker first. A folder left
    // without it would never be recognised as ours again, so every path that
    // gives up puts the marker back for a later stale-session cleanup.
    let keep_ownership = || {
        if path.exists() && !marker.exists() {
            let _ = write_session_marker(&marker, pid, created);
        }
    };
    let mut last_os_error = 0;
    for attempt in 0..REMOVE_RETRIES {
        if Instant::now() >= deadline {
            keep_ownership();
            return Err("runtime_cleanup_deadline".into());
        }
        match fs::remove_dir_all(path) {
            Ok(()) => {
                return if Instant::now() <= deadline {
                    Ok(())
                } else {
                    Err("runtime_cleanup_completed_late".into())
                }
            }
            Err(error)
                if attempt + 1 < REMOVE_RETRIES
                    && matches!(error.raw_os_error(), Some(32 | 145)) =>
            {
                last_os_error = error.raw_os_error().unwrap_or(0);
                std::thread::sleep(
                    REMOVE_RETRY_DELAY.min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(error) => {
                last_os_error = error.raw_os_error().unwrap_or(last_os_error);
                keep_ownership();
                return Err(format!(
                    "runtime_session_cleanup_failed:os_error={last_os_error}"
                ));
            }
        }
    }
    keep_ownership();
    Err(format!(
        "runtime_session_cleanup_failed:os_error={last_os_error}"
    ))
}

#[cfg(windows)]
fn is_directory_link(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_directory_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, STILL_ACTIVE},
        System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return false;
    }
    let mut exit_code = 0u32;
    let ok = unsafe { GetExitCodeProcess(process, &mut exit_code) } != 0;
    unsafe {
        CloseHandle(process);
    }
    ok && exit_code == STILL_ACTIVE as u32
}

#[cfg(not(windows))]
fn process_is_alive(pid: u32) -> bool {
    pid == std::process::id()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "codex-pencil-runtime-test-{label}-{}",
            Uuid::new_v4()
        ))
    }

    #[test]
    fn workspace_cwd_is_empty_owned_and_removed_on_drop() {
        let root = test_root("lifecycle");
        fs::create_dir(&root).expect("test root should be created");
        let workspace = RuntimeWorkspace::create_under(&root)
            .expect("owned runtime workspace should be created");
        let cwd = workspace.cwd().to_path_buf();
        let session = cwd
            .parent()
            .expect("cwd must have an owned session parent")
            .to_path_buf();

        assert!(cwd.starts_with(root.join(BASE_NAME)));
        assert!(fs::read_dir(&cwd)
            .expect("runtime cwd should be readable")
            .next()
            .is_none());
        assert!(session.join(SESSION_MARKER).is_file());

        drop(workspace);
        assert!(!session.exists());
        assert!(root.join(BASE_NAME).join(BASE_MARKER).is_file());
        fs::remove_dir_all(root).expect("owned test root should be removed");
    }

    #[test]
    fn startup_cleanup_removes_only_stale_marked_dead_sessions() {
        let root = test_root("cleanup");
        let base = root.join(BASE_NAME);
        prepare_base(&base).expect("owned base should be prepared");

        let stale = base.join("client-stale");
        fs::create_dir(&stale).expect("stale fixture should be created");
        write_session_marker(&stale.join(SESSION_MARKER), 4242, 1)
            .expect("stale marker should be written");

        let active = base.join("client-active");
        fs::create_dir(&active).expect("active fixture should be created");
        write_session_marker(&active.join(SESSION_MARKER), 4343, 1)
            .expect("active marker should be written");

        let unowned = base.join("client-unowned");
        fs::create_dir(&unowned).expect("unowned fixture should be created");
        fs::write(unowned.join("sentinel"), b"synthetic")
            .expect("unowned sentinel should be written");

        cleanup_stale_sessions(&base, 120, |pid| pid == 4343)
            .expect("bounded cleanup should succeed");
        assert!(!stale.exists());
        assert!(active.exists());
        assert!(unowned.join("sentinel").is_file());

        remove_owned_session(&active).expect("owned active fixture should clean up");
        fs::remove_dir_all(root).expect("owned test root should be removed");
    }

    #[test]
    fn wrong_root_marker_and_unmarked_session_are_never_deleted() {
        let root = test_root("ownership");
        let base = root.join(BASE_NAME);
        fs::create_dir_all(&base).expect("base fixture should be created");
        fs::write(base.join(BASE_MARKER), b"not-owned\n")
            .expect("wrong marker fixture should be written");
        assert_eq!(
            prepare_base(&base),
            Err("runtime_root_not_owned".to_string())
        );

        let unmarked = base.join("client-unmarked");
        fs::create_dir(&unmarked).expect("unmarked fixture should be created");
        assert_eq!(
            remove_owned_session(&unmarked),
            Err("runtime_session_not_owned".to_string())
        );
        assert!(unmarked.exists());
        fs::remove_dir_all(root).expect("owned test root should be removed");
    }
}
