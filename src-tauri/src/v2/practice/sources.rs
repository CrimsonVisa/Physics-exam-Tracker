use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

use super::extraction::{ParsedKeyEntry, parse_answer_key, parse_question_paper, review_reasons};
use super::pyq::V2QuestionOption;

const SOURCE_TYPE_QUESTION_PAPER: &str = "question_paper";
const SOURCE_TYPE_ANSWER_KEY: &str = "answer_key";
const SOURCE_TYPE_SUPPORTING: &str = "supporting";

const STATUS_PENDING: &str = "source_pending";
const STATUS_ACQUIRED: &str = "acquired";
const STATUS_VERIFIED: &str = "verified";
const STATUS_NEEDS_REVIEW: &str = "needs_review";
const STATUS_UNAVAILABLE: &str = "unavailable";

const REVIEW_EXTRACTED: &str = "extracted";
const REVIEW_NEEDS_REVIEW: &str = "needs_review";
const REVIEW_VERIFIED: &str = "verified";
const REVIEW_REJECTED: &str = "rejected";

const TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS pyq_sources (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 2100),
    paper_code TEXT NOT NULL CHECK (length(trim(paper_code)) BETWEEN 1 AND 20),
    source_type TEXT NOT NULL CHECK (source_type IN ('question_paper','answer_key','supporting')),
    authority TEXT NOT NULL DEFAULT '' CHECK (length(authority) <= 300),
    institution TEXT NOT NULL DEFAULT '' CHECK (length(institution) <= 300),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 500),
    source_url TEXT CHECK (source_url IS NULL OR source_url LIKE 'https://%'),
    resource_id INTEGER REFERENCES resources(id) ON DELETE SET NULL,
    sha256 TEXT CHECK (
        sha256 IS NULL OR (
            length(sha256) = 64 AND sha256 = lower(sha256)
            AND sha256 NOT GLOB '*[^0-9a-f]*'
        )
    ),
    size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes > 0),
    page_count INTEGER CHECK (page_count IS NULL OR page_count > 0),
    published_at TEXT,
    acquired_at TEXT,
    session_label TEXT NOT NULL DEFAULT '' CHECK (length(session_label) <= 120),
    document_version TEXT NOT NULL DEFAULT '' CHECK (length(document_version) <= 120),
    key_status TEXT NOT NULL DEFAULT '' CHECK (key_status IN ('','provisional','final','revised','withdrawn')),
    verification_status TEXT NOT NULL DEFAULT 'source_pending'
        CHECK (verification_status IN ('source_pending','acquired','verified','needs_review','unavailable')),
    verification_notes TEXT NOT NULL DEFAULT '' CHECK (length(verification_notes) <= 4000),
    associated_source_id INTEGER REFERENCES pyq_sources(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_answer_keys (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES pyq_sources(id) ON DELETE RESTRICT,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 2100),
    paper_code TEXT NOT NULL CHECK (length(trim(paper_code)) BETWEEN 1 AND 20),
    session_label TEXT NOT NULL DEFAULT '' CHECK (length(session_label) <= 120),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) BETWEEN 1 AND 120),
    key_status TEXT NOT NULL DEFAULT 'final'
        CHECK (key_status IN ('provisional','final','revised','withdrawn')),
    published_at TEXT,
    verified_at TEXT,
    verified_by TEXT NOT NULL DEFAULT '' CHECK (length(verified_by) <= 200),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    supersedes_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL,
    entry_count INTEGER NOT NULL DEFAULT 0 CHECK (entry_count >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (source_id, version_label)
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_answer_key_entries (
    id INTEGER PRIMARY KEY,
    answer_key_id INTEGER NOT NULL REFERENCES pyq_answer_keys(id) ON DELETE CASCADE,
    question_number TEXT NOT NULL CHECK (length(trim(question_number)) BETWEEN 1 AND 60),
    session_label TEXT NOT NULL DEFAULT '' CHECK (length(session_label) <= 120),
    question_type_code TEXT NOT NULL CHECK (question_type_code IN ('MCQ','MSQ','NAT')),
    section_code TEXT NOT NULL CHECK (length(section_code) BETWEEN 1 AND 40),
    key_value TEXT NOT NULL CHECK (length(key_value) BETWEEN 1 AND 200),
    marks REAL NOT NULL CHECK (marks > 0),
    key_status TEXT NOT NULL DEFAULT 'answer'
        CHECK (key_status IN ('answer','mta','missing','disputed')),
    official_question_id TEXT NOT NULL DEFAULT '' CHECK (length(official_question_id) <= 120),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 1000),
    created_at TEXT NOT NULL,
    UNIQUE (answer_key_id, question_number)
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_extraction_runs (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES pyq_sources(id) ON DELETE CASCADE,
    answer_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL,
    method TEXT NOT NULL CHECK (method IN ('pdf_text','manual','ocr','assisted')),
    status TEXT NOT NULL DEFAULT 'completed'
        CHECK (status IN ('pending','completed','failed')),
    pages_processed INTEGER NOT NULL DEFAULT 0 CHECK (pages_processed >= 0),
    questions_found INTEGER NOT NULL DEFAULT 0 CHECK (questions_found >= 0),
    questions_reconciled INTEGER NOT NULL DEFAULT 0 CHECK (questions_reconciled >= 0),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_extraction_candidates (
    id INTEGER PRIMARY KEY,
    extraction_run_id INTEGER NOT NULL REFERENCES pyq_extraction_runs(id) ON DELETE CASCADE,
    source_id INTEGER NOT NULL REFERENCES pyq_sources(id) ON DELETE RESTRICT,
    answer_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL,
    answer_key_entry_id INTEGER REFERENCES pyq_answer_key_entries(id) ON DELETE SET NULL,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 2100),
    paper_code TEXT NOT NULL CHECK (length(trim(paper_code)) BETWEEN 1 AND 20),
    session_label TEXT NOT NULL DEFAULT '' CHECK (length(session_label) <= 120),
    page_number INTEGER CHECK (page_number IS NULL OR page_number > 0),
    question_number TEXT NOT NULL CHECK (length(trim(question_number)) BETWEEN 1 AND 60),
    section_code TEXT NOT NULL DEFAULT '' CHECK (length(section_code) <= 40),
    question_type_code TEXT NOT NULL DEFAULT '' CHECK (length(question_type_code) <= 20),
    statement TEXT NOT NULL DEFAULT '' CHECK (length(statement) <= 20000),
    options_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(options_json)),
    correct_answer TEXT NOT NULL DEFAULT '' CHECK (length(correct_answer) <= 4000),
    correct_range TEXT NOT NULL DEFAULT '' CHECK (length(correct_range) <= 200),
    max_marks REAL CHECK (max_marks IS NULL OR max_marks > 0),
    negative_marks REAL NOT NULL DEFAULT 0 CHECK (negative_marks <= 0),
    unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks <= 0),
    numeric_tolerance REAL NOT NULL DEFAULT 0 CHECK (numeric_tolerance >= 0),
    difficulty INTEGER NOT NULL DEFAULT 3 CHECK (difficulty BETWEEN 1 AND 5),
    syllabus_node_id INTEGER REFERENCES exam_syllabus_nodes(id) ON DELETE SET NULL,
    concept_ids_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(concept_ids_json)),
    review_status TEXT NOT NULL DEFAULT 'extracted'
        CHECK (review_status IN ('extracted','needs_review','verified','rejected')),
    review_notes TEXT NOT NULL DEFAULT '' CHECK (length(review_notes) <= 4000),
    reviewed_at TEXT,
    reviewed_by TEXT NOT NULL DEFAULT '' CHECK (length(reviewed_by) <= 200),
    verified_question_id INTEGER REFERENCES pyq_questions(id) ON DELETE SET NULL,
    raw_text TEXT NOT NULL DEFAULT '' CHECK (length(raw_text) <= 50000),
    dedupe_key TEXT NOT NULL DEFAULT '' CHECK (length(dedupe_key) <= 200),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (source_id, session_label, question_number)
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_answer_key_conflicts (
    id INTEGER PRIMARY KEY,
    question_id INTEGER NOT NULL REFERENCES pyq_questions(id) ON DELETE CASCADE,
    old_answer_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL,
    new_answer_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL,
    old_key_value TEXT NOT NULL DEFAULT '' CHECK (length(old_key_value) <= 4000),
    new_key_value TEXT NOT NULL DEFAULT '' CHECK (length(new_key_value) <= 4000),
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open','resolved','dismissed')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    UNIQUE (question_id, new_answer_key_id)
) STRICT;
"#;

const INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_pyq_sources_exam_year
    ON pyq_sources(exam_id, year, source_type, verification_status);
CREATE INDEX IF NOT EXISTS idx_pyq_sources_hash
    ON pyq_sources(sha256);
CREATE INDEX IF NOT EXISTS idx_pyq_answer_keys_exam_year
    ON pyq_answer_keys(exam_id, year, paper_code, key_status);
CREATE INDEX IF NOT EXISTS idx_pyq_answer_key_entries_key
    ON pyq_answer_key_entries(answer_key_id, question_number);
CREATE INDEX IF NOT EXISTS idx_pyq_extraction_candidates_source
    ON pyq_extraction_candidates(source_id, review_status, question_number);
CREATE INDEX IF NOT EXISTS idx_pyq_extraction_candidates_exam
    ON pyq_extraction_candidates(exam_id, year, review_status);
CREATE INDEX IF NOT EXISTS idx_pyq_answer_key_conflicts_question
    ON pyq_answer_key_conflicts(question_id, status);
"#;

pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    connection
        .execute_batch(TABLES)
        .map_err(|error| AppError::database("PYQ source tables could not be prepared", error))?;
    let columns: &[(&str, &str, &str)] = &[
        (
            "pyq_questions",
            "source_id",
            "ALTER TABLE pyq_questions ADD COLUMN source_id INTEGER REFERENCES pyq_sources(id) ON DELETE SET NULL",
        ),
        (
            "pyq_questions",
            "answer_key_id",
            "ALTER TABLE pyq_questions ADD COLUMN answer_key_id INTEGER REFERENCES pyq_answer_keys(id) ON DELETE SET NULL",
        ),
    ];
    for (table, column, sql) in columns {
        if !super::patterns::has_column(connection, table, column)? {
            connection.execute_batch(sql).map_err(|error| {
                AppError::database(
                    &format!("The PYQ source column {table}.{column} could not be added"),
                    error,
                )
            })?;
        }
    }
    connection
        .execute_batch(INDEXES)
        .map_err(|error| AppError::database("PYQ source indexes could not be prepared", error))
}

