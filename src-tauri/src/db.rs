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
    M::up(include_str!("../migrations/0004_checklist_and_planner.sql")),
    M::up(include_str!("../migrations/0005_pyq_and_mock_tests.sql")),
    M::up(include_str!("../migrations/0006_revision_scheduler.sql")),
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
    use crate::{
        mock_tests, planner, pyq, sessions,
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
    }

    #[test]
    fn migrations_are_versioned_and_repeatable() {
        let mut connection = test_connection();

        assert_eq!(schema_version(&connection), 6);
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
                       'revision_history'
                   )",
                [],
                |row| row.get(0),
            )
            .expect("migration tables should be inspectable");
        assert_eq!(table_count, 15);

        migrate(&mut connection).expect("a repeated migration should be a no-op");
        assert_eq!(schema_version(&connection), 6);
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

        assert_eq!(schema_version(&connection), 6);
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

        assert_eq!(schema_version(&connection), 6);
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

        assert_eq!(schema_version(&connection), 6);
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

        assert_eq!(schema_version(&connection), 6);
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
