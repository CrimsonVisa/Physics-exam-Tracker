use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use chrono::{NaiveDate, Utc};
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::{error::AppError, settings, syllabus, timer};

pub const DATABASE_FILE_NAME: &str = "csir-net-tracker.sqlite3";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

const MIGRATIONS_SLICE: &[M<'static>] = &[
    M::up(include_str!("../migrations/0001_settings.sql")),
    M::up(include_str!("../migrations/0002_syllabus.sql")),
    M::up(include_str!(
        "../migrations/0003_study_timer_and_sessions.sql"
    )),
];
const MIGRATIONS: Migrations<'static> = Migrations::from_slice(MIGRATIONS_SLICE);

/// Tauri-managed application state. Database initialization failures are kept
/// as an error state so the window can still open and explain the problem.
pub struct AppState {
    database: Result<Mutex<Connection>, AppError>,
}

impl AppState {
    pub fn initialize(data_dir: &Path, today: NaiveDate) -> Self {
        match Self::try_initialize(data_dir, today) {
            Ok(connection) => Self {
                database: Ok(Mutex::new(connection)),
            },
            Err(error) => {
                eprintln!("Local database initialization failed: {error:?}");
                Self {
                    database: Err(AppError::database_unavailable(
                        "The local database could not be initialized. Check that your Windows \
                         application-data folder is writable, then restart the app.",
                    )),
                }
            }
        }
    }

    pub fn unavailable(error: AppError) -> Self {
        Self {
            database: Err(error),
        }
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let database = self.database.as_ref().map_err(|error| error.clone())?;
        let mut connection = database.lock().map_err(|_| {
            AppError::internal(
                "The local database is in an inconsistent state. Restart the app and try again.",
            )
        })?;

        operation(&mut connection)
    }

    fn try_initialize(data_dir: &Path, today: NaiveDate) -> Result<Connection, AppError> {
        fs::create_dir_all(data_dir).map_err(|error| {
            AppError::database("The application-data folder could not be created", error)
        })?;

        open_database(&database_path(data_dir), today)
    }
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
    migrate(&mut connection)?;
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
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    configure(&connection).expect("test database should be configured");
    migrate(&mut connection).expect("test database should be migrated");
    connection
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
    use crate::settings::{Settings, SettingsInput};

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
    }

    #[test]
    fn migrations_are_versioned_and_repeatable() {
        let mut connection = test_connection();

        assert_eq!(schema_version(&connection), 3);
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
                       'study_sessions'
                   )",
                [],
                |row| row.get(0),
            )
            .expect("migration tables should be inspectable");
        assert_eq!(table_count, 8);

        migrate(&mut connection).expect("a repeated migration should be a no-op");
        assert_eq!(schema_version(&connection), 3);
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
            )
            .expect("version one settings should be saved");
        }

        let connection =
            open_database(&path, date(2026, 10, 29)).expect("database should upgrade and open");

        assert_eq!(schema_version(&connection), 3);
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
        assert_eq!(session_count, 0);
        assert_eq!(timer_state, ("idle".to_owned(), "focus".to_owned(), 0));
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
}
