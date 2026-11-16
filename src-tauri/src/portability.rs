use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::{Local, SecondsFormat, Utc};
use rusqlite::{
    Connection, OptionalExtension, ToSql, Transaction, params_from_iter, types::ValueRef,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{backup, date_only, db, error::AppError, exams};

pub const JSON_EXPORT_FORMAT_VERSION: u32 = 4;
pub const MIN_JSON_IMPORT_FORMAT_VERSION: u32 = 1;
pub const MIN_JSON_IMPORT_SCHEMA_VERSION: i64 = 8;
pub const MAX_IMPORT_BYTES: u64 = 100 * 1024 * 1024;
pub const CSV_BOM: &str = "\u{feff}";

const CORE_TABLES: [&str; 16] = backup::CORE_BACKUP_TABLES;

const RESOURCE_TABLES: [&str; 4] = [
    "study_resources",
    "resource_progress",
    "resource_local_files",
    "resource_discovery_runs",
];

const V2_TABLES: [&str; 20] = [
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
    "study_resources",
    "resource_progress",
    "resource_local_files",
    "resource_discovery_runs",
];

const V3_TABLES: [&str; 25] = [
    "settings",
    "syllabus_sources",
    "syllabus_sections",
    "syllabus_topics",
    "syllabus_progress",
    "timer_settings",
    "study_sessions",
    "checklist_tasks",
    "planned_study_blocks",
    "pyq_attempts",
    "mock_tests",
    "revision_settings",
    "revision_schedules",
    "revision_history",
    "revision_plan_items",
    "timer_state",
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

const V4_TABLES: [&str; 27] = [
    "exams",
    "exam_profiles",
    "settings",
    "syllabus_sources",
    "syllabus_sections",
    "syllabus_topics",
    "syllabus_progress",
    "timer_settings",
    "study_sessions",
    "checklist_tasks",
    "planned_study_blocks",
    "pyq_attempts",
    "mock_tests",
    "revision_settings",
    "revision_schedules",
    "revision_history",
    "revision_plan_items",
    "timer_state",
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

const CORE_DELETE_ORDER: [&str; 21] = [
    "revision_session_links",
    "revision_deferrals",
    "revision_daily_plan_items",
    "revision_daily_plans",
    "revision_plan_items",
    "dashboard_sections",
    "revision_history",
    "revision_schedules",
    "revision_settings",
    "mock_tests",
    "pyq_attempts",
    "planned_study_blocks",
    "checklist_tasks",
    "study_sessions",
    "timer_state",
    "timer_settings",
    "syllabus_progress",
    "syllabus_topics",
    "syllabus_sections",
    "syllabus_sources",
    "settings",
];

const V2_DELETE_ORDER: [&str; 25] = [
    "revision_session_links",
    "revision_deferrals",
    "revision_daily_plan_items",
    "revision_daily_plans",
    "revision_plan_items",
    "resource_local_files",
    "resource_progress",
    "resource_discovery_runs",
    "study_resources",
    "dashboard_sections",
    "revision_history",
    "revision_schedules",
    "revision_settings",
    "mock_tests",
    "pyq_attempts",
    "planned_study_blocks",
    "checklist_tasks",
    "study_sessions",
    "timer_state",
    "timer_settings",
    "syllabus_progress",
    "syllabus_topics",
    "syllabus_sections",
    "syllabus_sources",
    "settings",
];

const V3_DELETE_ORDER: [&str; 25] = [
    "revision_session_links",
    "revision_deferrals",
    "revision_daily_plan_items",
    "revision_daily_plans",
    "revision_plan_items",
    "resource_local_files",
    "resource_progress",
    "resource_discovery_runs",
    "study_resources",
    "dashboard_sections",
    "revision_history",
    "revision_schedules",
    "revision_settings",
    "mock_tests",
    "pyq_attempts",
    "planned_study_blocks",
    "checklist_tasks",
    "study_sessions",
    "timer_state",
    "timer_settings",
    "syllabus_progress",
    "syllabus_topics",
    "syllabus_sections",
    "syllabus_sources",
    "settings",
];

const V4_DELETE_ORDER: [&str; 27] = [
    "exam_profiles",
    "exams",
    "revision_session_links",
    "revision_deferrals",
    "revision_daily_plan_items",
    "revision_daily_plans",
    "revision_plan_items",
    "resource_local_files",
    "resource_progress",
    "resource_discovery_runs",
    "study_resources",
    "dashboard_sections",
    "revision_history",
    "revision_schedules",
    "revision_settings",
    "mock_tests",
    "pyq_attempts",
    "planned_study_blocks",
    "checklist_tasks",
    "study_sessions",
    "timer_state",
    "timer_settings",
    "syllabus_progress",
    "syllabus_topics",
    "syllabus_sections",
    "syllabus_sources",
    "settings",
];

fn tables_for_format(format_version: u32) -> &'static [&'static str] {
    if format_version >= 4 {
        &V4_TABLES
    } else if format_version >= 3 {
        &V3_TABLES
    } else if format_version >= 2 {
        &V2_TABLES
    } else {
        &CORE_TABLES
    }
}

fn delete_order_for_format(format_version: u32) -> &'static [&'static str] {
    if format_version >= 4 {
        &V4_DELETE_ORDER
    } else if format_version >= 3 {
        &V3_DELETE_ORDER
    } else if format_version >= 2 {
        &V2_DELETE_ORDER
    } else {
        &CORE_DELETE_ORDER
    }
}

fn format_includes_resources(format_version: u32) -> bool {
    format_version >= 2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonBackupPreferences {
    pub automatic_enabled: bool,
    pub frequency: String,
    pub retention_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportEnvelope {
    pub format_version: u32,
    pub application_version: String,
    pub schema_version: i64,
    pub exported_at: String,
    pub database_id: String,
    #[serde(default)]
    pub import_batch_id: Option<String>,
    pub record_counts: BTreeMap<String, i64>,
    pub tables: BTreeMap<String, Vec<Value>>,
    pub backup_preferences: JsonBackupPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchInfo {
    pub imported_at: String,
    pub strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvFileResult {
    pub file_name: String,
    pub rows: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportResult {
    pub folder_name: String,
    pub directory: String,
    pub files: Vec<CsvFileResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonExportResult {
    pub file_name: String,
    pub directory: String,
    pub bytes: u64,
    pub record_counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTableConflict {
    pub table: String,
    pub rows: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImportPreview {
    pub file_name: String,
    pub file_sha256: String,
    pub format_version: u32,
    pub application_version: String,
    pub schema_version: i64,
    pub exported_at: String,
    pub database_id: String,
    pub import_batch_id: String,
    pub already_imported: Option<ImportBatchInfo>,
    pub record_counts: BTreeMap<String, i64>,
    pub existing_counts: BTreeMap<String, i64>,
    pub conflicts: Vec<ImportTableConflict>,
    pub warnings: Vec<String>,
    pub merge_supported: bool,
    pub replacement_supported: bool,
    pub merge_description: String,
    pub replacement_description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonImportStrategy {
    Merge,
    Replace,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTableResult {
    pub imported: i64,
    pub skipped: i64,
    pub conflicting: i64,
    pub rejected: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonImportResult {
    pub strategy: String,
    pub imported: i64,
    pub skipped: i64,
    pub conflicting: i64,
    pub rejected: i64,
    pub per_table: BTreeMap<String, ImportTableResult>,
    pub safety_backup_file: String,
    pub batch_id: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

impl ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            Self::Null => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Null,
            )),
            Self::Integer(value) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(*value),
            )),
            Self::Real(value) => Ok(rusqlite::types::ToSqlOutput::Owned(
                rusqlite::types::Value::Real(*value),
            )),
            Self::Text(value) => Ok(rusqlite::types::ToSqlOutput::Borrowed(
                rusqlite::types::ValueRef::Text(value.as_bytes()),
            )),
        }
    }
}

pub fn export_csv_datasets(
    connection: &Connection,
    destination: &Path,
) -> Result<CsvExportResult, AppError> {
    let destination = prepare_export_directory(destination)?;
    let folder_name = unique_folder_name(&destination, "csv-export")?;
    let final_directory = destination.join(&folder_name);
    let temp_directory = destination.join(format!(
        ".csir-net-tracker-csv-export-{}.tmp",
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&temp_directory).map_err(|error| {
        eprintln!("CSV export folder could not be created: {error}");
        AppError::export_failed("The CSV export folder could not be created.")
    })?;

    let datasets = csv_datasets();
    let mut files = Vec::new();
    for (file_name, sql) in datasets {
        match write_csv_dataset(connection, &temp_directory, file_name, sql) {
            Ok(rows) => files.push(CsvFileResult {
                file_name: file_name.to_owned(),
                rows,
            }),
            Err(error) => {
                let _ = fs::remove_dir_all(&temp_directory);
                return Err(error);
            }
        }
    }

    fs::rename(&temp_directory, &final_directory).map_err(|error| {
        eprintln!("CSV export could not be finalized: {error}");
        let _ = fs::remove_dir_all(&temp_directory);
        AppError::export_failed("The CSV export could not be completed.")
    })?;

    Ok(CsvExportResult {
        folder_name,
        directory: destination.display().to_string(),
        files,
    })
}

fn csv_datasets() -> [(&'static str, &'static str); 17] {
    [
        (
            "exams.csv",
            "SELECT
                e.id AS exam_id,
                e.code AS exam_code,
                e.name AS exam_name,
                e.short_name,
                e.authority,
                e.official_url,
                e.display_order,
                p.is_active,
                p.exam_date,
                p.legacy_date_notice_state,
                p.created_at,
                p.updated_at
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             ORDER BY e.display_order, e.id",
        ),
        (
            "exam_profiles.csv",
            "SELECT
                p.exam_id,
                e.code AS exam_code,
                p.is_active,
                p.exam_date,
                p.legacy_date_notice_state,
                p.created_at,
                p.updated_at
             FROM exam_profiles p
             JOIN exams e ON e.id = p.exam_id
             ORDER BY e.display_order, e.id",
        ),
        (
            "syllabus_topics_progress.csv",
            "SELECT
                t.id AS topic_id,
                t.code AS topic_code,
                t.title AS topic_title,
                s.id AS section_id,
                s.code AS section_code,
                s.title AS section_title,
                t.category,
                t.is_archived,
                t.archived_at,
                t.source_id,
                t.source_reference,
                t.display_order,
                p.topic_id AS progress_topic_id,
                COALESCE(p.status, 'Not started') AS progress_status,
                COALESCE(p.priority, 'Normal') AS progress_priority,
                COALESCE(p.confidence, 3) AS progress_confidence,
                COALESCE(p.notes, '') AS progress_notes,
                p.created_at AS progress_created_at,
                p.updated_at AS progress_updated_at,
                t.created_at AS topic_created_at,
                t.updated_at AS topic_updated_at
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             ORDER BY s.display_order, t.display_order, t.id",
        ),
        (
            "study_sessions.csv",
            "SELECT
                ss.id,
                ss.session_date,
                ss.started_at,
                ss.ended_at,
                ss.duration_seconds,
                ss.source,
                ss.topic_id,
                t.code AS topic_code,
                t.title AS topic_title,
                ss.notes,
                ss.created_at,
                ss.updated_at
             FROM study_sessions ss
             LEFT JOIN syllabus_topics t ON t.id = ss.topic_id
             ORDER BY ss.session_date, ss.id",
        ),
        (
            "checklist_tasks.csv",
            "SELECT
                ct.id,
                ct.title,
                ct.description,
                ct.scheduled_date,
                ct.deadline_date,
                ct.topic_id,
                t.code AS topic_code,
                ct.priority,
                ct.status,
                ct.schedule_anchor,
                ct.schedule_offset_days,
                ct.completed_at,
                ct.created_at,
                ct.updated_at
             FROM checklist_tasks ct
             LEFT JOIN syllabus_topics t ON t.id = ct.topic_id
             ORDER BY ct.scheduled_date, ct.id",
        ),
        (
            "planned_study_blocks.csv",
            "SELECT
                b.id,
                b.title,
                b.topic_id,
                t.code AS topic_code,
                b.calendar_date,
                b.start_time,
                b.end_time,
                b.status,
                b.notes,
                b.linked_session_id,
                b.created_at,
                b.updated_at
             FROM planned_study_blocks b
             LEFT JOIN syllabus_topics t ON t.id = b.topic_id
             ORDER BY b.calendar_date, b.start_time, b.id",
        ),
        (
            "pyq_attempts.csv",
            "SELECT
                p.id,
                p.question_year,
                p.exam_session,
                p.part,
                p.topic_id,
                t.code AS topic_code,
                p.question_reference,
                p.attempt_date,
                p.status,
                p.marks_obtained,
                p.max_marks,
                p.time_spent_seconds,
                p.notes,
                p.difficulty,
                p.import_key,
                p.created_at,
                p.updated_at
             FROM pyq_attempts p
             JOIN syllabus_topics t ON t.id = p.topic_id
             ORDER BY p.attempt_date, p.id",
        ),
        (
            "mock_tests.csv",
            "SELECT
                id,
                test_name,
                test_date,
                duration_minutes,
                max_marks,
                marks_obtained,
                part_a_questions,
                part_a_correct,
                part_a_incorrect,
                part_a_unanswered,
                part_a_correct_marks,
                part_a_incorrect_marks,
                part_a_unanswered_marks,
                part_b_questions,
                part_b_correct,
                part_b_incorrect,
                part_b_unanswered,
                part_b_correct_marks,
                part_b_incorrect_marks,
                part_b_unanswered_marks,
                part_c_questions,
                part_c_correct,
                part_c_incorrect,
                part_c_unanswered,
                part_c_correct_marks,
                part_c_incorrect_marks,
                part_c_unanswered_marks,
                notes,
                review_observations,
                import_key,
                created_at,
                updated_at
             FROM mock_tests
             ORDER BY test_date, id",
        ),
        (
            "revision_schedules.csv",
            "SELECT
                r.topic_id,
                t.code AS topic_code,
                t.title AS topic_title,
                r.status,
                r.last_revision_date,
                r.next_revision_date,
                r.current_interval_days,
                r.completed_revisions,
                r.previous_outcome,
                r.mastery_estimate,
                r.notes,
                r.deferred_until,
                r.created_at,
                r.updated_at
             FROM revision_schedules r
             JOIN syllabus_topics t ON t.id = r.topic_id
             ORDER BY r.next_revision_date, r.topic_id",
        ),
        (
            "revision_history.csv",
            "SELECT
                h.id,
                h.topic_id,
                t.code AS topic_code,
                t.title AS topic_title,
                h.revision_date,
                h.scheduled_for_date,
                h.outcome,
                h.notes,
                h.interval_before_days,
                h.interval_after_days,
                h.mastery_before,
                h.mastery_after,
                h.created_at
             FROM revision_history h
             JOIN syllabus_topics t ON t.id = h.topic_id
             ORDER BY h.revision_date, h.id",
        ),
        (
            "revision_plan_items.csv",
            "SELECT
                rpi.id,
                rpi.topic_id,
                t.code AS topic_code,
                t.title AS topic_title,
                rpi.status,
                rpi.estimated_minutes,
                rpi.triage_score,
                rpi.lane,
                rpi.source_due_date,
                rpi.planned_block_id,
                rpi.revision_history_id,
                rpi.removed_at,
                rpi.created_at,
                rpi.updated_at
             FROM revision_plan_items rpi
             JOIN syllabus_topics t ON t.id = rpi.topic_id
             ORDER BY rpi.id",
        ),
        (
            "revision_daily_plans.csv",
            "SELECT
                id,
                plan_date,
                version,
                effective_budget_minutes,
                max_topics,
                planned_minutes,
                completed_minutes,
                status,
                created_at,
                last_rebalanced_at,
                updated_at
             FROM revision_daily_plans
             ORDER BY plan_date, version",
        ),
        (
            "revision_daily_plan_items.csv",
            "SELECT
                dpi.daily_plan_id,
                dpi.plan_item_id,
                dp.plan_date,
                dp.version,
                dpi.position,
                dpi.estimated_minutes,
                dpi.lane,
                dpi.triage_score,
                dpi.membership_status,
                dpi.planned_block_id,
                dpi.planned_block_title,
                dpi.planned_block_date,
                dpi.planned_block_start_time,
                dpi.planned_block_end_time,
                dpi.carried_from_daily_plan_id,
                dpi.created_at,
                dpi.updated_at
             FROM revision_daily_plan_items dpi
             JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
             ORDER BY dpi.daily_plan_id, dpi.position, dpi.plan_item_id",
        ),
        (
            "revision_deferrals.csv",
            "SELECT
                d.id,
                d.topic_id,
                t.code AS topic_code,
                d.plan_item_id,
                d.event_kind,
                d.overridden_deferral_id,
                d.previous_due_date,
                d.new_due_date,
                d.occurred_on,
                d.reason,
                d.notes,
                d.created_at
             FROM revision_deferrals d
             JOIN syllabus_topics t ON t.id = d.topic_id
             ORDER BY d.occurred_on, d.id",
        ),
        (
            "revision_session_links.csv",
            "SELECT
                rsl.plan_item_id,
                rsl.study_session_id,
                ss.session_date,
                ss.duration_seconds,
                ss.source,
                rsl.created_at
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             ORDER BY ss.session_date, rsl.study_session_id",
        ),
        (
            "settings.csv",
            "SELECT
                id,
                exam_date,
                preparation_start_date,
                daily_study_target_minutes,
                created_at,
                updated_at
             FROM settings
             ORDER BY id",
        ),
        (
            "dashboard_layout.csv",
            "SELECT
                section_key,
                visible,
                display_order,
                updated_at
             FROM dashboard_sections
             ORDER BY display_order, section_key",
        ),
    ]
}

fn write_csv_dataset(
    connection: &Connection,
    directory: &Path,
    file_name: &str,
    sql: &str,
) -> Result<i64, AppError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("A CSV export query could not be prepared", error))?;
    let headers = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let column_count = headers.len();
    let mut rows = statement
        .query([])
        .map_err(|error| AppError::database("A CSV export query could not run", error))?;

    let path = directory.join(file_name);
    let temp = directory.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let mut output = fs::File::create(&temp)
        .map_err(|_| AppError::export_failed("A CSV export file could not be created."))?;
    output
        .write_all(CSV_BOM.as_bytes())
        .map_err(|_| AppError::export_failed("A CSV export file could not be written."))?;
    write_csv_row(&mut output, &headers)?;
    let mut count = 0_i64;

    while let Some(row) = rows
        .next()
        .map_err(|error| AppError::database("A CSV export row could not be read", error))?
    {
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            let value = row.get_ref(index).map_err(|error| {
                AppError::database("A CSV export value could not be read", error)
            })?;
            values.push(value_to_csv(value));
        }
        write_csv_row(&mut output, &values)?;
        count += 1;
    }
    drop(rows);
    drop(statement);
    output
        .flush()
        .map_err(|_| AppError::export_failed("A CSV export file could not be flushed."))?;
    fs::rename(&temp, &path).map_err(|error| {
        eprintln!("CSV export file could not be finalized: {error}");
        let _ = fs::remove_file(&temp);
        AppError::export_failed("A CSV export file could not be finalized.")
    })?;
    Ok(count)
}

fn write_csv_row(output: &mut fs::File, values: &[String]) -> Result<(), AppError> {
    let line = values
        .iter()
        .map(|value| csv_escape(value))
        .collect::<Vec<_>>()
        .join(",");
    output
        .write_all(line.as_bytes())
        .and_then(|_| output.write_all(b"\r\n"))
        .map_err(|_| AppError::export_failed("A CSV export file could not be written."))
}

pub fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\r' | '\n'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn value_to_csv(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
        ValueRef::Blob(value) => {
            let mut encoded = String::with_capacity(value.len() * 2 + 2);
            encoded.push_str("0x");
            for byte in value {
                encoded.push_str(&format!("{byte:02x}"));
            }
            encoded
        }
    }
}

pub fn export_json(
    connection: &Connection,
    destination: &Path,
) -> Result<JsonExportResult, AppError> {
    let destination = prepare_export_directory(destination)?;
    let schema_version = backup::schema_version(connection)?;
    let database_id = backup::ensure_database_identifier(connection, None)?;
    let mut tables = BTreeMap::new();
    let mut record_counts = BTreeMap::new();
    for table in V4_TABLES {
        let rows = table_to_json(connection, table)?;
        record_counts.insert(table.to_owned(), rows.len() as i64);
        tables.insert(table.to_owned(), rows);
    }
    let backup_settings = backup::load_settings(connection)?;
    let envelope = JsonExportEnvelope {
        format_version: JSON_EXPORT_FORMAT_VERSION,
        application_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version,
        exported_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        database_id,
        import_batch_id: Some(Uuid::new_v4().simple().to_string()),
        record_counts,
        tables,
        backup_preferences: JsonBackupPreferences {
            automatic_enabled: backup_settings.automatic_enabled,
            frequency: backup_settings.frequency,
            retention_count: backup_settings.retention_count,
        },
    };

    validate_envelope_with_temporary_database(&envelope).map_err(|_| {
        AppError::export_failed("The complete JSON export failed database validation.")
    })?;
    let bytes = serde_json::to_vec_pretty(&envelope)
        .map_err(|_| AppError::export_failed("The JSON export could not be encoded."))?;
    let parsed: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::export_failed("The JSON export failed validation."))?;
    validate_export_shape(&parsed, &envelope)?;

    let file_name = unique_export_file_name(&destination, "json-export", "json")?;
    let path = destination.join(&file_name);
    let temp = destination.join(format!(".{file_name}.{}.tmp", Uuid::new_v4().simple()));
    let mut output = fs::File::create(&temp)
        .map_err(|_| AppError::export_failed("The JSON export file could not be created."))?;
    output
        .write_all(&bytes)
        .and_then(|_| output.write_all(b"\n"))
        .and_then(|_| output.flush())
        .map_err(|_| AppError::export_failed("The JSON export file could not be written."))?;
    fs::rename(&temp, &path).map_err(|error| {
        eprintln!("JSON export could not be finalized: {error}");
        let _ = fs::remove_file(&temp);
        AppError::export_failed("The JSON export could not be finalized.")
    })?;
    Ok(JsonExportResult {
        file_name,
        directory: destination.display().to_string(),
        bytes: bytes.len() as u64 + 1,
        record_counts: envelope.record_counts,
    })
}

fn validate_export_shape(parsed: &Value, envelope: &JsonExportEnvelope) -> Result<(), AppError> {
    let object = parsed
        .as_object()
        .ok_or_else(|| AppError::export_failed("The JSON export is not an object."))?;
    if object
        .get("formatVersion")
        .and_then(Value::as_u64)
        .is_none()
    {
        return Err(AppError::export_failed(
            "The JSON export format version is missing.",
        ));
    }
    let tables = object
        .get("tables")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::export_failed("The JSON export tables are missing."))?;
    for table in tables_for_format(envelope.format_version) {
        let rows = tables
            .get(*table)
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::export_failed("A JSON export table is missing."))?;
        if rows.len() as i64 != envelope.record_counts.get(*table).copied().unwrap_or(-1) {
            return Err(AppError::export_failed(
                "The JSON export table counts did not validate.",
            ));
        }
        if rows.iter().any(|row| !row.is_object()) {
            return Err(AppError::export_failed(
                "The JSON export contains a non-object record.",
            ));
        }
    }
    Ok(())
}

