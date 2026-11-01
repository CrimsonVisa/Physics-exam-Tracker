use std::collections::HashSet;

use chrono::{Datelike, Days, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{self, EVIDENCE_WINDOW_DAYS, MINIMUM_EVIDENCE_QUESTIONS, WEAK_ACCURACY_THRESHOLD},
    date_only,
    error::AppError,
    mock_tests, planner, pyq, revision, sessions, settings, syllabus,
};

pub const DASHBOARD_SECTION_KEYS: [&str; 9] = [
    "exam-countdown",
    "syllabus-progress",
    "study-totals",
    "today-study",
    "revision-status",
    "deadlines-blocks",
    "recent-pyq",
    "latest-mock",
    "insights",
];
pub const MAX_RECENT_PYQ: i64 = 5;
pub const MAX_DASHBOARD_DEADLINES: usize = 6;
pub const MAX_DASHBOARD_BLOCKS: usize = 6;
pub const MAX_INSIGHTS: usize = 5;
pub const MINIMUM_PART_INSIGHT_ATTEMPTS: i64 = 10;
pub const PART_ACCURACY_GAP: f64 = 10.0;
pub const TIMELINE_MAX_DAYS: i64 = 3_660;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSection {
    pub key: String,
    pub visible: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardLayout {
    pub sections: Vec<DashboardSection>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSectionInput {
    pub key: String,
    pub visible: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightMetric {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionableInsight {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub reason: String,
    pub metrics: Vec<InsightMetric>,
    pub target_view: String,
    pub target_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationDashboard {
    pub today: String,
    pub settings: settings::Settings,
    pub preparation_day_number: i64,
    pub preparation_elapsed_days: i64,
    pub days_until_exam: Option<i64>,
    pub syllabus: syllabus::SyllabusSummary,
    pub revision: revision::RevisionStats,
    pub daily_revision_target: i64,
    pub total_study_seconds: i64,
    pub today_study_seconds: i64,
    pub today_session_count: i64,
    pub week_start: String,
    pub week_end: String,
    pub week_study_seconds: i64,
    pub month_start: String,
    pub month_study_seconds: i64,
    pub daily_study_target_minutes: i64,
    pub today_target_percent: f64,
    pub upcoming_deadlines: Vec<planner::ChecklistTask>,
    pub upcoming_blocks: Vec<planner::PlannedBlock>,
    pub overdue_task_count: i64,
    pub recent_pyq: Vec<pyq::PyqAttempt>,
    pub latest_mock: Option<mock_tests::MockTest>,
    pub insights: Vec<ActionableInsight>,
    pub layout: DashboardLayout,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: String,
    pub date: String,
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub reference_id: Option<i64>,
    pub topic_code: Option<String>,
    pub duration_seconds: Option<i64>,
    pub calculated_percentage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationTimeline {
    pub from_date: String,
    pub to_date: String,
    pub today: String,
    pub events: Vec<TimelineEvent>,
    pub notes: Vec<String>,
}

pub fn load_layout(connection: &Connection) -> Result<DashboardLayout, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT section_key, visible, display_order
             FROM dashboard_sections
             ORDER BY display_order, section_key",
        )
        .map_err(|error| AppError::database("Dashboard layout could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(DashboardSection {
                key: row.get(0)?,
                visible: row.get::<_, i64>(1)? != 0,
                display_order: row.get(2)?,
            })
        })
        .map_err(|error| AppError::database("Dashboard layout could not be queried", error))?;
    let loaded = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Dashboard layout could not be read", error))?;

    let mut by_key = loaded
        .into_iter()
        .map(|section| (section.key.clone(), section))
        .collect::<std::collections::HashMap<_, _>>();
    let mut sections = DASHBOARD_SECTION_KEYS
        .iter()
        .enumerate()
        .map(|(index, key)| {
            by_key.remove(*key).unwrap_or(DashboardSection {
                key: (*key).to_owned(),
                visible: true,
                display_order: index as i64,
            })
        })
        .collect::<Vec<_>>();
    sections.sort_by(|left, right| {
        left.display_order
            .cmp(&right.display_order)
            .then_with(|| left.key.cmp(&right.key))
    });

    Ok(DashboardLayout { sections })
}

pub fn update_layout(
    connection: &mut Connection,
    sections: Vec<DashboardSectionInput>,
) -> Result<DashboardLayout, AppError> {
    validate_layout(&sections)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The dashboard-layout transaction could not be started",
            error,
        )
    })?;

    transaction
        .execute("DELETE FROM dashboard_sections", [])
        .map_err(|error| AppError::database("The dashboard layout could not be replaced", error))?;
    for section in &sections {
        transaction
            .execute(
                "INSERT INTO dashboard_sections (
                    section_key, visible, display_order, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )",
                params![
                    section.key,
                    i64::from(section.visible),
                    section.display_order
                ],
            )
            .map_err(|error| AppError::database("A dashboard section could not be saved", error))?;
    }

    let layout = load_layout(&transaction)?;
    transaction.commit().map_err(|error| {
        AppError::database("The dashboard layout could not be committed", error)
    })?;
    Ok(layout)
}

pub fn reset_layout(connection: &mut Connection) -> Result<DashboardLayout, AppError> {
    update_layout(connection, default_layout_input())
}

