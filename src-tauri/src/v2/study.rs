use std::{
    collections::{HashMap, HashSet},
    time::Duration as StdDuration,
};

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params, types::Type};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use uuid::Uuid;

use crate::{date_only, error::AppError};

pub const HEARTBEAT_INTERVAL: StdDuration = StdDuration::from_secs(5);
pub const STALE_AFTER_MS: i64 = 20_000;
pub const MIN_FOCUS_MINUTES: i64 = 1;
pub const MAX_FOCUS_MINUTES: i64 = 240;
pub const MIN_SHORT_BREAK_MINUTES: i64 = 1;
pub const MAX_SHORT_BREAK_MINUTES: i64 = 60;
pub const MIN_LONG_BREAK_MINUTES: i64 = 1;
pub const MAX_LONG_BREAK_MINUTES: i64 = 120;
pub const MIN_SESSION_SECONDS: i64 = 1;
pub const MAX_SESSION_SECONDS: i64 = 86_400;

const TIMER_DATA: &str = "SELECT
        status, mode, duration_ms, accumulated_ms, segment_started_at, last_heartbeat_at,
        session_date, active_exam_id, recovery_reason, recovery_excluded_ms,
        clock_skew_detected, last_event, last_event_at, last_event_duration_ms,
        last_completed_session_id
     FROM timer_state WHERE id = 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerStatus {
    Idle,
    Running,
    Paused,
}

impl TimerStatus {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "idle" => Ok(Self::Idle),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            _ => Err(AppError::timer_unavailable()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerMode {
    Focus,
    ShortBreak,
    LongBreak,
}

impl TimerMode {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "focus" => Ok(Self::Focus),
            "shortBreak" | "short_break" => Ok(Self::ShortBreak),
            "longBreak" | "long_break" => Ok(Self::LongBreak),
            _ => Err(AppError::validation(
                "mode",
                "Choose focus, short break, or long break.",
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::ShortBreak => "short_break",
            Self::LongBreak => "long_break",
        }
    }

    fn duration_ms(self, settings: &V2TimerSettings) -> i64 {
        let minutes = match self {
            Self::Focus => settings.focus_minutes,
            Self::ShortBreak => settings.short_break_minutes,
            Self::LongBreak => settings.long_break_minutes,
        };
        minutes.saturating_mul(60_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecoveryReason {
    Restart,
    Inactivity,
    ClockChange,
    Sleep,
}

impl RecoveryReason {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "restart" => Ok(Self::Restart),
            "inactivity" => Ok(Self::Inactivity),
            "clock_change" => Ok(Self::ClockChange),
            "sleep" => Ok(Self::Sleep),
            _ => Err(AppError::timer_unavailable()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Inactivity => "inactivity",
            Self::ClockChange => "clock_change",
            Self::Sleep => "sleep",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LastTimerEvent {
    Completed,
    Discarded,
}

impl LastTimerEvent {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "completed" => Ok(Self::Completed),
            "discarded" => Ok(Self::Discarded),
            _ => Err(AppError::timer_unavailable()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TimerSettings {
    pub focus_minutes: i64,
    pub short_break_minutes: i64,
    pub long_break_minutes: i64,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TimerSettingsInput {
    pub focus_minutes: i64,
    pub short_break_minutes: i64,
    pub long_break_minutes: i64,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StartTimerInput {
    pub mode: String,
    pub session_date: String,
    #[serde(default)]
    pub active_exam_id: Option<i64>,
    #[serde(default)]
    pub allocations: Vec<V2TopicAllocationInput>,
    /// Optional planned focus duration in minutes. When omitted, the timer
    /// uses the persisted focus/break setting exactly as before. Quick-start
    /// configurations use this offset so a planned session length is honoured
    /// without changing the user's timer settings.
    #[serde(default)]
    pub duration_minutes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TopicAllocation {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub node_id: i64,
    pub node_title: String,
    pub subject_title: String,
    pub allocated_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TopicAllocationInput {
    pub exam_id: i64,
    pub node_id: i64,
    pub allocated_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SetTimerAllocationsInput {
    pub allocations: Vec<V2TopicAllocationInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TimerState {
    pub status: TimerStatus,
    pub mode: TimerMode,
    pub duration_ms: i64,
    pub accumulated_ms: i64,
    pub elapsed_ms: i64,
    pub remaining_ms: i64,
    pub segment_started_at: Option<String>,
    pub session_date: Option<String>,
    pub active_exam_id: Option<i64>,
    pub recovery_reason: Option<RecoveryReason>,
    pub recovery_excluded_ms: i64,
    pub clock_skew_detected: bool,
    pub last_event: Option<LastTimerEvent>,
    pub last_event_at: Option<String>,
    pub last_event_duration_ms: i64,
    pub last_completed_session_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TimerSnapshot {
    pub state: V2TimerState,
    pub settings: V2TimerSettings,
    pub allocations: Vec<V2TopicAllocation>,
    pub server_now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SessionAllocation {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub exam_is_active: bool,
    pub allocated_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StudySession {
    pub id: i64,
    pub session_date: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_seconds: i64,
    pub source: String,
    pub notes: String,
    pub primary_exam_id: Option<i64>,
    pub allocated_seconds: i64,
    pub unallocated_seconds: i64,
    pub allocations: Vec<V2SessionAllocation>,
    pub topic_allocations: Vec<V2TopicAllocation>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SessionAllocationInput {
    pub exam_id: i64,
    pub allocated_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SetSessionAllocationsInput {
    pub session_id: i64,
    pub allocations: Vec<V2SessionAllocationInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CreateManualSessionInput {
    pub session_date: String,
    pub duration_seconds: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub primary_exam_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2UpdateSessionInput {
    pub session_id: i64,
    pub session_date: String,
    pub duration_seconds: i64,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub primary_exam_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamStudyTotal {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub exam_is_active: bool,
    pub allocated_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StudyTotals {
    pub total_seconds: i64,
    pub session_count: i64,
    pub allocated_seconds: i64,
    pub unallocated_seconds: i64,
    pub exams: Vec<V2ExamStudyTotal>,
    pub topics: Vec<V2TopicStudyTotal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TopicStudyTotal {
    pub exam_id: i64,
    pub exam_code: String,
    pub node_id: i64,
    pub node_title: String,
    pub subject_title: String,
    pub allocated_seconds: i64,
    pub session_count: i64,
    pub last_studied_date: Option<String>,
}

#[derive(Debug, Clone)]
struct TimerRow {
    status: TimerStatus,
    mode: TimerMode,
    duration_ms: i64,
    accumulated_ms: i64,
    segment_started_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
    session_date: Option<NaiveDate>,
    active_exam_id: Option<i64>,
    recovery_reason: Option<RecoveryReason>,
    recovery_excluded_ms: i64,
    clock_skew_detected: bool,
    last_event: Option<LastTimerEvent>,
    last_event_at: Option<DateTime<Utc>>,
    last_event_duration_ms: i64,
    last_completed_session_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct TimerCompletion;

/// Repair the singleton timer rows without changing configured values. This
/// keeps databases created before the timer seed existed usable.
pub fn ensure_defaults(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer defaults transaction could not start", error)
    })?;
    let now = format_utc(Utc::now());

    transaction
        .execute(
            "INSERT INTO timer_settings (
                id, focus_minutes, short_break_minutes, long_break_minutes,
                notifications_enabled, created_at, updated_at
             ) VALUES (1, 25, 5, 15, 0, ?1, ?1)
             ON CONFLICT(id) DO NOTHING",
            [&now],
        )
        .map_err(|error| AppError::database("V2 timer settings could not be repaired", error))?;

    transaction
        .execute(
            "INSERT INTO timer_state (
                id, status, mode, duration_ms, accumulated_ms,
                last_event_duration_ms, created_at, updated_at
             ) VALUES (1, 'idle', 'focus', 0, 0, 0, ?1, ?1)
             ON CONFLICT(id) DO NOTHING",
            [&now],
        )
        .map_err(|error| AppError::database("V2 timer state could not be repaired", error))?;

    transaction
        .commit()
        .map_err(|error| AppError::database("V2 timer defaults could not be committed", error))
}

pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    ensure_topic_allocation_tables(connection)?;
    ensure_defaults(connection)?;
    recover_after_restart(connection, Utc::now())
}

/// Additive topic-level allocations. These tables are created idempotently so
/// existing schema-1 databases and existing backups remain usable without a
/// destructive schema migration.
fn ensure_topic_allocation_tables(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS timer_topic_allocations (
                exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
                node_id INTEGER NOT NULL,
                allocated_seconds INTEGER NOT NULL CHECK (allocated_seconds > 0),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (exam_id, node_id),
                FOREIGN KEY (exam_id, node_id)
                    REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
            ) STRICT;

             CREATE TABLE IF NOT EXISTS session_topic_allocations (
                session_id INTEGER NOT NULL REFERENCES study_sessions(id) ON DELETE CASCADE,
                exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
                node_id INTEGER NOT NULL,
                allocated_seconds INTEGER NOT NULL CHECK (allocated_seconds > 0),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (session_id, node_id),
                FOREIGN KEY (exam_id, node_id)
                    REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
            ) STRICT;

             CREATE INDEX IF NOT EXISTS idx_session_topic_allocations_exam
                ON session_topic_allocations(exam_id, node_id);",
        )
        .map_err(|error| {
            AppError::database(
                "The additive topic-allocation tables could not be prepared",
                error,
            )
        })
}

pub fn load_settings(connection: &Connection) -> Result<V2TimerSettings, AppError> {
    connection
        .query_row(
            "SELECT focus_minutes, short_break_minutes, long_break_minutes,
                    notifications_enabled
             FROM timer_settings WHERE id = 1",
            [],
            |row| {
                Ok(V2TimerSettings {
                    focus_minutes: row.get(0)?,
                    short_break_minutes: row.get(1)?,
                    long_break_minutes: row.get(2)?,
                    notifications_enabled: row.get::<_, i64>(3)? == 1,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("V2 timer settings could not be loaded", error))?
        .ok_or_else(AppError::timer_unavailable)
}

pub fn update_settings(
    connection: &mut Connection,
    input: V2TimerSettingsInput,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    validate_settings(&input)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer settings transaction could not start", error)
    })?;
    let changed = transaction
        .execute(
            "UPDATE timer_settings
             SET focus_minutes = ?1,
                 short_break_minutes = ?2,
                 long_break_minutes = ?3,
                 notifications_enabled = ?4,
                 updated_at = ?5
             WHERE id = 1",
            params![
                input.focus_minutes,
                input.short_break_minutes,
                input.long_break_minutes,
                i64::from(input.notifications_enabled),
                format_utc(now),
            ],
        )
        .map_err(|error| AppError::database("V2 timer settings could not be saved", error))?;
    if changed != 1 {
        return Err(AppError::timer_unavailable());
    }
    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("V2 timer settings could not be committed", error))?;
    Ok(snapshot)
}

pub fn get_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer snapshot transaction could not start", error)
    })?;
    let _ = reconcile_locked(&transaction, now)?;
    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer snapshot could not commit", error))?;
    Ok(snapshot)
}

pub fn set_timer_allocations(
    connection: &mut Connection,
    input: V2SetTimerAllocationsInput,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The timer allocation transaction could not start", error)
    })?;
    let _ = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;
    if row.status == TimerStatus::Idle {
        return Err(AppError::timer_not_running());
    }
    if row.mode != TimerMode::Focus {
        return Err(AppError::validation(
            "allocations",
            "Break timers do not have study-topic allocations.",
        ));
    }
    validate_topic_allocations(&transaction, &input.allocations, row.duration_ms / 1_000)?;
    replace_timer_topic_allocations(&transaction, &input.allocations, now)?;
    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer allocations could not commit", error))?;
    Ok(snapshot)
}

fn validate_topic_allocations(
    transaction: &Transaction<'_>,
    allocations: &[V2TopicAllocationInput],
    maximum_seconds: i64,
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    let mut total = 0_i64;
    for allocation in allocations {
        if allocation.exam_id <= 0 || allocation.node_id <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Choose an examination and topic for every allocation.",
            ));
        }
        if allocation.allocated_seconds <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Allocated time must be greater than zero.",
            ));
        }
        if !seen.insert((allocation.exam_id, allocation.node_id)) {
            return Err(AppError::validation(
                "allocations",
                "Each examination topic can appear only once in the timer allocation.",
            ));
        }
        total = total
            .checked_add(allocation.allocated_seconds)
            .ok_or_else(|| {
                AppError::validation("allocations", "The allocation total is invalid.")
            })?;
        match exam_active_state(transaction, allocation.exam_id)? {
            None => {
                return Err(AppError::validation(
                    "allocations",
                    "The selected examination does not exist.",
                ));
            }
            Some(false) => {
                return Err(AppError::forbidden(
                    "allocations",
                    "Inactive examinations cannot receive new study time.",
                ));
            }
            Some(true) => {}
        }
        let node: Option<(String, i64)> = transaction
            .query_row(
                "SELECT node_kind, is_archived FROM exam_syllabus_nodes
                 WHERE exam_id = ?1 AND id = ?2",
                params![allocation.exam_id, allocation.node_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("The selected syllabus topic could not be read", error)
            })?;
        let Some((kind, archived)) = node else {
            return Err(AppError::validation(
                "allocations",
                "The selected syllabus topic does not belong to that examination.",
            ));
        };
        if kind != "topic" || archived == 1 {
            return Err(AppError::validation(
                "allocations",
                "Choose an active topic from the examination syllabus.",
            ));
        }
    }
    if total > maximum_seconds {
        return Err(AppError::validation(
            "allocations",
            format!(
                "Topic allocations total {total} seconds, which exceeds the {maximum_seconds}-second timer duration."
            ),
        ));
    }
    Ok(())
}

fn replace_timer_topic_allocations(
    transaction: &Transaction<'_>,
    allocations: &[V2TopicAllocationInput],
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    transaction
        .execute("DELETE FROM timer_topic_allocations", [])
        .map_err(|error| {
            AppError::database(
                "Previous timer topic allocations could not be cleared",
                error,
            )
        })?;
    let now_text = format_utc(now);
    for allocation in allocations {
        transaction
            .execute(
                "INSERT INTO timer_topic_allocations (
                    exam_id, node_id, allocated_seconds, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![
                    allocation.exam_id,
                    allocation.node_id,
                    allocation.allocated_seconds,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("A timer topic allocation could not be saved", error)
            })?;
    }
    Ok(())
}

pub fn start_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
    input: V2StartTimerInput,
) -> Result<V2TimerSnapshot, AppError> {
    let mode = TimerMode::parse(&input.mode)?;
    let session_date = date_only::parse_date_only(&input.session_date).ok_or_else(|| {
        AppError::validation(
            "sessionDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    if mode != TimerMode::Focus && (input.active_exam_id.is_some() || !input.allocations.is_empty())
    {
        return Err(AppError::validation(
            "activeExamId",
            "Break timers do not allocate study time to an examination.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer start transaction could not start", error)
    })?;
    let _ = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;
    if row.status != TimerStatus::Idle {
        return Err(AppError::timer_active());
    }
    let primary_exam_id = input
        .allocations
        .first()
        .map(|allocation| allocation.exam_id)
        .or(input.active_exam_id);
    if let Some(exam_id) = input.active_exam_id {
        ensure_active_exam(&transaction, exam_id)?;
    }

    let settings = load_settings(&transaction)?;
    let duration_ms = match input.duration_minutes {
        None => mode.duration_ms(&settings),
        Some(minutes) => {
            if mode != TimerMode::Focus {
                return Err(AppError::validation(
                    "durationMinutes",
                    "Session durations can be customised only for focus timers.",
                ));
            }
            if !(1..=240).contains(&minutes) {
                return Err(AppError::validation(
                    "durationMinutes",
                    "The session duration must be between 1 and 240 minutes.",
                ));
            }
            minutes * 60_000
        }
    };
    validate_topic_allocations(&transaction, &input.allocations, duration_ms / 1_000)?;
    replace_timer_topic_allocations(&transaction, &input.allocations, now)?;
    let now_text = format_utc(now);
    let instance_id = Uuid::new_v4().simple().to_string();

    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'running',
                 mode = ?1,
                 timer_instance_id = ?2,
                 duration_ms = ?3,
                 accumulated_ms = 0,
                 segment_started_at = ?4,
                 last_heartbeat_at = ?4,
                 session_date = ?5,
                 active_exam_id = ?6,
                 concept_id = NULL,
                 syllabus_node_id = NULL,
                 planned_block_id = NULL,
                 revision_plan_item_id = NULL,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 clock_skew_detected = 0,
                 last_event = NULL,
                 last_event_at = NULL,
                 last_event_duration_ms = 0,
                 last_completed_session_id = NULL,
                 updated_at = ?4
             WHERE id = 1",
            params![
                mode.as_str(),
                instance_id,
                duration_ms,
                now_text,
                date_only::format_date_only(session_date),
                primary_exam_id,
            ],
        )
        .map_err(|error| AppError::database("The V2 study timer could not start", error))?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer start could not commit", error))?;
    Ok(snapshot)
}

pub fn pause_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer pause transaction could not start", error)
    })?;
    let completion = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;

    match row.status {
        TimerStatus::Running => pause_locked(&transaction, &row, now, now, None)?,
        TimerStatus::Paused => {}
        TimerStatus::Idle if completion.is_some() => {}
        TimerStatus::Idle => return Err(AppError::timer_not_running()),
    }

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer pause could not commit", error))?;
    Ok(snapshot)
}

pub fn resume_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer resume transaction could not start", error)
    })?;
    let _ = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;

    if row.status == TimerStatus::Idle {
        return Err(AppError::timer_not_running());
    }
    if row.status != TimerStatus::Paused {
        return Err(AppError::timer_not_paused());
    }
    if row.accumulated_ms >= row.duration_ms {
        return Err(AppError::timer_empty());
    }

    let now_text = format_utc(now);
    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'running',
                 timer_instance_id = COALESCE(timer_instance_id, ?1),
                 segment_started_at = ?2,
                 last_heartbeat_at = ?2,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 clock_skew_detected = 0,
                 last_event = NULL,
                 last_event_at = NULL,
                 last_event_duration_ms = 0,
                 last_completed_session_id = NULL,
                 updated_at = ?2
             WHERE id = 1",
            params![Uuid::new_v4().simple().to_string(), now_text],
        )
        .map_err(|error| AppError::database("The V2 study timer could not resume", error))?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer resume could not commit", error))?;
    Ok(snapshot)
}

pub fn finish_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer finish transaction could not start", error)
    })?;
    let completion = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;

