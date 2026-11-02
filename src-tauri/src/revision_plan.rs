use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use crate::{
    analytics, date_only,
    error::AppError,
    revision::{self, EvidenceClass, RevisionRecommendation, RevisionTopic, RevisionTopicState},
    settings,
};

const DEFAULT_FOCUS_MINUTES: i64 = 25;
const MIN_ESTIMATE_MINUTES: i64 = 10;
const MAX_ESTIMATE_MINUTES: i64 = 120;
const ESTIMATE_ROUNDING: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionPlanBlockSnapshot {
    pub id: Option<i64>,
    pub title: Option<String>,
    pub calendar_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionPlanMembership {
    pub daily_plan_id: i64,
    pub plan_item_id: i64,
    pub topic: RevisionTopic,
    pub position: i64,
    pub estimated_minutes: i64,
    pub lane: String,
    pub triage_score: f64,
    pub membership_status: String,
    pub item_status: String,
    pub actual_seconds: i64,
    pub remaining_estimate_minutes: f64,
    pub carried_from_daily_plan_id: Option<i64>,
    pub planned_block: RevisionPlanBlockSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionBacklogItem {
    pub plan_item_id: i64,
    pub topic: RevisionTopic,
    pub status: String,
    pub estimated_minutes: i64,
    pub lane: String,
    pub triage_score: f64,
    pub source_due_date: String,
    pub actual_seconds: i64,
    pub remaining_estimate_minutes: f64,
    pub deferred_until: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDeferredItem {
    pub topic: RevisionTopic,
    pub plan_item_id: Option<i64>,
    pub original_due_date: String,
    pub previous_due_date: String,
    pub deferred_until: String,
    pub deferral_count: i64,
    pub latest_reason: String,
    pub latest_notes: String,
    pub latest_occurred_on: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionUnscheduledUrgent {
    pub topic: RevisionTopic,
    pub estimate_minutes: i64,
    pub reason: String,
    pub lane: String,
    pub triage_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDailyPlan {
    pub today: String,
    pub plan_id: Option<i64>,
    pub plan_date: Option<String>,
    pub version: Option<i64>,
    pub status: Option<String>,
    pub needs_generation: bool,
    pub effective_budget_minutes: i64,
    pub max_topics: i64,
    pub planned_minutes: i64,
    pub completed_minutes: f64,
    pub actual_minutes_today: f64,
    pub remaining_work_minutes: f64,
    pub remaining_daily_budget_minutes: f64,
    pub over_budget: bool,
    pub over_limit: bool,
    pub scheduled: Vec<RevisionPlanMembership>,
    pub completed: Vec<RevisionPlanMembership>,
    pub backlog: Vec<RevisionBacklogItem>,
    pub deferred: Vec<RevisionDeferredItem>,
    pub unscheduled_urgent: Vec<RevisionUnscheduledUrgent>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionPlanHistoryEntry {
    pub plan_id: i64,
    pub plan_date: String,
    pub version: i64,
    pub status: String,
    pub effective_budget_minutes: i64,
    pub max_topics: i64,
    pub planned_minutes: i64,
    pub completed_minutes: f64,
    pub created_at: String,
    pub last_rebalanced_at: Option<String>,
    pub memberships: Vec<RevisionPlanMembership>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDeferralEvent {
    pub id: i64,
    pub topic_id: i64,
    pub plan_item_id: Option<i64>,
    pub event_kind: String,
    pub overridden_deferral_id: Option<i64>,
    pub previous_due_date: String,
    pub new_due_date: String,
    pub occurred_on: String,
    pub reason: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionDeferralResult {
    pub plan: RevisionDailyPlan,
    pub event: RevisionDeferralEvent,
    pub schedule: revision::RevisionSchedule,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionIntervalDecision {
    pub interval_before_days: i64,
    pub interval_after_days: i64,
    pub timing_days: i64,
    pub timing_factor: f64,
    pub evidence_status: String,
    pub evidence_factor: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateGroup {
    Backlog,
    Scheduled,
    Optional,
}

struct Candidate {
    topic: RevisionTopic,
    estimate_minutes: i64,
    lane: String,
    triage_score: f64,
    days_overdue: i64,
    next_due_date: Option<String>,
    priority_need: f64,
    deferral_count: i64,
    group: CandidateGroup,
    urgent: bool,
    topic_id: i64,
    code: String,
}

struct PlanContext {
    settings: revision::RevisionSettings,
    states: Vec<RevisionTopicState>,
    recommendations: HashMap<i64, RevisionRecommendation>,
    focus_minutes: i64,
    total_study_target_minutes: i64,
}

#[derive(Debug, Clone)]
struct WorkItemRow {
    id: i64,
    topic_id: i64,
    status: String,
    estimated_minutes: i64,
    triage_score: f64,
    lane: String,
    planned_block_id: Option<i64>,
    revision_history_id: Option<i64>,
    removed_at: Option<String>,
}

#[derive(Debug, Clone)]
struct PlanHeader {
    id: i64,
    plan_date: String,
    version: i64,
    effective_budget_minutes: i64,
    max_topics: i64,
    planned_minutes: i64,
    status: String,
    created_at: String,
    last_rebalanced_at: Option<String>,
}

#[derive(Debug, Clone)]
struct MembershipRow {
    daily_plan_id: i64,
    plan_item_id: i64,
    topic_id: i64,
    position: i64,
    estimated_minutes: i64,
    lane: String,
    triage_score: f64,
    membership_status: String,
    planned_block_id: Option<i64>,
    planned_block_title: Option<String>,
    planned_block_date: Option<String>,
    planned_block_start_time: Option<String>,
    planned_block_end_time: Option<String>,
    carried_from_daily_plan_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct SelectedItem {
    work_item_id: i64,
    topic_id: i64,
    position: i64,
    estimated_minutes: i64,
    lane: String,
    triage_score: f64,
    membership_status: String,
    planned_block_id: Option<i64>,
    planned_block_title: Option<String>,
    planned_block_date: Option<String>,
    planned_block_start_time: Option<String>,
    planned_block_end_time: Option<String>,
    carried_from_daily_plan_id: Option<i64>,
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

fn load_context(connection: &Connection, today: NaiveDate) -> Result<PlanContext, AppError> {
    let settings = revision::load_settings(connection)?;
    let states = revision::topic_states(connection, today)?;
    let evidence_from = analytics::evidence_cutoff(today);
    let evidence_to = date_only::format_date_only(today);
    let performance =
        analytics::topic_performance(connection, None, None, &evidence_from, &evidence_to)?;
    let performance_by_topic = performance
        .into_iter()
        .map(|entry| (entry.topic_id, entry))
        .collect::<HashMap<_, _>>();
    let recommendations = revision::ranked_recommendations(&states, &performance_by_topic)
        .into_iter()
        .map(|recommendation| (recommendation.topic.id, recommendation))
        .collect::<HashMap<_, _>>();
    let focus_minutes: i64 = connection
        .query_row(
            "SELECT focus_minutes FROM timer_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Timer settings could not be loaded", error))?
        .unwrap_or(DEFAULT_FOCUS_MINUTES);
    let application_settings = settings::load(connection)?;

    Ok(PlanContext {
        settings,
        states,
        recommendations,
        focus_minutes,
        total_study_target_minutes: application_settings.daily_study_target_minutes,
    })
}

fn effective_budget(settings: &revision::RevisionSettings, total_minutes: i64) -> i64 {
    settings.daily_revision_minutes.min(total_minutes).max(1)
}

fn estimate_minutes(
    connection: &Connection,
    topic_id: i64,
    fallback_focus_minutes: i64,
) -> Result<i64, AppError> {
    let linked = connection
        .prepare(
            "SELECT ss.duration_seconds
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             JOIN revision_plan_items pi ON pi.id = rsl.plan_item_id
             WHERE pi.topic_id = ?1
             ORDER BY ss.session_date DESC, ss.id DESC
             LIMIT 5",
        )
        .map_err(|error| {
            AppError::database("Revision duration evidence could not be prepared", error)
        })?
        .query_map([topic_id], |row| row.get::<_, i64>(0))
        .map_err(|error| {
            AppError::database("Revision duration evidence could not be queried", error)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| {
            AppError::database("Revision duration evidence could not be read", error)
        })?;
    let seconds = if linked.len() >= 3 {
        Some(linked)
    } else {
        let topic = connection
            .prepare(
                "SELECT duration_seconds
                 FROM study_sessions
                 WHERE topic_id = ?1
                 ORDER BY session_date DESC, id DESC
                 LIMIT 5",
            )
            .map_err(|error| {
                AppError::database("Topic duration evidence could not be prepared", error)
            })?
            .query_map([topic_id], |row| row.get::<_, i64>(0))
            .map_err(|error| {
                AppError::database("Topic duration evidence could not be queried", error)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| {
                AppError::database("Topic duration evidence could not be read", error)
            })?;
        if topic.len() >= 3 {
            Some(topic)
        } else {
            let global = connection
                .prepare(
                    "SELECT duration_seconds
                     FROM study_sessions
                     WHERE source = 'timer'
                     ORDER BY session_date DESC, id DESC
                     LIMIT 20",
                )
                .map_err(|error| {
                    AppError::database("Global duration evidence could not be prepared", error)
                })?
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(|error| {
                    AppError::database("Global duration evidence could not be queried", error)
                })?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    AppError::database("Global duration evidence could not be read", error)
                })?;
            if global.len() >= 3 {
                Some(global)
            } else {
                None
            }
        }
    };

    let minutes = if let Some(mut values) = seconds {
        values.sort_unstable();
        let median = values[values.len() / 2] as f64 / 60.0;
        median.round() as i64
    } else {
        fallback_focus_minutes
    };
    Ok(
        ((minutes.max(MIN_ESTIMATE_MINUTES) + ESTIMATE_ROUNDING / 2) / ESTIMATE_ROUNDING
            * ESTIMATE_ROUNDING)
            .clamp(MIN_ESTIMATE_MINUTES, MAX_ESTIMATE_MINUTES),
    )
}

fn load_active_plan(
    connection: &Connection,
    today: NaiveDate,
) -> Result<Option<PlanHeader>, AppError> {
    let today_text = date_only::format_date_only(today);
    connection
        .query_row(
            "SELECT id, plan_date, version, effective_budget_minutes, max_topics,
                    planned_minutes, status, created_at, last_rebalanced_at
             FROM revision_daily_plans
             WHERE plan_date = ?1 AND status = 'active'",
            [today_text],
            |row| {
                Ok(PlanHeader {
                    id: row.get(0)?,
                    plan_date: row.get(1)?,
                    version: row.get(2)?,
                    effective_budget_minutes: row.get(3)?,
                    max_topics: row.get(4)?,
                    planned_minutes: row.get(5)?,
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    last_rebalanced_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("The active revision plan could not be loaded", error))
}

fn load_memberships(connection: &Connection, plan_id: i64) -> Result<Vec<MembershipRow>, AppError> {
    connection
        .prepare(
            "SELECT dpi.daily_plan_id, dpi.plan_item_id, pi.topic_id, dpi.position,
                    dpi.estimated_minutes, dpi.lane, dpi.triage_score,
                    dpi.membership_status, dpi.planned_block_id,
                    dpi.planned_block_title, dpi.planned_block_date,
                    dpi.planned_block_start_time, dpi.planned_block_end_time,
                    dpi.carried_from_daily_plan_id
             FROM revision_daily_plan_items dpi
             JOIN revision_plan_items pi ON pi.id = dpi.plan_item_id
             WHERE dpi.daily_plan_id = ?1
             ORDER BY dpi.position, dpi.plan_item_id",
        )
        .map_err(|error| AppError::database("Plan memberships could not be prepared", error))?
        .query_map([plan_id], |row| {
            Ok(MembershipRow {
                daily_plan_id: row.get(0)?,
                plan_item_id: row.get(1)?,
                topic_id: row.get(2)?,
                position: row.get(3)?,
                estimated_minutes: row.get(4)?,
                lane: row.get(5)?,
                triage_score: row.get(6)?,
                membership_status: row.get(7)?,
                planned_block_id: row.get(8)?,
                planned_block_title: row.get(9)?,
                planned_block_date: row.get(10)?,
                planned_block_start_time: row.get(11)?,
                planned_block_end_time: row.get(12)?,
                carried_from_daily_plan_id: row.get(13)?,
            })
        })
        .map_err(|error| AppError::database("Plan memberships could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Plan memberships could not be read", error))
}

fn selected_for_version(
    connection: &Connection,
    plan_id: i64,
    states: &[RevisionTopicState],
) -> Result<Vec<SelectedItem>, AppError> {
    let state_topics = states
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();
    let mut result = Vec::new();
    for row in load_memberships(connection, plan_id)? {
        if state_topics.contains_key(&row.topic_id) {
            result.push(selected_from_membership(&row, &row.membership_status));
        }
    }
    Ok(result)
}

fn append_completed(selected: &mut Vec<SelectedItem>, mut completed: Vec<SelectedItem>) {
    let start = selected.len() as i64;
    for (offset, item) in completed.iter_mut().enumerate() {
        item.position = start + offset as i64;
        selected.push(item.clone());
    }
}

fn in_progress_selected(
    connection: &Connection,
    states: &[RevisionTopicState],
    today: NaiveDate,
) -> Result<Vec<SelectedItem>, AppError> {
    let today_text = date_only::format_date_only(today);
    let state_topics = states
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();
    let work_items = load_active_work_items(connection)?;
    let mut result = Vec::new();
    for item in work_items.values() {
        if item.status != "in_progress" {
            continue;
        }
        let Some(state) = state_topics.get(&item.topic_id) else {
            continue;
        };
        if state.topic.archived || state.due_state == "suspended" {
            continue;
        }
        if state
            .schedule
            .as_ref()
            .and_then(|schedule| schedule.deferred_until.as_deref())
            .is_some_and(|date| date > today_text.as_str())
        {
            continue;
        }
        result.push(selected_from_work_item(
            connection,
            item,
            result.len() as i64,
            "in_progress",
        )?);
    }
    result.sort_by_key(|item| item.work_item_id);
    Ok(result)
}

fn load_work_item(connection: &Connection, id: i64) -> Result<Option<WorkItemRow>, AppError> {
    connection
        .query_row(
            "SELECT id, topic_id, status, estimated_minutes, triage_score, lane,
                    planned_block_id, revision_history_id, removed_at
             FROM revision_plan_items WHERE id = ?1",
            [id],
            |row| {
                Ok(WorkItemRow {
                    id: row.get(0)?,
                    topic_id: row.get(1)?,
                    status: row.get(2)?,
                    estimated_minutes: row.get(3)?,
                    triage_score: row.get(4)?,
                    lane: row.get(5)?,
                    planned_block_id: row.get(6)?,
                    revision_history_id: row.get(7)?,
                    removed_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("A revision work item could not be loaded", error))
}

fn active_work_item(
    connection: &Connection,
    topic_id: i64,
) -> Result<Option<WorkItemRow>, AppError> {
    connection
        .query_row(
            "SELECT id, topic_id, status, estimated_minutes, triage_score, lane,
                    planned_block_id, revision_history_id, removed_at
             FROM revision_plan_items
             WHERE topic_id = ?1
               AND status IN ('planned', 'in_progress', 'backlog', 'deferred')
             ORDER BY id DESC LIMIT 1",
            [topic_id],
            |row| {
                Ok(WorkItemRow {
                    id: row.get(0)?,
                    topic_id: row.get(1)?,
                    status: row.get(2)?,
                    estimated_minutes: row.get(3)?,
                    triage_score: row.get(4)?,
                    lane: row.get(5)?,
                    planned_block_id: row.get(6)?,
                    revision_history_id: row.get(7)?,
                    removed_at: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| {
            AppError::database("The active revision work item could not be loaded", error)
        })
}

fn load_active_work_items(connection: &Connection) -> Result<HashMap<i64, WorkItemRow>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, topic_id, status, estimated_minutes, triage_score, lane,
                    planned_block_id, revision_history_id, removed_at
             FROM revision_plan_items
             WHERE status IN ('planned', 'in_progress', 'backlog', 'deferred')",
        )
        .map_err(|error| {
            AppError::database("Active revision work items could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(WorkItemRow {
                id: row.get(0)?,
                topic_id: row.get(1)?,
                status: row.get(2)?,
                estimated_minutes: row.get(3)?,
                triage_score: row.get(4)?,
                lane: row.get(5)?,
                planned_block_id: row.get(6)?,
                revision_history_id: row.get(7)?,
                removed_at: row.get(8)?,
            })
        })
        .map_err(|error| {
            AppError::database("Active revision work items could not be queried", error)
        })?;
    let mut result = HashMap::new();
    for row in rows {
        let row = row.map_err(|error| {
            AppError::database("Active revision work item could not be read", error)
        })?;
        result.insert(row.topic_id, row);
    }
    Ok(result)
}

fn deferral_count(connection: &Connection, topic_id: i64) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT count(*) FROM revision_deferrals
             WHERE topic_id = ?1 AND event_kind = 'defer'",
            [topic_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Deferral count could not be read", error))
}

fn candidate_from_state(
    connection: &Connection,
    context: &PlanContext,
    state: &RevisionTopicState,
    work_item: Option<&WorkItemRow>,
    today: NaiveDate,
) -> Result<Option<Candidate>, AppError> {
    let Some(schedule) = state.schedule.as_ref() else {
        return Ok(None);
    };
    if state.topic.archived || schedule.status != "active" {
        return Ok(None);
    }
    let today_text = date_only::format_date_only(today);
    if schedule
        .deferred_until
        .as_deref()
        .is_some_and(|date| date > today_text.as_str())
    {
        return Ok(None);
    }
    if state.due_state == "suspended" || state.due_state == "not_scheduled" {
        return Ok(None);
    }
    let recommendation = context.recommendations.get(&state.topic.id);
    let estimate = work_item.map_or_else(
        || estimate_minutes(connection, state.topic.id, context.focus_minutes),
        |item| Ok(item.estimated_minutes),
    )?;
    let lane = recommendation
        .map(|item| item.lane.clone())
        .or_else(|| work_item.map(|item| item.lane.clone()))
        .unwrap_or_else(|| "maintain".to_owned());
    let triage_score = recommendation
        .map(|item| item.triage_score)
        .or_else(|| work_item.map(|item| item.triage_score))
        .unwrap_or(0.0);
    let priority_need = recommendation.map_or(50.0, |item| item.priority_need);
    let group = if state.due_state == "overdue"
        || work_item.is_some_and(|item| item.status == "backlog")
    {
        CandidateGroup::Backlog
    } else if state.due_state == "due_today" || state.days_until_due.is_some_and(|days| days <= 7) {
        CandidateGroup::Scheduled
    } else {
        CandidateGroup::Optional
    };
    Ok(Some(Candidate {
        topic: state.topic.clone(),
        estimate_minutes: estimate,
        lane,
        triage_score,
        days_overdue: state.days_overdue,
        next_due_date: Some(schedule.next_revision_date.clone()),
        priority_need,
        deferral_count: deferral_count(connection, state.topic.id)?,
        group,
        urgent: state.due_state == "overdue"
            || state.due_state == "due_today"
            || state.days_until_due.is_some_and(|days| days <= 7),
        topic_id: state.topic.id,
        code: state.topic.code.clone(),
    }))
}

fn build_candidates(
    connection: &Connection,
    context: &PlanContext,
    today: NaiveDate,
    work_items: &HashMap<i64, WorkItemRow>,
) -> Result<Vec<Candidate>, AppError> {
    let mut result = Vec::new();
    for state in &context.states {
        if let Some(candidate) = candidate_from_state(
            connection,
            context,
            state,
            work_items.get(&state.topic.id),
            today,
        )? {
            result.push(candidate);
        }
    }
    result.sort_by(|left, right| {
        right
            .triage_score
            .partial_cmp(&left.triage_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.deferral_count.cmp(&left.deferral_count))
            .then_with(|| right.days_overdue.cmp(&left.days_overdue))
            .then_with(|| {
                right
                    .priority_need
                    .partial_cmp(&left.priority_need)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.next_due_date.cmp(&right.next_due_date))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.topic_id.cmp(&right.topic_id))
    });
    Ok(result)
}

fn selected_from_work_item(
    connection: &Connection,
    item: &WorkItemRow,
    position: i64,
    membership_status: &str,
) -> Result<SelectedItem, AppError> {
    let block = block_snapshot(connection, item.planned_block_id)?;
    Ok(SelectedItem {
        work_item_id: item.id,
        topic_id: item.topic_id,
        position,
        estimated_minutes: item.estimated_minutes,
        lane: item.lane.clone(),
        triage_score: item.triage_score,
        membership_status: membership_status.to_owned(),
        planned_block_id: block.id,
        planned_block_title: block.title,
        planned_block_date: block.calendar_date,
        planned_block_start_time: block.start_time,
        planned_block_end_time: block.end_time,
        carried_from_daily_plan_id: None,
    })
}

fn selected_from_membership(row: &MembershipRow, membership_status: &str) -> SelectedItem {
    SelectedItem {
        work_item_id: row.plan_item_id,
        topic_id: row.topic_id,
        position: row.position,
        estimated_minutes: row.estimated_minutes,
        lane: row.lane.clone(),
        triage_score: row.triage_score,
        membership_status: membership_status.to_owned(),
        planned_block_id: row.planned_block_id,
        planned_block_title: row.planned_block_title.clone(),
        planned_block_date: row.planned_block_date.clone(),
        planned_block_start_time: row.planned_block_start_time.clone(),
        planned_block_end_time: row.planned_block_end_time.clone(),
        carried_from_daily_plan_id: row.carried_from_daily_plan_id,
    }
}

fn completed_topics_for_date(
    connection: &Connection,
    date: &str,
) -> Result<HashSet<i64>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT pi.topic_id
             FROM revision_daily_plan_items dpi
             JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
             JOIN revision_plan_items pi ON pi.id = dpi.plan_item_id
             WHERE dp.plan_date = ?1 AND dpi.membership_status = 'completed'",
        )
        .map_err(|error| {
            AppError::database("Completed plan topics could not be prepared", error)
        })?;
    statement
        .query_map([date], |row| row.get::<_, i64>(0))
        .map_err(|error| AppError::database("Completed plan topics could not be queried", error))?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| AppError::database("Completed plan topics could not be read", error))
}

struct Allocation {
    selected: Vec<SelectedItem>,
    overflow: Vec<RevisionUnscheduledUrgent>,
    planned_minutes: i64,
    completed_minutes: f64,
    actual_minutes_today: f64,
}

fn allocate(
    connection: &Transaction<'_>,
    context: &PlanContext,
    today: NaiveDate,
    preserved: Vec<SelectedItem>,
    budget: i64,
    max_topics: i64,
) -> Result<Allocation, AppError> {
    let today_text = date_only::format_date_only(today);
    let completed_topics = completed_topics_for_date(connection, &today_text)?;
    let completed_minutes = completed_minutes_for_date(connection, &today_text)?;
    let actual_minutes_today = actual_minutes_for_date(connection, &today_text)?;
    let work_items = load_active_work_items(connection)?;
    let candidates = build_candidates(connection, context, today, &work_items)?;
    let mut selected = preserved;
    let mut used_topics = completed_topics.clone();
    for item in &selected {
        used_topics.insert(item.topic_id);
    }
    let preserved_commit = selected
        .iter()
        .filter(|item| item.membership_status == "in_progress")
        .map(|item| {
            let lifetime = actual_seconds(connection, item.work_item_id)?;
            let actual_today = actual_seconds_on(connection, item.work_item_id, &today_text)?;
            let remaining = remaining_estimate(item.estimated_minutes, lifetime);
            Ok(actual_today as f64 / 60.0 + remaining)
        })
        .collect::<Result<Vec<f64>, AppError>>()?
        .into_iter()
        .sum::<f64>();
    let mut remaining = (budget as f64 - completed_minutes - preserved_commit).max(0.0);
    let mut overflow = Vec::new();

    let backlog = candidates
        .iter()
        .filter(|candidate| candidate.group == CandidateGroup::Backlog)
        .collect::<Vec<_>>();
    let scheduled = candidates
        .iter()
        .filter(|candidate| candidate.group == CandidateGroup::Scheduled)
        .collect::<Vec<_>>();
    let optional = candidates
        .iter()
        .filter(|candidate| candidate.group == CandidateGroup::Optional)
        .collect::<Vec<_>>();
    let backlog_reserve = if !backlog.is_empty() && !scheduled.is_empty() {
        (remaining * 0.5).ceil()
    } else {
        remaining
    };

    let mut select_group = |group: &[&Candidate],
                            cap: &mut f64,
                            remaining: &mut f64|
     -> Result<(), AppError> {
        for candidate in group {
            if used_topics.contains(&candidate.topic_id) {
                continue;
            }
            if used_topics.len() as i64 >= max_topics {
                if candidate.urgent {
                    overflow.push(RevisionUnscheduledUrgent {
                        topic: candidate.topic.clone(),
                        estimate_minutes: candidate.estimate_minutes,
                        reason: "topic limit".to_owned(),
                        lane: candidate.lane.clone(),
                        triage_score: candidate.triage_score,
                    });
                }
                continue;
            }
            let existing = work_items.get(&candidate.topic_id);
            let cost = if existing.is_some_and(|item| item.status == "in_progress") {
                let lifetime = actual_seconds(connection, existing.unwrap().id)?;
                let actual_today =
                    actual_seconds_on(connection, existing.unwrap().id, &today_text)?;
                actual_today as f64 / 60.0
                    + remaining_estimate(existing.unwrap().estimated_minutes, lifetime)
            } else {
                candidate.estimate_minutes as f64
            };
            if cost > *cap || cost > *remaining {
                if candidate.urgent {
                    overflow.push(RevisionUnscheduledUrgent {
                        topic: candidate.topic.clone(),
                        estimate_minutes: candidate.estimate_minutes,
                        reason: "budget".to_owned(),
                        lane: candidate.lane.clone(),
                        triage_score: candidate.triage_score,
                    });
                }
                continue;
            }
            let item = match existing {
                Some(item) => {
                    if item.status == "deferred" || item.status == "backlog" {
                        connection
                            .execute(
                                "UPDATE revision_plan_items
                                 SET status = 'planned', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                                 WHERE id = ?1",
                                [item.id],
                            )
                            .map_err(|error| AppError::database("A work item could not be planned", error))?;
                    }
                    item.clone()
                }
                None => {
                    connection
                        .execute(
                            "INSERT INTO revision_plan_items (
                                topic_id, status, estimated_minutes, triage_score, lane,
                                source_due_date, created_at, updated_at
                             ) VALUES (?1, 'planned', ?2, ?3, ?4, ?5,
                                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                            params![
                                candidate.topic_id,
                                candidate.estimate_minutes,
                                candidate.triage_score,
                                candidate.lane,
                                candidate.next_due_date.as_deref().unwrap_or(&today_text),
                            ],
                        )
                        .map_err(|error| {
                            AppError::database("A work item could not be created", error)
                        })?;
                    let id = connection.last_insert_rowid();
                    WorkItemRow {
                        id,
                        topic_id: candidate.topic_id,
                        status: "planned".to_owned(),
                        estimated_minutes: candidate.estimate_minutes,
                        triage_score: candidate.triage_score,
                        lane: candidate.lane.clone(),
                        planned_block_id: None,
                        revision_history_id: None,
                        removed_at: None,
                    }
                }
            };
            let membership_status = if item.status == "in_progress" {
                "in_progress"
            } else {
                "planned"
            };
            let selected_item = selected_from_work_item(
                connection,
                &item,
                selected.len() as i64,
                membership_status,
            )?;
            selected.push(selected_item);
            used_topics.insert(candidate.topic_id);
            *remaining -= cost;
            *cap -= cost;
        }
        Ok(())
    };

    let mut backlog_cap = backlog_reserve;
    select_group(&backlog, &mut backlog_cap, &mut remaining)?;
    let mut scheduled_cap = remaining;
    select_group(&scheduled, &mut scheduled_cap, &mut remaining)?;
    let mut extra_cap = remaining;
    select_group(&backlog, &mut extra_cap, &mut remaining)?;
    let mut optional_cap = remaining;
    select_group(&optional, &mut optional_cap, &mut remaining)?;

    for (position, item) in selected.iter_mut().enumerate() {
        item.position = position as i64;
    }
    let planned_minutes = selected.iter().map(|item| item.estimated_minutes).sum();
    Ok(Allocation {
        selected,
        overflow,
        planned_minutes,
        completed_minutes,
        actual_minutes_today,
    })
}

fn reconcile_plan_state(
    connection: &mut Connection,
    context: &PlanContext,
    today: NaiveDate,
) -> Result<(), AppError> {
    let today_text = date_only::format_date_only(today);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The revision plan reconciliation transaction could not start",
            error,
        )
    })?;

    transaction
        .execute(
            "UPDATE revision_daily_plans
             SET status = 'closed',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE status = 'active' AND plan_date < ?1",
            [&today_text],
        )
        .map_err(|error| AppError::database("Old revision plans could not be closed", error))?;

    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = 'backlog',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE status = 'planned'
               AND id NOT IN (
                   SELECT dpi.plan_item_id
                   FROM revision_daily_plan_items dpi
                   JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
                   WHERE dp.plan_date = ?1 AND dp.status = 'active'
               )",
            [&today_text],
        )
        .map_err(|error| AppError::database("Old planned work could not be carried", error))?;

    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = 'backlog',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE status = 'deferred'
               AND topic_id IN (
                   SELECT rs.topic_id
                   FROM revision_schedules rs
                   WHERE rs.status = 'active'
                     AND (rs.deferred_until IS NULL OR rs.deferred_until <= ?1)
               )",
            [&today_text],
        )
        .map_err(|error| {
            AppError::database("Eligible deferred work could not be released", error)
        })?;
    transaction
        .execute(
            "UPDATE revision_schedules
             SET deferred_until = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE deferred_until IS NOT NULL AND deferred_until <= ?1",
            [&today_text],
        )
        .map_err(|error| AppError::database("Returned deferrals could not be cleared", error))?;

    for state in &context.states {
        let Some(schedule) = state.schedule.as_ref() else {
            continue;
        };
        if state.topic.archived || schedule.status != "active" {
            continue;
        }
        if schedule
            .deferred_until
            .as_deref()
            .is_some_and(|date| date > today_text.as_str())
        {
            continue;
        }
        if schedule.next_revision_date >= today_text {
            continue;
        }
        if active_work_item(&transaction, state.topic.id)?.is_some() {
            continue;
        }
        let recommendation = context.recommendations.get(&state.topic.id);
        let estimate = estimate_minutes(&transaction, state.topic.id, context.focus_minutes)?;
        transaction
            .execute(
                "INSERT INTO revision_plan_items (
                    topic_id, status, estimated_minutes, triage_score, lane,
                    source_due_date, created_at, updated_at
                 ) VALUES (?1, 'backlog', ?2, ?3, ?4, ?5,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    state.topic.id,
                    estimate,
                    recommendation.map_or(0.0, |item| item.triage_score),
                    recommendation.map_or_else(|| "maintain".to_owned(), |item| item.lane.clone()),
                    schedule.next_revision_date.clone(),
                ],
            )
            .map_err(|error| {
                AppError::database("Overdue revision work could not be created", error)
            })?;
    }

    transaction.commit().map_err(|error| {
        AppError::database("The revision plan reconciliation could not commit", error)
    })
}

pub(crate) fn reconcile_for_revision_outcome(
    connection: &mut Connection,
    today: NaiveDate,
) -> Result<(), AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)
}

fn insert_memberships(
    transaction: &Transaction<'_>,
    plan_id: i64,
    selected: &[SelectedItem],
) -> Result<(), AppError> {
    for item in selected {
        transaction
            .execute(
                "INSERT INTO revision_daily_plan_items (
                    daily_plan_id, plan_item_id, position, estimated_minutes, lane,
                    triage_score, membership_status, planned_block_id,
                    planned_block_title, planned_block_date, planned_block_start_time,
                    planned_block_end_time, carried_from_daily_plan_id, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    plan_id,
                    item.work_item_id,
                    item.position,
                    item.estimated_minutes,
                    item.lane,
                    item.triage_score,
                    item.membership_status,
                    item.planned_block_id,
                    item.planned_block_title,
                    item.planned_block_date,
                    item.planned_block_start_time,
                    item.planned_block_end_time,
                    item.carried_from_daily_plan_id,
                ],
            )
            .map_err(|error| {
                AppError::database("A daily plan membership could not be saved", error)
            })?;
    }
    Ok(())
}

fn create_plan_version(
    transaction: &Transaction<'_>,
    old: Option<&PlanHeader>,
    today_text: &str,
    budget: i64,
    max_topics: i64,
    selected: &[SelectedItem],
    completed_minutes: f64,
) -> Result<PlanHeader, AppError> {
    if let Some(old) = old {
        transaction
            .execute(
                "UPDATE revision_daily_plans
                 SET status = 'superseded',
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [old.id],
            )
            .map_err(|error| {
                AppError::database("The previous plan version could not be superseded", error)
            })?;
    }
    let version = old.map_or(1, |plan| plan.version + 1);
    let planned_minutes: i64 = selected
        .iter()
        .filter(|item| item.membership_status != "completed")
        .map(|item| item.estimated_minutes)
        .sum();
    transaction
        .execute(
            "INSERT INTO revision_daily_plans (
                plan_date, version, effective_budget_minutes, max_topics,
                planned_minutes, completed_minutes, status, created_at,
                last_rebalanced_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active',
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                CASE WHEN ?2 > 1 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE NULL END,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![
                today_text,
                version,
                budget,
                max_topics,
                planned_minutes,
                completed_minutes,
            ],
        )
        .map_err(|error| AppError::database("A daily plan version could not be created", error))?;
    let plan_id = transaction.last_insert_rowid();
    insert_memberships(transaction, plan_id, selected)?;
    Ok(PlanHeader {
        id: plan_id,
        plan_date: today_text.to_owned(),
        version,
        effective_budget_minutes: budget,
        max_topics,
        planned_minutes,
        status: "active".to_owned(),
        created_at: String::new(),
        last_rebalanced_at: None,
    })
}

pub fn generate_daily_plan(
    connection: &mut Connection,
    today: NaiveDate,
) -> Result<RevisionDailyPlan, AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    if load_active_plan(connection, today)?.is_some() {
        return build_plan_view(connection, &context, today);
    }
    let budget = effective_budget(&context.settings, context.total_study_target_minutes);
    let max_topics = context.settings.daily_revision_target;
    let today_text = date_only::format_date_only(today);
    let preserved = in_progress_selected(connection, &context.states, today)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The daily plan generation transaction could not start",
            error,
        )
    })?;
    let allocation = allocate(&transaction, &context, today, preserved, budget, max_topics)?;
    create_plan_version(
        &transaction,
        None,
        &today_text,
        budget,
        max_topics,
        &allocation.selected,
        allocation.completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The daily plan generation could not commit", error))?;
    let mut view = build_plan_view(connection, &context, today)?;
    view.unscheduled_urgent = allocation.overflow;
    view.planned_minutes = allocation.planned_minutes;
    view.actual_minutes_today = allocation.actual_minutes_today;
    Ok(view)
}

pub fn rebalance_daily_plan(
    connection: &mut Connection,
    today: NaiveDate,
) -> Result<RevisionDailyPlan, AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let Some(plan) = load_active_plan(connection, today)? else {
        return generate_daily_plan(connection, today);
    };
    let rows = load_memberships(connection, plan.id)?;
    let mut preserved = Vec::new();
    let mut completed = Vec::new();
    for row in &rows {
        if row.membership_status == "in_progress" {
            preserved.push(selected_from_membership(row, "in_progress"));
        } else if row.membership_status == "completed" {
            completed.push(selected_from_membership(row, "completed"));
        }
    }
    let budget = effective_budget(&context.settings, context.total_study_target_minutes);
    let max_topics = context.settings.daily_revision_target;
    let today_text = date_only::format_date_only(today);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The daily plan rebalance transaction could not start",
            error,
        )
    })?;
    let allocation = allocate(&transaction, &context, today, preserved, budget, max_topics)?;
    let selected_ids = allocation
        .selected
        .iter()
        .map(|item| item.work_item_id)
        .collect::<HashSet<_>>();
    for row in &rows {
        if row.membership_status == "planned" && !selected_ids.contains(&row.plan_item_id) {
            transaction
                .execute(
                    "UPDATE revision_plan_items
                     SET status = 'backlog',
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1 AND status = 'planned'",
                    [row.plan_item_id],
                )
                .map_err(|error| {
                    AppError::database("Unselected planned work could not move to backlog", error)
                })?;
        }
    }
    let mut selected = allocation.selected;
    append_completed(&mut selected, completed);
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        budget,
        max_topics,
        &selected,
        allocation.completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The daily plan rebalance could not commit", error))?;
    build_plan_view(connection, &context, today)
}