fn table_to_json(connection: &Connection, table: &str) -> Result<Vec<Value>, AppError> {
    let columns = table_columns(connection, table)?;
    let sql = format!("SELECT * FROM {table} ORDER BY rowid");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("A JSON export table could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            let mut object = Map::new();
            for (index, column) in columns.iter().enumerate() {
                let value = row.get_ref(index)?;
                object.insert(column.clone(), sqlite_value_to_json(value));
            }
            Ok(Value::Object(object))
        })
        .map_err(|error| AppError::database("A JSON export table could not be queried", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A JSON export table could not be read", error))
}

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::from(value),
        ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::Array(
            value
                .iter()
                .map(|byte| Value::from(*byte))
                .collect::<Vec<_>>(),
        ),
    }
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>, AppError> {
    let sql = format!("PRAGMA table_info({table})");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("A database table could not be inspected", error))?;
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database("Database columns could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Database columns could not be read", error))
}

fn prepare_export_directory(destination: &Path) -> Result<PathBuf, AppError> {
    if !destination.is_absolute() {
        return Err(AppError::export_failed(
            "Choose an absolute folder for exports.",
        ));
    }
    fs::create_dir_all(destination).map_err(|error| {
        eprintln!("Export folder could not be prepared: {error}");
        AppError::export_failed(
            "The export folder is unavailable or cannot be written. Choose another folder.",
        )
    })?;
    let directory = destination
        .canonicalize()
        .map_err(|_| AppError::export_failed("The export folder is unavailable."))?;
    if backup::is_network_path(&directory) {
        return Err(AppError::export_failed(
            "Network folders are not supported for export. Choose a local or removable-drive folder so study data is not transmitted over a network.",
        ));
    }
    Ok(directory)
}

fn unique_folder_name(destination: &Path, kind: &str) -> Result<String, AppError> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    for counter in 0..=1_000 {
        let suffix = if counter == 0 {
            String::new()
        } else {
            format!("-{counter}")
        };
        let name = format!("csir-net-tracker-{kind}-{stamp}{suffix}");
        if !destination.join(&name).exists() {
            return Ok(name);
        }
    }
    Err(AppError::export_failed(
        "A unique export folder name could not be created.",
    ))
}

fn unique_export_file_name(
    destination: &Path,
    kind: &str,
    extension: &str,
) -> Result<String, AppError> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    for counter in 0..=1_000 {
        let suffix = if counter == 0 {
            String::new()
        } else {
            format!("-{counter}")
        };
        let name = format!("csir-net-tracker-{kind}-{stamp}{suffix}.{extension}");
        if !destination.join(&name).exists() {
            return Ok(name);
        }
    }
    Err(AppError::export_failed(
        "A unique export file name could not be created.",
    ))
}

pub fn preview_json_import(
    connection: &Connection,
    path: &Path,
) -> Result<JsonImportPreview, AppError> {
    let (bytes, sha256) = read_import_file(path)?;
    let envelope = parse_envelope(&bytes)?;
    validate_envelope_with_temporary_database(&envelope)?;
    let import_batch_id = effective_import_batch_id(&envelope, &sha256)?;
    let already_imported = lookup_import_batch(connection, &import_batch_id)?;
    let existing_counts = existing_table_counts(connection)?;
    let conflicts = detect_merge_conflicts(connection, &envelope)?;
    let mut warnings = vec![
        "Every record is validated in a temporary database before any application data is changed."
            .to_owned(),
        "A verified safety backup of the current database is created before import.".to_owned(),
    ];
    if envelope.format_version < 4 {
        warnings.push(
            "This legacy format does not contain exam profiles. Local profiles are preserved, and its old CSIR date is not applied as a confirmed profile date."
                .to_owned(),
        );
    }
    if envelope.format_version < 3
        && [
            "revision_plan_items",
            "revision_daily_plans",
            "revision_daily_plan_items",
            "revision_deferrals",
            "revision_session_links",
        ]
        .iter()
        .any(|table| existing_counts.get(*table).copied().unwrap_or(0) > 0)
    {
        warnings.push(
            "This legacy format does not describe daily revision plans or deferrals. Merge preserves local plan history; Replace clears it because plan IDs cannot be reconstructed."
                .to_owned(),
        );
    }
    if format_includes_resources(envelope.format_version) {
        let invalid_local = invalid_local_file_rows(&envelope.tables["resource_local_files"]);
        if invalid_local > 0 {
            warnings.push(format!(
                "{invalid_local} local PDF reference(s) are missing, have a different size, or fail SHA-256 verification. They will be skipped; the resource catalogue and study progress remain."
            ));
        }
        if existing_counts.get("study_resources").copied().unwrap_or(0) > 0
            || existing_counts
                .get("resource_local_files")
                .copied()
                .unwrap_or(0)
                > 0
        {
            warnings.push(
                "Complete replacement, if selected, removes current resource records, study progress, and local-file references. PDF files on disk are not deleted."
                    .to_owned(),
            );
        }
    }
    if let Some(previous) = &already_imported {
        warnings.push(format!(
            "This exact export was already imported with the {} strategy on {}. Merge is disabled to prevent duplicate sessions and tasks.",
            previous.strategy, previous.imported_at
        ));
    }
    let unknown_topics = imported_topic_codes(&envelope)
        .into_iter()
        .filter(|code| {
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM syllabus_topics WHERE code = ?1)",
                    [code],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
                == 0
        })
        .count();
    if unknown_topics > 0 {
        warnings.push(format!(
            "{unknown_topics} imported topic code(s) are not in this installation's syllabus catalog. Related merge rows will be rejected; replacement preserves the exported catalog."
        ));
    }
    let merge_supported = already_imported.is_none();

    Ok(JsonImportPreview {
        file_name: path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "backup.json".to_owned()),
        file_sha256: sha256,
        format_version: envelope.format_version,
        application_version: envelope.application_version,
        schema_version: envelope.schema_version,
        exported_at: envelope.exported_at,
        database_id: envelope.database_id,
        import_batch_id,
        already_imported,
        record_counts: envelope.record_counts,
        existing_counts,
        conflicts,
        warnings,
        merge_supported,
        replacement_supported: true,
        merge_description: if envelope.format_version >= 4 {
            "Merge appends study history under new local IDs, matches syllabus topics by stable code, matches resources by resource key, preserves existing resource progress/notes/local PDF references, keeps current settings, timer state, revision rules, dashboard layout, local exam profiles/dates, and local revision plan history. Daily plan versions, deferrals, and session links from the export are not merged because their IDs are installation-specific; use Replace for exact plan history.".to_owned()
        } else if format_includes_resources(envelope.format_version) {
            "Merge appends study history under new local IDs, matches syllabus topics by stable code, matches resources by resource key, preserves existing resource progress/notes/local PDF references, keeps current settings, timer state, revision rules, dashboard layout, and local revision plan history. Daily plan versions, deferrals, and session links from the export are not merged because their IDs are installation-specific; use Replace for exact plan history.".to_owned()
        } else {
            "Merge appends study history under new local IDs, matches syllabus topics by stable code, skips conflicting progress/schedules/import-key rows, keeps the current settings, timer state, revision rules, dashboard layout, and local revision plan history, and does not merge exported plan versions. This format version has no resource tables.".to_owned()
        },
        replacement_description: if envelope.format_version >= 4 {
            "Replace removes current exam profiles, core records, revision plan versions/deferrals/session links, resources, study progress, and local-file references, then inserts the validated export in foreign-key order. Missing local PDF references are skipped; PDF files on disk are never deleted. Current backup settings and the local database identifier are kept.".to_owned()
        } else if envelope.format_version >= 3 {
            "Replace removes current core records, revision plan versions/deferrals/session links, resources, study progress, and local-file references, then inserts the validated export in foreign-key order. Local exam profiles are preserved because this format has no profile data. Missing local PDF references are skipped; PDF files on disk are never deleted. Current backup settings and the local database identifier are kept.".to_owned()
        } else if format_includes_resources(envelope.format_version) {
            "Replace removes current core records, revision plan versions/deferrals/session links, resources, study progress, and local-file references, then inserts the validated legacy export in foreign-key order. Legacy formats cannot reconstruct daily plan history, so it is cleared. Missing local PDF references are skipped; PDF files on disk are never deleted. Current backup settings and the local database identifier are kept.".to_owned()
        } else {
            "Replace removes all current core user records and revision plan versions/deferrals/session links, then inserts the validated core export, preserving exported IDs and relationships. Resource metadata is not touched by this legacy format. Current backup settings and the local database identifier are kept.".to_owned()
        },
    })
}

