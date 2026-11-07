use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{error::AppError, resources::local_file_status};

pub const BACKUP_FORMAT_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_DESTINATION_CHARS: usize = 2_048;
pub const BACKUP_PREFIX: &str = "csir-net-tracker";
pub const MANIFEST_SUFFIX: &str = ".manifest.json";
pub const POST_MIGRATION_REASON: &str = "post-migration";
pub const PRE_MIGRATION_REASON: &str = "pre-migration";
pub const PRE_RESTORE_REASON: &str = "pre-restore";
pub const DAILY_AUTOMATIC_RETENTION: i64 = 14;
const RESTORE_OPERATION_FILE: &str = "restore-operation.json";
const MIGRATION_STATE_FILE: &str = "migration-state.json";
const MIGRATION_ORIGIN_FRESH: &str = "fresh";
const MIGRATION_ORIGIN_PENDING: &str = "migration_pending";
const MIGRATION_ORIGIN_UPGRADED: &str = "upgraded";

/// Core tables present in JSON format 1 and every backup schema.
pub const CORE_BACKUP_TABLES: [&str; 16] = [
    "settings",
    "syllabus_sources",
    "syllabus_sections",
    "syllabus_topics",
    "syllabus_progress",
    "timer_settings",
    "timer_state",
    "study_sessions",
    "checklist_tasks",
    "planned_study_blocks",
    "pyq_attempts",
    "mock_tests",
    "revision_settings",
    "revision_schedules",
    "revision_history",
    "dashboard_sections",
];

