use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use chrono::{NaiveDate, Utc};
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::{backup, error::AppError, settings, syllabus, timer};

pub const DATABASE_FILE_NAME: &str = "csir-net-tracker.sqlite3";
pub const LATEST_SCHEMA_VERSION: i64 = 12;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) const MIGRATIONS_SLICE: &[M<'static>] = &[
    M::up(include_str!("../migrations/0001_settings.sql")),
    M::up(include_str!("../migrations/0002_syllabus.sql")),
    M::up(include_str!(
        "../migrations/0003_study_timer_and_sessions.sql"
    )),
    M::up(include_str!("../migrations/0004_checklist_and_planner.sql")),
    M::up(include_str!("../migrations/0005_pyq_and_mock_tests.sql")),
    M::up(include_str!("../migrations/0006_revision_scheduler.sql")),
    M::up(include_str!("../migrations/0007_dashboard_layout.sql")),
    M::up(include_str!(
        "../migrations/0008_backup_and_portability.sql"
    )),
    M::up(include_str!("../migrations/0009_json_import_batches.sql")),
    M::up(include_str!("../migrations/0010_study_resources.sql")),
    M::up(include_str!("../migrations/0011_revision_lifecycle.sql")),
    M::up(include_str!("../migrations/0012_exam_profiles.sql")),
];
const MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATIONS_SLICE);

/// Tauri-managed application state. Database initialization failures are kept
/// as an error state so the window can still open and explain the problem.
pub struct AppState {
    database: Mutex<Result<Connection, AppError>>,
    data_dir: Option<PathBuf>,
    database_path: Option<PathBuf>,
}

impl AppState {
    pub fn initialize(data_dir: &Path, today: NaiveDate) -> Self {
        let path = database_path(data_dir);
        match Self::try_initialize(data_dir, today) {
            Ok(connection) => Self {
                database: Mutex::new(Ok(connection)),
                data_dir: Some(data_dir.to_path_buf()),
                database_path: Some(path),
            },
            Err(error) => {
                eprintln!("Local database initialization failed: {error:?}");
                Self {
                    database: Mutex::new(Err(AppError::database_unavailable(format!(
                        "The local database could not be initialized. {} Recovery files, if any, \
                         have been preserved. Restart the app after addressing the problem.",
                        error.message
                    )))),
                    data_dir: Some(data_dir.to_path_buf()),
                    database_path: Some(path),
                }
            }
        }
    }

    pub fn unavailable(error: AppError) -> Self {
        Self {
            database: Mutex::new(Err(error)),
            data_dir: None,
            database_path: None,
        }
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The local database is in an inconsistent state. Restart the app and try again.",
            )
        })?;
        let connection = guard.as_mut().map_err(|error| error.clone())?;
        operation(connection)
    }

    pub fn data_dir(&self) -> Result<&Path, AppError> {
        self.data_dir
            .as_deref()
            .ok_or_else(AppError::database_unavailable_message)
    }

    pub fn database_path(&self) -> Option<&Path> {
        self.database_path.as_deref()
    }

    pub fn restore_from_backup(
        &self,
        backup_path: &Path,
        today: NaiveDate,
    ) -> Result<backup::BackupResult, AppError> {
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The local database is in a consistent state lock. Restart the app and try again.",
            )
        })?;
        self.restore_from_backup_locked(backup_path, today, &mut guard)
    }

    pub fn restore_recovery_safety(
        &self,
        today: NaiveDate,
    ) -> Result<backup::BackupResult, AppError> {
        let data_dir = self.data_dir()?;
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The local database is in a consistent state lock. Restart the app and try again.",
            )
        })?;
        let safety_path = backup::restore_safety_path(data_dir).ok_or_else(|| {
            AppError::restore_failed(
                "The pre-restore safety snapshot is no longer available or could not be verified.",
            )
        })?;
        self.restore_from_backup_locked(&safety_path, today, &mut guard)
    }

    fn restore_from_backup_locked(
        &self,
        backup_path: &Path,
        today: NaiveDate,
        guard: &mut std::sync::MutexGuard<'_, Result<Connection, AppError>>,
    ) -> Result<backup::BackupResult, AppError> {
        let data_dir = self.data_dir()?;
        let current_path = self
            .database_path
            .clone()
            .ok_or_else(AppError::database_unavailable_message)?;
        let connection = guard.as_mut().map_err(|error| error.clone())?;
        let schema_version = backup::schema_version(connection)?;
        if schema_version > LATEST_SCHEMA_VERSION {
            return Err(AppError::restore_failed(format!(
                "This live database uses schema version {schema_version}, which is newer than this application supports. It was not replaced. Use the newer application version."
            )));
        }
        let selected = backup_path
            .canonicalize()
            .map_err(|_| AppError::restore_failed("The selected backup file is unavailable."))?;
        // Reject the live database before verification: verification of a
        // WAL-mode live database can create sidecars, and failure must never
        // trigger sidecar cleanup on an arbitrary user-selected path.
        if current_path
            .canonicalize()
            .ok()
            .is_some_and(|current| current == selected)
        {
            return Err(AppError::restore_failed(
                "Select a backup file, not the live application database.",
            ));
        }
        let verified = backup::verify_backup(&selected)?;
        if verified.manifest.schema_version > schema_version {
            return Err(AppError::restore_failed(format!(
                "This backup uses schema version {}, but this app supports up to schema {}. Update the app instead of downgrading the database.",
                verified.manifest.schema_version, schema_version
            )));
        }
        if verified.manifest.schema_version < 1 {
            return Err(AppError::restore_failed(
                "The backup schema version is not supported.",
            ));
        }
        let database_id = backup::database_identifier(connection)?;

        let safety = backup::create_safety_backup(connection, data_dir, "pre-restore", None)?;
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        let _ = backup::record_success(connection, &safety.manifest);

        let parent = current_path
            .parent()
            .ok_or_else(|| AppError::restore_failed("The live database location is invalid."))?;
        let token = uuid::Uuid::new_v4().simple().to_string();
        let temp_path = parent.join(format!(".csir-net-tracker-restore-{token}.sqlite3.tmp"));
        let rollback_path = parent.join(format!(
            ".csir-net-tracker-restore-{token}.sqlite3.rollback"
        ));
        let started_at = Utc::now().to_rfc3339();
        let operation = backup::RestoreOperation {
            token: token.clone(),
            stage: "safety_created".to_owned(),
            source_backup_path: selected.display().to_string(),
            source_schema_version: verified.manifest.schema_version,
            safety_backup_path: safety_path.display().to_string(),
            safety_schema_version: safety.manifest.schema_version,
            live_path: current_path.display().to_string(),
            rollback_path: rollback_path.display().to_string(),
            temp_path: temp_path.display().to_string(),
            started_at: started_at.clone(),
            updated_at: started_at,
            database_id,
        };
        backup::write_restore_operation(data_dir, &operation)?;
        if let Err(error) = fs::copy(&selected, &temp_path) {
            eprintln!("Restore candidate could not be copied: {error}");
            let _ = fs::remove_file(&temp_path);
            if !temp_path.exists() {
                let _ = backup::clear_restore_operation(data_dir);
            }
            return Err(AppError::restore_failed(
                "The selected backup could not be copied into the application-data folder.",
            ));
        }
        if let Err(error) = backup::verify_database_file(&temp_path) {
            let _ = fs::remove_file(&temp_path);
            let _ = remove_database_sidecars(&temp_path);
            if !temp_path.exists() {
                let _ = backup::clear_restore_operation(data_dir);
            }
            return Err(error);
        }
        if let Err(error) = remove_database_sidecars(&temp_path) {
            let _ = fs::remove_file(&temp_path);
            let _ = remove_database_sidecars(&temp_path);
            if !temp_path.exists() {
                let _ = backup::clear_restore_operation(data_dir);
            }
            return Err(AppError::file_operation(
                "The verified restore copy could not be prepared for installation. The original database was not changed.",
                error,
            ));
        }
        if let Err(error) = backup::update_restore_stage(data_dir, "candidate_verified") {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }

        let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        let previous = std::mem::replace(
            &mut **guard,
            Err(AppError::internal("The database is being restored.")),
        );
        drop(previous);

        let remove_sidecars_result = if restore_fault("remove_live_sidecars") {
            Err(std::io::Error::other("injected sidecar failure"))
        } else {
            remove_database_sidecars(&current_path)
        };
        if remove_sidecars_result.is_err() {
            let _ = fs::remove_file(&temp_path);
            let reopened = reopen_current_database(&current_path, today);
            let cleared = reopened.is_ok()
                && !temp_path.exists()
                && backup::clear_restore_operation(data_dir).is_ok();
            **guard = reopened;
            return Err(AppError::restore_failed(if cleared {
                "The database WAL files could not be prepared for restore. The original database was reopened."
            } else {
                "The database WAL files could not be prepared for restore. Recovery files were preserved; restart the app."
            }));
        }
        let move_live_result = if restore_fault("live_rename") {
            Err(std::io::Error::other("injected live rename failure"))
        } else {
            fs::rename(&current_path, &rollback_path)
        };
        if let Err(error) = move_live_result {
            eprintln!("Live database could not be moved for restore: {error}");
            let _ = fs::remove_file(&temp_path);
            let reopened = reopen_current_database(&current_path, today);
            let cleared = reopened.is_ok()
                && !temp_path.exists()
                && backup::clear_restore_operation(data_dir).is_ok();
            **guard = reopened;
            return Err(AppError::restore_failed(if cleared {
                "The live database could not be moved aside. The original database was reopened."
            } else {
                "The live database could not be moved aside. Recovery files were preserved; restart the app."
            }));
        }
        let live_renamed_stage = if restore_fault("stage_live_renamed") {
            Err(AppError::restore_failed(
                "injected live-renamed stage failure",
            ))
        } else {
            backup::update_restore_stage(data_dir, "live_renamed")
        };
        if let Err(error) = live_renamed_stage {
            let recovery = recover_failed_restore(
                data_dir,
                &current_path,
                &rollback_path,
                &safety_path,
                today,
            );
            match recovery {
                Ok(outcome) => {
                    **guard = Ok(outcome.connection);
                    if let Err(stage_error) = backup::update_restore_stage(data_dir, outcome.stage)
                    {
                        eprintln!(
                            "A recovered restore stage could not be recorded: {stage_error:?}"
                        );
                    }
                }
                Err(recovery_error) => **guard = Err(recovery_error),
            }
            return Err(AppError::restore_failed(format!(
                "The restore stage could not be recorded after moving the live database. Recovery files were preserved. {error:?}"
            )));
        }
        let install_result = if restore_fault("candidate_install_rename") {
            Err(std::io::Error::other("injected candidate install failure"))
        } else {
            fs::rename(&temp_path, &current_path)
        };
        if let Err(error) = install_result {
            eprintln!("Restore candidate could not be installed: {error}");
            match recover_failed_restore(
                data_dir,
                &current_path,
                &rollback_path,
                &safety_path,
                today,
            ) {
                Ok(outcome) => {
                    **guard = Ok(outcome.connection);
                    if let Err(stage_error) = backup::update_restore_stage(data_dir, outcome.stage)
                    {
                        eprintln!(
                            "A recovered restore stage could not be recorded: {stage_error:?}"
                        );
                    }
                    Err(AppError::restore_failed(
                        "The backup could not be installed. The original database was recovered; preserved candidates remain available.",
                    ))
                }
                Err(recovery_error) => {
                    **guard = Err(recovery_error);
                    Err(AppError::restore_failed(
                        "The backup could not be installed and automatic recovery failed. All recovery files were preserved. Restart the app for guided recovery.",
                    ))
                }
            }
        } else if let Err(error) = if restore_fault("stage_candidate_installed") {
            Err(AppError::restore_failed(
                "injected candidate-installed stage failure",
            ))
        } else {
            backup::update_restore_stage(data_dir, "candidate_installed")
        } {
            match recover_failed_restore(
                data_dir,
                &current_path,
                &rollback_path,
                &safety_path,
                today,
            ) {
                Ok(outcome) => {
                    **guard = Ok(outcome.connection);
                    if let Err(stage_error) = backup::update_restore_stage(data_dir, outcome.stage)
                    {
                        eprintln!(
                            "A recovered restore stage could not be recorded: {stage_error:?}"
                        );
                    }
                }
                Err(recovery_error) => **guard = Err(recovery_error),
            }
            Err(AppError::restore_failed(format!(
                "The restore stage could not be recorded after installing the candidate. Recovery files were preserved. {error:?}"
            )))
        } else {
            match open_database(&current_path, today) {
                Ok(restored) => {
                    **guard = Ok(restored);
                    let opened_stage = if restore_fault("stage_opened") {
                        Err(AppError::restore_failed("injected opened-stage failure"))
                    } else {
                        backup::update_restore_stage(data_dir, "opened")
                    };
                    if let Err(error) = opened_stage {
                        return Err(AppError::restore_failed(format!(
                            "The restored database opened, but its recovery stage could not be recorded. Recovery files were preserved. {error:?}"
                        )));
                    }
                    let mut warning = None;
                    match fs::remove_file(&rollback_path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => {
                            warning = Some(format!(
                                "The restored database is open, but the rollback file could not be removed ({error}). It remains protected for review."
                            ));
                        }
                    }
                    if warning.is_none()
                        && let Err(error) = if restore_fault("sidecar_clear") {
                            Err(AppError::restore_failed("injected sidecar cleanup failure"))
                        } else {
                            backup::clear_restore_operation(data_dir)
                        }
                    {
                        warning = Some(format!(
                            "The restored database is open, but the restore-operation metadata could not be cleared ({error:?})."
                        ));
                    }
                    let mut result = safety;
                    result.warning = warning;
                    Ok(result)
                }
                Err(error) => {
                    eprintln!("Restored database could not be opened: {error:?}");
                    match recover_failed_restore(
                        data_dir,
                        &current_path,
                        &rollback_path,
                        &safety_path,
                        today,
                    ) {
                        Ok(outcome) => {
                            **guard = Ok(outcome.connection);
                            if let Err(stage_error) =
                                backup::update_restore_stage(data_dir, outcome.stage)
                            {
                                eprintln!(
                                    "A recovered restore stage could not be recorded: {stage_error:?}"
                                );
                            }
                            Err(AppError::restore_failed(
                                "The backup could not be opened after replacement. The original database was recovered; preserved candidates remain available.",
                            ))
                        }
                        Err(recovery_error) => {
                            **guard = Err(recovery_error);
                            Err(AppError::restore_failed(
                                "The restored database could not be opened and recovery from the verified safety snapshot failed. All recovery files were preserved. Restart the app for guided recovery.",
                            ))
                        }
                    }
                }
            }
        }
    }

    fn try_initialize(data_dir: &Path, today: NaiveDate) -> Result<Connection, AppError> {
        fs::create_dir_all(data_dir).map_err(|error| {
            AppError::database("The application-data folder could not be created", error)
        })?;

        let path = database_path(data_dir);
        backup::recover_interrupted_restore_before_open(data_dir, &path, LATEST_SCHEMA_VERSION)?;
        open_database(&path, today)
    }
}

