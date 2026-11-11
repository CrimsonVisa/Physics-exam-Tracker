use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::AppError,
    v2::{SCHEMA_1_CHECKSUM, SCHEMA_2_CHECKSUM, SCHEMA_4_CHECKSUM, V2_SCHEMA_VERSION},
};

const BACKUP_FORMAT_VERSION: i64 = 1;
const RESTORE_OPERATION_FILE: &str = "restore-operation.json";
const RESTORE_CANDIDATE_FILE: &str = "restore-candidate.sqlite3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BackupSettings {
    pub automatic_enabled: bool,
    pub frequency: String,
    pub destination: Option<String>,
    pub retention_count: i64,
    pub last_success_at: Option<String>,
    pub last_success_file: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_failure_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BackupSettingsInput {
    pub automatic_enabled: bool,
    pub frequency: String,
    #[serde(default)]
    pub destination: Option<String>,
    pub retention_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BackupFile {
    pub file_name: String,
    pub path: String,
    pub size_bytes: i64,
    pub modified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BackupOverview {
    pub settings: V2BackupSettings,
    pub default_directory: String,
    pub schema_version: i64,
    pub database_path: String,
    pub files: Vec<V2BackupFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BackupRecord {
    pub file_name: String,
    pub path: String,
    pub size_bytes: i64,
    pub created_at: String,
    pub schema_version: i64,
    pub exam_count: i64,
    pub removed_old_backups: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RestorePreview {
    pub path: String,
    pub file_name: String,
    pub size_bytes: i64,
    pub schema_version: i64,
    pub target_schema_version: i64,
    pub requires_upgrade: bool,
    pub exam_count: i64,
    pub integrity_check: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RestoreResult {
    pub restored_from: String,
    pub safety_backup: String,
    pub schema_version: i64,
    pub exam_count: i64,
    pub integrity_check: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RestoreRecovery {
    pub safety_directory: String,
    pub files: Vec<V2BackupFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RestoreInput {
    pub path: String,
}

pub fn default_backup_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

pub fn safety_backup_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("backups").join("safety")
}

fn v1_data_directory(data_dir: &Path) -> PathBuf {
    data_dir
        .parent()
        .map(|parent| parent.join(crate::app_paths::V1_APP_IDENTIFIER))
        .unwrap_or_else(|| PathBuf::from(crate::app_paths::V1_APP_IDENTIFIER))
}

/// Resolve a path for containment checks. Existing paths are canonicalized so
/// junctions, symlinks, and Windows case differences cannot hide V1 storage.
/// For a path that does not exist yet, the nearest existing ancestor is
/// canonicalized and the remaining components are appended.
fn resolve_for_containment(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let mut missing = Vec::new();
    let mut current = path.to_path_buf();
    while let Some(name) = current.file_name().map(|value| value.to_os_string()) {
        missing.push(name);
        let Some(parent) = current.parent() else {
            break;
        };
        if parent.as_os_str().is_empty() {
            break;
        }
        current = parent.to_path_buf();
        if let Ok(canonical) = current.canonicalize() {
            let mut resolved = canonical;
            for component in missing.iter().rev() {
                resolved.push(component);
            }
            return resolved;
        }
    }
    path.to_path_buf()
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

fn path_is_within(child: &Path, parent: &Path) -> bool {
    let child = path_key(child);
    let parent = path_key(parent);
    child == parent || child.starts_with(&format!("{parent}\\"))
}

/// Reject backup destinations that resolve inside the V1 application-data
/// directory. This guard is applied before settings are saved and before any
/// backup file is created, listed, or pruned.
pub fn ensure_backup_destination_allowed(
    data_dir: &Path,
    destination: &Path,
) -> Result<(), AppError> {
    if destination
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(AppError::validation(
            "destination",
            "Backup destination cannot contain '..' path traversal components.",
        ));
    }
    if !destination.is_absolute() {
        return Err(AppError::validation(
            "destination",
            "Backup destination must be an absolute folder path.",
        ));
    }
    let destination = resolve_for_containment(destination);
    let v1_directory = resolve_for_containment(&v1_data_directory(data_dir));
    if path_is_within(&destination, &v1_directory) {
        return Err(AppError::validation(
            "destination",
            "Backup destination cannot be inside the V1 application-data folder. Choose a V2 or non-V1 location.",
        ));
    }
    Ok(())
}

pub fn load_settings(connection: &Connection) -> Result<V2BackupSettings, AppError> {
    connection
        .query_row(
            "SELECT automatic_enabled, frequency, destination, retention_count,
                    last_success_at, last_success_file, last_failure_at, last_failure_message
             FROM backup_settings WHERE id = 1",
            [],
            |row| {
                Ok(V2BackupSettings {
                    automatic_enabled: row.get::<_, i64>(0)? == 1,
                    frequency: row.get(1)?,
                    destination: row.get(2)?,
                    retention_count: row.get(3)?,
                    last_success_at: row.get(4)?,
                    last_success_file: row.get(5)?,
                    last_failure_at: row.get(6)?,
                    last_failure_message: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Backup settings could not be loaded", error))?
        .ok_or_else(|| AppError::database_unavailable("Backup settings are missing."))
}

pub fn update_settings(
    connection: &mut Connection,
    data_dir: &Path,
    input: V2BackupSettingsInput,
    now: DateTime<Utc>,
) -> Result<V2BackupSettings, AppError> {
    if !matches!(input.frequency.as_str(), "daily" | "weekly" | "manual_only") {
        return Err(AppError::validation(
            "frequency",
            "Choose daily, weekly, or manual only.",
        ));
    }
    if !(1..=100).contains(&input.retention_count) {
        return Err(AppError::validation(
            "retentionCount",
            "Retention must be between 1 and 100 backups.",
        ));
    }
    let destination = input
        .destination
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned());
    if let Some(path) = &destination {
        if path.chars().count() > 2_048 {
            return Err(AppError::validation(
                "destination",
                "Backup destination is too long.",
            ));
        }
        let candidate = Path::new(path);
        if !candidate.is_absolute() {
            return Err(AppError::validation(
                "destination",
                "Backup destination must be an absolute folder path.",
            ));
        }
        ensure_backup_destination_allowed(data_dir, candidate)?;
    }
    connection
        .execute(
            "UPDATE backup_settings
             SET automatic_enabled = ?1, frequency = ?2, destination = ?3,
                 retention_count = ?4, updated_at = ?5
             WHERE id = 1",
            params![
                i64::from(input.automatic_enabled),
                input.frequency,
                destination,
                input.retention_count,
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
            ],
        )
        .map_err(|error| AppError::database("Backup settings could not be saved", error))?;
    load_settings(connection)
}

pub fn get_overview(
    connection: &Connection,
    data_dir: &Path,
    database_path: &Path,
) -> Result<V2BackupOverview, AppError> {
    let settings = load_settings(connection)?;
    let directory = backup_directory(data_dir, &settings)?;
    let files = list_backup_files(&directory)?;
    Ok(V2BackupOverview {
        settings,
        default_directory: directory.display().to_string(),
        schema_version: V2_SCHEMA_VERSION,
        database_path: database_path.display().to_string(),
        files,
    })
}

pub fn create_manual_backup(
    connection: &mut Connection,
    data_dir: &Path,
    now: DateTime<Utc>,
) -> Result<V2BackupRecord, AppError> {
    create_backup(connection, data_dir, "manual", now)
}

pub fn run_automatic_backup_if_due(
    connection: &mut Connection,
    data_dir: &Path,
    now: DateTime<Utc>,
) -> Result<Option<V2BackupRecord>, AppError> {
    let settings = load_settings(connection)?;
    if !settings.automatic_enabled || settings.frequency == "manual_only" {
        return Ok(None);
    }
    let due = match settings.last_success_at.as_deref() {
        None => true,
        Some(value) => {
            let last = DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|value| value.with_timezone(&Utc));
            match last {
                None => true,
                Some(last) => {
                    let elapsed = now - last;
                    if settings.frequency == "weekly" {
                        elapsed.num_days() >= 7
                    } else {
                        elapsed.num_days() >= 1
                    }
                }
            }
        }
    };
    if !due {
        return Ok(None);
    }
    create_backup(connection, data_dir, "automatic", now).map(Some)
}

pub fn preview_restore(path: &str) -> Result<V2RestorePreview, AppError> {
    let path = Path::new(path.trim());
    let preview = verify_backup_file(path)?;
    Ok(preview)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreOperation {
    op_id: String,
    stage: String,
    created_at: String,
    updated_at: String,
    candidate_path: String,
    candidate_sha: String,
    safety_path: String,
    safety_sha: String,
    previous_live_path: String,
    live_path: String,
    last_error: Option<String>,
}

fn restore_operation_path(data_dir: &Path) -> PathBuf {
    data_dir.join(RESTORE_OPERATION_FILE)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::v2_file_operation("The recovery metadata folder could not be created", error)
        })?;
    }
    let temp = PathBuf::from(format!("{}.tmp", path.display()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        AppError::restore_failed(format!(
            "The recovery journal could not be serialized: {error}"
        ))
    })?;
    {
        let mut file = fs::File::create(&temp).map_err(|error| {
            AppError::v2_file_operation("The recovery journal could not be written", error)
        })?;
        file.write_all(&bytes).map_err(|error| {
            AppError::v2_file_operation("The recovery journal could not be written", error)
        })?;
        file.sync_all().map_err(|error| {
            AppError::v2_file_operation("The recovery journal could not be flushed", error)
        })?;
    }
    replace_file(&temp, path).map_err(|error| {
        AppError::restore_failed(format!(
            "The recovery journal could not be installed: {error:?}"
        ))
    })
}

fn read_restore_operation(data_dir: &Path) -> Result<Option<RestoreOperation>, AppError> {
    let path = restore_operation_path(data_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|error| {
        AppError::restore_failed(format!("The restore journal could not be read: {error}"))
    })?;
    serde_json::from_str(&text).map(Some).map_err(|error| {
        AppError::restore_failed(format!("The restore journal is invalid: {error}"))
    })
}

fn update_restore_stage(
    data_dir: &Path,
    operation: &mut RestoreOperation,
    stage: &str,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    operation.stage = stage.to_owned();
    operation.updated_at = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    write_json_atomic(&restore_operation_path(data_dir), operation)
}

fn restore_candidate_path(data_dir: &Path) -> PathBuf {
    data_dir.join(RESTORE_CANDIDATE_FILE)
}

fn restore_candidate_temp_path(data_dir: &Path) -> PathBuf {
    data_dir.join(format!("{RESTORE_CANDIDATE_FILE}.tmp"))
}

fn database_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn remove_database_sidecars(path: &Path) -> Result<(), AppError> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = database_sidecar_path(path, suffix);
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::v2_file_operation(
                    "The database sidecar could not be prepared for restore",
                    error,
                ));
            }
        }
    }
    Ok(())
}

fn remove_database_sidecars_quiet(path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let _ = fs::remove_file(database_sidecar_path(path, suffix));
    }
}