/// Every table counted for a current-schema backup, including the exam
/// profiles and the four study resource tables. Older schemas are counted only
/// for tables that actually exist.
pub const BACKUP_TABLES: [&str; 27] = [
    "exams",
    "exam_profiles",
    "settings",
    "syllabus_sources",
    "syllabus_sections",
    "syllabus_topics",
    "syllabus_progress",
    "timer_settings",
    "timer_state",
    "study_sessions",
    "checklist_tasks",
    "planned_study_blocks",
    "pyq_attempts",
    "mock_tests",
    "revision_settings",
    "revision_schedules",
    "revision_history",
    "revision_plan_items",
    "revision_daily_plans",
    "revision_daily_plan_items",
    "revision_deferrals",
    "revision_session_links",
    "dashboard_sections",
    "study_resources",
    "resource_progress",
    "resource_local_files",
    "resource_discovery_runs",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub format_version: u32,
    pub application_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub database_id: String,
    pub reason: String,
    pub backup_file_name: String,
    pub backup_size_bytes: u64,
    pub sha256: String,
    pub integrity_check: String,
    /// Optional for compatibility with manifests written before v1.2.0. New
    /// manifests include counts for all core and resource tables.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub record_counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub automatic_enabled: bool,
    pub frequency: String,
    pub destination: Option<String>,
    pub retention_count: i64,
    pub last_success_at: Option<String>,
    pub last_success_file: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_failure_message: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettingsInput {
    pub automatic_enabled: bool,
    pub frequency: String,
    pub destination: Option<String>,
    pub retention_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoverySnapshot {
    pub kind: String,
    pub reason: String,
    pub file_name: String,
    pub directory: String,
    pub path: String,
    pub schema_version: i64,
    pub created_at: String,
    pub database_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOverview {
    pub settings: BackupSettings,
    pub default_directory: String,
    pub schema_version: i64,
    pub application_version: String,
    pub database_id: String,
    pub recovery_snapshots: Vec<RecoverySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOperation {
    pub token: String,
    pub stage: String,
    pub source_backup_path: String,
    pub source_schema_version: i64,
    pub safety_backup_path: String,
    pub safety_schema_version: i64,
    pub live_path: String,
    pub rollback_path: String,
    pub temp_path: String,
    pub started_at: String,
    pub updated_at: String,
    /// Absent in older sidecars created before this correction.
    #[serde(default)]
    pub database_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRecovery {
    pub stage: String,
    pub safety_file_name: Option<String>,
    pub safety_path: Option<String>,
    pub safety_schema_version: Option<i64>,
    pub safety_verified: bool,
    pub live_path: String,
    pub live_valid: bool,
    /// `valid`, `newer_schema`, `invalid`, or `missing`.
    pub live_status: String,
    pub source_backup_path: Option<String>,
    pub rollback_files: Vec<String>,
    pub temp_files: Vec<String>,
    pub deletion_files: Vec<String>,
    pub keep_allowed: bool,
    pub keep_blocked_reason: Option<String>,
    pub leftover_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveDatabaseState {
    Valid,
    NewerSchema,
    Invalid,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepRecoveryResult {
    pub removed_files: Vec<String>,
    pub preserved_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationState {
    pub schema_version: i64,
    pub origin: String,
    #[serde(default)]
    pub post_migration_snapshot: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub file_name: String,
    pub directory: String,
    pub manifest: BackupManifest,
    pub removed_old_backups: i64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupOutcome {
    pub attempted: bool,
    pub success: bool,
    pub message: String,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileRestoreSummary {
    pub registered: i64,
    pub available: i64,
    pub missing: i64,
    pub changed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedBackup {
    pub manifest: BackupManifest,
    pub record_counts: BTreeMap<String, i64>,
    pub local_files: Option<LocalFileRestoreSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub manifest: BackupManifest,
    pub record_counts: BTreeMap<String, i64>,
    pub local_files: Option<LocalFileRestoreSummary>,
    pub current_schema_version: i64,
    pub requires_upgrade: bool,
    pub warnings: Vec<String>,
}

pub fn default_backup_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

pub fn safety_backup_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("backups").join("safety")
}

fn migration_state_path(data_dir: &Path) -> PathBuf {
    data_dir.join("backups").join(MIGRATION_STATE_FILE)
}

fn canonical_for_comparison(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let parent = path.parent()?.canonicalize().ok()?;
    Some(parent.join(path.file_name()?))
}

fn normalized_path(path: &Path) -> String {
    canonical_for_comparison(path)
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase()
}

fn paths_match(left: &Path, right: &Path) -> bool {
    normalized_path(left) == normalized_path(right)
}

fn path_is_within(path: &Path, directory: &Path) -> bool {
    let path = normalized_path(path);
    let directory = normalized_path(directory);
    path == directory
        || path.starts_with(&format!(
            "{}{}",
            directory.trim_end_matches('\\'),
            std::path::MAIN_SEPARATOR
        ))
}

pub(crate) fn restore_candidate_path_allowed(data_dir: &Path, path: &Path, suffix: &str) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.parent()
        .is_some_and(|parent| paths_match(parent, data_dir))
        && name.starts_with(".csir-net-tracker-restore-")
        && name.ends_with(suffix)
}

fn safety_path_allowed(data_dir: &Path, path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    path_is_within(path, &safety_backup_directory(data_dir))
        && name.starts_with(&format!("{BACKUP_PREFIX}-{PRE_RESTORE_REASON}-"))
        && name.ends_with(".sqlite3")
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::file_operation("The recovery metadata folder could not be created", error)
        })?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| AppError::internal("Recovery metadata could not be encoded."))?;
    let temp = path.with_extension(format!("json.{}.tmp", Uuid::new_v4().simple()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|error| {
            AppError::file_operation("Recovery metadata could not be prepared", error)
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_all())
        .map_err(|error| {
            let _ = fs::remove_file(&temp);
            AppError::file_operation("Recovery metadata could not be flushed", error)
        })?;
    drop(file);
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        AppError::file_operation("Recovery metadata could not be moved into place", error)
    })?;
    // Directory fsync is best effort on Windows; the file contents above are
    // flushed before the atomic replacement.
    if let Some(parent) = path.parent()
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}

pub fn read_migration_state(data_dir: &Path) -> Option<MigrationState> {
    let bytes = fs::read(migration_state_path(data_dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_migration_state(data_dir: &Path, state: &MigrationState) -> Result<(), AppError> {
    write_json_atomic(&migration_state_path(data_dir), state)
}

pub fn record_fresh_install(data_dir: &Path) -> Result<(), AppError> {
    write_migration_state(
        data_dir,
        &MigrationState {
            schema_version: 12,
            origin: MIGRATION_ORIGIN_FRESH.to_owned(),
            post_migration_snapshot: None,
            updated_at: Utc::now().to_rfc3339(),
        },
    )
}

pub fn record_migration_pending(data_dir: &Path) -> Result<(), AppError> {
    write_migration_state(
        data_dir,
        &MigrationState {
            schema_version: 12,
            origin: MIGRATION_ORIGIN_PENDING.to_owned(),
            post_migration_snapshot: None,
            updated_at: Utc::now().to_rfc3339(),
        },
    )
}

pub fn is_migration_pending(data_dir: &Path) -> bool {
    read_migration_state(data_dir).is_some_and(|state| state.origin == MIGRATION_ORIGIN_PENDING)
}

pub fn record_upgraded_install(data_dir: &Path, manifest: &BackupManifest) -> Result<(), AppError> {
    write_migration_state(
        data_dir,
        &MigrationState {
            schema_version: manifest.schema_version,
            origin: MIGRATION_ORIGIN_UPGRADED.to_owned(),
            post_migration_snapshot: Some(manifest.backup_file_name.clone()),
            updated_at: Utc::now().to_rfc3339(),
        },
    )
}

pub fn is_fresh_install(data_dir: &Path) -> bool {
    let fresh_marker =
        read_migration_state(data_dir).is_some_and(|state| state.origin == MIGRATION_ORIGIN_FRESH);
    // A pre-migration artifact means an upgrade was attempted. Treat a stale
    // `fresh` marker as untrusted rather than skipping the required repair.
    fresh_marker && !has_pre_migration_artifact(data_dir)
}

fn has_pre_migration_artifact(data_dir: &Path) -> bool {
    backup_files_with_prefix(
        &data_dir.join("backups"),
        &format!("{BACKUP_PREFIX}-{PRE_MIGRATION_REASON}-"),
    )
    .into_iter()
    .any(|path| path.is_file())
}

fn backup_files_with_prefix(directory: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sqlite3"))
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[cfg(test)]
pub fn latest_pre_migration_backup(data_dir: &Path) -> Option<PathBuf> {
    let prefix = format!("{BACKUP_PREFIX}-{PRE_MIGRATION_REASON}-");
    backup_files_with_prefix(&data_dir.join("backups"), &prefix)
        .into_iter()
        .rfind(|path| verify_backup(path).is_ok())
}

pub fn latest_post_migration_backup(data_dir: &Path) -> Option<PathBuf> {
    let prefix = format!("{BACKUP_PREFIX}-{POST_MIGRATION_REASON}-");
    backup_files_with_prefix(&safety_backup_directory(data_dir), &prefix)
        .into_iter()
        .rfind(|path| verify_backup(path).is_ok())
}

pub fn post_migration_snapshot_required(data_dir: &Path) -> bool {
    // Only a genuine fresh installation may skip the post-migration snapshot.
    // `migration_pending`, `upgraded`, missing, and corrupt markers all require
    // a valid post-migration snapshot or repair.
    if is_fresh_install(data_dir) {
        return false;
    }
    // A valid post-migration snapshot is sufficient even if the marker is
    // missing or its recorded filename is stale. This preserves an existing
    // valid snapshot instead of creating duplicates. Without a valid snapshot,
    // repair is required regardless of whether the pre-migration artifact
    // still exists or verifies.
    latest_post_migration_backup(data_dir).is_none()
}

pub fn create_post_migration_backup(
    connection: &Connection,
    data_dir: &Path,
) -> Result<BackupResult, AppError> {
    create_backup(
        connection,
        &safety_backup_directory(data_dir),
        POST_MIGRATION_REASON,
        None,
    )
}

pub fn enable_daily_automatic_backups(
    connection: &Connection,
    retention_count: i64,
) -> Result<(), AppError> {
    let retention = retention_count.clamp(1, 100);
    connection
        .execute(
            "UPDATE backup_settings
             SET automatic_enabled = 1,
                 frequency = 'daily',
                 retention_count = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            [retention],
        )
        .map_err(|error| {
            AppError::database(
                "Daily automatic backups could not be enabled after migration",
                error,
            )
        })?;
    Ok(())
}

pub fn clear_pre_migration_last_success(connection: &Connection) -> Result<(), AppError> {
    let prefix = format!("{BACKUP_PREFIX}-{PRE_MIGRATION_REASON}-");
    connection
        .execute(
            "UPDATE backup_settings
             SET last_success_at = NULL,
                 last_success_file = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1
               AND last_success_file LIKE ?1",
            [format!("{prefix}%")],
        )
        .map_err(|error| {
            AppError::database(
                "A pre-migration backup could not be cleared from last-success state",
                error,
            )
        })?;
    Ok(())
}

pub fn list_recovery_snapshots(data_dir: &Path) -> Vec<RecoverySnapshot> {
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    let pre_prefix = format!("{BACKUP_PREFIX}-{PRE_MIGRATION_REASON}-");
    let post_prefix = format!("{BACKUP_PREFIX}-{POST_MIGRATION_REASON}-");
    let restore_prefix = format!("{BACKUP_PREFIX}-{PRE_RESTORE_REASON}-");
    for path in backup_files_with_prefix(&data_dir.join("backups"), &pre_prefix) {
        candidates.push((PRE_MIGRATION_REASON.to_owned(), path));
    }
    for path in backup_files_with_prefix(&safety_backup_directory(data_dir), &post_prefix) {
        candidates.push((POST_MIGRATION_REASON.to_owned(), path));
    }
    for path in backup_files_with_prefix(&safety_backup_directory(data_dir), &restore_prefix) {
        candidates.push((PRE_RESTORE_REASON.to_owned(), path));
    }

    let mut snapshots = Vec::new();
    for (reason, path) in candidates {
        let Ok(verified) = verify_backup(&path) else {
            continue;
        };
        let kind = match reason.as_str() {
            PRE_MIGRATION_REASON => "pre_migration",
            POST_MIGRATION_REASON => "post_migration",
            PRE_RESTORE_REASON => "pre_restore",
            _ => "recovery",
        };
        snapshots.push(RecoverySnapshot {
            kind: kind.to_owned(),
            reason,
            file_name: verified.manifest.backup_file_name,
            directory: path
                .parent()
                .map(|parent| parent.display().to_string())
                .unwrap_or_default(),
            path: path.display().to_string(),
            schema_version: verified.manifest.schema_version,
            created_at: verified.manifest.created_at,
            database_id: verified.manifest.database_id,
        });
    }
    snapshots.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    snapshots
}

fn restore_operation_path(data_dir: &Path) -> PathBuf {
    data_dir.join("backups").join(RESTORE_OPERATION_FILE)
}

pub fn write_restore_operation(
    data_dir: &Path,
    operation: &RestoreOperation,
) -> Result<(), AppError> {
    write_json_atomic(&restore_operation_path(data_dir), operation)
}

pub fn read_restore_operation(data_dir: &Path) -> Option<RestoreOperation> {
    let bytes = fs::read(restore_operation_path(data_dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn update_restore_stage(data_dir: &Path, stage: &str) -> Result<(), AppError> {
    let mut operation = read_restore_operation(data_dir).ok_or_else(|| {
        AppError::restore_failed(
            "The restore-operation metadata is missing, so the restore stage cannot be recorded safely.",
        )
    })?;
    operation.stage = stage.to_owned();
    operation.updated_at = chrono::Utc::now().to_rfc3339();
    write_restore_operation(data_dir, &operation)
}

pub fn clear_restore_operation(data_dir: &Path) -> Result<(), AppError> {
    let path = restore_operation_path(data_dir);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::file_operation(
            "The restore-operation metadata could not be cleared",
            error,
        )),
    }
}

fn existing_restore_candidate(data_dir: &Path, value: &str, suffix: &str) -> Option<String> {
    let path = PathBuf::from(value);
    (restore_candidate_path_allowed(data_dir, &path, suffix) && path.is_file())
        .then(|| path.display().to_string())
}

fn safety_is_usable(
    data_dir: &Path,
    operation: &RestoreOperation,
    current_schema: i64,
) -> (Option<PathBuf>, Option<i64>, bool) {
    let safety = PathBuf::from(&operation.safety_backup_path);
    if !safety_path_allowed(data_dir, &safety) {
        return (None, None, false);
    }
    let Ok(verified) = verify_backup(&safety) else {
        return (Some(safety), None, false);
    };
    let schema = verified.manifest.schema_version;
    let identity_ok = operation
        .database_id
        .as_deref()
        .is_none_or(|expected| expected == verified.manifest.database_id);
    let valid = (1..=current_schema).contains(&schema) && identity_ok;
    (Some(safety), Some(schema), valid)
}

fn database_sidecars_present(path: &Path) -> bool {
    ["-wal", "-shm", "-journal"]
        .iter()
        .any(|suffix| snapshot_sidecar_path(path, suffix).exists())
}

fn database_identity_from_file(path: &Path) -> Option<String> {
    let sidecars = ["-wal", "-shm"]
        .iter()
        .map(|suffix| snapshot_sidecar_path(path, suffix))
        .collect::<Vec<_>>();
    let existed = sidecars
        .iter()
        .map(|sidecar| sidecar.exists())
        .collect::<Vec<_>>();
    let connection = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;
    let identity = database_identifier(&connection).ok().flatten();
    drop(connection);
    for (sidecar, existed) in sidecars.iter().zip(existed) {
        if !existed {
            let _ = fs::remove_file(sidecar);
        }
    }
    identity
}

fn temp_candidate_is_source_copy(operation: &RestoreOperation, temp: &Path) -> bool {
    if database_sidecars_present(temp) {
        return false;
    }
    let source = Path::new(&operation.source_backup_path);
    let Ok(verified) = verify_backup(source) else {
        return false;
    };
    let Ok(actual_hash) = sha256_file(temp) else {
        return false;
    };
    verified.manifest.sha256 == actual_hash
}

pub fn restore_safety_path(data_dir: &Path) -> Option<PathBuf> {
    let operation = read_restore_operation(data_dir)?;
    let path = PathBuf::from(&operation.safety_backup_path);
    if !safety_path_allowed(data_dir, &path) {
        return None;
    }
    let verified = verify_backup(&path).ok()?;
    if operation
        .database_id
        .as_deref()
        .is_some_and(|expected| expected != verified.manifest.database_id)
    {
        return None;
    }
    Some(path)
}

fn leftover_restore_files(data_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(data_dir) {
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".csir-net-tracker-restore-")
                && (name.ends_with(".rollback")
                    || name.ends_with(".tmp")
                    || name.ends_with(".failed"))
            {
                files.push(entry.path().display().to_string());
            }
        }
    }
    files
}

pub fn restore_recovery(data_dir: &Path, live_path: &Path, current_schema: i64) -> RestoreRecovery {
    let operation = read_restore_operation(data_dir);
    let live_state = live_database_state(live_path, current_schema);
    let live_valid = live_state == LiveDatabaseState::Valid;
    let live_status = live_state_name(live_state).to_owned();
    let leftover_files = leftover_restore_files(data_dir);

    let Some(operation) = operation else {
        return RestoreRecovery {
            stage: "unknown".to_owned(),
            safety_file_name: None,
            safety_path: None,
            safety_schema_version: None,
            safety_verified: false,
            live_path: live_path.display().to_string(),
            live_valid,
            live_status,
            source_backup_path: None,
            rollback_files: Vec::new(),
            temp_files: Vec::new(),
            deletion_files: Vec::new(),
            keep_allowed: false,
            keep_blocked_reason: Some(
                "The restore-operation metadata is missing, so no recovery candidate can be identified safely."
                    .to_owned(),
            ),
            leftover_files,
        };
    };

    let live_matches = paths_match(Path::new(&operation.live_path), live_path);
    let rollback_recorded = PathBuf::from(&operation.rollback_path);
    let temp_recorded = PathBuf::from(&operation.temp_path);
    let recorded_paths_valid =
        restore_candidate_path_allowed(data_dir, &rollback_recorded, ".rollback")
            && restore_candidate_path_allowed(data_dir, &temp_recorded, ".tmp")
            && safety_path_allowed(data_dir, Path::new(&operation.safety_backup_path));
    let (safety_path, safety_schema_version, safety_verified) =
        safety_is_usable(data_dir, &operation, current_schema);
    let safety_file_name = safety_path.as_ref().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    let rollback_files =
        existing_restore_candidate(data_dir, &operation.rollback_path, ".rollback")
            .into_iter()
            .collect::<Vec<_>>();
    let temp_files = existing_restore_candidate(data_dir, &operation.temp_path, ".tmp")
        .into_iter()
        .collect::<Vec<_>>();
    let rollback_proven = rollback_files.is_empty()
        || operation.database_id.as_deref().is_some_and(|expected| {
            let rollback = Path::new(&operation.rollback_path);
            !database_sidecars_present(rollback)
                && database_identity_from_file(rollback).as_deref() == Some(expected)
        });
    let temp_proven = temp_files.is_empty()
        || temp_candidate_is_source_copy(&operation, Path::new(&operation.temp_path));
    let candidates_proven = rollback_proven && temp_proven;
    let candidates_exist = !rollback_files.is_empty() || !temp_files.is_empty();
    let known_candidate_paths = [&operation.rollback_path, &operation.temp_path]
        .into_iter()
        .map(|value| normalized_path(Path::new(value)))
        .collect::<HashSet<_>>();
    let unexplained_files = leftover_files
        .iter()
        .filter(|file| !known_candidate_paths.contains(&normalized_path(Path::new(file))))
        .count();
    let safe_stage = matches!(
        operation.stage.as_str(),
        "opened" | "recovered_original" | "recovered_safety"
    );
    let keep_allowed = live_valid
        && live_matches
        && recorded_paths_valid
        && unexplained_files == 0
        && candidates_proven
        && (!candidates_exist || (safe_stage && safety_verified));
    let keep_blocked_reason = if !live_matches {
        Some(
            "The database path recorded by the interrupted restore does not match the live database. All recovery files are being preserved."
                .to_owned(),
        )
    } else if !recorded_paths_valid {
        Some(
            "The restore-operation metadata refers to a path outside the protected application recovery locations. All recovery files are being preserved."
                .to_owned(),
        )
    } else if live_state == LiveDatabaseState::NewerSchema {
        Some(
            "The live database uses a newer schema than this application supports. It has not been replaced; use the newer application version or make an explicit recovery decision."
                .to_owned(),
        )
    } else if !live_valid {
        Some(
            "The live database could not be verified. All recovery files are being preserved."
                .to_owned(),
        )
    } else if unexplained_files > 0 {
        Some(
            "Unidentified restore files remain in the application-data folder. Recovery metadata is being preserved until they are reviewed."
                .to_owned(),
        )
    } else if candidates_exist && !safe_stage {
        Some(
            "The live database may be the installed restore candidate, not the completed database. The rollback/temporary candidates are being preserved."
                .to_owned(),
        )
    } else if candidates_exist && !candidates_proven {
        Some(
            "The preserved rollback/temporary candidates could not be proven to be redundant copies. They are being preserved."
                .to_owned(),
        )
    } else if candidates_exist && !safety_verified {
        Some(
            "The pre-restore safety snapshot is missing or invalid. Deleting the rollback/temporary candidates could remove the only remaining copy of the newer data."
                .to_owned(),
        )
    } else {
        None
    };
    let deletion_files = if keep_allowed && safe_stage && safety_verified {
        rollback_files
            .iter()
            .filter(|_| rollback_proven)
            .chain(temp_files.iter().filter(|_| temp_proven))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    RestoreRecovery {
        stage: operation.stage,
        safety_file_name,
        safety_path: safety_path.map(|path| path.display().to_string()),
        safety_schema_version,
        safety_verified,
        live_path: live_path.display().to_string(),
        live_valid,
        live_status,
        source_backup_path: Some(operation.source_backup_path),
        rollback_files,
        temp_files,
        deletion_files,
        keep_allowed,
        keep_blocked_reason,
        leftover_files,
    }
}

pub fn keep_restore_recovery(
    data_dir: &Path,
    live_path: &Path,
    current_schema: i64,
    confirmed: bool,
) -> Result<KeepRecoveryResult, AppError> {
    if !confirmed {
        return Err(AppError::restore_failed(
            "Confirm that the listed recovery candidates may be deleted before keeping the live database.",
        ));
    }
    let recovery = restore_recovery(data_dir, live_path, current_schema);
    if !recovery.keep_allowed {
        return Err(AppError::restore_failed(
            recovery.keep_blocked_reason.as_deref().unwrap_or(
                "The live database and preserved recovery candidates cannot be identified safely.",
            ),
        ));
    }

    let mut removed_files = Vec::new();
    for file in &recovery.deletion_files {
        let path = PathBuf::from(file);
        let suffix = if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".rollback"))
        {
            ".rollback"
        } else {
            ".tmp"
        };
        if !restore_candidate_path_allowed(data_dir, &path, suffix) {
            return Err(AppError::restore_failed(
                "A recovery candidate is outside the allowed application-data location. No metadata was cleared.",
            ));
        }
        match fs::remove_file(&path) {
            Ok(()) => removed_files.push(path.display().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::file_operation(
                    "A preserved recovery candidate could not be deleted. No metadata was cleared.",
                    error,
                ));
            }
        }
    }
    for file in &recovery.deletion_files {
        if Path::new(file).exists() {
            return Err(AppError::restore_failed(
                "A preserved recovery candidate could not be deleted. No metadata was cleared.",
            ));
        }
    }
    clear_restore_operation(data_dir)?;
    let preserved_files = leftover_restore_files(data_dir);
    Ok(KeepRecoveryResult {
        removed_files,
        preserved_files,
    })
}

pub(crate) fn preserve_database_with_sidecars(
    source: &Path,
    destination: &Path,
) -> Result<(), AppError> {
    if destination.exists() {
        return Err(AppError::restore_failed(
            "The preserved-database destination is already occupied. All recovery files were kept.",
        ));
    }
    fs::copy(source, destination).map_err(|error| {
        AppError::file_operation(
            "The database could not be copied to its protected recovery path. All recovery files were kept.",
            error,
        )
    })?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let source_sidecar = snapshot_sidecar_path(source, suffix);
        if source_sidecar.exists() {
            let destination_sidecar = snapshot_sidecar_path(destination, suffix);
            if let Err(error) = fs::copy(&source_sidecar, &destination_sidecar) {
                return Err(AppError::file_operation(
                    "A database sidecar could not be copied to its protected recovery path. All recovery files were kept.",
                    error,
                ));
            }
        }
    }
    for path in [
        destination.to_path_buf(),
        snapshot_sidecar_path(destination, "-wal"),
        snapshot_sidecar_path(destination, "-shm"),
        snapshot_sidecar_path(destination, "-journal"),
    ] {
        if path.exists()
            && let Err(error) = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .and_then(|file| file.sync_all())
        {
            return Err(AppError::file_operation(
                "The protected database copy could not be flushed before detaching the live file. All recovery files were kept.",
                error,
            ));
        }
    }
    // Keep the copies isolated from the live path before installing a
    // replacement. The copies are the recovery artifact; removing the source
    // only after all copies succeed keeps a complete path on every failure.
    fs::remove_file(source).map_err(|error| {
        AppError::file_operation(
            "The original live database could not be detached after it was copied. All recovery files were kept.",
            error,
        )
    })?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = snapshot_sidecar_path(source, suffix);
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::file_operation(
                    "A live database sidecar could not be detached after it was copied. All recovery files were kept.",
                    error,
                ));
            }
        }
    }
    Ok(())
}

fn remove_live_sidecars(path: &Path) -> std::io::Result<()> {
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        match fs::remove_file(PathBuf::from(sidecar)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn preserve_invalid_live_database(
    data_dir: &Path,
    operation: &RestoreOperation,
    current_path: &Path,
) -> Result<(), AppError> {
    if !current_path.exists() {
        return Ok(());
    }
    // Never reuse operation.temp_path for this. A temp path is treated as a
    // disposable restore candidate; a displaced live database is a protected
    // recovery artifact.
    let preserved = data_dir.join(format!(
        ".csir-net-tracker-restore-{}-{}.sqlite3.failed",
        operation.token,
        Uuid::new_v4().simple()
    ));
    if preserved.exists() {
        return Err(AppError::restore_failed(
            "An invalid live database was detected, but its preservation path is already occupied. All recovery files were kept.",
        ));
    }
    preserve_database_with_sidecars(current_path, &preserved)
}

pub fn recover_interrupted_restore_before_open(
    data_dir: &Path,
    current_path: &Path,
    current_schema: i64,
) -> Result<(), AppError> {
    let live_state = live_database_state(current_path, current_schema);
    let Some(operation) = read_restore_operation(data_dir) else {
        let leftovers = leftover_restore_files(data_dir);
        if leftovers.is_empty() {
            return Ok(());
        }
        if live_state == LiveDatabaseState::Valid {
            for file in leftovers {
                eprintln!(
                    "An unverified restore leftover file was preserved for manual review: {file}"
                );
            }
            return Ok(());
        }
        if live_state == LiveDatabaseState::NewerSchema {
            return Err(AppError::restore_failed(
                "The live database uses a newer schema than this application supports. It was not replaced. Use the newer application version or make an explicit recovery decision; all recovery files were preserved.",
            ));
        }
        return Err(AppError::restore_failed(
            "An incomplete restore was detected, but its restore-operation metadata is missing or unreadable. All recovery files were preserved for manual review.",
        ));
    };

    if !paths_match(Path::new(&operation.live_path), current_path) {
        return Err(AppError::restore_failed(
            "The live database path does not match the interrupted restore metadata. All recovery files were preserved for manual review.",
        ));
    }

    if live_state == LiveDatabaseState::Valid {
        return Ok(());
    }
    if live_state == LiveDatabaseState::NewerSchema {
        return Err(AppError::restore_failed(
            "The live database uses a newer schema than this application supports. It was not replaced by the older safety snapshot. Use the newer application version or make an explicit recovery decision; all recovery files were preserved.",
        ));
    }

    let safety_path = PathBuf::from(&operation.safety_backup_path);
    if !safety_path_allowed(data_dir, &safety_path) {
        return Err(AppError::restore_failed(
            "The pre-restore safety snapshot is outside the protected application backup folder. All recovery files were preserved for manual review.",
        ));
    }
    let verified = verify_backup(&safety_path).map_err(|_| {
        AppError::restore_failed(
            "An interrupted restore was detected, but its pre-restore safety snapshot is missing or invalid. All recovery files were preserved for manual review.",
        )
    })?;
    if !(1..=current_schema).contains(&verified.manifest.schema_version) {
        return Err(AppError::restore_failed(
            "The pre-restore safety snapshot uses a schema that this application cannot recover. All recovery files were preserved for manual review.",
        ));
    }
    if operation
        .database_id
        .as_deref()
        .is_some_and(|expected| expected != verified.manifest.database_id)
    {
        return Err(AppError::restore_failed(
            "The pre-restore safety snapshot does not match the interrupted restore database identity. All recovery files were preserved for manual review.",
        ));
    }

    preserve_invalid_live_database(data_dir, &operation, current_path)?;
    let recovery_temp = data_dir.join(format!(
        ".csir-net-tracker-restore-{}.sqlite3.recovery.tmp",
        operation.token
    ));
    if recovery_temp.exists() && verify_database_file(&recovery_temp).is_err() {
        // This is a derived copy of the already-verified safety snapshot, so a
        // failed previous copy can be replaced safely.
        fs::remove_file(&recovery_temp).map_err(|error| {
            AppError::file_operation(
                "A failed recovery copy could not be replaced. All recovery files were kept.",
                error,
            )
        })?;
    }
    if !recovery_temp.exists() {
        // The safety snapshot was verified above and contains every committed
        // change, including WAL-backed changes from before the restore began.
        // Copy and verify the replacement before moving it into the live path.
        fs::copy(&safety_path, &recovery_temp).map_err(|error| {
            AppError::file_operation(
                "The pre-restore safety snapshot could not be copied for recovery. All recovery files were kept.",
                error,
            )
        })?;
        verify_database_file(&recovery_temp)?;
    }
    remove_snapshot_sidecars(&recovery_temp);
    remove_live_sidecars(current_path).map_err(|error| {
        AppError::file_operation(
            "The stale live WAL files could not be removed before safety recovery. All recovery files were kept.",
            error,
        )
    })?;
    fs::rename(&recovery_temp, current_path).map_err(|error| {
        AppError::file_operation(
            "The verified recovery copy could not be installed at the live database path. All recovery files were kept.",
            error,
        )
    })?;
    verify_database_file(current_path)?;
    if let Err(error) = update_restore_stage(data_dir, "recovered_safety") {
        eprintln!("The recovered-safety restore stage could not be recorded: {error:?}");
    }
    Ok(())
}

pub fn database_identifier(connection: &Connection) -> Result<Option<String>, AppError> {
    let has_metadata: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'database_metadata'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Database identity could not be checked", error))?;
    if has_metadata == 0 {
        return Ok(None);
    }

    connection
        .query_row(
            "SELECT database_id FROM database_metadata WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Database identity could not be read", error))
}

pub fn ensure_database_identifier(
    connection: &Connection,
    preferred: Option<&str>,
) -> Result<String, AppError> {
    if let Some(existing) = database_identifier(connection)? {
        return Ok(existing);
    }

    let identifier = preferred
        .map(str::to_owned)
        .filter(|value| is_database_id(value))
        .unwrap_or_else(new_database_id);
    connection
        .execute(
            "INSERT INTO database_metadata (id, database_id, created_at)
             VALUES (1, ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(id) DO NOTHING",
            [&identifier],
        )
        .map_err(|error| AppError::database("Database identity could not be stored", error))?;

    database_identifier(connection)?
        .ok_or_else(|| AppError::internal("The new database identity could not be reloaded."))
}

pub fn new_database_id() -> String {
    Uuid::new_v4().simple().to_string()
}

fn is_database_id(value: &str) -> bool {
    value.len() == 32
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value == value.to_lowercase()
}

pub fn load_settings(connection: &Connection) -> Result<BackupSettings, AppError> {
    connection
        .query_row(
            "SELECT
                automatic_enabled,
                frequency,
                destination,
                retention_count,
                last_success_at,
                last_success_file,
                last_failure_at,
                last_failure_message,
                updated_at
             FROM backup_settings
             WHERE id = 1",
            [],
            |row| {
                Ok(BackupSettings {
                    automatic_enabled: row.get::<_, i64>(0)? != 0,
                    frequency: row.get(1)?,
                    destination: row.get(2)?,
                    retention_count: row.get(3)?,
                    last_success_at: row.get(4)?,
                    last_success_file: row.get(5)?,
                    last_failure_at: row.get(6)?,
                    last_failure_message: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Backup settings could not be loaded", error))?
        .ok_or_else(|| AppError::internal("The backup settings row is missing."))
}

pub fn update_settings(
    connection: &mut Connection,
    input: BackupSettingsInput,
) -> Result<BackupSettings, AppError> {
    let frequency = input.frequency.trim().to_lowercase();
    if !matches!(
        frequency.as_str(),
        "daily" | "weekly" | "before_important_operations"
    ) {
        return Err(AppError::validation(
            "frequency",
            "Choose daily, weekly, or before important operations.",
        ));
    }
    if !(1..=100).contains(&input.retention_count) {
        return Err(AppError::validation(
            "retentionCount",
            "Retained automatic backups must be between 1 and 100.",
        ));
    }
    let destination = input
        .destination
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(destination) = &destination {
        if destination.chars().count() > MAX_DESTINATION_CHARS {
            return Err(AppError::validation(
                "destination",
                "The selected backup folder path is too long.",
            ));
        }
        if !Path::new(destination).is_absolute() {
            return Err(AppError::validation(
                "destination",
                "Choose an absolute backup folder path.",
            ));
        }
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The backup-settings transaction could not be started",
            error,
        )
    })?;
    transaction
        .execute(
            "UPDATE backup_settings
             SET automatic_enabled = ?1,
                 frequency = ?2,
                 destination = ?3,
                 retention_count = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![
                i64::from(input.automatic_enabled),
                frequency,
                destination,
                input.retention_count,
            ],
        )
        .map_err(|error| AppError::database("Backup settings could not be saved", error))?;
    let settings = load_settings(&transaction)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Backup settings could not be committed", error))?;
    Ok(settings)
}

pub fn overview(connection: &Connection, data_dir: &Path) -> Result<BackupOverview, AppError> {
    let database_id = ensure_database_identifier(connection, None)?;
    Ok(BackupOverview {
        settings: load_settings(connection)?,
        default_directory: default_backup_directory(data_dir).display().to_string(),
        schema_version: schema_version(connection)?,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        database_id,
        recovery_snapshots: list_recovery_snapshots(data_dir),
    })
}

pub fn create_manual_backup(
    connection: &Connection,
    destination: Option<&str>,
    data_dir: &Path,
) -> Result<BackupResult, AppError> {
    let settings = load_settings(connection)?;
    let directory = match destination {
        Some(value) => PathBuf::from(value.trim()),
        None => settings
            .destination
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| default_backup_directory(data_dir)),
    };
    match create_backup(connection, &directory, "manual", None) {
        Ok(result) => {
            let _ = record_success(connection, &result.manifest);
            Ok(result)
        }
        Err(error) => {
            let _ = record_failure(connection, &error.message);
            Err(error)
        }
    }
}

pub fn create_safety_backup(
    connection: &Connection,
    data_dir: &Path,
    reason: &str,
    database_id: Option<&str>,
) -> Result<BackupResult, AppError> {
    create_backup(
        connection,
        &safety_backup_directory(data_dir),
        reason,
        database_id,
    )
}

pub fn create_backup(
    connection: &Connection,
    destination: &Path,
    reason: &str,
    preferred_database_id: Option<&str>,
) -> Result<BackupResult, AppError> {
    if !destination.is_absolute() {
        return Err(AppError::backup_failed(
            "Choose an absolute folder for backups.",
        ));
    }
    fs::create_dir_all(destination).map_err(|error| {
        eprintln!("The backup folder could not be prepared: {error}");
        AppError::backup_failed(
            "The backup folder is unavailable or cannot be written. Choose another folder and try again.",
        )
    })?;
    let directory = destination.canonicalize().map_err(|_| {
        AppError::backup_failed(
            "The backup folder is unavailable. Check the drive and permissions, then try again.",
        )
    })?;
    if !directory.is_dir() {
        return Err(AppError::backup_failed(
            "The selected backup destination is not a folder.",
        ));
    }
    if is_network_path(&directory) {
        return Err(AppError::backup_failed(
            "Network folders are not supported for backup. Choose a local or removable-drive folder so study data is not transmitted over a network.",
        ));
    }

    let schema_version = schema_version(connection)?;
    let record_counts = table_counts(connection)?;
    let database_id = match preferred_database_id.filter(|value| is_database_id(value)) {
        Some(value) => value.to_owned(),
        None => ensure_database_identifier(connection, None)?,
    };
    let file_name = next_backup_file_name(&directory, reason, schema_version)?;
    let final_path = directory.join(&file_name);
    let temp_path = directory.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));

    connection
        .backup(rusqlite::MAIN_DB, &temp_path, None)
        .map_err(|error| {
            eprintln!("SQLite online backup failed: {error}");
            let _ = fs::remove_file(&temp_path);
            remove_snapshot_sidecars(&temp_path);
            AppError::backup_failed(
                "The backup could not be completed. Check the destination drive and free space, then try again.",
            )
        })?;
    checkpoint_snapshot(&temp_path);
    remove_snapshot_sidecars(&temp_path);

    if let Err(error) = verify_snapshot(&temp_path) {
        let _ = fs::remove_file(&temp_path);
        remove_snapshot_sidecars(&temp_path);
        return Err(error);
    }
    remove_snapshot_sidecars(&temp_path);
    let size_bytes = fs::metadata(&temp_path)
        .map_err(|_| AppError::backup_failed("The completed backup could not be inspected."))?
        .len();
    if size_bytes == 0 || size_bytes > MAX_BACKUP_BYTES {
        let _ = fs::remove_file(&temp_path);
        remove_snapshot_sidecars(&temp_path);
        return Err(AppError::backup_failed(
            "The completed backup has an unexpected size and was discarded.",
        ));
    }
    let sha256 = sha256_file(&temp_path)
        .map_err(|_| AppError::backup_failed("The completed backup could not be checksummed."))?;

    fs::rename(&temp_path, &final_path).map_err(|_| {
        let _ = fs::remove_file(&temp_path);
        remove_snapshot_sidecars(&temp_path);
        AppError::backup_failed("The completed backup could not be moved into place.")
    })?;
    remove_snapshot_sidecars(&final_path);

    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version,
        created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        database_id,
        reason: reason.to_owned(),
        backup_file_name: file_name.clone(),
        backup_size_bytes: size_bytes,
        sha256,
        integrity_check: "ok".to_owned(),
        record_counts,
    };
    if let Err(error) = write_manifest(&final_path, &manifest) {
        let _ = fs::remove_file(&final_path);
        let _ = fs::remove_file(manifest_path(&final_path));
        return Err(error);
    }

    Ok(BackupResult {
        file_name,
        directory: directory.display().to_string(),
        manifest,
        removed_old_backups: 0,
        warning: None,
    })
}

pub fn verify_backup(path: &Path) -> Result<VerifiedBackup, AppError> {
    let canonical = path.canonicalize();
    let wal = canonical
        .as_ref()
        .ok()
        .map(|path| snapshot_sidecar_path(path, "-wal"));
    let shm = canonical
        .as_ref()
        .ok()
        .map(|path| snapshot_sidecar_path(path, "-shm"));
    let wal_existed = wal.as_ref().is_some_and(|path| path.exists());
    let shm_existed = shm.as_ref().is_some_and(|path| path.exists());
    let result = verify_backup_contents(path);
    if result.is_ok() {
        // Only clean sidecars created by read-only verification of a file that
        // was successfully proven to be a managed backup. Never touch sidecars
        // that existed before verification, and never clean on failure because
        // the selected file may be a live or unrelated user database.
        if !wal_existed && let Some(path) = &wal {
            let _ = fs::remove_file(path);
        }
        if !shm_existed && let Some(path) = &shm {
            let _ = fs::remove_file(path);
        }
    }
    result
}

fn verify_backup_contents(path: &Path) -> Result<VerifiedBackup, AppError> {
    let path = path
        .canonicalize()
        .map_err(|_| AppError::restore_failed("The selected backup file is unavailable."))?;
    if !path.is_file() {
        return Err(AppError::restore_failed(
            "The selected backup is not a file.",
        ));
    }
    let metadata = fs::metadata(&path)
        .map_err(|_| AppError::restore_failed("The selected backup could not be inspected."))?;
    if metadata.len() == 0 || metadata.len() > MAX_BACKUP_BYTES {
        return Err(AppError::restore_failed(
            "The selected backup has an invalid size.",
        ));
    }

    let manifest = read_manifest(&path)?;
    validate_manifest(&manifest, &path, metadata.len())?;
    verify_snapshot(&path)?;
    let connection = Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| AppError::restore_failed("The selected backup could not be opened."))?;
    let schema = schema_version(&connection)?;
    if schema != manifest.schema_version {
        return Err(AppError::restore_failed(
            "The backup manifest schema does not match the database inside the file.",
        ));
    }
    let record_counts = table_counts(&connection)?;
    validate_manifest_counts(&manifest, &record_counts)?;
    let local_files = local_file_restore_summary(&connection)?;

    Ok(VerifiedBackup {
        manifest,
        record_counts,
        local_files,
    })
}

fn validate_manifest_counts(
    manifest: &BackupManifest,
    actual_counts: &BTreeMap<String, i64>,
) -> Result<(), AppError> {
    for (table, expected) in &manifest.record_counts {
        if !is_known_backup_table(table) {
            continue;
        }
        let actual = actual_counts.get(table).copied().unwrap_or(0);
        if actual != *expected {
            return Err(AppError::restore_failed(
                "The backup manifest record counts do not match the database inside the file.",
            ));
        }
    }
    Ok(())
}

fn is_known_backup_table(table: &str) -> bool {
    BACKUP_TABLES.contains(&table)
}

pub fn preview_restore(
    path: &Path,
    current_schema_version: i64,
    current_database_path: Option<&Path>,
) -> Result<RestorePreview, AppError> {
    let selected = path
        .canonicalize()
        .map_err(|_| AppError::restore_failed("The selected backup file is unavailable."))?;
    // Reject the live database before verification: read-only verification of
    // a WAL-mode live database can create sidecars, and failed verification
    // must never trigger cleanup on an arbitrary user-selected file.
    if let Some(current_path) = current_database_path
        && current_path
            .canonicalize()
            .ok()
            .is_some_and(|current| current == selected)
    {
        return Err(AppError::restore_failed(
            "Select a backup file, not the live application database.",
        ));
    }
    let verified = verify_backup(&selected)?;
    if verified.manifest.schema_version > current_schema_version {
        return Err(AppError::restore_failed(format!(
            "This backup uses schema version {}, but this app supports up to schema {}. Update the app instead of downgrading the database.",
            verified.manifest.schema_version, current_schema_version
        )));
    }
    if verified.manifest.schema_version < 1 {
        return Err(AppError::restore_failed(
            "The backup schema version is not supported.",
        ));
    }

    let mut warnings = vec![
        "Restoring replaces the current application data with the selected backup.".to_owned(),
        "A verified safety backup of the current database is created first. If restore fails, the original database is recovered.".to_owned(),
    ];
    if verified.manifest.schema_version < current_schema_version {
        warnings.push(format!(
            "This backup is schema {}. It will be upgraded through the normal migration process to schema {} after restore.",
            verified.manifest.schema_version, current_schema_version
        ));
    }
    if verified.manifest.database_id.is_empty() {
        warnings.push("The backup does not expose a database identifier.".to_owned());
    }
    if let Some(local_files) = &verified.local_files {
        if local_files.registered > 0 {
            warnings.push(
                "Local PDF paths are references only. The backup does not contain PDF bytes, and restore never copies, downloads, or overwrites PDF files."
                    .to_owned(),
            );
        }
        if local_files.missing > 0 {
            warnings.push(format!(
                "{} local PDF reference(s) are unavailable on this computer. The catalogue entry, study status, notes, and history will still be restored; these PDFs will show as Local copy missing until re-linked or replaced.",
                local_files.missing
            ));
        }
        if local_files.changed > 0 {
            warnings.push(format!(
                "{} local PDF reference(s) point to files whose size differs from the recorded copy. They will show as Local copy changed until re-linked or replaced.",
                local_files.changed
            ));
        }
    }

    Ok(RestorePreview {
        requires_upgrade: verified.manifest.schema_version < current_schema_version,
        manifest: verified.manifest,
        record_counts: verified.record_counts,
        local_files: verified.local_files,
        current_schema_version,
        warnings,
    })
}

pub fn automatic_backup(
    connection: &mut Connection,
    data_dir: &Path,
    now: DateTime<Utc>,
) -> Result<AutoBackupOutcome, AppError> {
    let settings = load_settings(connection)?;
    if !settings.automatic_enabled {
        return Ok(AutoBackupOutcome {
            attempted: false,
            success: true,
            message: "Automatic backups are disabled.".to_owned(),
            file_name: None,
        });
    }
    if settings.frequency == "before_important_operations" {
        return Ok(AutoBackupOutcome {
            attempted: false,
            success: true,
            message:
                "Automatic backups are configured for important operations. Safety backups are created before destructive actions."
                    .to_owned(),
            file_name: None,
        });
    }
    if !automatic_backup_due(&settings, now) {
        return Ok(AutoBackupOutcome {
            attempted: false,
            success: true,
            message: "The scheduled automatic backup is not due yet.".to_owned(),
            file_name: None,
        });
    }

    let directory = settings
        .destination
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_backup_directory(data_dir));
    match create_backup(connection, &directory, "automatic", None) {
        Ok(mut result) => {
            let _ = record_success(connection, &result.manifest);
            let refreshed = load_settings(connection).unwrap_or(settings);
            let cleanup = cleanup_automatic_backups(
                &directory,
                refreshed.retention_count,
                refreshed.last_success_file.as_deref(),
            );
            result.removed_old_backups = cleanup.removed;
            result.warning = cleanup.warning;
            Ok(AutoBackupOutcome {
                attempted: true,
                success: true,
                message: result.warning.map_or_else(
                    || "Automatic backup completed and passed integrity verification.".to_owned(),
                    |warning| {
                        format!(
                            "Automatic backup completed and passed integrity verification. {warning}"
                        )
                    },
                ),
                file_name: Some(result.file_name),
            })
        }
        Err(error) => {
            let _ = record_failure(connection, &error.message);
            Ok(AutoBackupOutcome {
                attempted: true,
                success: false,
                message: error.message,
                file_name: None,
            })
        }
    }
}

