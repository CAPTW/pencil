use crate::{
    terminology::{
        normalize_match_key, EntryStatus, EntryType, LanguageScope, TerminologyEntry,
        TerminologyEntryDraft, TerminologyError, TerminologyStoreV1,
    },
    terminology_import_export::{
        apply_import_plan, dry_run_import, export_csv, export_json, ImportFormat, ImportPlan,
        ImportReport,
    },
    terminology_store::{
        load_store_from_path_at, reset_store_to_path, save_store_to_path, StoreRecovery,
    },
    terminology_validation::TerminologySuggestion,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

const MAX_IMPORT_PLANS: usize = 8;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeRecoveryCode {
    BackupRecovered,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum TerminologyRuntimeSnapshot {
    Ready {
        store: TerminologyStoreV1,
        recovery: Option<RuntimeRecoveryCode>,
    },
    Unrecoverable {
        reason: &'static str,
    },
    Uninitialized {
        reason: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntrySort {
    SourceText,
    Priority,
    RecentlyUpdated,
    UsageCount,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EntryQuery {
    pub(crate) query: Option<String>,
    pub(crate) profile_id: Option<String>,
    pub(crate) entry_type: Option<EntryType>,
    pub(crate) status: Option<EntryStatus>,
    pub(crate) source_language: Option<LanguageScope>,
    pub(crate) target_language: Option<LanguageScope>,
    pub(crate) sort: EntrySort,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportPlanPreview {
    pub(crate) plan_id: String,
    pub(crate) base_revision: u64,
    pub(crate) expires_at_ms: u64,
    pub(crate) report: ImportReport,
}

pub(crate) struct TerminologyRuntime {
    path: Option<PathBuf>,
    store: Option<TerminologyStoreV1>,
    recovery: Option<RuntimeRecoveryCode>,
    plans: HashMap<String, ImportPlan>,
}

impl Default for TerminologyRuntime {
    fn default() -> Self {
        Self {
            path: None,
            store: None,
            recovery: None,
            plans: HashMap::new(),
        }
    }
}

impl TerminologyRuntime {
    pub(crate) fn open(path: PathBuf, now_ms: u64) -> Self {
        match load_store_from_path_at(&path, now_ms) {
            Ok(loaded) => Self {
                path: Some(path),
                store: Some(loaded.store),
                recovery: loaded.recovery.map(|recovery| match recovery {
                    StoreRecovery::BackupRecovered => RuntimeRecoveryCode::BackupRecovered,
                }),
                plans: HashMap::new(),
            },
            Err(_) => Self {
                path: Some(path),
                store: None,
                recovery: None,
                plans: HashMap::new(),
            },
        }
    }

    pub(crate) fn snapshot(&self) -> TerminologyRuntimeSnapshot {
        match (&self.path, &self.store) {
            (_, Some(store)) => TerminologyRuntimeSnapshot::Ready {
                store: store.clone(),
                recovery: self.recovery,
            },
            (Some(_), None) => TerminologyRuntimeSnapshot::Unrecoverable {
                reason: TerminologyError::StoreUnrecoverable.code(),
            },
            (None, None) => TerminologyRuntimeSnapshot::Uninitialized {
                reason: "terminology_store_uninitialized",
            },
        }
    }

    pub(crate) fn store(&self) -> Result<&TerminologyStoreV1, TerminologyError> {
        self.store.as_ref().ok_or(if self.path.is_some() {
            TerminologyError::StoreUnrecoverable
        } else {
            TerminologyError::StoreIo
        })
    }

    pub(crate) fn reset(&mut self, now_ms: u64) -> Result<(), TerminologyError> {
        let path = self.path.clone().ok_or(TerminologyError::StoreIo)?;
        let next = TerminologyStoreV1::new(now_ms);
        reset_store_to_path(&path, &next)?;
        self.store = Some(next);
        self.recovery = None;
        self.plans.clear();
        Ok(())
    }

    pub(crate) fn add_profile(
        &mut self,
        id: String,
        name: String,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| store.add_profile(id, name, now_ms))
    }

    pub(crate) fn rename_profile(
        &mut self,
        profile_id: &str,
        name: String,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| store.rename_profile(profile_id, name, now_ms))
    }

    pub(crate) fn set_profile_enabled(
        &mut self,
        profile_id: &str,
        enabled: bool,
        active_profile_id: &str,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| {
            store.set_profile_enabled(profile_id, enabled, active_profile_id, now_ms)
        })
    }

    pub(crate) fn add_entry(
        &mut self,
        id: String,
        draft: TerminologyEntryDraft,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| store.add_entry(id, draft, now_ms))
    }

    pub(crate) fn update_entry(
        &mut self,
        id: &str,
        draft: TerminologyEntryDraft,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| store.update_entry(id, draft, now_ms))
    }

    pub(crate) fn set_entry_status(
        &mut self,
        id: &str,
        status: EntryStatus,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        self.mutate(|store| store.set_entry_status(id, status, now_ms))
    }

    pub(crate) fn delete_entry(&mut self, id: &str) -> Result<(), TerminologyError> {
        self.mutate(|store| store.delete_entry(id))
    }

    pub(crate) fn save_suggestion(
        &mut self,
        id: String,
        profile_id: String,
        suggestion: TerminologySuggestion,
        now_ms: u64,
    ) -> Result<(), TerminologyError> {
        suggestion.validate()?;
        self.add_entry(id, suggestion.into_suggested_draft(profile_id), now_ms)
    }

    pub(crate) fn increment_usage(
        &mut self,
        entry_ids: &[String],
        now_ms: u64,
    ) -> Result<bool, TerminologyError> {
        let mut changed = false;
        self.mutate(|store| {
            changed = store.increment_usage(entry_ids, now_ms)?;
            Ok(())
        })?;
        Ok(changed)
    }

    pub(crate) fn validate_active_profile(&self, profile_id: &str) -> Result<(), TerminologyError> {
        if self.store()?.enabled_profile(profile_id).is_some() {
            Ok(())
        } else {
            Err(TerminologyError::ActiveProfileRequired)
        }
    }

    pub(crate) fn fallback_active_profile_id(&self) -> Option<String> {
        self.store
            .as_ref()?
            .profiles
            .iter()
            .find(|profile| profile.id == crate::terminology::GENERAL_PROFILE_ID && profile.enabled)
            .or_else(|| {
                self.store.as_ref()?.profiles.iter().find(|profile| {
                    profile.id != crate::terminology::GLOBAL_PROFILE_ID && profile.enabled
                })
            })
            .map(|profile| profile.id.clone())
    }

    pub(crate) fn query_entries(
        &self,
        query: &EntryQuery,
    ) -> Result<Vec<TerminologyEntry>, TerminologyError> {
        let search_key = query
            .query
            .as_deref()
            .map(|value| {
                if value.chars().count() > 256 || value.contains(['\r', '\n']) {
                    Err(TerminologyError::InvalidTerm)
                } else {
                    normalize_match_key(value, false)
                }
            })
            .transpose()?;
        let mut entries = self
            .store()?
            .entries
            .iter()
            .filter(|entry| {
                query
                    .profile_id
                    .as_deref()
                    .is_none_or(|profile_id| entry.profile_id == profile_id)
                    && query
                        .entry_type
                        .is_none_or(|value| entry.entry_type == value)
                    && query.status.is_none_or(|value| entry.status == value)
                    && query
                        .source_language
                        .is_none_or(|value| entry.source_language == value)
                    && query
                        .target_language
                        .is_none_or(|value| entry.target_language == value)
                    && search_key
                        .as_deref()
                        .is_none_or(|key| entry_contains(entry, key))
            })
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| match query.sort {
            EntrySort::SourceText => normalized_source(left)
                .cmp(&normalized_source(right))
                .then_with(|| left.id.cmp(&right.id)),
            EntrySort::Priority => right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.id.cmp(&right.id)),
            EntrySort::RecentlyUpdated => right
                .updated_at_ms
                .cmp(&left.updated_at_ms)
                .then_with(|| left.id.cmp(&right.id)),
            EntrySort::UsageCount => right
                .usage_count
                .cmp(&left.usage_count)
                .then_with(|| left.id.cmp(&right.id)),
        });
        Ok(entries)
    }

    pub(crate) fn export(&self, format: ImportFormat) -> Result<String, TerminologyError> {
        match format {
            ImportFormat::Json => export_json(self.store()?),
            ImportFormat::Csv => export_csv(self.store()?),
        }
    }

    pub(crate) fn dry_run_import(
        &mut self,
        format: ImportFormat,
        text: &str,
        now_ms: u64,
    ) -> Result<ImportPlanPreview, TerminologyError> {
        self.plans.retain(|_, plan| plan.expires_at_ms >= now_ms);
        if self.plans.len() >= MAX_IMPORT_PLANS {
            if let Some(oldest) = self
                .plans
                .iter()
                .min_by(|left, right| {
                    left.1
                        .expires_at_ms
                        .cmp(&right.1.expires_at_ms)
                        .then_with(|| left.0.cmp(right.0))
                })
                .map(|(plan_id, _)| plan_id.clone())
            {
                self.plans.remove(&oldest);
            }
        }
        let plan = dry_run_import(
            &self.import_base_store(now_ms)?,
            format,
            text,
            uuid::Uuid::new_v4().to_string(),
            now_ms,
        )?;
        let preview = ImportPlanPreview {
            plan_id: plan.plan_id.clone(),
            base_revision: plan.base_revision,
            expires_at_ms: plan.expires_at_ms,
            report: plan.report.clone(),
        };
        self.plans.insert(plan.plan_id.clone(), plan);
        Ok(preview)
    }

    pub(crate) fn apply_import(
        &mut self,
        plan_id: &str,
        now_ms: u64,
    ) -> Result<ImportReport, TerminologyError> {
        let plan = self
            .plans
            .get(plan_id)
            .cloned()
            .ok_or(TerminologyError::StaleImportPlan)?;
        if now_ms > plan.expires_at_ms {
            self.plans.remove(plan_id);
            return Err(TerminologyError::ImportPlanExpired);
        }
        self.plans.retain(|candidate_id, candidate| {
            candidate_id == plan_id || candidate.expires_at_ms >= now_ms
        });
        let current = self.import_base_store(now_ms)?;
        let next = apply_import_plan(&current, &plan, now_ms)?;
        let report = plan.report.clone();
        if self.store.as_ref() != Some(&next) {
            self.persist(next)?;
        }
        self.plans.remove(plan_id);
        Ok(report)
    }

    #[cfg(test)]
    pub(crate) fn apply_import_with_failure(
        &mut self,
        plan_id: &str,
        now_ms: u64,
        failure: crate::terminology_store::StoreFailurePoint,
    ) -> Result<ImportReport, TerminologyError> {
        let plan = self
            .plans
            .get(plan_id)
            .cloned()
            .ok_or(TerminologyError::StaleImportPlan)?;
        if now_ms > plan.expires_at_ms {
            self.plans.remove(plan_id);
            return Err(TerminologyError::ImportPlanExpired);
        }
        let next = apply_import_plan(self.store()?, &plan, now_ms)?;
        let report = plan.report.clone();
        if self.store()? != &next {
            let path = self.path.as_ref().ok_or(TerminologyError::StoreIo)?;
            crate::terminology_store::save_store_to_path_with_failure(path, &next, failure)?;
            self.store = Some(next);
            self.recovery = None;
        }
        self.plans.remove(plan_id);
        Ok(report)
    }

    fn mutate(
        &mut self,
        mutation: impl FnOnce(&mut TerminologyStoreV1) -> Result<(), TerminologyError>,
    ) -> Result<(), TerminologyError> {
        let mut next = self.store()?.clone();
        mutation(&mut next)?;
        if self.store()? == &next {
            return Ok(());
        }
        self.persist(next)
    }

    fn import_base_store(&self, now_ms: u64) -> Result<TerminologyStoreV1, TerminologyError> {
        match (&self.path, &self.store) {
            (_, Some(store)) => Ok(store.clone()),
            (Some(_), None) => Ok(TerminologyStoreV1::new(now_ms)),
            (None, None) => Err(TerminologyError::StoreIo),
        }
    }

    fn persist(&mut self, next: TerminologyStoreV1) -> Result<(), TerminologyError> {
        let path = self.path.as_ref().ok_or(TerminologyError::StoreIo)?;
        save_store_to_path(path, &next)?;
        self.store = Some(next);
        self.recovery = None;
        self.plans.clear();
        Ok(())
    }
}

fn normalized_source(entry: &TerminologyEntry) -> String {
    normalize_match_key(&entry.source_text, false).unwrap_or_default()
}

fn entry_contains(entry: &TerminologyEntry, query: &str) -> bool {
    std::iter::once(entry.source_text.as_str())
        .chain(entry.preferred_text.as_deref())
        .chain(entry.aliases.iter().map(String::as_str))
        .chain(entry.note.as_deref())
        .any(|value| {
            normalize_match_key(value, false)
                .ok()
                .is_some_and(|value| value.contains(query))
        })
}
