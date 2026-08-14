use crate::settings::RewriteMode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

pub(crate) const GLOBAL_PROFILE_ID: &str = "global";
pub(crate) const GENERAL_PROFILE_ID: &str = "general";
pub(crate) const MAX_PROFILES: usize = 64;
pub(crate) const MAX_ENTRIES: usize = 10_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) enum LanguageScope {
    #[serde(rename = "any")]
    Any,
    #[serde(rename = "ko")]
    Ko,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
    #[serde(rename = "zh-Hans")]
    ZhHans,
    #[serde(rename = "zh-Hant")]
    ZhHant,
}

impl LanguageScope {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Ko => "ko",
            Self::En => "en",
            Self::Ja => "ja",
            Self::ZhHans => "zh-Hans",
            Self::ZhHant => "zh-Hant",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntryType {
    Translation,
    Preferred,
    Protected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntryStatus {
    Approved,
    Suggested,
    Disabled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntryMatchMode {
    WholePhrase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologyProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) enabled: bool,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologyEntry {
    pub(crate) id: String,
    pub(crate) profile_id: String,
    #[serde(rename = "type")]
    pub(crate) entry_type: EntryType,
    pub(crate) status: EntryStatus,
    pub(crate) source_text: String,
    pub(crate) preferred_text: Option<String>,
    pub(crate) source_language: LanguageScope,
    pub(crate) target_language: LanguageScope,
    pub(crate) aliases: Vec<String>,
    pub(crate) match_mode: EntryMatchMode,
    pub(crate) case_sensitive: bool,
    pub(crate) priority: u16,
    pub(crate) usage_count: u64,
    pub(crate) occurrence_count: u64,
    pub(crate) note: Option<String>,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologyEntryDraft {
    pub(crate) profile_id: String,
    #[serde(rename = "type")]
    pub(crate) entry_type: EntryType,
    pub(crate) status: EntryStatus,
    pub(crate) source_text: String,
    pub(crate) preferred_text: Option<String>,
    pub(crate) source_language: LanguageScope,
    pub(crate) target_language: LanguageScope,
    pub(crate) aliases: Vec<String>,
    pub(crate) match_mode: EntryMatchMode,
    pub(crate) case_sensitive: bool,
    pub(crate) priority: u16,
    pub(crate) usage_count: u64,
    pub(crate) occurrence_count: u64,
    pub(crate) note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TerminologyStoreV1 {
    pub(crate) schema_version: u32,
    pub(crate) revision: u64,
    pub(crate) profiles: Vec<TerminologyProfile>,
    pub(crate) entries: Vec<TerminologyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminologyError {
    InvalidSchema,
    InvalidId,
    InvalidProfile,
    InvalidEntry,
    InvalidTerm,
    InvalidPreferred,
    InvalidAlias,
    InvalidNote,
    InvalidPriority,
    DuplicateId,
    DuplicateProfileName,
    DuplicateAlias,
    ReservedProfile,
    ActiveProfileRequired,
    ProfileNotFound,
    EntryNotFound,
    ProfileLimit,
    EntryLimit,
    RevisionExhausted,
    NormalizationFailed,
    StoreIo,
    StoreUnrecoverable,
    ImportInvalid,
    ImportTooLarge,
    StaleImportPlan,
    ImportPlanExpired,
}

impl TerminologyError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::InvalidSchema => "terminology_schema_invalid",
            Self::InvalidId => "terminology_id_invalid",
            Self::InvalidProfile => "terminology_profile_invalid",
            Self::InvalidEntry => "terminology_entry_invalid",
            Self::InvalidTerm => "terminology_term_invalid",
            Self::InvalidPreferred => "terminology_preferred_invalid",
            Self::InvalidAlias => "terminology_alias_invalid",
            Self::InvalidNote => "terminology_note_invalid",
            Self::InvalidPriority => "terminology_priority_invalid",
            Self::DuplicateId => "terminology_id_conflict",
            Self::DuplicateProfileName => "terminology_profile_name_conflict",
            Self::DuplicateAlias => "terminology_alias_conflict",
            Self::ReservedProfile => "terminology_reserved_profile",
            Self::ActiveProfileRequired => "terminology_active_profile_required",
            Self::ProfileNotFound => "terminology_profile_not_found",
            Self::EntryNotFound => "terminology_entry_not_found",
            Self::ProfileLimit => "terminology_profile_limit",
            Self::EntryLimit => "terminology_entry_limit",
            Self::RevisionExhausted => "terminology_revision_exhausted",
            Self::NormalizationFailed => "terminology_normalization_failed",
            Self::StoreIo => "terminology_store_io_failed",
            Self::StoreUnrecoverable => "terminology_store_unrecoverable",
            Self::ImportInvalid => "terminology_import_invalid",
            Self::ImportTooLarge => "terminology_import_too_large",
            Self::StaleImportPlan => "terminology_import_plan_stale",
            Self::ImportPlanExpired => "terminology_import_plan_expired",
        }
    }
}

impl TerminologyStoreV1 {
    pub(crate) fn new(now_ms: u64) -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            profiles: vec![
                TerminologyProfile {
                    id: GLOBAL_PROFILE_ID.to_string(),
                    name: "Global".to_string(),
                    enabled: true,
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                },
                TerminologyProfile {
                    id: GENERAL_PROFILE_ID.to_string(),
                    name: "General".to_string(),
                    enabled: true,
                    created_at_ms: now_ms,
                    updated_at_ms: now_ms,
                },
            ],
            entries: Vec::new(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), TerminologyError> {
        if self.schema_version != 1 {
            return Err(TerminologyError::InvalidSchema);
        }
        if self.profiles.len() > MAX_PROFILES {
            return Err(TerminologyError::ProfileLimit);
        }
        if self.entries.len() > MAX_ENTRIES {
            return Err(TerminologyError::EntryLimit);
        }

        let mut profile_ids = HashSet::new();
        let mut profile_names = HashSet::new();
        for profile in &self.profiles {
            validate_id(&profile.id)?;
            let name = validate_profile_name(&profile.name)?;
            if name != profile.name {
                return Err(TerminologyError::InvalidProfile);
            }
            if !profile_ids.insert(profile.id.as_str()) {
                return Err(TerminologyError::DuplicateId);
            }
            if !profile_names.insert(normalize_match_key(name, false)?) {
                return Err(TerminologyError::DuplicateProfileName);
            }
            if profile.created_at_ms > profile.updated_at_ms {
                return Err(TerminologyError::InvalidProfile);
            }
        }

        let global = self
            .profiles
            .iter()
            .find(|profile| profile.id == GLOBAL_PROFILE_ID)
            .ok_or(TerminologyError::InvalidProfile)?;
        if !global.enabled {
            return Err(TerminologyError::InvalidProfile);
        }
        if !self
            .profiles
            .iter()
            .any(|profile| profile.id == GENERAL_PROFILE_ID)
        {
            return Err(TerminologyError::InvalidProfile);
        }

        let mut entry_ids = HashSet::new();
        for entry in &self.entries {
            validate_id(&entry.id)?;
            if profile_ids.contains(entry.id.as_str()) || !entry_ids.insert(entry.id.as_str()) {
                return Err(TerminologyError::DuplicateId);
            }
            if !profile_ids.contains(entry.profile_id.as_str()) {
                return Err(TerminologyError::ProfileNotFound);
            }
            validate_entry(entry)?;
        }
        Ok(())
    }

    pub(crate) fn add_profile(
        &mut self,
        id: String,
        name: String,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        if self.profiles.len() >= MAX_PROFILES {
            return Err(TerminologyError::ProfileLimit);
        }
        validate_id(&id)?;
        if self.profiles.iter().any(|profile| profile.id == id)
            || self.entries.iter().any(|entry| entry.id == id)
        {
            return Err(TerminologyError::DuplicateId);
        }
        let name = validate_profile_name(&name)?.to_string();
        self.ensure_unique_profile_name(&name, None)?;
        self.next_revision()?;
        self.profiles.push(TerminologyProfile {
            id,
            name,
            enabled: true,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        });
        Ok(())
    }

    pub(crate) fn rename_profile(
        &mut self,
        profile_id: &str,
        name: String,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        if profile_id == GLOBAL_PROFILE_ID {
            return Err(TerminologyError::ReservedProfile);
        }
        let name = validate_profile_name(&name)?.to_string();
        self.ensure_unique_profile_name(&name, Some(profile_id))?;
        let profile = self
            .profiles
            .iter_mut()
            .find(|profile| profile.id == profile_id)
            .ok_or(TerminologyError::ProfileNotFound)?;
        if profile.name == name {
            return Ok(());
        }
        profile.name = name;
        profile.updated_at_ms = now_ms;
        self.next_revision()
    }

    pub(crate) fn set_profile_enabled(
        &mut self,
        profile_id: &str,
        enabled: bool,
        active_profile_id: &str,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        if profile_id == GLOBAL_PROFILE_ID && !enabled {
            return Err(TerminologyError::ReservedProfile);
        }
        if profile_id == active_profile_id && !enabled {
            return Err(TerminologyError::ActiveProfileRequired);
        }
        let profile = self
            .profiles
            .iter_mut()
            .find(|profile| profile.id == profile_id)
            .ok_or(TerminologyError::ProfileNotFound)?;
        if profile.enabled == enabled {
            return Ok(());
        }
        profile.enabled = enabled;
        profile.updated_at_ms = now_ms;
        self.next_revision()
    }

    pub(crate) fn add_entry(
        &mut self,
        id: String,
        draft: TerminologyEntryDraft,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        if self.entries.len() >= MAX_ENTRIES {
            return Err(TerminologyError::EntryLimit);
        }
        validate_id(&id)?;
        if self.entries.iter().any(|entry| entry.id == id)
            || self.profiles.iter().any(|profile| profile.id == id)
        {
            return Err(TerminologyError::DuplicateId);
        }
        if !self
            .profiles
            .iter()
            .any(|profile| profile.id == draft.profile_id)
        {
            return Err(TerminologyError::ProfileNotFound);
        }
        let entry = canonical_entry(id, draft, now_ms, now_ms)?;
        self.next_revision()?;
        self.entries.push(entry);
        Ok(())
    }

    pub(crate) fn update_entry(
        &mut self,
        id: &str,
        draft: TerminologyEntryDraft,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        if !self
            .profiles
            .iter()
            .any(|profile| profile.id == draft.profile_id)
        {
            return Err(TerminologyError::ProfileNotFound);
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(TerminologyError::EntryNotFound)?;
        let created_at_ms = self.entries[index].created_at_ms;
        let next = canonical_entry(id.to_string(), draft, created_at_ms, now_ms)?;
        if self.entries[index] == next {
            return Ok(());
        }
        self.next_revision()?;
        self.entries[index] = next;
        Ok(())
    }

    pub(crate) fn set_entry_status(
        &mut self,
        id: &str,
        status: EntryStatus,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(TerminologyError::EntryNotFound)?;
        if entry.status == status {
            return Ok(());
        }
        entry.status = status;
        entry.updated_at_ms = now_ms;
        self.next_revision()
    }

    pub(crate) fn delete_entry(&mut self, id: &str) -> Result<(), TerminologyError> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(TerminologyError::EntryNotFound)?;
        self.next_revision()?;
        self.entries.remove(index);
        Ok(())
    }

    pub(crate) fn enabled_profile(&self, id: &str) -> Option<&TerminologyProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.id == id && profile.enabled && id != GLOBAL_PROFILE_ID)
    }

    pub(crate) fn increment_usage(
        &mut self,
        entry_ids: &[String],
        now_ms: u64,
    ) -> Result<bool, TerminologyError> {
        let ids = entry_ids.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut changed = false;
        for entry in &mut self.entries {
            if ids.contains(entry.id.as_str()) {
                entry.usage_count = entry
                    .usage_count
                    .checked_add(1)
                    .ok_or(TerminologyError::RevisionExhausted)?;
                entry.updated_at_ms = now_ms;
                changed = true;
            }
        }
        if changed {
            self.next_revision()?;
        }
        Ok(changed)
    }

    fn ensure_unique_profile_name(
        &self,
        name: &str,
        except_id: Option<&str>,
    ) -> Result<(), TerminologyError> {
        let key = normalize_match_key(name, false)?;
        if self.profiles.iter().any(|profile| {
            Some(profile.id.as_str()) != except_id
                && normalize_match_key(&profile.name, false).ok().as_deref() == Some(key.as_str())
        }) {
            return Err(TerminologyError::DuplicateProfileName);
        }
        Ok(())
    }

    fn next_revision(&mut self) -> Result<(), TerminologyError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(TerminologyError::RevisionExhausted)?;
        Ok(())
    }
}