fn automatic_backup_due(settings: &BackupSettings, now: DateTime<Utc>) -> bool {
    let Some(last_success) = &settings.last_success_at else {
        return true;
    };
    let Ok(last) = DateTime::parse_from_rfc3339(last_success) else {
        return true;
    };
    let elapsed = now.signed_duration_since(last.with_timezone(&Utc));
    match settings.frequency.as_str() {
        "weekly" => elapsed.num_days() >= 7,
        _ => elapsed.num_hours() >= 24,
    }
}

pub fn record_success(connection: &Connection, manifest: &BackupManifest) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE backup_settings
             SET last_success_at = ?1,
                 last_success_file = ?2,
                 last_failure_at = NULL,
                 last_failure_message = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![manifest.created_at, manifest.backup_file_name],
        )
        .map_err(|error| {
            AppError::database("The successful backup could not be recorded", error)
        })?;
    Ok(())
}

pub fn record_failure(connection: &Connection, message: &str) -> Result<(), AppError> {
    let message = sanitize_failure_message(message);
    connection
        .execute(
            "UPDATE backup_settings
             SET last_failure_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 last_failure_message = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            [message],
        )
        .map_err(|error| AppError::database("The backup failure could not be recorded", error))?;
    Ok(())
}

fn sanitize_failure_message(message: &str) -> String {
    let mut value = message
        .lines()
        .next()
        .unwrap_or("The backup failed.")
        .trim()
        .to_owned();
    if value.chars().count() > 1_000 {
        value = value.chars().take(1_000).collect();
    }
    value
}

