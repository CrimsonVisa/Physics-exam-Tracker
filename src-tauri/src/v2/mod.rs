use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{app_paths::V2_APP_IDENTIFIER, error::AppError};

pub mod admin;
pub mod analytics;
pub mod analytics_charts;
pub mod backup;
pub mod builtin_constants;
mod commands;
pub mod concepts;
pub mod dashboard;
pub mod flashcards;
pub mod mastery;
pub mod planner;
pub mod practice;
pub mod quick_start;
pub mod reference;
pub mod resources;
pub mod revision;
pub mod search;
pub mod sticky_notes;
pub mod study;
pub mod syllabus;

pub use commands::{
    v2_acquire_pyq_source, v2_apply_exam_date_change, v2_apply_pyq_import, v2_apply_restore_backup,
    v2_archive_mock_test, v2_archive_pyq_question, v2_archive_resource, v2_check_integrity,
    v2_complete_revision, v2_create_manual_backup, v2_create_manual_session, v2_defer_revision,
    v2_delete_block, v2_delete_custom_constant, v2_delete_custom_formula, v2_delete_custom_unit,
    v2_delete_quick_start_config, v2_delete_sticky_note, v2_delete_study_session, v2_delete_task,
    v2_discard_timer, v2_extract_pyq_source, v2_finish_timer, v2_generate_revision_daily_plan,
    v2_get_agenda, v2_get_analytics, v2_get_analytics_charts, v2_get_app_settings,
    v2_get_backup_overview, v2_get_builtin_constants, v2_get_concept_detail,
    v2_get_dashboard_sections, v2_get_database_status, v2_get_exam_syllabus,
    v2_get_flashcard_detail, v2_get_flashcard_review_queue, v2_get_flashcard_stats,
    v2_get_planner_day, v2_get_practice_analytics, v2_get_practice_options, v2_get_pyq_answer_key,
    v2_get_pyq_extraction_candidate, v2_get_pyq_question_detail, v2_get_pyq_source_counts,
    v2_get_pyq_source_detail, v2_get_question_mock_session, v2_get_restore_recovery,
    v2_get_revision_daily_plan, v2_get_revision_dashboard, v2_get_study_totals, v2_get_timer,
    v2_global_search, v2_heartbeat_timer, v2_import_resource_file, v2_initialize_database,
    v2_link_block_session, v2_link_resource_exam, v2_link_revision_plan_block,
    v2_list_custom_constants, v2_list_custom_formulas, v2_list_custom_units, v2_list_exam_profiles,
    v2_list_exams, v2_list_flashcards, v2_list_mock_tests, v2_list_pyq_answer_key_conflicts,
    v2_list_pyq_answer_keys, v2_list_pyq_attempts, v2_list_pyq_extraction_candidates,
    v2_list_pyq_questions, v2_list_pyq_sources, v2_list_question_mock_candidates,
    v2_list_quick_start_configs, v2_list_resources, v2_list_sessions, v2_list_sticky_notes,
    v2_move_revision_plan_item, v2_pause_timer, v2_preview_exam_date_change,
    v2_preview_pyq_answer_key, v2_preview_pyq_import, v2_preview_restore_backup,
    v2_rebalance_revision_daily_plan, v2_reconcile_pyq_source, v2_record_pyq_attempt,
    v2_reject_pyq_extraction_candidate, v2_remove_resource_file, v2_replay_concept_mastery,
    v2_reschedule_revision, v2_reschedule_task, v2_reset_dashboard_sections,
    v2_restore_recovery_safety, v2_resume_timer, v2_review_flashcard,
    v2_run_automatic_backup_if_due, v2_save_block, v2_save_custom_constant, v2_save_custom_formula,
    v2_save_custom_unit, v2_save_dashboard_sections, v2_save_flashcard, v2_save_mock_test,
    v2_save_pyq_answer_key, v2_save_pyq_question, v2_save_pyq_source, v2_save_question_mock_state,
    v2_save_quick_start_config, v2_save_resource, v2_save_sticky_note, v2_save_task,
    v2_schedule_revision_concept, v2_search_concepts, v2_set_block_status, v2_set_exam_active,
    v2_set_flashcard_archived, v2_set_mock_test_status, v2_set_pyq_source_status,
    v2_set_revision_status, v2_set_session_allocations, v2_set_task_status,
    v2_set_timer_allocations, v2_start_question_mock, v2_start_timer, v2_submit_question_mock,
    v2_update_app_settings, v2_update_backup_settings, v2_update_concept_mapping,
    v2_update_concept_mastery, v2_update_pyq_extraction_candidate, v2_update_resource_progress,
    v2_update_revision_settings, v2_update_study_session, v2_update_syllabus_node_progress,
    v2_update_timer_settings, v2_verify_mastery_run, v2_verify_pyq_extraction_candidate,
};

pub const V2_DATABASE_FILE_NAME: &str = "physics-tracker.sqlite3";
pub const V2_SCHEMA_VERSION: i64 = 5;
pub(crate) const SCHEMA_VERSION_3: i64 = 3;
pub(crate) const SCHEMA_VERSION_4: i64 = 4;
// Frozen checksum of the schema.sql payload shipped with schema version 3.
// Historical databases and backups carry this checksum in their append-only
// migration ledger, so it must never be regenerated from the current schema.
pub(crate) const SCHEMA_3_CHECKSUM: &str =
    "b3f05cdb342a9b3d065a51ab144c0c40f91b9d3078793dae57a9f69732b5d29b";
// Frozen checksum of the schema.sql payload shipped with schema version 4.
pub(crate) const SCHEMA_4_CHECKSUM: &str =
    "cbea905c3040e9d2bfdc4fe72c7fe92e42201cba5df90db4840baac2d5a6050a";

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SCHEMA_SQL: &str = include_str!("schema.sql");
const SEED_SQL: &str = include_str!("seed.sql");
const UPGRADE_SQL_V2_2: &str = include_str!("schema_upgrade_v2_2.sql");
const UPGRADE_SQL_V2_3: &str = include_str!("schema_upgrade_v2_3.sql");
const UPGRADE_SQL_V2_4: &str = include_str!("schema_upgrade_v2_4.sql");
const UPGRADE_SQL_V2_5: &str = include_str!("schema_upgrade_v2_5.sql");
const SCHEMA_VERSION_2: i64 = 2;
// Generated from the schema.sql shipped with the initial V2 baseline.
pub(crate) const SCHEMA_1_CHECKSUM: &str =
    "6a60abbde82d0b6e963ddf5086b7a4f43c9f5acf111f54763714727a27d00ab8";
// Generated from the schema.sql shipped with the QoL-3 baseline. It is kept
// stable so schema-2 databases and backups can be upgraded without rewriting
// their historical migration ledger.
pub(crate) const SCHEMA_2_CHECKSUM: &str =
    "7a2c00bdca6741f4219b5889633615c02391888705294ea3bb83d8db4a360463";

struct BuiltInExam {
    code: &'static str,
    name: &'static str,
    short_name: &'static str,
    authority: &'static str,
    official_url: &'static str,
    display_order: i64,
}

