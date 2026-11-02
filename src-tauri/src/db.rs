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
pub const LATEST_SCHEMA_VERSION: i64 = 11;
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
                    database: Mutex::new(Err(AppError::database_unavailable(
                        "The local database could not be initialized. Check that your Windows \
                         application-data folder is writable, then restart the app.",
                    ))),
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
        let data_dir = self.data_dir()?;
        let current_path = self
            .database_path
            .clone()
            .ok_or_else(AppError::database_unavailable_message)?;
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The local database is in a consistent state lock. Restart the app and try again.",
            )
        })?;
        let connection = guard.as_mut().map_err(|error| error.clone())?;
        let schema_version = backup::schema_version(connection)?;
        let verified = backup::verify_backup(backup_path)?;
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
        let backup_path = backup_path
            .canonicalize()
            .map_err(|_| AppError::restore_failed("The selected backup file is unavailable."))?;
        if current_path
            .canonicalize()
            .ok()
            .is_some_and(|current| current == backup_path)
        {
            return Err(AppError::restore_failed(
                "Select a backup file, not the live application database.",
            ));
        }

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
        if let Err(error) = fs::copy(&backup_path, &temp_path) {
            eprintln!("Restore candidate could not be copied: {error}");
            let _ = fs::remove_file(&temp_path);
            return Err(AppError::restore_failed(
                "The selected backup could not be copied into the application-data folder.",
            ));
        }
        if let Err(error) = backup::verify_database_file(&temp_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }

        let _ = connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        let previous = std::mem::replace(
            &mut *guard,
            Err(AppError::internal("The database is being restored.")),
        );
        drop(previous);

        if remove_database_sidecars(&current_path).is_err() {
            let _ = fs::remove_file(&temp_path);
            *guard = reopen_current_database(&current_path, today);
            return Err(AppError::restore_failed(
                "The database WAL files could not be prepared for restore. The original database was reopened.",
            ));
        }
        if let Err(error) = fs::rename(&current_path, &rollback_path) {
            eprintln!("Live database could not be moved for restore: {error}");
            let _ = fs::remove_file(&temp_path);
            *guard = reopen_current_database(&current_path, today);
            return Err(AppError::restore_failed(
                "The live database could not be moved aside. The original database was reopened.",
            ));
        }
        if let Err(error) = fs::rename(&temp_path, &current_path) {
            eprintln!("Restore candidate could not be installed: {error}");
            let _ = fs::rename(&rollback_path, &current_path);
            *guard = reopen_current_database(&current_path, today);
            return Err(AppError::restore_failed(
                "The backup could not be installed. The original database was reopened.",
            ));
        }

        match open_database(&current_path, today) {
            Ok(restored) => {
                *guard = Ok(restored);
                let _ = fs::remove_file(&rollback_path);
                Ok(safety)
            }
            Err(error) => {
                eprintln!("Restored database could not be opened: {error:?}");
                let recovered = recover_from_safety_snapshot(&safety_path, &current_path, today);
                if recovered.is_ok() {
                    let _ = fs::remove_file(&rollback_path);
                }
                *guard = recovered;
                Err(AppError::restore_failed(
                    "The backup could not be opened after replacement. The original database was recovered from the verified safety snapshot.",
                ))
            }
        }
    }

    fn try_initialize(data_dir: &Path, today: NaiveDate) -> Result<Connection, AppError> {
        fs::create_dir_all(data_dir).map_err(|error| {
            AppError::database("The application-data folder could not be created", error)
        })?;

        open_database(&database_path(data_dir), today)
    }
}

impl AppError {
    fn database_unavailable_message() -> Self {
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

fn reopen_current_database(path: &Path, today: NaiveDate) -> Result<Connection, AppError> {
    open_database(path, today).map_err(|error| {
        eprintln!("The original database could not be reopened: {error:?}");
        AppError::database_unavailable(
            "The original database could not be reopened. Restart the app; if it still fails, use the verified safety backup.",
        )
    })
}

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
    let current_database_id = backup::database_identifier(&connection)?;
    let mut pre_migration_backup = None;
    let mut pre_migration_database_id = None;

    if existing_version > 0 && existing_version < LATEST_SCHEMA_VERSION {
        let database_id = current_database_id.unwrap_or_else(backup::new_database_id);
        let backup_directory = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        let result = backup::create_backup(
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
        pre_migration_backup = Some(result);
        pre_migration_database_id = Some(database_id);
    }

    migrate(&mut connection)?;
    backup::ensure_database_identifier(&connection, pre_migration_database_id.as_deref())?;
    if let Some(result) = &pre_migration_backup {
        backup::record_success(&connection, &result.manifest)?;
    }
    settings::ensure_defaults(&mut connection, today)?;
    syllabus::seed(&mut connection)?;
    timer::ensure_defaults(&mut connection)?;
    timer::recover_after_restart(&mut connection, Utc::now())?;

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

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn schema_version(connection: &Connection) -> i64 {
        connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version should be readable")
    }

    #[test]
    fn migration_definitions_are_valid() {
        MIGRATIONS.validate().expect("migrations should be valid");
        assert_eq!(LATEST_SCHEMA_VERSION, MIGRATIONS_SLICE.len() as i64);
    }

    #[test]
    fn migrations_are_versioned_and_repeatable() {
        let mut connection = test_connection();

        assert_eq!(schema_version(&connection), 11);
        let table_count: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master
                 WHERE type = 'table'
                   AND name IN (
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
        assert_eq!(table_count, 23);

        migrate(&mut connection).expect("a repeated migration should be a no-op");
        assert_eq!(schema_version(&connection), 11);
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

        assert_eq!(schema_version(&connection), 11);
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

        assert_eq!(schema_version(&connection), 11);
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

        assert_eq!(schema_version(&connection), 11);
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

        assert_eq!(schema_version(&connection), 11);
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
        assert_eq!(schema_version(&connection), 11);

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
        assert_eq!(schema_version(&connection), 11);

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
        assert_eq!(schema_version(&connection), 11);
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
        assert_eq!(schema_version(&connection), 11);
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
            11
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
}
