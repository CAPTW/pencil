use crate::{
    shortcut::PrimaryShortcut,
    translation::{RewriteIntent, TranslationApplyFormat, TranslationTargetLanguage},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RewriteMode {
    Grammar,
    Natural,
    Concise,
    Polite,
    Translate,
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
            Self::Translate => "Translate the text into the selected target language.",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Grammar => "grammar",
            Self::Natural => "natural",
            Self::Concise => "concise",
            Self::Polite => "polite",
            Self::Translate => "translate",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub schema_version: u32,
    pub mode: RewriteMode,
    pub restore_clipboard: bool,
    pub auto_rewrite: bool,
    pub shortcut: ShortcutSettings,
    pub translation: TranslationSettings,
    pub terminology: TerminologySettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub primary: PrimaryShortcut,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            primary: PrimaryShortcut::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSettings {
    pub source_language: String,
    pub target_language: TranslationTargetLanguage,
    pub apply_format: TranslationApplyFormat,
}

impl Default for TranslationSettings {
    fn default() -> Self {
        Self {
            source_language: "auto".to_string(),
            target_language: TranslationTargetLanguage::default(),
            apply_format: TranslationApplyFormat::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologySettings {
    pub enabled: bool,
    pub active_profile_id: String,
    pub use_approved_terminology: bool,
    pub suggest_terminology: bool,
    pub auto_save_suggestions: bool,
}

impl Default for TerminologySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            active_profile_id: crate::terminology::GENERAL_PROFILE_ID.to_string(),
            use_approved_terminology: true,
            suggest_terminology: true,
            auto_save_suggestions: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SettingsRecoveryCode {
    Migrated,
    InvalidFields,
    BackupRecovered,
}

pub(crate) struct SettingsLoad {
    pub(crate) settings: AppSettings,
    pub(crate) recovery: Option<SettingsRecoveryCode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SaveFailurePoint {
    AfterBackupMoved,
}

pub(crate) fn decode_settings(text: &str) -> SettingsLoad {
    let value = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Object(value)) => value,
        _ => {
            return SettingsLoad {
                settings: AppSettings::default(),
                recovery: Some(SettingsRecoveryCode::InvalidFields),
            }
        }
    };
    let mut settings = AppSettings::default();
    let mut migrated = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(3);
    let mut invalid = false;

    if let Some(mode) = value.get("mode") {
        match mode.as_str() {
            Some("grammar") => settings.mode = RewriteMode::Grammar,
            Some("natural") => settings.mode = RewriteMode::Natural,
            Some("concise") => settings.mode = RewriteMode::Concise,
            Some("polite") => settings.mode = RewriteMode::Polite,
            Some("translate") => settings.mode = RewriteMode::Translate,
            Some("translate_en") => {
                settings.mode = RewriteMode::Translate;
                settings.translation.target_language = TranslationTargetLanguage::En;
                migrated = true;
            }
            Some("translate_ko") => {
                settings.mode = RewriteMode::Translate;
                settings.translation.target_language = TranslationTargetLanguage::Ko;
                migrated = true;
            }
            _ => invalid = true,
        }
    }

    if let Some(value) = value.get("restoreClipboard") {
        if let Some(value) = value.as_bool() {
            settings.restore_clipboard = value;
        } else {
            invalid = true;
        }
    }
    if let Some(value) = value.get("autoRewrite") {
        if let Some(value) = value.as_bool() {
            settings.auto_rewrite = value;
        } else {
            invalid = true;
        }
    }

    if let Some(primary) = value
        .get("shortcut")
        .and_then(serde_json::Value::as_object)
        .and_then(|shortcut| shortcut.get("primary"))
    {
        match serde_json::from_value::<PrimaryShortcut>(primary.clone()) {
            Ok(stored) => {
                let candidate = crate::shortcut::ShortcutCandidate {
                    modifiers: stored.modifiers.clone(),
                    key: stored.key.clone(),
                };
                match PrimaryShortcut::from_candidate(candidate) {
                    Ok(normalized) if normalized == stored => {
                        settings.shortcut.primary = normalized
                    }
                    _ => invalid = true,
                }
            }
            Err(_) => invalid = true,
        }
    } else if value.contains_key("shortcut") || value.get("schemaVersion").is_some() {
        migrated = true;
    }

    if let Some(translation) = value.get("translation") {
        let Some(translation) = translation.as_object() else {
            return SettingsLoad {
                settings,
                recovery: Some(SettingsRecoveryCode::InvalidFields),
            };
        };

        match translation
            .get("sourceLanguage")
            .and_then(serde_json::Value::as_str)
        {
            Some("auto") => {}
            Some(_) => invalid = true,
            None => migrated = true,
        }
        match translation.get("targetLanguage") {
            Some(target) => {
                match serde_json::from_value::<TranslationTargetLanguage>(target.clone()) {
                    Ok(target) => settings.translation.target_language = target,
                    Err(_) => invalid = true,
                }
            }
            None => migrated = true,
        }
        match translation.get("applyFormat") {
            Some(format) => {
                match serde_json::from_value::<TranslationApplyFormat>(format.clone()) {
                    Ok(format) => settings.translation.apply_format = format,
                    Err(_) => invalid = true,
                }
            }
            None => migrated = true,
        }
    } else if value.get("schemaVersion").is_some() {
        migrated = true;
    }

    if let Some(terminology) = value.get("terminology") {
        let Some(terminology) = terminology.as_object() else {
            return SettingsLoad {
                settings,
                recovery: Some(SettingsRecoveryCode::InvalidFields),
            };
        };

        match terminology.get("enabled") {
            Some(enabled) => match enabled.as_bool() {
                Some(enabled) => settings.terminology.enabled = enabled,
                None => invalid = true,
            },
            None => migrated = true,
        }
        match terminology
            .get("activeProfileId")
            .and_then(serde_json::Value::as_str)
        {
            Some(profile_id) if valid_profile_id(profile_id) => {
                settings.terminology.active_profile_id = profile_id.to_string();
            }
            Some(_) => invalid = true,
            None => migrated = true,
        }
        match terminology.get("useApprovedTerminology") {
            Some(enabled) => match enabled.as_bool() {
                Some(enabled) => settings.terminology.use_approved_terminology = enabled,
                None => invalid = true,
            },
            None => migrated = true,
        }
        match terminology.get("suggestTerminology") {
            Some(enabled) => match enabled.as_bool() {
                Some(enabled) => settings.terminology.suggest_terminology = enabled,
                None => invalid = true,
            },
            None => migrated = true,
        }
        match terminology.get("autoSaveSuggestions") {
            Some(enabled) => match enabled.as_bool() {
                Some(false) => {}
                Some(true) | None => invalid = true,
            },
            None => migrated = true,
        }
    } else {
        migrated = true;
    }

    SettingsLoad {
        settings,
        recovery: if invalid {
            Some(SettingsRecoveryCode::InvalidFields)
        } else if migrated {
            Some(SettingsRecoveryCode::Migrated)
        } else {
            None
        },
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: 3,
            mode: RewriteMode::Grammar,
            restore_clipboard: true,
            auto_rewrite: true,
            shortcut: ShortcutSettings::default(),
            translation: TranslationSettings::default(),
            terminology: TerminologySettings::default(),
        }
    }
}

impl AppSettings {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 3 {
            return Err("settings_schema_unsupported");
        }
        let normalized = PrimaryShortcut::from_candidate(crate::shortcut::ShortcutCandidate {
            modifiers: self.shortcut.primary.modifiers.clone(),
            key: self.shortcut.primary.key.clone(),
        })
        .map_err(|_| "invalid_primary_shortcut")?;
        if normalized != self.shortcut.primary {
            return Err("invalid_primary_shortcut");
        }
        if self.translation.source_language != "auto" {
            return Err("translation_source_must_be_auto");
        }
        if !valid_profile_id(&self.terminology.active_profile_id) {
            return Err("terminology_active_profile_invalid");
        }
        if self.terminology.auto_save_suggestions {
            return Err("terminology_auto_save_must_be_false");
        }
        Ok(())
    }

    pub(crate) fn rewrite_intent(&self) -> Result<RewriteIntent, &'static str> {
        RewriteIntent::new(
            self.mode,
            if self.mode == RewriteMode::Translate {
                Some(self.translation.target_language)
            } else {
                None
            },
        )
    }
}

fn valid_profile_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn load_state(app: &AppHandle) -> Result<SettingsLoad, String> {
    let path = settings_path(app)?;
    load_from_path(&path)
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    settings.validate().map_err(str::to_string)?;
    let path = settings_path(app)?;
    save_to_path(&path, settings)
}

pub(crate) fn load_from_path(path: &Path) -> Result<SettingsLoad, String> {
    if path.exists() {
        match fs::read_to_string(path) {
            Ok(text) => {
                let loaded = decode_settings(&text);
                if loaded.recovery != Some(SettingsRecoveryCode::InvalidFields) {
                    return Ok(loaded);
                }
            }
            Err(_) => {}
        }
    }

    let backup = backup_path(path);
    if backup.exists() {
        let text =
            fs::read_to_string(&backup).map_err(|_| "settings_backup_read_failed".to_string())?;
        let mut loaded = decode_settings(&text);
        if loaded.recovery != Some(SettingsRecoveryCode::InvalidFields) {
            loaded.recovery = Some(SettingsRecoveryCode::BackupRecovered);
            return Ok(loaded);
        }
    }

    if path.exists() {
        Err("settings_invalid".to_string())
    } else {
        Ok(SettingsLoad {
            settings: AppSettings::default(),
            recovery: None,
        })
    }
}

pub(crate) fn save_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
    save_to_path_internal(path, settings, None)
}