fn canonical_entry(
    id: String,
    draft: TerminologyEntryDraft,
    created_at_ms: u64,
    updated_at_ms: u64,
) -> Result<TerminologyEntry, TerminologyError> {
    let source_text = validate_single_line(&draft.source_text, 256, TerminologyError::InvalidTerm)?;
    let mut preferred_text = match draft.preferred_text {
        Some(value) => Some(validate_single_line(
            &value,
            512,
            TerminologyError::InvalidPreferred,
        )?),
        None => None,
    };
    let mut aliases = Vec::with_capacity(draft.aliases.len());
    if draft.aliases.len() > 32 {
        return Err(TerminologyError::InvalidAlias);
    }
    let mut alias_keys = BTreeSet::new();
    for alias in draft.aliases {
        let alias = validate_single_line(&alias, 256, TerminologyError::InvalidAlias)?;
        let key = normalize_match_key(&alias, draft.case_sensitive)?;
        if !alias_keys.insert(key) {
            return Err(TerminologyError::DuplicateAlias);
        }
        aliases.push(alias);
    }
    let note = match draft.note {
        Some(note) if note.chars().count() <= 1024 => Some(note),
        Some(_) => return Err(TerminologyError::InvalidNote),
        None => None,
    };
    if draft.priority > 1_000 {
        return Err(TerminologyError::InvalidPriority);
    }
    let case_sensitive = if draft.entry_type == EntryType::Protected {
        preferred_text = None;
        true
    } else {
        if preferred_text.is_none() {
            return Err(TerminologyError::InvalidPreferred);
        }
        draft.case_sensitive
    };

    let entry = TerminologyEntry {
        id,
        profile_id: draft.profile_id,
        entry_type: draft.entry_type,
        status: draft.status,
        source_text,
        preferred_text,
        source_language: draft.source_language,
        target_language: draft.target_language,
        aliases,
        match_mode: draft.match_mode,
        case_sensitive,
        priority: draft.priority,
        usage_count: draft.usage_count,
        occurrence_count: draft.occurrence_count,
        note,
        created_at_ms,
        updated_at_ms,
    };
    validate_entry(&entry)?;
    Ok(entry)
}