const BUILT_IN_EXAMS: [BuiltInExam; 4] = [
    BuiltInExam {
        code: "csir-net-physics",
        name: "CSIR-UGC NET Physical Sciences",
        short_name: "CSIR-NET",
        authority: "CSIR-HRDG / NTA",
        official_url: "https://csirnet.nta.nic.in/",
        display_order: 1,
    },
    BuiltInExam {
        code: "gate-physics",
        name: "GATE Physics (PH)",
        short_name: "GATE PH",
        authority: "IIT",
        official_url: "https://gate.iitm.ac.in/",
        display_order: 2,
    },
    BuiltInExam {
        code: "tifr-gs-physics",
        name: "TIFR Graduate School Physics",
        short_name: "TIFR GS",
        authority: "TIFR",
        official_url: "https://www.tifr.res.in/academics/",
        display_order: 3,
    },
    BuiltInExam {
        code: "jest-physics",
        name: "JEST Physics",
        short_name: "JEST",
        authority: "JEST Consortium",
        official_url: "https://www.jest.org.in/",
        display_order: 4,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub short_name: String,
    pub authority: String,
    pub official_url: String,
    pub display_order: i64,
    pub is_active: bool,
    pub exam_date: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum V2DatabaseState {
    NotInitialized,
    Healthy,
    UnsupportedSchema,
    MetadataMismatch,
    IntegrityFailure,
    ForeignKeyFailure,
    Inaccessible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DatabaseStatus {
    pub state: V2DatabaseState,
    pub app_identifier: String,
    pub data_dir: Option<String>,
    pub database_path: Option<String>,
    pub schema_version: Option<i64>,
    pub expected_schema_version: i64,
    pub metadata_schema_version: Option<i64>,
    pub database_id_valid: bool,
    pub migration_checksum_valid: bool,
    pub initialized: bool,
    pub foreign_keys_enabled: bool,
    pub journal_mode: Option<String>,
    pub integrity_check: Option<String>,
    pub foreign_key_violations: Option<i64>,
    pub exam_count: i64,
    pub active_exam_count: i64,
    pub exams: Vec<V2ExamSummary>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2IntegrityReport {
    pub ok: bool,
    pub integrity_check: String,
    pub foreign_key_violations: i64,
    pub schema_version: i64,
    pub exam_count: i64,
    pub issues: Vec<String>,
}

/// Tauri-managed V2 state. V2 is intentionally independent from the V1
/// AppState so the existing V1 application and database remain untouched.
pub struct V2State {
    database: Mutex<Result<Connection, AppError>>,
    data_dir: Option<PathBuf>,
    database_path: Option<PathBuf>,
}

impl V2State {
    pub fn initialize(data_dir: &Path) -> Self {
        let database_path = data_dir.join(V2_DATABASE_FILE_NAME);
        if let Err(error) = fs::create_dir_all(data_dir) {
            return Self {
                database: Mutex::new(Err(AppError::v2_file_operation(
                    "The V2 application-data folder could not be created",
                    error,
                ))),
                data_dir: Some(data_dir.to_path_buf()),
                database_path: Some(database_path),
            };
        }

        if let Err(error) = backup::recover_interrupted_restore(data_dir) {
            eprintln!("V2 interrupted restore recovery failed: {error:?}");
            return Self {
                database: Mutex::new(Err(error)),
                data_dir: Some(data_dir.to_path_buf()),
                database_path: Some(database_path),
            };
        }

        match open_or_initialize(&database_path) {
            Ok(mut connection) => match prepare_connection(&mut connection) {
                Ok(()) => Self {
                    database: Mutex::new(Ok(connection)),
                    data_dir: Some(data_dir.to_path_buf()),
                    database_path: Some(database_path),
                },
                Err(error) => {
                    eprintln!("V2 database preparation failed: {error:?}");
                    Self {
                        database: Mutex::new(Err(error)),
                        data_dir: Some(data_dir.to_path_buf()),
                        database_path: Some(database_path),
                    }
                }
            },
            Err(error) => {
                eprintln!("V2 database initialization failed: {error:?}");
                Self {
                    database: Mutex::new(Err(error)),
                    data_dir: Some(data_dir.to_path_buf()),
                    database_path: Some(database_path),
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

    pub fn retry_initialize(&self) -> Result<(), AppError> {
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The V2 database is in an inconsistent lock state. Restart the app and try again.",
            )
        })?;
        if guard.is_ok() {
            return Ok(());
        }
        let path = self.database_path.as_deref().ok_or_else(|| {
            AppError::database_unavailable("The V2 database path is unavailable.")
        })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError::v2_file_operation(
                    "The V2 application-data folder could not be created",
                    error,
                )
            })?;
        }
        backup::recover_interrupted_restore(path.parent().ok_or_else(|| {
            AppError::database_unavailable("The V2 database location is invalid.")
        })?)?;
        let mut connection = open_or_initialize(path)?;
        prepare_connection(&mut connection)?;
        *guard = Ok(connection);
        Ok(())
    }

    pub fn restore_backup(
        &self,
        input: backup::V2RestoreInput,
        now: chrono::DateTime<Utc>,
    ) -> Result<backup::V2RestoreResult, AppError> {
        let data_dir = self.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        let database_path = self.database_path().ok_or_else(|| {
            AppError::database_unavailable("The V2 database path is unavailable.")
        })?;
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The V2 database is in an inconsistent lock state. Restart the app and try again.",
            )
        })?;
        let current = std::mem::replace(
            &mut *guard,
            Err(AppError::database_unavailable(
                "The V2 database is being restored.",
            )),
        );
        let connection = match current {
            Ok(connection) => connection,
            Err(error) => {
                *guard = Err(error.clone());
                return Err(error);
            }
        };
        drop(connection);
        let result = backup::apply_restore(&data_dir, &database_path, input, now);
        if let Err(error) = &result {
            eprintln!("V2 restore failed: {error:?}");
        }
        if let Err(error) = backup::recover_interrupted_restore(&data_dir) {
            eprintln!("V2 restore recovery after failure failed: {error:?}");
            *guard = Err(error.clone());
            return Err(error);
        }
        let reopened = open_or_initialize(&database_path).and_then(|mut connection| {
            prepare_connection(&mut connection)?;
            Ok(connection)
        });
        match reopened {
            Ok(connection) => {
                *guard = Ok(connection);
                result
            }
            Err(error) => {
                eprintln!("The restored V2 database could not be reopened: {error:?}");
                *guard = Err(error.clone());
                if result.is_ok() { Err(error) } else { result }
            }
        }
    }

    pub fn restore_recovery_safety(
        &self,
        input: backup::V2RestoreInput,
        now: chrono::DateTime<Utc>,
    ) -> Result<backup::V2RestoreResult, AppError> {
        let data_dir = self.data_directory().ok_or_else(|| {
            AppError::database_unavailable("The V2 data directory is unavailable.")
        })?;
        let path = Path::new(input.path.trim());
        if !path.starts_with(backup::safety_backup_directory(&data_dir)) {
            return Err(AppError::validation(
                "path",
                "Choose a safety backup from the V2 recovery folder.",
            ));
        }
        self.restore_backup(input, now)
    }

    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut guard = self.database.lock().map_err(|_| {
            AppError::internal(
                "The V2 database is in an inconsistent lock state. Restart the app and try again.",
            )
        })?;
        let connection = guard.as_mut().map_err(|error| error.clone())?;
        operation(connection)
    }

    pub fn data_directory(&self) -> Option<PathBuf> {
        self.data_dir.clone()
    }

    pub fn database_path(&self) -> Option<PathBuf> {
        self.database_path.clone()
    }

    pub fn status(&self) -> V2DatabaseStatus {
        let mut status = V2DatabaseStatus {
            state: V2DatabaseState::NotInitialized,
            app_identifier: V2_APP_IDENTIFIER.to_owned(),
            data_dir: self
                .data_dir
                .as_ref()
                .map(|path| path.display().to_string()),
            database_path: self
                .database_path
                .as_ref()
                .map(|path| path.display().to_string()),
            schema_version: None,
            expected_schema_version: V2_SCHEMA_VERSION,
            metadata_schema_version: None,
            database_id_valid: false,
            migration_checksum_valid: false,
            initialized: false,
            foreign_keys_enabled: false,
            journal_mode: None,
            integrity_check: None,
            foreign_key_violations: None,
            exam_count: 0,
            active_exam_count: 0,
            exams: Vec::new(),
            error_code: None,
            error_message: None,
        };

        let guard = match self.database.lock() {
            Ok(guard) => guard,
            Err(_) => {
                set_status_error(
                    &mut status,
                    V2DatabaseState::Inaccessible,
                    "internal_error",
                    "The V2 database lock is inconsistent. Restart the app and try again.",
                );
                return status;
            }
        };

        let connection = match guard.as_ref() {
            Ok(connection) => connection,
            Err(error) => {
                let state = match error.code {
                    "unsupported_schema" => V2DatabaseState::UnsupportedSchema,
                    "schema_integrity_error" => V2DatabaseState::MetadataMismatch,
                    "integrity_error" => V2DatabaseState::IntegrityFailure,
                    "foreign_key_error" => V2DatabaseState::ForeignKeyFailure,
                    "database_not_initialized" => V2DatabaseState::NotInitialized,
                    _ => V2DatabaseState::Inaccessible,
                };
                set_status_error(&mut status, state, error.code, &error.message);
                return status;
            }
        };

        status.foreign_keys_enabled = foreign_keys_enabled(connection).unwrap_or(false);
        status.journal_mode = journal_mode(connection).ok();
        let version = match user_version(connection) {
            Ok(version) => version,
            Err(error) => {
                set_status_error(
                    &mut status,
                    V2DatabaseState::Inaccessible,
                    error.code,
                    &error.message,
                );
                return status;
            }
        };
        status.schema_version = Some(version);
        if version > V2_SCHEMA_VERSION {
            set_status_error(
                &mut status,
                V2DatabaseState::UnsupportedSchema,
                "unsupported_schema",
                &format!(
                    "V2 schema version {version} is newer than the supported version {V2_SCHEMA_VERSION}."
                ),
            );
            return status;
        }
        if version != V2_SCHEMA_VERSION {
            let state = if version == 0 {
                V2DatabaseState::NotInitialized
            } else {
                V2DatabaseState::MetadataMismatch
            };
            set_status_error(
                &mut status,
                state,
                if version == 0 {
                    "database_not_initialized"
                } else {
                    "schema_integrity_error"
                },
                &format!("V2 schema version {version} does not match {V2_SCHEMA_VERSION}."),
            );
            return status;
        }

        let metadata_version = metadata_schema_version(connection).ok().flatten();
        status.metadata_schema_version = metadata_version;
        let identity = database_identity(connection).ok().flatten();
        status.database_id_valid = valid_database_identity(identity.as_deref());
        let expected_checksum = initial_schema_checksum();
        status.migration_checksum_valid =
            stored_schema_checksum(connection).ok().flatten().as_deref()
                == Some(expected_checksum.as_str());

        let exams_result = exams(connection);
        let exams_ok = exams_result
            .as_ref()
            .map(|exams| {
                exams.len() == 4
                    && BUILT_IN_EXAMS
                        .iter()
                        .all(|expected| exams.iter().any(|exam| exam.code == expected.code))
            })
            .unwrap_or(false);
        if let Ok(exams) = exams_result {
            status.exam_count = exams.len() as i64;
            status.active_exam_count = exams.iter().filter(|exam| exam.is_active).count() as i64;
            status.exams = exams;
        }

        if metadata_version != Some(V2_SCHEMA_VERSION)
            || !status.database_id_valid
            || !status.migration_checksum_valid
            || !exams_ok
        {
            set_status_error(
                &mut status,
                V2DatabaseState::MetadataMismatch,
                "schema_integrity_error",
                "The V2 database metadata, identity, migration checksum, or exam seed does not match the expected schema.",
            );
            return status;
        }
        if !status.foreign_keys_enabled {
            set_status_error(
                &mut status,
                V2DatabaseState::MetadataMismatch,
                "foreign_key_error",
                "V2 foreign-key enforcement is disabled on the database connection.",
            );
            return status;
        }
        let integrity =
            connection.query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0));
        match integrity {
            Ok(integrity) => {
                status.integrity_check = Some(integrity.clone());
                if integrity != "ok" {
                    set_status_error(
                        &mut status,
                        V2DatabaseState::IntegrityFailure,
                        "integrity_error",
                        &format!("The V2 quick integrity check failed: {integrity}"),
                    );
                    return status;
                }
            }
            Err(error) => {
                set_status_error(
                    &mut status,
                    V2DatabaseState::Inaccessible,
                    "database_error",
                    &format!("The V2 quick integrity check could not run: {error}"),
                );
                return status;
            }
        }

        match foreign_key_violation_count(connection) {
            Ok(count) => {
                status.foreign_key_violations = Some(count);
                if count > 0 {
                    set_status_error(
                        &mut status,
                        V2DatabaseState::ForeignKeyFailure,
                        "foreign_key_error",
                        &format!("The V2 database has {count} foreign-key violation(s)."),
                    );
                    return status;
                }
            }
            Err(error) => {
                set_status_error(
                    &mut status,
                    V2DatabaseState::Inaccessible,
                    error.code,
                    &error.message,
                );
                return status;
            }
        }

        status.initialized = true;
        status.state = V2DatabaseState::Healthy;
        status
    }

    pub fn list_exams(&self) -> Result<Vec<V2ExamSummary>, AppError> {
        self.with_connection(|connection| exams(connection))
    }

    pub fn check_integrity(&self, full: bool) -> Result<V2IntegrityReport, AppError> {
        self.with_connection(|connection| integrity_report(connection, full))
    }
}

