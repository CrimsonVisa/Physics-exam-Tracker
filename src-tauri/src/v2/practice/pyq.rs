use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    csv_import::{
        ImportFieldError, ImportPreview, ImportResult, ImportRow, ImportRowStatus, csv_template,
        preview_from_rows, read_table, result_from_rows, stable_import_key, values_by_header,
    },
    error::AppError,
    v2::mastery,
};

use super::scoring::{AnswerKind, ScoredResponse, score_question};

const QUESTION_HEADERS: &[&str] = &[
    "exam_code",
    "year",
    "session_label",
    "section_code",
    "question_type_code",
    "question_number",
    "statement",
    "correct_answer",
    "correct_range",
    "max_marks",
    "negative_marks",
    "unanswered_marks",
    "difficulty",
    "source_title",
    "source_page",
    "source_url",
    "answer_key_source",
    "answer_key_version",
    "answer_key_verified",
    "verification_status",
    "numeric_tolerance",
    "syllabus_node_code",
    "concept_slugs",
    "dedupe_key",
    "official_explanation",
    "third_party_solution",
    "user_note",
];

const QUESTION_HEADERS_V2: &[&str] = &[
    "exam_code",
    "year",
    "session_label",
    "section_code",
    "question_type_code",
    "question_number",
    "statement",
    "correct_answer",
    "correct_range",
    "max_marks",
    "negative_marks",
    "unanswered_marks",
    "difficulty",
    "source_title",
    "source_page",
    "source_url",
    "answer_key_source",
    "answer_key_version",
    "answer_key_verified",
    "verification_status",
    "numeric_tolerance",
    "syllabus_node_code",
    "concept_slugs",
    "dedupe_key",
    "official_explanation",
    "third_party_solution",
    "user_note",
    "options_json",
    "pattern_version_id",
    "source_id",
    "answer_key_id",
];