fn now_text(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn valid_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqSource {
    pub id: i64,
    pub exam_id: i64,
    pub year: i64,
    pub paper_code: String,
    pub source_type: String,
    pub authority: String,
    pub institution: String,
    pub title: String,
    pub source_url: Option<String>,
    pub resource_id: Option<i64>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub page_count: Option<i64>,
    pub published_at: Option<String>,
    pub acquired_at: Option<String>,
    pub session_label: String,
    pub document_version: String,
    pub key_status: String,
    pub verification_status: String,
    pub verification_notes: String,
    pub associated_source_id: Option<i64>,
    pub resource_file_name: Option<String>,
    pub resource_file_path: Option<String>,
    pub resource_file_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqSourceInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub exam_id: i64,
    pub year: i64,
    pub paper_code: String,
    pub source_type: String,
    #[serde(default)]
    pub authority: String,
    #[serde(default)]
    pub institution: String,
    pub title: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub resource_id: Option<i64>,
    #[serde(default)]
    pub page_count: Option<i64>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub session_label: String,
    #[serde(default)]
    pub document_version: String,
    #[serde(default)]
    pub key_status: String,
    #[serde(default)]
    pub verification_status: Option<String>,
    #[serde(default)]
    pub verification_notes: String,
    #[serde(default)]
    pub associated_source_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AcquireSourceInput {
    #[serde(flatten)]
    pub source: V2PyqSourceInput,
    pub file_path: String,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: String,
}

fn default_storage_mode() -> String {
    "managed".to_owned()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqSourceAcquisition {
    pub source: V2PyqSource,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqSourceFilter {
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SourceStatusInput {
    pub source_id: i64,
    pub verification_status: String,
    #[serde(default)]
    pub verification_notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqAnswerKeyEntry {
    pub id: i64,
    pub answer_key_id: i64,
    pub question_number: String,
    pub session_label: String,
    pub question_type_code: String,
    pub section_code: String,
    pub key_value: String,
    pub marks: f64,
    pub key_status: String,
    pub official_question_id: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqAnswerKey {
    pub id: i64,
    pub source_id: i64,
    pub exam_id: i64,
    pub year: i64,
    pub paper_code: String,
    pub session_label: String,
    pub version_label: String,
    pub key_status: String,
    pub published_at: Option<String>,
    pub verified_at: Option<String>,
    pub verified_by: String,
    pub notes: String,
    pub supersedes_key_id: Option<i64>,
    pub entry_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub source_title: String,
    pub source_url: Option<String>,
    #[serde(default)]
    pub entries: Vec<V2PyqAnswerKeyEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PyqAnswerKeyInput {
    pub source_id: i64,
    pub version_label: String,
    #[serde(default = "default_key_status")]
    pub key_status: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub verified_by: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub supersedes_key_id: Option<i64>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub entries: Vec<V2ParsedAnswerKeyEntryInput>,
}

fn default_key_status() -> String {
    "final".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ParsedAnswerKeyEntryInput {
    pub question_number: String,
    #[serde(default)]
    pub session_label: String,
    pub question_type_code: String,
    pub section_code: String,
    pub key_value: String,
    pub marks: f64,
    #[serde(default)]
    pub key_status: String,
    #[serde(default)]
    pub official_question_id: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnswerKeyPreview {
    pub entries: Vec<ParsedKeyEntry>,
    pub missing_question_numbers: Vec<i64>,
    pub duplicate_question_numbers: Vec<String>,
    pub parsed_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionRun {
    pub id: i64,
    pub source_id: i64,
    pub answer_key_id: Option<i64>,
    pub method: String,
    pub status: String,
    pub pages_processed: i64,
    pub questions_found: i64,
    pub questions_reconciled: i64,
    pub notes: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionInput {
    pub source_id: i64,
    pub text: String,
    #[serde(default = "default_extraction_method")]
    pub method: String,
    #[serde(default)]
    pub answer_key_id: Option<i64>,
    #[serde(default)]
    pub notes: String,
}

fn default_extraction_method() -> String {
    "pdf_text".to_owned()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionResult {
    pub run: V2ExtractionRun,
    pub extracted: i64,
    pub clean: i64,
    pub needs_review: i64,
    pub missing_key: i64,
    pub skipped_existing: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionCandidate {
    pub id: i64,
    pub extraction_run_id: i64,
    pub source_id: i64,
    pub source_title: String,
    pub source_url: Option<String>,
    pub answer_key_id: Option<i64>,
    pub answer_key_version: Option<String>,
    pub answer_key_entry_id: Option<i64>,
    pub exam_id: i64,
    pub year: i64,
    pub paper_code: String,
    pub session_label: String,
    pub page_number: Option<i64>,
    pub question_number: String,
    pub section_code: String,
    pub question_type_code: String,
    pub statement: String,
    pub options: Vec<V2QuestionOption>,
    pub correct_answer: String,
    pub correct_range: String,
    pub max_marks: Option<f64>,
    pub negative_marks: f64,
    pub unanswered_marks: f64,
    pub numeric_tolerance: f64,
    pub difficulty: i64,
    pub syllabus_node_id: Option<i64>,
    pub syllabus_node_title: Option<String>,
    pub concept_ids: Vec<i64>,
    pub review_status: String,
    pub review_notes: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by: String,
    pub verified_question_id: Option<i64>,
    pub raw_text: String,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionCandidateFilter {
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub source_id: Option<i64>,
    #[serde(default)]
    pub review_status: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExtractionCandidateUpdate {
    pub candidate_id: i64,
    pub statement: String,
    #[serde(default)]
    pub options: Vec<V2QuestionOption>,
    #[serde(default)]
    pub correct_answer: String,
    #[serde(default)]
    pub correct_range: String,
    pub max_marks: f64,
    #[serde(default)]
    pub negative_marks: f64,
    #[serde(default)]
    pub numeric_tolerance: f64,
    #[serde(default = "default_difficulty")]
    pub difficulty: i64,
    #[serde(default)]
    pub section_code: String,
    #[serde(default)]
    pub question_type_code: String,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    #[serde(default)]
    pub concept_ids: Vec<i64>,
    #[serde(default)]
    pub review_notes: String,
    #[serde(default)]
    pub reviewed_by: String,
}

fn default_difficulty() -> i64 {
    3
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CandidateDecisionInput {
    pub candidate_id: i64,
    #[serde(default)]
    pub reviewed_by: String,
    #[serde(default)]
    pub review_notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnswerKeyConflict {
    pub id: i64,
    pub question_id: i64,
    pub old_answer_key_id: Option<i64>,
    pub new_answer_key_id: Option<i64>,
    pub old_key_value: String,
    pub new_key_value: String,
    pub status: String,
    pub notes: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SourceDetail {
    pub source: V2PyqSource,
    pub answer_keys: Vec<V2PyqAnswerKey>,
    pub extraction_runs: Vec<V2ExtractionRun>,
    pub candidate_counts: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SourceCounts {
    pub sources: i64,
    pub answer_keys: i64,
    pub key_entries: i64,
    pub extraction_runs: i64,
    pub candidates: i64,
    pub candidates_verified: i64,
    pub candidates_pending: i64,
    pub candidates_rejected: i64,
    pub verified_questions: i64,
    pub key_conflicts: i64,
}

fn validate_source_input(
    connection: &Connection,
    input: &V2PyqSourceInput,
) -> Result<(), AppError> {
    if !matches!(
        input.source_type.as_str(),
        SOURCE_TYPE_QUESTION_PAPER | SOURCE_TYPE_ANSWER_KEY | SOURCE_TYPE_SUPPORTING
    ) {
        return Err(AppError::validation(
            "sourceType",
            "Choose question_paper, answer_key or supporting.",
        ));
    }
    if !(1900..=2100).contains(&input.year) {
        return Err(AppError::validation(
            "year",
            "Source year must be between 1900 and 2100.",
        ));
    }
    let paper_code = input.paper_code.trim();
    if paper_code.is_empty() || paper_code.chars().count() > 20 {
        return Err(AppError::validation(
            "paperCode",
            "Paper code must be between 1 and 20 characters.",
        ));
    }
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 500 {
        return Err(AppError::validation(
            "title",
            "Source title must be between 1 and 500 characters.",
        ));
    }
    if input.authority.chars().count() > 300 || input.institution.chars().count() > 300 {
        return Err(AppError::validation(
            "institution",
            "Authority and institution must be 300 characters or fewer.",
        ));
    }
    if let Some(url) = input.source_url.as_deref()
        && !url.trim().is_empty()
        && !url.trim().starts_with("https://")
    {
        return Err(AppError::validation(
            "sourceUrl",
            "Source URL must begin with https://.",
        ));
    }
    if input.page_count.is_some_and(|pages| pages <= 0) {
        return Err(AppError::validation(
            "pageCount",
            "Page count must be positive.",
        ));
    }
    if let Some(published) = input.published_at.as_deref()
        && !published.trim().is_empty()
        && !valid_date(published.trim())
    {
        return Err(AppError::validation(
            "publishedAt",
            "Publication date must use YYYY-MM-DD form.",
        ));
    }
    if input.session_label.chars().count() > 120 {
        return Err(AppError::validation(
            "sessionLabel",
            "Session label must be 120 characters or fewer.",
        ));
    }
    if input.document_version.chars().count() > 120 {
        return Err(AppError::validation(
            "documentVersion",
            "Document version must be 120 characters or fewer.",
        ));
    }
    if !matches!(
        input.key_status.trim(),
        "" | "provisional" | "final" | "revised" | "withdrawn"
    ) {
        return Err(AppError::validation(
            "keyStatus",
            "Choose provisional, final, revised or withdrawn.",
        ));
    }
    if input.source_type != SOURCE_TYPE_ANSWER_KEY && !input.key_status.trim().is_empty() {
        return Err(AppError::validation(
            "keyStatus",
            "Only answer-key sources can carry a key status.",
        ));
    }
    if let Some(status) = input.verification_status.as_deref()
        && !matches!(
            status,
            STATUS_PENDING
                | STATUS_ACQUIRED
                | STATUS_VERIFIED
                | STATUS_NEEDS_REVIEW
                | STATUS_UNAVAILABLE
        )
    {
        return Err(AppError::validation(
            "verificationStatus",
            "Choose source_pending, acquired, verified, needs_review or unavailable.",
        ));
    }
    let exam_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
            [input.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Source examination could not be checked", error))?;
    if exam_exists == 0 {
        return Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        ));
    }
    if let Some(associated) = input.associated_source_id {
        let parent: Option<(i64, i64)> = connection
            .query_row(
                "SELECT exam_id, year FROM pyq_sources WHERE id = ?1",
                [associated],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| AppError::database("Associated source could not be checked", error))?;
        match parent {
            Some((exam_id, year)) if exam_id == input.exam_id && year == input.year => {}
            Some(_) => {
                return Err(AppError::validation(
                    "associatedSourceId",
                    "The associated source must belong to the same examination and year.",
                ));
            }
            None => {
                return Err(AppError::validation(
                    "associatedSourceId",
                    "Choose an existing associated source.",
                ));
            }
        }
    }
    Ok(())
}

fn map_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2PyqSource> {
    Ok(V2PyqSource {
        id: row.get(0)?,
        exam_id: row.get(1)?,
        year: row.get(2)?,
        paper_code: row.get(3)?,
        source_type: row.get(4)?,
        authority: row.get(5)?,
        institution: row.get(6)?,
        title: row.get(7)?,
        source_url: row.get(8)?,
        resource_id: row.get(9)?,
        sha256: row.get(10)?,
        size_bytes: row.get(11)?,
        page_count: row.get(12)?,
        published_at: row.get(13)?,
        acquired_at: row.get(14)?,
        session_label: row.get(15)?,
        document_version: row.get(16)?,
        key_status: row.get(17)?,
        verification_status: row.get(18)?,
        verification_notes: row.get(19)?,
        associated_source_id: row.get(20)?,
        resource_file_name: row.get(21)?,
        resource_file_path: row.get(22)?,
        resource_file_status: row.get(23)?,
        created_at: row.get(24)?,
        updated_at: row.get(25)?,
    })
}

fn source_select() -> &'static str {
    "SELECT s.id, s.exam_id, s.year, s.paper_code, s.source_type, s.authority,
            s.institution, s.title, s.source_url, s.resource_id, s.sha256, s.size_bytes,
            s.page_count, s.published_at, s.acquired_at, s.session_label,
            s.document_version, s.key_status, s.verification_status, s.verification_notes,
            s.associated_source_id, rf.file_name, rf.file_path, rf.status,
            s.created_at, s.updated_at
     FROM pyq_sources s
     LEFT JOIN resource_files rf ON rf.resource_id = s.resource_id"
}

pub fn list_sources(
    connection: &Connection,
    filter: V2PyqSourceFilter,
) -> Result<Vec<V2PyqSource>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "{} WHERE (?1 IS NULL OR s.exam_id = ?1)
                 AND (?2 IS NULL OR s.year = ?2)
                 AND (?3 IS NULL OR s.source_type = ?3)
             ORDER BY s.year DESC, s.source_type, s.id",
            source_select()
        ))
        .map_err(|error| AppError::database("PYQ sources could not be prepared", error))?;
    let rows = statement
        .query_map(
            params![filter.exam_id, filter.year, filter.source_type],
            map_source,
        )
        .map_err(|error| AppError::database("PYQ sources could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A PYQ source record is invalid", error))?;
    Ok(rows)
}

pub fn get_source(connection: &Connection, source_id: i64) -> Result<V2PyqSource, AppError> {
    connection
        .query_row(
            &format!("{} WHERE s.id = ?1", source_select()),
            [source_id],
            map_source,
        )
        .optional()
        .map_err(|error| AppError::database("PYQ source could not be read", error))?
        .ok_or_else(|| AppError::validation("sourceId", "This PYQ source is no longer available."))
}

fn resource_file_metadata(
    connection: &Connection,
    resource_id: i64,
) -> Result<(String, i64), AppError> {
    connection
        .query_row(
            "SELECT sha256, size_bytes FROM resource_files WHERE resource_id = ?1",
            [resource_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Managed source file could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "resourceId",
                "Import the PDF into this resource before linking it to a source.",
            )
        })
}

pub fn save_source(
    connection: &mut Connection,
    input: V2PyqSourceInput,
    now: DateTime<Utc>,
) -> Result<V2PyqSource, AppError> {
    validate_source_input(connection, &input)?;
    let resource_metadata = match input.resource_id {
        Some(resource_id) => {
            let active: i64 = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM resources WHERE id = ?1 AND is_archived = 0)",
                    [resource_id],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    AppError::database("Source resource could not be checked", error)
                })?;
            if active == 0 {
                return Err(AppError::validation(
                    "resourceId",
                    "Choose an active managed resource.",
                ));
            }
            Some(resource_file_metadata(connection, resource_id)?)
        }
        None => None,
    };
    let requested_status = input
        .verification_status
        .as_deref()
        .unwrap_or(STATUS_PENDING)
        .trim()
        .to_owned();
    let mut status = requested_status;
    if status == STATUS_VERIFIED && resource_metadata.is_none() {
        status = STATUS_NEEDS_REVIEW.to_owned();
    }
    if matches!(status.as_str(), STATUS_ACQUIRED | STATUS_VERIFIED) && resource_metadata.is_none() {
        status = STATUS_NEEDS_REVIEW.to_owned();
    }
    if input.source_type == SOURCE_TYPE_ANSWER_KEY
        && status == STATUS_VERIFIED
        && let Some(id) = input.id
    {
        let verified_key: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pyq_answer_keys
                    WHERE source_id = ?1 AND verified_at IS NOT NULL
                 )",
                [id],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("Answer-key source state could not be checked", error)
            })?;
        if verified_key == 0 {
            return Err(AppError::validation(
                "verificationStatus",
                "Save and verify the answer key before marking its source verified.",
            ));
        }
    }
    let now_text = now_text(now);
    let source_url = input
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let published_at = input
        .published_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let id = if let Some(id) = input.id {
        let existing: Option<(Option<i64>, Option<String>)> = connection
            .query_row(
                "SELECT resource_id, acquired_at FROM pyq_sources WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| AppError::database("PYQ source could not be checked", error))?;
        let Some((existing_resource, existing_acquired)) = existing else {
            return Err(AppError::validation(
                "sourceId",
                "This PYQ source is no longer available.",
            ));
        };
        let resource_id = input.resource_id.or(existing_resource);
        let acquired_at = if resource_id.is_some() {
            existing_acquired.or_else(|| Some(now_text.clone()))
        } else {
            None
        };
        connection
            .execute(
                "UPDATE pyq_sources SET
                    exam_id = ?1, year = ?2, paper_code = ?3, source_type = ?4,
                    authority = ?5, institution = ?6, title = ?7, source_url = ?8,
                    resource_id = ?9, sha256 = ?10, size_bytes = ?11, page_count = ?12,
                    published_at = ?13, acquired_at = ?14, session_label = ?15,
                    document_version = ?16, key_status = ?17, verification_status = ?18,
                    verification_notes = ?19, associated_source_id = ?20, updated_at = ?21
                 WHERE id = ?22",
                params![
                    input.exam_id,
                    input.year,
                    input.paper_code.trim(),
                    input.source_type,
                    input.authority.trim(),
                    input.institution.trim(),
                    input.title.trim(),
                    source_url,
                    resource_id,
                    resource_metadata.as_ref().map(|value| &value.0),
                    resource_metadata.as_ref().map(|value| value.1),
                    input.page_count,
                    published_at,
                    acquired_at,
                    input.session_label.trim(),
                    input.document_version.trim(),
                    input.key_status.trim(),
                    status,
                    input.verification_notes.trim(),
                    input.associated_source_id,
                    now_text,
                    id,
                ],
            )
            .map_err(|error| AppError::database("PYQ source could not be updated", error))?;
        id
    } else {
        connection
            .execute(
                "INSERT INTO pyq_sources(
                    exam_id, year, paper_code, source_type, authority, institution, title,
                    source_url, resource_id, sha256, size_bytes, page_count, published_at,
                    acquired_at, session_label, document_version, key_status,
                    verification_status, verification_notes, associated_source_id,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                           ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?21)",
                params![
                    input.exam_id,
                    input.year,
                    input.paper_code.trim(),
                    input.source_type,
                    input.authority.trim(),
                    input.institution.trim(),
                    input.title.trim(),
                    source_url,
                    input.resource_id,
                    resource_metadata.as_ref().map(|value| &value.0),
                    resource_metadata.as_ref().map(|value| value.1),
                    input.page_count,
                    published_at,
                    if input.resource_id.is_some() {
                        Some(now_text.clone())
                    } else {
                        None
                    },
                    input.session_label.trim(),
                    input.document_version.trim(),
                    input.key_status.trim(),
                    status,
                    input.verification_notes.trim(),
                    input.associated_source_id,
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("PYQ source could not be stored", error))?;
        connection.last_insert_rowid()
    };
    get_source(connection, id)
}

pub fn acquire_source(
    connection: &mut Connection,
    data_dir: &Path,
    input: V2AcquireSourceInput,
    now: DateTime<Utc>,
) -> Result<V2PyqSourceAcquisition, AppError> {
    validate_source_input(connection, &input.source)?;
    let path = Path::new(input.file_path.trim());
    if !path.is_file() {
        return Err(AppError::validation(
            "filePath",
            "Choose an existing PDF file.",
        ));
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
    {
        return Err(AppError::validation(
            "filePath",
            "Only PDF files are supported.",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| AppError::file_operation("The official PDF could not be read", error))?;
    if bytes.is_empty() {
        return Err(AppError::validation("filePath", "The PDF file is empty."));
    }
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());
    let duplicate: Option<i64> = connection
        .query_row(
            "SELECT id FROM pyq_sources WHERE sha256 = ?1 LIMIT 1",
            [&sha256],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Duplicate source hash could not be checked", error))?;
    if let Some(id) = duplicate {
        return Ok(V2PyqSourceAcquisition {
            source: get_source(connection, id)?,
            duplicate: true,
        });
    }
    let identity: Option<i64> = connection
        .query_row(
            "SELECT id FROM pyq_sources
             WHERE exam_id = ?1 AND year = ?2 AND paper_code = ?3 AND source_type = ?4
               AND COALESCE(source_url, '') = COALESCE(?5, '')
               AND document_version = ?6
             LIMIT 1",
            params![
                input.source.exam_id,
                input.source.year,
                input.source.paper_code.trim(),
                input.source.source_type,
                input
                    .source
                    .source_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                input.source.document_version.trim(),
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("Existing source identity could not be checked", error)
        })?;
    if let Some(id) = identity {
        let existing = get_source(connection, id)?;
        if existing.resource_id.is_some() {
            return Ok(V2PyqSourceAcquisition {
                source: existing,
                duplicate: true,
            });
        }
    }
    let resource_id = if let Some(existing_id) = identity {
        let existing = get_source(connection, existing_id)?;
        existing.resource_id
    } else {
        None
    };
    let resource_id = if let Some(resource_id) = resource_id {
        resource_id
    } else {
        let source_url = input
            .source
            .source_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let existing_resource: Option<i64> = if let Some(url) = source_url {
            connection
                .query_row("SELECT id FROM resources WHERE url = ?1", [url], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(|error| {
                    AppError::database("Source resource could not be checked", error)
                })?
        } else {
            None
        };
        let resource_id = if let Some(resource_id) = existing_resource {
            resource_id
        } else {
            crate::v2::resources::save_resource(
                connection,
                crate::v2::resources::V2ResourceInput {
                    id: None,
                    title: input.source.title.trim().to_owned(),
                    description: format!(
                        "Official {} for GATE Physics (PH) {}.",
                        if input.source.source_type == SOURCE_TYPE_ANSWER_KEY {
                            "final answer key"
                        } else {
                            "question paper"
                        },
                        input.source.year
                    ),
                    subject_area: "Physics".to_owned(),
                    url: source_url.map(str::to_owned),
                    resource_type: "paper".to_owned(),
                    source: format!(
                        "{} - {}",
                        input.source.authority.trim(),
                        input.source.institution.trim()
                    )
                    .trim_matches(' ')
                    .trim_matches('-')
                    .trim()
                    .to_owned(),
                    concept_ids: Vec::new(),
                    exam_links: vec![crate::v2::resources::V2ResourceExamLinkInput {
                        exam_id: input.source.exam_id,
                        category: "Official PYQ source".to_owned(),
                        syllabus_node_id: None,
                        priority: "high".to_owned(),
                        notes: "Registered official GATE Physics document.".to_owned(),
                    }],
                },
                now,
            )?
            .id
        };
        crate::v2::resources::import_resource_file(
            connection,
            data_dir,
            crate::v2::resources::V2ResourceFileInput {
                resource_id,
                file_path: path.display().to_string(),
                storage_mode: input.storage_mode.clone(),
            },
            now,
        )?;
        resource_id
    };
    let mut source_input = input.source;
    source_input.id = identity;
    source_input.resource_id = Some(resource_id);
    if source_input.verification_status.as_deref() == Some(STATUS_VERIFIED) {
        source_input.verification_status = Some(STATUS_NEEDS_REVIEW.to_owned());
    } else if source_input.verification_status.is_none() {
        source_input.verification_status = Some(STATUS_ACQUIRED.to_owned());
    }
    let source = save_source(connection, source_input, now)?;
    Ok(V2PyqSourceAcquisition {
        source,
        duplicate: false,
    })
}

pub fn set_source_status(
    connection: &mut Connection,
    input: V2SourceStatusInput,
    now: DateTime<Utc>,
) -> Result<V2PyqSource, AppError> {
    if !matches!(
        input.verification_status.as_str(),
        STATUS_PENDING
            | STATUS_ACQUIRED
            | STATUS_VERIFIED
            | STATUS_NEEDS_REVIEW
            | STATUS_UNAVAILABLE
    ) {
        return Err(AppError::validation(
            "verificationStatus",
            "Choose source_pending, acquired, verified, needs_review or unavailable.",
        ));
    }
    let source = get_source(connection, input.source_id)?;
    if input.verification_status == STATUS_VERIFIED {
        if source.resource_id.is_none() || source.sha256.is_none() {
            return Err(AppError::validation(
                "verificationStatus",
                "Attach the managed PDF and record its hash before verifying the source.",
            ));
        }
        if source.source_type == SOURCE_TYPE_ANSWER_KEY {
            let verified_key: i64 = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM pyq_answer_keys
                        WHERE source_id = ?1 AND verified_at IS NOT NULL
                     )",
                    [input.source_id],
                    |row| row.get(0),
                )
                .map_err(|error| AppError::database("Answer key could not be checked", error))?;
            if verified_key == 0 {
                return Err(AppError::validation(
                    "verificationStatus",
                    "Import and verify the answer key before verifying its source.",
                ));
            }
        }
    }
    let notes = if input.verification_notes.trim().is_empty() {
        source.verification_notes
    } else {
        input.verification_notes.trim().to_owned()
    };
    connection
        .execute(
            "UPDATE pyq_sources SET verification_status = ?1, verification_notes = ?2,
                updated_at = ?3 WHERE id = ?4",
            params![
                input.verification_status,
                notes,
                now_text(now),
                input.source_id
            ],
        )
        .map_err(|error| AppError::database("PYQ source status could not be updated", error))?;
    get_source(connection, input.source_id)
}

pub fn preview_answer_key_text(text: &str) -> V2AnswerKeyPreview {
    let entries = parse_answer_key(text);
    let present = entries
        .iter()
        .filter_map(|entry| entry.question_number.parse::<i64>().ok())
        .collect::<BTreeSet<_>>();
    let missing_question_numbers = (1..=65)
        .filter(|number| !present.contains(number))
        .collect::<Vec<_>>();
    V2AnswerKeyPreview {
        parsed_count: entries.len() as i64,
        entries,
        missing_question_numbers,
        duplicate_question_numbers: Vec::new(),
    }
}

fn map_answer_key(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2PyqAnswerKey> {
    Ok(V2PyqAnswerKey {
        id: row.get(0)?,
        source_id: row.get(1)?,
        exam_id: row.get(2)?,
        year: row.get(3)?,
        paper_code: row.get(4)?,
        session_label: row.get(5)?,
        version_label: row.get(6)?,
        key_status: row.get(7)?,
        published_at: row.get(8)?,
        verified_at: row.get(9)?,
        verified_by: row.get(10)?,
        notes: row.get(11)?,
        supersedes_key_id: row.get(12)?,
        entry_count: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        source_title: row.get(16)?,
        source_url: row.get(17)?,
        entries: Vec::new(),
    })
}

fn answer_key_select() -> &'static str {
    "SELECT k.id, k.source_id, k.exam_id, k.year, k.paper_code, k.session_label,
            k.version_label, k.key_status, k.published_at, k.verified_at, k.verified_by,
            k.notes, k.supersedes_key_id, k.entry_count, k.created_at, k.updated_at,
            s.title, s.source_url
     FROM pyq_answer_keys k
     JOIN pyq_sources s ON s.id = k.source_id"
}

pub fn list_answer_keys(
    connection: &Connection,
    exam_id: Option<i64>,
    year: Option<i64>,
) -> Result<Vec<V2PyqAnswerKey>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "{} WHERE (?1 IS NULL OR k.exam_id = ?1) AND (?2 IS NULL OR k.year = ?2)
             ORDER BY k.year DESC, k.id DESC",
            answer_key_select()
        ))
        .map_err(|error| AppError::database("Answer keys could not be prepared", error))?;
    let rows = statement
        .query_map(params![exam_id, year], map_answer_key)
        .map_err(|error| AppError::database("Answer keys could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An answer key record is invalid", error))?;
    Ok(rows)
}

pub fn get_answer_key(
    connection: &Connection,
    answer_key_id: i64,
) -> Result<V2PyqAnswerKey, AppError> {
    let mut key = connection
        .query_row(
            &format!("{} WHERE k.id = ?1", answer_key_select()),
            [answer_key_id],
            map_answer_key,
        )
        .optional()
        .map_err(|error| AppError::database("Answer key could not be read", error))?
        .ok_or_else(|| {
            AppError::validation("answerKeyId", "This answer key is no longer available.")
        })?;
    let mut statement = connection
        .prepare(
            "SELECT id, answer_key_id, question_number, session_label, question_type_code,
                    section_code, key_value, marks, key_status, official_question_id, notes
             FROM pyq_answer_key_entries WHERE answer_key_id = ?1 ORDER BY question_number + 0, question_number",
        )
        .map_err(|error| AppError::database("Answer-key entries could not be prepared", error))?;
    key.entries = statement
        .query_map([answer_key_id], |row| {
            Ok(V2PyqAnswerKeyEntry {
                id: row.get(0)?,
                answer_key_id: row.get(1)?,
                question_number: row.get(2)?,
                session_label: row.get(3)?,
                question_type_code: row.get(4)?,
                section_code: row.get(5)?,
                key_value: row.get(6)?,
                marks: row.get(7)?,
                key_status: row.get(8)?,
                official_question_id: row.get(9)?,
                notes: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database("Answer-key entries could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An answer-key entry is invalid", error))?;
    Ok(key)
}

fn insert_key_conflicts(
    connection: &Connection,
    source_id: i64,
    old_key_id: i64,
    new_key_id: i64,
    now: &str,
) -> Result<i64, AppError> {
    let old_entries = {
        let mut statement = connection
            .prepare(
                "SELECT question_number, key_value, marks, question_type_code, section_code
                 FROM pyq_answer_key_entries WHERE answer_key_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Old answer-key entries could not be prepared", error)
            })?;
        statement
            .query_map([old_key_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| AppError::database("Old answer-key entries could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("An old answer-key entry is invalid", error))?
    };
    let new_entries = {
        let mut statement = connection
            .prepare(
                "SELECT question_number, key_value, marks, question_type_code, section_code
                 FROM pyq_answer_key_entries WHERE answer_key_id = ?1",
            )
            .map_err(|error| {
                AppError::database("New answer-key entries could not be prepared", error)
            })?;
        statement
            .query_map([new_key_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| AppError::database("New answer-key entries could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A new answer-key entry is invalid", error))?
    };
    let new_by_number = new_entries
        .into_iter()
        .map(|entry| (entry.0.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut conflicts = 0;
    for (question_number, old_value, old_marks, old_type, old_section) in old_entries {
        let Some((_, new_value, new_marks, new_type, new_section)) =
            new_by_number.get(&question_number)
        else {
            continue;
        };
        if old_value == *new_value
            && (old_marks - *new_marks).abs() < 1e-9
            && old_type == *new_type
            && old_section == *new_section
        {
            continue;
        }
        let mut statement = connection
            .prepare(
                "SELECT id FROM pyq_questions
                 WHERE source_id = ?1 AND question_number = ?2 AND is_archived = 0",
            )
            .map_err(|error| {
                AppError::database("Key-conflict questions could not be prepared", error)
            })?;
        let questions = statement
            .query_map(params![source_id, question_number], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| AppError::database("Key-conflict questions could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A key-conflict question is invalid", error))?;
        for question_id in questions {
            connection
                .execute(
                    "INSERT OR IGNORE INTO pyq_answer_key_conflicts(
                        question_id, old_answer_key_id, new_answer_key_id, old_key_value,
                        new_key_value, status, notes, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 'open', ?6, ?7)",
                    params![
                        question_id,
                        old_key_id,
                        new_key_id,
                        old_value,
                        new_value,
                        format!(
                            "Revised answer key changes {old_type}/{old_section} Q{question_number} from {old_value} to {new_value}."
                        ),
                        now,
                    ],
                )
                .map_err(|error| AppError::database("Answer-key conflict could not be stored", error))?;
            conflicts += 1;
        }
    }
    Ok(conflicts)
}

#[allow(clippy::too_many_lines)]
pub fn save_answer_key(
    connection: &mut Connection,
    input: V2PyqAnswerKeyInput,
    now: DateTime<Utc>,
) -> Result<V2PyqAnswerKey, AppError> {
    let source = get_source(connection, input.source_id)?;
    if source.source_type != SOURCE_TYPE_ANSWER_KEY {
        return Err(AppError::validation(
            "sourceId",
            "Choose a source registered as an answer key.",
        ));
    }
    let version_label = input.version_label.trim();
    if version_label.is_empty() || version_label.chars().count() > 120 {
        return Err(AppError::validation(
            "versionLabel",
            "Key version label must be between 1 and 120 characters.",
        ));
    }
    if !matches!(
        input.key_status.as_str(),
        "provisional" | "final" | "revised" | "withdrawn"
    ) {
        return Err(AppError::validation(
            "keyStatus",
            "Choose provisional, final, revised or withdrawn.",
        ));
    }
    if let Some(published) = input.published_at.as_deref()
        && !published.trim().is_empty()
        && !valid_date(published.trim())
    {
        return Err(AppError::validation(
            "publishedAt",
            "Publication date must use YYYY-MM-DD form.",
        ));
    }
    if let Some(supersedes) = input.supersedes_key_id {
        let valid: Option<(i64, i64)> = connection
            .query_row(
                "SELECT exam_id, year FROM pyq_answer_keys WHERE id = ?1",
                [supersedes],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("Superseded answer key could not be checked", error)
            })?;
        match valid {
            Some((exam_id, year)) if exam_id == source.exam_id && year == source.year => {}
            Some(_) => {
                return Err(AppError::validation(
                    "supersedesKeyId",
                    "The superseded key must belong to the same examination and year.",
                ));
            }
            None => {
                return Err(AppError::validation(
                    "supersedesKeyId",
                    "Choose an existing answer key to supersede.",
                ));
            }
        }
    }
    let entries = if input.text.trim().is_empty() {
        input
            .entries
            .iter()
            .map(|entry| ParsedKeyEntry {
                question_number: entry.question_number.clone(),
                session_label: entry.session_label.clone(),
                question_type_code: entry.question_type_code.to_ascii_uppercase(),
                section_code: entry.section_code.to_ascii_uppercase(),
                key_value: entry.key_value.trim().to_owned(),
                marks: entry.marks,
                key_status: if entry.key_status.trim().is_empty() {
                    "answer".to_owned()
                } else {
                    entry.key_status.trim().to_owned()
                },
            })
            .collect::<Vec<_>>()
    } else {
        parse_answer_key(&input.text)
    };
    if entries.is_empty() {
        return Err(AppError::validation(
            "text",
            "The answer key did not produce any question entries.",
        ));
    }
    let mut seen = HashSet::new();
    for entry in &entries {
        let number = entry.question_number.trim();
        if number.is_empty() || number.chars().count() > 60 {
            return Err(AppError::validation(
                "text",
                "Every answer-key entry needs a question number.",
            ));
        }
        if !seen.insert(number.to_owned()) {
            return Err(AppError::validation(
                "text",
                format!("Question {number} appears more than once in the answer key."),
            ));
        }
        if !matches!(entry.question_type_code.as_str(), "MCQ" | "MSQ" | "NAT") {
            return Err(AppError::validation(
                "text",
                format!("Question {number} has an unsupported question type."),
            ));
        }
        if !matches!(entry.section_code.as_str(), "GA" | "PH") {
            return Err(AppError::validation(
                "text",
                format!("Question {number} has an unsupported section."),
            ));
        }
        if entry.key_value.trim().is_empty() {
            return Err(AppError::validation(
                "text",
                format!("Question {number} has an empty key value."),
            ));
        }
        if entry.marks != 1.0 && entry.marks != 2.0 {
            return Err(AppError::validation(
                "text",
                format!("Question {number} has an unexpected mark value."),
            ));
        }
        if !matches!(
            entry.key_status.as_str(),
            "answer" | "mta" | "missing" | "disputed"
        ) {
            return Err(AppError::validation(
                "text",
                format!("Question {number} has an unsupported key status."),
            ));
        }
    }
    if input.verified && entries.len() < 65 {
        return Err(AppError::validation(
            "verified",
            "A final answer key must contain all 65 question entries.",
        ));
    }
    let now_text = now_text(now);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("Answer-key transaction could not start", error))?;
    let key_id = {
        transaction
            .execute(
                "INSERT INTO pyq_answer_keys(
                source_id, exam_id, year, paper_code, session_label, version_label,
                key_status, published_at, verified_at, verified_by, notes,
                supersedes_key_id, entry_count, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
                params![
                    source.id,
                    source.exam_id,
                    source.year,
                    source.paper_code,
                    source.session_label,
                    version_label,
                    input.key_status,
                    input
                        .published_at
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty()),
                    if input.verified {
                        Some(now_text.clone())
                    } else {
                        None
                    },
                    input.verified_by.trim(),
                    input.notes.trim(),
                    input.supersedes_key_id,
                    entries.len() as i64,
                    now_text,
                ],
            )
            .map_err(|error| {
                if matches!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::ConstraintViolation)
                ) {
                    AppError::validation(
                        "versionLabel",
                        "This answer-key version already exists for the source.",
                    )
                } else {
                    AppError::database("Answer key could not be stored", error)
                }
            })?;
        transaction.last_insert_rowid()
    };
    for entry in &entries {
        transaction
            .execute(
                "INSERT INTO pyq_answer_key_entries(
                    answer_key_id, question_number, session_label, question_type_code,
                    section_code, key_value, marks, key_status, official_question_id,
                    notes, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    key_id,
                    entry.question_number.trim(),
                    entry.session_label.trim(),
                    entry.question_type_code,
                    entry.section_code,
                    entry.key_value.trim(),
                    entry.marks,
                    entry.key_status,
                    input
                        .entries
                        .iter()
                        .find(|candidate| candidate.question_number.trim()
                            == entry.question_number.trim())
                        .map(|candidate| candidate.official_question_id.trim())
                        .unwrap_or(""),
                    input
                        .entries
                        .iter()
                        .find(|candidate| candidate.question_number.trim()
                            == entry.question_number.trim())
                        .map(|candidate| candidate.notes.trim())
                        .unwrap_or(""),
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Answer-key entry could not be stored", error))?;
    }
    let _ = if let Some(old_key_id) = input.supersedes_key_id {
        let paper_source_id = source.associated_source_id.unwrap_or(source.id);
        insert_key_conflicts(&transaction, paper_source_id, old_key_id, key_id, &now_text)?
    } else {
        0
    };
    if input.verified && source.resource_id.is_some() && source.sha256.is_some() {
        transaction
            .execute(
                "UPDATE pyq_sources SET verification_status = 'verified', updated_at = ?1
                 WHERE id = ?2",
                params![now_text, source.id],
            )
            .map_err(|error| {
                AppError::database("Answer-key source could not be verified", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Answer key could not be committed", error))?;
    get_answer_key(connection, key_id)
}

pub fn list_answer_key_conflicts(
    connection: &Connection,
    exam_id: Option<i64>,
) -> Result<Vec<V2AnswerKeyConflict>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT c.id, c.question_id, c.old_answer_key_id, c.new_answer_key_id,
                    c.old_key_value, c.new_key_value, c.status, c.notes, c.created_at,
                    c.resolved_at
             FROM pyq_answer_key_conflicts c
             JOIN pyq_questions q ON q.id = c.question_id
             WHERE (?1 IS NULL OR q.exam_id = ?1)
             ORDER BY c.created_at DESC, c.id DESC",
        )
        .map_err(|error| AppError::database("Answer-key conflicts could not be prepared", error))?;
    statement
        .query_map([exam_id], |row| {
            Ok(V2AnswerKeyConflict {
                id: row.get(0)?,
                question_id: row.get(1)?,
                old_answer_key_id: row.get(2)?,
                new_answer_key_id: row.get(3)?,
                old_key_value: row.get(4)?,
                new_key_value: row.get(5)?,
                status: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                resolved_at: row.get(9)?,
            })
        })
        .map_err(|error| AppError::database("Answer-key conflicts could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An answer-key conflict is invalid", error))
}

fn official_section_code(connection: &Connection, exam_id: i64, official: &str) -> String {
    let candidate = match official {
        "GA" => "gate-ga".to_owned(),
        "PH" => "gate-ph".to_owned(),
        _ => official.to_ascii_lowercase(),
    };
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exam_sections WHERE exam_id = ?1 AND code = ?2)",
            params![exam_id, candidate],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if exists == 1 {
        candidate
    } else {
        official.to_ascii_lowercase()
    }
}

fn key_entry_map(
    connection: &Connection,
    answer_key_id: Option<i64>,
) -> Result<BTreeMap<String, V2PyqAnswerKeyEntry>, AppError> {
    let Some(answer_key_id) = answer_key_id else {
        return Ok(BTreeMap::new());
    };
    let key = get_answer_key(connection, answer_key_id)?;
    Ok(key
        .entries
        .into_iter()
        .map(|entry| (entry.question_number.clone(), entry))
        .collect())
}

fn parsed_from_entry(entry: &V2PyqAnswerKeyEntry) -> ParsedKeyEntry {
    ParsedKeyEntry {
        question_number: entry.question_number.clone(),
        session_label: entry.session_label.clone(),
        question_type_code: entry.question_type_code.clone(),
        section_code: entry.section_code.clone(),
        key_value: entry.key_value.clone(),
        marks: entry.marks,
        key_status: entry.key_status.clone(),
    }
}

fn candidate_negative_marks(question_type: &str, marks: f64) -> f64 {
    if question_type.eq_ignore_ascii_case("MCQ") {
        -(marks / 3.0)
    } else {
        0.0
    }
}

fn map_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2ExtractionCandidate> {
    let options_json: String = row.get(17)?;
    let concept_ids_json: String = row.get(27)?;
    Ok(V2ExtractionCandidate {
        id: row.get(0)?,
        extraction_run_id: row.get(1)?,
        source_id: row.get(2)?,
        source_title: row.get(3)?,
        source_url: row.get(4)?,
        answer_key_id: row.get(5)?,
        answer_key_version: row.get(6)?,
        answer_key_entry_id: row.get(7)?,
        exam_id: row.get(8)?,
        year: row.get(9)?,
        paper_code: row.get(10)?,
        session_label: row.get(11)?,
        page_number: row.get(12)?,
        question_number: row.get(13)?,
        section_code: row.get(14)?,
        question_type_code: row.get(15)?,
        statement: row.get(16)?,
        options: serde_json::from_str(&options_json).unwrap_or_default(),
        correct_answer: row.get(18)?,
        correct_range: row.get(19)?,
        max_marks: row.get(20)?,
        negative_marks: row.get(21)?,
        unanswered_marks: row.get(22)?,
        numeric_tolerance: row.get(23)?,
        difficulty: row.get(24)?,
        syllabus_node_id: row.get(25)?,
        syllabus_node_title: row.get(26)?,
        concept_ids: serde_json::from_str(&concept_ids_json).unwrap_or_default(),
        review_status: row.get(28)?,
        review_notes: row.get(29)?,
        reviewed_at: row.get(30)?,
        reviewed_by: row.get(31)?,
        verified_question_id: row.get(32)?,
        raw_text: row.get(33)?,
        dedupe_key: row.get(34)?,
        created_at: row.get(35)?,
        updated_at: row.get(36)?,
    })
}

fn candidate_select() -> &'static str {
    "SELECT c.id, c.extraction_run_id, c.source_id, s.title, s.source_url,
            c.answer_key_id, k.version_label, c.answer_key_entry_id, c.exam_id, c.year,
            c.paper_code, c.session_label, c.page_number, c.question_number,
            c.section_code, c.question_type_code, c.statement, c.options_json,
            c.correct_answer, c.correct_range, c.max_marks, c.negative_marks,
            c.unanswered_marks, c.numeric_tolerance, c.difficulty, c.syllabus_node_id,
            n.title, c.concept_ids_json, c.review_status, c.review_notes, c.reviewed_at,
            c.reviewed_by, c.verified_question_id, c.raw_text, c.dedupe_key, c.created_at,
            c.updated_at
     FROM pyq_extraction_candidates c
     JOIN pyq_sources s ON s.id = c.source_id
     LEFT JOIN pyq_answer_keys k ON k.id = c.answer_key_id
     LEFT JOIN exam_syllabus_nodes n ON n.id = c.syllabus_node_id"
}

pub fn list_candidates(
    connection: &Connection,
    filter: V2ExtractionCandidateFilter,
) -> Result<Vec<V2ExtractionCandidate>, AppError> {
    let limit = filter.limit.unwrap_or(200).clamp(1, 1000);
    let offset = filter.offset.unwrap_or(0).max(0);
    let query = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{value}%"));
    let mut statement = connection
        .prepare(&format!(
            "{} WHERE (?1 IS NULL OR c.exam_id = ?1)
                 AND (?2 IS NULL OR c.year = ?2)
                 AND (?3 IS NULL OR c.source_id = ?3)
                 AND (?4 IS NULL OR c.review_status = ?4)
                 AND (?5 IS NULL OR c.statement LIKE ?5 OR c.question_number LIKE ?5)
             ORDER BY c.year, c.question_number + 0, c.question_number, c.id
             LIMIT ?6 OFFSET ?7",
            candidate_select()
        ))
        .map_err(|error| {
            AppError::database("Extraction candidates could not be prepared", error)
        })?;
    statement
        .query_map(
            params![
                filter.exam_id,
                filter.year,
                filter.source_id,
                filter.review_status,
                query,
                limit,
                offset
            ],
            map_candidate,
        )
        .map_err(|error| AppError::database("Extraction candidates could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An extraction candidate is invalid", error))
}

pub fn get_candidate(
    connection: &Connection,
    candidate_id: i64,
) -> Result<V2ExtractionCandidate, AppError> {
    connection
        .query_row(
            &format!("{} WHERE c.id = ?1", candidate_select()),
            [candidate_id],
            map_candidate,
        )
        .optional()
        .map_err(|error| AppError::database("Extraction candidate could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "candidateId",
                "This extraction candidate is no longer available.",
            )
        })
}

#[allow(clippy::too_many_lines)]
pub fn extract_source(
    connection: &mut Connection,
    input: V2ExtractionInput,
    now: DateTime<Utc>,
) -> Result<V2ExtractionResult, AppError> {
    if !matches!(
        input.method.as_str(),
        "pdf_text" | "manual" | "ocr" | "assisted"
    ) {
        return Err(AppError::validation(
            "method",
            "Choose pdf_text, manual, ocr or assisted.",
        ));
    }
    let source = get_source(connection, input.source_id)?;
    if source.source_type != SOURCE_TYPE_QUESTION_PAPER {
        return Err(AppError::validation(
            "sourceId",
            "Choose a source registered as a question paper.",
        ));
    }
    let answer_key_id = if let Some(answer_key_id) = input.answer_key_id {
        let key = get_answer_key(connection, answer_key_id)?;
        if key.exam_id != source.exam_id || key.year != source.year {
            return Err(AppError::validation(
                "answerKeyId",
                "The answer key must belong to the same examination and year as the paper.",
            ));
        }
        Some(answer_key_id)
    } else {
        connection
            .query_row(
                "SELECT k.id
                 FROM pyq_answer_keys k
                 JOIN pyq_sources s ON s.id = k.source_id
                 WHERE s.associated_source_id = ?1 AND k.exam_id = ?2 AND k.year = ?3
                   AND k.verified_at IS NOT NULL
                 ORDER BY k.id DESC LIMIT 1",
                params![source.id, source.exam_id, source.year],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| AppError::database("Associated answer key could not be read", error))?
    };
    let key_entries = key_entry_map(connection, answer_key_id)?;
    let questions = parse_question_paper(&input.text);
    if questions.is_empty() {
        return Err(AppError::validation(
            "text",
            "The extracted text did not produce any question boundaries.",
        ));
    }
    let exam_code: String = connection
        .query_row(
            "SELECT code FROM exams WHERE id = ?1",
            [source.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Source examination code could not be read", error))?;
    let now_text = now_text(now);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("Extraction transaction could not start", error))?;
    let run_id = {
        transaction
            .execute(
                "INSERT INTO pyq_extraction_runs(
                    source_id, answer_key_id, method, status, pages_processed,
                    questions_found, questions_reconciled, notes, started_at, created_at
                 ) VALUES (?1, ?2, ?3, 'completed', ?4, ?5, 0, ?6, ?7, ?7)",
                params![
                    source.id,
                    answer_key_id,
                    input.method,
                    questions
                        .iter()
                        .map(|question| question.page_number)
                        .max()
                        .unwrap_or(0),
                    questions.len() as i64,
                    input.notes.trim(),
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Extraction run could not be stored", error))?;
        transaction.last_insert_rowid()
    };
    let mut extracted = 0i64;
    let mut clean = 0i64;
    let mut needs_review = 0i64;
    let mut missing_key = 0i64;
    let mut skipped_existing = 0i64;
    for question in &questions {
        let existing: i64 = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM pyq_extraction_candidates
                    WHERE source_id = ?1 AND session_label = ?2 AND question_number = ?3
                 )",
                params![source.id, source.session_label, question.question_number],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("Existing extraction candidate could not be checked", error)
            })?;
        if existing == 1 {
            skipped_existing += 1;
            continue;
        }
        let key_entry = key_entries.get(&question.question_number);
        let key_session_matches = key_entry.is_none_or(|entry| {
            source.session_label.trim().is_empty()
                || entry.session_label.trim().is_empty()
                || entry.session_label.trim() == source.session_label.trim()
        });
        let key_entry = key_entry.filter(|_| key_session_matches);
        let reasons = review_reasons(question, key_entry.map(parsed_from_entry).as_ref());
        let review_status = if reasons.is_empty() {
            clean += 1;
            REVIEW_EXTRACTED
        } else {
            needs_review += 1;
            REVIEW_NEEDS_REVIEW
        };
        if key_entry.is_none() {
            missing_key += 1;
        }
        let (section_code, question_type, max_marks, negative_marks, correct_answer, correct_range) =
            if let Some(entry) = key_entry {
                let section_code =
                    official_section_code(&transaction, source.exam_id, &entry.section_code);
                let negative = candidate_negative_marks(&entry.question_type_code, entry.marks);
                let (answer, range) = if entry.question_type_code == "NAT" {
                    (String::new(), entry.key_value.clone())
                } else if entry.key_status == "mta" {
                    (String::new(), String::new())
                } else {
                    (
                        super::scoring::normalize_choice(&entry.key_value),
                        String::new(),
                    )
                };
                (
                    section_code,
                    entry.question_type_code.clone(),
                    Some(entry.marks),
                    negative,
                    answer,
                    range,
                )
            } else {
                (
                    String::new(),
                    String::new(),
                    None,
                    0.0,
                    String::new(),
                    String::new(),
                )
            };
        let options_json = serde_json::to_string(&question.options).map_err(|error| {
            AppError::database("Extracted answer choices could not be encoded", error)
        })?;
        let dedupe_key = format!(
            "{exam_code}-{}-{}-{}",
            source.year,
            source.session_label.trim(),
            question.question_number
        );
        transaction
            .execute(
                "INSERT INTO pyq_extraction_candidates(
                    extraction_run_id, source_id, answer_key_id, answer_key_entry_id,
                    exam_id, year, paper_code, session_label, page_number, question_number,
                    section_code, question_type_code, statement, options_json,
                    correct_answer, correct_range, max_marks, negative_marks,
                    unanswered_marks, numeric_tolerance, difficulty, syllabus_node_id,
                    concept_ids_json, review_status, review_notes, raw_text, dedupe_key,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                           ?15, ?16, ?17, ?18, 0, 0, 3, NULL, '[]', ?19, ?20, ?21, ?22, ?23, ?23)",
                params![
                    run_id,
                    source.id,
                    answer_key_id,
                    key_entry.map(|entry| entry.id),
                    source.exam_id,
                    source.year,
                    source.paper_code,
                    source.session_label,
                    question.page_number,
                    question.question_number,
                    section_code,
                    question_type,
                    question.statement,
                    options_json,
                    correct_answer,
                    correct_range,
                    max_marks,
                    negative_marks,
                    review_status,
                    reasons.join(" "),
                    question.raw_text,
                    dedupe_key,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("Extraction candidate could not be stored", error)
            })?;
        extracted += 1;
    }
    let pages_processed = questions
        .iter()
        .map(|question| question.page_number)
        .max()
        .unwrap_or(0);
    if extracted == 0 && skipped_existing > 0 {
        transaction
            .execute("DELETE FROM pyq_extraction_runs WHERE id = ?1", [run_id])
            .map_err(|error| {
                AppError::database("Empty extraction run could not be removed", error)
            })?;
        transaction
            .commit()
            .map_err(|error| AppError::database("Extraction could not be committed", error))?;
        let run = V2ExtractionRun {
            id: 0,
            source_id: source.id,
            answer_key_id,
            method: input.method,
            status: "completed".to_owned(),
            pages_processed,
            questions_found: questions.len() as i64,
            questions_reconciled: 0,
            notes: format!(
                "{} No new candidates were stored; existing candidates were preserved.",
                input.notes.trim()
            )
            .trim()
            .to_owned(),
            started_at: now_text.clone(),
            completed_at: Some(now_text.clone()),
            created_at: now_text,
        };
        return Ok(V2ExtractionResult {
            run,
            extracted,
            clean,
            needs_review,
            missing_key,
            skipped_existing,
        });
    }
    transaction
        .execute(
            "UPDATE pyq_extraction_runs SET questions_reconciled = ?1, completed_at = ?2
             WHERE id = ?3",
            params![clean, now_text, run_id],
        )
        .map_err(|error| AppError::database("Extraction run totals could not be saved", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Extraction could not be committed", error))?;
    let run = get_extraction_run(connection, run_id)?;
    Ok(V2ExtractionResult {
        run,
        extracted,
        clean,
        needs_review,
        missing_key,
        skipped_existing,
    })
}

fn get_extraction_run(connection: &Connection, run_id: i64) -> Result<V2ExtractionRun, AppError> {
    connection
        .query_row(
            "SELECT id, source_id, answer_key_id, method, status, pages_processed,
                    questions_found, questions_reconciled, notes, started_at, completed_at,
                    created_at
             FROM pyq_extraction_runs WHERE id = ?1",
            [run_id],
            |row| {
                Ok(V2ExtractionRun {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    answer_key_id: row.get(2)?,
                    method: row.get(3)?,
                    status: row.get(4)?,
                    pages_processed: row.get(5)?,
                    questions_found: row.get(6)?,
                    questions_reconciled: row.get(7)?,
                    notes: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    created_at: row.get(11)?,
                })
            },
        )
        .map_err(|error| AppError::database("Extraction run could not be read", error))
}

pub fn reconcile_source_candidates(
    connection: &mut Connection,
    source_id: i64,
    answer_key_id: i64,
    now: DateTime<Utc>,
) -> Result<i64, AppError> {
    let source = get_source(connection, source_id)?;
    let key = get_answer_key(connection, answer_key_id)?;
    if key.exam_id != source.exam_id || key.year != source.year {
        return Err(AppError::validation(
            "answerKeyId",
            "The answer key must belong to the same examination and year as the paper.",
        ));
    }
    let entries = key_entry_map(connection, Some(answer_key_id))?;
    let now_text = now_text(now);
    let mut updated = 0i64;
    let candidates = {
        let mut statement = connection
            .prepare(
                "SELECT id, question_number FROM pyq_extraction_candidates
                 WHERE source_id = ?1 AND review_status IN ('extracted','needs_review')",
            )
            .map_err(|error| {
                AppError::database("Candidates for reconciliation could not be prepared", error)
            })?;
        statement
            .query_map([source_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| {
                AppError::database("Candidates for reconciliation could not be read", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                AppError::database("A candidate for reconciliation is invalid", error)
            })?
    };
    for (candidate_id, question_number) in candidates {
        let Some(entry) = entries.get(&question_number) else {
            connection
                .execute(
                    "UPDATE pyq_extraction_candidates
                     SET review_status = 'needs_review',
                         review_notes = CASE WHEN review_notes = '' THEN ?1 ELSE review_notes || ' ' || ?1 END,
                         updated_at = ?2
                     WHERE id = ?3",
                    params![
                        "No matching final answer-key entry was found after reconciliation.",
                        now_text,
                        candidate_id
                    ],
                )
                .map_err(|error| AppError::database("Candidate could not be reconciled", error))?;
            updated += 1;
            continue;
        };
        let section_code = official_section_code(connection, source.exam_id, &entry.section_code);
        let negative = candidate_negative_marks(&entry.question_type_code, entry.marks);
        let (answer, range) = if entry.question_type_code == "NAT" {
            (String::new(), entry.key_value.clone())
        } else if entry.key_status == "mta" {
            (String::new(), String::new())
        } else {
            (
                super::scoring::normalize_choice(&entry.key_value),
                String::new(),
            )
        };
        connection
            .execute(
                "UPDATE pyq_extraction_candidates
                 SET answer_key_id = ?1, answer_key_entry_id = ?2, section_code = ?3,
                     question_type_code = ?4, max_marks = ?5, negative_marks = ?6,
                     correct_answer = ?7, correct_range = ?8, updated_at = ?9
                 WHERE id = ?10",
                params![
                    answer_key_id,
                    entry.id,
                    section_code,
                    entry.question_type_code,
                    entry.marks,
                    negative,
                    answer,
                    range,
                    now_text,
                    candidate_id
                ],
            )
            .map_err(|error| AppError::database("Candidate could not be reconciled", error))?;
        updated += 1;
    }
    Ok(updated)
}

pub fn update_candidate(
    connection: &mut Connection,
    input: V2ExtractionCandidateUpdate,
    now: DateTime<Utc>,
) -> Result<V2ExtractionCandidate, AppError> {
    let candidate = get_candidate(connection, input.candidate_id)?;
    if matches!(
        candidate.review_status.as_str(),
        REVIEW_VERIFIED | REVIEW_REJECTED
    ) {
        return Err(AppError::validation(
            "candidateId",
            "Verified or rejected candidates cannot be edited.",
        ));
    }
    if input.statement.trim().is_empty() {
        return Err(AppError::validation(
            "statement",
            "Question text cannot be empty.",
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
    if !input.numeric_tolerance.is_finite() || input.numeric_tolerance < 0.0 {
        return Err(AppError::validation(
            "numericTolerance",
            "Numeric tolerance cannot be negative.",
        ));
    }
    if !(1..=5).contains(&input.difficulty) {
        return Err(AppError::validation(
            "difficulty",
            "Difficulty must be between 1 and 5.",
        ));
    }
    if !matches!(input.question_type_code.as_str(), "MCQ" | "MSQ" | "NAT") {
        return Err(AppError::validation(
            "questionTypeCode",
            "Choose MCQ, MSQ or NAT.",
        ));
    }
    let section_id: Option<i64> = connection
        .query_row(
            "SELECT id FROM exam_sections WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
            params![candidate.exam_id, input.section_code],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Candidate section could not be checked", error))?;
    if section_id.is_none() {
        return Err(AppError::validation(
            "sectionCode",
            "Choose an active section code for this examination.",
        ));
    }
    if input.question_type_code == "NAT" && !input.options.is_empty() {
        return Err(AppError::validation(
            "options",
            "Numerical questions cannot have answer choices.",
        ));
    }
    let options_json = serde_json::to_string(&input.options)
        .map_err(|error| AppError::database("Candidate options could not be encoded", error))?;
    let mut concept_ids = input.concept_ids;
    concept_ids.sort_unstable();
    concept_ids.dedup();
    for concept_id in &concept_ids {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1 AND is_archived = 0)",
                [concept_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Candidate concept could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "conceptIds",
                "Choose active shared concepts.",
            ));
        }
    }
    let concept_ids_json = serde_json::to_string(&concept_ids)
        .map_err(|error| AppError::database("Candidate concepts could not be encoded", error))?;
    if let Some(node_id) = input.syllabus_node_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM exam_syllabus_nodes
                    WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0
                 )",
                params![node_id, candidate.exam_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("Candidate syllabus topic could not be checked", error)
            })?;
        if exists == 0 {
            return Err(AppError::validation(
                "syllabusNodeId",
                "Choose an active syllabus topic from this examination.",
            ));
        }
    }
    let review_notes = if input.review_notes.trim().is_empty() {
        "Reviewer edited the extracted record.".to_owned()
    } else {
        input.review_notes.trim().to_owned()
    };
    connection
        .execute(
            "UPDATE pyq_extraction_candidates
             SET statement = ?1, options_json = ?2, correct_answer = ?3, correct_range = ?4,
                 max_marks = ?5, negative_marks = ?6, numeric_tolerance = ?7, difficulty = ?8,
                 section_code = ?9, question_type_code = ?10, syllabus_node_id = ?11,
                 concept_ids_json = ?12, review_status = 'needs_review', review_notes = ?13,
                 reviewed_at = ?14, reviewed_by = ?15, updated_at = ?14
             WHERE id = ?16",
            params![
                input.statement.trim(),
                options_json,
                input.correct_answer.trim(),
                input.correct_range.trim(),
                input.max_marks,
                input.negative_marks,
                input.numeric_tolerance,
                input.difficulty,
                input.section_code,
                input.question_type_code,
                input.syllabus_node_id,
                concept_ids_json,
                review_notes,
                now_text(now),
                input.reviewed_by.trim(),
                input.candidate_id,
            ],
        )
        .map_err(|error| AppError::database("Extraction candidate could not be updated", error))?;
    get_candidate(connection, input.candidate_id)
}

#[allow(clippy::too_many_lines)]
pub fn verify_candidate(
    connection: &mut Connection,
    input: V2CandidateDecisionInput,
    now: DateTime<Utc>,
) -> Result<V2ExtractionCandidate, AppError> {
    let candidate = get_candidate(connection, input.candidate_id)?;
    if matches!(
        candidate.review_status.as_str(),
        REVIEW_VERIFIED | REVIEW_REJECTED
    ) {
        return Err(AppError::validation(
            "candidateId",
            "This candidate has already been reviewed.",
        ));
    }
    let source = get_source(connection, candidate.source_id)?;
    if source.verification_status != STATUS_VERIFIED {
        return Err(AppError::validation(
            "candidateId",
            "Mark the paper source verified before approving extracted questions.",
        ));
    }
    let Some(answer_key_id) = candidate.answer_key_id else {
        return Err(AppError::validation(
            "candidateId",
            "Associate a final answer key before approving this question.",
        ));
    };
    let key = get_answer_key(connection, answer_key_id)?;
    if key.verified_at.is_none() {
        return Err(AppError::validation(
            "candidateId",
            "Verify the final answer key before approving extracted questions.",
        ));
    }
    let Some(entry_id) = candidate.answer_key_entry_id else {
        return Err(AppError::validation(
            "candidateId",
            "The candidate is not reconciled with an answer-key entry.",
        ));
    };
    let entry = key
        .entries
        .iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| {
            AppError::validation(
                "candidateId",
                "The associated answer-key entry is no longer available.",
            )
        })?;
    if entry.question_number != candidate.question_number
        || entry.question_type_code != candidate.question_type_code
    {
        return Err(AppError::validation(
            "candidateId",
            "The candidate no longer matches its answer-key entry.",
        ));
    }
    let key_matches = if candidate.question_type_code == "NAT" {
        candidate.correct_range.trim() == entry.key_value.trim()
    } else {
        super::scoring::normalize_choice(&candidate.correct_answer)
            == super::scoring::normalize_choice(&entry.key_value)
    };
    if !key_matches {
        return Err(AppError::validation(
            "candidateId",
            "Reconcile the candidate answer with the final answer key before approving it.",
        ));
    }
    let pattern_version_id: Option<i64> = connection
        .query_row(
            "SELECT id FROM exam_pattern_versions
             WHERE exam_id = ?1 AND verification_status = 'verified'
             ORDER BY id DESC LIMIT 1",
            [candidate.exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Verified pattern version could not be read", error))?;
    let Some(pattern_version_id) = pattern_version_id else {
        return Err(AppError::validation(
            "candidateId",
            "A verified pattern version is required before approving questions.",
        ));
    };
    let section_id: i64 = connection
        .query_row(
            "SELECT id FROM exam_sections
             WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
            params![candidate.exam_id, candidate.section_code],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Candidate section could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "candidateId",
                "The candidate section is not active in this examination.",
            )
        })?;
    let question_type_id: i64 = connection
        .query_row(
            "SELECT id FROM exam_question_types
             WHERE exam_id = ?1 AND code = ?2 AND is_archived = 0",
            params![candidate.exam_id, candidate.question_type_code],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Candidate question type could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "candidateId",
                "The candidate question type is not active in this examination.",
            )
        })?;
    let existing: Option<(i64, Option<i64>)> = connection
        .query_row(
            "SELECT id, answer_key_id FROM pyq_questions
             WHERE source_id = ?1 AND question_number = ?2 AND is_archived = 0",
            params![candidate.source_id, candidate.question_number],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            AppError::database("Existing verified question could not be checked", error)
        })?;
    if let Some((question_id, existing_key)) = existing {
        if existing_key == Some(answer_key_id) {
            connection
                .execute(
                    "UPDATE pyq_extraction_candidates
                     SET review_status = 'verified', verified_question_id = ?1,
                         reviewed_at = ?2, reviewed_by = ?3,
                         review_notes = CASE WHEN ?4 = '' THEN review_notes ELSE ?4 END,
                         updated_at = ?2
                     WHERE id = ?5",
                    params![
                        question_id,
                        now_text(now),
                        input.reviewed_by.trim(),
                        input.review_notes.trim(),
                        candidate.id
                    ],
                )
                .map_err(|error| AppError::database("Candidate could not be linked", error))?;
            return get_candidate(connection, candidate.id);
        }
        return Err(AppError::validation(
            "candidateId",
            "A verified question already exists for this source and question number with a different answer key.",
        ));
    }
    let source_page = candidate
        .page_number
        .map(|page| format!("p. {page}"))
        .unwrap_or_default();
    let input_question = super::pyq::V2PyqQuestionInput {
        id: None,
        exam_id: candidate.exam_id,
        year: candidate.year,
        session_label: candidate.session_label.clone(),
        section_id,
        question_type_id,
        question_number: candidate.question_number.clone(),
        statement: candidate.statement.clone(),
        correct_answer: candidate.correct_answer.clone(),
        correct_range: candidate.correct_range.clone(),
        max_marks: candidate.max_marks.unwrap_or(0.0),
        negative_marks: candidate.negative_marks,
        unanswered_marks: candidate.unanswered_marks,
        difficulty: candidate.difficulty,
        source_title: source.title.clone(),
        source_page,
        source_url: source.source_url.clone(),
        answer_key_source: key.source_title.clone(),
        answer_key_version: key.version_label.clone(),
        answer_key_verified: true,
        verification_status: "verified".to_owned(),
        numeric_tolerance: candidate.numeric_tolerance,
        options: candidate.options.clone(),
        pattern_version_id: Some(pattern_version_id),
        syllabus_node_id: candidate.syllabus_node_id,
        topic_ids: Vec::new(),
        concept_ids: candidate.concept_ids.clone(),
        resource_id: source.resource_id,
        dedupe_key: candidate.dedupe_key.clone(),
        official_explanation: String::new(),
        third_party_solution: String::new(),
        user_note: "Imported from an official source and reconciled with the final answer key."
            .to_owned(),
        source_id: Some(source.id),
        answer_key_id: Some(answer_key_id),
    };
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("Candidate verification transaction could not start", error)
    })?;
    let question_id = super::pyq::save_question_in_transaction(&transaction, input_question, now)?;
    transaction
        .execute(
            "UPDATE pyq_extraction_candidates
             SET review_status = 'verified', verified_question_id = ?1, reviewed_at = ?2,
                 reviewed_by = ?3,
                 review_notes = CASE WHEN ?4 = '' THEN review_notes ELSE ?4 END,
                 updated_at = ?2
             WHERE id = ?5",
            params![
                question_id,
                now_text(now),
                input.reviewed_by.trim(),
                input.review_notes.trim(),
                candidate.id
            ],
        )
        .map_err(|error| AppError::database("Candidate could not be marked verified", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("Candidate verification could not be committed", error)
    })?;
    get_candidate(connection, candidate.id)
}

pub fn reject_candidate(
    connection: &mut Connection,
    input: V2CandidateDecisionInput,
    now: DateTime<Utc>,
) -> Result<V2ExtractionCandidate, AppError> {
    let candidate = get_candidate(connection, input.candidate_id)?;
    if matches!(
        candidate.review_status.as_str(),
        REVIEW_VERIFIED | REVIEW_REJECTED
    ) {
        return Err(AppError::validation(
            "candidateId",
            "This candidate has already been reviewed.",
        ));
    }
    connection
        .execute(
            "UPDATE pyq_extraction_candidates
             SET review_status = 'rejected', reviewed_at = ?1, reviewed_by = ?2,
                 review_notes = CASE WHEN ?3 = '' THEN review_notes ELSE ?3 END,
                 updated_at = ?1
             WHERE id = ?4",
            params![
                now_text(now),
                input.reviewed_by.trim(),
                input.review_notes.trim(),
                candidate.id
            ],
        )
        .map_err(|error| AppError::database("Candidate could not be rejected", error))?;
    get_candidate(connection, candidate.id)
}

pub fn source_detail(connection: &Connection, source_id: i64) -> Result<V2SourceDetail, AppError> {
    let source = get_source(connection, source_id)?;
    let answer_keys = list_answer_keys(connection, Some(source.exam_id), Some(source.year))?
        .into_iter()
        .filter(|key| key.source_id == source_id)
        .collect::<Vec<_>>();
    let mut statement = connection
        .prepare(
            "SELECT id, source_id, answer_key_id, method, status, pages_processed,
                    questions_found, questions_reconciled, notes, started_at, completed_at,
                    created_at
             FROM pyq_extraction_runs WHERE source_id = ?1 ORDER BY id DESC",
        )
        .map_err(|error| {
            AppError::database("Source extraction runs could not be prepared", error)
        })?;
    let extraction_runs = statement
        .query_map([source_id], |row| {
            Ok(V2ExtractionRun {
                id: row.get(0)?,
                source_id: row.get(1)?,
                answer_key_id: row.get(2)?,
                method: row.get(3)?,
                status: row.get(4)?,
                pages_processed: row.get(5)?,
                questions_found: row.get(6)?,
                questions_reconciled: row.get(7)?,
                notes: row.get(8)?,
                started_at: row.get(9)?,
                completed_at: row.get(10)?,
                created_at: row.get(11)?,
            })
        })
        .map_err(|error| AppError::database("Source extraction runs could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A source extraction run is invalid", error))?;
    let mut counts = BTreeMap::new();
    let mut count_statement = connection
        .prepare(
            "SELECT review_status, COUNT(*) FROM pyq_extraction_candidates
             WHERE source_id = ?1 GROUP BY review_status",
        )
        .map_err(|error| {
            AppError::database("Source candidate counts could not be prepared", error)
        })?;
    let rows = count_statement
        .query_map([source_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| AppError::database("Source candidate counts could not be read", error))?;
    for row in rows {
        let (status, count) =
            row.map_err(|error| AppError::database("A source candidate count is invalid", error))?;
        counts.insert(status, count);
    }
    Ok(V2SourceDetail {
        source,
        answer_keys,
        extraction_runs,
        candidate_counts: counts,
    })
}

pub fn counts(connection: &Connection) -> Result<V2SourceCounts, AppError> {
    let count = |sql: &str| -> Result<i64, AppError> {
        connection
            .query_row(sql, [], |row| row.get(0))
            .map_err(|error| AppError::database("Source summary could not be read", error))
    };
    Ok(V2SourceCounts {
        sources: count("SELECT COUNT(*) FROM pyq_sources")?,
        answer_keys: count("SELECT COUNT(*) FROM pyq_answer_keys")?,
        key_entries: count("SELECT COUNT(*) FROM pyq_answer_key_entries")?,
        extraction_runs: count("SELECT COUNT(*) FROM pyq_extraction_runs")?,
        candidates: count("SELECT COUNT(*) FROM pyq_extraction_candidates")?,
        candidates_verified: count(
            "SELECT COUNT(*) FROM pyq_extraction_candidates WHERE review_status = 'verified'",
        )?,
        candidates_pending: count(
            "SELECT COUNT(*) FROM pyq_extraction_candidates WHERE review_status IN ('extracted','needs_review')",
        )?,
        candidates_rejected: count(
            "SELECT COUNT(*) FROM pyq_extraction_candidates WHERE review_status = 'rejected'",
        )?,
        verified_questions: count(
            "SELECT COUNT(*) FROM pyq_questions WHERE source_id IS NOT NULL AND verification_status = 'verified'",
        )?,
        key_conflicts: count(
            "SELECT COUNT(*) FROM pyq_answer_key_conflicts WHERE status = 'open'",
        )?,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;
    use tempfile::TempDir;

    use super::*;
    use crate::v2::V2State;

    fn state() -> (TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn fake_pdf(root: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.path().join(name);
        fs::write(&path, bytes).expect("fake pdf");
        path
    }

    fn source_input(year: i64, source_type: &str, title: &str, url: &str) -> V2PyqSourceInput {
        V2PyqSourceInput {
            id: None,
            exam_id: 2,
            year,
            paper_code: "PH".to_owned(),
            source_type: source_type.to_owned(),
            authority: "GATE Organising Institute".to_owned(),
            institution: "IIT Test".to_owned(),
            title: title.to_owned(),
            source_url: Some(url.to_owned()),
            resource_id: None,
            page_count: Some(40),
            published_at: None,
            session_label: "5".to_owned(),
            document_version: "final".to_owned(),
            key_status: if source_type == SOURCE_TYPE_ANSWER_KEY {
                "final".to_owned()
            } else {
                String::new()
            },
            verification_status: None,
            verification_notes: String::new(),
            associated_source_id: None,
        }
    }

    fn acquire(
        state: &V2State,
        root: &TempDir,
        input: V2PyqSourceInput,
        file: &str,
        bytes: &[u8],
    ) -> V2PyqSourceAcquisition {
        let path = fake_pdf(root, file, bytes);
        let data_dir = state.data_directory().expect("data dir");
        state
            .with_connection(|connection| {
                super::acquire_source(
                    connection,
                    &data_dir,
                    V2AcquireSourceInput {
                        source: input,
                        file_path: path.display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    Utc::now(),
                )
            })
            .expect("acquire source")
    }

    fn full_key_entries(
        q1: &str,
        q2: &str,
        q3: &str,
        q4: &str,
    ) -> Vec<V2ParsedAnswerKeyEntryInput> {
        let mut entries = Vec::new();
        for number in 1..=65 {
            let (question_type, section, key_value, marks) = match number {
                1 => ("MCQ", "GA", q1, 1.0),
                2 => ("MSQ", "PH", q2, 1.0),
                3 => ("NAT", "PH", q3, 1.0),
                4 => ("MCQ", "GA", q4, 1.0),
                5 => ("MCQ", "GA", "A", 1.0),
                6..=10 => ("MCQ", "GA", "A", 2.0),
                11..=35 => ("MCQ", "PH", "A", 1.0),
                36..=65 => ("MCQ", "PH", "A", 2.0),
                _ => unreachable!(),
            };
            entries.push(V2ParsedAnswerKeyEntryInput {
                question_number: number.to_string(),
                session_label: "5".to_owned(),
                question_type_code: question_type.to_owned(),
                section_code: section.to_owned(),
                key_value: key_value.to_owned(),
                marks,
                key_status: "answer".to_owned(),
                official_question_id: String::new(),
                notes: String::new(),
            });
        }
        entries
    }

    fn extraction_text() -> String {
        r#"
===== PAGE 1 =====
GATE 2022 Physics (PH)
Q.1 Choose the correct option from the following four alternatives.
(A) Alpha
(B) Beta
(C) Gamma
(D) Delta
===== PAGE 2 =====
Q.2 Select all correct options from the following four alternatives.
(A) One
(B) Two
(C) Three
(D) Four
===== PAGE 3 =====
Q.3 A numerical question with a verified range and no answer choices.
===== PAGE 4 =====
Q.4 This question has no extracted answer choices and should remain pending review.
"#
        .to_owned()
    }

    #[test]
    fn source_registration_links_managed_pdf_and_detects_duplicate_hash() {
        let (root, state) = state();
        let first = acquire(
            &state,
            &root,
            source_input(
                2022,
                SOURCE_TYPE_QUESTION_PAPER,
                "GATE 2022 PH paper",
                "https://example.test/ph-2022.pdf",
            ),
            "ph-2022.pdf",
            b"%PDF-1.4 synthetic paper",
        );
        assert!(!first.duplicate);
        assert_eq!(first.source.verification_status, STATUS_ACQUIRED);
        assert!(first.source.resource_id.is_some());
        assert_eq!(first.source.sha256.as_ref().map(String::len), Some(64));
        assert_eq!(first.source.size_bytes, Some(24));
        assert_eq!(
            first.source.resource_file_name.as_deref(),
            Some("ph-2022.pdf")
        );

        let duplicate = acquire(
            &state,
            &root,
            source_input(
                2022,
                SOURCE_TYPE_QUESTION_PAPER,
                "GATE 2022 PH paper",
                "https://example.test/ph-2022.pdf",
            ),
            "ph-2022-copy.pdf",
            b"%PDF-1.4 synthetic paper",
        );
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.source.id, first.source.id);
        let count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM pyq_sources", [], |row| row.get(0))
                    .map_err(|error| AppError::database("source count", error))
            })
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn answer_keys_preserve_versions_and_missing_entries_are_reported() {
        let (root, state) = state();
        let source = acquire(
            &state,
            &root,
            source_input(
                2022,
                SOURCE_TYPE_ANSWER_KEY,
                "GATE 2022 PH final key",
                "https://example.test/ph-2022-key.pdf",
            ),
            "ph-2022-key.pdf",
            b"%PDF-1.4 synthetic key",
        )
        .source;
        let text = "1 5 MCQ GA A 1\n2 5 MSQ PH A,C 2\n3 5 NAT PH 2.4 to 2.6 1";
        let preview = preview_answer_key_text(text);
        assert_eq!(preview.parsed_count, 3);
        assert_eq!(preview.missing_question_numbers.len(), 62);
        let key = state
            .with_connection(|connection| {
                super::save_answer_key(
                    connection,
                    V2PyqAnswerKeyInput {
                        source_id: source.id,
                        version_label: "final-1".to_owned(),
                        key_status: "final".to_owned(),
                        published_at: None,
                        verified: false,
                        verified_by: String::new(),
                        notes: String::new(),
                        supersedes_key_id: None,
                        text: text.to_owned(),
                        entries: Vec::new(),
                    },
                    Utc::now(),
                )
            })
            .expect("save key");
        assert_eq!(key.entry_count, 3);
        let second = state
            .with_connection(|connection| {
                super::save_answer_key(
                    connection,
                    V2PyqAnswerKeyInput {
                        source_id: source.id,
                        version_label: "final-2".to_owned(),
                        key_status: "revised".to_owned(),
                        published_at: None,
                        verified: false,
                        verified_by: String::new(),
                        notes: "Test revision".to_owned(),
                        supersedes_key_id: Some(key.id),
                        text: text.to_owned(),
                        entries: Vec::new(),
                    },
                    Utc::now(),
                )
            })
            .expect("save revised key");
        assert_ne!(second.id, key.id);
        let keys = state
            .with_connection(|connection| super::list_answer_keys(connection, Some(2), Some(2022)))
            .expect("keys");
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn extraction_reconciles_mcq_msq_and_nat_with_page_references() {
        let (root, state) = state();
        let paper = acquire(
            &state,
            &root,
            source_input(
                2022,
                SOURCE_TYPE_QUESTION_PAPER,
                "GATE 2022 PH paper",
                "https://example.test/ph-2022-paper.pdf",
            ),
            "ph-2022-paper.pdf",
            b"%PDF-1.4 synthetic paper",
        )
        .source;
        let mut key_input = source_input(
            2022,
            SOURCE_TYPE_ANSWER_KEY,
            "GATE 2022 PH final key",
            "https://example.test/ph-2022-final-key.pdf",
        );
        key_input.associated_source_id = Some(paper.id);
        let key_source = acquire(
            &state,
            &root,
            key_input,
            "ph-2022-final-key.pdf",
            b"%PDF-1.4 synthetic final key",
        )
        .source;
        let key = state
            .with_connection(|connection| {
                super::save_answer_key(
                    connection,
                    V2PyqAnswerKeyInput {
                        source_id: key_source.id,
                        version_label: "final-2022".to_owned(),
                        key_status: "final".to_owned(),
                        published_at: None,
                        verified: true,
                        verified_by: "local reviewer".to_owned(),
                        notes: String::new(),
                        supersedes_key_id: None,
                        text: String::new(),
                        entries: full_key_entries("A", "A,C", "2.4 to 2.6", "B"),
                    },
                    Utc::now(),
                )
            })
            .expect("save full key");
        let extracted = state
            .with_connection(|connection| {
                super::extract_source(
                    connection,
                    V2ExtractionInput {
                        source_id: paper.id,
                        text: extraction_text(),
                        method: "manual".to_owned(),
                        answer_key_id: Some(key.id),
                        notes: "Synthetic extraction".to_owned(),
                    },
                    Utc::now(),
                )
            })
            .expect("extract");
        assert_eq!(extracted.extracted, 4);
        assert_eq!(extracted.clean, 3);
        assert_eq!(extracted.needs_review, 1);
        let candidates = state
            .with_connection(|connection| {
                super::list_candidates(
                    connection,
                    V2ExtractionCandidateFilter {
                        source_id: Some(paper.id),
                        ..V2ExtractionCandidateFilter::default()
                    },
                )
            })
            .expect("candidates");
        assert_eq!(candidates.len(), 4);
        let q1 = candidates
            .iter()
            .find(|candidate| candidate.question_number == "1")
            .expect("q1");
        assert_eq!(q1.page_number, Some(1));
        assert_eq!(q1.question_type_code, "MCQ");
        assert_eq!(q1.section_code, "gate-ga");
        assert_eq!(q1.correct_answer, "A");
        assert_eq!(q1.options.len(), 4);
        assert_eq!(q1.review_status, REVIEW_EXTRACTED);
        let q2 = candidates
            .iter()
            .find(|candidate| candidate.question_number == "2")
            .expect("q2");
        assert_eq!(q2.question_type_code, "MSQ");
        assert_eq!(q2.correct_answer, "A,C");
        let q3 = candidates
            .iter()
            .find(|candidate| candidate.question_number == "3")
            .expect("q3");
        assert_eq!(q3.question_type_code, "NAT");
        assert_eq!(q3.correct_range, "2.4 to 2.6");
        assert_eq!(q3.correct_answer, "");
        let q4 = candidates
            .iter()
            .find(|candidate| candidate.question_number == "4")
            .expect("q4");
        assert_eq!(q4.review_status, REVIEW_NEEDS_REVIEW);
        assert!(q4.review_notes.contains("four answer choices"));
        assert!(q3.page_number == Some(3));

        let again = state
            .with_connection(|connection| {
                super::extract_source(
                    connection,
                    V2ExtractionInput {
                        source_id: paper.id,
                        text: extraction_text(),
                        method: "manual".to_owned(),
                        answer_key_id: Some(key.id),
                        notes: "Idempotency check".to_owned(),
                    },
                    Utc::now(),
                )
            })
            .expect("repeat extraction");
        assert_eq!(again.extracted, 0);
        assert_eq!(again.skipped_existing, 4);
        let run_count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM pyq_extraction_runs WHERE source_id = ?1",
                        [paper.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("run count", error))
            })
            .expect("run count");
        assert_eq!(run_count, 1);
    }

    #[test]
    fn verified_candidates_become_mock_eligible_and_revised_keys_flag_conflicts() {
        let (root, state) = state();
        let paper = acquire(
            &state,
            &root,
            source_input(
                2022,
                SOURCE_TYPE_QUESTION_PAPER,
                "GATE 2022 PH paper",
                "https://example.test/ph-2022-paper-2.pdf",
            ),
            "ph-2022-paper-2.pdf",
            b"%PDF-1.4 synthetic paper 2",
        )
        .source;
        let mut key_input = source_input(
            2022,
            SOURCE_TYPE_ANSWER_KEY,
            "GATE 2022 PH final key",
            "https://example.test/ph-2022-final-key-2.pdf",
        );
        key_input.associated_source_id = Some(paper.id);
        let key_source = acquire(
            &state,
            &root,
            key_input,
            "ph-2022-final-key-2.pdf",
            b"%PDF-1.4 synthetic final key 2",
        )
        .source;
        let key = state
            .with_connection(|connection| {
                super::save_answer_key(
                    connection,
                    V2PyqAnswerKeyInput {
                        source_id: key_source.id,
                        version_label: "final-2022-a".to_owned(),
                        key_status: "final".to_owned(),
                        published_at: None,
                        verified: true,
                        verified_by: "local reviewer".to_owned(),
                        notes: String::new(),
                        supersedes_key_id: None,
                        text: String::new(),
                        entries: full_key_entries("A", "A,C", "2.4 to 2.6", "B"),
                    },
                    Utc::now(),
                )
            })
            .expect("save key");
        state
            .with_connection(|connection| {
                super::set_source_status(
                    connection,
                    V2SourceStatusInput {
                        source_id: paper.id,
                        verification_status: STATUS_VERIFIED.to_owned(),
                        verification_notes: "Paper identity checked.".to_owned(),
                    },
                    Utc::now(),
                )
            })
            .expect("verify paper source");
        let extracted = state
            .with_connection(|connection| {
                super::extract_source(
                    connection,
                    V2ExtractionInput {
                        source_id: paper.id,
                        text: extraction_text(),
                        method: "manual".to_owned(),
                        answer_key_id: Some(key.id),
                        notes: String::new(),
                    },
                    Utc::now(),
                )
            })
            .expect("extract");
        assert_eq!(extracted.extracted, 4);
        let candidates = state
            .with_connection(|connection| {
                super::list_candidates(
                    connection,
                    V2ExtractionCandidateFilter {
                        source_id: Some(paper.id),
                        ..V2ExtractionCandidateFilter::default()
                    },
                )
            })
            .expect("candidates");
        for candidate in candidates
            .iter()
            .filter(|candidate| candidate.question_number != "4")
        {
            state
                .with_connection(|connection| {
                    super::verify_candidate(
                        connection,
                        V2CandidateDecisionInput {
                            candidate_id: candidate.id,
                            reviewed_by: "local reviewer".to_owned(),
                            review_notes: "Approved after source and key review.".to_owned(),
                        },
                        Utc::now(),
                    )
                })
                .expect("verify candidate");
        }
        let verified_count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM pyq_questions WHERE exam_id = 2 AND verification_status = 'verified'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("verified count", error))
            })
            .expect("verified count");
        assert_eq!(verified_count, 3);
        let mock_candidates = state
            .with_connection(|connection| super::super::question_mock::candidates(connection, 2, 2))
            .expect("mock candidates");
        assert_eq!(mock_candidates.len(), 3);
        assert!(
            mock_candidates
                .iter()
                .any(|candidate| candidate.question.question_type_code == "MCQ")
        );
        assert!(
            mock_candidates
                .iter()
                .any(|candidate| candidate.question.question_type_code == "MSQ")
        );
        assert!(
            mock_candidates
                .iter()
                .any(|candidate| candidate.question.question_type_code == "NAT")
        );

        state
            .with_connection(|connection| {
                super::save_answer_key(
                    connection,
                    V2PyqAnswerKeyInput {
                        source_id: key_source.id,
                        version_label: "revised-2022-b".to_owned(),
                        key_status: "revised".to_owned(),
                        published_at: None,
                        verified: true,
                        verified_by: "local reviewer".to_owned(),
                        notes: "Revised final key".to_owned(),
                        supersedes_key_id: Some(key.id),
                        text: String::new(),
                        entries: full_key_entries("B", "A,C", "2.4 to 2.6", "B"),
                    },
                    Utc::now(),
                )
            })
            .expect("save revised key");
        let conflicts = state
            .with_connection(|connection| super::list_answer_key_conflicts(connection, Some(2)))
            .expect("conflicts");
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].old_key_value, "A");
        assert_eq!(conflicts[0].new_key_value, "B");
        let question = state
            .with_connection(|connection| {
                super::super::pyq::detail(connection, 2, conflicts[0].question_id)
            })
            .expect("question");
        assert_eq!(question.correct_answer, "A");
    }

    #[test]
    fn source_and_extraction_schema_prepare_is_idempotent() {
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| super::counts(connection))
            .expect("before");
        state
            .with_connection(super::super::prepare)
            .expect("first prepare");
        state
            .with_connection(super::super::prepare)
            .expect("second prepare");
        let after = state
            .with_connection(|connection| super::counts(connection))
            .expect("after");
        assert_eq!(before.sources, after.sources);
        assert_eq!(before.answer_keys, after.answer_keys);
        assert_eq!(before.candidates, after.candidates);
        let patterns: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM exam_patterns", [], |row| row.get(0))
                    .map_err(|error| AppError::database("patterns", error))
            })
            .expect("patterns");
        assert_eq!(patterns, 4);
    }
}