struct CleanupResult {
    removed: i64,
    warning: Option<String>,
}

fn automatic_timestamp_key(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let rest = name
        .strip_prefix("csir-net-tracker-automatic-v")
        .unwrap_or(&name);
    let stem = rest
        .split_once('-')
        .map(|(_, value)| value.trim_end_matches(".sqlite3"))
        .unwrap_or_else(|| rest.trim_end_matches(".sqlite3"));
    let (stamp, counter) = if stem.len() > 15 && stem.as_bytes().get(15) == Some(&b'-') {
        let counter = stem[16..].parse::<u64>().unwrap_or(0);
        (&stem[..15], counter)
    } else {
        (stem, 0)
    };
    format!("{stamp}-{counter:010}")
}

fn cleanup_automatic_backups_with<Verify, Remove>(
    directory: &Path,
    keep: i64,
    last_success_file: Option<&str>,
    verify: Verify,
    remove: Remove,
) -> CleanupResult
where
    Verify: Fn(&Path) -> bool,
    Remove: Fn(&Path) -> std::io::Result<()>,
{
    let keep = keep.max(1) as usize;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            return CleanupResult {
                removed: 0,
                warning: Some(
                    "The backup succeeded, but old automatic backups could not be inspected. No files were pruned."
                        .to_owned(),
                ),
            };
        }
    };

    let mut valid = Vec::new();
    let mut invalid_preserved = 0_usize;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned());
        let Some(name) = name else {
            continue;
        };
        if !name.starts_with("csir-net-tracker-automatic-") || !name.ends_with(".sqlite3") {
            continue;
        }
        if verify(&path) {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            valid.push((automatic_timestamp_key(&path), modified, name, path));
        } else {
            // A verification failure may be transient (antivirus, lock,
            // permission, or partial visibility). Preserve the file and its
            // manifest rather than deleting data we cannot inspect.
            invalid_preserved += 1;
        }
    }
    valid.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
    });
    if valid.is_empty() {
        return CleanupResult {
            removed: 0,
            warning: Some(
                "The automatic backup succeeded, but old backups could not be pruned because no valid automatic backup was found. No files were pruned."
                    .to_owned(),
            ),
        };
    }

    let last_success_name = last_success_file.and_then(|value| {
        Path::new(value)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    let mut protected = valid
        .iter()
        .take(keep)
        .map(|(_, _, name, _)| name.clone())
        .collect::<HashSet<_>>();
    if let Some((_, _, name, _)) = valid.first() {
        protected.insert(name.clone());
    }
    if let Some(name) = last_success_name {
        protected.insert(name);
    }

    let mut removed = 0_i64;
    let mut warnings = Vec::new();
    for (_, _, name, path) in valid.into_iter().skip(keep) {
        if protected.contains(&name) {
            continue;
        }
        let manifest = manifest_path(&path);
        if let Err(error) = remove(&path) {
            warnings.push(format!(
                "Automatic backup {name} could not be deleted ({error}); its manifest was preserved."
            ));
            continue;
        }
        if manifest.exists()
            && let Err(error) = remove(&manifest)
        {
            warnings.push(format!(
                "Automatic backup {name} was deleted, but its manifest could not be removed ({error}). The prune was not counted as complete."
            ));
            continue;
        }
        removed += 1;
    }
    if invalid_preserved > 0 {
        warnings.push(format!(
            "{invalid_preserved} unverifiable automatic backup(s) were preserved rather than deleted."
        ));
    }

    CleanupResult {
        removed,
        warning: (!warnings.is_empty()).then(|| warnings.join(" ")),
    }
}

fn cleanup_automatic_backups(
    directory: &Path,
    keep: i64,
    last_success_file: Option<&str>,
) -> CleanupResult {
    cleanup_automatic_backups_with(
        directory,
        keep,
        last_success_file,
        |path| verify_backup(path).is_ok(),
        |path| fs::remove_file(path),
    )
}

pub fn table_counts(connection: &Connection) -> Result<BTreeMap<String, i64>, AppError> {
    let mut counts = BTreeMap::new();
    for table in BACKUP_TABLES {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
                 )",
                [table],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("A backup table could not be inspected", error))?;
        if !exists {
            continue;
        }
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| AppError::database("A backup table count could not be read", error))?;
        counts.insert(table.to_owned(), count);
    }
    Ok(counts)
}