impl AppError {
    pub(crate) fn database_unavailable_message() -> Self {
        Self::database_unavailable(
            "The local database is unavailable. Restart the app and try again.",
        )
    }
}

fn database_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn remove_database_sidecars(path: &Path) -> std::io::Result<()> {
    for suffix in ["-wal", "-shm"] {
        let sidecar = database_sidecar_path(path, suffix);
        match fs::remove_file(sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn restore_fault(point: &str) -> bool {
    #[cfg(test)]
    {
        tests::fault_active(point)
    }
    #[cfg(not(test))]
    {
        let _ = point;
        false
    }
}

struct FailedRestoreRecovery {
    connection: Connection,
    stage: &'static str,
}

fn preserve_failed_live_database(data_dir: &Path, current_path: &Path) -> Result<(), AppError> {
    if !current_path.exists() {
        return Ok(());
    }
    let preserved = data_dir.join(format!(
        ".csir-net-tracker-restore-recovery-{}.sqlite3.failed",
        uuid::Uuid::new_v4().simple()
    ));
    backup::preserve_database_with_sidecars(current_path, &preserved).map_err(|error| {
        AppError::file_operation(
            "The failed live database could not be preserved before recovery. All recovery files were kept.",
            error.message,
        )
    })
}

fn recover_from_verified_safety(
    data_dir: &Path,
    current_path: &Path,
    safety_path: &Path,
    today: NaiveDate,
) -> Result<Connection, AppError> {
    if backup::live_database_state(current_path, LATEST_SCHEMA_VERSION)
        == backup::LiveDatabaseState::NewerSchema
    {
        return Err(AppError::restore_failed(
            "The live database uses a newer schema than this application supports. It was not replaced by an older safety snapshot. All recovery files were kept.",
        ));
    }
    if restore_fault("safety_restore_copy") {
        return Err(AppError::restore_failed(
            "The verified pre-restore safety snapshot could not be installed. All recovery files were preserved.",
        ));
    }
    let recovery_temp = data_dir.join(format!(
        ".csir-net-tracker-restore-recovery-{}.sqlite3.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    fs::copy(safety_path, &recovery_temp).map_err(|error| {
        AppError::file_operation(
            "The verified pre-restore safety snapshot could not be copied for recovery. All recovery files were kept.",
            error,
        )
    })?;
    if let Err(error) = backup::verify_database_file(&recovery_temp) {
        let _ = fs::remove_file(&recovery_temp);
        let _ = remove_database_sidecars(&recovery_temp);
        return Err(error);
    }
    remove_database_sidecars(&recovery_temp).map_err(|error| {
        AppError::file_operation(
            "The verified recovery copy could not be prepared for installation. All recovery files were kept.",
            error,
        )
    })?;
    preserve_failed_live_database(data_dir, current_path)?;
    remove_database_sidecars(current_path).map_err(|error| {
        AppError::file_operation(
            "The live WAL files could not be prepared for safety recovery. All recovery files were kept.",
            error,
        )
    })?;
    fs::rename(&recovery_temp, current_path).map_err(|error| {
        AppError::file_operation(
            "The verified recovery copy could not be installed. All recovery files were kept.",
            error,
        )
    })?;
    if !backup::verify_live_recovery_database(current_path, LATEST_SCHEMA_VERSION) {
        return Err(AppError::restore_failed(
            "The recovered database did not pass live-database verification. All recovery files were kept.",
        ));
    }
    open_database(current_path, today)
}

fn recover_failed_restore(
    data_dir: &Path,
    current_path: &Path,
    rollback_path: &Path,
    safety_path: &Path,
    today: NaiveDate,
) -> Result<FailedRestoreRecovery, AppError> {
    if backup::live_database_state(current_path, LATEST_SCHEMA_VERSION)
        == backup::LiveDatabaseState::NewerSchema
    {
        return Err(AppError::restore_failed(
            "The live database uses a newer schema than this application supports. It was not replaced. All recovery files were kept.",
        ));
    }
    if backup::restore_candidate_path_allowed(data_dir, rollback_path, ".rollback")
        && rollback_path.is_file()
        && !restore_fault("rollback_restore_rename")
    {
        preserve_failed_live_database(data_dir, current_path)?;
        if fs::rename(rollback_path, current_path).is_ok()
            && backup::verify_live_recovery_database(current_path, LATEST_SCHEMA_VERSION)
            && let Ok(connection) = open_database(current_path, today)
        {
            return Ok(FailedRestoreRecovery {
                connection,
                stage: "recovered_original",
            });
        }
        preserve_failed_live_database(data_dir, current_path)?;
    }

    let connection = recover_from_verified_safety(data_dir, current_path, safety_path, today)?;
    Ok(FailedRestoreRecovery {
        connection,
        stage: "recovered_safety",
    })
}

fn reopen_current_database(path: &Path, today: NaiveDate) -> Result<Connection, AppError> {
    if restore_fault("reopen_database")
        || !backup::verify_live_recovery_database(path, LATEST_SCHEMA_VERSION)
    {
        return Err(AppError::database_unavailable(
            "The original database is missing or could not be reopened. Restart the app; recovery files were preserved.",
        ));
    }
    open_database(path, today).map_err(|error| {
        eprintln!("The original database could not be reopened: {error:?}");
        AppError::database_unavailable(
            "The original database could not be reopened. Restart the app; if it still fails, use the verified safety backup.",
        )
    })
}

#[cfg(test)]
fn recover_from_safety_snapshot(
    safety_path: &Path,
    current_path: &Path,
    today: NaiveDate,
) -> Result<Connection, AppError> {
    let _ = fs::remove_file(current_path);
    let _ = remove_database_sidecars(current_path);
    fs::copy(safety_path, current_path).map_err(|_| {
        AppError::database_unavailable(
            "Restore failed and the verified safety snapshot could not be copied back. Restart the app and restore from the safety backup.",
        )
    })?;
    backup::verify_database_file(current_path).map_err(|_| {
        AppError::database_unavailable(
            "Restore failed and the recovered safety snapshot did not pass verification. Restart the app and restore from the safety backup.",
        )
    })?;
    open_database(current_path, today)
}

pub fn database_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DATABASE_FILE_NAME)
}

/// Open, configure, migrate, and seed the database. Calling this repeatedly on
/// the same file is deterministic and never overwrites saved settings.
pub fn open_database(path: &Path, today: NaiveDate) -> Result<Connection, AppError> {
    let mut connection = Connection::open(path)
        .map_err(|error| AppError::database("The local database could not be opened", error))?;

    configure(&connection)?;
    let existing_version = backup::schema_version(&connection)?;
    if existing_version > LATEST_SCHEMA_VERSION {
        return Err(AppError::migration(
            &format!(
                "This database uses schema version {existing_version}, which is newer than this application supports. It was not modified. Use the newer application version."
            ),
            "newer database schema",
        ));
    }
    let fresh_install = existing_version == 0;
    let current_database_id = backup::database_identifier(&connection)?;
    let mut pre_migration_database_id = None;
    let mut migrated_from_older_schema = false;

    if existing_version > 0 && existing_version < LATEST_SCHEMA_VERSION {
        let database_id = current_database_id.unwrap_or_else(backup::new_database_id);
        let backup_directory = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        backup::create_backup(
            &connection,
            &backup_directory,
            "pre-migration",
            Some(&database_id),
        )
        .map_err(|error| {
            eprintln!("Pre-migration safety backup failed: {error:?}");
            AppError::backup_failed(
                "The database needs an upgrade, but the required pre-migration safety backup failed. The database was not upgraded. Check free space and permissions, then restart the app.",
            )
        })?;
        backup::record_migration_pending(path.parent().unwrap_or_else(|| Path::new(".")))
            .map_err(|error| {
                eprintln!("Migration pending state could not be recorded: {error:?}");
                AppError::backup_failed(
                    "The pre-migration snapshot was created, but the migration-pending state could not be recorded durably. The database was not upgraded. Check free space and permissions, then restart the app.",
                )
            })?;
        pre_migration_database_id = Some(database_id);
        migrated_from_older_schema = true;
    }

    migrate(&mut connection)?;
    backup::ensure_database_identifier(&connection, pre_migration_database_id.as_deref())?;
    backup::clear_pre_migration_last_success(&connection)?;
    settings::ensure_defaults(&mut connection, today)?;
    syllabus::seed(&mut connection)?;
    timer::ensure_defaults(&mut connection)?;
    timer::recover_after_restart(&mut connection, Utc::now())?;

    let data_dir = path.parent().unwrap_or_else(|| Path::new("."));
    if migrated_from_older_schema
        || (!fresh_install && backup::post_migration_snapshot_required(data_dir))
    {
        // Enabling the policy first leaves no state in which a verified
        // post-migration snapshot exists but daily backups were never
        // enabled; a failure aborts initialization and the repair path runs
        // again on the next start.
        backup::enable_daily_automatic_backups(&connection, backup::DAILY_AUTOMATIC_RETENTION)?;
        let post_migration = backup::create_post_migration_backup(&connection, data_dir)
            .map_err(|error| {
                eprintln!("Post-migration recovery backup failed: {error:?}");
                AppError::backup_failed(
                    "The database was migrated, but the required post-migration recovery snapshot could not be created or verified. The pre-migration backup has been preserved. Check free space and permissions, then restart the app.",
                )
            })?;
        backup::record_success(&connection, &post_migration.manifest)?;
        backup::record_upgraded_install(data_dir, &post_migration.manifest).map_err(|error| {
            eprintln!("Migration state could not be recorded: {error:?}");
            AppError::backup_failed(
                "The post-migration recovery snapshot was created, but the migration state could not be recorded durably. Restart the app to retry safely.",
            )
        })?;
    } else if fresh_install {
        backup::record_fresh_install(data_dir).map_err(|error| {
            eprintln!("Fresh-install state could not be recorded: {error:?}");
            AppError::backup_failed(
                "The application database was created, but its fresh-install state could not be recorded. Check free space and permissions, then restart the app.",
            )
        })?;
    } else if backup::is_migration_pending(data_dir)
        || backup::read_migration_state(data_dir).is_none()
    {
        // Adopt an already-valid post-migration snapshot from an installation
        // created before the durable marker existed or from an interrupted
        // migration, without creating a duplicate.
        if let Some(path) = backup::latest_post_migration_backup(data_dir) {
            if backup::is_migration_pending(data_dir) {
                backup::enable_daily_automatic_backups(
                    &connection,
                    backup::DAILY_AUTOMATIC_RETENTION,
                )?;
            }
            let verified = backup::verify_backup(&path)?;
            backup::record_success(&connection, &verified.manifest)?;
            backup::record_upgraded_install(data_dir, &verified.manifest)?;
        }
    }

    Ok(connection)
}

fn configure(connection: &Connection) -> Result<(), AppError> {
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| AppError::database("SQLite foreign keys could not be enabled", error))?;

    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| AppError::database("The SQLite busy timeout could not be set", error))?;

    let _journal_mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|error| AppError::database("SQLite WAL mode could not be enabled", error))?;

    connection
        .pragma_update(None, "synchronous", "NORMAL")
        .map_err(|error| AppError::database("SQLite synchronous mode could not be set", error))?;

    Ok(())
}

fn migrate(connection: &mut Connection) -> Result<(), AppError> {
    MIGRATIONS.to_latest(connection).map_err(|error| {
        AppError::migration("The local database schema could not be prepared", error)
    })
}

#[cfg(test)]
pub(crate) fn test_connection() -> Connection {
    in_memory_connection().expect("in-memory database should open")
}

pub(crate) fn in_memory_connection() -> Result<Connection, AppError> {
    let mut connection = Connection::open_in_memory()
        .map_err(|error| AppError::database("An in-memory database could not be opened", error))?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| {
            AppError::database("In-memory foreign keys could not be enabled", error)
        })?;
    migrate(&mut connection)?;
    Ok(connection)
}