fn question_headers(input: &str) -> &'static [&'static str] {
    let first_line = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if first_line.contains("options_json") {
        QUESTION_HEADERS_V2
    } else {
        QUESTION_HEADERS
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeOptions {
    pub exam: ExamOption,
    pub patterns: Vec<PatternVersionOption>,
    pub sections: Vec<SectionOption>,
    pub question_types: Vec<QuestionTypeOption>,
    pub topics: Vec<TopicOption>,
    pub concepts: Vec<ConceptOption>,
    pub resources: Vec<ResourceOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptOption {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub domain: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamOption {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub short_name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternVersionOption {
    pub id: i64,
    pub pattern_code: String,
    pub pattern_title: String,
    pub variant: String,
    pub version_label: String,
    pub verification_status: String,
    pub effective_from: Option<String>,
    pub duration_minutes: Option<i64>,
    pub total_marks: f64,
    pub total_questions: i64,
    pub attempt_limit: Option<i64>,
    pub source_title: String,
    pub source_url: Option<String>,
    pub source_reference: String,
    pub verified_on: Option<String>,
    pub notes: String,
    pub sections: Vec<PatternSectionOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternSectionOption {
    pub id: i64,
    pub section_id: i64,
    pub code: String,
    pub title: String,
    pub display_order: i64,
    pub question_count: i64,
    pub max_attempts: Option<i64>,
    pub max_marks: f64,
    pub correct_marks: f64,
    pub incorrect_marks: f64,
    pub unanswered_marks: f64,
    pub question_types: Vec<String>,
    pub marking_variants: Vec<serde_json::Value>,
    pub numeric_tolerance: f64,
    pub partial_rule: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionOption {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub default_question_type_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionTypeOption {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub answer_format: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicOption {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub node_kind: String,
    pub category: String,
    pub parent_id: Option<i64>,
    pub display_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOption {
    pub id: i64,
    pub title: String,
    pub resource_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionOption {
    pub key: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqQuestionSummary {
    pub id: i64,
    pub exam_id: i64,
    pub year: i64,
    pub session_label: String,
    pub question_number: String,
    pub section_id: i64,
    pub section_code: String,
    pub section_title: String,
    pub question_type_id: i64,
    pub question_type_code: String,
    pub pattern_version_id: Option<i64>,
    pub pattern_version_label: Option<String>,
    pub statement: String,
    pub max_marks: f64,
    pub negative_marks: f64,
    pub unanswered_marks: f64,
    pub difficulty: i64,
    pub source_title: String,
    pub source_page: String,
    pub source_url: Option<String>,
    pub answer_key_source: String,
    pub answer_key_version: String,
    pub answer_key_verified: bool,
    pub verification_status: String,
    pub numeric_tolerance: f64,
    pub correct_range: String,
    pub official_explanation: String,
    pub third_party_solution: String,
    pub user_note: String,
    pub resource_id: Option<i64>,
    pub source_id: Option<i64>,
    pub answer_key_id: Option<i64>,
    pub is_archived: bool,
    pub attempt_count: i64,
    pub concept_count: i64,
    pub syllabus_node_id: Option<i64>,
    pub syllabus_node_title: Option<String>,
    pub topic_ids: Vec<i64>,
    pub options: Vec<V2QuestionOption>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqQuestionTopic {
    pub node_id: i64,
    pub node_code: String,
    pub node_title: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqQuestionConcept {
    pub concept_id: i64,
    pub slug: String,
    pub title: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqAttempt {
    pub id: i64,
    pub exam_id: i64,
    pub question_id: i64,
    pub question_number: String,
    pub question_statement: String,
    pub attempt_date: String,
    pub selected_answer: String,
    pub status: String,
    pub marks_obtained: f64,
    pub max_marks_at_attempt: f64,
    pub negative_marks_at_attempt: f64,
    pub unanswered_marks_at_attempt: f64,
    pub time_spent_seconds: i64,
    pub confidence: Option<i64>,
    pub mistakes: String,
    pub review_status: String,
    pub notes: String,
    pub pattern_version_id: Option<i64>,
    pub answer_key_version: String,
    pub correct_answer_at_attempt: String,
    pub scoring_engine: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqQuestionDetail {
    pub question: V2PyqQuestionSummary,
    pub correct_answer: String,
    pub topics: Vec<V2PyqQuestionTopic>,
    pub concepts: Vec<V2PyqQuestionConcept>,
    pub attempts: Vec<V2PyqAttempt>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeExamInput {
    pub exam_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqFilter {
    pub exam_id: i64,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub section_id: Option<i64>,
    #[serde(default)]
    pub question_type_id: Option<i64>,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    #[serde(default)]
    pub verification_status: Option<String>,
    #[serde(default)]
    pub question_id: Option<i64>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub include_archived: Option<bool>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqQuestionInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub exam_id: i64,
    pub year: i64,
    #[serde(default)]
    pub session_label: String,
    pub section_id: i64,
    pub question_type_id: i64,
    pub question_number: String,
    #[serde(default)]
    pub statement: String,
    #[serde(default)]
    pub correct_answer: String,
    #[serde(default)]
    pub correct_range: String,
    pub max_marks: f64,
    #[serde(default)]
    pub negative_marks: f64,
    #[serde(default)]
    pub unanswered_marks: f64,
    #[serde(default = "default_difficulty")]
    pub difficulty: i64,
    #[serde(default)]
    pub source_title: String,
    #[serde(default)]
    pub source_page: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub answer_key_source: String,
    #[serde(default)]
    pub answer_key_version: String,
    #[serde(default)]
    pub answer_key_verified: bool,
    #[serde(default = "default_verification_status")]
    pub verification_status: String,
    #[serde(default)]
    pub numeric_tolerance: f64,
    #[serde(default)]
    pub options: Vec<V2QuestionOption>,
    #[serde(default)]
    pub pattern_version_id: Option<i64>,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    #[serde(default)]
    pub topic_ids: Vec<i64>,
    #[serde(default)]
    pub concept_ids: Vec<i64>,
    #[serde(default)]
    pub resource_id: Option<i64>,
    #[serde(default)]
    pub source_id: Option<i64>,
    #[serde(default)]
    pub answer_key_id: Option<i64>,
    #[serde(default)]
    pub dedupe_key: String,
    #[serde(default)]
    pub official_explanation: String,
    #[serde(default)]
    pub third_party_solution: String,
    #[serde(default)]
    pub user_note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqAttemptInput {
    pub exam_id: i64,
    pub question_id: i64,
    pub attempt_date: String,
    #[serde(default)]
    pub selected_answer: String,
    #[serde(default)]
    pub time_spent_seconds: i64,
    #[serde(default)]
    pub confidence: Option<i64>,
    #[serde(default)]
    pub mistakes: String,
    #[serde(default)]
    pub review_status: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeImportPreview {
    pub preview: ImportPreview,
    pub headers: Vec<String>,
    pub template: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeImportResult {
    pub result: ImportResult,
}

fn default_difficulty() -> i64 {
    3
}

fn default_verification_status() -> String {
    "pending".to_owned()
}

fn option_strings(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn option_values(value: &str) -> Vec<serde_json::Value> {
    serde_json::from_str::<Vec<serde_json::Value>>(value).unwrap_or_default()
}

pub fn options(
    connection: &Connection,
    input: V2PracticeExamInput,
) -> Result<V2PracticeOptions, AppError> {
    let exam = connection
        .query_row(
            "SELECT e.id, e.code, e.name, e.short_name, p.is_active
             FROM exams e JOIN exam_profiles p ON p.exam_id = e.id
             WHERE e.id = ?1",
            [input.exam_id],
            |row| {
                Ok(ExamOption {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    name: row.get(2)?,
                    short_name: row.get(3)?,
                    is_active: row.get::<_, i64>(4)? == 1,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Practice exam could not be read", error))?
        .ok_or_else(|| AppError::validation("examId", "Choose an existing examination."))?;

    let mut pattern_statement = connection
        .prepare(
            "SELECT pv.id, p.code, p.title, p.variant, pv.version_label,
                    pv.verification_status, pv.effective_from, pv.duration_minutes,
                    pv.total_marks, pv.total_questions, pv.attempt_limit,
                    pv.source_title, pv.source_url, pv.source_reference, pv.verified_on, pv.notes
             FROM exam_pattern_versions pv
             JOIN exam_patterns p ON p.id = pv.pattern_id
             WHERE pv.exam_id = ?1
             ORDER BY CASE pv.verification_status WHEN 'verified' THEN 0 ELSE 1 END,
                      pv.id",
        )
        .map_err(|error| AppError::database("Practice patterns could not be prepared", error))?;
    let pattern_rows = pattern_statement
        .query_map([input.exam_id], |row| {
            Ok(PatternVersionOption {
                id: row.get(0)?,
                pattern_code: row.get(1)?,
                pattern_title: row.get(2)?,
                variant: row.get(3)?,
                version_label: row.get(4)?,
                verification_status: row.get(5)?,
                effective_from: row.get(6)?,
                duration_minutes: row.get(7)?,
                total_marks: row.get(8)?,
                total_questions: row.get(9)?,
                attempt_limit: row.get(10)?,
                source_title: row.get(11)?,
                source_url: row.get(12)?,
                source_reference: row.get(13)?,
                verified_on: row.get(14)?,
                notes: row.get(15)?,
                sections: Vec::new(),
            })
        })
        .map_err(|error| AppError::database("Practice patterns could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice pattern is invalid", error))?;
    drop(pattern_statement);

    let mut patterns = pattern_rows;
    for pattern in &mut patterns {
        let mut section_statement = connection
            .prepare(
                "SELECT ps.id, ps.section_id, ps.code, ps.title, ps.display_order,
                        ps.question_count, ps.max_attempts, ps.max_marks, ps.correct_marks,
                        ps.incorrect_marks, ps.unanswered_marks, ps.question_types_json,
                        ps.marking_variants_json, ps.numeric_tolerance, ps.partial_rule, ps.notes
                 FROM exam_pattern_sections ps
                 WHERE ps.pattern_version_id = ?1
                 ORDER BY ps.display_order, ps.id",
            )
            .map_err(|error| AppError::database("Pattern sections could not be prepared", error))?;
        pattern.sections = section_statement
            .query_map([pattern.id], |row| {
                let question_types: String = row.get(11)?;
                let marking_variants: String = row.get(12)?;
                Ok(PatternSectionOption {
                    id: row.get(0)?,
                    section_id: row.get(1)?,
                    code: row.get(2)?,
                    title: row.get(3)?,
                    display_order: row.get(4)?,
                    question_count: row.get(5)?,
                    max_attempts: row.get(6)?,
                    max_marks: row.get(7)?,
                    correct_marks: row.get(8)?,
                    incorrect_marks: row.get(9)?,
                    unanswered_marks: row.get(10)?,
                    question_types: option_strings(&question_types),
                    marking_variants: option_values(&marking_variants),
                    numeric_tolerance: row.get(13)?,
                    partial_rule: row.get(14)?,
                    notes: row.get(15)?,
                })
            })
            .map_err(|error| AppError::database("Pattern sections could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A pattern section is invalid", error))?;
    }

    let sections = query_sections(connection, input.exam_id)?;
    let question_types = query_question_types(connection, input.exam_id)?;
    let topics = query_topics(connection, input.exam_id)?;
    let concepts = query_concepts(connection)?;
    let resources = query_resources(connection)?;

    Ok(V2PracticeOptions {
        exam,
        patterns,
        sections,
        question_types,
        topics,
        concepts,
        resources,
    })
}

fn query_sections(connection: &Connection, exam_id: i64) -> Result<Vec<SectionOption>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, code, title, default_question_type_id
             FROM exam_sections
             WHERE exam_id = ?1 AND is_archived = 0
             ORDER BY display_order, id",
        )
        .map_err(|error| AppError::database("Practice sections could not be prepared", error))?;
    statement
        .query_map([exam_id], |row| {
            Ok(SectionOption {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                default_question_type_id: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database("Practice sections could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice section is invalid", error))
}

fn query_question_types(
    connection: &Connection,
    exam_id: i64,
) -> Result<Vec<QuestionTypeOption>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, code, title, answer_format FROM exam_question_types
             WHERE exam_id = ?1 AND is_archived = 0 ORDER BY code",
        )
        .map_err(|error| AppError::database("Question types could not be prepared", error))?;
    statement
        .query_map([exam_id], |row| {
            Ok(QuestionTypeOption {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                answer_format: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database("Question types could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A question type is invalid", error))
}

fn query_topics(connection: &Connection, exam_id: i64) -> Result<Vec<TopicOption>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, code, title, node_kind, category, parent_id, display_order
             FROM exam_syllabus_nodes
             WHERE exam_id = ?1 AND is_archived = 0
             ORDER BY display_order, id",
        )
        .map_err(|error| AppError::database("Practice topics could not be prepared", error))?;
    statement
        .query_map([exam_id], |row| {
            Ok(TopicOption {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                node_kind: row.get(3)?,
                category: row.get(4)?,
                parent_id: row.get(5)?,
                display_order: row.get(6)?,
            })
        })
        .map_err(|error| AppError::database("Practice topics could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice topic is invalid", error))
}

fn query_concepts(connection: &Connection) -> Result<Vec<ConceptOption>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, slug, title, domain FROM concepts
             WHERE is_archived = 0 ORDER BY domain COLLATE NOCASE, title COLLATE NOCASE, id",
        )
        .map_err(|error| AppError::database("Practice concepts could not be prepared", error))?;
    statement
        .query_map([], |row| {
            Ok(ConceptOption {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                domain: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database("Practice concepts could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice concept is invalid", error))
}

fn query_resources(connection: &Connection) -> Result<Vec<ResourceOption>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, resource_type FROM resources
             WHERE is_archived = 0 ORDER BY title COLLATE NOCASE, id",
        )
        .map_err(|error| AppError::database("Practice resources could not be prepared", error))?;
    statement
        .query_map([], |row| {
            Ok(ResourceOption {
                id: row.get(0)?,
                title: row.get(1)?,
                resource_type: row.get(2)?,
            })
        })
        .map_err(|error| AppError::database("Practice resources could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice resource is invalid", error))
}

pub fn list_questions(
    connection: &Connection,
    filter: V2PyqFilter,
) -> Result<Vec<V2PyqQuestionSummary>, AppError> {
    validate_exam(connection, filter.exam_id)?;
    let limit = filter.limit.unwrap_or(200).clamp(1, 500);
    let offset = filter.offset.unwrap_or(0).max(0);
    let include_archived = filter.include_archived.unwrap_or(false);
    let verification = filter
        .verification_status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let search = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{}%", value.replace('%', "\\%").replace('_', "\\_")));

    let mut sql = String::from(
        "SELECT q.id, q.exam_id, q.year, q.session_label, q.question_number,
                q.section_id, s.code, s.title, q.question_type_id, t.code,
                q.pattern_version_id, pv.version_label, q.statement, q.max_marks,
                q.negative_marks, q.unanswered_marks, q.difficulty, q.source_title,
                q.source_page, q.source_url, q.answer_key_source, q.answer_key_version,
                q.answer_key_verified, q.verification_status, q.numeric_tolerance,
                q.correct_range, q.official_explanation, q.third_party_solution,
                q.user_note, q.resource_id, q.is_archived,
                (SELECT COUNT(*) FROM pyq_attempts a WHERE a.question_id = q.id),
                (SELECT COUNT(*) FROM pyq_question_concepts pc WHERE pc.question_id = q.id),
                q.syllabus_node_id, sn.title,
                COALESCE((
                    SELECT json_group_array(qt.node_id)
                    FROM pyq_question_topics qt WHERE qt.question_id = q.id
                ), '[]'),
                q.options_json,
                q.source_id, q.answer_key_id
         FROM pyq_questions q
         JOIN exam_sections s ON s.id = q.section_id
         JOIN exam_question_types t ON t.id = q.question_type_id
         LEFT JOIN exam_pattern_versions pv ON pv.id = q.pattern_version_id
         LEFT JOIN exam_syllabus_nodes sn ON sn.id = q.syllabus_node_id
         WHERE q.exam_id = ?1",
    );
    let mut values: Vec<rusqlite::types::Value> = vec![filter.exam_id.into()];
    if !include_archived {
        sql.push_str(" AND q.is_archived = 0");
    }
    if let Some(year) = filter.year {
        sql.push_str(" AND q.year = ?");
        values.push(year.into());
    }
    if let Some(section_id) = filter.section_id {
        sql.push_str(" AND q.section_id = ?");
        values.push(section_id.into());
    }
    if let Some(question_type_id) = filter.question_type_id {
        sql.push_str(" AND q.question_type_id = ?");
        values.push(question_type_id.into());
    }
    if let Some(question_id) = filter.question_id {
        sql.push_str(" AND q.id = ?");
        values.push(question_id.into());
    }
    if let Some(node_id) = filter.syllabus_node_id {
        sql.push_str(
            " AND (q.syllabus_node_id = ? OR EXISTS (
                SELECT 1 FROM pyq_question_topics qt
                WHERE qt.question_id = q.id AND qt.node_id = ?
            ))",
        );
        values.push(node_id.into());
        values.push(node_id.into());
    }
    if let Some(status) = verification {
        sql.push_str(" AND q.verification_status = ?");
        values.push(status.to_owned().into());
    }
    if let Some(search) = &search {
        sql.push_str(
            " AND (q.statement LIKE ? ESCAPE '\\' OR q.source_title LIKE ? ESCAPE '\\'
                   OR q.source_page LIKE ? ESCAPE '\\' OR q.official_explanation LIKE ? ESCAPE '\\'
                   OR q.third_party_solution LIKE ? ESCAPE '\\' OR q.user_note LIKE ? ESCAPE '\\')",
        );
        for _ in 0..6 {
            values.push(search.clone().into());
        }
    }
    sql.push_str(" ORDER BY q.year DESC, q.session_label DESC, s.display_order, q.question_number LIMIT ? OFFSET ?");
    values.push(limit.into());
    values.push(offset.into());

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("PYQ questions could not be prepared", error))?;
    statement
        .query_map(rusqlite::params_from_iter(values), map_question)
        .map_err(|error| AppError::database("PYQ questions could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A PYQ question is invalid", error))
}

fn map_question(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2PyqQuestionSummary> {
    Ok(V2PyqQuestionSummary {
        id: row.get(0)?,
        exam_id: row.get(1)?,
        year: row.get(2)?,
        session_label: row.get(3)?,
        question_number: row.get(4)?,
        section_id: row.get(5)?,
        section_code: row.get(6)?,
        section_title: row.get(7)?,
        question_type_id: row.get(8)?,
        question_type_code: row.get(9)?,
        pattern_version_id: row.get(10)?,
        pattern_version_label: row.get(11)?,
        statement: row.get(12)?,
        max_marks: row.get(13)?,
        negative_marks: row.get(14)?,
        unanswered_marks: row.get(15)?,
        difficulty: row.get(16)?,
        source_title: row.get(17)?,
        source_page: row.get(18)?,
        source_url: row.get(19)?,
        answer_key_source: row.get(20)?,
        answer_key_version: row.get(21)?,
        answer_key_verified: row.get::<_, i64>(22)? == 1,
        verification_status: row.get(23)?,
        numeric_tolerance: row.get(24)?,
        correct_range: row.get(25)?,
        official_explanation: row.get(26)?,
        third_party_solution: row.get(27)?,
        user_note: row.get(28)?,
        resource_id: row.get(29)?,
        is_archived: row.get::<_, i64>(30)? == 1,
        attempt_count: row.get(31)?,
        concept_count: row.get(32)?,
        syllabus_node_id: row.get(33)?,
        syllabus_node_title: row.get(34)?,
        topic_ids: serde_json::from_str(&row.get::<_, String>(35)?).unwrap_or_default(),
        options: serde_json::from_str(&row.get::<_, String>(36)?).unwrap_or_default(),
        source_id: row.get(37)?,
        answer_key_id: row.get(38)?,
    })
}

pub fn detail(
    connection: &Connection,
    exam_id: i64,
    question_id: i64,
) -> Result<V2PyqQuestionDetail, AppError> {
    let question = list_questions(
        connection,
        V2PyqFilter {
            exam_id,
            include_archived: Some(true),
            question_id: Some(question_id),
            limit: Some(1),
            ..default_filter(exam_id)
        },
    )?
    .into_iter()
    .next()
    .ok_or_else(AppError::pyq_not_found)?;

    let mut topic_statement = connection
        .prepare(
            "SELECT n.id, n.code, n.title, qt.is_primary
             FROM pyq_question_topics qt
             JOIN exam_syllabus_nodes n ON n.id = qt.node_id
             WHERE qt.question_id = ?1 ORDER BY qt.is_primary DESC, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("PYQ topics could not be prepared", error))?;
    let topics = topic_statement
        .query_map([question_id], |row| {
            Ok(V2PyqQuestionTopic {
                node_id: row.get(0)?,
                node_code: row.get(1)?,
                node_title: row.get(2)?,
                is_primary: row.get::<_, i64>(3)? == 1,
            })
        })
        .map_err(|error| AppError::database("PYQ topics could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A PYQ topic is invalid", error))?;
    drop(topic_statement);

    let mut concept_statement = connection
        .prepare(
            "SELECT c.id, c.slug, c.title, pc.is_primary
             FROM pyq_question_concepts pc
             JOIN concepts c ON c.id = pc.concept_id
             WHERE pc.question_id = ?1 ORDER BY pc.is_primary DESC, c.title COLLATE NOCASE, c.id",
        )
        .map_err(|error| AppError::database("PYQ concepts could not be prepared", error))?;
    let concept_rows = concept_statement
        .query_map([question_id], |row| {
            Ok(V2PyqQuestionConcept {
                concept_id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                is_primary: row.get::<_, i64>(3)? == 1,
            })
        })
        .map_err(|error| AppError::database("PYQ concepts could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A PYQ concept is invalid", error))?;
    drop(concept_statement);

    let correct_answer: String = connection
        .query_row(
            "SELECT correct_answer FROM pyq_questions WHERE id = ?1 AND exam_id = ?2",
            params![question_id, exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("PYQ answer key could not be read", error))?;
    Ok(V2PyqQuestionDetail {
        attempts: list_attempts(connection, exam_id, Some(question_id))?,
        question,
        correct_answer,
        topics,
        concepts: concept_rows,
    })
}

fn default_filter(exam_id: i64) -> V2PyqFilter {
    V2PyqFilter {
        exam_id,
        year: None,
        section_id: None,
        question_type_id: None,
        syllabus_node_id: None,
        verification_status: None,
        question_id: None,
        query: None,
        include_archived: None,
        limit: None,
        offset: None,
    }
}

fn validate_exam(connection: &Connection, exam_id: i64) -> Result<String, AppError> {
    connection
        .query_row("SELECT code FROM exams WHERE id = ?1", [exam_id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database("Exam could not be checked", error))?
        .ok_or_else(|| AppError::validation("examId", "Choose an existing examination."))
}

fn require_active_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    let active: Option<i64> = connection
        .query_row(
            "SELECT is_active FROM exam_profiles WHERE exam_id = ?1",
            [exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Exam activity could not be checked", error))?;
    match active {
        Some(1) => Ok(()),
        Some(_) => Err(AppError::forbidden(
            "examId",
            "Inactive examinations are read-only. Reactivate the examination to record practice.",
        )),
        None => Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        )),
    }
}

fn validate_section(
    connection: &Connection,
    exam_id: i64,
    section_id: i64,
) -> Result<(String, String), AppError> {
    connection
        .query_row(
            "SELECT code, title FROM exam_sections
             WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0",
            params![section_id, exam_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Section could not be checked", error))?
        .ok_or_else(|| AppError::validation("sectionId", "Choose a section from this examination."))
}

fn validate_type(
    connection: &Connection,
    exam_id: i64,
    question_type_id: i64,
) -> Result<String, AppError> {
    connection
        .query_row(
            "SELECT code FROM exam_question_types
             WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0",
            params![question_type_id, exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Question type could not be checked", error))?
        .ok_or_else(|| {
            AppError::validation(
                "questionTypeId",
                "Choose a question type from this examination.",
            )
        })
}

fn validate_topic(connection: &Connection, exam_id: i64, node_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exam_syllabus_nodes
             WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0)",
            params![node_id, exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Syllabus topic could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "syllabusNodeId",
            "Choose an active syllabus topic from this examination.",
        ));
    }
    Ok(())
}

fn validate_concept(connection: &Connection, concept_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1 AND is_archived = 0)",
            [concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Concept could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "conceptIds",
            "Choose an active shared concept.",
        ));
    }
    Ok(())
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-6
}

#[allow(clippy::too_many_arguments)]
fn validate_pattern_marking(
    connection: &Connection,
    exam_id: i64,
    pattern_version_id: i64,
    section_id: i64,
    type_code: &str,
    max_marks: f64,
    negative_marks: f64,
    unanswered_marks: f64,
) -> Result<(), AppError> {
    let section: Option<(String, String, f64)> = connection
        .query_row(
            "SELECT ps.question_types_json, ps.marking_variants_json, ps.unanswered_marks
             FROM exam_pattern_sections ps
             JOIN exam_pattern_versions pv ON pv.id = ps.pattern_version_id
             WHERE ps.pattern_version_id = ?1 AND pv.exam_id = ?2
               AND ps.section_id = ?3 AND pv.verification_status = 'verified'",
            params![pattern_version_id, exam_id, section_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Pattern marking could not be checked", error))?;
    let Some((question_types_json, marking_variants_json, pattern_unanswered)) = section else {
        return Err(AppError::validation(
            "patternVersionId",
            "The selected verified pattern does not contain this section.",
        ));
    };
    let question_types: Vec<String> =
        serde_json::from_str(&question_types_json).unwrap_or_default();
    if !question_types
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(type_code))
    {
        return Err(AppError::validation(
            "questionTypeId",
            format!("Question type {type_code} is not part of this pattern section."),
        ));
    }
    if !approximately_equal(unanswered_marks, pattern_unanswered) {
        return Err(AppError::validation(
            "unansweredMarks",
            "Unanswered marks do not match the verified pattern section.",
        ));
    }
    let marking_variants: Vec<serde_json::Value> =
        serde_json::from_str(&marking_variants_json).unwrap_or_default();
    let matches_variant = marking_variants.iter().any(|variant| {
        let variant_type = variant
            .get("questionType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let correct = variant.get("correct").and_then(serde_json::Value::as_f64);
        let incorrect = variant.get("incorrect").and_then(serde_json::Value::as_f64);
        variant_type.eq_ignore_ascii_case(type_code)
            && correct.is_some_and(|value| approximately_equal(value, max_marks))
            && incorrect.is_some_and(|value| approximately_equal(value, negative_marks))
    });
    if !matches_variant {
        return Err(AppError::validation(
            "maxMarks",
            "Marks must match a verified marking variant for this pattern section and question type.",
        ));
    }
    Ok(())
}

fn validate_question_options(
    answer_kind: AnswerKind,
    correct_answer: &str,
    options: &[V2QuestionOption],
) -> Result<Vec<V2QuestionOption>, AppError> {
    if options.is_empty() {
        return Ok(Vec::new());
    }
    if answer_kind == AnswerKind::Numeric {
        return Err(AppError::validation(
            "options",
            "Numerical questions cannot have answer choices.",
        ));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(options.len());
    for option in options {
        let key = option.key.trim().to_ascii_uppercase();
        if key.is_empty()
            || key.chars().count() > 8
            || !key
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        {
            return Err(AppError::validation(
                "options",
                "Each option key must be 1-8 uppercase letters or digits.",
            ));
        }
        if !seen.insert(key.clone()) {
            return Err(AppError::validation(
                "options",
                format!("Option key {key} appears more than once."),
            ));
        }
        let text = option.text.trim();
        if text.is_empty() {
            return Err(AppError::validation(
                "options",
                "Option text cannot be empty.",
            ));
        }
        normalized.push(V2QuestionOption {
            key,
            text: text.to_owned(),
        });
    }
    let keys = normalized
        .iter()
        .map(|option| option.key.as_str())
        .collect::<HashSet<_>>();
    let correct_keys = super::scoring::normalize_choice(correct_answer);
    for key in correct_keys.split(',').filter(|key| !key.is_empty()) {
        if !keys.contains(key) {
            return Err(AppError::validation(
                "correctAnswer",
                format!("Correct answer {key} is not one of the recorded options."),
            ));
        }
    }
    Ok(normalized)
}

fn validate_answer(
    answer_kind: AnswerKind,
    correct_answer: &str,
    correct_range: &str,
) -> Result<(), AppError> {
    match answer_kind {
        AnswerKind::SingleChoice => {
            let normalized = super::scoring::normalize_choice(correct_answer);
            if normalized.is_empty() || normalized.contains(',') {
                return Err(AppError::validation(
                    "correctAnswer",
                    "A single-choice answer must be one option, for example A.",
                ));
            }
        }
        AnswerKind::MultiChoice => {
            let normalized = super::scoring::normalize_choice(correct_answer);
            if normalized.is_empty() {
                return Err(AppError::validation(
                    "correctAnswer",
                    "A multiple-select answer must list at least one option.",
                ));
            }
        }
        AnswerKind::Numeric => {
            if correct_answer.trim().is_empty() && correct_range.trim().is_empty() {
                return Err(AppError::validation(
                    "correctAnswer",
                    "A numerical answer needs a value or a verified range.",
                ));
            }
            if super::scoring::normalize_numeric(correct_answer).is_none()
                && correct_range.trim().is_empty()
            {
                return Err(AppError::validation(
                    "correctAnswer",
                    "The numerical answer must be a number or a verified range.",
                ));
            }
        }
    }
    Ok(())
}

pub fn save_question(
    connection: &mut Connection,
    input: V2PyqQuestionInput,
) -> Result<V2PyqQuestionDetail, AppError> {
    require_active_exam(connection, input.exam_id)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ question transaction could not start", error)
    })?;
    let id = save_question_in_transaction(&transaction, input.clone(), Utc::now())?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ question could not be committed", error))?;
    detail(connection, input.exam_id, id)
}

#[allow(clippy::too_many_lines)]
pub(super) fn save_question_in_transaction(
    transaction: &Transaction<'_>,
    input: V2PyqQuestionInput,
    now: chrono::DateTime<Utc>,
) -> Result<i64, AppError> {
    validate_exam(transaction, input.exam_id)?;
    validate_section(transaction, input.exam_id, input.section_id)?;
    let type_code = validate_type(transaction, input.exam_id, input.question_type_id)?;
    let answer_kind = AnswerKind::from_code(&type_code)
        .ok_or_else(|| AppError::validation("questionTypeId", "Unsupported question type."))?;
    if !(1900..=2100).contains(&input.year) {
        return Err(AppError::validation(
            "year",
            "Question year must be between 1900 and 2100.",
        ));
    }
    let session_label = input.session_label.trim().to_owned();
    if session_label.chars().count() > 120 {
        return Err(AppError::validation(
            "sessionLabel",
            "Session label must be 120 characters or fewer.",
        ));
    }
    let question_number = input.question_number.trim().to_owned();
    if question_number.is_empty() || question_number.chars().count() > 60 {
        return Err(AppError::validation(
            "questionNumber",
            "Question number must be between 1 and 60 characters.",
        ));
    }
    let statement = input.statement.trim().to_owned();
    if statement.chars().count() > 20_000 {
        return Err(AppError::validation(
            "statement",
            "Question statement must be 20,000 characters or fewer.",
        ));
    }
    if statement.is_empty() && input.source_title.trim().is_empty() && input.source_url.is_none() {
        return Err(AppError::validation(
            "statement",
            "Provide question text or a local/official source reference.",
        ));
    }
    if !(1..=5).contains(&input.difficulty) {
        return Err(AppError::validation(
            "difficulty",
            "Difficulty must be between 1 and 5.",
        ));
    }
    if !input.max_marks.is_finite() || input.max_marks <= 0.0 {
        return Err(AppError::validation(
            "maxMarks",
            "Maximum marks must be positive.",
        ));
    }
    if !input.negative_marks.is_finite() || input.negative_marks > 0.0 {
        return Err(AppError::validation(
            "negativeMarks",
            "Negative marks must be zero or less.",
        ));
    }
    if !input.unanswered_marks.is_finite() || input.unanswered_marks > 0.0 {
        return Err(AppError::validation(
            "unansweredMarks",
            "Unanswered marks must be zero or less.",
        ));
    }
    if matches!(answer_kind, AnswerKind::MultiChoice | AnswerKind::Numeric)
        && input.negative_marks < 0.0
    {
        return Err(AppError::validation(
            "negativeMarks",
            "This question type does not use negative marking in the supported patterns.",
        ));
    }
    if !input.numeric_tolerance.is_finite() || input.numeric_tolerance < 0.0 {
        return Err(AppError::validation(
            "numericTolerance",
            "Numeric tolerance cannot be negative.",
        ));
    }
    let status = input.verification_status.trim().to_ascii_lowercase();
    if !matches!(
        status.as_str(),
        "draft" | "pending" | "verified" | "rejected"
    ) {
        return Err(AppError::validation(
            "verificationStatus",
            "Choose draft, pending, verified or rejected.",
        ));
    }
    let correct_answer = input.correct_answer.trim().to_owned();
    let correct_range = input.correct_range.trim().to_owned();
    validate_answer(answer_kind, &correct_answer, &correct_range)?;
    let options = validate_question_options(answer_kind, &correct_answer, &input.options)?;
    let options_json = serde_json::to_string(&options).map_err(|error| {
        AppError::validation("options", format!("Options could not be encoded: {error}"))
    })?;
    if status == "verified" && !input.answer_key_verified {
        return Err(AppError::validation(
            "answerKeyVerified",
            "A verified question must have a verified answer key.",
        ));
    }
    if input.answer_key_verified && correct_answer.is_empty() && correct_range.is_empty() {
        return Err(AppError::validation(
            "correctAnswer",
            "An answer key cannot be verified without an answer.",
        ));
    }
    if let Some(pattern_version_id) = input.pattern_version_id {
        validate_pattern_marking(
            transaction,
            input.exam_id,
            pattern_version_id,
            input.section_id,
            &type_code,
            input.max_marks,
            input.negative_marks,
            input.unanswered_marks,
        )?;
    }
    if let Some(resource_id) = input.resource_id {
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM resources WHERE id = ?1 AND is_archived = 0)",
                [resource_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Resource could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "resourceId",
                "Choose an active managed resource or leave the source reference blank.",
            ));
        }
    }
    if let Some(source_id) = input.source_id {
        let source: Option<(i64, i64)> = transaction
            .query_row(
                "SELECT exam_id, year FROM pyq_sources WHERE id = ?1",
                [source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| AppError::database("Question source could not be checked", error))?;
        match source {
            Some((exam_id, year)) if exam_id == input.exam_id && year == input.year => {}
            Some(_) => {
                return Err(AppError::validation(
                    "sourceId",
                    "The source must belong to the same examination and year as the question.",
                ));
            }
            None => {
                return Err(AppError::validation(
                    "sourceId",
                    "Choose an existing registered source.",
                ));
            }
        }
    }
    if let Some(answer_key_id) = input.answer_key_id {
        let key: Option<(i64, i64, Option<i64>)> = transaction
            .query_row(
                "SELECT k.exam_id, k.year, s.associated_source_id
                 FROM pyq_answer_keys k
                 JOIN pyq_sources s ON s.id = k.source_id
                 WHERE k.id = ?1",
                [answer_key_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("Question answer key could not be checked", error)
            })?;
        match key {
            Some((exam_id, year, associated_source_id))
                if exam_id == input.exam_id
                    && year == input.year
                    && input.source_id.is_none_or(|expected| {
                        associated_source_id == Some(expected) || associated_source_id.is_none()
                    }) => {}
            Some(_) => {
                return Err(AppError::validation(
                    "answerKeyId",
                    "The answer key must belong to the same examination, year and question-paper source.",
                ));
            }
            None => {
                return Err(AppError::validation(
                    "answerKeyId",
                    "Choose an existing answer key.",
                ));
            }
        }
    }
    if let Some(node_id) = input.syllabus_node_id {
        validate_topic(transaction, input.exam_id, node_id)?;
    }
    let mut topic_ids = input.topic_ids.clone();
    if let Some(node_id) = input.syllabus_node_id {
        topic_ids.push(node_id);
    }
    topic_ids.sort_unstable();
    topic_ids.dedup();
    for node_id in &topic_ids {
        validate_topic(transaction, input.exam_id, *node_id)?;
    }
    let mut concept_ids = input.concept_ids.clone();
    concept_ids.sort_unstable();
    concept_ids.dedup();
    for concept_id in &concept_ids {
        validate_concept(transaction, *concept_id)?;
    }
    let source_url = input
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(url) = &source_url
        && !url.starts_with("https://")
    {
        return Err(AppError::validation(
            "sourceUrl",
            "Source URL must begin with https://.",
        ));
    }
    let dedupe_key = if input.dedupe_key.trim().is_empty() {
        stable_import_key(
            "pyq",
            &[
                &input.exam_id.to_string(),
                &input.year.to_string(),
                &session_label,
                &input.section_id.to_string(),
                &question_number,
            ],
        )
    } else {
        input.dedupe_key.trim().to_owned()
    };
    if dedupe_key.chars().count() > 200 {
        return Err(AppError::validation("dedupeKey", "Dedupe key is too long."));
    }
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let existing_id = if let Some(id) = input.id {
        let owner: Option<i64> = transaction
            .query_row(
                "SELECT exam_id FROM pyq_questions WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AppError::database("Question could not be checked", error))?;
        if owner != Some(input.exam_id) {
            return Err(AppError::pyq_not_found());
        }
        Some(id)
    } else {
        None
    };

    let excluded_id = existing_id.unwrap_or(-1);
    let duplicate_identity: i64 = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pyq_questions
                WHERE exam_id = ?1 AND year = ?2 AND session_label = ?3
                  AND section_id = ?4 AND question_number = ?5 AND id <> ?6
             )",
            params![
                input.exam_id,
                input.year,
                session_label,
                input.section_id,
                question_number,
                excluded_id,
            ],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Question identity could not be checked", error))?;
    let duplicate_key: i64 = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pyq_questions
                WHERE exam_id = ?1 AND dedupe_key = ?2 AND id <> ?3
             )",
            params![input.exam_id, dedupe_key, excluded_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Question dedupe key could not be checked", error))?;
    if duplicate_identity == 1 {
        return Err(AppError::validation(
            "questionNumber",
            "A question with this year, session, section and number already exists.",
        ));
    }
    if duplicate_key == 1 {
        return Err(AppError::validation(
            "dedupeKey",
            "A question with this duplicate-detection key already exists.",
        ));
    }

    let id = if let Some(id) = existing_id {
        transaction
            .execute(
                "UPDATE pyq_questions SET
                    year = ?1, session_label = ?2, section_id = ?3, question_type_id = ?4,
                    question_number = ?5, statement = ?6, correct_answer = ?7, max_marks = ?8,
                    negative_marks = ?9, unanswered_marks = ?10, difficulty = ?11,
                    source_url = ?12, dedupe_key = ?13, pattern_version_id = ?14,
                    syllabus_node_id = ?15, source_title = ?16, source_page = ?17,
                    answer_key_source = ?18, answer_key_version = ?19, answer_key_verified = ?20,
                    verification_status = ?21, numeric_tolerance = ?22, correct_range = ?23,
                    official_explanation = ?24, third_party_solution = ?25, user_note = ?26,
                    resource_id = ?27, source_id = ?28, answer_key_id = ?29, updated_at = ?30
                 WHERE id = ?31",
                params![
                    input.year,
                    session_label,
                    input.section_id,
                    input.question_type_id,
                    question_number,
                    statement,
                    correct_answer,
                    input.max_marks,
                    input.negative_marks,
                    input.unanswered_marks,
                    input.difficulty,
                    source_url,
                    dedupe_key,
                    input.pattern_version_id,
                    input.syllabus_node_id,
                    input.source_title.trim(),
                    input.source_page.trim(),
                    input.answer_key_source.trim(),
                    input.answer_key_version.trim(),
                    i64::from(input.answer_key_verified),
                    status,
                    input.numeric_tolerance,
                    correct_range,
                    input.official_explanation.trim(),
                    input.third_party_solution.trim(),
                    input.user_note.trim(),
                    input.resource_id,
                    input.source_id,
                    input.answer_key_id,
                    now_text,
                    id,
                ],
            )
            .map_err(|error| AppError::database("The PYQ question could not be updated", error))?;
        transaction
            .execute(
                "UPDATE pyq_questions SET options_json = ?1 WHERE id = ?2",
                params![options_json, id],
            )
            .map_err(|error| {
                AppError::database("PYQ question options could not be updated", error)
            })?;
        id
    } else {
        transaction
            .execute(
                "INSERT INTO pyq_questions(
                    exam_id, section_id, question_type_id, year, session_label, question_number,
                    statement, correct_answer, max_marks, negative_marks, unanswered_marks,
                    difficulty, source_url, dedupe_key, is_archived, created_at, updated_at,
                    pattern_version_id, syllabus_node_id, source_title, source_page,
                    answer_key_source, answer_key_version, answer_key_verified,
                    verification_status, numeric_tolerance, correct_range,
                    official_explanation, third_party_solution, user_note, resource_id,
                    source_id, answer_key_id, options_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                           0, ?15, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                           ?26, ?27, ?28, ?29, ?30, ?31, ?32)",
                params![
                    input.exam_id,
                    input.section_id,
                    input.question_type_id,
                    input.year,
                    session_label,
                    question_number,
                    statement,
                    correct_answer,
                    input.max_marks,
                    input.negative_marks,
                    input.unanswered_marks,
                    input.difficulty,
                    source_url,
                    dedupe_key,
                    now_text,
                    input.pattern_version_id,
                    input.syllabus_node_id,
                    input.source_title.trim(),
                    input.source_page.trim(),
                    input.answer_key_source.trim(),
                    input.answer_key_version.trim(),
                    i64::from(input.answer_key_verified),
                    status,
                    input.numeric_tolerance,
                    correct_range,
                    input.official_explanation.trim(),
                    input.third_party_solution.trim(),
                    input.user_note.trim(),
                    input.resource_id,
                    input.source_id,
                    input.answer_key_id,
                    options_json,
                ],
            )
            .map_err(|error| AppError::database("The PYQ question could not be stored", error))?;
        transaction.last_insert_rowid()
    };

    transaction
        .execute(
            "DELETE FROM pyq_question_topics WHERE question_id = ?1",
            [existing_id.unwrap_or(id)],
        )
        .map_err(|error| AppError::database("PYQ topic mappings could not be replaced", error))?;
    for node_id in &topic_ids {
        transaction
            .execute(
                "INSERT INTO pyq_question_topics(exam_id, question_id, node_id, is_primary, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    input.exam_id,
                    id,
                    node_id,
                    i64::from(input.syllabus_node_id == Some(*node_id)),
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("PYQ topic mapping could not be stored", error))?;
    }
    transaction
        .execute(
            "DELETE FROM pyq_question_concepts WHERE question_id = ?1",
            [id],
        )
        .map_err(|error| AppError::database("PYQ concept mappings could not be replaced", error))?;
    for concept_id in &concept_ids {
        transaction
            .execute(
                "INSERT INTO pyq_question_concepts(
                    exam_id, question_id, concept_id, weight, is_primary, created_at
                 ) VALUES (?1, ?2, ?3, 1, ?4, ?5)",
                params![
                    input.exam_id,
                    id,
                    concept_id,
                    i64::from(concept_ids.first() == Some(concept_id)),
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("PYQ concept mapping could not be stored", error)
            })?;
    }
    Ok(id)
}

pub fn archive_question(
    connection: &mut Connection,
    exam_id: i64,
    question_id: i64,
    archived: bool,
) -> Result<(), AppError> {
    require_active_exam(connection, exam_id)?;
    let changed = connection
        .execute(
            "UPDATE pyq_questions SET is_archived = ?1, updated_at = ?2
             WHERE id = ?3 AND exam_id = ?4",
            params![
                i64::from(archived),
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                question_id,
                exam_id,
            ],
        )
        .map_err(|error| {
            AppError::database("PYQ question archive state could not be saved", error)
        })?;
    if changed == 0 {
        return Err(AppError::pyq_not_found());
    }
    Ok(())
}

pub fn list_attempts(
    connection: &Connection,
    exam_id: i64,
    question_id: Option<i64>,
) -> Result<Vec<V2PyqAttempt>, AppError> {
    let mut sql = String::from(
        "SELECT a.id, a.exam_id, a.question_id, q.question_number, q.statement,
                a.attempt_date, a.selected_answer, a.status, COALESCE(a.marks_obtained, 0),
                a.max_marks_at_attempt, a.negative_marks_at_attempt, a.time_spent_seconds,
                a.confidence, a.mistakes, a.review_status, a.notes, a.pattern_version_id,
                a.answer_key_version, a.correct_answer_at_attempt, a.scoring_engine,
                a.unanswered_marks_at_attempt
         FROM pyq_attempts a JOIN pyq_questions q ON q.id = a.question_id
         WHERE a.exam_id = ?1",
    );
    if question_id.is_some() {
        sql.push_str(" AND a.question_id = ?2");
    }
    sql.push_str(" ORDER BY a.attempt_date DESC, a.id DESC LIMIT 200");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("PYQ attempts could not be prepared", error))?;
    let rows = if let Some(question_id) = question_id {
        statement
            .query_map(params![exam_id, question_id], map_attempt)
            .map_err(|error| AppError::database("PYQ attempts could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
    } else {
        statement
            .query_map([exam_id], map_attempt)
            .map_err(|error| AppError::database("PYQ attempts could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
    };
    rows.map_err(|error| AppError::database("A PYQ attempt is invalid", error))
}

fn map_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2PyqAttempt> {
    Ok(V2PyqAttempt {
        id: row.get(0)?,
        exam_id: row.get(1)?,
        question_id: row.get(2)?,
        question_number: row.get(3)?,
        question_statement: row.get(4)?,
        attempt_date: row.get(5)?,
        selected_answer: row.get(6)?,
        status: row.get(7)?,
        marks_obtained: row.get(8)?,
        max_marks_at_attempt: row.get(9)?,
        negative_marks_at_attempt: row.get(10)?,
        time_spent_seconds: row.get(11)?,
        confidence: row.get(12)?,
        mistakes: row.get(13)?,
        review_status: row.get(14)?,
        notes: row.get(15)?,
        pattern_version_id: row.get(16)?,
        answer_key_version: row.get(17)?,
        correct_answer_at_attempt: row.get(18)?,
        scoring_engine: row.get(19)?,
        unanswered_marks_at_attempt: row.get(20)?,
    })
}

pub fn record_attempt(
    connection: &mut Connection,
    input: V2PyqAttemptInput,
) -> Result<V2PyqAttempt, AppError> {
    require_active_exam(connection, input.exam_id)?;
    if !(0..=86_400).contains(&input.time_spent_seconds) {
        return Err(AppError::validation(
            "timeSpentSeconds",
            "Time spent must be between 0 and 86,400 seconds.",
        ));
    }
    if let Some(confidence) = input.confidence
        && !(1..=5).contains(&confidence)
    {
        return Err(AppError::validation(
            "confidence",
            "Confidence must be 1-5.",
        ));
    }
    let review_status = if input.review_status.trim().is_empty() {
        "unreviewed".to_owned()
    } else {
        input.review_status.trim().to_owned()
    };
    if !matches!(review_status.as_str(), "unreviewed" | "reviewed") {
        return Err(AppError::validation(
            "reviewStatus",
            "Choose unreviewed or reviewed.",
        ));
    }
    let question = connection
        .query_row(
            "SELECT q.exam_id, q.section_id, s.code, t.code, q.correct_answer,
                    q.correct_range, q.numeric_tolerance, q.max_marks, q.negative_marks,
                    q.unanswered_marks, q.difficulty, q.pattern_version_id,
                    q.answer_key_version, q.verification_status, q.answer_key_verified,
                    q.is_archived
             FROM pyq_questions q
             JOIN exam_sections s ON s.id = q.section_id
             JOIN exam_question_types t ON t.id = q.question_type_id
             WHERE q.id = ?1 AND q.exam_id = ?2",
            params![input.question_id, input.exam_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, f64>(6)?,
                    row.get::<_, f64>(7)?,
                    row.get::<_, f64>(8)?,
                    row.get::<_, f64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("PYQ question could not be read", error))?
        .ok_or_else(AppError::pyq_not_found)?;
    if question.15 == 1 {
        return Err(AppError::validation(
            "questionId",
            "Archived questions keep their history but cannot receive new attempts.",
        ));
    }
    let exam_code = validate_exam(connection, input.exam_id)?;
    let selected = input.selected_answer.trim();
    let scored = if selected.is_empty() {
        ScoredResponse {
            status: "skipped".to_owned(),
            marks: 0.0,
            normalized_answer: String::new(),
            engine: exam_code.clone(),
        }
    } else {
        if question.13 != "verified" || question.14 != 1 || question.4.is_empty() {
            return Err(AppError::validation(
                "questionId",
                "This question does not have a verified answer key. Record a skipped attempt or finish answer-key verification first.",
            ));
        }
        score_question(
            &exam_code,
            &question.2,
            &question.3,
            question.7,
            &question.4,
            &question.5,
            selected,
            question.6,
        )
        .map_err(|message| AppError::validation("selectedAnswer", message))?
    };
    if scored.status != "skipped" && selected.is_empty() {
        return Err(AppError::validation(
            "selectedAnswer",
            "Choose an answer or leave it blank to record a skipped attempt.",
        ));
    }
    let now = Utc::now();
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The PYQ attempt transaction could not start", error)
    })?;
    transaction
        .execute(
            "INSERT INTO pyq_attempts(
                exam_id, question_id, attempt_date, selected_answer, status, marks_obtained,
                max_marks_at_attempt, negative_marks_at_attempt, time_spent_seconds,
                confidence, mistakes, review_status, notes, created_at, updated_at,
                pattern_version_id, answer_key_version, correct_answer_at_attempt,
                numeric_tolerance_at_attempt, difficulty_at_attempt,
                unanswered_marks_at_attempt, scoring_engine
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14,
                       ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                input.exam_id,
                input.question_id,
                input.attempt_date,
                scored.normalized_answer,
                scored.status,
                scored.marks,
                question.7,
                question.8,
                input.time_spent_seconds,
                input.confidence,
                input.mistakes.trim(),
                review_status,
                input.notes.trim(),
                now_text,
                question.11,
                question.12,
                question.4,
                question.6,
                question.10,
                question.9,
                scored.engine,
            ],
        )
        .map_err(|error| AppError::database("The PYQ attempt could not be saved", error))?;
    let attempt_id = transaction.last_insert_rowid();
    if scored.status != "skipped" && question.13 == "verified" && question.14 == 1 {
        let mut statement = transaction
            .prepare(
                "SELECT concept_id FROM pyq_question_concepts
                 WHERE question_id = ?1 AND exam_id = ?2 ORDER BY is_primary DESC, concept_id",
            )
            .map_err(|error| {
                AppError::database("PYQ evidence concepts could not be read", error)
            })?;
        let concept_ids = statement
            .query_map(params![input.question_id, input.exam_id], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| AppError::database("PYQ evidence concepts could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A PYQ evidence concept is invalid", error))?;
        drop(statement);
        for concept_id in concept_ids {
            mastery::record_evidence_in_transaction(
                &transaction,
                mastery::V2EvidenceRecordInput {
                    concept_id,
                    source_kind: "pyq_attempt".to_owned(),
                    source_id: Some(attempt_id),
                    source_version: 1,
                    exam_id: Some(input.exam_id),
                    local_date: input.attempt_date.clone(),
                    occurred_at: now_text.clone(),
                    raw_value: json!({
                        "status": scored.status,
                        "difficulty": question.10,
                        "selectedAnswer": scored.normalized_answer,
                    }),
                    notes: format!(
                        "PYQ attempt {} for question {}.",
                        attempt_id, input.question_id
                    ),
                },
                mastery::ReplayReason::SourceCreated,
                now,
            )?;
        }
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("The PYQ attempt could not be committed", error))?;
    list_attempts(connection, input.exam_id, Some(input.question_id))?
        .into_iter()
        .find(|attempt| attempt.id == attempt_id)
        .ok_or_else(AppError::pyq_not_found)
}

pub fn preview_import(
    connection: &mut Connection,
    input: &str,
) -> Result<V2PracticeImportPreview, AppError> {
    let headers = question_headers(input);
    let table = read_table(input, headers)?;
    let rows = run_import(connection, &table, false)?;
    Ok(V2PracticeImportPreview {
        preview: preview_from_rows(rows),
        headers: headers.iter().map(|header| (*header).to_owned()).collect(),
        template: csv_template(headers),
    })
}

pub fn apply_import(
    connection: &mut Connection,
    input: &str,
) -> Result<V2PracticeImportResult, AppError> {
    let headers = question_headers(input);
    let table = read_table(input, headers)?;
    let rows = run_import(connection, &table, true)?;
    Ok(V2PracticeImportResult {
        result: result_from_rows(rows),
    })
}

fn run_import(
    connection: &mut Connection,
    table: &crate::csv_import::CsvTable,
    apply: bool,
) -> Result<Vec<ImportRow>, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The PYQ import transaction could not start", error))?;
    let now = Utc::now();
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for record in &table.rows {
        let row_number = record.line;
        let values = match values_by_header(table, record) {
            Ok(values) => values,
            Err(error) => {
                rows.push(ImportRow {
                    row_number,
                    status: ImportRowStatus::Invalid,
                    summary: format!("Row {row_number}"),
                    errors: vec![error],
                });
                continue;
            }
        };
        let input = match question_input_from_csv(&transaction, &values) {
            Ok(input) => input,
            Err(errors) => {
                rows.push(ImportRow {
                    row_number,
                    status: ImportRowStatus::Invalid,
                    summary: format!("Row {row_number}"),
                    errors,
                });
                continue;
            }
        };
        let natural_key = stable_import_key(
            "pyq-row",
            &[
                &input.exam_id.to_string(),
                &input.year.to_string(),
                &input.session_label,
                &input.section_id.to_string(),
                &input.question_number,
            ],
        );
        let dedupe = if input.dedupe_key.trim().is_empty() {
            natural_key.clone()
        } else {
            input.dedupe_key.trim().to_owned()
        };
        if !seen.insert(dedupe.clone()) || question_exists(&transaction, &input)? {
            rows.push(ImportRow {
                row_number,
                status: ImportRowStatus::Duplicate,
                summary: format!(
                    "{} Ã‚Â· {} Ã‚Â· {}",
                    input.year, input.session_label, input.question_number
                ),
                errors: vec![ImportFieldError::new(
                    "dedupeKey",
                    "This question already exists or is duplicated in the file.",
                )],
            });
            continue;
        }
        transaction
            .execute_batch("SAVEPOINT practice_import_row")
            .map_err(|error| AppError::database("PYQ import savepoint failed", error))?;
        let saved = save_question_in_transaction(&transaction, input.clone(), now);
        match saved {
            Ok(_) => {
                transaction
                    .execute_batch("RELEASE practice_import_row")
                    .map_err(|error| AppError::database("PYQ import release failed", error))?;
                rows.push(ImportRow {
                    row_number,
                    status: if apply {
                        ImportRowStatus::Imported
                    } else {
                        ImportRowStatus::Valid
                    },
                    summary: format!(
                        "{} Ã‚Â· {} Ã‚Â· {}",
                        input.year, input.session_label, input.question_number
                    ),
                    errors: Vec::new(),
                });
            }
            Err(error) => {
                transaction
                    .execute_batch("ROLLBACK TO practice_import_row; RELEASE practice_import_row")
                    .map_err(|rollback| {
                        AppError::database("PYQ import rollback failed", rollback)
                    })?;
                rows.push(ImportRow {
                    row_number,
                    status: ImportRowStatus::Invalid,
                    summary: format!(
                        "{} Ã‚Â· {} Ã‚Â· {}",
                        input.year, input.session_label, input.question_number
                    ),
                    errors: vec![ImportFieldError::new("row", error.message)],
                });
            }
        }
    }
    if apply {
        transaction
            .commit()
            .map_err(|error| AppError::database("The PYQ import could not be committed", error))?;
    } else {
        transaction.rollback().map_err(|error| {
            AppError::database("The PYQ import preview could not roll back", error)
        })?;
    }
    Ok(rows)
}

fn question_exists(connection: &Connection, input: &V2PyqQuestionInput) -> Result<bool, AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pyq_questions
                WHERE exam_id = ?1
                  AND (
                    (year = ?2 AND session_label = ?3 AND section_id = ?4 AND question_number = ?5)
                    OR (?6 <> '' AND dedupe_key = ?6)
                  )
             )",
            params![
                input.exam_id,
                input.year,
                input.session_label.trim(),
                input.section_id,
                input.question_number.trim(),
                input.dedupe_key.trim(),
            ],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("PYQ duplicate check failed", error))?;
    Ok(exists == 1)
}

fn question_input_from_csv(
    connection: &Connection,
    values: &std::collections::HashMap<&str, &str>,
) -> Result<V2PyqQuestionInput, Vec<ImportFieldError>> {
    let mut errors = Vec::new();
    let exam_code = required(values, "exam_code", &mut errors);
    let exam_id = match exam_code {
        Some(code) => connection
            .query_row("SELECT id FROM exams WHERE code = ?1", [code], |row| {
                row.get::<_, i64>(0)
            })
            .optional()
            .ok()
            .flatten(),
        None => None,
    };
    if exam_id.is_none() {
        errors.push(ImportFieldError::new(
            "exam_code",
            "Choose one of the four supported examination codes.",
        ));
    }
    let year = parse_i64(values.get("year").copied(), "year", &mut errors);
    let session_label = values
        .get("session_label")
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned();
    let section_code = required(values, "section_code", &mut errors);
    let type_code = required(values, "question_type_code", &mut errors);
    let question_number = required(values, "question_number", &mut errors);
    let statement = values
        .get("statement")
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned();
    let correct_answer = values
        .get("correct_answer")
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned();
    let correct_range = values
        .get("correct_range")
        .copied()
        .unwrap_or("")
        .trim()
        .to_owned();
    let max_marks = parse_f64(values.get("max_marks").copied(), "max_marks", &mut errors);
    let negative_marks = parse_f64_or_default(
        values.get("negative_marks").copied(),
        "negative_marks",
        0.0,
        &mut errors,
    );
    let unanswered_marks = parse_f64_or_default(
        values.get("unanswered_marks").copied(),
        "unanswered_marks",
        0.0,
        &mut errors,
    );
    let difficulty = parse_i64_or_default(
        values.get("difficulty").copied(),
        "difficulty",
        3,
        &mut errors,
    );
    let numeric_tolerance = parse_f64_or_default(
        values.get("numeric_tolerance").copied(),
        "numeric_tolerance",
        0.0,
        &mut errors,
    );
    let answer_key_verified = parse_bool(values.get("answer_key_verified").copied());
    let verification_status = values
        .get("verification_status")
        .copied()
        .unwrap_or("pending")
        .trim()
        .to_lowercase();
    let section_id = match (exam_id, section_code) {
        (Some(exam_id), Some(code)) => connection
            .query_row(
                "SELECT id FROM exam_sections WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
                params![exam_id, code],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten(),
        _ => None,
    };
    if section_id.is_none() {
        errors.push(ImportFieldError::new(
            "section_code",
            "Choose an active section code for the selected examination.",
        ));
    }
    let question_type_id = match (exam_id, type_code) {
        (Some(exam_id), Some(code)) => connection
            .query_row(
                "SELECT id FROM exam_question_types WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
                params![exam_id, code],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten(),
        _ => None,
    };
    if question_type_id.is_none() {
        errors.push(ImportFieldError::new(
            "question_type_code",
            "Choose an active question type code for the selected examination.",
        ));
    }
    let syllabus_node_id = match (exam_id, values.get("syllabus_node_code").copied()) {
        (Some(exam_id), Some(code)) if !code.trim().is_empty() => connection
            .query_row(
                "SELECT id FROM exam_syllabus_nodes
                 WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
                params![exam_id, code.trim()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten(),
        _ => None,
    };
    if values
        .get("syllabus_node_code")
        .is_some_and(|value| !value.trim().is_empty())
        && syllabus_node_id.is_none()
    {
        errors.push(ImportFieldError::new(
            "syllabus_node_code",
            "The syllabus topic code was not found for this examination.",
        ));
    }
    let mut concept_ids = Vec::new();
    if let Some(slugs) = values.get("concept_slugs").copied() {
        for slug in slugs
            .split([',', ';', '|'])
            .map(str::trim)
            .filter(|slug| !slug.is_empty())
        {
            let concept = connection
                .query_row(
                    "SELECT id FROM concepts WHERE slug = ?1 AND is_archived = 0",
                    [slug],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .ok()
                .flatten();
            if let Some(concept) = concept {
                concept_ids.push(concept);
            } else {
                errors.push(ImportFieldError::new(
                    "concept_slugs",
                    format!("Shared concept '{slug}' was not found."),
                ));
            }
        }
    }
    let options = match values
        .get("options_json")
        .copied()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => match serde_json::from_str::<Vec<V2QuestionOption>>(value) {
            Ok(options) => options,
            Err(_) => {
                errors.push(ImportFieldError::new(
                    "options_json",
                    "Answer choices must be a JSON array of key/text objects.",
                ));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    let pattern_version_id = parse_optional_i64(
        values.get("pattern_version_id").copied(),
        "pattern_version_id",
        &mut errors,
    );
    let source_id = parse_optional_i64(values.get("source_id").copied(), "source_id", &mut errors);
    let answer_key_id = parse_optional_i64(
        values.get("answer_key_id").copied(),
        "answer_key_id",
        &mut errors,
    );
    if errors.is_empty() {
        Ok(V2PyqQuestionInput {
            id: None,
            exam_id: exam_id.expect("validated"),
            year: year.expect("validated"),
            session_label,
            section_id: section_id.expect("validated"),
            question_type_id: question_type_id.expect("validated"),
            question_number: question_number.expect("validated").to_owned(),
            statement,
            correct_answer,
            correct_range,
            max_marks: max_marks.expect("validated"),
            negative_marks,
            unanswered_marks,
            difficulty,
            source_title: values
                .get("source_title")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            source_page: values
                .get("source_page")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            source_url: values
                .get("source_url")
                .copied()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            answer_key_source: values
                .get("answer_key_source")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            answer_key_version: values
                .get("answer_key_version")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            answer_key_verified,
            verification_status,
            numeric_tolerance,
            options,
            pattern_version_id,
            syllabus_node_id,
            topic_ids: Vec::new(),
            concept_ids,
            resource_id: None,
            source_id,
            answer_key_id,
            dedupe_key: values
                .get("dedupe_key")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            official_explanation: values
                .get("official_explanation")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            third_party_solution: values
                .get("third_party_solution")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
            user_note: values
                .get("user_note")
                .copied()
                .unwrap_or("")
                .trim()
                .to_owned(),
        })
    } else {
        Err(errors)
    }
}

fn required<'a>(
    values: &'a std::collections::HashMap<&str, &str>,
    field: &'static str,
    errors: &mut Vec<ImportFieldError>,
) -> Option<&'a str> {
    let value = values.get(field).copied().unwrap_or("").trim();
    if value.is_empty() {
        errors.push(ImportFieldError::new(field, "This field is required."));
        None
    } else {
        Some(value)
    }
}

fn parse_optional_i64(
    value: Option<&str>,
    field: &str,
    errors: &mut Vec<ImportFieldError>,
) -> Option<i64> {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        return None;
    }
    value.parse::<i64>().map(Some).unwrap_or_else(|_| {
        errors.push(ImportFieldError::new(field, "Enter a whole number."));
        None
    })
}

fn parse_i64(value: Option<&str>, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<i64> {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        errors.push(ImportFieldError::new(field, "This field is required."));
        return None;
    }
    value.parse::<i64>().map(Some).unwrap_or_else(|_| {
        errors.push(ImportFieldError::new(field, "Enter a whole number."));
        None
    })
}

fn parse_i64_or_default(
    value: Option<&str>,
    field: &str,
    default: i64,
    errors: &mut Vec<ImportFieldError>,
) -> i64 {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        return default;
    }
    value.parse::<i64>().unwrap_or_else(|_| {
        errors.push(ImportFieldError::new(field, "Enter a whole number."));
        default
    })
}

fn parse_f64(value: Option<&str>, field: &str, errors: &mut Vec<ImportFieldError>) -> Option<f64> {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        errors.push(ImportFieldError::new(field, "This field is required."));
        return None;
    }
    value.parse::<f64>().map(Some).unwrap_or_else(|_| {
        errors.push(ImportFieldError::new(field, "Enter a number."));
        None
    })
}

fn parse_f64_or_default(
    value: Option<&str>,
    field: &str,
    default: f64,
    errors: &mut Vec<ImportFieldError>,
) -> f64 {
    let value = value.unwrap_or("").trim();
    if value.is_empty() {
        return default;
    }
    value.parse::<f64>().unwrap_or_else(|_| {
        errors.push(ImportFieldError::new(field, "Enter a number."));
        default
    })
}

fn parse_bool(value: Option<&str>) -> bool {
    matches!(
        value.unwrap_or("").trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y"
    )
}