fn clear_restore_operation(data_dir: &Path) {
    let _ = fs::remove_file(restore_operation_path(data_dir));
    let candidate = restore_candidate_path(data_dir);
    let _ = fs::remove_file(&candidate);
    remove_database_sidecars_quiet(&candidate);
    let temp = restore_candidate_temp_path(data_dir);
    let _ = fs::remove_file(&temp);
    remove_database_sidecars_quiet(&temp);
}

fn copy_file_durable(source: &Path, destination: &Path) -> Result<(), AppError> {
    fs::copy(source, destination).map_err(|error| {
        AppError::v2_file_operation("The restore file could not be copied", error)
    })?;
    let file = fs::OpenOptions::new()
        .write(true)
        .open(destination)
        .map_err(|error| {
            AppError::v2_file_operation("The restore copy could not be opened for flushing", error)
        })?;
    file.sync_all().map_err(|error| {
        AppError::v2_file_operation("The restore copy could not be flushed to disk", error)
    })?;
    Ok(())
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), AppError> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(first_error) => {
            // Windows cannot always replace an existing target while a reader
            // or antivirus scanner holds it. Keep the fallback explicit and
            // never remove the destination unless the replacement exists.
            if destination.exists() {
                fs::remove_file(destination).map_err(|_| {
                    AppError::restore_failed(format!(
                        "The file {} could not be replaced: {first_error}",
                        destination.display()
                    ))
                })?;
                fs::rename(source, destination).map_err(|error| {
                    AppError::v2_file_operation(
                        "The restore file could not be installed atomically",
                        error,
                    )
                })
            } else {
                Err(AppError::v2_file_operation(
                    "The restore file could not be installed",
                    first_error,
                ))
            }
        }
    }
}

fn verify_snapshot(path: &Path) -> Result<(), AppError> {
    verify_backup_file(path).map(|_| ())
}

fn restore_operation_paths_valid(data_dir: &Path, operation: &RestoreOperation) -> bool {
    let live = data_dir.join(super::V2_DATABASE_FILE_NAME);
    Path::new(&operation.live_path) == live
        && Path::new(&operation.candidate_path) == restore_candidate_path(data_dir)
        && Path::new(&operation.previous_live_path).parent() == Some(data_dir)
        && Path::new(&operation.safety_path).starts_with(safety_backup_directory(data_dir))
}

fn preserve_invalid_live(
    data_dir: &Path,
    live_path: &Path,
    operation_id: &str,
) -> Result<(), AppError> {
    if !live_path.exists() || verify_snapshot(live_path).is_ok() {
        return Ok(());
    }
    let preserved = data_dir.join(format!(
        "physics-tracker.failed-restore-{operation_id}-{}.sqlite3",
        Uuid::new_v4().simple()
    ));
    if let Err(error) = fs::rename(live_path, &preserved) {
        return Err(AppError::v2_file_operation(
            "The damaged live database could not be preserved before recovery",
            error,
        ));
    }
    for suffix in ["-wal", "-shm"] {
        let source = database_sidecar_path(live_path, suffix);
        if source.exists() {
            let _ = fs::rename(&source, database_sidecar_path(&preserved, suffix));
        }
    }
    Ok(())
}

fn previous_live_path(database_path: &Path, operation_id: &str) -> PathBuf {
    let stem = database_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("physics-tracker");
    database_path.with_file_name(format!("{stem}.pre-restore-{operation_id}.sqlite3"))
}

fn upgrade_candidate_file(path: &Path) -> Result<(), AppError> {
    let mut connection = Connection::open(path).map_err(|error| {
        AppError::database("The restore candidate could not be upgraded", error)
    })?;
    super::configure(&connection)?;
    super::upgrade_open_connection_to_current(&mut connection)?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| {
            AppError::database(
                "The upgraded restore candidate could not be checkpointed",
                error,
            )
        })?;
    drop(connection);
    remove_database_sidecars(path)?;
    verify_snapshot(path)?;
    Ok(())
}

fn create_verified_candidate(
    data_dir: &Path,
    source_path: &Path,
    requires_upgrade: bool,
) -> Result<(PathBuf, String), AppError> {
    let candidate = restore_candidate_path(data_dir);
    let temp = restore_candidate_temp_path(data_dir);
    let _ = fs::remove_file(&candidate);
    remove_database_sidecars_quiet(&candidate);
    let _ = fs::remove_file(&temp);
    remove_database_sidecars_quiet(&temp);

    let source_sha = sha256_file(source_path)?;
    copy_file_durable(source_path, &temp)?;
    let result = (|| -> Result<String, AppError> {
        verify_snapshot(&temp)?;
        let copied_sha = sha256_file(&temp)?;
        if copied_sha != source_sha {
            return Err(AppError::restore_failed(
                "The restore candidate did not match the selected backup checksum.",
            ));
        }
        if requires_upgrade {
            upgrade_candidate_file(&temp)?;
        }
        let expected_candidate_sha = sha256_file(&temp)?;
        remove_database_sidecars(&temp)?;
        replace_file(&temp, &candidate)?;
        let candidate_sha = sha256_file(&candidate)?;
        if candidate_sha != expected_candidate_sha {
            return Err(AppError::restore_failed(
                "The installed restore candidate did not match the prepared candidate.",
            ));
        }
        Ok(candidate_sha)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
        remove_database_sidecars_quiet(&temp);
        let _ = fs::remove_file(&candidate);
        remove_database_sidecars_quiet(&candidate);
    }
    result.map(|sha| (candidate, sha))
}

fn checkpoint_database(database_path: &Path) -> Result<(), AppError> {
    let connection = Connection::open(database_path)
        .map_err(|error| AppError::database("The live database could not be reopened", error))?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| AppError::database("The live WAL could not be checkpointed", error))?;
    Ok(())
}

fn create_safety_snapshot(
    database_path: &Path,
    safety_path: &Path,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if let Some(parent) = safety_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::v2_file_operation("The safety backup folder could not be created", error)
        })?;
    }
    let temp = PathBuf::from(format!("{}.tmp", safety_path.display()));
    let _ = fs::remove_file(&temp);
    let result = (|| -> Result<(), AppError> {
        let connection = Connection::open(database_path).map_err(|error| {
            AppError::database(
                "The live database could not be opened for safety backup",
                error,
            )
        })?;
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|error| AppError::database("The live WAL could not be checkpointed", error))?;
        snapshot_to_path(&connection, &temp)?;
        drop(connection);
        verify_snapshot(&temp)?;
        remove_database_sidecars(&temp)?;
        replace_file(&temp, safety_path)?;
        let preview = verify_backup_file(safety_path)?;
        write_manifest(safety_path, preview.schema_version, preview.exam_count, now)?;
        verify_backup_file(safety_path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
        remove_database_sidecars_quiet(&temp);
    }
    result
}

fn verify_restored_database(path: &Path) -> Result<(i64, i64, String), AppError> {
    let mut connection = Connection::open(path)
        .map_err(|error| AppError::database("The restored database could not be opened", error))?;
    super::configure(&connection)?;
    super::prepare_connection(&mut connection)?;
    let values = verify_connection(&connection)?;
    super::verify_connection(&connection, false)?;
    Ok(values)
}

fn rollback_restore(
    data_dir: &Path,
    database_path: &Path,
    operation: &RestoreOperation,
) -> Result<(), AppError> {
    preserve_invalid_live(data_dir, database_path, &operation.op_id)?;
    remove_database_sidecars(database_path)?;
    let previous = Path::new(&operation.previous_live_path);
    if previous.is_file() && verify_snapshot(previous).is_ok() {
        replace_file(previous, database_path)?;
        verify_snapshot(database_path)?;
        return Ok(());
    }
    let safety = Path::new(&operation.safety_path);
    if safety.is_file()
        && sha256_file(safety)? == operation.safety_sha
        && verify_snapshot(safety).is_ok()
    {
        let recovery_temp = data_dir.join(format!(
            "{RESTORE_CANDIDATE_FILE}.recovery-{}.tmp",
            operation.op_id
        ));
        let _ = fs::remove_file(&recovery_temp);
        copy_file_durable(safety, &recovery_temp)?;
        verify_snapshot(&recovery_temp)?;
        remove_database_sidecars(&recovery_temp)?;
        replace_file(&recovery_temp, database_path)?;
        verify_snapshot(database_path)?;
        return Ok(());
    }
    Err(AppError::restore_failed(
        "The restore could not be completed and no verified previous or safety database was available. All recovery files were preserved.",
    ))
}

fn cleanup_untouched_restore(data_dir: &Path) {
    clear_restore_operation(data_dir);
}

struct RestoreRecoveryFiles<'a> {
    data_dir: &'a Path,
    live: &'a Path,
    candidate: &'a Path,
    candidate_sha: &'a str,
    previous: &'a Path,
    safety: &'a Path,
    safety_sha: &'a str,
    operation_id: &'a str,
}

