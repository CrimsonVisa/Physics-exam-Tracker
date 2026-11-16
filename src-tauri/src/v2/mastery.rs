use std::collections::HashMap;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter, types::Type};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{date_only, error::AppError};

pub const DEFAULT_EVIDENCE_LIMIT: i64 = 200;
pub const DEFAULT_HISTORY_LIMIT: i64 = 20;
const CONFIDENCE_RECENT_DAYS: i64 = 90;
const FLOAT_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptMasterySummary {
    pub concept_id: i64,
    pub mastery_score: f64,
    pub confidence: i64,
    pub evidence_count: i64,
    pub last_evidence_at: Option<String>,
    pub last_reviewed_on: Option<String>,
    pub last_recalculated_at: String,
    pub last_recalc_run_id: i64,
    pub algorithm_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ConceptEvidence {
    pub id: i64,
    pub source_kind: String,
    pub source_id: Option<i64>,
    pub source_version: i64,
    pub exam_id: Option<i64>,
    pub exam_code: Option<String>,
    pub exam_short_name: Option<String>,
    pub local_date: String,
    pub occurred_at: String,
    pub raw_value: Value,
    pub algorithm_version_id: i64,
    pub algorithm_version: String,
    pub notes: String,
    pub created_at: String,
    pub is_voided: bool,
    pub void_reason: Option<String>,
    pub voided_at: Option<String>,
    pub replacement_evidence_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MasteryRunSummary {
    pub id: i64,
    pub reason: String,
    pub algorithm_version_id: Option<i64>,
    pub algorithm_version: Option<String>,
    pub evidence_count: i64,
    pub evidence_fingerprint: String,
    pub resulting_score: f64,
    pub resulting_confidence: i64,
    pub started_at: String,
    pub finished_at: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MasteryUpdate {
    pub concept_id: i64,
    pub run_id: i64,
    pub mastery: V2ConceptMasterySummary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MasteryReplayVerification {
    pub run_id: i64,
    pub matches: bool,
    pub stored_score: f64,
    pub recomputed_score: f64,
    pub stored_confidence: i64,
    pub recomputed_confidence: i64,
    pub stored_fingerprint: String,
    pub recomputed_fingerprint: String,
    pub result_count: i64,
    pub mismatches: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2UpdateConceptMasteryInput {
    pub concept_id: i64,
    pub target_score: f64,
    pub local_date: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2EvidenceRecordInput {
    pub concept_id: i64,
    pub source_kind: String,
    #[serde(default)]
    pub source_id: Option<i64>,
    #[serde(default = "default_source_version")]
    pub source_version: i64,
    #[serde(default)]
    pub exam_id: Option<i64>,
    pub local_date: String,
    pub occurred_at: String,
    pub raw_value: Value,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ReplayConceptMasteryInput {
    pub concept_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2VerifyMasteryRunInput {
    pub run_id: i64,
}

fn default_source_version() -> i64 {
    1
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayReason {
    SourceCreated,
    SourceEdited,
    SourceDeleted,
    ManualOverride,
    AlgorithmChange,
    Repair,
}

impl ReplayReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceCreated => "source_created",
            Self::SourceEdited => "source_edited",
            Self::SourceDeleted => "source_deleted",
            Self::ManualOverride => "manual_override",
            Self::AlgorithmChange => "algorithm_change",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceKind {
    StudySession,
    Revision,
    PyqAttempt,
    MockConceptResult,
    Manual,
}

impl EvidenceKind {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "study_session" => Ok(Self::StudySession),
            "revision" => Ok(Self::Revision),
            "pyq_attempt" => Ok(Self::PyqAttempt),
            "mock_concept_result" => Ok(Self::MockConceptResult),
            "manual" => Ok(Self::Manual),
            _ => Err(AppError::validation(
                "sourceKind",
                "Choose a supported mastery evidence source.",
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::StudySession => "study_session",
            Self::Revision => "revision",
            Self::PyqAttempt => "pyq_attempt",
            Self::MockConceptResult => "mock_concept_result",
            Self::Manual => "manual",
        }
    }

    fn category(self) -> Category {
        match self {
            Self::StudySession => Category::Study,
            Self::Revision => Category::Revision,
            Self::PyqAttempt => Category::Pyq,
            Self::MockConceptResult => Category::Mock,
            Self::Manual => Category::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Study,
    Revision,
    Pyq,
    Mock,
    Manual,
}

impl Category {
    fn as_str(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Revision => "revision",
            Self::Pyq => "pyq",
            Self::Mock => "mock",
            Self::Manual => "manual",
        }
    }

    fn cap_index(self) -> Option<usize> {
        match self {
            Self::Study => Some(0),
            Self::Revision => Some(1),
            Self::Pyq => Some(2),
            Self::Mock => Some(3),
            Self::Manual => None,
        }
    }
}

#[derive(Debug, Clone)]
struct AlgorithmVersion {
    id: i64,
    version: String,
    parameters: AlgorithmParameters,
}

#[derive(Debug, Clone)]
struct AlgorithmParameters {
    study_cap: f64,
    revision_cap: f64,
    pyq_cap: f64,
    mock_cap: f64,
    total_positive_cap: f64,
    total_negative_cap: f64,
    study_per_minute_divisor: f64,
    study_max_base: f64,
    revision_deltas: HashMap<String, f64>,
    pyq_correct: f64,
    pyq_incorrect: f64,
    mock_correct: f64,
    mock_incorrect: f64,
    difficulty_multipliers: HashMap<i64, f64>,
}

impl AlgorithmParameters {
    fn parse(version: &str, text: &str) -> Result<Self, AppError> {
        let value: Value = serde_json::from_str(text).map_err(|error| {
            AppError::v2_integrity(format!(
                "Mastery algorithm version {version} has invalid parameters: {error}"
            ))
        })?;
        let study_cap = number_field(&value, "studyCap", version)?;
        let revision_cap = number_field(&value, "revisionCap", version)?;
        let pyq_cap = number_field(&value, "pyqCap", version)?;
        let mock_cap = number_field(&value, "mockCap", version)?;
        let total_positive_cap = number_field(&value, "totalPositiveCap", version)?;
        let total_negative_cap = number_field(&value, "totalNegativeCap", version)?;
        let study_per_minute_divisor = number_field(&value, "studyPerMinuteDivisor", version)?;
        let study_max_base = number_field(&value, "studyMaxBase", version)?;
        if [
            study_cap,
            revision_cap,
            pyq_cap,
            mock_cap,
            total_positive_cap,
            total_negative_cap,
            study_max_base,
        ]
        .iter()
        .any(|number| !number.is_finite() || *number < 0.0)
            || !study_per_minute_divisor.is_finite()
            || study_per_minute_divisor <= 0.0
        {
            return Err(AppError::v2_integrity(format!(
                "Mastery algorithm version {version} has out-of-range parameters."
            )));
        }

        let revision_deltas = object_number_map(&value, "revisionDeltas", version)?;
        for outcome in ["again", "difficult", "good", "easy"] {
            if !revision_deltas.contains_key(outcome) {
                return Err(AppError::v2_integrity(format!(
                    "Mastery algorithm version {version} is missing the {outcome} revision delta."
                )));
            }
        }

        let difficulty_map = object_number_map(&value, "difficultyMultipliers", version)?;
        let mut difficulty_multipliers = HashMap::new();
        for difficulty in 1..=5 {
            let key = difficulty.to_string();
            let multiplier = difficulty_map.get(&key).copied().ok_or_else(|| {
                AppError::v2_integrity(format!(
                    "Mastery algorithm version {version} is missing difficulty {difficulty}."
                ))
            })?;
            if !multiplier.is_finite() || multiplier <= 0.0 {
                return Err(AppError::v2_integrity(format!(
                    "Mastery algorithm version {version} has an invalid difficulty multiplier."
                )));
            }
            difficulty_multipliers.insert(difficulty, multiplier);
        }

        Ok(Self {
            study_cap,
            revision_cap,
            pyq_cap,
            mock_cap,
            total_positive_cap,
            total_negative_cap,
            study_per_minute_divisor,
            study_max_base,
            revision_deltas,
            pyq_correct: number_field(&value, "pyqCorrect", version)?,
            pyq_incorrect: number_field(&value, "pyqIncorrect", version)?,
            mock_correct: number_field(&value, "mockCorrect", version)?,
            mock_incorrect: number_field(&value, "mockIncorrect", version)?,
            difficulty_multipliers,
        })
    }

    fn category_caps(&self, category: Category) -> (f64, f64) {
        match category {
            Category::Study => (self.study_cap, 0.0),
            Category::Revision => (self.revision_cap, self.revision_cap),
            Category::Pyq => (self.pyq_cap, self.pyq_cap),
            Category::Mock => (self.mock_cap, self.mock_cap),
            Category::Manual => (0.0, 0.0),
        }
    }
}

fn number_field(value: &Value, field: &str, version: &str) -> Result<f64, AppError> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| {
            AppError::v2_integrity(format!(
                "Mastery algorithm version {version} is missing numeric parameter {field}."
            ))
        })
}

fn object_number_map(
    value: &Value,
    field: &str,
    version: &str,
) -> Result<HashMap<String, f64>, AppError> {
    let object = value.get(field).and_then(Value::as_object).ok_or_else(|| {
        AppError::v2_integrity(format!(
            "Mastery algorithm version {version} is missing object parameter {field}."
        ))
    })?;
    let mut result = HashMap::new();
    for (key, value) in object {
        let number = value
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| {
                AppError::v2_integrity(format!(
                    "Mastery algorithm version {version} has a non-numeric {field} entry."
                ))
            })?;
        result.insert(key.clone(), number);
    }
    Ok(result)
}

#[derive(Debug, Clone)]
struct EvidenceRow {
    id: i64,
    concept_id: i64,
    source_kind: String,
    source_id: Option<i64>,
    source_version: i64,
    exam_id: Option<i64>,
    local_date: String,
    occurred_at: String,
    raw_json: String,
    raw_value: Value,
    algorithm_version_id: i64,
    notes: String,
}

#[derive(Debug, Clone)]
struct EventBase {
    category: Category,
    base: f64,
    manual_target: Option<f64>,
}

#[derive(Debug, Clone)]
struct EvidenceResult {
    evidence_id: i64,
    algorithm_version_id: i64,
    applied_delta: f64,
    score_before: f64,
    score_after: f64,
    category: String,
    cap_consumed: f64,
}

#[derive(Debug, Clone)]
struct ReplayComputation {
    score: f64,
    confidence: i64,
    fingerprint: String,
    last_evidence_at: Option<String>,
    last_reviewed_on: Option<String>,
    results: Vec<EvidenceResult>,
}

#[derive(Debug, Clone, Default)]
struct CapPair {
    positive: f64,
    negative: f64,
}

#[derive(Debug, Clone, Default)]
struct DayCaps {
    categories: [CapPair; 4],
    total_positive: f64,
    total_negative: f64,
}

fn active_algorithm_version(connection: &Connection) -> Result<AlgorithmVersion, AppError> {
    let row = connection
        .query_row(
            "SELECT id, version, parameters_json
             FROM mastery_algorithm_versions
             WHERE is_active = 1
             ORDER BY id DESC
             LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            AppError::database("The active mastery algorithm could not be loaded", error)
        })?
        .ok_or_else(|| {
            AppError::v2_integrity("No active mastery algorithm version is configured.")
        })?;
    Ok(AlgorithmVersion {
        id: row.0,
        parameters: AlgorithmParameters::parse(&row.1, &row.2)?,
        version: row.1,
    })
}

pub fn load_mastery_summary(
    connection: &Connection,
    concept_id: i64,
) -> Result<Option<V2ConceptMasterySummary>, AppError> {
    Ok(load_mastery_summaries(connection, &[concept_id])?.remove(&concept_id))
}

pub fn load_mastery_summaries(
    connection: &Connection,
    concept_ids: &[i64],
) -> Result<HashMap<i64, V2ConceptMasterySummary>, AppError> {
    if concept_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = vec!["?"; concept_ids.len()].join(",");
    let sql = format!(
        "SELECT cm.concept_id, cm.mastery_score, cm.confidence, cm.evidence_count,
                cm.last_evidence_at, cm.last_reviewed_on, cm.last_recalculated_at,
                cm.last_recalc_run_id, mav.version
         FROM concept_mastery cm
         LEFT JOIN mastery_algorithm_versions mav ON mav.id = cm.last_algorithm_version_id
         WHERE cm.concept_id IN ({placeholders})"
    );
    let mut statement = connection.prepare(&sql).map_err(|error| {
        AppError::database("Concept mastery summaries could not be prepared", error)
    })?;
    let rows = statement
        .query_map(params_from_iter(concept_ids.iter().copied()), |row| {
            Ok(V2ConceptMasterySummary {
                concept_id: row.get(0)?,
                mastery_score: row.get(1)?,
                confidence: row.get(2)?,
                evidence_count: row.get(3)?,
                last_evidence_at: row.get(4)?,
                last_reviewed_on: row.get(5)?,
                last_recalculated_at: row.get(6)?,
                last_recalc_run_id: row.get(7)?,
                algorithm_version: row.get(8)?,
            })
        })
        .map_err(|error| {
            AppError::database("Concept mastery summaries could not be read", error)
        })?;
    let mut summaries = HashMap::new();
    for row in rows {
        let summary = row.map_err(|error| {
            AppError::database("A concept mastery summary row is invalid", error)
        })?;
        summaries.insert(summary.concept_id, summary);
    }
    Ok(summaries)
}

pub fn load_evidence(
    connection: &Connection,
    concept_id: i64,
    limit: i64,
) -> Result<Vec<V2ConceptEvidence>, AppError> {
    if !(1..=500).contains(&limit) {
        return Err(AppError::validation(
            "evidenceLimit",
            "Evidence limit must be between 1 and 500.",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.source_kind, e.source_id, e.source_version, e.exam_id,
                    ex.code, ex.short_name, e.local_date, e.occurred_at,
                    e.raw_value_json, e.algorithm_version_id, mav.version,
                    e.notes, e.created_at, v.evidence_id IS NOT NULL,
                    v.reason, v.voided_at, v.replacement_evidence_id
             FROM concept_evidence e
             LEFT JOIN exams ex ON ex.id = e.exam_id
             LEFT JOIN mastery_algorithm_versions mav ON mav.id = e.algorithm_version_id
             LEFT JOIN concept_evidence_voids v ON v.evidence_id = e.id
             WHERE e.concept_id = ?1
             ORDER BY e.occurred_at DESC, e.id DESC
             LIMIT ?2",
        )
        .map_err(|error| AppError::database("Concept evidence could not be prepared", error))?;
    let rows = statement
        .query_map(params![concept_id, limit], |row| {
            let raw_text: String = row.get(9)?;
            let raw_value = serde_json::from_str(&raw_text).map_err(|_| {
                rusqlite::Error::InvalidColumnType(9, "raw_value_json".to_owned(), Type::Text)
            })?;
            Ok(V2ConceptEvidence {
                id: row.get(0)?,
                source_kind: row.get(1)?,
                source_id: row.get(2)?,
                source_version: row.get(3)?,
                exam_id: row.get(4)?,
                exam_code: row.get(5)?,
                exam_short_name: row.get(6)?,
                local_date: row.get(7)?,
                occurred_at: row.get(8)?,
                raw_value,
                algorithm_version_id: row.get(10)?,
                algorithm_version: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                notes: row.get(12)?,
                created_at: row.get(13)?,
                is_voided: row.get::<_, i64>(14)? == 1,
                void_reason: row.get(15)?,
                voided_at: row.get(16)?,
                replacement_evidence_id: row.get(17)?,
            })
        })
        .map_err(|error| AppError::database("Concept evidence could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Concept evidence contains an invalid row", error))
}

pub fn load_history(
    connection: &Connection,
    concept_id: i64,
    limit: i64,
) -> Result<Vec<V2MasteryRunSummary>, AppError> {
    if !(1..=200).contains(&limit) {
        return Err(AppError::validation(
            "historyLimit",
            "Mastery history limit must be between 1 and 200.",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT r.id, r.reason, r.algorithm_version_id, mav.version,
                    r.evidence_count, r.evidence_fingerprint, r.resulting_score,
                    r.resulting_confidence, r.started_at, r.finished_at, r.notes
             FROM mastery_recalc_runs r
             LEFT JOIN mastery_algorithm_versions mav ON mav.id = r.algorithm_version_id
             WHERE r.concept_id = ?1
             ORDER BY r.finished_at DESC, r.id DESC
             LIMIT ?2",
        )
        .map_err(|error| AppError::database("Mastery history could not be prepared", error))?;
    let rows = statement
        .query_map(params![concept_id, limit], |row| {
            Ok(V2MasteryRunSummary {
                id: row.get(0)?,
                reason: row.get(1)?,
                algorithm_version_id: row.get(2)?,
                algorithm_version: row.get(3)?,
                evidence_count: row.get(4)?,
                evidence_fingerprint: row.get(5)?,
                resulting_score: row.get(6)?,
                resulting_confidence: row.get(7)?,
                started_at: row.get(8)?,
                finished_at: row.get(9)?,
                notes: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database("Mastery history could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Mastery history contains an invalid row", error))
}

pub fn update_manual_mastery(
    connection: &mut Connection,
    input: V2UpdateConceptMasteryInput,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    if !input.target_score.is_finite() || !(0.0..=100.0).contains(&input.target_score) {
        return Err(AppError::validation(
            "targetScore",
            "Manual mastery must be a number between 0 and 100.",
        ));
    }
    let local_date = date_only::parse_date_only(&input.local_date).ok_or_else(|| {
        AppError::validation(
            "localDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Evidence notes must be 2,000 characters or fewer.",
        ));
    }
    record_evidence(
        connection,
        V2EvidenceRecordInput {
            concept_id: input.concept_id,
            source_kind: EvidenceKind::Manual.as_str().to_owned(),
            source_id: None,
            source_version: 1,
            exam_id: None,
            local_date: date_only::format_date_only(local_date),
            occurred_at: format_utc(now),
            raw_value: json!({ "target_score": input.target_score }),
            notes: input.notes,
        },
        ReplayReason::ManualOverride,
        now,
    )
}

pub fn record_evidence(
    connection: &mut Connection,
    input: V2EvidenceRecordInput,
    reason: ReplayReason,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mastery evidence transaction could not start", error)
    })?;
    let update = record_evidence_in_transaction(&transaction, input, reason, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Mastery evidence could not be committed", error))?;
    Ok(update)
}

pub(crate) fn record_evidence_in_transaction(
    transaction: &Transaction<'_>,
    input: V2EvidenceRecordInput,
    reason: ReplayReason,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    let kind = EvidenceKind::parse(&input.source_kind)?;
    if input.source_version < 1 {
        return Err(AppError::validation(
            "sourceVersion",
            "Evidence source version must be at least 1.",
        ));
    }
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Evidence notes must be 2,000 characters or fewer.",
        ));
    }
    let local_date = date_only::parse_date_only(&input.local_date).ok_or_else(|| {
        AppError::validation(
            "localDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    let occurred_at = parse_timestamp(&input.occurred_at).ok_or_else(|| {
        AppError::validation("occurredAt", "Occurred time must be a valid timestamp.")
    })?;
    let raw_json = serde_json::to_string(&input.raw_value).map_err(|error| {
        AppError::validation(
            "rawValue",
            format!("The evidence value is invalid: {error}"),
        )
    })?;

    ensure_active_concept(transaction, input.concept_id)?;
    validate_source_reference(transaction, &input, kind)?;
    if let Some(exam_id) = input.exam_id {
        ensure_active_exam(transaction, exam_id)?;
    }
    let active = active_algorithm_version(transaction)?;
    let evidence = EvidenceRow {
        id: 0,
        concept_id: input.concept_id,
        source_kind: kind.as_str().to_owned(),
        source_id: input.source_id,
        source_version: input.source_version,
        exam_id: input.exam_id,
        local_date: date_only::format_date_only(local_date),
        occurred_at: format_utc(occurred_at),
        raw_json: raw_json.clone(),
        raw_value: input.raw_value.clone(),
        algorithm_version_id: active.id,
        notes: input.notes.clone(),
    };
    let _ = event_base_with_factor(&evidence, &active.parameters, 1.0)?;

    let now_text = format_utc(now);
    transaction
        .execute(
            "INSERT INTO concept_evidence (
                concept_id, source_kind, source_id, source_version, exam_id, local_date,
                occurred_at, raw_value_json, algorithm_version_id, notes, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                input.concept_id,
                kind.as_str(),
                input.source_id,
                input.source_version,
                input.exam_id,
                date_only::format_date_only(local_date),
                format_utc(occurred_at),
                raw_json,
                active.id,
                input.notes,
                now_text,
            ],
        )
        .map_err(|error| {
            if matches!(
                error.sqlite_error_code(),
                Some(rusqlite::ErrorCode::ConstraintViolation)
            ) {
                AppError::validation(
                    "evidence",
                    "This evidence already exists for the concept and source version.",
                )
            } else {
                AppError::database("Mastery evidence could not be saved", error)
            }
        })?;

    replay_in_transaction(transaction, input.concept_id, reason, now)
}

pub(crate) fn preview_revision_outcome(
    connection: &Connection,
    concept_id: i64,
    exam_id: Option<i64>,
    local_date: &str,
    occurred_at: DateTime<Utc>,
    outcome: &str,
    now: DateTime<Utc>,
) -> Result<f64, AppError> {
    let active = active_algorithm_version(connection)?;
    let mut evidence = load_active_evidence(connection, concept_id)?;
    let raw_value = json!({ "outcome": outcome });
    let raw_json = serde_json::to_string(&raw_value).map_err(|error| {
        AppError::validation(
            "outcome",
            format!("The revision outcome is invalid: {error}"),
        )
    })?;
    evidence.push(EvidenceRow {
        id: i64::MAX,
        concept_id,
        source_kind: EvidenceKind::Revision.as_str().to_owned(),
        source_id: Some(0),
        source_version: 1,
        exam_id,
        local_date: local_date.to_owned(),
        occurred_at: format_utc(occurred_at),
        raw_json,
        raw_value,
        algorithm_version_id: active.id,
        notes: String::new(),
    });
    let computation = compute_replay(connection, &evidence, now, |_| Ok(1.0))?;
    Ok(computation.score)
}

#[allow(dead_code)]
pub fn void_evidence(
    connection: &mut Connection,
    evidence_id: i64,
    reason: &str,
    replacement_evidence_id: Option<i64>,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    let replay_reason = match reason {
        "source_edited" => ReplayReason::SourceEdited,
        "source_deleted" => ReplayReason::SourceDeleted,
        "manual_correction" => ReplayReason::Repair,
        _ => {
            return Err(AppError::validation(
                "reason",
                "Choose source_edited, source_deleted, or manual_correction.",
            ));
        }
    };
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The evidence void transaction could not start", error)
    })?;
    let concept_id = transaction
        .query_row(
            "SELECT concept_id FROM concept_evidence WHERE id = ?1",
            [evidence_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Evidence could not be checked", error))?
        .ok_or_else(AppError::evidence_not_found)?;
    let already_voided: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concept_evidence_voids WHERE evidence_id = ?1)",
            [evidence_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Evidence void state could not be checked", error))?;
    if already_voided == 1 {
        return Err(AppError::validation(
            "evidenceId",
            "This evidence has already been voided.",
        ));
    }
    if let Some(replacement_id) = replacement_evidence_id {
        let replacement_concept = transaction
            .query_row(
                "SELECT concept_id FROM concept_evidence WHERE id = ?1",
                [replacement_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| {
                AppError::database("Replacement evidence could not be checked", error)
            })?
            .ok_or_else(AppError::evidence_not_found)?;
        if replacement_concept != concept_id {
            return Err(AppError::validation(
                "replacementEvidenceId",
                "Replacement evidence must belong to the same concept.",
            ));
        }
    }

    transaction
        .execute(
            "INSERT INTO concept_evidence_voids (
                evidence_id, voided_at, reason, replacement_evidence_id, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?2)",
            params![
                evidence_id,
                format_utc(now),
                reason,
                replacement_evidence_id
            ],
        )
        .map_err(|error| AppError::database("Evidence could not be voided", error))?;

    let update = replay_in_transaction(&transaction, concept_id, replay_reason, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Evidence void could not be committed", error))?;
    Ok(update)
}

pub fn replay_current(
    connection: &mut Connection,
    input: V2ReplayConceptMasteryInput,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mastery replay transaction could not start", error)
    })?;
    ensure_concept_exists(&transaction, input.concept_id)?;
    let update = replay_in_transaction(&transaction, input.concept_id, ReplayReason::Repair, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The mastery replay could not be committed", error))?;
    Ok(update)
}

pub fn verify_mastery_run(
    connection: &Connection,
    input: V2VerifyMasteryRunInput,
) -> Result<V2MasteryReplayVerification, AppError> {
    let run = connection
        .query_row(
            "SELECT concept_id, resulting_score, resulting_confidence, evidence_fingerprint,
                    finished_at, evidence_count
             FROM mastery_recalc_runs WHERE id = ?1",
            [input.run_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("The mastery run could not be loaded", error))?
        .ok_or_else(AppError::mastery_run_not_found)?;

    let finished_at = parse_timestamp(&run.4)
        .ok_or_else(|| AppError::v2_integrity("The mastery run timestamp is invalid."))?;
    let result_rows = {
        let mut statement = connection
            .prepare(
                "SELECT evidence_id, algorithm_version_id, applied_delta, score_before,
                        score_after, category, cap_consumed
                 FROM mastery_replay_results
                 WHERE recalc_run_id = ?1
                 ORDER BY id ASC",
            )
            .map_err(|error| AppError::database("Mastery results could not be prepared", error))?;
        let rows = statement
            .query_map([input.run_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, f64>(6)?,
                ))
            })
            .map_err(|error| AppError::database("Mastery results could not be read", error))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A mastery result row is invalid", error))?
    };

    let mut evidence = Vec::with_capacity(result_rows.len());
    for result in &result_rows {
        evidence.push(load_evidence_row_by_id(connection, result.0)?);
    }
    let computation = compute_replay(connection, &evidence, finished_at, |row| {
        pyq_factor(connection, row.concept_id, row.source_id)
    })?;

    let mut mismatches = Vec::new();
    if (computation.score - run.1).abs() > FLOAT_TOLERANCE {
        mismatches.push(format!(
            "Score differs: stored {}, recomputed {}.",
            run.1, computation.score
        ));
    }
    if computation.confidence != run.2 {
        mismatches.push(format!(
            "Confidence differs: stored {}, recomputed {}.",
            run.2, computation.confidence
        ));
    }
    if computation.fingerprint != run.3 {
        mismatches.push("Evidence fingerprint differs.".to_owned());
    }
    if run.5 != result_rows.len() as i64 {
        mismatches.push("Stored evidence count differs from the replay result count.".to_owned());
    }
    if evidence.iter().any(|row| row.concept_id != run.0) {
        mismatches.push("A replayed evidence row belongs to a different concept.".to_owned());
    }
    if result_rows.len() != computation.results.len() {
        mismatches.push("Replay result count differs.".to_owned());
    }
    for stored in &result_rows {
        let computed = computation
            .results
            .iter()
            .find(|result| result.evidence_id == stored.0);
        match computed {
            None => mismatches.push(format!("Evidence {} has no replayed result.", stored.0)),
            Some(computed) => {
                if computed.algorithm_version_id != stored.1
                    || (computed.applied_delta - stored.2).abs() > FLOAT_TOLERANCE
                    || (computed.score_before - stored.3).abs() > FLOAT_TOLERANCE
                    || (computed.score_after - stored.4).abs() > FLOAT_TOLERANCE
                    || computed.category != stored.5
                    || (computed.cap_consumed - stored.6).abs() > FLOAT_TOLERANCE
                {
                    mismatches.push(format!("Evidence {} replay result differs.", stored.0));
                }
            }
        }
    }

    Ok(V2MasteryReplayVerification {
        run_id: input.run_id,
        matches: mismatches.is_empty(),
        stored_score: run.1,
        recomputed_score: computation.score,
        stored_confidence: run.2,
        recomputed_confidence: computation.confidence,
        stored_fingerprint: run.3,
        recomputed_fingerprint: computation.fingerprint,
        result_count: result_rows.len() as i64,
        mismatches,
    })
}

fn replay_in_transaction(
    transaction: &Transaction<'_>,
    concept_id: i64,
    reason: ReplayReason,
    now: DateTime<Utc>,
) -> Result<V2MasteryUpdate, AppError> {
    let active = active_algorithm_version(transaction)?;
    let evidence = load_active_evidence(transaction, concept_id)?;
    let computation = compute_replay(transaction, &evidence, now, |row| {
        pyq_factor(transaction, row.concept_id, row.source_id)
    })?;

    let now_text = format_utc(now);
    transaction
        .execute(
            "INSERT INTO mastery_recalc_runs (
                concept_id, reason, algorithm_version_id, evidence_count,
                evidence_fingerprint, resulting_score, resulting_confidence,
                started_at, finished_at, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, '')",
            params![
                concept_id,
                reason.as_str(),
                active.id,
                computation.results.len() as i64,
                computation.fingerprint,
                computation.score,
                computation.confidence,
                now_text,
            ],
        )
        .map_err(|error| {
            AppError::database("The mastery recalculation run could not be saved", error)
        })?;
    let run_id = transaction.last_insert_rowid();

    for result in &computation.results {
        transaction
            .execute(
                "INSERT INTO mastery_replay_results (
                    recalc_run_id, evidence_id, concept_id, algorithm_version_id,
                    applied_delta, score_before, score_after, category, cap_consumed,
                    created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    run_id,
                    result.evidence_id,
                    concept_id,
                    result.algorithm_version_id,
                    result.applied_delta,
                    result.score_before,
                    result.score_after,
                    result.category,
                    result.cap_consumed,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("A mastery replay result could not be saved", error)
            })?;
    }

    transaction
        .execute(
            "INSERT INTO concept_mastery (
                concept_id, mastery_score, confidence, last_reviewed_on, last_evidence_at,
                evidence_count, last_algorithm_version_id, last_recalc_run_id,
                last_recalculated_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(concept_id) DO UPDATE SET
                mastery_score = excluded.mastery_score,
                confidence = excluded.confidence,
                last_reviewed_on = excluded.last_reviewed_on,
                last_evidence_at = excluded.last_evidence_at,
                evidence_count = excluded.evidence_count,
                last_algorithm_version_id = excluded.last_algorithm_version_id,
                last_recalc_run_id = excluded.last_recalc_run_id,
                last_recalculated_at = excluded.last_recalculated_at,
                updated_at = excluded.updated_at",
            params![
                concept_id,
                computation.score,
                computation.confidence,
                computation.last_reviewed_on,
                computation.last_evidence_at,
                computation.results.len() as i64,
                active.id,
                run_id,
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Concept mastery could not be updated", error))?;

    Ok(V2MasteryUpdate {
        concept_id,
        run_id,
        mastery: V2ConceptMasterySummary {
            concept_id,
            mastery_score: computation.score,
            confidence: computation.confidence,
            evidence_count: computation.results.len() as i64,
            last_evidence_at: computation.last_evidence_at,
            last_reviewed_on: computation.last_reviewed_on,
            last_recalculated_at: now_text,
            last_recalc_run_id: run_id,
            algorithm_version: Some(active.version),
        },
    })
}

fn compute_replay<F>(
    connection: &Connection,
    evidence: &[EvidenceRow],
    now: DateTime<Utc>,
    mut pyq_factor_for: F,
) -> Result<ReplayComputation, AppError>
where
    F: FnMut(&EvidenceRow) -> Result<f64, AppError>,
{
    let mut versions: HashMap<i64, AlgorithmVersion> = HashMap::new();
    let mut score = 0.0_f64;
    let mut caps: HashMap<String, DayCaps> = HashMap::new();
    let mut results = Vec::with_capacity(evidence.len());

    for row in evidence {
        let version = if let Some(version) = versions.get(&row.algorithm_version_id) {
            version.clone()
        } else {
            let version = load_algorithm_version(connection, row.algorithm_version_id)?;
            versions.insert(version.id, version.clone());
            version
        };
        let factor = if row.source_kind == EvidenceKind::PyqAttempt.as_str() {
            pyq_factor_for(row)?
        } else {
            1.0
        };
        let event = event_base_with_factor(row, &version.parameters, factor)?;
        let score_before = score;
        let (applied_delta, cap_consumed);

        if event.category == Category::Manual {
            let target = event.manual_target.ok_or_else(|| {
                AppError::v2_integrity("Manual evidence is missing its target score.")
            })?;
            score = target;
            applied_delta = target - score_before;
            cap_consumed = 0.0;
        } else {
            let index = event.category.cap_index().ok_or_else(|| {
                AppError::v2_integrity("Evidence category is missing a cap index.")
            })?;
            let (positive_cap, negative_cap) = version.parameters.category_caps(event.category);
            let day = caps.entry(row.local_date.clone()).or_default();

            if event.base > 0.0 {
                let category_remaining = (positive_cap - day.categories[index].positive).max(0.0);
                let total_remaining =
                    (version.parameters.total_positive_cap - day.total_positive).max(0.0);
                let applied = event.base.min(category_remaining).min(total_remaining);
                day.categories[index].positive += applied;
                day.total_positive += applied;
                applied_delta = applied;
                cap_consumed = applied;
                score += applied * (1.0 - score / 100.0);
            } else if event.base < 0.0 {
                let category_remaining = (negative_cap - day.categories[index].negative).max(0.0);
                let total_remaining =
                    (version.parameters.total_negative_cap - day.total_negative).max(0.0);
                let applied = event.base.max(-category_remaining).max(-total_remaining);
                day.categories[index].negative += -applied;
                day.total_negative += -applied;
                applied_delta = applied;
                cap_consumed = -applied;
                score += applied * (score / 100.0);
            } else {
                applied_delta = 0.0;
                cap_consumed = 0.0;
            }
        }

        score = score.clamp(0.0, 100.0);
        results.push(EvidenceResult {
            evidence_id: row.id,
            algorithm_version_id: row.algorithm_version_id,
            applied_delta,
            score_before,
            score_after: score,
            category: event.category.as_str().to_owned(),
            cap_consumed,
        });
    }

    let latest_occurred = evidence
        .iter()
        .filter_map(|row| parse_timestamp(&row.occurred_at))
        .max();
    let last_reviewed_on = evidence.iter().map(|row| row.local_date.clone()).max();
    let confidence = confidence_for(evidence.len() as i64, latest_occurred, now);

    Ok(ReplayComputation {
        score,
        confidence,
        fingerprint: evidence_fingerprint(evidence),
        last_evidence_at: latest_occurred.map(format_utc),
        last_reviewed_on,
        results,
    })
}

fn event_base_with_factor(
    evidence: &EvidenceRow,
    parameters: &AlgorithmParameters,
    pyq_factor: f64,
) -> Result<EventBase, AppError> {
    let kind = EvidenceKind::parse(&evidence.source_kind)?;
    let category = kind.category();
    let value = &evidence.raw_value;

    let event = match kind {
        EvidenceKind::StudySession => {
            let duration = value
                .get("duration_seconds")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::validation(
                        "rawValue",
                        "Study evidence requires an integer duration_seconds.",
                    )
                })?;
            if !(1..=86_400).contains(&duration) {
                return Err(AppError::validation(
                    "rawValue",
                    "Study evidence duration_seconds must be between 1 and 86,400.",
                ));
            }
            let divisor_seconds = parameters.study_per_minute_divisor * 60.0;
            let base = ((duration as f64) / divisor_seconds)
                .floor()
                .max(1.0)
                .min(parameters.study_max_base);
            EventBase {
                category,
                base,
                manual_target: None,
            }
        }
        EvidenceKind::Revision => {
            let outcome = value
                .get("outcome")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AppError::validation("rawValue", "Revision evidence requires an outcome.")
                })?;
            let base = parameters
                .revision_deltas
                .get(outcome)
                .copied()
                .ok_or_else(|| {
                    AppError::validation(
                        "rawValue",
                        "Revision outcome must be again, difficult, good, or easy.",
                    )
                })?;
            EventBase {
                category,
                base,
                manual_target: None,
            }
        }
        EvidenceKind::PyqAttempt => {
            let status = value.get("status").and_then(Value::as_str).ok_or_else(|| {
                AppError::validation("rawValue", "PYQ evidence requires a status.")
            })?;
            let difficulty = value
                .get("difficulty")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::validation("rawValue", "PYQ evidence requires a difficulty 1-5.")
                })?;
            if !(1..=5).contains(&difficulty) {
                return Err(AppError::validation(
                    "rawValue",
                    "PYQ difficulty must be between 1 and 5.",
                ));
            }
            let base = match status {
                "correct" => parameters.pyq_correct,
                "incorrect" => parameters.pyq_incorrect,
                "skipped" => 0.0,
                _ => {
                    return Err(AppError::validation(
                        "rawValue",
                        "PYQ status must be correct, incorrect, or skipped.",
                    ));
                }
            };
            let multiplier = parameters
                .difficulty_multipliers
                .get(&difficulty)
                .copied()
                .ok_or_else(|| {
                    AppError::v2_integrity("The PYQ difficulty multiplier is missing.")
                })?;
            EventBase {
                category,
                base: base * multiplier * pyq_factor,
                manual_target: None,
            }
        }
        EvidenceKind::MockConceptResult => {
            let correct = value
                .get("correct")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::validation("rawValue", "Mock evidence requires correct >= 0.")
                })?;
            let incorrect = value
                .get("incorrect")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    AppError::validation("rawValue", "Mock evidence requires incorrect >= 0.")
                })?;
            if correct < 0 || incorrect < 0 {
                return Err(AppError::validation(
                    "rawValue",
                    "Mock evidence counts cannot be negative.",
                ));
            }
            EventBase {
                category,
                base: correct as f64 * parameters.mock_correct
                    + incorrect as f64 * parameters.mock_incorrect,
                manual_target: None,
            }
        }
        EvidenceKind::Manual => {
            let target = value
                .get("target_score")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    AppError::validation("rawValue", "Manual evidence requires target_score.")
                })?;
            if !target.is_finite() || !(0.0..=100.0).contains(&target) {
                return Err(AppError::validation(
                    "rawValue",
                    "Manual evidence target_score must be between 0 and 100.",
                ));
            }
            EventBase {
                category,
                base: 0.0,
                manual_target: Some(target),
            }
        }
    };

    if !event.base.is_finite() || !pyq_factor.is_finite() || pyq_factor <= 0.0 {
        return Err(AppError::v2_integrity(
            "The mastery evidence produced an invalid base delta.",
        ));
    }
    Ok(event)
}