fn local_file_restore_summary(
    connection: &Connection,
) -> Result<Option<LocalFileRestoreSummary>, AppError> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type = 'table' AND name = 'resource_local_files'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Local PDF references could not be inspected", error)
        })?;
    if !exists {
        return Ok(None);
    }

    let mut statement = connection
        .prepare("SELECT file_path, size_bytes FROM resource_local_files")
        .map_err(|error| AppError::database("Local PDF references could not be read", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| AppError::database("Local PDF references could not be read", error))?;
    let mut summary = LocalFileRestoreSummary {
        registered: 0,
        available: 0,
        missing: 0,
        changed: 0,
    };
    for row in rows {
        let (path, size) = row
            .map_err(|error| AppError::database("Local PDF references could not be read", error))?;
        summary.registered += 1;
        let (exists, size_matches) = local_file_status(&path, size);
        if !exists {
            summary.missing += 1;
        } else if size_matches {
            summary.available += 1;
        } else {
            summary.changed += 1;
        }
    }
    Ok(Some(summary))
}

pub fn schema_version(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| AppError::database("The database schema version could not be read", error))
}

pub fn verify_database_file(path: &Path) -> Result<(), AppError> {
    verify_snapshot(path)
}

pub(crate) fn live_database_state(path: &Path, current_schema: i64) -> LiveDatabaseState {
    if !path.is_file() {
        return LiveDatabaseState::Missing;
    }
    let Ok(connection) = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return LiveDatabaseState::Invalid;
    };
    let Ok(schema) = schema_version(&connection) else {
        return LiveDatabaseState::Invalid;
    };
    if schema > current_schema {
        return LiveDatabaseState::NewerSchema;
    }
    if (1..=current_schema).contains(&schema) && verify_database_file(path).is_ok() {
        LiveDatabaseState::Valid
    } else {
        LiveDatabaseState::Invalid
    }
}

fn live_state_name(state: LiveDatabaseState) -> &'static str {
    match state {
        LiveDatabaseState::Valid => "valid",
        LiveDatabaseState::NewerSchema => "newer_schema",
        LiveDatabaseState::Invalid => "invalid",
        LiveDatabaseState::Missing => "missing",
    }
}

pub(crate) fn verify_live_recovery_database(path: &Path, current_schema: i64) -> bool {
    live_database_state(path, current_schema) == LiveDatabaseState::Valid
}

fn verify_snapshot(path: &Path) -> Result<(), AppError> {
    let connection = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| AppError::restore_failed("The backup database could not be opened."))?;
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| AppError::restore_failed("The backup integrity could not be checked."))?;
    if integrity != "ok" {
        return Err(AppError::restore_failed(
            "The backup failed SQLite integrity verification.",
        ));
    }
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|_| AppError::restore_failed("Backup relationships could not be checked."))?;
    let mut rows = statement
        .query([])
        .map_err(|_| AppError::restore_failed("Backup relationships could not be checked."))?;
    if rows
        .next()
        .map_err(|_| AppError::restore_failed("Backup relationships could not be checked."))?
        .is_some()
    {
        return Err(AppError::restore_failed(
            "The backup contains records that violate database relationships.",
        ));
    }
    Ok(())
}

fn validate_manifest(
    manifest: &BackupManifest,
    path: &Path,
    actual_size: u64,
) -> Result<(), AppError> {
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(AppError::restore_failed(
            "This backup manifest format is not supported by this app version.",
        ));
    }
    if !is_database_id(&manifest.database_id) {
        return Err(AppError::restore_failed(
            "The backup manifest has an invalid database identifier.",
        ));
    }
    let expected_name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    if manifest.backup_file_name != expected_name {
        return Err(AppError::restore_failed(
            "The backup manifest does not describe this file name.",
        ));
    }
    if manifest.backup_size_bytes != actual_size {
        return Err(AppError::restore_failed(
            "The backup file size does not match its manifest. The file may be truncated.",
        ));
    }
    if manifest.integrity_check != "ok" {
        return Err(AppError::restore_failed(
            "The backup manifest does not record a successful integrity check.",
        ));
    }
    let actual_hash = sha256_file(path)?;
    if !constant_time_equal(&manifest.sha256, &actual_hash) {
        return Err(AppError::restore_failed(
            "The backup file does not match its manifest. Choose an unmodified backup.",
        ));
    }
    Ok(())
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn read_manifest(path: &Path) -> Result<BackupManifest, AppError> {
    let manifest_file = manifest_path(path);
    let bytes = fs::read(&manifest_file).map_err(|_| {
        AppError::restore_failed(
            "The backup manifest is missing. Choose a backup created by this application without renaming its manifest.",
        )
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::restore_failed("The backup manifest is not valid JSON."))
}

fn write_manifest(path: &Path, manifest: &BackupManifest) -> Result<(), AppError> {
    let manifest_file = manifest_path(path);
    let temp_file = manifest_file.with_extension(format!("{}.tmp", Uuid::new_v4().simple()));
    let mut bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|_| AppError::backup_failed("The backup manifest could not be encoded."))?;
    bytes.push(b'\n');
    fs::write(&temp_file, bytes)
        .map_err(|_| AppError::backup_failed("The backup manifest could not be written."))?;
    fs::rename(&temp_file, &manifest_file).map_err(|_| {
        let _ = fs::remove_file(&temp_file);
        AppError::backup_failed("The backup manifest could not be moved into place.")
    })?;
    Ok(())
}