fn read_import_file(path: &Path) -> Result<(Vec<u8>, String), AppError> {
    let path = path
        .canonicalize()
        .map_err(|_| AppError::import_failed("The selected JSON export is unavailable."))?;
    let metadata = fs::metadata(&path)
        .map_err(|_| AppError::import_failed("The selected JSON export could not be inspected."))?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_IMPORT_BYTES {
        return Err(AppError::import_failed(
            "The selected JSON export has an invalid size.",
        ));
    }
    let bytes = fs::read(&path)
        .map_err(|_| AppError::import_failed("The selected JSON export could not be read."))?;
    let sha = sha256_bytes(&bytes);
    Ok((bytes, sha))
}

fn parse_envelope(bytes: &[u8]) -> Result<JsonExportEnvelope, AppError> {
    let envelope: JsonExportEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| AppError::import_failed("The selected JSON export is not valid JSON."))?;
    if !(MIN_JSON_IMPORT_FORMAT_VERSION..=JSON_EXPORT_FORMAT_VERSION)
        .contains(&envelope.format_version)
    {
        return Err(AppError::import_failed(
            "This JSON export format version is not supported.",
        ));
    }
    if envelope.format_version >= 4 {
        if envelope.schema_version != db::LATEST_SCHEMA_VERSION {
            return Err(AppError::import_failed(format!(
                "JSON format version {} requires schema version {}; this export uses schema {}.",
                envelope.format_version,
                db::LATEST_SCHEMA_VERSION,
                envelope.schema_version
            )));
        }
    } else if envelope.format_version == 3 {
        if envelope.schema_version != 11 {
            return Err(AppError::import_failed(
                "JSON format version 3 requires schema version 11.",
            ));
        }
    } else if envelope.format_version == 2 {
        if envelope.schema_version != 10 {
            return Err(AppError::import_failed(
                "JSON format version 2 requires schema version 10.",
            ));
        }
    } else if !(MIN_JSON_IMPORT_SCHEMA_VERSION..=db::LATEST_SCHEMA_VERSION)
        .contains(&envelope.schema_version)
    {
        return Err(AppError::import_failed(format!(
            "This JSON export uses schema version {}. JSON import supports schemas {} through {}; use the SQLite backup restore workflow for older databases.",
            envelope.schema_version,
            MIN_JSON_IMPORT_SCHEMA_VERSION,
            db::LATEST_SCHEMA_VERSION
        )));
    }
    if !is_hex_identifier(&envelope.database_id) {
        return Err(AppError::import_failed(
            "The JSON export database identifier is invalid.",
        ));
    }
    if let Some(batch_id) = &envelope.import_batch_id
        && !is_hex_identifier(batch_id)
    {
        return Err(AppError::import_failed(
            "The JSON export import-batch identifier is invalid.",
        ));
    }
    let expected_tables = tables_for_format(envelope.format_version);
    for table in expected_tables {
        let rows = envelope
            .tables
            .get(*table)
            .ok_or_else(|| AppError::import_failed("A required JSON export table is missing."))?;
        if rows.len() as i64 != envelope.record_counts.get(*table).copied().unwrap_or(-1) {
            return Err(AppError::import_failed(
                "The JSON export record counts do not match its tables.",
            ));
        }
    }
    if envelope
        .tables
        .keys()
        .any(|table| !expected_tables.contains(&table.as_str()))
    {
        return Err(AppError::import_failed(
            "The JSON export contains an unknown table.",
        ));
    }
    if envelope.format_version >= 4 {
        validate_exam_profiles(&envelope.tables)?;
    }
    if format_includes_resources(envelope.format_version) {
        validate_resource_identities(&envelope.tables)?;
    }
    for singleton in [
        "settings",
        "timer_settings",
        "timer_state",
        "revision_settings",
    ] {
        if envelope.tables[singleton].len() != 1 {
            return Err(AppError::import_failed(
                "The JSON export is missing a required singleton configuration record.",
            ));
        }
    }
    if envelope.tables["dashboard_sections"].len() != 9 {
        return Err(AppError::import_failed(
            "The JSON export must contain the complete dashboard layout.",
        ));
    }
    Ok(envelope)
}

fn validate_envelope_with_temporary_database(
    envelope: &JsonExportEnvelope,
) -> Result<(), AppError> {
    let mut connection = db::in_memory_connection()?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The JSON validation transaction could not be started",
            error,
        )
    })?;
    let tables = tables_for_format(envelope.format_version);
    for table in tables {
        validate_table_rows(
            &transaction,
            table,
            &envelope.tables[*table],
            envelope.format_version >= 3,
        )?;
    }
    transaction
        .execute_batch(
            "DROP TRIGGER IF EXISTS revision_history_no_update;
             DROP TRIGGER IF EXISTS revision_history_no_delete;
             DROP TRIGGER IF EXISTS revision_deferrals_no_update;
             DROP TRIGGER IF EXISTS revision_deferrals_no_delete;
             DROP TRIGGER IF EXISTS revision_daily_plan_items_insert_active;
             DROP TRIGGER IF EXISTS revision_daily_plan_items_no_delete;
             DROP TRIGGER IF EXISTS revision_daily_plans_no_delete;
             DROP TRIGGER IF EXISTS revision_plan_items_completed_no_delete;",
        )
        .map_err(|error| {
            AppError::database("JSON validation protection could not be prepared", error)
        })?;
    for table in delete_order_for_format(envelope.format_version) {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| {
                AppError::database("Validation defaults could not be cleared", error)
            })?;
    }
    insert_all_tables(&transaction, &envelope.tables, tables)?;
    verify_foreign_keys(&transaction)?;
    transaction.rollback().map_err(|error| {
        AppError::database("The JSON validation transaction could not close", error)
    })
}

fn validate_exam_profiles(tables: &BTreeMap<String, Vec<Value>>) -> Result<(), AppError> {
    let exams = &tables["exams"];
    let profiles = &tables["exam_profiles"];
    if exams.len() != 4 || profiles.len() != 4 {
        return Err(AppError::import_failed(
            "A format 4 export must contain exactly four exam profiles.",
        ));
    }

    let mut exam_ids = HashSet::new();
    let mut codes = HashSet::new();
    for row in exams {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("An exam record has no valid identifier."))?;
        let code = row
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::import_failed("An exam record has no stable code."))?;
        if !exam_ids.insert(id) || !codes.insert(code.to_owned()) {
            return Err(AppError::import_failed(
                "The export contains duplicate exam identifiers or codes.",
            ));
        }
        if row.get("display_order").and_then(Value::as_i64).is_none() {
            return Err(AppError::import_failed(
                "An exam record has no display order.",
            ));
        }
    }

    let expected_codes = [
        exams::CSIR_EXAM_CODE,
        "gate-physics",
        "tifr-gs-physics",
        "jest-physics",
    ];
    if expected_codes.iter().any(|code| !codes.contains(*code)) {
        return Err(AppError::import_failed(
            "The export is missing one of the four built-in examinations.",
        ));
    }

    let mut profile_ids = HashSet::new();
    for row in profiles {
        let exam_id = row
            .get("exam_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("An exam profile has no exam identifier."))?;
        if !exam_ids.contains(&exam_id) || !profile_ids.insert(exam_id) {
            return Err(AppError::import_failed(
                "An exam profile references an unknown or duplicate examination.",
            ));
        }
        let active = row
            .get("is_active")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("An exam profile has no active state."))?;
        if active != 0 && active != 1 {
            return Err(AppError::import_failed(
                "An exam profile has an invalid active state.",
            ));
        }
        let notice = row
            .get("legacy_date_notice_state")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::import_failed("An exam profile has no legacy-date notice state.")
            })?;
        if !matches!(notice, "none" | "pending" | "dismissed" | "transferred") {
            return Err(AppError::import_failed(
                "An exam profile has an invalid legacy-date notice state.",
            ));
        }
        let exam_date = row.get("exam_date").and_then(Value::as_str);
        if row.get("exam_date").is_some_and(|value| !value.is_null()) && exam_date.is_none() {
            return Err(AppError::import_failed(
                "An exam profile has an invalid exam date.",
            ));
        }
        if let Some(date) = exam_date
            && date_only::parse_date_only(date).is_none()
        {
            return Err(AppError::import_failed(
                "An exam profile has an invalid calendar date.",
            ));
        }
        if notice == "pending" && exam_date.is_some() {
            return Err(AppError::import_failed(
                "A pending legacy-date notice cannot have a confirmed exam date.",
            ));
        }
    }

    Ok(())
}

fn validate_resource_identities(tables: &BTreeMap<String, Vec<Value>>) -> Result<(), AppError> {
    let resources = &tables["study_resources"];
    let mut resource_ids = HashSet::new();
    let mut resource_keys = HashSet::new();
    for row in resources {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A resource id is invalid."))?;
        let key = row
            .get("resource_key")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::import_failed("A resource key is invalid."))?;
        if !resource_ids.insert(id) {
            return Err(AppError::import_failed(
                "The JSON export contains duplicate resource ids.",
            ));
        }
        if !resource_keys.insert(key.to_owned()) {
            return Err(AppError::import_failed(
                "The JSON export contains duplicate resource keys.",
            ));
        }
    }

    let mut progress_ids = HashSet::new();
    for row in &tables["resource_progress"] {
        let resource_id = row
            .get("resource_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A resource progress id is invalid."))?;
        if !resource_ids.contains(&resource_id) {
            return Err(AppError::import_failed(
                "A resource progress row refers to a missing resource.",
            ));
        }
        if !progress_ids.insert(resource_id) {
            return Err(AppError::import_failed(
                "The JSON export contains duplicate resource progress rows.",
            ));
        }
    }

    let mut local_ids = HashSet::new();
    let mut local_paths = HashSet::new();
    for row in &tables["resource_local_files"] {
        let resource_id = row
            .get("resource_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A local file resource id is invalid."))?;
        let file_path = row
            .get("file_path")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::import_failed("A local file path is invalid."))?;
        if !resource_ids.contains(&resource_id) {
            return Err(AppError::import_failed(
                "A local PDF reference refers to a missing resource.",
            ));
        }
        if !local_ids.insert(resource_id) {
            return Err(AppError::import_failed(
                "The JSON export contains duplicate local PDF references.",
            ));
        }
        if !local_paths.insert(file_path.to_owned()) {
            return Err(AppError::import_failed(
                "The JSON export contains duplicate local PDF paths.",
            ));
        }
    }
    Ok(())
}

fn validate_table_rows(
    connection: &Connection,
    table: &str,
    rows: &[Value],
    require_complete_rows: bool,
) -> Result<(), AppError> {
    let columns = table_columns(connection, table)?;
    let allowed = columns.iter().collect::<HashSet<_>>();
    for row in rows {
        let object = row
            .as_object()
            .ok_or_else(|| AppError::import_failed("A JSON export row is not an object."))?;
        if require_complete_rows && object.len() != columns.len() {
            return Err(AppError::import_failed(format!(
                "A record in table {table} does not contain every exported field."
            )));
        }
        for column in object.keys() {
            if !allowed.contains(column) {
                return Err(AppError::import_failed(format!(
                    "A record in table {table} contains an unknown field."
                )));
            }
        }
        if table == "revision_deferrals" {
            for field in ["previous_due_date", "new_due_date", "occurred_on"] {
                let valid = object
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|value| date_only::parse_date_only(value).is_some());
                if !valid {
                    return Err(AppError::import_failed(format!(
                        "A revision deferral contains an invalid {field} date."
                    )));
                }
            }
        }
    }
    Ok(())
}

fn json_to_sql(value: &Value, table: &str, column: &str) -> Result<SqlValue, AppError> {
    match value {
        Value::Null => Ok(SqlValue::Null),
        Value::Bool(value) => Ok(SqlValue::Integer(i64::from(*value))),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                Ok(SqlValue::Integer(integer))
            } else if let Some(real) = number.as_f64() {
                Ok(SqlValue::Real(real))
            } else {
                Err(AppError::import_failed(format!(
                    "A numeric value in {table}.{column} is invalid."
                )))
            }
        }
        Value::String(value) => Ok(SqlValue::Text(value.clone())),
        _ => Err(AppError::import_failed(format!(
            "A value in {table}.{column} has an unsupported JSON type."
        ))),
    }
}

fn insert_all_tables(
    transaction: &Transaction<'_>,
    tables: &BTreeMap<String, Vec<Value>>,
    table_order: &[&str],
) -> Result<(), AppError> {
    for table in table_order {
        insert_rows(transaction, table, &tables[*table], false)?;
    }
    Ok(())
}

fn insert_rows(
    transaction: &Transaction<'_>,
    table: &str,
    rows: &[Value],
    ignore_conflicts: bool,
) -> Result<usize, AppError> {
    if rows.is_empty() {
        return Ok(0);
    }
    let table_columns = table_columns(transaction, table)?;
    let mut columns = Vec::new();
    for column in table_columns {
        let present = rows.iter().all(|row| row.get(&column).is_some());
        let absent = rows.iter().all(|row| row.get(&column).is_none());
        if present {
            columns.push(column);
        } else if !absent {
            return Err(AppError::import_failed(format!(
                "A record in table {table} uses an inconsistent set of fields."
            )));
        }
    }
    if columns.is_empty() {
        return Err(AppError::import_failed(format!(
            "A record in table {table} does not contain any known fields."
        )));
    }
    let insert = if ignore_conflicts {
        "INSERT OR IGNORE"
    } else {
        "INSERT"
    };
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "{insert} INTO {table} ({}) VALUES ({placeholders})",
        columns.join(", ")
    );
    let mut statement = transaction.prepare(&sql).map_err(|error| {
        AppError::database("A JSON import statement could not be prepared", error)
    })?;
    let mut imported = 0;
    for row in rows {
        let object = row
            .as_object()
            .ok_or_else(|| AppError::import_failed("A JSON export row is not an object."))?;
        let values = columns
            .iter()
            .map(|column| {
                let value = object
                    .get(column)
                    .ok_or_else(|| AppError::import_failed("A JSON export field is missing."))?;
                json_to_sql(value, table, column)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let changed = statement
            .execute(params_from_iter(values.iter()))
            .map_err(|error| {
                eprintln!("A JSON import record failed database validation: {error}");
                AppError::import_failed(
                    "The JSON export contains a record that violates database constraints or duplicates an identifier.",
                )
            })?;
        imported += changed;
    }
    Ok(imported)
}

fn verify_foreign_keys(connection: &Connection) -> Result<(), AppError> {
    let mut statement = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|error| AppError::database("Relation verification could not prepare", error))?;
    let mut rows = statement
        .query([])
        .map_err(|error| AppError::database("Relation verification could not run", error))?;
    if rows
        .next()
        .map_err(|error| AppError::database("Relation verification could not read", error))?
        .is_some()
    {
        return Err(AppError::import_failed(
            "The export contains records that violate database relationships.",
        ));
    }
    Ok(())
}

