use std::time::Duration as StdDuration;

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{date_only, error::AppError, sessions};

pub const HEARTBEAT_INTERVAL: StdDuration = StdDuration::from_secs(5);
pub const STALE_AFTER_MS: i64 = 20_000;
pub const DEFAULT_FOCUS_MINUTES: i64 = 25;
pub const DEFAULT_SHORT_BREAK_MINUTES: i64 = 5;
pub const DEFAULT_LONG_BREAK_MINUTES: i64 = 15;
pub const MIN_FOCUS_MINUTES: i64 = 1;
pub const MAX_FOCUS_MINUTES: i64 = 240;
pub const MIN_SHORT_BREAK_MINUTES: i64 = 1;
pub const MAX_SHORT_BREAK_MINUTES: i64 = 60;
pub const MIN_LONG_BREAK_MINUTES: i64 = 1;
pub const MAX_LONG_BREAK_MINUTES: i64 = 120;

const SELECT_SETTINGS: &str = "SELECT focus_minutes, short_break_minutes, long_break_minutes, \
     notifications_enabled FROM timer_settings WHERE id = 1";

const SELECT_TIMER: &str = "SELECT
        status,
        mode,
        duration_ms,
        accumulated_ms,
        first_started_at,
        segment_started_at,
        last_heartbeat_at,
        topic_id,
        session_date,
        recovery_reason,
        recovery_excluded_ms,
        recovery_at,
        last_event,
        last_event_at,
        last_event_duration_ms,
        last_completed_session_id
     FROM timer_state WHERE id = 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerMode {
    Focus,
    ShortBreak,
    LongBreak,
}

impl TimerMode {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "focus" => Ok(Self::Focus),
            "short_break" => Ok(Self::ShortBreak),
            "long_break" => Ok(Self::LongBreak),
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

    fn duration_ms(self, settings: &TimerSettings) -> i64 {
        let minutes = match self {
            Self::Focus => settings.focus_minutes,
            Self::ShortBreak => settings.short_break_minutes,
            Self::LongBreak => settings.long_break_minutes,
        };

        minutes * 60_000
    }