fn recover_to_live(files: RestoreRecoveryFiles<'_>) -> Result<(), AppError> {
    let RestoreRecoveryFiles {
        data_dir,
        live,
        candidate,
        candidate_sha,
        previous,
        safety,
        safety_sha,
        operation_id,
    } = files;
    let candidate_valid = candidate.is_file()
        && sha256_file(candidate).ok().as_deref() == Some(candidate_sha)
        && verify_snapshot(candidate).is_ok();
    if candidate_valid {
        preserve_invalid_live(data_dir, live, operation_id)?;
        let installed = remove_database_sidecars(live)
            .and_then(|_| replace_file(candidate, live))
            .and_then(|_| verify_snapshot(live));
        if installed.is_ok() {
            return Ok(());
        }
    }
    if previous.is_file() && verify_snapshot(previous).is_ok() {
        preserve_invalid_live(data_dir, live, operation_id)?;
        let installed = remove_database_sidecars(live)
            .and_then(|_| replace_file(previous, live))
            .and_then(|_| verify_snapshot(live));
        if installed.is_ok() {
            return Ok(());
        }
    }
    if safety.is_file()
        && sha256_file(safety).ok().as_deref() == Some(safety_sha)
        && verify_snapshot(safety).is_ok()
    {
        preserve_invalid_live(data_dir, live, operation_id)?;
        let recovery_temp = data_dir.join(format!(
            "{RESTORE_CANDIDATE_FILE}.recovery-{}.tmp",
            Uuid::new_v4().simple()
        ));
        let _ = fs::remove_file(&recovery_temp);
        let installed = copy_file_durable(safety, &recovery_temp)
            .and_then(|_| verify_snapshot(&recovery_temp))
            .and_then(|_| remove_database_sidecars(&recovery_temp))
            .and_then(|_| remove_database_sidecars(live))
            .and_then(|_| replace_file(&recovery_temp, live))
            .and_then(|_| verify_snapshot(live));
        if installed.is_ok() {
            return Ok(());
        }
    }
    Err(AppError::restore_failed(
        "The interrupted restore could not select a verified live, candidate, previous, or safety database. All recovery files were preserved.",
    ))
}

fn preserve_corrupt_restore_journal(data_dir: &Path) -> Result<(), AppError> {
    let path = restore_operation_path(data_dir);
    if !path.exists() {
        return Ok(());
    }
    let preserved = data_dir.join(format!(
        "restore-operation.invalid-{}-{}.json",
        Utc::now().format("%Y%m%d%H%M%S"),
        Uuid::new_v4().simple()
    ));
    fs::rename(&path, &preserved).map_err(|error| {
        AppError::v2_file_operation(
            "The damaged restore journal could not be preserved for review",
            error,
        )
    })
}

fn latest_previous_live(data_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(data_dir).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| {
                        name.contains(".pre-restore-") && name.ends_with(".sqlite3")
                    })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    candidates.into_iter().next()
}

fn latest_safety_snapshot(data_dir: &Path) -> Option<PathBuf> {
    let directory = safety_backup_directory(data_dir);
    let entries = fs::read_dir(directory).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("ptbackup")
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    candidates.into_iter().next()
}

fn recover_without_restore_journal(data_dir: &Path) -> Result<(), AppError> {
    let live = data_dir.join(super::V2_DATABASE_FILE_NAME);
    if live.is_file() && verify_snapshot(&live).is_ok() {
        return Ok(());
    }
    let candidate = restore_candidate_path(data_dir);
    let previous = latest_previous_live(data_dir);
    let safety = latest_safety_snapshot(data_dir);
    preserve_invalid_live(data_dir, &live, "orphan")?;
    remove_database_sidecars(&live)?;
    if candidate.is_file() && verify_snapshot(&candidate).is_ok() {
        replace_file(&candidate, &live)?;
        if verify_snapshot(&live).is_ok() {
            return Ok(());
        }
    }
    if let Some(previous) = previous
        && previous.is_file()
        && verify_snapshot(&previous).is_ok()
    {
        remove_database_sidecars(&live)?;
        replace_file(&previous, &live)?;
        if verify_snapshot(&live).is_ok() {
            return Ok(());
        }
    }
    if let Some(safety) = safety
        && safety.is_file()
        && verify_snapshot(&safety).is_ok()
    {
        remove_database_sidecars(&live)?;
        let temp = data_dir.join(format!(
            "{RESTORE_CANDIDATE_FILE}.recovery-{}.tmp",
            Uuid::new_v4().simple()
        ));
        let _ = fs::remove_file(&temp);
        copy_file_durable(&safety, &temp)?;
        verify_snapshot(&temp)?;
        remove_database_sidecars(&temp)?;
        replace_file(&temp, &live)?;
        if verify_snapshot(&live).is_ok() {
            return Ok(());
        }
    }
    Err(AppError::restore_failed(
        "A damaged restore journal was found and no verified live, candidate, previous, or safety database could be selected. All recovery files were preserved for manual review.",
    ))
}

pub fn recover_interrupted_restore(data_dir: &Path) -> Result<bool, AppError> {
    let operation = match read_restore_operation(data_dir) {
        Ok(Some(operation)) => operation,
        Ok(None) => {
            let temp = restore_candidate_temp_path(data_dir);
            let _ = fs::remove_file(&temp);
            remove_database_sidecars_quiet(&temp);
            let _ = fs::remove_file(PathBuf::from(format!(
                "{}.tmp",
                restore_operation_path(data_dir).display()
            )));
            let live = data_dir.join(super::V2_DATABASE_FILE_NAME);
            if live.is_file() && verify_snapshot(&live).is_ok() {
                let candidate = restore_candidate_path(data_dir);
                let _ = fs::remove_file(&candidate);
                remove_database_sidecars_quiet(&candidate);
                return Ok(false);
            }
            let has_recovery_source = restore_candidate_path(data_dir).is_file()
                || latest_previous_live(data_dir).is_some()
                || latest_safety_snapshot(data_dir).is_some();
            if has_recovery_source {
                recover_without_restore_journal(data_dir)?;
                return Ok(true);
            }
            return Ok(false);
        }
        Err(error) => {
            preserve_corrupt_restore_journal(data_dir)?;
            eprintln!("V2 restore journal was damaged and has been preserved: {error:?}");
            recover_without_restore_journal(data_dir)?;
            return Ok(true);
        }
    };
    if !restore_operation_paths_valid(data_dir, &operation) {
        return Err(AppError::restore_failed(
            "The interrupted restore journal references an unexpected file location. All recovery files were preserved for manual review.",
        ));
    }
    let live = PathBuf::from(&operation.live_path);
    let candidate = PathBuf::from(&operation.candidate_path);
    let previous = PathBuf::from(&operation.previous_live_path);
    let safety = PathBuf::from(&operation.safety_path);
    let live_valid = live.is_file() && verify_snapshot(&live).is_ok();

    match operation.stage.as_str() {
        "validated" | "safety_created" | "db_closed" => {
            if !live_valid {
                preserve_invalid_live(data_dir, &live, &operation.op_id)?;
                recover_to_live(RestoreRecoveryFiles {
                    data_dir,
                    live: &live,
                    candidate: &candidate,
                    candidate_sha: &operation.candidate_sha,
                    previous: &previous,
                    safety: &safety,
                    safety_sha: &operation.safety_sha,
                    operation_id: &operation.op_id,
                })?;
            }
        }
        "live_renamed" => {
            if !live_valid {
                let candidate_valid = candidate.is_file()
                    && sha256_file(&candidate).ok().as_deref()
                        == Some(operation.candidate_sha.as_str())
                    && verify_snapshot(&candidate).is_ok();
                if candidate_valid {
                    preserve_invalid_live(data_dir, &live, &operation.op_id)?;
                    remove_database_sidecars(&live)?;
                    replace_file(&candidate, &live)?;
                    verify_snapshot(&live)?;
                } else {
                    preserve_invalid_live(data_dir, &live, &operation.op_id)?;
                    recover_to_live(RestoreRecoveryFiles {
                        data_dir,
                        live: &live,
                        candidate: &candidate,
                        candidate_sha: &operation.candidate_sha,
                        previous: &previous,
                        safety: &safety,
                        safety_sha: &operation.safety_sha,
                        operation_id: &operation.op_id,
                    })?;
                }
            }
        }
        "candidate_installed" | "verified" | "cleanup_complete" => {
            if !live_valid {
                preserve_invalid_live(data_dir, &live, &operation.op_id)?;
                recover_to_live(RestoreRecoveryFiles {
                    data_dir,
                    live: &live,
                    candidate: Path::new(""),
                    candidate_sha: "",
                    previous: &previous,
                    safety: &safety,
                    safety_sha: &operation.safety_sha,
                    operation_id: &operation.op_id,
                })?;
            }
        }
        _ => {
            return Err(AppError::restore_failed(
                "An interrupted restore journal has an unknown stage. Recovery files were preserved.",
            ));
        }
    }
    clear_restore_operation(data_dir);
    Ok(true)
}

