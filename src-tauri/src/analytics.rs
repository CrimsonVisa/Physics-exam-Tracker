use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use crate::{error::AppError, mock_tests};

pub const MINIMUM_WEAK_ATTEMPTS: i64 = 5;
pub const WEAK_ACCURACY_THRESHOLD: f64 = 50.0;
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
    pub accuracy: Option<f64>,
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
    pub minimum_weak_attempts: i64,
    pub weak_accuracy_threshold: f64,
    pub by_topic: Vec<PyqTopicPerformance>,
    pub by_unit: Vec<PyqUnitPerformance>,
    pub by_year: Vec<PyqYearPerformance>,
    pub by_part: Vec<PyqPartPerformance>,
    pub weak_topics: Vec<PyqTopicPerformance>,
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
pub struct MockAnalytics {
    pub test_count: i64,
    pub average_percentage: Option<f64>,
    pub best_percentage: Option<f64>,
    pub latest_percentage: Option<f64>,
    pub trend: Vec<MockTrendPoint>,
    pub by_part: Vec<MockPartPerformance>,
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

pub fn pyq(connection: &Connection) -> Result<PyqAnalytics, AppError> {
    let (total_records, correct, incorrect, skipped, outcome_unknown): (i64, i64, i64, i64, i64) =
        connection
            .query_row(
                "SELECT
                    count(*),
                    COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'attempted' THEN 1 ELSE 0 END), 0)
                 FROM pyq_attempts",
                [],
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

    let by_topic = query_topic_performance(connection)?;
    let mut weak_topics = by_topic
        .iter()
        .filter(|topic| {
            topic.attempts >= MINIMUM_WEAK_ATTEMPTS
                && topic
                    .accuracy
                    .is_some_and(|accuracy| accuracy < WEAK_ACCURACY_THRESHOLD)
        })
        .cloned()
        .collect::<Vec<_>>();
    weak_topics.sort_by(|left, right| {
        left.accuracy
            .partial_cmp(&right.accuracy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(PyqAnalytics {
        total_records,
        correct,
        incorrect,
        skipped,
        outcome_unknown,
        attempts: correct + incorrect,
        accuracy: accuracy(correct, incorrect),
        minimum_weak_attempts: MINIMUM_WEAK_ATTEMPTS,
        weak_accuracy_threshold: WEAK_ACCURACY_THRESHOLD,
        by_topic,
        by_unit: query_unit_performance(connection)?,
        by_year: query_year_performance(connection)?,
        by_part: query_part_performance(connection)?,
        weak_topics,
    })
}

fn query_topic_performance(connection: &Connection) -> Result<Vec<PyqTopicPerformance>, AppError> {
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
                COALESCE(SUM(CASE WHEN p.status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts p
             JOIN syllabus_topics t ON t.id = p.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             GROUP BY t.id
             ORDER BY s.display_order, t.display_order, t.code",
        )
        .map_err(|error| AppError::database("PYQ topic analytics could not be prepared", error))?;

    statement
        .query_map([], |row| {
            let correct: i64 = row.get(6)?;
            let incorrect: i64 = row.get(7)?;
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
                skipped: row.get(8)?,
                attempts,
                accuracy: accuracy(correct, incorrect),
                sufficient_data: attempts >= MINIMUM_WEAK_ATTEMPTS,
            })
        })
        .map_err(|error| AppError::database("PYQ topic analytics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("PYQ topic analytics could not be read", error))
}

fn query_unit_performance(connection: &Connection) -> Result<Vec<PyqUnitPerformance>, AppError> {
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
             GROUP BY s.id
             ORDER BY s.display_order",
        )
        .map_err(|error| AppError::database("PYQ unit analytics could not be prepared", error))?;

    statement
        .query_map([], |row| {
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

fn query_year_performance(connection: &Connection) -> Result<Vec<PyqYearPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                question_year,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             GROUP BY question_year
             ORDER BY question_year DESC",
        )
        .map_err(|error| AppError::database("PYQ year analytics could not be prepared", error))?;

    statement
        .query_map([], |row| {
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

fn query_part_performance(connection: &Connection) -> Result<Vec<PyqPartPerformance>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                part,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             GROUP BY part
             ORDER BY part",
        )
        .map_err(|error| AppError::database("PYQ part analytics could not be prepared", error))?;

    statement
        .query_map([], |row| {
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

pub fn mocks(connection: &Connection) -> Result<MockAnalytics, AppError> {
    let tests = mock_tests::list(connection)?;
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
    for test in &tests {
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

    Ok(MockAnalytics {
        test_count,
        average_percentage,
        best_percentage,
        latest_percentage,
        trend,
        by_part: ["A", "B", "C"]
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
            .collect(),
    })
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
    use crate::{db, pyq, sessions, syllabus};

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

    fn pyq_input(topic_id: i64, year: i64, part: &str, status: &str, date: &str) -> pyq::PyqInput {
        pyq::PyqInput {
            question_year: year,
            exam_session: String::new(),
            part: part.to_owned(),
            topic_id,
            question_reference: "Q1".to_owned(),
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

        for status in ["correct", "correct", "incorrect", "incorrect", "skipped"] {
            pyq::create(
                &mut connection,
                pyq_input(mmp, 2024, "B", status, "2026-10-30"),
            )
            .expect("PYQ should save");
        }
        for status in ["correct", "correct", "incorrect", "incorrect"] {
            pyq::create(
                &mut connection,
                pyq_input(ga, 2023, "A", status, "2026-10-29"),
            )
            .expect("PYQ should save");
        }

        let analytics = pyq(&connection).expect("analytics should load");
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

        let weak = pyq(&connection).expect("analytics should load");
        assert!(weak.weak_topics.is_empty());

        for _ in 0..2 {
            pyq::create(
                &mut connection,
                pyq_input(mmp, 2024, "C", "incorrect", "2026-10-28"),
            )
            .expect("PYQ should save");
        }
        let analytics = pyq(&connection).expect("analytics should load");
        let weak = analytics
            .weak_topics
            .iter()
            .find(|topic| topic.code == "MMP-C01")
            .expect("MMP should be weak after enough attempts");
        assert_eq!(weak.attempts, 6);
        assert_eq!(weak.accuracy, Some(33.33333333333333));
        assert!(weak.sufficient_data);
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
}
