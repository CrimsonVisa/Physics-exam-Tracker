use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::mock::{self, V2MockAnalytics};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeAnalyticsInput {
    pub exam_id: i64,
    #[serde(default)]
    pub from_date: Option<String>,
    #[serde(default)]
    pub to_date: Option<String>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub section_id: Option<i64>,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeAnalytics {
    pub exam_id: i64,
    pub pyq_attempts: i64,
    pub distinct_questions: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub accuracy_percentage: Option<f64>,
    pub marks_obtained: f64,
    pub max_marks_attempted: f64,
    pub percentage: Option<f64>,
    pub time_spent_seconds: i64,
    pub average_seconds_per_question: Option<f64>,
    pub verified_questions: i64,
    pub verified_questions_in_scope: i64,
    pub total_questions_in_scope: i64,
    pub verification_coverage_percentage: Option<f64>,
    pub by_year: Vec<V2PracticeBreakdown>,
    pub by_section: Vec<V2PracticeBreakdown>,
    pub by_topic: Vec<V2PracticeTopicBreakdown>,
    pub pyq_trend: Vec<V2PracticeTrendPoint>,
    pub mocks: V2MockAnalytics,
    pub insufficient_pyq_data: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeBreakdown {
    pub key: String,
    pub label: String,
    pub attempts: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub accuracy_percentage: Option<f64>,
    pub percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeTopicBreakdown {
    pub node_id: Option<i64>,
    pub label: String,
    pub attempts: i64,
    pub correct: i64,
    pub incorrect: i64,
    pub unanswered: i64,
    pub accuracy_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PracticeTrendPoint {
    pub date: String,
    pub attempts: i64,
    pub accuracy_percentage: Option<f64>,
}

struct AttemptRow {
    attempt_date: String,
    year: i64,
    section_id: i64,
    section_code: String,
    section_title: String,
    node_id: Option<i64>,
    node_title: Option<String>,
    status: String,
    marks: f64,
    max_marks: f64,
    seconds: i64,
    verification_status: String,
    question_id: i64,
}

pub fn analytics(
    connection: &Connection,
    input: V2PracticeAnalyticsInput,
) -> Result<V2PracticeAnalytics, AppError> {
    let exam_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
            [input.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Practice analytics exam could not be checked", error)
        })?;
    if exam_exists == 0 {
        return Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        ));
    }
    if let Some(from) = input.from_date.as_deref()
        && !valid_date(from)
    {
        return Err(AppError::validation(
            "fromDate",
            "Enter a valid start date.",
        ));
    }
    if let Some(to) = input.to_date.as_deref()
        && !valid_date(to)
    {
        return Err(AppError::validation("toDate", "Enter a valid end date."));
    }
    if let (Some(from), Some(to)) = (input.from_date.as_deref(), input.to_date.as_deref())
        && from > to
    {
        return Err(AppError::validation(
            "toDate",
            "End date cannot be before the start date.",
        ));
    }
    let mut sql = String::from(
        "SELECT a.attempt_date, q.year, q.section_id, s.code, s.title, q.syllabus_node_id,
                n.title, a.status, COALESCE(a.marks_obtained,0), a.max_marks_at_attempt,
                a.time_spent_seconds, q.verification_status, q.id
         FROM pyq_attempts a
         JOIN pyq_questions q ON q.id = a.question_id
         JOIN exam_sections s ON s.id = q.section_id
         LEFT JOIN exam_syllabus_nodes n ON n.id = q.syllabus_node_id
         WHERE a.exam_id = ?1",
    );
    let mut values: Vec<rusqlite::types::Value> = vec![input.exam_id.into()];
    if let Some(from) = input.from_date.as_deref() {
        sql.push_str(" AND a.attempt_date >= ?");
        values.push(from.to_owned().into());
    }
    if let Some(to) = input.to_date.as_deref() {
        sql.push_str(" AND a.attempt_date <= ?");
        values.push(to.to_owned().into());
    }
    if let Some(year) = input.year {
        sql.push_str(" AND q.year = ?");
        values.push(year.into());
    }
    if let Some(section_id) = input.section_id {
        sql.push_str(" AND q.section_id = ?");
        values.push(section_id.into());
    }
    if let Some(node_id) = input.syllabus_node_id {
        sql.push_str(
            " AND (q.syllabus_node_id = ? OR EXISTS (
                SELECT 1 FROM pyq_question_topics qt
                WHERE qt.question_id = q.id AND qt.node_id = ?
            ))",
        );
        values.push(node_id.into());
        values.push(node_id.into());
    }
    sql.push_str(" ORDER BY a.attempt_date, a.id");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Practice analytics could not be prepared", error))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok(AttemptRow {
                attempt_date: row.get(0)?,
                year: row.get(1)?,
                section_id: row.get(2)?,
                section_code: row.get(3)?,
                section_title: row.get(4)?,
                node_id: row.get(5)?,
                node_title: row.get(6)?,
                status: row.get(7)?,
                marks: row.get(8)?,
                max_marks: row.get(9)?,
                seconds: row.get(10)?,
                verification_status: row.get(11)?,
                question_id: row.get(12)?,
            })
        })
        .map_err(|error| AppError::database("Practice analytics could not be read", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A practice analytics row is invalid", error))?;
    drop(statement);

    let mut question_ids = std::collections::HashSet::new();
    let mut verified_question_ids = std::collections::HashSet::new();
    let mut correct = 0_i64;
    let mut incorrect = 0_i64;
    let mut unanswered = 0_i64;
    let mut marks = 0.0;
    let mut max_marks = 0.0;
    let mut seconds = 0_i64;
    let mut by_year: HashMap<i64, BreakdownBuilder> = HashMap::new();
    let mut by_section: HashMap<i64, BreakdownBuilder> = HashMap::new();
    let mut by_topic: HashMap<Option<i64>, TopicBuilder> = HashMap::new();
    let mut trend: HashMap<String, (i64, i64, i64)> = HashMap::new();
    for row in &rows {
        question_ids.insert(row.question_id);
        if row.verification_status == "verified" {
            verified_question_ids.insert(row.question_id);
        }
        let unanswered_row = row.status == "skipped" || row.status == "unanswered";
        if unanswered_row {
            unanswered += 1;
        } else if row.status == "correct" {
            correct += 1;
        } else {
            incorrect += 1;
        }
        marks += row.marks;
        max_marks += row.max_marks;
        seconds += row.seconds;
        by_year
            .entry(row.year)
            .or_insert_with(|| BreakdownBuilder::new(row.year.to_string(), row.year.to_string()))
            .add(&row.status, row.marks, row.max_marks);
        by_section
            .entry(row.section_id)
            .or_insert_with(|| {
                BreakdownBuilder::new(row.section_code.clone(), row.section_title.clone())
            })
            .add(&row.status, row.marks, row.max_marks);
        by_topic
            .entry(row.node_id)
            .or_insert_with(|| {
                TopicBuilder::new(
                    row.node_id,
                    row.node_title
                        .clone()
                        .unwrap_or_else(|| "Unclassified".to_owned()),
                )
            })
            .add(&row.status);
        let point = trend.entry(row.attempt_date.clone()).or_insert((0, 0, 0));
        point.0 += 1;
        if row.status == "correct" {
            point.1 += 1;
        } else if row.status != "skipped" && row.status != "unanswered" {
            point.2 += 1;
        }
    }
    let (total_questions_in_scope, verified_questions_in_scope): (i64, i64) = {
        let mut question_sql = String::from(
            "SELECT COUNT(*), COALESCE(SUM(CASE WHEN q.verification_status = 'verified' THEN 1 ELSE 0 END), 0)
             FROM pyq_questions q WHERE q.exam_id = ?1 AND q.is_archived = 0",
        );
        let mut question_values: Vec<rusqlite::types::Value> = vec![input.exam_id.into()];
        if let Some(year) = input.year {
            question_sql.push_str(" AND q.year = ?");
            question_values.push(year.into());
        }
        if let Some(section_id) = input.section_id {
            question_sql.push_str(" AND q.section_id = ?");
            question_values.push(section_id.into());
        }
        if let Some(node_id) = input.syllabus_node_id {
            question_sql.push_str(
                " AND (q.syllabus_node_id = ? OR EXISTS (
                    SELECT 1 FROM pyq_question_topics qt
                    WHERE qt.question_id = q.id AND qt.node_id = ?
                ))",
            );
            question_values.push(node_id.into());
            question_values.push(node_id.into());
        }
        connection
            .query_row(
                &question_sql,
                rusqlite::params_from_iter(question_values),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| AppError::database("PYQ scope count could not be read", error))?
    };
    let verified_questions = verified_question_ids.len() as i64;
    let mut by_year = by_year
        .into_values()
        .map(BreakdownBuilder::finish)
        .collect::<Vec<_>>();
    let mut by_section = by_section
        .into_values()
        .map(BreakdownBuilder::finish)
        .collect::<Vec<_>>();
    let mut by_topic = by_topic
        .into_values()
        .map(TopicBuilder::finish)
        .collect::<Vec<_>>();
    by_year.sort_by(|left, right| right.key.cmp(&left.key));
    by_section.sort_by(|left, right| left.label.cmp(&right.label));
    by_topic.sort_by(|left, right| {
        right
            .attempts
            .cmp(&left.attempts)
            .then(left.label.cmp(&right.label))
    });
    let mut pyq_trend = trend
        .into_iter()
        .map(
            |(date, (attempts, correct, incorrect))| V2PracticeTrendPoint {
                date,
                attempts,
                accuracy_percentage: percentage(correct, correct + incorrect),
            },
        )
        .collect::<Vec<_>>();
    pyq_trend.sort_by(|left, right| left.date.cmp(&right.date));
    let mock_analytics = mock::analytics(connection, input.exam_id)?;
    let attempt_count = rows.len() as i64;
    Ok(V2PracticeAnalytics {
        exam_id: input.exam_id,
        pyq_attempts: attempt_count,
        distinct_questions: question_ids.len() as i64,
        correct,
        incorrect,
        unanswered,
        accuracy_percentage: percentage(correct, correct + incorrect),
        marks_obtained: round3(marks),
        max_marks_attempted: round3(max_marks),
        percentage: percentage_f(marks, max_marks),
        time_spent_seconds: seconds,
        average_seconds_per_question: if attempt_count > 0 {
            Some(round3(seconds as f64 / attempt_count as f64))
        } else {
            None
        },
        verified_questions,
        verified_questions_in_scope,
        total_questions_in_scope,
        verification_coverage_percentage: percentage(
            verified_questions_in_scope,
            total_questions_in_scope,
        ),
        by_year,
        by_section,
        by_topic,
        pyq_trend,
        mocks: mock_analytics,
        insufficient_pyq_data: attempt_count < 2,
    })
}