pub fn apply_restore(
    data_dir: &Path,
    database_path: &Path,
    input: V2RestoreInput,
    now: DateTime<Utc>,
) -> Result<V2RestoreResult, AppError> {
    if read_restore_operation(data_dir)?.is_some() {
        return Err(AppError::restore_failed(
            "An interrupted restore is already pending. Restart the application to recover it before restoring again.",
        ));
    }
    let source_path = Path::new(input.path.trim());
    let preview = verify_backup_file(source_path)?;
    if source_path
        .canonicalize()
        .ok()
        .zip(database_path.canonicalize().ok())
        .is_some_and(|(source, live)| source == live)
    {
        return Err(AppError::validation(
            "path",
            "Choose a backup file, not the live application database.",
        ));
    }

    let (candidate_path, candidate_sha) =
        create_verified_candidate(data_dir, source_path, preview.requires_upgrade)?;
    let operation_id = Uuid::new_v4().simple().to_string();
    let safety_path =
        safety_backup_directory(data_dir).join(format!("pre-restore-{operation_id}.ptbackup"));
    if let Err(error) = create_safety_snapshot(database_path, &safety_path, now) {
        cleanup_untouched_restore(data_dir);
        return Err(error);
    }
    let safety_sha = sha256_file(&safety_path)?;
    let previous_path = previous_live_path(database_path, &operation_id);
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut operation = RestoreOperation {
        op_id: operation_id.clone(),
        stage: "validated".to_owned(),
        created_at: now_text.clone(),
        updated_at: now_text,
        candidate_path: candidate_path.display().to_string(),
        candidate_sha,
        safety_path: safety_path.display().to_string(),
        safety_sha,
        previous_live_path: previous_path.display().to_string(),
        live_path: database_path.display().to_string(),
        last_error: None,
    };
    if let Err(error) = write_json_atomic(&restore_operation_path(data_dir), &operation) {
        cleanup_untouched_restore(data_dir);
        return Err(error);
    }
    if let Err(error) = update_restore_stage(data_dir, &mut operation, "safety_created", now) {
        cleanup_untouched_restore(data_dir);
        return Err(error);
    }
    if let Err(error) = checkpoint_database(database_path) {
        cleanup_untouched_restore(data_dir);
        return Err(error);
    }
    if let Err(error) = update_restore_stage(data_dir, &mut operation, "db_closed", now) {
        cleanup_untouched_restore(data_dir);
        return Err(error);
    }

    if previous_path.exists()
        && let Err(error) = fs::remove_file(&previous_path)
    {
        cleanup_untouched_restore(data_dir);
        return Err(AppError::v2_file_operation(
            "A stale previous live database could not be removed",
            error,
        ));
    }
    if let Err(error) = fs::rename(database_path, &previous_path) {
        cleanup_untouched_restore(data_dir);
        return Err(AppError::v2_file_operation(
            "The live database could not be moved aside for restore",
            error,
        ));
    }
    if let Err(error) = remove_database_sidecars(database_path) {
        let rollback = rollback_restore(data_dir, database_path, &operation);
        if rollback.is_ok() {
            cleanup_untouched_restore(data_dir);
        }
        return Err(error);
    }
    if let Err(error) = update_restore_stage(data_dir, &mut operation, "live_renamed", now) {
        let rollback = rollback_restore(data_dir, database_path, &operation);
        if rollback.is_ok() {
            cleanup_untouched_restore(data_dir);
        }
        return Err(error);
    }
    if let Err(error) = replace_file(&candidate_path, database_path) {
        let rollback = rollback_restore(data_dir, database_path, &operation);
        if rollback.is_ok() {
            cleanup_untouched_restore(data_dir);
        }
        return Err(AppError::restore_failed(format!(
            "The restore candidate could not be installed. {}{}",
            error.message,
            if rollback.is_ok() {
                " The original database was recovered."
            } else {
                " All recovery files were preserved."
            }
        )));
    }
    if let Err(error) = update_restore_stage(data_dir, &mut operation, "candidate_installed", now) {
        let rollback = rollback_restore(data_dir, database_path, &operation);
        if rollback.is_ok() {
            cleanup_untouched_restore(data_dir);
        }
        return Err(error);
    }

    match verify_restored_database(database_path) {
        Ok((schema_version, exam_count, integrity_check)) => {
            let _ = update_restore_stage(data_dir, &mut operation, "verified", now);
            clear_restore_operation(data_dir);
            Ok(V2RestoreResult {
                restored_from: preview.path,
                safety_backup: safety_path.display().to_string(),
                schema_version,
                exam_count,
                integrity_check,
            })
        }
        Err(error) => {
            let rollback = rollback_restore(data_dir, database_path, &operation);
            if rollback.is_ok() {
                cleanup_untouched_restore(data_dir);
                Err(AppError::restore_failed(format!(
                    "The restored database did not pass verification and the original database was recovered. {error:?}"
                )))
            } else {
                Err(AppError::restore_failed(format!(
                    "The restored database did not pass verification and automatic rollback failed. All recovery files were preserved. {error:?}"
                )))
            }
        }
    }
}

pub fn get_restore_recovery(data_dir: &Path) -> Result<V2RestoreRecovery, AppError> {
    let directory = safety_backup_directory(data_dir);
    Ok(V2RestoreRecovery {
        safety_directory: directory.display().to_string(),
        files: list_backup_files(&directory)?,
    })
}

fn create_backup(
    connection: &mut Connection,
    data_dir: &Path,
    label: &str,
    now: DateTime<Utc>,
) -> Result<V2BackupRecord, AppError> {
    let settings = load_settings(connection)?;
    let directory = backup_directory(data_dir, &settings)?;
    fs::create_dir_all(&directory).map_err(|error| {
        AppError::v2_file_operation("The backup folder could not be created", error)
    })?;
    let stamp = now.format("%Y%m%d-%H%M%S");
    let file_name = format!("physics-tracker-{label}-{stamp}.ptbackup");
    let path = directory.join(&file_name);
    if path.exists() {
        fs::remove_file(&path).map_err(|error| {
            AppError::v2_file_operation(
                "An existing same-second backup could not be replaced",
                error,
            )
        })?;
    }
    snapshot_to_path(connection, &path)?;
    let verified = verify_backup_file(&path)?;
    let schema_version = verified.schema_version;
    let exam_count = verified.exam_count;
    write_manifest(&path, schema_version, exam_count, now)?;
    let size_bytes = fs::metadata(&path)
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    connection
        .execute(
            "UPDATE backup_settings
             SET last_success_at = ?1, last_success_file = ?2, last_failure_at = NULL,
                 last_failure_message = NULL, updated_at = ?1
             WHERE id = 1",
            params![now_text, path.display().to_string()],
        )
        .map_err(|error| AppError::database("Backup status could not be saved", error))?;
    let removed_old_backups = apply_retention(data_dir, &directory, settings.retention_count)?;
    Ok(V2BackupRecord {
        file_name,
        path: path.display().to_string(),
        size_bytes,
        created_at: now_text,
        schema_version,
        exam_count,
        removed_old_backups,
    })
}

fn backup_directory(data_dir: &Path, settings: &V2BackupSettings) -> Result<PathBuf, AppError> {
    let directory = settings
        .destination
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_backup_directory(data_dir));
    ensure_backup_destination_allowed(data_dir, &directory)?;
    Ok(directory)
}

fn snapshot_to_path(connection: &Connection, path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::v2_file_operation("The backup folder could not be created", error)
        })?;
    }
    let mut destination = Connection::open(path)
        .map_err(|error| AppError::database("The backup file could not be created", error))?;
    let backup = rusqlite::backup::Backup::new(connection, &mut destination)
        .map_err(|error| AppError::database("The backup could not start", error))?;
    backup
        .run_to_completion(5, Duration::from_millis(5), None)
        .map_err(|error| AppError::database("The backup could not complete", error))?;
    drop(backup);
    destination
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|error| AppError::database("The backup checkpoint could not complete", error))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    format_version: i64,
    schema_version: i64,
    exam_count: i64,
    size_bytes: i64,
    sha256: String,
    created_at: String,
}

fn manifest_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.manifest.json", path.display()))
}

fn sha256_file(path: &Path) -> Result<String, AppError> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::file_operation("The backup checksum could not be read", error)
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn write_manifest(
    path: &Path,
    schema_version: i64,
    exam_count: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::file_operation("The backup metadata could not be read", error)
    })?;
    let manifest = BackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        schema_version,
        exam_count,
        size_bytes: metadata.len() as i64,
        sha256: sha256_file(path)?,
        created_at: now.to_rfc3339_opts(SecondsFormat::Millis, true),
    };
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        AppError::backup_failed(format!(
            "The backup manifest could not be serialized: {error}"
        ))
    })?;
    fs::write(manifest_path(path), bytes).map_err(|error| {
        AppError::v2_file_operation("The backup manifest could not be written", error)
    })
}

fn supported_restore_schema(version: i64) -> bool {
    (1..=V2_SCHEMA_VERSION).contains(&version)
}

fn expected_schema_checksum(version: i64) -> Option<String> {
    match version {
        1 => Some(SCHEMA_1_CHECKSUM.to_owned()),
        2 => Some(SCHEMA_2_CHECKSUM.to_owned()),
        3 => Some(super::SCHEMA_3_CHECKSUM.to_owned()),
        4 => Some(SCHEMA_4_CHECKSUM.to_owned()),
        V2_SCHEMA_VERSION => Some(super::initial_schema_checksum()),
        _ => None,
    }
}

fn verify_manifest(path: &Path, size_bytes: u64) -> Result<(), AppError> {
    let text = fs::read_to_string(manifest_path(path)).map_err(|error| {
        AppError::restore_failed(format!("The backup manifest could not be read: {error}"))
    })?;
    let manifest: BackupManifest = serde_json::from_str(&text).map_err(|error| {
        AppError::restore_failed(format!("The backup manifest is invalid: {error}"))
    })?;
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(AppError::restore_failed(
            "The backup manifest format version is unsupported.",
        ));
    }
    if !supported_restore_schema(manifest.schema_version) || manifest.exam_count != 4 {
        return Err(AppError::restore_failed(
            "The backup manifest schema or exam count is invalid.",
        ));
    }
    if manifest.size_bytes != size_bytes as i64 {
        return Err(AppError::restore_failed(
            "The backup file size does not match its manifest.",
        ));
    }
    if manifest.sha256 != sha256_file(path)? {
        return Err(AppError::restore_failed(
            "The backup checksum does not match its manifest.",
        ));
    }
    Ok(())
}

fn verify_backup_file(path: &Path) -> Result<V2RestorePreview, AppError> {
    if !path.is_file() {
        return Err(AppError::validation(
            "path",
            "Choose an existing .ptbackup file.",
        ));
    }
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::file_operation("The backup file could not be read", error))?;
    if metadata.len() == 0 || metadata.len() > 2 * 1024 * 1024 * 1024 {
        return Err(AppError::validation(
            "path",
            "The backup file size is not valid.",
        ));
    }
    if manifest_path(path).is_file() {
        verify_manifest(path, metadata.len())?;
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| {
        AppError::restore_failed(format!("The backup is not a SQLite database: {error}"))
    })?;
    let integrity_check: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(|error| {
            AppError::restore_failed(format!("The backup integrity check failed: {error}"))
        })?;
    if integrity_check != "ok" {
        return Err(AppError::restore_failed(format!(
            "The backup integrity check returned {integrity_check}."
        )));
    }
    let schema_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| {
            AppError::restore_failed(format!("The backup schema could not be read: {error}"))
        })?;
    if !supported_restore_schema(schema_version) {
        return Err(AppError::restore_failed(format!(
            "The backup schema version {schema_version} is not supported."
        )));
    }
    let exam_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM exams", [], |row| row.get(0))
        .map_err(|error| {
            AppError::restore_failed(format!("The backup exam list could not be read: {error}"))
        })?;
    if exam_count != 4 {
        return Err(AppError::restore_failed(
            "The backup does not contain exactly four examinations.",
        ));
    }
    let metadata_version: Option<i64> = connection
        .query_row(
            "SELECT schema_version FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::restore_failed(format!("The backup metadata could not be read: {error}"))
        })?;
    if metadata_version != Some(schema_version) {
        return Err(AppError::restore_failed(
            "The backup schema metadata is missing or unsupported.",
        ));
    }
    let identity: Option<String> = connection
        .query_row(
            "SELECT database_id FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::restore_failed(format!(
                "The backup database identity could not be read: {error}"
            ))
        })?;
    if !super::valid_database_identity(identity.as_deref()) {
        return Err(AppError::restore_failed(
            "The backup database identity metadata is missing or invalid.",
        ));
    }
    let checksum: Option<String> = connection
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [schema_version],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::restore_failed(format!(
                "The backup migration checksum could not be read: {error}"
            ))
        })?;
    let expected_checksum = expected_schema_checksum(schema_version).ok_or_else(|| {
        AppError::restore_failed("The backup schema version does not have a known checksum.")
    })?;
    if checksum.as_deref() != Some(expected_checksum.as_str()) {
        return Err(AppError::restore_failed(
            "The backup migration checksum does not match the expected schema.",
        ));
    }
    let violations: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|error| {
            AppError::restore_failed(format!("The backup foreign-key check failed: {error}"))
        })?;
    if violations != 0 {
        return Err(AppError::restore_failed(
            "The backup contains foreign-key violations.",
        ));
    }
    Ok(V2RestorePreview {
        path: path.display().to_string(),
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("backup.ptbackup")
            .to_owned(),
        size_bytes: metadata.len() as i64,
        schema_version,
        target_schema_version: V2_SCHEMA_VERSION,
        requires_upgrade: schema_version != V2_SCHEMA_VERSION,
        exam_count,
        integrity_check,
    })
}

