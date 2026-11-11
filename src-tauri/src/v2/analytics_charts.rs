use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::date_only;
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsChartsInput {
    #[serde(default)]
    pub exam_id: Option<i64>,
    pub from_date: String,
    pub to_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ChartPoint {
    pub label: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StudyByExam {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub allocated_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StudyBySubject {
    pub exam_id: i64,
    pub exam_code: String,
    pub subject_node_id: Option<i64>,
    pub subject_title: String,
    pub allocated_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2WeeklyConsistency {
    pub week_start: String,
    pub label: String,
    pub eligible_days: i64,
    pub active_days: i64,
    pub study_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuestionOutcomes {
    pub attempts: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub graded_attempts: i64,
    pub accuracy_percentage: Option<f64>,
    pub insufficient_sample: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamQuestionOutcomes {
    pub exam_id: i64,
    pub exam_code: String,
    pub outcomes: V2QuestionOutcomes,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TopicAccuracy {
    pub exam_id: i64,
    pub exam_code: String,
    pub node_id: i64,
    pub topic_title: String,
    pub attempts: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub graded_attempts: i64,
    pub accuracy_percentage: Option<f64>,
    pub insufficient_sample: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SyllabusCompletion {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub topic_count: i64,
    pub completed_topic_count: i64,
    pub completion_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockTrendPoint {
    pub exam_id: i64,
    pub exam_code: String,
    pub test_id: i64,
    pub test_date: String,
    pub test_name: String,
    pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MockExamSummary {
    pub exam_id: i64,
    pub exam_code: String,
    pub completed_count: i64,
    pub average_percentage: Option<f64>,
    pub best_percentage: Option<f64>,
    pub last_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsCharts {
    pub range_from: String,
    pub range_to: String,
    pub total_seconds: i64,
    pub allocated_seconds: i64,
    pub unallocated_seconds: i64,
    pub study_by_exam: Vec<V2StudyByExam>,
    pub study_by_subject: Vec<V2StudyBySubject>,
    pub daily_study: Vec<V2ChartPoint>,
    pub weekly_consistency: Vec<V2WeeklyConsistency>,
    pub question_outcomes: V2QuestionOutcomes,
    pub outcomes_by_exam: Vec<V2ExamQuestionOutcomes>,
    pub topic_accuracy: Vec<V2TopicAccuracy>,
    pub syllabus_completion: Vec<V2SyllabusCompletion>,
    pub mock_trend: Vec<V2MockTrendPoint>,
    pub mock_by_exam: Vec<V2MockExamSummary>,
}

fn percentage(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some((numerator as f64 * 100.0 / denominator as f64 * 1000.0).round() / 1000.0)
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn validate_input(
    connection: &Connection,
    input: &V2AnalyticsChartsInput,
) -> Result<(NaiveDate, NaiveDate), AppError> {
    let from = date_only::parse_date_only(&input.from_date)
        .ok_or_else(|| AppError::validation("fromDate", "Enter a real calendar start date."))?;
    let to = date_only::parse_date_only(&input.to_date)
        .ok_or_else(|| AppError::validation("toDate", "Enter a real calendar end date."))?;
    if to < from {
        return Err(AppError::validation(
            "toDate",
            "End date cannot be before the start date.",
        ));
    }
    if (to - from).num_days() + 1 > 365 {
        return Err(AppError::validation(
            "toDate",
            "Analytics ranges cannot exceed 365 days.",
        ));
    }
    if let Some(exam_id) = input.exam_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE id=?1)",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("The analytics examination could not be checked", error)
            })?;
        if exists == 0 {
            return Err(AppError::validation(
                "examId",
                "The selected examination does not exist.",
            ));
        }
    }
    Ok((from, to))
}

fn study_by_exam(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<Vec<V2StudyByExam>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, e.short_name,
                    SUM(a.allocated_seconds), COUNT(DISTINCT a.session_id)
             FROM session_exam_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             JOIN exams e ON e.id = a.exam_id
             WHERE s.session_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             GROUP BY a.exam_id, e.code, e.short_name, e.display_order
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("Study-by-exam chart could not be prepared", error))?;
    let rows = statement
        .query_map(params![from, to, exam_id], |row| {
            Ok(V2StudyByExam {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                exam_short_name: row.get(2)?,
                allocated_seconds: row.get(3)?,
                session_count: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database("Study-by-exam chart could not be read", error))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(
            row.map_err(|error| AppError::database("A study-by-exam row is invalid", error))?,
        );
    }
    Ok(values)
}

fn study_by_subject(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<Vec<V2StudyBySubject>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, parent.id, COALESCE(parent.title, ''),
                    SUM(a.allocated_seconds), COUNT(DISTINCT a.session_id)
             FROM session_topic_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
                ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             WHERE s.session_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             GROUP BY a.exam_id, e.code, parent.id, parent.title, e.display_order
             ORDER BY e.display_order, SUM(a.allocated_seconds) DESC",
        )
        .map_err(|error| {
            AppError::database("Study-by-subject chart could not be prepared", error)
        })?;
    let rows = statement
        .query_map(params![from, to, exam_id], |row| {
            Ok(V2StudyBySubject {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                subject_node_id: row.get(2)?,
                subject_title: row.get(3)?,
                allocated_seconds: row.get(4)?,
                session_count: row.get(5)?,
            })
        })
        .map_err(|error| AppError::database("Study-by-subject chart could not be read", error))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(
            row.map_err(|error| AppError::database("A study-by-subject row is invalid", error))?,
        );
    }
    Ok(values)
}

fn daily_study(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<Vec<V2ChartPoint>, AppError> {
    let sql = if exam_id.is_some() {
        "SELECT s.session_date, SUM(a.allocated_seconds), COUNT(DISTINCT s.id)
         FROM session_exam_allocations a
         JOIN study_sessions s ON s.id = a.session_id
         WHERE a.exam_id = ?3 AND s.session_date BETWEEN ?1 AND ?2
         GROUP BY s.session_date ORDER BY s.session_date"
    } else {
        "SELECT session_date, SUM(duration_seconds), COUNT(*)
         FROM study_sessions WHERE session_date BETWEEN ?1 AND ?2
         GROUP BY session_date ORDER BY session_date"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Daily chart could not be prepared", error))?;
    let mut points = Vec::new();
    if exam_id.is_some() {
        let rows = statement
            .query_map(params![from, to, exam_id], |row| {
                Ok(V2ChartPoint {
                    label: row.get(0)?,
                    seconds: row.get(1)?,
                    session_count: row.get(2)?,
                })
            })
            .map_err(|error| AppError::database("Daily chart could not be read", error))?;
        for row in rows {
            points.push(
                row.map_err(|error| AppError::database("A daily chart row is invalid", error))?,
            );
        }
    } else {
        let rows = statement
            .query_map(params![from, to], |row| {
                Ok(V2ChartPoint {
                    label: row.get(0)?,
                    seconds: row.get(1)?,
                    session_count: row.get(2)?,
                })
            })
            .map_err(|error| AppError::database("Daily chart could not be read", error))?;
        for row in rows {
            points.push(
                row.map_err(|error| AppError::database("A daily chart row is invalid", error))?,
            );
        }
    }
    Ok(points)
}

fn weekly_consistency(
    from: NaiveDate,
    to: NaiveDate,
    daily: &[V2ChartPoint],
) -> Vec<V2WeeklyConsistency> {
    let by_date: HashMap<&str, &V2ChartPoint> = daily
        .iter()
        .map(|point| (point.label.as_str(), point))
        .collect();
    let mut weeks: BTreeMap<NaiveDate, V2WeeklyConsistency> = BTreeMap::new();
    let mut current = from;
    while current <= to {
        let week_start = current - Duration::days(current.weekday().num_days_from_monday() as i64);
        let entry = weeks
            .entry(week_start)
            .or_insert_with(|| V2WeeklyConsistency {
                week_start: date_only::format_date_only(week_start),
                label: date_only::format_date_only(week_start),
                eligible_days: 0,
                active_days: 0,
                study_seconds: 0,
                session_count: 0,
            });
        entry.eligible_days += 1;
        if let Some(point) = by_date.get(date_only::format_date_only(current).as_str())
            && point.seconds > 0
        {
            entry.active_days += 1;
            entry.study_seconds += point.seconds;
            entry.session_count += point.session_count;
        }
        current += Duration::days(1);
    }
    weeks.into_values().collect()
}

struct OutcomeBuilder {
    attempts: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
}

impl OutcomeBuilder {
    fn new() -> Self {
        Self {
            attempts: 0,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
        }
    }

    fn add(&mut self, status: &str) {
        self.attempts += 1;
        if status == "skipped" || status == "unanswered" {
            self.unanswered += 1;
        } else if status == "correct" {
            self.correct += 1;
        } else {
            self.incorrect += 1;
        }
    }

    fn finish(&self) -> V2QuestionOutcomes {
        let graded = self.correct + self.incorrect;
        V2QuestionOutcomes {
            attempts: self.attempts,
            correct: self.correct,
            incorrect: self.incorrect,
            unanswered: self.unanswered,
            graded_attempts: graded,
            accuracy_percentage: percentage(self.correct, graded),
            insufficient_sample: graded < 5,
        }
    }
}

fn question_outcomes(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<(V2QuestionOutcomes, Vec<V2ExamQuestionOutcomes>), AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, a.status
             FROM pyq_attempts a
             JOIN exams e ON e.id = a.exam_id
             WHERE a.attempt_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             ORDER BY e.display_order",
        )
        .map_err(|error| {
            AppError::database("Question outcome chart could not be prepared", error)
        })?;
    let rows = statement
        .query_map(params![from, to, exam_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Question outcome chart could not be read", error))?;
    let mut total = OutcomeBuilder::new();
    let mut by_exam: BTreeMap<i64, (String, OutcomeBuilder)> = BTreeMap::new();
    for row in rows {
        let (id, code, status) = row.map_err(|error| {
            AppError::database("A question outcome chart row is invalid", error)
        })?;
        total.add(&status);
        by_exam
            .entry(id)
            .or_insert_with(|| (code, OutcomeBuilder::new()))
            .1
            .add(&status);
    }
    let outcomes_by_exam = by_exam
        .into_iter()
        .map(|(id, (code, builder))| V2ExamQuestionOutcomes {
            exam_id: id,
            exam_code: code,
            outcomes: builder.finish(),
        })
        .collect();
    Ok((total.finish(), outcomes_by_exam))
}

fn topic_accuracy(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<Vec<V2TopicAccuracy>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, n.id, n.title, a.status
             FROM pyq_attempts a
             JOIN pyq_questions q ON q.id = a.question_id
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n
                ON n.exam_id = a.exam_id AND n.id = q.syllabus_node_id
             WHERE a.attempt_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             ORDER BY e.display_order, a.exam_id, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("Topic accuracy chart could not be prepared", error))?;
    let rows = statement
        .query_map(params![from, to, exam_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| AppError::database("Topic accuracy chart could not be read", error))?;
    let mut values: BTreeMap<(i64, i64), (String, String, OutcomeBuilder)> = BTreeMap::new();
    for row in rows {
        let (exam, exam_code, node_id, title, status) = row
            .map_err(|error| AppError::database("A topic accuracy chart row is invalid", error))?;
        values
            .entry((exam, node_id))
            .or_insert_with(|| (exam_code, title, OutcomeBuilder::new()))
            .2
            .add(&status);
    }
    Ok(values
        .into_iter()
        .map(|((exam_id, node_id), (exam_code, topic_title, builder))| {
            let outcomes = builder.finish();
            V2TopicAccuracy {
                exam_id,
                exam_code,
                node_id,
                topic_title,
                attempts: outcomes.attempts,
                correct: outcomes.correct,
                incorrect: outcomes.incorrect,
                unanswered: outcomes.unanswered,
                graded_attempts: outcomes.graded_attempts,
                accuracy_percentage: outcomes.accuracy_percentage,
                insufficient_sample: outcomes.insufficient_sample,
            }
        })
        .collect())
}

fn syllabus_completion(
    connection: &Connection,
    exam_id: Option<i64>,
) -> Result<Vec<V2SyllabusCompletion>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.code, e.short_name,
                    COUNT(CASE WHEN n.node_kind='topic' AND n.is_archived=0 THEN 1 END),
                    COALESCE(SUM(CASE
                        WHEN n.node_kind='topic' AND n.is_archived=0
                             AND p.status IN ('revised','confident') THEN 1 ELSE 0 END), 0)
             FROM exams e
             LEFT JOIN exam_syllabus_nodes n ON n.exam_id = e.id
             LEFT JOIN exam_node_progress p
                ON p.exam_id = n.exam_id AND p.node_id = n.id
             WHERE (?1 IS NULL OR e.id = ?1)
             GROUP BY e.id, e.code, e.short_name, e.display_order
             ORDER BY e.display_order",
        )
        .map_err(|error| {
            AppError::database("Syllabus completion chart could not be prepared", error)
        })?;
    let rows = statement
        .query_map([exam_id], |row| {
            let topic_count: i64 = row.get(3)?;
            let completed: i64 = row.get(4)?;
            Ok(V2SyllabusCompletion {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                exam_short_name: row.get(2)?,
                topic_count,
                completed_topic_count: completed,
                completion_percentage: percentage(completed, topic_count),
            })
        })
        .map_err(|error| {
            AppError::database("Syllabus completion chart could not be read", error)
        })?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|error| {
            AppError::database("A syllabus completion chart row is invalid", error)
        })?);
    }
    Ok(values)
}

fn mock_analytics(
    connection: &Connection,
    exam_id: Option<i64>,
    from: &str,
    to: &str,
) -> Result<(Vec<V2MockTrendPoint>, Vec<V2MockExamSummary>), AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.code, m.id, m.test_date, m.test_name,
                    CASE
                        WHEN m.percentage IS NOT NULL THEN m.percentage
                        WHEN m.max_marks > 0 AND m.marks_obtained IS NOT NULL
                            THEN m.marks_obtained * 100.0 / m.max_marks
                        ELSE NULL
                    END
             FROM mock_tests m
             JOIN exams e ON e.id = m.exam_id
             WHERE m.status='completed' AND m.is_archived=0
               AND (m.percentage IS NOT NULL
                    OR (m.max_marks > 0 AND m.marks_obtained IS NOT NULL))
               AND m.test_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR m.exam_id = ?3)
             ORDER BY e.display_order, m.test_date, m.id",
        )
        .map_err(|error| AppError::database("Mock trend chart could not be prepared", error))?;
    let rows = statement
        .query_map(params![from, to, exam_id], |row| {
            Ok(V2MockTrendPoint {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                test_id: row.get(2)?,
                test_date: row.get(3)?,
                test_name: row.get(4)?,
                percentage: row.get(5)?,
            })
        })
        .map_err(|error| AppError::database("Mock trend chart could not be read", error))?;
    let mut trend = Vec::new();
    let mut by_exam: BTreeMap<i64, (String, Vec<f64>, Option<f64>)> = BTreeMap::new();
    for row in rows {
        let point =
            row.map_err(|error| AppError::database("A mock trend row is invalid", error))?;
        by_exam
            .entry(point.exam_id)
            .or_insert_with(|| (point.exam_code.clone(), Vec::new(), None))
            .1
            .push(point.percentage);
        by_exam
            .entry(point.exam_id)
            .or_insert_with(|| (point.exam_code.clone(), Vec::new(), None))
            .2 = Some(point.percentage);
        trend.push(point);
    }
    let summaries = by_exam
        .into_iter()
        .map(|(id, (code, values, last))| {
            let average = if values.is_empty() {
                None
            } else {
                Some(round3(values.iter().sum::<f64>() / values.len() as f64))
            };
            let best = values.iter().copied().reduce(f64::max);
            V2MockExamSummary {
                exam_id: id,
                exam_code: code,
                completed_count: values.len() as i64,
                average_percentage: average,
                best_percentage: best,
                last_percentage: last,
            }
        })
        .collect();
    Ok((trend, summaries))
}