fn set_status_error(
    status: &mut V2DatabaseStatus,
    state: V2DatabaseState,
    code: &str,
    message: &str,
) {
    status.state = state;
    status.error_code = Some(code.to_owned());
    status.error_message = Some(message.to_owned());
}

fn prepare_connection(connection: &mut Connection) -> Result<(), AppError> {
    syllabus::prepare(connection)?;
    study::prepare(connection)?;
    revision::prepare(connection)?;
    concepts::prepare(connection)?;
    practice::prepare(connection)?;
    builtin_constants::prepare(connection)
}

fn open_or_initialize(path: &Path) -> Result<Connection, AppError> {
    if !path.exists() {
        move_orphan_sidecars(path)?;
        return create_database(path);
    }

    let mut connection = Connection::open(path)
        .map_err(|error| AppError::database("The V2 database could not be opened", error))?;
    configure(&connection)?;
    verify_features(&connection)?;

    let version = user_version(&connection)?;
    if version > V2_SCHEMA_VERSION {
        return Err(AppError::unsupported_schema(version, V2_SCHEMA_VERSION));
    }
    if version == V2_SCHEMA_VERSION {
        ensure_database_identity(&mut connection)?;
        repair_missing_exams(&mut connection)?;
        verify_connection(&connection, false)?;
        return Ok(connection);
    }
    if version == 0 {
        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("The V2 schema state could not be read", error))?;
        drop(connection);
        if table_count > 0 {
            move_incomplete_database_aside(path)?;
        } else {
            move_orphan_sidecars(path)?;
        }
        return create_database(path);
    }
    // Supported intermediate versions are 1 (initial V2 schema), 2
    // (quick-start configurations), 3 (flashcards) and 4 (custom references).
    // Upgrade in place so a schema-newer interstitial never appears for data
    // that this build can additively extend. Older databases pass through every
    // additive step.
    if version == 1 {
        upgrade_database_1_to_2(&mut connection)?;
        upgrade_database_2_to_3(&mut connection)?;
        upgrade_database_3_to_4(&mut connection)?;
        upgrade_database_4_to_5(&mut connection)?;
        ensure_database_identity(&mut connection)?;
        repair_missing_exams(&mut connection)?;
        verify_connection(&connection, false)?;
        return Ok(connection);
    }
    if version == 2 {
        upgrade_database_2_to_3(&mut connection)?;
        upgrade_database_3_to_4(&mut connection)?;
        upgrade_database_4_to_5(&mut connection)?;
        ensure_database_identity(&mut connection)?;
        repair_missing_exams(&mut connection)?;
        verify_connection(&connection, false)?;
        return Ok(connection);
    }
    if version == SCHEMA_VERSION_3 {
        upgrade_database_3_to_4(&mut connection)?;
        upgrade_database_4_to_5(&mut connection)?;
        ensure_database_identity(&mut connection)?;
        repair_missing_exams(&mut connection)?;
        verify_connection(&connection, false)?;
        return Ok(connection);
    }
    if version == SCHEMA_VERSION_4 {
        upgrade_database_4_to_5(&mut connection)?;
        ensure_database_identity(&mut connection)?;
        repair_missing_exams(&mut connection)?;
        verify_connection(&connection, false)?;
        return Ok(connection);
    }

    Err(AppError::unsupported_schema(version, V2_SCHEMA_VERSION))
}

/// Upgrade a version-1 V2 database to version 2. The quick-start tables are
/// additive; no existing row is touched. The ledger row is pinned to version 2
/// with the historical schema-2 checksum so later upgrades retain an accurate
/// migration trail.
pub(crate) fn upgrade_database_1_to_2(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::migration("The V2 schema upgrade could not start", error))?;
    transaction
        .execute_batch(UPGRADE_SQL_V2_2)
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade could not be applied", error)
        })?;
    let token = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    // Ledger rows are append-only by design (update/delete triggers are
    // blocked). If a previous upgrade attempt already recorded the row, its
    // checksum is verified instead of overwritten.
    transaction
        .execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, 'v2_quick_start_configs', ?2, ?3)",
            params![SCHEMA_VERSION_2, SCHEMA_2_CHECKSUM, token],
        )
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade ledger could not be written", error)
        })?;
    let recorded_checksum: Option<String> = transaction
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [SCHEMA_VERSION_2],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::migration("The V2 upgrade ledger could not be read", error))?;
    if recorded_checksum.as_deref() != Some(SCHEMA_2_CHECKSUM) {
        return Err(AppError::internal(
            "The V2 schema-2 upgrade ledger checksum does not match this application's schema.",
        ));
    }
    transaction
        .execute(
            "UPDATE app_metadata SET schema_version = ?1, updated_at = ?2 WHERE id = 1",
            params![SCHEMA_VERSION_2, token],
        )
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade metadata could not be updated", error)
        })?;
    transaction
        .execute_batch("PRAGMA user_version = 2;")
        .map_err(|error| {
            AppError::migration("The V2 schema version could not be recorded", error)
        })?;
    transaction.commit().map_err(|error| {
        AppError::migration("The V2 schema upgrade could not be committed", error)
    })?;
    Ok(())
}

/// Upgrade a version-2 V2 database to version 3 by adding flashcards.
/// The DDL is additive and idempotent; no existing row is touched. The ledger
/// row stays pinned to the historical version-3 checksum even after this build
/// advances the current schema to version 4.
pub(crate) fn upgrade_database_2_to_3(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::migration("The V2 schema upgrade could not start", error))?;
    transaction
        .execute_batch(UPGRADE_SQL_V2_3)
        .map_err(|error| {
            AppError::migration(
                "The V2 flashcard schema upgrade could not be applied",
                error,
            )
        })?;
    let token = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, 'v2_flashcards', ?2, ?3)",
            params![SCHEMA_VERSION_3, SCHEMA_3_CHECKSUM, token],
        )
        .map_err(|error| {
            AppError::migration(
                "The V2 flashcard upgrade ledger could not be written",
                error,
            )
        })?;
    let recorded_checksum: Option<String> = transaction
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [SCHEMA_VERSION_3],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::migration("The V2 upgrade ledger could not be read", error))?;
    if recorded_checksum.as_deref() != Some(SCHEMA_3_CHECKSUM) {
        return Err(AppError::internal(
            "The V2 schema-3 upgrade ledger checksum does not match the historical schema.",
        ));
    }
    transaction
        .execute(
            "UPDATE app_metadata SET schema_version = ?1, updated_at = ?2 WHERE id = 1",
            params![SCHEMA_VERSION_3, token],
        )
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade metadata could not be updated", error)
        })?;
    transaction
        .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION_3};"))
        .map_err(|error| {
            AppError::migration("The V2 schema version could not be recorded", error)
        })?;
    transaction.commit().map_err(|error| {
        AppError::migration("The V2 schema upgrade could not be committed", error)
    })?;
    Ok(())
}

