use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::AppError;

use super::pyq::{
    PatternSectionOption, PatternVersionOption, V2PracticeExamInput, V2PyqFilter,
    V2PyqQuestionSummary, V2QuestionOption,
};
use super::scoring::{self, normalize_choice};

pub const MOCK_KIND_QUESTION_LEVEL: &str = "question_level";

const TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS mock_test_questions (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL,
    mock_test_id INTEGER NOT NULL,
    pattern_version_id INTEGER NOT NULL
        REFERENCES exam_pattern_versions(id) ON DELETE RESTRICT,
    pattern_section_id INTEGER NOT NULL
        REFERENCES exam_pattern_sections(id) ON DELETE RESTRICT,
    section_id INTEGER NOT NULL,
    question_id INTEGER NOT NULL REFERENCES pyq_questions(id) ON DELETE RESTRICT,
    display_order INTEGER NOT NULL CHECK (display_order >= 0),
    section_order INTEGER NOT NULL DEFAULT 0 CHECK (section_order >= 0),
    question_number TEXT NOT NULL CHECK (length(question_number) <= 60),
    section_code TEXT NOT NULL CHECK (length(section_code) <= 40),
    section_title TEXT NOT NULL CHECK (length(section_title) <= 200),
    question_type_code TEXT NOT NULL CHECK (length(question_type_code) <= 40),
    topic_id INTEGER,
    topic_title TEXT NOT NULL DEFAULT '' CHECK (length(topic_title) <= 200),
    max_marks REAL NOT NULL CHECK (max_marks > 0),
    negative_marks REAL NOT NULL DEFAULT 0 CHECK (negative_marks <= 0),
    unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks <= 0),
    numeric_tolerance REAL NOT NULL DEFAULT 0 CHECK (numeric_tolerance >= 0),
    section_max_attempts INTEGER CHECK (
        section_max_attempts IS NULL OR section_max_attempts > 0
    ),
    first_answered_at TEXT,
    not_evaluated INTEGER NOT NULL DEFAULT 0 CHECK (not_evaluated IN (0,1)),
    correct_answer_at_attempt TEXT NOT NULL DEFAULT ''
        CHECK (length(correct_answer_at_attempt) <= 4000),
    correct_range_at_attempt TEXT NOT NULL DEFAULT ''
        CHECK (length(correct_range_at_attempt) <= 200),
    answer_key_version TEXT NOT NULL DEFAULT ''
        CHECK (length(answer_key_version) <= 120),
    question_snapshot_json TEXT NOT NULL CHECK (json_valid(question_snapshot_json)),
    selected_answer TEXT NOT NULL DEFAULT '' CHECK (length(selected_answer) <= 4000),
    visited INTEGER NOT NULL DEFAULT 0 CHECK (visited IN (0,1)),
    marked_for_review INTEGER NOT NULL DEFAULT 0 CHECK (marked_for_review IN (0,1)),
    response_status TEXT NOT NULL DEFAULT 'unanswered'
        CHECK (response_status IN ('unanswered','correct','incorrect')),
    marks_obtained REAL NOT NULL DEFAULT 0,
    time_spent_seconds INTEGER NOT NULL DEFAULT 0 CHECK (time_spent_seconds >= 0),
    last_saved_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (mock_test_id, display_order),
    UNIQUE (mock_test_id, question_id),
    FOREIGN KEY (exam_id, mock_test_id)
        REFERENCES mock_tests(exam_id, id) ON DELETE CASCADE,
    FOREIGN KEY (exam_id, section_id)
        REFERENCES exam_sections(exam_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (topic_id) REFERENCES exam_syllabus_nodes(id) ON DELETE SET NULL
) STRICT;

CREATE INDEX IF NOT EXISTS idx_mock_test_questions_mock_order
    ON mock_test_questions(mock_test_id, display_order);
CREATE INDEX IF NOT EXISTS idx_mock_test_questions_section
    ON mock_test_questions(mock_test_id, section_id, display_order);
CREATE INDEX IF NOT EXISTS idx_mock_test_questions_topic
    ON mock_test_questions(mock_test_id, topic_id);
"#;

pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    connection.execute_batch(TABLES).map_err(|error| {
        AppError::database("Question-level mock tables could not be prepared", error)
    })?;

    let columns: &[(&str, &str, &str)] = &[
        (
            "mock_tests",
            "mock_kind",
            "ALTER TABLE mock_tests ADD COLUMN mock_kind TEXT NOT NULL DEFAULT 'aggregate' CHECK (mock_kind IN ('aggregate','question_level'))",
        ),
        (
            "mock_tests",
            "deadline_at",
            "ALTER TABLE mock_tests ADD COLUMN deadline_at TEXT",
        ),
        (
            "mock_tests",
            "current_question_id",
            "ALTER TABLE mock_tests ADD COLUMN current_question_id INTEGER",
        ),
        (
            "mock_tests",
            "last_activity_at",
            "ALTER TABLE mock_tests ADD COLUMN last_activity_at TEXT",
        ),
        (
            "mock_tests",
            "submission_kind",
            "ALTER TABLE mock_tests ADD COLUMN submission_kind TEXT NOT NULL DEFAULT '' CHECK (submission_kind IN ('','manual','automatic'))",
        ),
        (
            "mock_test_questions",
            "section_max_attempts",
            "ALTER TABLE mock_test_questions ADD COLUMN section_max_attempts INTEGER CHECK (section_max_attempts IS NULL OR section_max_attempts > 0)",
        ),
        (
            "mock_test_questions",
            "first_answered_at",
            "ALTER TABLE mock_test_questions ADD COLUMN first_answered_at TEXT",
        ),
        (
            "mock_test_questions",
            "not_evaluated",
            "ALTER TABLE mock_test_questions ADD COLUMN not_evaluated INTEGER NOT NULL DEFAULT 0 CHECK (not_evaluated IN (0,1))",
        ),
    ];
    for (table, column, sql) in columns {
        if !super::patterns::has_column(connection, table, column)? {
            connection.execute_batch(sql).map_err(|error| {
                AppError::database(
                    &format!("The question-level mock column {table}.{column} could not be added"),
                    error,
                )
            })?;
        }
    }
    connection
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_mock_tests_kind_status
             ON mock_tests(exam_id, mock_kind, status, deadline_at);",
        )
        .map_err(|error| {
            AppError::database("Question-level mock indexes could not be prepared", error)
        })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockCandidate {
    pub question: V2PyqQuestionSummary,
    pub pattern_section_id: i64,
    pub pattern_section_code: String,
    pub pattern_section_title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StartQuestionMockInput {
    pub exam_id: i64,
    pub pattern_version_id: i64,
    pub test_name: String,
    pub test_date: String,
    #[serde(default = "default_mode")]
    pub test_mode: String,
    #[serde(default)]
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub focus_node_id: Option<i64>,
    #[serde(default)]
    pub notes: String,
    pub question_ids: Vec<i64>,
}

fn default_mode() -> String {
    "custom".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SaveQuestionMockInput {
    pub exam_id: i64,
    pub mock_id: i64,
    #[serde(default)]
    pub mock_question_id: Option<i64>,
    #[serde(default)]
    pub selected_answer: Option<String>,
    #[serde(default)]
    pub marked_for_review: Option<bool>,
    #[serde(default)]
    pub current_question_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SubmitQuestionMockInput {
    pub exam_id: i64,
    pub mock_id: i64,
    #[serde(default)]
    pub confirmation: bool,
    #[serde(default)]
    pub automatic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockConcept {
    pub concept_id: i64,
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockQuestion {
    pub id: i64,
    pub question_id: i64,
    pub display_order: i64,
    pub section_id: i64,
    pub pattern_section_id: i64,
    pub section_code: String,
    pub section_title: String,
    pub section_order: i64,
    pub question_type_code: String,
    pub question_number: String,
    pub topic_id: Option<i64>,
    pub topic_title: String,
    pub statement: String,
    pub options: Vec<V2QuestionOption>,
    pub selected_answer: String,
    pub visited: bool,
    pub marked_for_review: bool,
    pub not_evaluated: bool,
    pub response_status: String,
    pub marks_obtained: f64,
    pub max_marks: f64,
    pub negative_marks: f64,
    pub unanswered_marks: f64,
    pub numeric_tolerance: f64,
    pub time_spent_seconds: i64,
    pub correct_answer: Option<String>,
    pub correct_range: Option<String>,
    pub answer_key_version: String,
    pub official_explanation: String,
    pub third_party_solution: String,
    pub user_note: String,
    pub source_title: String,
    pub source_page: String,
    pub source_url: Option<String>,
    pub difficulty: i64,
    pub year: i64,
    pub session_label: String,
    pub concepts: Vec<V2QuestionMockConcept>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockSection {
    pub id: i64,
    pub section_id: i64,
    pub pattern_section_id: Option<i64>,
    pub section_code: String,
    pub section_title: String,
    pub display_order: i64,
    pub max_attempts: Option<i64>,
    pub question_count: i64,
    pub answered: i64,
    pub marked: i64,
    pub not_visited: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub max_marks: f64,
    pub marks_obtained: f64,
    pub time_spent_seconds: i64,
    pub average_time_seconds: Option<f64>,
    pub accuracy_percentage: Option<f64>,
    pub correct_marks: f64,
    pub incorrect_marks: f64,
    pub unanswered_marks: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockTopic {
    pub topic_id: i64,
    pub topic_title: String,
    pub questions: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub accuracy_percentage: Option<f64>,
    pub average_time_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockCompatibleAttempt {
    pub mock_id: i64,
    pub test_date: String,
    pub test_name: String,
    pub marks_obtained: Option<f64>,
    pub max_marks: f64,
    pub percentage: Option<f64>,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockSession {
    pub mock_id: i64,
    pub exam_id: i64,
    pub pattern_version_id: Option<i64>,
    pub pattern_version_label: Option<String>,
    pub pattern_title: Option<String>,
    pub test_name: String,
    pub test_date: String,
    pub duration_minutes: i64,
    pub test_mode: String,
    pub status: String,
    pub mock_kind: String,
    pub started_at: Option<String>,
    pub deadline_at: Option<String>,
    pub ended_at: Option<String>,
    pub server_now: String,
    pub remaining_seconds: Option<i64>,
    pub current_question_id: Option<i64>,
    pub last_activity_at: Option<String>,
    pub submission_kind: String,
    pub max_marks: f64,
    pub marks_obtained: Option<f64>,
    pub percentage: Option<f64>,
    pub available_questions: i64,
    pub attempted_questions: i64,
    pub correct_questions: i64,
    pub incorrect_questions: i64,
    pub unanswered_questions: i64,
    pub total_time_seconds: i64,
    pub average_time_seconds: Option<f64>,
    pub questions: Vec<V2QuestionMockQuestion>,
    pub sections: Vec<V2QuestionMockSection>,
    pub topics: Vec<V2QuestionMockTopic>,
    pub compatible_attempts: Vec<V2QuestionMockCompatibleAttempt>,
    pub scheme_snapshot_json: String,
    pub notes: String,
    pub review_observations: String,
    pub is_archived: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionMockSaveResult {
    pub saved: bool,
    pub status: String,
    pub auto_submitted: bool,
    pub server_now: String,
    pub remaining_seconds: Option<i64>,
    pub current_question_id: Option<i64>,
    pub answered_questions: i64,
    pub marked_questions: i64,
    pub not_visited_questions: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuestionSnapshot {
    statement: String,
    options: Vec<V2QuestionOption>,
    official_explanation: String,
    third_party_solution: String,
    user_note: String,
    source_title: String,
    source_page: String,
    source_url: Option<String>,
    difficulty: i64,
    year: i64,
    session_label: String,
    concepts: Vec<V2QuestionMockConcept>,
}

#[derive(Debug, Clone)]
struct StartItem {
    candidate: V2QuestionMockCandidate,
    pattern_section: PatternSectionOption,
    correct_answer: String,
    correct_range: String,
    answer_key_version: String,
    snapshot: String,
}
#[derive(Default)]
struct SectionAccumulator {
    section_id: i64,
    section_code: String,
    section_title: String,
    section_order: i64,
    question_count: i64,
    visited: i64,
    answered: i64,
    marked: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
    max_marks: f64,
    marks_obtained: f64,
    time_spent_seconds: i64,
}

pub fn candidates(
    connection: &Connection,
    exam_id: i64,
    pattern_version_id: i64,
) -> Result<Vec<V2QuestionMockCandidate>, AppError> {
    let options = super::pyq::options(connection, V2PracticeExamInput { exam_id })?;
    let version = options
        .patterns
        .iter()
        .find(|pattern| pattern.id == pattern_version_id)
        .ok_or_else(|| {
            AppError::validation(
                "patternVersionId",
                "Choose a pattern version from this examination.",
            )
        })?;
    if version.verification_status != "verified" {
        return Err(AppError::validation(
            "patternVersionId",
            "Question-level mocks require a verified pattern version.",
        ));
    }
    let sections_by_section_id = version
        .sections
        .iter()
        .map(|section| (section.section_id, section))
        .collect::<HashMap<_, _>>();
    let questions = super::pyq::list_questions(
        connection,
        V2PyqFilter {
            exam_id,
            year: None,
            section_id: None,
            question_type_id: None,
            syllabus_node_id: None,
            verification_status: Some("verified".to_owned()),
            question_id: None,
            query: None,
            include_archived: Some(false),
            limit: Some(500),
            offset: Some(0),
        },
    )?;
    let mut result = Vec::new();
    for question in questions {
        if question.pattern_version_id != Some(pattern_version_id)
            || !question.answer_key_verified
            || question.verification_status != "verified"
        {
            continue;
        }
        let Some(section) = sections_by_section_id.get(&question.section_id) else {
            continue;
        };
        if !section
            .question_types
            .iter()
            .any(|code| code.eq_ignore_ascii_case(&question.question_type_code))
        {
            continue;
        }
        let choices = question.question_type_code.eq_ignore_ascii_case("MCQ")
            || question.question_type_code.eq_ignore_ascii_case("MSQ");
        if choices && question.options.len() < 2 {
            continue;
        }
        if !choices {
            let has_answer: i64 = connection
                .query_row(
                    "SELECT CASE WHEN trim(correct_answer) <> '' OR trim(correct_range) <> '' THEN 1 ELSE 0 END
                     FROM pyq_questions WHERE id = ?1",
                    [question.id],
                    |row| row.get(0),
                )
                .map_err(|error| AppError::database("Question answer key could not be checked", error))?;
            if has_answer == 0 {
                continue;
            }
        }
        result.push(V2QuestionMockCandidate {
            question,
            pattern_section_id: section.id,
            pattern_section_code: section.code.clone(),
            pattern_section_title: section.title.clone(),
        });
    }
    result.sort_by(|left, right| {
        let left_section = sections_by_section_id
            .get(&left.question.section_id)
            .map(|section| section.display_order)
            .unwrap_or(0);
        let right_section = sections_by_section_id
            .get(&right.question.section_id)
            .map(|section| section.display_order)
            .unwrap_or(0);
        left_section
            .cmp(&right_section)
            .then_with(|| {
                question_number_value(&left.question.question_number)
                    .cmp(&question_number_value(&right.question.question_number))
            })
            .then_with(|| left.question.id.cmp(&right.question.id))
    });
    Ok(result)
}

fn question_number_value(value: &str) -> i64 {
    value.trim().parse::<i64>().unwrap_or(i64::MAX)
}

fn active_exam_code(connection: &Connection, exam_id: i64) -> Result<String, AppError> {
    let row: Option<(String, i64)> = connection
        .query_row(
            "SELECT e.code, p.is_active
             FROM exams e JOIN exam_profiles p ON p.exam_id = e.id
             WHERE e.id = ?1",
            [exam_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            AppError::database("Question-level mock exam could not be checked", error)
        })?;
    match row {
        Some((code, 1)) => Ok(code),
        Some(_) => Err(AppError::forbidden(
            "examId",
            "Inactive examinations are read-only. Reactivate the examination to start or submit a mock.",
        )),
        None => Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        )),
    }
}

fn validate_focus_node(
    connection: &Connection,
    exam_id: i64,
    node_id: i64,
) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM exam_syllabus_nodes
                WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0
             )",
            params![node_id, exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Mock focus topic could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "focusNodeId",
            "Choose an active syllabus topic from this examination.",
        ));
    }
    Ok(())
}

fn build_scheme_snapshot(version: &PatternVersionOption) -> Value {
    json!({
        "schemaVersion": 1,
        "mockKind": MOCK_KIND_QUESTION_LEVEL,
        "patternVersionId": version.id,
        "patternVersionLabel": version.version_label,
        "patternTitle": version.pattern_title,
        "durationMinutes": version.duration_minutes,
        "patternTotalMarks": version.total_marks,
        "patternTotalQuestions": version.total_questions,
        "sections": version.sections.iter().map(|section| json!({
            "patternSectionId": section.id,
            "sectionId": section.section_id,
            "code": section.code,
            "title": section.title,
            "displayOrder": section.display_order,
            "questionCount": section.question_count,
            "maxAttempts": section.max_attempts,
            "maxMarks": section.max_marks,
            "correctMarks": section.correct_marks,
            "incorrectMarks": section.incorrect_marks,
            "unansweredMarks": section.unanswered_marks,
            "questionTypes": section.question_types,
            "markingVariants": section.marking_variants,
            "numericTolerance": section.numeric_tolerance,
            "partialRule": section.partial_rule,
        })).collect::<Vec<_>>()
    })
}

fn build_question_snapshot(
    connection: &Connection,
    question: &V2PyqQuestionSummary,
) -> Result<String, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT c.id, c.slug, c.title
             FROM pyq_question_concepts pc
             JOIN concepts c ON c.id = pc.concept_id
             WHERE pc.question_id = ?1
             ORDER BY pc.is_primary DESC, c.title COLLATE NOCASE, c.id",
        )
        .map_err(|error| AppError::database("Question concepts could not be prepared", error))?;
    let concepts = statement
        .query_map([question.id], |row| {
            Ok(V2QuestionMockConcept {
                concept_id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
            })
        })
        .map_err(|error| AppError::database("Question concepts could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A question concept is invalid", error))?;
    drop(statement);
    let snapshot = json!({
        "statement": question.statement,
        "options": question.options,
        "officialExplanation": question.official_explanation,
        "thirdPartySolution": question.third_party_solution,
        "userNote": question.user_note,
        "sourceTitle": question.source_title,
        "sourcePage": question.source_page,
        "sourceUrl": question.source_url,
        "difficulty": question.difficulty,
        "year": question.year,
        "sessionLabel": question.session_label,
        "concepts": concepts,
    });
    serde_json::to_string(&snapshot)
        .map_err(|error| AppError::database("Question snapshot could not be encoded", error))
}

fn start_items(
    connection: &Connection,
    exam_id: i64,
    version: &PatternVersionOption,
    input: &V2StartQuestionMockInput,
) -> Result<(Vec<StartItem>, String, i64), AppError> {
    let candidates = candidates(connection, exam_id, version.id)?;
    let by_id = candidates
        .into_iter()
        .map(|candidate| (candidate.question.id, candidate))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::with_capacity(input.question_ids.len());
    let mut seen = HashSet::new();
    for question_id in &input.question_ids {
        if !seen.insert(*question_id) {
            return Err(AppError::validation(
                "questionIds",
                "A question can appear only once in a mock.",
            ));
        }
        let candidate = by_id.get(question_id).ok_or_else(|| {
            AppError::validation(
                "questionIds",
                format!(
                    "Question {question_id} is not a verified, answer-key-verified question in the selected pattern."
                ),
            )
        })?;
        selected.push(candidate.clone());
    }
    if selected.is_empty() {
        return Err(AppError::validation(
            "questionIds",
            "Select at least one eligible question.",
        ));
    }
    let mode = input.test_mode.trim().to_ascii_lowercase();
    if !matches!(
        mode.as_str(),
        "full_length" | "section" | "custom" | "topic" | "past_paper"
    ) {
        return Err(AppError::validation(
            "testMode",
            "Choose full_length, section, custom, topic or past_paper.",
        ));
    }
    let section_order = version
        .sections
        .iter()
        .map(|section| (section.id, section.display_order))
        .collect::<HashMap<_, _>>();
    let section_by_id = version
        .sections
        .iter()
        .map(|section| (section.id, section))
        .collect::<HashMap<_, _>>();
    selected.sort_by(|left, right| {
        let left_order = section_order
            .get(&left.pattern_section_id)
            .copied()
            .unwrap_or(0);
        let right_order = section_order
            .get(&right.pattern_section_id)
            .copied()
            .unwrap_or(0);
        left_order
            .cmp(&right_order)
            .then_with(|| {
                question_number_value(&left.question.question_number)
                    .cmp(&question_number_value(&right.question.question_number))
            })
            .then_with(|| left.question.id.cmp(&right.question.id))
    });
    if mode == "full_length" {
        for section in &version.sections {
            let count = selected
                .iter()
                .filter(|item| item.pattern_section_id == section.id)
                .count() as i64;
            if count != section.question_count {
                return Err(AppError::validation(
                    "questionIds",
                    format!(
                        "Full-length mock section {} requires {} questions; {} selected.",
                        section.code, section.question_count, count
                    ),
                ));
            }
        }
    } else if mode == "section" {
        let first = selected[0].pattern_section_id;
        if selected.iter().any(|item| item.pattern_section_id != first) {
            return Err(AppError::validation(
                "questionIds",
                "A section mock must use questions from a single pattern section.",
            ));
        }
        let section = section_by_id.get(&first).ok_or_else(|| {
            AppError::validation("questionIds", "The selected section is not in the pattern.")
        })?;
        if selected.len() as i64 > section.question_count {
            return Err(AppError::validation(
                "questionIds",
                format!(
                    "Section {} accepts at most {} questions.",
                    section.code, section.question_count
                ),
            ));
        }
    }
    if mode == "topic" && input.focus_node_id.is_none() {
        return Err(AppError::validation(
            "focusNodeId",
            "Choose a syllabus topic for a topic-based mock.",
        ));
    }
    let mut start_items = Vec::with_capacity(selected.len());
    for candidate in selected {
        let pattern_section = section_by_id
            .get(&candidate.pattern_section_id)
            .ok_or_else(|| {
                AppError::validation(
                    "questionIds",
                    "The selected question is not in the selected pattern version.",
                )
            })?;
        let (correct_answer, correct_range, answer_key_version): (String, String, String) =
            connection
                .query_row(
                    "SELECT correct_answer, correct_range, answer_key_version
                 FROM pyq_questions WHERE id = ?1",
                    [candidate.question.id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|error| {
                    AppError::database("Question answer key could not be read", error)
                })?;
        let snapshot = build_question_snapshot(connection, &candidate.question)?;
        start_items.push(StartItem {
            candidate,
            pattern_section: (*pattern_section).clone(),
            correct_answer,
            correct_range,
            answer_key_version,
            snapshot,
        });
    }
    let scheme = build_scheme_snapshot(version);
    let question_count = start_items.len() as i64;
    Ok((start_items, scheme.to_string(), question_count))
}

#[allow(clippy::too_many_lines)]
pub fn start(
    connection: &mut Connection,
    input: V2StartQuestionMockInput,
) -> Result<V2QuestionMockSession, AppError> {
    let _exam_code = active_exam_code(connection, input.exam_id)?;
    if chrono::NaiveDate::parse_from_str(&input.test_date, "%Y-%m-%d").is_err() {
        return Err(AppError::validation(
            "testDate",
            "Enter a valid date in YYYY-MM-DD form.",
        ));
    }
    let test_name = input.test_name.trim().to_owned();
    if test_name.is_empty() || test_name.chars().count() > 200 {
        return Err(AppError::validation(
            "testName",
            "Mock name must be between 1 and 200 characters.",
        ));
    }
    if let Some(node_id) = input.focus_node_id {
        validate_focus_node(connection, input.exam_id, node_id)?;
    }
    let options = super::pyq::options(
        connection,
        V2PracticeExamInput {
            exam_id: input.exam_id,
        },
    )?;
    let version = options
        .patterns
        .iter()
        .find(|pattern| pattern.id == input.pattern_version_id)
        .ok_or_else(|| {
            AppError::validation(
                "patternVersionId",
                "Choose a pattern version from this examination.",
            )
        })?;
    if version.verification_status != "verified" {
        return Err(AppError::validation(
            "patternVersionId",
            "Question-level mocks require a verified pattern version.",
        ));
    }
    let (items, scheme_snapshot, _question_count) =
        start_items(connection, input.exam_id, version, &input)?;
    let duration = input
        .duration_minutes
        .or(version.duration_minutes)
        .ok_or_else(|| {
            AppError::validation(
                "durationMinutes",
                "This verified pattern does not state a duration; enter the time limit.",
            )
        })?;
    if !(1..=1440).contains(&duration) {
        return Err(AppError::validation(
            "durationMinutes",
            "Duration must be between 1 and 1,440 minutes.",
        ));
    }
    if input.test_mode.eq_ignore_ascii_case("full_length")
        && let Some(expected) = version.duration_minutes
        && input
            .duration_minutes
            .is_some_and(|value| value != expected)
    {
        return Err(AppError::validation(
            "durationMinutes",
            format!("The verified full-length pattern duration is {expected} minutes."),
        ));
    }
    let total_max = items
        .iter()
        .map(|item| item.candidate.question.max_marks)
        .sum::<f64>();
    if total_max <= 0.0 {
        return Err(AppError::validation(
            "questionIds",
            "The selected questions do not have a positive maximum mark.",
        ));
    }
    let now = Utc::now();
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let deadline_text =
        (now + Duration::minutes(duration)).to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let mode = input.test_mode.trim().to_ascii_lowercase();
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("Question-level mock transaction could not start", error)
    })?;
    transaction
        .execute(
            "INSERT INTO mock_tests(
                exam_id, test_name, test_date, duration_minutes, max_marks, marks_obtained,
                status, notes, review_observations, is_archived, created_at, updated_at,
                pattern_version_id, test_mode, scheme_snapshot_json, available_questions,
                attempted_questions, correct_questions, incorrect_questions,
                unanswered_questions, percentage, focus_node_id, started_at, ended_at,
                mock_kind, deadline_at, current_question_id, last_activity_at, submission_kind
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, 'in_progress', ?6, '', 0, ?7, ?7,
                       ?8, ?9, ?10, ?11, 0, 0, 0, ?11, NULL, ?12, ?7, NULL,
                       'question_level', ?13, NULL, ?7, '')",
            params![
                input.exam_id,
                test_name,
                input.test_date,
                duration,
                total_max,
                input.notes.trim(),
                now_text,
                input.pattern_version_id,
                mode,
                scheme_snapshot,
                items.len() as i64,
                input.focus_node_id,
                deadline_text,
            ],
        )
        .map_err(|error| AppError::database("Question-level mock could not be stored", error))?;
    let mock_id = transaction.last_insert_rowid();
    let mut first_item_id = None;
    let mut sections: BTreeMap<i64, SectionAccumulator> = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let order = index as i64 + 1;
        let section =
            sections
                .entry(item.pattern_section.id)
                .or_insert_with(|| SectionAccumulator {
                    section_id: item.pattern_section.section_id,
                    section_code: item.pattern_section.code.clone(),
                    section_title: item.pattern_section.title.clone(),
                    section_order: item.pattern_section.display_order,
                    question_count: 0,
                    visited: 0,
                    answered: 0,
                    marked: 0,
                    correct: 0,
                    incorrect: 0,
                    unanswered: 0,
                    max_marks: 0.0,
                    marks_obtained: 0.0,
                    time_spent_seconds: 0,
                });
        section.question_count += 1;
        section.unanswered += 1;
        section.max_marks += item.candidate.question.max_marks;
        transaction
            .execute(
                "INSERT INTO mock_test_questions(
                    exam_id, mock_test_id, pattern_version_id, pattern_section_id, section_id,
                    question_id, display_order, section_order, question_number, section_code,
                    section_title, question_type_code, topic_id, topic_title, max_marks,
                    negative_marks, unanswered_marks, numeric_tolerance,
                    correct_answer_at_attempt, correct_range_at_attempt, answer_key_version,
                    question_snapshot_json, selected_answer, visited, marked_for_review,
                    response_status, marks_obtained, time_spent_seconds, section_max_attempts,
                    first_answered_at, not_evaluated, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                           ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, '', 0, 0, 'unanswered', 0, 0,
                           ?23, NULL, 0, ?24, ?24)",
                params![
                    input.exam_id,
                    mock_id,
                    input.pattern_version_id,
                    item.pattern_section.id,
                    item.pattern_section.section_id,
                    item.candidate.question.id,
                    order,
                    item.pattern_section.display_order,
                    item.candidate.question.question_number,
                    item.pattern_section.code,
                    item.pattern_section.title,
                    item.candidate.question.question_type_code,
                    item.candidate.question.syllabus_node_id,
                    item.candidate
                        .question
                        .syllabus_node_title
                        .clone()
                        .unwrap_or_default(),
                    item.candidate.question.max_marks,
                    item.candidate.question.negative_marks,
                    item.candidate.question.unanswered_marks,
                    item.candidate.question.numeric_tolerance,
                    item.correct_answer,
                    item.correct_range,
                    item.answer_key_version,
                    item.snapshot,
                    item.pattern_section.max_attempts,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("Question-level mock item could not be stored", error)
            })?;
        let item_id = transaction.last_insert_rowid();
        if first_item_id.is_none() {
            first_item_id = Some(item_id);
        }
    }
    for (pattern_section_id, section) in &sections {
        let section_snapshot = json!({
            "schemaVersion": 1,
            "patternSectionId": pattern_section_id,
            "sectionCode": section.section_code,
            "sectionTitle": section.section_title,
            "questionCount": section.question_count,
            "maxMarks": section.max_marks,
        });
        transaction
            .execute(
                "INSERT INTO mock_test_sections(
                    exam_id, mock_test_id, section_id, question_count, correct, incorrect,
                    unanswered, max_marks, marks_obtained, time_spent_minutes, notes,
                    created_at, updated_at, pattern_section_id, question_type_id,
                    correct_marks, incorrect_marks, unanswered_marks, section_snapshot_json
                 ) SELECT ?1, ?2, ps.section_id, ?3, 0, 0, ?3, ?4, 0, 0, '', ?5, ?5, ps.id,
                          es.default_question_type_id, ps.correct_marks, ps.incorrect_marks,
                          ps.unanswered_marks, ?6
                  FROM exam_pattern_sections ps
                  JOIN exam_sections es ON es.id = ps.section_id
                  WHERE ps.id = ?7",
                params![
                    input.exam_id,
                    mock_id,
                    section.question_count,
                    section.max_marks,
                    now_text,
                    section_snapshot.to_string(),
                    pattern_section_id,
                ],
            )
            .map_err(|error| {
                AppError::database("Question-level mock section could not be stored", error)
            })?;
    }
    if let Some(first) = first_item_id {
        transaction
            .execute(
                "UPDATE mock_test_questions SET visited = 1 WHERE id = ?1",
                [first],
            )
            .map_err(|error| {
                AppError::database("First mock question could not be visited", error)
            })?;
        transaction
            .execute(
                "UPDATE mock_tests SET current_question_id = ?1, last_activity_at = ?2 WHERE id = ?3",
                params![first, now_text, mock_id],
            )
            .map_err(|error| AppError::database("Question-level mock navigation state could not be stored", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Question-level mock could not be committed", error))?;
    build_session(connection, input.exam_id, mock_id, Utc::now())
}

fn parse_deadline(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn remaining_seconds(deadline: Option<&str>, now: DateTime<Utc>) -> Option<i64> {
    deadline
        .and_then(parse_deadline)
        .map(|deadline| (deadline - now).num_seconds().max(0))
}

fn accrue_current_item_time(
    transaction: &rusqlite::Transaction<'_>,
    mock_id: i64,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let row: Option<(Option<i64>, Option<String>)> = transaction
        .query_row(
            "SELECT current_question_id, last_activity_at FROM mock_tests WHERE id = ?1",
            [mock_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Mock activity state could not be read", error))?;
    if let Some((Some(item_id), Some(last))) = row
        && let Some(last) = parse_deadline(&last)
    {
        let seconds = (now - last).num_seconds().clamp(0, 3600);
        if seconds > 0 {
            transaction
                .execute(
                    "UPDATE mock_test_questions
                     SET time_spent_seconds = time_spent_seconds + ?1
                     WHERE id = ?2 AND mock_test_id = ?3",
                    params![seconds, item_id, mock_id],
                )
                .map_err(|error| AppError::database("Question time could not be accrued", error))?;
        }
    }
    Ok(())
}

fn save_result(
    connection: &Connection,
    exam_id: i64,
    mock_id: i64,
    now: DateTime<Utc>,
    auto_submitted: bool,
) -> Result<V2QuestionMockSaveResult, AppError> {
    let (status, deadline, current): (String, Option<String>, Option<i64>) = connection
        .query_row(
            "SELECT status, deadline_at, current_question_id
             FROM mock_tests WHERE id = ?1 AND exam_id = ?2 AND mock_kind = 'question_level'",
            params![mock_id, exam_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| AppError::database("Mock save state could not be read", error))?;
    let counts: (i64, i64, i64) = connection
        .query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(CASE WHEN selected_answer <> '' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(marked_for_review), 0)
             FROM mock_test_questions WHERE mock_test_id = ?1",
            [mock_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| AppError::database("Mock response counts could not be read", error))?;
    let not_visited: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM mock_test_questions WHERE mock_test_id = ?1 AND visited = 0",
            [mock_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Mock visited count could not be read", error))?;
    Ok(V2QuestionMockSaveResult {
        saved: true,
        status,
        auto_submitted,
        server_now: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        remaining_seconds: remaining_seconds(deadline.as_deref(), now),
        current_question_id: current,
        answered_questions: counts.1,
        marked_questions: counts.2,
        not_visited_questions: not_visited,
    })
}

pub fn save_state(
    connection: &mut Connection,
    input: V2SaveQuestionMockInput,
) -> Result<V2QuestionMockSaveResult, AppError> {
    let _ = active_exam_code(connection, input.exam_id)?;
    let now = Utc::now();
    if maybe_auto_submit(connection, input.exam_id, input.mock_id, now)? {
        return save_result(connection, input.exam_id, input.mock_id, now, true);
    }
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("Question-level mock save could not start", error))?;
    let status: Option<String> = transaction
        .query_row(
            "SELECT status FROM mock_tests
             WHERE id = ?1 AND exam_id = ?2 AND mock_kind = 'question_level'",
            params![input.mock_id, input.exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("Question-level mock status could not be read", error)
        })?;
    let Some(status) = status else {
        return Err(AppError::mock_test_not_found());
    };
    if status == "completed" {
        drop(transaction);
        return save_result(connection, input.exam_id, input.mock_id, now, false);
    }
    if status != "in_progress" {
        return Err(AppError::validation(
            "mockId",
            "This mock is not accepting responses.",
        ));
    }
    accrue_current_item_time(&transaction, input.mock_id, now)?;
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    if let Some(mock_question_id) = input.mock_question_id {
        let item_type: Option<String> = transaction
            .query_row(
                "SELECT question_type_code FROM mock_test_questions
                 WHERE id = ?1 AND mock_test_id = ?2",
                params![mock_question_id, input.mock_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AppError::database("Mock question type could not be read", error))?;
        let item_type = item_type.ok_or_else(|| {
            AppError::validation(
                "mockQuestionId",
                "The mock question does not belong to this mock.",
            )
        })?;
        if let Some(selected_answer) = input.selected_answer {
            let selected =
                if item_type.eq_ignore_ascii_case("MCQ") || item_type.eq_ignore_ascii_case("MSQ") {
                    normalize_choice(&selected_answer)
                } else {
                    selected_answer.trim().to_owned()
                };
            let (old_selected, old_first_answered): (String, Option<String>) = transaction
                .query_row(
                    "SELECT selected_answer, first_answered_at
                     FROM mock_test_questions WHERE id = ?1 AND mock_test_id = ?2",
                    params![mock_question_id, input.mock_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|error| {
                    AppError::database("Mock response state could not be read", error)
                })?;
            let first_answered = if selected.is_empty() {
                None
            } else if old_selected.trim().is_empty() {
                Some(now_text.clone())
            } else {
                old_first_answered
            };
            transaction
                .execute(
                    "UPDATE mock_test_questions
                     SET selected_answer = ?1, first_answered_at = ?2, visited = 1,
                         response_status = CASE WHEN ?1 = '' THEN 'unanswered' ELSE response_status END,
                         marks_obtained = CASE WHEN ?1 = '' THEN 0 ELSE marks_obtained END,
                         last_saved_at = ?3, updated_at = ?3
                     WHERE id = ?4 AND mock_test_id = ?5",
                    params![
                        selected,
                        first_answered,
                        now_text,
                        mock_question_id,
                        input.mock_id,
                    ],
                )
                .map_err(|error| AppError::database("Mock response could not be saved", error))?;
        }
        if let Some(marked) = input.marked_for_review {
            transaction
                .execute(
                    "UPDATE mock_test_questions
                     SET marked_for_review = ?1, visited = 1, last_saved_at = ?2, updated_at = ?2
                     WHERE id = ?3 AND mock_test_id = ?4",
                    params![i64::from(marked), now_text, mock_question_id, input.mock_id],
                )
                .map_err(|error| {
                    AppError::database("Mock review flag could not be saved", error)
                })?;
        }
        transaction
            .execute(
                "UPDATE mock_test_questions SET visited = 1 WHERE id = ?1 AND mock_test_id = ?2",
                params![mock_question_id, input.mock_id],
            )
            .map_err(|error| {
                AppError::database("Mock question visit state could not be saved", error)
            })?;
    }
    if let Some(current) = input.current_question_id {
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM mock_test_questions WHERE id = ?1 AND mock_test_id = ?2)",
                params![current, input.mock_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Mock current question could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "currentQuestionId",
                "The current question does not belong to this mock.",
            ));
        }
        transaction
            .execute(
                "UPDATE mock_test_questions SET visited = 1 WHERE id = ?1",
                [current],
            )
            .map_err(|error| {
                AppError::database("Mock current question could not be visited", error)
            })?;
        transaction
            .execute(
                "UPDATE mock_tests SET current_question_id = ?1 WHERE id = ?2",
                params![current, input.mock_id],
            )
            .map_err(|error| {
                AppError::database("Mock navigation state could not be saved", error)
            })?;
    }
    transaction
        .execute(
            "UPDATE mock_tests SET last_activity_at = ?1 WHERE id = ?2",
            params![now_text, input.mock_id],
        )
        .map_err(|error| AppError::database("Mock activity time could not be saved", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Mock response could not be committed", error))?;
    save_result(connection, input.exam_id, input.mock_id, now, false)
}

#[allow(clippy::too_many_lines)]
fn submit_internal(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
    now: DateTime<Utc>,
    automatic: bool,
) -> Result<bool, AppError> {
    let exam_code = active_exam_code(connection, exam_id)?;
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("Mock submission could not start", error))?;
    let status: Option<String> = transaction
        .query_row(
            "SELECT status FROM mock_tests
             WHERE id = ?1 AND exam_id = ?2 AND mock_kind = 'question_level'",
            params![mock_id, exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Mock submission status could not be read", error))?;
    let Some(status) = status else {
        return Err(AppError::mock_test_not_found());
    };
    if status == "completed" {
        return Ok(false);
    }
    if status != "in_progress" {
        return Err(AppError::validation(
            "mockId",
            "Only an in-progress question-level mock can be submitted.",
        ));
    }
    accrue_current_item_time(&transaction, mock_id, now)?;
    let items = {
        let mut statement = transaction
            .prepare(
                "SELECT id, section_id, section_code, question_type_code, max_marks,
                        correct_answer_at_attempt, correct_range_at_attempt,
                        numeric_tolerance, selected_answer, marked_for_review,
                        time_spent_seconds, display_order, first_answered_at,
                        section_max_attempts
                 FROM mock_test_questions WHERE mock_test_id = ?1 ORDER BY display_order",
            )
            .map_err(|error| AppError::database("Mock questions could not be prepared", error))?;
        statement
            .query_map([mock_id], |row| {
                Ok(ScoringItem {
                    id: row.get(0)?,
                    section_id: row.get(1)?,
                    section_code: row.get(2)?,
                    question_type_code: row.get(3)?,
                    max_marks: row.get(4)?,
                    correct_answer: row.get(5)?,
                    correct_range: row.get(6)?,
                    numeric_tolerance: row.get(7)?,
                    selected_answer: row.get(8)?,
                    marked_for_review: row.get::<_, i64>(9)? == 1,
                    time_spent_seconds: row.get(10)?,
                    display_order: row.get(11)?,
                    first_answered_at: row.get(12)?,
                    section_max_attempts: row.get(13)?,
                })
            })
            .map_err(|error| AppError::database("Mock questions could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A mock question is invalid", error))?
    };
    if items.is_empty() {
        return Err(AppError::validation(
            "mockId",
            "This mock has no questions.",
        ));
    }
    let mut answered_by_section: BTreeMap<i64, Vec<&ScoringItem>> = BTreeMap::new();
    for item in &items {
        if !item.selected_answer.trim().is_empty() {
            answered_by_section
                .entry(item.section_id)
                .or_default()
                .push(item);
        }
    }
    let mut evaluated_ids = HashSet::new();
    for answered in answered_by_section.values_mut() {
        answered.sort_by(|left, right| {
            left.first_answered_at
                .as_deref()
                .unwrap_or("9999-12-31T23:59:59Z")
                .cmp(
                    right
                        .first_answered_at
                        .as_deref()
                        .unwrap_or("9999-12-31T23:59:59Z"),
                )
                .then_with(|| left.display_order.cmp(&right.display_order))
        });
        let limit = answered.first().and_then(|item| item.section_max_attempts);
        for (index, item) in answered.iter().enumerate() {
            if limit.is_none_or(|limit| index < limit as usize) {
                evaluated_ids.insert(item.id);
            }
        }
    }
    let mut section_totals: BTreeMap<i64, SectionAccumulator> = BTreeMap::new();
    let mut total_max = 0.0;
    let mut total_marks = 0.0;
    let mut correct_total = 0;
    let mut incorrect_total = 0;
    let mut unanswered_total = 0;
    let mut scoring_json = Vec::with_capacity(items.len());
    for item in items {
        let not_evaluated =
            !item.selected_answer.trim().is_empty() && !evaluated_ids.contains(&item.id);
        let (status, marks) = if item.selected_answer.trim().is_empty() || not_evaluated {
            ("unanswered".to_owned(), 0.0)
        } else {
            let scored = scoring::score_question(
                &exam_code,
                &item.section_code,
                &item.question_type_code,
                item.max_marks,
                &item.correct_answer,
                &item.correct_range,
                &item.selected_answer,
                item.numeric_tolerance,
            )
            .map_err(|message| AppError::validation("mockId", message))?;
            (scored.status, scored.marks)
        };
        transaction
            .execute(
                "UPDATE mock_test_questions
                 SET response_status = ?1, marks_obtained = ?2, not_evaluated = ?3,
                     last_saved_at = ?4, updated_at = ?4
                 WHERE id = ?5",
                params![status, marks, i64::from(not_evaluated), now_text, item.id],
            )
            .map_err(|error| AppError::database("Mock question score could not be saved", error))?;
        total_max += item.max_marks;
        total_marks += marks;
        match status.as_str() {
            "correct" => correct_total += 1,
            "incorrect" => incorrect_total += 1,
            _ => unanswered_total += 1,
        }
        let section = section_totals
            .entry(item.section_id)
            .or_insert_with(|| SectionAccumulator {
                section_id: item.section_id,
                section_code: String::new(),
                section_title: String::new(),
                section_order: 0,
                question_count: 0,
                visited: 0,
                answered: 0,
                marked: 0,
                correct: 0,
                incorrect: 0,
                unanswered: 0,
                max_marks: 0.0,
                marks_obtained: 0.0,
                time_spent_seconds: 0,
            });
        section.question_count += 1;
        section.max_marks += item.max_marks;
        section.marks_obtained += marks;
        section.time_spent_seconds += item.time_spent_seconds;
        match status.as_str() {
            "correct" => section.correct += 1,
            "incorrect" => section.incorrect += 1,
            _ => section.unanswered += 1,
        }
        scoring_json.push(json!({
            "mockQuestionId": item.id,
            "status": status,
            "selectedAnswer": item.selected_answer,
            "marksObtained": marks,
            "maxMarks": item.max_marks,
            "markedForReview": item.marked_for_review,
            "notEvaluated": not_evaluated,
        }));
    }
    if total_max <= 0.0 {
        return Err(AppError::validation(
            "mockId",
            "Mock maximum marks are invalid.",
        ));
    }
    let percentage = (total_marks * 100.0 / total_max * 1000.0).round() / 1000.0;
    for section in section_totals.values() {
        transaction
            .execute(
                "UPDATE mock_test_sections
                 SET correct = ?1, incorrect = ?2, unanswered = ?3, max_marks = ?4,
                     marks_obtained = ?5, time_spent_minutes = ?6, updated_at = ?7
                 WHERE mock_test_id = ?8 AND section_id = ?9",
                params![
                    section.correct,
                    section.incorrect,
                    section.unanswered,
                    section.max_marks,
                    section.marks_obtained,
                    section.time_spent_seconds / 60,
                    now_text,
                    mock_id,
                    section.section_id,
                ],
            )
            .map_err(|error| AppError::database("Mock section totals could not be saved", error))?;
    }
    let submission_kind = if automatic { "automatic" } else { "manual" };
    transaction
        .execute(
            "UPDATE mock_tests
             SET max_marks = ?1, marks_obtained = ?2, percentage = ?3,
                 available_questions = ?4, attempted_questions = ?5, correct_questions = ?6,
                 incorrect_questions = ?7, unanswered_questions = ?8, status = 'completed',
                 ended_at = ?9, updated_at = ?9, current_question_id = NULL,
                 last_activity_at = ?9, submission_kind = ?10
             WHERE id = ?11 AND exam_id = ?12 AND status = 'in_progress'",
            params![
                total_max,
                total_marks,
                percentage,
                correct_total + incorrect_total + unanswered_total,
                correct_total + incorrect_total,
                correct_total,
                incorrect_total,
                unanswered_total,
                now_text,
                submission_kind,
                mock_id,
                exam_id,
            ],
        )
        .map_err(|error| AppError::database("Mock submission totals could not be saved", error))?;
    let sections_json = {
        let mut statement = transaction
            .prepare(
                "SELECT json_group_array(json_object(
                    'sectionId', section_id,
                    'patternSectionId', pattern_section_id,
                    'questionCount', question_count,
                    'correct', correct,
                    'incorrect', incorrect,
                    'unanswered', unanswered,
                    'maxMarks', max_marks,
                    'marksObtained', marks_obtained,
                    'timeSpentMinutes', time_spent_minutes,
                    'sectionSnapshot', json(section_snapshot_json)
                )) FROM mock_test_sections WHERE mock_test_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Mock section summary could not be prepared", error)
            })?;
        statement
            .query_row([mock_id], |row| row.get::<_, String>(0))
            .map_err(|error| AppError::database("Mock section summary could not be read", error))?
    };
    let marking_context: String = transaction
        .query_row(
            "SELECT scheme_snapshot_json FROM mock_tests WHERE id = ?1",
            [mock_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Mock scheme snapshot could not be read", error))?;
    let latest: Option<i64> = transaction
        .query_row(
            "SELECT MAX(version) FROM mock_test_score_history WHERE mock_test_id = ?1",
            [mock_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Mock score history could not be read", error))?
        .flatten();
    let version = latest.map_or(1, |version| version + 1);
    let history_sections = json!({
        "mockKind": MOCK_KIND_QUESTION_LEVEL,
        "sections": serde_json::from_str::<Value>(&sections_json).unwrap_or(Value::Array(Vec::new())),
        "questions": scoring_json,
    });
    transaction
        .execute(
            "INSERT INTO mock_test_score_history(
                exam_id, mock_test_id, version, max_marks, marks_obtained, sections_json,
                marking_context_json, recorded_at, reason
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                exam_id,
                mock_id,
                version,
                total_max,
                total_marks,
                history_sections.to_string(),
                marking_context,
                now_text,
                if automatic {
                    "Automatic question-level submission"
                } else {
                    "Question-level submission"
                },
            ],
        )
        .map_err(|error| AppError::database("Mock score history could not be stored", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Mock submission could not be committed", error))?;
    Ok(true)
}

#[derive(Debug, Clone)]
struct ScoringItem {
    id: i64,
    section_id: i64,
    section_code: String,
    question_type_code: String,
    max_marks: f64,
    correct_answer: String,
    correct_range: String,
    numeric_tolerance: f64,
    selected_answer: String,
    marked_for_review: bool,
    time_spent_seconds: i64,
    display_order: i64,
    first_answered_at: Option<String>,
    section_max_attempts: Option<i64>,
}

fn maybe_auto_submit(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
    now: DateTime<Utc>,
) -> Result<bool, AppError> {
    let row: Option<(String, Option<String>)> = connection
        .query_row(
            "SELECT status, deadline_at FROM mock_tests
             WHERE id = ?1 AND exam_id = ?2 AND mock_kind = 'question_level'",
            params![mock_id, exam_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Mock deadline could not be read", error))?;
    let Some((status, deadline)) = row else {
        return Err(AppError::mock_test_not_found());
    };
    if status == "in_progress"
        && deadline
            .as_deref()
            .and_then(parse_deadline)
            .is_some_and(|deadline| deadline <= now)
    {
        submit_internal(connection, exam_id, mock_id, now, true)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn get_session(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
) -> Result<V2QuestionMockSession, AppError> {
    let now = Utc::now();
    maybe_auto_submit(connection, exam_id, mock_id, now)?;
    build_session(connection, exam_id, mock_id, now)
}

pub fn submit(
    connection: &mut Connection,
    input: V2SubmitQuestionMockInput,
) -> Result<V2QuestionMockSession, AppError> {
    if !input.automatic && !input.confirmation {
        return Err(AppError::validation(
            "confirmation",
            "Confirm the submission before closing the mock.",
        ));
    }
    let now = Utc::now();
    submit_internal(
        connection,
        input.exam_id,
        input.mock_id,
        now,
        input.automatic,
    )?;
    build_session(connection, input.exam_id, input.mock_id, now)
}

pub fn expire_due(connection: &mut Connection, now: DateTime<Utc>) -> Result<(), AppError> {
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let due = {
        let mut statement = connection
            .prepare(
                "SELECT exam_id, id FROM mock_tests
                 WHERE mock_kind = 'question_level' AND status = 'in_progress'
                   AND deadline_at IS NOT NULL AND deadline_at <= ?1",
            )
            .map_err(|error| {
                AppError::database("Due question-level mocks could not be prepared", error)
            })?;
        statement
            .query_map([now_text], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| {
                AppError::database("Due question-level mocks could not be read", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A due question-level mock is invalid", error))?
    };
    for (exam_id, mock_id) in due {
        if let Err(error) = submit_internal(connection, exam_id, mock_id, now, true) {
            eprintln!("Automatic question-level mock submission failed for {mock_id}: {error:?}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines, clippy::type_complexity)]
fn build_session(
    connection: &Connection,
    exam_id: i64,
    mock_id: i64,
    now: DateTime<Utc>,
) -> Result<V2QuestionMockSession, AppError> {
    let header: Option<(
        i64,
        Option<i64>,
        Option<String>,
        Option<String>,
        String,
        String,
        i64,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        f64,
        Option<f64>,
        Option<f64>,
        i64,
        i64,
        i64,
        i64,
        i64,
        Option<i64>,
        Option<String>,
        String,
        String,
        String,
        i64,
        i64,
    )> = connection
        .query_row(
            "SELECT m.id, m.pattern_version_id, pv.version_label, p.title, m.test_name,
                    m.test_date, m.duration_minutes, m.test_mode, m.status, m.mock_kind,
                    m.started_at, m.deadline_at, m.ended_at, m.submission_kind, m.max_marks,
                    m.marks_obtained, m.percentage, m.available_questions,
                    m.attempted_questions, m.correct_questions, m.incorrect_questions,
                    m.unanswered_questions, m.current_question_id, m.last_activity_at,
                    m.scheme_snapshot_json, m.notes, m.review_observations,
                    COALESCE(m.is_archived,0), m.exam_id
             FROM mock_tests m
             LEFT JOIN exam_pattern_versions pv ON pv.id = m.pattern_version_id
             LEFT JOIN exam_patterns p ON p.id = pv.pattern_id
             WHERE m.id = ?1 AND m.exam_id = ?2 AND m.mock_kind = 'question_level'",
            params![mock_id, exam_id],
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
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                    row.get(15)?,
                    row.get(16)?,
                    row.get(17)?,
                    row.get(18)?,
                    row.get(19)?,
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                    row.get(23)?,
                    row.get(24)?,
                    row.get(25)?,
                    row.get(26)?,
                    row.get(27)?,
                    row.get(28)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            AppError::database("Question-level mock session could not be read", error)
        })?;
    let Some((
        id,
        pattern_version_id,
        pattern_version_label,
        pattern_title,
        test_name,
        test_date,
        duration_minutes,
        test_mode,
        status,
        mock_kind,
        started_at,
        deadline_at,
        ended_at,
        submission_kind,
        max_marks,
        marks_obtained,
        percentage,
        available_questions,
        attempted_questions,
        correct_questions,
        incorrect_questions,
        unanswered_questions,
        current_question_id,
        last_activity_at,
        scheme_snapshot_json,
        notes,
        review_observations,
        is_archived,
        _header_exam_id,
    )) = header
    else {
        return Err(AppError::mock_test_not_found());
    };
    let questions = load_questions(connection, mock_id, status == "completed")?;
    let section_meta: HashMap<i64, (Option<i64>, f64, f64, f64, i64, Option<i64>)> = {
        let mut statement = connection
            .prepare(
                "SELECT ms.section_id, ms.pattern_section_id, ms.correct_marks,
                        ms.incorrect_marks, ms.unanswered_marks, ms.id, ps.max_attempts
                 FROM mock_test_sections ms
                 LEFT JOIN exam_pattern_sections ps ON ps.id = ms.pattern_section_id
                 WHERE ms.mock_test_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Mock section metadata could not be prepared", error)
            })?;
        statement
            .query_map([mock_id], |row| {
                Ok((
                    row.get(0)?,
                    (
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ),
                ))
            })
            .map_err(|error| AppError::database("Mock section metadata could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A mock section metadata row is invalid", error))?
            .into_iter()
            .collect()
    };
    let mut section_accumulators: BTreeMap<i64, SectionAccumulator> = BTreeMap::new();
    for question in &questions {
        let section = section_accumulators
            .entry(question.section_id)
            .or_insert_with(|| SectionAccumulator {
                section_id: question.section_id,
                section_code: question.section_code.clone(),
                section_title: question.section_title.clone(),
                section_order: question.section_order,
                question_count: 0,
                visited: 0,
                answered: 0,
                marked: 0,
                correct: 0,
                incorrect: 0,
                unanswered: 0,
                max_marks: 0.0,
                marks_obtained: 0.0,
                time_spent_seconds: 0,
            });
        section.question_count += 1;
        section.max_marks += question.max_marks;
        section.marks_obtained += question.marks_obtained;
        section.time_spent_seconds += question.time_spent_seconds;
        section.visited += i64::from(question.visited);
        section.answered += i64::from(!question.selected_answer.trim().is_empty());
        section.marked += i64::from(question.marked_for_review);
        match question.response_status.as_str() {
            "correct" if status == "completed" => section.correct += 1,
            "incorrect" if status == "completed" => section.incorrect += 1,
            _ if status == "completed" => section.unanswered += 1,
            _ => {}
        }
    }
    if status != "completed" {
        for section in section_accumulators.values_mut() {
            section.unanswered = section.question_count;
            section.correct = 0;
            section.incorrect = 0;
        }
    }
    let mut sections = section_accumulators
        .into_values()
        .map(|section| {
            let attempted = section.correct + section.incorrect;
            let (
                pattern_section_id,
                correct_marks,
                incorrect_marks,
                unanswered_marks,
                row_id,
                max_attempts,
            ) = section_meta.get(&section.section_id).copied().unwrap_or((
                None,
                0.0,
                0.0,
                0.0,
                section.section_id,
                None,
            ));
            V2QuestionMockSection {
                id: row_id,
                section_id: section.section_id,
                pattern_section_id,
                section_code: section.section_code,
                section_title: section.section_title,
                display_order: section.section_order,
                max_attempts,
                question_count: section.question_count,
                answered: section.answered,
                marked: section.marked,
                not_visited: section.question_count - section.visited,
                correct: section.correct,
                incorrect: section.incorrect,
                unanswered: section.unanswered,
                max_marks: section.max_marks,
                marks_obtained: section.marks_obtained,
                time_spent_seconds: section.time_spent_seconds,
                average_time_seconds: if section.question_count > 0 {
                    Some(round3(
                        section.time_spent_seconds as f64 / section.question_count as f64,
                    ))
                } else {
                    None
                },
                accuracy_percentage: if attempted > 0 {
                    Some(round3(section.correct as f64 * 100.0 / attempted as f64))
                } else {
                    None
                },
                correct_marks,
                incorrect_marks,
                unanswered_marks,
            }
        })
        .collect::<Vec<_>>();
    sections.sort_by_key(|section| section.display_order);
    let topics = if status == "completed" {
        topic_results(&questions)
    } else {
        Vec::new()
    };
    let compatible_attempts = if status == "completed" {
        compatible_attempts(connection, exam_id, pattern_version_id, mock_id)?
    } else {
        Vec::new()
    };
    let total_time_seconds = questions
        .iter()
        .map(|question| question.time_spent_seconds)
        .sum::<i64>();
    let average_time_seconds = if questions.is_empty() {
        None
    } else {
        Some(round3(total_time_seconds as f64 / questions.len() as f64))
    };
    let is_in_progress = status == "in_progress";
    Ok(V2QuestionMockSession {
        mock_id: id,
        exam_id,
        pattern_version_id,
        pattern_version_label,
        pattern_title,
        test_name,
        test_date,
        duration_minutes,
        test_mode,
        status,
        mock_kind,
        started_at,
        deadline_at: deadline_at.clone(),
        ended_at,
        server_now: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        remaining_seconds: if is_in_progress {
            remaining_seconds(deadline_at.as_deref(), now)
        } else {
            None
        },
        current_question_id,
        last_activity_at,
        submission_kind,
        max_marks,
        marks_obtained,
        percentage,
        available_questions,
        attempted_questions,
        correct_questions,
        incorrect_questions,
        unanswered_questions,
        total_time_seconds,
        average_time_seconds,
        questions,
        sections,
        topics,
        compatible_attempts,
        scheme_snapshot_json,
        notes,
        review_observations,
        is_archived: is_archived == 1,
    })
}

fn load_questions(
    connection: &Connection,
    mock_id: i64,
    reveal_answers: bool,
) -> Result<Vec<V2QuestionMockQuestion>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, question_id, display_order, section_id, pattern_section_id,
                    section_code, section_title, section_order, question_type_code,
                    question_number, topic_id, topic_title, question_snapshot_json,
                    selected_answer, visited, marked_for_review, response_status,
                    marks_obtained, max_marks, negative_marks, unanswered_marks,
                    numeric_tolerance, time_spent_seconds, correct_answer_at_attempt,
                    correct_range_at_attempt, answer_key_version, not_evaluated
             FROM mock_test_questions WHERE mock_test_id = ?1 ORDER BY display_order",
        )
        .map_err(|error| AppError::database("Mock questions could not be prepared", error))?;
    let rows = statement
        .query_map([mock_id], |row| {
            let snapshot_json: String = row.get(12)?;
            let snapshot: QuestionSnapshot =
                serde_json::from_str(&snapshot_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        12,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(V2QuestionMockQuestion {
                id: row.get(0)?,
                question_id: row.get(1)?,
                display_order: row.get(2)?,
                section_id: row.get(3)?,
                pattern_section_id: row.get(4)?,
                section_code: row.get(5)?,
                section_title: row.get(6)?,
                section_order: row.get(7)?,
                question_type_code: row.get(8)?,
                question_number: row.get(9)?,
                topic_id: row.get(10)?,
                topic_title: row.get(11)?,
                statement: snapshot.statement,
                options: snapshot.options,
                selected_answer: row.get(13)?,
                visited: row.get::<_, i64>(14)? == 1,
                marked_for_review: row.get::<_, i64>(15)? == 1,
                not_evaluated: row.get::<_, i64>(26)? == 1,
                response_status: row.get(16)?,
                marks_obtained: row.get(17)?,
                max_marks: row.get(18)?,
                negative_marks: row.get(19)?,
                unanswered_marks: row.get(20)?,
                numeric_tolerance: row.get(21)?,
                time_spent_seconds: row.get(22)?,
                correct_answer: if reveal_answers {
                    Some(row.get(23)?)
                } else {
                    None
                },
                correct_range: if reveal_answers {
                    Some(row.get(24)?)
                } else {
                    None
                },
                answer_key_version: row.get(25)?,
                official_explanation: if reveal_answers {
                    snapshot.official_explanation
                } else {
                    String::new()
                },
                third_party_solution: if reveal_answers {
                    snapshot.third_party_solution
                } else {
                    String::new()
                },
                user_note: if reveal_answers {
                    snapshot.user_note
                } else {
                    String::new()
                },
                source_title: snapshot.source_title,
                source_page: snapshot.source_page,
                source_url: snapshot.source_url,
                difficulty: snapshot.difficulty,
                year: snapshot.year,
                session_label: snapshot.session_label,
                concepts: snapshot.concepts,
            })
        })
        .map_err(|error| AppError::database("Mock questions could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A mock question is invalid", error))?;
    Ok(rows)
}

fn topic_results(questions: &[V2QuestionMockQuestion]) -> Vec<V2QuestionMockTopic> {
    let mut topics: BTreeMap<i64, (String, i64, i64, i64, i64, i64)> = BTreeMap::new();
    for question in questions {
        let Some(topic_id) = question.topic_id else {
            continue;
        };
        let entry = topics
            .entry(topic_id)
            .or_insert_with(|| (question.topic_title.clone(), 0, 0, 0, 0, 0));
        entry.1 += 1;
        match question.response_status.as_str() {
            "correct" => entry.2 += 1,
            "incorrect" => entry.3 += 1,
            _ => entry.4 += 1,
        }
        entry.5 += question.time_spent_seconds;
    }
    topics
        .into_iter()
        .map(
            |(topic_id, (title, questions, correct, incorrect, unanswered, time))| {
                let attempted = correct + incorrect;
                V2QuestionMockTopic {
                    topic_id,
                    topic_title: title,
                    questions,
                    correct,
                    incorrect,
                    unanswered,
                    accuracy_percentage: if attempted > 0 {
                        Some(round3(correct as f64 * 100.0 / attempted as f64))
                    } else {
                        None
                    },
                    average_time_seconds: if questions > 0 {
                        Some(round3(time as f64 / questions as f64))
                    } else {
                        None
                    },
                }
            },
        )
        .collect()
}

fn compatible_attempts(
    connection: &Connection,
    exam_id: i64,
    pattern_version_id: Option<i64>,
    current_id: i64,
) -> Result<Vec<V2QuestionMockCompatibleAttempt>, AppError> {
    let Some(pattern_version_id) = pattern_version_id else {
        return Ok(Vec::new());
    };
    let mut statement = connection
        .prepare(
            "SELECT id, test_date, test_name, marks_obtained, max_marks,
                    CASE WHEN max_marks > 0 AND marks_obtained IS NOT NULL
                         THEN marks_obtained * 100.0 / max_marks ELSE NULL END
             FROM mock_tests
             WHERE exam_id = ?1 AND pattern_version_id = ?2
               AND mock_kind = 'question_level' AND status = 'completed' AND is_archived = 0
             ORDER BY test_date, id",
        )
        .map_err(|error| {
            AppError::database("Compatible mock attempts could not be prepared", error)
        })?;
    statement
        .query_map(params![exam_id, pattern_version_id], |row| {
            Ok(V2QuestionMockCompatibleAttempt {
                mock_id: row.get(0)?,
                test_date: row.get(1)?,
                test_name: row.get(2)?,
                marks_obtained: row.get(3)?,
                max_marks: row.get(4)?,
                percentage: row.get(5)?,
                current: row.get::<_, i64>(0)? == current_id,
            })
        })
        .map_err(|error| AppError::database("Compatible mock attempts could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A compatible mock attempt is invalid", error))
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use rusqlite::params;
    use tempfile::TempDir;

    use super::*;
    use crate::v2::V2State;

    fn count(connection: &Connection, sql: &str) -> i64 {
        connection
            .query_row(sql, [], |row| row.get(0))
            .expect("count query")
    }

    fn counts(connection: &Connection) -> Vec<i64> {
        vec![
            count(connection, "SELECT COUNT(*) FROM exam_syllabus_nodes"),
            count(connection, "SELECT COUNT(*) FROM concepts"),
            count(connection, "SELECT COUNT(*) FROM exam_node_concepts"),
            count(connection, "SELECT COUNT(*) FROM exam_patterns"),
            count(connection, "SELECT COUNT(*) FROM exam_pattern_versions"),
            count(connection, "SELECT COUNT(*) FROM exam_pattern_sections"),
            count(connection, "SELECT COUNT(*) FROM mock_test_questions"),
        ]
    }

    fn state() -> (TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    #[allow(clippy::too_many_arguments)]
    fn make_question(
        state: &V2State,
        exam_id: i64,
        section_code: &str,
        type_code: &str,
        question_number: &str,
        correct_answer: &str,
        options: &[(&str, &str)],
        max_marks: f64,
        negative_marks: f64,
        tolerance: f64,
    ) -> super::super::pyq::V2PyqQuestionDetail {
        state
            .with_connection(|connection| {
                let practice =
                    super::super::pyq::options(connection, V2PracticeExamInput { exam_id })?;
                let version = practice
                    .patterns
                    .iter()
                    .find(|pattern| pattern.verification_status == "verified")
                    .expect("verified pattern");
                let section = version
                    .sections
                    .iter()
                    .find(|section| section.code == section_code)
                    .expect("pattern section");
                let question_type = practice
                    .question_types
                    .iter()
                    .find(|question_type| question_type.code == type_code)
                    .expect("question type");
                let topic = practice
                    .topics
                    .iter()
                    .find(|topic| topic.node_kind == "topic");
                super::super::pyq::save_question(
                    connection,
                    super::super::pyq::V2PyqQuestionInput {
                        id: None,
                        exam_id,
                        year: 2099,
                        session_label: "M11 synthetic".to_owned(),
                        section_id: section.section_id,
                        question_type_id: question_type.id,
                        question_number: question_number.to_owned(),
                        statement: format!("Synthetic M11 question {question_number}."),
                        correct_answer: correct_answer.to_owned(),
                        correct_range: String::new(),
                        max_marks,
                        negative_marks,
                        unanswered_marks: 0.0,
                        difficulty: 3,
                        source_title: "Synthetic M11 source".to_owned(),
                        source_page: "1".to_owned(),
                        source_url: None,
                        answer_key_source: "Synthetic M11 key".to_owned(),
                        answer_key_version: "m11-1".to_owned(),
                        answer_key_verified: true,
                        verification_status: "verified".to_owned(),
                        numeric_tolerance: tolerance,
                        options: options
                            .iter()
                            .map(|(key, text)| V2QuestionOption {
                                key: (*key).to_owned(),
                                text: (*text).to_owned(),
                            })
                            .collect(),
                        pattern_version_id: Some(version.id),
                        syllabus_node_id: topic.map(|topic| topic.id),
                        topic_ids: topic.map(|topic| vec![topic.id]).unwrap_or_default(),
                        concept_ids: Vec::new(),
                        resource_id: None,
                        source_id: None,
                        answer_key_id: None,
                        dedupe_key: String::new(),
                        official_explanation: "Synthetic official explanation.".to_owned(),
                        third_party_solution: "Synthetic third-party solution.".to_owned(),
                        user_note: "Synthetic user note.".to_owned(),
                    },
                )
            })
            .expect("question")
    }

    fn start_mock(
        state: &V2State,
        exam_id: i64,
        pattern_version_id: i64,
        question_ids: Vec<i64>,
        mode: &str,
        duration_minutes: Option<i64>,
    ) -> V2QuestionMockSession {
        state
            .with_connection(|connection| {
                super::start(
                    connection,
                    V2StartQuestionMockInput {
                        exam_id,
                        pattern_version_id,
                        test_name: "Synthetic M11 mock".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        test_mode: mode.to_owned(),
                        duration_minutes,
                        focus_node_id: None,
                        notes: String::new(),
                        question_ids,
                    },
                )
            })
            .expect("start mock")
    }

    fn save_answer(
        state: &V2State,
        exam_id: i64,
        mock_id: i64,
        mock_question_id: i64,
        selected_answer: &str,
    ) -> V2QuestionMockSaveResult {
        state
            .with_connection(|connection| {
                super::save_state(
                    connection,
                    V2SaveQuestionMockInput {
                        exam_id,
                        mock_id,
                        mock_question_id: Some(mock_question_id),
                        selected_answer: Some(selected_answer.to_owned()),
                        marked_for_review: None,
                        current_question_id: Some(mock_question_id),
                    },
                )
            })
            .expect("save answer")
    }

    fn submit_mock(
        state: &V2State,
        exam_id: i64,
        mock_id: i64,
        automatic: bool,
    ) -> V2QuestionMockSession {
        state
            .with_connection(|connection| {
                super::submit(
                    connection,
                    V2SubmitQuestionMockInput {
                        exam_id,
                        mock_id,
                        confirmation: true,
                        automatic,
                    },
                )
            })
            .expect("submit mock")
    }

    #[test]
    fn question_level_mock_scores_mcq_msq_and_numeric_questions() {
        let (_root, state) = state();
        let mcq = make_question(
            &state,
            2,
            "gate-ga",
            "MCQ",
            "G1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            1.0,
            -1.0 / 3.0,
            0.0,
        );
        let msq = make_question(
            &state,
            2,
            "gate-ph",
            "MSQ",
            "G2",
            "A,C",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            2.0,
            0.0,
            0.0,
        );
        let nat = make_question(
            &state,
            2,
            "gate-ph",
            "NAT",
            "G3",
            "1.0",
            &[],
            2.0,
            0.0,
            0.01,
        );
        let pattern_version_id = mcq.question.pattern_version_id.expect("pattern");
        let session = start_mock(
            &state,
            2,
            pattern_version_id,
            vec![mcq.question.id, msq.question.id, nat.question.id],
            "custom",
            Some(60),
        );
        assert_eq!(session.status, "in_progress");
        assert_eq!(session.questions.len(), 3);
        assert!(
            session
                .questions
                .iter()
                .all(|question| question.correct_answer.is_none())
        );
        let mcq_item = session
            .questions
            .iter()
            .find(|question| question.question_id == mcq.question.id)
            .expect("mcq item");
        let msq_item = session
            .questions
            .iter()
            .find(|question| question.question_id == msq.question.id)
            .expect("msq item");
        let nat_item = session
            .questions
            .iter()
            .find(|question| question.question_id == nat.question.id)
            .expect("nat item");
        save_answer(&state, 2, session.mock_id, mcq_item.id, "B");
        save_answer(&state, 2, session.mock_id, msq_item.id, "A,C");
        save_answer(&state, 2, session.mock_id, nat_item.id, "1.005");
        state
            .with_connection(|connection| {
                super::save_state(
                    connection,
                    V2SaveQuestionMockInput {
                        exam_id: 2,
                        mock_id: session.mock_id,
                        mock_question_id: Some(msq_item.id),
                        selected_answer: None,
                        marked_for_review: Some(true),
                        current_question_id: Some(msq_item.id),
                    },
                )
            })
            .expect("mark for review");
        let persisted = state
            .with_connection(|connection| super::get_session(connection, 2, session.mock_id))
            .expect("resume");
        assert_eq!(persisted.status, "in_progress");
        assert_eq!(persisted.current_question_id, Some(msq_item.id));
        assert!(
            persisted
                .questions
                .iter()
                .find(|question| question.id == msq_item.id)
                .expect("msq")
                .marked_for_review
        );
        assert_eq!(
            persisted
                .questions
                .iter()
                .find(|question| question.id == nat_item.id)
                .expect("nat")
                .selected_answer,
            "1.005"
        );
        assert!(persisted.questions.iter().all(|question| question.visited));
        let completed = submit_mock(&state, 2, session.mock_id, false);
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.marks_obtained, Some(5.0));
        assert_eq!(completed.max_marks, 5.0);
        assert_eq!(completed.percentage, Some(100.0));
        assert_eq!(completed.correct_questions, 3);
        assert_eq!(completed.incorrect_questions, 0);
        assert_eq!(completed.unanswered_questions, 0);
        assert_eq!(completed.sections.len(), 2);
        assert!(!completed.topics.is_empty());
        assert_eq!(completed.compatible_attempts.len(), 1);
        assert!(completed.compatible_attempts[0].current);
        let reviewed = completed
            .questions
            .iter()
            .find(|question| question.question_id == msq.question.id)
            .expect("reviewed msq");
        assert_eq!(reviewed.correct_answer.as_deref(), Some("A,C"));
        assert_eq!(
            reviewed.official_explanation,
            "Synthetic official explanation."
        );
        assert_eq!(
            reviewed.third_party_solution,
            "Synthetic third-party solution."
        );
        assert_eq!(reviewed.user_note, "Synthetic user note.");
        let again = submit_mock(&state, 2, session.mock_id, false);
        assert_eq!(again.mock_id, completed.mock_id);
        let history_count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM mock_test_score_history WHERE mock_test_id = ?1",
                        [session.mock_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("history", error))
            })
            .expect("history count");
        assert_eq!(history_count, 1);
    }

    #[test]
    fn question_level_mock_applies_csir_negative_marks_and_unanswered_state() {
        let (_root, state) = state();
        let first = make_question(
            &state,
            1,
            "csir-705-part-a",
            "MCQ",
            "C1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            2.0,
            -0.5,
            0.0,
        );
        let second = make_question(
            &state,
            1,
            "csir-705-part-a",
            "MCQ",
            "C2",
            "C",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            2.0,
            -0.5,
            0.0,
        );
        let pattern_version_id = first.question.pattern_version_id.expect("pattern");
        let incomplete = state
            .with_connection(|connection| {
                super::start(
                    connection,
                    V2StartQuestionMockInput {
                        exam_id: 1,
                        pattern_version_id,
                        test_name: "Incomplete".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        test_mode: "full_length".to_owned(),
                        duration_minutes: Some(180),
                        focus_node_id: None,
                        notes: String::new(),
                        question_ids: vec![first.question.id],
                    },
                )
            })
            .expect_err("incomplete full length must fail");
        assert_eq!(incomplete.code, "validation_error");
        let session = start_mock(
            &state,
            1,
            pattern_version_id,
            vec![first.question.id, second.question.id],
            "custom",
            Some(30),
        );
        let first_item = session
            .questions
            .iter()
            .find(|question| question.question_id == first.question.id)
            .expect("first");
        save_answer(&state, 1, session.mock_id, first_item.id, "A");
        let completed = submit_mock(&state, 1, session.mock_id, false);
        assert_eq!(completed.status, "completed");
        assert_eq!(completed.marks_obtained, Some(-0.5));
        assert_eq!(completed.max_marks, 4.0);
        assert_eq!(completed.correct_questions, 0);
        assert_eq!(completed.incorrect_questions, 1);
        assert_eq!(completed.unanswered_questions, 1);
        assert_eq!(completed.percentage, Some(-12.5));
        assert_eq!(completed.sections[0].correct, 0);
        assert_eq!(completed.sections[0].incorrect, 1);
        assert_eq!(completed.sections[0].unanswered, 1);
    }

    #[test]
    fn question_level_mock_uses_tifr_and_jest_section_rules() {
        let (_root, state) = state();
        let tifr = make_question(
            &state,
            3,
            "tifr-gs-section-b",
            "MCQ",
            "T1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            5.0,
            0.0,
            0.0,
        );
        let tifr_pattern = tifr.question.pattern_version_id.expect("pattern");
        let tifr_session = start_mock(
            &state,
            3,
            tifr_pattern,
            vec![tifr.question.id],
            "custom",
            Some(60),
        );
        let tifr_item = &tifr_session.questions[0];
        save_answer(&state, 3, tifr_session.mock_id, tifr_item.id, "A");
        let tifr_done = submit_mock(&state, 3, tifr_session.mock_id, false);
        assert_eq!(tifr_done.marks_obtained, Some(0.0));
        assert_eq!(tifr_done.questions[0].correct_answer.as_deref(), Some("B"));

        let jest = make_question(
            &state,
            4,
            "jest-part-a",
            "MCQ",
            "J1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            1.0,
            -1.0 / 3.0,
            0.0,
        );
        let jest_pattern = jest.question.pattern_version_id.expect("pattern");
        let jest_session = start_mock(
            &state,
            4,
            jest_pattern,
            vec![jest.question.id],
            "custom",
            Some(60),
        );
        let jest_item = &jest_session.questions[0];
        save_answer(&state, 4, jest_session.mock_id, jest_item.id, "A");
        let jest_done = submit_mock(&state, 4, jest_session.mock_id, false);
        assert!((jest_done.marks_obtained.expect("marks") + 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn question_level_mock_recovers_after_restart_and_auto_submits_at_deadline() {
        let root = tempfile::tempdir().expect("temporary directory");
        let first_state = V2State::initialize(&root.path().join("v2"));
        let question = make_question(
            &first_state,
            1,
            "csir-705-part-a",
            "MCQ",
            "R1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            2.0,
            -0.5,
            0.0,
        );
        let pattern_version_id = question.question.pattern_version_id.expect("pattern");
        let session = start_mock(
            &first_state,
            1,
            pattern_version_id,
            vec![question.question.id],
            "custom",
            Some(60),
        );
        let item = &session.questions[0];
        save_answer(&first_state, 1, session.mock_id, item.id, "B");
        drop(first_state);
        let reopened = V2State::initialize(&root.path().join("v2"));
        let resumed = reopened
            .with_connection(|connection| super::get_session(connection, 1, session.mock_id))
            .expect("resume after restart");
        assert_eq!(resumed.status, "in_progress");
        assert_eq!(resumed.questions[0].selected_answer, "B");
        reopened
            .with_connection(|connection| {
                let past = (Utc::now() - Duration::minutes(1))
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                connection
                    .execute(
                        "UPDATE mock_tests SET deadline_at = ?1 WHERE id = ?2",
                        params![past, session.mock_id],
                    )
                    .map_err(|error| AppError::database("deadline", error))?;
                Ok(())
            })
            .expect("expire");
        let expired = reopened
            .with_connection(|connection| super::get_session(connection, 1, session.mock_id))
            .expect("auto submit");
        assert_eq!(expired.status, "completed");
        assert_eq!(expired.submission_kind, "automatic");
        assert_eq!(expired.marks_obtained, Some(2.0));
        let history_count: i64 = reopened
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM mock_test_score_history WHERE mock_test_id = ?1",
                        [session.mock_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("history", error))
            })
            .expect("history");
        assert_eq!(history_count, 1);
    }

    #[test]
    fn question_level_mock_preserves_start_snapshots_and_enforces_pending_and_inactive_rules() {
        let (_root, state) = state();
        let question = make_question(
            &state,
            1,
            "csir-705-part-a",
            "MCQ",
            "S1",
            "B",
            &[
                ("A", "Alpha"),
                ("B", "Beta"),
                ("C", "Gamma"),
                ("D", "Delta"),
            ],
            2.0,
            -0.5,
            0.0,
        );
        let pattern_version_id = question.question.pattern_version_id.expect("pattern");
        let session = start_mock(
            &state,
            1,
            pattern_version_id,
            vec![question.question.id],
            "custom",
            Some(30),
        );
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE pyq_questions SET correct_answer = 'A' WHERE id = ?1",
                        [question.question.id],
                    )
                    .map_err(|error| AppError::database("answer key", error))?;
                connection
                    .execute(
                        "UPDATE exam_pattern_sections SET correct_marks = 9 WHERE id = (SELECT pattern_section_id FROM mock_test_questions WHERE mock_test_id = ?1)",
                        [session.mock_id],
                    )
                    .map_err(|error| AppError::database("pattern marks", error))?;
                Ok(())
            })
            .expect("edit source");
        let item = &session.questions[0];
        save_answer(&state, 1, session.mock_id, item.id, "B");
        let completed = submit_mock(&state, 1, session.mock_id, false);
        assert_eq!(completed.marks_obtained, Some(2.0));
        assert_eq!(completed.questions[0].max_marks, 2.0);
        assert_eq!(completed.questions[0].correct_answer.as_deref(), Some("B"));

        let pending_id = state
            .with_connection(|connection| {
                let options =
                    super::super::pyq::options(connection, V2PracticeExamInput { exam_id: 3 })?;
                Ok(options
                    .patterns
                    .iter()
                    .find(|pattern| pattern.verification_status == "pending")
                    .map(|pattern| pattern.id)
                    .expect("pending pattern"))
            })
            .expect("pending id");
        let pending_error = state
            .with_connection(|connection| {
                super::start(
                    connection,
                    V2StartQuestionMockInput {
                        exam_id: 3,
                        pattern_version_id: pending_id,
                        test_name: "Pending".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        test_mode: "custom".to_owned(),
                        duration_minutes: Some(60),
                        focus_node_id: None,
                        notes: String::new(),
                        question_ids: vec![question.question.id],
                    },
                )
            })
            .expect_err("pending pattern must fail");
        assert_eq!(pending_error.code, "validation_error");

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_profiles SET is_active = 0 WHERE exam_id = 1",
                        [],
                    )
                    .map_err(|error| AppError::database("deactivate", error))
            })
            .expect("deactivate");
        let read_after_deactivation = state
            .with_connection(|connection| super::get_session(connection, 1, session.mock_id))
            .expect("history read");
        assert_eq!(read_after_deactivation.status, "completed");
        let inactive_error = state
            .with_connection(|connection| {
                super::start(
                    connection,
                    V2StartQuestionMockInput {
                        exam_id: 1,
                        pattern_version_id,
                        test_name: "Inactive".to_owned(),
                        test_date: "2026-11-05".to_owned(),
                        test_mode: "custom".to_owned(),
                        duration_minutes: Some(30),
                        focus_node_id: None,
                        notes: String::new(),
                        question_ids: vec![question.question.id],
                    },
                )
            })
            .expect_err("inactive exam must fail");
        assert_eq!(inactive_error.code, "forbidden");
    }

    #[test]
    fn question_level_mock_enforces_csir_first_n_attempt_limit() {
        let (_root, state) = state();
        let mut question_ids = Vec::new();
        let mut pattern_version_id = None;
        for index in 1..=16 {
            let question = make_question(
                &state,
                1,
                "csir-705-part-a",
                "MCQ",
                &index.to_string(),
                "B",
                &[
                    ("A", "Alpha"),
                    ("B", "Beta"),
                    ("C", "Gamma"),
                    ("D", "Delta"),
                ],
                2.0,
                -0.5,
                0.0,
            );
            pattern_version_id = question.question.pattern_version_id;
            question_ids.push(question.question.id);
        }
        let session = start_mock(
            &state,
            1,
            pattern_version_id.expect("pattern"),
            question_ids,
            "custom",
            Some(60),
        );
        assert_eq!(session.questions.len(), 16);
        for question in &session.questions {
            save_answer(&state, 1, session.mock_id, question.id, "B");
        }
        let completed = submit_mock(&state, 1, session.mock_id, false);
        assert_eq!(completed.correct_questions, 15);
        assert_eq!(completed.incorrect_questions, 0);
        assert_eq!(completed.unanswered_questions, 1);
        assert_eq!(completed.marks_obtained, Some(30.0));
        assert_eq!(
            completed
                .questions
                .iter()
                .filter(|question| question.not_evaluated)
                .count(),
            1
        );
        assert_eq!(completed.sections[0].max_attempts, Some(15));
        assert_eq!(completed.sections[0].correct, 15);
        assert_eq!(completed.sections[0].unanswered, 1);
    }

    #[test]
    fn question_level_schema_prepare_is_idempotent() {
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| Ok::<_, AppError>(counts(connection)))
            .expect("before");
        state
            .with_connection(super::super::prepare)
            .expect("first prepare");
        state
            .with_connection(super::super::prepare)
            .expect("second prepare");
        let after = state
            .with_connection(|connection| Ok::<_, AppError>(counts(connection)))
            .expect("after");
        assert_eq!(before, after);
        assert_eq!(after[0], 328);
        assert_eq!(after[1], 64);
        assert_eq!(after[2], 108);
        assert_eq!(after[3], 4);
        assert_eq!(after[4], 6);
        assert_eq!(after[5], 10);
    }
}
