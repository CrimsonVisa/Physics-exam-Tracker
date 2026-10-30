use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::AppError;

pub const BACKUP_FORMAT_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_DESTINATION_CHARS: usize = 2_048;
pub const BACKUP_PREFIX: &str = "csir-net-tracker";
pub const MANIFEST_SUFFIX: &str = ".manifest.json";

pub const BACKUP_TABLES: [&str; 16] = [
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
pub struct BackupOverview {
    pub settings: BackupSettings,
    pub default_directory: String,
    pub schema_version: i64,
    pub application_version: String,
    pub database_id: String,
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
pub struct VerifiedBackup {
    pub manifest: BackupManifest,
    pub record_counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub manifest: BackupManifest,
    pub record_counts: BTreeMap<String, i64>,
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

    Ok(VerifiedBackup {
        manifest,
        record_counts,
    })
}

pub fn preview_restore(
    path: &Path,
    current_schema_version: i64,
    current_database_path: Option<&Path>,
) -> Result<RestorePreview, AppError> {
    let verified = verify_backup(path)?;
    if let Some(current_path) = current_database_path
        && current_path
            .canonicalize()
            .ok()
            .is_some_and(|current| current == path.canonicalize().unwrap_or_default())
    {
        return Err(AppError::restore_failed(
            "Select a backup file, not the live application database.",
        ));
    }
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

    Ok(RestorePreview {
        requires_upgrade: verified.manifest.schema_version < current_schema_version,
        manifest: verified.manifest,
        record_counts: verified.record_counts,
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
            let cleanup = cleanup_automatic_backups(&directory, settings.retention_count);
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

fn cleanup_automatic_backups(directory: &Path, keep: i64) -> CleanupResult {
    let keep = keep.max(1) as usize;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            return CleanupResult {
                removed: 0,
                warning: Some(
                    "The backup succeeded, but old automatic backups could not be inspected."
                        .to_owned(),
                ),
            };
        }
    };

    let mut backups = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            if !name.starts_with("csir-net-tracker-automatic-") || !name.ends_with(".sqlite3") {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect::<Vec<_>>();
    backups.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));

    let mut valid_paths = Vec::new();
    let mut invalid_paths = Vec::new();
    for (_, path) in &backups {
        if verify_backup(path).is_ok() {
            valid_paths.push(path.clone());
        } else {
            invalid_paths.push(path.clone());
        }
    }
    if valid_paths.is_empty() {
        return CleanupResult {
            removed: 0,
            warning: Some(
                "The automatic backup succeeded, but old backups could not be pruned because no valid automatic backup was found."
                    .to_owned(),
            ),
        };
    }

    let keep_paths = valid_paths
        .iter()
        .take(keep)
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let mut removed = 0_i64;
    for path in valid_paths.into_iter().skip(keep).chain(invalid_paths) {
        if keep_paths.contains(&path) {
            continue;
        }
        let manifest = manifest_path(&path);
        let backup_removed = fs::remove_file(&path).is_ok();
        if manifest.exists() {
            let _ = fs::remove_file(manifest);
        }
        if backup_removed {
            removed += 1;
        }
    }

    CleanupResult {
        removed,
        warning: None,
    }
}

pub fn table_counts(connection: &Connection) -> Result<BTreeMap<String, i64>, AppError> {
    let mut counts = BTreeMap::new();
    for table in BACKUP_TABLES {
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| AppError::database("A backup table count could not be read", error))?;
        counts.insert(table.to_owned(), count);
    }
    Ok(counts)
}

pub fn schema_version(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| AppError::database("The database schema version could not be read", error))
}

pub fn verify_database_file(path: &Path) -> Result<(), AppError> {
    verify_snapshot(path)
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
    use crate::{db, sessions, syllabus};

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
        assert_eq!(result.manifest.schema_version, 8);
        assert_eq!(result.manifest.application_version, "0.1.0");
        assert_eq!(result.manifest.integrity_check, "ok");
        assert_eq!(
            result.manifest.backup_size_bytes,
            fs::metadata(&backup_path).unwrap().len()
        );
        assert_eq!(result.manifest.sha256.len(), 64);

        let verified = verify_backup(&backup_path).expect("backup should verify");
        assert_eq!(verified.record_counts["study_sessions"], 1);
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
        let preview = preview_restore(&path, 8, None).expect("current schema should load");
        assert_eq!(preview.manifest.schema_version, 8);
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
        let cleanup = cleanup_automatic_backups(destination.path(), 2);
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
        let preview = preview_restore(&destination.path().join(&result.file_name), 8, None)
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
