use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{
    csv_import::{
        CsvTable, ImportFieldError, ImportPreview, ImportResult, ImportRow, ImportRowStatus,
        csv_template, preview_from_rows, read_table, result_from_rows, stable_import_key,
        values_by_header,
    },
    date_only,
    error::AppError,
};

pub const PYQ_STATUSES: [&str; 4] = ["attempted", "correct", "incorrect", "skipped"];
pub const PYQ_PARTS: [&str; 3] = ["A", "B", "C"];
pub const MIN_QUESTION_YEAR: i64 = 1900;
pub const MAX_QUESTION_YEAR: i64 = 2100;
pub const MAX_SESSION_CHARS: usize = 100;
pub const MAX_REFERENCE_CHARS: usize = 200;
pub const MAX_NOTES_CHARS: usize = 2_000;
pub const MAX_SEARCH_CHARS: usize = 100;
pub const MAX_TIME_SPENT_SECONDS: i64 = 86_400;
pub const MAX_MARKS: f64 = 100_000.0;

pub const CSV_HEADERS: &[&str] = &[
    "question_year",
    "exam_session",
    "part",
    "topic_code",
    "question_reference",
    "attempt_date",
    "status",
    "marks_obtained",
    "max_marks",
    "time_spent_seconds",
    "notes",
    "difficulty",
    "external_id",
];