pub fn get_daily_plan(
    connection: &mut Connection,
    today: NaiveDate,
) -> Result<RevisionDailyPlan, AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    build_plan_view(connection, &context, today)
}

pub fn interval_preview(
    connection: &Connection,
    today: NaiveDate,
    topic_id: i64,
    outcome: &str,
    revision_date: Option<&str>,
) -> Result<RevisionIntervalDecision, AppError> {
    if !revision::REVISION_OUTCOMES.contains(&outcome) {
        return Err(AppError::validation(
            "outcome",
            "Choose Again, Difficult, Good or Easy.",
        ));
    }
    let state = revision::topic_states(connection, today)?
        .into_iter()
        .find(|state| state.topic.id == topic_id)
        .ok_or_else(AppError::topic_not_found)?;
    let schedule = state
        .schedule
        .as_ref()
        .ok_or_else(AppError::revision_not_scheduled)?;
    let scheduled_for = parse_date(&schedule.next_revision_date, "scheduledForDate")?;
    let actual = match revision_date {
        Some(value) => parse_date(value, "revisionDate")?,
        None => today,
    };
    let timing_days = (actual - scheduled_for).num_days();
    let evidence_status = revision::evidence_status_for_topic(connection, today, topic_id)?;
    let interval_after = revision::next_interval_days_with_context(
        &revision::load_settings(connection)?,
        schedule.current_interval_days,
        outcome,
        timing_days,
        evidence_status,
    );
    let timing_factor =
        (1.0 + 0.15 * (timing_days as f64 / 14.0).clamp(-1.0, 1.0)).clamp(0.85, 1.15);
    let evidence_factor = if evidence_status == EvidenceClass::Weak {
        0.8
    } else {
        1.0
    };
    let evidence_label = match evidence_status {
        EvidenceClass::Weak => "weak",
        EvidenceClass::Strong => "strong",
        EvidenceClass::Moderate => "moderate",
        EvidenceClass::Insufficient => "insufficient",
        EvidenceClass::Stale => "stale",
        EvidenceClass::SkippedOnly => "skipped_only",
        EvidenceClass::Unscored => "unscored",
        EvidenceClass::None => "none",
    };
    let mut reasons = vec![format!(
        "Interval changes from {} to {} days.",
        schedule.current_interval_days, interval_after
    )];
    if matches!(outcome, "good" | "easy") {
        reasons.push(format!(
            "Timing adjustment: {timing_days:+} days from the scheduled date (factor {timing_factor:.3})."
        ));
        if evidence_status == EvidenceClass::Weak {
            reasons.push("Recent PYQ evidence is weak; interval growth is reduced.".to_owned());
        }
    } else {
        reasons.push("Again/Difficult reset to the configured outcome interval.".to_owned());
    }
    Ok(RevisionIntervalDecision {
        interval_before_days: schedule.current_interval_days,
        interval_after_days: interval_after,
        timing_days,
        timing_factor,
        evidence_status: evidence_label.to_owned(),
        evidence_factor,
        reasons,
    })
}

