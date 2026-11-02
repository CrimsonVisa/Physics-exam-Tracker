use std::collections::HashMap;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{
        self, EVIDENCE_WINDOW_DAYS, MINIMUM_EVIDENCE_QUESTIONS, STRONG_ACCURACY_THRESHOLD,
        WEAK_ACCURACY_THRESHOLD,
    },
    date_only,
    error::AppError,
};

pub const REVISION_OUTCOMES: [&str; 4] = ["again", "difficult", "good", "easy"];
pub const MAX_INTERVAL_DAYS: i64 = 3_650;
pub const MAX_NOTES_CHARS: usize = 2_000;
pub const UPCOMING_WINDOW_DAYS: i64 = 30;
pub const RECENT_HISTORY_DAYS: i64 = 14;
pub const RECENT_HISTORY_LIMIT: i64 = 20;
pub const MAX_RECOMMENDATIONS: usize = 8;
pub const MOCK_SCOPE_NOTE: &str = "Mock tests are stored at part level and do not map to individual syllabus topics. They remain available in Analytics, but are not used to rank topic recommendations.";

const HIGH_PRIORITY_VALUES: [&str; 2] = ["High-Yield", "High"];
const URGENT_WINDOW_DAYS: i64 = 7;

fn normalize_priority(value: String) -> String {
    match value.as_str() {
        "High-Yield" | "High" | "Normal" | "Low" => value,
        _ => "Normal".to_owned(),
    }
}

fn normalize_syllabus_status(value: String) -> String {
    match value.as_str() {
        "Not started" | "Learning" | "Revised" | "Confident" => value,
        _ => "Not started".to_owned(),
    }
}

fn normalize_confidence(value: i64) -> i64 {
    if (1..=5).contains(&value) { value } else { 3 }
}

fn normalize_category(value: String) -> String {
    if value.is_empty() {
        "Core".to_owned()
    } else {
        value
    }
}