fn validate_layout(sections: &[DashboardSectionInput]) -> Result<(), AppError> {
    if sections.len() != DASHBOARD_SECTION_KEYS.len() {
        return Err(AppError::validation(
            "sections",
            "The dashboard layout must include every dashboard section.",
        ));
    }

    let known = DASHBOARD_SECTION_KEYS
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut keys = HashSet::new();
    let mut orders = HashSet::new();
    for section in sections {
        if !known.contains(section.key.as_str()) {
            return Err(AppError::validation(
                "sections",
                "The dashboard layout contains an unknown section.",
            ));
        }
        if !keys.insert(section.key.as_str()) {
            return Err(AppError::validation(
                "sections",
                "The dashboard layout cannot contain duplicate sections.",
            ));
        }
        if !(0..DASHBOARD_SECTION_KEYS.len() as i64).contains(&section.display_order) {
            return Err(AppError::validation(
                "sections",
                "Dashboard section order must be a unique zero-based position.",
            ));
        }
        if !orders.insert(section.display_order) {
            return Err(AppError::validation(
                "sections",
                "Dashboard section order values must be unique.",
            ));
        }
    }

    if keys.len() != DASHBOARD_SECTION_KEYS.len() {
        return Err(AppError::validation(
            "sections",
            "The dashboard layout must include every dashboard section.",
        ));
    }

    Ok(())
}

fn default_layout_input() -> Vec<DashboardSectionInput> {
    DASHBOARD_SECTION_KEYS
        .iter()
        .enumerate()
        .map(|(index, key)| DashboardSectionInput {
            key: (*key).to_owned(),
            visible: true,
            display_order: index as i64,
        })
        .collect()
}

pub fn preparation_dashboard(
    connection: &Connection,
    today: NaiveDate,
) -> Result<PreparationDashboard, AppError> {
    let settings = settings::load(connection)?;
    let daily_study_target_minutes = settings.daily_study_target_minutes;
    let preparation_start = parse_date(&settings.preparation_start_date, "preparationStartDate")?;
    let exam_date = parse_date(&settings.exam_date, "examDate")?;
    let today_text = date_only::format_date_only(today);
    let preparation_elapsed_days = if today >= preparation_start {
        (today - preparation_start).num_days()
    } else {
        0
    };
    let preparation_day_number = if today >= preparation_start {
        preparation_elapsed_days + 1
    } else {
        0
    };
    let days_until_exam = if exam_date >= today {
        Some((exam_date - today).num_days())
    } else {
        None
    };

    let syllabus_summary = syllabus::summary(connection)?;
    let revision_dashboard = revision::dashboard(connection, today)?;
    let week_start = monday_of(today);
    let week_start_text = date_only::format_date_only(week_start);
    let week_end_text = date_only::format_date_only(week_start + Days::new(6));
    let week_totals = sessions::totals(connection, &week_start_text, &today_text, today, None)?;
    let today_session_count = week_totals
        .daily
        .iter()
        .find(|day| day.date == today_text)
        .map_or(0, |day| day.session_count);
    let total_study_seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0) FROM study_sessions",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Total study time could not be calculated", error))?;
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .ok_or_else(|| AppError::internal("The current month could not be represented."))?;
    let month_start_text = date_only::format_date_only(month_start);
    let month_study_seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2",
            params![month_start_text, today_text],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Monthly study time could not be calculated", error))?;

    let agenda = planner::agenda(connection, today)?;
    let mut upcoming_deadlines = agenda.upcoming_deadlines;
    upcoming_deadlines.truncate(MAX_DASHBOARD_DEADLINES);
    let overdue_task_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM checklist_tasks
             WHERE status = 'open'
               AND COALESCE(deadline_date, scheduled_date) < ?1",
            [today_text.clone()],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Overdue tasks could not be counted", error))?;

    let block_end = date_only::format_date_only(today + Days::new(6));
    let mut upcoming_blocks = planner::list_blocks(connection, &today_text, &block_end)?
        .into_iter()
        .filter(|block| block.status == "planned")
        .collect::<Vec<_>>();
    upcoming_blocks.truncate(MAX_DASHBOARD_BLOCKS);

    let recent_pyq = pyq::recent(connection, MAX_RECENT_PYQ)?;
    let latest_mock = mock_tests::list(connection)?.into_iter().next();
    let target_seconds = settings.daily_study_target_minutes * 60;
    let today_target_percent = if target_seconds <= 0 {
        0.0
    } else {
        round_one(week_totals.today_seconds as f64 / target_seconds as f64 * 100.0)
    };
    let insights = actionable_insights(connection, today, &revision_dashboard.stats)?;
    let layout = load_layout(connection)?;

    Ok(PreparationDashboard {
        today: today_text,
        settings,
        preparation_day_number,
        preparation_elapsed_days,
        days_until_exam,
        syllabus: syllabus_summary,
        revision: revision_dashboard.stats,
        daily_revision_target: revision_dashboard.settings.daily_revision_target,
        total_study_seconds,
        today_study_seconds: week_totals.today_seconds,
        today_session_count,
        week_start: week_start_text,
        week_end: week_end_text,
        week_study_seconds: week_totals.range_seconds,
        month_start: month_start_text,
        month_study_seconds,
        daily_study_target_minutes,
        today_target_percent,
        upcoming_deadlines,
        upcoming_blocks,
        overdue_task_count,
        recent_pyq,
        latest_mock,
        insights,
        layout,
    })
}