fn load_deferral_event(
    connection: &Connection,
    id: i64,
) -> Result<RevisionDeferralEvent, AppError> {
    connection
        .query_row(
            "SELECT id, topic_id, plan_item_id, event_kind, overridden_deferral_id,
                    previous_due_date, new_due_date, occurred_on, reason, notes, created_at
             FROM revision_deferrals WHERE id = ?1",
            [id],
            |row| {
                Ok(RevisionDeferralEvent {
                    id: row.get(0)?,
                    topic_id: row.get(1)?,
                    plan_item_id: row.get(2)?,
                    event_kind: row.get(3)?,
                    overridden_deferral_id: row.get(4)?,
                    previous_due_date: row.get(5)?,
                    new_due_date: row.get(6)?,
                    occurred_on: row.get(7)?,
                    reason: row.get(8)?,
                    notes: row.get(9)?,
                    created_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("The deferral event could not be loaded", error))?
        .ok_or_else(|| AppError::internal("The deferral event could not be reloaded."))
}

fn validate_deferral_input(reason: &str, notes: &str) -> Result<(), AppError> {
    if !matches!(reason, "time" | "difficulty" | "energy" | "life" | "other") {
        return Err(AppError::validation(
            "reason",
            "Choose time, difficulty, energy, life or other.",
        ));
    }
    if notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Deferral notes must be 2000 characters or fewer.",
        ));
    }
    Ok(())
}