fn load_active_evidence(
    connection: &Connection,
    concept_id: i64,
) -> Result<Vec<EvidenceRow>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.concept_id, e.source_kind, e.source_id, e.source_version,
                    e.exam_id, e.local_date, e.occurred_at, e.raw_value_json,
                    e.algorithm_version_id, e.notes
             FROM concept_evidence e
             WHERE e.concept_id = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM concept_evidence_voids v WHERE v.evidence_id = e.id
               )
             ORDER BY e.occurred_at ASC, e.id ASC",
        )
        .map_err(|error| {
            AppError::database("Active concept evidence could not be prepared", error)
        })?;
    let rows = statement
        .query_map([concept_id], map_evidence_row)
        .map_err(|error| AppError::database("Active concept evidence could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|error| {
        AppError::database("Active concept evidence contains an invalid row", error)
    })
}

fn load_evidence_row_by_id(
    connection: &Connection,
    evidence_id: i64,
) -> Result<EvidenceRow, AppError> {
    connection
        .query_row(
            "SELECT id, concept_id, source_kind, source_id, source_version, exam_id,
                    local_date, occurred_at, raw_value_json, algorithm_version_id, notes
             FROM concept_evidence WHERE id = ?1",
            [evidence_id],
            map_evidence_row,
        )
        .optional()
        .map_err(|error| AppError::database("Historical evidence could not be loaded", error))?
        .ok_or_else(AppError::evidence_not_found)
}