    fn notification_copy(self, duration_ms: i64) -> (&'static str, String) {
        match self {
            Self::Focus => (
                "Focus session complete",
                format!(
                    "{} recorded. Take a break when you are ready.",
                    format_minutes(duration_ms)
                ),
            ),
            Self::ShortBreak => (
                "Short break complete",
                "Ready for the next focus session?".to_owned(),
            ),
            Self::LongBreak => (
                "Long break complete",
                "Ready for the next focus session?".to_owned(),
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    Restart,
    Inactivity,
}

impl RecoveryReason {
    fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "restart" => Ok(Self::Restart),
            "inactivity" => Ok(Self::Inactivity),
            _ => Err(AppError::timer_unavailable()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Restart => "restart",
            Self::Inactivity => "inactivity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
pub struct TimerSettings {
    pub focus_minutes: i64,
    pub short_break_minutes: i64,
    pub long_break_minutes: i64,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSettingsInput {
    pub focus_minutes: i64,
    pub short_break_minutes: i64,
    pub long_break_minutes: i64,
    pub notifications_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerTopic {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerRecovery {
    pub reason: RecoveryReason,
    pub excluded_ms: i64,
    pub recovered_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    pub status: TimerStatus,
    pub mode: TimerMode,
    pub duration_ms: i64,
    pub accumulated_ms: i64,
    pub remaining_ms: i64,
    pub first_started_at: Option<String>,
    pub session_date: Option<String>,
    pub topic: Option<TimerTopic>,
    pub last_event: Option<LastTimerEvent>,
    pub last_event_at: Option<String>,
    pub last_event_duration_ms: i64,
    pub last_completed_session_id: Option<i64>,
    pub recovery: Option<TimerRecovery>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSnapshot {
    pub state: TimerState,
    pub settings: TimerSettings,
    pub server_now_ms: i64,
}

#[derive(Debug, Clone)]
struct TimerRow {
    status: TimerStatus,
    mode: TimerMode,
    duration_ms: i64,
    accumulated_ms: i64,
    first_started_at: Option<DateTime<Utc>>,
    segment_started_at: Option<DateTime<Utc>>,
    last_heartbeat_at: Option<DateTime<Utc>>,
    topic_id: Option<i64>,
    session_date: Option<NaiveDate>,
    recovery_reason: Option<RecoveryReason>,
    recovery_excluded_ms: i64,
    recovery_at: Option<DateTime<Utc>>,
    last_event: Option<LastTimerEvent>,
    last_event_at: Option<DateTime<Utc>>,
    last_event_duration_ms: i64,
    last_completed_session_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TimerCompletion {
    pub mode: TimerMode,
    pub duration_ms: i64,
}

#[derive(Debug, Clone)]
pub struct TimerNotice {
    pub mode: TimerMode,
    pub duration_ms: i64,
}

/// Create the singleton timer settings and state rows without overwriting user
/// values. The migration inserts both rows for new databases; this also repairs
/// an accidentally missing row without fabricating study sessions.
pub fn ensure_defaults(connection: &mut Connection) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The timer settings transaction could not be started", error)
    })?;

    transaction
        .execute(
            "INSERT INTO timer_settings (
                id, focus_minutes, short_break_minutes, long_break_minutes,
                notifications_enabled, created_at, updated_at
             ) VALUES (
                1, ?1, ?2, ?3, 0,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )
             ON CONFLICT(id) DO NOTHING",
            params![
                DEFAULT_FOCUS_MINUTES,
                DEFAULT_SHORT_BREAK_MINUTES,
                DEFAULT_LONG_BREAK_MINUTES,
            ],
        )
        .map_err(|error| AppError::database("Default timer settings could not be stored", error))?;

    transaction
        .execute(
            "INSERT INTO timer_state (
                id, status, mode, duration_ms, accumulated_ms,
                last_event_duration_ms, created_at, updated_at
             ) VALUES (
                1, 'idle', 'focus', 0, 0, 0,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )
             ON CONFLICT(id) DO NOTHING",
            [],
        )
        .map_err(|error| AppError::database("Default timer state could not be stored", error))?;

    transaction
        .commit()
        .map_err(|error| AppError::database("Default timer state could not be committed", error))
}

pub fn load_settings(connection: &Connection) -> Result<TimerSettings, AppError> {
    connection
        .query_row(SELECT_SETTINGS, [], |row| {
            Ok(TimerSettings {
                focus_minutes: row.get(0)?,
                short_break_minutes: row.get(1)?,
                long_break_minutes: row.get(2)?,
                notifications_enabled: row.get::<_, i64>(3)? != 0,
            })
        })
        .optional()
        .map_err(|error| AppError::database("Timer settings could not be loaded", error))?
        .ok_or_else(AppError::timer_unavailable)
}

pub fn update_settings(
    connection: &mut Connection,
    input: TimerSettingsInput,
) -> Result<TimerSettings, AppError> {
    validate_settings(&input)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The timer settings transaction could not be started", error)
    })?;

    let changed = transaction
        .execute(
            "UPDATE timer_settings
             SET focus_minutes = ?1,
                 short_break_minutes = ?2,
                 long_break_minutes = ?3,
                 notifications_enabled = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![
                input.focus_minutes,
                input.short_break_minutes,
                input.long_break_minutes,
                i64::from(input.notifications_enabled),
            ],
        )
        .map_err(|error| AppError::database("Timer settings could not be saved", error))?;

    if changed != 1 {
        return Err(AppError::timer_unavailable());
    }

    let settings = load_settings(&transaction)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("Timer settings could not be committed", error))?;

    Ok(settings)
}

pub fn snapshot(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<TimerSnapshot, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

    let _ = reconcile_locked(&transaction, now)?;
    let snapshot = build_snapshot(&transaction, now)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The timer snapshot could not be committed", error))?;

    Ok(snapshot)
}

pub fn start(
    connection: &mut Connection,
    now: DateTime<Utc>,
    local_date: NaiveDate,
    mode: &str,
    topic_id: Option<i64>,
) -> Result<TimerSnapshot, AppError> {
    let mode = TimerMode::parse(mode)?;
    if let Some(topic_id) = topic_id {
        ensure_active_topic(connection, topic_id)?;
    }

    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

    let _ = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;
    if row.status != TimerStatus::Idle {
        return Err(AppError::timer_active());
    }

    let settings = load_settings(&transaction)?;
    let duration_ms = mode.duration_ms(&settings);
    let now_text = format_utc(now);
    let session_date = date_only::format_date_only(local_date);

    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'running',
                 mode = ?1,
                 duration_ms = ?2,
                 accumulated_ms = 0,
                 first_started_at = ?3,
                 segment_started_at = ?3,
                 last_heartbeat_at = ?3,
                 topic_id = ?4,
                 session_date = ?5,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 recovery_at = NULL,
                 last_event = NULL,
                 last_event_at = NULL,
                 last_event_duration_ms = 0,
                 last_completed_session_id = NULL,
                 updated_at = ?3
             WHERE id = 1",
            params![mode.as_str(), duration_ms, now_text, topic_id, session_date],
        )
        .map_err(|error| AppError::database("The study timer could not be started", error))?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer start could not be committed", error))?;

    Ok(snapshot)
}

pub fn pause(connection: &mut Connection, now: DateTime<Utc>) -> Result<TimerSnapshot, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

    let completion = reconcile_locked(&transaction, now)?;
    let row = load_timer(&transaction)?;

    match row.status {
        TimerStatus::Running => {
            pause_locked(&transaction, &row, now, now, None)?;
        }
        TimerStatus::Paused => {}
        TimerStatus::Idle if completion.is_none() => return Err(AppError::timer_not_running()),
        TimerStatus::Idle => {}
    }

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer pause could not be committed", error))?;

    Ok(snapshot)
}