pub fn timeline(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    today: NaiveDate,
) -> Result<PreparationTimeline, AppError> {
    let from = parse_date(from_date, "fromDate")?;
    let to = parse_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }
    if (to - from).num_days() + 1 > TIMELINE_MAX_DAYS {
        return Err(AppError::validation(
            "fromDate",
            format!("Choose a timeline range of {TIMELINE_MAX_DAYS} days or fewer."),
        ));
    }

    let from_text = date_only::format_date_only(from);
    let to_text = date_only::format_date_only(to);
    let today_text = date_only::format_date_only(today);
    let mut events = Vec::new();

    let settings = settings::load(connection)?;
    let exam_date = parse_date(&settings.exam_date, "examDate")?;
    if exam_date >= from && exam_date <= to {
        let status = if exam_date < today {
            "past"
        } else if exam_date == today {
            "today"
        } else {
            "upcoming"
        };
        events.push(TimelineEvent {
            id: "exam:target".to_owned(),
            date: settings.exam_date.clone(),
            kind: "exam".to_owned(),
            title: "Target exam date".to_owned(),
            subtitle: "Configured in Settings. Verify the official subject date.".to_owned(),
            status: status.to_owned(),
            reference_id: None,
            topic_code: None,
            duration_seconds: None,
            calculated_percentage: None,
        });
    }

    let tasks = planner::list_tasks(
        connection,
        planner::TaskFilter {
            status: Some("all".to_owned()),
            ..planner::TaskFilter::default()
        },
        today,
    )?;
    for task in tasks {
        let Some(deadline) = task.deadline_date.clone() else {
            continue;
        };
        if deadline < from_text || deadline > to_text {
            continue;
        }
        let topic = task
            .topic
            .as_ref()
            .map(|topic| format!("{} · {}", topic.code, topic.title));
        events.push(TimelineEvent {
            id: format!("deadline:{}", task.id),
            date: deadline.clone(),
            kind: "deadline".to_owned(),
            title: task.title.clone(),
            subtitle: format!(
                "{} deadline{}{}",
                task.priority,
                if task.status == "completed" {
                    " · completed"
                } else {
                    ""
                },
                topic.map_or_else(String::new, |value| format!(" · {value}"))
            ),
            status: task.status.clone(),
            reference_id: Some(task.id),
            topic_code: task.topic.map(|topic| topic.code),
            duration_seconds: None,
            calculated_percentage: None,
        });
    }

    let blocks = planner::list_blocks(connection, &from_text, &to_text)?;
    let session_ids = connection
        .prepare(
            "SELECT id
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2",
        )
        .map_err(|error| AppError::database("Timeline session ids could not be prepared", error))?
        .query_map(params![from_text, to_text], |row| row.get::<_, i64>(0))
        .map_err(|error| AppError::database("Timeline session ids could not be queried", error))?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Timeline session ids could not be read", error))?;

    for block in blocks {
        if block
            .linked_session_id
            .is_some_and(|session_id| session_ids.contains(&session_id))
        {
            continue;
        }
        let topic = block
            .topic
            .as_ref()
            .map(|topic| format!("{} · {}", topic.code, topic.title));
        events.push(TimelineEvent {
            id: format!("block:{}", block.id),
            date: block.calendar_date.clone(),
            kind: "planned_block".to_owned(),
            title: block.title.clone(),
            subtitle: format!(
                "{}–{}{}{}",
                block.start_time,
                block.end_time,
                if block.status == "planned" {
                    " · plan only, not study time"
                } else {
                    ""
                },
                topic.map_or_else(String::new, |value| format!(" · {value}"))
            ),
            status: block.status.clone(),
            reference_id: Some(block.id),
            topic_code: block.topic.map(|topic| topic.code),
            duration_seconds: None,
            calculated_percentage: None,
        });
    }

    for item in revision::calendar_items(connection, &from_text, &to_text)? {
        let status = if item.next_revision_date < today_text {
            "overdue"
        } else if item.next_revision_date == today_text {
            "due_today"
        } else {
            "scheduled"
        };
        events.push(TimelineEvent {
            id: format!("revision:{}", item.topic_id),
            date: item.next_revision_date.clone(),
            kind: "revision".to_owned(),
            title: format!("Revision · {}", item.topic_code),
            subtitle: format!(
                "{} · {} completed · {} mastery",
                item.topic_title,
                item.completed_revisions,
                format_percentage(item.mastery_estimate)
            ),
            status: status.to_owned(),
            reference_id: Some(item.topic_id),
            topic_code: Some(item.topic_code),
            duration_seconds: None,
            calculated_percentage: None,
        });
    }

    let mut session_statement = connection
        .prepare(
            "SELECT
                ss.id,
                ss.session_date,
                ss.duration_seconds,
                ss.notes,
                t.code,
                t.title
             FROM study_sessions ss
             LEFT JOIN syllabus_topics t ON t.id = ss.topic_id
             WHERE ss.session_date >= ?1 AND ss.session_date <= ?2
             ORDER BY ss.session_date, ss.id",
        )
        .map_err(|error| AppError::database("Timeline sessions could not be prepared", error))?;
    let session_rows = session_statement
        .query_map(params![from_text, to_text], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|error| AppError::database("Timeline sessions could not be queried", error))?;
    for row in session_rows {
        let (id, date, duration_seconds, notes, topic_code, topic_title) =
            row.map_err(|error| AppError::database("A timeline session could not be read", error))?;
        let title = topic_code.as_ref().map_or_else(
            || "Study session".to_owned(),
            |code| format!("{code} study session"),
        );
        events.push(TimelineEvent {
            id: format!("session:{id}"),
            date,
            kind: "study_session".to_owned(),
            title,
            subtitle: format!(
                "{}{}",
                topic_title.unwrap_or_else(|| "No syllabus topic linked".to_owned()),
                if notes.is_empty() {
                    String::new()
                } else {
                    format!(" · {notes}")
                }
            ),
            status: "completed".to_owned(),
            reference_id: Some(id),
            topic_code,
            duration_seconds: Some(duration_seconds),
            calculated_percentage: None,
        });
    }

    for test in mock_tests::list(connection)? {
        if test.test_date < from_text || test.test_date > to_text {
            continue;
        }
        events.push(TimelineEvent {
            id: format!("mock:{}", test.id),
            date: test.test_date.clone(),
            kind: "mock_test".to_owned(),
            title: test.test_name.clone(),
            subtitle: format!(
                "Calculated {}% · {:.2}/{:.2} marks · reported {}",
                format_percentage(test.computed_percentage),
                test.computed_marks,
                test.max_marks,
                test.marks_obtained
                    .map_or_else(|| "not recorded".to_owned(), |marks| format!("{marks:.2}"))
            ),
            status: "completed".to_owned(),
            reference_id: Some(test.id),
            topic_code: None,
            duration_seconds: None,
            calculated_percentage: Some(test.computed_percentage),
        });
    }

    events.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| kind_order(&left.kind).cmp(&kind_order(&right.kind)))
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(PreparationTimeline {
        from_date: from_text,
        to_date: to_text,
        today: today_text,
        events,
        notes: vec![
            "Timeline entries are read-only projections of saved records. Displaying the timeline never changes source data.".to_owned(),
            "A planned block linked to a study session is represented by the completed study session once both fall in the selected range, avoiding a duplicate for the same underlying activity.".to_owned(),
            "Revision entries are future schedule dates. Completed revision outcomes remain in Revision history.".to_owned(),
        ],
    })
}