fn existing_table_counts(connection: &Connection) -> Result<BTreeMap<String, i64>, AppError> {
    let mut counts = backup::table_counts(connection)?;
    for table in RESOURCE_TABLES {
        let count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| AppError::database("Resource counts could not be read", error))?;
        counts.insert(table.to_owned(), count);
    }
    Ok(counts)
}

fn invalid_local_file_rows(rows: &[Value]) -> i64 {
    rows.iter()
        .filter(|row| !local_file_import_is_valid(row))
        .count() as i64
}

fn local_file_import_is_valid(row: &Value) -> bool {
    let Some(path) = row.get("file_path").and_then(Value::as_str) else {
        return false;
    };
    let Some(expected_size) = row.get("size_bytes").and_then(Value::as_i64) else {
        return false;
    };
    let Some(expected_sha) = row.get("sha256").and_then(Value::as_str) else {
        return false;
    };
    let path = Path::new(path);
    if !path.is_absolute() || !path.is_file() {
        return false;
    }
    if !path
        .metadata()
        .is_ok_and(|metadata| metadata.len() as i64 == expected_size)
    {
        return false;
    }
    sha256_file(path).is_some_and(|actual| actual == expected_sha)
}

fn sha256_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

fn detect_merge_conflicts(
    connection: &Connection,
    envelope: &JsonExportEnvelope,
) -> Result<Vec<ImportTableConflict>, AppError> {
    let mut conflicts = Vec::new();
    for table in [
        "settings",
        "timer_settings",
        "timer_state",
        "revision_settings",
        "dashboard_sections",
    ] {
        let rows = envelope.tables.get(table).map_or(0, Vec::len) as i64;
        if rows > 0 {
            conflicts.push(ImportTableConflict {
                table: table.to_owned(),
                rows,
                reason:
                    "Merge keeps the current installation configuration instead of overwriting it."
                        .to_owned(),
            });
        }
    }

    if envelope.format_version >= 4 {
        for table in ["exams", "exam_profiles"] {
            let rows = envelope.tables.get(table).map_or(0, Vec::len) as i64;
            if rows > 0 {
                conflicts.push(ImportTableConflict {
                    table: table.to_owned(),
                    rows,
                    reason:
                        "Merge keeps local exam profiles, activation states and dates instead of overwriting them."
                            .to_owned(),
                });
            }
        }
    }

    let existing_pyq = existing_values(connection, "pyq_attempts", "import_key")?;
    let imported_pyq = rows_with_existing_key(
        &envelope.tables["pyq_attempts"],
        "import_key",
        &existing_pyq,
    );
    if imported_pyq > 0 {
        conflicts.push(ImportTableConflict {
            table: "pyq_attempts".to_owned(),
            rows: imported_pyq,
            reason:
                "Matching import keys are skipped so existing PYQ attempts are not overwritten."
                    .to_owned(),
        });
    }

    let existing_mocks = existing_values(connection, "mock_tests", "import_key")?;
    let imported_mocks = rows_with_existing_key(
        &envelope.tables["mock_tests"],
        "import_key",
        &existing_mocks,
    );
    if imported_mocks > 0 {
        conflicts.push(ImportTableConflict {
            table: "mock_tests".to_owned(),
            rows: imported_mocks,
            reason: "Matching import keys are skipped so existing mock tests are not overwritten."
                .to_owned(),
        });
    }

    let imported_topic_map = imported_topics_by_id(&envelope.tables["syllabus_topics"])?;
    let current_topic_map = current_topic_ids_by_code(connection)?;
    let imported_progress = envelope.tables["syllabus_progress"]
        .iter()
        .filter(|row| {
            let topic_id = row.get("topic_id").and_then(Value::as_i64);
            topic_id
                .and_then(|id| imported_topic_map.get(&id))
                .and_then(|code| current_topic_map.get(code))
                .is_some_and(|current_id| {
                    connection
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM syllabus_progress WHERE topic_id = ?1)",
                            [current_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .unwrap_or(0)
                        != 0
                })
        })
        .count() as i64;
    if imported_progress > 0 {
        conflicts.push(ImportTableConflict {
            table: "syllabus_progress".to_owned(),
            rows: imported_progress,
            reason: "Topics that already have progress keep the current progress row.".to_owned(),
        });
    }

    let imported_schedules = envelope.tables["revision_schedules"]
        .iter()
        .filter(|row| {
            let topic_id = row.get("topic_id").and_then(Value::as_i64);
            topic_id
                .and_then(|id| imported_topic_map.get(&id))
                .and_then(|code| current_topic_map.get(code))
                .is_some_and(|current_id| {
                    connection
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM revision_schedules WHERE topic_id = ?1)",
                            [current_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .unwrap_or(0)
                        != 0
                })
        })
        .count() as i64;
    if imported_schedules > 0 {
        conflicts.push(ImportTableConflict {
            table: "revision_schedules".to_owned(),
            rows: imported_schedules,
            reason: "Topics that already have a revision schedule keep the current schedule."
                .to_owned(),
        });
    }

    if format_includes_resources(envelope.format_version) {
        let existing_keys = existing_values(connection, "study_resources", "resource_key")?;
        let matching_resources = rows_with_existing_key(
            &envelope.tables["study_resources"],
            "resource_key",
            &existing_keys,
        );
        if matching_resources > 0 {
            conflicts.push(ImportTableConflict {
                table: "study_resources".to_owned(),
                rows: matching_resources,
                reason: "Matching resource keys are kept unchanged so existing study progress and local PDF references are not overwritten."
                    .to_owned(),
            });
        }

        let imported_keys = envelope.tables["study_resources"]
            .iter()
            .filter_map(|row| {
                Some((
                    row.get("id")?.as_i64()?,
                    row.get("resource_key")?.as_str()?.to_owned(),
                ))
            })
            .collect::<HashMap<_, _>>();
        let imported_progress = envelope.tables["resource_progress"]
            .iter()
            .filter(|row| {
                row.get("resource_id")
                    .and_then(Value::as_i64)
                    .and_then(|id| imported_keys.get(&id))
                    .is_some_and(|key| existing_keys.contains(key))
            })
            .count() as i64;
        if imported_progress > 0 {
            conflicts.push(ImportTableConflict {
                table: "resource_progress".to_owned(),
                rows: imported_progress,
                reason: "Resources that already exist keep their current study status and personal notes."
                    .to_owned(),
            });
        }

        let imported_local = envelope.tables["resource_local_files"]
            .iter()
            .filter(|row| {
                row.get("resource_id")
                    .and_then(Value::as_i64)
                    .and_then(|id| imported_keys.get(&id))
                    .is_some_and(|key| existing_keys.contains(key))
            })
            .count() as i64;
        if imported_local > 0 {
            conflicts.push(ImportTableConflict {
                table: "resource_local_files".to_owned(),
                rows: imported_local,
                reason: "Resources that already exist keep their current local PDF reference."
                    .to_owned(),
            });
        }

        let invalid_local = invalid_local_file_rows(&envelope.tables["resource_local_files"]);
        if invalid_local > 0 {
            conflicts.push(ImportTableConflict {
                table: "resource_local_files".to_owned(),
                rows: invalid_local,
                reason: "Missing, size-mismatched, or checksum-mismatched local PDF references will be skipped."
                    .to_owned(),
            });
        }
    }

    Ok(conflicts)
}

fn existing_values(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<HashSet<String>, AppError> {
    let sql = format!("SELECT {column} FROM {table} WHERE {column} IS NOT NULL");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Import conflict data could not be prepared", error))?;
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| AppError::database("Import conflict data could not be queried", error))?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Import conflict data could not be read", error))
}

fn rows_with_existing_key(rows: &[Value], column: &str, existing: &HashSet<String>) -> i64 {
    rows.iter()
        .filter(|row| {
            row.get(column)
                .and_then(Value::as_str)
                .is_some_and(|value| existing.contains(value))
        })
        .count() as i64
}

fn imported_topics_by_id(rows: &[Value]) -> Result<HashMap<i64, String>, AppError> {
    let mut result = HashMap::new();
    for row in rows {
        let id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("An imported topic id is invalid."))?;
        let code = row
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::import_failed("An imported topic code is invalid."))?;
        result.insert(id, code.to_owned());
    }
    Ok(result)
}

fn imported_topic_codes(envelope: &JsonExportEnvelope) -> HashSet<String> {
    envelope.tables["syllabus_topics"]
        .iter()
        .filter_map(|row| row.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn current_topic_ids_by_code(connection: &Connection) -> Result<HashMap<String, i64>, AppError> {
    let mut statement = connection
        .prepare("SELECT code, id FROM syllabus_topics")
        .map_err(|error| AppError::database("Syllabus identities could not be prepared", error))?;
    statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| AppError::database("Syllabus identities could not be queried", error))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(|error| AppError::database("Syllabus identities could not be read", error))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_hex_identifier(value: &str) -> bool {
    value.len() == 32
        && value == value.to_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn effective_import_batch_id(
    envelope: &JsonExportEnvelope,
    file_sha256: &str,
) -> Result<String, AppError> {
    if let Some(batch_id) = &envelope.import_batch_id {
        if !is_hex_identifier(batch_id) {
            return Err(AppError::import_failed(
                "The JSON export import-batch identifier is invalid.",
            ));
        }
        return Ok(batch_id.clone());
    }
    Ok(file_sha256[..32].to_owned())
}

fn lookup_import_batch(
    connection: &Connection,
    batch_id: &str,
) -> Result<Option<ImportBatchInfo>, AppError> {
    connection
        .query_row(
            "SELECT imported_at, strategy
             FROM json_import_batches
             WHERE batch_id = ?1",
            [batch_id],
            |row| {
                Ok(ImportBatchInfo {
                    imported_at: row.get(0)?,
                    strategy: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Import batch identity could not be checked", error))
}

fn record_import_batch(
    transaction: &Transaction<'_>,
    batch_id: &str,
    envelope: &JsonExportEnvelope,
    file_sha256: &str,
    strategy: JsonImportStrategy,
    per_table: &BTreeMap<String, ImportTableResult>,
) -> Result<(), AppError> {
    let imported = per_table
        .values()
        .map(|result| result.imported)
        .sum::<i64>();
    let skipped = per_table.values().map(|result| result.skipped).sum::<i64>();
    let conflicting = per_table
        .values()
        .map(|result| result.conflicting)
        .sum::<i64>();
    let rejected = per_table
        .values()
        .map(|result| result.rejected)
        .sum::<i64>();
    transaction
        .execute(
            "INSERT INTO json_import_batches (
                batch_id, source_database_id, file_sha256, strategy,
                imported_at, imported_count, skipped_count,
                conflicting_count, rejected_count
             ) VALUES (
                ?1, ?2, ?3, ?4,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?5, ?6,
                ?7, ?8
             )
             ON CONFLICT(batch_id) DO UPDATE SET
                source_database_id = excluded.source_database_id,
                file_sha256 = excluded.file_sha256,
                strategy = excluded.strategy,
                imported_at = excluded.imported_at,
                imported_count = excluded.imported_count,
                skipped_count = excluded.skipped_count,
                conflicting_count = excluded.conflicting_count,
                rejected_count = excluded.rejected_count",
            params_from_iter([
                SqlValue::Text(batch_id.to_owned()),
                SqlValue::Text(envelope.database_id.clone()),
                SqlValue::Text(file_sha256.to_owned()),
                SqlValue::Text(match strategy {
                    JsonImportStrategy::Merge => "merge".to_owned(),
                    JsonImportStrategy::Replace => "replace".to_owned(),
                }),
                SqlValue::Integer(imported),
                SqlValue::Integer(skipped),
                SqlValue::Integer(conflicting),
                SqlValue::Integer(rejected),
            ]),
        )
        .map_err(|error| AppError::database("Import batch metadata could not be stored", error))?;
    Ok(())
}

pub fn apply_json_import(
    connection: &mut Connection,
    path: &Path,
    strategy: JsonImportStrategy,
    expected_sha256: &str,
    data_dir: &Path,
) -> Result<JsonImportResult, AppError> {
    let (bytes, sha256) = read_import_file(path)?;
    if sha256 != expected_sha256 {
        return Err(AppError::import_failed(
            "The selected JSON export changed after preview. Preview it again before importing.",
        ));
    }
    let envelope = parse_envelope(&bytes)?;
    validate_envelope_with_temporary_database(&envelope)?;
    let batch_id = effective_import_batch_id(&envelope, &sha256)?;
    if strategy == JsonImportStrategy::Merge
        && let Some(previous) = lookup_import_batch(connection, &batch_id)?
    {
        return Err(AppError::import_failed(format!(
            "This exact JSON export was already imported with the {} strategy on {}. Merge was blocked to prevent duplicate sessions and tasks. Choose complete replacement only if you intend to replace current data.",
            previous.strategy, previous.imported_at
        )));
    }

    let reason = match strategy {
        JsonImportStrategy::Merge => "pre-json-import",
        JsonImportStrategy::Replace => "pre-json-replace",
    };
    let safety = backup::create_safety_backup(connection, data_dir, reason, None)?;
    let _ = backup::record_success(connection, &safety.manifest);

    let mut per_table = empty_import_counts(envelope.format_version);
    let merge_warnings = match strategy {
        JsonImportStrategy::Replace => {
            replace_from_json(connection, &envelope, &sha256, &batch_id, &mut per_table)?;
            if envelope.format_version < 4 {
                refresh_legacy_exam_date_notice(connection)?;
            }
            Vec::new()
        }
        JsonImportStrategy::Merge => {
            merge_from_json(connection, &envelope, &sha256, &batch_id, &mut per_table)?
        }
    };

    let mut imported = 0_i64;
    let mut skipped = 0_i64;
    let mut conflicting = 0_i64;
    let mut rejected = 0_i64;
    for result in per_table.values() {
        imported += result.imported;
        skipped += result.skipped;
        conflicting += result.conflicting;
        rejected += result.rejected;
    }

    let mut warnings = merge_warnings;
    let skipped_local = per_table
        .get("resource_local_files")
        .map_or(0, |result| result.skipped);
    if skipped_local > 0 {
        warnings.push(format!(
            "{skipped_local} local PDF reference(s) were skipped because the file was missing, changed, or already belonged to another resource. The resource catalogue and study progress remain available."
        ));
    }
    warnings.push("The import ran in one transaction and was verified with SQLite foreign-key checks before commit.".to_owned());
    warnings.push(
        "Reload the application after import so every screen reads the restored data.".to_owned(),
    );

    Ok(JsonImportResult {
        strategy: match strategy {
            JsonImportStrategy::Merge => "merge".to_owned(),
            JsonImportStrategy::Replace => "replace".to_owned(),
        },
        imported,
        skipped,
        conflicting,
        rejected,
        per_table,
        safety_backup_file: safety.file_name,
        batch_id,
        warnings,
    })
}

fn refresh_legacy_exam_date_notice(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE exam_profiles
             SET legacy_date_notice_state = 'pending',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = (SELECT id FROM exams WHERE code = ?1)
               AND exam_date IS NULL
               AND EXISTS (SELECT 1 FROM settings WHERE id = 1)",
            [exams::CSIR_EXAM_CODE],
        )
        .map_err(|error| {
            AppError::database(
                "The imported legacy CSIR date could not be marked unconfirmed",
                error,
            )
        })?;
    Ok(())
}

fn empty_import_counts(format_version: u32) -> BTreeMap<String, ImportTableResult> {
    tables_for_format(format_version)
        .iter()
        .map(|table| {
            (
                (*table).to_owned(),
                ImportTableResult {
                    imported: 0,
                    skipped: 0,
                    conflicting: 0,
                    rejected: 0,
                },
            )
        })
        .collect()
}

fn replace_from_json(
    connection: &mut Connection,
    envelope: &JsonExportEnvelope,
    file_sha256: &str,
    batch_id: &str,
    per_table: &mut BTreeMap<String, ImportTableResult>,
) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The JSON replacement transaction could not be started",
            error,
        )
    })?;
    transaction
        .execute_batch(
            "DROP TRIGGER IF EXISTS revision_history_no_update;
             DROP TRIGGER IF EXISTS revision_history_no_delete;
             DROP TRIGGER IF EXISTS revision_deferrals_no_update;
             DROP TRIGGER IF EXISTS revision_deferrals_no_delete;
             DROP TRIGGER IF EXISTS revision_daily_plan_items_insert_active;
             DROP TRIGGER IF EXISTS revision_daily_plan_items_no_delete;
             DROP TRIGGER IF EXISTS revision_daily_plans_no_delete;
             DROP TRIGGER IF EXISTS revision_plan_items_completed_no_delete;",
        )
        .map_err(|error| {
            AppError::database("Revision-history protection could not be prepared", error)
        })?;
    for table in delete_order_for_format(envelope.format_version) {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| AppError::database("Existing records could not be cleared", error))?;
    }

    for table in tables_for_format(envelope.format_version) {
        let rows = if *table == "resource_local_files" {
            envelope.tables[*table]
                .iter()
                .filter(|row| local_file_import_is_valid(row))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            envelope.tables[*table].clone()
        };
        let imported = insert_rows(&transaction, table, &rows, false)? as i64;
        if imported != rows.len() as i64 {
            return Err(AppError::import_failed(
                "The replacement did not insert every expected record.",
            ));
        }
        let result = per_table.get_mut(*table).expect("table result exists");
        result.imported = imported;
        if *table == "resource_local_files" {
            result.skipped = envelope.tables[*table].len() as i64 - imported;
        }
    }

    transaction
        .execute_batch(
            "CREATE TRIGGER revision_history_no_update
             BEFORE UPDATE ON revision_history
             BEGIN
                 SELECT RAISE(ABORT, 'revision history is append-only');
             END;
             CREATE TRIGGER revision_history_no_delete
             BEFORE DELETE ON revision_history
             BEGIN
                 SELECT RAISE(ABORT, 'revision history is append-only');
             END;
             CREATE TRIGGER revision_deferrals_no_update
             BEFORE UPDATE ON revision_deferrals
             BEGIN
                 SELECT RAISE(ABORT, 'revision deferrals are append-only');
             END;
             CREATE TRIGGER revision_deferrals_no_delete
             BEFORE DELETE ON revision_deferrals
             BEGIN
                 SELECT RAISE(ABORT, 'revision deferrals are append-only');
             END;
             CREATE TRIGGER revision_daily_plan_items_insert_active
             BEFORE INSERT ON revision_daily_plan_items
             WHEN (
                 SELECT status FROM revision_daily_plans
                 WHERE id = NEW.daily_plan_id
             ) <> 'active'
             BEGIN
                 SELECT RAISE(ABORT, 'daily plan memberships require an active plan');
             END;
             CREATE TRIGGER revision_daily_plan_items_no_delete
             BEFORE DELETE ON revision_daily_plan_items
             BEGIN
                 SELECT RAISE(ABORT, 'daily plan memberships are immutable');
             END;
             CREATE TRIGGER revision_daily_plans_no_delete
             BEFORE DELETE ON revision_daily_plans
             BEGIN
                 SELECT RAISE(ABORT, 'daily plan versions are immutable');
             END;
             CREATE TRIGGER revision_plan_items_completed_no_delete
             BEFORE DELETE ON revision_plan_items
             WHEN OLD.status = 'completed'
             BEGIN
                 SELECT RAISE(ABORT, 'completed revision work items cannot be deleted');
             END;",
        )
        .map_err(|error| {
            AppError::database("Revision-history protection could not be restored", error)
        })?;
    record_import_batch(
        &transaction,
        batch_id,
        envelope,
        file_sha256,
        JsonImportStrategy::Replace,
        per_table,
    )?;
    verify_foreign_keys(&transaction)?;
    verify_imported_counts(&transaction, envelope, per_table)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The JSON replacement could not be committed", error))
}