fn validate_entry(entry: &TerminologyEntry) -> Result<(), TerminologyError> {
    if entry.created_at_ms > entry.updated_at_ms || entry.priority > 1_000 {
        return Err(TerminologyError::InvalidEntry);
    }
    if entry.profile_id.trim().is_empty() {
        return Err(TerminologyError::InvalidProfile);
    }
    if validate_single_line(&entry.source_text, 256, TerminologyError::InvalidTerm)?
        != entry.source_text
    {
        return Err(TerminologyError::InvalidTerm);
    }
    match entry.entry_type {
        EntryType::Protected if entry.preferred_text.is_some() || !entry.case_sensitive => {
            return Err(TerminologyError::InvalidPreferred)
        }
        EntryType::Translation | EntryType::Preferred => {
            let preferred = entry
                .preferred_text
                .as_deref()
                .ok_or(TerminologyError::InvalidPreferred)?;
            if validate_single_line(preferred, 512, TerminologyError::InvalidPreferred)?
                != preferred
            {
                return Err(TerminologyError::InvalidPreferred);
            }
        }
        EntryType::Protected => {}
    }
    if entry.aliases.len() > 32 {
        return Err(TerminologyError::InvalidAlias);
    }
    let mut aliases = BTreeSet::new();
    for alias in &entry.aliases {
        if validate_single_line(alias, 256, TerminologyError::InvalidAlias)? != *alias {
            return Err(TerminologyError::InvalidAlias);
        }
        if !aliases.insert(normalize_match_key(alias, entry.case_sensitive)?) {
            return Err(TerminologyError::DuplicateAlias);
        }
    }
    if entry
        .note
        .as_ref()
        .is_some_and(|note| note.chars().count() > 1024)
    {
        return Err(TerminologyError::InvalidNote);
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), TerminologyError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(TerminologyError::InvalidId);
    }
    Ok(())
}