    match row.status {
        TimerStatus::Idle if completion.is_some() => {}
        TimerStatus::Idle => return Err(AppError::timer_not_running()),
        TimerStatus::Running | TimerStatus::Paused => {
            complete_locked(&transaction, &row, now, now)?;
        }
    }

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer finish could not commit", error))?;
    Ok(snapshot)
}

/// Discarding never reconciles or saves a session, even if the configured
/// duration has already elapsed.
pub fn discard_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer discard transaction could not start", error)
    })?;
    let row = load_timer(&transaction)?;
    if row.status != TimerStatus::Idle {
        let elapsed_ms = match row.status {
            TimerStatus::Running => active_until(
                row.accumulated_ms,
                row.segment_started_at
                    .ok_or_else(AppError::timer_unavailable)?,
                now,
            ),
            TimerStatus::Paused => row.accumulated_ms,
            TimerStatus::Idle => 0,
        }
        .min(row.duration_ms);

        transaction
            .execute(
                "UPDATE timer_state
                 SET status = 'idle',
                     timer_instance_id = NULL,
                     accumulated_ms = 0,
                     segment_started_at = NULL,
                     last_heartbeat_at = NULL,
                     session_date = NULL,
                     active_exam_id = NULL,
                     concept_id = NULL,
                     syllabus_node_id = NULL,
                     planned_block_id = NULL,
                     revision_plan_item_id = NULL,
                     recovery_reason = NULL,
                     recovery_excluded_ms = 0,
                     clock_skew_detected = 0,
                     last_event = 'discarded',
                     last_event_at = ?1,
                     last_event_duration_ms = ?2,
                     last_completed_session_id = NULL,
                     updated_at = ?1
                 WHERE id = 1",
                params![format_utc(now), elapsed_ms],
            )
            .map_err(|error| AppError::database("The V2 timer could not be discarded", error))?;
    }
    transaction
        .execute("DELETE FROM timer_topic_allocations", [])
        .map_err(|error| {
            AppError::database(
                "The discarded timer allocations could not be cleared",
                error,
            )
        })?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer discard could not commit", error))?;
    Ok(snapshot)
}

pub fn heartbeat_timer(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer heartbeat transaction could not start", error)
    })?;
    let completion = reconcile_locked(&transaction, now)?;

    if completion.is_none() {
        let row = load_timer(&transaction)?;
        if row.status == TimerStatus::Running {
            transaction
                .execute(
                    "UPDATE timer_state
                     SET last_heartbeat_at = ?1, updated_at = ?1
                     WHERE id = 1",
                    [format_utc(now)],
                )
                .map_err(|error| {
                    AppError::database("The V2 timer heartbeat could not be stored", error)
                })?;
        }
    }

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer heartbeat could not commit", error))?;
    Ok(snapshot)
}

/// A running timer is paused at its last persisted heartbeat after a restart.
/// Time while the application was closed is never counted as study time.
pub fn recover_after_restart(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 timer recovery transaction could not start", error)
    })?;
    let row = load_timer(&transaction)?;
    if row.status == TimerStatus::Running {
        let segment_started = row
            .segment_started_at
            .ok_or_else(AppError::timer_unavailable)?;
        let last_heartbeat = row
            .last_heartbeat_at
            .unwrap_or(segment_started)
            .max(segment_started);

        if now < last_heartbeat || now < segment_started {
            pause_locked(
                &transaction,
                &row,
                last_heartbeat,
                now,
                Some((RecoveryReason::ClockChange, 0)),
            )?;
        } else {
            let elapsed_to_heartbeat =
                active_until(row.accumulated_ms, segment_started, last_heartbeat)
                    .min(row.duration_ms);
            if elapsed_to_heartbeat >= row.duration_ms {
                let end_at = segment_started
                    + chrono::Duration::milliseconds(row.duration_ms - row.accumulated_ms);
                complete_locked(&transaction, &row, now, end_at)?;
            } else {
                let excluded_ms = (now - last_heartbeat).num_milliseconds().max(0);
                pause_locked(
                    &transaction,
                    &row,
                    last_heartbeat,
                    now,
                    Some((RecoveryReason::Restart, excluded_ms)),
                )?;
            }
        }
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The V2 timer recovery could not commit", error))
}

