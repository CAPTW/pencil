//! Cross-process exclusion for the same trusted owner-root namespace.
//! The fixed empty lock file contains no documents, tokens, PID or endpoint.
use std::{
    fs::{File, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::Path,
};

pub(super) struct Lease {
    _file: File,
}
impl Lease {
    pub(super) fn acquire(root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|_| "gemma_owner_root")?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(root.join("gemma-owner.lock"))
            .map_err(|_| "gemma_owner_unavailable")?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "owned child of cross_process_exclusion_and_release only"]
    fn lease_child() {
        let path = std::env::var("GRAMMAR_LEASE_TEST_ROOT").unwrap();
        let expectation = std::env::var("GRAMMAR_LEASE_EXPECT").unwrap();
        let lease = Lease::acquire(Path::new(&path));
        assert_eq!(lease.is_ok(), expectation != "blocked");
        if expectation == "exit-without-drop" {
            std::process::exit(0);
        }
    }
    #[test]
    fn cross_process_exclusion_and_release() {
        let root =
            std::env::temp_dir().join(format!("grammar-lease-test-{}", uuid::Uuid::new_v4()));
        let child = |expect: &str| {
            let mut process = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--ignored",
                    "--exact",
                    "local_gemma::lease::tests::lease_child",
                ])
                .env("GRAMMAR_LEASE_TEST_ROOT", &root)
                .env("GRAMMAR_LEASE_EXPECT", expect)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Some(status) = process.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                if std::time::Instant::now() >= end {
                    let _ = process.kill();
                    let _ = process.wait();
                    panic!("owned lease child timeout");
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        let lease = Lease::acquire(&root).unwrap();
        assert!(Lease::acquire(&root).is_err());
        child("blocked");
        drop(lease);
        child("available");
        child("exit-without-drop");
        drop(Lease::acquire(&root).unwrap());
        assert_eq!(
            std::fs::metadata(root.join("gemma-owner.lock"))
                .unwrap()
                .len(),
            0
        );
        std::fs::remove_file(root.join("gemma-owner.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