pub fn defer_revision(
    connection: &mut Connection,
    today: NaiveDate,
    topic_id: i64,
    new_date: &str,
    reason: &str,
    notes: &str,
) -> Result<RevisionDeferralResult, AppError> {
    validate_deferral_input(reason, notes)?;
    let new_date = parse_date(new_date, "newDate")?;
    let today_text = date_only::format_date_only(today);
    if new_date < today {
        return Err(AppError::validation(
            "newDate",
            "A deferral return date cannot be earlier than today.",
        ));
    }
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let state = context
        .states
        .iter()
        .find(|state| state.topic.id == topic_id)
        .ok_or_else(AppError::topic_not_found)?;
    if state.topic.archived || state.due_state == "suspended" {
        return Err(AppError::validation(
            "topicId",
            "Archived or suspended topics cannot be deferred.",
        ));
    }
    let schedule = state
        .schedule
        .as_ref()
        .ok_or_else(AppError::revision_not_scheduled)?;
    let previous = parse_date(&schedule.next_revision_date, "nextRevisionDate")?;
    if new_date <= previous {
        return Err(AppError::validation(
            "newDate",
            "A deferral must move the due date later.",
        ));
    }
    let plan = load_active_plan(connection, today)?;
    let mut selected = plan
        .as_ref()
        .map(|plan| selected_for_version(connection, plan.id, &context.states))
        .transpose()?
        .unwrap_or_default();
    if selected
        .iter()
        .any(|item| item.topic_id == topic_id && item.membership_status == "completed")
    {
        return Err(AppError::validation(
            "topicId",
            "This revision was already completed today and cannot be deferred.",
        ));
    }
    selected.retain(|item| item.topic_id != topic_id || item.membership_status == "completed");
    let estimate = estimate_minutes(connection, topic_id, context.focus_minutes)?;
    let recommendation = context.recommendations.get(&topic_id);
    let new_date_text = date_only::format_date_only(new_date);
    let next_status = if new_date > today {
        "deferred"
    } else {
        "backlog"
    };
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The revision deferral transaction could not start", error)
    })?;
    let item = match active_work_item(&transaction, topic_id)? {
        Some(item) => {
            transaction
                .execute(
                    "UPDATE revision_plan_items
                     SET status = ?1,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?2",
                    params![next_status, item.id],
                )
                .map_err(|error| {
                    AppError::database("The work item could not be deferred", error)
                })?;
            Some(item.id)
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO revision_plan_items (
                        topic_id, status, estimated_minutes, triage_score, lane,
                        source_due_date, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    params![
                        topic_id,
                        next_status,
                        estimate,
                        recommendation.map_or(0.0, |item| item.triage_score),
                        recommendation
                            .map_or_else(|| "maintain".to_owned(), |item| item.lane.clone()),
                        schedule.next_revision_date,
                    ],
                )
                .map_err(|error| {
                    AppError::database("A deferred work item could not be created", error)
                })?;
            Some(transaction.last_insert_rowid())
        }
    };
    transaction
        .execute(
            "UPDATE revision_schedules
             SET next_revision_date = ?1,
                 deferred_until = CASE WHEN ?1 <= ?2 THEN NULL ELSE ?1 END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?3",
            params![new_date_text, today_text, topic_id],
        )
        .map_err(|error| AppError::database("The revision could not be deferred", error))?;
    transaction
        .execute(
            "INSERT INTO revision_deferrals (
                topic_id, plan_item_id, event_kind, previous_due_date,
                new_due_date, occurred_on, reason, notes, created_at
             ) VALUES (?1, ?2, 'defer', ?3, ?4, ?5, ?6, ?7,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![
                topic_id,
                item,
                schedule.next_revision_date,
                new_date_text,
                today_text,
                reason,
                notes,
            ],
        )
        .map_err(|error| AppError::database("The deferral event could not be recorded", error))?;
    let event_id = transaction.last_insert_rowid();
    if let Some(plan) = plan.as_ref() {
        let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
        create_plan_version(
            &transaction,
            Some(plan),
            &today_text,
            plan.effective_budget_minutes,
            plan.max_topics,
            &selected,
            completed_minutes,
        )?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("The deferral could not commit", error))?;
    let event = load_deferral_event(connection, event_id)?;
    let refreshed = load_context(connection, today)?;
    let schedule = refreshed
        .states
        .iter()
        .find(|state| state.topic.id == topic_id)
        .and_then(|state| state.schedule.clone())
        .ok_or_else(AppError::revision_not_scheduled)?;
    Ok(RevisionDeferralResult {
        plan: build_plan_view(connection, &refreshed, today)?,
        event,
        schedule,
    })
}

pub fn override_deferred_revision(
    connection: &mut Connection,
    today: NaiveDate,
    topic_id: i64,
    override_date: &str,
    reason: &str,
    notes: &str,
) -> Result<RevisionDeferralResult, AppError> {
    validate_deferral_input(reason, notes)?;
    let today_text = date_only::format_date_only(today);
    let override_date = parse_date(override_date, "overrideDate")?;
    if override_date < today {
        return Err(AppError::validation(
            "overrideDate",
            "An override date cannot be earlier than today.",
        ));
    }
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let state = context
        .states
        .iter()
        .find(|state| state.topic.id == topic_id)
        .ok_or_else(AppError::topic_not_found)?;
    if state.topic.archived || state.due_state == "suspended" {
        return Err(AppError::validation(
            "topicId",
            "Archived or suspended topics cannot be overridden.",
        ));
    }
    let schedule = state
        .schedule
        .as_ref()
        .ok_or_else(AppError::revision_not_scheduled)?;
    let current_return = schedule
        .deferred_until
        .clone()
        .ok_or_else(|| AppError::validation("topicId", "This topic is not actively deferred."))?;
    if override_date >= parse_date(&current_return, "deferredUntil")? {
        return Err(AppError::validation(
            "overrideDate",
            "The override must bring the topic back earlier than its deferred return date.",
        ));
    }
    let previous_deferral: i64 = connection
        .query_row(
            "SELECT id FROM revision_deferrals
             WHERE topic_id = ?1 AND event_kind = 'defer'
             ORDER BY occurred_on DESC, id DESC LIMIT 1",
            [topic_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The active deferral could not be loaded", error))?
        .ok_or_else(|| AppError::validation("topicId", "This topic is not actively deferred."))?;
    let override_text = date_only::format_date_only(override_date);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The deferral override transaction could not start", error)
    })?;
    transaction
        .execute(
            "UPDATE revision_schedules
             SET next_revision_date = ?1,
                 deferred_until = CASE WHEN ?1 <= ?2 THEN NULL ELSE ?1 END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?3",
            params![override_text, today_text, topic_id],
        )
        .map_err(|error| {
            AppError::database("The deferred revision could not be overridden", error)
        })?;
    transaction
        .execute(
            "INSERT INTO revision_deferrals (
                topic_id, plan_item_id, event_kind, overridden_deferral_id,
                previous_due_date, new_due_date, occurred_on, reason, notes, created_at
             ) VALUES (?1,
                (SELECT id FROM revision_plan_items WHERE topic_id = ?1
                 AND status IN ('planned', 'in_progress', 'backlog', 'deferred')
                 ORDER BY id DESC LIMIT 1),
                'override', ?2, ?3, ?4, ?5, ?6, ?7,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![
                topic_id,
                previous_deferral,
                current_return,
                override_text,
                today_text,
                reason,
                notes,
            ],
        )
        .map_err(|error| AppError::database("The override event could not be recorded", error))?;
    let event_id = transaction.last_insert_rowid();
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = CASE WHEN ?1 <= ?2 THEN 'backlog' ELSE 'deferred' END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE topic_id = ?3
               AND status IN ('planned', 'in_progress', 'backlog', 'deferred')",
            params![override_text, today_text, topic_id],
        )
        .map_err(|error| {
            AppError::database("The deferred work item could not be updated", error)
        })?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The deferral override could not commit", error))?;
    let event = load_deferral_event(connection, event_id)?;
    let refreshed = load_context(connection, today)?;
    let schedule = refreshed
        .states
        .iter()
        .find(|state| state.topic.id == topic_id)
        .and_then(|state| state.schedule.clone())
        .ok_or_else(AppError::revision_not_scheduled)?;
    Ok(RevisionDeferralResult {
        plan: build_plan_view(connection, &refreshed, today)?,
        event,
        schedule,
    })
}

pub fn plan_history(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<RevisionPlanHistoryEntry>, AppError> {
    let from = parse_date(from_date, "fromDate")?;
    let to = parse_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }
    let mut topics = revision::load_topics(connection)?;
    topics.extend(revision::load_archived_topics(connection)?);
    let topics = topics
        .into_iter()
        .map(|topic| (topic.id, topic))
        .collect::<HashMap<_, _>>();
    let mut statement = connection
        .prepare(
            "SELECT id, plan_date, version, status, effective_budget_minutes,
                    max_topics, planned_minutes, created_at, last_rebalanced_at
             FROM revision_daily_plans
             WHERE plan_date >= ?1 AND plan_date <= ?2
             ORDER BY plan_date, version",
        )
        .map_err(|error| {
            AppError::database("Revision plan history could not be prepared", error)
        })?;
    let headers = statement
        .query_map(
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            |row| {
                Ok(PlanHeader {
                    id: row.get(0)?,
                    plan_date: row.get(1)?,
                    version: row.get(2)?,
                    status: row.get(3)?,
                    effective_budget_minutes: row.get(4)?,
                    max_topics: row.get(5)?,
                    planned_minutes: row.get(6)?,
                    created_at: row.get(7)?,
                    last_rebalanced_at: row.get(8)?,
                })
            },
        )
        .map_err(|error| AppError::database("Revision plan history could not be queried", error))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Revision plan history could not be read", error))?;
    let mut result = Vec::new();
    for header in headers {
        let mut memberships = Vec::new();
        for row in load_memberships(connection, header.id)? {
            if let Some(topic) = topics.get(&row.topic_id) {
                memberships.push(membership_view(connection, &row, topic)?);
            }
        }
        let completed_minutes =
            completed_minutes_for_plan(connection, header.id, &header.plan_date)?;
        result.push(RevisionPlanHistoryEntry {
            plan_id: header.id,
            plan_date: header.plan_date,
            version: header.version,
            status: header.status,
            effective_budget_minutes: header.effective_budget_minutes,
            max_topics: header.max_topics,
            planned_minutes: header.planned_minutes,
            completed_minutes,
            created_at: header.created_at,
            last_rebalanced_at: header.last_rebalanced_at,
            memberships,
        });
    }
    Ok(result)
}

pub(crate) fn pending_work_item_for_topic(
    connection: &Connection,
    topic_id: i64,
) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT id FROM revision_plan_items
             WHERE topic_id = ?1
               AND status IN ('planned', 'in_progress', 'backlog')
             ORDER BY id DESC
             LIMIT 1",
            [topic_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("A pending revision work item could not be found", error)
        })
}

pub(crate) fn active_plan_item_for_topic(
    connection: &Connection,
    topic_id: i64,
    plan_date: &str,
) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT dpi.plan_item_id
             FROM revision_daily_plan_items dpi
             JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
             JOIN revision_plan_items pi ON pi.id = dpi.plan_item_id
             WHERE dp.status = 'active'
               AND dp.plan_date = ?1
               AND pi.topic_id = ?2
               AND dpi.membership_status <> 'completed'
             ORDER BY dpi.plan_item_id
             LIMIT 1",
            params![plan_date, topic_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database(
                "The active plan item for a revision could not be found",
                error,
            )
        })
}

