use chrono::{DateTime, Datelike, Days, NaiveDate, Utc, Weekday};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::{
    date_only,
    error::AppError,
    v2::mastery::{self, ReplayReason, V2EvidenceRecordInput},
};

const OUTCOMES: [&str; 4] = ["again", "difficult", "good", "easy"];
const MAX_INTERVAL_DAYS: i64 = 3_650;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionSettings {
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionSettingsInput {
    pub initial_again_days: i64,
    pub initial_difficult_days: i64,
    pub initial_good_days: i64,
    pub initial_easy_days: i64,
    pub growth_multiplier: f64,
    pub minimum_interval_days: i64,
    pub maximum_interval_days: i64,
    pub include_weekends: bool,
    pub daily_revision_target: i64,
    #[serde(default)]
    pub reminders_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionSchedule {
    pub concept_id: i64,
    pub concept_slug: String,
    pub concept_title: String,
    pub concept_domain: String,
    pub status: String,
    pub last_revision_date: Option<String>,
    pub next_revision_date: String,
    pub current_interval_days: i64,
    pub completed_revisions: i64,
    pub previous_outcome: Option<String>,
    pub notes: String,
    pub overdue_days: i64,
    pub mastery_score: Option<f64>,
    pub selection_reason: String,
    pub eligible: bool,
    pub approved_exam_codes: Vec<String>,
    pub pending_exam_codes: Vec<String>,
    pub already_scheduled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionHistoryEntry {
    pub id: i64,
    pub concept_id: i64,
    pub concept_title: String,
    pub trigger_exam_id: Option<i64>,
    pub revision_date: String,
    pub scheduled_for_date: String,
    pub outcome: String,
    pub notes: String,
    pub interval_before_days: i64,
    pub interval_after_days: i64,
    pub mastery_before: f64,
    pub mastery_after: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionStats {
    pub due_today: i64,
    pub overdue: i64,
    pub completed_today: i64,
    pub completed_last_7_days: i64,
    pub total_completed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionDashboard {
    pub settings: V2RevisionSettings,
    pub due_today: Vec<V2RevisionSchedule>,
    pub overdue: Vec<V2RevisionSchedule>,
    pub upcoming: Vec<V2RevisionSchedule>,
    pub awaiting_approval: Vec<V2RevisionSchedule>,
    pub recent_history: Vec<V2RevisionHistoryEntry>,
    pub stats: V2RevisionStats,
    pub exam_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionPlanItem {
    pub plan_item_id: i64,
    pub concept_id: i64,
    pub concept_title: String,
    pub concept_domain: String,
    pub membership_status: String,
    pub lane: String,
    pub triage_score: f64,
    pub estimated_minutes: i64,
    pub source_due_date: String,
    pub position: i64,
    pub planned_block_id: Option<i64>,
    pub planned_block_title: Option<String>,
    pub selection_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionDailyPlan {
    pub id: i64,
    pub plan_date: String,
    pub exam_id: Option<i64>,
    pub version: i64,
    pub effective_budget_minutes: i64,
    pub max_topics: i64,
    pub planned_minutes: i64,
    pub completed_minutes: f64,
    pub status: String,
    pub items: Vec<V2RevisionPlanItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionDashboardInput {
    #[serde(default)]
    pub exam_id: Option<i64>,
    pub today: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionScheduleInput {
    pub concept_id: i64,
    #[serde(default)]
    pub next_revision_date: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionRescheduleInput {
    pub concept_id: i64,
    pub next_revision_date: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionStatusInput {
    pub concept_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CompleteRevisionInput {
    pub concept_id: i64,
    pub outcome: String,
    pub revision_date: String,
    #[serde(default)]
    pub trigger_exam_id: Option<i64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub plan_item_id: Option<i64>,
    #[serde(default)]
    pub session_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DeferRevisionInput {
    pub concept_id: i64,
    pub new_revision_date: String,
    pub reason: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub plan_item_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RevisionPlanInput {
    pub plan_date: String,
    #[serde(default)]
    pub exam_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2MoveRevisionPlanItemInput {
    pub plan_item_id: i64,
    pub target: String,
    pub plan_date: String,
    #[serde(default)]
    pub exam_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2LinkRevisionBlockInput {
    pub plan_item_id: i64,
    pub block_id: i64,
    pub plan_date: String,
    #[serde(default)]
    pub exam_id: Option<i64>,
}

pub fn prepare(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "DROP INDEX IF EXISTS ux_revision_plan_active_concept;
             CREATE UNIQUE INDEX IF NOT EXISTS ux_revision_plan_active_combined_concept
                ON revision_plan_items(concept_id)
                WHERE status IN ('planned','in_progress','backlog','deferred')
                  AND focus_exam_id IS NULL;
             CREATE UNIQUE INDEX IF NOT EXISTS ux_revision_plan_active_exam_concept
                ON revision_plan_items(concept_id, focus_exam_id)
                WHERE status IN ('planned','in_progress','backlog','deferred')
                  AND focus_exam_id IS NOT NULL;",
        )
        .map_err(|error| {
            AppError::database(
                "The revision plan scope indexes could not be prepared",
                error,
            )
        })
}

pub fn load_settings(connection: &Connection) -> Result<V2RevisionSettings, AppError> {
    connection
        .query_row(
            "SELECT initial_again_days, initial_difficult_days, initial_good_days,
                    initial_easy_days, growth_multiplier, minimum_interval_days,
                    maximum_interval_days, include_weekends, daily_revision_target,
                    reminders_enabled
             FROM revision_settings WHERE id = 1",
            [],
            |row| {
                Ok(V2RevisionSettings {
                    initial_again_days: row.get(0)?,
                    initial_difficult_days: row.get(1)?,
                    initial_good_days: row.get(2)?,
                    initial_easy_days: row.get(3)?,
                    growth_multiplier: row.get(4)?,
                    minimum_interval_days: row.get(5)?,
                    maximum_interval_days: row.get(6)?,
                    include_weekends: row.get::<_, i64>(7)? == 1,
                    daily_revision_target: row.get(8)?,
                    reminders_enabled: row.get::<_, i64>(9)? == 1,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Revision settings could not be loaded", error))?
        .ok_or_else(AppError::revision_settings_not_found)
}

pub fn update_settings(
    connection: &mut Connection,
    input: V2RevisionSettingsInput,
    now: DateTime<Utc>,
) -> Result<V2RevisionSettings, AppError> {
    if !(1..=3_650).contains(&input.initial_again_days)
        || !(1..=3_650).contains(&input.initial_difficult_days)
        || !(1..=3_650).contains(&input.initial_good_days)
        || !(1..=3_650).contains(&input.initial_easy_days)
        || !(1..=3_650).contains(&input.minimum_interval_days)
        || !(1..=3_650).contains(&input.maximum_interval_days)
    {
        return Err(AppError::validation(
            "minimumIntervalDays",
            "Revision intervals must be between 1 and 3,650 days.",
        ));
    }
    if input.minimum_interval_days > input.maximum_interval_days {
        return Err(AppError::validation(
            "maximumIntervalDays",
            "Maximum interval must be at least the minimum interval.",
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
                "Initial intervals must lie within the minimum and maximum interval.",
            ));
        }
    }
    if !input.growth_multiplier.is_finite() || !(1.0..=10.0).contains(&input.growth_multiplier) {
        return Err(AppError::validation(
            "growthMultiplier",
            "Growth multiplier must be between 1.0 and 10.0.",
        ));
    }
    if !(1..=100).contains(&input.daily_revision_target) {
        return Err(AppError::validation(
            "dailyRevisionTarget",
            "Daily revision target must be between 1 and 100 concepts.",
        ));
    }
    connection
        .execute(
            "UPDATE revision_settings
             SET initial_again_days = ?1, initial_difficult_days = ?2,
                 initial_good_days = ?3, initial_easy_days = ?4,
                 growth_multiplier = ?5, minimum_interval_days = ?6,
                 maximum_interval_days = ?7, include_weekends = ?8,
                 daily_revision_target = ?9, reminders_enabled = ?10, updated_at = ?11
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
                now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            ],
        )
        .map_err(|error| AppError::database("Revision settings could not be saved", error))?;
    load_settings(connection)
}

pub fn dashboard(
    connection: &Connection,
    input: V2RevisionDashboardInput,
) -> Result<V2RevisionDashboard, AppError> {
    let today = parse_date(&input.today, "today")?;
    let settings = load_settings(connection)?;
    let schedules = schedules_for_exam(connection, input.exam_id, today)?;
    let due_today: Vec<V2RevisionSchedule> = schedules
        .iter()
        .filter(|schedule| schedule.next_revision_date == input.today)
        .cloned()
        .collect();
    let overdue: Vec<V2RevisionSchedule> = schedules
        .iter()
        .filter(|schedule| schedule.next_revision_date < input.today)
        .cloned()
        .collect();
    let upcoming: Vec<V2RevisionSchedule> = schedules
        .iter()
        .filter(|schedule| schedule.next_revision_date > input.today)
        .take(30)
        .cloned()
        .collect();
    let awaiting_approval = awaiting_approval_schedules(connection, input.exam_id, today)?;
    let week_start = today
        .checked_sub_days(Days::new(6))
        .unwrap_or(today)
        .format("%Y-%m-%d")
        .to_string();
    let stats = V2RevisionStats {
        due_today: due_today.len() as i64,
        overdue: overdue.len() as i64,
        completed_today: connection
            .query_row(
                "SELECT COUNT(*) FROM revision_history WHERE revision_date = ?1",
                [&input.today],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Revision count could not be read", error))?,
        completed_last_7_days: connection
            .query_row(
                "SELECT COUNT(*) FROM revision_history
                 WHERE revision_date >= ?1 AND revision_date <= ?2",
                params![week_start, input.today],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Revision count could not be read", error))?,
        total_completed: connection
            .query_row("SELECT COUNT(*) FROM revision_history", [], |row| {
                row.get(0)
            })
            .map_err(|error| AppError::database("Revision count could not be read", error))?,
    };
    Ok(V2RevisionDashboard {
        settings,
        due_today,
        overdue,
        upcoming,
        awaiting_approval,
        recent_history: history(connection, input.exam_id, 20)?,
        stats,
        exam_id: input.exam_id,
    })
}

pub fn schedule_concept(
    connection: &mut Connection,
    input: V2RevisionScheduleInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<V2RevisionSchedule, AppError> {
    let settings = load_settings(connection)?;
    let concept_ok: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1 AND is_archived = 0)",
            [input.concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Concept could not be checked", error))?;
    if concept_ok == 0 {
        return Err(AppError::concept_not_found());
    }
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Revision notes must be 2,000 characters or fewer.",
        ));
    }
    let already_scheduled: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM revision_schedules WHERE concept_id = ?1)",
            [input.concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Revision schedule could not be checked", error))?;
    if already_scheduled == 0 {
        let next = match input.next_revision_date.as_deref() {
            Some(value) if !value.trim().is_empty() => parse_date(value, "nextRevisionDate")?,
            _ => schedule_next_date(today, settings.initial_good_days, settings.include_weekends)?,
        };
        let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        connection
            .execute(
                "INSERT INTO revision_schedules (
                    concept_id, status, last_revision_date, next_revision_date,
                    current_interval_days, completed_revisions, previous_outcome, notes,
                    created_at, updated_at
                 ) VALUES (?1, 'active', NULL, ?2, ?3, 0, NULL, ?4, ?5, ?5)",
                params![
                    input.concept_id,
                    date_only::format_date_only(next),
                    settings.initial_good_days,
                    input.notes.trim(),
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Revision schedule could not be saved", error))?;
    }
    let mut schedule = load_schedule(connection, input.concept_id, today)?;
    schedule.already_scheduled = already_scheduled == 1;
    Ok(schedule)
}

/// Give an approved concept its first revision schedule. Existing schedules
/// (including suspended or previously reviewed ones) are never reset.
pub fn ensure_initial_schedule(
    connection: &Connection,
    concept_id: i64,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let settings = load_settings(connection)?;
    let next = schedule_next_date(today, settings.initial_good_days, settings.include_weekends)?;
    connection
        .execute(
            "INSERT INTO revision_schedules (
                concept_id, status, last_revision_date, next_revision_date,
                current_interval_days, completed_revisions, previous_outcome, notes,
                created_at, updated_at
             ) VALUES (?1, 'active', NULL, ?2, ?3, 0, NULL, '', ?4, ?4)
             ON CONFLICT(concept_id) DO NOTHING",
            params![
                concept_id,
                date_only::format_date_only(next),
                settings.initial_good_days,
                now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            ],
        )
        .map_err(|error| {
            AppError::database("Initial revision schedule could not be saved", error)
        })?;
    Ok(())
}

pub fn reschedule(
    connection: &mut Connection,
    input: V2RevisionRescheduleInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<V2RevisionSchedule, AppError> {
    let next = parse_date(&input.next_revision_date, "nextRevisionDate")?;
    let changed = connection
        .execute(
            "UPDATE revision_schedules
             SET next_revision_date = ?1, updated_at = ?2
             WHERE concept_id = ?3",
            params![
                date_only::format_date_only(next),
                now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                input.concept_id
            ],
        )
        .map_err(|error| AppError::database("Revision could not be rescheduled", error))?;
    if changed != 1 {
        return Err(AppError::revision_not_scheduled());
    }
    load_schedule(connection, input.concept_id, today)
}

pub fn set_status(
    connection: &mut Connection,
    input: V2RevisionStatusInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<V2RevisionSchedule, AppError> {
    if !matches!(input.status.as_str(), "active" | "suspended") {
        return Err(AppError::validation(
            "status",
            "Revision schedule status must be active or suspended.",
        ));
    }
    let changed = connection
        .execute(
            "UPDATE revision_schedules SET status = ?1, updated_at = ?2
             WHERE concept_id = ?3",
            params![
                input.status,
                now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                input.concept_id
            ],
        )
        .map_err(|error| {
            AppError::database("Revision schedule status could not be saved", error)
        })?;
    if changed != 1 {
        return Err(AppError::revision_not_scheduled());
    }
    load_schedule(connection, input.concept_id, today)
}

pub fn complete_revision(
    connection: &mut Connection,
    input: V2CompleteRevisionInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<V2RevisionDashboard, AppError> {
    let outcome = input.outcome.trim().to_lowercase();
    if !OUTCOMES.contains(&outcome.as_str()) {
        return Err(AppError::validation(
            "outcome",
            "Choose again, difficult, good, or easy.",
        ));
    }
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Revision notes must be 2,000 characters or fewer.",
        ));
    }
    let revision_date = parse_date(&input.revision_date, "revisionDate")?;
    if let Some(exam_id) = input.trigger_exam_id {
        validate_active_exam(connection, exam_id)?;
    }
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision completion transaction could not start", error)
    })?;
    let schedule = load_schedule_row(&transaction, input.concept_id)?;
    if schedule.status != "active" {
        return Err(AppError::revision_suspended());
    }
    let settings = load_settings(&transaction)?;
    let scheduled_for = parse_date(&schedule.next_revision_date, "scheduledForDate")?;
    let interval_before = schedule.current_interval_days;
    let interval_after = next_interval(settings.clone(), interval_before, &outcome);
    let next_date = schedule_next_date(revision_date, interval_after, settings.include_weekends)?;
    let mastery_before = mastery_before(&transaction, input.concept_id)?;
    let mastery_after = mastery::preview_revision_outcome(
        &transaction,
        input.concept_id,
        input.trigger_exam_id,
        &date_only::format_date_only(revision_date),
        now,
        &outcome,
        now,
    )?;
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT INTO revision_history (
                concept_id, trigger_exam_id, revision_date, scheduled_for_date, outcome,
                notes, interval_before_days, interval_after_days, mastery_before,
                mastery_after, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                input.concept_id,
                input.trigger_exam_id,
                date_only::format_date_only(revision_date),
                date_only::format_date_only(scheduled_for),
                outcome,
                input.notes.trim(),
                interval_before,
                interval_after,
                mastery_before,
                mastery_after,
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Revision history could not be saved", error))?;
    let history_id = transaction.last_insert_rowid();
    transaction
        .execute(
            "UPDATE revision_schedules
             SET last_revision_date = ?1, next_revision_date = ?2,
                 current_interval_days = ?3, completed_revisions = completed_revisions + 1,
                 previous_outcome = ?4, updated_at = ?5
             WHERE concept_id = ?6",
            params![
                date_only::format_date_only(revision_date),
                date_only::format_date_only(next_date),
                interval_after,
                outcome,
                now_text,
                input.concept_id,
            ],
        )
        .map_err(|error| AppError::database("Revision schedule could not be updated", error))?;
    mastery::record_evidence_in_transaction(
        &transaction,
        V2EvidenceRecordInput {
            concept_id: input.concept_id,
            source_kind: "revision".to_owned(),
            source_id: Some(history_id),
            source_version: 1,
            exam_id: input.trigger_exam_id,
            local_date: date_only::format_date_only(revision_date),
            occurred_at: now_text.clone(),
            raw_value: serde_json::json!({ "outcome": outcome }),
            notes: input.notes.clone(),
        },
        ReplayReason::SourceCreated,
        now,
    )?;

    if let Some(plan_item_id) = input.plan_item_id {
        complete_plan_item(
            &transaction,
            plan_item_id,
            history_id,
            input.session_id,
            now_text,
        )?;
    } else if let Some(plan_item_id) = active_plan_item_for_concept(&transaction, input.concept_id)?
    {
        complete_plan_item(
            &transaction,
            plan_item_id,
            history_id,
            input.session_id,
            now_text,
        )?;
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("Revision completion could not be committed", error))?;
    dashboard(
        connection,
        V2RevisionDashboardInput {
            exam_id: input.trigger_exam_id,
            today: date_only::format_date_only(today),
        },
    )
}

pub fn defer_revision(
    connection: &mut Connection,
    input: V2DeferRevisionInput,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> Result<V2RevisionDashboard, AppError> {
    let new_date = parse_date(&input.new_revision_date, "newRevisionDate")?;
    if !matches!(
        input.reason.as_str(),
        "time" | "difficulty" | "energy" | "life" | "other"
    ) {
        return Err(AppError::validation(
            "reason",
            "Choose time, difficulty, energy, life, or other.",
        ));
    }
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Deferral notes must be 2,000 characters or fewer.",
        ));
    }
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision deferral transaction could not start", error)
    })?;
    let schedule = load_schedule_row(&transaction, input.concept_id)?;
    let previous_due = parse_date(&schedule.next_revision_date, "nextRevisionDate")?;
    if new_date <= previous_due {
        return Err(AppError::validation(
            "newRevisionDate",
            "A deferred revision must move to a later date.",
        ));
    }
    let plan_item_id = match input.plan_item_id {
        Some(id) => Some(id),
        None => active_plan_item_for_concept(&transaction, input.concept_id)?,
    };
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT INTO revision_deferrals (
                concept_id, plan_item_id, event_kind, overridden_deferral_id,
                previous_due_date, new_due_date, occurred_on, reason, notes, created_at
             ) VALUES (?1, ?2, 'defer', NULL, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.concept_id,
                plan_item_id,
                date_only::format_date_only(previous_due),
                date_only::format_date_only(new_date),
                date_only::format_date_only(today),
                input.reason,
                input.notes.trim(),
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Revision deferral could not be saved", error))?;
    transaction
        .execute(
            "UPDATE revision_schedules SET next_revision_date = ?1, updated_at = ?2
             WHERE concept_id = ?3",
            params![
                date_only::format_date_only(new_date),
                now_text,
                input.concept_id
            ],
        )
        .map_err(|error| AppError::database("Deferred revision date could not be saved", error))?;
    if let Some(plan_item_id) = plan_item_id {
        transaction
            .execute(
                "UPDATE revision_plan_items
                 SET status = 'deferred', active_daily_plan_id = NULL, updated_at = ?1
                 WHERE id = ?2 AND status <> 'completed'",
                params![now_text, plan_item_id],
            )
            .map_err(|error| {
                AppError::database("Revision plan item could not be deferred", error)
            })?;
        transaction
            .execute(
                "UPDATE revision_daily_plan_items
                 SET membership_status = 'deferred', updated_at = ?1
                 WHERE plan_item_id = ?2
                   AND daily_plan_id IN (SELECT id FROM revision_daily_plans WHERE status = 'active')",
                params![now_text, plan_item_id],
            )
            .map_err(|error| AppError::database("Revision membership could not be deferred", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision deferral could not be committed", error))?;
    dashboard(
        connection,
        V2RevisionDashboardInput {
            exam_id: None,
            today: date_only::format_date_only(today),
        },
    )
}

pub fn generate_daily_plan(
    connection: &mut Connection,
    input: V2RevisionPlanInput,
    now: DateTime<Utc>,
) -> Result<V2RevisionDailyPlan, AppError> {
    let plan_date = parse_date(&input.plan_date, "planDate")?;
    if let Some(exam_id) = input.exam_id {
        validate_active_exam(connection, exam_id)?;
    }
    if let Some(plan_id) = load_active_plan(connection, plan_date, input.exam_id)? {
        return reconcile_active_plan(connection, plan_id, plan_date, input.exam_id, now);
    }
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision plan transaction could not start", error)
    })?;
    let settings = load_settings(&transaction)?;
    let version: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM revision_daily_plans
             WHERE plan_date = ?1 AND exam_id IS ?2",
            params![date_only::format_date_only(plan_date), input.exam_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| AppError::database("Revision plan version could not be read", error))?
        + 1;
    let budget = (settings.daily_revision_target * 30).clamp(30, 1_440);
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT INTO revision_daily_plans (
                plan_date, exam_id, version, effective_budget_minutes, max_topics,
                planned_minutes, completed_minutes, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 'active', ?6, ?6)",
            params![
                date_only::format_date_only(plan_date),
                input.exam_id,
                version,
                budget,
                settings.daily_revision_target,
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Revision plan could not be created", error))?;
    let plan_id = transaction.last_insert_rowid();
    let candidates = due_candidates(
        &transaction,
        plan_date,
        input.exam_id,
        settings.daily_revision_target,
    )?;
    let planned_minutes = append_due_candidates(
        &transaction,
        plan_id,
        input.exam_id,
        &candidates,
        0,
        &now_text,
    )?;
    transaction
        .execute(
            "UPDATE revision_daily_plans SET planned_minutes = ?1 WHERE id = ?2",
            params![planned_minutes, plan_id],
        )
        .map_err(|error| AppError::database("Revision plan totals could not be saved", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision plan could not be committed", error))?;
    load_plan(connection, plan_id)
}

fn reconcile_active_plan(
    connection: &mut Connection,
    plan_id: i64,
    plan_date: NaiveDate,
    exam_id: Option<i64>,
    now: DateTime<Utc>,
) -> Result<V2RevisionDailyPlan, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision plan reconciliation could not start", error)
    })?;
    let settings = load_settings(&transaction)?;
    let candidates = due_candidates(
        &transaction,
        plan_date,
        exam_id,
        settings.daily_revision_target,
    )?;
    let members = {
        let mut member_statement = transaction
            .prepare(
                "SELECT p.concept_id, i.membership_status, i.position
                 FROM revision_daily_plan_items i
                 JOIN revision_plan_items p ON p.id = i.plan_item_id
                 WHERE i.daily_plan_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Revision plan members could not be prepared", error)
            })?;
        member_statement
            .query_map([plan_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|error| AppError::database("Revision plan members could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| AppError::database("A revision plan member is invalid", error))?
    };
    let present: std::collections::HashSet<i64> = members
        .iter()
        .map(|(concept_id, _, _)| *concept_id)
        .collect();
    let used_topics = members
        .iter()
        .filter(|(_, status, _)| !matches!(status.as_str(), "removed" | "cancelled" | "deferred"))
        .count() as i64;
    let available = (settings.daily_revision_target - used_topics).max(0) as usize;
    let missing: Vec<DueCandidate> = candidates
        .into_iter()
        .filter(|candidate| !present.contains(&candidate.concept_id))
        .take(available)
        .collect();
    if !missing.is_empty() {
        let next_position = members
            .iter()
            .map(|(_, _, position)| *position)
            .max()
            .map(|position| position + 1)
            .unwrap_or(0);
        let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let added = append_due_candidates(
            &transaction,
            plan_id,
            exam_id,
            &missing,
            next_position,
            &now_text,
        )?;
        transaction
            .execute(
                "UPDATE revision_daily_plans
                 SET planned_minutes = planned_minutes + ?1, updated_at = ?2
                 WHERE id = ?3",
                params![added, now_text, plan_id],
            )
            .map_err(|error| {
                AppError::database("Revision plan totals could not be updated", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision plan could not be reconciled", error))?;
    load_plan(connection, plan_id)
}

fn append_due_candidates(
    transaction: &Transaction<'_>,
    plan_id: i64,
    exam_id: Option<i64>,
    candidates: &[DueCandidate],
    start_position: i64,
    now_text: &str,
) -> Result<i64, AppError> {
    let mut planned_minutes = 0_i64;
    for (offset, candidate) in candidates.iter().enumerate() {
        let existing: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, status FROM revision_plan_items
                 WHERE concept_id = ?1
                   AND focus_exam_id IS ?2
                   AND status IN ('planned','in_progress','backlog','deferred')
                 ORDER BY id DESC LIMIT 1",
                params![candidate.concept_id, exam_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("Revision plan item could not be checked", error)
            })?;
        let plan_item_id = if let Some((existing, _status)) = existing {
            transaction
                .execute(
                    "UPDATE revision_plan_items
                     SET status = CASE WHEN status = 'deferred' THEN 'planned' ELSE status END,
                         active_daily_plan_id = ?1, updated_at = ?2
                     WHERE id = ?3",
                    params![plan_id, now_text, existing],
                )
                .map_err(|error| {
                    AppError::database("Revision item could not be attached", error)
                })?;
            existing
        } else {
            transaction
                .execute(
                    "INSERT INTO revision_plan_items (
                        concept_id, focus_exam_id, active_daily_plan_id, status,
                        estimated_minutes, triage_score, lane, source_due_date,
                        created_at, updated_at
                     ) VALUES (?1, ?2, ?3, 'planned', ?4, ?5, ?6, ?7, ?8, ?8)",
                    params![
                        candidate.concept_id,
                        exam_id,
                        plan_id,
                        candidate.estimated_minutes,
                        candidate.triage_score,
                        candidate.lane,
                        candidate.source_due_date,
                        now_text,
                    ],
                )
                .map_err(|error| AppError::database("Revision item could not be created", error))?;
            transaction.last_insert_rowid()
        };
        transaction
            .execute(
                "INSERT INTO revision_daily_plan_items (
                    daily_plan_id, plan_item_id, position, estimated_minutes, lane,
                    triage_score, membership_status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'planned', ?7, ?7)",
                params![
                    plan_id,
                    plan_item_id,
                    start_position + offset as i64,
                    candidate.estimated_minutes,
                    candidate.lane,
                    candidate.triage_score,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("Revision membership could not be created", error)
            })?;
        planned_minutes += candidate.estimated_minutes;
    }
    Ok(planned_minutes)
}

pub fn rebalance_daily_plan(
    connection: &mut Connection,
    input: V2RevisionPlanInput,
    now: DateTime<Utc>,
) -> Result<V2RevisionDailyPlan, AppError> {
    let plan_date = parse_date(&input.plan_date, "planDate")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision rebalance transaction could not start", error)
    })?;
    if let Some(plan) = load_active_plan(&transaction, plan_date, input.exam_id)? {
        transaction
            .execute(
                "UPDATE revision_daily_plans SET status = 'superseded', updated_at = ?1
                 WHERE id = ?2",
                params![
                    now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    plan
                ],
            )
            .map_err(|error| AppError::database("Revision plan could not be superseded", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision rebalance could not be committed", error))?;
    generate_daily_plan(connection, input, now)
}

pub fn get_daily_plan(
    connection: &Connection,
    input: V2RevisionPlanInput,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    let plan_date = parse_date(&input.plan_date, "planDate")?;
    match load_active_plan(connection, plan_date, input.exam_id)? {
        Some(plan_id) => load_plan(connection, plan_id).map(Some),
        None => Ok(None),
    }
}

pub fn move_plan_item(
    connection: &mut Connection,
    input: V2MoveRevisionPlanItemInput,
    now: DateTime<Utc>,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    if !matches!(input.target.as_str(), "backlog" | "removed" | "cancelled") {
        return Err(AppError::validation(
            "target",
            "Choose backlog, removed, or cancelled.",
        ));
    }
    let plan_date = parse_date(&input.plan_date, "planDate")?;
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision move transaction could not start", error)
    })?;
    let status = match input.target.as_str() {
        "backlog" => "backlog",
        "removed" => "removed",
        _ => "cancelled",
    };
    let cancelled_at = (status == "cancelled").then_some(now_text.clone());
    let removed_at = (status == "removed").then_some(now_text.clone());
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = ?1, active_daily_plan_id = NULL, cancelled_at = ?2,
                 removed_at = ?3, updated_at = ?4
             WHERE id = ?5 AND status <> 'completed'",
            params![
                status,
                cancelled_at,
                removed_at,
                now_text,
                input.plan_item_id
            ],
        )
        .map_err(|error| AppError::database("Revision item could not be moved", error))?;
    let membership_status = if status == "cancelled" {
        "cancelled"
    } else {
        "removed"
    };
    transaction
        .execute(
            "UPDATE revision_daily_plan_items SET membership_status = ?1, updated_at = ?2
             WHERE plan_item_id = ?3
               AND daily_plan_id IN (SELECT id FROM revision_daily_plans WHERE status = 'active')",
            params![membership_status, now_text, input.plan_item_id],
        )
        .map_err(|error| AppError::database("Revision membership could not be moved", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Revision move could not be committed", error))?;
    get_daily_plan(
        connection,
        V2RevisionPlanInput {
            plan_date: date_only::format_date_only(plan_date),
            exam_id: input.exam_id,
        },
    )
}

pub fn link_plan_block(
    connection: &mut Connection,
    input: V2LinkRevisionBlockInput,
    now: DateTime<Utc>,
) -> Result<Option<V2RevisionDailyPlan>, AppError> {
    let plan_date = parse_date(&input.plan_date, "planDate")?;
    let block = connection
        .query_row(
            "SELECT title, calendar_date, start_time, end_time
             FROM planned_study_blocks WHERE id = ?1",
            [input.block_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("Planned block could not be loaded", error))?
        .ok_or_else(AppError::block_not_found)?;
    if block.1 != date_only::format_date_only(plan_date) {
        return Err(AppError::validation(
            "blockId",
            "Choose a planned block on the same date as the revision plan.",
        ));
    }
    let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    connection
        .execute(
            "UPDATE revision_plan_items SET planned_block_id = ?1, updated_at = ?2
             WHERE id = ?3 AND status <> 'completed'",
            params![input.block_id, now_text, input.plan_item_id],
        )
        .map_err(|error| {
            AppError::database("Revision item block link could not be saved", error)
        })?;
    connection
        .execute(
            "UPDATE revision_daily_plan_items
             SET planned_block_id = ?1, planned_block_title = ?2, planned_block_date = ?3,
                 planned_block_start_time = ?4, planned_block_end_time = ?5, updated_at = ?6
             WHERE plan_item_id = ?7
               AND daily_plan_id IN (SELECT id FROM revision_daily_plans WHERE status = 'active')",
            params![
                input.block_id,
                block.0,
                block.1,
                block.2,
                block.3,
                now_text,
                input.plan_item_id
            ],
        )
        .map_err(|error| {
            AppError::database("Revision membership block link could not be saved", error)
        })?;
    get_daily_plan(
        connection,
        V2RevisionPlanInput {
            plan_date: date_only::format_date_only(plan_date),
            exam_id: input.exam_id,
        },
    )
}

fn complete_plan_item(
    transaction: &Transaction<'_>,
    plan_item_id: i64,
    history_id: i64,
    session_id: Option<i64>,
    now_text: String,
) -> Result<(), AppError> {
    if let Some(session_id) = session_id {
        let exists: i64 = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM study_sessions WHERE id = ?1)",
                [session_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Study session could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::session_not_found());
        }
        transaction
            .execute(
                "INSERT INTO revision_session_links (plan_item_id, study_session_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![plan_item_id, session_id, now_text],
            )
            .map_err(|error| {
                AppError::database("Revision session link could not be saved", error)
            })?;
    }
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = 'completed', revision_history_id = ?1, active_daily_plan_id = NULL,
                 updated_at = ?2
             WHERE id = ?3 AND status <> 'completed'",
            params![history_id, now_text, plan_item_id],
        )
        .map_err(|error| AppError::database("Revision item could not be completed", error))?;
    transaction
        .execute(
            "UPDATE revision_daily_plan_items
             SET membership_status = 'completed', updated_at = ?1
             WHERE plan_item_id = ?2
               AND daily_plan_id IN (SELECT id FROM revision_daily_plans WHERE status = 'active')",
            params![now_text, plan_item_id],
        )
        .map_err(|error| AppError::database("Revision membership could not be completed", error))?;
    Ok(())
}

fn schedule_next_date(
    from: NaiveDate,
    interval_days: i64,
    include_weekends: bool,
) -> Result<NaiveDate, AppError> {
    let added = from
        .checked_add_days(Days::new(interval_days.max(1) as u64))
        .ok_or_else(|| AppError::validation("nextRevisionDate", "Next date is out of range."))?;
    if include_weekends {
        return Ok(added);
    }
    let adjusted = match added.weekday() {
        Weekday::Sat => added.checked_add_days(Days::new(2)),
        Weekday::Sun => added.checked_add_days(Days::new(1)),
        _ => Some(added),
    }
    .ok_or_else(|| AppError::validation("nextRevisionDate", "Next date is out of range."))?;
    Ok(adjusted)
}

fn next_interval(settings: V2RevisionSettings, current: i64, outcome: &str) -> i64 {
    let base = match outcome {
        "again" => settings.initial_again_days,
        "difficult" => settings.initial_difficult_days,
        "good" => settings.initial_good_days,
        "easy" => settings.initial_easy_days,
        _ => settings.initial_good_days,
    };
    let candidate = if matches!(outcome, "good" | "easy") {
        base.max((current as f64 * settings.growth_multiplier).round() as i64)
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

fn mastery_before(connection: &Connection, concept_id: i64) -> Result<f64, AppError> {
    connection
        .query_row(
            "SELECT mastery_score FROM concept_mastery WHERE concept_id = ?1",
            [concept_id],
            |row| row.get::<_, f64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Concept mastery could not be loaded", error))
        .map(|value| value.unwrap_or(0.0))
}

fn revision_reason(due_date: &str, today: NaiveDate, mastery_score: Option<f64>) -> String {
    let due = if let Some(date) = date_only::parse_date_only(due_date) {
        if date < today {
            format!("Overdue by {} day(s)", (today - date).num_days())
        } else if date == today {
            "Due today".to_owned()
        } else {
            format!("Scheduled for {due_date}")
        }
    } else {
        format!("Scheduled for {due_date}")
    };
    let mastery = match mastery_score {
        Some(score) => format!("mastery {score:.1}"),
        None => "no mastery evidence".to_owned(),
    };
    format!("{due}; {mastery}; approved concept-to-syllabus mapping")
}

fn load_schedule_row(connection: &Connection, concept_id: i64) -> Result<ScheduleRow, AppError> {
    connection
        .query_row(
            "SELECT status, next_revision_date, current_interval_days
             FROM revision_schedules WHERE concept_id = ?1",
            [concept_id],
            |row| {
                Ok(ScheduleRow {
                    status: row.get(0)?,
                    next_revision_date: row.get(1)?,
                    current_interval_days: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Revision schedule could not be loaded", error))?
        .ok_or_else(AppError::revision_not_scheduled)
}

fn load_schedule(
    connection: &Connection,
    concept_id: i64,
    today: NaiveDate,
) -> Result<V2RevisionSchedule, AppError> {
    let today_text = date_only::format_date_only(today);
    let (approved_exam_codes, pending_exam_codes) = schedule_exam_codes(connection, concept_id)?;
    connection
        .query_row(
            "SELECT r.concept_id, c.slug, c.title, c.domain, r.status,
                    r.last_revision_date, r.next_revision_date, r.current_interval_days,
                    r.completed_revisions, r.previous_outcome, r.notes, cm.mastery_score
             FROM revision_schedules r
             JOIN concepts c ON c.id = r.concept_id
             LEFT JOIN concept_mastery cm ON cm.concept_id = r.concept_id
             WHERE r.concept_id = ?1",
            [concept_id],
            |row| {
                let next: String = row.get(6)?;
                let overdue_days = date_only::parse_date_only(&next)
                    .map(|date| (today - date).num_days().max(0))
                    .unwrap_or(0);
                Ok(V2RevisionSchedule {
                    concept_id: row.get(0)?,
                    concept_slug: row.get(1)?,
                    concept_title: row.get(2)?,
                    concept_domain: row.get(3)?,
                    status: row.get(4)?,
                    last_revision_date: row.get(5)?,
                    next_revision_date: next,
                    current_interval_days: row.get(7)?,
                    completed_revisions: row.get(8)?,
                    previous_outcome: row.get(9)?,
                    notes: row.get(10)?,
                    overdue_days,
                    mastery_score: row.get(11)?,
                    selection_reason: String::new(),
                    eligible: !approved_exam_codes.is_empty(),
                    approved_exam_codes,
                    pending_exam_codes,
                    already_scheduled: false,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Revision schedule could not be read", error))?
        .ok_or_else(AppError::revision_not_scheduled)
        .map(|mut schedule| {
            schedule.overdue_days = if schedule.next_revision_date < today_text {
                schedule.overdue_days
            } else {
                0
            };
            schedule.selection_reason =
                revision_reason(&schedule.next_revision_date, today, schedule.mastery_score);
            schedule
        })
}

fn schedule_exam_codes(
    connection: &Connection,
    concept_id: i64,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.code, nc.status
             FROM exam_node_concepts nc
             JOIN exams e ON e.id = nc.exam_id
             JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
             WHERE nc.concept_id = ?1 AND nc.status IN ('approved','proposed','reviewed')
             ORDER BY e.display_order, e.code",
        )
        .map_err(|error| AppError::database("Revision eligibility could not be prepared", error))?;
    let rows = statement
        .query_map([concept_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| AppError::database("Revision eligibility could not be read", error))?;
    let mut approved = Vec::new();
    let mut pending = Vec::new();
    for row in rows {
        let (code, status) = row
            .map_err(|error| AppError::database("A revision eligibility row is invalid", error))?;
        let target = if status == "approved" {
            &mut approved
        } else {
            &mut pending
        };
        if !target.contains(&code) {
            target.push(code);
        }
    }
    Ok((approved, pending))
}

fn schedules_for_exam(
    connection: &Connection,
    exam_id: Option<i64>,
    today: NaiveDate,
) -> Result<Vec<V2RevisionSchedule>, AppError> {
    let mut schedules = if let Some(exam_id) = exam_id {
        let mut statement = connection
            .prepare(
                "SELECT r.concept_id
                 FROM revision_schedules r
                 JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
                 JOIN exam_node_concepts nc
                   ON nc.concept_id = r.concept_id AND nc.exam_id = ?1
                  AND nc.status = 'approved'
                 WHERE r.status = 'active'
                 ORDER BY r.next_revision_date, c.title",
            )
            .map_err(|error| {
                AppError::database("Revision schedules could not be prepared", error)
            })?;
        let rows = statement
            .query_map([exam_id], |row| row.get::<_, i64>(0))
            .map_err(|error| AppError::database("Revision schedules could not be read", error))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(load_schedule(
                connection,
                row.map_err(|error| AppError::database("A revision schedule is invalid", error))?,
                today,
            )?);
        }
        result
    } else {
        let mut statement = connection
            .prepare(
                "SELECT r.concept_id
                 FROM revision_schedules r
                 JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
                 WHERE r.status = 'active'
                   AND EXISTS (
                     SELECT 1 FROM exam_node_concepts nc
                     JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
                     WHERE nc.concept_id = r.concept_id AND nc.status = 'approved'
                   )
                 ORDER BY r.next_revision_date, c.title",
            )
            .map_err(|error| {
                AppError::database("Revision schedules could not be prepared", error)
            })?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|error| AppError::database("Revision schedules could not be read", error))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(load_schedule(
                connection,
                row.map_err(|error| AppError::database("A revision schedule is invalid", error))?,
                today,
            )?);
        }
        result
    };
    schedules.sort_by(|left, right| {
        left.next_revision_date
            .cmp(&right.next_revision_date)
            .then_with(|| left.concept_title.cmp(&right.concept_title))
    });
    Ok(schedules)
}

fn awaiting_approval_schedules(
    connection: &Connection,
    exam_id: Option<i64>,
    today: NaiveDate,
) -> Result<Vec<V2RevisionSchedule>, AppError> {
    let mut schedules = if let Some(exam_id) = exam_id {
        let mut statement = connection
            .prepare(
                "SELECT r.concept_id
                 FROM revision_schedules r
                 JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
                 WHERE r.status = 'active'
                   AND EXISTS (
                     SELECT 1 FROM exam_node_concepts nc
                     JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
                     WHERE nc.concept_id = r.concept_id AND nc.exam_id = ?1
                       AND nc.status IN ('proposed','reviewed')
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM exam_node_concepts nc
                     JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
                     WHERE nc.concept_id = r.concept_id AND nc.exam_id = ?1
                       AND nc.status = 'approved'
                   )
                 ORDER BY r.next_revision_date, c.title",
            )
            .map_err(|error| {
                AppError::database("Awaiting-approval revisions could not be prepared", error)
            })?;
        let rows = statement
            .query_map([exam_id], |row| row.get::<_, i64>(0))
            .map_err(|error| {
                AppError::database("Awaiting-approval revisions could not be read", error)
            })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(load_schedule(
                connection,
                row.map_err(|error| {
                    AppError::database("An awaiting-approval revision is invalid", error)
                })?,
                today,
            )?);
        }
        result
    } else {
        let mut statement = connection
            .prepare(
                "SELECT r.concept_id
                 FROM revision_schedules r
                 JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
                 WHERE r.status = 'active'
                   AND NOT EXISTS (
                     SELECT 1 FROM exam_node_concepts nc
                     JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
                     WHERE nc.concept_id = r.concept_id AND nc.status = 'approved'
                   )
                 ORDER BY r.next_revision_date, c.title",
            )
            .map_err(|error| {
                AppError::database("Awaiting-approval revisions could not be prepared", error)
            })?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|error| {
                AppError::database("Awaiting-approval revisions could not be read", error)
            })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(load_schedule(
                connection,
                row.map_err(|error| {
                    AppError::database("An awaiting-approval revision is invalid", error)
                })?,
                today,
            )?);
        }
        result
    };
    schedules.sort_by(|left, right| {
        left.next_revision_date
            .cmp(&right.next_revision_date)
            .then_with(|| left.concept_title.cmp(&right.concept_title))
    });
    Ok(schedules)
}

fn history(
    connection: &Connection,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2RevisionHistoryEntry>, AppError> {
    let sql = if exam_id.is_some() {
        "SELECT h.id, h.concept_id, c.title, h.trigger_exam_id, h.revision_date,
                h.scheduled_for_date, h.outcome, h.notes, h.interval_before_days,
                h.interval_after_days, h.mastery_before, h.mastery_after
         FROM revision_history h
         JOIN concepts c ON c.id = h.concept_id
         WHERE h.trigger_exam_id = ?1
         ORDER BY h.revision_date DESC, h.id DESC LIMIT ?2"
    } else {
        "SELECT h.id, h.concept_id, c.title, h.trigger_exam_id, h.revision_date,
                h.scheduled_for_date, h.outcome, h.notes, h.interval_before_days,
                h.interval_after_days, h.mastery_before, h.mastery_after
         FROM revision_history h
         JOIN concepts c ON c.id = h.concept_id
         ORDER BY h.revision_date DESC, h.id DESC LIMIT ?1"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Revision history could not be prepared", error))?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(V2RevisionHistoryEntry {
            id: row.get(0)?,
            concept_id: row.get(1)?,
            concept_title: row.get(2)?,
            trigger_exam_id: row.get(3)?,
            revision_date: row.get(4)?,
            scheduled_for_date: row.get(5)?,
            outcome: row.get(6)?,
            notes: row.get(7)?,
            interval_before_days: row.get(8)?,
            interval_after_days: row.get(9)?,
            mastery_before: row.get(10)?,
            mastery_after: row.get(11)?,
        })
    };
    let rows = if let Some(exam_id) = exam_id {
        statement
            .query_map(params![exam_id, limit], map_row)
            .map_err(|error| AppError::database("Revision history could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
    } else {
        statement
            .query_map(params![limit], map_row)
            .map_err(|error| AppError::database("Revision history could not be read", error))?
            .collect::<rusqlite::Result<Vec<_>>>()
    };
    rows.map_err(|error| AppError::database("Revision history contains an invalid row", error))
}

#[derive(Debug, Clone)]
struct ScheduleRow {
    status: String,
    next_revision_date: String,
    current_interval_days: i64,
}

#[derive(Debug, Clone)]
struct DueCandidate {
    concept_id: i64,
    estimated_minutes: i64,
    triage_score: f64,
    lane: String,
    source_due_date: String,
}

fn due_candidates(
    connection: &Connection,
    today: NaiveDate,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<DueCandidate>, AppError> {
    let today_text = date_only::format_date_only(today);
    let sql = if exam_id.is_some() {
        "SELECT r.concept_id, r.next_revision_date, COALESCE(cm.mastery_score, 0)
         FROM revision_schedules r
         JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
         JOIN exam_node_concepts nc
           ON nc.concept_id = r.concept_id AND nc.exam_id = ?1
          AND nc.status = 'approved'
         JOIN exam_profiles ep ON ep.exam_id = ?1 AND ep.is_active = 1
         LEFT JOIN concept_mastery cm ON cm.concept_id = r.concept_id
         WHERE r.status = 'active' AND r.next_revision_date <= ?2
         GROUP BY r.concept_id
         ORDER BY r.next_revision_date, c.title
         LIMIT ?3"
    } else {
        "SELECT r.concept_id, r.next_revision_date, COALESCE(cm.mastery_score, 0)
         FROM revision_schedules r
         JOIN concepts c ON c.id = r.concept_id AND c.is_archived = 0
         LEFT JOIN concept_mastery cm ON cm.concept_id = r.concept_id
         WHERE r.status = 'active' AND r.next_revision_date <= ?1
           AND EXISTS (
             SELECT 1 FROM exam_node_concepts nc
             JOIN exam_profiles ep ON ep.exam_id = nc.exam_id AND ep.is_active = 1
             WHERE nc.concept_id = r.concept_id AND nc.status = 'approved'
           )
         GROUP BY r.concept_id
         ORDER BY r.next_revision_date, c.title
         LIMIT ?2"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Revision candidates could not be prepared", error))?;
    let mut result = Vec::new();
    if let Some(exam_id) = exam_id {
        let rows = statement
            .query_map(params![exam_id, today_text, limit], |row| {
                candidate_from_row(row, today)
            })
            .map_err(|error| AppError::database("Revision candidates could not be read", error))?;
        for row in rows {
            result.push(
                row.map_err(|error| AppError::database("A revision candidate is invalid", error))?,
            );
        }
    } else {
        let rows = statement
            .query_map(params![today_text, limit], |row| {
                candidate_from_row(row, today)
            })
            .map_err(|error| AppError::database("Revision candidates could not be read", error))?;
        for row in rows {
            result.push(
                row.map_err(|error| AppError::database("A revision candidate is invalid", error))?,
            );
        }
    }
    Ok(result)
}

fn candidate_from_row(row: &rusqlite::Row<'_>, today: NaiveDate) -> rusqlite::Result<DueCandidate> {
    let due: String = row.get(1)?;
    let mastery: f64 = row.get(2)?;
    let overdue_days = date_only::parse_date_only(&due)
        .map(|date| (today - date).num_days().max(0))
        .unwrap_or(0);
    let triage = (40.0 + overdue_days as f64 * 8.0 + (100.0 - mastery) * 0.3)
        .clamp(0.0, 100.0)
        .round();
    let lane = if overdue_days > 0 || mastery < 40.0 {
        "urgent_review"
    } else if mastery < 70.0 {
        "strengthen"
    } else {
        "maintain"
    };
    Ok(DueCandidate {
        concept_id: row.get(0)?,
        estimated_minutes: 30,
        triage_score: triage,
        lane: lane.to_owned(),
        source_due_date: due,
    })
}

fn load_active_plan(
    connection: &Connection,
    plan_date: NaiveDate,
    exam_id: Option<i64>,
) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT id FROM revision_daily_plans
             WHERE plan_date = ?1 AND exam_id IS ?2 AND status = 'active'
             ORDER BY version DESC LIMIT 1",
            params![date_only::format_date_only(plan_date), exam_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Revision plan could not be loaded", error))
}

fn load_plan(connection: &Connection, plan_id: i64) -> Result<V2RevisionDailyPlan, AppError> {
    let (plan_date, exam_id, version, budget, max_topics, planned_minutes, completed, status) =
        connection
            .query_row(
                "SELECT plan_date, exam_id, version, effective_budget_minutes, max_topics,
                        planned_minutes, completed_minutes, status
                 FROM revision_daily_plans WHERE id = ?1",
                [plan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, f64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .map_err(|error| AppError::database("Revision plan could not be loaded", error))?;
    let plan_date_value = date_only::parse_date_only(&plan_date)
        .ok_or_else(|| AppError::database_unavailable("The revision plan date is invalid."))?;
    let mut statement = connection
        .prepare(
            "SELECT p.id, p.concept_id, c.title, c.domain, i.membership_status,
                    i.lane, i.triage_score, i.estimated_minutes, p.source_due_date,
                    i.position, i.planned_block_id, i.planned_block_title,
                    cm.mastery_score
             FROM revision_daily_plan_items i
             JOIN revision_plan_items p ON p.id = i.plan_item_id
             JOIN concepts c ON c.id = p.concept_id
             LEFT JOIN concept_mastery cm ON cm.concept_id = p.concept_id
             WHERE i.daily_plan_id = ?1
             ORDER BY i.position, p.id",
        )
        .map_err(|error| AppError::database("Revision plan items could not be prepared", error))?;
    let rows = statement
        .query_map([plan_id], |row| {
            let source_due_date: String = row.get(8)?;
            Ok(V2RevisionPlanItem {
                plan_item_id: row.get(0)?,
                concept_id: row.get(1)?,
                concept_title: row.get(2)?,
                concept_domain: row.get(3)?,
                membership_status: row.get(4)?,
                lane: row.get(5)?,
                triage_score: row.get(6)?,
                estimated_minutes: row.get(7)?,
                source_due_date: source_due_date.clone(),
                position: row.get(9)?,
                planned_block_id: row.get(10)?,
                planned_block_title: row.get(11)?,
                selection_reason: revision_reason(&source_due_date, plan_date_value, row.get(12)?),
            })
        })
        .map_err(|error| AppError::database("Revision plan items could not be read", error))?;
    let items = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A revision plan item is invalid", error))?;
    Ok(V2RevisionDailyPlan {
        id: plan_id,
        plan_date,
        exam_id,
        version,
        effective_budget_minutes: budget,
        max_topics,
        planned_minutes,
        completed_minutes: completed,
        status,
        items,
    })
}

fn active_plan_item_for_concept(
    connection: &Connection,
    concept_id: i64,
) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT i.id FROM revision_plan_items i
             JOIN revision_daily_plans p ON p.id = i.active_daily_plan_id AND p.status = 'active'
             WHERE i.concept_id = ?1
               AND i.status IN ('planned','in_progress')
             ORDER BY i.id DESC LIMIT 1",
            [concept_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Active revision item could not be loaded", error))
}

fn validate_active_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    let active = connection
        .query_row(
            "SELECT is_active FROM exam_profiles WHERE exam_id = ?1",
            [exam_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Examination could not be checked", error))?;
    match active {
        None => Err(AppError::validation(
            "examId",
            "The selected examination does not exist.",
        )),
        Some(0) => Err(AppError::forbidden(
            "examId",
            "Inactive examinations cannot receive new revision work.",
        )),
        Some(_) => Ok(()),
    }
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;
    use crate::v2::concepts::{self, V2ConceptMappingUpdateInput};

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
            + chrono::Duration::seconds(seconds)
    }

    fn concept_id(state: &V2State) -> i64 {
        state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM concepts WHERE slug = 'kinematics'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("test concept", error))
            })
            .expect("concept")
    }

    fn add_mapping(connection: &Connection, concept_id: i64, exam_id: i64) -> i64 {
        let now = "2026-11-04T10:00:00Z";
        connection
            .execute(
                "INSERT INTO exam_content_versions (
                    exam_id, content_type, version_label, source_reference,
                    review_status, created_at
                 ) VALUES (?1, 'syllabus', ?2, 'fixture', 'verified', ?3)",
                params![exam_id, format!("v1-{exam_id}"), now],
            )
            .unwrap();
        let version = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_sources (
                    exam_id, content_version_id, code, title, publisher,
                    source_reference, verified_on, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'Source', 'Publisher', 'fixture',
                           '2026-11-04', ?4, ?4)",
                params![exam_id, version, format!("S{exam_id}"), now],
            )
            .unwrap();
        let source = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes (
                    exam_id, source_id, code, title, node_kind, display_order,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, 'topic', 1, ?5, ?5)",
                params![
                    exam_id,
                    source,
                    format!("T{exam_id}"),
                    format!("Kinematics {exam_id}"),
                    now
                ],
            )
            .unwrap();
        let node = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_node_concepts (
                    exam_id, node_id, concept_id, weight, created_at, status, origin,
                    source_reference, provenance, content_version_id, reviewed_at,
                    reviewed_by, updated_at
                 ) VALUES (?1, ?2, ?3, 1, ?4, 'approved', 'curated', 'fixture',
                           'test approved mapping', ?5, ?4, 'tester', ?4)",
                params![exam_id, node, concept_id, now, version],
            )
            .unwrap();
        node
    }

    #[test]
    fn schedules_complete_defer_and_record_mastery_evidence() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        state
            .with_connection(|connection| {
                add_mapping(connection, concept, 1);
                Ok(())
            })
            .expect("mapping");
        let scheduled = state
            .with_connection(|connection| {
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: "Start".to_owned(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )
            })
            .expect("schedule");
        assert_eq!(scheduled.next_revision_date, "2026-11-04");

        let dashboard = state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("dashboard");
        assert_eq!(dashboard.stats.due_today, 1);
        assert!(dashboard.recent_history.is_empty());

        let completed = state
            .with_connection(|connection| {
                complete_revision(
                    connection,
                    V2CompleteRevisionInput {
                        concept_id: concept,
                        outcome: "good".to_owned(),
                        revision_date: "2026-11-04".to_owned(),
                        trigger_exam_id: None,
                        notes: "Completed".to_owned(),
                        plan_item_id: None,
                        session_id: None,
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(1),
                )
            })
            .expect("complete");
        assert_eq!(completed.recent_history.len(), 1);
        assert_eq!(completed.recent_history[0].outcome, "good");
        assert!(
            completed.recent_history[0].mastery_after >= completed.recent_history[0].mastery_before
        );
        let evidence_count = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM concept_evidence
                         WHERE concept_id = ?1 AND source_kind = 'revision'",
                        [concept],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("test evidence", error))
            })
            .expect("evidence count");
        assert_eq!(evidence_count, 1);

        let rescheduled = state
            .with_connection(|connection| {
                reschedule(
                    connection,
                    V2RevisionRescheduleInput {
                        concept_id: concept,
                        next_revision_date: "2026-11-20".to_owned(),
                    },
                    date_only::parse_date_only("2026-11-05").unwrap(),
                    at(2),
                )
            })
            .expect("reschedule");
        assert_eq!(rescheduled.next_revision_date, "2026-11-20");

        let deferred = state
            .with_connection(|connection| {
                defer_revision(
                    connection,
                    V2DeferRevisionInput {
                        concept_id: concept,
                        new_revision_date: "2026-11-25".to_owned(),
                        reason: "time".to_owned(),
                        notes: "Busy".to_owned(),
                        plan_item_id: None,
                    },
                    date_only::parse_date_only("2026-11-05").unwrap(),
                    at(3),
                )
            })
            .expect("defer");
        let schedule = deferred
            .upcoming
            .iter()
            .find(|schedule| schedule.concept_id == concept)
            .expect("upcoming");
        assert_eq!(schedule.next_revision_date, "2026-11-25");
        assert!(
            schedule
                .selection_reason
                .starts_with("Scheduled for 2026-11-25")
        );
    }

    #[test]
    fn daily_plan_generation_completion_and_rebalance_use_mapped_concepts() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        state
            .with_connection(|connection| {
                add_mapping(connection, concept, 1);
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: String::new(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )
            })
            .expect("schedule");

        let plan = state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(1),
                )
            })
            .expect("plan");
        assert_eq!(plan.exam_id, Some(1));
        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].concept_id, concept);
        let plan_item_id = plan.items[0].plan_item_id;

        let rebalanced = state
            .with_connection(|connection| {
                rebalance_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(2),
                )
            })
            .expect("rebalance");
        assert_eq!(rebalanced.version, 2);
        assert_eq!(rebalanced.items.len(), 1);
        assert_eq!(rebalanced.items[0].concept_id, concept);

        let completed = state
            .with_connection(|connection| {
                complete_revision(
                    connection,
                    V2CompleteRevisionInput {
                        concept_id: concept,
                        outcome: "easy".to_owned(),
                        revision_date: "2026-11-04".to_owned(),
                        trigger_exam_id: Some(1),
                        notes: String::new(),
                        plan_item_id: Some(plan_item_id),
                        session_id: None,
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(3),
                )
            })
            .expect("complete plan item");
        assert!(completed.stats.completed_today >= 1);

        let active = state
            .with_connection(|connection| {
                get_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                )
            })
            .expect("active plan");
        assert!(active.is_some());
    }

    #[test]
    fn revision_scope_indexes_are_installed() {
        let (_root, state) = state();
        let names: Vec<String> = state
            .with_connection(|connection| {
                connection
                    .prepare(
                        "SELECT name FROM sqlite_master
                         WHERE type = 'index' AND tbl_name = 'revision_plan_items'
                         ORDER BY name",
                    )
                    .map_err(|error| AppError::database("index list", error))?
                    .query_map([], |row| row.get(0))
                    .map_err(|error| AppError::database("index list", error))?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| AppError::database("index list", error))
            })
            .expect("index list");
        assert!(
            names
                .iter()
                .any(|name| name == "ux_revision_plan_active_combined_concept")
        );
        assert!(
            names
                .iter()
                .any(|name| name == "ux_revision_plan_active_exam_concept")
        );
        assert!(
            !names
                .iter()
                .any(|name| name == "ux_revision_plan_active_concept")
        );
    }

    #[test]
    fn approved_mappings_drive_combined_and_exam_specific_plans_without_duplicates() {
        let (_root, combined_state) = state();
        let combined_concept = concept_id(&combined_state);
        combined_state
            .with_connection(|connection| {
                add_mapping(connection, combined_concept, 1);
                add_mapping(connection, combined_concept, 2);
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: combined_concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: String::new(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )
            })
            .expect("combined fixture");

        let combined = combined_state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("combined dashboard");
        assert_eq!(combined.stats.due_today, 1);
        assert_eq!(combined.due_today.len(), 1);
        assert!(combined.due_today[0].selection_reason.contains("Due today"));
        assert!(
            combined.due_today[0]
                .selection_reason
                .contains("approved concept-to-syllabus mapping")
        );

        let combined_plan = combined_state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: None,
                    },
                    at(1),
                )
            })
            .expect("combined plan");
        assert_eq!(combined_plan.items.len(), 1);
        assert_eq!(combined_plan.items[0].concept_id, combined_concept);
        assert!(
            combined_plan.items[0]
                .selection_reason
                .contains("Due today")
        );

        let specific = combined_state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: Some(1),
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("exam dashboard");
        assert_eq!(specific.stats.due_today, 1);
        let specific_plan = combined_state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(2),
                )
            })
            .expect("exam plan");
        assert_eq!(specific_plan.items.len(), 1);
        assert_eq!(specific_plan.items[0].concept_id, combined_concept);
    }

    #[test]
    fn proposed_mappings_are_not_eligible_until_approved() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        state
            .with_connection(|connection| {
                add_mapping(connection, concept, 1);
                connection
                    .execute(
                        "UPDATE exam_node_concepts SET status = 'proposed'
                         WHERE concept_id = ?1 AND exam_id = 1",
                        [concept],
                    )
                    .map_err(|error| AppError::database("propose mapping", error))?;
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: String::new(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )?;
                Ok(())
            })
            .expect("proposed mapping schedule");

        let before = state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("proposed dashboard");
        assert_eq!(before.stats.due_today, 0);

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_node_concepts SET status = 'approved'
                         WHERE concept_id = ?1 AND exam_id = 1",
                        [concept],
                    )
                    .map_err(|error| AppError::database("approve mapping", error))?;
                Ok(())
            })
            .expect("approve mapping");
        let after = state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("approved dashboard");
        assert_eq!(after.stats.due_today, 1);
    }

    #[test]
    fn approving_a_mapping_creates_and_preserves_an_initial_schedule() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        let node = state
            .with_connection(|connection| {
                let node = add_mapping(connection, concept, 1);
                connection
                    .execute(
                        "UPDATE exam_node_concepts SET status = 'proposed'
                         WHERE concept_id = ?1 AND exam_id = 1",
                        [concept],
                    )
                    .map_err(|error| AppError::database("propose mapping", error))?;
                Ok(node)
            })
            .expect("proposed mapping");
        let before: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM revision_schedules WHERE concept_id = ?1",
                        [concept],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("schedule count", error))
            })
            .expect("schedule count");
        assert_eq!(before, 0);

        let detail = state
            .with_connection(|connection| {
                concepts::update_mapping(
                    connection,
                    V2ConceptMappingUpdateInput {
                        exam_id: 1,
                        node_id: node,
                        concept_id: concept,
                        status: "approved".to_owned(),
                        notes: Some("Approved locally.".to_owned()),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )
            })
            .expect("approve mapping");
        assert!(
            detail
                .mappings
                .iter()
                .find(|mapping| mapping.node_id == node)
                .is_some_and(|mapping| mapping.is_approved)
        );
        let (next, interval, completed, updated) = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT next_revision_date, current_interval_days,
                                completed_revisions, updated_at
                         FROM revision_schedules WHERE concept_id = ?1",
                        [concept],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, i64>(1)?,
                                row.get::<_, i64>(2)?,
                                row.get::<_, String>(3)?,
                            ))
                        },
                    )
                    .map_err(|error| AppError::database("initial schedule", error))
            })
            .expect("initial schedule");
        assert_eq!(next, "2026-11-09");
        assert_eq!(interval, 5);
        assert_eq!(completed, 0);

        state
            .with_connection(|connection| {
                concepts::update_mapping(
                    connection,
                    V2ConceptMappingUpdateInput {
                        exam_id: 1,
                        node_id: node,
                        concept_id: concept,
                        status: "approved".to_owned(),
                        notes: Some("Approved again.".to_owned()),
                    },
                    date_only::parse_date_only("2026-11-20").unwrap(),
                    at(60),
                )
            })
            .expect("repeat approval");
        let (next_after, completed_after, updated_after) = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT next_revision_date, completed_revisions, updated_at
                         FROM revision_schedules WHERE concept_id = ?1",
                        [concept],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, i64>(1)?,
                                row.get::<_, String>(2)?,
                            ))
                        },
                    )
                    .map_err(|error| AppError::database("preserved schedule", error))
            })
            .expect("preserved schedule");
        assert_eq!(next_after, next);
        assert_eq!(completed_after, 0);
        assert_eq!(updated_after, updated);
    }

    #[test]
    fn proposed_schedules_are_reported_as_awaiting_approval_until_approved() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        let node = state
            .with_connection(|connection| {
                let node = add_mapping(connection, concept, 1);
                connection
                    .execute(
                        "UPDATE exam_node_concepts SET status = 'proposed'
                         WHERE concept_id = ?1 AND exam_id = 1",
                        [concept],
                    )
                    .map_err(|error| AppError::database("propose mapping", error))?;
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: String::new(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )?;
                Ok(node)
            })
            .expect("proposed schedule");
        let before = state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("proposed dashboard");
        assert_eq!(before.stats.due_today, 0);
        assert_eq!(before.awaiting_approval.len(), 1);
        assert_eq!(before.awaiting_approval[0].concept_id, concept);
        assert!(!before.awaiting_approval[0].eligible);
        assert_eq!(
            before.awaiting_approval[0].pending_exam_codes,
            vec!["csir-net-physics"]
        );

        state
            .with_connection(|connection| {
                concepts::update_mapping(
                    connection,
                    V2ConceptMappingUpdateInput {
                        exam_id: 1,
                        node_id: node,
                        concept_id: concept,
                        status: "approved".to_owned(),
                        notes: None,
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(1),
                )
            })
            .expect("approve proposed mapping");
        let after = state
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-04".to_owned(),
                    },
                )
            })
            .expect("approved dashboard");
        assert_eq!(after.stats.due_today, 1);
        assert!(after.awaiting_approval.is_empty());
        assert_eq!(after.due_today[0].next_revision_date, "2026-11-04");
        assert!(after.due_today[0].eligible);
        assert_eq!(
            after.due_today[0].approved_exam_codes,
            vec!["csir-net-physics"]
        );
    }

    #[test]
    fn repeated_schedule_requests_do_not_reset_progress_or_due_dates() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        state
            .with_connection(|connection| {
                add_mapping(connection, concept, 1);
                Ok(())
            })
            .expect("mapping");
        let first = state
            .with_connection(|connection| {
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-10".to_owned()),
                        notes: "First schedule".to_owned(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(0),
                )
            })
            .expect("first schedule");
        assert!(!first.already_scheduled);

        let completed = state
            .with_connection(|connection| {
                complete_revision(
                    connection,
                    V2CompleteRevisionInput {
                        concept_id: concept,
                        outcome: "good".to_owned(),
                        revision_date: "2026-11-10".to_owned(),
                        trigger_exam_id: None,
                        notes: String::new(),
                        plan_item_id: None,
                        session_id: None,
                    },
                    date_only::parse_date_only("2026-11-10").unwrap(),
                    at(1),
                )
            })
            .expect("complete revision");
        let next_after_completion = completed
            .upcoming
            .iter()
            .find(|schedule| schedule.concept_id == concept)
            .expect("next upcoming")
            .next_revision_date
            .clone();

        let repeated = state
            .with_connection(|connection| {
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-12-01".to_owned()),
                        notes: "Should not reset".to_owned(),
                    },
                    date_only::parse_date_only("2026-11-11").unwrap(),
                    at(2),
                )
            })
            .expect("repeated schedule");
        assert!(repeated.already_scheduled);
        assert_eq!(repeated.next_revision_date, next_after_completion);
        assert_eq!(repeated.completed_revisions, 1);
        let history_count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM revision_history WHERE concept_id = ?1",
                        [concept],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("history count", error))
            })
            .expect("history count");
        assert_eq!(history_count, 1);
    }

    #[test]
    fn generating_an_existing_plan_reconciles_newly_due_concepts_without_duplicates() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        state
            .with_connection(|connection| {
                add_mapping(connection, concept, 1);
                Ok(())
            })
            .expect("mapping");
        let empty = state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(0),
                )
            })
            .expect("empty plan");
        assert_eq!(empty.version, 1);
        assert!(empty.items.is_empty());

        state
            .with_connection(|connection| {
                schedule_concept(
                    connection,
                    V2RevisionScheduleInput {
                        concept_id: concept,
                        next_revision_date: Some("2026-11-04".to_owned()),
                        notes: String::new(),
                    },
                    date_only::parse_date_only("2026-11-04").unwrap(),
                    at(1),
                )?;
                Ok(())
            })
            .expect("due schedule");

        let reconciled = state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(2),
                )
            })
            .expect("reconciled plan");
        assert_eq!(reconciled.version, 1);
        assert_eq!(reconciled.items.len(), 1);
        assert_eq!(reconciled.items[0].concept_id, concept);
        assert_eq!(reconciled.planned_minutes, 30);

        let repeated = state
            .with_connection(|connection| {
                generate_daily_plan(
                    connection,
                    V2RevisionPlanInput {
                        plan_date: "2026-11-04".to_owned(),
                        exam_id: Some(1),
                    },
                    at(3),
                )
            })
            .expect("repeated plan");
        assert_eq!(repeated.version, 1);
        assert_eq!(repeated.items.len(), 1);
        assert_eq!(repeated.planned_minutes, 30);
    }

    #[test]
    fn schedules_and_history_survive_reopening_the_database() {
        let root = tempfile::tempdir().expect("temporary directory");
        let path = root.path().join("v2");
        {
            let state = V2State::initialize(&path);
            let concept = concept_id(&state);
            state
                .with_connection(|connection| {
                    add_mapping(connection, concept, 1);
                    schedule_concept(
                        connection,
                        V2RevisionScheduleInput {
                            concept_id: concept,
                            next_revision_date: Some("2026-11-05".to_owned()),
                            notes: "Persisted schedule".to_owned(),
                        },
                        date_only::parse_date_only("2026-11-04").unwrap(),
                        at(0),
                    )?;
                    complete_revision(
                        connection,
                        V2CompleteRevisionInput {
                            concept_id: concept,
                            outcome: "good".to_owned(),
                            revision_date: "2026-11-05".to_owned(),
                            trigger_exam_id: None,
                            notes: String::new(),
                            plan_item_id: None,
                            session_id: None,
                        },
                        date_only::parse_date_only("2026-11-05").unwrap(),
                        at(1),
                    )?;
                    Ok(())
                })
                .expect("persisted revision");
        }

        let reopened = V2State::initialize(&path);
        let dashboard = reopened
            .with_connection(|connection| {
                dashboard(
                    connection,
                    V2RevisionDashboardInput {
                        exam_id: None,
                        today: "2026-11-05".to_owned(),
                    },
                )
            })
            .expect("reopened dashboard");
        let schedule = dashboard
            .upcoming
            .iter()
            .find(|schedule| schedule.concept_title == "Kinematics")
            .expect("persisted upcoming schedule");
        assert_eq!(schedule.completed_revisions, 1);
        assert_eq!(schedule.previous_outcome.as_deref(), Some("good"));
        assert_eq!(dashboard.recent_history.len(), 1);
        assert_eq!(dashboard.recent_history[0].outcome, "good");
    }
}