const SELECT_SCHEDULE_FIELDS: &str = "status, last_revision_date, next_revision_date, \
    current_interval_days, completed_revisions, previous_outcome, mastery_estimate, \
    notes, deferred_until, updated_at";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSettings {
    pub initial_again_days: i64,
    pub initial_difficult_days: i64,
    pub initial_good_days: i64,
    pub initial_easy_days: i64,
    pub growth_multiplier: f64,
    pub minimum_interval_days: i64,
    pub maximum_interval_days: i64,
    pub include_weekends: bool,
    pub daily_revision_target: i64,
    pub daily_revision_minutes: i64,
    pub reminders_enabled: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSettingsInput {
    pub initial_again_days: i64,
    pub initial_difficult_days: i64,
    pub initial_good_days: i64,
    pub initial_easy_days: i64,
    pub growth_multiplier: f64,
    pub minimum_interval_days: i64,
    pub maximum_interval_days: i64,
    pub include_weekends: bool,
    pub daily_revision_target: i64,
    pub daily_revision_minutes: i64,
    pub reminders_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionTopic {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
    pub section_code: String,
    pub section_title: String,
    pub priority: String,
    pub confidence: i64,
    pub syllabus_status: String,
    pub category: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSchedule {
    pub status: String,
    pub last_revision_date: Option<String>,
    pub next_revision_date: String,
    pub current_interval_days: i64,
    pub completed_revisions: i64,
    pub previous_outcome: Option<String>,
    pub mastery_estimate: f64,
    pub notes: String,
    pub deferred_until: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionTopicState {
    pub topic: RevisionTopic,
    pub schedule: Option<RevisionSchedule>,
    pub due_state: String,
    pub days_overdue: i64,
    pub days_until_due: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionStats {
    pub scheduled_count: i64,
    pub active_count: i64,
    pub suspended_count: i64,
    pub due_today_count: i64,
    pub overdue_count: i64,
    pub completed_today: i64,
    pub completed_last_7_days: i64,
    pub total_completed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionHistoryEntry {
    pub id: i64,
    pub topic_id: i64,
    pub revision_date: String,
    pub scheduled_for_date: String,
    pub outcome: String,
    pub notes: String,
    pub interval_before_days: i64,
    pub interval_after_days: i64,
    pub mastery_before: f64,
    pub mastery_after: f64,
    pub created_at: String,
    pub plan_item_id: Option<i64>,
    pub actual_seconds: Option<i64>,
    pub timing_days: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionHistoryWithTopic {
    pub history: RevisionHistoryEntry,
    pub topic: RevisionTopic,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionRecommendation {
    pub topic: RevisionTopic,
    pub scheduled: bool,
    pub due_state: String,
    pub next_revision_date: Option<String>,
    pub previous_outcome: Option<String>,
    pub mastery_estimate: Option<f64>,
    pub pyq_attempts: i64,
    pub pyq_questions: i64,
    pub pyq_accuracy: Option<f64>,
    pub pyq_evidence_accuracy: Option<f64>,
    pub pyq_last_attempt_date: Option<String>,
    pub pyq_sufficient: bool,
    pub lane: String,
    pub triage_score: f64,
    pub days_overdue: i64,
    pub days_until_due: Option<i64>,
    pub due_urgency: f64,
    pub priority_need: f64,
    pub confidence_need: f64,
    pub status_need: f64,
    pub confidence_status_need: f64,
    pub pyq_evidence_need: f64,
    pub revision_risk: f64,
    pub evidence_status: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDashboard {
    pub today: String,
    pub settings: RevisionSettings,
    pub stats: RevisionStats,
    pub due_today: Vec<RevisionTopicState>,
    pub overdue: Vec<RevisionTopicState>,
    pub upcoming: Vec<RevisionTopicState>,
    pub suspended: Vec<RevisionTopicState>,
    pub completed_recent: Vec<RevisionHistoryWithTopic>,
    pub topics: Vec<RevisionTopicState>,
    pub archived_topics: Vec<RevisionTopicState>,
    pub recommendations: Vec<RevisionRecommendation>,
    pub mock_scope_note: String,
    pub mock_test_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    pub topic_id: i64,
    pub status: Option<String>,
    pub due_state: String,
    pub next_revision_date: Option<String>,
    pub completed_revisions: i64,
    pub mastery_estimate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionCalendarItem {
    pub topic_id: i64,
    pub topic_code: String,
    pub topic_title: String,
    pub section_code: String,
    pub section_title: String,
    pub next_revision_date: String,
    pub current_interval_days: i64,
    pub completed_revisions: i64,
    pub mastery_estimate: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRevisionInput {
    pub outcome: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub plan_item_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionCompletion {
    pub state: RevisionTopicState,
    pub history: RevisionHistoryEntry,
    pub plan_item_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionReminder {
    pub date: String,
    pub due_today: i64,
    pub overdue: i64,
    pub total: i64,
}

pub fn load_settings(connection: &Connection) -> Result<RevisionSettings, AppError> {
    connection
        .query_row(
            "SELECT
                initial_again_days,
                initial_difficult_days,
                initial_good_days,
                initial_easy_days,
                growth_multiplier,
                minimum_interval_days,
                maximum_interval_days,
                include_weekends,
                daily_revision_target,
                daily_revision_minutes,
                reminders_enabled,
                updated_at
             FROM revision_settings
             WHERE id = 1",
            [],
            |row| {
                Ok(RevisionSettings {
                    initial_again_days: row.get(0)?,
                    initial_difficult_days: row.get(1)?,
                    initial_good_days: row.get(2)?,
                    initial_easy_days: row.get(3)?,
                    growth_multiplier: row.get(4)?,
                    minimum_interval_days: row.get(5)?,
                    maximum_interval_days: row.get(6)?,
                    include_weekends: row.get::<_, i64>(7)? != 0,
                    daily_revision_target: row.get(8)?,
                    daily_revision_minutes: row.get(9)?,
                    reminders_enabled: row.get::<_, i64>(10)? != 0,
                    updated_at: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Revision settings could not be loaded", error))?
        .ok_or_else(AppError::revision_settings_not_found)
}

pub fn update_settings(
    connection: &mut Connection,
    input: RevisionSettingsInput,
) -> Result<RevisionSettings, AppError> {
    validate_settings(&input)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-settings transaction could not be started",
            error,
        )
    })?;

    let changed = transaction
        .execute(
            "UPDATE revision_settings
             SET initial_again_days = ?1,
                 initial_difficult_days = ?2,
                 initial_good_days = ?3,
                 initial_easy_days = ?4,
                 growth_multiplier = ?5,
                 minimum_interval_days = ?6,
                 maximum_interval_days = ?7,
                 include_weekends = ?8,
                 daily_revision_target = ?9,
                 daily_revision_minutes = ?10,
                 reminders_enabled = ?11,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![
                input.initial_again_days,
                input.initial_difficult_days,
                input.initial_good_days,
                input.initial_easy_days,
                input.growth_multiplier,
                input.minimum_interval_days,
                input.maximum_interval_days,
                i64::from(input.include_weekends),
                input.daily_revision_target,
                input.daily_revision_minutes,
                i64::from(input.reminders_enabled),
            ],
        )
        .map_err(|error| AppError::database("Revision settings could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::revision_settings_not_found());
    }

    let settings = load_settings(&transaction)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision settings could not be committed", error))?;

    Ok(settings)
}

fn validate_settings(input: &RevisionSettingsInput) -> Result<(), AppError> {
    for (field, value) in [
        ("initialAgainDays", input.initial_again_days),
        ("initialDifficultDays", input.initial_difficult_days),
        ("initialGoodDays", input.initial_good_days),
        ("initialEasyDays", input.initial_easy_days),
        ("minimumIntervalDays", input.minimum_interval_days),
        ("maximumIntervalDays", input.maximum_interval_days),
    ] {
        if !(1..=MAX_INTERVAL_DAYS).contains(&value) {
            return Err(AppError::validation(
                field,
                format!("Intervals must be between 1 and {MAX_INTERVAL_DAYS} days."),
            ));
        }
    }

    if input.minimum_interval_days > input.maximum_interval_days {
        return Err(AppError::validation(
            "minimumIntervalDays",
            "Minimum interval cannot exceed maximum interval.",
        ));
    }

    for (field, value) in [
        ("initialAgainDays", input.initial_again_days),
        ("initialDifficultDays", input.initial_difficult_days),
        ("initialGoodDays", input.initial_good_days),
        ("initialEasyDays", input.initial_easy_days),
    ] {
        if value < input.minimum_interval_days || value > input.maximum_interval_days {
            return Err(AppError::validation(
                field,
                "Initial outcome intervals must stay between the minimum and maximum intervals.",
            ));
        }
    }

    if !input.growth_multiplier.is_finite() || !(1.0..=10.0).contains(&input.growth_multiplier) {
        return Err(AppError::validation(
            "growthMultiplier",
            "Growth multiplier must be a finite number from 1.0 to 10.0.",
        ));
    }

    if !(1..=100).contains(&input.daily_revision_target) {
        return Err(AppError::validation(
            "dailyRevisionTarget",
            "Daily revision target must be between 1 and 100.",
        ));
    }

    if !(5..=1440).contains(&input.daily_revision_minutes) {
        return Err(AppError::validation(
            "dailyRevisionMinutes",
            "Daily revision minutes must be between 5 and 1440.",
        ));
    }

    Ok(())
}

pub fn dashboard(connection: &Connection, today: NaiveDate) -> Result<RevisionDashboard, AppError> {
    let settings = load_settings(connection)?;
    let topics = topic_states(connection, today)?;
    let archived_topics = archived_topic_states(connection, today)?;
    let mut due_today = Vec::new();
    let mut overdue = Vec::new();
    let mut upcoming = Vec::new();
    let mut suspended = Vec::new();

    for state in &topics {
        match state.due_state.as_str() {
            "overdue" => overdue.push(state.clone()),
            "due_today" => due_today.push(state.clone()),
            "upcoming"
                if state
                    .days_until_due
                    .is_some_and(|days| days <= UPCOMING_WINDOW_DAYS) =>
            {
                upcoming.push(state.clone())
            }
            "suspended" => suspended.push(state.clone()),
            _ => {}
        }
    }

    sort_due_states(&mut due_today);
    sort_due_states(&mut overdue);
    sort_due_states(&mut upcoming);
    suspended.sort_by(|left, right| left.topic.code.cmp(&right.topic.code));

    let today_text = date_only::format_date_only(today);
    let completed_today = count_history_since(connection, today)?;
    let completed_last_7_days = count_history_between(
        connection,
        today
            .checked_sub_days(Days::new(6))
            .unwrap_or(NaiveDate::MIN),
        today,
    )?;
    let total_completed = connection
        .query_row("SELECT count(*) FROM revision_history", [], |row| {
            row.get(0)
        })
        .map_err(|error| AppError::database("Revision history could not be counted", error))?;
    let stats = RevisionStats {
        scheduled_count: topics
            .iter()
            .filter(|state| state.schedule.is_some())
            .count() as i64,
        active_count: topics
            .iter()
            .filter(|state| {
                state
                    .schedule
                    .as_ref()
                    .is_some_and(|s| s.status == "active")
            })
            .count() as i64,
        suspended_count: suspended.len() as i64,
        due_today_count: due_today.len() as i64,
        overdue_count: overdue.len() as i64,
        completed_today,
        completed_last_7_days,
        total_completed,
    };

    let completed_recent = recent_history(connection, today)?;
    let evidence_from = analytics::evidence_cutoff(today);
    let evidence_to = date_only::format_date_only(today);
    let pyq_performance =
        analytics::topic_performance(connection, None, None, &evidence_from, &evidence_to)?;
    let pyq_by_topic = pyq_performance
        .into_iter()
        .map(|topic| (topic.topic_id, topic))
        .collect::<HashMap<_, _>>();
    let recommendations = recommendations(&topics, &pyq_by_topic);
    let mock_test_count = connection
        .query_row("SELECT count(*) FROM mock_tests", [], |row| row.get(0))
        .map_err(|error| AppError::database("Mock tests could not be counted", error))?;

    Ok(RevisionDashboard {
        today: today_text,
        settings,
        stats,
        due_today,
        overdue,
        upcoming,
        suspended,
        completed_recent,
        topics,
        archived_topics,
        recommendations,
        mock_scope_note: MOCK_SCOPE_NOTE.to_owned(),
        mock_test_count,
    })
}

pub fn summaries(
    connection: &Connection,
    today: NaiveDate,
) -> Result<Vec<RevisionSummary>, AppError> {
    Ok(topic_states(connection, today)?
        .into_iter()
        .map(|state| RevisionSummary {
            topic_id: state.topic.id,
            status: state
                .schedule
                .as_ref()
                .map(|schedule| schedule.status.clone()),
            due_state: state.due_state,
            next_revision_date: state
                .schedule
                .as_ref()
                .map(|schedule| schedule.next_revision_date.clone()),
            completed_revisions: state
                .schedule
                .as_ref()
                .map_or(0, |schedule| schedule.completed_revisions),
            mastery_estimate: state
                .schedule
                .as_ref()
                .map(|schedule| schedule.mastery_estimate),
        })
        .collect())
}

pub fn history(
    connection: &Connection,
    topic_id: i64,
) -> Result<Vec<RevisionHistoryEntry>, AppError> {
    load_topic(connection, topic_id)?;
    let mut statement = connection
        .prepare(
            "SELECT
                h.id,
                h.topic_id,
                h.revision_date,
                h.scheduled_for_date,
                h.outcome,
                h.notes,
                h.interval_before_days,
                h.interval_after_days,
                h.mastery_before,
                h.mastery_after,
                h.created_at,
                pi.id,
                (
                    SELECT SUM(ss.duration_seconds)
                    FROM revision_session_links rsl
                    JOIN study_sessions ss ON ss.id = rsl.study_session_id
                    WHERE rsl.plan_item_id = pi.id
                )
             FROM revision_history h
             LEFT JOIN revision_plan_items pi ON pi.revision_history_id = h.id
             WHERE h.topic_id = ?1
             ORDER BY h.revision_date DESC, h.id DESC",
        )
        .map_err(|error| AppError::database("Revision history could not be prepared", error))?;

    statement
        .query_map([topic_id], map_history)
        .map_err(|error| AppError::database("Revision history could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Revision history could not be read", error))
}

pub fn schedule_topic(
    connection: &mut Connection,
    topic_id: i64,
    next_date: &str,
    today: NaiveDate,
) -> Result<RevisionTopicState, AppError> {
    validate_id(topic_id, "topicId")?;
    let next = parse_date(next_date, "nextRevisionDate")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-schedule transaction could not be started",
            error,
        )
    })?;
    let topic = load_topic(&transaction, topic_id)?;
    ensure_topic_is_mutable(&topic)?;

    if let Some(schedule) = load_schedule(&transaction, topic_id)? {
        let state = build_state(topic, Some(schedule), today)?;
        transaction.commit().map_err(|error| {
            AppError::database("The revision schedule could not be committed", error)
        })?;
        return Ok(state);
    }

    let settings = load_settings(&transaction)?;
    let next_text = date_only::format_date_only(next);
    transaction
        .execute(
            "INSERT INTO revision_schedules (
                topic_id, status, last_revision_date, next_revision_date,
                current_interval_days, completed_revisions, previous_outcome,
                mastery_estimate, notes, created_at, updated_at
             ) VALUES (
                ?1, 'active', NULL, ?2,
                ?3, 0, NULL,
                0, '',
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![topic_id, next_text, settings.minimum_interval_days],
        )
        .map_err(|error| AppError::database("The revision schedule could not be created", error))?;

    let schedule = load_schedule(&transaction, topic_id)?
        .ok_or_else(|| AppError::internal("The new revision schedule could not be reloaded."))?;
    let state = build_state(topic, Some(schedule), today)?;
    transaction.commit().map_err(|error| {
        AppError::database("The revision schedule could not be committed", error)
    })?;

    Ok(state)
}

pub fn reschedule(
    connection: &mut Connection,
    topic_id: i64,
    next_date: &str,
    today: NaiveDate,
) -> Result<RevisionTopicState, AppError> {
    validate_id(topic_id, "topicId")?;
    let next = parse_date(next_date, "nextRevisionDate")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-reschedule transaction could not be started",
            error,
        )
    })?;
    let topic = load_topic(&transaction, topic_id)?;
    ensure_topic_is_mutable(&topic)?;
    load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;

    transaction
        .execute(
            "UPDATE revision_schedules
             SET next_revision_date = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?2",
            params![date_only::format_date_only(next), topic_id],
        )
        .map_err(|error| AppError::database("The revision could not be rescheduled", error))?;

    let schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    let state = build_state(topic, Some(schedule), today)?;
    transaction.commit().map_err(|error| {
        AppError::database("The revision reschedule could not be committed", error)
    })?;

    Ok(state)
}

pub fn update_notes(
    connection: &mut Connection,
    topic_id: i64,
    notes: &str,
    today: NaiveDate,
) -> Result<RevisionTopicState, AppError> {
    validate_id(topic_id, "topicId")?;
    validate_notes(notes)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision-notes transaction could not be started", error)
    })?;
    let topic = load_topic(&transaction, topic_id)?;
    ensure_topic_is_mutable(&topic)?;
    load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;

    transaction
        .execute(
            "UPDATE revision_schedules
             SET notes = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?2",
            params![notes, topic_id],
        )
        .map_err(|error| AppError::database("Revision notes could not be updated", error))?;

    let schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    let state = build_state(topic, Some(schedule), today)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision notes could not be committed", error))?;

    Ok(state)
}

pub fn set_status(
    connection: &mut Connection,
    topic_id: i64,
    status: &str,
    today: NaiveDate,
) -> Result<RevisionTopicState, AppError> {
    validate_id(topic_id, "topicId")?;
    if !matches!(status, "active" | "suspended") {
        return Err(AppError::validation(
            "status",
            "Revision status must be active or suspended.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-status transaction could not be started",
            error,
        )
    })?;
    let topic = load_topic(&transaction, topic_id)?;
    ensure_topic_is_mutable(&topic)?;
    load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;

    transaction
        .execute(
            "UPDATE revision_schedules
             SET status = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?2",
            params![status, topic_id],
        )
        .map_err(|error| AppError::database("Revision status could not be updated", error))?;

    let schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    let state = build_state(topic, Some(schedule), today)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision status could not be committed", error))?;

    Ok(state)
}

pub fn complete(
    connection: &mut Connection,
    topic_id: i64,
    input: CompleteRevisionInput,
    today: NaiveDate,
) -> Result<RevisionCompletion, AppError> {
    validate_id(topic_id, "topicId")?;
    let outcome = input.outcome.trim().to_lowercase();
    if !REVISION_OUTCOMES.contains(&outcome.as_str()) {
        return Err(AppError::validation(
            "outcome",
            "Choose Again, Difficult, Good or Easy.",
        ));
    }
    validate_notes(&input.notes)?;
    let plan_item_id = input.plan_item_id;
    crate::revision_plan::reconcile_for_revision_outcome(connection, today)?;

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-completion transaction could not be started",
            error,
        )
    })?;
    let settings = load_settings(&transaction)?;
    let topic = load_topic(&transaction, topic_id)?;
    ensure_topic_is_mutable(&topic)?;
    let schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    if schedule.status == "suspended" {
        return Err(AppError::revision_suspended());
    }
    let today_text = date_only::format_date_only(today);
    if schedule
        .deferred_until
        .as_deref()
        .is_some_and(|deferred_until| deferred_until > today_text.as_str())
    {
        return Err(AppError::validation(
            "topicId",
            "This revision is deferred and cannot be completed before its return date.",
        ));
    }
    let plan_item_id = match plan_item_id {
        Some(plan_item_id) => Some(plan_item_id),
        None => match crate::revision_plan::active_plan_item_for_topic(
            &transaction,
            topic_id,
            &today_text,
        )? {
            Some(plan_item_id) => Some(plan_item_id),
            None => crate::revision_plan::pending_work_item_for_topic(&transaction, topic_id)?,
        },
    };

    let interval_before = schedule.current_interval_days;
    let mastery_before = schedule.mastery_estimate;
    let scheduled_for_date = parse_date(&schedule.next_revision_date, "scheduledForDate")?;
    let timing_days = (today - scheduled_for_date).num_days();
    let evidence_status = evidence_status_for_topic(&transaction, today, topic_id)?;
    let interval_after = next_interval_days_with_context(
        &settings,
        interval_before,
        &outcome,
        timing_days,
        evidence_status,
    );
    let mastery_after =
        next_mastery_estimate(mastery_before, schedule.completed_revisions, &outcome);
    let next_date = schedule_next_date(today, interval_after, settings.include_weekends)?;
    let next_text = date_only::format_date_only(next_date);

    transaction
        .execute(
            "UPDATE revision_schedules
             SET last_revision_date = ?1,
                 next_revision_date = ?2,
                 current_interval_days = ?3,
                 completed_revisions = completed_revisions + 1,
                 previous_outcome = ?4,
                 mastery_estimate = ?5,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?6",
            params![
                today_text,
                next_text,
                interval_after,
                outcome,
                mastery_after,
                topic_id,
            ],
        )
        .map_err(|error| AppError::database("The revision schedule could not be updated", error))?;

    transaction
        .execute(
            "INSERT INTO revision_history (
                topic_id, revision_date, scheduled_for_date, outcome, notes,
                interval_before_days, interval_after_days, mastery_before,
                mastery_after, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8,
                ?9,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                topic_id,
                today_text,
                schedule.next_revision_date,
                outcome,
                input.notes,
                interval_before,
                interval_after,
                mastery_before,
                mastery_after,
            ],
        )
        .map_err(|error| AppError::database("The revision history could not be appended", error))?;

    let history_id = transaction.last_insert_rowid();
    let linked_plan_item = crate::revision_plan::attach_revision_outcome(
        &transaction,
        topic_id,
        history_id,
        plan_item_id,
        &today_text,
    )?;
    let history = load_history_entry(&transaction, history_id)?;
    let updated_schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    let state = build_state(topic, Some(updated_schedule), today)?;
    transaction.commit().map_err(|error| {
        AppError::database("The revision completion could not be committed", error)
    })?;

    Ok(RevisionCompletion {
        state,
        history,
        plan_item_id: linked_plan_item,
    })
}

pub fn prepare_reminder(
    connection: &mut Connection,
    today: NaiveDate,
) -> Result<Option<RevisionReminder>, AppError> {
    let settings = load_settings(connection)?;
    if !settings.reminders_enabled {
        return Ok(None);
    }

    let today_text = date_only::format_date_only(today);
    let last_reminder_date: Option<String> = connection
        .query_row(
            "SELECT last_reminder_date FROM revision_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("The revision reminder date could not be read", error)
        })?;
    if last_reminder_date.as_deref() == Some(today_text.as_str()) {
        return Ok(None);
    }

    let due_today: i64 = connection
        .query_row(
            "SELECT count(*)
             FROM revision_schedules r
             JOIN syllabus_topics t ON t.id = r.topic_id
             WHERE r.status = 'active'
               AND t.is_archived = 0
               AND r.next_revision_date = ?1",
            [&today_text],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Due revisions could not be counted", error))?;
    let overdue: i64 = connection
        .query_row(
            "SELECT count(*)
             FROM revision_schedules r
             JOIN syllabus_topics t ON t.id = r.topic_id
             WHERE r.status = 'active'
               AND t.is_archived = 0
               AND r.next_revision_date < ?1",
            [&today_text],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Overdue revisions could not be counted", error))?;
    let total = due_today + overdue;
    if total == 0 {
        return Ok(None);
    }

    connection
        .execute(
            "UPDATE revision_settings
             SET last_reminder_date = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            [&today_text],
        )
        .map_err(|error| {
            AppError::database("The revision reminder could not be recorded", error)
        })?;

    Ok(Some(RevisionReminder {
        date: today_text,
        due_today,
        overdue,
        total,
    }))
}

pub fn calendar_items(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<RevisionCalendarItem>, AppError> {
    let from = parse_date(from_date, "fromDate")?;
    let to = parse_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    let mut statement = connection
        .prepare(
            "SELECT
                t.id,
                t.code,
                t.title,
                s.code,
                s.title,
                r.next_revision_date,
                r.current_interval_days,
                r.completed_revisions,
                r.mastery_estimate
             FROM revision_schedules r
             JOIN syllabus_topics t ON t.id = r.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             WHERE r.status = 'active'
               AND t.is_archived = 0
               AND r.next_revision_date >= ?1
               AND r.next_revision_date <= ?2
             ORDER BY r.next_revision_date, s.display_order, t.display_order",
        )
        .map_err(|error| {
            AppError::database("Revision calendar items could not be prepared", error)
        })?;

    statement
        .query_map(
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            |row| {
                Ok(RevisionCalendarItem {
                    topic_id: row.get(0)?,
                    topic_code: row.get(1)?,
                    topic_title: row.get(2)?,
                    section_code: row.get(3)?,
                    section_title: row.get(4)?,
                    next_revision_date: row.get(5)?,
                    current_interval_days: row.get(6)?,
                    completed_revisions: row.get(7)?,
                    mastery_estimate: row.get(8)?,
                })
            },
        )
        .map_err(|error| AppError::database("Revision calendar items could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Revision calendar items could not be read", error))
}

pub fn scheduled_outcome_interval(settings: &RevisionSettings, outcome: &str) -> i64 {
    match outcome {
        "again" => settings.initial_again_days,
        "difficult" => settings.initial_difficult_days,
        "good" => settings.initial_good_days,
        "easy" => settings.initial_easy_days,
        _ => settings.initial_good_days,
    }
}

#[cfg(test)]
pub fn next_interval_days(
    settings: &RevisionSettings,
    current_interval_days: i64,
    outcome: &str,
) -> i64 {
    next_interval_days_with_context(
        settings,
        current_interval_days,
        outcome,
        0,
        EvidenceClass::None,
    )
}

pub fn next_interval_days_with_context(
    settings: &RevisionSettings,
    current_interval_days: i64,
    outcome: &str,
    timing_days: i64,
    evidence_status: EvidenceClass,
) -> i64 {
    let base = scheduled_outcome_interval(settings, outcome);
    let candidate = if matches!(outcome, "good" | "easy") {
        let grown = (current_interval_days as f64 * settings.growth_multiplier).round() as i64;
        base.max(grown)
    } else {
        base
    };

    let adjusted = if matches!(outcome, "good" | "easy") {
        let timing_factor =
            (1.0 + 0.15 * (timing_days as f64 / 14.0).clamp(-1.0, 1.0)).clamp(0.85, 1.15);
        let evidence_factor = if evidence_status == EvidenceClass::Weak {
            0.8
        } else {
            1.0
        };
        ((candidate as f64 * timing_factor * evidence_factor).round() as i64)
            .clamp(1, MAX_INTERVAL_DAYS)
            .max(base)
    } else {
        candidate
    };

    adjusted
        .clamp(
            settings.minimum_interval_days,
            settings.maximum_interval_days,
        )
        .clamp(1, MAX_INTERVAL_DAYS)
}

pub fn next_mastery_estimate(
    previous_mastery: f64,
    completed_revisions: i64,
    outcome: &str,
) -> f64 {
    let outcome_score = match outcome {
        "again" => 0.0,
        "difficult" => 35.0,
        "good" => 70.0,
        "easy" => 95.0,
        _ => 50.0,
    };
    let raw = if completed_revisions == 0 {
        outcome_score
    } else {
        previous_mastery * 0.7 + outcome_score * 0.3
    };

    (raw.clamp(0.0, 100.0) * 10.0).round() / 10.0
}

pub fn schedule_next_date(
    from: NaiveDate,
    interval_days: i64,
    include_weekends: bool,
) -> Result<NaiveDate, AppError> {
    let outside_calendar = || {
        AppError::validation(
            "nextRevisionDate",
            "The next revision date falls outside the supported calendar.",
        )
    };
    let added = from
        .checked_add_days(Days::new(interval_days.max(1) as u64))
        .filter(|date| date.year() <= 9999)
        .ok_or_else(outside_calendar)?;

    if include_weekends {
        return Ok(added);
    }

    let adjusted = match added.weekday() {
        Weekday::Sat => added.checked_add_days(Days::new(2)),
        Weekday::Sun => added.checked_add_days(Days::new(1)),
        _ => Some(added),
    }
    .filter(|date| date.year() <= 9999)
    .ok_or_else(outside_calendar)?;

    Ok(adjusted)
}

pub(crate) fn topic_states(
    connection: &Connection,
    today: NaiveDate,
) -> Result<Vec<RevisionTopicState>, AppError> {
    let topics = load_topics(connection)?;
    let mut schedules = load_schedules(connection)?;

    topics
        .into_iter()
        .map(|topic| {
            let schedule = schedules.remove(&topic.id);
            build_state(topic, schedule, today)
        })
        .collect()
}

fn archived_topic_states(
    connection: &Connection,
    today: NaiveDate,
) -> Result<Vec<RevisionTopicState>, AppError> {
    let topics = load_archived_topics(connection)?;
    let mut schedules = load_schedules(connection)?;
    let mut states = topics
        .into_iter()
        .map(|topic| {
            let schedule = schedules.remove(&topic.id);
            build_state(topic, schedule, today)
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    states.sort_by(|left, right| left.topic.code.cmp(&right.topic.code));
    Ok(states)
}

fn build_state(
    topic: RevisionTopic,
    schedule: Option<RevisionSchedule>,
    today: NaiveDate,
) -> Result<RevisionTopicState, AppError> {
    let (due_state, days_overdue, days_until_due) = match &schedule {
        None => ("not_scheduled".to_owned(), 0, None),
        Some(schedule) if schedule.status == "suspended" => ("suspended".to_owned(), 0, None),
        Some(schedule) => {
            let next = parse_date(&schedule.next_revision_date, "nextRevisionDate")?;
            if next < today {
                ("overdue".to_owned(), (today - next).num_days(), None)
            } else if next == today {
                ("due_today".to_owned(), 0, Some(0))
            } else {
                ("upcoming".to_owned(), 0, Some((next - today).num_days()))
            }
        }
    };

    Ok(RevisionTopicState {
        topic,
        schedule,
        due_state,
        days_overdue,
        days_until_due,
    })
}

pub(crate) fn load_topics(connection: &Connection) -> Result<Vec<RevisionTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                t.id,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title,
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.status, 'Not started'),
                t.category
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             WHERE t.is_archived = 0
             ORDER BY s.display_order, t.display_order, t.code",
        )
        .map_err(|error| AppError::database("Revision topics could not be prepared", error))?;

    statement
        .query_map([], |row| {
            Ok(RevisionTopic {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                section_code: row.get(4)?,
                section_title: row.get(5)?,
                priority: normalize_priority(row.get(6)?),
                confidence: normalize_confidence(row.get(7)?),
                syllabus_status: normalize_syllabus_status(row.get(8)?),
                category: normalize_category(row.get(9)?),
            })
        })
        .map_err(|error| AppError::database("Revision topics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Revision topics could not be read", error))
}

pub(crate) fn load_archived_topics(
    connection: &Connection,
) -> Result<Vec<RevisionTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                t.id,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title,
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.status, 'Not started'),
                t.category
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             WHERE t.is_archived = 1
               AND (
                    EXISTS (SELECT 1 FROM revision_schedules r WHERE r.topic_id = t.id)
                    OR EXISTS (SELECT 1 FROM revision_history h WHERE h.topic_id = t.id)
               )
             ORDER BY s.display_order, t.display_order, t.code",
        )
        .map_err(|error| {
            AppError::database("Archived revision topics could not be prepared", error)
        })?;

    statement
        .query_map([], |row| {
            Ok(RevisionTopic {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
                section_code: row.get(4)?,
                section_title: row.get(5)?,
                priority: normalize_priority(row.get(6)?),
                confidence: normalize_confidence(row.get(7)?),
                syllabus_status: normalize_syllabus_status(row.get(8)?),
                category: normalize_category(row.get(9)?),
            })
        })
        .map_err(|error| {
            AppError::database("Archived revision topics could not be queried", error)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Archived revision topics could not be read", error))
}

fn ensure_topic_is_mutable(topic: &RevisionTopic) -> Result<(), AppError> {
    if topic.archived {
        return Err(AppError::validation(
            "topicId",
            "Archived topics are read-only. Unarchive the topic before changing its revision schedule.",
        ));
    }
    Ok(())
}

fn load_topic(connection: &Connection, topic_id: i64) -> Result<RevisionTopic, AppError> {
    connection
        .query_row(
            "SELECT
                t.id,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title,
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.status, 'Not started'),
                t.category
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             WHERE t.id = ?1",
            [topic_id],
            |row| {
                Ok(RevisionTopic {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    title: row.get(2)?,
                    archived: row.get::<_, i64>(3)? != 0,
                    section_code: row.get(4)?,
                    section_title: row.get(5)?,
                    priority: normalize_priority(row.get(6)?),
                    confidence: normalize_confidence(row.get(7)?),
                    syllabus_status: normalize_syllabus_status(row.get(8)?),
                    category: normalize_category(row.get(9)?),
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("The revision topic could not be loaded", error))?
        .ok_or_else(AppError::topic_not_found)
}

fn load_schedules(connection: &Connection) -> Result<HashMap<i64, RevisionSchedule>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT topic_id, {SELECT_SCHEDULE_FIELDS} FROM revision_schedules"
        ))
        .map_err(|error| AppError::database("Revision schedules could not be prepared", error))?;

    statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, map_schedule(row, 1)?)))
        .map_err(|error| AppError::database("Revision schedules could not be queried", error))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(|error| AppError::database("Revision schedules could not be read", error))
}