pub(crate) fn attach_session(
    transaction: &Transaction<'_>,
    plan_item_id: i64,
    study_session_id: i64,
) -> Result<(), AppError> {
    let Some(item) = load_work_item(transaction, plan_item_id)? else {
        return Ok(());
    };
    if item.removed_at.is_some() || item.status == "removed" {
        return Ok(());
    }
    transaction
        .execute(
            "INSERT INTO revision_session_links (plan_item_id, study_session_id, created_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![plan_item_id, study_session_id],
        )
        .map_err(|error| {
            AppError::database(
                "The focus session could not be linked to the revision",
                error,
            )
        })?;
    let active_plan: Option<(i64, String)> = transaction
        .query_row(
            "SELECT dpi.daily_plan_id, dp.plan_date
             FROM revision_daily_plan_items dpi
             JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
             WHERE dpi.plan_item_id = ?1 AND dp.status = 'active'
             ORDER BY dp.id DESC LIMIT 1",
            [plan_item_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The active plan membership could not be found", error)
        })?;
    if item.revision_history_id.is_some() || item.status == "completed" {
        if let Some((plan_id, plan_date)) = active_plan {
            let completed = completed_minutes_for_date(transaction, &plan_date)?;
            transaction
                .execute(
                    "UPDATE revision_daily_plans
                     SET completed_minutes = ?1,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?2",
                    params![completed, plan_id],
                )
                .map_err(|error| {
                    AppError::database("The plan completion total could not be refreshed", error)
                })?;
        }
        return Ok(());
    }
    if item.status == "deferred" {
        return Ok(());
    }
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = 'in_progress',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            [plan_item_id],
        )
        .map_err(|error| {
            AppError::database(
                "The revision work item could not be marked in progress",
                error,
            )
        })?;
    if let Some((plan_id, _)) = active_plan {
        transaction
            .execute(
                "UPDATE revision_daily_plan_items
                 SET membership_status = CASE
                        WHEN membership_status = 'completed' THEN membership_status
                        ELSE 'in_progress'
                     END,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE daily_plan_id = ?1 AND plan_item_id = ?2
                   AND membership_status <> 'completed'",
                params![plan_id, plan_item_id],
            )
            .map_err(|error| {
                AppError::database("The plan membership could not be marked in progress", error)
            })?;
    }
    Ok(())
}

pub(crate) fn attach_revision_outcome(
    transaction: &Transaction<'_>,
    topic_id: i64,
    history_id: i64,
    plan_item_id: Option<i64>,
    today_text: &str,
) -> Result<Option<i64>, AppError> {
    let Some(plan_item_id) = plan_item_id else {
        return Ok(None);
    };
    let item = load_work_item(transaction, plan_item_id)?.ok_or_else(|| {
        AppError::validation("planItemId", "The revision work item no longer exists.")
    })?;
    if item.topic_id != topic_id {
        return Err(AppError::validation(
            "planItemId",
            "The revision work item belongs to a different topic.",
        ));
    }
    if item.status == "deferred" {
        return Err(AppError::validation(
            "planItemId",
            "This revision is deferred and cannot be completed before its return date.",
        ));
    }
    if item.revision_history_id.is_some() || item.status == "completed" {
        return Err(AppError::validation(
            "planItemId",
            "This revision plan item has already been completed.",
        ));
    }
    if item.removed_at.is_some() || item.status == "removed" {
        return Err(AppError::validation(
            "planItemId",
            "A removed revision plan item cannot be completed.",
        ));
    }
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = 'completed',
                 revision_history_id = ?1,
                 removed_at = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![history_id, plan_item_id],
        )
        .map_err(|error| {
            AppError::database("The revision work item could not be completed", error)
        })?;
    let active_plan: Option<i64> = transaction
        .query_row(
            "SELECT dpi.daily_plan_id
             FROM revision_daily_plan_items dpi
             JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
             WHERE dpi.plan_item_id = ?1
               AND dp.status = 'active'
               AND dp.plan_date = ?2
             ORDER BY dp.id DESC LIMIT 1",
            params![plan_item_id, today_text],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The active plan membership could not be found", error)
        })?;
    if let Some(plan_id) = active_plan {
        transaction
            .execute(
                "UPDATE revision_daily_plan_items
                 SET membership_status = 'completed',
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE daily_plan_id = ?1 AND plan_item_id = ?2
                   AND membership_status <> 'completed'",
                params![plan_id, plan_item_id],
            )
            .map_err(|error| {
                AppError::database("The plan membership could not be completed", error)
            })?;
        let completed = completed_minutes_for_date(transaction, today_text)?;
        transaction
            .execute(
                "UPDATE revision_daily_plans
                 SET completed_minutes = ?1,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?2",
                params![completed, plan_id],
            )
            .map_err(|error| {
                AppError::database("The plan completion total could not be updated", error)
            })?;
    }
    Ok(Some(plan_item_id))
}

pub fn add_revision_plan_item(
    connection: &mut Connection,
    today: NaiveDate,
    topic_id: i64,
) -> Result<RevisionDailyPlan, AppError> {
    let today_text = date_only::format_date_only(today);
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let plan = match load_active_plan(connection, today)? {
        Some(plan) => plan,
        None => {
            generate_daily_plan(connection, today)?;
            load_active_plan(connection, today)?
                .ok_or_else(|| AppError::internal("The generated plan could not be loaded."))?
        }
    };
    let state = context
        .states
        .iter()
        .find(|state| state.topic.id == topic_id)
        .ok_or_else(AppError::topic_not_found)?;
    if state.topic.archived || state.due_state == "suspended" {
        return Err(AppError::validation(
            "topicId",
            "Archived or suspended topics cannot be added to today's plan.",
        ));
    }
    if state
        .schedule
        .as_ref()
        .and_then(|schedule| schedule.deferred_until.as_deref())
        .is_some_and(|date| date > date_only::format_date_only(today).as_str())
    {
        return Err(AppError::validation(
            "topicId",
            "This topic is deferred. Override the deferral before adding it to today's plan.",
        ));
    }
    let mut selected = selected_for_version(connection, plan.id, &context.states)?;
    if selected.iter().any(|item| item.topic_id == topic_id) {
        return build_plan_view(connection, &context, today);
    }
    let recommendation = context.recommendations.get(&topic_id);
    let estimate = estimate_minutes(connection, topic_id, context.focus_minutes)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The manual plan-add transaction could not start", error)
    })?;
    let item = if let Some(mut active) = active_work_item(&transaction, topic_id)? {
        if active.status != "in_progress" {
            transaction
                .execute(
                    "UPDATE revision_plan_items
                     SET status = 'planned', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?1",
                    [active.id],
                )
                .map_err(|error| AppError::database("The work item could not be planned", error))?;
            active.status = "planned".to_owned();
        }
        active
    } else {
        transaction
            .execute(
                "INSERT INTO revision_plan_items (
                    topic_id, status, estimated_minutes, triage_score, lane,
                    source_due_date, created_at, updated_at
                 ) VALUES (?1, 'planned', ?2, ?3, ?4, ?5,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    topic_id,
                    estimate,
                    recommendation.map_or(0.0, |item| item.triage_score),
                    recommendation.map_or_else(|| "maintain".to_owned(), |item| item.lane.clone()),
                    state
                        .schedule
                        .as_ref()
                        .map_or(today_text.clone(), |schedule| schedule
                            .next_revision_date
                            .clone()),
                ],
            )
            .map_err(|error| {
                AppError::database("A manual work item could not be created", error)
            })?;
        WorkItemRow {
            id: transaction.last_insert_rowid(),
            topic_id,
            status: "planned".to_owned(),
            estimated_minutes: estimate,
            triage_score: recommendation.map_or(0.0, |item| item.triage_score),
            lane: recommendation.map_or_else(|| "maintain".to_owned(), |item| item.lane.clone()),
            planned_block_id: None,
            revision_history_id: None,
            removed_at: None,
        }
    };
    let membership_status = if item.status == "in_progress" {
        "in_progress"
    } else {
        "planned"
    };
    let mut selected_item = selected_from_work_item(
        &transaction,
        &item,
        selected.len() as i64,
        membership_status,
    )?;
    selected_item.carried_from_daily_plan_id = None;
    selected.push(selected_item);
    let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        plan.effective_budget_minutes,
        plan.max_topics,
        &selected,
        completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The manual plan add could not commit", error))?;
    build_plan_view(connection, &context, today)
}

pub fn move_revision_plan_item(
    connection: &mut Connection,
    today: NaiveDate,
    plan_item_id: i64,
    target: &str,
) -> Result<RevisionDailyPlan, AppError> {
    if !matches!(target, "backlog" | "removed") {
        return Err(AppError::validation(
            "target",
            "Choose backlog or removed for a plan item.",
        ));
    }
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let plan = load_active_plan(connection, today)?
        .ok_or_else(|| AppError::validation("planItemId", "An active plan is required."))?;
    let mut selected = selected_for_version(connection, plan.id, &context.states)?;
    let Some(position) = selected
        .iter()
        .position(|item| item.work_item_id == plan_item_id)
    else {
        return Err(AppError::validation(
            "planItemId",
            "That work item is not in today's active plan.",
        ));
    };
    if selected[position].membership_status == "completed" {
        return Err(AppError::validation(
            "planItemId",
            "Completed work cannot be moved out of the plan.",
        ));
    }
    let current = load_work_item(connection, plan_item_id)?
        .ok_or_else(|| AppError::validation("planItemId", "The work item was not found."))?;
    let next_status = if target == "backlog" && current.status == "in_progress" {
        "in_progress"
    } else {
        target
    };
    selected.remove(position);
    let today_text = date_only::format_date_only(today);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The plan-item move transaction could not start", error)
    })?;
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET status = ?1,
                 removed_at = CASE WHEN ?1 = 'removed' THEN ?2 ELSE removed_at END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3",
            params![next_status, today_text, plan_item_id],
        )
        .map_err(|error| AppError::database("The work item could not be moved", error))?;
    let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        plan.effective_budget_minutes,
        plan.max_topics,
        &selected,
        completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The plan-item move could not commit", error))?;
    build_plan_view(connection, &context, today)
}

pub fn reorder_revision_plan_items(
    connection: &mut Connection,
    today: NaiveDate,
    plan_id: i64,
    ordered_item_ids: &[i64],
) -> Result<RevisionDailyPlan, AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let plan = load_active_plan(connection, today)?
        .filter(|plan| plan.id == plan_id)
        .ok_or_else(|| AppError::validation("planId", "Choose the active plan version."))?;
    let (active, completed): (Vec<_>, Vec<_>) =
        selected_for_version(connection, plan.id, &context.states)?
            .into_iter()
            .partition(|item| item.membership_status != "completed");
    let ordered_set = ordered_item_ids.iter().copied().collect::<HashSet<_>>();
    if active.len() != ordered_item_ids.len()
        || ordered_set.len() != ordered_item_ids.len()
        || active
            .iter()
            .any(|item| !ordered_set.contains(&item.work_item_id))
    {
        return Err(AppError::validation(
            "orderedPlanItemIds",
            "The reordered IDs must exactly match the active plan.",
        ));
    }
    let by_id = active
        .into_iter()
        .map(|item| (item.work_item_id, item))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::new();
    for (position, id) in ordered_item_ids.iter().enumerate() {
        let mut item = by_id
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::validation("orderedPlanItemIds", "A plan item is missing."))?;
        item.position = position as i64;
        selected.push(item);
    }
    append_completed(&mut selected, completed);
    let today_text = date_only::format_date_only(today);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The plan reorder transaction could not start", error)
    })?;
    let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        plan.effective_budget_minutes,
        plan.max_topics,
        &selected,
        completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The plan reorder could not commit", error))?;
    build_plan_view(connection, &context, today)
}

pub fn update_revision_plan_estimate(
    connection: &mut Connection,
    today: NaiveDate,
    plan_item_id: i64,
    estimated_minutes: i64,
) -> Result<RevisionDailyPlan, AppError> {
    if !(5..=240).contains(&estimated_minutes) {
        return Err(AppError::validation(
            "estimatedMinutes",
            "Estimated minutes must be between 5 and 240.",
        ));
    }
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let plan = load_active_plan(connection, today)?
        .ok_or_else(|| AppError::validation("planItemId", "An active plan is required."))?;
    let mut selected = selected_for_version(connection, plan.id, &context.states)?;
    let Some(item) = selected
        .iter_mut()
        .find(|item| item.work_item_id == plan_item_id)
    else {
        return Err(AppError::validation(
            "planItemId",
            "That work item is not in today's active plan.",
        ));
    };
    if item.membership_status == "completed" {
        return Err(AppError::validation(
            "planItemId",
            "Completed work estimates cannot be changed.",
        ));
    }
    item.estimated_minutes = estimated_minutes;
    let today_text = date_only::format_date_only(today);
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The plan estimate transaction could not start", error)
    })?;
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET estimated_minutes = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![estimated_minutes, plan_item_id],
        )
        .map_err(|error| AppError::database("The plan estimate could not be saved", error))?;
    let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        plan.effective_budget_minutes,
        plan.max_topics,
        &selected,
        completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The plan estimate could not commit", error))?;
    build_plan_view(connection, &context, today)
}

pub fn link_revision_plan_block(
    connection: &mut Connection,
    today: NaiveDate,
    plan_item_id: i64,
    block_id: i64,
) -> Result<RevisionDailyPlan, AppError> {
    let context = load_context(connection, today)?;
    reconcile_plan_state(connection, &context, today)?;
    let plan = load_active_plan(connection, today)?
        .ok_or_else(|| AppError::validation("planItemId", "An active plan is required."))?;
    let mut selected = selected_for_version(connection, plan.id, &context.states)?;
    let Some(item) = selected
        .iter_mut()
        .find(|item| item.work_item_id == plan_item_id)
    else {
        return Err(AppError::validation(
            "planItemId",
            "That work item is not in today's active plan.",
        ));
    };
    if item.membership_status == "completed" {
        return Err(AppError::validation(
            "planItemId",
            "Completed work cannot be linked to a different block.",
        ));
    }
    let block = block_snapshot(connection, Some(block_id))?;
    if block.title.is_none() {
        return Err(AppError::validation(
            "blockId",
            "Choose an existing planned block.",
        ));
    }
    item.planned_block_id = block.id;
    item.planned_block_title = block.title;
    item.planned_block_date = block.calendar_date;
    item.planned_block_start_time = block.start_time;
    item.planned_block_end_time = block.end_time;
    let today_text = date_only::format_date_only(today);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The block-link transaction could not start", error))?;
    transaction
        .execute(
            "UPDATE revision_plan_items
             SET planned_block_id = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![block_id, plan_item_id],
        )
        .map_err(|error| {
            AppError::database("The work-item block link could not be saved", error)
        })?;
    let completed_minutes = completed_minutes_for_date(&transaction, &today_text)?;
    create_plan_version(
        &transaction,
        Some(&plan),
        &today_text,
        plan.effective_budget_minutes,
        plan.max_topics,
        &selected,
        completed_minutes,
    )?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The block link could not commit", error))?;
    build_plan_view(connection, &context, today)
}