#[test]
#[ignore = "manual M12 GATE PH source, key, extraction and approval import"]
fn import_gate_ph_official_sources() {
    let data_dir = std::path::PathBuf::from(std::env::var("M12_DATA_DIR").expect("M12_DATA_DIR"));
    let raw_dir = std::path::PathBuf::from(std::env::var("M12_RAW_DIR").expect("M12_RAW_DIR"));
    let text_dir = std::path::PathBuf::from(std::env::var("M12_TEXT_DIR").expect("M12_TEXT_DIR"));
    let verify_selected = std::env::var("M12_VERIFY").as_deref() == Ok("1");
    let state = crate::v2::V2State::initialize(&data_dir);
    let summary = state
            .with_connection(|connection| -> Result<serde_json::Value, AppError> {
                let documents = [
                    (2021, "ph_2021.pdf", "ph_2021_key.pdf", "IIT Bombay", 40, 3,
                     "https://gate2026.iitg.ac.in/doc/download/2021/ph_2021.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/Answer_keys2021/ph_2021.pdf"),
                    (2022, "ph_2022.pdf", "ph_2022_key.pdf", "IIT Kharagpur", 44, 2,
                     "https://gate2026.iitg.ac.in/doc/download/2022/ph_2022.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/Answer_keys2022/ph_2022.pdf"),
                    (2023, "ph_2023.pdf", "ph_2023_key.pdf", "IIT Kanpur", 48, 2,
                     "https://gate2026.iitg.ac.in/doc/download/2023/ph_2023.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/Answer_keys2023/PH_ANS_GATE2023.pdf"),
                    (2024, "ph_2024_iisc_qp4.pdf", "ph_2024_iisc_key.pdf", "IISc Bengaluru", 44, 2,
                     "https://gate2026.iitg.ac.in/doc/download/2024/PH24S2.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/2024/PHFinalAnswerKey.pdf"),
                    (2025, "ph_2025_a.pdf", "ph_2025_key_a.pdf", "IIT Roorkee", 39, 2,
                     "https://gate2026.iitg.ac.in/doc/download/2025/PH2025.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/2025_Key/PH_Keys.pdf"),
                    (2026, "ph_2026.pdf", "ph_2026_key.pdf", "IIT Guwahati", 40, 2,
                     "https://gate2026.iitg.ac.in/doc/download/2026/QPs/PH.pdf",
                     "https://gate2026.iitg.ac.in/doc/download/2026/Keys/PH_Keys.pdf"),
                ];
                let now = Utc::now();
                let mut result = serde_json::Map::new();
                let mut per_year = serde_json::Map::new();
                for (year, paper_file, key_file, institution, paper_pages, key_pages, paper_url, key_url)
                    in documents
                {
                    let paper = crate::v2::practice::sources::acquire_source(
                        connection,
                        &data_dir,
                        V2AcquireSourceInput {
                            source: V2PyqSourceInput {
                                id: None,
                                exam_id: 2,
                                year,
                                paper_code: "PH".to_owned(),
                                source_type: SOURCE_TYPE_QUESTION_PAPER.to_owned(),
                                authority: format!("GATE {year} Organizing Institute"),
                                institution: institution.to_owned(),
                                title: format!("GATE {year} Physics (PH) Master Question Paper"),
                                source_url: Some(paper_url.to_owned()),
                                resource_id: None,
                                page_count: Some(paper_pages),
                                published_at: None,
                                session_label: if year == 2021 { String::new() } else {
                                    match year { 2022 => "5", 2023 => "6", 2024 => "2", 2025 => "7", 2026 => "2", _ => "" }.to_owned()
                                },
                                document_version: "official-archive".to_owned(),
                                key_status: String::new(),
                                verification_status: None,
                                verification_notes: String::new(),
                                associated_source_id: None,
                            },
                            file_path: raw_dir.join(paper_file).display().to_string(),
                            storage_mode: "managed".to_owned(),
                        },
                        now,
                    )?;
                    if paper.source.verification_status != STATUS_VERIFIED {
                        set_source_status(
                            connection,
                            V2SourceStatusInput {
                                source_id: paper.source.id,
                                verification_status: STATUS_VERIFIED.to_owned(),
                                verification_notes: format!(
                                    "Official GATE {year} Physics (PH) paper retrieved from the GATE 2026 archive; managed PDF hash and page count verified."
                                ),
                            },
                            now,
                        )?;
                    }
                    let key_input = V2PyqSourceInput {
                        id: None,
                        exam_id: 2,
                        year,
                        paper_code: "PH".to_owned(),
                        source_type: SOURCE_TYPE_ANSWER_KEY.to_owned(),
                        authority: format!("GATE {year} Organizing Institute"),
                        institution: institution.to_owned(),
                        title: format!("GATE {year} Physics (PH) Final Answer Key"),
                        source_url: Some(key_url.to_owned()),
                        resource_id: None,
                        page_count: Some(key_pages),
                        published_at: None,
                        session_label: if year == 2021 { String::new() } else {
                            match year { 2022 => "5", 2023 => "6", 2024 => "2", 2025 => "7", 2026 => "2", _ => "" }.to_owned()
                        },
                        document_version: "final".to_owned(),
                        key_status: "final".to_owned(),
                        verification_status: None,
                        verification_notes: String::new(),
                        associated_source_id: Some(paper.source.id),
                    };
                    let key_source;
                    if year == 2021 {
                        key_source = crate::v2::practice::sources::acquire_source(
                            connection,
                            &data_dir,
                            V2AcquireSourceInput {
                                source: key_input,
                                file_path: raw_dir.join(key_file).display().to_string(),
                                storage_mode: "managed".to_owned(),
                            },
                            now,
                        )?;
                        set_source_status(
                            connection,
                            V2SourceStatusInput {
                                source_id: key_source.source.id,
                                verification_status: STATUS_NEEDS_REVIEW.to_owned(),
                                verification_notes: "Official answer-key PDF is image-only; OCR or manual transcription is required before key registration.".to_owned(),
                            },
                            now,
                        )?;
                        per_year.insert(
                            year.to_string(),
                            serde_json::json!({
                                "paperSourceId": paper.source.id,
                                "keySourceId": key_source.source.id,
                                "paperSha256": paper.source.sha256,
                                "paperSize": paper.source.size_bytes,
                                "keySha256": key_source.source.sha256,
                                "keySize": key_source.source.size_bytes,
                                "extraction": "pending_image_only",
                            }),
                        );
                        continue;
                    }
                    key_source = crate::v2::practice::sources::acquire_source(
                        connection,
                        &data_dir,
                        V2AcquireSourceInput {
                            source: key_input,
                            file_path: raw_dir.join(key_file).display().to_string(),
                            storage_mode: "managed".to_owned(),
                        },
                        now,
                    )?;
                    let key_text_name = match year {
                        2022 => "ph_2022_key.txt",
                        2023 => "ph_2023_key.txt",
                        2024 => "ph_2024_iisc_key.txt",
                        2025 => "ph_2025_key_a.txt",
                        2026 => "ph_2026_key.txt",
                        _ => unreachable!(),
                    };
                    let key_text = fs::read_to_string(text_dir.join(key_text_name))
                        .map_err(|error| AppError::file_operation("Answer-key text could not be read", error))?;
                    let preview = preview_answer_key_text(&key_text);
                    if preview.parsed_count != 65 {
                        return Err(AppError::validation(
                            "text",
                            format!("GATE {year} key parsed {} entries instead of 65.", preview.parsed_count),
                        ));
                    }
                    let existing_key = list_answer_keys(connection, Some(2), Some(year))?
                        .into_iter()
                        .find(|key| key.source_id == key_source.source.id && key.version_label == "final-archive");
                    let key = if let Some(existing) = existing_key {
                        existing
                    } else {
                        save_answer_key(
                            connection,
                            V2PyqAnswerKeyInput {
                                source_id: key_source.source.id,
                                version_label: "final-archive".to_owned(),
                                key_status: "final".to_owned(),
                                published_at: None,
                                verified: true,
                                verified_by: "local reviewer".to_owned(),
                                notes: format!("Final GATE {year} Physics (PH) answer key parsed from the official PDF text layer; 65 entries."),
                                supersedes_key_id: None,
                                text: key_text,
                                entries: Vec::new(),
                            },
                            now,
                        )?
                    };
                    let paper_text_name = match year {
                        2022 => "ph_2022.txt",
                        2023 => "ph_2023.txt",
                        2024 => "ph_2024_iisc_qp4.txt",
                        2025 => "ph_2025_a.txt",
                        2026 => "ph_2026.txt",
                        _ => unreachable!(),
                    };
                    let paper_text = fs::read_to_string(text_dir.join(paper_text_name))
                        .map_err(|error| AppError::file_operation("Question-paper text could not be read", error))?;
                    let extraction = extract_source(
                        connection,
                        V2ExtractionInput {
                            source_id: paper.source.id,
                            text: paper_text,
                            method: "pdf_text".to_owned(),
                            answer_key_id: Some(key.id),
                            notes: "Official archive text layer. Appended key pages are excluded from question extraction.".to_owned(),
                        },
                        now,
                    )?;
                    per_year.insert(
                        year.to_string(),
                        serde_json::json!({
                            "paperSourceId": paper.source.id,
                            "keySourceId": key_source.source.id,
                            "keyId": key.id,
                            "paperSha256": paper.source.sha256,
                            "paperSize": paper.source.size_bytes,
                            "keySha256": key_source.source.sha256,
                            "keySize": key_source.source.size_bytes,
                            "entries": key.entry_count,
                            "extracted": extraction.extracted,
                            "clean": extraction.clean,
                            "needsReview": extraction.needs_review,
                            "missingKey": extraction.missing_key,
                            "skippedExisting": extraction.skipped_existing,
                        }),
                    );
                }
                result.insert("years".to_owned(), serde_json::Value::Object(per_year));
                result.insert("counts".to_owned(), serde_json::to_value(counts(connection)?).unwrap_or(serde_json::Value::Null));
                if verify_selected {
                    let selected = [
                        (2022, vec![1, 6, 18, 52]),
                        (2023, vec![2, 4, 10, 12, 28, 29, 34]),
                        (2024, vec![14, 21]),
                        (2025, vec![1, 2, 4, 15, 20, 24, 25, 47]),
                        (2026, vec![3, 4, 5, 9, 35]),
                    ];
                    let mut verified = Vec::new();
                    for (year, numbers) in selected {
                        let candidates = list_candidates(
                            connection,
                            V2ExtractionCandidateFilter {
                                exam_id: Some(2),
                                year: Some(year),
                                limit: Some(500),
                                ..V2ExtractionCandidateFilter::default()
                            },
                        )?;
                        for number in numbers {
                            let candidate = candidates.iter().find(|candidate| candidate.question_number == number.to_string()).ok_or_else(|| {
                                AppError::validation("candidateId", format!("Missing GATE {year} Q{number} extraction candidate."))
                            })?;
                            if candidate.review_status == REVIEW_VERIFIED {
                                continue;
                            }
                            let mut statement = candidate.statement.clone();
                            let mut options = candidate.options.clone();
                            if year == 2023 && number == 28 {
                                statement = statement.replace("Joule -Thomson", "Joule-Thomson");
                                for option in &mut options {
                                    option.text = option.text.replace("Joule -Thomson", "Joule-Thomson");
                                }
                            } else if year == 2023 && number == 29 {
                                statement = statement
                                    .replace("d euteron", "deuteron")
                                    .replace("non - spherical", "non-spherical")
                                    .replace("3S1", "^3S_1")
                                    .replace("3D1", "^3D_1")
                                    .replace("3P1", "^3P_1");
                                for option in &mut options {
                                    option.text = option.text
                                        .replace("d euteron", "deuteron")
                                        .replace("non - spherical", "non-spherical")
                                        .replace("3S1", "^3S_1")
                                        .replace("3D1", "^3D_1")
                                        .replace("3P1", "^3P_1");
                                }
                            } else if year == 2023 && number == 34 {
                                statement = statement.replace("t he", "the");
                            } else if year == 2025 && number == 20 {
                                statement = statement.replace("are  similar", "are similar");
                            } else if year == 2025 && number == 47 {
                                statement = statement.replace("is o f", "is of");
                                for option in &mut options {
                                    option.text = option.text.replace("o f", "of");
                                }
                            }
                            update_candidate(
                                connection,
                                V2ExtractionCandidateUpdate {
                                    candidate_id: candidate.id,
                                    statement,
                                    options,
                                    correct_answer: candidate.correct_answer.clone(),
                                    correct_range: candidate.correct_range.clone(),
                                    max_marks: candidate.max_marks.unwrap_or(0.0),
                                    negative_marks: candidate.negative_marks,
                                    numeric_tolerance: candidate.numeric_tolerance,
                                    difficulty: candidate.difficulty,
                                    section_code: candidate.section_code.clone(),
                                    question_type_code: candidate.question_type_code.clone(),
                                    syllabus_node_id: candidate.syllabus_node_id,
                                    concept_ids: candidate.concept_ids.clone(),
                                    review_notes: if candidate.review_notes.is_empty() {
                                        "Text and key checked against the official page; approved for the initial verified subset.".to_owned()
                                    } else {
                                        format!("{} Text checked against the official page; approved for the initial verified subset.", candidate.review_notes)
                                    },
                                    reviewed_by: "local reviewer".to_owned(),
                                },
                                now,
                            )?;
                            verify_candidate(
                                connection,
                                V2CandidateDecisionInput {
                                    candidate_id: candidate.id,
                                    reviewed_by: "local reviewer".to_owned(),
                                    review_notes: "Approved only after source, key, page and text-quality review.".to_owned(),
                                },
                                now,
                            )?;
                            verified.push(format!("{year}-Q{number}"));
                        }
                    }
                    result.insert("verified".to_owned(), serde_json::json!(verified));
                }
                result.insert("finalCounts".to_owned(), serde_json::to_value(counts(connection)?).unwrap_or(serde_json::Value::Null));
                Ok(serde_json::Value::Object(result))
            })
            .expect("GATE import");
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).expect("summary json")
    );
}
