use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RewriteMode {
    Grammar,
    Natural,
    Concise,
    Polite,
    TranslateEn,
    TranslateKo,
}

impl Default for RewriteMode {
    fn default() -> Self {
        Self::Grammar
    }
}

impl RewriteMode {
    pub fn instruction(self) -> &'static str {
        match self {
            Self::Grammar => "Correct grammar, spelling, punctuation, and obvious wording issues while preserving the original tone and meaning.",
            Self::Natural => "Make the text sound natural, fluent, and native while preserving the original meaning.",
            Self::Concise => "Make the text shorter and clearer. Remove redundancy without losing important meaning.",
            Self::Polite => "Make the text courteous, professional, and warm while preserving the original intent.",
            Self::TranslateEn => "Translate the text into natural English. Preserve names, numbers, code, URLs, and formatting where appropriate.",
            Self::TranslateKo => "Translate the text into natural Korean. Preserve names, numbers, code, URLs, and formatting where appropriate.",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Grammar => "grammar",
            Self::Natural => "natural",
            Self::Concise => "concise",
            Self::Polite => "polite",
            Self::TranslateEn => "translate_en",
            Self::TranslateKo => "translate_ko",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub mode: RewriteMode,
    pub restore_clipboard: bool,
    pub auto_rewrite: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mode: RewriteMode::Grammar,
            restore_clipboard: true,
            auto_rewrite: true,
        }
    }
}

pub fn load(app: &AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let text = fs::read_to_string(&path).map_err(|error| format!("Could not read settings: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("Could not parse settings: {error}"))
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("Could not create settings directory: {error}"))?;
    }

    let text = serde_json::to_string_pretty(settings).map_err(|error| format!("Could not serialize settings: {error}"))?;
    fs::write(path, text).map_err(|error| format!("Could not write settings: {error}"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Could not resolve app config directory: {error}"))?;
    Ok(dir.join("settings.json"))
}