fn verify_imported_counts(
    transaction: &Transaction<'_>,
    envelope: &JsonExportEnvelope,
    per_table: &BTreeMap<String, ImportTableResult>,
) -> Result<(), AppError> {
    for table in tables_for_format(envelope.format_version) {
        let count: i64 = transaction
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| {
                AppError::database("An imported record count could not be verified", error)
            })?;
        let expected = if *table == "resource_local_files" {
            per_table.get(*table).map_or(0, |result| result.imported)
        } else {
            envelope.tables[*table].len() as i64
        };
        if count != expected {
            return Err(AppError::import_failed(
                "An imported table count did not match the validated export.",
            ));
        }
    }
    Ok(())
}

fn merge_from_json(
    connection: &mut Connection,
    envelope: &JsonExportEnvelope,
    file_sha256: &str,
    batch_id: &str,
    per_table: &mut BTreeMap<String, ImportTableResult>,
) -> Result<Vec<String>, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The JSON merge transaction could not be started", error)
    })?;
    let current_topics = current_topic_ids_by_code(&transaction)?;
    let imported_topics = imported_topics_by_id(&envelope.tables["syllabus_topics"])?;
    let mut imported_session_ids = HashMap::new();
    let mut existing_pyq_keys = existing_values(&transaction, "pyq_attempts", "import_key")?;
    let mut existing_mock_keys = existing_values(&transaction, "mock_tests", "import_key")?;
    let mut warnings = Vec::new();

    for table in [
        "settings",
        "timer_settings",
        "timer_state",
        "revision_settings",
        "dashboard_sections",
    ] {
        let rows = envelope.tables[table].len() as i64;
        let result = per_table.get_mut(table).expect("table result exists");
        result.skipped = rows;
    }

    if envelope.format_version >= 4 {
        for table in ["exams", "exam_profiles"] {
            let rows = envelope.tables[table].len() as i64;
            let result = per_table.get_mut(table).expect("table result exists");
            result.skipped = rows;
        }
    }

    for row in &envelope.tables["syllabus_progress"] {
        let Some(topic_id) = map_imported_topic(row, &imported_topics, &current_topics) else {
            per_table
                .get_mut("syllabus_progress")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM syllabus_progress WHERE topic_id = ?1)",
                [topic_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("Topic-progress conflicts could not be checked", error)
            })?;
        if exists != 0 {
            per_table
                .get_mut("syllabus_progress")
                .expect("table result exists")
                .conflicting += 1;
            continue;
        }
        let mut overrides = HashMap::new();
        overrides.insert("topic_id", SqlValue::Integer(topic_id));
        insert_merge_row(&transaction, "syllabus_progress", row, &[], overrides)?;
        per_table
            .get_mut("syllabus_progress")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["study_sessions"] {
        let topic_id = map_nullable_topic(row, "topic_id", &imported_topics, &current_topics);
        if topic_id.is_none() && row.get("topic_id").is_some_and(|value| !value.is_null()) {
            warnings.push(
                "A study session with an unknown topic was imported without a topic link."
                    .to_owned(),
            );
        }
        let imported_id = insert_merge_row(
            &transaction,
            "study_sessions",
            row,
            &["id"],
            HashMap::from([("topic_id", sql_value_or_null(topic_id))]),
        )?;
        if let Some(old_id) = row.get("id").and_then(Value::as_i64) {
            imported_session_ids.insert(old_id, imported_id);
        }
        per_table
            .get_mut("study_sessions")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["checklist_tasks"] {
        let topic_id = map_nullable_topic(row, "topic_id", &imported_topics, &current_topics);
        if topic_id.is_none() && row.get("topic_id").is_some_and(|value| !value.is_null()) {
            warnings.push(
                "A checklist task with an unknown topic was imported without a topic link."
                    .to_owned(),
            );
        }
        insert_merge_row(
            &transaction,
            "checklist_tasks",
            row,
            &["id"],
            HashMap::from([("topic_id", sql_value_or_null(topic_id))]),
        )?;
        per_table
            .get_mut("checklist_tasks")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["planned_study_blocks"] {
        let topic_id = map_nullable_topic(row, "topic_id", &imported_topics, &current_topics);
        let linked_session = row
            .get("linked_session_id")
            .and_then(Value::as_i64)
            .and_then(|id| imported_session_ids.get(&id).copied());
        if row
            .get("linked_session_id")
            .is_some_and(|value| !value.is_null())
            && linked_session.is_none()
        {
            per_table
                .get_mut("planned_study_blocks")
                .expect("table result exists")
                .rejected += 1;
            continue;
        }
        insert_merge_row(
            &transaction,
            "planned_study_blocks",
            row,
            &["id"],
            HashMap::from([
                ("topic_id", sql_value_or_null(topic_id)),
                ("linked_session_id", sql_value_or_null(linked_session)),
            ]),
        )?;
        per_table
            .get_mut("planned_study_blocks")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["pyq_attempts"] {
        if let Some(key) = row.get("import_key").and_then(Value::as_str)
            && existing_pyq_keys.contains(key)
        {
            per_table
                .get_mut("pyq_attempts")
                .expect("table result exists")
                .conflicting += 1;
            continue;
        }
        let Some(topic_id) = map_imported_topic(row, &imported_topics, &current_topics) else {
            per_table
                .get_mut("pyq_attempts")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        insert_merge_row(
            &transaction,
            "pyq_attempts",
            row,
            &["id"],
            HashMap::from([("topic_id", SqlValue::Integer(topic_id))]),
        )?;
        if let Some(key) = row.get("import_key").and_then(Value::as_str) {
            existing_pyq_keys.insert(key.to_owned());
        }
        per_table
            .get_mut("pyq_attempts")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["mock_tests"] {
        if let Some(key) = row.get("import_key").and_then(Value::as_str)
            && existing_mock_keys.contains(key)
        {
            per_table
                .get_mut("mock_tests")
                .expect("table result exists")
                .conflicting += 1;
            continue;
        }
        insert_merge_row(&transaction, "mock_tests", row, &["id"], HashMap::new())?;
        if let Some(key) = row.get("import_key").and_then(Value::as_str) {
            existing_mock_keys.insert(key.to_owned());
        }
        per_table
            .get_mut("mock_tests")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["revision_schedules"] {
        let Some(topic_id) = map_imported_topic(row, &imported_topics, &current_topics) else {
            per_table
                .get_mut("revision_schedules")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM revision_schedules WHERE topic_id = ?1)",
                [topic_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("Revision-schedule conflicts could not be checked", error)
            })?;
        if exists != 0 {
            per_table
                .get_mut("revision_schedules")
                .expect("table result exists")
                .conflicting += 1;
            continue;
        }
        insert_merge_row(
            &transaction,
            "revision_schedules",
            row,
            &[],
            HashMap::from([("topic_id", SqlValue::Integer(topic_id))]),
        )?;
        per_table
            .get_mut("revision_schedules")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["revision_history"] {
        let Some(topic_id) = map_imported_topic(row, &imported_topics, &current_topics) else {
            per_table
                .get_mut("revision_history")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        insert_merge_row(
            &transaction,
            "revision_history",
            row,
            &["id"],
            HashMap::from([("topic_id", SqlValue::Integer(topic_id))]),
        )?;
        per_table
            .get_mut("revision_history")
            .expect("table result exists")
            .imported += 1;
    }

    if envelope.format_version >= 3 {
        let lifecycle_tables = [
            "revision_plan_items",
            "revision_daily_plans",
            "revision_daily_plan_items",
            "revision_deferrals",
            "revision_session_links",
        ];
        for table in lifecycle_tables {
            let rows = envelope.tables[table].len() as i64;
            per_table
                .get_mut(table)
                .expect("table result exists")
                .skipped = rows;
        }
        warnings.push(
            "Daily revision plan versions, deferrals, and revision session links are not merged because plan IDs are installation-specific. Use Replace to restore them exactly; existing local plan history is preserved."
                .to_owned(),
        );
    }

    if format_includes_resources(envelope.format_version) {
        merge_resources(&transaction, envelope, per_table, &mut warnings)?;
    }

    record_import_batch(
        &transaction,
        batch_id,
        envelope,
        file_sha256,
        JsonImportStrategy::Merge,
        per_table,
    )?;
    verify_foreign_keys(&transaction)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The JSON merge could not be committed", error))?;
    Ok(warnings)
}

fn merge_resources(
    transaction: &Transaction<'_>,
    envelope: &JsonExportEnvelope,
    per_table: &mut BTreeMap<String, ImportTableResult>,
    warnings: &mut Vec<String>,
) -> Result<(), AppError> {
    let mut existing_keys = HashMap::new();
    {
        let mut statement = transaction
            .prepare("SELECT resource_key, id FROM study_resources")
            .map_err(|error| AppError::database("Existing resources could not be read", error))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| AppError::database("Existing resources could not be read", error))?;
        for row in rows {
            let (key, id) = row.map_err(|error| {
                AppError::database("Existing resources could not be read", error)
            })?;
            existing_keys.insert(key, id);
        }
    }
    let existing_progress = integer_column_set(transaction, "resource_progress", "resource_id")?;
    let existing_local = integer_column_set(transaction, "resource_local_files", "resource_id")?;
    let mut existing_paths = string_column_set(transaction, "resource_local_files", "file_path")?;
    let mut existing_runs = discovery_run_keys(transaction)?;
    let mut imported_ids = HashMap::new();

    for row in &envelope.tables["study_resources"] {
        let old_id = row
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A resource id is invalid."))?;
        let key = row
            .get("resource_key")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::import_failed("A resource key is invalid."))?;
        if let Some(current_id) = existing_keys.get(key) {
            imported_ids.insert(old_id, *current_id);
            per_table
                .get_mut("study_resources")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        let new_id =
            insert_merge_row(transaction, "study_resources", row, &["id"], HashMap::new())?;
        imported_ids.insert(old_id, new_id);
        per_table
            .get_mut("study_resources")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["resource_progress"] {
        let old_id = row
            .get("resource_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A resource progress id is invalid."))?;
        let Some(new_id) = imported_ids.get(&old_id).copied() else {
            per_table
                .get_mut("resource_progress")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        if existing_progress.contains(&new_id) {
            per_table
                .get_mut("resource_progress")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        insert_merge_row(
            transaction,
            "resource_progress",
            row,
            &[],
            HashMap::from([("resource_id", SqlValue::Integer(new_id))]),
        )?;
        per_table
            .get_mut("resource_progress")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["resource_local_files"] {
        let old_id = row
            .get("resource_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::import_failed("A local file resource id is invalid."))?;
        let Some(new_id) = imported_ids.get(&old_id).copied() else {
            per_table
                .get_mut("resource_local_files")
                .expect("table result exists")
                .rejected += 1;
            continue;
        };
        if existing_local.contains(&new_id) {
            per_table
                .get_mut("resource_local_files")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        if !local_file_import_is_valid(row) {
            warnings.push(
                "A local PDF reference was skipped because the file is missing, has a different size, or fails SHA-256 verification."
                    .to_owned(),
            );
            per_table
                .get_mut("resource_local_files")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        if row
            .get("file_path")
            .and_then(Value::as_str)
            .is_some_and(|path| existing_paths.contains(path))
        {
            warnings.push(
                "A local PDF reference was skipped because that file path is already linked to another resource."
                    .to_owned(),
            );
            per_table
                .get_mut("resource_local_files")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        insert_merge_row(
            transaction,
            "resource_local_files",
            row,
            &["resource_id"],
            HashMap::from([("resource_id", SqlValue::Integer(new_id))]),
        )?;
        if let Some(path) = row.get("file_path").and_then(Value::as_str) {
            existing_paths.insert(path.to_owned());
        }
        per_table
            .get_mut("resource_local_files")
            .expect("table result exists")
            .imported += 1;
    }

    for row in &envelope.tables["resource_discovery_runs"] {
        let started_at = row.get("started_at").and_then(Value::as_str).unwrap_or("");
        let finished_at = row.get("finished_at").and_then(Value::as_str);
        let key = format!("{started_at}|{}", finished_at.unwrap_or(""));
        if existing_runs.contains(&key) {
            per_table
                .get_mut("resource_discovery_runs")
                .expect("table result exists")
                .skipped += 1;
            continue;
        }
        insert_merge_row(
            transaction,
            "resource_discovery_runs",
            row,
            &["id"],
            HashMap::new(),
        )?;
        existing_runs.insert(key);
        per_table
            .get_mut("resource_discovery_runs")
            .expect("table result exists")
            .imported += 1;
    }

    Ok(())
}

fn integer_column_set(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
) -> Result<HashSet<i64>, AppError> {
    let mut statement = transaction
        .prepare(&format!("SELECT {column} FROM {table}"))
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })?;
    statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })
}

fn string_column_set(
    transaction: &Transaction<'_>,
    table: &str,
    column: &str,
) -> Result<HashSet<String>, AppError> {
    let mut statement = transaction
        .prepare(&format!("SELECT {column} FROM {table}"))
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })?;
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| {
            AppError::database("Existing resource references could not be read", error)
        })
}

fn discovery_run_keys(transaction: &Transaction<'_>) -> Result<HashSet<String>, AppError> {
    let mut statement = transaction
        .prepare("SELECT started_at, finished_at FROM resource_discovery_runs")
        .map_err(|error| AppError::database("Discovery runs could not be read", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(format!(
                "{}|{}",
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default()
            ))
        })
        .map_err(|error| AppError::database("Discovery runs could not be read", error))?;
    rows.collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Discovery runs could not be read", error))
}

fn insert_merge_row(
    transaction: &Transaction<'_>,
    table: &str,
    row: &Value,
    omitted: &[&str],
    overrides: HashMap<&str, SqlValue>,
) -> Result<i64, AppError> {
    let object = row
        .as_object()
        .ok_or_else(|| AppError::import_failed("A JSON export row is not an object."))?;
    let columns = table_columns(transaction, table)?
        .into_iter()
        .filter(|column| {
            !omitted.contains(&column.as_str())
                && (object.contains_key(column) || overrides.contains_key(column.as_str()))
        })
        .collect::<Vec<_>>();
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({placeholders})",
        columns.join(", ")
    );
    let values = columns
        .iter()
        .map(|column| {
            if let Some(value) = overrides.get(column.as_str()) {
                return Ok(clone_sql_value(value));
            }
            let value = object
                .get(column)
                .ok_or_else(|| AppError::import_failed("A JSON export field is missing."))?;
            json_to_sql(value, table, column)
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    transaction
        .execute(&sql, params_from_iter(values.iter()))
        .map_err(|error| {
            eprintln!("A JSON merge record failed database validation: {error}");
            AppError::import_failed(
                "A validated JSON merge record could not be inserted because of a database constraint conflict.",
            )
        })?;
    Ok(transaction.last_insert_rowid())
}

fn clone_sql_value(value: &SqlValue) -> SqlValue {
    match value {
        SqlValue::Null => SqlValue::Null,
        SqlValue::Integer(value) => SqlValue::Integer(*value),
        SqlValue::Real(value) => SqlValue::Real(*value),
        SqlValue::Text(value) => SqlValue::Text(value.clone()),
    }
}

fn sql_value_or_null(value: Option<i64>) -> SqlValue {
    value.map_or(SqlValue::Null, SqlValue::Integer)
}

fn map_imported_topic(
    row: &Value,
    imported_topics: &HashMap<i64, String>,
    current_topics: &HashMap<String, i64>,
) -> Option<i64> {
    row.get("topic_id")
        .and_then(Value::as_i64)
        .and_then(|id| imported_topics.get(&id))
        .and_then(|code| current_topics.get(code))
        .copied()
}

fn map_nullable_topic(
    row: &Value,
    column: &str,
    imported_topics: &HashMap<i64, String>,
    current_topics: &HashMap<String, i64>,
) -> Option<i64> {
    row.get(column)
        .and_then(Value::as_i64)
        .and_then(|id| imported_topics.get(&id))
        .and_then(|code| current_topics.get(code))
        .copied()
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, NaiveDate, TimeZone, Utc};
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        exams, mock_tests, planner, pyq, resources, revision, revision_plan, sessions, settings,
        syllabus, timer,
    };

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn database(tmp: &TempDir, name: &str) -> Connection {
        let path = tmp.path().join(name);
        let mut connection =
            db::open_database(&path, date(2026, 10, 30)).expect("database should open");
        syllabus::seed(&mut connection).expect("syllabus should seed");
        connection
    }

    fn trim_legacy_tables(value: &mut Value, format_version: u32) {
        let allowed = tables_for_format(format_version);
        let extra = value["tables"]
            .as_object()
            .expect("tables should be an object")
            .keys()
            .filter(|table| !allowed.contains(&table.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        for table in extra {
            value["tables"].as_object_mut().unwrap().remove(&table);
            value["recordCounts"]
                .as_object_mut()
                .unwrap()
                .remove(&table);
        }
    }

    fn topic_id(connection: &Connection, code: &str) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = ?1",
                [code],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    fn add_pyq(connection: &mut Connection, topic: i64, reference: &str, import_key: Option<&str>) {
        pyq::create(
            connection,
            pyq::PyqInput {
                question_year: 2024,
                exam_session: String::new(),
                part: "B".to_owned(),
                topic_id: topic,
                question_reference: reference.to_owned(),
                attempt_date: "2026-10-29".to_owned(),
                status: "correct".to_owned(),
                marks_obtained: None,
                max_marks: None,
                time_spent_seconds: 60,
                notes: "Quoted, comma\nnotes".to_owned(),
                difficulty: Some(2),
            },
        )
        .expect("PYQ should save");
        if let Some(key) = import_key {
            connection
                .execute(
                    "UPDATE pyq_attempts SET import_key = ?1 WHERE question_reference = ?2",
                    params_from_iter([key, reference]),
                )
                .expect("import key should update");
        }
    }

    fn add_mock(connection: &mut Connection, name: &str, import_key: Option<&str>) {
        mock_tests::create(
            connection,
            mock_tests::MockTestInput {
                test_name: name.to_owned(),
                test_date: "2026-10-28".to_owned(),
                duration_minutes: 180,
                max_marks: 100.0,
                marks_obtained: None,
                part_a: mock_tests::MockPartInput {
                    questions: 2,
                    correct: 1,
                    incorrect: 1,
                    unanswered: 0,
                    correct_marks: Some(2.0),
                    incorrect_marks: Some(-0.5),
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
        if let Some(key) = import_key {
            connection
                .execute(
                    "UPDATE mock_tests SET import_key = ?1 WHERE test_name = ?2",
                    params_from_iter([key, name]),
                )
                .expect("mock import key should update");
        }
    }

    fn add_resource(connection: &Connection, local_path: Option<&Path>) -> resources::Resource {
        let resource = resources::create_manual(
            connection,
            resources::ManualResourceInput {
                title: "Portable resource".to_owned(),
                description: Some("Portable resource description".to_owned()),
                category: "other".to_owned(),
                unit: "Quantum Mechanics".to_owned(),
                topic_code: Some("QM-C01".to_owned()),
                original_url: "https://drive.google.com/file/d/sourceResource/view?usp=sharing"
                    .to_owned(),
                source_page_url: Some(
                    "https://www.csirnetphysics.in/p/hand-written-notes_28.html".to_owned(),
                ),
            },
        )
        .expect("resource should save");
        resources::update_progress(
            connection,
            resource.id,
            resources::ResourceProgressInput {
                status: "studying".to_owned(),
                notes: "Portable resource notes".to_owned(),
            },
            "2026-10-30T12:00:00.000Z",
        )
        .expect("resource progress should save");
        if let Some(path) = local_path {
            attach_local_file(connection, resource.id, path);
        }
        resource
    }

    fn attach_local_file(connection: &Connection, resource_id: i64, path: &Path) {
        let metadata = fs::metadata(path).expect("local PDF should exist");
        let sha256 = sha256_file(path).expect("local PDF should hash");
        connection
            .execute(
                "INSERT INTO resource_local_files (
                    resource_id, file_name, file_path, sha256, size_bytes, imported_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params_from_iter([
                    SqlValue::Integer(resource_id),
                    SqlValue::Text(
                        path.file_name()
                            .expect("file name")
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    SqlValue::Text(path.to_string_lossy().into_owned()),
                    SqlValue::Text(sha256),
                    SqlValue::Integer(metadata.len() as i64),
                    SqlValue::Text("2026-10-30T12:00:00.000Z".to_owned()),
                ]),
            )
            .expect("local file should save");
    }

    fn populate_source(connection: &mut Connection) {
        let topic = topic_id(connection, "MMP-C01");
        syllabus::update_progress(
            connection,
            topic,
            syllabus::TopicProgressInput {
                status: "Learning".to_owned(),
                priority: "High".to_owned(),
                confidence: 4,
                notes: "Portable progress".to_owned(),
            },
        )
        .expect("progress should save");
        sessions::create_manual(
            connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-10-30".to_owned(),
                duration_seconds: 3_600,
                notes: "Comma, quote \" and newline\nsession".to_owned(),
            },
        )
        .expect("session should save");
        planner::create_task(
            connection,
            planner::TaskInput {
                title: "Portable task".to_owned(),
                description: "description".to_owned(),
                scheduled_date: "2026-10-30".to_owned(),
                deadline_date: Some("2026-11-02".to_owned()),
                topic_id: Some(topic),
                priority: "High".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: None,
            },
            date(2026, 10, 30),
        )
        .expect("task should save");
        planner::create_block(
            connection,
            planner::PlannedBlockInput {
                title: "Portable block".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-10-30".to_owned(),
                start_time: "10:00".to_owned(),
                end_time: "11:00".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("block should save");
        add_pyq(connection, topic, "SHARED", Some("shared-pyq"));
        add_pyq(connection, topic, "LOCAL", None);
        add_mock(connection, "Portable mock", Some("shared-mock"));
        revision::schedule_topic(connection, topic, "2026-10-28", date(2026, 10, 30))
            .expect("schedule should save");
        revision::complete(
            connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: "Portable revision".to_owned(),
            },
            date(2026, 10, 30),
        )
        .expect("revision should complete");
        connection
            .execute(
                "UPDATE dashboard_sections SET visible = 0 WHERE section_key = 'latest-mock'",
                [],
            )
            .expect("layout should update");
        add_resource(connection, None);
    }

    #[test]
    fn json_export_is_complete_and_csv_export_is_escaped_with_bom() {
        let source = tempfile::tempdir().expect("source temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut connection = database(&source, "source.sqlite3");
        populate_source(&mut connection);

        let exported =
            export_json(&connection, export_dir.path()).expect("JSON export should succeed");
        let path = export_dir.path().join(&exported.file_name);
        let bytes = fs::read(&path).expect("JSON export should be readable");
        let envelope: JsonExportEnvelope =
            serde_json::from_slice(&bytes).expect("JSON should parse");
        assert_eq!(envelope.format_version, 4);
        assert_eq!(envelope.schema_version, 12);
        assert_eq!(envelope.application_version, "2.1.0");
        assert_eq!(envelope.import_batch_id.as_deref().map(str::len), Some(32));
        assert_eq!(envelope.tables.keys().count(), 27);
        assert_eq!(envelope.record_counts["study_sessions"], 1);
        assert_eq!(envelope.record_counts["revision_history"], 1);
        assert_eq!(envelope.record_counts["study_resources"], 1);
        assert_eq!(envelope.record_counts["resource_progress"], 1);
        assert_eq!(envelope.tables["study_resources"].len(), 1);
        assert_eq!(envelope.tables["resource_progress"].len(), 1);
        assert_eq!(envelope.tables["settings"].len(), 1);
        assert_eq!(envelope.tables["dashboard_sections"].len(), 9);
        assert!(!envelope.backup_preferences.automatic_enabled);

        let csv =
            export_csv_datasets(&connection, export_dir.path()).expect("CSV export should succeed");
        assert_eq!(csv.files.len(), 17);
        let csv_dir = export_dir.path().join(&csv.folder_name);
        for file in [
            "exams.csv",
            "exam_profiles.csv",
            "revision_plan_items.csv",
            "revision_daily_plans.csv",
            "revision_daily_plan_items.csv",
            "revision_deferrals.csv",
            "revision_session_links.csv",
        ] {
            assert!(
                csv_dir.join(file).is_file(),
                "the new lifecycle dataset {file} should be exported"
            );
        }
        let session_csv = csv_dir.join("study_sessions.csv");
        let content = fs::read_to_string(&session_csv).expect("CSV should be readable");
        assert!(content.starts_with(CSV_BOM));
        assert!(content.contains("Comma, quote"));
        assert!(content.contains("\"Comma, quote \"\" and newline\nsession\""));
        assert!(content.contains("duration_seconds"));
        assert!(content.contains("topic_code"));
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
        let source_sessions: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(source_sessions, 1);
    }

    #[test]
    fn merge_preview_and_apply_preserve_existing_data_map_topics_and_skip_conflicts() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        populate_source(&mut source_db);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut target_db = database(&target, "target.sqlite3");
        let target_topic = topic_id(&target_db, "MMP-C01");
        sessions::create_manual(
            &mut target_db,
            sessions::StudySessionInput {
                topic_id: Some(target_topic),
                session_date: "2026-10-29".to_owned(),
                duration_seconds: 600,
                notes: "existing target session".to_owned(),
            },
        )
        .expect("target session should save");
        add_pyq(
            &mut target_db,
            target_topic,
            "TARGET-SHARED",
            Some("shared-pyq"),
        );
        add_mock(&mut target_db, "Target shared mock", Some("shared-mock"));
        revision::schedule_topic(
            &mut target_db,
            target_topic,
            "2026-11-01",
            date(2026, 10, 30),
        )
        .expect("target schedule should save");
        let settings_before = settings::load(&target_db).expect("settings should load");

        let preview = preview_json_import(&target_db, &export_path).expect("preview should load");
        assert!(preview.merge_supported && preview.replacement_supported);
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "pyq_attempts")
        );
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "mock_tests")
        );
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "revision_schedules")
        );
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "settings")
        );
        assert_eq!(preview.record_counts["study_sessions"], 1);

        let result = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Merge,
            &preview.file_sha256,
            target.path(),
        )
        .expect("merge should succeed");
        assert_eq!(result.strategy, "merge");
        assert!(result.imported > 0);
        assert!(result.conflicting > 0);
        assert!(
            target
                .path()
                .join("backups")
                .join("safety")
                .join(&result.safety_backup_file)
                .is_file()
        );
        let sessions_count: i64 = target_db
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sessions_count, 2);
        let imported_session_topic: i64 = target_db
            .query_row(
                "SELECT topic_id FROM study_sessions WHERE duration_seconds = 3600",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(imported_session_topic, target_topic);
        let pyq_keys: i64 = target_db
            .query_row(
                "SELECT count(*) FROM pyq_attempts WHERE import_key = 'shared-pyq'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pyq_keys, 1);
        let schedules: i64 = target_db
            .query_row(
                "SELECT count(*) FROM revision_schedules WHERE topic_id = ?1",
                [target_topic],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(schedules, 1);
        let history_count: i64 = target_db
            .query_row("SELECT count(*) FROM revision_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(history_count, 1);
        assert_eq!(
            settings::load(&target_db).expect("settings should load"),
            settings_before
        );
        let conflict_error = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Merge,
            "bad-hash",
            target.path(),
        )
        .expect_err("changed hash should fail");
        assert_eq!(conflict_error.code, "import_failed");

        let repeated =
            preview_json_import(&target_db, &export_path).expect("repeated preview should load");
        assert!(repeated.already_imported.is_some());
        assert!(!repeated.merge_supported);
        let counts_before_repeat = backup::table_counts(&target_db).expect("counts should load");
        let repeated_error = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Merge,
            &repeated.file_sha256,
            target.path(),
        )
        .expect_err("repeated merge should be blocked");
        assert!(repeated_error.message.contains("already imported"));
        assert_eq!(
            backup::table_counts(&target_db).expect("counts should load"),
            counts_before_repeat
        );
        let batch_rows: i64 = target_db
            .query_row("SELECT count(*) FROM json_import_batches", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(batch_rows, 1);
    }

    #[test]
    fn replacement_restores_all_data_and_invalid_files_do_not_write() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        populate_source(&mut source_db);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);
        let envelope: JsonExportEnvelope =
            serde_json::from_slice(&fs::read(&export_path).unwrap()).expect("export should parse");

        let mut target_db = database(&target, "target.sqlite3");
        let target_topic = topic_id(&target_db, "MMP-C01");
        sessions::create_manual(
            &mut target_db,
            sessions::StudySessionInput {
                topic_id: Some(target_topic),
                session_date: "2026-10-29".to_owned(),
                duration_seconds: 600,
                notes: "target only".to_owned(),
            },
        )
        .expect("target session should save");
        let preview = preview_json_import(&target_db, &export_path).expect("preview should load");
        let before_counts = backup::table_counts(&target_db).expect("counts should load");

        let mut invalid_json: Value =
            serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        invalid_json["tables"]["study_sessions"][0]["duration_seconds"] = Value::from(0);
        let invalid_path = export_dir.path().join("invalid.json");
        fs::write(
            &invalid_path,
            serde_json::to_vec_pretty(&invalid_json).unwrap(),
        )
        .unwrap();
        assert_eq!(
            preview_json_import(&target_db, &invalid_path)
                .expect_err("invalid constraint should fail")
                .code,
            "import_failed"
        );
        assert_eq!(
            backup::table_counts(&target_db).expect("counts should load"),
            before_counts
        );

        let result = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("replace should succeed");
        assert_eq!(result.strategy, "replace");
        assert_eq!(result.rejected, 0);
        assert_eq!(
            result.imported,
            envelope.record_counts.values().sum::<i64>()
        );
        assert_eq!(
            existing_table_counts(&target_db).expect("counts should load"),
            envelope.record_counts
        );
        let target_only: i64 = target_db
            .query_row(
                "SELECT count(*) FROM study_sessions WHERE notes = 'target only'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(target_only, 0);
        let history_trigger: i64 = target_db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name = 'revision_history_no_delete'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(history_trigger, 1);
    }

    #[test]
    fn v2_resource_round_trip_preserves_metadata_progress_and_discovery_history() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let pdf = source.path().join("portable.pdf");
        fs::write(&pdf, b"%PDF-1.4\nportable\n").expect("pdf should write");
        let source_db = database(&source, "source.sqlite3");
        let resource = add_resource(&source_db, Some(&pdf));
        source_db
            .execute(
                "INSERT INTO resource_discovery_runs (
                    started_at, finished_at, status, pages_ok, pages_failed,
                    resources_seen, resources_added, resources_updated,
                    resources_removed, message
                 ) VALUES (
                    '2026-10-30T11:00:00.000Z', '2026-10-30T11:00:02.000Z',
                    'success', 6, 0, 1, 1, 0, 0, 'fixture discovery'
                 )",
                [],
            )
            .expect("discovery run should save");
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);
        let envelope: JsonExportEnvelope =
            serde_json::from_slice(&fs::read(&export_path).unwrap()).expect("export should parse");
        assert_eq!(envelope.format_version, 4);
        assert_eq!(envelope.record_counts["resource_discovery_runs"], 1);
        assert_eq!(
            envelope.tables["study_resources"][0]["original_url"],
            Value::String(
                "https://drive.google.com/file/d/sourceResource/view?usp=sharing".to_owned()
            )
        );
        assert_eq!(
            envelope.tables["study_resources"][0]["source_page_url"],
            Value::String("https://www.csirnetphysics.in/p/hand-written-notes_28.html".to_owned())
        );

        let mut target_db = database(&target, "target.sqlite3");
        let preview = preview_json_import(&target_db, &export_path).expect("preview should load");
        let result = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("replace should succeed");
        assert_eq!(result.per_table["study_resources"].imported, 1);
        assert_eq!(result.per_table["resource_progress"].imported, 1);
        assert_eq!(result.per_table["resource_local_files"].imported, 1);
        assert_eq!(result.per_table["resource_discovery_runs"].imported, 1);

        let imported = resources::list_resources(&target_db)
            .expect("resources should list")
            .into_iter()
            .find(|item| item.resource_key == resource.resource_key)
            .expect("resource should import");
        assert_eq!(imported.study_status, "studying");
        assert_eq!(imported.personal_notes, "Portable resource notes");
        assert_eq!(imported.source_page_url, resource.source_page_url);
        assert_eq!(imported.original_url, resource.original_url);
        assert_eq!(imported.external_id.as_deref(), Some("sourceResource"));
        assert!(imported.local_file.is_some());
        assert_eq!(
            imported.local_file.as_ref().unwrap().file_path,
            pdf.to_string_lossy()
        );
        assert_eq!(
            target_db
                .query_row("SELECT count(*) FROM resource_discovery_runs", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
    }

    #[test]
    fn v2_merge_preserves_existing_resource_progress_notes_and_local_files() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let source_db = {
            let mut connection = database(&source, "source.sqlite3");
            populate_source(&mut connection);
            connection
        };
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let target_pdf = target.path().join("target.pdf");
        fs::write(&target_pdf, b"%PDF-1.4\ntarget copy\n").expect("pdf should write");
        let mut target_db = database(&target, "target.sqlite3");
        let target_resource = add_resource(&target_db, Some(&target_pdf));
        resources::update_progress(
            &target_db,
            target_resource.id,
            resources::ResourceProgressInput {
                status: "completed".to_owned(),
                notes: "Keep target notes".to_owned(),
            },
            "2026-10-30T12:00:00.000Z",
        )
        .expect("target progress should save");

        let preview = preview_json_import(&target_db, &export_path).expect("preview should load");
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "study_resources")
        );
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "resource_progress")
        );
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| { warning.contains("Complete replacement, if selected") })
        );

        let result = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Merge,
            &preview.file_sha256,
            target.path(),
        )
        .expect("merge should succeed");
        assert_eq!(result.per_table["study_resources"].skipped, 1);
        assert_eq!(result.per_table["resource_progress"].skipped, 1);
        let kept = resources::list_resources(&target_db)
            .expect("resources should list")
            .into_iter()
            .find(|item| item.id == target_resource.id)
            .expect("target resource should remain");
        assert_eq!(kept.study_status, "completed");
        assert_eq!(kept.personal_notes, "Keep target notes");
        assert_eq!(
            kept.local_file.as_ref().map(|file| file.file_path.as_str()),
            Some(target_pdf.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn v2_missing_local_pdf_is_skipped_without_losing_resource_progress() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let pdf = source.path().join("portable.pdf");
        fs::write(&pdf, b"%PDF-1.4\nportable\n").expect("pdf should write");
        let source_db = database(&source, "source.sqlite3");
        let resource = add_resource(&source_db, Some(&pdf));
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut missing: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        missing["tables"]["resource_local_files"][0]["file_path"] = Value::String(
            source
                .path()
                .join("missing.pdf")
                .to_string_lossy()
                .into_owned(),
        );
        let missing_path = export_dir.path().join("missing-local.json");
        fs::write(&missing_path, serde_json::to_vec_pretty(&missing).unwrap()).unwrap();

        let mut target_db = database(&target, "target.sqlite3");
        let preview = preview_json_import(&target_db, &missing_path).expect("preview should load");
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("local PDF reference"))
        );
        let result = apply_json_import(
            &mut target_db,
            &missing_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("replace should succeed");
        assert_eq!(result.per_table["resource_local_files"].imported, 0);
        assert_eq!(result.per_table["resource_local_files"].skipped, 1);
        let imported = resources::list_resources(&target_db)
            .expect("resources should list")
            .into_iter()
            .find(|item| item.resource_key == resource.resource_key)
            .expect("resource should remain");
        assert!(imported.local_file.is_none());
        assert_eq!(imported.study_status, "studying");
        assert_eq!(imported.personal_notes, "Portable resource notes");
    }

    #[test]
    fn v2_checksum_mismatch_local_pdf_is_skipped() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let pdf = source.path().join("portable.pdf");
        fs::write(&pdf, b"%PDF-1.4\nportable checksum\n").expect("pdf should write");
        let source_db = database(&source, "source.sqlite3");
        add_resource(&source_db, Some(&pdf));
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut changed: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        changed["tables"]["resource_local_files"][0]["sha256"] = Value::String("a".repeat(64));
        let changed_path = export_dir.path().join("checksum-mismatch.json");
        fs::write(&changed_path, serde_json::to_vec_pretty(&changed).unwrap()).unwrap();

        let mut target_db = database(&target, "target.sqlite3");
        let preview = preview_json_import(&target_db, &changed_path).expect("preview should load");
        assert!(
            preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "resource_local_files")
        );
        let result = apply_json_import(
            &mut target_db,
            &changed_path,
            JsonImportStrategy::Merge,
            &preview.file_sha256,
            target.path(),
        )
        .expect("merge should succeed");
        assert_eq!(result.per_table["resource_local_files"].imported, 0);
        assert_eq!(result.per_table["resource_local_files"].skipped, 1);
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("local PDF reference"))
        );
        let imported = resources::list_resources(&target_db)
            .expect("resources should list")
            .into_iter()
            .next()
            .expect("resource should import");
        assert!(imported.local_file.is_none());
    }

    #[test]
    fn v2_duplicate_resource_keys_are_rejected() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let source_db = database(&source, "source.sqlite3");
        add_resource(&source_db, None);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut duplicate: Value =
            serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        let mut duplicated = duplicate["tables"]["study_resources"][0].clone();
        duplicated["id"] = Value::from(999);
        duplicate["tables"]["study_resources"]
            .as_array_mut()
            .unwrap()
            .push(duplicated);
        duplicate["recordCounts"]["study_resources"] = Value::from(2);
        let duplicate_path = export_dir.path().join("duplicate-resources.json");
        fs::write(
            &duplicate_path,
            serde_json::to_vec_pretty(&duplicate).unwrap(),
        )
        .unwrap();

        let target_db = database(&target, "target.sqlite3");
        let error =
            preview_json_import(&target_db, &duplicate_path).expect_err("duplicate should fail");
        assert!(error.message.contains("duplicate resource keys"));
    }

    #[test]
    fn v1_import_does_not_replace_resource_tables() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        populate_source(&mut source_db);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut legacy: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        legacy["formatVersion"] = Value::from(1);
        trim_legacy_tables(&mut legacy, 1);
        let legacy_path = export_dir.path().join("legacy-v1.json");
        fs::write(&legacy_path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let target_pdf = target.path().join("target.pdf");
        fs::write(&target_pdf, b"%PDF-1.4\ntarget\n").expect("pdf should write");
        let mut target_db = database(&target, "target.sqlite3");
        let target_resource = add_resource(&target_db, Some(&target_pdf));
        let preview =
            preview_json_import(&target_db, &legacy_path).expect("v1 preview should load");
        assert_eq!(preview.format_version, 1);
        assert!(!preview.record_counts.contains_key("study_resources"));

        apply_json_import(
            &mut target_db,
            &legacy_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("v1 replace should succeed");
        let kept = resources::list_resources(&target_db)
            .expect("resources should list")
            .into_iter()
            .find(|item| item.id == target_resource.id)
            .expect("v1 import must leave resources untouched");
        assert_eq!(
            kept.local_file.as_ref().map(|file| file.file_path.clone()),
            Some(target_pdf.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn latest_format_replacement_restores_versioned_revision_lifecycle_data() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let day = date(2026, 10, 30);
        let mut source_db = database(&source, "source.sqlite3");
        let first = topic_id(&source_db, "MMP-C01");
        let second = topic_id(&source_db, "MMP-C02");
        revision::schedule_topic(&mut source_db, first, "2026-10-30", day)
            .expect("first topic should schedule");
        let generated =
            revision_plan::generate_daily_plan(&mut source_db, day).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        let start = Utc
            .with_ymd_and_hms(2026, 10, 30, 9, 0, 0)
            .single()
            .expect("start should be valid");
        timer::start_with_plan(&mut source_db, start, day, "focus", Some(first), Some(item))
            .expect("timer should start");
        for second in (10..300).step_by(10) {
            timer::heartbeat(&mut source_db, start + Duration::seconds(second))
                .expect("heartbeat should succeed");
        }
        timer::finish(&mut source_db, start + Duration::minutes(5)).expect("timer should finish");
        revision::complete(
            &mut source_db,
            first,
            revision::CompleteRevisionInput {
                plan_item_id: Some(item),
                outcome: "good".to_owned(),
                notes: "Linked completion".to_owned(),
            },
            day,
        )
        .expect("revision should complete");
        revision::schedule_topic(&mut source_db, second, "2026-10-30", day)
            .expect("second topic should schedule");
        let extended = revision_plan::add_revision_plan_item(&mut source_db, day, second)
            .expect("second topic should join the plan");
        let second_item = extended
            .scheduled
            .iter()
            .find(|item| item.topic.id == second)
            .expect("second plan item should exist")
            .plan_item_id;
        let second_start = Utc
            .with_ymd_and_hms(2026, 10, 30, 10, 0, 0)
            .single()
            .expect("second start should be valid");
        timer::start_with_plan(
            &mut source_db,
            second_start,
            day,
            "focus",
            Some(second),
            Some(second_item),
        )
        .expect("second timer should start");
        revision_plan::defer_revision(
            &mut source_db,
            day,
            second,
            "2026-11-10",
            "time",
            "deferred for the test",
        )
        .expect("deferral should save");

        let counts = |connection: &Connection| -> (i64, i64, i64) {
            (
                connection
                    .query_row("SELECT count(*) FROM revision_daily_plans", [], |row| {
                        row.get(0)
                    })
                    .unwrap(),
                connection
                    .query_row("SELECT count(*) FROM revision_deferrals", [], |row| {
                        row.get(0)
                    })
                    .unwrap(),
                connection
                    .query_row("SELECT count(*) FROM revision_session_links", [], |row| {
                        row.get(0)
                    })
                    .unwrap(),
            )
        };
        let expected = counts(&source_db);
        assert_eq!(expected.0, 3);
        assert_eq!(expected.1, 1);
        assert_eq!(expected.2, 1);

        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut target_db = database(&target, "target.sqlite3");
        let target_topic = topic_id(&target_db, "MMP-C03");
        revision::schedule_topic(&mut target_db, target_topic, "2026-10-30", day)
            .expect("target topic should schedule");
        revision_plan::generate_daily_plan(&mut target_db, day)
            .expect("target plan should generate");
        revision_plan::defer_revision(
            &mut target_db,
            day,
            target_topic,
            "2026-11-05",
            "life",
            "existing target deferral",
        )
        .expect("target deferral should save");
        let preview = preview_json_import(&target_db, &export_path).expect("preview should load");
        assert_eq!(preview.format_version, 4);

        apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("latest format replacement should restore lifecycle data");

        assert_eq!(counts(&target_db), expected);
        let timer_plan_item: Option<i64> = target_db
            .query_row(
                "SELECT revision_plan_item_id FROM timer_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("imported timer state should load");
        assert!(
            timer_plan_item.is_some(),
            "the running timer should keep its imported plan-item link"
        );
        let deferral_id: i64 = target_db
            .query_row("SELECT id FROM revision_deferrals LIMIT 1", [], |row| {
                row.get(0)
            })
            .expect("deferral should import");
        assert!(
            target_db
                .execute(
                    "UPDATE revision_deferrals SET notes = 'changed' WHERE id = ?1",
                    [deferral_id],
                )
                .is_err(),
            "deferral triggers should be restored"
        );
        let trigger_count: i64 = target_db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'trigger'
                 AND name IN (
                    'revision_daily_plan_items_insert_active',
                    'revision_daily_plan_items_no_delete',
                    'revision_daily_plans_no_delete',
                    'revision_deferrals_no_update',
                    'revision_deferrals_no_delete',
                    'revision_plan_items_completed_no_delete'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("triggers should be queryable");
        assert_eq!(trigger_count, 6);
    }

    #[test]
    fn format_four_round_trip_preserves_explicit_dates_and_merge_keeps_local_profiles() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let merge_target = tempfile::tempdir().expect("merge target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        let source_csir = exams::list(&source_db)
            .expect("source profiles")
            .into_iter()
            .find(|profile| profile.code == exams::CSIR_EXAM_CODE)
            .expect("CSIR profile");
        exams::update_date(&mut source_db, source_csir.exam_id, Some("2027-06-15"))
            .expect("source date should save");

        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);
        let envelope: JsonExportEnvelope =
            serde_json::from_slice(&fs::read(&export_path).expect("export should read"))
                .expect("export should parse");
        assert_eq!(envelope.format_version, 4);
        assert_eq!(envelope.tables["exams"].len(), 4);
        assert_eq!(envelope.tables["exam_profiles"].len(), 4);
        assert!(envelope.tables["exam_profiles"].iter().any(|profile| {
            profile["exam_id"].as_i64() == Some(source_csir.exam_id)
                && profile["exam_date"] == Value::String("2027-06-15".to_owned())
        }));

        let mut target_db = database(&target, "target.sqlite3");
        let preview =
            preview_json_import(&target_db, &export_path).expect("format 4 should preview");
        apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("format 4 replace should succeed");
        let restored = exams::list(&target_db)
            .expect("restored profiles")
            .into_iter()
            .find(|profile| profile.code == exams::CSIR_EXAM_CODE)
            .expect("restored CSIR profile");
        assert_eq!(restored.exam_date.as_deref(), Some("2027-06-15"));

        let mut merge_db = database(&merge_target, "merge.sqlite3");
        let local = exams::list(&merge_db)
            .expect("merge profiles")
            .into_iter()
            .find(|profile| profile.code == exams::CSIR_EXAM_CODE)
            .expect("merge CSIR profile");
        exams::update_date(&mut merge_db, local.exam_id, Some("2028-01-01"))
            .expect("local date should save");
        let merge_preview =
            preview_json_import(&merge_db, &export_path).expect("merge should preview");
        assert!(
            merge_preview
                .conflicts
                .iter()
                .any(|conflict| conflict.table == "exam_profiles" && conflict.rows == 4)
        );
        let result = apply_json_import(
            &mut merge_db,
            &export_path,
            JsonImportStrategy::Merge,
            &merge_preview.file_sha256,
            merge_target.path(),
        )
        .expect("format 4 merge should succeed");
        assert_eq!(result.per_table["exam_profiles"].imported, 0);
        assert_eq!(result.per_table["exam_profiles"].skipped, 4);
        let kept = exams::list(&merge_db)
            .expect("merge profiles")
            .into_iter()
            .find(|profile| profile.code == exams::CSIR_EXAM_CODE)
            .expect("merge CSIR profile");
        assert_eq!(kept.exam_date.as_deref(), Some("2028-01-01"));
    }

    #[test]
    fn format_three_replace_preserves_local_profiles_and_marks_legacy_date_unconfirmed() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let source_db = database(&source, "source.sqlite3");
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut legacy: Value =
            serde_json::from_slice(&fs::read(&export_path).expect("export should read"))
                .expect("export should parse");
        legacy["formatVersion"] = Value::from(3);
        legacy["schemaVersion"] = Value::from(11);
        trim_legacy_tables(&mut legacy, 3);
        let legacy_path = export_dir.path().join("legacy-v3.json");
        fs::write(
            &legacy_path,
            serde_json::to_vec_pretty(&legacy).expect("legacy should encode"),
        )
        .expect("legacy should write");

        let mut target_db = database(&target, "target.sqlite3");
        let preview =
            preview_json_import(&target_db, &legacy_path).expect("format 3 should preview");
        assert_eq!(preview.format_version, 3);
        assert_eq!(preview.schema_version, 11);
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("legacy format does not contain exam profiles"))
        );

        apply_json_import(
            &mut target_db,
            &legacy_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("format 3 replace should succeed");

        let profiles: Vec<(String, i64, Option<String>, String)> = target_db
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
        assert_eq!(profiles[0].0, exams::CSIR_EXAM_CODE);
        assert_eq!(profiles[0].2, None);
        assert_eq!(profiles[0].3, "pending");
    }

    #[test]
    fn latest_format_import_rejects_malformed_deferral_dates() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        let topic = topic_id(&source_db, "MMP-C01");
        revision::schedule_topic(&mut source_db, topic, "2026-10-30", date(2026, 10, 30))
            .expect("topic should schedule");
        revision_plan::generate_daily_plan(&mut source_db, date(2026, 10, 30))
            .expect("plan should generate");
        revision_plan::defer_revision(
            &mut source_db,
            date(2026, 10, 30),
            topic,
            "2026-11-10",
            "time",
            "fixture",
        )
        .expect("deferral should save");
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);
        let original: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        let target_db = database(&target, "target.sqlite3");

        for (index, (field, invalid)) in [
            ("previous_due_date", "not-a-date"),
            ("previous_due_date", "2026-02-30"),
            ("new_due_date", "not-a-date"),
            ("new_due_date", "2026-02-30"),
            ("occurred_on", "not-a-date"),
            ("occurred_on", "2026-02-30"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut malformed = original.clone();
            malformed["tables"]["revision_deferrals"][0][field] = Value::String(invalid.to_owned());
            let path = export_dir.path().join(format!("malformed-{index}.json"));
            fs::write(&path, serde_json::to_vec_pretty(&malformed).unwrap()).unwrap();
            let error = preview_json_import(&target_db, &path)
                .expect_err("malformed deferral dates should be rejected");
            assert_eq!(
                error.code, "import_failed",
                "field {field}, value {invalid}"
            );
        }
    }

    #[test]
    fn v2_schema_ten_import_uses_defaults_for_new_lifecycle_columns() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let mut source_db = database(&source, "source.sqlite3");
        populate_source(&mut source_db);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);

        let mut legacy: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        legacy["formatVersion"] = Value::from(2);
        legacy["schemaVersion"] = Value::from(10);
        trim_legacy_tables(&mut legacy, 2);
        for (table, column) in [
            ("revision_settings", "daily_revision_minutes"),
            ("revision_schedules", "deferred_until"),
            ("timer_state", "revision_plan_item_id"),
        ] {
            for row in legacy["tables"][table]
                .as_array_mut()
                .expect("table rows should be an array")
            {
                row.as_object_mut()
                    .expect("row should be an object")
                    .remove(column);
            }
        }
        let legacy_path = export_dir.path().join("legacy-v2.json");
        fs::write(&legacy_path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

        let mut target_db = database(&target, "target.sqlite3");
        let target_topic = topic_id(&target_db, "MMP-C03");
        revision::schedule_topic(
            &mut target_db,
            target_topic,
            "2026-10-30",
            date(2026, 10, 30),
        )
        .expect("target topic should schedule");
        revision_plan::generate_daily_plan(&mut target_db, date(2026, 10, 30))
            .expect("target plan should generate");
        let preview =
            preview_json_import(&target_db, &legacy_path).expect("v2 preview should load");
        assert_eq!(preview.format_version, 2);
        assert_eq!(preview.schema_version, 10);
        assert!(
            preview
                .warnings
                .iter()
                .any(|warning| warning.contains("legacy format does not describe"))
        );
        apply_json_import(
            &mut target_db,
            &legacy_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect("v2 replace should succeed");

        let daily_minutes: i64 = target_db
            .query_row(
                "SELECT daily_revision_minutes FROM revision_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("legacy revision settings should import");
        assert_eq!(daily_minutes, 90);
        let plan_item: Option<i64> = target_db
            .query_row(
                "SELECT revision_plan_item_id FROM timer_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .expect("legacy timer state should import");
        assert_eq!(plan_item, None);
        let plan_count: i64 = target_db
            .query_row("SELECT count(*) FROM revision_daily_plans", [], |row| {
                row.get(0)
            })
            .expect("plan count should be readable");
        assert_eq!(plan_count, 0);
    }

    #[test]
    fn invalid_json_format_newer_schema_and_duplicate_ids_are_rejected() {
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let target_db = database(&target, "target.sqlite3");

        let bad_path = export_dir.path().join("bad.json");
        fs::write(&bad_path, b"{not-json").unwrap();
        assert_eq!(
            preview_json_import(&target_db, &bad_path)
                .expect_err("bad JSON should fail")
                .code,
            "import_failed"
        );

        let source = tempfile::tempdir().expect("source temp");
        let mut source_db = database(&source, "source.sqlite3");
        populate_source(&mut source_db);
        let exported = export_json(&source_db, export_dir.path()).expect("export should succeed");
        let export_path = export_dir.path().join(&exported.file_name);
        let mut older: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        older["formatVersion"] = Value::from(1);
        older["schemaVersion"] = Value::from(8);
        older.as_object_mut().unwrap().remove("importBatchId");
        trim_legacy_tables(&mut older, 1);
        let older_path = export_dir.path().join("older.json");
        fs::write(&older_path, serde_json::to_vec_pretty(&older).unwrap()).unwrap();
        let older_preview =
            preview_json_import(&target_db, &older_path).expect("schema 8 JSON should be accepted");
        assert_eq!(older_preview.schema_version, 8);
        assert_eq!(older_preview.format_version, 1);
        assert_eq!(older_preview.import_batch_id.len(), 32);

        let mut newer: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        newer["schemaVersion"] = Value::from(99);
        let newer_path = export_dir.path().join("newer.json");
        fs::write(&newer_path, serde_json::to_vec_pretty(&newer).unwrap()).unwrap();
        assert!(
            preview_json_import(&target_db, &newer_path)
                .expect_err("newer schema should fail")
                .message
                .contains("schema 99")
        );

        let mut duplicate: Value =
            serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        let duplicated = duplicate["tables"]["study_sessions"][0].clone();
        duplicate["tables"]["study_sessions"]
            .as_array_mut()
            .unwrap()
            .push(duplicated);
        duplicate["recordCounts"]["study_sessions"] = Value::from(2);
        let duplicate_path = export_dir.path().join("duplicate.json");
        fs::write(
            &duplicate_path,
            serde_json::to_vec_pretty(&duplicate).unwrap(),
        )
        .unwrap();
        assert_eq!(
            preview_json_import(&target_db, &duplicate_path)
                .expect_err("duplicate primary key should fail")
                .code,
            "import_failed"
        );
    }
}

#[cfg(test)]
mod rollback_tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{backup, db, mock_tests, resources, sessions, syllabus};

    #[test]
    fn replacement_failure_rolls_back_data_and_restores_append_only_triggers() {
        let source = tempfile::tempdir().expect("source temp");
        let target = tempfile::tempdir().expect("target temp");
        let export_dir = tempfile::tempdir().expect("export temp");
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();

        let source_path = source.path().join("source.sqlite3");
        let mut source_db = db::open_database(&source_path, today).expect("source opens");
        syllabus::seed(&mut source_db).expect("syllabus seeds");
        mock_tests::create(
            &mut source_db,
            mock_tests::MockTestInput {
                test_name: "Trigger source mock".to_owned(),
                test_date: "2026-10-30".to_owned(),
                duration_minutes: 60,
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
        .expect("mock saves");
        let exported = export_json(&source_db, export_dir.path()).expect("export succeeds");
        let export_path = export_dir.path().join(&exported.file_name);
        let preview = preview_json_import(&source_db, &export_path).expect("preview loads");

        let target_path = target.path().join("target.sqlite3");
        let mut target_db = db::open_database(&target_path, today).expect("target opens");
        syllabus::seed(&mut target_db).expect("syllabus seeds");
        let target_topic: i64 = target_db
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = 'MMP-C01'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        sessions::create_manual(
            &mut target_db,
            sessions::StudySessionInput {
                topic_id: Some(target_topic),
                session_date: "2026-10-30".to_owned(),
                duration_seconds: 600,
                notes: "keep on rollback".to_owned(),
            },
        )
        .expect("target session saves");
        let target_resource = resources::create_manual(
            &target_db,
            resources::ManualResourceInput {
                title: "Rollback resource".to_owned(),
                description: None,
                category: "other".to_owned(),
                unit: "Unmapped".to_owned(),
                topic_code: None,
                original_url: "https://example.com/rollback-resource.pdf".to_owned(),
                source_page_url: None,
            },
        )
        .expect("target resource saves");
        resources::update_progress(
            &target_db,
            target_resource.id,
            resources::ResourceProgressInput {
                status: "studying".to_owned(),
                notes: "Keep rollback resource".to_owned(),
            },
            "2026-10-30T12:00:00.000Z",
        )
        .expect("target resource progress saves");
        let before = backup::table_counts(&target_db).expect("counts load");
        target_db
            .execute_batch(
                "CREATE TRIGGER fail_mock_replace
                 BEFORE INSERT ON mock_tests
                 BEGIN
                     SELECT RAISE(ABORT, 'forced replacement failure');
                 END;",
            )
            .expect("failure trigger installs");

        let error = apply_json_import(
            &mut target_db,
            &export_path,
            JsonImportStrategy::Replace,
            &preview.file_sha256,
            target.path(),
        )
        .expect_err("replacement should fail");
        assert_eq!(error.code, "import_failed");
        assert_eq!(
            backup::table_counts(&target_db).expect("counts load"),
            before
        );
        let notes: String = target_db
            .query_row("SELECT notes FROM study_sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(notes, "keep on rollback");
        let kept_resource = resources::list_resources(&target_db)
            .expect("resources load")
            .into_iter()
            .find(|resource| resource.id == target_resource.id)
            .expect("resource should survive rollback");
        assert_eq!(kept_resource.study_status, "studying");
        assert_eq!(kept_resource.personal_notes, "Keep rollback resource");
        let revision_trigger: i64 = target_db
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'trigger' AND name = 'revision_history_no_delete'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision_trigger, 1);
        assert!(target.path().join("backups").join("safety").is_dir());
    }
}