#[cfg(test)]
pub(crate) fn test_connection_with_defaults(today: NaiveDate) -> Connection {
    let mut connection = test_connection();
    settings::ensure_defaults(&mut connection, today).expect("test defaults should be created");
    connection
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        backup, mock_tests, planner, pyq, sessions,
        settings::{Settings, SettingsInput},
    };
    use std::cell::RefCell;
    use std::sync::{Arc, mpsc};

    thread_local! {
        static RESTORE_FAULT: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    pub(super) fn fault_active(point: &str) -> bool {
        RESTORE_FAULT.with(|fault| {
            fault
                .borrow()
                .as_deref()
                .is_some_and(|value| value.split(',').any(|candidate| candidate == point))
        })
    }

    fn inject_fault(point: &str) {
        RESTORE_FAULT.with(|fault| *fault.borrow_mut() = Some(point.to_owned()));
    }

    fn clear_fault() {
        RESTORE_FAULT.with(|fault| *fault.borrow_mut() = None);
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn schema_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version should be readable")
    }

    fn state_with_session(data_dir: &Path, today: NaiveDate, note: &str) -> AppState {
        let state = AppState::initialize(data_dir, today);
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                         ) VALUES (
                            NULL, '2026-11-02', 1200, 'manual', ?1,
                            '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                         )",
                        [note],
                    )
                    .map_err(|error| AppError::database("session should save", error))?;
                Ok(())
            })
            .expect("session should save");
        state
    }

    fn test_restore_operation(
        live_path: &Path,
        safety_path: &Path,
        stage: &str,
        rollback_path: &Path,
        temp_path: &Path,
    ) -> backup::RestoreOperation {
        backup::RestoreOperation {
            token: "test".to_owned(),
            stage: stage.to_owned(),
            source_backup_path: "selected.sqlite3".to_owned(),
            source_schema_version: 12,
            safety_backup_path: safety_path.display().to_string(),
            safety_schema_version: 12,
            live_path: live_path.display().to_string(),
            rollback_path: rollback_path.display().to_string(),
            temp_path: temp_path.display().to_string(),
            started_at: "2026-11-02T00:00:00Z".to_owned(),
            updated_at: "2026-11-02T00:00:00Z".to_owned(),
            database_id: None,
        }
    }

    #[test]
    fn migration_definitions_are_valid() {
        MIGRATIONS.validate().expect("migrations should be valid");
        assert_eq!(LATEST_SCHEMA_VERSION, MIGRATIONS_SLICE.len() as i64);
    }

    #[test]
    fn migrations_are_versioned_and_repeatable() {
        let mut connection = test_connection();

        assert_eq!(schema_version(&connection), 12);
        let table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table'
                   AND name IN (
                       'exams',
                       'exam_profiles',
                       'settings',
                       'syllabus_sources',
                       'syllabus_sections',
                       'syllabus_topics',
                       'syllabus_progress',
                       'timer_settings',
                       'timer_state',
                       'study_sessions',
                       'checklist_tasks',
                       'planned_study_blocks',
                       'pyq_attempts',
                       'mock_tests',
                       'revision_settings',
                       'revision_schedules',
                       'revision_history',
                       'dashboard_sections',
                       'database_metadata',
                       'backup_settings',
                       'json_import_batches',
                       'study_resources',
                       'resource_progress',
                       'resource_local_files',
                       'resource_discovery_runs'
                   )",
                [],
                |row| row.get(0),
            )
            .expect("migration tables should be inspectable");
        assert_eq!(table_count, 25);

        migrate(&mut connection).expect("a repeated migration should be a no-op");
        assert_eq!(schema_version(&connection), 12);
    }

    #[test]
    fn version_one_database_upgrades_without_losing_settings_or_creating_progress_or_sessions() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);

        {
            let mut connection = Connection::open(&path).expect("version one database should open");
            configure(&connection).expect("version one database should be configured");
            Migrations::from_slice(&MIGRATIONS_SLICE[..1])
                .to_latest(&mut connection)
                .expect("version one schema should apply");
            assert_eq!(schema_version(&connection), 1);

            settings::ensure_defaults(&mut connection, date(2026, 10, 29))
                .expect("version one defaults should be created");
            settings::update(
                &mut connection,
                SettingsInput {
                    exam_date: "2027-06-15".to_owned(),
                    preparation_start_date: "2026-11-01".to_owned(),
                    daily_study_target_minutes: 300,
                },
                date(2026, 10, 29),
            )
            .expect("version one settings should be saved");
        }

        let connection =
            open_database(&path, date(2026, 10, 29)).expect("database should upgrade and open");

        assert_eq!(schema_version(&connection), 12);
        assert_eq!(
            settings::load(&connection).expect("settings should survive the upgrade"),
            Settings {
                exam_date: "2027-06-15".to_owned(),
                preparation_start_date: "2026-11-01".to_owned(),
                daily_study_target_minutes: 300,
            }
        );

        let topic_count: i64 = connection
            .query_row("SELECT count(*) FROM syllabus_topics", [], |row| row.get(0))
            .expect("seeded topics should be countable");
        let progress_count: i64 = connection
            .query_row("SELECT count(*) FROM syllabus_progress", [], |row| {
                row.get(0)
            })
            .expect("progress rows should be countable");
        assert_eq!(topic_count, 182);
        assert_eq!(progress_count, 0);

        let session_count: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .expect("session rows should be countable");
        let timer_state: (String, String, i64) = connection
            .query_row(
                "SELECT status, mode, accumulated_ms FROM timer_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("timer state should exist after the upgrade");
        let task_count: i64 = connection
            .query_row("SELECT count(*) FROM checklist_tasks", [], |row| row.get(0))
            .expect("task rows should be countable");
        let block_count: i64 = connection
            .query_row("SELECT count(*) FROM planned_study_blocks", [], |row| {
                row.get(0)
            })
            .expect("planned blocks should be countable");
        let pyq_count: i64 = connection
            .query_row("SELECT count(*) FROM pyq_attempts", [], |row| row.get(0))
            .expect("PYQ rows should be countable");
        let mock_count: i64 = connection
            .query_row("SELECT count(*) FROM mock_tests", [], |row| row.get(0))
            .expect("mock rows should be countable");
        assert_eq!(session_count, 0);
        assert_eq!(task_count, 0);
        assert_eq!(block_count, 0);
        assert_eq!(pyq_count, 0);
        assert_eq!(mock_count, 0);
        assert_eq!(timer_state, ("idle".to_owned(), "focus".to_owned(), 0));
    }

    #[test]
    fn version_three_database_upgrades_without_losing_progress_timer_or_sessions() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);

        {
            let mut connection =
                Connection::open(&path).expect("version three database should open");
            configure(&connection).expect("version three database should be configured");
            Migrations::from_slice(&MIGRATIONS_SLICE[..3])
                .to_latest(&mut connection)
                .expect("version three schema should apply");
            assert_eq!(schema_version(&connection), 3);

            settings::ensure_defaults(&mut connection, date(2026, 10, 29))
                .expect("version three settings should be created");
            syllabus::seed(&mut connection).expect("version three syllabus should be seeded");
            timer::ensure_defaults(&mut connection).expect("version three timer should exist");

            settings::update(
                &mut connection,
                SettingsInput {
                    exam_date: "2027-02-20".to_owned(),
                    preparation_start_date: "2026-11-15".to_owned(),
                    daily_study_target_minutes: 210,
                },
                date(2026, 10, 29),
            )
            .expect("version three settings should update");

            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'GA-01'",
                    [],
                    |row| row.get(0),
                )
                .expect("seeded topic should exist");
            syllabus::update_progress(
                &mut connection,
                topic_id,
                syllabus::TopicProgressInput {
                    status: "Learning".to_owned(),
                    priority: "High".to_owned(),
                    confidence: 4,
                    notes: "Keep this progress".to_owned(),
                },
            )
            .expect("version three progress should save");

            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-29".to_owned(),
                    duration_seconds: 2_700,
                    notes: "Keep this session".to_owned(),
                },
            )
            .expect("version three session should save");

            connection
                .execute(
                    "UPDATE timer_state
                     SET status = 'paused',
                         duration_ms = 1500000,
                         accumulated_ms = 12345,
                         first_started_at = '2026-10-29T10:00:00.000Z',
                         segment_started_at = NULL,
                         last_heartbeat_at = NULL,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = 1",
                    [],
                )
                .expect("version three timer state should update");
        }

        let connection =
            open_database(&path, date(2026, 10, 29)).expect("database should upgrade and open");

        assert_eq!(schema_version(&connection), 12);
        assert_eq!(
            settings::load(&connection).expect("settings should survive the upgrade"),
            Settings {
                exam_date: "2027-02-20".to_owned(),
                preparation_start_date: "2026-11-15".to_owned(),
                daily_study_target_minutes: 210,
            }
        );

        let topic_id: i64 = connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = 'GA-01'",
                [],
                |row| row.get(0),
            )
            .expect("seeded topic should still exist");
        let progress = syllabus::load_progress(&connection, topic_id)
            .expect("the original topic progress should survive");
        assert_eq!(progress.status, "Learning");
        assert_eq!(progress.priority, "High");
        assert_eq!(progress.confidence, 4);
        assert_eq!(progress.notes, "Keep this progress");

        let saved_sessions = sessions::list(&connection, sessions::StudySessionFilter::default())
            .expect("sessions should survive the upgrade");
        assert_eq!(saved_sessions.len(), 1);
        assert_eq!(saved_sessions[0].session_date, "2026-10-29");
        assert_eq!(saved_sessions[0].duration_seconds, 2_700);
        assert_eq!(saved_sessions[0].notes, "Keep this session");

        let timer_state: (String, i64) = connection
            .query_row(
                "SELECT status, accumulated_ms FROM timer_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("timer state should survive the upgrade");
        assert_eq!(timer_state, ("paused".to_owned(), 12_345));

        let task_count: i64 = connection
            .query_row("SELECT count(*) FROM checklist_tasks", [], |row| row.get(0))
            .expect("task rows should be countable");
        let block_count: i64 = connection
            .query_row("SELECT count(*) FROM planned_study_blocks", [], |row| {
                row.get(0)
            })
            .expect("planned blocks should be countable");
        assert_eq!(task_count, 0);
        assert_eq!(block_count, 0);
    }

    #[test]
    fn version_four_database_upgrades_without_losing_phase_five_data() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);

        {
            let mut connection =
                Connection::open(&path).expect("version four database should open");
            configure(&connection).expect("version four database should be configured");
            Migrations::from_slice(&MIGRATIONS_SLICE[..4])
                .to_latest(&mut connection)
                .expect("version four schema should apply");
            assert_eq!(schema_version(&connection), 4);

            settings::ensure_defaults(&mut connection, date(2026, 10, 30))
                .expect("version four settings should be created");
            syllabus::seed(&mut connection).expect("version four syllabus should be seeded");
            timer::ensure_defaults(&mut connection).expect("version four timer should exist");

            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                    [],
                    |row| row.get(0),
                )
                .expect("seeded topic should exist");
            syllabus::update_progress(
                &mut connection,
                topic_id,
                syllabus::TopicProgressInput {
                    status: "Revised".to_owned(),
                    priority: "High-Yield".to_owned(),
                    confidence: 5,
                    notes: "Phase 3 progress".to_owned(),
                },
            )
            .expect("progress should save");
            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-30".to_owned(),
                    duration_seconds: 1_800,
                    notes: "Phase 4 session".to_owned(),
                },
            )
            .expect("session should save");
            planner::create_task(
                &mut connection,
                planner::TaskInput {
                    title: "Phase 5 task".to_owned(),
                    description: String::new(),
                    scheduled_date: "2026-10-30".to_owned(),
                    deadline_date: None,
                    topic_id: Some(topic_id),
                    priority: "High".to_owned(),
                    schedule_anchor: "manual".to_owned(),
                    schedule_offset_days: None,
                },
                date(2026, 10, 30),
            )
            .expect("checklist task should save");
            planner::create_block(
                &mut connection,
                planner::PlannedBlockInput {
                    title: "Phase 5 block".to_owned(),
                    topic_id: Some(topic_id),
                    calendar_date: "2026-10-30".to_owned(),
                    start_time: "10:00".to_owned(),
                    end_time: "11:30".to_owned(),
                    status: "planned".to_owned(),
                    notes: String::new(),
                    linked_session_id: None,
                },
            )
            .expect("planned block should save");
        }

        let connection =
            open_database(&path, date(2026, 10, 30)).expect("database should upgrade and open");

        assert_eq!(schema_version(&connection), 12);
        assert_eq!(
            settings::load(&connection)
                .expect("settings should load")
                .exam_date,
            "2026-12-19"
        );
        assert_eq!(
            syllabus::summary(&connection)
                .expect("syllabus should load")
                .total_topics,
            182
        );
        let progress = connection
            .query_row(
                "SELECT status, priority, notes FROM syllabus_progress LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("progress should survive");
        assert_eq!(
            progress,
            (
                "Revised".to_owned(),
                "High-Yield".to_owned(),
                "Phase 3 progress".to_owned()
            )
        );

        let session = connection
            .query_row(
                "SELECT session_date, duration_seconds, notes FROM study_sessions LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("session should survive");
        assert_eq!(
            session,
            ("2026-10-30".to_owned(), 1_800, "Phase 4 session".to_owned())
        );

        let task = connection
            .query_row(
                "SELECT title, status FROM checklist_tasks LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("task should survive");
        assert_eq!(task, ("Phase 5 task".to_owned(), "open".to_owned()));

        let block = connection
            .query_row(
                "SELECT title, status FROM planned_study_blocks LIMIT 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .expect("block should survive");
        assert_eq!(block, ("Phase 5 block".to_owned(), "planned".to_owned()));

        let pyq_count: i64 = connection
            .query_row("SELECT count(*) FROM pyq_attempts", [], |row| row.get(0))
            .expect("PYQ rows should be countable");
        let mock_count: i64 = connection
            .query_row("SELECT count(*) FROM mock_tests", [], |row| row.get(0))
            .expect("mock rows should be countable");
        assert_eq!(pyq_count, 0);
        assert_eq!(mock_count, 0);
    }

    #[test]
    fn version_five_database_upgrades_without_losing_phase_six_data() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);

        {
            let mut connection =
                Connection::open(&path).expect("version five database should open");
            configure(&connection).expect("version five database should be configured");
            Migrations::from_slice(&MIGRATIONS_SLICE[..5])
                .to_latest(&mut connection)
                .expect("version five schema should apply");
            assert_eq!(schema_version(&connection), 5);

            settings::ensure_defaults(&mut connection, date(2026, 10, 30))
                .expect("version five settings should be created");
            syllabus::seed(&mut connection).expect("version five syllabus should be seeded");
            timer::ensure_defaults(&mut connection).expect("version five timer should exist");

            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                    [],
                    |row| row.get(0),
                )
                .expect("seeded topic should exist");
            syllabus::update_progress(
                &mut connection,
                topic_id,
                syllabus::TopicProgressInput {
                    status: "Confident".to_owned(),
                    priority: "High".to_owned(),
                    confidence: 5,
                    notes: "Phase 3 progress".to_owned(),
                },
            )
            .expect("progress should save");
            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-29".to_owned(),
                    duration_seconds: 2_400,
                    notes: "Phase 4 session".to_owned(),
                },
            )
            .expect("session should save");
            planner::create_task(
                &mut connection,
                planner::TaskInput {
                    title: "Phase 5 task".to_owned(),
                    description: String::new(),
                    scheduled_date: "2026-10-30".to_owned(),
                    deadline_date: None,
                    topic_id: Some(topic_id),
                    priority: "High".to_owned(),
                    schedule_anchor: "manual".to_owned(),
                    schedule_offset_days: None,
                },
                date(2026, 10, 30),
            )
            .expect("checklist task should save");
            planner::create_block(
                &mut connection,
                planner::PlannedBlockInput {
                    title: "Phase 5 block".to_owned(),
                    topic_id: Some(topic_id),
                    calendar_date: "2026-10-30".to_owned(),
                    start_time: "10:00".to_owned(),
                    end_time: "11:00".to_owned(),
                    status: "planned".to_owned(),
                    notes: String::new(),
                    linked_session_id: None,
                },
            )
            .expect("planned block should save");
            pyq::create(
                &mut connection,
                pyq::PyqInput {
                    question_year: 2024,
                    exam_session: "June 2024".to_owned(),
                    part: "B".to_owned(),
                    topic_id,
                    question_reference: "Phase 6 PYQ".to_owned(),
                    attempt_date: "2026-10-28".to_owned(),
                    status: "correct".to_owned(),
                    marks_obtained: Some(2.0),
                    max_marks: Some(2.0),
                    time_spent_seconds: 90,
                    notes: "Keep this PYQ".to_owned(),
                    difficulty: Some(3),
                },
            )
            .expect("PYQ should save");
            mock_tests::create(
                &mut connection,
                mock_tests::MockTestInput {
                    test_name: "Phase 6 mock".to_owned(),
                    test_date: "2026-10-27".to_owned(),
                    duration_minutes: 180,
                    max_marks: 200.0,
                    marks_obtained: Some(100.0),
                    part_a: mock_tests::MockPartInput {
                        questions: 1,
                        correct: 1,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: Some(2.0),
                        incorrect_marks: Some(0.0),
                        unanswered_marks: Some(0.0),
                    },
                    part_b: mock_tests::MockPartInput {
                        questions: 0,
                        correct: 0,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: None,
                        incorrect_marks: None,
                        unanswered_marks: None,
                    },
                    part_c: mock_tests::MockPartInput {
                        questions: 0,
                        correct: 0,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: None,
                        incorrect_marks: None,
                        unanswered_marks: None,
                    },
                    notes: "Keep this mock".to_owned(),
                    review_observations: String::new(),
                },
            )
            .expect("mock test should save");
        }

        let connection =
            open_database(&path, date(2026, 10, 30)).expect("database should upgrade and open");

        assert_eq!(schema_version(&connection), 12);
        let progress: (String, String) = connection
            .query_row(
                "SELECT status, notes FROM syllabus_progress LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("progress should survive");
        assert_eq!(
            progress,
            ("Confident".to_owned(), "Phase 3 progress".to_owned())
        );

        let session_count: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .expect("sessions should be countable");
        let task_count: i64 = connection
            .query_row("SELECT count(*) FROM checklist_tasks", [], |row| row.get(0))
            .expect("tasks should be countable");
        let block_count: i64 = connection
            .query_row("SELECT count(*) FROM planned_study_blocks", [], |row| {
                row.get(0)
            })
            .expect("blocks should be countable");
        let pyq_count: i64 = connection
            .query_row("SELECT count(*) FROM pyq_attempts", [], |row| row.get(0))
            .expect("PYQs should be countable");
        let mock_count: i64 = connection
            .query_row("SELECT count(*) FROM mock_tests", [], |row| row.get(0))
            .expect("mocks should be countable");
        assert_eq!(session_count, 1);
        assert_eq!(task_count, 1);
        assert_eq!(block_count, 1);
        assert_eq!(pyq_count, 1);
        assert_eq!(mock_count, 1);

        let revision_settings_count: i64 = connection
            .query_row("SELECT count(*) FROM revision_settings", [], |row| {
                row.get(0)
            })
            .expect("revision settings should exist");
        let revision_schedule_count: i64 = connection
            .query_row("SELECT count(*) FROM revision_schedules", [], |row| {
                row.get(0)
            })
            .expect("revision schedules should be empty");
        let revision_history_count: i64 = connection
            .query_row("SELECT count(*) FROM revision_history", [], |row| {
                row.get(0)
            })
            .expect("revision history should be empty");
        assert_eq!(revision_settings_count, 1);
        assert_eq!(revision_schedule_count, 0);
        assert_eq!(revision_history_count, 0);
    }

    #[test]
    fn version_six_database_upgrades_without_losing_phase_seven_data() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);

        {
            let mut connection = Connection::open(&path).expect("version six database should open");
            configure(&connection).expect("version six database should be configured");
            Migrations::from_slice(&MIGRATIONS_SLICE[..6])
                .to_latest(&mut connection)
                .expect("version six schema should apply");
            assert_eq!(schema_version(&connection), 6);

            settings::ensure_defaults(&mut connection, today)
                .expect("version six settings should be created");
            syllabus::seed(&mut connection).expect("version six syllabus should seed");
            timer::ensure_defaults(&mut connection).expect("version six timer should exist");

            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                    [],
                    |row| row.get(0),
                )
                .expect("topic should exist");
            syllabus::update_progress(
                &mut connection,
                topic_id,
                syllabus::TopicProgressInput {
                    status: "Revised".to_owned(),
                    priority: "High-Yield".to_owned(),
                    confidence: 4,
                    notes: "Phase 3 progress".to_owned(),
                },
            )
            .expect("progress should save");
            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-29".to_owned(),
                    duration_seconds: 2_400,
                    notes: "Phase 4 session".to_owned(),
                },
            )
            .expect("session should save");
            planner::create_task(
                &mut connection,
                planner::TaskInput {
                    title: "Phase 5 task".to_owned(),
                    description: String::new(),
                    scheduled_date: "2026-10-30".to_owned(),
                    deadline_date: Some("2026-11-02".to_owned()),
                    topic_id: Some(topic_id),
                    priority: "High".to_owned(),
                    schedule_anchor: "manual".to_owned(),
                    schedule_offset_days: None,
                },
                today,
            )
            .expect("task should save");
            planner::create_block(
                &mut connection,
                planner::PlannedBlockInput {
                    title: "Phase 5 block".to_owned(),
                    topic_id: Some(topic_id),
                    calendar_date: "2026-10-30".to_owned(),
                    start_time: "10:00".to_owned(),
                    end_time: "11:00".to_owned(),
                    status: "planned".to_owned(),
                    notes: String::new(),
                    linked_session_id: None,
                },
            )
            .expect("block should save");
            pyq::create(
                &mut connection,
                pyq::PyqInput {
                    question_year: 2024,
                    exam_session: String::new(),
                    part: "B".to_owned(),
                    topic_id,
                    question_reference: "Phase 6 PYQ".to_owned(),
                    attempt_date: "2026-10-28".to_owned(),
                    status: "correct".to_owned(),
                    marks_obtained: None,
                    max_marks: None,
                    time_spent_seconds: 60,
                    notes: String::new(),
                    difficulty: None,
                },
            )
            .expect("PYQ should save");
            mock_tests::create(
                &mut connection,
                mock_tests::MockTestInput {
                    test_name: "Phase 6 mock".to_owned(),
                    test_date: "2026-10-27".to_owned(),
                    duration_minutes: 180,
                    max_marks: 100.0,
                    marks_obtained: None,
                    part_a: mock_tests::MockPartInput {
                        questions: 1,
                        correct: 1,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: Some(2.0),
                        incorrect_marks: Some(0.0),
                        unanswered_marks: Some(0.0),
                    },
                    part_b: mock_tests::MockPartInput {
                        questions: 0,
                        correct: 0,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: None,
                        incorrect_marks: None,
                        unanswered_marks: None,
                    },
                    part_c: mock_tests::MockPartInput {
                        questions: 0,
                        correct: 0,
                        incorrect: 0,
                        unanswered: 0,
                        correct_marks: None,
                        incorrect_marks: None,
                        unanswered_marks: None,
                    },
                    notes: String::new(),
                    review_observations: String::new(),
                },
            )
            .expect("mock should save");
            connection
                .execute(
                    "INSERT INTO revision_schedules (
                        topic_id, status, last_revision_date, next_revision_date,
                        current_interval_days, completed_revisions, previous_outcome,
                        mastery_estimate, notes, created_at, updated_at
                     ) VALUES (
                        ?1, 'active', '2026-10-29', '2026-11-06', 8, 1, 'good',
                        70.0, '', '2026-10-29T10:00:00.000Z',
                        '2026-10-29T10:00:00.000Z'
                     )",
                    [topic_id],
                )
                .expect("version six revision schedule should save");
            connection
                .execute(
                    "INSERT INTO revision_history (
                        topic_id, revision_date, scheduled_for_date, outcome, notes,
                        interval_before_days, interval_after_days, mastery_before,
                        mastery_after, created_at
                     ) VALUES (
                        ?1, '2026-10-29', '2026-10-29', 'good', 'Phase 7 revision',
                        5, 8, 0.0, 70.0, '2026-10-29T10:00:00.000Z'
                     )",
                    [topic_id],
                )
                .expect("version six revision history should save");
        }

        let connection = open_database(&path, today).expect("database should upgrade and open");
        assert_eq!(schema_version(&connection), 12);

        let progress: (String, String) = connection
            .query_row(
                "SELECT status, notes FROM syllabus_progress LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("progress should survive");
        assert_eq!(
            progress,
            ("Revised".to_owned(), "Phase 3 progress".to_owned())
        );

        let counts: (i64, i64, i64, i64, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM study_sessions),
                    (SELECT COUNT(*) FROM checklist_tasks),
                    (SELECT COUNT(*) FROM planned_study_blocks),
                    (SELECT COUNT(*) FROM pyq_attempts),
                    (SELECT COUNT(*) FROM mock_tests),
                    (SELECT COUNT(*) FROM revision_schedules),
                    (SELECT COUNT(*) FROM revision_history),
                    (SELECT COUNT(*) FROM dashboard_sections)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("source and layout counts should be readable");
        assert_eq!(counts, (1, 1, 1, 1, 1, 1, 1, 9));
        assert_eq!(
            install_layout_order(&connection),
            vec![
                "exam-countdown",
                "syllabus-progress",
                "study-totals",
                "today-study",
                "revision-status",
                "deadlines-blocks",
                "recent-pyq",
                "latest-mock",
                "insights"
            ]
        );
        assert_eq!(
            connection
                .query_row("SELECT notes FROM revision_history LIMIT 1", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("revision history should survive"),
            "Phase 7 revision"
        );
    }

    fn install_layout_order(connection: &Connection) -> Vec<String> {
        let mut statement = connection
            .prepare("SELECT section_key FROM dashboard_sections ORDER BY display_order")
            .expect("layout should prepare");
        statement
            .query_map([], |row| row.get(0))
            .expect("layout should query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("layout should load")
    }

    #[test]
    fn version_eight_database_upgrades_without_losing_phase_nine_data() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);

        {
            let mut connection =
                Connection::open(&path).expect("version eight database should open");
            configure(&connection).expect("version eight database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..8])
                .to_latest(&mut connection)
                .expect("version eight schema should apply");
            assert_eq!(schema_version(&connection), 8);

            settings::ensure_defaults(&mut connection, today)
                .expect("version eight settings should exist");
            syllabus::seed(&mut connection).expect("version eight syllabus should seed");
            timer::ensure_defaults(&mut connection)
                .expect("version eight timer settings should exist");
            settings::update(
                &mut connection,
                SettingsInput {
                    exam_date: "2027-01-10".to_owned(),
                    preparation_start_date: "2026-10-01".to_owned(),
                    daily_study_target_minutes: 300,
                },
                today,
            )
            .expect("version eight settings should update");

            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                    [],
                    |row| row.get(0),
                )
                .expect("topic should exist");
            syllabus::update_progress(
                &mut connection,
                topic_id,
                syllabus::TopicProgressInput {
                    status: "Confident".to_owned(),
                    priority: "High-Yield".to_owned(),
                    confidence: 5,
                    notes: "Phase 9 progress".to_owned(),
                },
            )
            .expect("version eight progress should save");
            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-30".to_owned(),
                    duration_seconds: 3_600,
                    notes: "Phase 9 session".to_owned(),
                },
            )
            .expect("version eight session should save");
        }

        let connection =
            open_database(&path, today).expect("version eight database should upgrade");
        assert_eq!(schema_version(&connection), 12);

        let settings = settings::load(&connection).expect("upgraded settings should load");
        assert_eq!(settings.exam_date, "2027-01-10");
        assert_eq!(settings.preparation_start_date, "2026-10-01");
        assert_eq!(settings.daily_study_target_minutes, 300);
        assert_eq!(
            connection
                .query_row(
                    "SELECT notes FROM syllabus_progress WHERE topic_id = (
                        SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'
                     )",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("upgraded progress should load"),
            "Phase 9 progress"
        );
        assert_eq!(
            connection
                .query_row("SELECT notes FROM study_sessions", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("upgraded session should load"),
            "Phase 9 session"
        );

        let import_batches: i64 = connection
            .query_row("SELECT count(*) FROM json_import_batches", [], |row| {
                row.get(0)
            })
            .expect("import batch table should exist");
        assert_eq!(import_batches, 0);
        assert_eq!(install_layout_order(&connection).len(), 9);

        let backups_dir = directory.path().join("backups");
        let mut entries = fs::read_dir(&backups_dir)
            .expect("pre-migration backups should exist")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-pre-migration-")
                    && entry.file_name().to_string_lossy().ends_with(".sqlite3")
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        assert_eq!(entries.len(), 1);
        let verified = backup::verify_backup(&entries[0].path())
            .expect("version eight migration backup should verify");
        assert_eq!(verified.manifest.schema_version, 8);
    }

    #[test]
    fn version_nine_database_upgrades_without_losing_existing_data() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);

        {
            let mut connection =
                Connection::open(&path).expect("version nine database should open");
            configure(&connection).expect("version nine database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..9])
                .to_latest(&mut connection)
                .expect("version nine schema should apply");
            assert_eq!(schema_version(&connection), 9);
            settings::ensure_defaults(&mut connection, today)
                .expect("version nine settings should exist");
            syllabus::seed(&mut connection).expect("version nine syllabus should seed");
            let topic_id: i64 = connection
                .query_row(
                    "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                    [],
                    |row| row.get(0),
                )
                .expect("topic should exist");
            sessions::create_manual(
                &mut connection,
                sessions::StudySessionInput {
                    topic_id: Some(topic_id),
                    session_date: "2026-10-30".to_owned(),
                    duration_seconds: 1_800,
                    notes: "Version nine session".to_owned(),
                },
            )
            .expect("version nine session should save");
        }

        let connection = open_database(&path, today).expect("version nine database should upgrade");
        assert_eq!(schema_version(&connection), 12);
        assert_eq!(
            connection
                .query_row("SELECT notes FROM study_sessions", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("session should survive"),
            "Version nine session"
        );
        assert_eq!(install_layout_order(&connection).len(), 9);
        for table in [
            "study_resources",
            "resource_progress",
            "resource_local_files",
            "resource_discovery_runs",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("new resource table should exist");
            assert_eq!(count, 0);
        }
    }

    #[test]
    fn version_eleven_upgrades_to_four_active_exam_profiles_without_copying_the_legacy_date() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);

        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            assert_eq!(schema_version(&connection), 11);
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
            connection
                .execute(
                    "UPDATE settings
                     SET exam_date = '2027-05-10',
                         preparation_start_date = '2026-10-29'
                     WHERE id = 1",
                    [],
                )
                .expect("legacy exam date should save");
            connection
                .execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-11-02', 1800, 'manual', 'preserve across exam migration',
                        '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                     )",
                    [],
                )
                .expect("session should save");
        }

        let connection =
            open_database(&path, today).expect("version eleven database should upgrade");
        assert_eq!(schema_version(&connection), 12);

        let profiles: Vec<(String, i64, Option<String>, String)> = connection
            .prepare(
                "SELECT e.code, p.is_active, p.exam_date, p.legacy_date_notice_state
                 FROM exam_profiles p
                 JOIN exams e ON e.id = p.exam_id
                 ORDER BY e.display_order",
            )
            .expect("profiles should prepare")
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .expect("profiles should query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("profiles should load");

        assert_eq!(profiles.len(), 4);
        assert!(profiles.iter().all(|profile| profile.1 == 1));
        assert!(profiles.iter().all(|profile| profile.2.is_none()));
        assert_eq!(profiles[0].0, "csir-net-physics");
        assert_eq!(profiles[0].3, "pending");
        assert!(profiles[1..].iter().all(|profile| profile.3 == "none"));

        let legacy_date: String = connection
            .query_row("SELECT exam_date FROM settings WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("legacy setting should remain");
        assert_eq!(legacy_date, "2027-05-10");
        let session_count: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .expect("session should remain");
        assert_eq!(session_count, 1);

        let post_migration_path = backup::latest_post_migration_backup(directory.path())
            .expect("production post-migration snapshot should exist");
        let post_migration_verified = backup::verify_backup(&post_migration_path)
            .expect("post-migration snapshot should verify");
        assert_eq!(post_migration_verified.manifest.schema_version, 12);
        assert_eq!(post_migration_verified.manifest.reason, "post-migration");
        assert!(
            post_migration_verified
                .manifest
                .backup_file_name
                .starts_with("csir-net-tracker-post-migration-v12-")
        );
        assert_eq!(post_migration_verified.record_counts["study_sessions"], 1);
        assert_eq!(post_migration_verified.record_counts["exam_profiles"], 4);

        let (last_success_file, automatic_enabled, retention): (String, i64, i64) = connection
            .query_row(
                "SELECT last_success_file, automatic_enabled, retention_count
                 FROM backup_settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("last-success state should load");
        assert_eq!(
            last_success_file,
            post_migration_verified.manifest.backup_file_name
        );
        assert!(!last_success_file.contains("pre-migration"));
        assert_eq!(automatic_enabled, 1);
        assert_eq!(retention, 14);

        connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-11-03', 600, 'manual', 'created after exam migration',
                    '2026-11-03T00:00:00Z', '2026-11-03T00:00:00Z'
                 )",
                [],
            )
            .expect("post-migration session should save");
        let current_backup_dir = tempfile::tempdir().expect("current backup temp");
        let current_backup =
            backup::create_backup(&connection, current_backup_dir.path(), "manual", None)
                .expect("current backup should succeed");
        let current_backup_path = current_backup_dir.path().join(&current_backup.file_name);
        let current_verified =
            backup::verify_backup(&current_backup_path).expect("current backup should verify");
        assert_eq!(current_verified.manifest.schema_version, 12);
        assert_eq!(current_verified.record_counts["study_sessions"], 2);
        assert_eq!(current_verified.record_counts["exam_profiles"], 4);

        let backups_dir = directory.path().join("backups");
        let backup_name = fs::read_dir(&backups_dir)
            .expect("pre-migration backups should exist")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .find(|name| {
                name.starts_with("csir-net-tracker-pre-migration-") && name.ends_with(".sqlite3")
            })
            .expect("a pre-migration backup should exist");
        let verified = backup::verify_backup(&backups_dir.join(backup_name))
            .expect("pre-migration backup should verify");
        assert_eq!(verified.manifest.schema_version, 11);

        let post_count_before = fs::read_dir(directory.path().join("backups").join("safety"))
            .expect("safety backups should exist")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-post-migration-")
            })
            .count();
        drop(connection);
        let reopened = open_database(&path, today).expect("normal startup should reopen");
        assert_eq!(schema_version(&reopened), 12);
        let post_count_after = fs::read_dir(directory.path().join("backups").join("safety"))
            .expect("safety backups should exist")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-post-migration-")
            })
            .count();
        assert_eq!(post_count_after, post_count_before);
    }

    #[test]
    fn failed_post_migration_snapshot_blocks_startup_then_repairs_on_retry() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
        }
        let safety_path = directory.path().join("backups").join("safety");
        fs::create_dir_all(directory.path().join("backups")).expect("backups should exist");
        fs::write(&safety_path, b"blocking file").expect("safety blocker should write");

        let error = open_database(&path, today).expect_err("post-migration backup should fail");
        assert_eq!(error.code, "backup_failed");
        let connection = Connection::open(&path).expect("migrated database should reopen");
        assert_eq!(schema_version(&connection), 12);
        drop(connection);
        assert!(backup::latest_post_migration_backup(directory.path()).is_none());
        assert!(backup::latest_pre_migration_backup(directory.path()).is_some());
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("failed snapshot should leave a pending marker")
                .origin,
            "migration_pending"
        );

        fs::remove_file(&safety_path).expect("safety blocker should be removed");
        let repaired = open_database(&path, today).expect("retry should create the post snapshot");
        assert_eq!(schema_version(&repaired), 12);
        assert!(backup::latest_post_migration_backup(directory.path()).is_some());
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("retry should record upgraded state")
                .origin,
            "upgraded"
        );
    }

    #[test]
    fn fresh_marker_then_old_schema_post_failure_repairs_on_restart() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        fs::create_dir_all(&data_dir).expect("app data should exist");
        let path = database_path(&data_dir);
        let today = date(2026, 11, 2);
        drop(open_database(&path, today).expect("fresh install should open"));
        assert_eq!(
            backup::read_migration_state(&data_dir)
                .expect("fresh marker should exist")
                .origin,
            "fresh"
        );

        let source = tempfile::tempdir().expect("source temp");
        let source_path = source.path().join(DATABASE_FILE_NAME);
        {
            let mut connection =
                Connection::open(&source_path).expect("version eleven source should open");
            configure(&connection).expect("source should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("source version eleven should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("source settings should exist");
        }
        remove_database_sidecars(&path).expect("target sidecars should be removed");
        fs::remove_file(&path).expect("fresh target database should be removable");
        fs::copy(&source_path, &path).expect("version eleven database should be installed");

        let safety_blocker = data_dir.join("backups").join("safety");
        fs::create_dir_all(data_dir.join("backups")).expect("backups folder should exist");
        fs::write(&safety_blocker, b"blocking file").expect("safety blocker should write");
        let error = open_database(&path, today).expect_err("post snapshot should fail");
        assert_eq!(error.code, "backup_failed");
        assert_eq!(
            backup::read_migration_state(&data_dir)
                .expect("migration-pending marker should exist")
                .origin,
            "migration_pending"
        );
        assert!(backup::latest_post_migration_backup(&data_dir).is_none());
        assert!(backup::latest_pre_migration_backup(&data_dir).is_some());

        fs::remove_file(&safety_blocker).expect("safety blocker should be removed");
        let repaired = open_database(&path, today).expect("retry should repair the post snapshot");
        assert_eq!(schema_version(&repaired), 12);
        assert!(backup::latest_post_migration_backup(&data_dir).is_some());
        assert_eq!(
            backup::read_migration_state(&data_dir)
                .expect("upgraded marker should exist")
                .origin,
            "upgraded"
        );
    }

    #[test]
    fn pending_marker_with_valid_post_snapshot_is_adopted_and_finalized() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
        }
        drop(open_database(&path, today).expect("first upgrade should succeed"));
        let post_before = backup::latest_post_migration_backup(directory.path()).expect("post");
        backup::record_migration_pending(directory.path()).expect("pending marker should write");

        let connection = open_database(&path, today).expect("pending marker should finalize");
        assert_eq!(schema_version(&connection), 12);
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("marker should exist")
                .origin,
            "upgraded"
        );
        assert_eq!(
            backup::latest_post_migration_backup(directory.path()).expect("post should remain"),
            post_before
        );
    }

    #[test]
    fn corrupt_marker_with_missing_post_snapshot_repairs() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
        }
        drop(open_database(&path, today).expect("upgrade should succeed"));
        let post = backup::latest_post_migration_backup(directory.path()).expect("post snapshot");
        fs::remove_file(&post).expect("post snapshot should be removed");
        fs::remove_file(backup::manifest_path(&post)).expect("post manifest should be removed");
        fs::write(
            directory
                .path()
                .join("backups")
                .join("migration-state.json"),
            b"{corrupt",
        )
        .expect("marker should be corrupted");

        let connection = open_database(&path, today).expect("corrupt marker should repair");
        assert_eq!(schema_version(&connection), 12);
        assert!(backup::latest_post_migration_backup(directory.path()).is_some());
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("repaired marker should exist")
                .origin,
            "upgraded"
        );
    }

    #[test]
    fn newer_schema_live_database_is_not_replaced_by_older_safety_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "newer schema contents");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);

        let connection = Connection::open(&live_path).expect("newer live should open");
        connection
            .execute_batch("PRAGMA user_version = 13;")
            .expect("newer schema should be set");
        drop(connection);
        let newer_bytes = fs::read(&live_path).expect("newer database should be readable");
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "live_renamed",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert_eq!(recovery.live_status, "newer_schema");
        assert!(!recovery.keep_allowed);

        let first_start = AppState::initialize(&data_dir, today);
        assert!(first_start.with_connection(|_| Ok(())).is_err());
        assert_eq!(
            fs::read(&live_path).expect("newer database should remain"),
            newer_bytes
        );
        assert!(backup::read_restore_operation(&data_dir).is_some());

        let second_start = AppState::initialize(&data_dir, today);
        assert!(second_start.with_connection(|_| Ok(())).is_err());
        assert_eq!(
            fs::read(&live_path).expect("newer database should remain after restart"),
            newer_bytes
        );
        assert!(backup::read_restore_operation(&data_dir).is_some());
    }

    #[test]
    fn displaced_live_database_survives_safety_recovery_and_keep_rejection() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "safety contents");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);

        let live = Connection::open(&live_path).expect("live should open");
        live.execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("foreign keys should be disabled");
        live.execute(
            "INSERT INTO study_sessions (
                topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
             ) VALUES (
                999999, '2026-11-02', 600, 'manual', 'displaced live contents',
                '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
             )",
            [],
        )
        .expect("foreign-key-invalid live session should insert");
        drop(live);

        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "live_renamed",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();
        backup::recover_interrupted_restore_before_open(&data_dir, &live_path, 12)
            .expect("safety recovery should succeed");

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(recovery.live_valid);
        assert_eq!(recovery.live_status, "valid");
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("Unidentified"))
        );
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());

        let failed = fs::read_dir(&data_dir)
            .expect("app data should exist")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.to_string_lossy().ends_with(".failed"))
            .expect("displaced live database should be preserved as .failed");
        let displaced = Connection::open(&failed).expect("displaced database should open");
        let notes: String = displaced
            .query_row(
                "SELECT notes FROM study_sessions WHERE notes = 'displaced live contents'",
                [],
                |row| row.get(0),
            )
            .expect("displaced contents should remain recoverable");
        assert_eq!(notes, "displaced live contents");
    }

    #[test]
    fn each_schema_upgrade_creates_its_own_post_migration_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
        }

        let first = open_database(&path, today).expect("first upgrade should succeed");
        assert_eq!(schema_version(&first), 12);
        drop(first);
        let first_pre =
            backup::latest_pre_migration_backup(directory.path()).expect("first pre snapshot");
        let first_post =
            backup::latest_post_migration_backup(directory.path()).expect("first post snapshot");
        assert!(
            fs::read_dir(directory.path().join("backups").join("safety"))
                .expect("safety backups should exist")
                .filter_map(Result::ok)
                .all(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    !name.ends_with("-wal") && !name.ends_with("-shm")
                })
        );
        let post_count = || {
            fs::read_dir(directory.path().join("backups").join("safety"))
                .expect("safety backups should exist")
                .filter_map(Result::ok)
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    name.starts_with("csir-net-tracker-post-migration-")
                        && name.ends_with(".sqlite3")
                })
                .count()
        };
        assert_eq!(post_count(), 1);

        remove_database_sidecars(&path).expect("restore sidecars should be removed");
        fs::copy(&first_pre, &path).expect("older snapshot should replace the live database");
        let second = open_database(&path, today).expect("second upgrade should succeed");
        assert_eq!(schema_version(&second), 12);
        drop(second);
        assert_eq!(post_count(), 2);
        assert_ne!(
            backup::latest_post_migration_backup(directory.path()).expect("second post snapshot"),
            first_post
        );

        let reopened = open_database(&path, today).expect("normal startup should succeed");
        assert_eq!(schema_version(&reopened), 12);
        drop(reopened);
        assert_eq!(post_count(), 2);
    }

    #[test]
    fn app_state_lock_serializes_recovery_style_operations() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let state = Arc::new(AppState::initialize(&data_dir, date(2026, 11, 2)));
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let holder_state = Arc::clone(&state);
        let holder = std::thread::spawn(move || {
            holder_state
                .with_connection(|_| {
                    locked_tx.send(()).expect("lock notification should send");
                    release_rx
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .expect("lock should be released");
                    Ok(())
                })
                .expect("holder operation should succeed");
        });
        locked_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("holder should acquire the lock");

        let (acquired_tx, acquired_rx) = mpsc::channel();
        let waiter_state = Arc::clone(&state);
        let waiter = std::thread::spawn(move || {
            waiter_state
                .with_connection(|_| Ok(()))
                .expect("waiter operation should succeed");
            acquired_tx
                .send(())
                .expect("acquire notification should send");
        });
        assert!(
            acquired_rx
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err(),
            "a second recovery-style operation must not enter while the lock is held"
        );
        release_tx.send(()).expect("lock release should send");
        assert!(
            acquired_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .is_ok(),
            "the waiting operation should proceed after release"
        );
        holder.join().expect("holder thread should join");
        waiter.join().expect("waiter thread should join");
    }

    #[test]
    fn fresh_install_records_fresh_marker_without_post_migration_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        let connection = open_database(&path, today).expect("fresh install should open");
        assert_eq!(schema_version(&connection), 12);
        let marker = backup::read_migration_state(directory.path()).expect("fresh marker");
        assert_eq!(marker.origin, "fresh");
        assert!(marker.post_migration_snapshot.is_none());
        assert!(backup::latest_post_migration_backup(directory.path()).is_none());
        let settings = backup::load_settings(&connection).expect("settings should load");
        assert!(!settings.automatic_enabled);
    }

    #[test]
    fn stale_fresh_marker_with_pre_artifact_still_repairs_post_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        drop(open_database(&path, today).expect("fresh install should open"));
        fs::write(
            directory
                .path()
                .join("backups")
                .join("csir-net-tracker-pre-migration-v11-20260101-000000.sqlite3"),
            b"stale pre artifact",
        )
        .expect("pre artifact should write");
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("marker should remain fresh")
                .origin,
            "fresh"
        );

        let connection = open_database(&path, today).expect("repair should succeed");
        assert_eq!(schema_version(&connection), 12);
        assert!(backup::latest_post_migration_backup(directory.path()).is_some());
        assert_eq!(
            backup::read_migration_state(directory.path())
                .expect("marker should be upgraded")
                .origin,
            "upgraded"
        );
    }

    #[test]
    fn missing_marker_pre_and_post_requires_and_creates_post_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        drop(open_database(&path, today).expect("first fresh start"));
        let marker_path = directory
            .path()
            .join("backups")
            .join("migration-state.json");
        fs::remove_file(&marker_path).expect("marker should be removed");
        assert!(backup::read_migration_state(directory.path()).is_none());
        assert!(backup::latest_pre_migration_backup(directory.path()).is_none());
        assert!(backup::latest_post_migration_backup(directory.path()).is_none());

        let connection = open_database(&path, today).expect("repair should create post snapshot");
        let post = backup::latest_post_migration_backup(directory.path())
            .expect("post snapshot should exist");
        let verified = backup::verify_backup(&post).expect("post snapshot should verify");
        assert_eq!(verified.manifest.reason, "post-migration");
        let marker = backup::read_migration_state(directory.path()).expect("marker should exist");
        assert_eq!(marker.origin, "upgraded");
        assert_eq!(
            marker.post_migration_snapshot.as_deref(),
            Some(verified.manifest.backup_file_name.as_str())
        );
        let settings = backup::load_settings(&connection).expect("settings should load");
        assert!(settings.automatic_enabled);
        assert_eq!(settings.retention_count, 14);
        assert_eq!(
            settings.last_success_file.as_deref(),
            Some(verified.manifest.backup_file_name.as_str())
        );
    }

    #[test]
    fn valid_post_snapshot_without_marker_is_adopted_without_duplication() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        let connection = open_database(&path, today).expect("fresh install should open");
        let existing = backup::create_post_migration_backup(&connection, directory.path())
            .expect("existing post snapshot should be created");
        connection
            .execute(
                "UPDATE backup_settings
                 SET last_success_at = '2020-01-01T00:00:00Z',
                     last_success_file = 'csir-net-tracker-pre-migration-v11-missing.sqlite3'
                 WHERE id = 1",
                [],
            )
            .unwrap();
        drop(connection);
        fs::remove_file(
            directory
                .path()
                .join("backups")
                .join("migration-state.json"),
        )
        .expect("marker should be removed");

        let connection = open_database(&path, today).expect("adoption should succeed");
        let marker = backup::read_migration_state(directory.path()).expect("marker should exist");
        assert_eq!(marker.origin, "upgraded");
        let post = backup::latest_post_migration_backup(directory.path()).expect("post remains");
        assert_eq!(
            post.file_name().unwrap().to_string_lossy(),
            existing.file_name
        );
        let settings = backup::load_settings(&connection).expect("settings should load");
        assert_eq!(
            settings.last_success_file.as_deref(),
            Some(existing.file_name.as_str())
        );
        let count = fs::read_dir(directory.path().join("backups").join("safety"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with("csir-net-tracker-post-migration-") && name.ends_with(".sqlite3")
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn corrupt_pre_and_missing_post_still_repair_and_preserve_corrupt_pre() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        {
            let mut connection =
                Connection::open(&path).expect("version eleven database should open");
            configure(&connection).expect("version eleven database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("version eleven schema should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("version eleven settings should exist");
        }
        drop(open_database(&path, today).expect("first upgrade should succeed"));
        let pre = backup::latest_pre_migration_backup(directory.path()).expect("pre snapshot");
        let post = backup::latest_post_migration_backup(directory.path()).expect("post snapshot");
        fs::write(&pre, b"corrupt pre").expect("pre should be corrupted");
        fs::write(&post, b"corrupt post").expect("post should be corrupted");

        let connection = open_database(&path, today).expect("repair should succeed");
        let repaired = backup::latest_post_migration_backup(directory.path())
            .expect("replacement post snapshot");
        assert_ne!(repaired, post);
        let verified = backup::verify_backup(&repaired).expect("replacement should verify");
        assert_eq!(verified.manifest.schema_version, 12);
        assert_eq!(
            backup::load_settings(&connection)
                .expect("settings should load")
                .last_success_file
                .as_deref(),
            Some(verified.manifest.backup_file_name.as_str())
        );
        assert_eq!(
            fs::read(&pre).expect("corrupt pre preserved"),
            b"corrupt pre"
        );
        assert_eq!(
            fs::read(&post).expect("corrupt post preserved"),
            b"corrupt post"
        );
    }

    #[test]
    fn stale_last_success_is_replaced_when_post_snapshot_is_repaired() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 11, 2);
        let connection = open_database(&path, today).expect("fresh install should open");
        connection
            .execute(
                "UPDATE backup_settings
                 SET last_success_at = '2020-01-01T00:00:00Z',
                     last_success_file = 'csir-net-tracker-manual-v12-missing.sqlite3'
                 WHERE id = 1",
                [],
            )
            .expect("stale last success should write");
        drop(connection);
        fs::remove_file(
            directory
                .path()
                .join("backups")
                .join("migration-state.json"),
        )
        .expect("marker should be removed");

        let connection = open_database(&path, today).expect("repair should succeed");
        let post = backup::latest_post_migration_backup(directory.path()).expect("post snapshot");
        let verified = backup::verify_backup(&post).expect("post should verify");
        let settings = backup::load_settings(&connection).expect("settings should load");
        assert_eq!(
            settings.last_success_file.as_deref(),
            Some(verified.manifest.backup_file_name.as_str())
        );
        assert_ne!(
            settings.last_success_file.as_deref(),
            Some("csir-net-tracker-manual-v12-missing.sqlite3")
        );
    }

    #[test]
    fn connection_uses_foreign_keys_and_a_busy_timeout() {
        let connection = test_connection();
        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        let busy_timeout: i64 = connection
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();

        assert_eq!(foreign_keys, 1);
        assert_eq!(busy_timeout, 5_000);
    }

    #[test]
    fn file_database_uses_wal_mode() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let connection = open_database(&path, date(2026, 10, 29)).expect("database should open");
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode.to_lowercase(), "wal");
    }

    #[test]
    fn app_state_creates_the_app_data_directory_and_defaults() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("nested").join("app-data");
        let state = AppState::initialize(&data_dir, date(2026, 10, 29));

        assert!(data_dir.is_dir());
        assert!(database_path(&data_dir).is_file());

        let settings: Settings = state
            .with_connection(|connection| settings::load(connection))
            .expect("settings should load through managed state");
        assert_eq!(settings.exam_date, "2026-12-19");
        assert_eq!(settings.preparation_start_date, "2026-10-29");
        assert_eq!(settings.daily_study_target_minutes, 240);
    }

    #[test]
    fn initialization_failure_becomes_a_structured_command_error() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let not_a_directory = directory.path().join("file");
        fs::write(&not_a_directory, b"not a directory").unwrap();

        let state = AppState::initialize(&not_a_directory, date(2026, 10, 29));
        let error = state
            .with_connection(|connection| settings::load(connection))
            .expect_err("initialization should fail");

        assert_eq!(error.code, "database_unavailable");
        assert!(error.message.contains("application-data folder"));
    }

    #[test]
    fn migration_creates_a_verified_backup_with_the_same_database_identity() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);
        {
            let mut connection =
                Connection::open(&path).expect("version seven database should open");
            configure(&connection).expect("database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..7])
                .to_latest(&mut connection)
                .expect("version seven schema should apply");
            settings::ensure_defaults(&mut connection, today).expect("settings should exist");
            syllabus::seed(&mut connection).expect("syllabus should seed");
        }

        let connection = open_database(&path, today).expect("database should upgrade");
        assert_eq!(schema_version(&connection), 12);
        let metadata_id = backup::database_identifier(&connection)
            .expect("identity should load")
            .expect("identity should exist");
        let backups_dir = directory.path().join("backups");
        let mut entries = fs::read_dir(&backups_dir)
            .expect("backups should exist")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("csir-net-tracker-pre-migration-")
                    && entry.file_name().to_string_lossy().ends_with(".sqlite3")
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        assert_eq!(entries.len(), 1);
        let verified =
            backup::verify_backup(&entries[0].path()).expect("migration backup should verify");
        assert_eq!(verified.manifest.schema_version, 7);
        assert_eq!(verified.manifest.database_id, metadata_id);
    }

    #[test]
    fn migration_backup_failure_aborts_upgrade_without_changing_schema() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);
        {
            let mut connection =
                Connection::open(&path).expect("version seven database should open");
            configure(&connection).expect("database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..7])
                .to_latest(&mut connection)
                .expect("version seven schema should apply");
            settings::ensure_defaults(&mut connection, today).expect("settings should exist");
            syllabus::seed(&mut connection).expect("syllabus should seed");
        }
        fs::write(directory.path().join("backups"), b"blocking file")
            .expect("blocking file should write");

        let error = open_database(&path, today).expect_err("migration should abort");
        assert_eq!(error.code, "backup_failed");
        let connection = Connection::open(&path).expect("database should still open");
        assert_eq!(schema_version(&connection), 7);
    }

    #[test]
    fn corrupt_database_is_not_deleted_or_silently_overwritten() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        fs::create_dir_all(&data_dir).unwrap();
        let path = database_path(&data_dir);
        let corrupt = b"not a sqlite database";
        fs::write(&path, corrupt).expect("corrupt database should be written");
        let before = fs::read(&path).unwrap();

        let state = AppState::initialize(&data_dir, date(2026, 10, 30));
        let error = state
            .with_connection(|connection| settings::load(connection))
            .expect_err("corrupt database should not open");
        assert_eq!(error.code, "database_unavailable");
        assert_eq!(fs::read(&path).unwrap(), before);
    }

    #[test]
    fn old_schema_restore_preserves_newer_records_in_safety_and_can_restore_them() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        {
            let mut connection =
                Connection::open(&source_path).expect("source database should open");
            configure(&connection).expect("source database should configure");
            Migrations::from_slice(&MIGRATIONS_SLICE[..11])
                .to_latest(&mut connection)
                .expect("source schema eleven should apply");
            settings::ensure_defaults(&mut connection, today)
                .expect("source settings should exist");
            connection
                .execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-10-01', 600, 'manual', 'older snapshot session',
                        '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z'
                     )",
                    [],
                )
                .expect("source session should save");
        }
        let source_connection =
            Connection::open(&source_path).expect("source database should reopen");
        let old_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "pre-migration", None)
                .expect("old backup should succeed");
        drop(source_connection);

        let target_dir = target.path().join("app-data");
        let state = AppState::initialize(&target_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-11-02', 1200, 'manual', 'newer live session',
                        '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                     )",
                    [],
                ).map_err(|error| AppError::database("newer session should save", error))?;
                Ok(())
            })
            .expect("newer session should save");

        let safety = state
            .restore_from_backup(&backup_dir.path().join(&old_backup.file_name), today)
            .expect("old-schema restore should succeed");
        let live_notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("old session should load", error))
            })
            .expect("old session should load");
        assert_eq!(live_notes, "older snapshot session");

        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        let safety_connection = Connection::open(&safety_path).expect("safety should open");
        let safety_notes: String = safety_connection
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("safety session should load");
        assert_eq!(safety_notes, "newer live session");
        drop(safety_connection);

        state
            .restore_from_backup(&safety_path, today)
            .expect("safety restore should succeed");
        let recovered_notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("recovered session should load", error))
            })
            .expect("recovered session should load");
        assert_eq!(recovered_notes, "newer live session");
    }

    #[test]
    fn interrupted_restore_recovers_only_when_live_database_is_invalid_and_preserves_candidates() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = AppState::initialize(&data_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-11-02', 1200, 'manual', 'newer safety session',
                        '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                     )",
                    [],
                ).map_err(|error| AppError::database("safety session should save", error))?;
                Ok(())
            })
            .expect("safety session should save");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety backup should succeed");
        drop(state);

        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        let rollback_path = data_dir.join(".csir-net-tracker-restore-test.rollback");
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        let operation = backup::RestoreOperation {
            token: "test".to_owned(),
            stage: "live_renamed".to_owned(),
            source_backup_path: "interrupted-source.sqlite3".to_owned(),
            source_schema_version: 11,
            safety_backup_path: safety_path.display().to_string(),
            safety_schema_version: 12,
            live_path: live_path.display().to_string(),
            rollback_path: rollback_path.display().to_string(),
            temp_path: temp_path.display().to_string(),
            started_at: "2026-11-02T00:00:00Z".to_owned(),
            updated_at: "2026-11-02T00:00:00Z".to_owned(),
            database_id: None,
        };
        backup::write_restore_operation(&data_dir, &operation).expect("operation should write");
        fs::write(&live_path, b"not a database").expect("broken live file should write");
        fs::write(&rollback_path, b"old rollback").expect("rollback file should write");
        fs::write(&temp_path, b"candidate").expect("temp file should write");

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.live_valid);
        assert!(recovery.safety_verified);
        assert_eq!(recovery.safety_schema_version, Some(12));
        assert!(!recovery.keep_allowed);

        backup::recover_interrupted_restore_before_open(&data_dir, &live_path, 12)
            .expect("interrupted restore should recover the safety snapshot");
        let recovered = Connection::open(&live_path).expect("recovered database should open");
        let notes: String = recovered
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("recovered session should load");
        assert_eq!(notes, "newer safety session");
        drop(recovered);
        assert!(safety_path.is_file());
        assert!(rollback_path.is_file());
        assert!(temp_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());

        let keep_error = backup::keep_restore_recovery(&data_dir, &live_path, 12, true)
            .expect_err("unsafe keep must be rejected");
        assert!(
            keep_error.message.contains("Unidentified restore files")
                || keep_error.message.contains("installed restore candidate")
        );
        assert!(rollback_path.is_file());
        assert!(temp_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());
    }

    #[test]
    fn interrupted_restore_does_not_overwrite_a_valid_live_database() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = AppState::initialize(&data_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-11-02', 1200, 'manual', 'current live session',
                        '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                     )",
                    [],
                ).map_err(|error| AppError::database("live session should save", error))?;
                Ok(())
            })
            .expect("live session should save");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety backup should succeed");
        drop(state);
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        let operation = backup::RestoreOperation {
            token: "test-valid".to_owned(),
            stage: "candidate_installed".to_owned(),
            source_backup_path: "selected.sqlite3".to_owned(),
            source_schema_version: 11,
            safety_backup_path: safety_path.display().to_string(),
            safety_schema_version: 12,
            live_path: live_path.display().to_string(),
            rollback_path: data_dir
                .join(".csir-net-tracker-restore-valid.rollback")
                .display()
                .to_string(),
            temp_path: data_dir
                .join(".csir-net-tracker-restore-valid.tmp")
                .display()
                .to_string(),
            started_at: "2026-11-02T00:00:00Z".to_owned(),
            updated_at: "2026-11-02T00:00:00Z".to_owned(),
            database_id: None,
        };
        backup::write_restore_operation(&data_dir, &operation).expect("operation should write");

        backup::recover_interrupted_restore_before_open(&data_dir, &live_path, 12)
            .expect("valid live database should remain untouched");
        let live = Connection::open(&live_path).expect("live database should open");
        let notes: String = live
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("live session should load");
        assert_eq!(notes, "current live session");
        assert!(safety_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());
    }

    #[test]
    fn empty_live_file_is_not_treated_as_a_valid_recovered_database() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "safety contents");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);
        fs::write(&live_path, b"").expect("empty live file should write");
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "live_renamed",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();
        backup::recover_interrupted_restore_before_open(&data_dir, &live_path, 12)
            .expect("empty live file should be recovered from safety");
        let connection = Connection::open(&live_path).expect("recovered live should open");
        let notes: String = connection
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("recovered notes should load");
        assert_eq!(notes, "safety contents");
    }

    #[test]
    fn missing_live_and_metadata_fail_closed_instead_of_creating_a_fresh_database() {
        for corrupt_metadata in [false, true] {
            let directory = tempfile::tempdir().expect("temporary directory should be created");
            let data_dir = directory.path().join("app-data");
            fs::create_dir_all(data_dir.join("backups")).expect("app data should exist");
            let orphan = data_dir.join(".csir-net-tracker-restore-orphan.rollback");
            fs::write(&orphan, b"orphan rollback").expect("orphan should write");
            if corrupt_metadata {
                fs::write(
                    data_dir.join("backups").join("restore-operation.json"),
                    b"{broken",
                )
                .expect("corrupt metadata should write");
            }
            let state = AppState::initialize(&data_dir, date(2026, 11, 2));
            assert!(
                state.with_connection(|_| Ok(())).is_err(),
                "startup must not initialize over an unresolved restore"
            );
            assert!(orphan.is_file());
            assert!(
                !database_path(&data_dir).exists(),
                "a missing live path must not become a fresh database"
            );
        }
    }

    #[test]
    fn stale_stage_with_preserved_candidates_blocks_keep() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "stale stage session");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);
        let rollback_path = data_dir.join(".csir-net-tracker-restore-test.rollback");
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        fs::copy(&live_path, &rollback_path).expect("rollback copy should succeed");
        fs::write(&temp_path, b"candidate").expect("temp should write");
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "obsolete-stage",
            &rollback_path,
            &temp_path,
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("installed restore candidate"))
        );
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
        assert!(rollback_path.is_file());
        assert!(temp_path.is_file());
    }

    #[test]
    fn keep_is_blocked_for_installed_candidate_with_valid_safety_and_rollback() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "original session");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);
        let rollback_path = data_dir.join(".csir-net-tracker-restore-test.rollback");
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        fs::copy(&live_path, &rollback_path).expect("rollback copy should succeed");
        fs::write(&temp_path, b"candidate").expect("temp should write");
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "candidate_installed",
            &rollback_path,
            &temp_path,
        );
        backup::write_restore_operation(&data_dir, &operation).expect("operation should write");

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(recovery.live_valid);
        assert!(recovery.safety_verified);
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("installed restore candidate"))
        );
        let error = backup::keep_restore_recovery(&data_dir, &live_path, 12, true)
            .expect_err("keep must preserve candidate installed state");
        assert!(error.message.contains("installed restore candidate"));
        assert!(rollback_path.is_file());
        assert!(temp_path.is_file());
        assert!(safety_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());
    }

    #[test]
    fn keep_is_blocked_when_safety_is_missing_or_corrupt() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "original session");
        let live_path = database_path(&data_dir);
        drop(state);
        let rollback_path = data_dir.join(".csir-net-tracker-restore-test.rollback");
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        fs::copy(&live_path, &rollback_path).expect("rollback copy should succeed");
        fs::write(&temp_path, b"candidate").expect("temp should write");

        for corrupt in [false, true] {
            let safety_path = data_dir
                .join("backups")
                .join("safety")
                .join("csir-net-tracker-pre-restore-v12-20261102-000000.sqlite3");
            if corrupt {
                fs::create_dir_all(safety_path.parent().unwrap()).unwrap();
                fs::write(&safety_path, b"not a safety snapshot").unwrap();
            } else {
                let _ = fs::remove_file(&safety_path);
            }
            let operation = test_restore_operation(
                &live_path,
                &safety_path,
                "candidate_installed",
                &rollback_path,
                &temp_path,
            );
            backup::write_restore_operation(&data_dir, &operation).unwrap();
            let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
            assert!(!recovery.safety_verified);
            assert!(!recovery.keep_allowed);
            assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
            assert!(rollback_path.is_file());
            assert!(temp_path.is_file());
            assert!(backup::read_restore_operation(&data_dir).is_some());
        }
    }

    #[test]
    fn keep_revalidation_runs_under_the_app_state_lock() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "locked keep");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "opened",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();

        state
            .with_connection(|connection| {
                let schema = backup::schema_version(connection)?;
                backup::keep_restore_recovery(&data_dir, &live_path, schema, true)
                    .map_err(|error| AppError::restore_failed(error.message))
            })
            .expect("keep should run while the app lock is held");
        assert!(backup::read_restore_operation(&data_dir).is_none());
    }

    #[test]
    fn keep_requires_confirmation_and_removes_candidates_only_when_opened() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "opened session");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let source_dir = tempfile::tempdir().expect("source backup temp");
        let source_backup = state
            .with_connection(|connection| {
                backup::create_backup(connection, source_dir.path(), "manual", None)
            })
            .expect("source backup should succeed");
        let source_path = source_dir.path().join(&source_backup.file_name);
        let database_id = state
            .with_connection(|connection| backup::database_identifier(connection))
            .expect("database identity should load");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);
        let rollback_path = data_dir.join(".csir-net-tracker-restore-test.rollback");
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        fs::copy(&live_path, &rollback_path).expect("rollback copy should succeed");
        fs::copy(&source_path, &temp_path).expect("temp source copy should succeed");
        let mut operation = test_restore_operation(
            &live_path,
            &safety_path,
            "opened",
            &rollback_path,
            &temp_path,
        );
        operation.source_backup_path = source_path.display().to_string();
        operation.database_id = database_id;
        backup::write_restore_operation(&data_dir, &operation).expect("operation should write");

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(recovery.keep_allowed);
        assert_eq!(recovery.deletion_files.len(), 2);
        let error = backup::keep_restore_recovery(&data_dir, &live_path, 12, false)
            .expect_err("confirmation must be required");
        assert!(error.message.contains("Confirm"));
        assert!(rollback_path.is_file());
        assert!(temp_path.is_file());

        let result = backup::keep_restore_recovery(&data_dir, &live_path, 12, true)
            .expect("confirmed keep should succeed");
        assert_eq!(result.removed_files.len(), 2);
        assert!(!rollback_path.exists());
        assert!(!temp_path.exists());
        assert!(safety_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_none());
    }

    #[test]
    fn keep_preserves_a_temp_candidate_without_source_provenance() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "provenance session");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety should succeed");
        let live_path = database_path(&data_dir);
        let safety_path = PathBuf::from(&safety.directory).join(&safety.file_name);
        drop(state);
        let temp_path = data_dir.join(".csir-net-tracker-restore-test.tmp");
        fs::write(&temp_path, b"not the selected source").expect("temp should write");
        let operation = test_restore_operation(
            &live_path,
            &safety_path,
            "opened",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &temp_path,
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("proven"))
        );
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
        assert!(temp_path.is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());
    }

    #[test]
    fn keep_is_blocked_without_metadata_or_with_a_mismatched_live_path() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "metadata session");
        let live_path = database_path(&data_dir);
        drop(state);

        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.keep_allowed);
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
        assert!(backup::read_restore_operation(&data_dir).is_none());

        let safety_path = data_dir
            .join("backups")
            .join("safety")
            .join("csir-net-tracker-pre-restore-v12-20261102-000000.sqlite3");
        fs::create_dir_all(safety_path.parent().unwrap()).unwrap();
        fs::write(&safety_path, b"not a database").unwrap();
        let operation = test_restore_operation(
            &data_dir.join("different.sqlite3"),
            &safety_path,
            "opened",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();
        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("does not match"))
        );
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
        assert!(backup::read_restore_operation(&data_dir).is_some());

        let outside_safety = directory.path().join("outside-safety.sqlite3");
        fs::write(&outside_safety, b"not a safety snapshot").unwrap();
        let operation = test_restore_operation(
            &live_path,
            &outside_safety,
            "opened",
            &data_dir.join(".csir-net-tracker-restore-test.rollback"),
            &data_dir.join(".csir-net-tracker-restore-test.tmp"),
        );
        backup::write_restore_operation(&data_dir, &operation).unwrap();
        let recovery = backup::restore_recovery(&data_dir, &live_path, 12);
        assert!(!recovery.keep_allowed);
        assert!(
            recovery
                .keep_blocked_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("protected application recovery locations"))
        );
        assert!(backup::keep_restore_recovery(&data_dir, &live_path, 12, true).is_err());
    }

    #[test]
    fn failed_candidate_install_restores_original_and_never_creates_empty_database() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        source_connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-11-02', 600, 'manual', 'selected source',
                    '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        let source_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup");
        drop(source_connection);

        let data_dir = target.path().join("app-data");
        let state = state_with_session(&data_dir, today, "original database");
        inject_fault("candidate_install_rename");
        let error = state
            .restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today)
            .expect_err("candidate install should fail");
        clear_fault();
        assert!(error.message.contains("could not be installed"));
        let notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("notes should load", error))
            })
            .expect("original database should remain available");
        assert_eq!(notes, "original database");
        let operation = backup::read_restore_operation(&data_dir)
            .expect("recovery metadata should be preserved");
        assert_eq!(operation.stage, "recovered_original");
        assert!(PathBuf::from(&operation.temp_path).is_file());
        assert!(!database_path(&data_dir).is_dir());
    }

    #[test]
    fn failed_rollback_and_safety_restore_leave_files_and_metadata_preserved() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        source_connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-11-02', 600, 'manual', 'selected source',
                    '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        let source_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup");
        drop(source_connection);

        let data_dir = target.path().join("app-data");
        let state = state_with_session(&data_dir, today, "original database");
        inject_fault("candidate_install_rename,rollback_restore_rename,safety_restore_copy");
        let error = state
            .restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today)
            .expect_err("restore and recovery should fail closed");
        clear_fault();
        assert!(error.message.contains("automatic recovery failed"));
        assert!(
            backup::read_restore_operation(&data_dir).is_some(),
            "metadata must remain"
        );
        assert!(
            !database_path(&data_dir).exists(),
            "a missing live path must not become a fresh database"
        );
        let operation = backup::read_restore_operation(&data_dir).unwrap();
        assert!(PathBuf::from(&operation.rollback_path).is_file());
        assert!(PathBuf::from(&operation.temp_path).is_file());
        assert!(PathBuf::from(&operation.safety_backup_path).is_file());
        assert!(state.with_connection(|_| Ok(())).is_err());
    }

    #[test]
    fn safety_recovery_succeeds_when_candidate_and_rollback_install_fail() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        source_connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-11-02', 600, 'manual', 'selected source',
                    '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        let source_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup");
        drop(source_connection);

        let data_dir = target.path().join("app-data");
        let state = state_with_session(&data_dir, today, "original database");
        inject_fault("candidate_install_rename,rollback_restore_rename");
        let error = state
            .restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today)
            .expect_err("restore should fail");
        clear_fault();
        assert!(error.message.contains("original database was recovered"));
        let notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("notes should load", error))
            })
            .expect("safety snapshot should be usable");
        assert_eq!(notes, "original database");
        let operation =
            backup::read_restore_operation(&data_dir).expect("recovery metadata should remain");
        assert_eq!(operation.stage, "recovered_safety");
        assert!(PathBuf::from(&operation.rollback_path).is_file());
        assert!(PathBuf::from(&operation.temp_path).is_file());
        assert!(PathBuf::from(&operation.safety_backup_path).is_file());
    }

    #[test]
    fn opened_stage_or_sidecar_cleanup_failure_preserves_rollback_and_metadata() {
        for fault in ["stage_opened", "sidecar_clear"] {
            let source = tempfile::tempdir().expect("source temp");
            let target = tempfile::tempdir().expect("target temp");
            let backup_dir = tempfile::tempdir().expect("backup temp");
            let today = date(2026, 11, 2);
            let source_path = source.path().join(DATABASE_FILE_NAME);
            let source_connection = open_database(&source_path, today).expect("source should open");
            source_connection
                .execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-11-02', 600, 'manual', 'selected source',
                        '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                     )",
                    [],
                )
                .unwrap();
            let source_backup =
                backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                    .expect("source backup");
            drop(source_connection);
            let data_dir = target.path().join("app-data");
            let state = state_with_session(&data_dir, today, "original database");
            inject_fault(fault);
            let result =
                state.restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today);
            clear_fault();
            let notes: String = state
                .with_connection(|connection| {
                    connection
                        .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                        .map_err(|error| AppError::database("notes should load", error))
                })
                .expect("restored database should be usable");
            assert_eq!(notes, "selected source");
            if fault == "stage_opened" {
                assert!(result.is_err());
            } else {
                assert!(result.expect("restore should succeed").warning.is_some());
            }
            let operation = backup::read_restore_operation(&data_dir)
                .expect("recovery metadata should be preserved");
            if fault == "stage_opened" {
                assert!(PathBuf::from(&operation.rollback_path).is_file());
            } else {
                assert!(!PathBuf::from(&operation.rollback_path).exists());
            }
            assert!(PathBuf::from(&operation.safety_backup_path).is_file());
        }
    }

    #[test]
    fn reopen_failure_after_live_rename_does_not_create_an_empty_database() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        let source_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup");
        drop(source_connection);
        let data_dir = target.path().join("app-data");
        let state = state_with_session(&data_dir, today, "original database");
        inject_fault("live_rename,reopen_database");
        let error = state
            .restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today)
            .expect_err("live rename should fail");
        clear_fault();
        assert!(error.message.contains("Recovery files were preserved"));
        assert!(database_path(&data_dir).is_file());
        assert!(backup::read_restore_operation(&data_dir).is_some());
        assert!(state.with_connection(|_| Ok(())).is_err());
    }

    #[test]
    fn sidecar_update_failure_preserves_recovery_files() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 11, 2);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        let source_backup =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup");
        drop(source_connection);
        let data_dir = target.path().join("app-data");
        let state = state_with_session(&data_dir, today, "original database");
        inject_fault("stage_live_renamed");
        let error = state
            .restore_from_backup(&backup_dir.path().join(&source_backup.file_name), today)
            .expect_err("stage failure should abort");
        clear_fault();
        assert!(error.message.contains("stage could not be recorded"));
        assert!(backup::read_restore_operation(&data_dir).is_some());
        let operation = backup::read_restore_operation(&data_dir).unwrap();
        assert!(PathBuf::from(&operation.safety_backup_path).is_file());
        let notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("notes should load", error))
            })
            .expect("recovered original should be usable");
        assert_eq!(notes, "original database");
    }

    #[test]
    fn restoring_the_live_database_is_rejected_before_any_sidecar_cleanup() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 2);
        let state = state_with_session(&data_dir, today, "live database");
        let live_path = database_path(&data_dir);
        let wal = data_dir.join(format!("{DATABASE_FILE_NAME}-wal"));
        let shm = data_dir.join(format!("{DATABASE_FILE_NAME}-shm"));
        let wal_existed = wal.is_file();
        let shm_existed = shm.is_file();
        assert!(wal_existed, "live WAL sidecar should exist after a write");
        let error = state
            .restore_from_backup(&live_path, today)
            .expect_err("live database should be rejected");
        assert!(error.message.contains("not the live application database"));
        assert_eq!(wal.is_file(), wal_existed);
        assert_eq!(shm.is_file(), shm_existed);
    }

    #[test]
    fn restore_replaces_state_and_keeps_a_verified_safety_backup() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 10, 30);
        let source_path = source.path().join(DATABASE_FILE_NAME);
        let source_connection = open_database(&source_path, today).expect("source should open");
        source_connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-10-30', 3600, 'manual', 'source database', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'
                 )",
                [],
            )
            .expect("source session should insert");
        let backup_result =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup should succeed");
        drop(source_connection);

        let target_dir = target.path().join("app-data");
        let state = AppState::initialize(&target_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-10-30', 600, 'manual', 'target database', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'
                     )",
                    [],
                    ).map_err(|error| AppError::database("target session should insert", error))?;
                Ok(())
            })
            .expect("target session should save");

        let restored = state
            .restore_from_backup(&backup_dir.path().join(&backup_result.file_name), today)
            .expect("restore should succeed");
        assert!(
            target_dir
                .join("backups")
                .join("safety")
                .join(&restored.file_name)
                .is_file()
        );
        let notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("restored session should load", error))
            })
            .expect("restored state should query");
        assert_eq!(notes, "source database");
        assert_eq!(
            backup::schema_version(
                state
                    .database
                    .lock()
                    .unwrap()
                    .as_ref()
                    .expect("restored connection should exist")
            )
            .unwrap(),
            12
        );
    }

    #[test]
    fn safety_backup_failure_prevents_restore_and_preserves_current_data() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let backup_dir = tempfile::tempdir().expect("backup temp");
        let today = date(2026, 10, 30);
        let source_connection = open_database(&source.path().join(DATABASE_FILE_NAME), today)
            .expect("source should open");
        let backup_result =
            backup::create_backup(&source_connection, backup_dir.path(), "manual", None)
                .expect("source backup should succeed");
        drop(source_connection);

        let target_dir = target.path().join("app-data");
        let state = AppState::initialize(&target_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-10-30', 600, 'manual', 'must remain', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'
                     )",
                    [],
                    ).map_err(|error| AppError::database("target session should insert", error))?;
                Ok(())
            })
            .expect("target session should save");
        fs::create_dir_all(target_dir.join("backups")).unwrap();
        fs::write(target_dir.join("backups").join("safety"), b"blocking file").unwrap();

        let error = state
            .restore_from_backup(&backup_dir.path().join(&backup_result.file_name), today)
            .expect_err("restore should abort");
        assert_eq!(error.code, "backup_failed");
        let notes: String = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("current session should load", error))
            })
            .expect("current data should still load");
        assert_eq!(notes, "must remain");
    }

    #[test]
    fn failed_restore_can_recover_from_the_verified_safety_snapshot() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 10, 30);
        let state = AppState::initialize(&data_dir, today);
        state
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO study_sessions (
                        topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                     ) VALUES (
                        NULL, '2026-10-30', 1200, 'manual', 'recover me', '2026-10-30T00:00:00Z', '2026-10-30T00:00:00Z'
                     )",
                    [],
                    ).map_err(|error| AppError::database("session should insert", error))?;
                Ok(())
            })
            .expect("session should save");
        let safety = state
            .with_connection(|connection| {
                backup::create_safety_backup(connection, &data_dir, "pre-restore", None)
            })
            .expect("safety backup should succeed");
        let safety_path = data_dir
            .join("backups")
            .join("safety")
            .join(&safety.file_name);
        let current_path = database_path(&data_dir);
        fs::write(&current_path, b"not a database").expect("corruption should write");
        let recovered = recover_from_safety_snapshot(&safety_path, &current_path, today)
            .expect("safety snapshot should recover");
        let notes: String = recovered
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .expect("recovered session should load");
        assert_eq!(notes, "recover me");
    }

    #[test]
    fn normal_v1_startup_write_behavior_is_explicit() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let data_dir = directory.path().join("app-data");
        let today = date(2026, 11, 5);
        let state = AppState::initialize(&data_dir, today);
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE syllabus_sources
                         SET updated_at = '2000-01-01T00:00:00.000Z'",
                        [],
                    )
                    .map_err(|error| {
                        AppError::database("The V1 seed timestamp could not be reset", error)
                    })?;
                Ok(())
            })
            .expect("V1 seed timestamp reset");
        drop(state);

        let reopened = AppState::initialize(&data_dir, today);
        let refreshed: i64 = reopened
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM syllabus_sources
                         WHERE updated_at <> '2000-01-01T00:00:00.000Z'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| {
                        AppError::database("The V1 seed timestamp could not be read", error)
                    })
            })
            .expect("V1 seed timestamp read");
        assert!(
            refreshed > 0,
            "V1 startup currently refreshes seeded syllabus metadata and writes the V1 database"
        );
    }
}