fn map_evidence_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceRow> {
    let raw_text: String = row.get(8)?;
    let raw_value: Value = serde_json::from_str(&raw_text).map_err(|_| {
        rusqlite::Error::InvalidColumnType(8, "raw_value_json".to_owned(), Type::Text)
    })?;
    let raw_json = serde_json::to_string(&raw_value).map_err(|_| {
        rusqlite::Error::InvalidColumnType(8, "raw_value_json".to_owned(), Type::Text)
    })?;
    let occurred_text: String = row.get(7)?;
    let occurred_at = parse_timestamp(&occurred_text).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(7, "occurred_at".to_owned(), Type::Text)
    })?;
    Ok(EvidenceRow {
        id: row.get(0)?,
        concept_id: row.get(1)?,
        source_kind: row.get(2)?,
        source_id: row.get(3)?,
        source_version: row.get(4)?,
        exam_id: row.get(5)?,
        local_date: row.get(6)?,
        occurred_at: format_utc(occurred_at),
        raw_json,
        raw_value,
        algorithm_version_id: row.get(9)?,
        notes: row.get(10)?,
    })
}

fn load_algorithm_version(
    connection: &Connection,
    version_id: i64,
) -> Result<AlgorithmVersion, AppError> {
    let row = connection
        .query_row(
            "SELECT id, version, parameters_json
             FROM mastery_algorithm_versions WHERE id = ?1",
            [version_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            AppError::database("Mastery algorithm version could not be loaded", error)
        })?
        .ok_or_else(|| {
            AppError::v2_integrity("Mastery evidence references a missing algorithm version.")
        })?;
    Ok(AlgorithmVersion {
        id: row.0,
        parameters: AlgorithmParameters::parse(&row.1, &row.2)?,
        version: row.1,
    })
}

