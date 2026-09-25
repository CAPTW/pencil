use std::{
    env,
    path::{Path, PathBuf},
};

pub const SUPPORTED_CODEX_VERSION: &str = "codex-cli 0.144.6";

pub fn resolve_supported_codex() -> Result<PathBuf, String> {
    let path = resolve_codex_executable().ok_or_else(|| {
        "Codex CLI is required at runtime. Install it and ensure `codex.cmd` or `codex.exe` is available on PATH, or set `CODEX_PENCIL_CODEX_BIN`.".to_string()
    })?;
    let version = read_codex_version(&path)?;

    if !is_supported_codex_version(&version) {
        return Err(format!(
            "Unsupported Codex CLI version `{version}`. Codex Pencil currently supports exactly `{SUPPORTED_CODEX_VERSION}`."
        ));
    }

    Ok(path)
}

pub fn resolve_codex_executable() -> Option<PathBuf> {
    let explicit = env::var("CODEX_PENCIL_CODEX_BIN").ok();

    #[cfg(windows)]
    {
        let cmd_candidates = where_candidates("codex.cmd");
        let exe_candidates = where_candidates("codex.exe");
        select_codex_candidate(explicit.as_deref(), &cmd_candidates, &exe_candidates)
    }

    #[cfg(not(windows))]
    {
        let candidates = which_candidates("codex");
        select_codex_candidate(explicit.as_deref(), &candidates, &[])
    }
}

pub fn read_codex_version(path: &Path) -> Result<String, String> {
    crate::provider::cli::version_sync(path)
}

pub fn is_supported_codex_version(version: &str) -> bool {
    version == SUPPORTED_CODEX_VERSION
}

fn select_codex_candidate(
    explicit: Option<&str>,
    cmd_candidates: &[PathBuf],
    exe_candidates: &[PathBuf],
) -> Option<PathBuf> {
    explicit
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| cmd_candidates.first().cloned())
        .or_else(|| exe_candidates.first().cloned())
}

#[cfg(windows)]
fn where_candidates(name: &str) -> Vec<PathBuf> {
    crate::provider::cli::executable_candidates(name)
}

#[cfg(not(windows))]
fn which_candidates(name: &str) -> Vec<PathBuf> {
    let Ok(output) = std::process::Command::new("which").arg(name).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn explicit_override_wins_over_all_path_candidates() {
        let selected = select_codex_candidate(
            Some("C:\\tools\\pinned-codex.cmd"),
            &[PathBuf::from("C:\\path\\codex.cmd")],
            &[PathBuf::from("C:\\desktop\\codex.exe")],
        );

        assert_eq!(selected, Some(PathBuf::from("C:\\tools\\pinned-codex.cmd")));
    }

    #[test]
    fn blank_override_is_ignored_and_cmd_precedes_exe() {
        let selected = select_codex_candidate(
            Some("   "),
            &[PathBuf::from("C:\\path\\codex.cmd")],
            &[PathBuf::from("C:\\path\\codex.exe")],
        );

        assert_eq!(selected, Some(PathBuf::from("C:\\path\\codex.cmd")));
    }

    #[test]
    fn exe_is_used_only_when_no_cmd_candidate_exists() {
        let selected = select_codex_candidate(None, &[], &[PathBuf::from("C:\\path\\codex.exe")]);

        assert_eq!(selected, Some(PathBuf::from("C:\\path\\codex.exe")));
    }

    #[test]
    fn only_the_schema_authority_version_is_supported() {
        assert!(is_supported_codex_version("codex-cli 0.144.6"));
        assert!(!is_supported_codex_version("codex-cli 0.141.0"));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires the exact locally installed Codex CLI"]
    fn current_environment_resolves_the_supported_cli() {
        let path = resolve_supported_codex().expect("supported Codex CLI must resolve");
        let version = read_codex_version(&path).expect("resolved Codex CLI must report a version");

        assert_eq!(version, SUPPORTED_CODEX_VERSION);
    }
}