fn load_schedule(
    connection: &Connection,
    topic_id: i64,
) -> Result<Option<RevisionSchedule>, AppError> {
    connection
        .query_row(
            &format!("SELECT {SELECT_SCHEDULE_FIELDS} FROM revision_schedules WHERE topic_id = ?1"),
            [topic_id],
            |row| map_schedule(row, 0),
        )
        .optional()
        .map_err(|error| AppError::database("The revision schedule could not be loaded", error))
}

fn map_schedule(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<RevisionSchedule> {
    Ok(RevisionSchedule {
        status: row.get(offset)?,
        last_revision_date: row.get(offset + 1)?,
        next_revision_date: row.get(offset + 2)?,
        current_interval_days: row.get(offset + 3)?,
        completed_revisions: row.get(offset + 4)?,
        previous_outcome: row.get(offset + 5)?,
        mastery_estimate: row.get(offset + 6)?,
        notes: row.get(offset + 7)?,
        deferred_until: row.get(offset + 8)?,
        updated_at: row.get(offset + 9)?,
    })
}

fn count_history_since(connection: &Connection, date: NaiveDate) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT count(*) FROM revision_history WHERE revision_date = ?1",
            [date_only::format_date_only(date)],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Completed revisions could not be counted", error))
}

fn count_history_between(
    connection: &Connection,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT count(*) FROM revision_history
             WHERE revision_date >= ?1 AND revision_date <= ?2",
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Recent revisions could not be counted", error))
}

