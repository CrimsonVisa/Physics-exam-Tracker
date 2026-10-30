use std::collections::HashMap;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{MINIMUM_WEAK_ATTEMPTS, WEAK_ACCURACY_THRESHOLD},
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

const SELECT_SCHEDULE_FIELDS: &str = "status, last_revision_date, next_revision_date, \
    current_interval_days, completed_revisions, previous_outcome, mastery_estimate, \
    notes, updated_at";

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
    pub pyq_accuracy: Option<f64>,
    pub pyq_sufficient: bool,
    pub priority: String,
    pub score: i64,
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionCompletion {
    pub state: RevisionTopicState,
    pub history: RevisionHistoryEntry,
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
                    reminders_enabled: row.get::<_, i64>(9)? != 0,
                    updated_at: row.get(10)?,
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
                 reminders_enabled = ?10,
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

    Ok(())
}

pub fn dashboard(connection: &Connection, today: NaiveDate) -> Result<RevisionDashboard, AppError> {
    let settings = load_settings(connection)?;
    let topics = topic_states(connection, today)?;
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
    let pyq_performance = pyq_topic_performance(connection)?;
    let recommendations = recommendations(&topics, &pyq_performance);
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
                id,
                topic_id,
                revision_date,
                scheduled_for_date,
                outcome,
                notes,
                interval_before_days,
                interval_after_days,
                mastery_before,
                mastery_after,
                created_at
             FROM revision_history
             WHERE topic_id = ?1
             ORDER BY revision_date DESC, id DESC",
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

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision-completion transaction could not be started",
            error,
        )
    })?;
    let settings = load_settings(&transaction)?;
    let topic = load_topic(&transaction, topic_id)?;
    let schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    if schedule.status == "suspended" {
        return Err(AppError::revision_suspended());
    }

    let interval_before = schedule.current_interval_days;
    let mastery_before = schedule.mastery_estimate;
    let interval_after = next_interval_days(&settings, interval_before, &outcome);
    let mastery_after =
        next_mastery_estimate(mastery_before, schedule.completed_revisions, &outcome);
    let next_date = schedule_next_date(today, interval_after, settings.include_weekends)?;
    let next_text = date_only::format_date_only(next_date);
    let today_text = date_only::format_date_only(today);

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
    let history = load_history_entry(&transaction, history_id)?;
    let updated_schedule =
        load_schedule(&transaction, topic_id)?.ok_or_else(AppError::revision_not_scheduled)?;
    let state = build_state(topic, Some(updated_schedule), today)?;
    transaction.commit().map_err(|error| {
        AppError::database("The revision completion could not be committed", error)
    })?;

    Ok(RevisionCompletion { state, history })
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
            "SELECT count(*) FROM revision_schedules
             WHERE status = 'active' AND next_revision_date = ?1",
            [&today_text],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Due revisions could not be counted", error))?;
    let overdue: i64 = connection
        .query_row(
            "SELECT count(*) FROM revision_schedules
             WHERE status = 'active' AND next_revision_date < ?1",
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

pub fn next_interval_days(
    settings: &RevisionSettings,
    current_interval_days: i64,
    outcome: &str,
) -> i64 {
    let base = scheduled_outcome_interval(settings, outcome);
    let candidate = if matches!(outcome, "good" | "easy") {
        let grown = (current_interval_days as f64 * settings.growth_multiplier).round() as i64;
        base.max(grown)
    } else {
        base
    };

    candidate
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

fn topic_states(
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

fn load_topics(connection: &Connection) -> Result<Vec<RevisionTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                t.id,
                t.code,
                t.title,
                t.is_archived,
                s.code,
                s.title
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
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
            })
        })
        .map_err(|error| AppError::database("Revision topics could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Revision topics could not be read", error))
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
                s.title
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
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
        updated_at: row.get(offset + 8)?,
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
                s.title
             FROM revision_history h
             JOIN syllabus_topics t ON t.id = h.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
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
                    history: map_history_at(row, 0)?,
                    topic: RevisionTopic {
                        id: row.get(1)?,
                        code: row.get(11)?,
                        title: row.get(12)?,
                        archived: row.get::<_, i64>(13)? != 0,
                        section_code: row.get(14)?,
                        section_title: row.get(15)?,
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
                id,
                topic_id,
                revision_date,
                scheduled_for_date,
                outcome,
                notes,
                interval_before_days,
                interval_after_days,
                mastery_before,
                mastery_after,
                created_at
             FROM revision_history
             WHERE id = ?1",
            [history_id],
            |row| map_history_at(row, 0),
        )
        .optional()
        .map_err(|error| AppError::database("Revision history could not be loaded", error))?
        .ok_or_else(|| AppError::internal("The revision history entry could not be reloaded."))
}

