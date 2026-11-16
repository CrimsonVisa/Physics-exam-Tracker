use chrono::Days;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsPoint {
    pub label: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsSubject {
    pub exam_id: i64,
    pub exam_code: String,
    pub subject_title: String,
    pub allocated_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsTopic {
    pub exam_id: i64,
    pub exam_code: String,
    pub subject_title: String,
    pub node_id: i64,
    pub node_title: String,
    pub allocated_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MasteryBucket {
    pub label: String,
    pub concept_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Analytics {
    pub exam_id: Option<i64>,
    pub range_from: String,
    pub range_to: String,
    pub total_seconds: i64,
    pub session_count: i64,
    pub allocated_seconds: i64,
    pub unallocated_seconds: i64,
    pub topic_allocated_seconds: i64,
    pub topic_unallocated_seconds: i64,
    pub active_days: i64,
    pub consistency_percent: f64,
    pub daily: Vec<V2AnalyticsPoint>,
    pub weekly: Vec<V2AnalyticsPoint>,
    pub subjects: Vec<V2AnalyticsSubject>,
    pub topics: Vec<V2AnalyticsTopic>,
    pub mapped_concept_count: i64,
    pub evidenced_concept_count: i64,
    pub average_mastery: Option<f64>,
    pub mastery_buckets: Vec<V2MasteryBucket>,
    pub active_revision_schedules: i64,
    pub due_revision_count: i64,
    pub overdue_revision_count: i64,
    pub revision_completed_last_30_days: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AnalyticsInput {
    #[serde(default)]
    pub exam_id: Option<i64>,
    pub today: String,
    #[serde(default = "default_window_days")]
    pub window_days: i64,
}

fn default_window_days() -> i64 {
    30
}

pub fn get_analytics(
    connection: &Connection,
    input: V2AnalyticsInput,
) -> Result<V2Analytics, AppError> {
    let today = date_only::parse_date_only(&input.today).ok_or_else(|| {
        AppError::validation("today", "Enter a real calendar date in YYYY-MM-DD format.")
    })?;
    if !(7..=365).contains(&input.window_days) {
        return Err(AppError::validation(
            "windowDays",
            "Analytics window must be between 7 and 365 days.",
        ));
    }
    if let Some(exam_id) = input.exam_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Examination could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "examId",
                "The selected examination does not exist.",
            ));
        }
    }
    let from = today
        .checked_sub_days(Days::new((input.window_days - 1) as u64))
        .ok_or_else(|| AppError::validation("windowDays", "The analytics range is invalid."))?;
    let from_text = date_only::format_date_only(from);
    let to_text = date_only::format_date_only(today);

    let (total_seconds, session_count, allocated_seconds, daily, weekly) = if let Some(exam_id) =
        input.exam_id
    {
        let (total, count) = connection
            .query_row(
                "SELECT COALESCE(SUM(a.credit_seconds), 0), COUNT(DISTINCT a.session_id)
                     FROM session_exam_associations a
                     JOIN study_sessions s ON s.id = a.session_id
                     WHERE a.exam_id = ?1 AND s.session_date BETWEEN ?2 AND ?3",
                params![exam_id, from_text, to_text],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(|error| AppError::database("Exam study totals could not be read", error))?;
        let daily = daily_points(
            connection,
            "SELECT s.session_date, SUM(a.credit_seconds), COUNT(DISTINCT s.id)
                 FROM session_exam_associations a
                 JOIN study_sessions s ON s.id = a.session_id
                 WHERE a.exam_id = ?1 AND s.session_date BETWEEN ?2 AND ?3
                 GROUP BY s.session_date ORDER BY s.session_date",
            params![exam_id, from_text, to_text],
        )?;
        let weekly = weekly_points(
            connection,
            "SELECT strftime('%Y-W%W', s.session_date),
                        SUM(a.credit_seconds), COUNT(DISTINCT s.id)
                 FROM session_exam_associations a
                 JOIN study_sessions s ON s.id = a.session_id
                 WHERE a.exam_id = ?1 AND s.session_date BETWEEN ?2 AND ?3
                 GROUP BY strftime('%Y-W%W', s.session_date)
                 ORDER BY strftime('%Y-W%W', s.session_date)",
            params![exam_id, from_text, to_text],
        )?;
        (total, count, total, daily, weekly)
    } else {
        let (total, count) = connection
            .query_row(
                "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
                     FROM study_sessions WHERE session_date BETWEEN ?1 AND ?2",
                params![from_text, to_text],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(|error| AppError::database("Study totals could not be read", error))?;
        let allocated: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(MIN(s.duration_seconds, credits.credited_seconds)), 0)
                     FROM study_sessions s
                     JOIN (
                         SELECT session_id, SUM(credit_seconds) AS credited_seconds
                         FROM session_exam_associations
                         GROUP BY session_id
                     ) credits ON credits.session_id = s.id
                     WHERE s.session_date BETWEEN ?1 AND ?2",
                params![from_text, to_text],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Allocated study time could not be read", error))?;
        let daily = daily_points(
            connection,
            "SELECT session_date, SUM(duration_seconds), COUNT(*)
                 FROM study_sessions WHERE session_date BETWEEN ?1 AND ?2
                 GROUP BY session_date ORDER BY session_date",
            params![from_text, to_text],
        )?;
        let weekly = weekly_points(
            connection,
            "SELECT strftime('%Y-W%W', session_date), SUM(duration_seconds), COUNT(*)
                 FROM study_sessions WHERE session_date BETWEEN ?1 AND ?2
                 GROUP BY strftime('%Y-W%W', session_date)
                 ORDER BY strftime('%Y-W%W', session_date)",
            params![from_text, to_text],
        )?;
        (total, count, allocated, daily, weekly)
    };

    let active_days = daily.iter().filter(|point| point.seconds > 0).count() as i64;
    let consistency = if input.window_days > 0 {
        (active_days as f64 / input.window_days as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    let (mapped_concept_count, evidenced_concept_count, average_mastery) =
        if let Some(exam_id) = input.exam_id {
            connection
                .query_row(
                    "SELECT COUNT(DISTINCT nc.concept_id),
                            COUNT(DISTINCT CASE WHEN cm.evidence_count > 0 THEN nc.concept_id END),
                            AVG(CASE WHEN cm.evidence_count > 0 THEN cm.mastery_score END)
                     FROM exam_node_concepts nc
                     JOIN exam_syllabus_nodes n ON n.exam_id = nc.exam_id AND n.id = nc.node_id
                     JOIN concepts c ON c.id = nc.concept_id AND c.is_archived = 0
                     LEFT JOIN concept_mastery cm ON cm.concept_id = nc.concept_id
                     WHERE nc.exam_id = ?1 AND n.is_archived = 0
                       AND nc.status = 'approved'",
                    [exam_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get(2)?)),
                )
                .map_err(|error| AppError::database("Concept coverage could not be read", error))?
        } else {
            connection
                .query_row(
                    "SELECT
                        (SELECT COUNT(DISTINCT nc.concept_id)
                         FROM exam_node_concepts nc
                         JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
                         WHERE nc.status = 'approved'),
                        COUNT(CASE WHEN cm.evidence_count > 0 THEN 1 END),
                        AVG(CASE WHEN cm.evidence_count > 0 THEN cm.mastery_score END)
                     FROM concepts c
                     LEFT JOIN concept_mastery cm ON cm.concept_id = c.id
                     WHERE c.is_archived = 0",
                    [],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get(2)?)),
                )
                .map_err(|error| AppError::database("Concept coverage could not be read", error))?
        };

    let mastery_buckets = mastery_distribution(connection, input.exam_id)?;
    let (active_revision_schedules, due_revision_count, overdue_revision_count) =
        revision_counts(connection, input.exam_id, &to_text)?;
    let revision_completed_last_30_days = connection
        .query_row(
            "SELECT COUNT(*) FROM revision_history
             WHERE revision_date >= ?1 AND revision_date <= ?2
               AND (?3 IS NULL OR trigger_exam_id = ?3)",
            params![from_text, to_text, input.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Revision completion count could not be read", error)
        })?;
    let topic_allocated_seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(a.allocated_seconds), 0)
             FROM session_topic_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             WHERE s.session_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)",
            params![from_text, to_text, input.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Topic-allocated study time could not be read", error)
        })?;
    let subjects = subject_breakdown(connection, input.exam_id, &from_text, &to_text)?;
    let topics = topic_breakdown(connection, input.exam_id, &from_text, &to_text)?;

    Ok(V2Analytics {
        exam_id: input.exam_id,
        range_from: from_text,
        range_to: to_text,
        total_seconds,
        session_count,
        allocated_seconds,
        unallocated_seconds: (total_seconds - allocated_seconds).max(0),
        topic_allocated_seconds,
        topic_unallocated_seconds: (total_seconds - topic_allocated_seconds).max(0),
        active_days,
        consistency_percent: (consistency * 10.0).round() / 10.0,
        daily,
        weekly,
        subjects,
        topics,
        mapped_concept_count,
        evidenced_concept_count,
        average_mastery,
        mastery_buckets,
        active_revision_schedules,
        due_revision_count,
        overdue_revision_count,
        revision_completed_last_30_days,
    })
}

