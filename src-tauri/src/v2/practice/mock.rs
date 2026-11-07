use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::AppError;

use super::pyq::PatternSectionOption;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockSection {
    pub id: i64,
    pub section_id: i64,
    pub pattern_section_id: Option<i64>,
    pub section_code: String,
    pub section_title: String,
    pub question_type_id: Option<i64>,
    pub question_type_code: Option<String>,
    pub question_count: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub max_marks: f64,
    pub marks_obtained: f64,
    pub correct_marks: f64,
    pub incorrect_marks: f64,
    pub unanswered_marks: f64,
    pub time_spent_minutes: Option<i64>,
    pub notes: String,
    pub section_snapshot_json: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockScoreVersion {
    pub version: i64,
    pub max_marks: f64,
    pub marks_obtained: f64,
    pub percentage: Option<f64>,
    pub recorded_at: String,
    pub reason: String,
    pub sections_json: String,
    pub marking_context_json: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockTest {
    pub id: i64,
    pub exam_id: i64,
    pub pattern_version_id: Option<i64>,
    pub pattern_version_label: Option<String>,
    pub pattern_title: Option<String>,
    pub focus_node_id: Option<i64>,
    pub focus_node_title: Option<String>,
    pub test_name: String,
    pub test_date: String,
    pub duration_minutes: i64,
    pub test_mode: String,
    pub max_marks: f64,
    pub marks_obtained: Option<f64>,
    pub percentage: Option<f64>,
    pub status: String,
    pub notes: String,
    pub review_observations: String,
    pub is_archived: bool,
    pub available_questions: i64,
    pub attempted_questions: i64,
    pub correct_questions: i64,
    pub incorrect_questions: i64,
    pub unanswered_questions: i64,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub scheme_snapshot_json: String,
    pub mock_kind: String,
    pub deadline_at: Option<String>,
    pub current_question_id: Option<i64>,
    pub last_activity_at: Option<String>,
    pub submission_kind: String,
    pub sections: Vec<V2MockSection>,
    pub score_history: Vec<V2MockScoreVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockSectionInput {
    pub pattern_section_id: i64,
    pub question_count: i64,
    #[serde(default)]
    pub correct: i64,
    #[serde(default)]
    pub incorrect: i64,
    #[serde(default)]
    pub unanswered: i64,
    #[serde(default)]
    pub max_marks: Option<f64>,
    #[serde(default)]
    pub marks_obtained: Option<f64>,
    #[serde(default)]
    pub time_spent_minutes: Option<i64>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockTestInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub exam_id: i64,
    pub pattern_version_id: i64,
    pub test_name: String,
    pub test_date: String,
    pub duration_minutes: i64,
    #[serde(default = "default_mode")]
    pub test_mode: String,
    #[serde(default)]
    pub focus_node_id: Option<i64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub review_observations: String,
    pub sections: Vec<V2MockSectionInput>,
}

fn default_mode() -> String {
    "full_length".to_owned()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockAnalytics {
    pub completed_count: i64,
    pub average_percentage: Option<f64>,
    pub best_percentage: Option<f64>,
    pub last_percentage: Option<f64>,
    pub by_pattern: Vec<V2MockPatternAnalytics>,
    pub history: Vec<V2MockAnalyticsPoint>,
    pub insufficient_data: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockPatternAnalytics {
    pub pattern_version_id: Option<i64>,
    pub pattern_version_label: String,
    pub completed_count: i64,
    pub average_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockAnalyticsPoint {
    pub test_id: i64,
    pub test_date: String,
    pub test_name: String,
    pub percentage: f64,
    pub pattern_version_id: Option<i64>,
    pub pattern_version_label: Option<String>,
}

pub fn list_mocks(
    connection: &Connection,
    exam_id: i64,
    include_archived: bool,
) -> Result<Vec<V2MockTest>, AppError> {
    validate_exam(connection, exam_id)?;
    let mut sql = String::from(
        "SELECT m.id, m.exam_id, m.pattern_version_id, pv.version_label, p.title,
                m.focus_node_id, fn.title,
                m.test_name, m.test_date, m.duration_minutes, m.test_mode, m.max_marks,
                m.marks_obtained,
                CASE WHEN m.max_marks > 0 AND m.marks_obtained IS NOT NULL
                     THEN (m.marks_obtained * 100.0 / m.max_marks) ELSE NULL END,
                m.status, m.notes, m.review_observations, m.is_archived,
                m.available_questions, m.attempted_questions, m.correct_questions,
                m.incorrect_questions, m.unanswered_questions, m.started_at, m.ended_at,
                m.scheme_snapshot_json, m.mock_kind, m.deadline_at, m.current_question_id,
                m.last_activity_at, m.submission_kind
         FROM mock_tests m
         LEFT JOIN exam_pattern_versions pv ON pv.id = m.pattern_version_id
         LEFT JOIN exam_patterns p ON p.id = pv.pattern_id
         LEFT JOIN exam_syllabus_nodes fn ON fn.id = m.focus_node_id
         WHERE m.exam_id = ?1",
    );
    if !include_archived {
        sql.push_str(" AND m.is_archived = 0");
    }
    sql.push_str(" ORDER BY m.test_date DESC, m.id DESC LIMIT 200");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Mock tests could not be prepared", error))?;
    let rows = statement
        .query_map([exam_id], map_mock)
        .map_err(|error| AppError::database("Mock tests could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A mock test is invalid", error))?;
    drop(statement);
    let mut mocks = rows;
    for mock in &mut mocks {
        mock.sections = load_sections(connection, mock.id)?;
        mock.score_history = load_history(connection, mock.id)?;
    }
    Ok(mocks)
}

fn map_mock(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2MockTest> {
    Ok(V2MockTest {
        id: row.get(0)?,
        exam_id: row.get(1)?,
        pattern_version_id: row.get(2)?,
        pattern_version_label: row.get(3)?,
        pattern_title: row.get(4)?,
        focus_node_id: row.get(5)?,
        focus_node_title: row.get(6)?,
        test_name: row.get(7)?,
        test_date: row.get(8)?,
        duration_minutes: row.get(9)?,
        test_mode: row.get(10)?,
        max_marks: row.get(11)?,
        marks_obtained: row.get(12)?,
        percentage: row.get(13)?,
        status: row.get(14)?,
        notes: row.get(15)?,
        review_observations: row.get(16)?,
        is_archived: row.get::<_, i64>(17)? == 1,
        available_questions: row.get(18)?,
        attempted_questions: row.get(19)?,
        correct_questions: row.get(20)?,
        incorrect_questions: row.get(21)?,
        unanswered_questions: row.get(22)?,
        started_at: row.get(23)?,
        ended_at: row.get(24)?,
        scheme_snapshot_json: row.get(25)?,
        mock_kind: row.get(26)?,
        deadline_at: row.get(27)?,
        current_question_id: row.get(28)?,
        last_activity_at: row.get(29)?,
        submission_kind: row.get(30)?,
        sections: Vec::new(),
        score_history: Vec::new(),
    })
}

fn load_sections(connection: &Connection, mock_id: i64) -> Result<Vec<V2MockSection>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT ms.id, ms.section_id, ms.pattern_section_id, s.code, s.title,
                    ms.question_type_id, t.code, ms.question_count, ms.correct, ms.incorrect,
                    ms.unanswered, ms.max_marks, ms.marks_obtained, ms.correct_marks,
                    ms.incorrect_marks, ms.unanswered_marks, ms.time_spent_minutes, ms.notes,
                    ms.section_snapshot_json
             FROM mock_test_sections ms
             JOIN exam_sections s ON s.id = ms.section_id
             LEFT JOIN exam_question_types t ON t.id = ms.question_type_id
             WHERE ms.mock_test_id = ?1 ORDER BY s.display_order, ms.id",
        )
        .map_err(|error| AppError::database("Mock sections could not be prepared", error))?;
    statement
        .query_map([mock_id], |row| {
            Ok(V2MockSection {
                id: row.get(0)?,
                section_id: row.get(1)?,
                pattern_section_id: row.get(2)?,
                section_code: row.get(3)?,
                section_title: row.get(4)?,
                question_type_id: row.get(5)?,
                question_type_code: row.get(6)?,
                question_count: row.get(7)?,
                correct: row.get(8)?,
                incorrect: row.get(9)?,
                unanswered: row.get(10)?,
                max_marks: row.get(11)?,
                marks_obtained: row.get(12)?,
                correct_marks: row.get(13)?,
                incorrect_marks: row.get(14)?,
                unanswered_marks: row.get(15)?,
                time_spent_minutes: row.get(16)?,
                notes: row.get(17)?,
                section_snapshot_json: row.get(18)?,
            })
        })
        .map_err(|error| AppError::database("Mock sections could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A mock section is invalid", error))
}

fn load_history(
    connection: &Connection,
    mock_id: i64,
) -> Result<Vec<V2MockScoreVersion>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT version, max_marks, marks_obtained,
                    CASE WHEN max_marks > 0 THEN marks_obtained * 100.0 / max_marks ELSE NULL END,
                    recorded_at, reason, sections_json, marking_context_json
             FROM mock_test_score_history WHERE mock_test_id = ?1 ORDER BY version",
        )
        .map_err(|error| AppError::database("Mock score history could not be prepared", error))?;
    statement
        .query_map([mock_id], |row| {
            Ok(V2MockScoreVersion {
                version: row.get(0)?,
                max_marks: row.get(1)?,
                marks_obtained: row.get(2)?,
                percentage: row.get(3)?,
                recorded_at: row.get(4)?,
                reason: row.get(5)?,
                sections_json: row.get(6)?,
                marking_context_json: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database("Mock score history could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Mock score history is invalid", error))
}

fn validate_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    let active: Option<i64> = connection
        .query_row(
            "SELECT is_active FROM exam_profiles WHERE exam_id = ?1",
            [exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Mock exam activity could not be checked", error))?;
    match active {
        Some(1) => Ok(()),
        Some(_) => Err(AppError::forbidden(
            "examId",
            "Inactive examinations are read-only. Reactivate the examination to record a mock.",
        )),
        None => Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        )),
    }
}

fn pattern_section(
    connection: &Connection,
    exam_id: i64,
    pattern_version_id: i64,
    pattern_section_id: i64,
) -> Result<PatternSectionOption, AppError> {
    connection
        .query_row(
            "SELECT ps.id, ps.section_id, ps.code, ps.title, ps.display_order,
                    ps.question_count, ps.max_attempts, ps.max_marks, ps.correct_marks,
                    ps.incorrect_marks, ps.unanswered_marks, ps.question_types_json,
                    ps.marking_variants_json, ps.numeric_tolerance, ps.partial_rule, ps.notes
             FROM exam_pattern_sections ps
             JOIN exam_pattern_versions pv ON pv.id = ps.pattern_version_id
             WHERE ps.id = ?1 AND pv.exam_id = ?2 AND pv.id = ?3
               AND pv.verification_status = 'verified'",
            params![pattern_section_id, exam_id, pattern_version_id],
            |row| {
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
                    question_types: serde_json::from_str(&question_types).unwrap_or_default(),
                    marking_variants: serde_json::from_str(&marking_variants).unwrap_or_default(),
                    numeric_tolerance: row.get(13)?,
                    partial_rule: row.get(14)?,
                    notes: row.get(15)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Pattern section could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "sections",
                "Choose sections from the selected verified pattern version.",
            )
        })
}

fn section_marks(
    connection: &Connection,
    exam_id: i64,
    pattern_version_id: i64,
    input: &V2MockSectionInput,
) -> Result<(PatternSectionOption, f64, f64), AppError> {
    let pattern = pattern_section(
        connection,
        exam_id,
        pattern_version_id,
        input.pattern_section_id,
    )?;
    if input.question_count <= 0 || input.question_count > pattern.question_count {
        return Err(AppError::validation(
            "questionCount",
            format!(
                "Section {} accepts between 1 and {} questions.",
                pattern.code, pattern.question_count
            ),
        ));
    }
    if input.correct < 0 || input.incorrect < 0 || input.unanswered < 0 {
        return Err(AppError::validation(
            "sections",
            "Question counts cannot be negative.",
        ));
    }
    if input.correct + input.incorrect + input.unanswered != input.question_count {
        return Err(AppError::validation(
            "sections",
            "Correct, incorrect and unanswered counts must add up to the question count.",
        ));
    }
    if let Some(max_attempts) = pattern.max_attempts
        && input.correct + input.incorrect > max_attempts
    {
        return Err(AppError::validation(
            "sections",
            format!(
                "Section {} evaluates at most {} answered questions.",
                pattern.code, max_attempts
            ),
        ));
    }
    let single_rule = pattern.question_types.len() == 1 && pattern.marking_variants.len() <= 1;
    let evaluated_questions = pattern.max_attempts.map_or(input.question_count, |limit| {
        input.question_count.min(limit)
    });
    let computed_max = if single_rule {
        evaluated_questions as f64 * pattern.correct_marks
    } else if input.question_count == pattern.question_count {
        pattern.max_marks
    } else {
        0.0
    };
    let max_marks = input.max_marks.unwrap_or(computed_max);
    if max_marks <= 0.0 {
        return Err(AppError::validation(
            "maxMarks",
            format!(
                "Enter maximum marks for the mixed section {}.",
                pattern.code
            ),
        ));
    }
    let computed_marks = if single_rule {
        input.correct as f64 * pattern.correct_marks
            + input.incorrect as f64 * pattern.incorrect_marks
            + input.unanswered as f64 * pattern.unanswered_marks
    } else {
        0.0
    };
    let marks = match input.marks_obtained {
        Some(value) => value,
        None if !single_rule && (input.correct > 0 || input.incorrect > 0) => {
            return Err(AppError::validation(
                "marksObtained",
                format!(
                    "Enter marks obtained for mixed-marking section {}; aggregate counts alone cannot determine its score.",
                    pattern.code
                ),
            ));
        }
        None => computed_marks,
    };
    if !marks.is_finite() || marks < -max_marks || marks > max_marks {
        return Err(AppError::validation(
            "marksObtained",
            format!(
                "Marks for section {} must be between -{max_marks} and {max_marks}.",
                pattern.code
            ),
        ));
    }
    Ok((pattern, max_marks, marks))
}

pub fn save_mock(
    connection: &mut Connection,
    input: V2MockTestInput,
) -> Result<V2MockTest, AppError> {
    validate_exam(connection, input.exam_id)?;
    let pattern = connection
        .query_row(
            "SELECT pv.id, pv.total_marks, pv.total_questions, pv.duration_minutes
             FROM exam_pattern_versions pv
             WHERE pv.id = ?1 AND pv.exam_id = ?2 AND pv.verification_status = 'verified'",
            params![input.pattern_version_id, input.exam_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("Mock pattern could not be read", error))?
        .ok_or_else(|| {
            AppError::validation(
                "patternVersionId",
                "Choose a verified pattern version for this examination.",
            )
        })?;
    if input.sections.is_empty() {
        return Err(AppError::validation(
            "sections",
            "Add at least one section result.",
        ));
    }
    let mode = input.test_mode.trim().to_ascii_lowercase();
    if !matches!(
        mode.as_str(),
        "full_length" | "untimed" | "section" | "topic" | "custom" | "past_paper"
    ) {
        return Err(AppError::validation(
            "testMode",
            "Choose full_length, untimed, section, topic, custom or past_paper.",
        ));
    }
    if mode == "topic" && input.focus_node_id.is_none() {
        return Err(AppError::validation(
            "focusNodeId",
            "Choose a syllabus topic for a topic-based mock.",
        ));
    }
    if let Some(node_id) = input.focus_node_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM exam_syllabus_nodes
                    WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0
                 )",
                params![node_id, input.exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Mock focus topic could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "focusNodeId",
                "Choose an active syllabus topic from this examination.",
            ));
        }
    }
    if input.test_name.trim().is_empty() || input.test_name.chars().count() > 200 {
        return Err(AppError::validation(
            "testName",
            "Mock name must be between 1 and 200 characters.",
        ));
    }
    if !(1..=1440).contains(&input.duration_minutes) {
        return Err(AppError::validation(
            "durationMinutes",
            "Duration must be between 1 and 1,440 minutes.",
        ));
    }
    if let Some(expected) = pattern.3
        && input.duration_minutes != expected
        && mode == "full_length"
    {
        return Err(AppError::validation(
            "durationMinutes",
            format!("The verified full-length pattern duration is {expected} minutes."),
        ));
    }
    if !valid_date(&input.test_date) {
        return Err(AppError::validation(
            "testDate",
            "Enter a valid date in YYYY-MM-DD form.",
        ));
    }
    let mut seen = HashSet::new();
    let mut sections = Vec::with_capacity(input.sections.len());
    let mut total_max = 0.0;
    let mut total_marks = 0.0;
    let mut total_questions = 0_i64;
    let mut correct_total = 0_i64;
    let mut incorrect_total = 0_i64;
    let mut unanswered_total = 0_i64;
    for section in &input.sections {
        if !seen.insert(section.pattern_section_id) {
            return Err(AppError::validation(
                "sections",
                "A section can appear only once in a mock.",
            ));
        }
        let (pattern, max_marks, marks) =
            section_marks(connection, input.exam_id, input.pattern_version_id, section)?;
        total_max += max_marks;
        total_marks += marks;
        total_questions += section.question_count;
        correct_total += section.correct;
        incorrect_total += section.incorrect;
        unanswered_total += section.unanswered;
        sections.push((pattern, max_marks, marks, section.clone()));
    }
    if total_max <= 0.0 {
        return Err(AppError::validation(
            "sections",
            "Mock maximum marks are invalid.",
        ));
    }
    if mode == "full_length" {
        let mut statement = connection
            .prepare(
                "SELECT id, code, question_count FROM exam_pattern_sections
                 WHERE pattern_version_id = ?1 ORDER BY display_order, id",
            )
            .map_err(|error| AppError::database("Pattern sections could not be prepared", error))?;
        let expected = statement
            .query_map([input.pattern_version_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|error| AppError::database("Pattern sections could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A pattern section is invalid", error))?;
        for (expected_id, code, expected_count) in expected {
            let Some((_, _, _, section_input)) = sections
                .iter()
                .find(|(pattern, _, _, _)| pattern.id == expected_id)
            else {
                return Err(AppError::validation(
                    "sections",
                    format!("A full-length mock must include section {code}."),
                ));
            };
            if section_input.question_count != expected_count {
                return Err(AppError::validation(
                    "sections",
                    format!("Full-length section {code} must use all {expected_count} questions."),
                ));
            }
        }
        if (total_max - pattern.1).abs() > 1e-6 {
            return Err(AppError::validation(
                "sections",
                format!(
                    "Full-length mock maximum marks must equal the verified pattern total of {}.",
                    pattern.1
                ),
            ));
        }
        if total_questions != pattern.2 {
            return Err(AppError::validation(
                "sections",
                format!(
                    "Full-length mock must contain the verified pattern total of {} questions.",
                    pattern.2
                ),
            ));
        }
    }
    let percentage = (total_marks * 100.0 / total_max * 1000.0).round() / 1000.0;
    let scheme = json!({
        "schemaVersion": 1,
        "patternVersionId": input.pattern_version_id,
        "patternTotalMarks": pattern.1,
        "patternTotalQuestions": pattern.2,
        "patternDurationMinutes": pattern.3,
        "focusNodeId": input.focus_node_id,
        "sections": sections.iter().map(|(pattern, max_marks, marks, section)| json!({
            "patternSectionId": pattern.id,
            "sectionId": pattern.section_id,
            "sectionCode": pattern.code,
            "sectionTitle": pattern.title,
            "questionCount": section.question_count,
            "maxAttempts": pattern.max_attempts,
            "maxMarks": max_marks,
            "marksObtained": marks,
            "correct": section.correct,
            "incorrect": section.incorrect,
            "unanswered": section.unanswered,
            "correctMarks": pattern.correct_marks,
            "incorrectMarks": pattern.incorrect_marks,
            "unansweredMarks": pattern.unanswered_marks,
            "questionTypes": pattern.question_types,
            "markingVariants": pattern.marking_variants,
        })).collect::<Vec<_>>()
    });
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The mock transaction could not start", error))?;
    let existing_status = if let Some(id) = input.id {
        let status: Option<(i64, String, i64)> = transaction
            .query_row(
                "SELECT exam_id, status, is_archived FROM mock_tests WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| AppError::database("Mock could not be checked", error))?;
        match status {
            Some((exam_id, status, archived)) if exam_id == input.exam_id => {
                if archived == 1 {
                    return Err(AppError::validation(
                        "id",
                        "Archived mock tests cannot be edited.",
                    ));
                }
                if status == "completed" {
                    return Err(AppError::validation(
                        "id",
                        "Reopen the completed mock before changing its sections.",
                    ));
                }
                Some(status)
            }
            _ => return Err(AppError::mock_test_not_found()),
        }
    } else {
        None
    };
    let mock_id = if let Some(id) = input.id {
        transaction
            .execute(
                "UPDATE mock_tests SET pattern_version_id = ?1, test_name = ?2, test_date = ?3,
                    duration_minutes = ?4, test_mode = ?5, max_marks = ?6, marks_obtained = ?7,
                    percentage = ?8, notes = ?9, review_observations = ?10,
                    available_questions = ?11, attempted_questions = ?12, correct_questions = ?13,
                    incorrect_questions = ?14, unanswered_questions = ?15,
                    scheme_snapshot_json = ?16, focus_node_id = ?17, updated_at = ?18
                 WHERE id = ?19 AND exam_id = ?20",
                params![
                    input.pattern_version_id,
                    input.test_name.trim(),
                    input.test_date,
                    input.duration_minutes,
                    mode,
                    total_max,
                    total_marks,
                    percentage,
                    input.notes.trim(),
                    input.review_observations.trim(),
                    total_questions,
                    correct_total + incorrect_total,
                    correct_total,
                    incorrect_total,
                    unanswered_total,
                    scheme.to_string(),
                    input.focus_node_id,
                    now,
                    id,
                    input.exam_id,
                ],
            )
            .map_err(|error| AppError::database("Mock could not be updated", error))?;
        id
    } else {
        transaction
            .execute(
                "INSERT INTO mock_tests(
                    exam_id, test_name, test_date, duration_minutes, max_marks, marks_obtained,
                    status, notes, review_observations, is_archived, created_at, updated_at,
                    pattern_version_id, test_mode, scheme_snapshot_json, focus_node_id,
                    available_questions, attempted_questions, correct_questions,
                    incorrect_questions, unanswered_questions, percentage
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'planned', ?7, ?8, 0, ?9, ?9,
                           ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    input.exam_id,
                    input.test_name.trim(),
                    input.test_date,
                    input.duration_minutes,
                    total_max,
                    total_marks,
                    input.notes.trim(),
                    input.review_observations.trim(),
                    now,
                    input.pattern_version_id,
                    mode,
                    scheme.to_string(),
                    input.focus_node_id,
                    total_questions,
                    correct_total + incorrect_total,
                    correct_total,
                    incorrect_total,
                    unanswered_total,
                    percentage,
                ],
            )
            .map_err(|error| AppError::database("Mock could not be stored", error))?;
        transaction.last_insert_rowid()
    };
    transaction
        .execute(
            "DELETE FROM mock_test_sections WHERE mock_test_id = ?1",
            [mock_id],
        )
        .map_err(|error| AppError::database("Old mock sections could not be replaced", error))?;
    for (pattern, max_marks, marks, section) in &sections {
        let snapshot = json!({
            "schemaVersion": 1,
            "patternSectionId": pattern.id,
            "sectionCode": pattern.code,
            "sectionTitle": pattern.title,
            "maxAttempts": pattern.max_attempts,
            "maxMarks": max_marks,
            "correctMarks": pattern.correct_marks,
            "incorrectMarks": pattern.incorrect_marks,
            "unansweredMarks": pattern.unanswered_marks,
            "questionTypes": pattern.question_types,
            "markingVariants": pattern.marking_variants,
            "partialRule": pattern.partial_rule,
            "numericTolerance": pattern.numeric_tolerance,
        });
        let question_type_id: Option<i64> = transaction
            .query_row(
                "SELECT default_question_type_id FROM exam_sections WHERE id = ?1",
                [pattern.section_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AppError::database("Section question type could not be read", error))?
            .flatten();
        transaction
            .execute(
                "INSERT INTO mock_test_sections(
                    exam_id, mock_test_id, section_id, question_count, correct, incorrect,
                    unanswered, max_marks, marks_obtained, time_spent_minutes, notes,
                    created_at, updated_at, pattern_section_id, question_type_id,
                    correct_marks, incorrect_marks, unanswered_marks, section_snapshot_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12,
                           ?13, ?14, ?15, ?16, ?17, ?18)",
                params![
                    input.exam_id,
                    mock_id,
                    pattern.section_id,
                    section.question_count,
                    section.correct,
                    section.incorrect,
                    section.unanswered,
                    max_marks,
                    marks,
                    section.time_spent_minutes,
                    section.notes.trim(),
                    now,
                    pattern.id,
                    question_type_id,
                    pattern.correct_marks,
                    pattern.incorrect_marks,
                    pattern.unanswered_marks,
                    snapshot.to_string(),
                ],
            )
            .map_err(|error| AppError::database("Mock section could not be stored", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Mock could not be committed", error))?;
    let _ = existing_status;
    find_mock(connection, input.exam_id, mock_id)
}

fn find_mock(connection: &Connection, exam_id: i64, mock_id: i64) -> Result<V2MockTest, AppError> {
    list_mocks(connection, exam_id, true)?
        .into_iter()
        .find(|mock| mock.id == mock_id)
        .ok_or_else(AppError::mock_test_not_found)
}

fn valid_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub fn set_mock_status(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
    action: &str,
) -> Result<V2MockTest, AppError> {
    validate_exam(connection, exam_id)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let status: Option<String> = connection
        .query_row(
            "SELECT status FROM mock_tests WHERE id = ?1 AND exam_id = ?2 AND is_archived = 0",
            params![mock_id, exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Mock status could not be read", error))?;
    let Some(status) = status else {
        return Err(AppError::mock_test_not_found());
    };
    match action {
        "start" => {
            if status != "planned" {
                return Err(AppError::validation(
                    "action",
                    "Only a planned mock can be started.",
                ));
            }
            connection
                .execute(
                    "UPDATE mock_tests SET status = 'in_progress', started_at = ?1, updated_at = ?1
                     WHERE id = ?2 AND exam_id = ?3",
                    params![now, mock_id, exam_id],
                )
                .map_err(|error| AppError::database("Mock could not be started", error))?;
        }
        "reopen" => {
            if status != "completed" {
                return Err(AppError::validation(
                    "action",
                    "Only a completed mock can be reopened.",
                ));
            }
            connection
                .execute(
                    "UPDATE mock_tests SET status = 'in_progress', ended_at = NULL, updated_at = ?1
                     WHERE id = ?2 AND exam_id = ?3",
                    params![now, mock_id, exam_id],
                )
                .map_err(|error| AppError::database("Mock could not be reopened", error))?;
        }
        "cancel" => {
            if status == "completed" {
                return Err(AppError::validation(
                    "action",
                    "A completed mock cannot be cancelled.",
                ));
            }
            connection
                .execute(
                    "UPDATE mock_tests SET status = 'cancelled', updated_at = ?1
                     WHERE id = ?2 AND exam_id = ?3",
                    params![now, mock_id, exam_id],
                )
                .map_err(|error| AppError::database("Mock could not be cancelled", error))?;
        }
        "complete" => {
            if !matches!(status.as_str(), "planned" | "in_progress") {
                return Err(AppError::validation(
                    "action",
                    "Only a planned or in-progress mock can be completed.",
                ));
            }
            complete_mock(connection, exam_id, mock_id, now)?;
        }
        other => {
            return Err(AppError::validation(
                "action",
                format!("Unsupported mock action '{other}'."),
            ));
        }
    }
    find_mock(connection, exam_id, mock_id)
}

fn complete_mock(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
    now: String,
) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The mock completion transaction could not start", error)
    })?;
    let (max_marks, marks_obtained, available, attempted, correct, incorrect, unanswered): (
        f64,
        f64,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = transaction
        .query_row(
            "SELECT COALESCE(SUM(max_marks),0), COALESCE(SUM(marks_obtained),0),
                    COALESCE(SUM(question_count),0), COALESCE(SUM(correct + incorrect),0),
                    COALESCE(SUM(correct),0), COALESCE(SUM(incorrect),0),
                    COALESCE(SUM(unanswered),0)
             FROM mock_test_sections WHERE mock_test_id = ?1",
            [mock_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|error| AppError::database("Mock totals could not be read", error))?;
    if max_marks <= 0.0 || available <= 0 {
        return Err(AppError::validation(
            "sections",
            "A mock needs at least one populated section before completion.",
        ));
    }
    let percentage = (marks_obtained * 100.0 / max_marks * 1000.0).round() / 1000.0;
    transaction
        .execute(
            "UPDATE mock_tests SET max_marks = ?1, marks_obtained = ?2, percentage = ?3,
                available_questions = ?4, attempted_questions = ?5, correct_questions = ?6,
                incorrect_questions = ?7, unanswered_questions = ?8, status = 'completed',
                ended_at = ?9, updated_at = ?9
             WHERE id = ?10 AND exam_id = ?11",
            params![
                max_marks,
                marks_obtained,
                percentage,
                available,
                attempted,
                correct,
                incorrect,
                unanswered,
                now,
                mock_id,
                exam_id,
            ],
        )
        .map_err(|error| AppError::database("Mock completion totals could not be saved", error))?;
    let sections_json: String = {
        let mut statement = transaction
            .prepare(
                "SELECT json_group_array(json_object(
                    'sectionId', section_id,
                    'questionCount', question_count,
                    'correct', correct,
                    'incorrect', incorrect,
                    'unanswered', unanswered,
                    'maxMarks', max_marks,
                    'marksObtained', marks_obtained,
                    'patternSectionId', pattern_section_id,
                    'sectionSnapshot', json(section_snapshot_json)
                )) FROM mock_test_sections WHERE mock_test_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Mock section snapshot could not be prepared", error)
            })?;
        statement
            .query_row([mock_id], |row| row.get(0))
            .map_err(|error| AppError::database("Mock section snapshot could not be read", error))?
    };
    let marking_context: String = transaction
        .query_row(
            "SELECT scheme_snapshot_json FROM mock_tests WHERE id = ?1",
            [mock_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Mock marking context could not be read", error))?;
    let latest: Option<(i64, String, String, f64, f64)> = transaction
        .query_row(
            "SELECT version, sections_json, marking_context_json, max_marks, marks_obtained
             FROM mock_test_score_history WHERE mock_test_id = ?1 ORDER BY version DESC LIMIT 1",
            [mock_id],
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
        .optional()
        .map_err(|error| AppError::database("Mock score history could not be read", error))?;
    let same = latest
        .as_ref()
        .is_some_and(|(_, sections, context, max, marks)| {
            sections == &sections_json
                && context == &marking_context
                && (*max - max_marks).abs() < 1e-9
                && (*marks - marks_obtained).abs() < 1e-9
        });
    if !same {
        let version = latest.map(|(version, _, _, _, _)| version + 1).unwrap_or(1);
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
                    max_marks,
                    marks_obtained,
                    sections_json,
                    marking_context,
                    now,
                    if version == 1 {
                        "Initial completion"
                    } else {
                        "Recompletion"
                    },
                ],
            )
            .map_err(|error| AppError::database("Mock score history could not be stored", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Mock completion could not be committed", error))
}

pub fn archive_mock(
    connection: &mut Connection,
    exam_id: i64,
    mock_id: i64,
    archived: bool,
) -> Result<(), AppError> {
    validate_exam(connection, exam_id)?;
    let changed = connection
        .execute(
            "UPDATE mock_tests SET is_archived = ?1, updated_at = ?2
             WHERE id = ?3 AND exam_id = ?4",
            params![
                i64::from(archived),
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                mock_id,
                exam_id,
            ],
        )
        .map_err(|error| AppError::database("Mock archive state could not be saved", error))?;
    if changed == 0 {
        return Err(AppError::mock_test_not_found());
    }
    Ok(())
}

pub fn analytics(connection: &Connection, exam_id: i64) -> Result<V2MockAnalytics, AppError> {
    validate_exam(connection, exam_id)?;
    let completed = list_mocks(connection, exam_id, false)?
        .into_iter()
        .filter(|mock| mock.status == "completed" && !mock.is_archived)
        .collect::<Vec<_>>();
    let mut percentages = completed
        .iter()
        .filter_map(|mock| mock.percentage)
        .collect::<Vec<_>>();
    percentages.sort_by(|left, right| left.total_cmp(right));
    let average = if percentages.is_empty() {
        None
    } else {
        Some((percentages.iter().sum::<f64>() / percentages.len() as f64 * 1000.0).round() / 1000.0)
    };
    let best = percentages.last().copied();
    let last = completed.first().and_then(|mock| mock.percentage);
    let mut by_pattern = Vec::new();
    let mut pattern_ids = HashSet::new();
    for mock in &completed {
        if pattern_ids.insert(mock.pattern_version_id) {
            let matching = completed
                .iter()
                .filter(|candidate| candidate.pattern_version_id == mock.pattern_version_id)
                .filter_map(|candidate| candidate.percentage)
                .collect::<Vec<_>>();
            let avg = matching.iter().sum::<f64>() / matching.len() as f64;
            by_pattern.push(V2MockPatternAnalytics {
                pattern_version_id: mock.pattern_version_id,
                pattern_version_label: mock
                    .pattern_version_label
                    .clone()
                    .unwrap_or_else(|| "Unspecified pattern".to_owned()),
                completed_count: matching.len() as i64,
                average_percentage: Some((avg * 1000.0).round() / 1000.0),
            });
        }
    }
    let history = completed
        .iter()
        .filter_map(|mock| {
            mock.percentage.map(|percentage| V2MockAnalyticsPoint {
                test_id: mock.id,
                test_date: mock.test_date.clone(),
                test_name: mock.test_name.clone(),
                percentage,
                pattern_version_id: mock.pattern_version_id,
                pattern_version_label: mock.pattern_version_label.clone(),
            })
        })
        .collect::<Vec<_>>();
    Ok(V2MockAnalytics {
        completed_count: completed.len() as i64,
        average_percentage: average,
        best_percentage: best,
        last_percentage: last,
        by_pattern,
        insufficient_data: completed.len() < 2,
        history,
    })
}