#[cfg(test)]
pub(crate) fn save_to_path_with_failure(
    path: &Path,
    settings: &AppSettings,
    failure: SaveFailurePoint,
) -> Result<(), String> {
    save_to_path_internal(path, settings, Some(failure))
}

fn save_to_path_internal(
    path: &Path,
    settings: &AppSettings,
    failure: Option<SaveFailurePoint>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "settings_directory_create_failed".to_string())?;
    }

    let text =
        serde_json::to_vec_pretty(settings).map_err(|_| "settings_serialize_failed".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "settings_path_invalid".to_string())?;
    let operation_id = uuid::Uuid::new_v4();
    let temporary = path.with_file_name(format!(".{file_name}.{operation_id}.tmp"));
    let rollback = path.with_file_name(format!(".{file_name}.{operation_id}.rollback"));
    let backup = backup_path(path);

    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "settings_temp_create_failed".to_string())?;
        file.write_all(&text)
            .map_err(|_| "settings_temp_write_failed".to_string())?;
        file.sync_all()
            .map_err(|_| "settings_temp_sync_failed".to_string())?;
        Ok::<(), String>(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let backup_is_valid = settings_file_is_valid(&backup);
    let previous_destination = if backup_is_valid {
        rollback.clone()
    } else {
        backup.clone()
    };
    let mut previous_moved = false;
    if path.exists() {
        if !backup_is_valid && backup.exists() && fs::remove_file(&backup).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err("settings_backup_cleanup_failed".to_string());
        }
        if fs::rename(path, &previous_destination).is_err() {
            let _ = fs::remove_file(&temporary);
            return Err("settings_previous_stage_failed".to_string());
        }
        previous_moved = true;
    }

    if failure == Some(SaveFailurePoint::AfterBackupMoved) {
        let rollback_ok = !previous_moved || fs::rename(&previous_destination, path).is_ok();
        let _ = fs::remove_file(&temporary);
        return if rollback_ok {
            Err("settings_injected_failure".to_string())
        } else {
            Err("settings_rollback_failed".to_string())
        };
    }

    if fs::rename(&temporary, path).is_err() {
        let rollback_ok = !previous_moved || fs::rename(&previous_destination, path).is_ok();
        let _ = fs::remove_file(&temporary);
        return if rollback_ok {
            Err("settings_promote_failed".to_string())
        } else {
            Err("settings_rollback_failed".to_string())
        };
    }
    if previous_moved && previous_destination == rollback {
        let _ = fs::remove_file(&rollback);
    }
    Ok(())
}

fn settings_file_is_valid(path: &Path) -> bool {
    fs::read_to_string(path)
        .ok()
        .map(|text| decode_settings(&text).recovery != Some(SettingsRecoveryCode::InvalidFields))
        .unwrap_or(false)
}

fn backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("settings.json");
    path.with_file_name(format!("{file_name}.bak"))
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Could not resolve app config directory: {error}"))?;
    Ok(dir.join("settings.json"))
}
