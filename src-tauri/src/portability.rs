use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Write,
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

use crate::{backup, db, error::AppError};

pub const JSON_EXPORT_FORMAT_VERSION: u32 = 1;
pub const MIN_JSON_IMPORT_SCHEMA_VERSION: i64 = 8;
pub const MAX_IMPORT_BYTES: u64 = 100 * 1024 * 1024;
pub const CSV_BOM: &str = "\u{feff}";

const USER_TABLES: [&str; 16] = backup::BACKUP_TABLES;

const REPLACE_DELETE_ORDER: [&str; 16] = [
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

fn csv_datasets() -> [(&'static str, &'static str); 10] {
    [
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
    for table in USER_TABLES {
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
    for table in USER_TABLES {
        let rows = tables
            .get(table)
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::export_failed("A JSON export table is missing."))?;
        if rows.len() as i64 != envelope.record_counts.get(table).copied().unwrap_or(-1) {
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
    let existing_counts = backup::table_counts(connection)?;
    let conflicts = detect_merge_conflicts(connection, &envelope)?;
    let mut warnings = vec![
        "Every record is validated in a temporary database before any application data is changed."
            .to_owned(),
        "A verified safety backup of the current database is created before import.".to_owned(),
    ];
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
        merge_description: "Merge appends study history under new local IDs, matches syllabus topics by stable code, skips conflicting progress/schedules/import-key rows, and keeps the current settings, timer state, revision rules, and dashboard layout.".to_owned(),
        replacement_description: "Replace removes all current user records and inserts the complete validated export, preserving the exported IDs and relationships. Current backup settings and the local database identifier are kept.".to_owned(),
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
    if envelope.format_version != JSON_EXPORT_FORMAT_VERSION {
        return Err(AppError::import_failed(
            "This JSON export format version is not supported.",
        ));
    }
    if !(MIN_JSON_IMPORT_SCHEMA_VERSION..=db::LATEST_SCHEMA_VERSION)
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
    for table in USER_TABLES {
        let rows = envelope
            .tables
            .get(table)
            .ok_or_else(|| AppError::import_failed("A required JSON export table is missing."))?;
        if rows.len() as i64 != envelope.record_counts.get(table).copied().unwrap_or(-1) {
            return Err(AppError::import_failed(
                "The JSON export record counts do not match its tables.",
            ));
        }
    }
    if envelope
        .tables
        .keys()
        .any(|table| !USER_TABLES.contains(&table.as_str()))
    {
        return Err(AppError::import_failed(
            "The JSON export contains an unknown table.",
        ));
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
    for table in USER_TABLES {
        validate_table_rows(&transaction, table, &envelope.tables[table])?;
    }
    for table in REPLACE_DELETE_ORDER {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| {
                AppError::database("Validation defaults could not be cleared", error)
            })?;
    }
    insert_all_tables(&transaction, &envelope.tables)?;
    verify_foreign_keys(&transaction)?;
    transaction.rollback().map_err(|error| {
        AppError::database("The JSON validation transaction could not close", error)
    })
}

fn validate_table_rows(
    connection: &Connection,
    table: &str,
    rows: &[Value],
) -> Result<(), AppError> {
    let columns = table_columns(connection, table)?;
    let allowed = columns.iter().collect::<HashSet<_>>();
    for row in rows {
        let object = row
            .as_object()
            .ok_or_else(|| AppError::import_failed("A JSON export row is not an object."))?;
        if object.len() != columns.len() {
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
) -> Result<(), AppError> {
    for table in USER_TABLES {
        insert_rows(transaction, table, &tables[table], false)?;
    }
    Ok(())
}

fn insert_rows(
    transaction: &Transaction<'_>,
    table: &str,
    rows: &[Value],
    ignore_conflicts: bool,
) -> Result<usize, AppError> {
    let columns = table_columns(transaction, table)?;
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

    let mut per_table = empty_import_counts();
    let merge_warnings = match strategy {
        JsonImportStrategy::Replace => {
            replace_from_json(connection, &envelope, &sha256, &batch_id, &mut per_table)?;
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

fn empty_import_counts() -> BTreeMap<String, ImportTableResult> {
    USER_TABLES
        .into_iter()
        .map(|table| {
            (
                table.to_owned(),
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
             DROP TRIGGER IF EXISTS revision_history_no_delete;",
        )
        .map_err(|error| {
            AppError::database("Revision-history protection could not be prepared", error)
        })?;
    for table in REPLACE_DELETE_ORDER {
        transaction
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(|error| AppError::database("Existing records could not be cleared", error))?;
    }

    for table in USER_TABLES {
        let imported = insert_rows(&transaction, table, &envelope.tables[table], false)? as i64;
        if imported != envelope.tables[table].len() as i64 {
            return Err(AppError::import_failed(
                "The replacement did not insert every expected record.",
            ));
        }
        per_table
            .get_mut(table)
            .expect("table result exists")
            .imported = imported;
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
    verify_imported_counts(&transaction, envelope)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The JSON replacement could not be committed", error))
}

fn verify_imported_counts(
    transaction: &Transaction<'_>,
    envelope: &JsonExportEnvelope,
) -> Result<(), AppError> {
    for table in USER_TABLES {
        let count: i64 = transaction
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| {
                AppError::database("An imported record count could not be verified", error)
            })?;
        if count != envelope.tables[table].len() as i64 {
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
        .filter(|column| !omitted.contains(&column.as_str()))
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
    use chrono::NaiveDate;
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;
    use crate::{mock_tests, planner, pyq, revision, sessions, settings, syllabus};

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
        assert_eq!(envelope.format_version, 1);
        assert_eq!(envelope.schema_version, 10);
        assert_eq!(envelope.application_version, "1.1.0");
        assert_eq!(envelope.import_batch_id.as_deref().map(str::len), Some(32));
        assert_eq!(envelope.tables.keys().count(), 16);
        assert_eq!(envelope.record_counts["study_sessions"], 1);
        assert_eq!(envelope.record_counts["revision_history"], 1);
        assert_eq!(envelope.tables["settings"].len(), 1);
        assert_eq!(envelope.tables["dashboard_sections"].len(), 9);
        assert!(!envelope.backup_preferences.automatic_enabled);

        let csv =
            export_csv_datasets(&connection, export_dir.path()).expect("CSV export should succeed");
        assert_eq!(csv.files.len(), 10);
        let csv_dir = export_dir.path().join(&csv.folder_name);
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
            backup::table_counts(&target_db).expect("counts should load"),
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
        older["schemaVersion"] = Value::from(8);
        older.as_object_mut().unwrap().remove("importBatchId");
        let older_path = export_dir.path().join("older.json");
        fs::write(&older_path, serde_json::to_vec_pretty(&older).unwrap()).unwrap();
        let older_preview =
            preview_json_import(&target_db, &older_path).expect("schema 8 JSON should be accepted");
        assert_eq!(older_preview.schema_version, 8);
        assert_eq!(older_preview.import_batch_id.len(), 32);

        let mut newer: Value = serde_json::from_slice(&fs::read(&export_path).unwrap()).unwrap();
        newer["schemaVersion"] = Value::from(99);
        let newer_path = export_dir.path().join("newer.json");
        fs::write(&newer_path, serde_json::to_vec_pretty(&newer).unwrap()).unwrap();
        assert!(
            preview_json_import(&target_db, &newer_path)
                .expect_err("newer schema should fail")
                .message
                .contains("schema version 99")
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
    use crate::{backup, db, mock_tests, sessions, syllabus};

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