pub fn list_sessions(connection: &Connection) -> Result<Vec<V2StudySession>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, session_date, started_at, ended_at, duration_seconds, source,
                    notes, primary_exam_id, created_at, updated_at
             FROM study_sessions
             ORDER BY session_date DESC, id DESC",
        )
        .map_err(|error| {
            AppError::database("The V2 session history could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(V2StudySession {
                id: row.get(0)?,
                session_date: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                duration_seconds: row.get(4)?,
                source: row.get(5)?,
                notes: row.get(6)?,
                primary_exam_id: row.get(7)?,
                allocated_seconds: 0,
                unallocated_seconds: 0,
                allocations: Vec::new(),
                topic_allocations: Vec::new(),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|error| AppError::database("The V2 session history could not be read", error))?;

    let mut sessions = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("The V2 session history is invalid", error))?;
    drop(statement);

    if sessions.is_empty() {
        return Ok(sessions);
    }

    let mut allocation_statement = connection
        .prepare(
            "SELECT a.session_id, a.exam_id, e.code, e.name, p.is_active,
                    a.allocated_seconds
             FROM session_exam_allocations a
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_profiles p ON p.exam_id = e.id
             ORDER BY e.display_order",
        )
        .map_err(|error| {
            AppError::database("The V2 session allocations could not be prepared", error)
        })?;
    let allocation_rows = allocation_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                V2SessionAllocation {
                    exam_id: row.get(1)?,
                    exam_code: row.get(2)?,
                    exam_name: row.get(3)?,
                    exam_is_active: row.get::<_, i64>(4)? == 1,
                    allocated_seconds: row.get(5)?,
                },
            ))
        })
        .map_err(|error| {
            AppError::database("The V2 session allocations could not be read", error)
        })?;

    let mut allocations: HashMap<i64, Vec<V2SessionAllocation>> = HashMap::new();
    for row in allocation_rows {
        let (session_id, allocation) = row
            .map_err(|error| AppError::database("A V2 session allocation row is invalid", error))?;
        allocations.entry(session_id).or_default().push(allocation);
    }

    for session in &mut sessions {
        session.allocations = allocations.remove(&session.id).unwrap_or_default();
        session.topic_allocations = load_session_topic_allocations(connection, session.id)?;
        session.allocated_seconds = session
            .allocations
            .iter()
            .map(|allocation| allocation.allocated_seconds)
            .sum();
        session.unallocated_seconds = (session.duration_seconds - session.allocated_seconds).max(0);
    }

    Ok(sessions)
}

pub fn set_session_allocations(
    connection: &mut Connection,
    input: V2SetSessionAllocationsInput,
    now: DateTime<Utc>,
) -> Result<V2StudySession, AppError> {
    if input.session_id <= 0 {
        return Err(AppError::validation(
            "sessionId",
            "The study session identifier is invalid.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The V2 allocation transaction could not start", error)
    })?;
    let duration_seconds = transaction
        .query_row(
            "SELECT duration_seconds FROM study_sessions WHERE id = ?1",
            [input.session_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| AppError::database("The study session could not be checked", error))?
        .ok_or_else(AppError::session_not_found)?;

    let mut existing: HashMap<i64, (i64, String)> = HashMap::new();
    {
        let mut statement = transaction
            .prepare(
                "SELECT exam_id, allocated_seconds, created_at
                 FROM session_exam_allocations WHERE session_id = ?1",
            )
            .map_err(|error| {
                AppError::database("Existing session allocations could not be prepared", error)
            })?;
        let rows = statement
            .query_map([input.session_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    (row.get::<_, i64>(1)?, row.get::<_, String>(2)?),
                ))
            })
            .map_err(|error| {
                AppError::database("Existing session allocations could not be read", error)
            })?;
        for row in rows {
            let (exam_id, value) = row.map_err(|error| {
                AppError::database("An existing session allocation is invalid", error)
            })?;
            existing.insert(exam_id, value);
        }
    }

    let mut seen = HashSet::new();
    let mut total_seconds = 0_i64;
    for allocation in &input.allocations {
        if allocation.exam_id <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Choose an examination from the active list.",
            ));
        }
        if allocation.allocated_seconds <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Allocated time must be greater than zero.",
            ));
        }
        if !seen.insert(allocation.exam_id) {
            return Err(AppError::validation(
                "allocations",
                "Each examination can appear only once in a session allocation.",
            ));
        }
        total_seconds = total_seconds
            .checked_add(allocation.allocated_seconds)
            .ok_or_else(|| {
                AppError::validation("allocations", "The allocation total is invalid.")
            })?;
        match exam_active_state(&transaction, allocation.exam_id)? {
            None => {
                return Err(AppError::validation(
                    "allocations",
                    "The selected examination does not exist.",
                ));
            }
            Some(false)
                if existing.get(&allocation.exam_id).map(|value| value.0)
                    != Some(allocation.allocated_seconds) =>
            {
                return Err(AppError::forbidden(
                    "allocations",
                    "Inactive examinations cannot receive new or changed study time.",
                ));
            }
            Some(_) => {}
        }
    }

    if total_seconds > duration_seconds {
        return Err(AppError::validation(
            "allocations",
            format!(
                "Exam allocations total {total_seconds} seconds, which exceeds the {duration_seconds}-second session duration."
            ),
        ));
    }

    {
        let mut statement = transaction
            .prepare(
                "SELECT exam_id, SUM(allocated_seconds)
                 FROM session_topic_allocations
                 WHERE session_id = ?1
                 GROUP BY exam_id",
            )
            .map_err(|error| {
                AppError::database("Session topic allocations could not be checked", error)
            })?;
        let rows = statement
            .query_map([input.session_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| {
                AppError::database("Session topic allocations could not be read", error)
            })?;
        for row in rows {
            let (exam_id, topic_seconds) = row.map_err(|error| {
                AppError::database("A session topic allocation is invalid", error)
            })?;
            let exam_seconds = input
                .allocations
                .iter()
                .find(|allocation| allocation.exam_id == exam_id)
                .map(|allocation| allocation.allocated_seconds)
                .unwrap_or(0);
            if topic_seconds > exam_seconds {
                return Err(AppError::validation(
                    "allocations",
                    "Topic allocations exceed the matching examination allocation. Adjust the examination allocation first.",
                ));
            }
        }
    }

    transaction
        .execute(
            "DELETE FROM session_exam_allocations WHERE session_id = ?1",
            [input.session_id],
        )
        .map_err(|error| {
            AppError::database("Previous session allocations could not be replaced", error)
        })?;

    let now_text = format_utc(now);
    for allocation in &input.allocations {
        let created_at = existing
            .get(&allocation.exam_id)
            .filter(|(seconds, _)| *seconds == allocation.allocated_seconds)
            .map(|(_, created_at)| created_at.clone())
            .unwrap_or_else(|| now_text.clone());
        transaction
            .execute(
                "INSERT INTO session_exam_allocations (
                    session_id, exam_id, allocated_seconds, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    input.session_id,
                    allocation.exam_id,
                    allocation.allocated_seconds,
                    created_at,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("The session allocation could not be saved", error)
            })?;
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The session allocations could not commit", error))?;

    list_sessions(connection)?
        .into_iter()
        .find(|session| session.id == input.session_id)
        .ok_or_else(AppError::session_not_found)
}