fn actual_seconds(connection: &Connection, plan_item_id: i64) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(ss.duration_seconds), 0)
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             WHERE rsl.plan_item_id = ?1",
            [plan_item_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Work-item actual time could not be read", error))
}

fn actual_seconds_on(
    connection: &Connection,
    plan_item_id: i64,
    date: &str,
) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(ss.duration_seconds), 0)
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             WHERE rsl.plan_item_id = ?1 AND ss.session_date = ?2",
            params![plan_item_id, date],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Work-item daily actual time could not be read", error))
}

fn remaining_estimate(estimated_minutes: i64, lifetime_seconds: i64) -> f64 {
    (estimated_minutes as f64 - lifetime_seconds as f64 / 60.0).max(0.0)
}

fn block_snapshot(
    connection: &Connection,
    block_id: Option<i64>,
) -> Result<RevisionPlanBlockSnapshot, AppError> {
    let Some(block_id) = block_id else {
        return Ok(RevisionPlanBlockSnapshot {
            id: None,
            title: None,
            calendar_date: None,
            start_time: None,
            end_time: None,
        });
    };
    let block = connection
        .query_row(
            "SELECT title, calendar_date, start_time, end_time
             FROM planned_study_blocks WHERE id = ?1",
            [block_id],
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
        .map_err(|error| AppError::database("A planned block snapshot could not be read", error))?;
    Ok(match block {
        Some((title, calendar_date, start_time, end_time)) => RevisionPlanBlockSnapshot {
            id: Some(block_id),
            title: Some(title),
            calendar_date: Some(calendar_date),
            start_time: Some(start_time),
            end_time: Some(end_time),
        },
        None => RevisionPlanBlockSnapshot {
            id: Some(block_id),
            title: None,
            calendar_date: None,
            start_time: None,
            end_time: None,
        },
    })
}

fn membership_view(
    connection: &Connection,
    row: &MembershipRow,
    topic: &RevisionTopic,
) -> Result<RevisionPlanMembership, AppError> {
    let lifetime = actual_seconds(connection, row.plan_item_id)?;
    let item = load_work_item(connection, row.plan_item_id)?
        .ok_or_else(|| AppError::internal("A plan work item is missing."))?;
    Ok(RevisionPlanMembership {
        daily_plan_id: row.daily_plan_id,
        plan_item_id: row.plan_item_id,
        topic: topic.clone(),
        position: row.position,
        estimated_minutes: row.estimated_minutes,
        lane: row.lane.clone(),
        triage_score: row.triage_score,
        membership_status: row.membership_status.clone(),
        item_status: item.status,
        actual_seconds: lifetime,
        remaining_estimate_minutes: remaining_estimate(row.estimated_minutes, lifetime),
        carried_from_daily_plan_id: row.carried_from_daily_plan_id,
        planned_block: RevisionPlanBlockSnapshot {
            id: row.planned_block_id,
            title: row.planned_block_title.clone(),
            calendar_date: row.planned_block_date.clone(),
            start_time: row.planned_block_start_time.clone(),
            end_time: row.planned_block_end_time.clone(),
        },
    })
}

fn completed_minutes_for_plan(
    connection: &Connection,
    plan_id: i64,
    date: &str,
) -> Result<f64, AppError> {
    let seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(ss.duration_seconds), 0)
             FROM revision_daily_plan_items dpi
             JOIN revision_plan_items pi ON pi.id = dpi.plan_item_id
             JOIN revision_session_links rsl ON rsl.plan_item_id = pi.id
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             JOIN revision_history rh ON rh.id = pi.revision_history_id
             WHERE dpi.daily_plan_id = ?1
               AND dpi.membership_status = 'completed'
               AND ss.session_date = ?2
               AND rh.revision_date = ?2",
            params![plan_id, date],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Historical plan completed minutes could not be read", error)
        })?;
    Ok(seconds as f64 / 60.0)
}

fn completed_minutes_for_date(connection: &Connection, date: &str) -> Result<f64, AppError> {
    let seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(ss.duration_seconds), 0)
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             JOIN revision_plan_items pi ON pi.id = rsl.plan_item_id
             JOIN revision_history rh ON rh.id = pi.revision_history_id
             WHERE ss.session_date = ?1 AND rh.revision_date = ?1",
            [date],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Completed revision minutes could not be read", error)
        })?;
    Ok(seconds as f64 / 60.0)
}

fn actual_minutes_for_date(connection: &Connection, date: &str) -> Result<f64, AppError> {
    let seconds: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(ss.duration_seconds), 0)
             FROM revision_session_links rsl
             JOIN study_sessions ss ON ss.id = rsl.study_session_id
             WHERE ss.session_date = ?1",
            [date],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Revision actual minutes could not be read", error))?;
    Ok(seconds as f64 / 60.0)
}

fn build_plan_view(
    connection: &Connection,
    context: &PlanContext,
    today: NaiveDate,
) -> Result<RevisionDailyPlan, AppError> {
    let today_text = date_only::format_date_only(today);
    let effective = effective_budget(&context.settings, context.total_study_target_minutes);
    let Some(plan) = load_active_plan(connection, today)? else {
        let completed_minutes = completed_minutes_for_date(connection, &today_text)?;
        let actual_minutes_today = actual_minutes_for_date(connection, &today_text)?;
        return Ok(RevisionDailyPlan {
            today: today_text.clone(),
            plan_id: None,
            plan_date: None,
            version: None,
            status: None,
            needs_generation: true,
            effective_budget_minutes: effective,
            max_topics: context.settings.daily_revision_target,
            planned_minutes: 0,
            completed_minutes,
            actual_minutes_today,
            remaining_work_minutes: 0.0,
            remaining_daily_budget_minutes: (effective as f64 - actual_minutes_today).max(0.0),
            over_budget: actual_minutes_today > effective as f64,
            over_limit: false,
            scheduled: Vec::new(),
            completed: Vec::new(),
            backlog: backlog_items(connection, &context.states, today)?,
            deferred: deferred_items(connection, &context.states, today)?,
            unscheduled_urgent: Vec::new(),
        });
    };

    let states = context
        .states
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();
    let rows = load_memberships(connection, plan.id)?;
    let mut scheduled = Vec::new();
    let mut completed = Vec::new();
    for row in &rows {
        let Some(state) = states.get(&row.topic_id) else {
            continue;
        };
        let view = membership_view(connection, row, &state.topic)?;
        if row.membership_status == "completed" {
            completed.push(view);
        } else {
            scheduled.push(view);
        }
    }
    let completed_minutes = completed_minutes_for_date(connection, &today_text)?;
    let actual_today = actual_minutes_for_date(connection, &today_text)?;
    let planned_minutes: i64 = scheduled.iter().map(|item| item.estimated_minutes).sum();
    let remaining_work: f64 = scheduled
        .iter()
        .map(|item| item.remaining_estimate_minutes)
        .sum();
    let inprogress_actual: f64 = scheduled
        .iter()
        .filter(|item| item.membership_status == "in_progress")
        .map(|item| actual_seconds_on(connection, item.plan_item_id, &today_text))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum::<i64>() as f64
        / 60.0;
    let committed = completed_minutes
        + inprogress_actual
        + scheduled
            .iter()
            .filter(|item| item.membership_status == "in_progress")
            .map(|item| item.remaining_estimate_minutes)
            .sum::<f64>()
        + scheduled
            .iter()
            .filter(|item| item.membership_status == "planned")
            .map(|item| item.estimated_minutes as f64)
            .sum::<f64>();
    let remaining_budget = (effective as f64 - committed).max(0.0);
    let over_budget = committed > effective as f64 || planned_minutes > effective;
    let over_limit = scheduled.len() as i64 + completed.len() as i64 > plan.max_topics;

    Ok(RevisionDailyPlan {
        today: today_text.clone(),
        plan_id: Some(plan.id),
        plan_date: Some(plan.plan_date),
        version: Some(plan.version),
        status: Some(plan.status),
        needs_generation: false,
        effective_budget_minutes: plan.effective_budget_minutes,
        max_topics: plan.max_topics,
        planned_minutes,
        completed_minutes,
        actual_minutes_today: actual_today,
        remaining_work_minutes: remaining_work,
        remaining_daily_budget_minutes: remaining_budget,
        over_budget,
        over_limit,
        scheduled,
        completed,
        backlog: backlog_items(connection, &context.states, today)?,
        deferred: deferred_items(connection, &context.states, today)?,
        unscheduled_urgent: Vec::new(),
    })
}

fn backlog_items(
    connection: &Connection,
    states: &[RevisionTopicState],
    today: NaiveDate,
) -> Result<Vec<RevisionBacklogItem>, AppError> {
    let today_text = date_only::format_date_only(today);
    let state_by_topic = states
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();
    let mut result = Vec::new();
    let mut statement = connection
        .prepare(
            "SELECT rpi.id, rpi.topic_id, rpi.status, rpi.estimated_minutes,
                    rpi.triage_score, rpi.lane, rpi.source_due_date
             FROM revision_plan_items rpi
             WHERE rpi.status IN ('backlog', 'in_progress')
               AND NOT EXISTS (
                   SELECT 1
                   FROM revision_daily_plan_items dpi
                   JOIN revision_daily_plans dp ON dp.id = dpi.daily_plan_id
                   WHERE dpi.plan_item_id = rpi.id
                     AND dp.plan_date = ?1
                     AND dp.status = 'active'
               )
             ORDER BY rpi.triage_score DESC, rpi.id",
        )
        .map_err(|error| AppError::database("Revision backlog could not be prepared", error))?;
    let rows = statement
        .query_map([&today_text], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|error| AppError::database("Revision backlog could not be queried", error))?;
    for row in rows {
        let (id, topic_id, status, estimated, score, lane, source_due) =
            row.map_err(|error| AppError::database("Revision backlog could not be read", error))?;
        let Some(state) = state_by_topic.get(&topic_id) else {
            continue;
        };
        if state.topic.archived || state.due_state == "suspended" {
            continue;
        }
        let deferred_until = state
            .schedule
            .as_ref()
            .and_then(|s| s.deferred_until.clone());
        if deferred_until
            .as_deref()
            .is_some_and(|date| date > today_text.as_str())
        {
            continue;
        }
        let lifetime = actual_seconds(connection, id)?;
        result.push(RevisionBacklogItem {
            plan_item_id: id,
            topic: state.topic.clone(),
            status,
            estimated_minutes: estimated,
            lane,
            triage_score: score,
            source_due_date: source_due,
            actual_seconds: lifetime,
            remaining_estimate_minutes: remaining_estimate(estimated, lifetime),
            deferred_until,
        });
    }
    Ok(result)
}