pub fn resume(connection: &mut Connection, now: DateTime<Utc>) -> Result<TimerSnapshot, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

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
                 segment_started_at = ?1,
                 last_heartbeat_at = ?1,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 recovery_at = NULL,
                 last_event = NULL,
                 last_event_at = NULL,
                 last_event_duration_ms = 0,
                 last_completed_session_id = NULL,
                 updated_at = ?1
             WHERE id = 1",
            [now_text],
        )
        .map_err(|error| AppError::database("The study timer could not be resumed", error))?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer resume could not be committed", error))?;

    Ok(snapshot)
}

pub fn finish(connection: &mut Connection, now: DateTime<Utc>) -> Result<TimerSnapshot, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

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
    transaction.commit().map_err(|error| {
        AppError::database("The timer completion could not be committed", error)
    })?;

    Ok(snapshot)
}

/// Discarding never reconciles or saves a session, even if the configured
/// duration has elapsed. This guarantees discarded timers cannot become study
/// time by accident.
pub fn discard(connection: &mut Connection, now: DateTime<Utc>) -> Result<TimerSnapshot, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The timer transaction could not be started", error))?;

    let row = load_timer(&transaction)?;
    if row.status == TimerStatus::Idle {
        let snapshot = build_snapshot(&transaction, now)?;
        transaction.commit().map_err(|error| {
            AppError::database("The timer discard could not be committed", error)
        })?;
        return Ok(snapshot);
    }

    let active_elapsed = match row.status {
        TimerStatus::Running => {
            row.accumulated_ms
                + row
                    .segment_started_at
                    .map(|started| (now - started).num_milliseconds().max(0))
                    .unwrap_or(0)
        }
        TimerStatus::Paused => row.accumulated_ms,
        TimerStatus::Idle => 0,
    }
    .min(row.duration_ms);

    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'idle',
                 accumulated_ms = 0,
                 first_started_at = NULL,
                 segment_started_at = NULL,
                 last_heartbeat_at = NULL,
                 topic_id = NULL,
                 session_date = NULL,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 recovery_at = NULL,
                 last_event = 'discarded',
                 last_event_at = ?1,
                 last_event_duration_ms = ?2,
                 last_completed_session_id = NULL,
                 updated_at = ?1
             WHERE id = 1",
            params![format_utc(now), active_elapsed],
        )
        .map_err(|error| AppError::database("The study timer could not be discarded", error))?;

    let snapshot = build_snapshot(&transaction, now)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer discard could not be committed", error))?;

    Ok(snapshot)
}

/// Called once when the database opens. A running timer is paused at its last
/// persisted heartbeat, so time while the app was closed is not counted.
pub fn recover_after_restart(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The timer recovery transaction could not be started", error)
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
        let elapsed_to_heartbeat =
            active_until(row.accumulated_ms, segment_started, last_heartbeat);

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
                Some((RecoveryReason::Restart, excluded_ms, now)),
            )?;
        }
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The timer recovery could not be committed", error))
}

/// Persist a heartbeat and complete/pause the timer if it crossed a boundary.
/// The returned notice is Some only when notifications are enabled and the
/// timer naturally completed.
pub fn heartbeat(
    connection: &mut Connection,
    now: DateTime<Utc>,
) -> Result<Option<TimerNotice>, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The timer heartbeat transaction could not be started",
            error,
        )
    })?;

    let completion = reconcile_locked(&transaction, now)?;

    if completion.is_none() {
        let row = load_timer(&transaction)?;
        if row.status == TimerStatus::Running {
            let now_text = format_utc(now);
            transaction
                .execute(
                    "UPDATE timer_state
                     SET last_heartbeat_at = ?1, updated_at = ?1
                     WHERE id = 1",
                    [now_text],
                )
                .map_err(|error| {
                    AppError::database("The timer heartbeat could not be stored", error)
                })?;
        }
    }

    let settings = load_settings(&transaction)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The timer heartbeat could not be committed", error))?;

    match completion {
        Some(completion) if settings.notifications_enabled => Ok(Some(TimerNotice {
            mode: completion.mode,
            duration_ms: completion.duration_ms,
        })),
        _ => Ok(None),
    }
}

/// Background heartbeat. It is intentionally independent of the webview, so a
/// minimized window cannot stop timers or suppress completion notifications.
pub fn start_heartbeat(app: tauri::AppHandle) {
    let thread = std::thread::Builder::new()
        .name("study-timer-heartbeat".to_owned())
        .spawn(move || {
            loop {
                std::thread::sleep(HEARTBEAT_INTERVAL);

                let state = app.state::<crate::db::AppState>();
                match state.with_connection(|connection| heartbeat(connection, Utc::now())) {
                    Ok(Some(notice)) => send_notification(&app, &notice),
                    Ok(None) => {}
                    Err(error) => eprintln!("Study timer heartbeat failed: {error:?}"),
                }
            }
        });

    if let Err(error) = thread {
        eprintln!("Study timer heartbeat thread could not be started: {error}");
    }
}