fn validate_profile_name(value: &str) -> Result<&str, TerminologyError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 128 || trimmed.contains(['\r', '\n']) {
        return Err(TerminologyError::InvalidProfile);
    }
    Ok(trimmed)
}

fn validate_single_line(
    value: &str,
    max_chars: usize,
    error: TerminologyError,
) -> Result<String, TerminologyError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > max_chars || trimmed.contains(['\r', '\n']) {
        return Err(error);
    }
    normalize_nfc(trimmed)?;
    Ok(trimmed.to_string())
}

pub(crate) fn normalize_match_key(
    value: &str,
    case_sensitive: bool,
) -> Result<String, TerminologyError> {
    let nfc = normalize_nfc(value)?;
    let collapsed = nfc.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(if case_sensitive {
        collapsed
    } else {
        collapsed.to_lowercase()
    })
}

#[cfg(windows)]
pub(crate) fn normalize_nfc(value: &str) -> Result<String, TerminologyError> {
    use windows_sys::Win32::Globalization::{NormalizationC, NormalizeString};

    if value.is_empty() {
        return Ok(String::new());
    }
    let source = value.encode_utf16().collect::<Vec<_>>();
    let source_len =
        i32::try_from(source.len()).map_err(|_| TerminologyError::NormalizationFailed)?;
    let required = unsafe {
        NormalizeString(
            NormalizationC,
            source.as_ptr(),
            source_len,
            std::ptr::null_mut(),
            0,
        )
    };
    if required <= 0 {
        return Err(TerminologyError::NormalizationFailed);
    }
    let mut output = vec![0u16; required as usize];
    let written = unsafe {
        NormalizeString(
            NormalizationC,
            source.as_ptr(),
            source_len,
            output.as_mut_ptr(),
            required,
        )
    };
    if written <= 0 {
        return Err(TerminologyError::NormalizationFailed);
    }
    output.truncate(written as usize);
    String::from_utf16(&output).map_err(|_| TerminologyError::NormalizationFailed)
}

