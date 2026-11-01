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

pub const MIN_DURATION_MINUTES: i64 = 1;
pub const MAX_DURATION_MINUTES: i64 = 1_440;
pub const MAX_COUNT: i64 = 10_000;
pub const MAX_MARKS: f64 = 100_000.0;
pub const MAX_MARKING_VALUE: f64 = 1_000.0;
pub const MAX_NOTES_CHARS: usize = 2_000;

const SCORE_TOLERANCE_FACTOR: f64 = 1e-9;
const PERCENTAGE_TOLERANCE: f64 = 1e-9;

fn marks_tolerance(value: f64) -> f64 {
    value.abs().max(1.0) * SCORE_TOLERANCE_FACTOR
}

fn percentage_exceeds_maximum(computed_marks: f64, max_marks: f64) -> bool {
    max_marks <= 0.0 || computed_marks / max_marks * 100.0 > 100.0 + PERCENTAGE_TOLERANCE
}

pub const CSV_HEADERS: &[&str] = &[
    "test_name",
    "test_date",
    "duration_minutes",
    "max_marks",
    "marks_obtained",
    "part_a_questions",
    "part_a_correct",
    "part_a_incorrect",
    "part_a_unanswered",
    "part_a_correct_marks",
    "part_a_incorrect_marks",
    "part_a_unanswered_marks",
    "part_b_questions",
    "part_b_correct",
    "part_b_incorrect",
    "part_b_unanswered",
    "part_b_correct_marks",
    "part_b_incorrect_marks",
    "part_b_unanswered_marks",
    "part_c_questions",
    "part_c_correct",
    "part_c_incorrect",
    "part_c_unanswered",
    "part_c_correct_marks",
    "part_c_incorrect_marks",
    "part_c_unanswered_marks",
    "notes",
    "review_observations",
    "external_id",
];

const SELECT_MOCK: &str = "SELECT
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
        created_at,
        updated_at
     FROM mock_tests";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockPartResult {
    pub questions: i64,
    pub attempted: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub correct_marks: f64,
    pub incorrect_marks: f64,
    pub unanswered_marks: f64,
    pub marks: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockTest {
    pub id: i64,
    pub test_name: String,
    pub test_date: String,
    pub duration_minutes: i64,
    pub max_marks: f64,
    pub marks_obtained: Option<f64>,
    pub part_a: MockPartResult,
    pub part_b: MockPartResult,
    pub part_c: MockPartResult,
    pub total_questions: i64,
    pub total_attempted: i64,
    pub total_correct: i64,
    pub total_incorrect: i64,
    pub total_unanswered: i64,
    pub computed_marks: f64,
    pub computed_percentage: f64,
    pub configuration_valid: bool,
    pub notes: String,
    pub review_observations: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockPartInput {
    #[serde(default)]
    pub questions: i64,
    #[serde(default)]
    pub correct: i64,
    #[serde(default)]
    pub incorrect: i64,
    #[serde(default)]
    pub unanswered: i64,
    #[serde(default)]
    pub correct_marks: Option<f64>,
    #[serde(default)]
    pub incorrect_marks: Option<f64>,
    #[serde(default)]
    pub unanswered_marks: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockTestInput {
    pub test_name: String,
    pub test_date: String,
    pub duration_minutes: i64,
    pub max_marks: f64,
    #[serde(default)]
    pub marks_obtained: Option<f64>,
    pub part_a: MockPartInput,
    pub part_b: MockPartInput,
    pub part_c: MockPartInput,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub review_observations: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ValidatedPart {
    questions: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
    correct_marks: f64,
    incorrect_marks: f64,
    unanswered_marks: f64,
}

impl ValidatedPart {
    fn marks(&self) -> f64 {
        self.correct as f64 * self.correct_marks
            + self.incorrect as f64 * self.incorrect_marks
            + self.unanswered as f64 * self.unanswered_marks
    }
}

fn highest_possible_marks(parts: &[ValidatedPart]) -> f64 {
    parts
        .iter()
        .map(|part| part.questions as f64 * part.correct_marks)
        .sum()
}

fn maximum_marks_validation_message(maximum_possible: f64) -> String {
    format!(
        "Maximum marks must be at least the highest score this marking configuration can produce ({maximum_possible:.2})."
    )
}

struct ValidatedMock {
    test_name: String,
    test_date: String,
    duration_minutes: i64,
    max_marks: f64,
    marks_obtained: Option<f64>,
    parts: [ValidatedPart; 3],
    notes: String,
    review_observations: String,
}

impl ValidatedMock {
    fn computed_marks(&self) -> f64 {
        self.parts.iter().map(ValidatedPart::marks).sum()
    }

    fn summary(&self) -> String {
        format!(
            "{} · {} · {:.2}/{:.2}",
            self.test_name,
            self.test_date,
            self.computed_marks(),
            self.max_marks
        )
    }
}

pub fn template() -> String {
    csv_template(CSV_HEADERS)
}

pub fn list(connection: &Connection) -> Result<Vec<MockTest>, AppError> {
    let sql = format!("{SELECT_MOCK} ORDER BY test_date DESC, id DESC");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Mock tests could not be prepared", error))?;

    statement
        .query_map([], map_mock)
        .map_err(|error| AppError::database("Mock tests could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Mock tests could not be read", error))
}

pub fn create(connection: &mut Connection, input: MockTestInput) -> Result<MockTest, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mock-test transaction could not be started", error)
    })?;
    let validated = validate_input(input)?;
    insert(&transaction, &validated, None)?;
    let id = transaction.last_insert_rowid();
    let test = load(&transaction, id)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The mock test could not be committed", error))?;

    Ok(test)
}