pub fn create_manual_session(
    connection: &mut Connection,
    input: V2CreateManualSessionInput,
    now: DateTime<Utc>,
) -> Result<V2StudySession, AppError> {
    let session_date = date_only::parse_date_only(&input.session_date).ok_or_else(|| {
        AppError::validation(
            "sessionDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    validate_session_duration(input.duration_seconds)?;
    if input.notes.chars().count() > 4_000 {
        return Err(AppError::validation(
            "notes",
            "Session notes must be 4,000 characters or fewer.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The manual session transaction could not start", error)
    })?;
    if let Some(exam_id) = input.primary_exam_id {
        ensure_active_exam(&transaction, exam_id)?;
    }
    let now_text = format_utc(now);
    transaction
        .execute(
            "INSERT INTO study_sessions (
                session_date, started_at, ended_at, duration_seconds, source, notes,
                primary_exam_id, concept_id, syllabus_node_id, created_at, updated_at
             ) VALUES (?1, NULL, NULL, ?2, 'manual', ?3, ?4, NULL, NULL, ?5, ?5)",
            params![
                date_only::format_date_only(session_date),
                input.duration_seconds,
                input.notes.trim(),
                input.primary_exam_id,
                now_text,
            ],
        )
        .map_err(|error| {
            AppError::database("The manual study session could not be saved", error)
        })?;
    let session_id = transaction.last_insert_rowid();
    if let Some(exam_id) = input.primary_exam_id {
        transaction
            .execute(
                "INSERT INTO session_exam_allocations (
                    session_id, exam_id, allocated_seconds, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![session_id, exam_id, input.duration_seconds, now_text],
            )
            .map_err(|error| {
                AppError::database("The manual session allocation could not be saved", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("The manual session could not be committed", error))?;

    load_session_by_id(connection, session_id)
}

pub fn update_study_session(
    connection: &mut Connection,
    input: V2UpdateSessionInput,
    now: DateTime<Utc>,
) -> Result<V2StudySession, AppError> {
    validate_session_id(input.session_id)?;
    let session_date = date_only::parse_date_only(&input.session_date).ok_or_else(|| {
        AppError::validation(
            "sessionDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    validate_session_duration(input.duration_seconds)?;
    if input.notes.chars().count() > 4_000 {
        return Err(AppError::validation(
            "notes",
            "Session notes must be 4,000 characters or fewer.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The study session update transaction could not start",
            error,
        )
    })?;
    let (source, old_date, old_duration, old_primary_exam): (String, String, i64, Option<i64>) =
        transaction
            .query_row(
                "SELECT source, session_date, duration_seconds, primary_exam_id
                 FROM study_sessions WHERE id = ?1",
                [input.session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| AppError::database("The study session could not be loaded", error))?
            .ok_or_else(AppError::session_not_found)?;

    let new_date = date_only::format_date_only(session_date);
    if source == "timer"
        && (old_date != new_date
            || old_duration != input.duration_seconds
            || old_primary_exam != input.primary_exam_id)
    {
        return Err(AppError::validation(
            "sessionId",
            "Timer sessions keep their recorded date, duration, and primary examination. Only notes can be edited.",
        ));
    }
    if input.primary_exam_id != old_primary_exam
        && let Some(exam_id) = input.primary_exam_id
    {
        ensure_active_exam(&transaction, exam_id)?;
    }
    let allocated: i64 = transaction
        .query_row(
            "SELECT COALESCE(SUM(allocated_seconds), 0)
             FROM session_exam_allocations WHERE session_id = ?1",
            [input.session_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Session allocations could not be checked", error))?;
    if allocated > input.duration_seconds {
        return Err(AppError::validation(
            "durationSeconds",
            format!(
                "The session duration cannot be shorter than its {allocated} allocated seconds."
            ),
        ));
    }
    let topic_allocated: i64 = transaction
        .query_row(
            "SELECT COALESCE(SUM(allocated_seconds), 0)
             FROM session_topic_allocations WHERE session_id = ?1",
            [input.session_id],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("Session topic allocations could not be checked", error)
        })?;
    if topic_allocated > input.duration_seconds {
        return Err(AppError::validation(
            "durationSeconds",
            format!(
                "The session duration cannot be shorter than its {topic_allocated} allocated topic seconds."
            ),
        ));
    }

    transaction
        .execute(
            "UPDATE study_sessions
             SET session_date = ?1,
                 duration_seconds = ?2,
                 notes = ?3,
                 primary_exam_id = ?4,
                 updated_at = ?5
             WHERE id = ?6",
            params![
                new_date,
                input.duration_seconds,
                input.notes.trim(),
                input.primary_exam_id,
                format_utc(now),
                input.session_id,
            ],
        )
        .map_err(|error| AppError::database("The study session could not be updated", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("The study session update could not be committed", error)
    })?;
    load_session_by_id(connection, input.session_id)
}

pub fn delete_study_session(connection: &mut Connection, session_id: i64) -> Result<(), AppError> {
    validate_session_id(session_id)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The study session delete transaction could not start",
            error,
        )
    })?;
    let changed = transaction
        .execute("DELETE FROM study_sessions WHERE id = ?1", [session_id])
        .map_err(|error| AppError::database("The study session could not be deleted", error))?;
    if changed != 1 {
        return Err(AppError::session_not_found());
    }
    transaction.commit().map_err(|error| {
        AppError::database("The study session deletion could not be committed", error)
    })
}

pub fn study_totals(connection: &Connection) -> Result<V2StudyTotals, AppError> {
    let (total_seconds, session_count) = connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
             FROM study_sessions",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|error| AppError::database("V2 study totals could not be read", error))?;

    let allocated_seconds = connection
        .query_row(
            "SELECT COALESCE(SUM(allocated_seconds), 0)
             FROM session_exam_allocations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| AppError::database("V2 allocated study time could not be read", error))?;

    let mut statement = connection
        .prepare(
            "SELECT e.id, e.code, e.name, p.is_active,
                    COALESCE(SUM(a.allocated_seconds), 0),
                    COUNT(DISTINCT a.session_id)
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             LEFT JOIN session_exam_allocations a ON a.exam_id = e.id
             GROUP BY e.id, e.code, e.name, p.is_active, e.display_order
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("V2 exam study totals could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(V2ExamStudyTotal {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                exam_name: row.get(2)?,
                exam_is_active: row.get::<_, i64>(3)? == 1,
                allocated_seconds: row.get(4)?,
                session_count: row.get(5)?,
            })
        })
        .map_err(|error| AppError::database("V2 exam study totals could not be read", error))?;
    let exams = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A V2 exam study total is invalid", error))?;

    let mut topic_statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, a.node_id, n.title,
                    COALESCE(parent.title, ''), SUM(a.allocated_seconds),
                    COUNT(DISTINCT a.session_id), MAX(s.session_date)
             FROM session_topic_allocations a
             JOIN study_sessions s ON s.id = a.session_id
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n
                ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
                ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             GROUP BY a.exam_id, e.code, a.node_id, n.title, parent.title, e.display_order,
                      n.display_order
             ORDER BY e.display_order, n.display_order, n.id",
        )
        .map_err(|error| {
            AppError::database("V2 topic study totals could not be prepared", error)
        })?;
    let topic_rows = topic_statement
        .query_map([], |row| {
            Ok(V2TopicStudyTotal {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                node_id: row.get(2)?,
                node_title: row.get(3)?,
                subject_title: row.get(4)?,
                allocated_seconds: row.get(5)?,
                session_count: row.get(6)?,
                last_studied_date: row.get(7)?,
            })
        })
        .map_err(|error| AppError::database("V2 topic study totals could not be read", error))?;
    let topics = topic_rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A V2 topic study total is invalid", error))?;

    Ok(V2StudyTotals {
        total_seconds,
        session_count,
        allocated_seconds,
        unallocated_seconds: (total_seconds - allocated_seconds).max(0),
        exams,
        topics,
    })
}

fn validate_settings(input: &V2TimerSettingsInput) -> Result<(), AppError> {
    if !(MIN_FOCUS_MINUTES..=MAX_FOCUS_MINUTES).contains(&input.focus_minutes) {
        return Err(AppError::validation(
            "focusMinutes",
            "Focus length must be between 1 and 240 minutes.",
        ));
    }
    if !(MIN_SHORT_BREAK_MINUTES..=MAX_SHORT_BREAK_MINUTES).contains(&input.short_break_minutes) {
        return Err(AppError::validation(
            "shortBreakMinutes",
            "Short break length must be between 1 and 60 minutes.",
        ));
    }
    if !(MIN_LONG_BREAK_MINUTES..=MAX_LONG_BREAK_MINUTES).contains(&input.long_break_minutes) {
        return Err(AppError::validation(
            "longBreakMinutes",
            "Long break length must be between 1 and 120 minutes.",
        ));
    }
    Ok(())
}

fn reconcile_locked(
    transaction: &Transaction<'_>,
    now: DateTime<Utc>,
) -> Result<Option<TimerCompletion>, AppError> {
    let row = load_timer(transaction)?;
    if row.status != TimerStatus::Running {
        return Ok(None);
    }
    let segment_started = row
        .segment_started_at
        .ok_or_else(AppError::timer_unavailable)?;
    let last_heartbeat = row
        .last_heartbeat_at
        .unwrap_or(segment_started)
        .max(segment_started);

    if now < segment_started || now < last_heartbeat {
        pause_locked(
            transaction,
            &row,
            last_heartbeat,
            now,
            Some((RecoveryReason::ClockChange, 0)),
        )?;
        return Ok(None);
    }

    let stale = (now - last_heartbeat).num_milliseconds() > STALE_AFTER_MS;
    if stale {
        let elapsed =
            active_until(row.accumulated_ms, segment_started, last_heartbeat).min(row.duration_ms);
        if elapsed >= row.duration_ms {
            let end_at = segment_started
                + chrono::Duration::milliseconds(row.duration_ms - row.accumulated_ms);
            return complete_locked(transaction, &row, now, end_at).map(Some);
        }

        let excluded_ms = (now - last_heartbeat).num_milliseconds().max(0);
        pause_locked(
            transaction,
            &row,
            last_heartbeat,
            now,
            Some((RecoveryReason::Inactivity, excluded_ms)),
        )?;
        return Ok(None);
    }

    let elapsed = active_until(row.accumulated_ms, segment_started, now).min(row.duration_ms);
    if elapsed >= row.duration_ms {
        let end_at =
            segment_started + chrono::Duration::milliseconds(row.duration_ms - row.accumulated_ms);
        return complete_locked(transaction, &row, now, end_at).map(Some);
    }

    Ok(None)
}

fn complete_locked(
    transaction: &Transaction<'_>,
    row: &TimerRow,
    now: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> Result<TimerCompletion, AppError> {
    let elapsed_ms = match row.status {
        TimerStatus::Running => active_until(
            row.accumulated_ms,
            row.segment_started_at
                .ok_or_else(AppError::timer_unavailable)?,
            end_at,
        )
        .min(row.duration_ms),
        TimerStatus::Paused => row.accumulated_ms,
        TimerStatus::Idle => return Err(AppError::timer_not_running()),
    };
    if elapsed_ms <= 0 {
        return Err(AppError::timer_empty());
    }

    let duration_seconds =
        ((elapsed_ms + 500) / 1_000).clamp(MIN_SESSION_SECONDS, MAX_SESSION_SECONDS);
    let ended_at = end_at;
    let started_at = ended_at - chrono::Duration::milliseconds(elapsed_ms.max(1));
    let now_text = format_utc(now);

    let session_id = if row.mode == TimerMode::Focus {
        let session_date = row.session_date.ok_or_else(AppError::timer_unavailable)?;
        let planned = load_timer_topic_allocations(transaction)?
            .into_iter()
            .filter(|allocation| {
                exam_active_state(transaction, allocation.exam_id)
                    .ok()
                    .flatten()
                    == Some(true)
            })
            .collect::<Vec<_>>();
        let topic_allocations = scale_topic_allocations(planned, duration_seconds);
        let primary_exam_id = topic_allocations
            .first()
            .map(|allocation| allocation.exam_id)
            .or(row.active_exam_id);
        transaction
            .execute(
                "INSERT INTO study_sessions (
                    session_date, started_at, ended_at, duration_seconds, source, notes,
                    primary_exam_id, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, 'timer', '', ?5, ?6, ?6)",
                params![
                    date_only::format_date_only(session_date),
                    format_utc(started_at),
                    format_utc(ended_at),
                    duration_seconds,
                    primary_exam_id,
                    now_text,
                ],
            )
            .map_err(|error| {
                AppError::database("The completed study session could not be saved", error)
            })?;
        let session_id = transaction.last_insert_rowid();

        if topic_allocations.is_empty() {
            if let Some(exam_id) = row.active_exam_id
                && exam_active_state(transaction, exam_id)?.unwrap_or(false)
            {
                transaction
                    .execute(
                        "INSERT INTO session_exam_allocations (
                            session_id, exam_id, allocated_seconds, created_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?4)",
                        params![session_id, exam_id, duration_seconds, now_text],
                    )
                    .map_err(|error| {
                        AppError::database("The timer session allocation could not be saved", error)
                    })?;
            }
        } else {
            let mut exam_totals: HashMap<i64, i64> = HashMap::new();
            for allocation in &topic_allocations {
                *exam_totals.entry(allocation.exam_id).or_default() += allocation.allocated_seconds;
            }
            for (exam_id, allocated_seconds) in exam_totals {
                transaction
                    .execute(
                        "INSERT INTO session_exam_allocations (
                            session_id, exam_id, allocated_seconds, created_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?4)",
                        params![session_id, exam_id, allocated_seconds, now_text],
                    )
                    .map_err(|error| {
                        AppError::database("The timer exam allocation could not be saved", error)
                    })?;
            }
            for allocation in &topic_allocations {
                transaction
                    .execute(
                        "INSERT INTO session_topic_allocations (
                            session_id, exam_id, node_id, allocated_seconds, created_at, updated_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                        params![
                            session_id,
                            allocation.exam_id,
                            allocation.node_id,
                            allocation.allocated_seconds,
                            now_text,
                        ],
                    )
                    .map_err(|error| {
                        AppError::database("The timer topic allocation could not be saved", error)
                    })?;
            }
        }

        Some(session_id)
    } else {
        None
    };

    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'idle',
                 timer_instance_id = NULL,
                 accumulated_ms = 0,
                 segment_started_at = NULL,
                 last_heartbeat_at = NULL,
                 session_date = NULL,
                 active_exam_id = NULL,
                 concept_id = NULL,
                 syllabus_node_id = NULL,
                 planned_block_id = NULL,
                 revision_plan_item_id = NULL,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 clock_skew_detected = 0,
                 last_event = 'completed',
                 last_event_at = ?1,
                 last_event_duration_ms = ?2,
                 last_completed_session_id = ?3,
                 updated_at = ?1
             WHERE id = 1",
            params![now_text, elapsed_ms, session_id],
        )
        .map_err(|error| AppError::database("The completed timer could not be stored", error))?;
    transaction
        .execute("DELETE FROM timer_topic_allocations", [])
        .map_err(|error| {
            AppError::database(
                "The completed timer allocations could not be cleared",
                error,
            )
        })?;

    Ok(TimerCompletion)
}

fn pause_locked(
    transaction: &Transaction<'_>,
    row: &TimerRow,
    cutoff: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    recovery: Option<(RecoveryReason, i64)>,
) -> Result<(), AppError> {
    let segment_started = row
        .segment_started_at
        .ok_or_else(AppError::timer_unavailable)?;
    let accumulated_ms = active_until(row.accumulated_ms, segment_started, cutoff)
        .min(row.duration_ms)
        .max(0);
    let (recovery_reason, excluded_ms) = match recovery {
        Some((reason, excluded)) => (Some(reason.as_str()), excluded.max(0)),
        None => (None, 0),
    };

    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'paused',
                 accumulated_ms = ?1,
                 segment_started_at = NULL,
                 last_heartbeat_at = NULL,
                 recovery_reason = ?2,
                 recovery_excluded_ms = ?3,
                 clock_skew_detected = ?4,
                 updated_at = ?5
             WHERE id = 1",
            params![
                accumulated_ms,
                recovery_reason,
                excluded_ms,
                i64::from(matches!(recovery, Some((RecoveryReason::ClockChange, _)))),
                format_utc(updated_at),
            ],
        )
        .map_err(|error| AppError::database("The V2 study timer could not be paused", error))?;
    Ok(())
}