fn send_notification(app: &tauri::AppHandle, notice: &TimerNotice) {
    use tauri_plugin_notification::NotificationExt;

    let (title, body) = notice.mode.notification_copy(notice.duration_ms);
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        eprintln!("Study timer notification could not be shown: {error}");
    }
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

    let stale = (now - last_heartbeat).num_milliseconds() > STALE_AFTER_MS;
    if stale {
        let elapsed_before_gap = active_until(row.accumulated_ms, segment_started, last_heartbeat);

        if elapsed_before_gap >= row.duration_ms {
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
            Some((RecoveryReason::Inactivity, excluded_ms, now)),
        )?;
        return Ok(None);
    }

    let elapsed = active_until(row.accumulated_ms, segment_started, now);
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
        ),
        TimerStatus::Paused => row.accumulated_ms,
        TimerStatus::Idle => return Err(AppError::timer_not_running()),
    }
    .min(row.duration_ms);

    if elapsed_ms <= 0 {
        return Err(AppError::timer_empty());
    }

    let duration_seconds = ((elapsed_ms + 500) / 1_000).clamp(1, sessions::MAX_DURATION_SECONDS);

    let session_id = if row.mode == TimerMode::Focus {
        let first_started = row
            .first_started_at
            .ok_or_else(AppError::timer_unavailable)?;
        let session_date = row.session_date.ok_or_else(AppError::timer_unavailable)?;
        let ended_at = if end_at > first_started {
            end_at
        } else {
            first_started + chrono::Duration::milliseconds(elapsed_ms)
        };

        Some(sessions::insert_timer_session(
            transaction,
            row.topic_id,
            &date_only::format_date_only(session_date),
            &format_utc(first_started),
            &format_utc(ended_at),
            duration_seconds,
            "",
        )?)
    } else {
        None
    };

    let now_text = format_utc(now);
    transaction
        .execute(
            "UPDATE timer_state
             SET status = 'idle',
                 accumulated_ms = 0,
                 first_started_at = NULL,
                 segment_started_at = NULL,
                 last_heartbeat_at = NULL,
                 topic_id = NULL,
                 session_date = NULL,
                 recovery_reason = NULL,
                 recovery_excluded_ms = 0,
                 recovery_at = NULL,
                 last_event = 'completed',
                 last_event_at = ?1,
                 last_event_duration_ms = ?2,
                 last_completed_session_id = ?3,
                 updated_at = ?1
             WHERE id = 1",
            params![now_text, elapsed_ms, session_id],
        )
        .map_err(|error| AppError::database("The completed timer could not be stored", error))?;

    Ok(TimerCompletion {
        mode: row.mode,
        duration_ms: elapsed_ms,
    })
}

fn pause_locked(
    transaction: &Transaction<'_>,
    row: &TimerRow,
    cutoff: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    recovery: Option<(RecoveryReason, i64, DateTime<Utc>)>,
) -> Result<(), AppError> {
    let segment_started = row
        .segment_started_at
        .ok_or_else(AppError::timer_unavailable)?;
    let accumulated_ms = active_until(row.accumulated_ms, segment_started, cutoff)
        .min(row.duration_ms)
        .max(0);
    let (recovery_reason, excluded_ms, recovery_at) = match recovery {
        Some((reason, excluded, at)) => {
            (Some(reason.as_str()), excluded.max(0), Some(format_utc(at)))
        }
        None => (None, 0, None),
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
                 recovery_at = ?4,
                 updated_at = ?5
             WHERE id = 1",
            params![
                accumulated_ms,
                recovery_reason,
                excluded_ms,
                recovery_at,
                format_utc(updated_at),
            ],
        )
        .map_err(|error| AppError::database("The study timer could not be paused", error))?;

    Ok(())
}

fn build_snapshot(connection: &Connection, now: DateTime<Utc>) -> Result<TimerSnapshot, AppError> {
    let row = load_timer(connection)?;
    let settings = load_settings(connection)?;

    // An idle row has no configured duration of its own. Report the duration
    // the next timer would use so a freshly migrated database shows 25:00
    // instead of 00:00, and so changed settings are reflected before starting.
    let duration_ms = match row.status {
        TimerStatus::Idle => row.mode.duration_ms(&settings),
        TimerStatus::Running | TimerStatus::Paused => row.duration_ms,
    };

    let remaining_ms = match row.status {
        TimerStatus::Idle => duration_ms,
        TimerStatus::Paused => (duration_ms - row.accumulated_ms).max(0),
        TimerStatus::Running => {
            let segment_started = row
                .segment_started_at
                .ok_or_else(AppError::timer_unavailable)?;
            let elapsed = active_until(row.accumulated_ms, segment_started, now);
            (duration_ms - elapsed).max(0)
        }
    };

    let topic = match row.topic_id {
        Some(topic_id) => Some(load_topic(connection, topic_id)?),
        None => None,
    };

    let recovery = row.recovery_reason.map(|reason| TimerRecovery {
        reason,
        excluded_ms: row.recovery_excluded_ms,
        recovered_at: format_utc(row.recovery_at.unwrap_or(now)),
    });

    Ok(TimerSnapshot {
        state: TimerState {
            status: row.status,
            mode: row.mode,
            duration_ms,
            accumulated_ms: row.accumulated_ms,
            remaining_ms,
            first_started_at: row.first_started_at.map(format_utc),
            session_date: row.session_date.map(date_only::format_date_only),
            topic,
            last_event: row.last_event,
            last_event_at: row.last_event_at.map(format_utc),
            last_event_duration_ms: row.last_event_duration_ms,
            last_completed_session_id: row.last_completed_session_id,
            recovery,
        },
        settings,
        server_now_ms: now.timestamp_millis(),
    })
}