const SELECT_PYQ: &str = "SELECT
        p.id,
        p.question_year,
        p.exam_session,
        p.part,
        t.id,
        t.code,
        t.title,
        t.is_archived,
        s.code,
        s.title,
        p.question_reference,
        p.attempt_date,
        p.status,
        p.marks_obtained,
        p.max_marks,
        p.time_spent_seconds,
        p.notes,
        p.difficulty,
        p.created_at,
        p.updated_at
     FROM pyq_attempts p
     JOIN syllabus_topics t ON t.id = p.topic_id
     JOIN syllabus_sections s ON s.id = t.section_id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqTopic {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
    pub section_code: String,
    pub section_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqSection {
    pub code: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqOptions {
    pub years: Vec<i64>,
    pub sections: Vec<PyqSection>,
    pub topics: Vec<PyqTopic>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqAttempt {
    pub id: i64,
    pub question_year: i64,
    pub exam_session: String,
    pub part: String,
    pub topic: PyqTopic,
    pub question_reference: String,
    pub attempt_date: String,
    pub status: String,
    pub marks_obtained: Option<f64>,
    pub max_marks: Option<f64>,
    pub time_spent_seconds: i64,
    pub notes: String,
    pub difficulty: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PyqFilter {
    pub year: Option<i64>,
    pub part: Option<String>,
    pub section_code: Option<String>,
    pub topic_id: Option<i64>,
    pub status: Option<String>,
    pub query: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqInput {
    pub question_year: i64,
    #[serde(default)]
    pub exam_session: String,
    pub part: String,
    pub topic_id: i64,
    pub question_reference: String,
    pub attempt_date: String,
    pub status: String,
    #[serde(default)]
    pub marks_obtained: Option<f64>,
    #[serde(default)]
    pub max_marks: Option<f64>,
    #[serde(default)]
    pub time_spent_seconds: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub difficulty: Option<i64>,
}

struct ValidatedPyq {
    question_year: i64,
    exam_session: String,
    part: String,
    topic_id: i64,
    question_reference: String,
    attempt_date: String,
    status: String,
    marks_obtained: Option<f64>,
    max_marks: Option<f64>,
    time_spent_seconds: i64,
    notes: String,
    difficulty: Option<i64>,
}

pub fn template() -> String {
    csv_template(CSV_HEADERS)
}

pub fn options(connection: &Connection) -> Result<PyqOptions, AppError> {
    let years = connection
        .prepare(
            "SELECT DISTINCT question_year
             FROM pyq_attempts
             ORDER BY question_year DESC",
        )
        .map_err(|error| AppError::database("PYQ years could not be prepared", error))?
        .query_map([], |row| row.get(0))
        .map_err(|error| AppError::database("PYQ years could not be queried", error))?
        .collect::<rusqlite::Result<Vec<i64>>>()
        .map_err(|error| AppError::database("PYQ years could not be read", error))?;

    let sections = connection
        .prepare(
            "SELECT code, title
             FROM syllabus_sections
             ORDER BY display_order",
        )
        .map_err(|error| AppError::database("PYQ sections could not be prepared", error))?
        .query_map([], |row| {
            Ok(PyqSection {
                code: row.get(0)?,
                title: row.get(1)?,
            })
        })
        .map_err(|error| AppError::database("PYQ sections could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ sections could not be read", error))?;

    let topics = list_topics(connection)?;

    Ok(PyqOptions {
        years,
        sections,
        topics,
    })
}

pub fn list_topics(connection: &Connection) -> Result<Vec<PyqTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT t.id, t.code, t.title, t.is_archived, s.code, s.title
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             ORDER BY s.display_order, t.display_order, t.code",
        )
        .map_err(|error| AppError::database("PYQ topics could not be prepared", error))?;

    statement
        .query_map([], map_topic)
        .map_err(|error| AppError::database("PYQ topics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ topics could not be read", error))
}

pub fn list(connection: &Connection, filter: PyqFilter) -> Result<Vec<PyqAttempt>, AppError> {
    let year = filter.year;
    if let Some(year) = year
        && !(MIN_QUESTION_YEAR..=MAX_QUESTION_YEAR).contains(&year)
    {
        return Err(AppError::validation(
            "year",
            format!("Question year must be between {MIN_QUESTION_YEAR} and {MAX_QUESTION_YEAR}."),
        ));
    }

    let part = normalize_part(filter.part.as_deref().unwrap_or_default(), "part")?;
    let section_code = filter
        .section_code
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(section_code) = &section_code
        && !section_exists(connection, section_code)?
    {
        return Err(AppError::validation(
            "sectionCode",
            "Choose a syllabus unit.",
        ));
    }

    let topic_id = filter.topic_id;
    if let Some(topic_id) = topic_id {
        validate_id(topic_id, "topicId")?;
    }

    let status = normalize_status(filter.status.as_deref().unwrap_or_default(), "status")?;
    let query = filter
        .query
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if let Some(query) = &query
        && query.chars().count() > MAX_SEARCH_CHARS
    {
        return Err(AppError::validation(
            "query",
            format!("Search text must be {MAX_SEARCH_CHARS} characters or fewer."),
        ));
    }

    let from_date = validate_optional_date(filter.from_date.as_deref(), "fromDate")?;
    let to_date = validate_optional_date(filter.to_date.as_deref(), "toDate")?;
    if let (Some(from), Some(to)) = (&from_date, &to_date)
        && from > to
    {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    let pattern = query.map(|value| format!("%{}%", escape_like(&value)));
    let sql = format!(
        "{SELECT_PYQ}
         WHERE (?1 IS NULL OR p.question_year = ?1)
           AND (?2 IS NULL OR p.part = ?2)
           AND (?3 IS NULL OR s.code = ?3)
           AND (?4 IS NULL OR p.topic_id = ?4)
           AND (?5 IS NULL OR p.status = ?5)
           AND (
                ?6 IS NULL
                OR p.question_reference LIKE ?6 ESCAPE '!'
                OR p.exam_session LIKE ?6 ESCAPE '!'
                OR p.notes LIKE ?6 ESCAPE '!'
           )
           AND (?7 IS NULL OR p.attempt_date >= ?7)
           AND (?8 IS NULL OR p.attempt_date <= ?8)
         ORDER BY p.attempt_date DESC, p.id DESC"
    );

    query_attempts(
        connection,
        &sql,
        params![
            year,
            part,
            section_code,
            topic_id,
            status,
            pattern,
            from_date,
            to_date
        ],
    )
}

pub fn recent(connection: &Connection, limit: i64) -> Result<Vec<PyqAttempt>, AppError> {
    if !(1..=50).contains(&limit) {
        return Err(AppError::validation(
            "limit",
            "Recent PYQ limit must be between 1 and 50.",
        ));
    }

    let sql = format!(
        "{SELECT_PYQ}
         ORDER BY p.attempt_date DESC, p.id DESC
         LIMIT ?1"
    );
    query_attempts(connection, &sql, [limit])
}

pub fn create(connection: &mut Connection, input: PyqInput) -> Result<PyqAttempt, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ-attempt transaction could not be started", error)
    })?;
    let validated = validate_input(&transaction, input)?;

    transaction
        .execute(
            "INSERT INTO pyq_attempts (
                question_year, exam_session, part, topic_id, question_reference,
                attempt_date, status, marks_obtained, max_marks, time_spent_seconds,
                notes, difficulty, import_key, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.question_year,
                validated.exam_session,
                validated.part,
                validated.topic_id,
                validated.question_reference,
                validated.attempt_date,
                validated.status,
                validated.marks_obtained,
                validated.max_marks,
                validated.time_spent_seconds,
                validated.notes,
                validated.difficulty,
            ],
        )
        .map_err(|error| AppError::database("The PYQ attempt could not be saved", error))?;

    let id = transaction.last_insert_rowid();
    let attempt = load(&transaction, id)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ attempt could not be committed", error))?;

    Ok(attempt)
}

pub fn update(
    connection: &mut Connection,
    attempt_id: i64,
    input: PyqInput,
) -> Result<PyqAttempt, AppError> {
    validate_id(attempt_id, "id")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ-attempt transaction could not be started", error)
    })?;
    load(&transaction, attempt_id)?;
    let validated = validate_input(&transaction, input)?;

    let changed = transaction
        .execute(
            "UPDATE pyq_attempts
             SET question_year = ?1,
                 exam_session = ?2,
                 part = ?3,
                 topic_id = ?4,
                 question_reference = ?5,
                 attempt_date = ?6,
                 status = ?7,
                 marks_obtained = ?8,
                 max_marks = ?9,
                 time_spent_seconds = ?10,
                 notes = ?11,
                 difficulty = ?12,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?13",
            params![
                validated.question_year,
                validated.exam_session,
                validated.part,
                validated.topic_id,
                validated.question_reference,
                validated.attempt_date,
                validated.status,
                validated.marks_obtained,
                validated.max_marks,
                validated.time_spent_seconds,
                validated.notes,
                validated.difficulty,
                attempt_id,
            ],
        )
        .map_err(|error| AppError::database("The PYQ attempt could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::pyq_not_found());
    }

    let attempt = load(&transaction, attempt_id)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ attempt could not be committed", error))?;

    Ok(attempt)
}