fn subject_breakdown(
    connection: &Connection,
    exam_id: Option<i64>,
    from_text: &str,
    to_text: &str,
) -> Result<Vec<V2AnalyticsSubject>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, COALESCE(parent.title, ''),
                    SUM(a.allocated_seconds), COUNT(DISTINCT a.session_id)
             FROM session_topic_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
               ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             WHERE s.session_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             GROUP BY a.exam_id, e.code, parent.title, e.display_order
             ORDER BY SUM(a.allocated_seconds) DESC, e.display_order, parent.title",
        )
        .map_err(|error| AppError::database("Subject analytics could not be prepared", error))?;
    let rows = statement
        .query_map(params![from_text, to_text, exam_id], |row| {
            Ok(V2AnalyticsSubject {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                subject_title: row.get(2)?,
                allocated_seconds: row.get(3)?,
                session_count: row.get(4)?,
            })
        })
        .map_err(|error| AppError::database("Subject analytics could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A subject analytics row is invalid", error))
}

fn topic_breakdown(
    connection: &Connection,
    exam_id: Option<i64>,
    from_text: &str,
    to_text: &str,
) -> Result<Vec<V2AnalyticsTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, COALESCE(parent.title, ''), n.id, n.title,
                    SUM(a.allocated_seconds), COUNT(DISTINCT a.session_id)
             FROM session_topic_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
               ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             WHERE s.session_date BETWEEN ?1 AND ?2
               AND (?3 IS NULL OR a.exam_id = ?3)
             GROUP BY a.exam_id, e.code, parent.title, n.id, n.title,
                      e.display_order, n.display_order
             ORDER BY SUM(a.allocated_seconds) DESC, e.display_order,
                      parent.title, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("Topic analytics could not be prepared", error))?;
    let rows = statement
        .query_map(params![from_text, to_text, exam_id], |row| {
            Ok(V2AnalyticsTopic {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                subject_title: row.get(2)?,
                node_id: row.get(3)?,
                node_title: row.get(4)?,
                allocated_seconds: row.get(5)?,
                session_count: row.get(6)?,
            })
        })
        .map_err(|error| AppError::database("Topic analytics could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A topic analytics row is invalid", error))
}