fn pyq_factor(
    connection: &Connection,
    concept_id: i64,
    source_id: Option<i64>,
) -> Result<f64, AppError> {
    let question_id = source_id.ok_or_else(|| {
        AppError::v2_integrity("PYQ evidence is missing its question source identifier.")
    })?;
    let (total, weight): (f64, f64) = connection
        .query_row(
            "SELECT COALESCE(SUM(weight), 0),
                    COALESCE(MAX(CASE WHEN concept_id = ?2 THEN weight ELSE 0 END), 0)
             FROM pyq_question_concepts
             WHERE question_id = ?1",
            params![question_id, concept_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| AppError::database("PYQ concept mapping could not be loaded", error))?;
    if total <= 0.0 || weight <= 0.0 {
        return Ok(1.0);
    }
    Ok(weight / total)
}

fn confidence_for(
    evidence_count: i64,
    latest_occurred_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> i64 {
    match evidence_count {
        0..=1 => 1,
        2..=3 => 2,
        4..=6 => 3,
        7..=12 => 4,
        _ => {
            let recent = latest_occurred_at
                .map(|latest| (now - latest).num_days() <= CONFIDENCE_RECENT_DAYS)
                .unwrap_or(false);
            if recent { 5 } else { 4 }
        }
    }
}

fn evidence_fingerprint(evidence: &[EvidenceRow]) -> String {
    // The specification requires sha256(concat ordered active evidence fields)
    // but does not prescribe a byte layout. This canonical layout is stable:
    // rows are separated by 0x1e, fields by 0x1f, null ids are empty strings,
    // JSON is canonicalized first, and hashing an empty evidence set hashes
    // the empty byte string.
    let mut hasher = Sha256::new();
    for (index, row) in evidence.iter().enumerate() {
        if index > 0 {
            hasher.update([0x1e]);
        }
        let fields = [
            row.id.to_string(),
            row.concept_id.to_string(),
            row.source_kind.clone(),
            row.source_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.source_version.to_string(),
            row.exam_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.local_date.clone(),
            row.occurred_at.clone(),
            row.raw_json.clone(),
            row.algorithm_version_id.to_string(),
            row.notes.clone(),
        ];
        hasher.update(fields.join("\u{1f}").as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn ensure_concept_exists(connection: &Connection, concept_id: i64) -> Result<(), AppError> {
    if concept_id <= 0 {
        return Err(AppError::concept_not_found());
    }
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1)",
            [concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Concept could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::concept_not_found());
    }
    Ok(())
}

fn ensure_active_concept(connection: &Connection, concept_id: i64) -> Result<(), AppError> {
    let archived = connection
        .query_row(
            "SELECT is_archived FROM concepts WHERE id = ?1",
            [concept_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Concept could not be checked", error))?
        .ok_or_else(AppError::concept_not_found)?;
    if archived == 1 {
        return Err(AppError::validation(
            "conceptId",
            "Archived concepts cannot receive new mastery evidence.",
        ));
    }
    Ok(())
}

fn ensure_active_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    let active = connection
        .query_row(
            "SELECT is_active FROM exam_profiles WHERE exam_id = ?1",
            [exam_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Examination could not be checked", error))?
        .ok_or_else(|| {
            AppError::validation("examId", "The selected examination does not exist.")
        })?;
    if active != 1 {
        return Err(AppError::forbidden(
            "examId",
            "Inactive examinations cannot receive new mastery evidence.",
        ));
    }
    Ok(())
}

fn validate_source_reference(
    connection: &Connection,
    input: &V2EvidenceRecordInput,
    kind: EvidenceKind,
) -> Result<(), AppError> {
    match kind {
        EvidenceKind::Manual => {
            if input.source_id.is_some() {
                return Err(AppError::validation(
                    "sourceId",
                    "Manual evidence does not use a source identifier.",
                ));
            }
            if input.exam_id.is_some() {
                return Err(AppError::validation(
                    "examId",
                    "Manual mastery evidence is shared and has no examination.",
                ));
            }
        }
        EvidenceKind::StudySession => {
            let source_id = required_source_id(input)?;
            let (session_concept, primary_exam) = connection
                .query_row(
                    "SELECT concept_id, primary_exam_id FROM study_sessions WHERE id = ?1",
                    [source_id],
                    |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
                )
                .optional()
                .map_err(|error| {
                    AppError::database("Study session evidence source could not be checked", error)
                })?
                .ok_or_else(|| {
                    AppError::validation("sourceId", "The study session was not found.")
                })?;
            if session_concept != Some(input.concept_id) {
                return Err(AppError::validation(
                    "sourceId",
                    "The study session is not linked to this concept, so it cannot create study evidence.",
                ));
            }
            if let Some(exam_id) = input.exam_id
                && primary_exam != Some(exam_id)
            {
                let allocated: i64 = connection
                    .query_row(
                        "SELECT EXISTS(
                            SELECT 1 FROM session_exam_associations
                            WHERE session_id = ?1 AND exam_id = ?2
                            UNION ALL
                            SELECT 1 FROM session_exam_allocations
                            WHERE session_id = ?1 AND exam_id = ?2
                         )",
                        params![source_id, exam_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| {
                        AppError::database("Session exam allocation could not be checked", error)
                    })?;
                if allocated == 0 {
                    return Err(AppError::validation(
                        "examId",
                        "The study session is not allocated to this examination.",
                    ));
                }
            }
        }
        EvidenceKind::Revision => {
            let source_id = required_source_id(input)?;
            let (revision_concept, trigger_exam) = connection
                .query_row(
                    "SELECT concept_id, trigger_exam_id FROM revision_history WHERE id = ?1",
                    [source_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
                )
                .optional()
                .map_err(|error| {
                    AppError::database("Revision evidence source could not be checked", error)
                })?
                .ok_or_else(|| {
                    AppError::validation("sourceId", "The revision history row was not found.")
                })?;
            if revision_concept != input.concept_id {
                return Err(AppError::validation(
                    "sourceId",
                    "The revision history row belongs to a different concept.",
                ));
            }
            if let (Some(exam_id), Some(trigger_exam_id)) = (input.exam_id, trigger_exam)
                && exam_id != trigger_exam_id
            {
                return Err(AppError::validation(
                    "examId",
                    "The revision history row belongs to a different examination.",
                ));
            }
        }
        EvidenceKind::PyqAttempt => {
            let source_id = required_source_id(input)?;
            let (attempt_exam, question_id) = connection
                .query_row(
                    "SELECT exam_id, question_id FROM pyq_attempts WHERE id = ?1",
                    [source_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|error| {
                    AppError::database("PYQ evidence source could not be checked", error)
                })?
                .ok_or_else(|| {
                    AppError::validation("sourceId", "The PYQ attempt was not found.")
                })?;
            if input.exam_id != Some(attempt_exam) {
                return Err(AppError::validation(
                    "examId",
                    "PYQ evidence must use the examination of the attempt.",
                ));
            }
            let mapped: i64 = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM pyq_question_concepts
                        WHERE exam_id = ?1 AND question_id = ?2 AND concept_id = ?3
                     )",
                    params![attempt_exam, question_id, input.concept_id],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    AppError::database("PYQ concept mapping could not be checked", error)
                })?;
            if mapped == 0 {
                return Err(AppError::validation(
                    "sourceId",
                    "The PYQ question is not mapped to this concept.",
                ));
            }
        }
        EvidenceKind::MockConceptResult => {
            let source_id = required_source_id(input)?;
            let (mock_exam, mock_concept) = connection
                .query_row(
                    "SELECT exam_id, concept_id FROM mock_test_concept_results WHERE id = ?1",
                    [source_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()
                .map_err(|error| {
                    AppError::database("Mock evidence source could not be checked", error)
                })?
                .ok_or_else(|| {
                    AppError::validation("sourceId", "The mock concept result was not found.")
                })?;
            if mock_concept != input.concept_id {
                return Err(AppError::validation(
                    "sourceId",
                    "The mock concept result belongs to a different concept.",
                ));
            }
            if input.exam_id != Some(mock_exam) {
                return Err(AppError::validation(
                    "examId",
                    "Mock evidence must use the examination of the mock result.",
                ));
            }
        }
    }
    Ok(())
}

fn required_source_id(input: &V2EvidenceRecordInput) -> Result<i64, AppError> {
    input.source_id.filter(|value| *value > 0).ok_or_else(|| {
        AppError::validation(
            "sourceId",
            "This evidence source requires a positive source identifier.",
        )
    })
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn concept_id(state: &V2State, slug: &str) -> i64 {
        state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT id FROM concepts WHERE slug = ?1", [slug], |row| {
                        row.get(0)
                    })
                    .optional()
                    .map_err(|error| AppError::database("test concept", error))?
                    .ok_or_else(AppError::concept_not_found)
            })
            .expect("concept id")
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("base timestamp")
            .with_timezone(&Utc)
            + chrono::Duration::seconds(seconds)
    }

    fn evidence(
        concept_id: i64,
        source_kind: &str,
        source_id: Option<i64>,
        source_version: i64,
        local_date: &str,
        occurred_at: &str,
        raw_value: Value,
    ) -> V2EvidenceRecordInput {
        V2EvidenceRecordInput {
            concept_id,
            source_kind: source_kind.to_owned(),
            source_id,
            source_version,
            exam_id: None,
            local_date: local_date.to_owned(),
            occurred_at: occurred_at.to_owned(),
            raw_value,
            notes: String::new(),
        }
    }

    fn synthetic_evidence(
        id: i64,
        concept_id: i64,
        source_kind: &str,
        raw_value: Value,
    ) -> EvidenceRow {
        let raw_json = serde_json::to_string(&raw_value).expect("raw value");
        EvidenceRow {
            id,
            concept_id,
            source_kind: source_kind.to_owned(),
            source_id: Some(id),
            source_version: 1,
            exam_id: Some(1),
            local_date: "2026-11-04".to_owned(),
            occurred_at: format!("2026-11-04T10:{:02}:00.000Z", id),
            raw_json,
            raw_value,
            algorithm_version_id: 1,
            notes: String::new(),
        }
    }

    #[test]
    fn study_cap_formula_matches_the_worked_example() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let computation = state
            .with_connection(|connection| {
                let evidence = vec![
                    synthetic_evidence(
                        1,
                        concept,
                        "study_session",
                        json!({ "duration_seconds": 3600 }),
                    ),
                    synthetic_evidence(
                        2,
                        concept,
                        "study_session",
                        json!({ "duration_seconds": 7200 }),
                    ),
                    synthetic_evidence(
                        3,
                        concept,
                        "study_session",
                        json!({ "duration_seconds": 3600 }),
                    ),
                ];
                compute_replay(connection, &evidence, at(0), |_| Ok(1.0))
            })
            .expect("study replay");
        assert!((computation.score - 9.76).abs() < 1e-9);
        assert_eq!(computation.confidence, 2);
        assert_eq!(computation.results.len(), 3);
        assert!((computation.results[0].applied_delta - 4.0).abs() < 1e-9);
        assert!((computation.results[1].applied_delta - 6.0).abs() < 1e-9);
        assert!((computation.results[2].applied_delta - 0.0).abs() < 1e-9);
    }

    #[test]
    fn positive_negative_and_manual_formula_matches_the_worked_example() {
        let (_root, state) = state();
        let concept = concept_id(&state, "entropy");
        let computation = state
            .with_connection(|connection| {
                let evidence = vec![
                    synthetic_evidence(1, concept, "manual", json!({ "target_score": 50 })),
                    synthetic_evidence(2, concept, "revision", json!({ "outcome": "good" })),
                    synthetic_evidence(
                        3,
                        concept,
                        "pyq_attempt",
                        json!({ "status": "incorrect", "difficulty": 3 }),
                    ),
                    synthetic_evidence(
                        4,
                        concept,
                        "pyq_attempt",
                        json!({ "status": "correct", "difficulty": 3 }),
                    ),
                ];
                compute_replay(connection, &evidence, at(0), |_| Ok(1.0))
            })
            .expect("mixed replay");
        assert!((computation.score - 52.888).abs() < 1e-9);
        assert_eq!(computation.confidence, 3);
        assert!((computation.results[0].score_after - 50.0).abs() < 1e-9);
        assert!((computation.results[1].score_after - 52.5).abs() < 1e-9);
        assert!((computation.results[2].score_after - 50.925).abs() < 1e-9);
        assert!((computation.results[3].score_after - 52.888).abs() < 1e-9);
    }

    #[test]
    fn pyq_and_mock_base_deltas_follow_the_formula() {
        let (_root, state) = state();
        let version = state
            .with_connection(|connection| active_algorithm_version(connection))
            .expect("active version");
        let concept = concept_id(&state, "wave-particle-duality");
        let base = |kind: &str, raw: Value, factor: f64| {
            let row = EvidenceRow {
                id: 1,
                concept_id: concept,
                source_kind: kind.to_owned(),
                source_id: Some(1),
                source_version: 1,
                exam_id: Some(1),
                local_date: "2026-11-04".to_owned(),
                occurred_at: "2026-11-04T10:00:00.000Z".to_owned(),
                raw_json: serde_json::to_string(&raw).unwrap(),
                raw_value: raw,
                algorithm_version_id: version.id,
                notes: String::new(),
            };
            event_base_with_factor(&row, &version.parameters, factor)
                .expect("event base")
                .base
        };

        assert!(
            (base(
                "pyq_attempt",
                json!({"status":"correct","difficulty":3}),
                1.0
            ) - 4.0)
                .abs()
                < 1e-9
        );
        assert!(
            (base(
                "pyq_attempt",
                json!({"status":"incorrect","difficulty":3}),
                1.0
            ) + 3.0)
                .abs()
                < 1e-9
        );
        assert!(
            (base(
                "pyq_attempt",
                json!({"status":"correct","difficulty":5}),
                0.5
            ) - 2.5)
                .abs()
                < 1e-9
        );
        assert!(
            (base(
                "mock_concept_result",
                json!({"correct":3,"incorrect":2}),
                1.0
            ) - 5.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn confidence_boundaries_and_freshness_are_exact() {
        let now = at(0);
        let recent = at(-10);
        let old = at(-200 * 24 * 60 * 60);
        assert_eq!(confidence_for(0, None, now), 1);
        assert_eq!(confidence_for(1, Some(recent), now), 1);
        assert_eq!(confidence_for(2, Some(recent), now), 2);
        assert_eq!(confidence_for(3, Some(recent), now), 2);
        assert_eq!(confidence_for(4, Some(recent), now), 3);
        assert_eq!(confidence_for(6, Some(recent), now), 3);
        assert_eq!(confidence_for(7, Some(recent), now), 4);
        assert_eq!(confidence_for(12, Some(recent), now), 4);
        assert_eq!(confidence_for(13, Some(recent), now), 5);
        assert_eq!(confidence_for(13, Some(old), now), 4);
    }

    #[test]
    fn duplicate_evidence_is_rejected() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let input = evidence(
            concept,
            "study_session",
            Some(1),
            1,
            "2026-11-04",
            "2026-11-04T10:00:00.000Z",
            json!({ "duration_seconds": 900 }),
        );
        // A study source must be explicitly linked to the concept.
        let error = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    input.clone(),
                    ReplayReason::SourceCreated,
                    at(0),
                )
            })
            .expect_err("unlinked study source must fail");
        assert_eq!(error.code, "validation_error");

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, concept_id, created_at, updated_at
                         ) VALUES ('2026-11-04', 900, 'manual', ?1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [concept],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                Ok(())
            })
            .expect("session");

        let first = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    input.clone(),
                    ReplayReason::SourceCreated,
                    at(0),
                )
            })
            .expect("first evidence");
        assert_eq!(first.mastery.evidence_count, 1);

        let duplicate = state
            .with_connection(|connection| {
                record_evidence(connection, input, ReplayReason::SourceCreated, at(1))
            })
            .expect_err("duplicate evidence must fail");
        assert_eq!(duplicate.field, Some("evidence"));
    }

    #[test]
    fn evidence_void_and_replacement_replay_deterministically() {
        let (_root, state) = state();
        let concept = concept_id(&state, "entropy");
        let first = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    evidence(
                        concept,
                        "manual",
                        None,
                        1,
                        "2026-11-04",
                        "2026-11-04T10:00:00.000Z",
                        json!({ "target_score": 20 }),
                    ),
                    ReplayReason::SourceCreated,
                    at(0),
                )
            })
            .expect("first");
        let evidence_id = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM concept_evidence WHERE concept_id = ?1 ORDER BY id DESC LIMIT 1",
                        [concept],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("test evidence", error))
            })
            .expect("evidence id");
        let edited = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    evidence(
                        concept,
                        "manual",
                        None,
                        2,
                        "2026-11-04",
                        "2026-11-04T11:00:00.000Z",
                        json!({ "target_score": 80 }),
                    ),
                    ReplayReason::SourceEdited,
                    at(1),
                )
            })
            .expect("replacement");
        assert!(edited.mastery.mastery_score > first.mastery.mastery_score);

        let voided = state
            .with_connection(|connection| {
                void_evidence(connection, evidence_id, "source_edited", None, at(2))
            })
            .expect("void");
        assert_eq!(voided.mastery.evidence_count, 1);
        assert!((voided.mastery.mastery_score - 80.0).abs() < 1e-9);
    }

    #[test]
    fn empty_evidence_replay_records_zero_without_fabricating_a_score() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let update = state
            .with_connection(|connection| {
                replay_current(
                    connection,
                    V2ReplayConceptMasteryInput {
                        concept_id: concept,
                    },
                    at(0),
                )
            })
            .expect("empty replay");
        assert_eq!(update.mastery.evidence_count, 0);
        assert_eq!(update.mastery.mastery_score, 0.0);
        assert_eq!(update.mastery.confidence, 1);
    }

    #[test]
    fn current_replay_is_deterministic_and_history_verification_is_read_only() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let first = state
            .with_connection(|connection| {
                update_manual_mastery(
                    connection,
                    V2UpdateConceptMasteryInput {
                        concept_id: concept,
                        target_score: 40.0,
                        local_date: "2026-11-04".to_owned(),
                        notes: String::new(),
                    },
                    at(0),
                )
            })
            .expect("manual");
        let run_id = first.run_id;

        let second = state
            .with_connection(|connection| {
                update_manual_mastery(
                    connection,
                    V2UpdateConceptMasteryInput {
                        concept_id: concept,
                        target_score: 80.0,
                        local_date: "2026-11-04".to_owned(),
                        notes: String::new(),
                    },
                    at(1),
                )
            })
            .expect("manual second");
        assert!((second.mastery.mastery_score - 80.0).abs() < 1e-9);

        let verification = state
            .with_connection(|connection| {
                verify_mastery_run(connection, V2VerifyMasteryRunInput { run_id })
            })
            .expect("verification");
        assert!(verification.matches);

        let current = state
            .with_connection(|connection| load_mastery_summary(connection, concept))
            .expect("current mastery")
            .expect("mastery row");
        assert!((current.mastery_score - 80.0).abs() < 1e-9);
        assert_eq!(current.evidence_count, 2);
    }

    #[test]
    fn replay_fails_without_writing_when_evidence_is_invalid() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO concept_evidence (
                            concept_id, source_kind, source_version, local_date, occurred_at,
                            raw_value_json, algorithm_version_id, created_at
                         ) VALUES (?1, 'study_session', 1, '2026-11-04',
                                   '2026-11-04T10:00:00.000Z', '{\"duration_seconds\":0}', 1,
                                   '2026-11-04T10:00:00.000Z')",
                        [concept],
                    )
                    .map_err(|error| AppError::database("test invalid evidence", error))?;
                Ok(())
            })
            .expect("invalid evidence insert");
        let before = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM mastery_recalc_runs WHERE concept_id = ?1",
                        [concept],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("test count", error))
            })
            .expect("run count");

        let error = state
            .with_connection(|connection| {
                replay_current(
                    connection,
                    V2ReplayConceptMasteryInput {
                        concept_id: concept,
                    },
                    at(0),
                )
            })
            .expect_err("invalid evidence must fail");
        assert_eq!(error.code, "validation_error");
        let after = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM mastery_recalc_runs WHERE concept_id = ?1",
                        [concept],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("test count", error))
            })
            .expect("run count");
        assert_eq!(before, after);
    }

    #[test]
    fn multiple_algorithm_versions_apply_per_evidence_row() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO mastery_algorithm_versions (
                            id, version, parameters_json, description, is_active, created_at
                         ) VALUES (
                            2, '2.0',
                            '{\"studyCap\":10,\"revisionCap\":10,\"pyqCap\":10,\"mockCap\":8,
                              \"totalPositiveCap\":20,\"totalNegativeCap\":15,
                              \"studyPerMinuteDivisor\":15,\"studyMaxBase\":4,
                              \"revisionDeltas\":{\"again\":-6,\"difficult\":2,\"good\":5,\"easy\":8},
                              \"pyqCorrect\":4,\"pyqIncorrect\":-3,\"mockCorrect\":3,\"mockIncorrect\":-2,
                              \"difficultyMultipliers\":{\"1\":0.75,\"2\":0.75,\"3\":1.0,\"4\":1.25,\"5\":1.25}}',
                            'test version', 1, '2026-11-04T10:00:00Z'
                         )",
                        [],
                    )
                    .map_err(|error| AppError::database("test version", error))?;
                Ok(())
            })
            .expect("version");
        let first_session = state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, concept_id, created_at, updated_at
                         ) VALUES ('2026-11-04', 7200, 'manual', ?1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [concept],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                Ok(connection.last_insert_rowid())
            })
            .expect("first session");
        let first = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    evidence(
                        concept,
                        "study_session",
                        Some(first_session),
                        1,
                        "2026-11-04",
                        "2026-11-04T10:00:00.000Z",
                        json!({ "duration_seconds": 7200 }),
                    ),
                    ReplayReason::SourceCreated,
                    at(0),
                )
            })
            .expect("version 2 evidence");
        assert!((first.mastery.mastery_score - 4.0).abs() < 1e-9);
        assert_eq!(first.mastery.algorithm_version.as_deref(), Some("2.0"));

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE mastery_algorithm_versions SET is_active = 0 WHERE id = 2",
                        [],
                    )
                    .map_err(|error| AppError::database("deactivate test version", error))?;
                Ok(())
            })
            .expect("deactivate version");
        let second_session = state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, concept_id, created_at, updated_at
                         ) VALUES ('2026-11-04', 900, 'manual', ?1,
                                   '2026-11-04T11:00:00Z', '2026-11-04T11:00:00Z')",
                        [concept],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                Ok(connection.last_insert_rowid())
            })
            .expect("second session");
        let second = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    evidence(
                        concept,
                        "study_session",
                        Some(second_session),
                        1,
                        "2026-11-04",
                        "2026-11-04T11:00:00.000Z",
                        json!({ "duration_seconds": 7200 }),
                    ),
                    ReplayReason::SourceCreated,
                    at(1),
                )
            })
            .expect("version 1 evidence");
        assert!((second.mastery.mastery_score - 9.76).abs() < 1e-9);
        assert_eq!(second.mastery.algorithm_version.as_deref(), Some("1.0"));
        let verification = state
            .with_connection(|connection| {
                verify_mastery_run(
                    connection,
                    V2VerifyMasteryRunInput {
                        run_id: second.run_id,
                    },
                )
            })
            .expect("verification");
        assert!(verification.matches);
        assert_eq!(
            verification.stored_fingerprint,
            verification.recomputed_fingerprint
        );
    }

    #[test]
    fn shared_mastery_combines_exam_context_without_duplicating_the_record() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let sessions = state
            .with_connection(|connection| {
                let mut ids = Vec::new();
                for (exam_id, date) in [
                    (1_i64, "2026-11-04T10:00:00Z"),
                    (2_i64, "2026-11-04T11:00:00Z"),
                ] {
                    connection
                        .execute(
                            "INSERT INTO study_sessions (
                                session_date, duration_seconds, source, primary_exam_id,
                                concept_id, created_at, updated_at
                             ) VALUES ('2026-11-04', 1800, 'manual', ?1, ?2, ?3, ?3)",
                            params![exam_id, concept, date],
                        )
                        .map_err(|error| AppError::database("test session", error))?;
                    ids.push(connection.last_insert_rowid());
                }
                Ok(ids)
            })
            .expect("sessions");

        let before = state
            .with_connection(|connection| load_mastery_summary(connection, concept))
            .expect("mastery before");
        assert!(
            before.is_none(),
            "study duration alone must not create mastery"
        );

        for (index, session_id) in sessions.iter().enumerate() {
            let exam_id = if index == 0 { 1 } else { 2 };
            state
                .with_connection(|connection| {
                    record_evidence(
                        connection,
                        V2EvidenceRecordInput {
                            concept_id: concept,
                            source_kind: "study_session".to_owned(),
                            source_id: Some(*session_id),
                            source_version: 1,
                            exam_id: Some(exam_id),
                            local_date: "2026-11-04".to_owned(),
                            occurred_at: if index == 0 {
                                "2026-11-04T10:00:00.000Z".to_owned()
                            } else {
                                "2026-11-04T11:00:00.000Z".to_owned()
                            },
                            raw_value: json!({ "duration_seconds": 1800 }),
                            notes: String::new(),
                        },
                        ReplayReason::SourceCreated,
                        at(index as i64),
                    )
                    .map(|_| ())
                })
                .expect("study evidence");
        }

        let after = state
            .with_connection(|connection| load_mastery_summary(connection, concept))
            .expect("mastery after")
            .expect("mastery row");
        assert_eq!(after.evidence_count, 2);
        let rows = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM concept_mastery WHERE concept_id = ?1",
                        [concept],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("test mastery count", error))
            })
            .expect("mastery count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn evidence_requires_a_matching_source_concept() {
        let (_root, state) = state();
        let concept = concept_id(&state, "kinematics");
        let other = concept_id(&state, "newtons-laws");
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, concept_id, created_at, updated_at
                         ) VALUES ('2026-11-04', 900, 'manual', ?1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [other],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                Ok(())
            })
            .expect("session");
        let error = state
            .with_connection(|connection| {
                record_evidence(
                    connection,
                    evidence(
                        concept,
                        "study_session",
                        Some(1),
                        1,
                        "2026-11-04",
                        "2026-11-04T10:00:00.000Z",
                        json!({ "duration_seconds": 900 }),
                    ),
                    ReplayReason::SourceCreated,
                    at(0),
                )
            })
            .expect_err("mismatched source must fail");
        assert_eq!(error.field, Some("sourceId"));
    }
}