#[cfg(not(windows))]
pub(crate) fn normalize_nfc(value: &str) -> Result<String, TerminologyError> {
    Ok(value.to_string())
}

pub(crate) fn infer_source_language(value: &str) -> Option<LanguageScope> {
    let mut has_latin = false;
    let mut has_cjk = false;
    for character in value.chars() {
        let scalar = character as u32;
        if (0xAC00..=0xD7AF).contains(&scalar) || (0x1100..=0x11FF).contains(&scalar) {
            return Some(LanguageScope::Ko);
        }
        if (0x3040..=0x30FF).contains(&scalar) {
            return Some(LanguageScope::Ja);
        }
        if (0x4E00..=0x9FFF).contains(&scalar) {
            has_cjk = true;
        }
        if character.is_ascii_alphabetic() {
            has_latin = true;
        }
    }
    if has_cjk {
        None
    } else if has_latin {
        Some(LanguageScope::En)
    } else {
        None
    }
}

pub(crate) fn target_scope_for_mode(
    mode: RewriteMode,
    target: Option<crate::translation::TranslationTargetLanguage>,
) -> Option<LanguageScope> {
    if mode != RewriteMode::Translate {
        return None;
    }
    target.map(|target| match target {
        crate::translation::TranslationTargetLanguage::Ko => LanguageScope::Ko,
        crate::translation::TranslationTargetLanguage::En => LanguageScope::En,
        crate::translation::TranslationTargetLanguage::Ja => LanguageScope::Ja,
        crate::translation::TranslationTargetLanguage::ZhHans => LanguageScope::ZhHans,
        crate::translation::TranslationTargetLanguage::ZhHant => LanguageScope::ZhHant,
    })
}