fn deferred_items(
    connection: &Connection,
    states: &[RevisionTopicState],
    today: NaiveDate,
) -> Result<Vec<RevisionDeferredItem>, AppError> {
    let today_text = date_only::format_date_only(today);
    let state_by_topic = states
        .iter()
        .map(|state| (state.topic.id, state))
        .collect::<HashMap<_, _>>();
    let mut result = Vec::new();
    let mut statement = connection
        .prepare(
            "SELECT d.topic_id, d.plan_item_id, d.previous_due_date, d.new_due_date,
                    d.occurred_on, d.reason, d.notes,
                    (SELECT count(*) FROM revision_deferrals x
                     WHERE x.topic_id = d.topic_id AND x.event_kind = 'defer'),
                    COALESCE((
                        SELECT x.previous_due_date FROM revision_deferrals x
                        WHERE x.topic_id = d.topic_id AND x.event_kind = 'defer'
                        ORDER BY x.id LIMIT 1
                    ), d.previous_due_date)
             FROM revision_deferrals d
             WHERE d.id = (
                 SELECT x.id FROM revision_deferrals x
                 WHERE x.topic_id = d.topic_id
                 ORDER BY x.occurred_on DESC, x.id DESC LIMIT 1
             )",
        )
        .map_err(|error| AppError::database("Deferred revisions could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .map_err(|error| AppError::database("Deferred revisions could not be queried", error))?;
    for row in rows {
        let (topic_id, plan_item_id, previous, _new_due, occurred, reason, notes, count, original) =
            row.map_err(|error| AppError::database("Deferred revisions could not be read", error))?;
        let Some(state) = state_by_topic.get(&topic_id) else {
            continue;
        };
        if state.topic.archived || state.due_state == "suspended" {
            continue;
        }
        let Some(deferred_until) = state
            .schedule
            .as_ref()
            .and_then(|s| s.deferred_until.clone())
        else {
            continue;
        };
        if deferred_until <= today_text {
            continue;
        }
        result.push(RevisionDeferredItem {
            topic: state.topic.clone(),
            plan_item_id,
            original_due_date: original,
            previous_due_date: previous,
            deferred_until,
            deferral_count: count,
            latest_reason: reason,
            latest_notes: notes,
            latest_occurred_on: occurred,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use rusqlite::Connection;

    use super::*;
    use crate::{db, planner, pyq, revision, syllabus, timer};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn today() -> NaiveDate {
        date(2026, 10, 30)
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(today());
        syllabus::seed(&mut connection).expect("syllabus should seed");
        timer::ensure_defaults(&mut connection).expect("timer defaults should exist");
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

    fn schedule(connection: &mut Connection, topic_id: i64, next_date: &str) {
        revision::schedule_topic(connection, topic_id, next_date, today())
            .expect("topic should schedule");
    }

    fn set_limits(connection: &mut Connection, minutes: i64, topics: i64) {
        let settings = revision::load_settings(connection).expect("settings should load");
        revision::update_settings(
            connection,
            revision::RevisionSettingsInput {
                initial_again_days: settings.initial_again_days,
                initial_difficult_days: settings.initial_difficult_days,
                initial_good_days: settings.initial_good_days,
                initial_easy_days: settings.initial_easy_days,
                growth_multiplier: settings.growth_multiplier,
                minimum_interval_days: settings.minimum_interval_days,
                maximum_interval_days: settings.maximum_interval_days,
                include_weekends: settings.include_weekends,
                daily_revision_target: topics,
                daily_revision_minutes: minutes,
                reminders_enabled: settings.reminders_enabled,
            },
        )
        .expect("settings should update");
    }

    fn complete(connection: &mut Connection, topic_id: i64, plan_item_id: Option<i64>) {
        revision::complete(
            connection,
            topic_id,
            revision::CompleteRevisionInput {
                plan_item_id,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        )
        .expect("revision should complete");
    }

    fn start_and_finish_focus(
        connection: &mut Connection,
        topic_id: i64,
        plan_item_id: i64,
        minutes: i64,
    ) {
        let start = Utc
            .with_ymd_and_hms(2026, 10, 30, 9, 0, 0)
            .single()
            .expect("start time should be valid");
        timer::start_with_plan(
            connection,
            start,
            today(),
            "focus",
            Some(topic_id),
            Some(plan_item_id),
        )
        .expect("timer should start");
        for second in (10..(minutes * 60)).step_by(10) {
            timer::heartbeat(connection, start + Duration::seconds(second))
                .expect("timer heartbeat should succeed");
        }
        timer::finish(connection, start + Duration::minutes(minutes)).expect("timer should finish");
    }

    #[test]
    fn daily_plan_requires_explicit_generation_and_respects_limits() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        schedule(&mut connection, second, "2026-10-30");
        set_limits(&mut connection, 90, 1);

        let before = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert!(before.needs_generation);
        assert!(before.plan_id.is_none());
        assert!(before.scheduled.is_empty());

        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        assert!(!generated.needs_generation);
        assert_eq!(generated.version, Some(1));
        assert_eq!(generated.scheduled.len(), 1);
        assert!(generated.scheduled.len() as i64 <= generated.max_topics);
        assert!(generated.planned_minutes <= generated.effective_budget_minutes);
        assert_eq!(generated.unscheduled_urgent.len(), 1);

        let regenerated =
            generate_daily_plan(&mut connection, today()).expect("existing plan should load");
        assert_eq!(regenerated.plan_id, generated.plan_id);

        let history =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].memberships.len(), 1);
    }

    #[test]
    fn completed_work_survives_new_plan_versions_and_historical_versions_stay_immutable() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        schedule(&mut connection, second, "2026-10-30");
        set_limits(&mut connection, 90, 1);

        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let first_item = generated.scheduled[0].plan_item_id;
        let first_topic = generated.scheduled[0].topic.id;
        complete(&mut connection, first_topic, Some(first_item));

        let after_completion = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert_eq!(after_completion.completed.len(), 1);
        assert!(after_completion.scheduled.is_empty());
        assert_eq!(after_completion.version, Some(1));

        let other = if first_topic == first { second } else { first };
        let extended = add_revision_plan_item(&mut connection, today(), other)
            .expect("manual item should be added");
        assert_eq!(extended.version, Some(2));
        assert_eq!(extended.completed.len(), 1);
        assert_eq!(extended.scheduled.len(), 1);
        assert!(extended.over_limit);

        let history =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].status, "superseded");
        assert_eq!(history[1].status, "active");
        assert_eq!(
            history[0]
                .memberships
                .iter()
                .filter(|item| item.membership_status == "completed")
                .count(),
            1
        );
        assert_eq!(
            history[1]
                .memberships
                .iter()
                .filter(|item| item.membership_status == "completed")
                .count(),
            1
        );

        let changed = connection.execute(
            "UPDATE revision_daily_plan_items SET position = position + 1
             WHERE daily_plan_id = ?1",
            [history[0].plan_id],
        );
        assert!(changed.is_err(), "superseded memberships must be immutable");
        let completed_changed = connection.execute(
            "UPDATE revision_daily_plan_items SET position = position + 1
             WHERE daily_plan_id = ?1 AND membership_status = 'completed'",
            [history[1].plan_id],
        );
        assert!(
            completed_changed.is_err(),
            "completed memberships must be immutable"
        );
    }

    #[test]
    fn deferrals_are_append_only_and_override_brings_work_back_early() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;

        let first = defer_revision(
            &mut connection,
            today(),
            topic,
            "2026-11-05",
            "time",
            "family",
        )
        .expect("deferral should succeed");
        assert_eq!(first.event.event_kind, "defer");
        assert_eq!(first.event.plan_item_id, Some(item));
        assert_eq!(first.schedule.deferred_until.as_deref(), Some("2026-11-05"));
        assert!(first.plan.scheduled.is_empty());
        assert_eq!(first.plan.deferred.len(), 1);
        assert_eq!(first.plan.deferred[0].original_due_date, "2026-10-30");

        let second = defer_revision(
            &mut connection,
            today(),
            topic,
            "2026-11-10",
            "energy",
            "travel",
        )
        .expect("second deferral should succeed");
        assert_eq!(second.plan.deferred[0].deferral_count, 2);
        assert_eq!(second.plan.deferred[0].original_due_date, "2026-10-30");
        assert_eq!(second.plan.deferred[0].previous_due_date, "2026-11-05");

        let changed = connection.execute(
            "UPDATE revision_deferrals SET notes = 'changed' WHERE id = ?1",
            [first.event.id],
        );
        assert!(changed.is_err(), "deferrals must be append-only");
        let early_completion = revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: Some(item),
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        );
        assert!(
            early_completion.is_err(),
            "deferred work must be unavailable until its return date"
        );
        let same_day =
            override_deferred_revision(&mut connection, today(), topic, "2026-11-10", "time", "");
        assert!(
            same_day.is_err(),
            "an override must move the return date earlier"
        );

        let overridden = override_deferred_revision(
            &mut connection,
            today(),
            topic,
            "2026-10-30",
            "time",
            "return early",
        )
        .expect("override should succeed");
        assert_eq!(overridden.event.event_kind, "override");
        assert_eq!(
            overridden.event.previous_due_date, "2026-11-10",
            "override should reference the latest deferral"
        );
        assert_eq!(overridden.schedule.deferred_until, None);
        assert!(overridden.plan.deferred.is_empty());

        let next_day = generate_daily_plan(&mut connection, date(2026, 10, 31))
            .expect("next plan should generate");
        assert_eq!(next_day.scheduled.len(), 1);
        assert_eq!(next_day.scheduled[0].topic.id, topic);
        assert!(next_day.deferred.is_empty());
    }

    #[test]
    fn finishing_a_linked_timer_after_a_deferral_keeps_the_deferral() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        let start = Utc
            .with_ymd_and_hms(2026, 10, 30, 9, 0, 0)
            .single()
            .expect("start time should be valid");
        timer::start_with_plan(
            &mut connection,
            start,
            today(),
            "focus",
            Some(topic),
            Some(item),
        )
        .expect("timer should start");

        defer_revision(
            &mut connection,
            today(),
            topic,
            "2026-11-10",
            "life",
            "changed plans while the timer was running",
        )
        .expect("deferral should succeed");
        for second in (10..300).step_by(10) {
            timer::heartbeat(&mut connection, start + Duration::seconds(second))
                .expect("heartbeat should succeed");
        }
        timer::finish(&mut connection, start + Duration::minutes(5))
            .expect("finishing the timer must not fail because the item was deferred");

        let status: String = connection
            .query_row(
                "SELECT status FROM revision_plan_items WHERE id = ?1",
                [item],
                |row| row.get(0),
            )
            .expect("work item should load");
        assert_eq!(status, "deferred");
        let links: i64 = connection
            .query_row(
                "SELECT count(*) FROM revision_session_links WHERE plan_item_id = ?1",
                [item],
                |row| row.get(0),
            )
            .expect("links should load");
        assert_eq!(links, 1);
    }

    #[test]
    fn actual_time_is_attributed_to_the_session_date_without_double_counting() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;

        start_and_finish_focus(&mut connection, topic, item, 10);
        let in_progress = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert_eq!(in_progress.scheduled.len(), 1);
        assert_eq!(in_progress.scheduled[0].membership_status, "in_progress");
        assert_eq!(in_progress.scheduled[0].actual_seconds, 600);
        assert!((in_progress.scheduled[0].remaining_estimate_minutes - 15.0).abs() < 0.01);
        assert!((in_progress.actual_minutes_today - 10.0).abs() < 0.01);
        assert!((in_progress.completed_minutes - 0.0).abs() < 0.01);
        assert!((in_progress.remaining_work_minutes - 15.0).abs() < 0.01);
        assert!((in_progress.remaining_daily_budget_minutes - 65.0).abs() < 0.01);

        complete(&mut connection, topic, Some(item));
        let completed = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert_eq!(completed.completed.len(), 1);
        assert!(completed.scheduled.is_empty());
        assert!((completed.completed_minutes - 10.0).abs() < 0.01);
        assert!((completed.actual_minutes_today - 10.0).abs() < 0.01);

        let topic_history = revision::history(&connection, topic).expect("history should load");
        assert_eq!(topic_history[0].plan_item_id, Some(item));
        assert_eq!(topic_history[0].actual_seconds, Some(600));

        let plan_history =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert!((plan_history[0].completed_minutes - 10.0).abs() < 0.01);
        assert_eq!(
            plan_history[0].memberships[0].membership_status, "completed",
            "the completion stays on its original plan version"
        );
    }

    #[test]
    fn completing_today_without_an_explicit_item_links_the_active_membership() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;

        let completion = revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        )
        .expect("revision should complete");
        assert_eq!(completion.plan_item_id, Some(item));

        let plan = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert!(plan.scheduled.is_empty());
        assert_eq!(plan.completed.len(), 1);
        assert_eq!(plan.completed[0].plan_item_id, item);
    }

    #[test]
    fn completing_a_backlog_topic_without_a_plan_closes_its_work_item() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-20");
        set_limits(&mut connection, 90, 5);
        let unresolved = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert!(unresolved.needs_generation);
        let backlog_item = unresolved
            .backlog
            .iter()
            .find(|item| item.topic.id == topic)
            .expect("overdue topic should have a backlog item")
            .plan_item_id;

        let completion = revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        )
        .expect("revision should complete");
        assert_eq!(completion.plan_item_id, Some(backlog_item));

        let after = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert!(after.backlog.iter().all(|item| item.topic.id != topic));
        let status: String = connection
            .query_row(
                "SELECT status FROM revision_plan_items WHERE id = ?1",
                [backlog_item],
                |row| row.get(0),
            )
            .expect("work item should load");
        assert_eq!(status, "completed");
    }

    #[test]
    fn finishing_a_timer_after_completion_refreshes_the_active_plan_total() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        let start = Utc
            .with_ymd_and_hms(2026, 10, 30, 9, 0, 0)
            .single()
            .expect("start time should be valid");
        timer::start_with_plan(
            &mut connection,
            start,
            today(),
            "focus",
            Some(topic),
            Some(item),
        )
        .expect("timer should start");
        complete(&mut connection, topic, Some(item));
        for second in (10..300).step_by(10) {
            timer::heartbeat(&mut connection, start + Duration::seconds(second))
                .expect("heartbeat should succeed");
        }
        timer::finish(&mut connection, start + Duration::minutes(5))
            .expect("timer should finish after completion");

        let plan =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(plan.len(), 1);
        assert!((plan[0].completed_minutes - 5.0).abs() < 0.01);
        let history = revision::history(&connection, topic).expect("history should load");
        assert_eq!(history[0].actual_seconds, Some(300));
    }

    #[test]
    fn cross_date_completion_is_not_reattributed_to_the_outcome_date() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        start_and_finish_focus(&mut connection, topic, item, 10);

        revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: Some(item),
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            date(2026, 10, 31),
        )
        .expect("revision should complete on the next day");

        let next_day =
            get_daily_plan(&mut connection, date(2026, 10, 31)).expect("next-day plan should load");
        assert!(next_day.needs_generation);
        assert!((next_day.completed_minutes - 0.0).abs() < 0.01);
        assert!((next_day.actual_minutes_today - 0.0).abs() < 0.01);

        let previous =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(previous.len(), 1);
        assert!((previous[0].completed_minutes - 0.0).abs() < 0.01);
        let tracked = previous[0]
            .memberships
            .iter()
            .find(|membership| membership.plan_item_id == item)
            .expect("the prior-day membership should remain");
        assert_eq!(
            tracked.membership_status, "in_progress",
            "closing the prior-day plan leaves its membership snapshot unchanged"
        );
        assert_eq!(tracked.item_status, "completed");
        assert_eq!(tracked.actual_seconds, 600);
    }

    #[test]
    fn completing_yesterday_in_progress_after_midnight_does_not_write_to_yesterday_plan() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let day1 = today();
        let day2 = date(2026, 10, 31);
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated = generate_daily_plan(&mut connection, day1).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        start_and_finish_focus(&mut connection, topic, item, 5);

        let day2_start = Utc
            .with_ymd_and_hms(2026, 10, 31, 9, 0, 0)
            .single()
            .expect("day-two start should be valid");
        timer::start_with_plan(
            &mut connection,
            day2_start,
            day2,
            "focus",
            Some(topic),
            Some(item),
        )
        .expect("day-two timer should start");
        for second in (10..300).step_by(10) {
            timer::heartbeat(&mut connection, day2_start + Duration::seconds(second))
                .expect("day-two heartbeat should succeed");
        }
        timer::finish(&mut connection, day2_start + Duration::minutes(5))
            .expect("day-two timer should finish");

        revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: Some(item),
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            day2,
        )
        .expect("revision should complete after midnight");

        let (yesterday_status, stored_yesterday_minutes): (String, f64) = connection
            .query_row(
                "SELECT status, completed_minutes FROM revision_daily_plans
                 WHERE plan_date = '2026-10-30'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("yesterday plan should load");
        assert_eq!(yesterday_status, "closed");
        assert!((stored_yesterday_minutes - 0.0).abs() < 0.01);
        let yesterday =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("yesterday history");
        assert_eq!(yesterday.len(), 1);
        assert!((yesterday[0].completed_minutes - 0.0).abs() < 0.01);

        let day_two_view = get_daily_plan(&mut connection, day2).expect("day-two view should load");
        assert!(day_two_view.needs_generation);
        assert!((day_two_view.completed_minutes - 5.0).abs() < 0.01);
        assert!((day_two_view.actual_minutes_today - 5.0).abs() < 0.01);
    }

    #[test]
    fn late_timer_after_supersession_updates_derived_historical_minutes() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        let start = Utc
            .with_ymd_and_hms(2026, 10, 30, 9, 0, 0)
            .single()
            .expect("start time should be valid");
        timer::start_with_plan(
            &mut connection,
            start,
            today(),
            "focus",
            Some(first),
            Some(item),
        )
        .expect("timer should start");
        complete(&mut connection, first, Some(item));
        schedule(&mut connection, second, "2026-10-30");
        add_revision_plan_item(&mut connection, today(), second)
            .expect("a new plan version should be created");
        assert_eq!(
            load_active_plan(&connection, today())
                .expect("active plan should load")
                .expect("active plan should exist")
                .version,
            2
        );
        for second in (10..300).step_by(10) {
            timer::heartbeat(&mut connection, start + Duration::seconds(second))
                .expect("heartbeat should succeed");
        }
        timer::finish(&mut connection, start + Duration::minutes(5))
            .expect("timer should finish after supersession");

        let history =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(history.len(), 2);
        assert!((history[0].completed_minutes - 5.0).abs() < 0.01);
        assert!((history[1].completed_minutes - 5.0).abs() < 0.01);
        let completed = history[0]
            .memberships
            .iter()
            .find(|item| item.membership_status == "completed")
            .expect("the original version should retain its completed membership");
        assert_eq!(completed.actual_seconds, 300);
    }

    #[test]
    fn historical_plan_minutes_use_only_each_versions_completed_memberships() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let first_item = generated.scheduled[0].plan_item_id;
        complete(&mut connection, first, Some(first_item));

        schedule(&mut connection, second, "2026-10-30");
        let extended = add_revision_plan_item(&mut connection, today(), second)
            .expect("version two should be created");
        let second_item = extended.scheduled[0].plan_item_id;
        start_and_finish_focus(&mut connection, second, second_item, 10);
        complete(&mut connection, second, Some(second_item));

        let history =
            plan_history(&connection, "2026-10-30", "2026-10-30").expect("history should load");
        assert_eq!(history.len(), 2);
        assert!((history[0].completed_minutes - 0.0).abs() < 0.01);
        assert!((history[1].completed_minutes - 10.0).abs() < 0.01);
    }

    #[test]
    fn completed_work_and_historical_snapshots_are_database_immutable() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        complete(&mut connection, first, Some(item));

        assert!(
            connection
                .execute(
                    "UPDATE revision_plan_items SET estimated_minutes = 99 WHERE id = ?1",
                    [item],
                )
                .is_err(),
            "completed work items must not be editable"
        );
        assert!(
            connection
                .execute("DELETE FROM revision_plan_items WHERE id = ?1", [item])
                .is_err(),
            "completed work items must not be deletable"
        );

        schedule(&mut connection, second, "2026-10-30");
        let extended = add_revision_plan_item(&mut connection, today(), second)
            .expect("new plan version should be created");
        let second_item = extended.scheduled[0].plan_item_id;
        assert!(
            connection
                .execute(
                    "UPDATE revision_daily_plan_items SET estimated_minutes = 99
                     WHERE daily_plan_id = ?1 AND plan_item_id = ?2",
                    params![extended.plan_id.expect("plan id should exist"), second_item],
                )
                .is_err(),
            "active membership snapshots must not be structurally editable"
        );
        let old_plan_id = generated.plan_id.expect("original plan id should exist");
        assert!(
            connection
                .execute(
                    "UPDATE revision_daily_plans SET created_at = '2000-01-01T00:00:00Z'
                     WHERE id = ?1",
                    [old_plan_id],
                )
                .is_err(),
            "historical plan creation timestamps must be immutable"
        );
        assert!(
            connection
                .execute(
                    "UPDATE revision_daily_plans SET last_rebalanced_at = '2000-01-01T00:00:00Z'
                     WHERE id = ?1",
                    [old_plan_id],
                )
                .is_err(),
            "historical plan rebalance timestamps must be immutable"
        );
    }

    #[test]
    fn in_progress_work_is_carried_to_the_next_explicit_plan() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        start_and_finish_focus(&mut connection, topic, item, 5);
        connection
            .execute(
                "UPDATE revision_schedules SET next_revision_date = '2026-11-20'
                 WHERE topic_id = ?1",
                [topic],
            )
            .expect("schedule should move");

        let next_day = generate_daily_plan(&mut connection, date(2026, 10, 31))
            .expect("next plan should generate");
        assert_eq!(next_day.scheduled.len(), 1);
        assert_eq!(next_day.scheduled[0].membership_status, "in_progress");
        assert_eq!(next_day.scheduled[0].plan_item_id, item);
        assert_eq!(next_day.scheduled[0].actual_seconds, 300);
    }

    #[test]
    fn manual_additions_warn_about_budget_and_topic_limit_together() {
        let mut connection = connection();
        let first = topic_id(&connection, "MMP-C01");
        let second = topic_id(&connection, "MMP-C02");
        schedule(&mut connection, first, "2026-10-30");
        schedule(&mut connection, second, "2026-10-30");
        set_limits(&mut connection, 10, 1);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        assert!(generated.scheduled.is_empty());

        let one = add_revision_plan_item(&mut connection, today(), first)
            .expect("manual addition should succeed");
        assert!(one.over_budget);
        assert!(!one.over_limit);

        let two = add_revision_plan_item(&mut connection, today(), second)
            .expect("over-limit addition should succeed with a warning");
        assert!(two.over_budget);
        assert!(two.over_limit);
        assert_eq!(two.scheduled.len(), 2);
    }

    #[test]
    fn block_links_keep_an_immutable_membership_snapshot() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        set_limits(&mut connection, 90, 5);
        let generated =
            generate_daily_plan(&mut connection, today()).expect("plan should generate");
        let item = generated.scheduled[0].plan_item_id;
        let block = planner::create_block(
            &mut connection,
            planner::PlannedBlockInput {
                title: "Original block".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-10-30".to_owned(),
                start_time: "09:00".to_owned(),
                end_time: "09:25".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("block should be created");

        let linked = link_revision_plan_block(&mut connection, today(), item, block.id)
            .expect("block should link");
        assert_eq!(linked.version, Some(2));
        assert_eq!(
            linked.scheduled[0].planned_block.title.as_deref(),
            Some("Original block")
        );

        planner::update_block(
            &mut connection,
            block.id,
            planner::PlannedBlockInput {
                title: "Changed block".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-10-30".to_owned(),
                start_time: "10:00".to_owned(),
                end_time: "10:25".to_owned(),
                status: "planned".to_owned(),
                notes: String::new(),
                linked_session_id: None,
            },
        )
        .expect("block should update");
        let after_update = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert_eq!(
            after_update.scheduled[0].planned_block.title.as_deref(),
            Some("Original block"),
            "plan membership keeps its block snapshot"
        );

        planner::delete_block(&mut connection, block.id).expect("block should delete");
        let after_delete = get_daily_plan(&mut connection, today()).expect("plan should load");
        assert_eq!(
            after_delete.scheduled[0].planned_block.title.as_deref(),
            Some("Original block")
        );
        let work_item_block: Option<i64> = connection
            .query_row(
                "SELECT planned_block_id FROM revision_plan_items WHERE id = ?1",
                [item],
                |row| row.get(0),
            )
            .expect("work item should load");
        assert_eq!(work_item_block, None);
    }

    #[test]
    fn adaptive_intervals_use_timing_and_pyq_evidence_without_changing_failed_recall() {
        let connection = connection();
        let settings = revision::load_settings(&connection).expect("settings should load");
        let on_time = revision::next_interval_days_with_context(
            &settings,
            10,
            "good",
            0,
            EvidenceClass::Strong,
        );
        let late = revision::next_interval_days_with_context(
            &settings,
            10,
            "good",
            14,
            EvidenceClass::Strong,
        );
        let early = revision::next_interval_days_with_context(
            &settings,
            10,
            "good",
            -14,
            EvidenceClass::Strong,
        );
        let weak = revision::next_interval_days_with_context(
            &settings,
            10,
            "good",
            0,
            EvidenceClass::Weak,
        );
        assert!(late > on_time);
        assert!(early < on_time);
        assert!(weak < on_time);
        let low_current_early_weak_good = revision::next_interval_days_with_context(
            &settings,
            1,
            "good",
            -14,
            EvidenceClass::Weak,
        );
        let low_current_early_weak_easy = revision::next_interval_days_with_context(
            &settings,
            1,
            "easy",
            -14,
            EvidenceClass::Weak,
        );
        assert!(low_current_early_weak_good >= settings.initial_good_days);
        assert!(low_current_early_weak_easy >= settings.initial_easy_days);
        assert_eq!(
            revision::next_interval_days_with_context(
                &settings,
                10,
                "again",
                14,
                EvidenceClass::Weak,
            ),
            revision::next_interval_days_with_context(
                &settings,
                10,
                "again",
                -14,
                EvidenceClass::Strong,
            )
        );
    }

    #[test]
    fn early_completion_with_weak_evidence_keeps_the_initial_interval_floor() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-11-13");
        for index in 0..5 {
            pyq::create(
                &mut connection,
                pyq::PyqInput {
                    question_year: 2024,
                    exam_session: String::new(),
                    part: "B".to_owned(),
                    topic_id: topic,
                    question_reference: format!("weak-evidence-{index}"),
                    attempt_date: "2026-10-30".to_owned(),
                    status: "incorrect".to_owned(),
                    marks_obtained: None,
                    max_marks: None,
                    time_spent_seconds: 60,
                    notes: String::new(),
                    difficulty: None,
                },
            )
            .expect("weak PYQ evidence should save");
        }
        let settings = revision::load_settings(&connection).expect("settings should load");

        let completion = revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        )
        .expect("revision should complete");
        let schedule = completion.state.schedule.expect("schedule should remain");
        assert!(schedule.current_interval_days >= settings.initial_good_days);
    }

    #[test]
    fn interval_preview_reports_the_adaptive_decision() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-10-30");
        let settings = revision::load_settings(&connection).expect("settings should load");

        let preview = interval_preview(&connection, today(), topic, "good", Some("2026-11-13"))
            .expect("preview should load");
        assert_eq!(preview.timing_days, 14);
        assert!((preview.timing_factor - 1.15).abs() < 0.001);
        assert_eq!(preview.evidence_status, "none");
        assert!(preview.interval_after_days > 0);
        assert!(!preview.reasons.is_empty());

        let early = interval_preview(&connection, today(), topic, "good", Some("2026-10-16"))
            .expect("early preview should load");
        assert_eq!(early.timing_days, -14);
        assert!((early.timing_factor - 0.85).abs() < 0.001);
        assert!(early.interval_after_days >= settings.initial_good_days);
    }

    #[test]
    fn early_good_completion_keeps_the_initial_interval_floor() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        schedule(&mut connection, topic, "2026-11-13");
        let settings = revision::load_settings(&connection).expect("settings should load");

        let completion = revision::complete(
            &mut connection,
            topic,
            revision::CompleteRevisionInput {
                plan_item_id: None,
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            today(),
        )
        .expect("revision should complete");
        let schedule = completion.state.schedule.expect("schedule should remain");
        assert!(schedule.current_interval_days >= settings.initial_good_days);
    }
}