pub fn delete(connection: &mut Connection, attempt_id: i64) -> Result<(), AppError> {
    validate_id(attempt_id, "id")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ-attempt transaction could not be started", error)
    })?;
    let changed = transaction
        .execute("DELETE FROM pyq_attempts WHERE id = ?1", [attempt_id])
        .map_err(|error| AppError::database("The PYQ attempt could not be deleted", error))?;

    if changed != 1 {
        return Err(AppError::pyq_not_found());
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ attempt could not be committed", error))
}

pub fn preview_import(connection: &Connection, csv: &str) -> Result<ImportPreview, AppError> {
    let table = read_table(csv, CSV_HEADERS)?;
    let existing = existing_import_keys(connection)?;
    let topics = load_topic_code_map(connection)?;
    let rows = evaluate_import_rows(&table, &existing, &topics)?;

    Ok(preview_from_rows(rows))
}

pub fn apply_import(connection: &mut Connection, csv: &str) -> Result<ImportResult, AppError> {
    let table = read_table(csv, CSV_HEADERS)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ import transaction could not be started", error)
    })?;
    let existing = existing_import_keys(&transaction)?;
    let topics = load_topic_code_map(&transaction)?;

    let mut seen = HashSet::new();
    let mut result_rows = Vec::new();

    for record in &table.rows {
        let values = match values_by_header(&table, record) {
            Ok(values) => values,
            Err(error) => {
                result_rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Invalid,
                    summary: String::new(),
                    errors: vec![error],
                });
                continue;
            }
        };

        match validate_csv_row(&values, &topics) {
            Ok(valid) => {
                if existing.contains(&valid.import_key) || !seen.insert(valid.import_key.clone()) {
                    result_rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Duplicate,
                        summary: valid.summary(),
                        errors: vec![ImportFieldError::new(
                            "duplicate",
                            "This row was already imported or appears more than once in the file.",
                        )],
                    });
                    continue;
                }

                insert_imported(&transaction, &valid, &valid.import_key)?;
                result_rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Imported,
                    summary: valid.summary(),
                    errors: Vec::new(),
                });
            }
            Err(errors) => {
                result_rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Invalid,
                    summary: summary_from_values(&values),
                    errors,
                });
            }
        }
    }

    let result = result_from_rows(result_rows);
    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ import could not be committed", error))?;

    Ok(result)
}

fn evaluate_import_rows(
    table: &CsvTable,
    existing: &HashSet<String>,
    topics: &HashMap<String, i64>,
) -> Result<Vec<ImportRow>, AppError> {
    let mut seen = HashSet::new();
    let mut rows = Vec::with_capacity(table.rows.len());

    for record in &table.rows {
        let values = match values_by_header(table, record) {
            Ok(values) => values,
            Err(error) => {
                rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Invalid,
                    summary: String::new(),
                    errors: vec![error],
                });
                continue;
            }
        };

        match validate_csv_row(&values, topics) {
            Ok(valid) => {
                if existing.contains(&valid.import_key) || !seen.insert(valid.import_key.clone()) {
                    rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Duplicate,
                        summary: valid.summary(),
                        errors: vec![ImportFieldError::new(
                            "duplicate",
                            "This row was already imported or appears more than once in the file.",
                        )],
                    });
                } else {
                    rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Valid,
                        summary: valid.summary(),
                        errors: Vec::new(),
                    });
                }
            }
            Err(errors) => {
                rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Invalid,
                    summary: summary_from_values(&values),
                    errors,
                });
            }
        }
    }

    Ok(rows)
}

struct ImportedPyq {
    validated: ValidatedPyq,
    topic_code: String,
    import_key: String,
}

impl ImportedPyq {
    fn summary(&self) -> String {
        format!(
            "{} · Part {} · {} · {} · {}",
            self.validated.question_year,
            self.validated.part,
            self.topic_code,
            self.validated.question_reference,
            self.validated.attempt_date
        )
    }
}