fn valid_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

fn percentage(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some(round3(numerator as f64 * 100.0 / denominator as f64))
    }
}

fn percentage_f(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator <= 0.0 {
        None
    } else {
        Some(round3(numerator * 100.0 / denominator))
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

struct BreakdownBuilder {
    key: String,
    label: String,
    attempts: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
    marks: f64,
    max_marks: f64,
}

impl BreakdownBuilder {
    fn new(key: String, label: String) -> Self {
        Self {
            key,
            label,
            attempts: 0,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            marks: 0.0,
            max_marks: 0.0,
        }
    }

    fn add(&mut self, status: &str, marks: f64, max_marks: f64) {
        self.attempts += 1;
        if status == "skipped" || status == "unanswered" {
            self.unanswered += 1;
        } else if status == "correct" {
            self.correct += 1;
        } else {
            self.incorrect += 1;
        }
        self.marks += marks;
        self.max_marks += max_marks;
    }

    fn finish(self) -> V2PracticeBreakdown {
        V2PracticeBreakdown {
            key: self.key,
            label: self.label,
            attempts: self.attempts,
            correct: self.correct,
            incorrect: self.incorrect,
            unanswered: self.unanswered,
            accuracy_percentage: percentage(self.correct, self.correct + self.incorrect),
            percentage: percentage_f(self.marks, self.max_marks),
        }
    }
}

struct TopicBuilder {
    node_id: Option<i64>,
    label: String,
    attempts: i64,
    correct: i64,
    incorrect: i64,
    unanswered: i64,
}

impl TopicBuilder {
    fn new(node_id: Option<i64>, label: String) -> Self {
        Self {
            node_id,
            label,
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

    fn finish(self) -> V2PracticeTopicBreakdown {
        V2PracticeTopicBreakdown {
            node_id: self.node_id,
            label: self.label,
            attempts: self.attempts,
            correct: self.correct,
            incorrect: self.incorrect,
            unanswered: self.unanswered,
            accuracy_percentage: percentage(self.correct, self.correct + self.incorrect),
        }
    }
}