pub fn manifest_path(backup_path: &Path) -> PathBuf {
    let mut value = backup_path.as_os_str().to_os_string();
    value.push(MANIFEST_SUFFIX);
    PathBuf::from(value)
}

fn next_backup_file_name(
    directory: &Path,
    reason: &str,
    schema_version: i64,
) -> Result<String, AppError> {
    let reason = match reason {
        "manual" => "manual",
        "automatic" => "automatic",
        "pre-migration" => "pre-migration",
        "post-migration" => "post-migration",
        "pre-restore" => "pre-restore",
        "pre-json-import" => "pre-json-import",
        "pre-json-replace" => "pre-json-replace",
        _ => "safety",
    };
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    for counter in 0..=1_000 {
        let suffix = if counter == 0 {
            String::new()
        } else {
            format!("-{counter}")
        };
        let file_name =
            format!("{BACKUP_PREFIX}-{reason}-v{schema_version}-{stamp}{suffix}.sqlite3");
        let path = directory.join(&file_name);
        if !path.exists() && !manifest_path(&path).exists() {
            return Ok(file_name);
        }
    }
    Err(AppError::backup_failed(
        "A unique backup file name could not be created.",
    ))
}

fn snapshot_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn checkpoint_snapshot(path: &Path) {
    if let Ok(connection) = Connection::open(path) {
        let _ = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
    }
}

fn remove_snapshot_sidecars(path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(snapshot_sidecar_path(path, suffix));
    }
}

pub(crate) fn is_network_path(path: &Path) -> bool {
    let text = path.to_string_lossy().to_lowercase();
    if text.starts_with(r"\\?\unc\") || text.starts_with("//?/unc/") {
        return true;
    }
    if text.starts_with(r"\\?\") || text.starts_with("//?/") {
        return false;
    }
    text.starts_with(r"\\") || text.starts_with("//")
}

