use crate::terminology::{
    normalize_match_key, TerminologyEntry, TerminologyError, TerminologyProfile,
    TerminologyStoreV1, GENERAL_PROFILE_ID, GLOBAL_PROFILE_ID, MAX_ENTRIES, MAX_PROFILES,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const MAX_IMPORT_BYTES: usize = 2 * 1024 * 1024;
const PLAN_LIFETIME_MS: u64 = 5 * 60 * 1_000;
const CSV_COLUMNS: [&str; 17] = [
    "profile_id",
    "profile_name",
    "type",
    "status",
    "source_text",
    "preferred_text",
    "source_language",
    "target_language",
    "aliases_json",
    "match_mode",
    "case_sensitive",
    "priority",
    "usage_count",
    "occurrence_count",
    "note",
    "created_at_ms",
    "updated_at_ms",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportFormat {
    Json,
    Csv,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImportConflictKind {
    IdConflict,
    SemanticConflict,
    UnknownProfile,
    InvalidRow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportConflict {
    pub(crate) kind: ImportConflictKind,
    pub(crate) incoming_id: Option<String>,
    pub(crate) existing_id: Option<String>,
    pub(crate) row_number: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ImportReport {
    pub(crate) new_profiles: usize,
    pub(crate) new_entries: usize,
    pub(crate) identical_duplicates: usize,
    pub(crate) id_conflicts: usize,
    pub(crate) semantic_key_conflicts: usize,
    pub(crate) invalid_rows: usize,
    pub(crate) skipped_rows: usize,
    pub(crate) conflicts: Vec<ImportConflict>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ImportPlan {
    pub(crate) plan_id: String,
    pub(crate) base_revision: u64,
    pub(crate) expires_at_ms: u64,
    pub(crate) report: ImportReport,
    pub(crate) next_store: TerminologyStoreV1,
}

pub(crate) fn export_json(store: &TerminologyStoreV1) -> Result<String, TerminologyError> {
    store.validate()?;
    serde_json::to_string_pretty(store).map_err(|_| TerminologyError::InvalidEntry)
}

pub(crate) fn export_csv(store: &TerminologyStoreV1) -> Result<String, TerminologyError> {
    store.validate()?;
    let profile_names = store
        .profiles
        .iter()
        .map(|profile| (profile.id.as_str(), profile.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut output = String::new();
    output.push_str(&CSV_COLUMNS.join(","));
    output.push_str("\r\n");
    let mut entries = store.entries.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    for entry in entries {
        let aliases =
            serde_json::to_string(&entry.aliases).map_err(|_| TerminologyError::InvalidEntry)?;
        let fields = [
            entry.profile_id.clone(),
            profile_names
                .get(entry.profile_id.as_str())
                .copied()
                .unwrap_or_default()
                .to_string(),
            serde_value(entry.entry_type)?,
            serde_value(entry.status)?,
            entry.source_text.clone(),
            entry.preferred_text.clone().unwrap_or_default(),
            entry.source_language.code().to_string(),
            entry.target_language.code().to_string(),
            aliases,
            serde_value(entry.match_mode)?,
            entry.case_sensitive.to_string(),
            entry.priority.to_string(),
            entry.usage_count.to_string(),
            entry.occurrence_count.to_string(),
            entry.note.clone().unwrap_or_default(),
            entry.created_at_ms.to_string(),
            entry.updated_at_ms.to_string(),
        ];
        output.push_str(
            &fields
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push_str("\r\n");
    }
    Ok(output)
}

pub(crate) fn dry_run_import(
    current: &TerminologyStoreV1,
    format: ImportFormat,
    text: &str,
    plan_id: String,
    now_ms: u64,
) -> Result<ImportPlan, TerminologyError> {
    current.validate()?;
    if text.as_bytes().len() > MAX_IMPORT_BYTES {
        return Err(TerminologyError::ImportTooLarge);
    }
    if plan_id.is_empty() || plan_id.len() > 128 {
        return Err(TerminologyError::InvalidId);
    }
    let (profiles, entries, mut report) = match format {
        ImportFormat::Json => {
            let imported = serde_json::from_str::<TerminologyStoreV1>(text)
                .map_err(|_| TerminologyError::ImportInvalid)?;
            imported.validate()?;
            (
                imported.profiles,
                imported
                    .entries
                    .into_iter()
                    .map(|entry| (entry, None))
                    .collect(),
                ImportReport::default(),
            )
        }
        ImportFormat::Csv => parse_csv_import(text)?,
    };

    let mut next = current.clone();
    let blocked_profile_ids = merge_profiles(&mut next, profiles, &mut report)?;
    merge_entries(&mut next, entries, &blocked_profile_ids, &mut report)?;
    if next.profiles.len() > MAX_PROFILES || next.entries.len() > MAX_ENTRIES {
        return Err(TerminologyError::ImportTooLarge);
    }
    next.revision = if report.new_profiles + report.new_entries > 0 {
        current
            .revision
            .checked_add(1)
            .ok_or(TerminologyError::RevisionExhausted)?
    } else {
        current.revision
    };
    next.validate()?;
    Ok(ImportPlan {
        plan_id,
        base_revision: current.revision,
        expires_at_ms: now_ms.saturating_add(PLAN_LIFETIME_MS),
        report,
        next_store: next,
    })
}

pub(crate) fn apply_import_plan(
    current: &TerminologyStoreV1,
    plan: &ImportPlan,
    now_ms: u64,
) -> Result<TerminologyStoreV1, TerminologyError> {
    if current.revision != plan.base_revision {
        return Err(TerminologyError::StaleImportPlan);
    }
    if now_ms > plan.expires_at_ms {
        return Err(TerminologyError::ImportPlanExpired);
    }
    plan.next_store.validate()?;
    Ok(plan.next_store.clone())
}

fn merge_profiles(
    store: &mut TerminologyStoreV1,
    profiles: Vec<TerminologyProfile>,
    report: &mut ImportReport,
) -> Result<HashSet<String>, TerminologyError> {
    let mut blocked_profile_ids = HashSet::new();
    for profile in profiles {
        if matches!(profile.id.as_str(), GLOBAL_PROFILE_ID | GENERAL_PROFILE_ID) {
            if store
                .profiles
                .iter()
                .any(|existing| existing.id == profile.id)
            {
                continue;
            }
            report.id_conflicts += 1;
            report.skipped_rows += 1;
            blocked_profile_ids.insert(profile.id.clone());
            report.conflicts.push(ImportConflict {
                kind: ImportConflictKind::IdConflict,
                incoming_id: Some(profile.id),
                existing_id: None,
                row_number: None,
            });
            continue;
        }
        if let Some(existing) = store
            .profiles
            .iter()
            .find(|existing| existing.id == profile.id)
        {
            if normalize_match_key(&existing.name, false)?
                == normalize_match_key(&profile.name, false)?
                && existing.enabled == profile.enabled
            {
                report.identical_duplicates += 1;
            } else {
                report.id_conflicts += 1;
                report.skipped_rows += 1;
                blocked_profile_ids.insert(profile.id.clone());
                report.conflicts.push(ImportConflict {
                    kind: ImportConflictKind::IdConflict,
                    incoming_id: Some(profile.id),
                    existing_id: Some(existing.id.clone()),
                    row_number: None,
                });
            }
            continue;
        }
        if let Some(existing) = store.profiles.iter().find(|existing| {
            normalize_match_key(&existing.name, false).ok()
                == normalize_match_key(&profile.name, false).ok()
        }) {
            report.semantic_key_conflicts += 1;
            report.skipped_rows += 1;
            blocked_profile_ids.insert(profile.id.clone());
            report.conflicts.push(ImportConflict {
                kind: ImportConflictKind::SemanticConflict,
                incoming_id: Some(profile.id),
                existing_id: Some(existing.id.clone()),
                row_number: None,
            });
            continue;
        }
        if store.profiles.len() >= MAX_PROFILES {
            return Err(TerminologyError::ProfileLimit);
        }
        store.profiles.push(profile);
        report.new_profiles += 1;
    }
    Ok(blocked_profile_ids)
}

fn merge_entries(
    store: &mut TerminologyStoreV1,
    entries: Vec<(TerminologyEntry, Option<usize>)>,
    blocked_profile_ids: &HashSet<String>,
    report: &mut ImportReport,
) -> Result<(), TerminologyError> {
    for (entry, row_number) in entries {
        if blocked_profile_ids.contains(&entry.profile_id)
            || !store
                .profiles
                .iter()
                .any(|profile| profile.id == entry.profile_id)
        {
            report.skipped_rows += 1;
            report.conflicts.push(ImportConflict {
                kind: ImportConflictKind::UnknownProfile,
                incoming_id: Some(entry.id),
                existing_id: None,
                row_number,
            });
            continue;
        }
        if let Some(existing) = store
            .entries
            .iter()
            .find(|existing| existing.id == entry.id)
        {
            if existing == &entry {
                report.identical_duplicates += 1;
            } else {
                report.id_conflicts += 1;
                report.conflicts.push(ImportConflict {
                    kind: ImportConflictKind::IdConflict,
                    incoming_id: Some(entry.id),
                    existing_id: Some(existing.id.clone()),
                    row_number,
                });
            }
            report.skipped_rows += 1;
            continue;
        }
        if let Some(existing) = store
            .entries
            .iter()
            .find(|existing| semantic_key(existing).ok() == semantic_key(&entry).ok())
        {
            if semantically_identical(existing, &entry) {
                report.identical_duplicates += 1;
            } else {
                report.semantic_key_conflicts += 1;
                report.conflicts.push(ImportConflict {
                    kind: ImportConflictKind::SemanticConflict,
                    incoming_id: Some(entry.id),
                    existing_id: Some(existing.id.clone()),
                    row_number,
                });
            }
            report.skipped_rows += 1;
            continue;
        }
        if store.entries.len() >= MAX_ENTRIES {
            return Err(TerminologyError::EntryLimit);
        }
        store.entries.push(entry);
        report.new_entries += 1;
    }
    Ok(())
}

fn semantic_key(entry: &TerminologyEntry) -> Result<String, TerminologyError> {
    Ok(format!(
        "{}|{:?}|{}|{}|{}|{}",
        entry.profile_id,
        entry.entry_type,
        normalize_match_key(&entry.source_text, entry.case_sensitive)?,
        entry.source_language.code(),
        entry.target_language.code(),
        entry.case_sensitive
    ))
}

fn semantically_identical(left: &TerminologyEntry, right: &TerminologyEntry) -> bool {
    left.profile_id == right.profile_id
        && left.entry_type == right.entry_type
        && left.status == right.status
        && normalize_match_key(&left.source_text, left.case_sensitive).ok()
            == normalize_match_key(&right.source_text, right.case_sensitive).ok()
        && left.preferred_text == right.preferred_text
        && left.source_language == right.source_language
        && left.target_language == right.target_language
        && left.aliases == right.aliases
        && left.match_mode == right.match_mode
        && left.case_sensitive == right.case_sensitive
        && left.priority == right.priority
        && left.note == right.note
}

fn parse_csv_import(
    text: &str,
) -> Result<
    (
        Vec<TerminologyProfile>,
        Vec<(TerminologyEntry, Option<usize>)>,
        ImportReport,
    ),
    TerminologyError,
> {
    let rows = parse_csv(text)?;
    let Some(header) = rows.first() else {
        return Err(TerminologyError::ImportInvalid);
    };
    if header.len() != CSV_COLUMNS.len()
        || header.iter().map(String::as_str).collect::<Vec<_>>() != CSV_COLUMNS
        || header.iter().collect::<HashSet<_>>().len() != header.len()
    {
        return Err(TerminologyError::ImportInvalid);
    }

    let mut profiles = Vec::<TerminologyProfile>::new();
    let mut entries = Vec::new();
    let mut report = ImportReport::default();
    let mut seen_profiles = HashSet::new();
    for (row_index, row) in rows.into_iter().enumerate().skip(1) {
        if row.len() == 1 && row[0].is_empty() {
            continue;
        }
        validate_csv_row_shape(&row)?;
        let parsed = parse_csv_entry(&row);
        let Ok((profile, entry)) = parsed else {
            report.invalid_rows += 1;
            report.skipped_rows += 1;
            report.conflicts.push(ImportConflict {
                kind: ImportConflictKind::InvalidRow,
                incoming_id: None,
                existing_id: None,
                row_number: Some(row_index + 1),
            });
            continue;
        };
        if !matches!(profile.id.as_str(), GLOBAL_PROFILE_ID | GENERAL_PROFILE_ID)
            && seen_profiles.insert(profile.id.clone())
        {
            profiles.push(profile);
        }
        entries.push((entry, Some(row_index + 1)));
    }
    Ok((profiles, entries, report))
}

fn validate_csv_row_shape(row: &[String]) -> Result<(), TerminologyError> {
    if row.len() != CSV_COLUMNS.len() {
        return Err(TerminologyError::ImportInvalid);
    }
    let _: crate::terminology::EntryType = parse_enum(&row[2])?;
    let _: crate::terminology::EntryStatus = parse_enum(&row[3])?;
    let _: crate::terminology::LanguageScope = parse_enum(&row[6])?;
    let _: crate::terminology::LanguageScope = parse_enum(&row[7])?;
    let aliases = serde_json::from_str::<Vec<String>>(&row[8])
        .map_err(|_| TerminologyError::ImportInvalid)?;
    let _: crate::terminology::EntryMatchMode = parse_enum(&row[9])?;
    if !matches!(row[10].as_str(), "true" | "false")
        || row[11].parse::<u16>().is_err()
        || parse_u64(&row[12]).is_err()
        || parse_u64(&row[13]).is_err()
        || parse_u64(&row[15]).is_err()
        || parse_u64(&row[16]).is_err()
        || row[0].is_empty()
        || row[0].len() > 128
        || row[1].trim().is_empty()
        || row[1].chars().count() > 128
        || row[1].contains(['\r', '\n'])
        || row[4].trim().is_empty()
        || row[4].chars().count() > 256
        || row[4].contains(['\r', '\n'])
        || row[5].chars().count() > 512
        || row[5].contains(['\r', '\n'])
        || aliases.len() > 32
        || aliases.iter().any(|alias| {
            alias.trim().is_empty() || alias.chars().count() > 256 || alias.contains(['\r', '\n'])
        })
        || row[14].chars().count() > 1_024
    {
        return Err(TerminologyError::ImportInvalid);
    }
    Ok(())
}

fn parse_csv_entry(
    row: &[String],
) -> Result<(TerminologyProfile, TerminologyEntry), TerminologyError> {
    if row.len() != CSV_COLUMNS.len() {
        return Err(TerminologyError::ImportInvalid);
    }
    let aliases = serde_json::from_str::<Vec<String>>(&row[8])
        .map_err(|_| TerminologyError::ImportInvalid)?;
    let created_at_ms = parse_u64(&row[15])?;
    let updated_at_ms = parse_u64(&row[16])?;
    let profile = TerminologyProfile {
        id: row[0].clone(),
        name: row[1].clone(),
        enabled: true,
        created_at_ms,
        updated_at_ms,
    };
    let entry = TerminologyEntry {
        id: format!("import-{}", uuid::Uuid::new_v4()),
        profile_id: row[0].clone(),
        entry_type: parse_enum(&row[2])?,
        status: parse_enum(&row[3])?,
        source_text: row[4].clone(),
        preferred_text: (!row[5].is_empty()).then(|| row[5].clone()),
        source_language: parse_enum(&row[6])?,
        target_language: parse_enum(&row[7])?,
        aliases,
        match_mode: parse_enum(&row[9])?,
        case_sensitive: match row[10].as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(TerminologyError::ImportInvalid),
        },
        priority: row[11]
            .parse::<u16>()
            .map_err(|_| TerminologyError::ImportInvalid)?,
        usage_count: parse_u64(&row[12])?,
        occurrence_count: parse_u64(&row[13])?,
        note: (!row[14].is_empty()).then(|| row[14].clone()),
        created_at_ms,
        updated_at_ms,
    };
    let mut validation = TerminologyStoreV1::new(created_at_ms);
    if !matches!(profile.id.as_str(), GLOBAL_PROFILE_ID | GENERAL_PROFILE_ID) {
        validation.profiles.push(profile.clone());
    }
    validation.entries.push(entry.clone());
    validation.validate()?;
    Ok((profile, entry))
}

fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, TerminologyError> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut closed_quote = false;
    while let Some(character) = chars.next() {
        if quoted {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed_quote = true;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        if closed_quote && !matches!(character, ',' | '\r' | '\n') {
            return Err(TerminologyError::ImportInvalid);
        }
        match character {
            '"' if field.is_empty() && !closed_quote => quoted = true,
            '"' => return Err(TerminologyError::ImportInvalid),
            ',' => {
                row.push(std::mem::take(&mut field));
                closed_quote = false;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                closed_quote = false;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                closed_quote = false;
            }
            _ => field.push(character),
        }
    }
    if quoted {
        return Err(TerminologyError::ImportInvalid);
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn serde_value<T: Serialize>(value: T) -> Result<String, TerminologyError> {
    let serialized = serde_json::to_value(value).map_err(|_| TerminologyError::ImportInvalid)?;
    serialized
        .as_str()
        .map(str::to_string)
        .ok_or(TerminologyError::ImportInvalid)
}

fn parse_enum<T: for<'de> Deserialize<'de>>(value: &str) -> Result<T, TerminologyError> {
    serde_json::from_value(Value::String(value.to_string()))
        .map_err(|_| TerminologyError::ImportInvalid)
}

fn parse_u64(value: &str) -> Result<u64, TerminologyError> {
    value
        .parse::<u64>()
        .map_err(|_| TerminologyError::ImportInvalid)
}

use serde_json::Value;
