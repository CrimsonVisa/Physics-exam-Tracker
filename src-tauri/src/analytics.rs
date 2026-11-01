use std::collections::{BTreeMap, HashSet};

use chrono::{Days, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;

use crate::{date_only, error::AppError, mock_tests};

pub const MINIMUM_EVIDENCE_QUESTIONS: i64 = 5;
pub const EVIDENCE_WINDOW_DAYS: i64 = 365;
pub const WEAK_ACCURACY_THRESHOLD: f64 = 50.0;
pub const STRONG_ACCURACY_THRESHOLD: f64 = 75.0;
pub const MAX_STUDY_PERFORMANCE_DAYS: usize = 60;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqTopicPerformance {
    pub topic_id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
    pub section_code: String,
    pub section_title: String,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub attempts: i64,
    pub total_attempts: i64,
    pub distinct_questions: i64,
    pub evidence_questions: i64,
    pub accuracy: Option<f64>,
    pub evidence_accuracy: Option<f64>,
    pub last_scored_attempt_date: Option<String>,
    pub skipped_only: bool,
    pub sufficient_data: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqUnitPerformance {
    pub code: String,
    pub title: String,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub attempts: i64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqYearPerformance {
    pub year: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub attempts: i64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqPartPerformance {
    pub part: String,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub attempts: i64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqResponseTrendPoint {
    pub date: String,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub outcome_unknown: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PyqAnalytics {
    pub total_records: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub skipped: i64,
    pub outcome_unknown: i64,
    pub attempts: i64,
    pub accuracy: Option<f64>,
    pub minimum_evidence_questions: i64,
    pub evidence_window_days: i64,
    pub weak_accuracy_threshold: f64,
    pub by_topic: Vec<PyqTopicPerformance>,
    pub by_unit: Vec<PyqUnitPerformance>,
    pub by_year: Vec<PyqYearPerformance>,
    pub by_part: Vec<PyqPartPerformance>,
    pub weak_topics: Vec<PyqTopicPerformance>,
    pub response_trend: Vec<PyqResponseTrendPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockTrendPoint {
    pub id: i64,
    pub test_name: String,
    pub test_date: String,
    pub max_marks: f64,
    pub computed_marks: f64,
    pub computed_percentage: f64,
    pub configuration_valid: bool,
    pub reported_marks: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockPartPerformance {
    pub part: String,
    pub questions: i64,
    pub attempted: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockResponseTrendPoint {
    pub date: String,
    pub test_count: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub average_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockAnalytics {
    pub test_count: i64,
    pub average_percentage: Option<f64>,
    pub best_percentage: Option<f64>,
    pub latest_percentage: Option<f64>,
    pub total_questions: i64,
    pub total_attempted: i64,
    pub total_correct: i64,
    pub total_incorrect: i64,
    pub total_unanswered: i64,
    pub trend: Vec<MockTrendPoint>,
    pub by_part: Vec<MockPartPerformance>,
    pub response_trend: Vec<MockResponseTrendPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceAnalytics {
    pub from_date: String,
    pub to_date: String,
    pub pyq: PyqAnalytics,
    pub mock: MockAnalytics,
    pub separation_note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPerformanceDay {
    pub date: String,
    pub study_seconds: i64,
    pub pyq_correct: i64,
    pub pyq_incorrect: i64,
    pub pyq_accuracy: Option<f64>,
    pub mock_count: i64,
    pub mock_average_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyPerformance {
    pub days: Vec<StudyPerformanceDay>,
}

pub(crate) fn evidence_cutoff(today: NaiveDate) -> String {
    today
        .checked_sub_days(Days::new((EVIDENCE_WINDOW_DAYS - 1) as u64))
        .map(date_only::format_date_only)
        .unwrap_or_else(|| "0001-01-01".to_owned())
}

pub fn pyq(connection: &Connection, today: NaiveDate) -> Result<PyqAnalytics, AppError> {
    pyq_in_range(connection, None, None, today)
}

pub fn pyq_in_range(
    connection: &Connection,
    from_date: Option<&str>,
    to_date: Option<&str>,
    today: NaiveDate,
) -> Result<PyqAnalytics, AppError> {
    let from = normalize_optional_date(from_date, "fromDate")?;
    let to = normalize_optional_date(to_date, "toDate")?;
    validate_range_order(from.as_deref(), to.as_deref())?;
    let evidence_from = evidence_cutoff(today);
    let evidence_to = date_only::format_date_only(today);

    let (total_records, correct, incorrect, skipped, outcome_unknown): (i64, i64, i64, i64, i64) =
        connection
            .query_row(
                "SELECT
                    count(*),
                    COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'attempted' THEN 1 ELSE 0 END), 0)
                 FROM pyq_attempts
                 WHERE (?1 IS NULL OR attempt_date >= ?1)
                   AND (?2 IS NULL OR attempt_date <= ?2)",
                rusqlite::params![from, to],
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
            .map_err(|error| AppError::database("PYQ totals could not be calculated", error))?;

    let by_topic = topic_performance(
        connection,
        from.as_deref(),
        to.as_deref(),
        &evidence_from,
        &evidence_to,
    )?;
    let mut weak_topics = by_topic
        .iter()
        .filter(|topic| {
            topic.evidence_questions >= MINIMUM_EVIDENCE_QUESTIONS
                && topic
                    .evidence_accuracy
                    .is_some_and(|accuracy| accuracy < WEAK_ACCURACY_THRESHOLD)
        })
        .cloned()
        .collect::<Vec<_>>();
    weak_topics.sort_by(|left, right| {
        left.evidence_accuracy
            .partial_cmp(&right.evidence_accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .last_scored_attempt_date
                    .cmp(&left.last_scored_attempt_date)
            })
    });

    Ok(PyqAnalytics {
        total_records,
        correct,
        incorrect,
        skipped,
        outcome_unknown,
        attempts: correct + incorrect,
        accuracy: accuracy(correct, incorrect),
        minimum_evidence_questions: MINIMUM_EVIDENCE_QUESTIONS,
        evidence_window_days: EVIDENCE_WINDOW_DAYS,
        weak_accuracy_threshold: WEAK_ACCURACY_THRESHOLD,
        by_topic,
        by_unit: query_unit_performance(connection, from.as_deref(), to.as_deref())?,
        by_year: query_year_performance(connection, from.as_deref(), to.as_deref())?,
        by_part: query_part_performance(connection, from.as_deref(), to.as_deref())?,
        weak_topics,
        response_trend: query_pyq_response_trend(connection, from.as_deref(), to.as_deref())?,
    })
}

pub(crate) fn topic_performance(
    connection: &Connection,
    from: Option<&str>,
    to: Option<&str>,
    evidence_from: &str,
    evidence_to: &str,
) -> Result<Vec<PyqTopicPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                t.id,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title,
                COALESCE(SUM(CASE WHEN p.status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.status = 'skipped' THEN 1 ELSE 0 END), 0),
                COUNT(*),
                COUNT(DISTINCT CASE
                    WHEN p.status IN ('correct', 'incorrect') THEN
                        printf(
                            '%d|%s|%s|%s',
                            p.question_year,
                            lower(trim(p.exam_session)),
                            p.part,
                            lower(trim(p.question_reference))
                        )
                    ELSE NULL
                END),
                MAX(CASE
                    WHEN p.status IN ('correct', 'incorrect') THEN p.attempt_date
                    ELSE NULL
                END)
             FROM pyq_attempts p
             JOIN syllabus_topics t ON t.id = p.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             WHERE (?1 IS NULL OR p.attempt_date >= ?1)
               AND (?2 IS NULL OR p.attempt_date <= ?2)
             GROUP BY t.id
             ORDER BY s.display_order, t.display_order, t.code",
        )
        .map_err(|error| AppError::database("PYQ topic analytics could not be prepared", error))?;

    let mut topics = statement
        .query_map(rusqlite::params![from, to], |row| {
            let correct: i64 = row.get(6)?;
            let incorrect: i64 = row.get(7)?;
            let skipped: i64 = row.get(8)?;
            let total_attempts: i64 = row.get(9)?;
            let distinct_questions: i64 = row.get(10)?;
            let attempts = correct + incorrect;
            Ok(PyqTopicPerformance {
                topic_id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                section_code: row.get(4)?,
                section_title: row.get(5)?,
                correct,
                incorrect,
                skipped,
                attempts,
                total_attempts,
                distinct_questions,
                evidence_questions: 0,
                accuracy: accuracy(correct, incorrect),
                evidence_accuracy: None,
                last_scored_attempt_date: row.get(11)?,
                skipped_only: total_attempts > 0 && attempts == 0 && skipped == total_attempts,
                sufficient_data: false,
            })
        })
        .map_err(|error| AppError::database("PYQ topic analytics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ topic analytics could not be read", error))?;

    let mut evidence_statement = connection
        .prepare(
            "WITH scored AS (
                SELECT
                    p.topic_id,
                    p.status,
                    ROW_NUMBER() OVER (
                        PARTITION BY
                            p.topic_id,
                            p.question_year,
                            lower(trim(p.exam_session)),
                            p.part,
                            lower(trim(p.question_reference))
                        ORDER BY p.attempt_date DESC, p.id DESC
                    ) AS recency_rank
                FROM pyq_attempts p
                WHERE p.status IN ('correct', 'incorrect')
                  AND p.attempt_date >= ?1
                  AND p.attempt_date <= ?2
             )
             SELECT
                s.topic_id,
                t.code,
                t.title,
                t.is_archived,
                section.code,
                section.title,
                COUNT(*),
                SUM(CASE WHEN s.status = 'correct' THEN 1 ELSE 0 END),
                SUM(CASE WHEN s.status = 'incorrect' THEN 1 ELSE 0 END)
             FROM scored s
             JOIN syllabus_topics t ON t.id = s.topic_id
             JOIN syllabus_sections section ON section.id = t.section_id
             WHERE s.recency_rank = 1
             GROUP BY s.topic_id",
        )
        .map_err(|error| {
            AppError::database("PYQ latest-question evidence could not be prepared", error)
        })?;

    let evidence_rows = evidence_statement
        .query_map(rusqlite::params![evidence_from, evidence_to], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })
        .map_err(|error| {
            AppError::database("PYQ latest-question evidence could not be queried", error)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| {
            AppError::database("PYQ latest-question evidence could not be read", error)
        })?;

    let evidence_by_topic = evidence_rows
        .into_iter()
        .map(
            |(
                topic_id,
                code,
                title,
                archived,
                section_code,
                section_title,
                questions,
                correct,
                incorrect,
            )| {
                (
                    topic_id,
                    (
                        code,
                        title,
                        archived,
                        section_code,
                        section_title,
                        questions,
                        correct,
                        incorrect,
                    ),
                )
            },
        )
        .collect::<std::collections::HashMap<_, _>>();
    let existing_topic_ids = topics
        .iter()
        .map(|topic| topic.topic_id)
        .collect::<HashSet<_>>();

    for topic in &mut topics {
        if let Some((_, _, _, _, _, questions, correct, incorrect)) =
            evidence_by_topic.get(&topic.topic_id)
        {
            topic.evidence_questions = *questions;
            topic.evidence_accuracy = accuracy(*correct, *incorrect);
            topic.sufficient_data = *questions >= MINIMUM_EVIDENCE_QUESTIONS;
        }
    }

    let mut evidence_only = Vec::new();
    for (
        topic_id,
        (code, title, archived, section_code, section_title, questions, correct, incorrect),
    ) in evidence_by_topic
    {
        if existing_topic_ids.contains(&topic_id) {
            continue;
        }
        evidence_only.push(PyqTopicPerformance {
            topic_id,
            code,
            title,
            archived,
            section_code,
            section_title,
            correct: 0,
            incorrect: 0,
            skipped: 0,
            attempts: 0,
            total_attempts: 0,
            distinct_questions: 0,
            evidence_questions: questions,
            accuracy: None,
            evidence_accuracy: accuracy(correct, incorrect),
            last_scored_attempt_date: None,
            skipped_only: false,
            sufficient_data: questions >= MINIMUM_EVIDENCE_QUESTIONS,
        });
    }
    evidence_only.sort_by(|left, right| {
        left.section_code
            .cmp(&right.section_code)
            .then_with(|| left.code.cmp(&right.code))
    });
    topics.extend(evidence_only);

    Ok(topics)
}

fn query_unit_performance(
    connection: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<PyqUnitPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                s.code,
                s.title,
                COALESCE(SUM(CASE WHEN p.status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN p.status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts p
             JOIN syllabus_topics t ON t.id = p.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             WHERE (?1 IS NULL OR p.attempt_date >= ?1)
               AND (?2 IS NULL OR p.attempt_date <= ?2)
             GROUP BY s.id
             ORDER BY s.display_order",
        )
        .map_err(|error| AppError::database("PYQ unit analytics could not be prepared", error))?;

    statement
        .query_map(rusqlite::params![from, to], |row| {
            let correct: i64 = row.get(2)?;
            let incorrect: i64 = row.get(3)?;
            Ok(PyqUnitPerformance {
                code: row.get(0)?,
                title: row.get(1)?,
                correct,
                incorrect,
                skipped: row.get(4)?,
                attempts: correct + incorrect,
                accuracy: accuracy(correct, incorrect),
            })
        })
        .map_err(|error| AppError::database("PYQ unit analytics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ unit analytics could not be read", error))
}

fn query_year_performance(
    connection: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<PyqYearPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                question_year,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             WHERE (?1 IS NULL OR attempt_date >= ?1)
               AND (?2 IS NULL OR attempt_date <= ?2)
             GROUP BY question_year
             ORDER BY question_year DESC",
        )
        .map_err(|error| AppError::database("PYQ year analytics could not be prepared", error))?;

    statement
        .query_map(rusqlite::params![from, to], |row| {
            let correct: i64 = row.get(1)?;
            let incorrect: i64 = row.get(2)?;
            Ok(PyqYearPerformance {
                year: row.get(0)?,
                correct,
                incorrect,
                skipped: row.get(3)?,
                attempts: correct + incorrect,
                accuracy: accuracy(correct, incorrect),
            })
        })
        .map_err(|error| AppError::database("PYQ year analytics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ year analytics could not be read", error))
}

fn query_part_performance(
    connection: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<PyqPartPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                part,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             WHERE (?1 IS NULL OR attempt_date >= ?1)
               AND (?2 IS NULL OR attempt_date <= ?2)
             GROUP BY part
             ORDER BY part",
        )
        .map_err(|error| AppError::database("PYQ part analytics could not be prepared", error))?;

    statement
        .query_map(rusqlite::params![from, to], |row| {
            let correct: i64 = row.get(1)?;
            let incorrect: i64 = row.get(2)?;
            Ok(PyqPartPerformance {
                part: row.get(0)?,
                correct,
                incorrect,
                skipped: row.get(3)?,
                attempts: correct + incorrect,
                accuracy: accuracy(correct, incorrect),
            })
        })
        .map_err(|error| AppError::database("PYQ part analytics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ part analytics could not be read", error))
}

fn query_pyq_response_trend(
    connection: &Connection,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<PyqResponseTrendPoint>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                attempt_date,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'attempted' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             WHERE (?1 IS NULL OR attempt_date >= ?1)
               AND (?2 IS NULL OR attempt_date <= ?2)
             GROUP BY attempt_date
             ORDER BY attempt_date",
        )
        .map_err(|error| AppError::database("PYQ response trend could not be prepared", error))?;

    statement
        .query_map(rusqlite::params![from, to], |row| {
            Ok(PyqResponseTrendPoint {
                date: row.get(0)?,
                correct: row.get(1)?,
                incorrect: row.get(2)?,
                skipped: row.get(3)?,
                outcome_unknown: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database("PYQ response trend could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ response trend could not be read", error))
}

pub fn mocks(connection: &Connection) -> Result<MockAnalytics, AppError> {
    mock_analytics_from_tests(mock_tests::list(connection)?)
}

pub fn mocks_in_range(
    connection: &Connection,
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<MockAnalytics, AppError> {
    let from = normalize_optional_date(from_date, "fromDate")?;
    let to = normalize_optional_date(to_date, "toDate")?;
    validate_range_order(from.as_deref(), to.as_deref())?;
    let tests = mock_tests::list(connection)?
        .into_iter()
        .filter(|test| {
            from.as_deref()
                .is_none_or(|value| test.test_date.as_str() >= value)
                && to
                    .as_deref()
                    .is_none_or(|value| test.test_date.as_str() <= value)
        })
        .collect::<Vec<_>>();

    mock_analytics_from_tests(tests)
}

fn mock_analytics_from_tests(tests: Vec<mock_tests::MockTest>) -> Result<MockAnalytics, AppError> {
    let test_count = tests.len() as i64;
    let average_percentage = if tests.is_empty() {
        None
    } else {
        Some(
            tests
                .iter()
                .map(|test| test.computed_percentage)
                .sum::<f64>()
                / test_count as f64,
        )
    };
    let best_percentage = tests
        .iter()
        .map(|test| test.computed_percentage)
        .reduce(f64::max);
    let latest_percentage = tests.first().map(|test| test.computed_percentage);

    let mut trend = tests
        .iter()
        .rev()
        .map(|test| MockTrendPoint {
            id: test.id,
            test_name: test.test_name.clone(),
            test_date: test.test_date.clone(),
            max_marks: test.max_marks,
            computed_marks: test.computed_marks,
            computed_percentage: test.computed_percentage,
            configuration_valid: test.configuration_valid,
            reported_marks: test.marks_obtained,
        })
        .collect::<Vec<_>>();

    trend.sort_by(|left, right| {
        left.test_date
            .cmp(&right.test_date)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut parts = [
        MockPartAccumulator::default(),
        MockPartAccumulator::default(),
        MockPartAccumulator::default(),
    ];
    let mut response_trend: BTreeMap<String, MockResponseAccumulator> = BTreeMap::new();
    for test in &tests {
        let entry = response_trend.entry(test.test_date.clone()).or_default();
        entry.test_count += 1;
        entry.correct += test.total_correct;
        entry.incorrect += test.total_incorrect;
        entry.unanswered += test.total_unanswered;
        entry.percentages.push(test.computed_percentage);

        for (index, part) in [&test.part_a, &test.part_b, &test.part_c]
            .into_iter()
            .enumerate()
        {
            parts[index].questions += part.questions;
            parts[index].attempted += part.attempted;
            parts[index].correct += part.correct;
            parts[index].incorrect += part.incorrect;
            parts[index].unanswered += part.unanswered;
        }
    }

    let by_part = ["A", "B", "C"]
        .into_iter()
        .enumerate()
        .map(|(index, part)| MockPartPerformance {
            part: part.to_owned(),
            questions: parts[index].questions,
            attempted: parts[index].attempted,
            correct: parts[index].correct,
            incorrect: parts[index].incorrect,
            unanswered: parts[index].unanswered,
            accuracy: accuracy(parts[index].correct, parts[index].incorrect),
        })
        .collect::<Vec<_>>();

    let total_questions = by_part.iter().map(|part| part.questions).sum();
    let total_attempted = by_part.iter().map(|part| part.attempted).sum();
    let total_correct = by_part.iter().map(|part| part.correct).sum();
    let total_incorrect = by_part.iter().map(|part| part.incorrect).sum();
    let total_unanswered = by_part.iter().map(|part| part.unanswered).sum();
    let response_trend = response_trend
        .into_iter()
        .map(|(date, data)| {
            let count = data.percentages.len() as i64;
            MockResponseTrendPoint {
                date,
                test_count: data.test_count,
                correct: data.correct,
                incorrect: data.incorrect,
                unanswered: data.unanswered,
                average_percentage: if count == 0 {
                    None
                } else {
                    Some(data.percentages.iter().sum::<f64>() / count as f64)
                },
            }
        })
        .collect();

    Ok(MockAnalytics {
        test_count,
        average_percentage,
        best_percentage,
        latest_percentage,
        total_questions,
        total_attempted,
        total_correct,
        total_incorrect,
        total_unanswered,
        trend,
        by_part,
        response_trend,
    })
}

#[derive(Default)]
struct MockResponseAccumulator {
    test_count: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
    percentages: Vec<f64>,
}

pub fn performance(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    today: NaiveDate,
) -> Result<PerformanceAnalytics, AppError> {
    let from = normalize_required_date(from_date, "fromDate")?;
    let to = normalize_required_date(to_date, "toDate")?;
    validate_range_order(Some(&from), Some(&to))?;

    Ok(PerformanceAnalytics {
        pyq: pyq_in_range(connection, Some(&from), Some(&to), today)?,
        mock: mocks_in_range(connection, Some(&from), Some(&to))?,
        from_date: from,
        to_date: to,
        separation_note: "PYQ and mock-test results are calculated separately. No combined score is produced because they measure different records and marking rules.".to_owned(),
    })
}

fn normalize_required_date(value: &str, field: &'static str) -> Result<String, AppError> {
    let date = date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })?;
    Ok(date_only::format_date_only(date))
}

fn normalize_optional_date(
    value: Option<&str>,
    field: &'static str,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| normalize_required_date(value, field))
        .transpose()
}

fn validate_range_order(from: Option<&str>, to: Option<&str>) -> Result<(), AppError> {
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }
    Ok(())
}

#[derive(Default)]
struct MockPartAccumulator {
    questions: i64,
    attempted: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
}

pub fn study_performance(connection: &Connection) -> Result<StudyPerformance, AppError> {
    #[derive(Default)]
    struct Accumulator {
        study_seconds: i64,
        pyq_correct: i64,
        pyq_incorrect: i64,
        mock_percentages: Vec<f64>,
    }

    let mut by_date: BTreeMap<String, Accumulator> = BTreeMap::new();

    let mut pyq_statement = connection
        .prepare(
            "SELECT
                attempt_date,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             GROUP BY attempt_date",
        )
        .map_err(|error| {
            AppError::database("PYQ study-performance data could not be prepared", error)
        })?;
    let pyq_rows = pyq_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| {
            AppError::database("PYQ study-performance data could not be queried", error)
        })?;
    for row in pyq_rows {
        let (date, correct, incorrect) = row.map_err(|error| {
            AppError::database("PYQ study-performance data could not be read", error)
        })?;
        let entry = by_date.entry(date).or_default();
        entry.pyq_correct += correct;
        entry.pyq_incorrect += incorrect;
    }

    let mut session_statement = connection
        .prepare(
            "SELECT session_date, COALESCE(SUM(duration_seconds), 0)
             FROM study_sessions
             GROUP BY session_date",
        )
        .map_err(|error| {
            AppError::database("Study-time performance data could not be prepared", error)
        })?;
    let session_rows = session_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|error| {
            AppError::database("Study-time performance data could not be queried", error)
        })?;
    for row in session_rows {
        let (date, seconds) = row.map_err(|error| {
            AppError::database("Study-time performance data could not be read", error)
        })?;
        by_date.entry(date).or_default().study_seconds += seconds;
    }

    for test in mock_tests::list(connection)? {
        by_date
            .entry(test.test_date)
            .or_default()
            .mock_percentages
            .push(test.computed_percentage);
    }

    let mut days = by_date
        .into_iter()
        .filter(|(_, data)| {
            data.pyq_correct + data.pyq_incorrect > 0
                || !data.mock_percentages.is_empty()
                || data.study_seconds > 0
        })
        .map(|(date, data)| {
            let mock_count = data.mock_percentages.len() as i64;
            let mock_average_percentage = if data.mock_percentages.is_empty() {
                None
            } else {
                Some(data.mock_percentages.iter().sum::<f64>() / mock_count as f64)
            };
            StudyPerformanceDay {
                date,
                study_seconds: data.study_seconds,
                pyq_correct: data.pyq_correct,
                pyq_incorrect: data.pyq_incorrect,
                pyq_accuracy: accuracy(data.pyq_correct, data.pyq_incorrect),
                mock_count,
                mock_average_percentage,
            }
        })
        .collect::<Vec<_>>();

    days.sort_by(|left, right| right.date.cmp(&left.date));
    days.truncate(MAX_STUDY_PERFORMANCE_DAYS);

    Ok(StudyPerformance { days })
}

fn accuracy(correct: i64, incorrect: i64) -> Option<f64> {
    let attempts = correct + incorrect;
    if attempts == 0 {
        None
    } else {
        Some(correct as f64 / attempts as f64 * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;
    use crate::{db, pyq, revision, sessions, syllabus, topic_intelligence};

    fn connection() -> Connection {
        let mut connection =
            db::test_connection_with_defaults(NaiveDate::from_ymd_opt(2026, 10, 30).expect("date"));
        syllabus::seed(&mut connection).expect("syllabus should seed");
        connection
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 30).expect("date")
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

    fn pyq_input(topic_id: i64, year: i64, part: &str, status: &str, date: &str) -> pyq::PyqInput {
        pyq_input_with_reference(topic_id, year, part, status, date, "Q1")
    }

    fn pyq_input_with_reference(
        topic_id: i64,
        year: i64,
        part: &str,
        status: &str,
        date: &str,
        reference: &str,
    ) -> pyq::PyqInput {
        pyq::PyqInput {
            question_year: year,
            exam_session: String::new(),
            part: part.to_owned(),
            topic_id,
            question_reference: reference.to_owned(),
            attempt_date: date.to_owned(),
            status: status.to_owned(),
            marks_obtained: None,
            max_marks: None,
            time_spent_seconds: 60,
            notes: String::new(),
            difficulty: None,
        }
    }

    fn mock_part(questions: i64, correct: i64, incorrect: i64) -> mock_tests::MockPartInput {
        mock_tests::MockPartInput {
            questions,
            correct,
            incorrect,
            unanswered: questions - correct - incorrect,
            correct_marks: Some(2.0),
            incorrect_marks: Some(-0.5),
            unanswered_marks: Some(0.0),
        }
    }

    fn mock_input(name: &str, date: &str, correct_a: i64) -> mock_tests::MockTestInput {
        mock_tests::MockTestInput {
            test_name: name.to_owned(),
            test_date: date.to_owned(),
            duration_minutes: 180,
            max_marks: 100.0,
            marks_obtained: None,
            part_a: mock_part(10, correct_a, 10 - correct_a),
            part_b: mock_part(20, 10, 10),
            part_c: mock_part(10, 5, 5),
            notes: String::new(),
            review_observations: String::new(),
        }
    }

    #[test]
    fn pyq_analytics_counts_accuracy_units_years_and_weak_topics() {
        let mut connection = connection();
        let mmp = topic_id(&connection, "MMP-C01");
        let ga = topic_id(&connection, "GA-01");

        for (index, status) in ["correct", "correct", "incorrect", "incorrect", "skipped"]
            .into_iter()
            .enumerate()
        {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    mmp,
                    2024,
                    "B",
                    status,
                    "2026-10-30",
                    &format!("MMP-{index}"),
                ),
            )
            .expect("PYQ should save");
        }
        for (index, status) in ["correct", "correct", "incorrect", "incorrect"]
            .into_iter()
            .enumerate()
        {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    ga,
                    2023,
                    "A",
                    status,
                    "2026-10-29",
                    &format!("GA-{index}"),
                ),
            )
            .expect("PYQ should save");
        }

        let analytics = pyq(&connection, today()).expect("analytics should load");
        assert_eq!(analytics.total_records, 9);
        assert_eq!(analytics.correct, 4);
        assert_eq!(analytics.incorrect, 4);
        assert_eq!(analytics.skipped, 1);
        assert_eq!(analytics.attempts, 8);
        assert_eq!(analytics.accuracy, Some(50.0));

        let mmp_stats = analytics
            .by_topic
            .iter()
            .find(|topic| topic.code == "MMP-C01")
            .expect("MMP analytics should exist");
        assert_eq!(mmp_stats.attempts, 4);
        assert_eq!(mmp_stats.accuracy, Some(50.0));
        assert!(!mmp_stats.sufficient_data);

        let ga_stats = analytics
            .by_topic
            .iter()
            .find(|topic| topic.code == "GA-01")
            .expect("GA analytics should exist");
        assert_eq!(ga_stats.attempts, 4);
        assert_eq!(ga_stats.accuracy, Some(50.0));
        assert!(!ga_stats.sufficient_data);

        assert_eq!(analytics.by_unit.len(), 2);
        assert_eq!(analytics.by_year.len(), 2);
        assert_eq!(analytics.by_part.len(), 2);
        assert_eq!(analytics.by_year[0].year, 2024);
        assert_eq!(analytics.by_year[0].correct, 2);
        assert_eq!(analytics.by_year[0].incorrect, 2);
        assert_eq!(analytics.by_year[0].skipped, 1);

        let weak = pyq(&connection, today()).expect("analytics should load");
        assert!(weak.weak_topics.is_empty());

        for index in 0..2 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    mmp,
                    2024,
                    "C",
                    "incorrect",
                    "2026-10-28",
                    &format!("MMP-extra-{index}"),
                ),
            )
            .expect("PYQ should save");
        }
        let analytics = pyq(&connection, today()).expect("analytics should load");
        let weak = analytics
            .weak_topics
            .iter()
            .find(|topic| topic.code == "MMP-C01")
            .expect("MMP should be weak after enough distinct questions");
        assert_eq!(weak.attempts, 6);
        assert_eq!(weak.distinct_questions, 6);
        assert_eq!(weak.evidence_questions, 6);
        assert_eq!(weak.accuracy, Some(33.33333333333333));
        assert_eq!(weak.evidence_accuracy, Some(33.33333333333333));
        assert!(weak.sufficient_data);
    }

    #[test]
    fn repeated_references_do_not_inflate_topic_evidence() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");

        for _ in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2024,
                    "B",
                    "incorrect",
                    "2026-10-30",
                    "SAME-QUESTION",
                ),
            )
            .expect("PYQ should save");
        }

        let analytics = pyq(&connection, today()).expect("analytics should load");
        let stats = analytics
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("topic analytics should exist");
        assert_eq!(stats.attempts, 6);
        assert_eq!(stats.distinct_questions, 1);
        assert_eq!(stats.evidence_questions, 1);
        assert!(!stats.sufficient_data);
        assert!(analytics.weak_topics.is_empty());
    }

    #[test]
    fn topic_evidence_uses_the_latest_outcome_for_each_distinct_question() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2024,
                    "B",
                    "incorrect",
                    "2026-10-01",
                    &format!("LATEST-{index}"),
                ),
            )
            .expect("PYQ should save");
        }
        pyq::create(
            &mut connection,
            pyq_input_with_reference(topic, 2024, "B", "correct", "2026-10-30", "LATEST-0"),
        )
        .expect("recent reattempt should save");

        let analytics = pyq(&connection, today()).expect("analytics should load");
        let stats = analytics
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("topic analytics should exist");
        assert_eq!(stats.attempts, 6);
        assert_eq!(stats.distinct_questions, 5);
        assert_eq!(stats.evidence_questions, 5);
        assert_eq!(stats.accuracy, Some(16.666666666666664));
        assert_eq!(stats.evidence_accuracy, Some(20.0));
        assert_eq!(
            stats.last_scored_attempt_date.as_deref(),
            Some("2026-10-30")
        );
    }

    #[test]
    fn stale_questions_do_not_count_as_current_evidence() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2019,
                    "B",
                    "incorrect",
                    "2019-06-01",
                    &format!("STALE-{index}"),
                ),
            )
            .expect("PYQ should save");
        }

        let analytics = pyq(&connection, today()).expect("analytics should load");
        assert_eq!(analytics.evidence_window_days, EVIDENCE_WINDOW_DAYS);
        let stats = analytics
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("topic analytics should exist");
        assert_eq!(stats.attempts, 5);
        assert_eq!(stats.distinct_questions, 5);
        assert_eq!(stats.evidence_questions, 0);
        assert_eq!(stats.evidence_accuracy, None);
        assert!(!stats.sufficient_data);
        assert!(analytics.weak_topics.is_empty());
    }

    #[test]
    fn evidence_window_is_exactly_365_calendar_dates() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2024,
                    "B",
                    "incorrect",
                    "2025-10-31",
                    &format!("BOUNDARY-{index}"),
                ),
            )
            .expect("boundary PYQ should save");
        }
        pyq::create(
            &mut connection,
            pyq_input_with_reference(topic, 2023, "B", "correct", "2025-10-30", "OLDER"),
        )
        .expect("older PYQ should remain in history");

        let analytics = pyq(&connection, today()).expect("analytics should load");
        let stats = analytics
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("topic analytics should exist");
        assert_eq!(stats.attempts, 6);
        assert_eq!(stats.distinct_questions, 6);
        assert_eq!(stats.evidence_questions, 5);
        assert_eq!(stats.evidence_accuracy, Some(0.0));
        assert!(stats.sufficient_data);
    }

    #[test]
    fn future_and_older_attempts_do_not_count_as_current_evidence() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C02");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2024,
                    "B",
                    "correct",
                    "2027-01-01",
                    &format!("FUTURE-{index}"),
                ),
            )
            .expect("future PYQ should save for history");
        }

        let analytics = pyq(&connection, today()).expect("analytics should load");
        let stats = analytics
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("topic analytics should exist");
        assert_eq!(stats.distinct_questions, 5);
        assert_eq!(stats.evidence_questions, 0);
        assert_eq!(stats.evidence_accuracy, None);
        assert!(!stats.sufficient_data);
        assert!(
            !analytics
                .weak_topics
                .iter()
                .any(|entry| entry.topic_id == topic)
        );
    }

    #[test]
    fn short_report_ranges_do_not_limit_the_rolling_evidence_window() {
        let mut connection = connection();
        let topic = topic_id(&connection, "GA-01");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_with_reference(
                    topic,
                    2023,
                    "A",
                    "incorrect",
                    "2025-11-01",
                    &format!("OUTSIDE-RANGE-{index}"),
                ),
            )
            .expect("evidence outside the report range should save");
        }

        let filtered = performance(&connection, "2026-10-10", "2026-10-20", today())
            .expect("performance should load");
        let matching_topics = filtered
            .pyq
            .by_topic
            .iter()
            .filter(|entry| entry.topic_id == topic)
            .collect::<Vec<_>>();
        assert_eq!(
            matching_topics.len(),
            1,
            "evidence-only topic must not duplicate"
        );
        let filtered_topic = matching_topics[0];
        assert_eq!(filtered.pyq.total_records, 0);
        assert_eq!(filtered_topic.attempts, 0);
        assert_eq!(filtered_topic.correct, 0);
        assert_eq!(filtered_topic.incorrect, 0);
        assert_eq!(filtered_topic.skipped, 0);
        assert_eq!(filtered_topic.total_attempts, 0);
        assert_eq!(filtered_topic.distinct_questions, 0);
        assert_eq!(filtered_topic.accuracy, None);
        assert_eq!(filtered_topic.last_scored_attempt_date, None);
        assert_eq!(filtered_topic.evidence_questions, 5);
        assert_eq!(filtered_topic.evidence_accuracy, Some(0.0));
        assert!(
            filtered
                .pyq
                .weak_topics
                .iter()
                .any(|entry| entry.topic_id == topic),
            "filtered Analytics must use the rolling evidence window for weak claims"
        );

        let full = pyq(&connection, today()).expect("unfiltered analytics should load");
        let full_topic = full
            .by_topic
            .iter()
            .find(|entry| entry.topic_id == topic)
            .expect("unfiltered topic analytics should exist");
        assert_eq!(full_topic.distinct_questions, 5);
        assert_eq!(full_topic.evidence_questions, 5);

        let intelligence = topic_intelligence::topic_intelligence(&connection, today())
            .expect("topic intelligence should load");
        let weak = intelligence
            .weak
            .iter()
            .find(|entry| entry.code == "GA-01")
            .expect("topic intelligence should use the rolling evidence window");
        assert_eq!(weak.pyq_questions, 5);

        let revision = revision::dashboard(&connection, today()).expect("revision should load");
        let recommendation = revision
            .recommendations
            .iter()
            .find(|entry| entry.topic.code == "GA-01")
            .expect("revision should use the same rolling evidence window");
        assert_eq!(recommendation.pyq_questions, 5);
    }

    #[test]
    fn empty_report_ranges_do_not_fabricate_topics_without_scored_evidence() {
        let mut connection = connection();
        let topic = topic_id(&connection, "CM-C01");
        pyq::create(
            &mut connection,
            pyq_input_with_reference(topic, 2024, "A", "skipped", "2025-11-01", "SKIPPED"),
        )
        .expect("skipped PYQ should save");

        let filtered = performance(&connection, "2026-10-10", "2026-10-20", today())
            .expect("performance should load");
        assert_eq!(filtered.pyq.total_records, 0);
        assert!(filtered.pyq.by_topic.is_empty());
        assert!(filtered.pyq.weak_topics.is_empty());
    }

    #[test]
    fn mock_analytics_uses_calculated_scores_and_part_counts() {
        let mut connection = connection();
        mock_tests::create(&mut connection, mock_input("Mock 1", "2026-10-01", 5))
            .expect("mock should save");
        mock_tests::create(&mut connection, mock_input("Mock 2", "2026-10-15", 8))
            .expect("mock should save");

        let analytics = mocks(&connection).expect("analytics should load");
        assert_eq!(analytics.test_count, 2);
        assert_eq!(analytics.by_part.len(), 3);
        assert_eq!(analytics.by_part[0].questions, 20);
        assert_eq!(analytics.by_part[0].correct, 13);
        assert_eq!(analytics.by_part[0].incorrect, 7);
        assert_eq!(analytics.trend.len(), 2);
        assert_eq!(analytics.trend[0].test_name, "Mock 1");
        assert_eq!(analytics.trend[1].test_name, "Mock 2");
        assert!(analytics.average_percentage.is_some());
        assert!(analytics.best_percentage.is_some());
        assert_eq!(
            analytics.latest_percentage,
            analytics
                .trend
                .last()
                .map(|point| point.computed_percentage)
        );
    }

    #[test]
    fn study_performance_combines_dates_and_distinguishes_missing_outcomes() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        pyq::create(
            &mut connection,
            pyq_input(topic, 2024, "B", "correct", "2026-10-30"),
        )
        .expect("PYQ should save");
        pyq::create(
            &mut connection,
            pyq_input(topic, 2024, "B", "incorrect", "2026-10-29"),
        )
        .expect("PYQ should save");
        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-10-30".to_owned(),
                duration_seconds: 3_600,
                notes: String::new(),
            },
        )
        .expect("session should save");
        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-10-28".to_owned(),
                duration_seconds: 1_800,
                notes: "Study without a recorded outcome".to_owned(),
            },
        )
        .expect("session should save");

        let performance = study_performance(&connection).expect("performance should load");
        assert_eq!(performance.days.len(), 3);
        assert_eq!(performance.days[0].date, "2026-10-30");
        assert_eq!(performance.days[0].study_seconds, 3_600);
        assert_eq!(performance.days[0].pyq_correct, 1);
        assert_eq!(performance.days[0].pyq_incorrect, 0);
        assert_eq!(performance.days[0].pyq_accuracy, Some(100.0));
        assert_eq!(performance.days[1].date, "2026-10-29");
        assert_eq!(performance.days[1].pyq_accuracy, Some(0.0));
        assert_eq!(performance.days[1].study_seconds, 0);
        assert_eq!(performance.days[2].date, "2026-10-28");
        assert_eq!(performance.days[2].study_seconds, 1_800);
        assert_eq!(performance.days[2].pyq_accuracy, None);
        assert_eq!(performance.days[2].mock_count, 0);
    }

    #[test]
    fn filtered_performance_keeps_pyq_and_mock_metrics_separate_and_tracks_responses() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        pyq::create(
            &mut connection,
            pyq_input(topic, 2024, "A", "correct", "2026-09-30"),
        )
        .expect("earlier PYQ should save");
        pyq::create(
            &mut connection,
            pyq_input(topic, 2024, "B", "incorrect", "2026-10-15"),
        )
        .expect("later PYQ should save");
        mock_tests::create(&mut connection, mock_input("Early mock", "2026-10-01", 5))
            .expect("earlier mock should save");
        mock_tests::create(
            &mut connection,
            mock_input("Filtered mock", "2026-10-15", 8),
        )
        .expect("later mock should save");

        let filtered = performance(&connection, "2026-10-10", "2026-10-20", today())
            .expect("performance should load");
        assert_eq!(filtered.pyq.total_records, 1);
        assert_eq!(filtered.pyq.correct, 0);
        assert_eq!(filtered.pyq.incorrect, 1);
        assert_eq!(filtered.pyq.accuracy, Some(0.0));
        assert_eq!(filtered.pyq.by_topic.len(), 1);
        assert_eq!(filtered.pyq.response_trend[0].date, "2026-10-15");
        assert_eq!(filtered.pyq.response_trend[0].incorrect, 1);

        assert_eq!(filtered.mock.test_count, 1);
        assert_eq!(filtered.mock.trend[0].test_name, "Filtered mock");
        assert_eq!(
            filtered.mock.latest_percentage,
            filtered.mock.best_percentage
        );
        assert_eq!(filtered.mock.total_questions, 40);
        assert_eq!(filtered.mock.total_attempted, 40);
        assert_eq!(filtered.mock.total_unanswered, 0);
        assert_eq!(filtered.mock.response_trend[0].test_count, 1);
        assert_eq!(filtered.mock.response_trend[0].correct, 23);
        assert_eq!(filtered.mock.response_trend[0].incorrect, 17);
        assert_eq!(filtered.mock.response_trend[0].unanswered, 0);

        assert!(filtered.separation_note.contains("No combined score"));
        assert_eq!(
            filtered
                .pyq
                .response_trend
                .iter()
                .map(|point| point.date.clone())
                .collect::<Vec<_>>(),
            vec!["2026-10-15".to_owned()]
        );
        assert_eq!(
            pyq_in_range(&connection, Some("2026-10-01"), Some("2026-10-15"), today())
                .expect("bounded PYQ analytics should load")
                .total_records,
            1
        );
        assert_eq!(
            mocks_in_range(&connection, None, Some("2026-10-01"))
                .expect("bounded mock analytics should load")
                .test_count,
            1
        );
        assert_eq!(
            performance(&connection, "2026-10-20", "2026-10-10", today())
                .expect_err("reversed range should fail")
                .field,
            Some("fromDate")
        );
    }
}