/// Upgrade a version-3 V2 database to version 4 by adding the user-managed
/// formula, unit and constant tables. The DDL is additive and idempotent; no
/// existing row is touched.
pub(crate) fn upgrade_database_3_to_4(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::migration("The V2 schema upgrade could not start", error))?;
    transaction
        .execute_batch(UPGRADE_SQL_V2_4)
        .map_err(|error| {
            AppError::migration(
                "The V2 custom-reference schema upgrade could not be applied",
                error,
            )
        })?;
    let token = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, 'v2_custom_references', ?2, ?3)",
            params![SCHEMA_VERSION_4, SCHEMA_4_CHECKSUM, token],
        )
        .map_err(|error| {
            AppError::migration(
                "The V2 custom-reference upgrade ledger could not be written",
                error,
            )
        })?;
    let recorded_checksum: Option<String> = transaction
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [SCHEMA_VERSION_4],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::migration("The V2 upgrade ledger could not be read", error))?;
    if recorded_checksum.as_deref() != Some(SCHEMA_4_CHECKSUM) {
        return Err(AppError::internal(
            "The V2 schema-4 upgrade ledger checksum does not match the historical schema.",
        ));
    }
    transaction
        .execute(
            "UPDATE app_metadata SET schema_version = ?1, updated_at = ?2 WHERE id = 1",
            params![SCHEMA_VERSION_4, token],
        )
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade metadata could not be updated", error)
        })?;
    transaction
        .execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION_4};"))
        .map_err(|error| {
            AppError::migration("The V2 schema version could not be recorded", error)
        })?;
    transaction.commit().map_err(|error| {
        AppError::migration("The V2 schema upgrade could not be committed", error)
    })?;
    Ok(())
}

/// Upgrade a version-4 V2 database to version 5 by adding the immutable
/// built-in constants catalogue and persistent sticky notes. The DDL is
/// additive and idempotent; no existing row is touched.
pub(crate) fn upgrade_database_4_to_5(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::migration("The V2 schema upgrade could not start", error))?;
    transaction
        .execute_batch(UPGRADE_SQL_V2_5)
        .map_err(|error| {
            AppError::migration(
                "The V2 constants and sticky-notes schema upgrade could not be applied",
                error,
            )
        })?;
    let token = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name, checksum, applied_at)
             VALUES (?1, 'v2_constants_and_sticky_notes', ?2, ?3)",
            params![V2_SCHEMA_VERSION, initial_schema_checksum(), token],
        )
        .map_err(|error| {
            AppError::migration(
                "The V2 constants and sticky-notes upgrade ledger could not be written",
                error,
            )
        })?;
    let recorded_checksum: Option<String> = transaction
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [V2_SCHEMA_VERSION],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::migration("The V2 upgrade ledger could not be read", error))?;
    if recorded_checksum.as_deref() != Some(initial_schema_checksum().as_str()) {
        return Err(AppError::internal(
            "The V2 schema-5 upgrade ledger checksum does not match this application's schema.",
        ));
    }
    transaction
        .execute(
            "UPDATE app_metadata SET schema_version = ?1, updated_at = ?2 WHERE id = 1",
            params![V2_SCHEMA_VERSION, token],
        )
        .map_err(|error| {
            AppError::migration("The V2 schema upgrade metadata could not be updated", error)
        })?;
    transaction
        .execute_batch(&format!("PRAGMA user_version = {V2_SCHEMA_VERSION};"))
        .map_err(|error| {
            AppError::migration("The V2 schema version could not be recorded", error)
        })?;
    transaction.commit().map_err(|error| {
        AppError::migration("The V2 schema upgrade could not be committed", error)
    })?;
    Ok(())
}

/// Upgrade any supported existing V2 database file (versions 1 through 4) to
/// the current schema. Used by the restore candidate path, which always
/// operates on a disposable copy rather than the original backup.
pub(crate) fn upgrade_open_connection_to_current(
    connection: &mut Connection,
) -> Result<(), AppError> {
    let version = user_version(connection)?;
    match version {
        1 => {
            upgrade_database_1_to_2(connection)?;
            upgrade_database_2_to_3(connection)?;
            upgrade_database_3_to_4(connection)?;
            upgrade_database_4_to_5(connection)?;
        }
        2 => {
            upgrade_database_2_to_3(connection)?;
            upgrade_database_3_to_4(connection)?;
            upgrade_database_4_to_5(connection)?;
        }
        SCHEMA_VERSION_3 => {
            upgrade_database_3_to_4(connection)?;
            upgrade_database_4_to_5(connection)?;
        }
        SCHEMA_VERSION_4 => upgrade_database_4_to_5(connection)?,
        V2_SCHEMA_VERSION => {}
        other => return Err(AppError::unsupported_schema(other, V2_SCHEMA_VERSION)),
    }
    Ok(())
}

fn create_database(path: &Path) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::v2_file_operation(
                "The V2 application-data folder could not be created",
                error,
            )
        })?;
    }
    let mut connection = Connection::open(path)
        .map_err(|error| AppError::database("The V2 database could not be created", error))?;
    configure(&connection)?;
    verify_features(&connection)?;

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 initialization transaction could not start", error)
    })?;
    transaction
        .execute_batch(SCHEMA_SQL)
        .map_err(|error| AppError::migration("The V2 schema could not be created", error))?;
    let checksum = migration_checksum(SCHEMA_SQL);
    let seed_sql = SEED_SQL.replace(&"0".repeat(64), &checksum);
    transaction
        .execute_batch(&seed_sql)
        .map_err(|error| AppError::migration("The V2 seed could not be applied", error))?;
    transaction
        .execute_batch(&format!("PRAGMA user_version = {V2_SCHEMA_VERSION};"))
        .map_err(|error| {
            AppError::migration("The V2 schema version could not be recorded", error)
        })?;
    transaction.commit().map_err(|error| {
        AppError::migration("The V2 initialization could not be committed", error)
    })?;

    ensure_database_identity(&mut connection)?;
    repair_missing_exams(&mut connection)?;
    verify_connection(&connection, true)?;
    Ok(connection)
}

fn move_incomplete_database_aside(path: &Path) -> Result<(), AppError> {
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let token = Uuid::new_v4().simple();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(V2_DATABASE_FILE_NAME);
    let target = path.with_file_name(format!("{file_name}.incomplete-{stamp}-{token}"));
    fs::rename(path, &target).map_err(|error| {
        AppError::v2_file_operation(
            "The incomplete V2 database could not be moved aside for recovery",
            error,
        )
    })?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
        if sidecar.exists() {
            let target_sidecar = PathBuf::from(format!("{}{suffix}", target.display()));
            fs::rename(&sidecar, &target_sidecar).map_err(|error| {
                AppError::v2_file_operation(
                    "An incomplete V2 database sidecar could not be moved aside for recovery",
                    error,
                )
            })?;
        }
    }
    Ok(())
}

fn move_orphan_sidecars(path: &Path) -> Result<(), AppError> {
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let token = Uuid::new_v4().simple();
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
        if sidecar.exists() {
            let file_name = sidecar
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("physics-tracker.sidecar");
            let target = sidecar.with_file_name(format!("{file_name}.orphan-{stamp}-{token}"));
            fs::rename(&sidecar, &target).map_err(|error| {
                AppError::v2_file_operation(
                    "An orphaned V2 database sidecar could not be moved aside",
                    error,
                )
            })?;
        }
    }
    Ok(())
}

fn configure(connection: &Connection) -> Result<(), AppError> {
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| {
            AppError::database("V2 SQLite foreign keys could not be enabled", error)
        })?;
    connection.busy_timeout(BUSY_TIMEOUT).map_err(|error| {
        AppError::database("The V2 SQLite busy timeout could not be set", error)
    })?;
    let _journal_mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|error| AppError::database("V2 SQLite WAL mode could not be enabled", error))?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|error| {
            AppError::database("V2 SQLite synchronous mode could not be set", error)
        })?;
    Ok(())
}

fn verify_features(connection: &Connection) -> Result<(), AppError> {
    let version: String = connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|error| AppError::database("The V2 SQLite version could not be read", error))?;
    let mut parts = version.split('.');
    let major = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    if (major, minor) < (3, 37) {
        return Err(AppError::internal(format!(
            "SQLite {version} is too old for V2; version 3.37 or newer is required."
        )));
    }
    let json_ok: i64 = connection
        .query_row("SELECT json_valid('{}')", [], |row| row.get(0))
        .map_err(|error| AppError::database("SQLite JSON support could not be verified", error))?;
    if json_ok != 1 {
        return Err(AppError::internal(
            "The bundled SQLite build does not provide JSON1 support required by V2.",
        ));
    }
    if !foreign_keys_enabled(connection)? {
        return Err(AppError::internal(
            "V2 SQLite foreign keys are not enabled.",
        ));
    }
    Ok(())
}

/// Deterministic checksum for the canonical initial-schema payload.
/// The payload is the exact schema SQL with CRLF/CR normalized to LF.
pub(crate) fn migration_checksum(payload: &str) -> String {
    let normalized = payload.replace("\r\n", "\n").replace('\r', "\n");
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn initial_schema_checksum() -> String {
    migration_checksum(SCHEMA_SQL)
}

fn stored_schema_checksum(connection: &Connection) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT checksum FROM schema_migrations WHERE version=?1",
            [V2_SCHEMA_VERSION],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The V2 migration checksum could not be read", error))
}

fn metadata_schema_version(connection: &Connection) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT schema_version FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The V2 metadata schema version could not be read", error)
        })
}

fn database_identity(connection: &Connection) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT database_id FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The V2 database identity could not be read", error))
}

fn valid_database_identity(identity: Option<&str>) -> bool {
    let Some(identity) = identity else {
        return false;
    };
    identity.len() == 32
        && identity == identity.to_lowercase()
        && identity.bytes().all(|byte| byte.is_ascii_hexdigit())
        && identity != "0".repeat(32)
}

fn user_version(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| AppError::database("The V2 schema version could not be read", error))
}

fn foreign_keys_enabled(connection: &Connection) -> Result<bool, AppError> {
    let enabled: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|error| AppError::database("V2 foreign-key state could not be read", error))?;
    Ok(enabled == 1)
}

