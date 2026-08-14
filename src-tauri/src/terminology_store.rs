use crate::terminology::{TerminologyError, TerminologyStoreV1};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StoreRecovery {
    BackupRecovered,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoreLoad {
    pub(crate) store: TerminologyStoreV1,
    pub(crate) recovery: Option<StoreRecovery>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StoreFailurePoint {
    TempWrite,
    TempSync,
    BackupStage,
    PromoteRename,
    Rollback,
}

pub(crate) fn terminology_path(app: &AppHandle) -> Result<PathBuf, TerminologyError> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join("terminology.v1.json"))
        .map_err(|_| TerminologyError::StoreIo)
}

#[cfg(test)]
pub(crate) fn load_store_from_path(path: &Path) -> Result<StoreLoad, TerminologyError> {
    load_store_from_path_at(path, now_ms())
}

pub(crate) fn load_store_from_path_at(
    path: &Path,
    now_ms: u64,
) -> Result<StoreLoad, TerminologyError> {
    let main_exists = path.exists();
    let backup = backup_path(path);
    let backup_exists = backup.exists();

    if let Some(store) = read_valid_store(path) {
        return Ok(StoreLoad {
            store,
            recovery: None,
        });
    }
    if let Some(store) = read_valid_store(&backup) {
        if path.exists() {
            fs::remove_file(path).map_err(|_| TerminologyError::StoreIo)?;
        }
        save_store_to_path(path, &store)?;
        return Ok(StoreLoad {
            store,
            recovery: Some(StoreRecovery::BackupRecovered),
        });
    }
    if !main_exists && !backup_exists {
        return Ok(StoreLoad {
            store: TerminologyStoreV1::new(now_ms),
            recovery: None,
        });
    }
    Err(TerminologyError::StoreUnrecoverable)
}

pub(crate) fn save_store_to_path(
    path: &Path,
    store: &TerminologyStoreV1,
) -> Result<(), TerminologyError> {
    save_store_to_path_internal(path, store, None)
}

#[cfg(test)]
pub(crate) fn save_store_to_path_with_failure(
    path: &Path,
    store: &TerminologyStoreV1,
    failure: StoreFailurePoint,
) -> Result<(), TerminologyError> {
    save_store_to_path_internal(path, store, Some(failure))
}

pub(crate) fn reset_store_to_path(
    path: &Path,
    store: &TerminologyStoreV1,
) -> Result<(), TerminologyError> {
    store.validate()?;
    let backup = backup_path(path);
    for existing in [path, backup.as_path()] {
        if existing.exists() && read_valid_store(existing).is_none() {
            fs::remove_file(existing).map_err(|_| TerminologyError::StoreIo)?;
        }
    }
    save_store_to_path(path, store)
}

fn save_store_to_path_internal(
    path: &Path,
    store: &TerminologyStoreV1,
    failure: Option<StoreFailurePoint>,
) -> Result<(), TerminologyError> {
    store.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| TerminologyError::StoreIo)?;
    }
    let bytes = serde_json::to_vec_pretty(store).map_err(|_| TerminologyError::InvalidEntry)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(TerminologyError::StoreIo)?;
    let operation_id = uuid::Uuid::new_v4();
    let temporary = path.with_file_name(format!(".{file_name}.{operation_id}.tmp"));
    let rollback = path.with_file_name(format!(".{file_name}.{operation_id}.rollback"));
    let backup = backup_path(path);

    if failure == Some(StoreFailurePoint::TempWrite) {
        return Err(TerminologyError::StoreIo);
    }
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| TerminologyError::StoreIo)?;
        file.write_all(&bytes)
            .map_err(|_| TerminologyError::StoreIo)?;
        if failure == Some(StoreFailurePoint::TempSync) {
            return Err(TerminologyError::StoreIo);
        }
        file.sync_all().map_err(|_| TerminologyError::StoreIo)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let backup_is_valid = read_valid_store(&backup).is_some();
    let mut prior_backup_staged = false;
    let mut previous_moved = false;
    if path.exists() {
        if failure == Some(StoreFailurePoint::BackupStage) {
            let _ = fs::remove_file(&temporary);
            return Err(TerminologyError::StoreIo);
        }
        if backup.exists() {
            if backup_is_valid {
                if fs::rename(&backup, &rollback).is_err() {
                    let _ = fs::remove_file(&temporary);
                    return Err(TerminologyError::StoreIo);
                }
                prior_backup_staged = true;
            } else if fs::remove_file(&backup).is_err() {
                let _ = fs::remove_file(&temporary);
                return Err(TerminologyError::StoreIo);
            }
        }
        if fs::rename(path, &backup).is_err() {
            if prior_backup_staged {
                let _ = fs::rename(&rollback, &backup);
            }
            let _ = fs::remove_file(&temporary);
            return Err(TerminologyError::StoreIo);
        }
        previous_moved = true;
    }

    if failure == Some(StoreFailurePoint::Rollback) {
        let _ = fs::remove_file(&temporary);
        return Err(TerminologyError::StoreIo);
    }

    if failure == Some(StoreFailurePoint::PromoteRename) {
        if previous_moved {
            let _ = fs::rename(&backup, path);
        }
        if prior_backup_staged {
            let _ = fs::rename(&rollback, &backup);
        }
        let _ = fs::remove_file(&temporary);
        return Err(TerminologyError::StoreIo);
    }

    if fs::rename(&temporary, path).is_err() {
        if previous_moved {
            let _ = fs::rename(&backup, path);
        }
        if prior_backup_staged {
            let _ = fs::rename(&rollback, &backup);
        }
        let _ = fs::remove_file(&temporary);
        return Err(TerminologyError::StoreIo);
    }
    if prior_backup_staged {
        let _ = fs::remove_file(&rollback);
    }

    if read_valid_store(&backup).is_none() {
        if let Err(error) = write_backup_copy(&backup, &bytes) {
            if !previous_moved {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
    }
    Ok(())
}

fn write_backup_copy(path: &Path, bytes: &[u8]) -> Result<(), TerminologyError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(TerminologyError::StoreIo)?;
    let temporary = path.with_file_name(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| TerminologyError::StoreIo)?;
        file.write_all(bytes)
            .map_err(|_| TerminologyError::StoreIo)?;
        file.sync_all().map_err(|_| TerminologyError::StoreIo)?;
        if path.exists() {
            fs::remove_file(path).map_err(|_| TerminologyError::StoreIo)?;
        }
        fs::rename(&temporary, path).map_err(|_| TerminologyError::StoreIo)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_valid_store(path: &Path) -> Option<TerminologyStoreV1> {
    let text = fs::read_to_string(path).ok()?;
    let store = serde_json::from_str::<TerminologyStoreV1>(&text).ok()?;
    store.validate().ok()?;
    Some(store)
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name("terminology.v1.json.bak")
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}