fn actionable_insights(
    connection: &Connection,
    today: NaiveDate,
    revision_stats: &revision::RevisionStats,
) -> Result<Vec<ActionableInsight>, AppError> {
    let mut insights = Vec::new();
    let pyq = analytics::pyq(connection, today)?;

    if let Some(topic) = pyq.weak_topics.first() {
        insights.push(ActionableInsight {
            id: format!("weak-topic-{}", topic.topic_id),
            severity: "warning".to_owned(),
            title: format!("{} has low PYQ accuracy", topic.code),
            reason: format!(
                "This topic has evidence from {} distinct questions in the last {} days at {:.1}% accuracy. The weak-topic screen requires at least {} distinct questions and latest-outcome accuracy below {:.0}%.",
                topic.evidence_questions,
                EVIDENCE_WINDOW_DAYS,
                topic.evidence_accuracy.unwrap_or(0.0),
                MINIMUM_EVIDENCE_QUESTIONS,
                WEAK_ACCURACY_THRESHOLD
            ),
            metrics: vec![
                InsightMetric {
                    label: "Evidence accuracy".to_owned(),
                    value: format_percentage(topic.evidence_accuracy.unwrap_or(0.0)),
                },
                InsightMetric {
                    label: "Evidence questions".to_owned(),
                    value: topic.evidence_questions.to_string(),
                },
            ],
            target_view: "analytics:topics".to_owned(),
            target_label: "Open topic intelligence".to_owned(),
        });
    }

    if revision_stats.overdue_count > 0 {
        let oldest = connection
            .query_row(
                "SELECT MIN(r.next_revision_date)
                 FROM revision_schedules r
                 JOIN syllabus_topics t ON t.id = r.topic_id
                 WHERE r.status = 'active'
                   AND t.is_archived = 0
                   AND r.next_revision_date < ?1",
                [date_only::format_date_only(today)],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(|error| {
                AppError::database("The oldest overdue revision could not be read", error)
            })?;
        insights.push(ActionableInsight {
            id: "overdue-revisions".to_owned(),
            severity: "warning".to_owned(),
            title: format!(
                "{} revision{} overdue",
                revision_stats.overdue_count,
                if revision_stats.overdue_count == 1 { "" } else { "s" }
            ),
            reason: oldest.map_or_else(
                || "Open the revision queue and record an outcome or reschedule explicitly.".to_owned(),
                |date| {
                    format!(
                        "The oldest overdue revision is scheduled for {}. Open the revision queue to record an outcome or reschedule explicitly.",
                        date
                    )
                },
            ),
            metrics: vec![InsightMetric {
                label: "Overdue revisions".to_owned(),
                value: revision_stats.overdue_count.to_string(),
            }],
            target_view: "revision".to_owned(),
            target_label: "Open Revision".to_owned(),
        });
    }

    let agenda = planner::agenda(connection, today)?;
    if let Some(task) = agenda.upcoming_deadlines.first()
        && let Some(deadline) = &task.deadline_date
    {
        let deadline_date = parse_date(deadline, "deadlineDate")?;
        let days = (deadline_date - today).num_days();
        insights.push(ActionableInsight {
            id: format!("deadline-{}", task.id),
            severity: if days <= 2 { "warning" } else { "info" }.to_owned(),
            title: format!("Deadline approaching: {}", task.title),
            reason: format!(
                "The saved deadline is {} ({} day{} from today). This is a checklist deadline, not a study-time completion.",
                deadline,
                days,
                if days == 1 { "" } else { "s" }
            ),
            metrics: vec![
                InsightMetric {
                    label: "Deadline".to_owned(),
                    value: deadline.clone(),
                },
                InsightMetric {
                    label: "Days remaining".to_owned(),
                    value: days.to_string(),
                },
            ],
            target_view: "checklist".to_owned(),
            target_label: "Open Checklist".to_owned(),
        });
    }

    let last_seven_start = today.checked_sub_days(Days::new(7)).ok_or_else(|| {
        AppError::internal("The study comparison window is outside the calendar.")
    })?;
    let last_seven_end = today.checked_sub_days(Days::new(1)).ok_or_else(|| {
        AppError::internal("The study comparison window is outside the calendar.")
    })?;
    let previous_seven_start = today.checked_sub_days(Days::new(14)).ok_or_else(|| {
        AppError::internal("The study comparison window is outside the calendar.")
    })?;
    let previous_seven_end = today.checked_sub_days(Days::new(8)).ok_or_else(|| {
        AppError::internal("The study comparison window is outside the calendar.")
    })?;
    let current_seven = sum_sessions(connection, last_seven_start, last_seven_end)?;
    let previous_seven = sum_sessions(connection, previous_seven_start, previous_seven_end)?;
    if previous_seven > 0 && current_seven < previous_seven {
        let change =
            round_one((current_seven - previous_seven) as f64 / previous_seven as f64 * 100.0);
        insights.push(ActionableInsight {
            id: "study-time-decline".to_owned(),
            severity: "info".to_owned(),
            title: "Recorded study time is lower than the prior week".to_owned(),
            reason: format!(
                "The last seven completed days contain {} of study, compared with {} in the preceding seven days ({:.1}%). This is a saved-total comparison, not a claim about cause.",
                format_duration(current_seven),
                format_duration(previous_seven),
                change
            ),
            metrics: vec![
                InsightMetric {
                    label: "Last 7 completed days".to_owned(),
                    value: format_duration(current_seven),
                },
                InsightMetric {
                    label: "Prior 7 completed days".to_owned(),
                    value: format_duration(previous_seven),
                },
            ],
            target_view: "analytics:study".to_owned(),
            target_label: "Open study analytics".to_owned(),
        });
    }

    let part_a = pyq.by_part.iter().find(|part| part.part == "A");
    let other_correct = pyq
        .by_part
        .iter()
        .filter(|part| part.part != "A")
        .map(|part| part.correct)
        .sum::<i64>();
    let other_incorrect = pyq
        .by_part
        .iter()
        .filter(|part| part.part != "A")
        .map(|part| part.incorrect)
        .sum::<i64>();
    let other_attempts = other_correct + other_incorrect;
    if let Some(part_a) = part_a
        && part_a.attempts >= MINIMUM_PART_INSIGHT_ATTEMPTS
        && other_attempts >= MINIMUM_PART_INSIGHT_ATTEMPTS
        && let Some(part_a_accuracy) = part_a.accuracy
    {
        let other_accuracy = other_correct as f64 / other_attempts as f64 * 100.0;
        if part_a_accuracy + PART_ACCURACY_GAP < other_accuracy {
            insights.push(ActionableInsight {
                id: "pyq-part-a-gap".to_owned(),
                severity: "info".to_owned(),
                title: "PYQ Part A accuracy trails the other parts".to_owned(),
                reason: format!(
                    "Part A accuracy is {:.1}% across {} scored attempts, compared with {:.1}% across {} scored attempts in Parts B and C. This describes saved PYQ outcomes only.",
                    part_a_accuracy,
                    part_a.attempts,
                    other_accuracy,
                    other_attempts
                ),
                metrics: vec![
                    InsightMetric {
                        label: "Part A accuracy".to_owned(),
                        value: format_percentage(part_a_accuracy),
                    },
                    InsightMetric {
                        label: "Parts B/C accuracy".to_owned(),
                        value: format_percentage(other_accuracy),
                    },
                ],
                target_view: "analytics:performance".to_owned(),
                target_label: "Open exam performance".to_owned(),
            });
        }
    }

    insights.truncate(MAX_INSIGHTS);
    Ok(insights)
}

fn sum_sessions(connection: &Connection, from: NaiveDate, to: NaiveDate) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2",
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("A study-time comparison could not be calculated", error)
        })
}