fn journal_mode(connection: &Connection) -> Result<String, AppError> {
    connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|error| AppError::database("V2 journal mode could not be read", error))
}

fn ensure_database_identity(connection: &mut Connection) -> Result<(), AppError> {
    let database_id: Option<String> = connection
        .query_row(
            "SELECT database_id FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The V2 database identity could not be read", error))?;
    let database_id = database_id
        .ok_or_else(|| AppError::schema_integrity("The V2 app_metadata row is missing."))?;
    if database_id == "0".repeat(32) {
        let new_id = Uuid::new_v4().simple().to_string();
        connection
            .execute(
                "UPDATE app_metadata SET database_id=?1, updated_at=?2 WHERE id=1",
                params![new_id, Utc::now().to_rfc3339()],
            )
            .map_err(|error| {
                AppError::database("The V2 database identity could not be assigned", error)
            })?;
    }
    Ok(())
}

fn repair_missing_exams(connection: &mut Connection) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 exam recovery transaction could not start", error)
    })?;
    for exam in BUILT_IN_EXAMS {
        transaction
            .execute(
                "INSERT INTO exams(code,name,short_name,authority,official_url,display_order,created_at,updated_at)
                 SELECT ?1,?2,?3,?4,?5,?6,?7,?7
                 WHERE NOT EXISTS (SELECT 1 FROM exams WHERE code=?1)",
                params![
                    exam.code,
                    exam.name,
                    exam.short_name,
                    exam.authority,
                    exam.official_url,
                    exam.display_order,
                    now
                ],
            )
            .map_err(|error| AppError::database("A missing V2 exam record could not be recovered", error))?;
        transaction
            .execute(
                "INSERT INTO exam_profiles(exam_id,is_active,exam_date,created_at,updated_at)
                 SELECT id,1,NULL,?2,?2 FROM exams
                 WHERE code=?1
                   AND NOT EXISTS (SELECT 1 FROM exam_profiles WHERE exam_id=exams.id)",
                params![exam.code, now],
            )
            .map_err(|error| {
                AppError::database("A missing V2 exam profile could not be recovered", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 exam recovery could not be committed", error))
}

fn exams(connection: &Connection) -> Result<Vec<V2ExamSummary>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.code, e.name, e.short_name, e.authority, e.official_url,
                    e.display_order, p.is_active, p.exam_date
             FROM exams e
             JOIN exam_profiles p ON p.exam_id=e.id
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("The V2 exam list could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(V2ExamSummary {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                short_name: row.get(3)?,
                authority: row.get(4)?,
                official_url: row.get(5)?,
                display_order: row.get(6)?,
                is_active: row.get::<_, i64>(7)? == 1,
                exam_date: row.get(8)?,
            })
        })
        .map_err(|error| AppError::database("The V2 exam list could not be read", error))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(
            row.map_err(|error| AppError::database("A V2 exam row could not be read", error))?,
        );
    }
    Ok(result)
}

fn verify_connection(connection: &Connection, full: bool) -> Result<(), AppError> {
    verify_features(connection)?;
    let version = user_version(connection)?;
    if version != V2_SCHEMA_VERSION {
        return Err(AppError::unsupported_schema(version, V2_SCHEMA_VERSION));
    }
    let metadata_version: i64 = connection
        .query_row(
            "SELECT schema_version FROM app_metadata WHERE id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::schema_integrity(format!(
                "The V2 metadata row is missing or invalid: {error}"
            ))
        })?;
    if metadata_version != V2_SCHEMA_VERSION {
        return Err(AppError::unsupported_schema(
            metadata_version,
            V2_SCHEMA_VERSION,
        ));
    }
    let expected_checksum = initial_schema_checksum();
    let stored_checksum = stored_schema_checksum(connection)?;
    if stored_checksum.as_deref() != Some(expected_checksum.as_str()) {
        return Err(AppError::schema_integrity(
            "The V2 initial migration checksum does not match the schema payload.",
        ));
    }
    let identity = database_identity(connection)?;
    if !valid_database_identity(identity.as_deref()) {
        return Err(AppError::schema_integrity(
            "The V2 database identity metadata is missing or invalid.",
        ));
    }
    let exams_found = exams(connection)?;
    if exams_found.len() != 4 {
        return Err(AppError::schema_integrity(format!(
            "V2 requires exactly four built-in exams; found {}.",
            exams_found.len()
        )));
    }
    for expected in BUILT_IN_EXAMS {
        if !exams_found.iter().any(|exam| exam.code == expected.code) {
            return Err(AppError::schema_integrity(format!(
                "The V2 exam configuration is missing {}.",
                expected.code
            )));
        }
    }
    let check_sql = if full {
        "PRAGMA integrity_check"
    } else {
        "PRAGMA quick_check(1)"
    };
    let integrity: String = connection
        .query_row(check_sql, [], |row| row.get(0))
        .map_err(|error| {
            AppError::database("The V2 database integrity check failed to run", error)
        })?;
    if integrity != "ok" {
        return Err(AppError::v2_integrity(format!(
            "The V2 database integrity check failed: {integrity}"
        )));
    }
    let foreign_key_violations = foreign_key_violation_count(connection)?;
    if foreign_key_violations > 0 {
        return Err(AppError::v2_foreign_key(format!(
            "The V2 database has {foreign_key_violations} foreign-key violation(s)."
        )));
    }
    Ok(())
}