fn build_snapshot(
    connection: &Connection,
    now: DateTime<Utc>,
) -> Result<V2TimerSnapshot, AppError> {
    let row = load_timer(connection)?;
    let settings = load_settings(connection)?;
    let duration_ms = match row.status {
        TimerStatus::Idle => row.mode.duration_ms(&settings),
        TimerStatus::Running | TimerStatus::Paused => row.duration_ms,
    };
    let elapsed_ms = match row.status {
        TimerStatus::Idle => 0,
        TimerStatus::Paused => row.accumulated_ms.min(duration_ms),
        TimerStatus::Running => active_until(
            row.accumulated_ms,
            row.segment_started_at
                .ok_or_else(AppError::timer_unavailable)?,
            now,
        )
        .min(duration_ms),
    };

    Ok(V2TimerSnapshot {
        state: V2TimerState {
            status: row.status,
            mode: row.mode,
            duration_ms,
            accumulated_ms: row.accumulated_ms,
            elapsed_ms,
            remaining_ms: (duration_ms - elapsed_ms).max(0),
            segment_started_at: row.segment_started_at.map(format_utc),
            session_date: row.session_date.map(date_only::format_date_only),
            active_exam_id: row.active_exam_id,
            recovery_reason: row.recovery_reason,
            recovery_excluded_ms: row.recovery_excluded_ms,
            clock_skew_detected: row.clock_skew_detected,
            last_event: row.last_event,
            last_event_at: row.last_event_at.map(format_utc),
            last_event_duration_ms: row.last_event_duration_ms,
            last_completed_session_id: row.last_completed_session_id,
        },
        settings,
        allocations: load_timer_topic_allocations(connection)?,
        server_now_ms: now.timestamp_millis(),
    })
}

fn load_timer_topic_allocations(
    connection: &Connection,
) -> Result<Vec<V2TopicAllocation>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, e.name, a.node_id, n.title,
                    COALESCE(parent.title, ''), a.allocated_seconds
             FROM timer_topic_allocations a
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n
                ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
                ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             ORDER BY e.display_order, n.display_order, n.id",
        )
        .map_err(|error| {
            AppError::database("The timer topic allocations could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], map_topic_allocation)
        .map_err(|error| {
            AppError::database("The timer topic allocations could not be read", error)
        })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A timer topic allocation is invalid", error))
}

fn load_session_topic_allocations(
    connection: &Connection,
    session_id: i64,
) -> Result<Vec<V2TopicAllocation>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, e.name, a.node_id, n.title,
                    COALESCE(parent.title, ''), a.allocated_seconds
             FROM session_topic_allocations a
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n
                ON n.exam_id = a.exam_id AND n.id = a.node_id
             LEFT JOIN exam_syllabus_nodes parent
                ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             WHERE a.session_id = ?1
             ORDER BY e.display_order, n.display_order, n.id",
        )
        .map_err(|error| {
            AppError::database("The session topic allocations could not be prepared", error)
        })?;
    let rows = statement
        .query_map([session_id], map_topic_allocation)
        .map_err(|error| {
            AppError::database("The session topic allocations could not be read", error)
        })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A session topic allocation is invalid", error))
}

fn map_topic_allocation(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2TopicAllocation> {
    Ok(V2TopicAllocation {
        exam_id: row.get(0)?,
        exam_code: row.get(1)?,
        exam_name: row.get(2)?,
        node_id: row.get(3)?,
        node_title: row.get(4)?,
        subject_title: row.get(5)?,
        allocated_seconds: row.get(6)?,
    })
}

/// Planned allocations can exceed the actual duration when a timer is stopped
/// early. Scale them proportionally so the saved topic totals never exceed the
/// recorded session duration.
fn scale_topic_allocations(
    planned: Vec<V2TopicAllocation>,
    actual_seconds: i64,
) -> Vec<V2TopicAllocation> {
    let total: i64 = planned.iter().map(|item| item.allocated_seconds).sum();
    if total <= actual_seconds {
        return planned;
    }
    let mut remaining = actual_seconds;
    let count = planned.len();
    let mut result = Vec::new();
    for (index, mut item) in planned.into_iter().enumerate() {
        let slots_left = (count - index - 1) as i64;
        if remaining <= 0 {
            break;
        }
        let share = if slots_left == 0 {
            remaining
        } else {
            ((item.allocated_seconds as f64 * actual_seconds as f64 / total as f64).floor() as i64)
                .max(1)
                .min((remaining - slots_left).max(1))
        };
        if share > 0 && share <= remaining {
            item.allocated_seconds = share;
            result.push(item);
            remaining -= share;
        }
    }
    result
}

fn load_timer(connection: &Connection) -> Result<TimerRow, AppError> {
    connection
        .query_row(TIMER_DATA, [], map_timer_row)
        .optional()
        .map_err(|error| AppError::database("The V2 timer state could not be loaded", error))?
        .ok_or_else(AppError::timer_unavailable)
}

fn map_timer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimerRow> {
    let status_text: String = row.get(0)?;
    let mode_text: String = row.get(1)?;
    let recovery_text: Option<String> = row.get(8)?;
    let last_event_text: Option<String> = row.get(11)?;
    let session_date_text: Option<String> = row.get(6)?;

    let status = TimerStatus::parse(&status_text)
        .map_err(|_| rusqlite::Error::InvalidColumnType(0, "status".to_owned(), Type::Text))?;
    let mode = TimerMode::parse(&mode_text)
        .map_err(|_| rusqlite::Error::InvalidColumnType(1, "mode".to_owned(), Type::Text))?;
    let recovery_reason = recovery_text
        .as_deref()
        .map(RecoveryReason::parse)
        .transpose()
        .map_err(|_| {
            rusqlite::Error::InvalidColumnType(8, "recovery_reason".to_owned(), Type::Text)
        })?;
    let last_event = last_event_text
        .as_deref()
        .map(LastTimerEvent::parse)
        .transpose()
        .map_err(|_| rusqlite::Error::InvalidColumnType(11, "last_event".to_owned(), Type::Text))?;
    let session_date = session_date_text
        .as_deref()
        .map(|value| {
            date_only::parse_date_only(value).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(6, "session_date".to_owned(), Type::Text)
            })
        })
        .transpose()?;

    Ok(TimerRow {
        status,
        mode,
        duration_ms: row.get(2)?,
        accumulated_ms: row.get(3)?,
        segment_started_at: read_optional_timestamp(row, 4)?,
        last_heartbeat_at: read_optional_timestamp(row, 5)?,
        session_date,
        active_exam_id: row.get(7)?,
        recovery_reason,
        recovery_excluded_ms: row.get(9)?,
        clock_skew_detected: row.get::<_, i64>(10)? == 1,
        last_event,
        last_event_at: read_optional_timestamp(row, 12)?,
        last_event_duration_ms: row.get(13)?,
        last_completed_session_id: row.get(14)?,
    })
}

fn read_optional_timestamp(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|parsed| parsed.with_timezone(&Utc))
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(index, "timestamp".to_owned(), Type::Text)
                })
        })
        .transpose()
}

fn validate_session_duration(duration_seconds: i64) -> Result<(), AppError> {
    if !(MIN_SESSION_SECONDS..=MAX_SESSION_SECONDS).contains(&duration_seconds) {
        return Err(AppError::validation(
            "durationSeconds",
            "Session duration must be between 1 second and 24 hours.",
        ));
    }
    Ok(())
}

fn validate_session_id(session_id: i64) -> Result<(), AppError> {
    if session_id <= 0 {
        return Err(AppError::validation(
            "sessionId",
            "The study session identifier is invalid.",
        ));
    }
    Ok(())
}

fn load_session_by_id(
    connection: &Connection,
    session_id: i64,
) -> Result<V2StudySession, AppError> {
    list_sessions(connection)?
        .into_iter()
        .find(|session| session.id == session_id)
        .ok_or_else(AppError::session_not_found)
}

fn active_until(accumulated_ms: i64, started_at: DateTime<Utc>, until: DateTime<Utc>) -> i64 {
    let segment_ms = (until - started_at).num_milliseconds().max(0);
    accumulated_ms.max(0).saturating_add(segment_ms)
}

fn ensure_active_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    match exam_active_state(connection, exam_id)? {
        None => Err(AppError::validation(
            "activeExamId",
            "The selected examination does not exist.",
        )),
        Some(false) => Err(AppError::forbidden(
            "activeExamId",
            "Inactive examinations cannot receive new study time.",
        )),
        Some(true) => Ok(()),
    }
}