fn kind_order(kind: &str) -> i64 {
    match kind {
        "exam" => 0,
        "deadline" => 1,
        "revision" => 2,
        "planned_block" => 3,
        "study_session" => 4,
        "mock_test" => 5,
        _ => 6,
    }
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Days::new(date.weekday().num_days_from_monday() as u64)
}

fn format_percentage(value: f64) -> String {
    format!("{value:.1}%")
}

fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if hours > 0 && minutes > 0 {
        format!("{hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h")
    } else {
        format!("{minutes}m")
    }
}

fn round_one(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;
    use crate::{db, mock_tests, planner, pyq, revision, sessions, settings, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection(today: NaiveDate) -> Connection {
        let mut connection = db::test_connection_with_defaults(today);
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

    fn set_preparation(connection: &mut Connection, today: NaiveDate, start: &str) {
        settings::update(
            connection,
            settings::SettingsInput {
                exam_date: "2026-12-19".to_owned(),
                preparation_start_date: start.to_owned(),
                daily_study_target_minutes: 60,
            },
            today,
        )
        .expect("settings should update");
    }

    fn add_session(connection: &mut Connection, topic_id: i64, day: &str, seconds: i64) {
        sessions::create_manual(
            connection,
            sessions::StudySessionInput {
                topic_id: Some(topic_id),
                session_date: day.to_owned(),
                duration_seconds: seconds,
                notes: String::new(),
            },
        )
        .expect("session should save");
    }

    fn pyq_attempt(connection: &mut Connection, topic_id: i64, status: &str, reference: &str) {
        pyq::create(
            connection,
            pyq::PyqInput {
                question_year: 2024,
                exam_session: String::new(),
                part: "B".to_owned(),
                topic_id,
                question_reference: reference.to_owned(),
                attempt_date: "2026-10-30".to_owned(),
                status: status.to_owned(),
                marks_obtained: None,
                max_marks: None,
                time_spent_seconds: 60,
                notes: String::new(),
                difficulty: None,
            },
        )
        .expect("PYQ should save");
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

    fn add_mock(connection: &mut Connection, name: &str, day: &str) {
        mock_tests::create(
            connection,
            mock_tests::MockTestInput {
                test_name: name.to_owned(),
                test_date: day.to_owned(),
                duration_minutes: 180,
                max_marks: 100.0,
                marks_obtained: None,
                part_a: mock_part(10, 6, 4),
                part_b: mock_part(20, 10, 10),
                part_c: mock_part(10, 5, 5),
                notes: String::new(),
                review_observations: String::new(),
            },
        )
        .expect("mock should save");
    }

    #[test]
    fn layout_defaults_update_reset_and_validate() {
        let mut connection = connection(date(2026, 10, 30));
        let defaults = load_layout(&connection).expect("default layout should load");
        assert_eq!(defaults.sections.len(), DASHBOARD_SECTION_KEYS.len());
        assert_eq!(defaults.sections[0].key, "exam-countdown");
        assert!(defaults.sections.iter().all(|section| section.visible));

        let mut custom = defaults
            .sections
            .iter()
            .map(|section| DashboardSectionInput {
                key: section.key.clone(),
                visible: section.visible,
                display_order: section.display_order,
            })
            .collect::<Vec<_>>();
        custom[1].visible = false;
        custom[0].display_order = 1;
        custom[1].display_order = 0;
        let saved = update_layout(&mut connection, custom).expect("layout should save");
        assert_eq!(saved.sections[0].key, "syllabus-progress");
        assert!(!saved.sections[0].visible);
        assert_eq!(saved.sections[1].key, "exam-countdown");

        let original = saved.clone();
        let invalid = vec![DashboardSectionInput {
            key: "exam-countdown".to_owned(),
            visible: true,
            display_order: 0,
        }];
        assert_eq!(
            update_layout(&mut connection, invalid)
                .expect_err("partial layout should fail")
                .field,
            Some("sections")
        );
        let mut duplicate = original
            .sections
            .iter()
            .map(|section| DashboardSectionInput {
                key: section.key.clone(),
                visible: section.visible,
                display_order: section.display_order,
            })
            .collect::<Vec<_>>();
        duplicate[1].display_order = duplicate[0].display_order;
        assert_eq!(
            update_layout(&mut connection, duplicate)
                .expect_err("duplicate order should fail")
                .field,
            Some("sections")
        );
        let mut unknown = original
            .sections
            .iter()
            .map(|section| DashboardSectionInput {
                key: section.key.clone(),
                visible: section.visible,
                display_order: section.display_order,
            })
            .collect::<Vec<_>>();
        unknown[0].key = "unknown-section".to_owned();
        assert_eq!(
            update_layout(&mut connection, unknown)
                .expect_err("unknown section should fail")
                .field,
            Some("sections")
        );
        let mut duplicate_key = original
            .sections
            .iter()
            .map(|section| DashboardSectionInput {
                key: section.key.clone(),
                visible: section.visible,
                display_order: section.display_order,
            })
            .collect::<Vec<_>>();
        duplicate_key[1].key = duplicate_key[0].key.clone();
        assert_eq!(
            update_layout(&mut connection, duplicate_key)
                .expect_err("duplicate section should fail")
                .field,
            Some("sections")
        );
        assert_eq!(
            load_layout(&connection).expect("layout should survive"),
            original
        );

        let reset = reset_layout(&mut connection).expect("layout should reset");
        assert_eq!(reset, defaults);
    }

    #[test]
    fn layout_persists_across_database_reopen() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join(db::DATABASE_FILE_NAME);
        let today = date(2026, 10, 30);

        {
            let mut connection = db::open_database(&path, today).expect("database should open");
            let mut layout = load_layout(&connection)
                .expect("layout should load")
                .sections
                .into_iter()
                .map(|section| DashboardSectionInput {
                    key: section.key,
                    visible: section.visible,
                    display_order: section.display_order,
                })
                .collect::<Vec<_>>();
            layout.sort_by_key(|left| std::cmp::Reverse(left.display_order));
            for (index, section) in layout.iter_mut().enumerate() {
                section.display_order = index as i64;
                section.visible = section.key != "latest-mock";
            }
            update_layout(&mut connection, layout).expect("layout should save");
        }

        let connection = db::open_database(&path, today).expect("database should reopen");
        let loaded = load_layout(&connection).expect("persisted layout should load");
        assert_eq!(loaded.sections[0].key, "insights");
        assert!(
            loaded
                .sections
                .iter()
                .find(|section| section.key == "latest-mock")
                .is_some_and(|section| !section.visible)
        );
    }

    #[test]
    fn preparation_dashboard_uses_saved_sources_and_separates_planned_from_completed() {
        let today = date(2026, 10, 30);
        let mut connection = connection(today);
        set_preparation(&mut connection, today, "2026-10-01");
        let topic = topic_id(&connection, "MMP-C01");
        add_session(&mut connection, topic, "2026-10-29", 1_800);
        add_session(&mut connection, topic, "2026-10-30", 3_600);
        add_mock(&mut connection, "Dashboard mock", "2026-10-29");
        for index in 0..6 {
            pyq_attempt(
                &mut connection,
                topic,
                "incorrect",
                &format!("DASH-{index}"),
            );
        }
        revision::schedule_topic(&mut connection, topic, "2026-10-29", today)
            .expect("overdue revision should save");
        let archived_topic = topic_id(&connection, "GA-01");
        revision::schedule_topic(&mut connection, archived_topic, "2020-01-01", today)
            .expect("archived overdue revision should save");
        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [archived_topic],
            )
            .expect("revision topic should archive");
        planner::create_task(
            &mut connection,
            planner::TaskInput {
                title: "Dashboard deadline".to_owned(),
                description: String::new(),
                scheduled_date: "2026-10-30".to_owned(),
                deadline_date: Some("2026-11-02".to_owned()),
                topic_id: Some(topic),
                priority: "High".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: None,
            },
            today,
        )
        .expect("task should save");
        planner::create_block(
            &mut connection,
            planner::PlannedBlockInput {
                title: "Dashboard plan".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-11-02".to_owned(),
                start_time: "10:00".to_owned(),
                end_time: "11:00".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("block should save");

        let dashboard = preparation_dashboard(&connection, today).expect("dashboard should load");
        assert_eq!(dashboard.preparation_day_number, 30);
        assert_eq!(dashboard.preparation_elapsed_days, 29);
        assert_eq!(dashboard.days_until_exam, Some(50));
        assert_eq!(dashboard.syllabus.total_topics, 181);
        assert_eq!(dashboard.total_study_seconds, 5_400);
        assert_eq!(dashboard.today_study_seconds, 3_600);
        assert_eq!(dashboard.today_session_count, 1);
        assert_eq!(dashboard.week_study_seconds, 5_400);
        assert_eq!(dashboard.month_study_seconds, 5_400);
        assert_eq!(dashboard.today_target_percent, 100.0);
        assert_eq!(dashboard.revision.overdue_count, 1);
        assert_eq!(dashboard.overdue_task_count, 0);
        assert_eq!(dashboard.upcoming_deadlines.len(), 1);
        assert_eq!(dashboard.upcoming_blocks.len(), 1);
        assert_eq!(dashboard.recent_pyq.len(), 5);
        assert_eq!(
            dashboard
                .latest_mock
                .as_ref()
                .map(|test| test.test_name.as_str()),
            Some("Dashboard mock")
        );
        let weak_insight = dashboard
            .insights
            .iter()
            .find(|insight| insight.id.starts_with("weak-topic-"))
            .expect("weak-topic insight should exist");
        assert_eq!(weak_insight.target_view, "analytics:topics");
        let revision_insight = dashboard
            .insights
            .iter()
            .find(|insight| insight.id == "overdue-revisions")
            .expect("overdue revision insight should exist");
        assert_eq!(revision_insight.target_view, "revision");
        assert!(
            revision_insight
                .reason
                .contains("The oldest overdue revision is scheduled for 2026-10-29.")
        );
        assert!(!revision_insight.reason.contains("2020-01-01"));
        assert_eq!(dashboard.layout.sections.len(), 9);
        assert_eq!(dashboard.today, "2026-10-30");
    }

    #[test]
    fn timeline_deduplicates_linked_blocks_and_respects_date_and_type_sources() {
        let today = date(2026, 10, 30);
        let mut connection = connection(today);
        let topic = topic_id(&connection, "MMP-C01");
        planner::create_task(
            &mut connection,
            planner::TaskInput {
                title: "Timeline deadline".to_owned(),
                description: String::new(),
                scheduled_date: "2026-10-30".to_owned(),
                deadline_date: Some("2026-11-02".to_owned()),
                topic_id: Some(topic),
                priority: "High".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: None,
            },
            today,
        )
        .expect("task should save");
        revision::schedule_topic(&mut connection, topic, "2026-11-01", today)
            .expect("revision should save");
        let session = sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-10-30".to_owned(),
                duration_seconds: 3_600,
                notes: "Timeline session".to_owned(),
            },
        )
        .expect("session should save");
        planner::create_block(
            &mut connection,
            planner::PlannedBlockInput {
                title: "Linked completed block".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-10-30".to_owned(),
                start_time: "09:00".to_owned(),
                end_time: "10:00".to_owned(),
                status: "completed".to_owned(),
                notes: String::new(),
                linked_session_id: Some(session.id),
            },
        )
        .expect("linked block should save");
        planner::create_block(
            &mut connection,
            planner::PlannedBlockInput {
                title: "Unlinked plan".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-10-30".to_owned(),
                start_time: "11:00".to_owned(),
                end_time: "12:00".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("unlinked block should save");
        add_mock(&mut connection, "Timeline mock", "2026-10-30");

        let timeline = super::timeline(&connection, "2026-10-25", "2026-12-31", today)
            .expect("timeline should load");
        assert!(timeline.events.iter().any(|event| event.kind == "exam"));
        assert!(timeline.events.iter().any(|event| event.kind == "deadline"));
        assert!(timeline.events.iter().any(|event| event.kind == "revision"));
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.kind == "mock_test")
        );
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.kind == "study_session")
        );
        assert!(
            timeline
                .events
                .iter()
                .any(|event| event.title == "Unlinked plan")
        );
        assert!(
            !timeline
                .events
                .iter()
                .any(|event| event.title == "Linked completed block")
        );
        assert_eq!(
            timeline
                .events
                .iter()
                .filter(|event| event.id == format!("session:{}", session.id))
                .count(),
            1
        );
        let mut ids = std::collections::HashSet::new();
        assert!(
            timeline
                .events
                .iter()
                .all(|event| ids.insert(event.id.clone()))
        );
        assert_eq!(
            timeline
                .events
                .iter()
                .filter(|event| event.kind == "deadline")
                .count(),
            1
        );

        let single_day = super::timeline(&connection, "2026-10-30", "2026-10-30", today)
            .expect("single-day timeline should load");
        assert!(
            single_day
                .events
                .iter()
                .all(|event| event.date == "2026-10-30")
        );
        assert!(
            single_day
                .events
                .iter()
                .any(|event| event.kind == "study_session")
        );
        assert!(
            !single_day
                .events
                .iter()
                .any(|event| event.kind == "revision")
        );

        let task_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM checklist_tasks", [], |row| row.get(0))
            .expect("task count should load");
        assert_eq!(task_count, 1);
    }
}