pub fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path)
        .map_err(|_| AppError::restore_failed("The backup file could not be read."))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| AppError::restore_failed("The backup file could not be read."))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;
    use crate::{db, resources, sessions, syllabus};

    fn date(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn database(tmp: &TempDir) -> Connection {
        let path = tmp.path().join("tracker.sqlite3");
        let mut connection =
            db::open_database(&path, date(2026, 10, 30)).expect("database should open");
        syllabus::seed(&mut connection).expect("syllabus should seed");
        connection
    }

    fn topic_id(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                [],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    const NOW: &str = "2026-10-30T12:00:00.000Z";
    const FIRST_URL: &str = "https://drive.google.com/file/d/stage3-first/view";
    const SECOND_URL: &str = "https://example.com/stage3-second.pdf";
    const SOURCE_PAGE: &str = "https://example.com/stage3-notes";
    const PDF_PAYLOAD: &[u8] = b"%PDF-1.4\nstage3-unique-private-pdf-payload-7c1e9a\n%%EOF\n";

    struct ResourceFixture {
        first_id: i64,
        second_id: i64,
        first_key: String,
        local_file: resources::LocalResourceFile,
    }

    fn seed_resources(
        connection: &mut Connection,
        library: &Path,
        source_pdf: &Path,
    ) -> ResourceFixture {
        let first = resources::create_manual(
            connection,
            resources::ManualResourceInput {
                title: "Stage 3 Quantum Notes".to_owned(),
                description: Some("Backup integration fixture".to_owned()),
                category: "handwritten_notes".to_owned(),
                unit: "Quantum Mechanics".to_owned(),
                topic_code: None,
                original_url: FIRST_URL.to_owned(),
                source_page_url: Some(SOURCE_PAGE.to_owned()),
            },
        )
        .expect("first resource should save");
        let second = resources::create_manual(
            connection,
            resources::ManualResourceInput {
                title: "Stage 3 Reference PDF".to_owned(),
                description: Some("Second backup integration fixture".to_owned()),
                category: "recommended_books".to_owned(),
                unit: "Classical Mechanics".to_owned(),
                topic_code: None,
                original_url: SECOND_URL.to_owned(),
                source_page_url: Some(SOURCE_PAGE.to_owned()),
            },
        )
        .expect("second resource should save");
        resources::update_progress(
            connection,
            first.id,
            resources::ResourceProgressInput {
                status: "studying".to_owned(),
                notes: "Stage 3 personal notes for the first resource.".to_owned(),
            },
            NOW,
        )
        .expect("first progress should save");
        resources::update_progress(
            connection,
            second.id,
            resources::ResourceProgressInput {
                status: "completed".to_owned(),
                notes: "Stage 3 completed notes for the second resource.".to_owned(),
            },
            NOW,
        )
        .expect("second progress should save");
        connection
            .execute(
                "INSERT INTO resource_discovery_runs (
                    started_at, finished_at, status, pages_ok, pages_failed,
                    resources_seen, resources_added, resources_updated,
                    resources_removed, message
                 ) VALUES (?1, ?1, 'partial', 3, 1, 5, 2, 1, 0, 'Stage 3 discovery history')",
                [NOW],
            )
            .expect("discovery history should save");

        fs::write(source_pdf, PDF_PAYLOAD).expect("PDF fixture should write");
        let local_file =
            resources::import_local_pdf(connection, first.id, source_pdf, library, NOW)
                .expect("PDF should import");
        ResourceFixture {
            first_id: first.id,
            second_id: second.id,
            first_key: first.resource_key,
            local_file,
        }
    }

    #[test]
    fn full_backup_and_restore_preserve_resource_tables_and_exclude_pdf_bytes() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let pdf_source = tempfile::tempdir().expect("PDF source temp");
        let library = tempfile::tempdir().expect("PDF library temp");
        let mut connection = database(&source);
        let fixture = seed_resources(
            &mut connection,
            library.path(),
            &pdf_source.path().join("stage3.pdf"),
        );

        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let backup_path = destination.path().join(&result.file_name);

        for (table, expected) in [
            ("study_resources", 2),
            ("resource_progress", 2),
            ("resource_local_files", 1),
            ("resource_discovery_runs", 1),
        ] {
            assert_eq!(result.manifest.record_counts[table], expected);
        }
        let verified = verify_backup(&backup_path).expect("backup should verify");
        assert_eq!(verified.record_counts["study_resources"], 2);
        assert_eq!(verified.record_counts["resource_progress"], 2);
        assert_eq!(verified.record_counts["resource_local_files"], 1);
        assert_eq!(verified.record_counts["resource_discovery_runs"], 1);
        assert_eq!(
            verified.local_files.as_ref().map(|files| files.available),
            Some(1)
        );

        let backup_bytes = fs::read(&backup_path).expect("backup bytes should read");
        assert!(
            !backup_bytes
                .windows(PDF_PAYLOAD.len())
                .any(|window| window == PDF_PAYLOAD),
            "the SQLite backup must not contain PDF bytes"
        );

        let target_dir = tempfile::tempdir().expect("target temp");
        let sentinel = target_dir.path().join("keep-me.pdf");
        fs::write(&sentinel, b"unrelated user-owned file").expect("sentinel should write");
        let target = db::AppState::initialize(target_dir.path(), date(2026, 10, 30));
        target
            .restore_from_backup(&backup_path, date(2026, 10, 30))
            .expect("restore should succeed");

        let (
            catalogue,
            first_progress,
            second_progress,
            discovery_runs,
            local_row,
            integrity,
            foreign_keys,
        ) = target
            .with_connection(|connection| {
                let catalogue = resources::list_resources(connection)?;
                let first_progress: (String, String) = connection
                    .query_row(
                        "SELECT study_status, personal_notes
                         FROM resource_progress WHERE resource_id = ?1",
                        [fixture.first_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .expect("first progress should query");
                let second_progress: (String, String) = connection
                    .query_row(
                        "SELECT study_status, personal_notes
                         FROM resource_progress WHERE resource_id = ?1",
                        [fixture.second_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .expect("second progress should query");
                let discovery_runs: i64 = connection
                    .query_row("SELECT count(*) FROM resource_discovery_runs", [], |row| {
                        row.get(0)
                    })
                    .expect("discovery history should query");
                let local_row: (String, String, i64) = connection
                    .query_row(
                        "SELECT file_path, sha256, size_bytes
                         FROM resource_local_files WHERE resource_id = ?1",
                        [fixture.first_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .expect("local reference should query");
                let integrity: String = connection
                    .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                    .expect("integrity check should run");
                let foreign_keys: i64 = connection
                    .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                        row.get(0)
                    })
                    .expect("foreign-key check should run");
                Ok((
                    catalogue,
                    first_progress,
                    second_progress,
                    discovery_runs,
                    local_row,
                    integrity,
                    foreign_keys,
                ))
            })
            .expect("restored data should query");

        assert_eq!(catalogue.len(), 2);
        assert_eq!(integrity, "ok");
        assert_eq!(foreign_keys, 0);
        assert_eq!(discovery_runs, 1);
        assert_eq!(
            first_progress,
            (
                "studying".to_owned(),
                "Stage 3 personal notes for the first resource.".to_owned()
            )
        );
        assert_eq!(
            second_progress,
            (
                "completed".to_owned(),
                "Stage 3 completed notes for the second resource.".to_owned()
            )
        );
        assert_eq!(local_row.0, fixture.local_file.file_path);
        assert_eq!(local_row.1, fixture.local_file.sha256);
        assert_eq!(local_row.2, fixture.local_file.size_bytes);

        let first = catalogue
            .iter()
            .find(|resource| resource.id == fixture.first_id)
            .expect("first resource should survive");
        assert_eq!(first.resource_key, fixture.first_key);
        assert_eq!(first.original_url, FIRST_URL);
        assert_eq!(first.source_page_url.as_deref(), Some(SOURCE_PAGE));
        let second = catalogue
            .iter()
            .find(|resource| resource.id == fixture.second_id)
            .expect("second resource should survive");
        assert_eq!(second.original_url, SECOND_URL);
        assert_eq!(second.source_page_url.as_deref(), Some(SOURCE_PAGE));
        assert_eq!(
            fs::read(&sentinel).expect("sentinel should remain"),
            b"unrelated user-owned file"
        );
    }

    #[test]
    fn cross_machine_restore_keeps_metadata_and_allows_relinking_missing_pdfs() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let pdf_source = tempfile::tempdir().expect("PDF source temp");
        let library = tempfile::tempdir().expect("PDF library temp");
        let mut connection = database(&source);
        let fixture = seed_resources(
            &mut connection,
            library.path(),
            &pdf_source.path().join("stage3.pdf"),
        );
        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let backup_path = destination.path().join(&result.file_name);
        let stored_path = fixture.local_file.file_path.clone();

        drop(library);
        assert!(!Path::new(&stored_path).exists());

        let preview = preview_restore(&backup_path, 12, None).expect("preview should load");
        assert_eq!(preview.record_counts["study_resources"], 2);
        assert_eq!(
            preview
                .local_files
                .as_ref()
                .map(|files| (files.registered, files.missing)),
            Some((1, 1))
        );
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("unavailable on this computer"))
        );
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("references only"))
        );

        let target_dir = tempfile::tempdir().expect("target temp");
        let target = db::AppState::initialize(target_dir.path(), date(2026, 10, 30));
        target
            .restore_from_backup(&backup_path, date(2026, 10, 30))
            .expect("restore should succeed");

        let (catalogue, overview, notes) = target
            .with_connection(|connection| {
                let catalogue = resources::list_resources(connection)?;
                let overview = resources::overview(connection)?;
                let notes: String = connection
                    .query_row(
                        "SELECT personal_notes FROM resource_progress WHERE resource_id = ?1",
                        [fixture.first_id],
                        |row| row.get(0),
                    )
                    .expect("notes should query");
                Ok((catalogue, overview, notes))
            })
            .expect("restored resources should query");

        assert_eq!(catalogue.len(), 2);
        assert_eq!(overview.local_count, 0);
        assert_eq!(notes, "Stage 3 personal notes for the first resource.");
        let first = catalogue
            .iter()
            .find(|resource| resource.id == fixture.first_id)
            .expect("missing-PDF resource should remain visible");
        let local_file = first
            .local_file
            .as_ref()
            .expect("the local reference should survive");
        assert!(!local_file.exists);
        assert!(!local_file.size_matches);
        assert!(!Path::new(&stored_path).exists());
        assert!(!target_dir.path().join(&local_file.file_name).exists());
        assert!(
            !target_dir
                .path()
                .read_dir()
                .expect("target dir should read")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf")))
        );

        let relink_dir = tempfile::tempdir().expect("relink temp");
        let moved = relink_dir.path().join("moved-stage3.pdf");
        fs::write(&moved, PDF_PAYLOAD).expect("moved PDF should write");
        let relinked = target
            .with_connection(|connection| {
                resources::relink_local_pdf(connection, fixture.first_id, &moved)
            })
            .expect("missing PDF should re-link");
        assert!(relinked.exists && relinked.size_matches);
        assert_eq!(relinked.file_path, moved.to_string_lossy());
        assert_eq!(relinked.sha256, fixture.local_file.sha256);
        assert_eq!(
            fs::read(&moved).expect("re-linked file should remain"),
            PDF_PAYLOAD
        );
        let local_count = target
            .with_connection(|connection| resources::overview(connection))
            .expect("overview should reload")
            .local_count;
        assert_eq!(local_count, 1);
    }

    #[test]
    fn manifest_record_counts_must_match_backup_contents() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = database(&source);
        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let path = destination.path().join(&result.file_name);
        let manifest_file = manifest_path(&path);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_file).expect("manifest should read"))
                .expect("manifest should parse");
        value["recordCounts"]["study_sessions"] = serde_json::Value::from(99);
        fs::write(
            &manifest_file,
            serde_json::to_vec_pretty(&value).expect("manifest should encode"),
        )
        .expect("tampered manifest should write");

        let error = verify_backup(&path).expect_err("count mismatch should be rejected");
        assert!(error.message.contains("record counts"));
    }

    #[test]
    fn wal_backup_includes_live_records_and_verified_manifest() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let mut connection = database(&source);
        let topic = topic_id(&connection);
        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-10-30".to_owned(),
                duration_seconds: 2_400,
                notes: "WAL-backed session".to_owned(),
            },
        )
        .expect("session should save");

        let wal_path = source.path().join("tracker.sqlite3-wal");
        assert!(wal_path.exists(), "the test should exercise WAL data");

        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let backup_path = destination.path().join(&result.file_name);
        assert!(backup_path.is_file());
        assert!(manifest_path(&backup_path).is_file());
        assert!(
            fs::read_dir(destination.path())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp"))
        );
        assert_eq!(result.manifest.schema_version, 12);
        assert_eq!(result.manifest.application_version, "2.0.0");
        assert_eq!(result.manifest.integrity_check, "ok");
        assert_eq!(
            result.manifest.backup_size_bytes,
            fs::metadata(&backup_path).unwrap().len()
        );
        assert_eq!(result.manifest.sha256.len(), 64);

        let verified = verify_backup(&backup_path).expect("backup should verify");
        assert_eq!(verified.record_counts["study_sessions"], 1);
        assert_eq!(verified.record_counts["exams"], 4);
        assert_eq!(verified.record_counts["exam_profiles"], 4);
        let backup =
            Connection::open_with_flags(&backup_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("backup should open");
        let notes: String = backup
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("session should be present");
        assert_eq!(notes, "WAL-backed session");
        let source_schema: (i64, i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT count(*) FROM sqlite_master WHERE type = 'table'),
                    (SELECT count(*) FROM sqlite_master WHERE type = 'index'),
                    (SELECT count(*) FROM sqlite_master WHERE type = 'trigger')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("source schema should count");
        let backup_schema: (i64, i64, i64) = backup
            .query_row(
                "SELECT
                    (SELECT count(*) FROM sqlite_master WHERE type = 'table'),
                    (SELECT count(*) FROM sqlite_master WHERE type = 'index'),
                    (SELECT count(*) FROM sqlite_master WHERE type = 'trigger')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("backup schema should count");
        assert_eq!(source_schema, backup_schema);
    }

    #[test]
    fn preserving_a_displaced_database_copies_its_sidecars_before_detaching() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let source = directory.path().join("live.sqlite3");
        let destination = directory.path().join("live.sqlite3.failed");
        fs::write(&source, b"main").expect("main should write");
        for suffix in ["-wal", "-shm", "-journal"] {
            fs::write(snapshot_sidecar_path(&source, suffix), suffix.as_bytes())
                .expect("sidecar should write");
        }

        preserve_database_with_sidecars(&source, &destination)
            .expect("preservation should succeed");
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"main");
        for suffix in ["-wal", "-shm", "-journal"] {
            assert_eq!(
                fs::read(snapshot_sidecar_path(&destination, suffix)).unwrap(),
                suffix.as_bytes()
            );
            assert!(!snapshot_sidecar_path(&source, suffix).exists());
        }
    }

    #[test]
    fn selecting_live_or_unrelated_sqlite_files_never_removes_sidecars() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let live = directory.path().join("csir-net-tracker.sqlite3");
        fs::write(&live, b"live file").expect("live file should write");
        let live_wal = snapshot_sidecar_path(&live, "-wal");
        let live_shm = snapshot_sidecar_path(&live, "-shm");
        fs::write(&live_wal, b"live wal").expect("live wal should write");
        fs::write(&live_shm, b"live shm").expect("live shm should write");

        assert!(verify_backup(&live).is_err());
        assert_eq!(fs::read(&live_wal).unwrap(), b"live wal");
        assert_eq!(fs::read(&live_shm).unwrap(), b"live shm");
        assert!(
            preview_restore(&live, 12, Some(&live))
                .expect_err("live path should be rejected")
                .message
                .contains("not the live")
        );
        assert_eq!(fs::read(&live_wal).unwrap(), b"live wal");
        assert_eq!(fs::read(&live_shm).unwrap(), b"live shm");

        let unrelated = directory.path().join("unrelated.sqlite3");
        fs::write(&unrelated, b"not a database").expect("unrelated should write");
        let unrelated_wal = snapshot_sidecar_path(&unrelated, "-wal");
        let unrelated_shm = snapshot_sidecar_path(&unrelated, "-shm");
        fs::write(&unrelated_wal, b"unrelated wal").expect("wal should write");
        fs::write(&unrelated_shm, b"unrelated shm").expect("shm should write");
        assert!(verify_backup(&unrelated).is_err());
        assert_eq!(fs::read(&unrelated_wal).unwrap(), b"unrelated wal");
        assert_eq!(fs::read(&unrelated_shm).unwrap(), b"unrelated shm");
    }

    #[test]
    fn missing_manifest_truncated_file_and_changed_checksum_are_rejected() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = database(&source);
        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let path = destination.path().join(&result.file_name);
        let manifest = manifest_path(&path);
        let original = fs::read(&path).expect("backup should be readable");

        let missing = destination.path().join("missing-manifest.sqlite3");
        fs::write(&missing, &original).expect("copy should write");
        assert_eq!(
            verify_backup(&missing)
                .expect_err("missing manifest should fail")
                .code,
            "restore_failed"
        );

        let truncated = destination.path().join("truncated.sqlite3");
        fs::write(&truncated, &original[..original.len() / 2])
            .expect("truncated copy should write");
        let mut truncated_manifest = result.manifest.clone();
        truncated_manifest.backup_file_name = "truncated.sqlite3".to_owned();
        truncated_manifest.backup_size_bytes = (original.len() / 2) as u64;
        truncated_manifest.sha256 = sha256_file(&truncated).expect("hash should calculate");
        write_manifest(&truncated, &truncated_manifest).expect("manifest should write");
        assert_eq!(
            verify_backup(&truncated)
                .expect_err("truncated sqlite should fail")
                .code,
            "restore_failed"
        );

        fs::write(&path, b"changed").expect("tamper should write");
        assert_eq!(
            verify_backup(&path)
                .expect_err("changed file should fail")
                .code,
            "restore_failed"
        );
        assert!(manifest.exists());
    }

    #[test]
    fn restore_preview_rejects_newer_schemas_and_explains_older_upgrade() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = database(&source);
        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let path = destination.path().join(&result.file_name);

        let error = preview_restore(&path, 7, None).expect_err("newer schema should fail");
        assert!(error.message.contains("Update the app"));
        let preview = preview_restore(&path, 12, None).expect("current schema should load");
        assert_eq!(preview.manifest.schema_version, 12);
        assert!(!preview.requires_upgrade);
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("safety backup"))
        );
    }

    #[test]
    fn automatic_due_rules_and_retention_keep_recent_backups() {
        let now = Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap();
        let mut settings = BackupSettings {
            automatic_enabled: true,
            frequency: "daily".to_owned(),
            destination: None,
            retention_count: 2,
            last_success_at: None,
            last_success_file: None,
            last_failure_at: None,
            last_failure_message: None,
            updated_at: String::new(),
        };
        assert!(automatic_backup_due(&settings, now));
        settings.last_success_at = Some("2026-10-30T11:00:00.000Z".to_owned());
        assert!(!automatic_backup_due(&settings, now));
        settings.last_success_at = Some("2026-10-29T11:00:00.000Z".to_owned());
        assert!(automatic_backup_due(&settings, now));
        settings.frequency = "weekly".to_owned();
        assert!(!automatic_backup_due(&settings, now));
        settings.last_success_at = Some("2026-10-22T11:00:00.000Z".to_owned());
        assert!(automatic_backup_due(&settings, now));

        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = database(&source);
        for _ in 0..3 {
            create_backup(&connection, destination.path(), "automatic", None)
                .expect("automatic backup should succeed");
        }
        create_backup(&connection, destination.path(), "manual", None)
            .expect("manual backup should succeed");
        let cleanup = cleanup_automatic_backups(destination.path(), 2, None);
        assert_eq!(cleanup.removed, 1);
        let remaining_auto = fs::read_dir(destination.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-automatic-")
                    && entry.file_name().to_string_lossy().ends_with(".sqlite3")
            })
            .count();
        let remaining_manual = fs::read_dir(destination.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-manual-")
                    && entry.file_name().to_string_lossy().ends_with(".sqlite3")
            })
            .count();
        assert_eq!(remaining_auto, 2);
        assert_eq!(remaining_manual, 1);
    }

    #[test]
    fn pre_migration_artifact_never_remains_latest_success_and_is_immediately_due() {
        let source = tempfile::tempdir().expect("source temp");
        let connection = database(&source);
        connection
            .execute(
                "UPDATE backup_settings
                 SET last_success_at = '2026-01-01T00:00:00Z',
                     last_success_file = 'csir-net-tracker-pre-migration-v11-20260101-000000.sqlite3'
                 WHERE id = 1",
                [],
            )
            .expect("pre-migration last success should write");

        clear_pre_migration_last_success(&connection)
            .expect("pre-migration last success should clear");
        let settings = load_settings(&connection).expect("settings should load");
        assert!(settings.last_success_at.is_none());
        assert!(settings.last_success_file.is_none());
        assert!(automatic_backup_due(&settings, Utc::now()));

        connection
            .execute(
                "UPDATE backup_settings
                 SET last_success_at = '2026-01-02T00:00:00Z',
                     last_success_file = 'csir-net-tracker-manual-v12-20260102-000000.sqlite3'
                 WHERE id = 1",
                [],
            )
            .expect("manual last success should write");
        clear_pre_migration_last_success(&connection).expect("manual last success should stay");
        let settings = load_settings(&connection).expect("settings should reload");
        assert_eq!(
            settings.last_success_file.as_deref(),
            Some("csir-net-tracker-manual-v12-20260102-000000.sqlite3")
        );
    }

    #[test]
    fn automatic_retention_keeps_fourteen_and_never_prunes_protected_or_manual_backups() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = database(&source);
        for _ in 0..2 {
            create_backup(&connection, destination.path(), "pre-migration", None)
                .expect("pre-migration should save");
            create_backup(
                &connection,
                &safety_backup_directory(destination.path()),
                "post-migration",
                None,
            )
            .expect("post-migration should save");
            create_backup(
                &connection,
                &safety_backup_directory(destination.path()),
                "pre-restore",
                None,
            )
            .expect("pre-restore should save");
            create_backup(&connection, destination.path(), "manual", None)
                .expect("manual should save");
        }
        for _ in 0..15 {
            create_backup(&connection, destination.path(), "automatic", None)
                .expect("automatic should save");
        }

        let cleanup = cleanup_automatic_backups(destination.path(), 14, None);
        assert_eq!(cleanup.removed, 1);
        let count = |prefix: &str, directory: &Path| {
            fs::read_dir(directory)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.starts_with(prefix) && name.ends_with(".sqlite3")
                })
                .count()
        };
        assert_eq!(count("csir-net-tracker-automatic-", destination.path()), 14);
        assert_eq!(
            count("csir-net-tracker-pre-migration-", destination.path()),
            2
        );
        assert_eq!(count("csir-net-tracker-manual-", destination.path()), 2);
        assert_eq!(
            count(
                "csir-net-tracker-post-migration-",
                &safety_backup_directory(destination.path())
            ),
            2
        );
        assert_eq!(
            count(
                "csir-net-tracker-pre-restore-",
                &safety_backup_directory(destination.path())
            ),
            2
        );
    }

    fn automatic_fixture(directory: &Path, stamp: &str) -> (PathBuf, PathBuf) {
        let path = directory.join(format!("csir-net-tracker-automatic-v12-{stamp}.sqlite3"));
        fs::write(&path, b"fixture").expect("fixture backup should write");
        let manifest = manifest_path(&path);
        fs::write(&manifest, b"{}").expect("fixture manifest should write");
        (path, manifest)
    }

    #[test]
    fn retention_protects_newest_and_last_success_backups() {
        let directory = tempfile::tempdir().expect("retention temp");
        let (oldest, oldest_manifest) = automatic_fixture(directory.path(), "20260101-000000");
        let (middle, middle_manifest) = automatic_fixture(directory.path(), "20260102-000000");
        let (newest, newest_manifest) = automatic_fixture(directory.path(), "20260103-000000");
        let removed = std::cell::RefCell::new(Vec::new());
        let cleanup = cleanup_automatic_backups_with(
            directory.path(),
            1,
            middle.file_name().and_then(|name| name.to_str()),
            |_| true,
            |path| {
                removed.borrow_mut().push(path.to_path_buf());
                fs::remove_file(path)
            },
        );

        assert_eq!(cleanup.removed, 1);
        assert!(!oldest.exists());
        assert!(!oldest_manifest.exists());
        assert!(middle.exists());
        assert!(middle_manifest.exists());
        assert!(newest.exists());
        assert!(newest_manifest.exists());
    }

    #[test]
    fn retention_never_deletes_unverifiable_automatic_backups() {
        let directory = tempfile::tempdir().expect("retention temp");
        let (invalid, invalid_manifest) = automatic_fixture(directory.path(), "20260101-000000");
        let (newest, newest_manifest) = automatic_fixture(directory.path(), "20260102-000000");
        let invalid_name = invalid.file_name().unwrap().to_string_lossy().into_owned();
        let cleanup = cleanup_automatic_backups_with(
            directory.path(),
            1,
            None,
            |path| {
                path.file_name()
                    .map(|name| name.to_string_lossy() != invalid_name)
                    .unwrap_or(false)
            },
            |path| fs::remove_file(path),
        );

        assert_eq!(cleanup.removed, 0);
        assert!(
            cleanup
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("unverifiable"))
        );
        assert!(invalid.exists());
        assert!(invalid_manifest.exists());
        assert!(newest.exists());
        assert!(newest_manifest.exists());
    }

    #[test]
    fn retention_preserves_manifest_when_database_deletion_fails() {
        let directory = tempfile::tempdir().expect("retention temp");
        let (oldest, oldest_manifest) = automatic_fixture(directory.path(), "20260101-000000");
        let (newest, newest_manifest) = automatic_fixture(directory.path(), "20260102-000000");
        let cleanup = cleanup_automatic_backups_with(
            directory.path(),
            1,
            None,
            |_| true,
            |path| {
                if path == oldest {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "simulated lock",
                    ))
                } else {
                    fs::remove_file(path)
                }
            },
        );

        assert_eq!(cleanup.removed, 0);
        assert!(
            cleanup
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("could not be deleted"))
        );
        assert!(oldest.exists());
        assert!(oldest_manifest.exists());
        assert!(newest.exists());
        assert!(newest_manifest.exists());
    }

    #[test]
    fn retention_reports_manifest_failure_without_counting_success() {
        let directory = tempfile::tempdir().expect("retention temp");
        let (oldest, oldest_manifest) = automatic_fixture(directory.path(), "20260101-000000");
        let (newest, newest_manifest) = automatic_fixture(directory.path(), "20260102-000000");
        let cleanup = cleanup_automatic_backups_with(
            directory.path(),
            1,
            None,
            |_| true,
            |path| {
                if path == oldest_manifest {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "simulated manifest lock",
                    ))
                } else {
                    fs::remove_file(path)
                }
            },
        );

        assert_eq!(cleanup.removed, 0);
        assert!(
            cleanup
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("manifest could not be removed"))
        );
        assert!(!oldest.exists());
        assert!(oldest_manifest.exists());
        assert!(newest.exists());
        assert!(newest_manifest.exists());
    }

    #[test]
    fn retention_orders_by_filename_timestamp_not_modification_time() {
        let directory = tempfile::tempdir().expect("retention temp");
        let (oldest, _) = automatic_fixture(directory.path(), "20260101-000000");
        let (middle, _) = automatic_fixture(directory.path(), "20260102-000000");
        let (newest, _) = automatic_fixture(directory.path(), "20260103-000000");
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        std::fs::File::options()
            .write(true)
            .open(&oldest)
            .expect("oldest should open")
            .set_modified(future)
            .expect("mtime should change");

        let cleanup = cleanup_automatic_backups_with(
            directory.path(),
            2,
            None,
            |_| true,
            |path| fs::remove_file(path),
        );
        assert_eq!(cleanup.removed, 1);
        assert!(!oldest.exists());
        assert!(middle.exists());
        assert!(newest.exists());
    }

    #[test]
    fn unavailable_destination_records_failure_without_losing_settings() {
        let source = tempfile::tempdir().expect("source temp");
        let mut connection = database(&source);
        let blocker = source.path().join("not-a-folder");
        fs::write(&blocker, b"x").expect("blocker should write");
        let destination = blocker.join("child");
        let error = create_manual_backup(
            &connection,
            Some(destination.to_string_lossy().as_ref()),
            source.path(),
        )
        .expect_err("unavailable folder should fail");
        assert_eq!(error.code, "backup_failed");
        let settings = load_settings(&connection).expect("settings should load");
        assert!(settings.last_failure_message.is_some());
        assert!(
            connection
                .query_row("SELECT count(*) FROM backup_settings", [], |row| row
                    .get::<_, i64>(0))
                .unwrap()
                == 1
        );

        let invalid = BackupSettingsInput {
            automatic_enabled: true,
            frequency: "daily".to_owned(),
            destination: Some("relative-path".to_owned()),
            retention_count: 2,
        };
        assert_eq!(
            update_settings(&mut connection, invalid)
                .expect_err("relative path should fail")
                .field,
            Some("destination")
        );
        let invalid = BackupSettingsInput {
            automatic_enabled: true,
            frequency: "daily".to_owned(),
            destination: None,
            retention_count: 0,
        };
        assert_eq!(
            update_settings(&mut connection, invalid)
                .expect_err("retention should fail")
                .field,
            Some("retentionCount")
        );
    }
}