pub fn update(
    connection: &mut Connection,
    test_id: i64,
    input: MockTestInput,
) -> Result<MockTest, AppError> {
    validate_id(test_id, "id")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mock-test transaction could not be started", error)
    })?;
    load(&transaction, test_id)?;
    let validated = validate_input(input)?;

    let changed = transaction
        .execute(
            "UPDATE mock_tests
             SET test_name = ?1,
                 test_date = ?2,
                 duration_minutes = ?3,
                 max_marks = ?4,
                 marks_obtained = ?5,
                 part_a_questions = ?6,
                 part_a_correct = ?7,
                 part_a_incorrect = ?8,
                 part_a_unanswered = ?9,
                 part_a_correct_marks = ?10,
                 part_a_incorrect_marks = ?11,
                 part_a_unanswered_marks = ?12,
                 part_b_questions = ?13,
                 part_b_correct = ?14,
                 part_b_incorrect = ?15,
                 part_b_unanswered = ?16,
                 part_b_correct_marks = ?17,
                 part_b_incorrect_marks = ?18,
                 part_b_unanswered_marks = ?19,
                 part_c_questions = ?20,
                 part_c_correct = ?21,
                 part_c_incorrect = ?22,
                 part_c_unanswered = ?23,
                 part_c_correct_marks = ?24,
                 part_c_incorrect_marks = ?25,
                 part_c_unanswered_marks = ?26,
                 notes = ?27,
                 review_observations = ?28,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?29",
            params![
                validated.test_name,
                validated.test_date,
                validated.duration_minutes,
                validated.max_marks,
                validated.marks_obtained,
                validated.parts[0].questions,
                validated.parts[0].correct,
                validated.parts[0].incorrect,
                validated.parts[0].unanswered,
                validated.parts[0].correct_marks,
                validated.parts[0].incorrect_marks,
                validated.parts[0].unanswered_marks,
                validated.parts[1].questions,
                validated.parts[1].correct,
                validated.parts[1].incorrect,
                validated.parts[1].unanswered,
                validated.parts[1].correct_marks,
                validated.parts[1].incorrect_marks,
                validated.parts[1].unanswered_marks,
                validated.parts[2].questions,
                validated.parts[2].correct,
                validated.parts[2].incorrect,
                validated.parts[2].unanswered,
                validated.parts[2].correct_marks,
                validated.parts[2].incorrect_marks,
                validated.parts[2].unanswered_marks,
                validated.notes,
                validated.review_observations,
                test_id,
            ],
        )
        .map_err(|error| AppError::database("The mock test could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::mock_test_not_found());
    }

    let test = load(&transaction, test_id)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The mock test could not be committed", error))?;

    Ok(test)
}

pub fn delete(connection: &mut Connection, test_id: i64) -> Result<(), AppError> {
    validate_id(test_id, "id")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mock-test transaction could not be started", error)
    })?;
    let changed = transaction
        .execute("DELETE FROM mock_tests WHERE id = ?1", [test_id])
        .map_err(|error| AppError::database("The mock test could not be deleted", error))?;

    if changed != 1 {
        return Err(AppError::mock_test_not_found());
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The mock test could not be committed", error))
}

pub fn preview_import(connection: &Connection, csv: &str) -> Result<ImportPreview, AppError> {
    let table = read_table(csv, CSV_HEADERS)?;
    let existing = existing_import_keys(connection)?;
    let rows = evaluate_import_rows(&table, &existing)?;

    Ok(preview_from_rows(rows))
}