fn foreign_key_violation_count(connection: &Connection) -> Result<i64, AppError> {
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|error| {
            AppError::database("The V2 foreign-key check could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], |_| Ok(()))
        .map_err(|error| AppError::database("The V2 foreign-key check could not run", error))?;
    let mut count = 0_i64;
    for row in rows {
        row.map_err(|error| {
            AppError::database("A V2 foreign-key violation could not be read", error)
        })?;
        count += 1;
    }
    Ok(count)
}

fn integrity_report(connection: &Connection, full: bool) -> Result<V2IntegrityReport, AppError> {
    let check_sql = if full {
        "PRAGMA integrity_check"
    } else {
        "PRAGMA quick_check(1)"
    };
    let integrity: String = connection
        .query_row(check_sql, [], |row| row.get(0))
        .map_err(|error| AppError::database("The V2 integrity check failed to run", error))?;
    let foreign_key_violations = foreign_key_violation_count(connection)?;
    let schema_version = user_version(connection)?;
    let exam_count = exams(connection)?.len() as i64;
    let mut issues = Vec::new();
    if integrity != "ok" {
        issues.push(format!("integrity_check: {integrity}"));
    }
    if foreign_key_violations > 0 {
        issues.push(format!(
            "foreign_key_check: {foreign_key_violations} violation(s)"
        ));
    }
    if schema_version != V2_SCHEMA_VERSION {
        issues.push(format!(
            "schema_version: found {schema_version}, expected {V2_SCHEMA_VERSION}"
        ));
    }
    if exam_count != 4 {
        issues.push(format!("exams: found {exam_count}, expected 4"));
    }
    Ok(V2IntegrityReport {
        ok: issues.is_empty(),
        integrity_check: integrity,
        foreign_key_violations,
        schema_version,
        exam_count,
        issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_path(root: &Path, name: &str) -> PathBuf {
        root.join(name)
    }

    fn insert_revision_fixture(connection: &mut Connection) -> (i64, i64, i64) {
        let now = "2026-11-05T00:00:00Z";
        let mut concept_ids = Vec::new();
        for (slug, title) in [
            ("concept-one", "Concept One"),
            ("concept-two", "Concept Two"),
            ("concept-three", "Concept Three"),
        ] {
            connection
                .execute(
                    "INSERT INTO concepts(slug,title,domain,created_at,updated_at)
                     VALUES(?1,?2,'Physics',?3,?3)",
                    params![slug, title, now],
                )
                .expect("concept insert");
            concept_ids.push(connection.last_insert_rowid());
        }

        for (exam_id, label) in [(1_i64, "fixture-1"), (2_i64, "fixture-2")] {
            connection
                .execute(
                    "INSERT INTO exam_content_versions(exam_id,content_type,version_label,source_reference,verified_on,review_status,created_at)
                     VALUES(?1,'syllabus',?2,'fixture','2026-11-01','verified',?3)",
                    params![exam_id, label, now],
                )
                .expect("content version");
            let content_version_id = connection.last_insert_rowid();
            connection
                .execute(
                    "INSERT INTO exam_syllabus_sources(exam_id,content_version_id,code,title,publisher,source_reference,verified_on,created_at,updated_at)
                     VALUES(?1,?2,'SRC','Source','Publisher','fixture','2026-11-01',?3,?3)",
                    params![exam_id, content_version_id, now],
                )
                .expect("source");
            let source_id = connection.last_insert_rowid();
            connection
                .execute(
                    "INSERT INTO exam_syllabus_nodes(exam_id,source_id,code,title,node_kind,display_order,created_at,updated_at)
                     VALUES(?1,?2,'TOPIC','Topic','topic',1,?3,?3)",
                    params![exam_id, source_id, now],
                )
                .expect("node");
            let node_id = connection.last_insert_rowid();
            if exam_id == 1 {
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts(exam_id,node_id,concept_id,weight,created_at)
                         VALUES(1,?1,?2,1,?3)",
                        params![node_id, concept_ids[0], now],
                    )
                    .expect("mapping");
            }
        }

        (concept_ids[0], concept_ids[1], concept_ids[2])
    }

    fn insert_plan_item(connection: &Connection, concept_id: i64, focus_exam_id: i64) -> i64 {
        connection
            .execute(
                "INSERT INTO revision_plan_items(concept_id,focus_exam_id,status,estimated_minutes,triage_score,lane,source_due_date,created_at,updated_at)
                 VALUES(?1,?2,'planned',30,50,'strengthen','2026-11-05','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                params![concept_id, focus_exam_id],
            )
            .expect("plan item");
        connection.last_insert_rowid()
    }

    fn insert_daily_plan(
        connection: &Connection,
        exam_id: Option<i64>,
        plan_date: &str,
        version: i64,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO revision_daily_plans(plan_date,exam_id,version,effective_budget_minutes,max_topics,status,created_at,updated_at)
                 VALUES(?1,?2,?3,60,10,'active','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                params![plan_date, exam_id, version],
            )
            .expect("daily plan");
        connection.last_insert_rowid()
    }

    fn link_membership(connection: &Connection, daily_plan_id: i64, plan_item_id: i64) {
        connection
            .execute(
                "INSERT INTO revision_daily_plan_items(daily_plan_id,plan_item_id,position,estimated_minutes,lane,triage_score,membership_status,created_at,updated_at)
                 VALUES(?1,?2,0,30,'strengthen',50,'planned','2026-11-05T00:00:00Z','2026-11-05T00:00:00Z')",
                params![daily_plan_id, plan_item_id],
            )
            .expect("membership");
        connection
            .execute(
                "UPDATE revision_plan_items SET active_daily_plan_id=?1 WHERE id=?2",
                params![daily_plan_id, plan_item_id],
            )
            .expect("active plan id");
    }

    /// Reduce a freshly created current-schema database to the table set and
    /// version metadata of a historical schema, so the real upgrade dispatch
    /// can be exercised against temporary fixtures only.
    fn downgrade_database_fixture(state: &V2State, version: i64) {
        assert!((1..=SCHEMA_VERSION_4).contains(&version));
        state
            .with_connection(|connection| {
                // A fresh current-schema database carries the version-5 ledger
                // row. Remove it so the fixture's ledger matches the requested
                // historical version.
                connection
                    .execute_batch("DROP TRIGGER IF EXISTS trg_schema_migrations_no_delete;")
                    .expect("drop ledger delete guard");
                connection
                    .execute(
                        "DELETE FROM schema_migrations WHERE version=?1",
                        [V2_SCHEMA_VERSION],
                    )
                    .expect("remove current ledger row");
                connection
                    .execute_batch(
                        "CREATE TRIGGER trg_schema_migrations_no_delete
                         BEFORE DELETE ON schema_migrations
                         BEGIN SELECT RAISE(ABORT,'schema migrations are immutable'); END;",
                    )
                    .expect("restore ledger delete guard");
                if version <= 4 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS builtin_constants;
                             DROP TABLE IF EXISTS constant_catalog_versions;
                             DROP TABLE IF EXISTS sticky_notes;",
                        )
                        .expect("drop v5 tables");
                }
                if version <= 3 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS custom_formula_variables;
                             DROP TABLE IF EXISTS custom_formulas;
                             DROP TABLE IF EXISTS custom_units;
                             DROP TABLE IF EXISTS custom_constants;",
                        )
                        .expect("drop custom references");
                }
                if version <= 2 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS flashcard_reviews;
                             DROP TABLE IF EXISTS flashcard_schedule;
                             DROP TABLE IF EXISTS flashcard_exam_links;
                             DROP TABLE IF EXISTS flashcards;",
                        )
                        .expect("drop flashcards");
                }
                if version <= 1 {
                    connection
                        .execute_batch(
                            "DROP TABLE IF EXISTS quick_start_config_allocations;
                             DROP TABLE IF EXISTS quick_start_configs;",
                        )
                        .expect("drop quick start");
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(1,'v1_initial_schema_fixture',?1,'2026-01-01T00:00:00Z')",
                            [SCHEMA_1_CHECKSUM],
                        )
                        .expect("v1 ledger");
                }
                if version == 2 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(2,'v2_quick_start_configs',?1,'2026-01-01T00:00:00Z')",
                            [SCHEMA_2_CHECKSUM],
                        )
                        .expect("v2 ledger");
                }
                if version == 3 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(3,'v2_flashcards',?1,'2026-01-01T00:00:00Z')",
                            [SCHEMA_3_CHECKSUM],
                        )
                        .expect("v3 ledger");
                }
                if version == 4 {
                    connection
                        .execute(
                            "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                             VALUES(4,'v2_custom_references',?1,'2026-01-01T00:00:00Z')",
                            [SCHEMA_4_CHECKSUM],
                        )
                        .expect("v4 ledger");
                }
                connection
                    .execute(
                        "UPDATE app_metadata SET schema_version=?1 WHERE id=1",
                        [version],
                    )
                    .expect("metadata version");
                connection
                    .execute_batch(&format!("PRAGMA user_version = {version};"))
                    .expect("user version");
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .expect("checkpoint");
                Ok::<(), AppError>(())
            })
            .expect("downgrade fixture");
    }

    #[test]
    fn fresh_initialization_creates_schema_and_four_active_exams() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        let status = state.status();

        assert!(status.initialized, "{status:?}");
        assert_eq!(status.schema_version, Some(V2_SCHEMA_VERSION));
        assert_eq!(status.exam_count, 4);
        assert_eq!(status.active_exam_count, 4);
        assert_eq!(status.exams.len(), 4);
        assert!(status.exams.iter().all(|exam| exam.is_active));
        assert!(status.exams.iter().all(|exam| exam.exam_date.is_none()));
        assert_eq!(
            status
                .exams
                .iter()
                .map(|exam| exam.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                "csir-net-physics",
                "gate-physics",
                "tifr-gs-physics",
                "jest-physics"
            ]
        );
        assert!(status.foreign_keys_enabled);
        assert_eq!(status.journal_mode.as_deref(), Some("wal"));
        assert!(status.error_message.is_none());
    }

    #[test]
    fn repeated_initialization_is_idempotent() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        drop(V2State::initialize(&data_dir));
        let state = V2State::initialize(&data_dir);
        let status = state.status();

        assert!(status.initialized);
        assert_eq!(status.exam_count, 4);
        assert_eq!(status.active_exam_count, 4);
    }

    #[test]
    fn list_exams_returns_the_four_seeded_exams() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        let exams = state.list_exams().expect("exam list");
        assert_eq!(exams.len(), 4);
        assert_eq!(exams[3].code, "jest-physics");
        assert!(exams.iter().all(|exam| exam.exam_date.is_none()));
    }

    #[test]
    fn retry_initialize_recovers_after_the_schema_version_is_corrected() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        drop(V2State::initialize(&data_dir));
        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        {
            let connection = Connection::open(&path).expect("database open");
            connection
                .execute_batch("PRAGMA user_version = 99;")
                .expect("set newer version");
        }
        let state = V2State::initialize(&data_dir);
        assert!(!state.status().initialized);
        {
            let connection = Connection::open(&path).expect("database reopen");
            connection
                .execute_batch("PRAGMA user_version = 1;")
                .expect("restore version");
        }
        state.retry_initialize().expect("retry should recover");
        assert!(state.status().initialized);
        assert_eq!(state.list_exams().expect("exams").len(), 4);
    }

    #[test]
    fn supported_legacy_schema_versions_upgrade_to_version_five() {
        for version in [1_i64, 2, 3, 4] {
            let root = tempfile::tempdir().expect("temporary directory");
            let data_dir = date_path(root.path(), "v2");
            {
                let state = V2State::initialize(&data_dir);
                state
                    .with_connection(|connection| {
                        connection
                            .execute(
                                "UPDATE settings SET daily_study_target_minutes=137 WHERE id=1",
                                [],
                            )
                            .expect("sentinel row");
                        Ok::<(), AppError>(())
                    })
                    .expect("sentinel row");
                downgrade_database_fixture(&state, version);
            }

            let state = V2State::initialize(&data_dir);
            let status = state.status();
            assert!(
                status.initialized,
                "schema {version} -> 5 failed: {status:?}"
            );
            assert_eq!(status.schema_version, Some(V2_SCHEMA_VERSION));
            assert_eq!(status.metadata_schema_version, Some(V2_SCHEMA_VERSION));
            assert!(status.migration_checksum_valid);

            let (target, ledger_five, ledger_four, ledger_three) = state
                .with_connection(|connection| {
                    let target: i64 = connection
                        .query_row(
                            "SELECT daily_study_target_minutes FROM settings WHERE id=1",
                            [],
                            |row| row.get(0),
                        )
                        .expect("settings sentinel");
                    let ledger_five: String = connection
                        .query_row(
                            "SELECT checksum FROM schema_migrations WHERE version=?1",
                            [V2_SCHEMA_VERSION],
                            |row| row.get(0),
                        )
                        .expect("version five ledger");
                    let ledger_four: Option<String> = connection
                        .query_row(
                            "SELECT checksum FROM schema_migrations WHERE version=?1",
                            [SCHEMA_VERSION_4],
                            |row| row.get(0),
                        )
                        .optional()
                        .expect("version four ledger");
                    let ledger_three: Option<String> = connection
                        .query_row(
                            "SELECT checksum FROM schema_migrations WHERE version=?1",
                            [SCHEMA_VERSION_3],
                            |row| row.get(0),
                        )
                        .optional()
                        .expect("version three ledger");
                    Ok::<_, AppError>((target, ledger_five, ledger_four, ledger_three))
                })
                .expect("upgraded fixture");
            assert_eq!(target, 137, "existing rows must be preserved");
            assert_eq!(ledger_five, initial_schema_checksum());
            assert_eq!(ledger_four.as_deref(), Some(SCHEMA_4_CHECKSUM));
            if version <= 2 {
                assert_eq!(ledger_three.as_deref(), Some(SCHEMA_3_CHECKSUM));
            }

            let table_counts: (i64, i64) = state
                .with_connection(|connection| {
                    let custom_tables: i64 = connection
                        .query_row(
                            "SELECT COUNT(*) FROM sqlite_master
                             WHERE type='table' AND name IN
                               ('custom_formulas','custom_formula_variables',
                                'custom_units','custom_constants')",
                            [],
                            |row| row.get(0),
                        )
                        .expect("custom table count");
                    let v5_tables: i64 = connection
                        .query_row(
                            "SELECT COUNT(*) FROM sqlite_master
                             WHERE type='table' AND name IN
                               ('constant_catalog_versions','builtin_constants','sticky_notes')",
                            [],
                            |row| row.get(0),
                        )
                        .expect("v5 table count");
                    Ok::<_, AppError>((custom_tables, v5_tables))
                })
                .expect("upgraded table counts");
            assert_eq!(table_counts.0, 4);
            assert_eq!(table_counts.1, 3);
            assert_eq!(state.list_exams().expect("exams").len(), 4);
        }
    }

    #[test]
    fn failed_version_four_upgrade_rolls_back_entirely() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        let state = V2State::initialize(&data_dir);
        downgrade_database_fixture(&state, SCHEMA_VERSION_3);
        drop(state);

        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        let mut connection = Connection::open(&path).expect("database open");
        configure(&connection).expect("configure");
        connection
            .execute_batch("CREATE VIEW custom_units AS SELECT 1 AS value;")
            .expect("blocking view");
        let error = upgrade_database_3_to_4(&mut connection).expect_err("upgrade must fail");
        assert_eq!(error.code, "migration_error");

        let (version, metadata_version, ledger_four, formulas_table, view_count): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = connection
            .query_row(
                "SELECT (SELECT user_version FROM pragma_user_version),
                        (SELECT schema_version FROM app_metadata WHERE id=1),
                        (SELECT COUNT(*) FROM schema_migrations WHERE version=4),
                        (SELECT COUNT(*) FROM sqlite_master
                          WHERE type='table' AND name='custom_formulas'),
                        (SELECT COUNT(*) FROM sqlite_master
                          WHERE type='view' AND name='custom_units')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("rollback state");
        assert_eq!(version, SCHEMA_VERSION_3);
        assert_eq!(metadata_version, SCHEMA_VERSION_3);
        assert_eq!(ledger_four, 0);
        assert_eq!(formulas_table, 0, "partial DDL must roll back");
        assert_eq!(view_count, 1);
    }

    #[test]
    fn failed_version_five_upgrade_rolls_back_entirely() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        let state = V2State::initialize(&data_dir);
        downgrade_database_fixture(&state, SCHEMA_VERSION_4);
        drop(state);

        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        let mut connection = Connection::open(&path).expect("database open");
        configure(&connection).expect("configure");
        connection
            .execute_batch("CREATE VIEW sticky_notes AS SELECT 1 AS value;")
            .expect("blocking view");
        let error = upgrade_database_4_to_5(&mut connection).expect_err("upgrade must fail");
        assert_eq!(error.code, "migration_error");

        let (version, metadata_version, ledger_five, catalog_table, view_count): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = connection
            .query_row(
                "SELECT (SELECT user_version FROM pragma_user_version),
                        (SELECT schema_version FROM app_metadata WHERE id=1),
                        (SELECT COUNT(*) FROM schema_migrations WHERE version=5),
                        (SELECT COUNT(*) FROM sqlite_master
                          WHERE type='table' AND name='constant_catalog_versions'),
                        (SELECT COUNT(*) FROM sqlite_master
                          WHERE type='view' AND name='sticky_notes')",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("rollback state");
        assert_eq!(version, SCHEMA_VERSION_4);
        assert_eq!(metadata_version, SCHEMA_VERSION_4);
        assert_eq!(ledger_five, 0);
        assert_eq!(catalog_table, 0, "partial DDL must roll back");
        assert_eq!(view_count, 1);
    }

    #[test]
    fn fresh_database_has_unique_database_identity() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        let database_id: String = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT database_id FROM app_metadata WHERE id=1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("database id", error))
            })
            .expect("database id");
        assert_eq!(database_id.len(), 32);
        assert_ne!(database_id, "0".repeat(32));
    }

    #[test]
    fn migration_checksum_is_deterministic_and_content_sensitive() {
        let first = initial_schema_checksum();
        let second = migration_checksum(SCHEMA_SQL);
        assert_eq!(first, second);
        assert_eq!(migration_checksum("a\r\nb"), migration_checksum("a\nb"));
        assert_ne!(migration_checksum("a\nb"), migration_checksum("a\nc"));
    }

    #[test]
    fn stored_migration_checksum_matches_the_schema_payload() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        let checksum: String = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT checksum FROM schema_migrations WHERE version=?1",
                        [V2_SCHEMA_VERSION],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("checksum", error))
            })
            .expect("checksum");
        assert_eq!(checksum, initial_schema_checksum());
        assert_ne!(checksum, "0".repeat(64));
    }

    #[test]
    fn repeated_initialization_does_not_change_the_checksum() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        let first = {
            let state = V2State::initialize(&data_dir);
            state
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT checksum FROM schema_migrations WHERE version=?1",
                            [V2_SCHEMA_VERSION],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|error| AppError::database("checksum", error))
                })
                .expect("checksum")
        };
        drop(V2State::initialize(&data_dir));
        let second = {
            let state = V2State::initialize(&data_dir);
            state
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT checksum FROM schema_migrations WHERE version=?1",
                            [V2_SCHEMA_VERSION],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|error| AppError::database("checksum", error))
                })
                .expect("checksum")
        };
        assert_eq!(first, second);
    }

    #[test]
    fn status_reports_healthy_metadata_and_checks() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        let status = state.status();
        assert_eq!(status.state, V2DatabaseState::Healthy);
        assert!(status.initialized);
        assert_eq!(status.metadata_schema_version, Some(V2_SCHEMA_VERSION));
        assert!(status.database_id_valid);
        assert!(status.migration_checksum_valid);
        assert_eq!(status.integrity_check.as_deref(), Some("ok"));
        assert_eq!(status.foreign_key_violations, Some(0));
        assert!(status.error_code.is_none());
    }

    #[test]
    fn status_classifies_not_initialized_and_inaccessible_states() {
        let not_initialized = V2State::unavailable(AppError {
            code: "database_not_initialized",
            message: "The V2 database has not been initialized yet.".to_owned(),
            field: None,
        });
        assert_eq!(
            not_initialized.status().state,
            V2DatabaseState::NotInitialized
        );

        let inaccessible =
            V2State::unavailable(AppError::database_unavailable("database unavailable"));
        assert_eq!(inaccessible.status().state, V2DatabaseState::Inaccessible);

        let integrity = V2State::unavailable(AppError::v2_integrity("bad integrity"));
        let integrity_status = integrity.status();
        assert_eq!(integrity_status.state, V2DatabaseState::IntegrityFailure);
        assert!(!integrity_status.initialized);
    }

    #[test]
    fn status_classifies_metadata_mismatch() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        state
            .with_connection(|connection| {
                connection
                    .execute("UPDATE app_metadata SET schema_version=99 WHERE id=1", [])
                    .map_err(|error| AppError::database("metadata update", error))
            })
            .expect("metadata mismatch");
        let status = state.status();
        assert_eq!(status.state, V2DatabaseState::MetadataMismatch);
        assert!(!status.initialized);
    }

    #[test]
    fn status_classifies_foreign_key_failure() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        state
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "PRAGMA foreign_keys=OFF;
                         INSERT INTO study_sessions(session_date,duration_seconds,source,primary_exam_id,created_at,updated_at)
                         VALUES('2026-11-05',60,'manual',999,'2026-11-05T00:00:00Z','2026-11-05T00:00:00Z');
                         PRAGMA foreign_keys=ON;",
                    )
                    .map_err(|error| AppError::database("fk fixture", error))
            })
            .expect("fk fixture");
        let status = state.status();
        assert_eq!(status.state, V2DatabaseState::ForeignKeyFailure);
        assert!(!status.initialized);
        assert_eq!(status.foreign_key_violations, Some(1));
        state
            .with_connection(|connection| {
                connection
                    .execute("DELETE FROM study_sessions WHERE primary_exam_id=999", [])
                    .map_err(|error| AppError::database("fk cleanup", error))
            })
            .expect("fk cleanup");
        assert_eq!(state.status().state, V2DatabaseState::Healthy);
    }

    #[test]
    fn foreign_keys_are_enforced_and_integrity_is_clean() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        state
            .with_connection(|connection| {
                let enabled: i64 = connection
                    .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                    .map_err(|error| AppError::database("foreign keys", error))?;
                assert_eq!(enabled, 1);
                let error = connection.execute(
                    "INSERT INTO exam_profiles(exam_id,is_active,created_at,updated_at)
                     VALUES(999,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                    [],
                );
                assert!(error.is_err(), "invalid FK insert should fail");
                Ok(())
            })
            .expect("foreign key check");
        let report = state.check_integrity(true).expect("integrity report");
        assert!(report.ok, "{report:?}");
        assert_eq!(report.foreign_key_violations, 0);
        assert_eq!(report.exam_count, 4);
    }

    #[test]
    fn newer_schema_version_is_rejected_without_modifying_it() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        drop(V2State::initialize(&data_dir));
        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        {
            let connection = Connection::open(&path).expect("database open");
            connection
                .execute_batch("PRAGMA user_version = 99;")
                .expect("set newer version");
        }
        let state = V2State::initialize(&data_dir);
        let status = state.status();
        assert!(!status.initialized);
        assert_eq!(status.state, V2DatabaseState::UnsupportedSchema);
        assert_eq!(status.error_code.as_deref(), Some("unsupported_schema"));
        let version: i64 = Connection::open(&path)
            .expect("database reopen")
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("version");
        assert_eq!(version, 99);
    }

    #[test]
    fn partial_initialization_is_moved_aside_and_recovered() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        fs::create_dir_all(&data_dir).expect("data dir");
        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        {
            let connection = Connection::open(&path).expect("partial open");
            connection
                .execute_batch("CREATE TABLE partial_state(value TEXT);")
                .expect("partial table");
        }
        let state = V2State::initialize(&data_dir);
        assert!(state.status().initialized);
        let incomplete = fs::read_dir(&data_dir)
            .expect("data dir read")
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains(".incomplete-"));
        assert!(incomplete, "partial database should be preserved aside");
    }

    #[test]
    fn orphaned_sidecars_are_preserved_without_attaching_to_a_new_database() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = date_path(root.path(), "v2");
        fs::create_dir_all(&data_dir).expect("data dir");
        let path = data_dir.join(V2_DATABASE_FILE_NAME);
        fs::write(format!("{}-wal", path.display()), b"orphan wal").expect("orphan wal");
        let state = V2State::initialize(&data_dir);
        assert!(state.status().initialized);
        let orphan = fs::read_dir(&data_dir)
            .expect("data dir read")
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains(".orphan-"));
        assert!(orphan, "orphan sidecar should be preserved aside");
    }

    #[test]
    fn revision_plan_scope_is_protected_while_membership_exists() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&date_path(root.path(), "v2"));
        state
            .with_connection(|connection| {
                let (concept_one, concept_two, concept_three) = insert_revision_fixture(connection);
                let item = insert_plan_item(connection, concept_one, 1);
                let plan = insert_daily_plan(connection, None, "2026-11-05", 1);
                link_membership(connection, plan, item);

                let concept_error = connection
                    .execute(
                        "UPDATE revision_plan_items SET concept_id=?1 WHERE id=?2",
                        params![concept_two, item],
                    )
                    .expect_err("concept change should be blocked");
                assert!(concept_error.to_string().contains("concept"));

                let focus_error = connection
                    .execute(
                        "UPDATE revision_plan_items SET focus_exam_id=2 WHERE id=?1",
                        params![item],
                    )
                    .expect_err("focus exam change should be blocked");
                assert!(focus_error.to_string().contains("concept"));

                let unplanned = insert_plan_item(connection, concept_two, 1);
                connection
                    .execute(
                        "UPDATE revision_plan_items SET concept_id=?1 WHERE id=?2",
                        params![concept_three, unplanned],
                    )
                    .expect("concept change without membership");
                connection
                    .execute(
                        "UPDATE revision_plan_items SET focus_exam_id=2 WHERE id=?1",
                        params![unplanned],
                    )
                    .expect("focus exam change without membership");

                connection
                    .execute(
                        "UPDATE revision_daily_plan_items SET membership_status='removed'
                         WHERE daily_plan_id=?1 AND plan_item_id=?2",
                        params![plan, item],
                    )
                    .expect("membership removal");
                connection
                    .execute(
                        "UPDATE revision_daily_plans SET status='closed' WHERE id=?1",
                        params![plan],
                    )
                    .expect("plan close");

                let reassigned_plan = insert_daily_plan(connection, Some(1), "2026-11-05", 1);
                link_membership(connection, reassigned_plan, item);

                let ownership_error = connection
                    .execute(
                        "UPDATE revision_daily_plans SET exam_id=2 WHERE id=?1",
                        params![reassigned_plan],
                    )
                    .expect_err("plan ownership change should be blocked");
                assert!(ownership_error.to_string().contains("daily plan exam"));

                connection
                    .execute("UPDATE exam_profiles SET is_active=0 WHERE exam_id=1", [])
                    .expect("deactivate exam");
                let item_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM revision_plan_items WHERE id=?1",
                        params![item],
                        |row| row.get(0),
                    )
                    .expect("item count");
                let membership_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM revision_daily_plan_items WHERE plan_item_id=?1",
                        params![item],
                        |row| row.get(0),
                    )
                    .expect("membership count");
                assert_eq!(item_count, 1);
                assert_eq!(membership_count, 2);
                connection
                    .execute("UPDATE exam_profiles SET is_active=1 WHERE exam_id=1", [])
                    .expect("reactivate exam");
                Ok(())
            })
            .expect("revision plan scope");
    }

    fn collect_directory_files(
        directory: &Path,
        relative: &Path,
        files: &mut Vec<(String, Vec<u8>)>,
    ) {
        for entry in fs::read_dir(directory).expect("directory entry") {
            let entry = entry.expect("directory entry");
            let path = entry.path();
            let next_relative = relative.join(entry.file_name());
            if path.is_dir() {
                collect_directory_files(&path, &next_relative, files);
            } else {
                files.push((
                    next_relative.to_string_lossy().to_string(),
                    fs::read(&path).expect("file read"),
                ));
            }
        }
    }

    fn directory_fingerprint(directory: &Path) -> String {
        let mut files = Vec::new();
        collect_directory_files(directory, Path::new(""), &mut files);
        files.sort_by(|left, right| left.0.cmp(&right.0));
        let mut hasher = Sha256::new();
        for (name, bytes) in files {
            hasher.update(name.as_bytes());
            hasher.update(bytes);
        }
        format!("{:x}", hasher.finalize())
    }

    #[test]
    fn v1_data_is_isolated_and_untouched() {
        let root = tempfile::tempdir().expect("temporary directory");
        let v1_dir = root.path().join(crate::app_paths::V1_APP_IDENTIFIER);
        fs::create_dir_all(&v1_dir).expect("v1 dir");
        let v1_path = v1_dir.join("csir-net-tracker.sqlite3");
        fs::write(&v1_path, b"v1-sentinel").expect("v1 sentinel");
        let v2_dir = crate::app_paths::resolve_v2_data_dir(root.path()).expect("v2 path");
        assert_ne!(v2_dir, v1_dir);
        assert!(!v2_dir.starts_with(&v1_dir));
        let state = V2State::initialize(&v2_dir);
        assert!(state.status().initialized);
        assert_eq!(fs::read(&v1_path).expect("v1 read"), b"v1-sentinel");
    }

    #[test]
    fn v2_only_startup_does_not_open_or_mutate_v1_database_or_timer() {
        let root = tempfile::tempdir().expect("temporary directory");
        let v1_dir = root.path().join(crate::app_paths::V1_APP_IDENTIFIER);
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).expect("date");
        let v1_state = crate::db::AppState::initialize(&v1_dir, today);
        v1_state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE timer_state SET
                            status = 'running',
                            mode = 'focus',
                            duration_ms = 1500000,
                            accumulated_ms = 0,
                            first_started_at = '2026-11-05T07:00:00.000Z',
                            segment_started_at = '2026-11-05T07:00:00.000Z',
                            last_heartbeat_at = '2026-11-05T07:05:00.000Z',
                            session_date = '2026-11-05',
                            recovery_reason = NULL,
                            recovery_excluded_ms = 0,
                            recovery_at = NULL,
                            updated_at = '2026-11-05T07:05:00.000Z'
                         WHERE id = 1",
                        [],
                    )
                    .map_err(|error| {
                        AppError::database("The disposable V1 timer could not be saved", error)
                    })?;
                Ok(())
            })
            .expect("disposable V1 timer");
        drop(v1_state);
        let v1_before = directory_fingerprint(&v1_dir);

        let v2_dir = crate::app_paths::resolve_v2_data_dir(root.path()).expect("v2 path");
        let v2_state = V2State::initialize(&v2_dir);
        assert!(
            v2_state.status().initialized,
            "V2 startup must still initialize the V2 database"
        );
        assert_eq!(v2_state.status().exam_count, 4);
        assert_eq!(v2_state.list_exams().expect("V2 exams").len(), 4);

        let v1_after = directory_fingerprint(&v1_dir);
        assert_eq!(
            v1_before, v1_after,
            "V2 startup must not open, seed, migrate, or recover V1"
        );

        let connection = Connection::open_with_flags(
            v1_dir.join("csir-net-tracker.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("V1 read-only check");
        let status: String = connection
            .query_row("SELECT status FROM timer_state WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("V1 timer status");
        assert_eq!(status, "running", "V1 restart recovery must not run");
    }

    #[test]
    fn two_v2_databases_are_independent() {
        let root = tempfile::tempdir().expect("temporary directory");
        let first = V2State::initialize(&root.path().join("first"));
        let second = V2State::initialize(&root.path().join("second"));
        assert!(first.status().initialized);
        assert!(second.status().initialized);
        assert_ne!(first.status().database_path, second.status().database_path);
        assert_eq!(first.status().exam_count, 4);
        assert_eq!(second.status().exam_count, 4);
    }
}
