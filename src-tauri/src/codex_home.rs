use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

const APP_DIRECTORY: &str = "com.local.codexpencil";
const HOME_DIRECTORY: &str = "codex-home-v1";
const OWNER_MARKER: &str = ".codex-pencil-home-owner-v1";
const OWNER_MARKER_CONTENT: &[u8] = b"codex-pencil-codex-home-v1\n";

/// A persistent, app-owned Codex home used only for Codex Pencil authentication
/// and Codex runtime state. Keeping this separate from the user's general
/// `$CODEX_HOME` prevents personal MCP, plugin, skill, hook, and project settings
/// from becoming available to a writing turn. Codex itself remains the sole
/// reader/writer of its credential payloads.
pub(crate) struct CodexHome {
    path: PathBuf,
}

impl CodexHome {
    pub(crate) fn prepare() -> Result<Self, String> {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| "codex_home_local_app_data_unavailable".to_string())?;
        if !local_app_data.is_absolute() {
            return Err("codex_home_local_app_data_not_absolute".to_string());
        }
        Self::prepare_under(&local_app_data)
    }

    fn prepare_under(local_app_data: &Path) -> Result<Self, String> {
        let app_directory = local_app_data.join(APP_DIRECTORY);
        fs::create_dir_all(&app_directory)
            .map_err(|_| "codex_home_parent_create_failed".to_string())?;
        ensure_plain_directory(&app_directory, "codex_home_parent_not_owned")?;

        let path = app_directory.join(HOME_DIRECTORY);
        fs::create_dir_all(&path).map_err(|_| "codex_home_create_failed".to_string())?;
        ensure_plain_directory(&path, "codex_home_not_owned")?;
        ensure_owner_marker(&path.join(OWNER_MARKER))?;

        // The exact 0.144.6 app-server has no `--ignore-user-config` flag.
        // Refuse an app-home config rather than allowing an MCP/tool config to
        // merge underneath command-line restrictions.
        if path.join("config.toml").exists() {
            return Err("codex_home_configuration_not_isolated".to_string());
        }

        Ok(Self { path })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

fn ensure_owner_marker(path: &Path) -> Result<(), String> {
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut marker) => {
            marker
                .write_all(OWNER_MARKER_CONTENT)
                .map_err(|_| "codex_home_marker_write_failed".to_string())?;
            marker
                .sync_all()
                .map_err(|_| "codex_home_marker_sync_failed".to_string())
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| "codex_home_marker_metadata_failed".to_string())?;
            if !metadata.file_type().is_file() || is_link(&metadata) {
                return Err("codex_home_marker_not_owned".to_string());
            }
            let content =
                fs::read(path).map_err(|_| "codex_home_marker_read_failed".to_string())?;
            if content == OWNER_MARKER_CONTENT {
                Ok(())
            } else {
                Err("codex_home_marker_not_owned".to_string())
            }
        }
        Err(_) => Err("codex_home_marker_create_failed".to_string()),
    }
}

fn ensure_plain_directory(path: &Path, error: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| error.to_string())?;
    if metadata.file_type().is_dir() && !is_link(&metadata) {
        Ok(())
    } else {
        Err(error.to_string())
    }
}

#[cfg(windows)]
fn is_link(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temporary_parent() -> PathBuf {
        std::env::temp_dir().join(format!("codex-pencil-home-test-{}", Uuid::new_v4()))
    }

    #[test]
    fn creates_owned_isolated_home_and_reopens_it() {
        let parent = temporary_parent();
        let first = CodexHome::prepare_under(&parent).expect("owned test home must be created");
        assert!(first.path().join(OWNER_MARKER).is_file());
        assert!(!first.path().join("config.toml").exists());
        let second = CodexHome::prepare_under(&parent).expect("owned test home must reopen");
        assert_eq!(first.path(), second.path());
        fs::remove_dir_all(parent).expect("owned test home must be removable");
    }

    #[test]
    fn rejects_any_user_configuration_in_the_app_owned_home() {
        let parent = temporary_parent();
        let home = CodexHome::prepare_under(&parent).expect("owned test home must be created");
        fs::write(
            home.path().join("config.toml"),
            b"[mcp_servers.synthetic]\n",
        )
        .expect("synthetic config fixture must be written");
        assert_eq!(
            CodexHome::prepare_under(&parent).err().as_deref(),
            Some("codex_home_configuration_not_isolated")
        );
        fs::remove_dir_all(parent).expect("owned test home must be removable");
    }

    #[test]
    fn rejects_a_foreign_marker() {
        let parent = temporary_parent();
        let home = CodexHome::prepare_under(&parent).expect("owned test home must be created");
        fs::write(home.path().join(OWNER_MARKER), b"foreign\n")
            .expect("synthetic foreign marker must be written");
        assert_eq!(
            CodexHome::prepare_under(&parent).err().as_deref(),
            Some("codex_home_marker_not_owned")
        );
        fs::remove_dir_all(parent).expect("owned test home must be removable");
    }
}