#[cfg(test)]
mod automatic_tests {
    use chrono::TimeZone;

    use super::*;
    use crate::{db, syllabus};

    #[test]
    fn automatic_backups_are_off_by_default_then_run_once_and_record_success() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let destination = tempfile::tempdir().expect("destination should be created");
        let path = directory.path().join("tracker.sqlite3");
        let mut connection = db::open_database(
            &path,
            chrono::NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
        )
        .expect("database should open");
        syllabus::seed(&mut connection).expect("syllabus should seed");
        let settings = load_settings(&connection).expect("settings should load");
        assert!(!settings.automatic_enabled);
        let outcome = automatic_backup(
            &mut connection,
            directory.path(),
            Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap(),
        )
        .expect("automatic check should run");
        assert!(!outcome.attempted);

        let saved = update_settings(
            &mut connection,
            BackupSettingsInput {
                automatic_enabled: true,
                frequency: "daily".to_owned(),
                destination: Some(destination.path().display().to_string()),
                retention_count: 2,
            },
        )
        .expect("settings should update");
        assert!(saved.automatic_enabled);
        let outcome = automatic_backup(
            &mut connection,
            directory.path(),
            Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0).unwrap(),
        )
        .expect("automatic backup should run");
        assert!(outcome.attempted && outcome.success);
        let settings = load_settings(&connection).expect("settings should load");
        assert!(settings.last_success_at.is_some());
        assert!(settings.last_success_file.is_some());
        assert!(
            destination
                .path()
                .join(settings.last_success_file.unwrap())
                .is_file()
        );
        let second = automatic_backup(
            &mut connection,
            directory.path(),
            Utc.with_ymd_and_hms(2026, 10, 30, 13, 0, 0).unwrap(),
        )
        .expect("second check should run");
        assert!(!second.attempted);
    }
}

#[cfg(test)]
mod compatibility_tests {
    use rusqlite::Connection;
    use rusqlite_migration::Migrations;

    use super::*;
    use crate::db;

    #[test]
    fn manifests_without_record_counts_remain_compatible() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let connection = db::open_database(
            &source.path().join("tracker.sqlite3"),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
        )
        .expect("database should open");
        let result = create_backup(&connection, destination.path(), "manual", None)
            .expect("backup should succeed");
        let path = destination.path().join(&result.file_name);
        let manifest_file = manifest_path(&path);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_file).expect("manifest should read"))
                .expect("manifest should parse");
        assert!(
            value
                .as_object_mut()
                .expect("manifest should be an object")
                .remove("recordCounts")
                .is_some(),
            "the new manifest should include counts"
        );
        fs::write(
            &manifest_file,
            serde_json::to_vec_pretty(&value).expect("manifest should encode"),
        )
        .expect("legacy manifest should write");

        let verified = verify_backup(&path).expect("legacy manifest should remain valid");
        assert_eq!(verified.record_counts["study_sessions"], 0);
        assert_eq!(verified.record_counts["study_resources"], 0);
    }

    #[test]
    fn older_compatible_backup_previews_for_migration_upgrade() {
        let source = tempfile::tempdir().expect("source temp");
        let destination = tempfile::tempdir().expect("destination temp");
        let path = source.path().join("version-seven.sqlite3");
        let mut connection = Connection::open(&path).expect("database should open");
        Migrations::from_slice(&db::MIGRATIONS_SLICE[..7])
            .to_latest(&mut connection)
            .expect("version seven schema should apply");
        drop(connection);
        let connection = Connection::open(&path).expect("version seven database should reopen");
        assert_eq!(schema_version(&connection).unwrap(), 7);
        let result = create_backup(
            &connection,
            destination.path(),
            "manual",
            Some("0123456789abcdef0123456789abcdef"),
        )
        .expect("older backup should succeed");
        let preview = preview_restore(&destination.path().join(&result.file_name), 10, None)
            .expect("older schema should preview");
        assert!(preview.requires_upgrade);
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("normal migration process"))
        );
    }
}