fn load_timer(connection: &Connection) -> Result<TimerRow, AppError> {
    connection
        .query_row(SELECT_TIMER, [], map_timer_row)
        .optional()
        .map_err(|error| AppError::database("The timer state could not be loaded", error))?
        .ok_or_else(AppError::timer_unavailable)
}

fn map_timer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimerRow> {
    let status_text: String = row.get(0)?;
    let mode_text: String = row.get(1)?;
    let recovery_text: Option<String> = row.get(9)?;
    let last_event_text: Option<String> = row.get(12)?;

    // Invalid enum strings are converted to a command-safe error after row
    // mapping; SQLite CHECK constraints make this a corruption edge case.
    let status = TimerStatus::parse(&status_text).map_err(|_| {
        rusqlite::Error::InvalidColumnType(0, "status".to_owned(), rusqlite::types::Type::Text)
    })?;
    let mode = TimerMode::parse(&mode_text).map_err(|_| {
        rusqlite::Error::InvalidColumnType(1, "mode".to_owned(), rusqlite::types::Type::Text)
    })?;
    let recovery_reason = recovery_text
        .as_deref()
        .map(RecoveryReason::parse)
        .transpose()
        .map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                9,
                "recovery_reason".to_owned(),
                rusqlite::types::Type::Text,
            )
        })?;
    let last_event = last_event_text
        .as_deref()
        .map(LastTimerEvent::parse)
        .transpose()
        .map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                12,
                "last_event".to_owned(),
                rusqlite::types::Type::Text,
            )
        })?;

    Ok(TimerRow {
        status,
        mode,
        duration_ms: row.get(2)?,
        accumulated_ms: row.get(3)?,
        first_started_at: parse_optional_utc(row.get(4)?)?,
        segment_started_at: parse_optional_utc(row.get(5)?)?,
        last_heartbeat_at: parse_optional_utc(row.get(6)?)?,
        topic_id: row.get(7)?,
        session_date: parse_optional_date(row.get(8)?)?,
        recovery_reason,
        recovery_excluded_ms: row.get(10)?,
        recovery_at: parse_optional_utc(row.get(11)?)?,
        last_event,
        last_event_at: parse_optional_utc(row.get(13)?)?,
        last_event_duration_ms: row.get(14)?,
        last_completed_session_id: row.get(15)?,
    })
}

fn load_topic(connection: &Connection, topic_id: i64) -> Result<TimerTopic, AppError> {
    connection
        .query_row(
            "SELECT id, code, title, is_archived FROM syllabus_topics WHERE id = ?1",
            [topic_id],
            |row| {
                Ok(TimerTopic {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    title: row.get(2)?,
                    archived: row.get::<_, i64>(3)? != 0,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("The timer topic could not be loaded", error))?
        .ok_or_else(AppError::topic_not_found)
}

fn ensure_active_topic(connection: &Connection, topic_id: i64) -> Result<(), AppError> {
    if topic_id <= 0 {
        return Err(AppError::validation(
            "topicId",
            "Choose a topic from the active syllabus.",
        ));
    }

    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM syllabus_topics WHERE id = ?1 AND is_archived = 0
             )",
            [topic_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The syllabus topic could not be checked", error))?;

    if exists == 0 {
        return Err(AppError::topic_not_found());
    }

    Ok(())
}

fn validate_settings(input: &TimerSettingsInput) -> Result<(), AppError> {
    if !(MIN_FOCUS_MINUTES..=MAX_FOCUS_MINUTES).contains(&input.focus_minutes) {
        return Err(AppError::validation(
            "focusMinutes",
            format!(
                "Focus duration must be between {MIN_FOCUS_MINUTES} and {MAX_FOCUS_MINUTES} minutes."
            ),
        ));
    }
    if !(MIN_SHORT_BREAK_MINUTES..=MAX_SHORT_BREAK_MINUTES).contains(&input.short_break_minutes) {
        return Err(AppError::validation(
            "shortBreakMinutes",
            format!(
                "Short break must be between {MIN_SHORT_BREAK_MINUTES} and {MAX_SHORT_BREAK_MINUTES} minutes."
            ),
        ));
    }
    if !(MIN_LONG_BREAK_MINUTES..=MAX_LONG_BREAK_MINUTES).contains(&input.long_break_minutes) {
        return Err(AppError::validation(
            "longBreakMinutes",
            format!(
                "Long break must be between {MIN_LONG_BREAK_MINUTES} and {MAX_LONG_BREAK_MINUTES} minutes."
            ),
        ));
    }

    Ok(())
}

fn active_until(accumulated_ms: i64, segment_started: DateTime<Utc>, cutoff: DateTime<Utc>) -> i64 {
    accumulated_ms + (cutoff - segment_started).num_milliseconds().max(0)
}

fn parse_optional_utc(value: Option<String>) -> rusqlite::Result<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|parsed| parsed.with_timezone(&Utc))
                .map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "timestamp".to_owned(),
                        rusqlite::types::Type::Text,
                    )
                })
        })
        .transpose()
}