fn map_history(row: &rusqlite::Row<'_>) -> rusqlite::Result<RevisionHistoryEntry> {
    map_history_at(row, 0)
}

fn map_history_at(
    row: &rusqlite::Row<'_>,
    offset: usize,
) -> rusqlite::Result<RevisionHistoryEntry> {
    Ok(RevisionHistoryEntry {
        id: row.get(offset)?,
        topic_id: row.get(offset + 1)?,
        revision_date: row.get(offset + 2)?,
        scheduled_for_date: row.get(offset + 3)?,
        outcome: row.get(offset + 4)?,
        notes: row.get(offset + 5)?,
        interval_before_days: row.get(offset + 6)?,
        interval_after_days: row.get(offset + 7)?,
        mastery_before: row.get(offset + 8)?,
        mastery_after: row.get(offset + 9)?,
        created_at: row.get(offset + 10)?,
    })
}

fn pyq_topic_performance(
    connection: &Connection,
) -> Result<HashMap<i64, (i64, Option<f64>)>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT
                topic_id,
                COALESCE(SUM(CASE WHEN status = 'correct' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN status = 'incorrect' THEN 1 ELSE 0 END), 0)
             FROM pyq_attempts
             GROUP BY topic_id",
        )
        .map_err(|error| {
            AppError::database("PYQ recommendation data could not be prepared", error)
        })?;

    let rows = statement
        .query_map([], |row| {
            let topic_id: i64 = row.get(0)?;
            let correct: i64 = row.get(1)?;
            let incorrect: i64 = row.get(2)?;
            let attempts = correct + incorrect;
            let accuracy = if attempts == 0 {
                None
            } else {
                Some(correct as f64 / attempts as f64 * 100.0)
            };
            Ok((topic_id, (attempts, accuracy)))
        })
        .map_err(|error| AppError::database("PYQ recommendation data could not be queried", error))?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(|error| AppError::database("PYQ recommendation data could not be read", error))?;

    Ok(rows)
}