#[cfg(test)]
mod insight_tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{db, planner, pyq, revision, sessions, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn topic_id(connection: &rusqlite::Connection, code: &str) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = ?1",
                [code],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    #[test]
    fn insights_explain_deadlines_study_decline_and_part_a_gap_without_combining_scores() {
        let today = date(2026, 10, 30);
        let mut connection = db::test_connection_with_defaults(today);
        syllabus::seed(&mut connection).expect("syllabus should seed");
        let topic_a = topic_id(&connection, "MMP-C01");

        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic_a),
                session_date: "2026-10-20".to_owned(),
                duration_seconds: 7_200,
                notes: String::new(),
            },
        )
        .expect("previous-week session should save");
        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic_a),
                session_date: "2026-10-25".to_owned(),
                duration_seconds: 3_600,
                notes: String::new(),
            },
        )
        .expect("current-week session should save");
        planner::create_task(
            &mut connection,
            planner::TaskInput {
                title: "Upcoming deadline".to_owned(),
                description: String::new(),
                scheduled_date: "2026-10-30".to_owned(),
                deadline_date: Some("2026-10-31".to_owned()),
                topic_id: Some(topic_a),
                priority: "High".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: None,
            },
            today,
        )
        .expect("deadline should save");

        for index in 0..10 {
            pyq::create(
                &mut connection,
                pyq::PyqInput {
                    question_year: 2024,
                    exam_session: String::new(),
                    part: "A".to_owned(),
                    topic_id: topic_a,
                    question_reference: format!("PART-A-{index}"),
                    attempt_date: "2026-10-29".to_owned(),
                    status: if index < 3 { "correct" } else { "incorrect" }.to_owned(),
                    marks_obtained: None,
                    max_marks: None,
                    time_spent_seconds: 60,
                    notes: String::new(),
                    difficulty: None,
                },
            )
            .expect("Part A PYQ should save");
        }
        for index in 0..10 {
            pyq::create(
                &mut connection,
                pyq::PyqInput {
                    question_year: 2024,
                    exam_session: String::new(),
                    part: "B".to_owned(),
                    topic_id: topic_a,
                    question_reference: format!("PART-B-{index}"),
                    attempt_date: "2026-10-29".to_owned(),
                    status: "correct".to_owned(),
                    marks_obtained: None,
                    max_marks: None,
                    time_spent_seconds: 60,
                    notes: String::new(),
                    difficulty: None,
                },
            )
            .expect("Part B PYQ should save");
        }

        let revision_stats = revision::dashboard(&connection, today)
            .expect("revision dashboard should load")
            .stats;
        let insights =
            actionable_insights(&connection, today, &revision_stats).expect("insights should load");
        assert!(insights.iter().any(|insight| insight.id == "deadline-1"));
        assert!(
            insights
                .iter()
                .any(|insight| insight.id == "study-time-decline")
        );
        assert!(
            insights
                .iter()
                .any(|insight| insight.id == "pyq-part-a-gap")
        );
        assert!(
            insights
                .iter()
                .all(|insight| !insight.title.to_lowercase().contains("readiness"))
        );
        assert!(
            insights
                .iter()
                .all(|insight| !insight.reason.to_lowercase().contains("caused by"))
        );
        let deadline = insights
            .iter()
            .find(|insight| insight.id == "deadline-1")
            .expect("deadline insight should exist");
        assert_eq!(deadline.target_view, "checklist");
        let decline = insights
            .iter()
            .find(|insight| insight.id == "study-time-decline")
            .expect("study decline insight should exist");
        assert_eq!(decline.target_view, "analytics:study");
        let part_a = insights
            .iter()
            .find(|insight| insight.id == "pyq-part-a-gap")
            .expect("Part A insight should exist");
        assert_eq!(part_a.target_view, "analytics:performance");
        assert_eq!(insights.len(), 3);
        assert!(
            !insights
                .iter()
                .any(|insight| insight.id.starts_with("weak-topic-"))
        );
    }
}