fn parse_optional_date(value: Option<String>) -> rusqlite::Result<Option<NaiveDate>> {
    value
        .map(|value| {
            date_only::parse_date_only(&value).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "session_date".to_owned(),
                    rusqlite::types::Type::Text,
                )
            })
        })
        .transpose()
}

fn format_utc(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn format_minutes(duration_ms: i64) -> String {
    let minutes = (duration_ms as f64 / 60_000.0).round() as i64;
    if minutes == 1 {
        "1 minute".to_owned()
    } else {
        format!("{minutes} minutes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, syllabus};
    use chrono::TimeZone;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn at(hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 30, hour, minute, second)
            .single()
            .expect("test timestamp should be valid")
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(date(2026, 10, 30));
        syllabus::seed(&mut connection).expect("syllabus should be seeded");
        connection
    }

    fn topic_id(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = 'GA-01'",
                [],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    fn start_focus(
        connection: &mut Connection,
        now: DateTime<Utc>,
        topic_id: Option<i64>,
    ) -> TimerSnapshot {
        start(connection, now, date(2026, 10, 30), "focus", topic_id)
            .expect("focus timer should start")
    }

    fn advance_with_heartbeats(
        connection: &mut Connection,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) {
        let mut cursor = from;
        while cursor < to {
            cursor += chrono::Duration::seconds(5);
            let cursor = cursor.min(to);
            heartbeat(connection, cursor).expect("heartbeat should persist");
        }
    }

    #[test]
    fn defaults_are_documented_and_settings_validate() {
        let mut connection = connection();
        let settings = load_settings(&connection).unwrap();
        assert_eq!(
            settings,
            TimerSettings {
                focus_minutes: 25,
                short_break_minutes: 5,
                long_break_minutes: 15,
                notifications_enabled: false,
            }
        );

        let invalid = [
            ("focusMinutes", 0_i64, 5, 15),
            ("focusMinutes", 241, 5, 15),
            ("shortBreakMinutes", 25, 0, 15),
            ("shortBreakMinutes", 25, 61, 15),
            ("longBreakMinutes", 25, 5, 0),
            ("longBreakMinutes", 25, 5, 121),
        ];

        for (field, focus, short, long) in invalid {
            let error = update_settings(
                &mut connection,
                TimerSettingsInput {
                    focus_minutes: focus,
                    short_break_minutes: short,
                    long_break_minutes: long,
                    notifications_enabled: false,
                },
            )
            .expect_err("invalid settings should fail");
            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some(field));
        }

        let updated = update_settings(
            &mut connection,
            TimerSettingsInput {
                focus_minutes: 50,
                short_break_minutes: 10,
                long_break_minutes: 20,
                notifications_enabled: true,
            },
        )
        .unwrap();
        assert_eq!(updated.focus_minutes, 50);
        assert!(updated.notifications_enabled);
    }

    #[test]
    fn idle_snapshot_reports_the_next_configured_duration() {
        let mut connection = connection();
        let fresh = snapshot(&mut connection, at(10, 0, 0)).unwrap();
        assert_eq!(fresh.state.status, TimerStatus::Idle);
        assert_eq!(fresh.state.duration_ms, 25 * 60_000);
        assert_eq!(fresh.state.remaining_ms, 25 * 60_000);

        update_settings(
            &mut connection,
            TimerSettingsInput {
                focus_minutes: 40,
                short_break_minutes: 5,
                long_break_minutes: 15,
                notifications_enabled: false,
            },
        )
        .unwrap();

        let updated = snapshot(&mut connection, at(10, 0, 0)).unwrap();
        assert_eq!(updated.state.duration_ms, 40 * 60_000);
        assert_eq!(updated.state.remaining_ms, 40 * 60_000);
    }

    #[test]
    fn start_pause_resume_and_finish_save_only_active_focus_time() {
        let mut connection = connection();
        let topic = topic_id(&connection);
        let t0 = at(10, 0, 0);

        let started = start_focus(&mut connection, t0, Some(topic));
        assert_eq!(started.state.status, TimerStatus::Running);
        assert_eq!(started.state.remaining_ms, 25 * 60_000);
        assert_eq!(started.state.topic.as_ref().unwrap().code, "GA-01");

        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(10));
        let paused = pause(&mut connection, t0 + chrono::Duration::minutes(10)).unwrap();
        assert_eq!(paused.state.status, TimerStatus::Paused);
        assert_eq!(paused.state.accumulated_ms, 600_000);
        assert_eq!(paused.state.remaining_ms, 900_000);

        let resumed = resume(&mut connection, t0 + chrono::Duration::minutes(20)).unwrap();
        assert_eq!(resumed.state.status, TimerStatus::Running);
        assert_eq!(resumed.state.accumulated_ms, 600_000);

        advance_with_heartbeats(
            &mut connection,
            t0 + chrono::Duration::minutes(20),
            t0 + chrono::Duration::minutes(25),
        );
        let finished = finish(&mut connection, t0 + chrono::Duration::minutes(25)).unwrap();
        assert_eq!(finished.state.status, TimerStatus::Idle);
        assert_eq!(finished.state.last_event, Some(LastTimerEvent::Completed));
        assert_eq!(finished.state.last_event_duration_ms, 900_000);

        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, 900);
        assert_eq!(sessions[0].source, "timer");
        assert_eq!(sessions[0].session_date, "2026-10-30");
        assert_eq!(
            sessions[0].started_at.as_deref(),
            Some("2026-10-30T10:00:00.000Z")
        );
        assert_eq!(
            sessions[0].ended_at.as_deref(),
            Some("2026-10-30T10:25:00.000Z")
        );
    }

    #[test]
    fn elapsed_time_is_timestamp_based_between_snapshots() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);

        let snapshot = snapshot(&mut connection, t0 + chrono::Duration::seconds(10)).unwrap();
        assert_eq!(snapshot.state.remaining_ms, 25 * 60_000 - 10_000);
        assert_eq!(snapshot.state.accumulated_ms, 0);
        assert_eq!(
            snapshot.server_now_ms,
            (t0 + chrono::Duration::seconds(10)).timestamp_millis()
        );
    }

    #[test]
    fn minimized_accuracy_comes_from_heartbeats_and_wall_clock() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);

        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(5));
        let snapshot = snapshot(&mut connection, t0 + chrono::Duration::minutes(5)).unwrap();

        assert_eq!(snapshot.state.status, TimerStatus::Running);
        assert_eq!(snapshot.state.remaining_ms, 20 * 60_000);
    }

    #[test]
    fn restart_recovery_pauses_at_the_last_heartbeat_and_excludes_closed_time() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(10));

        recover_after_restart(&mut connection, t0 + chrono::Duration::minutes(30))
            .expect("restart recovery should succeed");

        let snapshot = snapshot(&mut connection, t0 + chrono::Duration::minutes(30)).unwrap();
        assert_eq!(snapshot.state.status, TimerStatus::Paused);
        assert_eq!(snapshot.state.accumulated_ms, 600_000);
        assert_eq!(snapshot.state.remaining_ms, 900_000);
        let recovery = snapshot.state.recovery.as_ref().unwrap();
        assert_eq!(recovery.reason, RecoveryReason::Restart);
        assert_eq!(recovery.excluded_ms, 1_200_000);
        assert!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .is_empty()
        );

        let resumed = resume(&mut connection, t0 + chrono::Duration::minutes(31)).unwrap();
        assert_eq!(resumed.state.status, TimerStatus::Running);
        assert!(resumed.state.recovery.is_none());
        advance_with_heartbeats(
            &mut connection,
            t0 + chrono::Duration::minutes(31),
            t0 + chrono::Duration::minutes(32),
        );
        finish(&mut connection, t0 + chrono::Duration::minutes(32)).unwrap();
        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions[0].duration_seconds, 660);
    }

    #[test]
    fn sleep_or_inactivity_gap_is_excluded_and_pauses_the_timer() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(10));

        let notice = heartbeat(&mut connection, t0 + chrono::Duration::minutes(40)).unwrap();
        assert!(notice.is_none());

        let snapshot = snapshot(&mut connection, t0 + chrono::Duration::minutes(41)).unwrap();
        assert_eq!(snapshot.state.status, TimerStatus::Paused);
        assert_eq!(snapshot.state.accumulated_ms, 600_000);
        let recovery = snapshot.state.recovery.as_ref().unwrap();
        assert_eq!(recovery.reason, RecoveryReason::Inactivity);
        assert_eq!(recovery.excluded_ms, 1_800_000);

        resume(&mut connection, t0 + chrono::Duration::minutes(41)).unwrap();
        advance_with_heartbeats(
            &mut connection,
            t0 + chrono::Duration::minutes(41),
            t0 + chrono::Duration::minutes(42),
        );
        finish(&mut connection, t0 + chrono::Duration::minutes(42)).unwrap();
        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions[0].duration_seconds, 660);
    }

    #[test]
    fn duplicate_active_timers_are_rejected() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);

        let error = start(&mut connection, t0, date(2026, 10, 30), "short_break", None)
            .expect_err("second timer should fail");
        assert_eq!(error.code, "timer_active");

        pause(&mut connection, t0 + chrono::Duration::minutes(1)).unwrap();
        let error = start(&mut connection, t0, date(2026, 10, 30), "focus", None)
            .expect_err("paused timer also blocks a new timer");
        assert_eq!(error.code, "timer_active");
    }

    #[test]
    fn focus_and_break_accounting_never_saves_break_time() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);

        start(&mut connection, t0, date(2026, 10, 30), "short_break", None).unwrap();
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(2));
        finish(&mut connection, t0 + chrono::Duration::minutes(2)).unwrap();
        assert_eq!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .len(),
            0
        );

        let long_break_start = t0 + chrono::Duration::minutes(3);
        start(
            &mut connection,
            long_break_start,
            date(2026, 10, 30),
            "long_break",
            None,
        )
        .unwrap();
        advance_with_heartbeats(
            &mut connection,
            long_break_start,
            long_break_start + chrono::Duration::minutes(2),
        );
        finish(
            &mut connection,
            long_break_start + chrono::Duration::minutes(2),
        )
        .unwrap();
        assert!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .is_empty()
        );

        let focus_start = long_break_start + chrono::Duration::minutes(3);
        start_focus(&mut connection, focus_start, None);
        advance_with_heartbeats(
            &mut connection,
            focus_start,
            focus_start + chrono::Duration::minutes(10),
        );
        finish(&mut connection, focus_start + chrono::Duration::minutes(10)).unwrap();
        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, 600);
    }

    #[test]
    fn discard_never_creates_a_completed_session() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);

        let discarded = discard(&mut connection, t0 + chrono::Duration::minutes(20)).unwrap();
        assert_eq!(discarded.state.status, TimerStatus::Idle);
        assert_eq!(discarded.state.last_event, Some(LastTimerEvent::Discarded));
        assert_eq!(discarded.state.last_event_duration_ms, 1_200_000);
        assert!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .is_empty()
        );

        let totals = sessions::totals(
            &connection,
            "2026-10-30",
            "2026-10-30",
            date(2026, 10, 30),
            None,
        )
        .unwrap();
        assert_eq!(totals.range_seconds, 0);
    }

    #[test]
    fn natural_completion_saves_a_focus_session_once_and_notifies_when_enabled() {
        let mut connection = connection();
        update_settings(
            &mut connection,
            TimerSettingsInput {
                focus_minutes: 1,
                short_break_minutes: 1,
                long_break_minutes: 1,
                notifications_enabled: true,
            },
        )
        .unwrap();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::seconds(55));

        let notice = heartbeat(&mut connection, t0 + chrono::Duration::minutes(1)).unwrap();
        assert!(notice.is_some());
        assert_eq!(notice.as_ref().unwrap().mode, TimerMode::Focus);

        let snapshot = snapshot(&mut connection, t0 + chrono::Duration::minutes(2)).unwrap();
        assert_eq!(snapshot.state.status, TimerStatus::Idle);
        assert_eq!(snapshot.state.last_event, Some(LastTimerEvent::Completed));

        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, 60);

        let second_notice = heartbeat(&mut connection, t0 + chrono::Duration::minutes(3)).unwrap();
        assert!(second_notice.is_none());
        assert_eq!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn notifications_are_disabled_by_default() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::seconds(1_495));

        let notice = heartbeat(&mut connection, t0 + chrono::Duration::minutes(25)).unwrap();
        assert!(notice.is_none());
        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn session_date_uses_the_local_start_date_across_midnight() {
        let mut connection = connection();
        let start_time = Utc.with_ymd_and_hms(2026, 10, 30, 23, 55, 0).unwrap();
        let _ = start(
            &mut connection,
            start_time,
            date(2026, 10, 30),
            "focus",
            None,
        )
        .expect("timer should start");
        advance_with_heartbeats(
            &mut connection,
            start_time,
            start_time + chrono::Duration::seconds(1_495),
        );
        finish(&mut connection, start_time + chrono::Duration::minutes(25)).unwrap();

        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, 1_500);
        assert_eq!(sessions[0].session_date, "2026-10-30");
        assert_eq!(
            sessions[0].ended_at.as_deref(),
            Some("2026-10-31T00:20:00.000Z")
        );
        assert_eq!(
            sessions[0].started_at.as_deref(),
            Some("2026-10-30T23:55:00.000Z")
        );
    }

    #[test]
    fn finish_without_elapsed_time_requires_discard() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);

        let error = finish(&mut connection, t0).expect_err("empty finish should fail");
        assert_eq!(error.code, "timer_empty");
        assert!(
            sessions::list(&connection, sessions::StudySessionFilter::default())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn invalid_modes_and_topics_are_rejected() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);

        let error = start(&mut connection, t0, date(2026, 10, 30), "deep_work", None)
            .expect_err("invalid mode should fail");
        assert_eq!(error.field, Some("mode"));

        let error = start(
            &mut connection,
            t0,
            date(2026, 10, 30),
            "focus",
            Some(999_999),
        )
        .expect_err("invalid topic should fail");
        assert_eq!(error.code, "topic_not_found");
    }

    #[test]
    fn finish_while_paused_uses_accumulated_time() {
        let mut connection = connection();
        let t0 = at(10, 0, 0);
        let _ = start_focus(&mut connection, t0, None);
        advance_with_heartbeats(&mut connection, t0, t0 + chrono::Duration::minutes(8));
        pause(&mut connection, t0 + chrono::Duration::minutes(8)).unwrap();

        finish(&mut connection, t0 + chrono::Duration::minutes(30)).unwrap();
        let sessions =
            sessions::list(&connection, sessions::StudySessionFilter::default()).unwrap();
        assert_eq!(sessions[0].duration_seconds, 480);
    }
}