fn exam_active_state(connection: &Connection, exam_id: i64) -> Result<Option<bool>, AppError> {
    connection
        .query_row(
            "SELECT p.is_active
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             WHERE e.id = ?1",
            [exam_id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .optional()
        .map_err(|error| AppError::database("The examination could not be checked", error))
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Keep a running timer checkpointed even if the webview is minimized or a
/// component unmounts. The database timestamps remain the source of truth.
pub fn start_heartbeat(app: tauri::AppHandle) {
    let thread = std::thread::Builder::new()
        .name("v2-study-timer-heartbeat".to_owned())
        .spawn(move || {
            loop {
                std::thread::sleep(HEARTBEAT_INTERVAL);
                let state = app.state::<crate::v2::V2State>();
                if let Err(error) = state.with_connection(|connection| {
                    heartbeat_timer(connection, Utc::now()).map(|_| ())
                }) {
                    eprintln!("V2 study timer heartbeat failed: {error:?}");
                }
            }
        });

    if let Err(error) = thread {
        eprintln!("V2 study timer heartbeat thread could not be started: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        let base = DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("base timestamp")
            .with_timezone(&Utc);
        base + chrono::Duration::seconds(seconds)
    }

    fn start_input(mode: &str, active_exam_id: Option<i64>) -> V2StartTimerInput {
        V2StartTimerInput {
            mode: mode.to_owned(),
            session_date: "2026-11-04".to_owned(),
            active_exam_id,
            allocations: Vec::new(),
            duration_minutes: None,
        }
    }

    fn start(state: &V2State, now: DateTime<Utc>, exam: Option<i64>) -> V2TimerSnapshot {
        state
            .with_connection(|connection| start_timer(connection, now, start_input("focus", exam)))
            .expect("timer should start")
    }

    fn session_count(state: &V2State) -> i64 {
        state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("test count", error))
            })
            .expect("session count")
    }

    fn checkpoint(state: &V2State, from_seconds: i64, until_seconds: i64) {
        let mut seconds = from_seconds + 10;
        while seconds < until_seconds {
            state
                .with_connection(|connection| heartbeat_timer(connection, at(seconds)))
                .expect("checkpoint heartbeat");
            seconds += 10;
        }
    }

    fn official_topic(state: &V2State, exam_id: i64, offset: i64) -> i64 {
        state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT id FROM exam_syllabus_nodes
                         WHERE exam_id = ?1 AND node_kind = 'topic'
                         ORDER BY id LIMIT 1 OFFSET ?2",
                        params![exam_id, offset],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("official topic", error))
            })
            .expect("official topic")
    }

    fn start_with_allocations(
        state: &V2State,
        now: DateTime<Utc>,
        allocations: Vec<(i64, i64, i64)>,
    ) -> Result<V2TimerSnapshot, AppError> {
        state.with_connection(|connection| {
            start_timer(
                connection,
                now,
                V2StartTimerInput {
                    mode: "focus".to_owned(),
                    session_date: "2026-11-04".to_owned(),
                    active_exam_id: None,
                    allocations: allocations
                        .into_iter()
                        .map(
                            |(exam_id, node_id, allocated_seconds)| V2TopicAllocationInput {
                                exam_id,
                                node_id,
                                allocated_seconds,
                            },
                        )
                        .collect(),
                    duration_minutes: None,
                },
            )
        })
    }

    fn allocation(
        state: &V2State,
        session_id: i64,
        allocations: Vec<(i64, i64)>,
        now: DateTime<Utc>,
    ) -> Result<V2StudySession, AppError> {
        state.with_connection(|connection| {
            set_session_allocations(
                connection,
                V2SetSessionAllocationsInput {
                    session_id,
                    allocations: allocations
                        .into_iter()
                        .map(|(exam_id, allocated_seconds)| V2SessionAllocationInput {
                            exam_id,
                            allocated_seconds,
                        })
                        .collect(),
                },
                now,
            )
        })
    }

    #[test]
    fn start_pause_resume_and_elapsed_are_timestamp_based() {
        let (_root, state) = state();
        let started = start(&state, at(0), None);

        assert_eq!(started.state.status, TimerStatus::Running);
        assert_eq!(started.state.duration_ms, 25 * 60_000);
        assert_eq!(started.state.elapsed_ms, 0);
        assert_eq!(started.state.remaining_ms, 25 * 60_000);
        checkpoint(&state, 0, 65);

        let paused = state
            .with_connection(|connection| pause_timer(connection, at(65)))
            .expect("timer should pause");
        assert_eq!(paused.state.status, TimerStatus::Paused);
        assert_eq!(paused.state.elapsed_ms, 65_000);

        let resumed = state
            .with_connection(|connection| resume_timer(connection, at(100)))
            .expect("timer should resume");
        assert_eq!(resumed.state.status, TimerStatus::Running);

        let running = state
            .with_connection(|connection| get_timer(connection, at(110)))
            .expect("timer should load");
        assert_eq!(running.state.elapsed_ms, 75_000);
        assert_eq!(running.state.remaining_ms, 25 * 60_000 - 75_000);
    }

    #[test]
    fn duplicate_active_timer_is_rejected_without_creating_a_session() {
        let (_root, state) = state();
        start(&state, at(0), None);

        let error = state
            .with_connection(|connection| {
                start_timer(connection, at(5), start_input("focus", None))
            })
            .expect_err("second timer must be rejected");

        assert_eq!(error.code, "timer_active");
        assert_eq!(session_count(&state), 0);
    }

    #[test]
    fn invalid_timer_transitions_are_rejected() {
        let (_root, state) = state();

        let pause_error = state
            .with_connection(|connection| pause_timer(connection, at(0)))
            .expect_err("idle timer cannot pause");
        assert_eq!(pause_error.code, "timer_not_running");

        start(&state, at(0), None);

        let resume_error = state
            .with_connection(|connection| resume_timer(connection, at(1)))
            .expect_err("running timer cannot resume");
        assert_eq!(resume_error.code, "timer_not_paused");

        let finish_error = state
            .with_connection(|connection| finish_timer(connection, at(0)))
            .expect_err("zero elapsed time cannot be saved");
        assert_eq!(finish_error.code, "timer_empty");
        assert_eq!(session_count(&state), 0);
    }

    #[test]
    fn finishing_early_saves_actual_time_and_allocates_to_the_active_exam() {
        let (_root, state) = state();
        start(&state, at(0), Some(1));
        checkpoint(&state, 0, 90);

        let finished = state
            .with_connection(|connection| finish_timer(connection, at(90)))
            .expect("timer should finish");
        assert_eq!(finished.state.status, TimerStatus::Idle);
        assert_eq!(finished.state.last_event, Some(LastTimerEvent::Completed));
        assert_eq!(finished.state.last_event_duration_ms, 90_000);
        assert!(finished.state.last_completed_session_id.is_some());

        let sessions = state
            .with_connection(|connection| list_sessions(connection))
            .expect("session list");
        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(session.duration_seconds, 90);
        assert_eq!(session.primary_exam_id, Some(1));
        assert_eq!(session.allocated_seconds, 90);
        assert_eq!(session.unallocated_seconds, 0);
        assert_eq!(session.allocations.len(), 1);
        assert_eq!(session.allocations[0].exam_id, 1);

        let duplicate = state
            .with_connection(|connection| finish_timer(connection, at(91)))
            .expect_err("idle timer cannot finish again");
        assert_eq!(duplicate.code, "timer_not_running");
        assert_eq!(session_count(&state), 1);
    }

    #[test]
    fn natural_completion_creates_exactly_one_full_session() {
        let (_root, state) = state();
        start(&state, at(0), Some(2));
        checkpoint(&state, 0, 25 * 60);

        let completed = state
            .with_connection(|connection| get_timer(connection, at(25 * 60)))
            .expect("timer should reconcile");
        assert_eq!(completed.state.status, TimerStatus::Idle);
        assert_eq!(completed.state.last_event, Some(LastTimerEvent::Completed));
        assert_eq!(completed.state.last_event_duration_ms, 25 * 60_000);

        let after = state
            .with_connection(|connection| heartbeat_timer(connection, at(25 * 60 + 5)))
            .expect("heartbeat after completion");
        assert_eq!(after.state.status, TimerStatus::Idle);
        assert_eq!(session_count(&state), 1);

        let sessions = state
            .with_connection(|connection| list_sessions(connection))
            .expect("session list");
        assert_eq!(sessions[0].duration_seconds, 25 * 60);
        assert_eq!(sessions[0].allocated_seconds, 25 * 60);
    }

    #[test]
    fn restart_recovery_pauses_at_the_last_heartbeat_and_excludes_closed_time() {
        let (_root, state) = state();
        start(&state, at(0), Some(1));
        state
            .with_connection(|connection| heartbeat_timer(connection, at(10)))
            .expect("heartbeat");

        state
            .with_connection(|connection| recover_after_restart(connection, at(60)))
            .expect("restart recovery");

        let recovered = state
            .with_connection(|connection| get_timer(connection, at(60)))
            .expect("recovered timer");
        assert_eq!(recovered.state.status, TimerStatus::Paused);
        assert_eq!(recovered.state.elapsed_ms, 10_000);
        assert_eq!(
            recovered.state.recovery_reason,
            Some(RecoveryReason::Restart)
        );
        assert_eq!(recovered.state.recovery_excluded_ms, 50_000);
        assert_eq!(session_count(&state), 0);

        let finished = state
            .with_connection(|connection| finish_timer(connection, at(61)))
            .expect("recovered timer should finish");
        assert_eq!(finished.state.last_event_duration_ms, 10_000);
        assert_eq!(session_count(&state), 1);
        assert_eq!(
            state
                .with_connection(|connection| list_sessions(connection))
                .expect("sessions")[0]
                .duration_seconds,
            10
        );
    }

    #[test]
    fn stale_heartbeat_pauses_instead_of_counting_unverified_time() {
        let (_root, state) = state();
        start(&state, at(0), None);
        state
            .with_connection(|connection| heartbeat_timer(connection, at(5)))
            .expect("first heartbeat");

        let stale = state
            .with_connection(|connection| heartbeat_timer(connection, at(30)))
            .expect("stale heartbeat should pause");
        assert_eq!(stale.state.status, TimerStatus::Paused);
        assert_eq!(stale.state.elapsed_ms, 5_000);
        assert_eq!(
            stale.state.recovery_reason,
            Some(RecoveryReason::Inactivity)
        );
        assert_eq!(stale.state.recovery_excluded_ms, 25_000);
        assert_eq!(session_count(&state), 0);
    }

    #[test]
    fn break_timers_never_create_study_sessions() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                start_timer(connection, at(0), start_input("shortBreak", None))
            })
            .expect("break timer should start");
        checkpoint(&state, 0, 5 * 60);

        let completed = state
            .with_connection(|connection| get_timer(connection, at(5 * 60)))
            .expect("break timer should complete");
        assert_eq!(completed.state.status, TimerStatus::Idle);
        assert_eq!(completed.state.last_event, Some(LastTimerEvent::Completed));
        assert_eq!(completed.state.last_completed_session_id, None);
        assert_eq!(completed.state.last_event_duration_ms, 5 * 60_000);
        assert_eq!(session_count(&state), 0);
    }

    #[test]
    fn discard_never_saves_a_session_and_allows_a_new_timer() {
        let (_root, state) = state();
        start(&state, at(0), Some(1));

        let discarded = state
            .with_connection(|connection| discard_timer(connection, at(120)))
            .expect("timer should discard");
        assert_eq!(discarded.state.status, TimerStatus::Idle);
        assert_eq!(discarded.state.last_event, Some(LastTimerEvent::Discarded));
        assert_eq!(discarded.state.last_event_duration_ms, 120_000);
        assert_eq!(discarded.state.last_completed_session_id, None);
        assert_eq!(session_count(&state), 0);

        let restarted = start(&state, at(130), None);
        assert_eq!(restarted.state.status, TimerStatus::Running);
    }

    #[test]
    fn inactive_and_missing_exams_cannot_start_a_timer() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_profiles SET is_active = 0 WHERE exam_id = 2",
                        [],
                    )
                    .map_err(|error| AppError::database("deactivate test exam", error))
            })
            .expect("deactivate exam");

        let inactive = state
            .with_connection(|connection| {
                start_timer(connection, at(0), start_input("focus", Some(2)))
            })
            .expect_err("inactive exam must be rejected");
        assert_eq!(inactive.code, "forbidden");

        let missing = state
            .with_connection(|connection| {
                start_timer(connection, at(0), start_input("focus", Some(99)))
            })
            .expect_err("missing exam must be rejected");
        assert_eq!(missing.code, "validation_error");
    }

    #[test]
    fn timer_settings_use_the_configured_durations_and_validate_ranges() {
        let (_root, state) = state();
        let updated = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2TimerSettingsInput {
                        focus_minutes: 45,
                        short_break_minutes: 10,
                        long_break_minutes: 20,
                        notifications_enabled: false,
                    },
                    at(0),
                )
            })
            .expect("settings should save");
        assert_eq!(updated.settings.focus_minutes, 45);

        let started = start(&state, at(1), None);
        assert_eq!(started.state.duration_ms, 45 * 60_000);

        let invalid = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2TimerSettingsInput {
                        focus_minutes: 0,
                        short_break_minutes: 10,
                        long_break_minutes: 20,
                        notifications_enabled: false,
                    },
                    at(2),
                )
            })
            .expect_err("invalid focus length must fail");
        assert_eq!(invalid.field, Some("focusMinutes"));
    }

    fn start_with_planned_duration(
        state: &V2State,
        now: DateTime<Utc>,
        mode: &str,
        minutes: Option<i64>,
        allocations: Vec<(i64, i64, i64)>,
    ) -> Result<V2TimerSnapshot, AppError> {
        state.with_connection(|connection| {
            start_timer(
                connection,
                now,
                V2StartTimerInput {
                    mode: mode.to_owned(),
                    session_date: "2026-11-04".to_owned(),
                    active_exam_id: None,
                    allocations: allocations
                        .into_iter()
                        .map(
                            |(exam_id, node_id, allocated_seconds)| V2TopicAllocationInput {
                                exam_id,
                                node_id,
                                allocated_seconds,
                            },
                        )
                        .collect(),
                    duration_minutes: minutes,
                },
            )
        })
    }

    #[test]
    fn planned_durations_below_equal_and_above_are_validated_against_the_timer() {
        let (_root, state) = state();
        let csir = official_topic(&state, 1, 0);
        let gate = official_topic(&state, 2, 0);

        // Allocation total above the planned duration is rejected.
        let above =
            start_with_planned_duration(&state, at(0), "focus", Some(30), vec![(1, csir, 31 * 60)])
                .expect_err("above the planned duration");
        assert_eq!(above.code, "validation_error");

        // Equal to the planned duration starts and honours the override.
        let equal = start_with_planned_duration(
            &state,
            at(0),
            "focus",
            Some(45),
            vec![(1, csir, 44 * 60), (2, gate, 60)],
        )
        .expect("planned focus timer");
        assert_eq!(equal.state.duration_ms, 45 * 60_000);
        assert_eq!(equal.state.remaining_ms, 45 * 60_000);
        state
            .with_connection(|connection| discard_timer(connection, at(10)))
            .expect("discard planned timer");

        // Below the planned duration is accepted too.
        let below = start_with_planned_duration(
            &state,
            at(20),
            "focus",
            Some(30),
            vec![(1, csir, 12 * 60), (2, gate, 12 * 60)],
        )
        .expect("below planned duration");
        assert_eq!(below.state.duration_ms, 30 * 60_000);
        state
            .with_connection(|connection| discard_timer(connection, at(30)))
            .expect("discard below timer");

        // Out-of-range planned durations are rejected before any timer runs.
        let zero = start_with_planned_duration(&state, at(40), "focus", Some(0), vec![])
            .expect_err("zero duration");
        assert_eq!(zero.code, "validation_error");
        let too_long = start_with_planned_duration(&state, at(40), "focus", Some(241), vec![])
            .expect_err("duration above 240");
        assert_eq!(too_long.code, "validation_error");

        // Break timers keep their own settings length and reject overrides.
        let break_override =
            start_with_planned_duration(&state, at(40), "shortBreak", Some(45), vec![])
                .expect_err("break override");
        assert_eq!(break_override.code, "validation_error");

        // Without an override the persisted settings still drive the duration.
        let plain = start_with_planned_duration(&state, at(40), "focus", None, vec![(1, csir, 60)])
            .expect("settings-driven timer");
        assert_eq!(plain.state.duration_ms, 25 * 60_000);
        assert_eq!(session_count(&state), 0);
    }

    #[test]
    fn one_shared_timer_allocates_multiple_exams_and_topics_without_double_counting() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2TimerSettingsInput {
                        focus_minutes: 2,
                        short_break_minutes: 5,
                        long_break_minutes: 15,
                        notifications_enabled: false,
                    },
                    at(0),
                )
            })
            .expect("two minute timer");
        let csir = official_topic(&state, 1, 0);
        let gate = official_topic(&state, 2, 0);
        let tifr = official_topic(&state, 3, 0);
        let started = start_with_allocations(
            &state,
            at(0),
            vec![(1, csir, 60), (2, gate, 30), (3, tifr, 30)],
        )
        .expect("multi-exam timer");
        assert_eq!(started.allocations.len(), 3);
        assert_eq!(started.state.duration_ms, 120_000);
        checkpoint(&state, 0, 120);
        let finished = state
            .with_connection(|connection| finish_timer(connection, at(120)))
            .expect("finish shared session");
        let session_id = finished
            .state
            .last_completed_session_id
            .expect("saved session");
        let sessions = state
            .with_connection(|connection| list_sessions(connection))
            .expect("sessions");
        let session = sessions
            .iter()
            .find(|session| session.id == session_id)
            .expect("session");
        assert_eq!(session.duration_seconds, 120);
        assert_eq!(session.allocated_seconds, 120);
        assert_eq!(session.topic_allocations.len(), 3);
        assert_eq!(
            session
                .topic_allocations
                .iter()
                .map(|allocation| allocation.allocated_seconds)
                .sum::<i64>(),
            120
        );
        assert_eq!(session.allocations.len(), 3);

        let totals = state
            .with_connection(|connection| study_totals(connection))
            .expect("study totals");
        assert_eq!(totals.session_count, 1);
        assert_eq!(totals.total_seconds, 120);
        assert_eq!(totals.allocated_seconds, 120);
        assert_eq!(
            totals
                .exams
                .iter()
                .map(|exam| exam.allocated_seconds)
                .sum::<i64>(),
            120
        );
        assert_eq!(totals.topics.len(), 3);
        assert_eq!(
            totals
                .topics
                .iter()
                .map(|topic| topic.allocated_seconds)
                .sum::<i64>(),
            120
        );

        let mastery_evidence: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM concept_evidence", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| AppError::database("mastery evidence count", error))
            })
            .expect("mastery evidence");
        assert_eq!(
            mastery_evidence, 0,
            "studying must not create mastery evidence"
        );
    }

    #[test]
    fn timer_allocations_reject_overruns_and_survive_restart_scaling() {
        let (_root, state) = state();
        let csir = official_topic(&state, 1, 0);
        let gate = official_topic(&state, 2, 0);
        state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2TimerSettingsInput {
                        focus_minutes: 1,
                        short_break_minutes: 5,
                        long_break_minutes: 15,
                        notifications_enabled: false,
                    },
                    at(0),
                )
            })
            .expect("one minute timer");
        let overrun = start_with_allocations(&state, at(0), vec![(1, csir, 60), (2, gate, 60)])
            .expect_err("allocations cannot exceed the timer duration");
        assert_eq!(overrun.code, "validation_error");
        assert_eq!(overrun.field, Some("allocations"));

        let started = start_with_allocations(&state, at(1), vec![(1, csir, 30), (2, gate, 30)])
            .expect("two topic timer");
        assert_eq!(started.allocations.len(), 2);
        state
            .with_connection(|connection| heartbeat_timer(connection, at(11)))
            .expect("heartbeat");
        state
            .with_connection(|connection| recover_after_restart(connection, at(60)))
            .expect("restart recovery");
        let recovered = state
            .with_connection(|connection| get_timer(connection, at(60)))
            .expect("recovered timer");
        assert_eq!(recovered.state.status, TimerStatus::Paused);
        assert_eq!(recovered.allocations.len(), 2);
        let finished = state
            .with_connection(|connection| finish_timer(connection, at(61)))
            .expect("finish recovered timer");
        let sessions = state
            .with_connection(|connection| list_sessions(connection))
            .expect("sessions");
        let session = sessions
            .iter()
            .find(|session| Some(session.id) == finished.state.last_completed_session_id)
            .expect("session");
        assert_eq!(session.duration_seconds, 10);
        assert!(
            session
                .topic_allocations
                .iter()
                .all(|item| item.allocated_seconds >= 1)
        );
        assert!(
            session
                .topic_allocations
                .iter()
                .map(|item| item.allocated_seconds)
                .sum::<i64>()
                <= session.duration_seconds
        );
    }

    #[test]
    fn copied_database_gains_additive_tables_without_changing_legacy_sessions() {
        use std::fs;

        let source_root = tempfile::tempdir().expect("source directory");
        let source_dir = source_root.path().join("v2");
        let source = V2State::initialize(&source_dir);
        source
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2TimerSettingsInput {
                        focus_minutes: 1,
                        short_break_minutes: 5,
                        long_break_minutes: 15,
                        notifications_enabled: false,
                    },
                    at(0),
                )
            })
            .expect("one minute timer");
        source
            .with_connection(|connection| {
                start_timer(connection, at(0), start_input("focus", Some(1)))
            })
            .expect("legacy timer");
        checkpoint(&source, 0, 60);
        let completed = source
            .with_connection(|connection| finish_timer(connection, at(60)))
            .expect("legacy timer session");
        let timer_session_id = completed
            .state
            .last_completed_session_id
            .expect("legacy timer session id");
        let manual = source
            .with_connection(|connection| {
                create_manual_session(
                    connection,
                    V2CreateManualSessionInput {
                        session_date: "2026-11-03".to_owned(),
                        duration_seconds: 600,
                        notes: "Legacy manual session".to_owned(),
                        primary_exam_id: None,
                    },
                    at(100),
                )
            })
            .expect("legacy manual session");

        let before = source
            .with_connection(|connection| list_sessions(connection))
            .expect("legacy sessions");
        assert_eq!(before.len(), 2);
        let legacy_timer = before
            .iter()
            .find(|session| session.id == timer_session_id)
            .expect("legacy timer session");
        assert_eq!(legacy_timer.allocations.len(), 1);
        assert_eq!(legacy_timer.allocations[0].allocated_seconds, 60);
        assert!(legacy_timer.topic_allocations.is_empty());
        let legacy_manual = before
            .iter()
            .find(|session| session.id == manual.id)
            .expect("legacy manual session");
        assert!(legacy_manual.allocations.is_empty());
        assert!(legacy_manual.topic_allocations.is_empty());
        let before_totals = source
            .with_connection(|connection| study_totals(connection))
            .expect("legacy totals");
        assert_eq!(before_totals.total_seconds, 660);
        assert_eq!(before_totals.allocated_seconds, 60);

        // Simulate a schema-1 database created before the additive tables
        // existed. The base schema, checksum, and user version are untouched.
        source
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "DROP TABLE session_topic_allocations;
                         DROP TABLE timer_topic_allocations;",
                    )
                    .map_err(|error| {
                        AppError::database("The additive tables could not be removed", error)
                    })
            })
            .expect("legacy table shape");
        let additive_tables: i64 = source
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master
                         WHERE type = 'table'
                           AND name IN ('session_topic_allocations', 'timer_topic_allocations')",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("table count", error))
            })
            .expect("table count");
        assert_eq!(additive_tables, 0);
        source
            .with_connection(|connection| {
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(|error| AppError::database("checkpoint", error))
            })
            .expect("wal checkpoint");
        drop(source);

        let destination_root = tempfile::tempdir().expect("destination directory");
        let destination_dir = destination_root.path().join("v2");
        fs::create_dir_all(&destination_dir).expect("destination folder");
        for entry in fs::read_dir(&source_dir).expect("source listing") {
            let entry = entry.expect("source entry");
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("physics-tracker.sqlite3") {
                fs::copy(entry.path(), destination_dir.join(&name)).expect("database copy");
            }
        }

        let copied = V2State::initialize(&destination_dir);
        let after = copied
            .with_connection(|connection| list_sessions(connection))
            .expect("copied sessions");
        assert_eq!(after.len(), before.len());
        let copied_timer = after
            .iter()
            .find(|session| session.id == timer_session_id)
            .expect("copied timer session");
        assert_eq!(copied_timer.duration_seconds, 60);
        assert_eq!(copied_timer.allocations.len(), 1);
        assert_eq!(copied_timer.allocations[0].exam_id, 1);
        assert_eq!(copied_timer.allocations[0].allocated_seconds, 60);
        assert!(copied_timer.topic_allocations.is_empty());
        let copied_manual = after
            .iter()
            .find(|session| session.id == manual.id)
            .expect("copied manual session");
        assert_eq!(copied_manual.notes, "Legacy manual session");
        assert!(copied_manual.allocations.is_empty());
        assert!(copied_manual.topic_allocations.is_empty());

        let after_totals = copied
            .with_connection(|connection| study_totals(connection))
            .expect("copied totals");
        assert_eq!(after_totals.total_seconds, before_totals.total_seconds);
        assert_eq!(after_totals.session_count, 2);
        assert_eq!(
            after_totals.allocated_seconds,
            before_totals.allocated_seconds
        );
        assert!(
            after_totals.topics.is_empty(),
            "no historical topic rows are fabricated"
        );

        let (additive_tables, topic_rows, exam_allocation_rows, integrity, foreign_keys): (
            i64,
            i64,
            i64,
            String,
            i64,
        ) = copied
            .with_connection(|connection| {
                let additive_tables = connection
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master
                         WHERE type = 'table'
                           AND name IN ('session_topic_allocations', 'timer_topic_allocations')",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("additive tables", error))?;
                let topic_rows = connection
                    .query_row(
                        "SELECT COUNT(*) FROM session_topic_allocations",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("topic rows", error))?;
                let exam_allocation_rows = connection
                    .query_row("SELECT COUNT(*) FROM session_exam_allocations", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| AppError::database("exam rows", error))?;
                let integrity = connection
                    .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                    .map_err(|error| AppError::database("integrity check", error))?;
                let foreign_keys = connection
                    .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| AppError::database("foreign key check", error))?;
                Ok((
                    additive_tables,
                    topic_rows,
                    exam_allocation_rows,
                    integrity,
                    foreign_keys,
                ))
            })
            .expect("copied integrity");
        assert_eq!(additive_tables, 2);
        assert_eq!(topic_rows, 0);
        assert_eq!(exam_allocation_rows, 1);
        assert_eq!(integrity, "ok");
        assert_eq!(foreign_keys, 0);
        assert_eq!(before_totals.session_count, 2);
    }

    #[test]
    fn allocations_validate_limits_and_preserve_unallocated_time() {
        let (_root, state) = state();
        start(&state, at(0), None);
        checkpoint(&state, 0, 600);
        let finished = state
            .with_connection(|connection| finish_timer(connection, at(600)))
            .expect("finish");
        let session_id = finished
            .state
            .last_completed_session_id
            .expect("session id");

        let one_exam = allocation(&state, session_id, vec![(1, 600)], at(700))
            .expect("single exam allocation");
        assert_eq!(one_exam.allocated_seconds, 600);
        assert_eq!(one_exam.unallocated_seconds, 0);

        let split = allocation(&state, session_id, vec![(1, 250), (2, 200)], at(701))
            .expect("multi exam allocation");
        assert_eq!(split.allocated_seconds, 450);
        assert_eq!(split.unallocated_seconds, 150);
        assert_eq!(split.allocations.len(), 2);

        let exceeded = allocation(&state, session_id, vec![(1, 400), (2, 300)], at(702))
            .expect_err("allocation must not exceed duration");
        assert_eq!(exceeded.code, "validation_error");
        assert_eq!(exceeded.field, Some("allocations"));

        let duplicate = allocation(&state, session_id, vec![(1, 100), (1, 100)], at(703))
            .expect_err("duplicate exam must fail");
        assert_eq!(duplicate.field, Some("allocations"));

        let zero = allocation(&state, session_id, vec![(1, 0)], at(704))
            .expect_err("zero allocation is rejected");
        assert_eq!(zero.field, Some("allocations"));

        let empty = allocation(&state, session_id, vec![], at(705))
            .expect("empty allocation clears the session");
        assert_eq!(empty.allocated_seconds, 0);
        assert_eq!(empty.unallocated_seconds, 600);
        assert!(empty.allocations.is_empty());
    }

    #[test]
    fn inactive_exams_reject_new_allocations_but_keep_history() {
        let (_root, state) = state();
        start(&state, at(0), None);
        checkpoint(&state, 0, 600);
        let session_id = state
            .with_connection(|connection| finish_timer(connection, at(600)))
            .expect("finish")
            .state
            .last_completed_session_id
            .expect("session id");

        allocation(&state, session_id, vec![(2, 120)], at(601))
            .expect("active allocation should save");

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_profiles SET is_active = 0 WHERE exam_id = 2",
                        [],
                    )
                    .map_err(|error| AppError::database("deactivate test exam", error))
            })
            .expect("deactivate exam");

        let unchanged = allocation(&state, session_id, vec![(1, 200), (2, 120)], at(602))
            .expect("unchanged historical allocation should survive");
        assert_eq!(unchanged.allocated_seconds, 320);
        let historical = unchanged
            .allocations
            .iter()
            .find(|allocation| allocation.exam_id == 2)
            .expect("historical allocation");
        assert!(!historical.exam_is_active);

        let changed = allocation(&state, session_id, vec![(1, 200), (2, 150)], at(603))
            .expect_err("inactive allocation cannot increase");
        assert_eq!(changed.code, "forbidden");

        let new_inactive = allocation(&state, session_id, vec![(2, 120), (3, 100)], at(604))
            .expect("active exam can still be added");
        assert_eq!(new_inactive.allocated_seconds, 220);

        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE exam_profiles SET is_active = 0 WHERE exam_id = 3",
                        [],
                    )
                    .map_err(|error| AppError::database("deactivate test exam", error))
            })
            .expect("deactivate exam");
        let rejected = allocation(&state, session_id, vec![(2, 120), (3, 120)], at(605))
            .expect_err("new inactive allocation must fail");
        assert_eq!(rejected.code, "forbidden");
    }

    #[test]
    fn totals_count_each_session_once_and_use_allocations_for_exams() {
        let (_root, state) = state();

        start(&state, at(0), None);
        checkpoint(&state, 0, 600);
        let first = state
            .with_connection(|connection| finish_timer(connection, at(600)))
            .expect("first session")
            .state
            .last_completed_session_id
            .expect("id");
        allocation(&state, first, vec![(1, 100)], at(601)).expect("first allocation");

        start(&state, at(700), None);
        checkpoint(&state, 700, 1000);
        let second = state
            .with_connection(|connection| finish_timer(connection, at(1000)))
            .expect("second session")
            .state
            .last_completed_session_id
            .expect("id");
        allocation(&state, second, vec![(1, 100), (2, 50)], at(1001)).expect("second allocation");

        let totals = state
            .with_connection(|connection| study_totals(connection))
            .expect("totals");
        assert_eq!(totals.total_seconds, 900);
        assert_eq!(totals.session_count, 2);
        assert_eq!(totals.allocated_seconds, 250);
        assert_eq!(totals.unallocated_seconds, 650);

        let csir = totals
            .exams
            .iter()
            .find(|exam| exam.exam_id == 1)
            .expect("csir total");
        assert_eq!(csir.allocated_seconds, 200);
        assert_eq!(csir.session_count, 2);

        let gate = totals
            .exams
            .iter()
            .find(|exam| exam.exam_id == 2)
            .expect("gate total");
        assert_eq!(gate.allocated_seconds, 50);
        assert_eq!(gate.session_count, 1);
    }

    #[test]
    fn session_history_is_empty_before_any_study_and_orders_newest_first() {
        let (_root, state) = state();
        assert!(
            state
                .with_connection(|connection| list_sessions(connection))
                .expect("empty history")
                .is_empty()
        );

        start(&state, at(0), None);
        checkpoint(&state, 0, 60);
        state
            .with_connection(|connection| finish_timer(connection, at(60)))
            .expect("first");
        start(&state, at(100), None);
        checkpoint(&state, 100, 220);
        state
            .with_connection(|connection| finish_timer(connection, at(220)))
            .expect("second");

        let sessions = state
            .with_connection(|connection| list_sessions(connection))
            .expect("history");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].duration_seconds, 120);
        assert_eq!(sessions[1].duration_seconds, 60);
    }

    #[test]
    fn manual_sessions_create_update_delete_and_allocate() {
        let (_root, state) = state();
        let created = state
            .with_connection(|connection| {
                create_manual_session(
                    connection,
                    V2CreateManualSessionInput {
                        session_date: "2026-11-04".to_owned(),
                        duration_seconds: 1_200,
                        notes: "Manual session".to_owned(),
                        primary_exam_id: Some(1),
                    },
                    at(0),
                )
            })
            .expect("manual session");
        assert_eq!(created.source, "manual");
        assert_eq!(created.duration_seconds, 1_200);
        assert_eq!(created.allocated_seconds, 1_200);

        let updated = state
            .with_connection(|connection| {
                update_study_session(
                    connection,
                    V2UpdateSessionInput {
                        session_id: created.id,
                        session_date: "2026-11-05".to_owned(),
                        duration_seconds: 1_500,
                        notes: "Edited note".to_owned(),
                        primary_exam_id: Some(1),
                    },
                    at(1),
                )
            })
            .expect("updated");
        assert_eq!(updated.notes, "Edited note");
        assert_eq!(updated.session_date, "2026-11-05");
        assert_eq!(updated.duration_seconds, 1_500);

        let invalid = state
            .with_connection(|connection| {
                update_study_session(
                    connection,
                    V2UpdateSessionInput {
                        session_id: created.id,
                        session_date: "2026-11-05".to_owned(),
                        duration_seconds: 500,
                        notes: String::new(),
                        primary_exam_id: Some(1),
                    },
                    at(2),
                )
            })
            .expect_err("duration below allocation");
        assert_eq!(invalid.field, Some("durationSeconds"));

        state
            .with_connection(|connection| delete_study_session(connection, created.id))
            .expect("delete");
        assert!(
            state
                .with_connection(|connection| list_sessions(connection))
                .expect("history")
                .is_empty()
        );
    }

    #[test]
    fn timer_sessions_only_allow_note_edits() {
        let (_root, state) = state();
        start(&state, at(0), Some(1));
        checkpoint(&state, 0, 60);
        let session_id = state
            .with_connection(|connection| finish_timer(connection, at(60)))
            .expect("finish")
            .state
            .last_completed_session_id
            .expect("session id");
        let notes = state
            .with_connection(|connection| {
                update_study_session(
                    connection,
                    V2UpdateSessionInput {
                        session_id,
                        session_date: "2026-11-04".to_owned(),
                        duration_seconds: 60,
                        notes: "After-session note".to_owned(),
                        primary_exam_id: Some(1),
                    },
                    at(61),
                )
            })
            .expect("notes");
        assert_eq!(notes.notes, "After-session note");

        let invalid = state
            .with_connection(|connection| {
                update_study_session(
                    connection,
                    V2UpdateSessionInput {
                        session_id,
                        session_date: "2026-11-04".to_owned(),
                        duration_seconds: 61,
                        notes: String::new(),
                        primary_exam_id: Some(1),
                    },
                    at(62),
                )
            })
            .expect_err("timer duration is immutable");
        assert_eq!(invalid.code, "validation_error");
    }
}