fn recommendations(
    states: &[RevisionTopicState],
    pyq: &HashMap<i64, (i64, Option<f64>)>,
) -> Vec<RevisionRecommendation> {
    let mut recommendations = Vec::new();

    for state in states {
        let pyq_stats = pyq.get(&state.topic.id).copied().unwrap_or((0, None));
        let pyq_attempts = pyq_stats.0;
        let pyq_accuracy = pyq_stats.1;
        let pyq_sufficient = pyq_attempts >= MINIMUM_WEAK_ATTEMPTS;
        let weak_unscheduled = state.schedule.is_none()
            && pyq_sufficient
            && pyq_accuracy.is_some_and(|accuracy| accuracy < WEAK_ACCURACY_THRESHOLD);

        if state.schedule.is_none() && !weak_unscheduled {
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

        let mut score = match state.due_state.as_str() {
            "overdue" => 100 + state.days_overdue.min(30),
            "due_today" => 80,
            "upcoming" => 20 - state.days_until_due.unwrap_or(0).min(20),
            "not_scheduled" => 30,
            _ => 0,
        };
        let mut reasons = Vec::new();

        match state.due_state.as_str() {
            "overdue" => reasons.push(format!(
                "{} day{} overdue",
                state.days_overdue,
                if state.days_overdue == 1 { "" } else { "s" }
            )),
            "due_today" => reasons.push("Due today".to_owned()),
            "upcoming" => reasons.push(format!(
                "Due in {} day{}",
                state.days_until_due.unwrap_or(0),
                if state.days_until_due == Some(1) {
                    ""
                } else {
                    "s"
                }
            )),
            "not_scheduled" => reasons.push("Not scheduled yet".to_owned()),
            _ => {}
        }

        if let Some(schedule) = &state.schedule {
            if let Some(outcome) = &schedule.previous_outcome {
                match outcome.as_str() {
                    "again" => {
                        score += 25;
                        reasons.push("Previous outcome was Again".to_owned());
                    }
                    "difficult" => {
                        score += 15;
                        reasons.push("Previous outcome was Difficult".to_owned());
                    }
                    "good" => {
                        score -= 5;
                        reasons.push("Previous outcome was Good".to_owned());
                    }
                    "easy" => {
                        score -= 10;
                        reasons.push("Previous outcome was Easy".to_owned());
                    }
                    _ => {}
                }
            }

            if schedule.completed_revisions > 0 {
                if schedule.mastery_estimate < 40.0 {
                    score += 20;
                } else if schedule.mastery_estimate < 60.0 {
                    score += 10;
                } else if schedule.mastery_estimate >= 80.0 {
                    score -= 5;
                }
                reasons.push(format!(
                    "Mastery estimate {:.1}%",
                    schedule.mastery_estimate
                ));
            }
        }

        if pyq_sufficient {
            let accuracy = pyq_accuracy.unwrap_or(0.0);
            if accuracy < WEAK_ACCURACY_THRESHOLD {
                score += 20;
            } else if accuracy >= 75.0 {
                score -= 10;
            }
            reasons.push(format!(
                "PYQ accuracy {:.1}% over {} scored attempts",
                accuracy, pyq_attempts
            ));
        } else if pyq_attempts > 0 {
            reasons.push(format!(
                "Only {} scored PYQ attempt{}; insufficient evidence for a weak-topic claim",
                pyq_attempts,
                if pyq_attempts == 1 { "" } else { "s" }
            ));
        }

        if state.due_state == "upcoming"
            && score <= 0
            && state.schedule.as_ref().is_some_and(|schedule| {
                schedule.completed_revisions > 0
                    && schedule.mastery_estimate >= 60.0
                    && schedule.previous_outcome.as_deref() != Some("again")
                    && schedule.previous_outcome.as_deref() != Some("difficult")
            })
        {
            continue;
        }

        let priority = if score >= 100 {
            "high"
        } else if score >= 40 {
            "medium"
        } else {
            "low"
        };

        recommendations.push(RevisionRecommendation {
            topic: state.topic.clone(),
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
            pyq_accuracy,
            pyq_sufficient,
            priority: priority.to_owned(),
            score,
            reasons,
        });
    }

    recommendations.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.next_revision_date.cmp(&right.next_revision_date))
            .then_with(|| left.topic.code.cmp(&right.topic.code))
    });
    recommendations.truncate(MAX_RECOMMENDATIONS);
    recommendations
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
            reminders_enabled: settings.reminders_enabled,
        }
    }

    fn pyq_input(topic_id: i64, status: &str, reference: &str) -> pyq::PyqInput {
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

        for status in [
            "correct",
            "incorrect",
            "incorrect",
            "incorrect",
            "incorrect",
            "incorrect",
        ] {
            pyq::create(&mut connection, pyq_input(weak, status, "WEAK")).expect("PYQ");
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
        for _ in 0..6 {
            pyq::create(
                &mut connection,
                pyq_input(unscheduled_weak, "incorrect", "UNSCHEDULED"),
            )
            .expect("PYQ");
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
        assert!(
            weak_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("overdue"))
        );
        assert!(
            weak_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy"))
        );

        let insufficient_recommendation = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.code == "GA-01")
            .expect("insufficient recommendation");
        assert!(
            insufficient_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("insufficient evidence"))
        );
        assert!(
            !insufficient_recommendation
                .reasons
                .iter()
                .any(|reason| reason.contains("PYQ accuracy"))
        );

        let unscheduled = dashboard
            .recommendations
            .iter()
            .find(|item| item.topic.code == "CM-C01")
            .expect("unscheduled weak recommendation");
        assert!(!unscheduled.scheduled);
        assert_eq!(unscheduled.due_state, "not_scheduled");
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