fn daily_points(
    connection: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<V2AnalyticsPoint>, AppError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Daily analytics could not be prepared", error))?;
    let rows = statement
        .query_map(params, |row| {
            Ok(V2AnalyticsPoint {
                label: row.get(0)?,
                seconds: row.get(1)?,
                session_count: row.get(2)?,
            })
        })
        .map_err(|error| AppError::database("Daily analytics could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A daily analytics row is invalid", error))
}

fn weekly_points(
    connection: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<V2AnalyticsPoint>, AppError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Weekly analytics could not be prepared", error))?;
    let rows = statement
        .query_map(params, |row| {
            Ok(V2AnalyticsPoint {
                label: row.get(0)?,
                seconds: row.get(1)?,
                session_count: row.get(2)?,
            })
        })
        .map_err(|error| AppError::database("Weekly analytics could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A weekly analytics row is invalid", error))
}

fn mastery_distribution(
    connection: &Connection,
    exam_id: Option<i64>,
) -> Result<Vec<V2MasteryBucket>, AppError> {
    let mut buckets = [0_i64; 5];
    let sql = if exam_id.is_some() {
        "SELECT cm.mastery_score FROM concept_mastery cm
         JOIN exam_node_concepts nc ON nc.concept_id = cm.concept_id AND nc.exam_id = ?1
         WHERE cm.evidence_count > 0 AND nc.status = 'approved'
         GROUP BY cm.concept_id"
    } else {
        "SELECT mastery_score FROM concept_mastery WHERE evidence_count > 0"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Mastery distribution could not be prepared", error))?;
    let mut rows = if let Some(exam_id) = exam_id {
        statement.query(params![exam_id])
    } else {
        statement.query([])
    }
    .map_err(|error| AppError::database("Mastery distribution could not be read", error))?;
    while let Some(row) = rows
        .next()
        .map_err(|error| AppError::database("A mastery distribution row is invalid", error))?
    {
        let score: f64 = row
            .get(0)
            .map_err(|error| AppError::database("A mastery score is invalid", error))?;
        let index = ((score / 20.0).floor() as usize).min(4);
        buckets[index] += 1;
    }
    Ok(vec![
        V2MasteryBucket {
            label: "0–20".to_owned(),
            concept_count: buckets[0],
        },
        V2MasteryBucket {
            label: "20–40".to_owned(),
            concept_count: buckets[1],
        },
        V2MasteryBucket {
            label: "40–60".to_owned(),
            concept_count: buckets[2],
        },
        V2MasteryBucket {
            label: "60–80".to_owned(),
            concept_count: buckets[3],
        },
        V2MasteryBucket {
            label: "80–100".to_owned(),
            concept_count: buckets[4],
        },
    ])
}

fn revision_counts(
    connection: &Connection,
    exam_id: Option<i64>,
    today: &str,
) -> Result<(i64, i64, i64), AppError> {
    let sql = if exam_id.is_some() {
        "SELECT COUNT(*),
                COUNT(CASE WHEN r.next_revision_date = ?2 THEN 1 END),
                COUNT(CASE WHEN r.next_revision_date < ?2 THEN 1 END)
         FROM revision_schedules r
         JOIN exam_node_concepts nc ON nc.concept_id = r.concept_id AND nc.exam_id = ?1
          AND nc.status = 'approved'
         WHERE r.status = 'active'"
    } else {
        "SELECT COUNT(*),
                COUNT(CASE WHEN next_revision_date = ?1 THEN 1 END),
                COUNT(CASE WHEN next_revision_date < ?1 THEN 1 END)
         FROM revision_schedules r
         WHERE r.status = 'active'
           AND EXISTS (
             SELECT 1 FROM exam_node_concepts nc
             JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
             WHERE nc.concept_id = r.concept_id AND nc.status = 'approved'
           )"
    };
    if let Some(exam_id) = exam_id {
        connection
            .query_row(sql, params![exam_id, today], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|error| AppError::database("Revision analytics could not be read", error))
    } else {
        connection
            .query_row(sql, params![today], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|error| AppError::database("Revision analytics could not be read", error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;

    #[test]
    fn two_hour_shared_session_counts_once_overall_and_fully_for_each_exam() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        state
            .with_connection(|connection| {
                crate::v2::study::create_manual_session(
                    connection,
                    crate::v2::study::V2CreateManualSessionInput {
                        session_date: "2026-11-04".to_owned(),
                        duration_seconds: 7_200,
                        notes: "Shared Classical Mechanics session".to_owned(),
                        primary_exam_id: None,
                        exam_ids: vec![1, 2, 4],
                    },
                    chrono::Utc::now(),
                )
                .map(|_| ())
            })
            .expect("shared session");

        let combined = state
            .with_connection(|connection| {
                get_analytics(
                    connection,
                    V2AnalyticsInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                        window_days: 30,
                    },
                )
            })
            .expect("combined analytics");
        assert_eq!(combined.total_seconds, 7_200);
        assert_eq!(combined.session_count, 1);
        assert_eq!(combined.allocated_seconds, 7_200);
        assert_eq!(combined.unallocated_seconds, 0);
        assert_eq!(combined.daily.len(), 1);
        assert_eq!(combined.daily[0].seconds, 7_200);
        assert_eq!(combined.weekly.len(), 1);
        assert_eq!(combined.weekly[0].seconds, 7_200);

        for exam_id in [1, 2, 4] {
            let exam = state
                .with_connection(|connection| {
                    get_analytics(
                        connection,
                        V2AnalyticsInput {
                            exam_id: Some(exam_id),
                            today: "2026-11-04".to_owned(),
                            window_days: 30,
                        },
                    )
                })
                .expect("exam analytics");
            assert_eq!(exam.total_seconds, 7_200, "exam {exam_id}");
            assert_eq!(exam.session_count, 1, "exam {exam_id}");
            assert_eq!(exam.allocated_seconds, 7_200, "exam {exam_id}");
            assert_eq!(exam.daily[0].seconds, 7_200, "exam {exam_id}");
            assert_eq!(exam.weekly[0].seconds, 7_200, "exam {exam_id}");
        }
    }

    #[test]
    fn combined_and_exam_analytics_use_recorded_values_once() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, primary_exam_id,
                            created_at, updated_at
                         ) VALUES ('2026-11-04', 3600, 'manual', 1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                let session_id = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO session_exam_associations (
                            session_id, exam_id, credit_seconds, created_at, updated_at
                         ) VALUES (?1, 1, 1800, '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [session_id],
                    )
                    .map_err(|error| AppError::database("test allocation", error))?;
                Ok::<_, AppError>(())
            })
            .expect("fixture");

        let combined = state
            .with_connection(|connection| {
                get_analytics(
                    connection,
                    V2AnalyticsInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                        window_days: 30,
                    },
                )
            })
            .expect("combined analytics");
        assert_eq!(combined.total_seconds, 3600);
        assert_eq!(combined.session_count, 1);
        assert_eq!(combined.allocated_seconds, 1800);
        assert_eq!(combined.unallocated_seconds, 1800);
        assert_eq!(combined.daily.len(), 1);
        assert_eq!(combined.daily[0].seconds, 3600);

        let exam = state
            .with_connection(|connection| {
                get_analytics(
                    connection,
                    V2AnalyticsInput {
                        exam_id: Some(1),
                        today: "2026-11-04".to_owned(),
                        window_days: 30,
                    },
                )
            })
            .expect("exam analytics");
        assert_eq!(exam.total_seconds, 1800);
        assert_eq!(exam.session_count, 1);
        assert!(exam.average_mastery.is_none());
    }

    #[test]
    fn analytics_breakdown_uses_topic_allocations_without_double_counting() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let (csir_topic, gate_topic, tifr_topic) = state
            .with_connection(|connection| {
                let topic = |exam_id: i64| {
                    connection
                        .query_row(
                            "SELECT id FROM exam_syllabus_nodes
                             WHERE exam_id = ?1 AND node_kind = 'topic'
                             ORDER BY id LIMIT 1",
                            [exam_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| AppError::database("test topic", error))
                };
                Ok((topic(1)?, topic(2)?, topic(3)?))
            })
            .expect("topics");

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, primary_exam_id,
                            created_at, updated_at
                         ) VALUES ('2026-11-04', 120, 'timer', 1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                let session_one = connection.last_insert_rowid();
                for (exam_id, seconds) in [(1, 60), (2, 30), (3, 30)] {
                    connection
                        .execute(
                            "INSERT INTO session_exam_associations (
                                session_id, exam_id, credit_seconds, created_at, updated_at
                             ) VALUES (?1, ?2, ?3, '2026-11-04T10:00:00Z',
                                       '2026-11-04T10:00:00Z')",
                            params![session_one, exam_id, seconds],
                        )
                        .map_err(|error| AppError::database("test exam allocation", error))?;
                }
                for (exam_id, node_id, seconds) in [
                    (1, csir_topic, 60),
                    (2, gate_topic, 30),
                    (3, tifr_topic, 30),
                ] {
                    connection
                        .execute(
                            "INSERT INTO session_topic_allocations (
                                session_id, exam_id, node_id, allocated_seconds,
                                created_at, updated_at
                             ) VALUES (?1, ?2, ?3, ?4, '2026-11-04T10:00:00Z',
                                       '2026-11-04T10:00:00Z')",
                            params![session_one, exam_id, node_id, seconds],
                        )
                        .map_err(|error| AppError::database("test topic allocation", error))?;
                }

                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, primary_exam_id,
                            created_at, updated_at
                         ) VALUES ('2026-11-04', 60, 'manual', 1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [],
                    )
                    .map_err(|error| AppError::database("legacy test session", error))?;
                let session_two = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO session_exam_associations (
                            session_id, exam_id, credit_seconds, created_at, updated_at
                         ) VALUES (?1, 1, 60, '2026-11-04T10:00:00Z',
                                   '2026-11-04T10:00:00Z')",
                        [session_two],
                    )
                    .map_err(|error| AppError::database("legacy exam allocation", error))?;
                Ok::<_, AppError>(())
            })
            .expect("analytics fixture");

        let combined = state
            .with_connection(|connection| {
                get_analytics(
                    connection,
                    V2AnalyticsInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                        window_days: 30,
                    },
                )
            })
            .expect("combined breakdown");
        assert_eq!(combined.total_seconds, 180);
        assert_eq!(combined.session_count, 2);
        assert_eq!(combined.allocated_seconds, 180);
        assert_eq!(combined.topic_allocated_seconds, 120);
        assert_eq!(combined.topic_unallocated_seconds, 60);
        assert_eq!(
            combined
                .subjects
                .iter()
                .map(|row| row.allocated_seconds)
                .sum::<i64>(),
            120
        );
        assert_eq!(
            combined
                .topics
                .iter()
                .map(|row| row.allocated_seconds)
                .sum::<i64>(),
            120
        );
        assert_eq!(combined.subjects.len(), 3);
        assert_eq!(combined.topics.len(), 3);
        assert_eq!(combined.daily[0].seconds, 180);

        let exam = state
            .with_connection(|connection| {
                get_analytics(
                    connection,
                    V2AnalyticsInput {
                        exam_id: Some(1),
                        today: "2026-11-04".to_owned(),
                        window_days: 30,
                    },
                )
            })
            .expect("exam breakdown");
        assert_eq!(exam.total_seconds, 120);
        assert_eq!(exam.topic_allocated_seconds, 60);
        assert_eq!(exam.topic_unallocated_seconds, 60);
        assert_eq!(exam.subjects.len(), 1);
        assert_eq!(exam.topics.len(), 1);
        assert_eq!(exam.topics[0].node_id, csir_topic);
    }
}