fn validate_csv_row(
    values: &HashMap<&str, &str>,
    topics: &HashMap<String, i64>,
) -> Result<ImportedPyq, Vec<ImportFieldError>> {
    let mut errors = Vec::new();
    let value = |field: &str| values.get(field).copied().unwrap_or("").trim();

    let question_year = parse_i64(value("question_year"), "question_year", &mut errors);
    if let Some(year) = question_year
        && !(MIN_QUESTION_YEAR..=MAX_QUESTION_YEAR).contains(&year)
    {
        errors.push(ImportFieldError::new(
            "question_year",
            format!("Question year must be between {MIN_QUESTION_YEAR} and {MAX_QUESTION_YEAR}."),
        ));
    }

    let exam_session = value("exam_session");
    if exam_session.chars().count() > MAX_SESSION_CHARS {
        errors.push(ImportFieldError::new(
            "exam_session",
            format!("Exam session must be {MAX_SESSION_CHARS} characters or fewer."),
        ));
    }

    let part = value("part").to_uppercase();
    if !PYQ_PARTS.contains(&part.as_str()) {
        errors.push(ImportFieldError::new("part", "Part must be A, B or C."));
    }

    let topic_code = value("topic_code");
    let topic_id = if topic_code.is_empty() {
        errors.push(ImportFieldError::new(
            "topic_code",
            "Enter a syllabus topic code.",
        ));
        None
    } else {
        let found = topics.get(&topic_code.to_lowercase()).copied();
        if found.is_none() {
            errors.push(ImportFieldError::new(
                "topic_code",
                format!("No syllabus topic has the code “{topic_code}”."),
            ));
        }
        found
    };

    let question_reference = value("question_reference");
    if question_reference.is_empty() {
        errors.push(ImportFieldError::new(
            "question_reference",
            "Enter a question reference or identifier.",
        ));
    } else if question_reference.chars().count() > MAX_REFERENCE_CHARS {
        errors.push(ImportFieldError::new(
            "question_reference",
            format!("Question reference must be {MAX_REFERENCE_CHARS} characters or fewer."),
        ));
    }

    let attempt_date = value("attempt_date");
    if date_only::parse_date_only(attempt_date).is_none() {
        errors.push(ImportFieldError::new(
            "attempt_date",
            "Attempt date must be a real calendar date in YYYY-MM-DD format.",
        ));
    }

    let status = value("status").to_lowercase();
    if !PYQ_STATUSES.contains(&status.as_str()) {
        errors.push(ImportFieldError::new(
            "status",
            "Status must be attempted, correct, incorrect or skipped.",
        ));
    }

    let marks_obtained = parse_optional_f64(value("marks_obtained"), "marks_obtained", &mut errors);
    if let Some(marks) = marks_obtained
        && !(-MAX_MARKS..=MAX_MARKS).contains(&marks)
    {
        errors.push(ImportFieldError::new(
            "marks_obtained",
            "Marks obtained must be between -100000 and 100000.",
        ));
    }

    let max_marks = parse_optional_f64(value("max_marks"), "max_marks", &mut errors);
    if let Some(marks) = max_marks
        && !(marks > 0.0 && marks <= MAX_MARKS)
    {
        errors.push(ImportFieldError::new(
            "max_marks",
            "Maximum marks must be greater than zero and at most 100000.",
        ));
    }
    if let (Some(obtained), Some(maximum)) = (marks_obtained, max_marks)
        && obtained > maximum
    {
        errors.push(ImportFieldError::new(
            "marks_obtained",
            "Marks obtained cannot exceed maximum marks.",
        ));
    }

    let time_spent_seconds = if value("time_spent_seconds").is_empty() {
        0
    } else {
        parse_i64(
            value("time_spent_seconds"),
            "time_spent_seconds",
            &mut errors,
        )
        .unwrap_or(0)
    };
    if !(0..=MAX_TIME_SPENT_SECONDS).contains(&time_spent_seconds) {
        errors.push(ImportFieldError::new(
            "time_spent_seconds",
            "Time spent must be between 0 and 86400 seconds.",
        ));
    }

    let notes = value("notes");
    if notes.chars().count() > MAX_NOTES_CHARS {
        errors.push(ImportFieldError::new(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    let difficulty = if value("difficulty").is_empty() {
        None
    } else {
        parse_i64(value("difficulty"), "difficulty", &mut errors)
            .map(Some)
            .unwrap_or(None)
    };
    if let Some(difficulty) = difficulty
        && !(1..=5).contains(&difficulty)
    {
        errors.push(ImportFieldError::new(
            "difficulty",
            "Difficulty must be a whole number from 1 to 5.",
        ));
    }

    let external_id = value("external_id");
    if external_id.chars().count() > 200 {
        errors.push(ImportFieldError::new(
            "external_id",
            "External ID must be 200 characters or fewer.",
        ));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let normalized_year = question_year.expect("validated question year").to_string();
    let normalized_topic_code = topic_code.to_lowercase();
    let import_key = if external_id.is_empty() {
        stable_import_key(
            "pyq",
            &[
                &normalized_year,
                exam_session,
                &part,
                &normalized_topic_code,
                question_reference,
                attempt_date,
                &status,
            ],
        )
    } else {
        stable_import_key("pyq-ext", &[external_id])
    };

    Ok(ImportedPyq {
        validated: ValidatedPyq {
            question_year: question_year.expect("validated question year"),
            exam_session: exam_session.to_owned(),
            part,
            topic_id: topic_id.expect("validated topic"),
            question_reference: question_reference.to_owned(),
            attempt_date: attempt_date.to_owned(),
            status,
            marks_obtained,
            max_marks,
            time_spent_seconds,
            notes: notes.to_owned(),
            difficulty,
        },
        topic_code: topic_code.to_owned(),
        import_key,
    })
}

fn summary_from_values(values: &HashMap<&str, &str>) -> String {
    let reference = values
        .get("question_reference")
        .copied()
        .unwrap_or("")
        .trim();
    let year = values.get("question_year").copied().unwrap_or("").trim();
    let part = values.get("part").copied().unwrap_or("").trim();
    let topic = values.get("topic_code").copied().unwrap_or("").trim();

    let mut parts = Vec::new();
    if !year.is_empty() {
        parts.push(year.to_owned());
    }
    if !part.is_empty() {
        parts.push(format!("Part {part}"));
    }
    if !topic.is_empty() {
        parts.push(topic.to_owned());
    }
    if !reference.is_empty() {
        parts.push(reference.to_owned());
    }

    parts.join(" · ")
}

fn insert_imported(
    connection: &Connection,
    imported: &ImportedPyq,
    import_key: &str,
) -> Result<(), AppError> {
    let validated = &imported.validated;
    connection
        .execute(
            "INSERT INTO pyq_attempts (
                question_year, exam_session, part, topic_id, question_reference,
                attempt_date, status, marks_obtained, max_marks, time_spent_seconds,
                notes, difficulty, import_key, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.question_year,
                validated.exam_session,
                validated.part,
                validated.topic_id,
                validated.question_reference,
                validated.attempt_date,
                validated.status,
                validated.marks_obtained,
                validated.max_marks,
                validated.time_spent_seconds,
                validated.notes,
                validated.difficulty,
                import_key,
            ],
        )
        .map_err(|error| AppError::database("An imported PYQ attempt could not be saved", error))?;

    Ok(())
}

fn load_topic_code_map(connection: &Connection) -> Result<HashMap<String, i64>, AppError> {
    connection
        .prepare("SELECT code, id FROM syllabus_topics")
        .map_err(|error| AppError::database("Syllabus topic codes could not be prepared", error))?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| AppError::database("Syllabus topic codes could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Syllabus topic codes could not be read", error))
        .map(|rows| {
            rows.into_iter()
                .map(|(code, id)| (code.to_lowercase(), id))
                .collect()
        })
}

fn existing_import_keys(connection: &Connection) -> Result<HashSet<String>, AppError> {
    connection
        .prepare("SELECT import_key FROM pyq_attempts WHERE import_key IS NOT NULL")
        .map_err(|error| {
            AppError::database("Existing PYQ import keys could not be prepared", error)
        })?
        .query_map([], |row| row.get(0))
        .map_err(|error| {
            AppError::database("Existing PYQ import keys could not be queried", error)
        })?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Existing PYQ import keys could not be read", error))
}

fn validate_input(connection: &Connection, input: PyqInput) -> Result<ValidatedPyq, AppError> {
    if !(MIN_QUESTION_YEAR..=MAX_QUESTION_YEAR).contains(&input.question_year) {
        return Err(AppError::validation(
            "questionYear",
            format!("Question year must be between {MIN_QUESTION_YEAR} and {MAX_QUESTION_YEAR}."),
        ));
    }

    let exam_session = input.exam_session.trim().to_owned();
    if exam_session.chars().count() > MAX_SESSION_CHARS {
        return Err(AppError::validation(
            "examSession",
            format!("Exam session must be {MAX_SESSION_CHARS} characters or fewer."),
        ));
    }

    let part = input.part.trim().to_uppercase();
    if !PYQ_PARTS.contains(&part.as_str()) {
        return Err(AppError::validation("part", "Choose Part A, B or C."));
    }

    validate_id(input.topic_id, "topicId")?;
    validate_topic(connection, input.topic_id)?;

    let question_reference = input.question_reference.trim().to_owned();
    if question_reference.is_empty() {
        return Err(AppError::validation(
            "questionReference",
            "Enter a question reference or identifier.",
        ));
    }
    if question_reference.chars().count() > MAX_REFERENCE_CHARS {
        return Err(AppError::validation(
            "questionReference",
            format!("Question reference must be {MAX_REFERENCE_CHARS} characters or fewer."),
        ));
    }

    if date_only::parse_date_only(&input.attempt_date).is_none() {
        return Err(AppError::validation(
            "attemptDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        ));
    }
    let attempt_date = date_only::format_date_only(
        date_only::parse_date_only(&input.attempt_date).expect("date was validated"),
    );

    let status = input.status.trim().to_lowercase();
    if !PYQ_STATUSES.contains(&status.as_str()) {
        return Err(AppError::validation(
            "status",
            "Choose attempted, correct, incorrect or skipped.",
        ));
    }

    if let Some(marks) = input.marks_obtained
        && (!marks.is_finite() || !(-MAX_MARKS..=MAX_MARKS).contains(&marks))
    {
        return Err(AppError::validation(
            "marksObtained",
            "Marks obtained must be a finite number between -100000 and 100000.",
        ));
    }

    if let Some(marks) = input.max_marks
        && (!marks.is_finite() || !(marks > 0.0 && marks <= MAX_MARKS))
    {
        return Err(AppError::validation(
            "maxMarks",
            "Maximum marks must be greater than zero and at most 100000.",
        ));
    }

    if let (Some(obtained), Some(maximum)) = (input.marks_obtained, input.max_marks)
        && obtained > maximum
    {
        return Err(AppError::validation(
            "marksObtained",
            "Marks obtained cannot exceed maximum marks.",
        ));
    }

    if !(0..=MAX_TIME_SPENT_SECONDS).contains(&input.time_spent_seconds) {
        return Err(AppError::validation(
            "timeSpentSeconds",
            "Time spent must be between 0 and 86400 seconds.",
        ));
    }

    if input.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    if let Some(difficulty) = input.difficulty
        && !(1..=5).contains(&difficulty)
    {
        return Err(AppError::validation(
            "difficulty",
            "Difficulty must be a whole number from 1 to 5.",
        ));
    }

    Ok(ValidatedPyq {
        question_year: input.question_year,
        exam_session,
        part,
        topic_id: input.topic_id,
        question_reference,
        attempt_date,
        status,
        marks_obtained: input.marks_obtained,
        max_marks: input.max_marks,
        time_spent_seconds: input.time_spent_seconds,
        notes: input.notes,
        difficulty: input.difficulty,
    })
}

fn load(connection: &Connection, attempt_id: i64) -> Result<PyqAttempt, AppError> {
    let sql = format!("{SELECT_PYQ} WHERE p.id = ?1");
    connection
        .query_row(&sql, [attempt_id], map_attempt)
        .optional()
        .map_err(|error| AppError::database("The PYQ attempt could not be loaded", error))?
        .ok_or_else(AppError::pyq_not_found)
}

fn query_attempts<P: rusqlite::Params>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<PyqAttempt>, AppError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("PYQ attempts could not be prepared", error))?;
    statement
        .query_map(params, map_attempt)
        .map_err(|error| AppError::database("PYQ attempts could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ attempts could not be read", error))
}

fn map_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<PyqAttempt> {
    Ok(PyqAttempt {
        id: row.get(0)?,
        question_year: row.get(1)?,
        exam_session: row.get(2)?,
        part: row.get(3)?,
        topic: map_topic_at(row, 4)?,
        question_reference: row.get(10)?,
        attempt_date: row.get(11)?,
        status: row.get(12)?,
        marks_obtained: row.get(13)?,
        max_marks: row.get(14)?,
        time_spent_seconds: row.get(15)?,
        notes: row.get(16)?,
        difficulty: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn map_topic(row: &rusqlite::Row<'_>) -> rusqlite::Result<PyqTopic> {
    map_topic_at(row, 0)
}

fn map_topic_at(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<PyqTopic> {
    Ok(PyqTopic {
        id: row.get(offset)?,
        code: row.get(offset + 1)?,
        title: row.get(offset + 2)?,
        archived: row.get::<_, i64>(offset + 3)? != 0,
        section_code: row.get(offset + 4)?,
        section_title: row.get(offset + 5)?,
    })
}

fn validate_topic(connection: &Connection, topic_id: i64) -> Result<(), AppError> {
    let found: Option<i64> = connection
        .query_row(
            "SELECT id FROM syllabus_topics WHERE id = ?1",
            [topic_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The syllabus topic could not be checked", error))?;

    if found.is_some() {
        Ok(())
    } else {
        Err(AppError::topic_not_found())
    }
}

fn section_exists(connection: &Connection, code: &str) -> Result<bool, AppError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM syllabus_sections WHERE code = ?1)",
            [code],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|error| AppError::database("The syllabus unit could not be checked", error))
}

fn normalize_part(value: &str, field: &'static str) -> Result<Option<String>, AppError> {
    let value = value.trim().to_uppercase();
    if value.is_empty() {
        return Ok(None);
    }
    if PYQ_PARTS.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(AppError::validation(field, "Choose Part A, B or C."))
    }
}

fn normalize_status(value: &str, field: &'static str) -> Result<Option<String>, AppError> {
    let value = value.trim().to_lowercase();
    if value.is_empty() {
        return Ok(None);
    }
    if PYQ_STATUSES.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(AppError::validation(
            field,
            "Choose attempted, correct, incorrect or skipped.",
        ))
    }
}

fn parse_i64(value: &str, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<i64> {
    if value.is_empty() {
        errors.push(ImportFieldError::new(field, "This field is required."));
        return None;
    }

    match value.parse::<i64>() {
        Ok(parsed) => Some(parsed),
        Err(_) => {
            errors.push(ImportFieldError::new(field, "Enter a whole number."));
            None
        }
    }
}

fn parse_optional_f64(value: &str, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<f64> {
    if value.is_empty() {
        return None;
    }

    match value.parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => Some(parsed),
        _ => {
            errors.push(ImportFieldError::new(field, "Enter a finite number."));
            None
        }
    }
}

fn validate_optional_date(
    value: Option<&str>,
    field: &'static str,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| {
            date_only::parse_date_only(value)
                .map(date_only::format_date_only)
                .ok_or_else(|| {
                    AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
                })
        })
        .transpose()
}

fn validate_id(value: i64, field: &'static str) -> Result<(), AppError> {
    if value <= 0 {
        Err(AppError::validation(
            field,
            "The selected record is invalid.",
        ))
    } else {
        Ok(())
    }
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '!' | '%' | '_') {
            escaped.push('!');
        }
        escaped.push(character);
    }

    escaped
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::db;
    use crate::syllabus;

    fn connection() -> Connection {
        let mut connection =
            db::test_connection_with_defaults(NaiveDate::from_ymd_opt(2026, 10, 30).expect("date"));
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

    fn input(topic_id: i64) -> PyqInput {
        PyqInput {
            question_year: 2024,
            exam_session: "June 2024".to_owned(),
            part: "B".to_owned(),
            topic_id,
            question_reference: "Q42".to_owned(),
            attempt_date: "2026-10-30".to_owned(),
            status: "correct".to_owned(),
            marks_obtained: Some(2.0),
            max_marks: Some(2.0),
            time_spent_seconds: 90,
            notes: "Useful revision".to_owned(),
            difficulty: Some(3),
        }
    }

    #[test]
    fn csv_template_matches_the_documented_header_contract() {
        let template = template();
        assert_eq!(template, format!("{}\r\n", CSV_HEADERS.join(",")));
        assert!(template.starts_with("question_year,exam_session,part,topic_code"));
        assert!(template.ends_with("difficulty,external_id\r\n"));
    }

    #[test]
    fn pyq_crud_and_filters_round_trip() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let created = create(&mut connection, input(topic)).expect("attempt should save");

        assert_eq!(created.question_year, 2024);
        assert_eq!(created.topic.code, "MMP-C01");
        assert_eq!(created.topic.section_code, "MMP");
        assert_eq!(created.status, "correct");
        assert_eq!(created.marks_obtained, Some(2.0));

        let mut updated_input = input(topic);
        updated_input.question_reference = "Q43".to_owned();
        updated_input.status = "incorrect".to_owned();
        updated_input.part = "A".to_owned();
        updated_input.marks_obtained = Some(-0.5);
        updated_input.notes = "Read the sign convention".to_owned();
        let updated =
            update(&mut connection, created.id, updated_input).expect("attempt should update");
        assert_eq!(updated.question_reference, "Q43");
        assert_eq!(updated.part, "A");
        assert_eq!(updated.status, "incorrect");

        let all = list(&connection, PyqFilter::default()).expect("attempts should list");
        assert_eq!(all.len(), 1);

        let filtered = list(
            &connection,
            PyqFilter {
                year: Some(2024),
                part: Some("a".to_owned()),
                section_code: Some("MMP".to_owned()),
                topic_id: Some(topic),
                status: Some("INCORRECT".to_owned()),
                query: Some("sign".to_owned()),
                from_date: Some("2026-10-30".to_owned()),
                to_date: Some("2026-10-30".to_owned()),
            },
        )
        .expect("filters should work");
        assert_eq!(filtered.len(), 1);

        let outside_date = list(
            &connection,
            PyqFilter {
                from_date: Some("2026-10-31".to_owned()),
                ..PyqFilter::default()
            },
        )
        .expect("date filter should work");
        assert!(outside_date.is_empty());

        let reversed_date = list(
            &connection,
            PyqFilter {
                from_date: Some("2026-11-01".to_owned()),
                to_date: Some("2026-10-01".to_owned()),
                ..PyqFilter::default()
            },
        )
        .expect_err("reversed dates should fail");
        assert_eq!(reversed_date.field, Some("fromDate"));

        let none = list(
            &connection,
            PyqFilter {
                year: Some(2023),
                ..PyqFilter::default()
            },
        )
        .expect("empty filter should work");
        assert!(none.is_empty());

        let options = options(&connection).expect("options should load");
        assert_eq!(options.years, vec![2024]);
        assert!(options.sections.iter().any(|section| section.code == "MMP"));
        assert!(options.topics.iter().any(|item| item.code == "MMP-C01"));

        delete(&mut connection, created.id).expect("attempt should delete");
        assert!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .is_empty()
        );
        assert_eq!(
            delete(&mut connection, created.id)
                .expect_err("missing attempt should fail")
                .code,
            "pyq_not_found"
        );
    }

    #[test]
    fn pyq_validation_rejects_bad_dates_parts_marks_and_topics() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let before = list(&connection, PyqFilter::default()).expect("attempts should list");

        let invalid = [
            (
                PyqInput {
                    question_year: 1899,
                    ..input(topic)
                },
                "questionYear",
            ),
            (
                PyqInput {
                    part: "D".to_owned(),
                    ..input(topic)
                },
                "part",
            ),
            (
                PyqInput {
                    attempt_date: "2026-02-30".to_owned(),
                    ..input(topic)
                },
                "attemptDate",
            ),
            (
                PyqInput {
                    status: "done".to_owned(),
                    ..input(topic)
                },
                "status",
            ),
            (
                PyqInput {
                    marks_obtained: Some(3.0),
                    max_marks: Some(2.0),
                    ..input(topic)
                },
                "marksObtained",
            ),
            (
                PyqInput {
                    time_spent_seconds: 86_401,
                    ..input(topic)
                },
                "timeSpentSeconds",
            ),
            (
                PyqInput {
                    difficulty: Some(6),
                    ..input(topic)
                },
                "difficulty",
            ),
            (
                PyqInput {
                    topic_id: 999_999,
                    ..input(topic)
                },
                "topicId",
            ),
            (
                PyqInput {
                    question_reference: "   ".to_owned(),
                    ..input(topic)
                },
                "questionReference",
            ),
        ];

        for (invalid_input, field) in invalid {
            let error = create(&mut connection, invalid_input).expect_err("input should fail");
            assert_eq!(error.field, Some(field), "unexpected error: {error:?}");
        }

        assert_eq!(
            list(&connection, PyqFilter::default()).expect("attempts should list"),
            before
        );
    }

    #[test]
    fn csv_preview_validate_duplicates_and_import_transaction() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let csv = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                   2024,June 2024,B,MMP-C01,Q1,2026-10-30,correct,2,2,90,First attempt,3,ext-1\n\
                   2023,June 2023,C,MMP-C01,Q2,2026-02-30,wrong,2,2,60,Bad date,4,\n\
                   2022,June 2022,A,GA-01,Q3,2026-10-28,skipped,,,0,,2,\n";

        let preview = preview_import(&connection, csv).expect("preview should load");
        assert_eq!(preview.total_rows, 3);
        assert_eq!(preview.valid_rows, 2);
        assert_eq!(preview.invalid_rows, 1);
        assert_eq!(preview.duplicate_rows, 0);
        assert!(
            preview.rows[1]
                .errors
                .iter()
                .any(|error| error.field == "attempt_date")
        );
        assert!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .is_empty()
        );

        let result = apply_import(&mut connection, csv).expect("import should run");
        assert_eq!(result.imported, 2);
        assert_eq!(result.rejected, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .len(),
            2
        );

        let second = apply_import(&mut connection, csv).expect("repeat import should run");
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped, 2);
        assert_eq!(second.rejected, 1);
        assert_eq!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .len(),
            2
        );

        let same_key_attempt =
            create(&mut connection, input(topic)).expect("manual attempt should save");
        assert!(same_key_attempt.id > 0);
    }

    #[test]
    fn csv_quoted_notes_are_parsed_and_mid_import_failures_roll_back() {
        let mut connection = connection();
        let quoted = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                      2024,,B,MMP-C01,\"Q \"\"42\"\"\",2026-10-30,correct,,,0,\"Line one, with comma\nLine two\",3,ext-quoted\n";
        let result = apply_import(&mut connection, quoted).expect("quoted import should run");
        assert_eq!(result.imported, 1);
        let saved = list(&connection, PyqFilter::default()).expect("attempts should list");
        assert_eq!(saved[0].question_reference, "Q \"42\"");
        assert_eq!(saved[0].notes, "Line one, with comma\nLine two");

        for attempt in saved {
            delete(&mut connection, attempt.id).expect("attempt should delete");
        }

        connection
            .execute_batch(
                "CREATE TRIGGER fail_pyq_import
                 BEFORE INSERT ON pyq_attempts
                 WHEN NEW.question_reference = 'FAIL'
                 BEGIN
                     SELECT RAISE(ABORT, 'forced import failure');
                 END;",
            )
            .expect("test trigger should install");

        let csv = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                   2024,,B,MMP-C01,FIRST,2026-10-30,correct,,,0,,,\n\
                   2024,,C,GA-01,FAIL,2026-10-29,incorrect,,,0,,,\n";
        let error = apply_import(&mut connection, csv)
            .expect_err("forced database failure should abort the import");
        assert_eq!(error.code, "database_error");
        assert!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .is_empty(),
            "the whole import transaction should roll back"
        );
    }

    #[test]
    fn csv_fingerprints_normalize_year_part_and_topic_code() {
        let connection = connection();
        let csv = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                   02024,,b,mmp-c01,Q1,2026-10-30,correct,,,0,,,\n\
                   2024,,B,MMP-C01,Q1,2026-10-30,correct,,,0,,,\n";

        let preview = preview_import(&connection, csv).expect("preview should load");
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.duplicate_rows, 1);
    }

    #[test]
    fn csv_external_id_detects_duplicates_and_invalid_headers_do_not_write() {
        let mut connection = connection();
        let csv = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                   2024,,B,MMP-C01,Q1,2026-10-30,correct,,,0,,,\n\
                   2024,,C,GA-01,Q2,2026-10-29,incorrect,,,0,,,\n";
        let result = apply_import(&mut connection, csv).expect("first import should run");
        assert_eq!(result.imported, 2);

        let duplicate = "question_year,exam_session,part,topic_code,question_reference,attempt_date,status,marks_obtained,max_marks,time_spent_seconds,notes,difficulty,external_id\n\
                         2024,,B,MMP-C01,Q1,2026-10-30,correct,,,0,,,ext-1\n\
                         2024,,B,MMP-C01,Q1,2026-10-30,correct,,,0,,,ext-1\n";
        let preview = preview_import(&connection, duplicate).expect("preview should work");
        assert_eq!(preview.duplicate_rows, 1);
        assert_eq!(preview.valid_rows, 1);

        let before = list(&connection, PyqFilter::default())
            .expect("attempts should list")
            .len();
        let header_error = preview_import(&connection, "wrong,headers\n1,2\n")
            .expect_err("bad header should fail");
        assert_eq!(header_error.field, Some("file"));
        assert_eq!(
            list(&connection, PyqFilter::default())
                .expect("attempts should list")
                .len(),
            before
        );
    }
}