fn verify_connection(connection: &Connection) -> Result<(i64, i64, String), AppError> {
    let integrity_check: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(|error| {
            AppError::database("The restored database integrity check failed", error)
        })?;
    if integrity_check != "ok" {
        return Err(AppError::v2_integrity(format!(
            "The restored database integrity check returned {integrity_check}."
        )));
    }
    let schema_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| {
            AppError::database("The restored schema version could not be read", error)
        })?;
    if schema_version != V2_SCHEMA_VERSION {
        return Err(AppError::v2_integrity(
            "The restored database schema version is unsupported.",
        ));
    }
    let exam_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM exams", [], |row| row.get(0))
        .map_err(|error| AppError::database("The restored exam list could not be read", error))?;
    if exam_count != 4 {
        return Err(AppError::v2_integrity(
            "The restored database does not contain exactly four examinations.",
        ));
    }
    let violations: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|error| AppError::database("The restored foreign-key check failed", error))?;
    if violations != 0 {
        return Err(AppError::v2_foreign_key(
            "The restored database contains foreign-key violations.",
        ));
    }
    Ok((schema_version, exam_count, integrity_check))
}

fn list_backup_files(directory: &Path) -> Result<Vec<V2BackupFile>, AppError> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let entries = fs::read_dir(directory).map_err(|error| {
        AppError::v2_file_operation("The backup folder could not be listed", error)
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::v2_file_operation("A backup folder entry could not be read", error)
        })?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("ptbackup")
        {
            continue;
        }
        let metadata = entry.metadata().map_err(|error| {
            AppError::v2_file_operation("A backup file could not be inspected", error)
        })?;
        let modified = metadata
            .modified()
            .ok()
            .map(|value| {
                let date: DateTime<Utc> = value.into();
                date.to_rfc3339_opts(SecondsFormat::Millis, true)
            })
            .unwrap_or_default();
        files.push(V2BackupFile {
            file_name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("backup.ptbackup")
                .to_owned(),
            path: path.display().to_string(),
            size_bytes: metadata.len() as i64,
            modified_at: modified,
        });
    }
    files.sort_by(|left, right| right.file_name.cmp(&left.file_name));
    Ok(files)
}