fn recent_history(
    connection: &Connection,
    today: NaiveDate,
) -> Result<Vec<RevisionHistoryWithTopic>, AppError> {
    let from = today
        .checked_sub_days(Days::new(RECENT_HISTORY_DAYS as u64))
        .unwrap_or(NaiveDate::MIN);
    let mut statement = connection
        .prepare(
            "SELECT
                h.id,
                h.topic_id,
                h.revision_date,
                h.scheduled_for_date,
                h.outcome,
                h.notes,
                h.interval_before_days,
                h.interval_after_days,
                h.mastery_before,
                h.mastery_after,
                h.created_at,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title,
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.status, 'Not started'),
                t.category,
                pi.id,
                (
                    SELECT SUM(ss.duration_seconds)
                    FROM revision_session_links rsl
                    JOIN study_sessions ss ON ss.id = rsl.study_session_id
                    WHERE rsl.plan_item_id = pi.id
                )
             FROM revision_history h
             JOIN syllabus_topics t ON t.id = h.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             LEFT JOIN revision_plan_items pi ON pi.revision_history_id = h.id
             WHERE h.revision_date >= ?1
             ORDER BY h.revision_date DESC, h.id DESC
             LIMIT ?2",
        )
        .map_err(|error| AppError::database("Recent revisions could not be prepared", error))?;

    statement
        .query_map(
            params![date_only::format_date_only(from), RECENT_HISTORY_LIMIT],
            |row| {
                Ok(RevisionHistoryWithTopic {
                    history: map_history_at(row, 0, Some(20), Some(21))?,
                    topic: RevisionTopic {
                        id: row.get(1)?,
                        code: row.get(11)?,
                        title: row.get(12)?,
                        archived: row.get::<_, i64>(13)? != 0,
                        section_code: row.get(14)?,
                        section_title: row.get(15)?,
                        priority: normalize_priority(row.get(16)?),
                        confidence: normalize_confidence(row.get(17)?),
                        syllabus_status: normalize_syllabus_status(row.get(18)?),
                        category: normalize_category(row.get(19)?),
                    },
                })
            },
        )
        .map_err(|error| AppError::database("Recent revisions could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Recent revisions could not be read", error))
}

fn load_history_entry(
    connection: &Connection,
    history_id: i64,
) -> Result<RevisionHistoryEntry, AppError> {
    connection
        .query_row(
            "SELECT
                h.id,
                h.topic_id,
                h.revision_date,
                h.scheduled_for_date,
                h.outcome,
                h.notes,
                h.interval_before_days,
                h.interval_after_days,
                h.mastery_before,
                h.mastery_after,
                h.created_at,
                pi.id,
                (
                    SELECT SUM(ss.duration_seconds)
                    FROM revision_session_links rsl
                    JOIN study_sessions ss ON ss.id = rsl.study_session_id
                    WHERE rsl.plan_item_id = pi.id
                )
             FROM revision_history h
             LEFT JOIN revision_plan_items pi ON pi.revision_history_id = h.id
             WHERE h.id = ?1",
            [history_id],
            |row| map_history_at(row, 0, Some(11), Some(12)),
        )
        .optional()
        .map_err(|error| AppError::database("Revision history could not be loaded", error))?
        .ok_or_else(|| AppError::internal("The revision history entry could not be reloaded."))
}

fn map_history(row: &rusqlite::Row<'_>) -> rusqlite::Result<RevisionHistoryEntry> {
    map_history_at(row, 0, Some(11), Some(12))
}

fn map_history_at(
    row: &rusqlite::Row<'_>,
    offset: usize,
    plan_item_offset: Option<usize>,
    actual_seconds_offset: Option<usize>,
) -> rusqlite::Result<RevisionHistoryEntry> {
    let revision_date: String = row.get(offset + 2)?;
    let scheduled_for_date: String = row.get(offset + 3)?;
    let timing_days = date_only::parse_date_only(&revision_date)
        .zip(date_only::parse_date_only(&scheduled_for_date))
        .map(|(revision, scheduled)| (revision - scheduled).num_days());
    let plan_item_id = match plan_item_offset {
        Some(index) => row.get(index)?,
        None => None,
    };
    let actual_seconds = match actual_seconds_offset {
        Some(index) => row.get(index)?,
        None => None,
    };

    Ok(RevisionHistoryEntry {
        id: row.get(offset)?,
        topic_id: row.get(offset + 1)?,
        revision_date,
        scheduled_for_date,
        outcome: row.get(offset + 4)?,
        notes: row.get(offset + 5)?,
        interval_before_days: row.get(offset + 6)?,
        interval_after_days: row.get(offset + 7)?,
        mastery_before: row.get(offset + 8)?,
        mastery_after: row.get(offset + 9)?,
        created_at: row.get(offset + 10)?,
        plan_item_id,
        actual_seconds,
        timing_days,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceClass {
    Weak,
    Strong,
    Moderate,
    Insufficient,
    Stale,
    SkippedOnly,
    Unscored,
    None,
}

impl EvidenceClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Weak => "weak",
            Self::Strong => "strong",
            Self::Moderate => "moderate",
            Self::Insufficient => "insufficient",
            Self::Stale => "stale",
            Self::SkippedOnly => "skipped_only",
            Self::Unscored => "unscored",
            Self::None => "none",
        }
    }

    fn is_insufficient(self) -> bool {
        matches!(
            self,
            Self::Insufficient | Self::Stale | Self::SkippedOnly | Self::Unscored | Self::None
        )
    }
}

struct EvidenceAssessment {
    class: EvidenceClass,
    need: f64,
    reason: String,
}

fn evidence_assessment(pyq: Option<&analytics::PyqTopicPerformance>) -> EvidenceAssessment {
    let Some(stats) = pyq else {
        return EvidenceAssessment {
            class: EvidenceClass::None,
            need: 0.0,
            reason: "No recent scored PYQ evidence.".to_owned(),
        };
    };

    if stats.skipped_only {
        return EvidenceAssessment {
            class: EvidenceClass::SkippedOnly,
            need: 0.0,
            reason:
                "PYQ evidence is skipped-only; skipped attempts do not count as scored evidence."
                    .to_owned(),
        };
    }

    if stats.evidence_questions >= MINIMUM_EVIDENCE_QUESTIONS {
        let Some(accuracy) = stats.evidence_accuracy else {
            return EvidenceAssessment {
                class: EvidenceClass::Insufficient,
                need: 0.0,
                reason: "Recent PYQ evidence has no scored accuracy yet.".to_owned(),
            };
        };
        let questions = stats.evidence_questions;
        if accuracy < WEAK_ACCURACY_THRESHOLD {
            return EvidenceAssessment {
                class: EvidenceClass::Weak,
                need: 100.0 * (questions as f64 / 10.0).min(1.0),
                reason: format!(
                    "Recent PYQ accuracy is weak ({accuracy:.1}%) across {questions} distinct questions."
                ),
            };
        }
        if accuracy >= STRONG_ACCURACY_THRESHOLD {
            return EvidenceAssessment {
                class: EvidenceClass::Strong,
                need: 0.0,
                reason: format!(
                    "Recent PYQ accuracy is strong ({accuracy:.1}%) across {questions} distinct questions."
                ),
            };
        }
        return EvidenceAssessment {
            class: EvidenceClass::Moderate,
            need: 0.0,
            reason: format!(
                "Recent PYQ accuracy is {accuracy:.1}% across {questions} distinct questions."
            ),
        };
    }

    if stats.evidence_questions > 0 {
        return EvidenceAssessment {
            class: EvidenceClass::Insufficient,
            need: 0.0,
            reason: format!(
                "Only {} distinct scored PYQ question{}; at least {} are required for weak or strong evidence.",
                stats.evidence_questions,
                if stats.evidence_questions == 1 {
                    ""
                } else {
                    "s"
                },
                MINIMUM_EVIDENCE_QUESTIONS
            ),
        };
    }

    if stats.attempts > 0 {
        return EvidenceAssessment {
            class: EvidenceClass::Stale,
            need: 0.0,
            reason: format!(
                "No scored PYQ evidence is within the last {} calendar days.",
                EVIDENCE_WINDOW_DAYS
            ),
        };
    }

    if stats.total_attempts > 0 {
        return EvidenceAssessment {
            class: EvidenceClass::Unscored,
            need: 0.0,
            reason: "PYQ attempts are recorded without a scored correct or incorrect outcome."
                .to_owned(),
        };
    }

    EvidenceAssessment {
        class: EvidenceClass::None,
        need: 0.0,
        reason: "No recent scored PYQ evidence.".to_owned(),
    }
}

pub(crate) fn evidence_status_for_topic(
    connection: &Connection,
    today: NaiveDate,
    topic_id: i64,
) -> Result<EvidenceClass, AppError> {
    let evidence_from = analytics::evidence_cutoff(today);
    let evidence_to = date_only::format_date_only(today);
    let performance =
        analytics::topic_performance(connection, None, None, &evidence_from, &evidence_to)?;
    let stats = performance.iter().find(|entry| entry.topic_id == topic_id);
    Ok(evidence_assessment(stats).class)
}

fn priority_need(priority: &str) -> f64 {
    match priority {
        "High-Yield" => 100.0,
        "High" => 75.0,
        "Normal" => 50.0,
        "Low" => 25.0,
        _ => 50.0,
    }
}

fn confidence_need(confidence: i64) -> f64 {
    match confidence {
        1 => 100.0,
        2 => 75.0,
        3 => 50.0,
        4 => 25.0,
        5 => 0.0,
        _ => 50.0,
    }
}

fn status_need(status: &str) -> f64 {
    match status {
        "Not started" => 100.0,
        "Learning" => 75.0,
        "Revised" => 50.0,
        "Confident" => 25.0,
        _ => 50.0,
    }
}

fn due_urgency(state: &RevisionTopicState) -> f64 {
    match state.due_state.as_str() {
        "overdue" => 100.0,
        "due_today" => 80.0,
        "upcoming" => (70.0 - 2.0 * state.days_until_due.unwrap_or(0) as f64).max(0.0),
        _ => 0.0,
    }
}

fn outcome_risk(outcome: Option<&str>) -> f64 {
    match outcome {
        Some("again") => 100.0,
        Some("difficult") => 75.0,
        Some("good") => 25.0,
        Some("easy") => 0.0,
        _ => 0.0,
    }
}

fn revision_risk(state: &RevisionTopicState) -> f64 {
    let Some(schedule) = state.schedule.as_ref() else {
        return 0.0;
    };
    let outcome = outcome_risk(schedule.previous_outcome.as_deref());
    let mastery = (100.0 - schedule.mastery_estimate).max(0.0);
    0.6 * outcome + 0.4 * mastery
}

const DUE_URGENCY_WEIGHT: f64 = 0.55;
const PRIORITY_WEIGHT: f64 = 0.10;
const CONFIDENCE_STATUS_WEIGHT: f64 = 0.10;
const EVIDENCE_WEIGHT: f64 = 0.15;
const REVISION_RISK_WEIGHT: f64 = 0.10;

/// Stage 3.2a triage score: `T = 0.55U + 0.10P + 0.10C + 0.15E + 0.10R`.
///
/// All components are normalized to 0-100 before weighting. The score is only
/// used to order recommendations; it never mutates schedules or mastery.
fn triage_score(
    due_urgency: f64,
    priority_need: f64,
    confidence_status_need: f64,
    pyq_evidence_need: f64,
    revision_risk: f64,
) -> f64 {
    round_two(
        DUE_URGENCY_WEIGHT * due_urgency
            + PRIORITY_WEIGHT * priority_need
            + CONFIDENCE_STATUS_WEIGHT * confidence_status_need
            + EVIDENCE_WEIGHT * pyq_evidence_need
            + REVISION_RISK_WEIGHT * revision_risk,
    )
}

fn is_high_priority(priority: &str) -> bool {
    HIGH_PRIORITY_VALUES.contains(&priority)
}

fn urgent_lane(state: &RevisionTopicState) -> bool {
    match state.due_state.as_str() {
        "overdue" | "due_today" => true,
        "upcoming" => state
            .days_until_due
            .is_some_and(|days| days <= URGENT_WINDOW_DAYS),
        _ => false,
    }
}

