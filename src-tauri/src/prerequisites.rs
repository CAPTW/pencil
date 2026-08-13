use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrerequisiteReport {
    pub commands: Vec<CommandCheck>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandCheck {
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub required_at_runtime: bool,
    pub required_for_build: bool,
    pub error: Option<String>,
}

pub fn check() -> PrerequisiteReport {
    PrerequisiteReport {
        commands: vec![
            check_command("codex", true, false),
            check_command("cargo", false, true),
            check_command("rustc", false, true),
        ],
    }
}

fn check_command(name: &str, required_at_runtime: bool, required_for_build: bool) -> CommandCheck {
    let path = find_command_path(name);
    let available = path.is_some();
    let version = path.as_deref().and_then(command_version);
    let error = if required_at_runtime && !available {
        Some("Codex CLI must be installed and available on PATH.".to_string())
    } else {
        None
    };

    CommandCheck {
        name: name.to_string(),
        available,
        version,
        path,
        required_at_runtime,
        required_for_build,
        error,
    }
}

fn command_version(path: &str) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

#[cfg(windows)]
fn find_command_path(name: &str) -> Option<String> {
    let output = Command::new("where.exe").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let candidates = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    candidates
        .iter()
        .find(|path| {
            let lower = path.to_ascii_lowercase();
            lower.ends_with(".cmd") || lower.ends_with(".exe")
        })
        .cloned()
        .or_else(|| candidates.first().cloned())
}

#[cfg(not(windows))]
fn find_command_path(name: &str) -> Option<String> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}