fn apply_retention(
    data_dir: &Path,
    directory: &Path,
    retention_count: i64,
) -> Result<i64, AppError> {
    ensure_backup_destination_allowed(data_dir, directory)?;
    let mut files = list_backup_files(directory)?
        .into_iter()
        .filter(|file| !file.file_name.starts_with("pre-restore-"))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| right.file_name.cmp(&left.file_name));
    let mut removed = 0;
    for file in files.into_iter().skip(retention_count as usize) {
        fs::remove_file(&file.path).map_err(|error| {
            AppError::v2_file_operation("An old backup could not be removed", error)
        })?;
        let _ = fs::remove_file(manifest_path(Path::new(&file.path)));
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{
        SCHEMA_3_CHECKSUM, SCHEMA_4_CHECKSUM, V2State, admin, reference, sticky_notes,
    };

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
            + chrono::Duration::seconds(seconds)
    }

    #[test]
    fn backup_preview_and_restore_round_trip_preserve_data() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        state
            .with_connection(|connection| {
                admin::update_settings(
                    connection,
                    admin::V2UpdateAppSettingsInput {
                        preparation_start_date: Some("2026-10-01".to_owned()),
                        daily_study_target_minutes: 180,
                    },
                    at(0),
                )
            })
            .expect("settings");

        let record = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        assert_eq!(record.exam_count, 4);
        assert!(Path::new(&record.path).is_file());
        assert_eq!(record.schema_version, V2_SCHEMA_VERSION);
        assert!(manifest_path(Path::new(&record.path)).is_file());

        let preview = preview_restore(&record.path).expect("preview");
        assert_eq!(preview.exam_count, 4);
        assert_eq!(preview.integrity_check, "ok");

        state
            .with_connection(|connection| {
                admin::update_settings(
                    connection,
                    admin::V2UpdateAppSettingsInput {
                        preparation_start_date: None,
                        daily_study_target_minutes: 210,
                    },
                    at(2),
                )
            })
            .expect("changed settings");
        let result = state
            .restore_backup(
                V2RestoreInput {
                    path: record.path.clone(),
                },
                at(3),
            )
            .expect("restore");
        assert_eq!(result.exam_count, 4);
        assert_eq!(result.integrity_check, "ok");
        assert!(!restore_operation_path(&data_dir).exists());
        assert!(!restore_candidate_path(&data_dir).exists());
        for suffix in ["-wal", "-shm"] {
            assert!(!database_sidecar_path(&restore_candidate_path(&data_dir), suffix).exists());
        }
        let foreign_key_violations = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|error| AppError::database("Foreign keys could not be checked", error))
            })
            .expect("foreign keys");
        assert_eq!(foreign_key_violations, 0);
        let restored = state
            .with_connection(|connection| admin::load_settings(connection))
            .expect("restored settings");
        assert_eq!(restored.daily_study_target_minutes, 180);
        assert_eq!(
            restored.preparation_start_date.as_deref(),
            Some("2026-10-01")
        );
        assert!(Path::new(&result.safety_backup).is_file());

        let recovery = get_restore_recovery(&data_dir).expect("recovery");
        assert!(!recovery.files.is_empty());
    }

    #[test]
    fn custom_reference_records_survive_manual_backup_and_restore() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        state
            .with_connection(|connection| {
                reference::save_custom_formula(
                    connection,
                    reference::V2CustomFormulaInput {
                        id: None,
                        name: "Backup formula".to_owned(),
                        latex: "F = ma".to_owned(),
                        plain_text: "F = m a".to_owned(),
                        category: "Classical mechanics".to_owned(),
                        description: "Newton's second law.".to_owned(),
                        notes: String::new(),
                        source: String::new(),
                        variables: vec![reference::V2CustomFormulaVariableInput {
                            symbol: "F".to_owned(),
                            meaning: "force".to_owned(),
                            unit: "N".to_owned(),
                        }],
                    },
                    at(0),
                )?;
                reference::save_custom_unit(
                    connection,
                    reference::V2CustomUnitInput {
                        id: None,
                        name: "Backup unit".to_owned(),
                        symbol: "bu".to_owned(),
                        quantity: "Length".to_owned(),
                        dimension: "L".to_owned(),
                        si_classification: "non_si".to_owned(),
                        conversion_kind: "scalar".to_owned(),
                        conversion_factor: "2.5".to_owned(),
                        conversion_reference_unit: "m".to_owned(),
                        conversion_note: String::new(),
                        description: "A test unit.".to_owned(),
                        notes: String::new(),
                        source: String::new(),
                    },
                    at(1),
                )?;
                reference::save_custom_constant(
                    connection,
                    reference::V2CustomConstantInput {
                        id: None,
                        name: "Backup constant".to_owned(),
                        symbol: "bc".to_owned(),
                        value_text: "6.62607015×10^-34".to_owned(),
                        unit: "J s".to_owned(),
                        category: "Fundamental".to_owned(),
                        uncertainty_text: String::new(),
                        precision_text: "9".to_owned(),
                        source: String::new(),
                        description: "A test constant.".to_owned(),
                        notes: String::new(),
                    },
                    at(2),
                )?;
                Ok::<(), AppError>(())
            })
            .expect("custom references");

        let record = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(3)))
            .expect("backup");

        state
            .with_connection(|connection| {
                admin::update_settings(
                    connection,
                    admin::V2UpdateAppSettingsInput {
                        preparation_start_date: None,
                        daily_study_target_minutes: 210,
                    },
                    at(4),
                )
            })
            .expect("changed settings");

        let result = state
            .restore_backup(
                V2RestoreInput {
                    path: record.path.clone(),
                },
                at(5),
            )
            .expect("restore");
        assert_eq!(result.exam_count, 4);

        let (formulas, units, constants) = state
            .with_connection(|connection| {
                Ok::<_, AppError>((
                    reference::list_custom_formulas(connection)?,
                    reference::list_custom_units(connection)?,
                    reference::list_custom_constants(connection)?,
                ))
            })
            .expect("restored custom references");
        assert_eq!(formulas.len(), 1);
        assert_eq!(formulas[0].name, "Backup formula");
        assert_eq!(formulas[0].variables[0].symbol, "F");
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].conversion_factor, "2.5");
        assert_eq!(constants.len(), 1);
        assert_eq!(constants[0].value_text, "6.62607015×10^-34");
        assert_eq!(constants[0].precision_text, "9");
    }

    #[test]
    fn sticky_notes_survive_manual_backup_and_restore() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        state
            .with_connection(|connection| {
                sticky_notes::save_sticky_note(
                    connection,
                    sticky_notes::V2StickyNoteInput {
                        id: None,
                        title: "Backup note".to_owned(),
                        body: "Preserve this note across restore.".to_owned(),
                        color: "blue".to_owned(),
                        is_pinned: true,
                        exam_id: Some(1),
                        subject_node_id: None,
                        topic_node_id: None,
                    },
                    at(0),
                )?;
                Ok::<(), AppError>(())
            })
            .expect("sticky note");

        let record = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        state
            .with_connection(|connection| sticky_notes::delete_sticky_note(connection, 1))
            .expect("delete note");
        state
            .restore_backup(
                V2RestoreInput {
                    path: record.path.clone(),
                },
                at(2),
            )
            .expect("restore");

        let notes = state
            .with_connection(|connection| sticky_notes::list_sticky_notes(connection))
            .expect("restored notes");
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Backup note");
        assert_eq!(notes[0].body, "Preserve this note across restore.");
        assert_eq!(notes[0].color, "blue");
        assert!(notes[0].is_pinned);
        assert_eq!(notes[0].exam_code.as_deref(), Some("csir-net-physics"));
    }

    #[test]
    fn invalid_backup_files_are_rejected_before_restore() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let invalid = tempfile::NamedTempFile::new().expect("file");
        fs::write(invalid.path(), b"not sqlite").expect("write");
        let error =
            preview_restore(&invalid.path().display().to_string()).expect_err("invalid backup");
        assert_eq!(error.code, "restore_failed");

        let missing = preview_restore(&data_dir.join("missing.ptbackup").display().to_string())
            .expect_err("missing backup");
        assert_eq!(missing.code, "validation_error");
    }

    #[test]
    fn automatic_backup_respects_disabled_and_due_settings() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let none = state
            .with_connection(|connection| run_automatic_backup_if_due(connection, &data_dir, at(0)))
            .expect("disabled");
        assert!(none.is_none());

        state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    &data_dir,
                    V2BackupSettingsInput {
                        automatic_enabled: true,
                        frequency: "daily".to_owned(),
                        destination: None,
                        retention_count: 5,
                    },
                    at(1),
                )
            })
            .expect("settings");
        let first = state
            .with_connection(|connection| run_automatic_backup_if_due(connection, &data_dir, at(2)))
            .expect("first")
            .expect("created");
        assert!(first.path.ends_with(".ptbackup"));
        let second = state
            .with_connection(|connection| run_automatic_backup_if_due(connection, &data_dir, at(3)))
            .expect("not due");
        assert!(second.is_none());
    }

    #[test]
    fn backup_destinations_inside_v1_are_rejected_before_any_write_or_retention() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = root.path().join(crate::app_paths::V2_APP_IDENTIFIER);
        let state = V2State::initialize(&data_dir);
        let v1_dir = root.path().join(crate::app_paths::V1_APP_IDENTIFIER);
        fs::create_dir_all(&v1_dir).expect("v1 dir");
        let sentinel = v1_dir.join("sentinel.txt");
        fs::write(&sentinel, b"v1 sentinel").expect("v1 sentinel");
        let v1_backup = v1_dir.join("old.ptbackup");
        fs::write(&v1_backup, b"v1 backup sentinel").expect("v1 backup");

        let invalid_input = |destination: String| V2BackupSettingsInput {
            automatic_enabled: false,
            frequency: "manual_only".to_owned(),
            destination: Some(destination),
            retention_count: 3,
        };
        let direct = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    &data_dir,
                    invalid_input(v1_dir.display().to_string()),
                    at(0),
                )
            })
            .expect_err("V1 destination must be rejected");
        assert_eq!(direct.code, "validation_error");
        assert_eq!(direct.field, Some("destination"));

        let case_variant = root
            .path()
            .join(crate::app_paths::V1_APP_IDENTIFIER.to_uppercase());
        let case_error = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    &data_dir,
                    invalid_input(case_variant.display().to_string()),
                    at(1),
                )
            })
            .expect_err("case variant must be rejected");
        assert_eq!(case_error.code, "validation_error");

        let traversal = data_dir
            .join("backups")
            .join("..")
            .join(crate::app_paths::V1_APP_IDENTIFIER);
        let traversal_error = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    &data_dir,
                    invalid_input(traversal.display().to_string()),
                    at(2),
                )
            })
            .expect_err("traversal must be rejected");
        assert_eq!(traversal_error.code, "validation_error");

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE backup_settings
                         SET automatic_enabled = 1, frequency = 'daily', destination = ?1,
                             last_success_at = NULL, last_success_file = NULL
                         WHERE id = 1",
                        [v1_dir.to_string_lossy().to_string()],
                    )
                    .map_err(|error| {
                        AppError::database("The V1 destination bypass could not be stored", error)
                    })?;
                Ok(())
            })
            .expect("stored invalid destination");

        let manual_error = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(3)))
            .expect_err("manual backup must reject a stored V1 destination");
        assert_eq!(manual_error.code, "validation_error");
        let automatic_error = state
            .with_connection(|connection| run_automatic_backup_if_due(connection, &data_dir, at(4)))
            .expect_err("automatic backup must reject a stored V1 destination");
        assert_eq!(automatic_error.code, "validation_error");
        let overview_error = state
            .with_connection(|connection| {
                get_overview(
                    connection,
                    &data_dir,
                    &data_dir.join(super::super::V2_DATABASE_FILE_NAME),
                )
            })
            .expect_err("overview must reject a stored V1 destination");
        assert_eq!(overview_error.code, "validation_error");
        let retention_error = apply_retention(&data_dir, &v1_dir, 1)
            .expect_err("retention must reject the V1 directory");
        assert_eq!(retention_error.code, "validation_error");

        assert_eq!(fs::read(&sentinel).expect("sentinel"), b"v1 sentinel");
        assert_eq!(
            fs::read(&v1_backup).expect("v1 backup sentinel"),
            b"v1 backup sentinel"
        );

        let valid_destination = data_dir.join("custom-backups");
        let saved = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    &data_dir,
                    V2BackupSettingsInput {
                        automatic_enabled: false,
                        frequency: "manual_only".to_owned(),
                        destination: Some(valid_destination.display().to_string()),
                        retention_count: 3,
                    },
                    at(5),
                )
            })
            .expect("valid V2 destination");
        assert_eq!(
            saved.destination.as_deref(),
            Some(valid_destination.display().to_string().as_str())
        );
        let record = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(6)))
            .expect("valid backup");
        assert!(Path::new(&record.path).is_file());
        assert!(Path::new(&record.path).starts_with(&valid_destination));
        assert_eq!(fs::read(&sentinel).expect("sentinel"), b"v1 sentinel");
    }

    fn checkpoint(state: &V2State) {
        state
            .with_connection(|connection| {
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(|error| {
                        AppError::database("The test checkpoint could not complete", error)
                    })
            })
            .expect("checkpoint");
    }

    fn set_target(state: &V2State, minutes: i64, seconds: i64) {
        state
            .with_connection(|connection| {
                admin::update_settings(
                    connection,
                    admin::V2UpdateAppSettingsInput {
                        preparation_start_date: None,
                        daily_study_target_minutes: minutes,
                    },
                    at(seconds),
                )
            })
            .expect("settings");
    }

    fn make_operation(
        database_path: &Path,
        stage: &str,
        candidate: &Path,
        safety: &Path,
    ) -> RestoreOperation {
        let operation_id = "test-operation".to_owned();
        let now_text = at(10).to_rfc3339_opts(SecondsFormat::Millis, true);
        RestoreOperation {
            op_id: operation_id.clone(),
            stage: stage.to_owned(),
            created_at: now_text.clone(),
            updated_at: now_text,
            candidate_path: candidate.display().to_string(),
            candidate_sha: sha256_file(candidate).expect("candidate checksum"),
            safety_path: safety.display().to_string(),
            safety_sha: sha256_file(safety).expect("safety checksum"),
            previous_live_path: previous_live_path(database_path, &operation_id)
                .display()
                .to_string(),
            live_path: database_path.display().to_string(),
            last_error: None,
        }
    }

    #[test]
    fn interrupted_restore_at_live_renamed_installs_verified_candidate() {
        let (root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let database_path = state.database_path().expect("database path");
        set_target(&state, 180, 0);
        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        set_target(&state, 210, 2);
        checkpoint(&state);
        drop(state);

        let candidate = restore_candidate_path(&data_dir);
        fs::copy(&backup.path, &candidate).expect("candidate copy");
        let safety = safety_backup_directory(&data_dir).join("pre-restore-test.ptbackup");
        fs::create_dir_all(safety.parent().expect("safety parent")).expect("safety folder");
        fs::copy(&database_path, &safety).expect("safety copy");
        let operation = make_operation(&database_path, "live_renamed", &candidate, &safety);
        write_json_atomic(&restore_operation_path(&data_dir), &operation).expect("journal");
        let previous = PathBuf::from(&operation.previous_live_path);
        fs::rename(&database_path, &previous).expect("move live");
        remove_database_sidecars(&database_path).expect("sidecars");

        let recovered = V2State::initialize(&data_dir);
        assert_eq!(
            recovered.status().state,
            crate::v2::V2DatabaseState::Healthy
        );
        let settings = recovered
            .with_connection(|connection| admin::load_settings(connection))
            .expect("settings");
        assert_eq!(settings.daily_study_target_minutes, 180);
        assert!(!restore_operation_path(&data_dir).exists());
        assert!(!candidate.exists());
        assert!(previous.is_file(), "previous live copy must be retained");
        assert!(safety.is_file(), "safety copy must be retained");
        drop(root);
    }

    #[test]
    fn interrupted_restore_after_install_rolls_back_a_damaged_live_database() {
        let (root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let database_path = state.database_path().expect("database path");
        set_target(&state, 180, 0);
        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        set_target(&state, 210, 2);
        checkpoint(&state);
        drop(state);

        let candidate = restore_candidate_path(&data_dir);
        fs::copy(&backup.path, &candidate).expect("candidate copy");
        let safety = safety_backup_directory(&data_dir).join("pre-restore-test.ptbackup");
        fs::create_dir_all(safety.parent().expect("safety parent")).expect("safety folder");
        fs::copy(&database_path, &safety).expect("safety copy");
        let operation = make_operation(&database_path, "candidate_installed", &candidate, &safety);
        write_json_atomic(&restore_operation_path(&data_dir), &operation).expect("journal");
        let previous = PathBuf::from(&operation.previous_live_path);
        fs::rename(&database_path, &previous).expect("move live");
        remove_database_sidecars(&database_path).expect("sidecars");
        fs::write(&database_path, b"damaged candidate").expect("damaged live");

        let recovered = V2State::initialize(&data_dir);
        assert_eq!(
            recovered.status().state,
            crate::v2::V2DatabaseState::Healthy
        );
        let settings = recovered
            .with_connection(|connection| admin::load_settings(connection))
            .expect("settings");
        assert_eq!(settings.daily_study_target_minutes, 210);
        assert!(!restore_operation_path(&data_dir).exists());
        assert!(safety.is_file());
        let preserved = fs::read_dir(&data_dir)
            .expect("data dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains(".failed-restore-")
            });
        assert!(preserved, "the damaged live database must be preserved");
        drop(root);
    }

    #[test]
    fn interrupted_restore_at_db_closed_keeps_valid_live_and_clears_journal() {
        let (root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let database_path = state.database_path().expect("database path");
        set_target(&state, 180, 0);
        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        set_target(&state, 210, 2);
        checkpoint(&state);
        drop(state);

        let candidate = restore_candidate_path(&data_dir);
        fs::copy(&backup.path, &candidate).expect("candidate copy");
        let safety = safety_backup_directory(&data_dir).join("pre-restore-test.ptbackup");
        fs::create_dir_all(safety.parent().expect("safety parent")).expect("safety folder");
        fs::copy(&database_path, &safety).expect("safety copy");
        let operation = make_operation(&database_path, "db_closed", &candidate, &safety);
        write_json_atomic(&restore_operation_path(&data_dir), &operation).expect("journal");

        let recovered = V2State::initialize(&data_dir);
        assert_eq!(
            recovered.status().state,
            crate::v2::V2DatabaseState::Healthy
        );
        let settings = recovered
            .with_connection(|connection| admin::load_settings(connection))
            .expect("settings");
        assert_eq!(settings.daily_study_target_minutes, 210);
        assert!(!restore_operation_path(&data_dir).exists());
        assert!(!candidate.exists());
        assert!(safety.is_file());
        drop(root);
    }

    #[test]
    fn interrupted_restore_uses_safety_when_live_and_previous_are_unavailable() {
        let (root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let database_path = state.database_path().expect("database path");
        set_target(&state, 180, 0);
        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        set_target(&state, 210, 2);
        checkpoint(&state);
        drop(state);

        let candidate = restore_candidate_path(&data_dir);
        fs::copy(&backup.path, &candidate).expect("candidate copy");
        let safety = safety_backup_directory(&data_dir).join("pre-restore-safety-only.ptbackup");
        fs::create_dir_all(safety.parent().expect("safety parent")).expect("safety folder");
        fs::copy(&database_path, &safety).expect("safety copy");
        let operation = make_operation(&database_path, "candidate_installed", &candidate, &safety);
        write_json_atomic(&restore_operation_path(&data_dir), &operation).expect("journal");
        let previous = PathBuf::from(&operation.previous_live_path);
        fs::rename(&database_path, &previous).expect("move live");
        remove_database_sidecars(&database_path).expect("sidecars");
        fs::write(&database_path, b"damaged live before safety recovery").expect("damaged live");
        fs::remove_file(&previous).expect("remove previous");

        let recovered = V2State::initialize(&data_dir);
        assert_eq!(
            recovered.status().state,
            crate::v2::V2DatabaseState::Healthy
        );
        let settings = recovered
            .with_connection(|connection| admin::load_settings(connection))
            .expect("settings");
        assert_eq!(settings.daily_study_target_minutes, 210);
        assert!(!restore_operation_path(&data_dir).exists());
        assert!(!candidate.exists());
        assert!(safety.is_file());
        drop(root);
    }

    #[test]
    fn recovery_is_safe_for_every_remaining_journal_stage() {
        for stage in [
            "validated",
            "safety_created",
            "db_closed",
            "verified",
            "cleanup_complete",
        ] {
            let (root, state) = state();
            let data_dir = state.data_directory().expect("data dir");
            let database_path = state.database_path().expect("database path");
            set_target(&state, 180, 0);
            let backup = state
                .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
                .expect("backup");
            set_target(&state, 210, 2);
            checkpoint(&state);
            drop(state);

            let candidate = restore_candidate_path(&data_dir);
            fs::copy(&backup.path, &candidate).expect("candidate copy");
            let safety = safety_backup_directory(&data_dir).join("pre-restore-stage.ptbackup");
            fs::create_dir_all(safety.parent().expect("safety parent")).expect("safety folder");
            fs::copy(&database_path, &safety).expect("safety copy");
            let operation = make_operation(&database_path, stage, &candidate, &safety);
            write_json_atomic(&restore_operation_path(&data_dir), &operation).expect("journal");

            if matches!(stage, "verified" | "cleanup_complete") {
                let previous = PathBuf::from(&operation.previous_live_path);
                fs::rename(&database_path, &previous).expect("move live");
                remove_database_sidecars(&database_path).expect("sidecars");
                fs::rename(&candidate, &database_path).expect("install candidate");
            }

            let recovered = V2State::initialize(&data_dir);
            assert_eq!(
                recovered.status().state,
                crate::v2::V2DatabaseState::Healthy,
                "stage {stage} must recover a valid database"
            );
            let expected_minutes = if matches!(stage, "verified" | "cleanup_complete") {
                180
            } else {
                210
            };
            let settings = recovered
                .with_connection(|connection| admin::load_settings(connection))
                .expect("settings");
            assert_eq!(
                settings.daily_study_target_minutes, expected_minutes,
                "stage {stage} selected the wrong authoritative database"
            );
            assert!(
                !restore_operation_path(&data_dir).exists(),
                "stage {stage} must clear the journal"
            );
            assert!(
                !candidate.exists(),
                "stage {stage} must clean the candidate"
            );
            assert!(safety.is_file(), "stage {stage} must retain safety");
            drop(root);
        }
    }

    #[test]
    fn damaged_journal_is_preserved_and_a_damaged_live_database_uses_candidate() {
        let (root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let database_path = state.database_path().expect("database path");
        set_target(&state, 180, 0);
        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(1)))
            .expect("backup");
        set_target(&state, 210, 2);
        checkpoint(&state);
        drop(state);

        let candidate = restore_candidate_path(&data_dir);
        fs::copy(&backup.path, &candidate).expect("candidate copy");
        fs::write(&database_path, b"damaged live database").expect("damage live");
        remove_database_sidecars(&database_path).expect("sidecars");
        fs::write(restore_operation_path(&data_dir), b"{not valid json").expect("journal");

        let recovered = V2State::initialize(&data_dir);
        assert_eq!(
            recovered.status().state,
            crate::v2::V2DatabaseState::Healthy
        );
        let settings = recovered
            .with_connection(|connection| admin::load_settings(connection))
            .expect("settings");
        assert_eq!(settings.daily_study_target_minutes, 180);
        assert!(!restore_operation_path(&data_dir).exists());
        let preserved_journal = fs::read_dir(&data_dir)
            .expect("data dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("restore-operation.invalid-")
            });
        assert!(preserved_journal, "the damaged journal must be preserved");
        drop(root);
    }

    fn legacy_backup(
        state: &V2State,
        data_dir: &Path,
        schema_version: i64,
        file_name: &str,
    ) -> PathBuf {
        state
            .with_connection(|connection| {
                if schema_version <= 2 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS flashcard_reviews;
                             DROP TABLE IF EXISTS flashcard_schedule;
                             DROP TABLE IF EXISTS flashcard_exam_links;
                             DROP TABLE IF EXISTS flashcards;",
                        )
                        .map_err(|error| AppError::database("drop flashcards", error))?;
                }
                if schema_version <= 3 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS custom_formula_variables;
                             DROP TABLE IF EXISTS custom_formulas;
                             DROP TABLE IF EXISTS custom_units;
                             DROP TABLE IF EXISTS custom_constants;",
                        )
                        .map_err(|error| AppError::database("drop custom references", error))?;
                }
                if schema_version <= 4 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS builtin_constants;
                             DROP TABLE IF EXISTS constant_catalog_versions;
                             DROP TABLE IF EXISTS sticky_notes;",
                        )
                        .map_err(|error| AppError::database("drop v5 tables", error))?;
                }
                if schema_version <= 1 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS quick_start_config_allocations;
                             DROP TABLE IF EXISTS quick_start_configs;",
                        )
                        .map_err(|error| AppError::database("drop quick start", error))?;
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(1,'v1_initial_schema_fixture',?1,?2)",
                            params![SCHEMA_1_CHECKSUM, at(0).to_rfc3339()],
                        )
                        .map_err(|error| AppError::database("legacy v1 ledger", error))?;
                }
                if schema_version == 2 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(2,'v2_quick_start_configs',?1,?2)",
                            params![SCHEMA_2_CHECKSUM, at(0).to_rfc3339()],
                        )
                        .map_err(|error| AppError::database("legacy v2 ledger", error))?;
                }
                if schema_version == 3 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(3,'v2_flashcards',?1,?2)",
                            params![SCHEMA_3_CHECKSUM, at(0).to_rfc3339()],
                        )
                        .map_err(|error| AppError::database("legacy v3 ledger", error))?;
                }
                if schema_version == 4 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(4,'v2_custom_references',?1,?2)",
                            params![SCHEMA_4_CHECKSUM, at(0).to_rfc3339()],
                        )
                        .map_err(|error| AppError::database("legacy v4 ledger", error))?;
                }
                connection
                    .execute(
                        "UPDATE app_metadata SET schema_version=?1 WHERE id=1",
                        [schema_version],
                    )
                    .map_err(|error| AppError::database("update legacy schema", error))?;
                connection
                    .execute_batch(&format!("PRAGMA user_version = {schema_version};"))
                    .map_err(|error| AppError::database("set legacy version", error))?;
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(|error| AppError::database("checkpoint legacy", error))?;
                Ok::<(), AppError>(())
            })
            .expect("legacy schema");
        let database_path = state.database_path().expect("source database");
        let backup_path = data_dir.join(file_name);
        fs::copy(&database_path, &backup_path).expect("copy legacy backup");
        write_manifest(&backup_path, schema_version, 4, at(10)).expect("legacy manifest");
        backup_path
    }

    #[test]
    fn schema_one_two_three_and_four_backups_restore_by_migrating_a_candidate_copy() {
        for schema_version in [1_i64, 2, 3, 4] {
            let (source_root, source_state) = state();
            let data_dir = source_state.data_directory().expect("source data dir");
            source_state
                .with_connection(|connection| {
                    admin::update_settings(
                        connection,
                        admin::V2UpdateAppSettingsInput {
                            preparation_start_date: Some("2026-10-01".to_owned()),
                            daily_study_target_minutes: 180,
                        },
                        at(0),
                    )
                })
                .expect("source settings");
            let backup_path = legacy_backup(
                &source_state,
                &data_dir,
                schema_version,
                &format!("legacy-schema-{schema_version}.ptbackup"),
            );
            let source_sha = sha256_file(&backup_path).expect("source sha");

            let preview = verify_backup_file(&backup_path).expect("legacy preview");
            assert_eq!(preview.schema_version, schema_version);
            assert_eq!(preview.target_schema_version, V2_SCHEMA_VERSION);
            assert!(preview.requires_upgrade);

            let target_root = tempfile::tempdir().expect("target directory");
            let target_dir = target_root.path().join("v2");
            let target_state = V2State::initialize(&target_dir);
            let target_database = target_state.database_path().expect("target database");
            drop(target_state);

            let result = apply_restore(
                &target_dir,
                &target_database,
                V2RestoreInput {
                    path: backup_path.display().to_string(),
                },
                at(20),
            )
            .expect("legacy restore");
            assert_eq!(result.schema_version, V2_SCHEMA_VERSION);
            assert_eq!(
                sha256_file(&backup_path).expect("source sha after"),
                source_sha
            );

            let restored = V2State::initialize(&target_dir);
            let settings = restored
                .with_connection(|connection| admin::load_settings(connection))
                .expect("restored settings");
            assert_eq!(settings.daily_study_target_minutes, 180);
            let (version, flashcard_table, custom_formula_table): (i64, i64, i64) = restored
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT (SELECT user_version FROM pragma_user_version),
                                    (SELECT COUNT(*) FROM sqlite_master
                                      WHERE type='table' AND name='flashcards'),
                                    (SELECT COUNT(*) FROM sqlite_master
                                      WHERE type='table' AND name='custom_formulas')",
                            [],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .map_err(|error| AppError::database("restored schema check", error))
                })
                .expect("restored schema check");
            assert_eq!(version, V2_SCHEMA_VERSION);
            assert_eq!(flashcard_table, 1);
            assert_eq!(custom_formula_table, 1);
            drop(restored);
            drop(source_state);
            drop(source_root);
            drop(target_root);
        }
    }

    fn install_safety_copy(
        target_dir: &Path,
        source_path: &Path,
        schema_version: i64,
        file_name: &str,
    ) -> PathBuf {
        let safety_dir = safety_backup_directory(target_dir);
        fs::create_dir_all(&safety_dir).expect("safety dir");
        let target = safety_dir.join(file_name);
        fs::copy(source_path, &target).expect("copy safety backup");
        write_manifest(&target, schema_version, 4, at(10)).expect("safety manifest");
        target
    }

    fn set_live_target(state: &V2State, minutes: i64) {
        state
            .with_connection(|connection| {
                admin::update_settings(
                    connection,
                    admin::V2UpdateAppSettingsInput {
                        preparation_start_date: None,
                        daily_study_target_minutes: minutes,
                    },
                    at(0),
                )
            })
            .expect("live settings");
    }

    #[test]
    fn safety_recovery_migrates_schema_one_two_three_and_four_on_candidate_copies() {
        for schema_version in [1_i64, 2, 3, 4] {
            let (source_root, source_state) = state();
            let source_dir = source_state.data_directory().expect("source data dir");
            set_live_target(&source_state, 180);
            let source_path = if schema_version == V2_SCHEMA_VERSION {
                PathBuf::from(
                    source_state
                        .with_connection(|connection| {
                            create_manual_backup(connection, &source_dir, at(1))
                        })
                        .expect("current backup")
                        .path,
                )
            } else {
                legacy_backup(
                    &source_state,
                    &source_dir,
                    schema_version,
                    &format!("legacy-safety-{schema_version}.ptbackup"),
                )
            };

            let target_root = tempfile::tempdir().expect("target directory");
            let target_dir = target_root.path().join("v2");
            let target_state = V2State::initialize(&target_dir);
            set_live_target(&target_state, 240);
            let safety_copy = install_safety_copy(
                &target_dir,
                &source_path,
                schema_version,
                &format!("pre-restore-schema-{schema_version}.ptbackup"),
            );
            let safety_sha = sha256_file(&safety_copy).expect("safety sha");

            let result = target_state
                .restore_recovery_safety(
                    V2RestoreInput {
                        path: safety_copy.display().to_string(),
                    },
                    at(20),
                )
                .expect("safety restore");
            assert_eq!(result.schema_version, V2_SCHEMA_VERSION);
            assert_eq!(
                sha256_file(&safety_copy).expect("safety sha after"),
                safety_sha
            );

            let (version, target, flashcard_table): (i64, i64, i64) = target_state
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT (SELECT user_version FROM pragma_user_version),
                                    (SELECT daily_study_target_minutes FROM settings WHERE id=1),
                                    (SELECT COUNT(*) FROM sqlite_master
                                      WHERE type='table' AND name='flashcards')",
                            [],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .map_err(|error| AppError::database("safety restore check", error))
                })
                .expect("safety restore check");
            assert_eq!(version, V2_SCHEMA_VERSION);
            assert_eq!(target, 180);
            assert_eq!(flashcard_table, 1);
            assert!(!restore_operation_path(&target_dir).exists());
            assert!(!restore_candidate_path(&target_dir).exists());

            drop(source_state);
            drop(source_root);
            drop(target_state);
            drop(target_root);
        }
    }

    #[test]
    fn safety_recovery_migration_failure_preserves_the_live_database() {
        let (source_root, source_state) = state();
        let source_dir = source_state.data_directory().expect("source data dir");
        let source_path = legacy_backup(&source_state, &source_dir, 2, "bad-migration.ptbackup");
        {
            let connection = Connection::open(&source_path).expect("open bad migration backup");
            connection
                .execute_batch("CREATE TABLE flashcards (id INTEGER PRIMARY KEY);")
                .expect("incompatible flashcard table");
            connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .expect("checkpoint bad migration backup");
        }
        write_manifest(&source_path, 2, 4, at(10)).expect("bad migration manifest");
        let source_sha = sha256_file(&source_path).expect("source sha");

        let target_root = tempfile::tempdir().expect("target directory");
        let target_dir = target_root.path().join("v2");
        let target_state = V2State::initialize(&target_dir);
        set_live_target(&target_state, 240);
        let safety_copy = install_safety_copy(&target_dir, &source_path, 2, "bad-safety.ptbackup");

        let error = target_state
            .restore_recovery_safety(
                V2RestoreInput {
                    path: safety_copy.display().to_string(),
                },
                at(20),
            )
            .expect_err("migration failure");
        assert!(error.code == "migration_error" || error.code == "restore_failed");
        assert_eq!(sha256_file(&safety_copy).expect("safety sha"), source_sha);
        let target: i64 = target_state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT daily_study_target_minutes FROM settings WHERE id=1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("live settings after failure", error))
            })
            .expect("live settings after failure");
        assert_eq!(target, 240);
        assert!(!restore_operation_path(&target_dir).exists());
        assert!(!restore_candidate_path(&target_dir).exists());

        drop(source_state);
        drop(source_root);
        drop(target_state);
        drop(target_root);
    }

    #[test]
    fn safety_recovery_verification_failure_rolls_back_to_the_live_database() {
        let (source_root, source_state) = state();
        let source_dir = source_state.data_directory().expect("source data dir");
        set_live_target(&source_state, 180);
        let source_path = PathBuf::from(
            source_state
                .with_connection(|connection| create_manual_backup(connection, &source_dir, at(1)))
                .expect("current backup")
                .path,
        );

        let target_root = tempfile::tempdir().expect("target directory");
        let target_dir = target_root.path().join("v2");
        let target_state = V2State::initialize(&target_dir);
        set_live_target(&target_state, 240);
        let safety_copy = install_safety_copy(
            &target_dir,
            &source_path,
            V2_SCHEMA_VERSION,
            "bad-verification.ptbackup",
        );
        {
            let connection = Connection::open(&safety_copy).expect("open bad verification backup");
            connection
                .execute_batch(
                    "PRAGMA ignore_check_constraints=ON;
                     DROP TRIGGER trg_exams_code_immutable;
                     UPDATE exams SET code='invalid-exam' WHERE id=4;
                     PRAGMA wal_checkpoint(TRUNCATE);",
                )
                .expect("invalid exam fixture");
        }
        write_manifest(&safety_copy, V2_SCHEMA_VERSION, 4, at(10))
            .expect("bad verification manifest");
        let safety_sha = sha256_file(&safety_copy).expect("safety sha");

        let error = target_state
            .restore_recovery_safety(
                V2RestoreInput {
                    path: safety_copy.display().to_string(),
                },
                at(20),
            )
            .expect_err("verification failure");
        assert!(error.code == "schema_integrity_error" || error.code == "restore_failed");
        assert_eq!(
            sha256_file(&safety_copy).expect("safety sha after"),
            safety_sha
        );
        let target: i64 = target_state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT daily_study_target_minutes FROM settings WHERE id=1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| {
                        AppError::database("live settings after verification failure", error)
                    })
            })
            .expect("live settings after verification failure");
        assert_eq!(target, 240);
        assert!(!restore_operation_path(&target_dir).exists());
        assert!(!restore_candidate_path(&target_dir).exists());
        let recovery = get_restore_recovery(&target_dir).expect("recovery after rollback");
        assert_eq!(
            recovery.files.len(),
            2,
            "the original safety copy and the pre-recovery safety snapshot should both remain"
        );

        drop(source_state);
        drop(source_root);
        drop(target_state);
        drop(target_root);
    }
}