/// Lane precedence is intentional: a due topic stays urgent even when it also
/// has weak evidence or low confidence, and explanations retain both signals.
fn recommendation_lane(
    state: &RevisionTopicState,
    topic: &RevisionTopic,
    evidence: EvidenceClass,
    high_priority_gap: bool,
) -> &'static str {
    if urgent_lane(state) {
        "urgent_review"
    } else if evidence == EvidenceClass::Weak || topic.confidence <= 2 || high_priority_gap {
        "strengthen"
    } else {
        "maintain"
    }
}

pub(crate) fn ranked_recommendations(
    states: &[RevisionTopicState],
    pyq: &HashMap<i64, analytics::PyqTopicPerformance>,
) -> Vec<RevisionRecommendation> {
    let mut recommendations = Vec::new();

    for state in states {
        let topic = &state.topic;
        let pyq_stats = pyq.get(&topic.id);
        let pyq_attempts = pyq_stats.map_or(0, |entry| entry.attempts);
        let pyq_questions = pyq_stats.map_or(0, |entry| entry.evidence_questions);
        let pyq_accuracy = pyq_stats.and_then(|entry| entry.accuracy);
        let pyq_evidence_accuracy = pyq_stats.and_then(|entry| entry.evidence_accuracy);
        let pyq_last_attempt_date = pyq_stats
            .and_then(|entry| entry.last_scored_attempt_date.as_deref())
            .map(str::to_owned);
        let pyq_sufficient = pyq_questions >= MINIMUM_EVIDENCE_QUESTIONS;
        let assessment = evidence_assessment(pyq_stats);
        let high_priority_gap =
            is_high_priority(&topic.priority) && assessment.class.is_insufficient();
        let weak_unscheduled = state.schedule.is_none() && assessment.class == EvidenceClass::Weak;

        if state.schedule.is_none() && !weak_unscheduled && !high_priority_gap {
            continue;
        }
        if state.due_state == "suspended" {
            continue;
        }
        if !matches!(
            state.due_state.as_str(),
            "overdue" | "due_today" | "upcoming" | "not_scheduled"
        ) {
            continue;
        }

        let due = due_urgency(state);
        let priority = priority_need(&topic.priority);
        let confidence = confidence_need(topic.confidence);
        let status = status_need(&topic.syllabus_status);
        let confidence_status = 0.6 * confidence + 0.4 * status;
        let evidence = assessment.need;
        let risk = revision_risk(state);
        let score = triage_score(due, priority, confidence_status, evidence, risk);
        let lane = recommendation_lane(state, topic, assessment.class, high_priority_gap);

        let mut reasons = Vec::new();
        match state.due_state.as_str() {
            "overdue" => reasons.push(format!(
                "Overdue by {} day{}.",
                state.days_overdue,
                if state.days_overdue == 1 { "" } else { "s" }
            )),
            "due_today" => reasons.push("Due today.".to_owned()),
            "upcoming" => reasons.push(format!(
                "Due in {} day{}.",
                state.days_until_due.unwrap_or(0),
                if state.days_until_due == Some(1) {
                    ""
                } else {
                    "s"
                }
            )),
            "not_scheduled" => reasons.push("Not scheduled.".to_owned()),
            _ => {}
        }
        reasons.push(format!("Syllabus priority: {}.", topic.priority));
        reasons.push(format!("Confidence marked {}/5.", topic.confidence));
        reasons.push(format!("Syllabus status: {}.", topic.syllabus_status));
        reasons.push(assessment.reason);
        if high_priority_gap {
            reasons.push("High-priority topic has insufficient recent PYQ evidence.".to_owned());
        }

        if let Some(schedule) = &state.schedule {
            if let Some(outcome) = &schedule.previous_outcome {
                reasons.push(format!(
                    "Previous revision outcome was {}.",
                    outcome_label(outcome)
                ));
            }
            if schedule.completed_revisions > 0 {
                if schedule.mastery_estimate < 40.0 {
                    reasons.push(format!(
                        "Revision mastery is low ({:.1}%).",
                        schedule.mastery_estimate
                    ));
                } else if schedule.mastery_estimate >= 80.0 {
                    reasons.push(format!(
                        "Revision mastery is high ({:.1}%).",
                        schedule.mastery_estimate
                    ));
                } else {
                    reasons.push(format!(
                        "Revision mastery is {:.1}%.",
                        schedule.mastery_estimate
                    ));
                }
            }
        }

        if lane == "maintain" {
            if assessment.class == EvidenceClass::Strong
                && state
                    .schedule
                    .as_ref()
                    .is_some_and(|schedule| schedule.mastery_estimate >= 80.0)
            {
                reasons.push("Strong evidence and high revision mastery.".to_owned());
            } else {
                reasons.push(
                    "Scheduled review; no stronger learning-risk signal was identified.".to_owned(),
                );
            }
        }

        recommendations.push(RevisionRecommendation {
            topic: topic.clone(),
            scheduled: state.schedule.is_some(),
            due_state: state.due_state.clone(),
            next_revision_date: state
                .schedule
                .as_ref()
                .map(|schedule| schedule.next_revision_date.clone()),
            previous_outcome: state
                .schedule
                .as_ref()
                .and_then(|schedule| schedule.previous_outcome.clone()),
            mastery_estimate: state
                .schedule
                .as_ref()
                .map(|schedule| schedule.mastery_estimate),
            pyq_attempts,
            pyq_questions,
            pyq_accuracy,
            pyq_evidence_accuracy,
            pyq_last_attempt_date,
            pyq_sufficient,
            lane: lane.to_owned(),
            triage_score: score,
            days_overdue: state.days_overdue,
            days_until_due: state.days_until_due,
            due_urgency: due,
            priority_need: priority,
            confidence_need: confidence,
            status_need: status,
            confidence_status_need: confidence_status,
            pyq_evidence_need: evidence,
            revision_risk: risk,
            evidence_status: assessment.class.as_str().to_owned(),
            reasons,
        });
    }

    recommendations.sort_by(|left, right| {
        right
            .triage_score
            .partial_cmp(&left.triage_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.days_overdue.cmp(&left.days_overdue))
            .then_with(|| {
                right
                    .priority_need
                    .partial_cmp(&left.priority_need)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.next_revision_date.cmp(&right.next_revision_date))
            .then_with(|| left.topic.code.cmp(&right.topic.code))
    });
    recommendations
}

fn recommendations(
    states: &[RevisionTopicState],
    pyq: &HashMap<i64, analytics::PyqTopicPerformance>,
) -> Vec<RevisionRecommendation> {
    let mut recommendations = ranked_recommendations(states, pyq);
    recommendations.truncate(MAX_RECOMMENDATIONS);
    recommendations
}

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "again" => "Again",
        "difficult" => "Difficult",
        "good" => "Good",
        "easy" => "Easy",
        _ => "Unknown",
    }
}

fn sort_due_states(states: &mut [RevisionTopicState]) {
    states.sort_by(|left, right| {
        left.schedule
            .as_ref()
            .map(|schedule| schedule.next_revision_date.as_str())
            .cmp(
                &right
                    .schedule
                    .as_ref()
                    .map(|schedule| schedule.next_revision_date.as_str()),
            )
            .then_with(|| left.topic.code.cmp(&right.topic.code))
    });
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

fn validate_notes(notes: &str) -> Result<(), AppError> {
    if notes.chars().count() > MAX_NOTES_CHARS {
        Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ))
    } else {
        Ok(())
    }
}