pub fn apply_import(connection: &mut Connection, csv: &str) -> Result<ImportResult, AppError> {
    let table = read_table(csv, CSV_HEADERS)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The mock-test import transaction could not be started",
            error,
        )
    })?;
    let existing = existing_import_keys(&transaction)?;
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

        match validate_csv_row(&values) {
            Ok(valid) => {
                let import_key = valid.import_key();
                if existing.contains(&import_key) || !seen.insert(import_key.clone()) {
                    result_rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Duplicate,
                        summary: valid.validated.summary(),
                        errors: vec![ImportFieldError::new(
                            "duplicate",
                            "This row was already imported or appears more than once in the file.",
                        )],
                    });
                    continue;
                }

                insert(&transaction, &valid.validated, Some(&import_key))?;
                result_rows.push(ImportRow {
                    row_number: record.line,
                    status: ImportRowStatus::Imported,
                    summary: valid.validated.summary(),
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
    transaction.commit().map_err(|error| {
        AppError::database("The mock-test import could not be committed", error)
    })?;

    Ok(result)
}

fn evaluate_import_rows(
    table: &CsvTable,
    existing: &HashSet<String>,
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

        match validate_csv_row(&values) {
            Ok(valid) => {
                let import_key = valid.import_key();
                if existing.contains(&import_key) || !seen.insert(import_key) {
                    rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Duplicate,
                        summary: valid.validated.summary(),
                        errors: vec![ImportFieldError::new(
                            "duplicate",
                            "This row was already imported or appears more than once in the file.",
                        )],
                    });
                } else {
                    rows.push(ImportRow {
                        row_number: record.line,
                        status: ImportRowStatus::Valid,
                        summary: valid.validated.summary(),
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

struct ImportedMock {
    validated: ValidatedMock,
    external_id: String,
}

impl ImportedMock {
    fn import_key(&self) -> String {
        if self.external_id.is_empty() {
            let duration = self.validated.duration_minutes.to_string();
            let max_marks = self.validated.max_marks.to_string();
            let reported = self
                .validated
                .marks_obtained
                .map(|value| value.to_string())
                .unwrap_or_default();
            let part_a = part_key(&self.validated.parts[0]);
            let part_b = part_key(&self.validated.parts[1]);
            let part_c = part_key(&self.validated.parts[2]);

            stable_import_key(
                "mock",
                &[
                    &self.validated.test_name,
                    &self.validated.test_date,
                    &duration,
                    &max_marks,
                    &reported,
                    &part_a,
                    &part_b,
                    &part_c,
                ],
            )
        } else {
            stable_import_key("mock-ext", &[&self.external_id])
        }
    }
}

fn part_key(part: &ValidatedPart) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        part.questions,
        part.correct,
        part.incorrect,
        part.unanswered,
        part.correct_marks,
        part.incorrect_marks,
        part.unanswered_marks
    )
}

fn validate_csv_row(values: &HashMap<&str, &str>) -> Result<ImportedMock, Vec<ImportFieldError>> {
    let mut errors = Vec::new();
    let value = |field: &str| values.get(field).copied().unwrap_or("").trim();

    let test_name = value("test_name").to_owned();
    if test_name.is_empty() {
        errors.push(ImportFieldError::new("test_name", "Enter a test name."));
    } else if test_name.chars().count() > 200 {
        errors.push(ImportFieldError::new(
            "test_name",
            "Test name must be 200 characters or fewer.",
        ));
    }

    let test_date = value("test_date");
    if date_only::parse_date_only(test_date).is_none() {
        errors.push(ImportFieldError::new(
            "test_date",
            "Test date must be a real calendar date in YYYY-MM-DD format.",
        ));
    }

    let duration_minutes =
        parse_required_i64(value("duration_minutes"), "duration_minutes", &mut errors);
    if let Some(duration) = duration_minutes
        && !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&duration)
    {
        errors.push(ImportFieldError::new(
            "duration_minutes",
            "Duration must be between 1 and 1440 minutes.",
        ));
    }

    let max_marks = parse_required_f64(value("max_marks"), "max_marks", &mut errors);
    if let Some(max_marks) = max_marks
        && !(max_marks > 0.0 && max_marks <= MAX_MARKS)
    {
        errors.push(ImportFieldError::new(
            "max_marks",
            "Maximum marks must be greater than zero and at most 100000.",
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
    if let (Some(obtained), Some(maximum)) = (marks_obtained, max_marks)
        && obtained > maximum
    {
        errors.push(ImportFieldError::new(
            "marks_obtained",
            "Reported marks cannot exceed maximum marks.",
        ));
    }

    let mut parts = Vec::with_capacity(3);
    for prefix in ["part_a", "part_b", "part_c"] {
        if let Some(part) = parse_csv_part(values, prefix, &mut errors) {
            parts.push(part);
        }
    }

    let notes = value("notes");
    if notes.chars().count() > MAX_NOTES_CHARS {
        errors.push(ImportFieldError::new(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    let review_observations = value("review_observations");
    if review_observations.chars().count() > MAX_NOTES_CHARS {
        errors.push(ImportFieldError::new(
            "review_observations",
            format!("Review observations must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    let external_id = value("external_id");
    if external_id.chars().count() > 200 {
        errors.push(ImportFieldError::new(
            "external_id",
            "External ID must be 200 characters or fewer.",
        ));
    }

    if !errors.is_empty() || parts.len() != 3 {
        if errors.is_empty() {
            errors.push(ImportFieldError::new(
                "row",
                "The part breakdown is incomplete.",
            ));
        }
        return Err(errors);
    }

    if parts.iter().map(|part| part.questions).sum::<i64>() == 0 {
        errors.push(ImportFieldError::new(
            "row",
            "A mock test must contain at least one question across Parts A, B and C.",
        ));
        return Err(errors);
    }

    let highest_possible = highest_possible_marks(&parts);
    if let Some(maximum) = max_marks
        && maximum + marks_tolerance(highest_possible) < highest_possible
    {
        errors.push(ImportFieldError::new(
            "max_marks",
            maximum_marks_validation_message(highest_possible),
        ));
        return Err(errors);
    }

    let validated = ValidatedMock {
        test_name,
        test_date: test_date.to_owned(),
        duration_minutes: duration_minutes.expect("validated duration"),
        max_marks: max_marks.expect("validated maximum marks"),
        marks_obtained,
        parts: [parts[0].clone(), parts[1].clone(), parts[2].clone()],
        notes: notes.to_owned(),
        review_observations: review_observations.to_owned(),
    };

    if percentage_exceeds_maximum(validated.computed_marks(), validated.max_marks) {
        errors.push(ImportFieldError::new(
            "max_marks",
            "The calculated score cannot exceed maximum marks. Review the part counts and marking configuration.",
        ));
        return Err(errors);
    }

    Ok(ImportedMock {
        validated,
        external_id: external_id.to_owned(),
    })
}

fn parse_csv_part(
    values: &HashMap<&str, &str>,
    prefix: &str,
    errors: &mut Vec<ImportFieldError>,
) -> Option<ValidatedPart> {
    let value = |suffix: &str| {
        values
            .get(format!("{prefix}_{suffix}").as_str())
            .copied()
            .unwrap_or("")
            .trim()
    };

    let questions = parse_required_i64(value("questions"), &format!("{prefix}_questions"), errors);
    let correct = parse_required_i64(value("correct"), &format!("{prefix}_correct"), errors);
    let incorrect = parse_required_i64(value("incorrect"), &format!("{prefix}_incorrect"), errors);
    let unanswered =
        parse_required_i64(value("unanswered"), &format!("{prefix}_unanswered"), errors);

    let mut part_errors = Vec::new();
    for (label, count) in [
        ("questions", questions),
        ("correct", correct),
        ("incorrect", incorrect),
        ("unanswered", unanswered),
    ] {
        if let Some(count) = count
            && !(0..=MAX_COUNT).contains(&count)
        {
            part_errors.push(ImportFieldError::new(
                format!("{prefix}_{label}"),
                "Question counts must be between 0 and 10000.",
            ));
        }
    }

    if let (Some(questions), Some(correct), Some(incorrect), Some(unanswered)) =
        (questions, correct, incorrect, unanswered)
    {
        let total = correct
            .checked_add(incorrect)
            .and_then(|value| value.checked_add(unanswered));
        if total != Some(questions) {
            part_errors.push(ImportFieldError::new(
                format!("{prefix}_questions"),
                "Correct + incorrect + unanswered must equal the question count.",
            ));
        }
    }

    let correct_marks = parse_optional_f64(
        value("correct_marks"),
        &format!("{prefix}_correct_marks"),
        errors,
    );
    let incorrect_marks = parse_optional_f64(
        value("incorrect_marks"),
        &format!("{prefix}_incorrect_marks"),
        errors,
    )
    .unwrap_or(0.0);
    let unanswered_marks = parse_optional_f64(
        value("unanswered_marks"),
        &format!("{prefix}_unanswered_marks"),
        errors,
    )
    .unwrap_or(0.0);

    if questions.unwrap_or(0) > 0 && correct_marks.is_none() {
        part_errors.push(ImportFieldError::new(
            format!("{prefix}_correct_marks"),
            "Enter the marks awarded for a correct answer.",
        ));
    }

    let correct_marks_value = correct_marks.unwrap_or(0.0);
    if !(0.0..=MAX_MARKING_VALUE).contains(&correct_marks_value) {
        part_errors.push(ImportFieldError::new(
            format!("{prefix}_correct_marks"),
            "Correct-answer marks must be between 0 and 1000.",
        ));
    }
    if !(-MAX_MARKING_VALUE..=0.0).contains(&incorrect_marks) {
        part_errors.push(ImportFieldError::new(
            format!("{prefix}_incorrect_marks"),
            "Incorrect-answer marks must be zero or negative.",
        ));
    }
    if !(-MAX_MARKING_VALUE..=0.0).contains(&unanswered_marks) {
        part_errors.push(ImportFieldError::new(
            format!("{prefix}_unanswered_marks"),
            "Unanswered marks must be zero or negative.",
        ));
    }

    errors.extend(part_errors);

    Some(ValidatedPart {
        questions: questions?,
        correct: correct?,
        incorrect: incorrect?,
        unanswered: unanswered?,
        correct_marks: correct_marks_value,
        incorrect_marks,
        unanswered_marks,
    })
}

fn parse_required_i64(value: &str, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<i64> {
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

fn parse_required_f64(value: &str, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<f64> {
    if value.is_empty() {
        errors.push(ImportFieldError::new(field, "This field is required."));
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

fn validate_input(input: MockTestInput) -> Result<ValidatedMock, AppError> {
    let test_name = input.test_name.trim().to_owned();
    if test_name.is_empty() {
        return Err(AppError::validation("testName", "Enter a test name."));
    }
    if test_name.chars().count() > 200 {
        return Err(AppError::validation(
            "testName",
            "Test name must be 200 characters or fewer.",
        ));
    }

    if date_only::parse_date_only(&input.test_date).is_none() {
        return Err(AppError::validation(
            "testDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        ));
    }
    let test_date = date_only::format_date_only(
        date_only::parse_date_only(&input.test_date).expect("date was validated"),
    );

    if !(MIN_DURATION_MINUTES..=MAX_DURATION_MINUTES).contains(&input.duration_minutes) {
        return Err(AppError::validation(
            "durationMinutes",
            "Duration must be between 1 and 1440 minutes.",
        ));
    }

    if !input.max_marks.is_finite() || !(input.max_marks > 0.0 && input.max_marks <= MAX_MARKS) {
        return Err(AppError::validation(
            "maxMarks",
            "Maximum marks must be a finite number greater than zero and at most 100000.",
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
    if let Some(marks) = input.marks_obtained
        && marks > input.max_marks
    {
        return Err(AppError::validation(
            "marksObtained",
            "Reported marks cannot exceed maximum marks.",
        ));
    }

    if input.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }
    if input.review_observations.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "reviewObservations",
            format!("Review observations must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    let parts = [
        validate_part("partA", input.part_a)?,
        validate_part("partB", input.part_b)?,
        validate_part("partC", input.part_c)?,
    ];

    if parts.iter().map(|part| part.questions).sum::<i64>() == 0 {
        return Err(AppError::validation(
            "partA",
            "A mock test must contain at least one question across Parts A, B and C.",
        ));
    }

    let highest_possible = highest_possible_marks(&parts);
    if input.max_marks + marks_tolerance(highest_possible) < highest_possible {
        return Err(AppError::validation(
            "maxMarks",
            maximum_marks_validation_message(highest_possible),
        ));
    }

    let validated = ValidatedMock {
        test_name,
        test_date,
        duration_minutes: input.duration_minutes,
        max_marks: input.max_marks,
        marks_obtained: input.marks_obtained,
        parts,
        notes: input.notes,
        review_observations: input.review_observations,
    };

    if validated.computed_marks() > validated.max_marks + marks_tolerance(validated.max_marks)
        || percentage_exceeds_maximum(validated.computed_marks(), validated.max_marks)
    {
        return Err(AppError::validation(
            "maxMarks",
            "The calculated score cannot exceed maximum marks. Review the part counts and marking configuration.",
        ));
    }

    Ok(validated)
}

fn validate_part(field: &'static str, input: MockPartInput) -> Result<ValidatedPart, AppError> {
    for count in [
        input.questions,
        input.correct,
        input.incorrect,
        input.unanswered,
    ] {
        if !(0..=MAX_COUNT).contains(&count) {
            return Err(AppError::validation(
                field,
                "Question counts must be between 0 and 10000.",
            ));
        }
    }

    if input.correct + input.incorrect + input.unanswered != input.questions {
        return Err(AppError::validation(
            field,
            "Correct + incorrect + unanswered must equal the question count.",
        ));
    }

    let correct_marks = input.correct_marks.unwrap_or(0.0);
    if input.questions > 0 && input.correct_marks.is_none() {
        return Err(AppError::validation(
            field,
            "Enter the marks awarded for a correct answer.",
        ));
    }
    if !correct_marks.is_finite() || !(0.0..=MAX_MARKING_VALUE).contains(&correct_marks) {
        return Err(AppError::validation(
            field,
            "Correct-answer marks must be between 0 and 1000.",
        ));
    }

    let incorrect_marks = input.incorrect_marks.unwrap_or(0.0);
    if !incorrect_marks.is_finite() || !(-MAX_MARKING_VALUE..=0.0).contains(&incorrect_marks) {
        return Err(AppError::validation(
            field,
            "Incorrect-answer marks must be zero or negative. Use 0 when wrong answers are not penalised.",
        ));
    }

    let unanswered_marks = input.unanswered_marks.unwrap_or(0.0);
    if !unanswered_marks.is_finite() || !(-MAX_MARKING_VALUE..=0.0).contains(&unanswered_marks) {
        return Err(AppError::validation(
            field,
            "Unanswered marks must be zero or negative. Use 0 when unanswered questions are not penalised.",
        ));
    }

    Ok(ValidatedPart {
        questions: input.questions,
        correct: input.correct,
        incorrect: input.incorrect,
        unanswered: input.unanswered,
        correct_marks,
        incorrect_marks,
        unanswered_marks,
    })
}

fn insert(
    connection: &Connection,
    validated: &ValidatedMock,
    import_key: Option<&str>,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO mock_tests (
                test_name, test_date, duration_minutes, max_marks, marks_obtained,
                part_a_questions, part_a_correct, part_a_incorrect, part_a_unanswered,
                part_a_correct_marks, part_a_incorrect_marks, part_a_unanswered_marks,
                part_b_questions, part_b_correct, part_b_incorrect, part_b_unanswered,
                part_b_correct_marks, part_b_incorrect_marks, part_b_unanswered_marks,
                part_c_questions, part_c_correct, part_c_incorrect, part_c_unanswered,
                part_c_correct_marks, part_c_incorrect_marks, part_c_unanswered_marks,
                notes, review_observations, import_key, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23, ?24, ?25, ?26,
                ?27, ?28, ?29,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.test_name,
                validated.test_date,
                validated.duration_minutes,
                validated.max_marks,
                validated.marks_obtained,
                validated.parts[0].questions,
                validated.parts[0].correct,
                validated.parts[0].incorrect,
                validated.parts[0].unanswered,
                validated.parts[0].correct_marks,
                validated.parts[0].incorrect_marks,
                validated.parts[0].unanswered_marks,
                validated.parts[1].questions,
                validated.parts[1].correct,
                validated.parts[1].incorrect,
                validated.parts[1].unanswered,
                validated.parts[1].correct_marks,
                validated.parts[1].incorrect_marks,
                validated.parts[1].unanswered_marks,
                validated.parts[2].questions,
                validated.parts[2].correct,
                validated.parts[2].incorrect,
                validated.parts[2].unanswered,
                validated.parts[2].correct_marks,
                validated.parts[2].incorrect_marks,
                validated.parts[2].unanswered_marks,
                validated.notes,
                validated.review_observations,
                import_key,
            ],
        )
        .map_err(|error| AppError::database("The mock test could not be saved", error))?;

    Ok(())
}

fn load(connection: &Connection, test_id: i64) -> Result<MockTest, AppError> {
    let sql = format!("{SELECT_MOCK} WHERE id = ?1");
    connection
        .query_row(&sql, [test_id], map_mock)
        .optional()
        .map_err(|error| AppError::database("The mock test could not be loaded", error))?
        .ok_or_else(AppError::mock_test_not_found)
}

fn map_mock(row: &rusqlite::Row<'_>) -> rusqlite::Result<MockTest> {
    let part_a = map_part(row, 6)?;
    let part_b = map_part(row, 13)?;
    let part_c = map_part(row, 20)?;
    let computed_marks = part_a.marks + part_b.marks + part_c.marks;
    let max_marks: f64 = row.get(4)?;
    let total_attempted = part_a.attempted + part_b.attempted + part_c.attempted;
    let configuration_valid = valid_marking_configuration(max_marks, [&part_a, &part_b, &part_c]);

    Ok(MockTest {
        id: row.get(0)?,
        test_name: row.get(1)?,
        test_date: row.get(2)?,
        duration_minutes: row.get(3)?,
        max_marks,
        marks_obtained: row.get(5)?,
        total_questions: part_a.questions + part_b.questions + part_c.questions,
        total_attempted,
        total_correct: part_a.correct + part_b.correct + part_c.correct,
        total_incorrect: part_a.incorrect + part_b.incorrect + part_c.incorrect,
        total_unanswered: part_a.unanswered + part_b.unanswered + part_c.unanswered,
        computed_marks,
        computed_percentage: computed_marks / max_marks * 100.0,
        configuration_valid,
        part_a,
        part_b,
        part_c,
        notes: row.get(27)?,
        review_observations: row.get(28)?,
        created_at: row.get(29)?,
        updated_at: row.get(30)?,
    })
}

fn map_part(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<MockPartResult> {
    let correct: i64 = row.get(offset + 1)?;
    let incorrect: i64 = row.get(offset + 2)?;
    let correct_marks: f64 = row.get(offset + 4)?;
    let incorrect_marks: f64 = row.get(offset + 5)?;
    let unanswered_marks: f64 = row.get(offset + 6)?;
    let unanswered: i64 = row.get(offset + 3)?;

    Ok(MockPartResult {
        questions: row.get(offset)?,
        attempted: correct + incorrect,
        correct,
        incorrect,
        unanswered,
        correct_marks,
        incorrect_marks,
        unanswered_marks,
        marks: correct as f64 * correct_marks
            + incorrect as f64 * incorrect_marks
            + unanswered as f64 * unanswered_marks,
    })
}

fn valid_marking_configuration(max_marks: f64, parts: [&MockPartResult; 3]) -> bool {
    let highest_possible = parts
        .iter()
        .map(|part| part.questions as f64 * part.correct_marks)
        .sum::<f64>();
    let computed_marks = parts.iter().map(|part| part.marks).sum::<f64>();
    let maximum_covers_possible = max_marks + marks_tolerance(highest_possible) >= highest_possible
        && !percentage_exceeds_maximum(computed_marks, max_marks);

    maximum_covers_possible
        && parts.iter().all(|part| {
            part.correct_marks.is_finite()
                && part.correct_marks >= 0.0
                && part.incorrect_marks.is_finite()
                && part.incorrect_marks <= 0.0
                && part.unanswered_marks.is_finite()
                && part.unanswered_marks <= 0.0
                && part.correct + part.incorrect + part.unanswered == part.questions
        })
}

fn existing_import_keys(connection: &Connection) -> Result<HashSet<String>, AppError> {
    connection
        .prepare("SELECT import_key FROM mock_tests WHERE import_key IS NOT NULL")
        .map_err(|error| {
            AppError::database("Existing mock import keys could not be prepared", error)
        })?
        .query_map([], |row| row.get(0))
        .map_err(|error| {
            AppError::database("Existing mock import keys could not be queried", error)
        })?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Existing mock import keys could not be read", error))
}

fn summary_from_values(values: &HashMap<&str, &str>) -> String {
    let name = values.get("test_name").copied().unwrap_or("").trim();
    let date = values.get("test_date").copied().unwrap_or("").trim();

    match (name.is_empty(), date.is_empty()) {
        (false, false) => format!("{name} · {date}"),
        (false, true) => name.to_owned(),
        _ => String::new(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn connection() -> Connection {
        db::test_connection_with_defaults(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 30).expect("date"),
        )
    }

    fn part(
        questions: i64,
        correct: i64,
        incorrect: i64,
        unanswered: i64,
        correct_marks: f64,
        incorrect_marks: f64,
    ) -> MockPartInput {
        MockPartInput {
            questions,
            correct,
            incorrect,
            unanswered,
            correct_marks: Some(correct_marks),
            incorrect_marks: Some(incorrect_marks),
            unanswered_marks: Some(0.0),
        }
    }

    fn input() -> MockTestInput {
        MockTestInput {
            test_name: "Mock 1".to_owned(),
            test_date: "2026-10-30".to_owned(),
            duration_minutes: 180,
            max_marks: 240.0,
            marks_obtained: Some(140.0),
            part_a: part(20, 15, 3, 2, 2.0, -0.5),
            part_b: part(40, 25, 10, 5, 3.0, -1.0),
            part_c: part(20, 10, 5, 5, 4.0, -1.0),
            notes: "First full mock".to_owned(),
            review_observations: "Time management needs work".to_owned(),
        }
    }

    #[test]
    fn csv_template_matches_the_documented_header_contract() {
        let template = template();
        assert_eq!(template, format!("{}\r\n", CSV_HEADERS.join(",")));
        assert!(template.starts_with("test_name,test_date,duration_minutes,max_marks"));
        assert!(template.ends_with("notes,review_observations,external_id\r\n"));
    }

    #[test]
    fn mock_crud_and_scoring_use_the_saved_marking_configuration() {
        let mut connection = connection();
        let created = create(&mut connection, input()).expect("mock should save");

        // 15*2 + 3*(-0.5) + 2*0 = 28.5
        // 25*3 + 10*(-1) + 5*0 = 65
        // 10*4 + 5*(-1) + 5*0 = 35
        assert_eq!(created.computed_marks, 128.5);
        assert!((created.computed_percentage - 53.54166666666667).abs() < 1e-9);
        assert_eq!(created.total_questions, 80);
        assert_eq!(created.total_attempted, 68);
        assert_eq!(created.total_correct, 50);
        assert_eq!(created.total_incorrect, 18);
        assert_eq!(created.total_unanswered, 12);
        assert_eq!(created.part_a.marks, 28.5);
        assert_eq!(created.marks_obtained, Some(140.0));

        let mut updated_input = input();
        updated_input.part_a.correct_marks = Some(1.0);
        updated_input.part_a.incorrect_marks = Some(0.0);
        updated_input.part_b.correct_marks = Some(3.5);
        updated_input.part_b.incorrect_marks = Some(-1.25);
        updated_input.part_c.correct_marks = Some(0.0);
        updated_input.part_c.incorrect_marks = Some(0.0);
        let updated =
            update(&mut connection, created.id, updated_input).expect("mock should update");
        assert_eq!(updated.part_a.marks, 15.0);
        assert_eq!(updated.part_b.marks, 75.0);
        assert_eq!(updated.part_c.marks, 0.0);
        assert_eq!(updated.computed_marks, 90.0);
        assert_eq!(updated.computed_percentage, 37.5);

        let all = list(&connection).expect("mocks should list");
        assert_eq!(all.len(), 1);
        delete(&mut connection, created.id).expect("mock should delete");
        assert!(list(&connection).expect("mocks should list").is_empty());
        assert_eq!(
            delete(&mut connection, created.id)
                .expect_err("missing mock should fail")
                .code,
            "mock_test_not_found"
        );
    }

    #[test]
    fn saved_invalid_marking_configurations_are_flagged_without_rewriting() {
        let mut connection = connection();
        let created = create(&mut connection, input()).expect("mock should save");
        assert!(created.configuration_valid);

        connection
            .execute(
                "UPDATE mock_tests SET max_marks = 200 WHERE id = ?1",
                [created.id],
            )
            .expect("legacy maximum should change");
        let lower_maximum = load(&connection, created.id).expect("mock should load");
        assert!(!lower_maximum.configuration_valid);
        assert_eq!(lower_maximum.max_marks, 200.0);
        assert!((lower_maximum.computed_percentage - 64.25).abs() < 1e-9);

        connection
            .execute(
                "UPDATE mock_tests
                 SET max_marks = 240,
                     part_a_incorrect_marks = 1
                 WHERE id = ?1",
                [created.id],
            )
            .expect("legacy penalty should change");
        let positive_penalty = load(&connection, created.id).expect("mock should load");
        assert!(!positive_penalty.configuration_valid);
    }

    #[test]
    fn mock_validation_rejects_maximum_marks_within_the_old_rounding_margin() {
        let mut connection = connection();
        let mut invalid = input();
        invalid.max_marks = 239.995;

        let error = create(&mut connection, invalid).expect_err("maximum must be rejected");
        assert_eq!(error.field, Some("maxMarks"));
    }

    #[test]
    fn mock_validation_rejects_tiny_maximum_marks_with_an_impossible_percentage() {
        let mut connection = connection();
        let mut invalid = input();
        invalid.max_marks = 0.001;
        invalid.marks_obtained = None;
        invalid.part_a = part(1, 1, 0, 0, 0.001000001, 0.0);
        invalid.part_b = part(0, 0, 0, 0, 0.0, 0.0);
        invalid.part_c = part(0, 0, 0, 0, 0.0, 0.0);

        let error = create(&mut connection, invalid).expect_err("percentage must be rejected");
        assert_eq!(error.field, Some("maxMarks"));
    }

    #[test]
    fn csv_import_rejects_tiny_maximum_marks_with_an_impossible_percentage() {
        let connection = connection();
        let header = CSV_HEADERS.join(",");
        let csv = format!(
            "{header}\n\
             Tiny Maximum,2026-10-30,180,0.001,,1,1,0,0,0.001000001,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,,,\n"
        );

        let preview = preview_import(&connection, &csv).expect("preview should load");
        assert_eq!(preview.valid_rows, 0);
        assert_eq!(preview.invalid_rows, 1);
        assert!(
            preview.rows[0]
                .errors
                .iter()
                .any(|error| error.field == "max_marks")
        );
    }

    #[test]
    fn mock_validation_rejects_inconsistent_breakdowns_and_bad_marking() {
        let mut connection = connection();
        let before = list(&connection).expect("mocks should list");

        let invalid = [
            (
                MockTestInput {
                    test_name: "  ".to_owned(),
                    ..input()
                },
                "testName",
            ),
            (
                MockTestInput {
                    test_date: "2026-02-30".to_owned(),
                    ..input()
                },
                "testDate",
            ),
            (
                MockTestInput {
                    duration_minutes: 0,
                    ..input()
                },
                "durationMinutes",
            ),
            (
                MockTestInput {
                    max_marks: 0.0,
                    ..input()
                },
                "maxMarks",
            ),
            (
                MockTestInput {
                    max_marks: 200.0,
                    ..input()
                },
                "maxMarks",
            ),
            (
                MockTestInput {
                    marks_obtained: Some(250.0),
                    ..input()
                },
                "marksObtained",
            ),
            (
                MockTestInput {
                    part_a: part(20, 15, 3, 2, 2.0, 1.0),
                    ..input()
                },
                "partA",
            ),
            (
                MockTestInput {
                    part_a: MockPartInput {
                        unanswered_marks: Some(1.0),
                        ..part(20, 15, 3, 2, 2.0, -0.5)
                    },
                    ..input()
                },
                "partA",
            ),
            (
                MockTestInput {
                    part_a: MockPartInput {
                        questions: 20,
                        correct: 19,
                        incorrect: 0,
                        unanswered: 0,
                        ..part(20, 19, 0, 0, 2.0, -0.5)
                    },
                    ..input()
                },
                "partA",
            ),
            (
                MockTestInput {
                    part_a: MockPartInput {
                        correct_marks: None,
                        ..part(20, 15, 3, 2, 2.0, -0.5)
                    },
                    ..input()
                },
                "partA",
            ),
            (
                MockTestInput {
                    part_a: part(20, 15, 3, 2, -1.0, -0.5),
                    ..input()
                },
                "partA",
            ),
            (
                MockTestInput {
                    part_a: part(0, 0, 0, 0, 0.0, 0.0),
                    part_b: part(0, 0, 0, 0, 0.0, 0.0),
                    part_c: part(0, 0, 0, 0, 0.0, 0.0),
                    ..input()
                },
                "partA",
            ),
        ];

        for (invalid_input, field) in invalid {
            let error = create(&mut connection, invalid_input).expect_err("input should fail");
            assert_eq!(error.field, Some(field), "unexpected error: {error:?}");
        }

        assert_eq!(list(&connection).expect("mocks should list"), before);
    }

    #[test]
    fn csv_preview_and_import_skip_duplicates_and_reject_invalid_rows() {
        let mut connection = connection();
        let csv = "test_name,test_date,duration_minutes,max_marks,marks_obtained,part_a_questions,part_a_correct,part_a_incorrect,part_a_unanswered,part_a_correct_marks,part_a_incorrect_marks,part_a_unanswered_marks,part_b_questions,part_b_correct,part_b_incorrect,part_b_unanswered,part_b_correct_marks,part_b_incorrect_marks,part_b_unanswered_marks,part_c_questions,part_c_correct,part_c_incorrect,part_c_unanswered,part_c_correct_marks,part_c_incorrect_marks,part_c_unanswered_marks,notes,review_observations,external_id\n\
                    Mock A,2026-10-30,180,240,,20,15,3,2,2,-0.5,0,40,25,10,5,3,-1,0,20,10,5,5,4,-1,0,Good run,Review later,mock-1\n\
                   Bad Date,2026-02-30,180,240,,20,15,3,2,2,-0.5,0,40,25,10,5,3,-1,0,20,10,5,5,4,-1,0,,,\n\
                    No Questions,2026-10-29,180,240,,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,,,\n";

        let preview = preview_import(&connection, csv).expect("preview should load");
        assert_eq!(preview.total_rows, 3);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.invalid_rows, 2);

        let result = apply_import(&mut connection, csv).expect("import should run");
        assert_eq!(result.imported, 1);
        assert_eq!(result.rejected, 2);
        assert_eq!(list(&connection).expect("mocks should list").len(), 1);

        let repeat = apply_import(&mut connection, csv).expect("repeat import should run");
        assert_eq!(repeat.imported, 0);
        assert_eq!(repeat.skipped, 1);
        assert_eq!(repeat.rejected, 2);
        assert_eq!(list(&connection).expect("mocks should list").len(), 1);
    }

    #[test]
    fn csv_import_rejects_positive_penalties_and_understated_maximum_marks() {
        let connection = connection();
        let header = CSV_HEADERS.join(",");
        let csv = format!(
            "{header}\n\
             Low Maximum,2026-10-30,180,10,,20,15,3,2,2,-0.5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,,,\n\
             Positive Penalty,2026-10-30,180,40,,1,1,0,0,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,,,\n"
        );

        let preview = preview_import(&connection, &csv).expect("preview should load");
        assert_eq!(preview.valid_rows, 0);
        assert_eq!(preview.invalid_rows, 2);
        assert!(
            preview.rows[0]
                .errors
                .iter()
                .any(|error| error.field == "max_marks")
        );
        assert!(
            preview.rows[1]
                .errors
                .iter()
                .any(|error| error.field == "part_a_incorrect_marks")
        );
    }

    #[test]
    fn csv_part_count_overflow_is_rejected_without_panicking() {
        let mut values = HashMap::new();
        values.insert("part_a_questions", "9000000000000000000");
        values.insert("part_a_correct", "9000000000000000000");
        values.insert("part_a_incorrect", "9000000000000000000");
        values.insert("part_a_unanswered", "0");
        values.insert("part_a_correct_marks", "2");
        values.insert("part_a_incorrect_marks", "0");
        values.insert("part_a_unanswered_marks", "0");

        let mut errors = Vec::new();
        let part = parse_csv_part(&values, "part_a", &mut errors);

        assert!(part.is_some());
        assert!(errors.iter().any(|error| error.field == "part_a_questions"));
    }

    #[test]
    fn csv_headers_and_marking_columns_are_validated_without_writes() {
        let connection = connection();
        let before = list(&connection).expect("mocks should list").len();

        let error = preview_import(&connection, "test_name,wrong\nMock,1\n")
            .expect_err("bad header should fail");
        assert_eq!(error.field, Some("file"));
        assert_eq!(list(&connection).expect("mocks should list").len(), before);

        let csv = "test_name,test_date,duration_minutes,max_marks,marks_obtained,part_a_questions,part_a_correct,part_a_incorrect,part_a_unanswered,part_a_correct_marks,part_a_incorrect_marks,part_a_unanswered_marks,part_b_questions,part_b_correct,part_b_incorrect,part_b_unanswered,part_b_correct_marks,part_b_incorrect_marks,part_b_unanswered_marks,part_c_questions,part_c_correct,part_c_incorrect,part_c_unanswered,part_c_correct_marks,part_c_incorrect_marks,part_c_unanswered_marks,notes,review_observations,external_id\n\
                   Missing Marking,2026-10-30,180,200,,20,15,3,2,,-0.5,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,,,\n";
        let preview = preview_import(&connection, csv).expect("preview should load");
        assert_eq!(preview.invalid_rows, 1);
        assert!(
            preview.rows[0]
                .errors
                .iter()
                .any(|error| error.field == "part_a_correct_marks")
        );
    }
}