pub fn get_analytics_charts(
    connection: &Connection,
    input: V2AnalyticsChartsInput,
) -> Result<V2AnalyticsCharts, AppError> {
    let (from, to) = validate_input(connection, &input)?;
    let from_text = date_only::format_date_only(from);
    let to_text = date_only::format_date_only(to);
    let study_by_exam = study_by_exam(connection, input.exam_id, &from_text, &to_text)?;
    let study_by_subject = study_by_subject(connection, input.exam_id, &from_text, &to_text)?;
    let daily_study = daily_study(connection, input.exam_id, &from_text, &to_text)?;
    let weekly_consistency = weekly_consistency(from, to, &daily_study);
    let allocated_seconds = study_by_exam
        .iter()
        .map(|entry| entry.allocated_seconds)
        .sum();
    let total_seconds = if input.exam_id.is_some() {
        allocated_seconds
    } else {
        connection
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0) FROM study_sessions
                 WHERE session_date BETWEEN ?1 AND ?2",
                params![from_text, to_text],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Total study time could not be read", error))?
    };
    let (question_outcomes, outcomes_by_exam) =
        question_outcomes(connection, input.exam_id, &from_text, &to_text)?;
    let topic_accuracy = topic_accuracy(connection, input.exam_id, &from_text, &to_text)?;
    let syllabus_completion = syllabus_completion(connection, input.exam_id)?;
    let (mock_trend, mock_by_exam) =
        mock_analytics(connection, input.exam_id, &from_text, &to_text)?;
    Ok(V2AnalyticsCharts {
        range_from: from_text,
        range_to: to_text,
        total_seconds,
        allocated_seconds,
        unallocated_seconds: (total_seconds - allocated_seconds).max(0),
        study_by_exam,
        study_by_subject,
        daily_study,
        weekly_consistency,
        question_outcomes,
        outcomes_by_exam,
        topic_accuracy,
        syllabus_completion,
        mock_trend,
        mock_by_exam,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;
    use chrono::TimeZone;

    fn at(seconds: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 10, 10, 0, 0)
            .single()
            .expect("timestamp")
            + Duration::seconds(seconds)
    }

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn input(exam_id: Option<i64>, from: &str, to: &str) -> V2AnalyticsChartsInput {
        V2AnalyticsChartsInput {
            exam_id,
            from_date: from.to_owned(),
            to_date: to.to_owned(),
        }
    }

    #[test]
    fn empty_charts_do_not_fabricate_data() {
        let (_root, state) = state();
        let charts = state
            .with_connection(|connection| {
                get_analytics_charts(connection, input(None, "2026-10-12", "2026-11-10"))
            })
            .expect("empty charts");
        assert_eq!(charts.total_seconds, 0);
        assert_eq!(charts.allocated_seconds, 0);
        assert_eq!(charts.unallocated_seconds, 0);
        assert!(charts.study_by_exam.is_empty());
        assert!(charts.daily_study.is_empty());
        assert_eq!(charts.question_outcomes.attempts, 0);
        assert_eq!(charts.question_outcomes.accuracy_percentage, None);
        assert!(charts.question_outcomes.insufficient_sample);
        assert!(charts.topic_accuracy.is_empty());
        assert!(charts.mock_trend.is_empty());
        assert_eq!(charts.syllabus_completion.len(), 4);
        assert!(
            charts
                .syllabus_completion
                .iter()
                .all(|entry| entry.topic_count > 0)
        );
    }

    #[test]
    fn study_charts_split_allocated_and_unallocated_time() {
        let (_root, state) = state();
        let charts = state
            .with_connection(|connection| {
                let now = at(0);
                connection
                    .execute(
                        "INSERT INTO study_sessions(session_date,started_at,ended_at,duration_seconds,source,notes,created_at,updated_at)
                         VALUES('2026-11-10','2026-11-10T09:00:00Z','2026-11-10T10:00:00Z',3600,'manual','',?1,?1)",
                        [now.to_rfc3339()],
                    )
                    .expect("study session");
                let session_id = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO session_exam_allocations(session_id,exam_id,allocated_seconds,created_at,updated_at)
                         VALUES(?1,1,1200,?2,?2)",
                        params![session_id, now.to_rfc3339()],
                    )
                    .expect("session allocation");
                get_analytics_charts(connection, input(None, "2026-11-10", "2026-11-10"))
            })
            .expect("study charts");
        assert_eq!(charts.total_seconds, 3600);
        assert_eq!(charts.allocated_seconds, 1200);
        assert_eq!(charts.unallocated_seconds, 2400);
        assert_eq!(charts.study_by_exam.len(), 1);
        assert_eq!(charts.study_by_exam[0].allocated_seconds, 1200);
        assert_eq!(charts.daily_study.len(), 1);
        assert_eq!(charts.daily_study[0].seconds, 3600);
        assert_eq!(charts.weekly_consistency.len(), 1);
        assert_eq!(charts.weekly_consistency[0].active_days, 1);
        assert_eq!(charts.weekly_consistency[0].eligible_days, 1);
    }

    #[test]
    fn mock_charts_preserve_negative_scores_and_exclude_incomplete_records() {
        let (_root, state) = state();
        let charts = state
            .with_connection(|connection| {
                let now = at(0).to_rfc3339();
                let section_id: i64 = connection
                    .query_row(
                        "SELECT id FROM exam_pattern_sections WHERE exam_id=1 ORDER BY id LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .expect("pattern section");
                let add_mock = |name: &str, date: &str, marks: Option<f64>, completed: bool| {
                    connection
                        .execute(
                            "INSERT INTO mock_tests(
                                exam_id, test_name, test_date, duration_minutes, max_marks,
                                marks_obtained, status, is_archived, created_at, updated_at
                             ) VALUES (1, ?1, ?2, 180, 100, ?3, 'planned', 0, ?4, ?4)",
                            params![name, date, marks, now],
                        )
                        .expect("mock fixture");
                    let id = connection.last_insert_rowid();
                    if completed {
                        connection
                            .execute(
                                "INSERT INTO mock_test_sections(
                                    exam_id, mock_test_id, section_id, question_count,
                                    correct, incorrect, unanswered, max_marks,
                                    marks_obtained, created_at, updated_at
                                 ) VALUES (1, ?1, ?2, 1, 0, 1, 0, 100, ?3, ?4, ?4)",
                                params![id, section_id, marks, now],
                            )
                            .expect("mock section");
                        connection
                            .execute("UPDATE mock_tests SET status='completed' WHERE id=?1", [id])
                            .expect("complete mock");
                    }
                    id
                };
                add_mock("Negative mock", "2026-11-01", Some(-12.5), true);
                add_mock("Positive mock", "2026-11-08", Some(42.5), true);
                add_mock("Archived mock", "2026-11-09", Some(90.0), true);
                connection
                    .execute(
                        "UPDATE mock_tests SET is_archived=1 WHERE test_name='Archived mock'",
                        [],
                    )
                    .expect("archive mock");
                add_mock("Planned mock", "2026-11-10", None, false);
                get_analytics_charts(connection, input(None, "2026-11-01", "2026-11-10"))
            })
            .expect("mock charts");
        assert_eq!(charts.mock_trend.len(), 2);
        assert_eq!(charts.mock_trend[0].percentage, -12.5);
        assert_eq!(charts.mock_trend[1].percentage, 42.5);
        assert_eq!(charts.mock_by_exam.len(), 1);
        let summary = &charts.mock_by_exam[0];
        assert_eq!(summary.completed_count, 2);
        assert_eq!(summary.average_percentage, Some(15.0));
        assert_eq!(summary.best_percentage, Some(42.5));
        assert_eq!(summary.last_percentage, Some(42.5));
    }

    #[test]
    fn chart_date_validation_rejects_invalid_ranges() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let reversed =
                    get_analytics_charts(connection, input(None, "2026-11-10", "2026-11-01"))
                        .unwrap_err();
                assert_eq!(reversed.field, Some("toDate"));
                let too_long =
                    get_analytics_charts(connection, input(None, "2025-01-01", "2026-11-10"))
                        .unwrap_err();
                assert_eq!(too_long.field, Some("toDate"));
                let invalid =
                    get_analytics_charts(connection, input(None, "not-a-date", "2026-11-10"))
                        .unwrap_err();
                assert_eq!(invalid.field, Some("fromDate"));
                Ok::<(), AppError>(())
            })
            .expect("date validation");
    }
}