fn validate_id(value: i64, field: &'static str) -> Result<(), AppError> {
    if value <= 0 {
        Err(AppError::validation(
            field,
            "The selected record is invalid.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;
    use crate::{db, mock_tests, planner, pyq, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(date(2026, 10, 30));
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

    fn settings_input(settings: &RevisionSettings) -> RevisionSettingsInput {
        RevisionSettingsInput {
            initial_again_days: settings.initial_again_days,
            initial_difficult_days: settings.initial_difficult_days,
            initial_good_days: settings.initial_good_days,
            initial_easy_days: settings.initial_easy_days,
            growth_multiplier: settings.growth_multiplier,
            minimum_interval_days: settings.minimum_interval_days,
            maximum_interval_days: settings.maximum_interval_days,
            include_weekends: settings.include_weekends,
            daily_revision_target: settings.daily_revision_target,
            daily_revision_minutes: settings.daily_revision_minutes,
            reminders_enabled: settings.reminders_enabled,
        }
    }

    fn pyq_input(topic_id: i64, status: &str, reference: &str) -> pyq::PyqInput {
        pyq_input_on(topic_id, status, reference, "2026-10-30")
    }

    fn recommendation_topic(
        id: i64,
        code: &str,
        priority: &str,
        confidence: i64,
        status: &str,
    ) -> RevisionTopic {
        RevisionTopic {
            id,
            code: code.to_owned(),
            title: format!("{code} title"),
            archived: false,
            section_code: "TST".to_owned(),
            section_title: "Test section".to_owned(),
            priority: priority.to_owned(),
            confidence,
            syllabus_status: status.to_owned(),
            category: "Core".to_owned(),
        }
    }

    fn recommendation_schedule(
        next_revision_date: &str,
        previous_outcome: Option<&str>,
        mastery_estimate: f64,
        completed_revisions: i64,
    ) -> RevisionSchedule {
        RevisionSchedule {
            status: "active".to_owned(),
            last_revision_date: Some("2026-10-20".to_owned()),
            next_revision_date: next_revision_date.to_owned(),
            current_interval_days: 5,
            completed_revisions,
            previous_outcome: previous_outcome.map(str::to_owned),
            mastery_estimate,
            notes: String::new(),
            deferred_until: None,
            updated_at: "2026-10-20T10:00:00.000Z".to_owned(),
        }
    }

    fn recommendation_state(
        topic: RevisionTopic,
        schedule: Option<RevisionSchedule>,
        due_state: &str,
        days_overdue: i64,
        days_until_due: Option<i64>,
    ) -> RevisionTopicState {
        RevisionTopicState {
            topic,
            schedule,
            due_state: due_state.to_owned(),
            days_overdue,
            days_until_due,
        }
    }

    fn recommendation_pyq(
        topic_id: i64,
        evidence_questions: i64,
        evidence_accuracy: f64,
        skipped_only: bool,
        attempts: i64,
        total_attempts: i64,
    ) -> analytics::PyqTopicPerformance {
        analytics::PyqTopicPerformance {
            topic_id,
            code: format!("TOPIC-{topic_id}"),
            title: format!("Topic {topic_id}"),
            archived: false,
            section_code: "TST".to_owned(),
            section_title: "Test section".to_owned(),
            correct: 0,
            incorrect: attempts,
            skipped: if skipped_only { total_attempts } else { 0 },
            attempts,
            total_attempts,
            distinct_questions: evidence_questions,
            evidence_questions,
            accuracy: (attempts > 0).then_some(evidence_accuracy),
            evidence_accuracy: (evidence_questions > 0).then_some(evidence_accuracy),
            last_scored_attempt_date: (attempts > 0).then(|| "2026-10-30".to_owned()),
            skipped_only,
            sufficient_data: evidence_questions >= MINIMUM_EVIDENCE_QUESTIONS,
        }
    }

    fn set_topic_progress(
        connection: &mut Connection,
        code: &str,
        status: &str,
        priority: &str,
        confidence: i64,
    ) {
        let id = topic_id(connection, code);
        syllabus::update_progress(
            connection,
            id,
            syllabus::TopicProgressInput {
                status: status.to_owned(),
                priority: priority.to_owned(),
                confidence,
                notes: String::new(),
            },
        )
        .expect("topic progress should save");
    }

    fn pyq_input_on(
        topic_id: i64,
        status: &str,
        reference: &str,
        attempt_date: &str,
    ) -> pyq::PyqInput {
        pyq::PyqInput {
            question_year: 2024,
            exam_session: String::new(),
            part: "B".to_owned(),
            topic_id,
            question_reference: reference.to_owned(),
            attempt_date: attempt_date.to_owned(),
            status: status.to_owned(),
            marks_obtained: None,
            max_marks: None,
            time_spent_seconds: 60,
            notes: String::new(),
            difficulty: None,
        }
    }

    fn minimal_mock_input() -> mock_tests::MockTestInput {
        mock_tests::MockTestInput {
            test_name: "Revision integration mock".to_owned(),
            test_date: "2026-10-30".to_owned(),
            duration_minutes: 180,
            max_marks: 100.0,
            marks_obtained: None,
            part_a: mock_tests::MockPartInput {
                questions: 1,
                correct: 1,
                incorrect: 0,
                unanswered: 0,
                correct_marks: Some(2.0),
                incorrect_marks: Some(0.0),
                unanswered_marks: Some(0.0),
            },
            part_b: mock_tests::MockPartInput {
                questions: 0,
                correct: 0,
                incorrect: 0,
                unanswered: 0,
                correct_marks: None,
                incorrect_marks: None,
                unanswered_marks: None,
            },
            part_c: mock_tests::MockPartInput {
                questions: 0,
                correct: 0,
                incorrect: 0,
                unanswered: 0,
                correct_marks: None,
                incorrect_marks: None,
                unanswered_marks: None,
            },
            notes: String::new(),
            review_observations: String::new(),
        }
    }

    #[test]
    fn default_intervals_growth_and_mastery_are_deterministic() {
        let connection = connection();
        let settings = load_settings(&connection).expect("settings should load");

        assert_eq!(settings.initial_again_days, 1);
        assert_eq!(settings.initial_difficult_days, 2);
        assert_eq!(settings.initial_good_days, 5);
        assert_eq!(settings.initial_easy_days, 10);
        assert_eq!(next_interval_days(&settings, 1, "again"), 1);
        assert_eq!(next_interval_days(&settings, 1, "difficult"), 2);
        assert_eq!(next_interval_days(&settings, 1, "good"), 5);
        assert_eq!(next_interval_days(&settings, 1, "easy"), 10);
        assert_eq!(next_interval_days(&settings, 10, "good"), 15);
        assert_eq!(next_interval_days(&settings, 20, "easy"), 30);

        assert_eq!(next_mastery_estimate(0.0, 0, "again"), 0.0);
        assert_eq!(next_mastery_estimate(0.0, 0, "difficult"), 35.0);
        assert_eq!(next_mastery_estimate(0.0, 0, "good"), 70.0);
        assert_eq!(next_mastery_estimate(0.0, 0, "easy"), 95.0);
        assert_eq!(next_mastery_estimate(70.0, 3, "good"), 70.0);
        assert_eq!(next_mastery_estimate(70.0, 3, "easy"), 77.5);

        let mut capped = settings.clone();
        capped.minimum_interval_days = 7;
        capped.maximum_interval_days = 20;
        capped.initial_again_days = 7;
        assert_eq!(next_interval_days(&capped, 18, "good"), 20);
        assert_eq!(next_interval_days(&capped, 17, "again"), 7);
        capped.growth_multiplier = 2.0;
        assert_eq!(next_interval_days(&capped, 6, "good"), 12);
    }

    #[test]
    fn weekend_and_calendar_boundary_scheduling_is_safe() {
        assert_eq!(
            schedule_next_date(date(2026, 10, 30), 1, true).expect("date"),
            date(2026, 10, 31)
        );
        assert_eq!(
            schedule_next_date(date(2026, 10, 30), 1, false).expect("date"),
            date(2026, 11, 2)
        );
        assert_eq!(
            schedule_next_date(date(2026, 10, 31), 1, false).expect("date"),
            date(2026, 11, 2)
        );
        assert_eq!(
            schedule_next_date(date(2024, 2, 28), 1, true).expect("date"),
            date(2024, 2, 29)
        );
        assert_eq!(
            schedule_next_date(date(2024, 2, 29), 1, true).expect("date"),
            date(2024, 3, 1)
        );
        assert_eq!(
            schedule_next_date(date(2026, 12, 31), 1, true).expect("date"),
            date(2027, 1, 1)
        );
        assert!(schedule_next_date(date(9999, 12, 31), 1, true).is_err());
    }

    #[test]
    fn schedule_lifecycle_completion_history_and_status_controls() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);

        let scheduled =
            schedule_topic(&mut connection, topic, "2026-10-30", today).expect("schedule");
        assert_eq!(scheduled.due_state, "due_today");
        assert_eq!(
            scheduled
                .schedule
                .as_ref()
                .expect("schedule")
                .completed_revisions,
            0
        );

        let duplicate = schedule_topic(&mut connection, topic, "2026-11-20", today)
            .expect("duplicate schedule should be idempotent");
        assert_eq!(
            duplicate
                .schedule
                .as_ref()
                .expect("schedule")
                .next_revision_date,
            "2026-10-30"
        );
        let schedule_count: i64 = connection
            .query_row("SELECT count(*) FROM revision_schedules", [], |row| {
                row.get(0)
            })
            .expect("schedule count");
        assert_eq!(schedule_count, 1);

        let first = complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "again".to_owned(),
                notes: "Could not recall the derivation".to_owned(),
            },
            today,
        )
        .expect("first completion");
        assert_eq!(
            first
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .completed_revisions,
            1
        );
        assert_eq!(
            first
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .current_interval_days,
            1
        );
        assert_eq!(
            first
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .next_revision_date,
            "2026-10-31"
        );
        assert_eq!(
            first
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .mastery_estimate,
            0.0
        );
        assert_eq!(first.history.outcome, "again");
        assert_eq!(first.history.scheduled_for_date, "2026-10-30");

        let second = complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "difficult".to_owned(),
                notes: "Partial recall".to_owned(),
            },
            date(2026, 10, 31),
        )
        .expect("second completion");
        assert_eq!(
            second
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .current_interval_days,
            2
        );
        assert_eq!(
            second
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .mastery_estimate,
            10.5
        );

        let third = complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            date(2026, 11, 2),
        )
        .expect("third completion");
        assert_eq!(
            third
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .current_interval_days,
            5
        );
        assert_eq!(
            third
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .mastery_estimate,
            28.4
        );

        let fourth = complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "easy".to_owned(),
                notes: String::new(),
            },
            date(2026, 11, 7),
        )
        .expect("fourth completion");
        assert_eq!(
            fourth
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .current_interval_days,
            10
        );
        assert_eq!(
            fourth
                .state
                .schedule
                .as_ref()
                .expect("schedule")
                .mastery_estimate,
            48.4
        );

        let history = history(&connection, topic).expect("history should load");
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].outcome, "easy");
        assert_eq!(history[3].outcome, "again");

        let notes = update_notes(&mut connection, topic, "Keep this note", today)
            .expect("notes should update");
        assert_eq!(
            notes.schedule.as_ref().expect("schedule").notes,
            "Keep this note"
        );

        let rescheduled =
            reschedule(&mut connection, topic, "2027-01-15", today).expect("reschedule");
        assert_eq!(
            rescheduled
                .schedule
                .as_ref()
                .expect("schedule")
                .next_revision_date,
            "2027-01-15"
        );

        let suspended = set_status(&mut connection, topic, "suspended", today).expect("suspend");
        assert_eq!(suspended.due_state, "suspended");
        assert_eq!(
            complete(
                &mut connection,
                topic,
                CompleteRevisionInput {
                    plan_item_id: None,
                    outcome: "good".to_owned(),
                    notes: String::new(),
                },
                today,
            )
            .expect_err("suspended schedule should reject completion")
            .code,
            "revision_suspended"
        );

        let active = set_status(&mut connection, topic, "active", today).expect("reactivate");
        assert_eq!(active.due_state, "upcoming");

        assert!(
            connection
                .execute(
                    "UPDATE revision_history SET notes = 'changed' WHERE id = ?1",
                    [history[0].id]
                )
                .is_err()
        );
        assert!(
            connection
                .execute(
                    "DELETE FROM revision_history WHERE id = ?1",
                    [history[0].id]
                )
                .is_err()
        );

        let progress = syllabus::load_progress(&connection, topic).expect("progress");
        assert_eq!(progress.status, "Not started");
    }

    #[test]
    fn completion_rolls_back_when_history_append_fails() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-10-30", today).expect("schedule");
        connection
            .execute_batch(
                "CREATE TRIGGER fail_revision_append
                 BEFORE INSERT ON revision_history
                 BEGIN
                     SELECT RAISE(ABORT, 'forced history failure');
                 END;",
            )
            .expect("trigger should install");

        let error = complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today,
        )
        .expect_err("completion should fail");
        assert_eq!(error.code, "database_error");

        let schedule = load_schedule(&connection, topic)
            .expect("schedule should load")
            .expect("schedule should exist");
        assert_eq!(schedule.completed_revisions, 0);
        assert_eq!(schedule.next_revision_date, "2026-10-30");
        let history_count: i64 = connection
            .query_row("SELECT count(*) FROM revision_history", [], |row| {
                row.get(0)
            })
            .expect("history count");
        assert_eq!(history_count, 0);
    }

    #[test]
    fn recommendations_use_due_dates_previous_outcomes_and_qualified_pyq_data() {
        let mut connection = connection();
        let weak = topic_id(&connection, "MMP-C01");
        let insufficient = topic_id(&connection, "GA-01");
        let unscheduled_weak = topic_id(&connection, "CM-C01");
        let today = date(2026, 10, 30);

        schedule_topic(&mut connection, weak, "2026-10-29", today).expect("weak schedule");
        schedule_topic(&mut connection, insufficient, "2026-10-30", today)
            .expect("insufficient schedule");

        for (index, status) in [
            "correct",
            "incorrect",
            "incorrect",
            "incorrect",
            "incorrect",
            "incorrect",
        ]
        .into_iter()
        .enumerate()
        {
            pyq::create(
                &mut connection,
                pyq_input(weak, status, &format!("WEAK-{index}")),
            )
            .expect("PYQ");
        }
        pyq::create(
            &mut connection,
            pyq_input(insufficient, "incorrect", "LOW-1"),
        )
        .expect("PYQ");
        pyq::create(
            &mut connection,
            pyq_input(insufficient, "incorrect", "LOW-2"),
        )
        .expect("PYQ");
        for index in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input(
                    unscheduled_weak,
                    "incorrect",
                    &format!("UNSCHEDULED-{index}"),
                ),
            )
            .expect("PYQ");
        }
        let stale = topic_id(&connection, "EMT-C01");
        for index in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input_on(stale, "incorrect", &format!("STALE-{index}"), "2019-06-01"),
            )
            .expect("stale PYQ should save in history");
        }
        mock_tests::create(&mut connection, minimal_mock_input()).expect("mock should save");

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        assert_eq!(dashboard.stats.overdue_count, 1);
        assert_eq!(dashboard.stats.due_today_count, 1);
        assert_eq!(dashboard.mock_test_count, 1);
        assert_eq!(dashboard.mock_scope_note, MOCK_SCOPE_NOTE);

        let weak_recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.code == "MMP-C01")
            .expect("weak recommendation");
        assert_eq!(weak_recommendation.lane, "urgent_review");
        assert_eq!(weak_recommendation.evidence_status, "weak");
        assert!(
            weak_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("Overdue by"))
        );
        assert!(
            weak_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );

        let insufficient_recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.code == "GA-01")
            .expect("insufficient recommendation");
        assert_eq!(insufficient_recommendation.evidence_status, "insufficient");
        assert!(
            insufficient_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("at least 5 are required"))
        );
        assert!(
            !insufficient_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );

        let unscheduled = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.code == "CM-C01")
            .expect("unscheduled weak recommendation");
        assert!(!unscheduled.scheduled);
        assert_eq!(unscheduled.due_state, "not_scheduled");
        assert_eq!(unscheduled.lane, "strengthen");
        assert_eq!(unscheduled.days_overdue, 0);
        assert_eq!(unscheduled.due_urgency, 0.0);
        assert!(
            !dashboard
                .recommendations
                .iter()
                .any(|item| item.topic.code == "EMT-C01"),
            "stale-only PYQ evidence must not create an unscheduled recommendation"
        );
    }

    #[test]
    fn high_yield_weak_topic_outranks_low_priority_strong_topic() {
        let high_yield = recommendation_topic(1, "AAA-HY", "High-Yield", 3, "Learning");
        let low_priority = recommendation_topic(2, "ZZZ-LOW", "Low", 3, "Learning");
        let schedule = || Some(recommendation_schedule("2026-11-09", Some("good"), 70.0, 3));
        let states = vec![
            recommendation_state(low_priority, schedule(), "upcoming", 0, Some(10)),
            recommendation_state(high_yield, schedule(), "upcoming", 0, Some(10)),
        ];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 6, 20.0, false, 6, 6));
        pyq.insert(2, recommendation_pyq(2, 6, 90.0, false, 6, 6));

        let result = recommendations(&states, &pyq);

        assert_eq!(result[0].topic.code, "AAA-HY");
        assert_eq!(result[0].lane, "strengthen");
        assert_eq!(result[0].evidence_status, "weak");
        assert_eq!(result[1].topic.code, "ZZZ-LOW");
        assert_eq!(result[1].lane, "maintain");
        assert!(result[0].triage_score > result[1].triage_score);
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );
        assert!(
            !result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("insufficient recent PYQ evidence"))
        );
    }

    #[test]
    fn high_priority_unscheduled_insufficient_evidence_strengthens_without_due_state() {
        let topic = recommendation_topic(1, "UNSCHED", "High-Yield", 3, "Learning");
        let states = vec![recommendation_state(topic, None, "not_scheduled", 0, None)];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 2, 0.0, false, 2, 2));

        let result = recommendations(&states, &pyq);

        assert_eq!(result.len(), 1);
        let recommendation = &result[0];
        assert_eq!(recommendation.lane, "strengthen");
        assert!(!recommendation.scheduled);
        assert_eq!(recommendation.due_state, "not_scheduled");
        assert_eq!(recommendation.days_overdue, 0);
        assert_eq!(recommendation.due_urgency, 0.0);
        assert_eq!(recommendation.next_revision_date, None);
        assert_eq!(recommendation.triage_score, 16.0);
        assert!(
            recommendation
                .reasons
                .iter()
                .any(|reason| reason == "Not scheduled.")
        );
        assert!(
            recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("High-priority topic has insufficient"))
        );
        assert!(
            !recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("Overdue") || reason.contains("Due today"))
        );
    }

    #[test]
    fn confidence_one_has_more_learning_need_than_confidence_five() {
        assert_eq!(confidence_need(1), 100.0);
        assert_eq!(confidence_need(5), 0.0);

        let low_confidence = recommendation_topic(1, "AAA-CONF1", "Normal", 1, "Revised");
        let high_confidence = recommendation_topic(2, "ZZZ-CONF5", "Normal", 5, "Revised");
        let schedule = || Some(recommendation_schedule("2026-11-09", Some("good"), 70.0, 3));
        let states = vec![
            recommendation_state(high_confidence, schedule(), "upcoming", 0, Some(10)),
            recommendation_state(low_confidence, schedule(), "upcoming", 0, Some(10)),
        ];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 6, 90.0, false, 6, 6));
        pyq.insert(2, recommendation_pyq(2, 6, 90.0, false, 6, 6));

        let result = recommendations(&states, &pyq);

        assert_eq!(result[0].topic.confidence, 1);
        assert_eq!(result[1].topic.confidence, 5);
        assert_eq!(result[0].confidence_status_need, 80.0);
        assert_eq!(result[1].confidence_status_need, 20.0);
        assert!((result[0].triage_score - result[1].triage_score - 6.0).abs() < 0.001);
    }

    #[test]
    fn status_values_produce_documented_need_scores() {
        assert_eq!(status_need("Not started"), 100.0);
        assert_eq!(status_need("Learning"), 75.0);
        assert_eq!(status_need("Revised"), 50.0);
        assert_eq!(status_need("Confident"), 25.0);
        assert_eq!(status_need("Unexpected"), 50.0);
    }

    #[test]
    fn urgency_boundaries_and_unscheduled_states_are_stable() {
        let topic = recommendation_topic(1, "BOUNDARY", "Normal", 3, "Learning");
        let scheduled = || Some(recommendation_schedule("2026-11-01", Some("good"), 70.0, 3));

        let due_today = recommendation_state(topic.clone(), scheduled(), "due_today", 0, Some(0));
        assert_eq!(due_urgency(&due_today), 80.0);
        assert!(urgent_lane(&due_today));

        let within_seven = recommendation_state(topic.clone(), scheduled(), "upcoming", 0, Some(7));
        assert_eq!(due_urgency(&within_seven), 56.0);
        assert!(urgent_lane(&within_seven));

        let eight_days = recommendation_state(topic.clone(), scheduled(), "upcoming", 0, Some(8));
        assert_eq!(due_urgency(&eight_days), 54.0);
        assert!(!urgent_lane(&eight_days));

        let unscheduled = recommendation_state(topic, None, "not_scheduled", 0, None);
        assert_eq!(due_urgency(&unscheduled), 0.0);
        assert!(!urgent_lane(&unscheduled));
    }

    #[test]
    fn revision_loader_defaults_missing_progress_without_duplicate_topics() {
        let connection = connection();
        let expected: i64 = connection
            .query_row(
                "SELECT count(*) FROM syllabus_topics WHERE is_archived = 0",
                [],
                |row| row.get(0),
            )
            .expect("active topic count should load");
        let topics = load_topics(&connection).expect("active topics should load");
        let ids = topics
            .iter()
            .map(|topic| topic.id)
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(topics.len() as i64, expected);
        assert_eq!(ids.len(), topics.len());

        let topic = topics
            .iter()
            .find(|topic| topic.code == "MMP-C01")
            .expect("seeded topic should be present");
        assert_eq!(topic.priority, "Normal");
        assert_eq!(topic.confidence, 3);
        assert_eq!(topic.syllabus_status, "Not started");
        assert!(!topic.category.is_empty());
    }

    #[test]
    fn archived_loader_returns_one_row_per_topic_with_metadata() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-11-20", today).expect("schedule");
        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [topic],
            )
            .expect("topic should archive");

        let archived = load_archived_topics(&connection).expect("archived topics should load");
        let active = load_topics(&connection).expect("active topics should load");
        let matches = archived
            .iter()
            .filter(|entry| entry.id == topic)
            .collect::<Vec<_>>();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].priority, "Normal");
        assert_eq!(matches[0].confidence, 3);
        assert_eq!(matches[0].syllabus_status, "Not started");
        assert!(!active.iter().any(|entry| entry.id == topic));
    }

    #[test]
    fn revision_loader_normalizes_invalid_progress_metadata() {
        let connection = connection();
        let topic = topic_id(&connection, "MMP-C01");

        connection
            .execute_batch("PRAGMA ignore_check_constraints = ON;")
            .expect("test database should allow invalid fixture metadata");
        connection
            .execute(
                "INSERT INTO syllabus_progress (
                    topic_id, status, priority, confidence, notes, created_at, updated_at
                 ) VALUES (?1, 'Bogus', 'Urgent', 9, '', '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z')",
                [topic],
            )
            .expect("invalid fixture metadata should be inserted for normalization testing");

        let loaded = load_topic(&connection, topic).expect("topic metadata should load");

        assert_eq!(loaded.priority, "Normal");
        assert_eq!(loaded.confidence, 3);
        assert_eq!(loaded.syllabus_status, "Not started");
        assert!(!loaded.category.is_empty());
    }

    #[test]
    fn five_distinct_weak_questions_qualify_for_weak_evidence() {
        let stats = recommendation_pyq(1, 5, 20.0, false, 5, 5);

        let assessment = evidence_assessment(Some(&stats));

        assert_eq!(assessment.class, EvidenceClass::Weak);
        assert_eq!(assessment.need, 50.0);
        assert!(assessment.reason.contains("weak"));
        assert!(assessment.reason.contains("5 distinct questions"));
    }

    #[test]
    fn fewer_than_five_questions_remain_insufficient_not_weak() {
        let stats = recommendation_pyq(1, 4, 0.0, false, 4, 4);

        let assessment = evidence_assessment(Some(&stats));

        assert_eq!(assessment.class, EvidenceClass::Insufficient);
        assert_eq!(assessment.need, 0.0);
        assert!(assessment.reason.contains("at least 5 are required"));
        assert!(!assessment.reason.contains("is weak"));
    }

    #[test]
    fn evidence_need_caps_at_ten_questions() {
        for questions in [10, 15] {
            let stats = recommendation_pyq(1, questions, 20.0, false, questions, questions);
            let assessment = evidence_assessment(Some(&stats));

            assert_eq!(assessment.class, EvidenceClass::Weak);
            assert_eq!(assessment.need, 100.0);
            assert!(assessment.reason.contains("weak"));
        }
    }

    #[test]
    fn moderate_unscored_and_none_evidence_are_classified_neutrally() {
        let moderate = recommendation_pyq(1, 6, 60.0, false, 6, 6);
        let moderate_assessment = evidence_assessment(Some(&moderate));
        assert_eq!(moderate_assessment.class, EvidenceClass::Moderate);
        assert_eq!(moderate_assessment.need, 0.0);
        assert!(moderate_assessment.reason.contains("60.0%"));

        let unscored = recommendation_pyq(2, 0, 0.0, false, 0, 2);
        let unscored_assessment = evidence_assessment(Some(&unscored));
        assert_eq!(unscored_assessment.class, EvidenceClass::Unscored);
        assert_eq!(unscored_assessment.need, 0.0);
        assert!(unscored_assessment.reason.contains("without a scored"));

        let no_stats = evidence_assessment(None);
        assert_eq!(no_stats.class, EvidenceClass::None);
        assert_eq!(no_stats.need, 0.0);

        let no_attempts = recommendation_pyq(3, 0, 0.0, false, 0, 0);
        let none_assessment = evidence_assessment(Some(&no_attempts));
        assert_eq!(none_assessment.class, EvidenceClass::None);
        assert_eq!(none_assessment.need, 0.0);
    }

    #[test]
    fn repeated_attempts_on_one_question_do_not_inflate_revision_evidence() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-11-20", today).expect("schedule");

        for _ in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input(topic, "incorrect", "SAME-QUESTION"),
            )
            .expect("PYQ should save");
        }

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        let recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.id == topic)
            .expect("scheduled topic should be recommended");

        assert_eq!(recommendation.pyq_attempts, 6);
        assert_eq!(recommendation.pyq_questions, 1);
        assert_eq!(recommendation.evidence_status, "insufficient");
        assert!(!recommendation.pyq_sufficient);
        assert_eq!(recommendation.pyq_evidence_need, 0.0);
    }

    #[test]
    fn stale_only_evidence_is_neutral_and_explicitly_labelled() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-11-20", today).expect("schedule");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_on(topic, "incorrect", &format!("STALE-{index}"), "2019-06-01"),
            )
            .expect("stale PYQ should remain in history");
        }

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        let recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.id == topic)
            .expect("scheduled topic should be recommended");

        assert_eq!(recommendation.evidence_status, "stale");
        assert_eq!(recommendation.pyq_questions, 0);
        assert_eq!(recommendation.pyq_evidence_need, 0.0);
        assert_eq!(recommendation.lane, "maintain");
        assert!(recommendation.reasons.iter().any(|reason| {
            reason == "No scored PYQ evidence is within the last 365 calendar days."
        }));
        assert!(
            !recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );
        assert!(
            !recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("recorded attempts fall outside"))
        );
    }

    #[test]
    fn stale_wording_stays_neutral_for_mixed_and_future_attempts() {
        let mut connection = connection();
        let mixed = topic_id(&connection, "MMP-C01");
        let future = topic_id(&connection, "GA-01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, mixed, "2026-11-20", today).expect("mixed schedule");
        schedule_topic(&mut connection, future, "2026-11-20", today).expect("future schedule");

        pyq::create(
            &mut connection,
            pyq_input_on(mixed, "incorrect", "OLD-SCORED", "2019-06-01"),
        )
        .expect("old scored PYQ should remain in history");
        pyq::create(
            &mut connection,
            pyq_input(mixed, "skipped", "RECENT-SKIPPED"),
        )
        .expect("recent skipped PYQ should save");

        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq_input_on(future, "correct", &format!("FUTURE-{index}"), "2027-01-01"),
            )
            .expect("future PYQ should remain in history");
        }

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        for topic in [mixed, future] {
            let recommendation = dashboard
                .recommendations
                .iter()
                .find(|item| item.topic.id == topic)
                .expect("scheduled topic should be recommended");

            assert_eq!(recommendation.evidence_status, "stale");
            assert_eq!(recommendation.pyq_questions, 0);
            assert_eq!(recommendation.pyq_evidence_need, 0.0);
            assert!(
                recommendation.reasons.iter().any(|reason| reason
                    == "No scored PYQ evidence is within the last 365 calendar days.")
            );
            assert!(
                !recommendation
                    .reasons
                    .iter()
                    .any(|reason| reason.contains("recorded attempts fall outside"))
            );
        }
    }

    #[test]
    fn skipped_only_evidence_is_neutral_and_explicitly_labelled() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-11-20", today).expect("schedule");

        for index in 0..3 {
            pyq::create(
                &mut connection,
                pyq_input(topic, "skipped", &format!("SKIPPED-{index}")),
            )
            .expect("skipped PYQ should save");
        }

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        let recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.id == topic)
            .expect("scheduled topic should be recommended");

        assert_eq!(recommendation.evidence_status, "skipped_only");
        assert_eq!(recommendation.pyq_questions, 0);
        assert_eq!(recommendation.pyq_evidence_need, 0.0);
        assert_eq!(recommendation.lane, "maintain");
        assert!(
            recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("skipped-only"))
        );
        assert!(
            !recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );
    }

    #[test]
    fn urgent_lane_takes_precedence_and_preserves_additional_reasons() {
        let topic = recommendation_topic(1, "URGENT", "High-Yield", 1, "Learning");
        let states = vec![recommendation_state(
            topic,
            Some(recommendation_schedule(
                "2026-10-22",
                Some("difficult"),
                35.0,
                3,
            )),
            "overdue",
            8,
            None,
        )];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 6, 20.0, false, 6, 6));

        let result = recommendations(&states, &pyq);

        assert_eq!(result[0].lane, "urgent_review");
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("Overdue by 8 days"))
        );
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy is weak"))
        );
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("Confidence marked 1/5"))
        );
    }

    #[test]
    fn strong_evidence_and_high_mastery_land_in_maintain() {
        let topic = recommendation_topic(1, "MAINTAIN", "Normal", 4, "Revised");
        let states = vec![recommendation_state(
            topic,
            Some(recommendation_schedule("2026-11-19", Some("good"), 90.0, 5)),
            "upcoming",
            0,
            Some(20),
        )];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 8, 90.0, false, 8, 8));

        let result = recommendations(&states, &pyq);

        assert_eq!(result[0].lane, "maintain");
        assert_eq!(result[0].evidence_status, "strong");
        assert_eq!(result[0].pyq_evidence_need, 0.0);
        assert!(
            result[0]
                .reasons
                .iter()
                .any(|reason| reason == "Strong evidence and high revision mastery.")
        );
    }

    #[test]
    fn fallback_maintain_uses_neutral_reason_for_stale_low_mastery() {
        let topic = recommendation_topic(1, "FALLBACK", "Normal", 4, "Revised");
        let states = vec![recommendation_state(
            topic,
            Some(recommendation_schedule("2026-11-19", Some("good"), 20.0, 3)),
            "upcoming",
            0,
            Some(20),
        )];
        let mut pyq = HashMap::new();
        pyq.insert(1, recommendation_pyq(1, 0, 0.0, false, 5, 5));

        let result = recommendations(&states, &pyq);

        assert_eq!(result[0].lane, "maintain");
        assert_eq!(result[0].evidence_status, "stale");
        assert!(result[0].reasons.iter().any(|reason| {
            reason == "Scheduled review; no stronger learning-risk signal was identified."
        }));
        assert!(
            !result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("maintain retention"))
        );
        assert!(
            !result[0]
                .reasons
                .iter()
                .any(|reason| reason.contains("Strong evidence"))
        );
    }

    #[test]
    fn archived_topics_never_enter_active_recommendations_even_when_high_risk() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        set_topic_progress(&mut connection, "MMP-C01", "Learning", "High-Yield", 1);
        schedule_topic(&mut connection, topic, "2026-10-30", today).expect("schedule");

        for index in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input(topic, "incorrect", &format!("ARCHIVED-{index}")),
            )
            .expect("PYQ should save before archiving");
        }

        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [topic],
            )
            .expect("topic should archive");

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        assert!(
            dashboard
                .archived_topics
                .iter()
                .any(|state| state.topic.id == topic)
        );
        assert!(
            !dashboard
                .recommendations
                .iter()
                .any(|recommendation| recommendation.topic.id == topic)
        );
    }

    #[test]
    fn equal_scores_use_deterministic_tie_breaks() {
        let no_pyq = HashMap::new();
        let schedule = || Some(recommendation_schedule("2026-10-30", Some("good"), 70.0, 3));

        let overdue_late = recommendation_topic(1, "ZZZ-OVERDUE", "Normal", 3, "Learning");
        let overdue_early = recommendation_topic(2, "AAA-OVERDUE", "Normal", 3, "Learning");
        let by_overdue = recommendations(
            &[
                recommendation_state(overdue_early, schedule(), "overdue", 2, None),
                recommendation_state(overdue_late, schedule(), "overdue", 8, None),
            ],
            &no_pyq,
        );
        assert_eq!(by_overdue[0].topic.code, "ZZZ-OVERDUE");
        assert_eq!(by_overdue[0].days_overdue, 8);

        let high_priority = recommendation_topic(3, "ZZZ-HIGH", "High", 5, "Confident");
        let normal_priority = recommendation_topic(4, "AAA-NORMAL", "Normal", 4, "Revised");
        let by_priority = recommendations(
            &[
                recommendation_state(normal_priority, schedule(), "upcoming", 0, Some(10)),
                recommendation_state(high_priority, schedule(), "upcoming", 0, Some(10)),
            ],
            &no_pyq,
        );
        assert_eq!(by_priority[0].triage_score, by_priority[1].triage_score);
        assert_eq!(by_priority[0].topic.code, "ZZZ-HIGH");

        let earlier_due = recommendation_topic(5, "ZZZ-EARLY", "Normal", 3, "Learning");
        let later_due = recommendation_topic(6, "AAA-LATE", "Normal", 3, "Learning");
        let by_due_date = recommendations(
            &[
                recommendation_state(
                    later_due,
                    Some(recommendation_schedule("2026-11-05", Some("good"), 70.0, 3)),
                    "upcoming",
                    0,
                    Some(10),
                ),
                recommendation_state(
                    earlier_due,
                    Some(recommendation_schedule("2026-11-01", Some("good"), 70.0, 3)),
                    "upcoming",
                    0,
                    Some(10),
                ),
            ],
            &no_pyq,
        );
        assert_eq!(by_due_date[0].topic.code, "ZZZ-EARLY");

        let code_a = recommendation_topic(7, "AAA-CODE", "Normal", 3, "Learning");
        let code_b = recommendation_topic(8, "ZZZ-CODE", "Normal", 3, "Learning");
        let by_code = recommendations(
            &[
                recommendation_state(code_b, schedule(), "upcoming", 0, Some(10)),
                recommendation_state(code_a, schedule(), "upcoming", 0, Some(10)),
            ],
            &no_pyq,
        );
        assert_eq!(by_code[0].topic.code, "AAA-CODE");
    }

    #[test]
    fn recommendations_are_capped_at_eight_in_code_order() {
        let mut states = Vec::new();
        for index in 1..=10 {
            let code = format!("TST-{index:02}");
            let topic = recommendation_topic(index, &code, "Normal", 3, "Learning");
            states.push(recommendation_state(
                topic,
                Some(recommendation_schedule("2026-10-30", Some("good"), 70.0, 3)),
                "due_today",
                0,
                Some(0),
            ));
        }

        let result = recommendations(&states, &HashMap::new());
        let codes = result
            .iter()
            .map(|recommendation| recommendation.topic.code.as_str())
            .collect::<Vec<_>>();

        assert_eq!(result.len(), MAX_RECOMMENDATIONS);
        assert_eq!(
            codes,
            vec![
                "TST-01", "TST-02", "TST-03", "TST-04", "TST-05", "TST-06", "TST-07", "TST-08"
            ]
        );
    }

    #[test]
    fn archived_topics_leave_active_queues_but_keep_revision_history() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);

        schedule_topic(&mut connection, topic, "2026-10-30", today).expect("schedule");
        complete(
            &mut connection,
            topic,
            CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: "Completed before archiving".to_owned(),
            },
            today,
        )
        .expect("revision should complete");

        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [topic],
            )
            .expect("topic should archive");

        let dashboard = dashboard(&connection, today).expect("dashboard should load");
        assert!(!dashboard.topics.iter().any(|state| state.topic.id == topic));
        let archived = dashboard
            .archived_topics
            .iter()
            .find(|state| state.topic.id == topic)
            .expect("archived history should remain available");
        assert!(archived.topic.archived);
        assert!(archived.schedule.is_some());
        assert!(
            !dashboard
                .recommendations
                .iter()
                .any(|recommendation| recommendation.topic.id == topic)
        );
        let calendar = calendar_items(&connection, "2026-10-30", "2027-12-31")
            .expect("calendar items should load");
        assert!(!calendar.iter().any(|item| item.topic_id == topic));

        let history = history(&connection, topic).expect("history should remain readable");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].outcome, "good");

        for error in [
            schedule_topic(&mut connection, topic, "2026-12-01", today)
                .expect_err("archived schedules must be read-only"),
            reschedule(&mut connection, topic, "2026-12-02", today)
                .expect_err("archived schedules must not be rescheduled"),
            update_notes(&mut connection, topic, "Do not save", today)
                .expect_err("archived notes must be read-only"),
            set_status(&mut connection, topic, "suspended", today)
                .expect_err("archived status must be read-only"),
            complete(
                &mut connection,
                topic,
                CompleteRevisionInput {
                    plan_item_id: None,
                    outcome: "good".to_owned(),
                    notes: "Do not save".to_owned(),
                },
                today,
            )
            .expect_err("archived revisions must not be completed"),
        ] {
            assert_eq!(error.field, Some("topicId"));
            assert!(error.message.contains("read-only"));
        }
    }

    #[test]
    fn calendar_and_planner_show_revision_dates_without_duplicates() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let today = date(2026, 10, 30);
        schedule_topic(&mut connection, topic, "2026-11-05", today).expect("schedule");
        planner::create_block(
            &mut connection,
            planner::PlannedBlockInput {
                title: "Revision study block".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-11-05".to_owned(),
                start_time: "10:00".to_owned(),
                end_time: "11:00".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("block should save");

        let calendar =
            planner::calendar(&connection, "2026-11-01", "2026-11-10", today).expect("calendar");
        assert_eq!(calendar.revisions.len(), 1);
        assert_eq!(calendar.blocks.len(), 1);
        assert_eq!(calendar.revisions[0].topic_code, "MMP-C01");

        let summaries = summaries(&connection, today).expect("summaries should load");
        let summary = summaries
            .iter()
            .find(|summary| summary.topic_id == topic)
            .expect("summary should exist");
        assert_eq!(summary.next_revision_date.as_deref(), Some("2026-11-05"));

        reschedule(&mut connection, topic, "2026-11-06", today).expect("reschedule");
        assert!(
            planner::calendar(&connection, "2026-11-05", "2026-11-05", today)
                .expect("old calendar")
                .revisions
                .is_empty()
        );
        let new_calendar = planner::calendar(&connection, "2026-11-06", "2026-11-06", today)
            .expect("new calendar");
        assert_eq!(new_calendar.revisions.len(), 1);
    }

    #[test]
    fn settings_validation_and_reminders_are_safe_and_optional() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let topic = topic_id(&connection, "MMP-C01");
        schedule_topic(&mut connection, topic, "2026-10-30", today).expect("schedule");

        let mut settings = load_settings(&connection).expect("settings");
        assert!(!settings.reminders_enabled);
        let mut invalid = settings_input(&settings);
        invalid.minimum_interval_days = 30;
        invalid.maximum_interval_days = 10;
        assert_eq!(
            update_settings(&mut connection, invalid)
                .expect_err("min > max should fail")
                .field,
            Some("minimumIntervalDays")
        );

        let mut invalid = settings_input(&settings);
        invalid.initial_good_days = 0;
        assert_eq!(
            update_settings(&mut connection, invalid)
                .expect_err("zero interval should fail")
                .field,
            Some("initialGoodDays")
        );

        let mut invalid = settings_input(&settings);
        invalid.growth_multiplier = 0.5;
        assert_eq!(
            update_settings(&mut connection, invalid)
                .expect_err("growth should fail")
                .field,
            Some("growthMultiplier")
        );

        assert!(
            prepare_reminder(&mut connection, today)
                .expect("reminder")
                .is_none()
        );
        settings.reminders_enabled = true;
        update_settings(&mut connection, settings_input(&settings)).expect("settings update");
        let reminder = prepare_reminder(&mut connection, today)
            .expect("reminder should load")
            .expect("due reminder");
        assert_eq!(reminder.due_today, 1);
        assert_eq!(reminder.overdue, 0);
        assert!(
            prepare_reminder(&mut connection, today)
                .expect("reminder")
                .is_none()
        );

        let next_day = prepare_reminder(&mut connection, date(2026, 10, 31))
            .expect("reminder should load")
            .expect("overdue reminder");
        assert_eq!(next_day.overdue, 1);
        assert_eq!(next_day.total, 1);
    }
}
