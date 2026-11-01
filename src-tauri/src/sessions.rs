use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError};

pub const MIN_DURATION_SECONDS: i64 = 1;
pub const MAX_DURATION_SECONDS: i64 = 86_400;
pub const MAX_NOTES_CHARS: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTopic {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySession {
    pub id: i64,
    pub topic: Option<SessionTopic>,
    pub session_date: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_seconds: i64,
    pub source: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StudySessionFilter {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub topic_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudySessionInput {
    #[serde(default)]
    pub topic_id: Option<i64>,
    pub session_date: String,
    pub duration_seconds: i64,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStudyTotal {
    pub date: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyStudyTotal {
    pub week_start: String,
    pub week_end: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTotals {
    pub range_from: String,
    pub range_to: String,
    pub range_seconds: i64,
    pub range_session_count: i64,
    pub today_date: String,
    pub today_seconds: i64,
    pub week_start: String,
    pub week_end: String,
    pub week_seconds: i64,
    pub daily: Vec<DailyStudyTotal>,
    pub weekly: Vec<WeeklyStudyTotal>,
}

struct ValidatedSessionInput {
    topic_id: Option<i64>,
    session_date: String,
    duration_seconds: i64,
    notes: String,
}

pub fn list(
    connection: &Connection,
    filter: StudySessionFilter,
) -> Result<Vec<StudySession>, AppError> {
    let from_date = validate_optional_date(filter.from_date.as_deref(), "fromDate")?;
    let to_date = validate_optional_date(filter.to_date.as_deref(), "toDate")?;

    if let (Some(from), Some(to)) = (&from_date, &to_date)
        && from > to
    {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    if let Some(topic_id) = filter.topic_id {
        validate_topic_id(topic_id)?;
        ensure_topic_exists(connection, topic_id)?;
    }

    let mut statement = connection
        .prepare(
            "SELECT
                s.id,
                s.topic_id,
                t.code,
                t.title,
                t.is_archived,
                s.session_date,
                s.started_at,
                s.ended_at,
                s.duration_seconds,
                s.source,
                s.notes,
                s.created_at,
                s.updated_at
             FROM study_sessions s
             LEFT JOIN syllabus_topics t ON t.id = s.topic_id
             WHERE (?1 IS NULL OR s.session_date >= ?1)
               AND (?2 IS NULL OR s.session_date <= ?2)
               AND (?3 IS NULL OR s.topic_id = ?3)
             ORDER BY s.session_date DESC, COALESCE(s.started_at, s.created_at) DESC, s.id DESC",
        )
        .map_err(|error| AppError::database("Study sessions could not be prepared", error))?;

    let rows = statement
        .query_map(
            params![from_date, to_date, filter.topic_id],
            map_session_row,
        )
        .map_err(|error| AppError::database("Study sessions could not be queried", error))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Study sessions could not be read", error))
}

pub fn create_manual(
    connection: &mut Connection,
    input: StudySessionInput,
) -> Result<StudySession, AppError> {
    let validated = validate_input(input)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The study-session transaction could not be started", error)
    })?;

    if let Some(topic_id) = validated.topic_id {
        ensure_active_topic(&transaction, topic_id)?;
    }

    transaction
        .execute(
            "INSERT INTO study_sessions (
                topic_id, session_date, started_at, ended_at, duration_seconds,
                source, notes, created_at, updated_at
             ) VALUES (
                ?1, ?2, NULL, NULL, ?3,
                'manual', ?4,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.topic_id,
                validated.session_date,
                validated.duration_seconds,
                validated.notes,
            ],
        )
        .map_err(|error| AppError::database("The study session could not be saved", error))?;

    let session_id = transaction.last_insert_rowid();
    let session = load_by_id(&transaction, session_id)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The study session could not be committed", error))?;

    Ok(session)
}

pub fn update(
    connection: &mut Connection,
    session_id: i64,
    input: StudySessionInput,
) -> Result<StudySession, AppError> {
    validate_session_id(session_id)?;
    let validated = validate_input(input)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The study-session transaction could not be started", error)
    })?;

    let existing = transaction
        .query_row(
            "SELECT topic_id, source, session_date, duration_seconds
             FROM study_sessions WHERE id = ?1",
            [session_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("The study session could not be checked", error))?
        .ok_or_else(AppError::session_not_found)?;

    let (existing_topic_id, source, existing_date, existing_duration) = existing;

    if source == "timer" {
        if validated.session_date != existing_date {
            return Err(AppError::validation(
                "sessionDate",
                "Timer-session dates are fixed by their recorded start timestamp.",
            ));
        }
        if validated.duration_seconds != existing_duration {
            return Err(AppError::validation(
                "durationSeconds",
                "Timer-session durations are fixed by their recorded timer. Edit only its topic or notes.",
            ));
        }
    }

    if let Some(topic_id) = validated.topic_id
        && existing_topic_id != Some(topic_id)
    {
        ensure_active_topic(&transaction, topic_id)?;
    }

    let changed = transaction
        .execute(
            "UPDATE study_sessions
             SET topic_id = ?1,
                 session_date = ?2,
                 duration_seconds = ?3,
                 notes = ?4,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?5",
            params![
                validated.topic_id,
                validated.session_date,
                validated.duration_seconds,
                validated.notes,
                session_id,
            ],
        )
        .map_err(|error| AppError::database("The study session could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::session_not_found());
    }

    let session = load_by_id(&transaction, session_id)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The study session could not be committed", error))?;

    Ok(session)
}

pub fn delete(connection: &mut Connection, session_id: i64) -> Result<(), AppError> {
    validate_session_id(session_id)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The study-session transaction could not be started", error)
    })?;

    let changed = transaction
        .execute("DELETE FROM study_sessions WHERE id = ?1", [session_id])
        .map_err(|error| AppError::database("The study session could not be deleted", error))?;

    if changed != 1 {
        return Err(AppError::session_not_found());
    }

    transaction.commit().map_err(|error| {
        AppError::database("The study-session deletion could not be committed", error)
    })
}

pub fn totals(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    today: NaiveDate,
    topic_id: Option<i64>,
) -> Result<StudyTotals, AppError> {
    let from = validate_required_date(from_date, "fromDate")?;
    let to = validate_required_date(to_date, "toDate")?;

    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    if let Some(topic_id) = topic_id {
        validate_topic_id(topic_id)?;
        ensure_topic_exists(connection, topic_id)?;
    }

    let range_from = date_only::format_date_only(from);
    let range_to = date_only::format_date_only(to);

    let mut statement = connection
        .prepare(
            "SELECT session_date, SUM(duration_seconds), COUNT(*)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
               AND (?3 IS NULL OR topic_id = ?3)
             GROUP BY session_date
             ORDER BY session_date",
        )
        .map_err(|error| AppError::database("Study session totals could not be prepared", error))?;

    let rows = statement
        .query_map(params![range_from, range_to, topic_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Study session totals could not be queried", error))?;

    let mut daily = Vec::new();
    let mut weekly: BTreeMap<String, (String, i64, i64)> = BTreeMap::new();
    let mut range_seconds = 0_i64;
    let mut range_session_count = 0_i64;

    for row in rows {
        let (date, seconds, session_count) =
            row.map_err(|error| AppError::database("A study total could not be read", error))?;
        let parsed = validate_required_date(&date, "sessionDate")?;
        let week_start = monday_of(parsed);
        let week_start_text = date_only::format_date_only(week_start);
        let week_end_text = date_only::format_date_only(week_start + Days::new(6));

        range_seconds += seconds;
        range_session_count += session_count;
        daily.push(DailyStudyTotal {
            date,
            seconds,
            session_count,
        });

        weekly
            .entry(week_start_text.clone())
            .and_modify(|entry| {
                entry.1 += seconds;
                entry.2 += session_count;
            })
            .or_insert((week_end_text, seconds, session_count));
    }

    let today_text = date_only::format_date_only(today);
    let today_seconds = sum_for_range(connection, &today_text, &today_text, topic_id)?;

    let week_start = monday_of(today);
    let week_start_text = date_only::format_date_only(week_start);
    let week_end_text = date_only::format_date_only(week_start + Days::new(6));
    let week_seconds = sum_for_range(connection, &week_start_text, &week_end_text, topic_id)?;

    Ok(StudyTotals {
        range_from,
        range_to,
        range_seconds,
        range_session_count,
        today_date: today_text,
        today_seconds,
        week_start: week_start_text,
        week_end: week_end_text,
        week_seconds,
        daily,
        weekly: weekly
            .into_iter()
            .map(
                |(week_start, (week_end, seconds, session_count))| WeeklyStudyTotal {
                    week_start,
                    week_end,
                    seconds,
                    session_count,
                },
            )
            .collect(),
    })
}

pub(crate) fn insert_timer_session(
    transaction: &Transaction<'_>,
    topic_id: Option<i64>,
    session_date: &str,
    started_at: &str,
    ended_at: &str,
    duration_seconds: i64,
    notes: &str,
) -> Result<i64, AppError> {
    let session_date = validate_required_date(session_date, "sessionDate")?;
    if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&duration_seconds) {
        return Err(AppError::internal(
            "The completed timer produced an invalid study duration.",
        ));
    }
    if notes.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::internal(
            "The completed timer produced overlong session notes.",
        ));
    }
    if let Some(topic_id) = topic_id {
        validate_topic_id(topic_id)?;
    }

    transaction
        .execute(
            "INSERT INTO study_sessions (
                topic_id, session_date, started_at, ended_at, duration_seconds,
                source, notes, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                'timer', ?6,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                topic_id,
                date_only::format_date_only(session_date),
                started_at,
                ended_at,
                duration_seconds,
                notes,
            ],
        )
        .map_err(|error| {
            AppError::database("The completed study session could not be saved", error)
        })?;

    Ok(transaction.last_insert_rowid())
}

fn load_by_id(connection: &Connection, session_id: i64) -> Result<StudySession, AppError> {
    connection
        .query_row(
            "SELECT
                s.id,
                s.topic_id,
                t.code,
                t.title,
                t.is_archived,
                s.session_date,
                s.started_at,
                s.ended_at,
                s.duration_seconds,
                s.source,
                s.notes,
                s.created_at,
                s.updated_at
             FROM study_sessions s
             LEFT JOIN syllabus_topics t ON t.id = s.topic_id
             WHERE s.id = ?1",
            [session_id],
            map_session_row,
        )
        .optional()
        .map_err(|error| AppError::database("The study session could not be loaded", error))?
        .ok_or_else(AppError::session_not_found)
}

fn map_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StudySession> {
    let topic_id: Option<i64> = row.get(1)?;
    let topic = match topic_id {
        Some(id) => Some(SessionTopic {
            id,
            code: row.get(2)?,
            title: row.get(3)?,
            archived: row.get::<_, i64>(4)? != 0,
        }),
        None => None,
    };

    Ok(StudySession {
        id: row.get(0)?,
        topic,
        session_date: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        duration_seconds: row.get(8)?,
        source: row.get(9)?,
        notes: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn validate_input(input: StudySessionInput) -> Result<ValidatedSessionInput, AppError> {
    let session_date = validate_required_date(&input.session_date, "sessionDate")?;

    if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&input.duration_seconds) {
        return Err(AppError::validation(
            "durationSeconds",
            "Duration must be between 1 second and 24 hours.",
        ));
    }

    if input.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    if let Some(topic_id) = input.topic_id {
        validate_topic_id(topic_id)?;
    }

    Ok(ValidatedSessionInput {
        topic_id: input.topic_id,
        session_date: date_only::format_date_only(session_date),
        duration_seconds: input.duration_seconds,
        notes: input.notes,
    })
}

fn validate_required_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    date_only::parse_date_only(value).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

fn validate_optional_date(
    value: Option<&str>,
    field: &'static str,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| validate_required_date(value, field).map(date_only::format_date_only))
        .transpose()
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

fn validate_topic_id(topic_id: i64) -> Result<(), AppError> {
    if topic_id <= 0 {
        return Err(AppError::validation(
            "topicId",
            "Choose a topic from the syllabus.",
        ));
    }

    Ok(())
}

fn ensure_active_topic(connection: &Connection, topic_id: i64) -> Result<(), AppError> {
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

fn ensure_topic_exists(connection: &Connection, topic_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM syllabus_topics WHERE id = ?1)",
            [topic_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The syllabus topic could not be checked", error))?;

    if exists == 0 {
        return Err(AppError::topic_not_found());
    }

    Ok(())
}

pub fn list_topics(connection: &Connection) -> Result<Vec<SessionTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, code, title, is_archived
             FROM syllabus_topics t
             WHERE t.is_archived = 0
                OR EXISTS (
                    SELECT 1 FROM study_sessions s WHERE s.topic_id = t.id
                )
             ORDER BY t.is_archived, t.code",
        )
        .map_err(|error| AppError::database("Session topics could not be prepared", error))?;

    let rows = statement
        .query_map([], |row| {
            Ok(SessionTopic {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|error| AppError::database("Session topics could not be queried", error))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Session topics could not be read", error))
}

fn sum_for_range(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    topic_id: Option<i64>,
) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
               AND (?3 IS NULL OR topic_id = ?3)",
            params![from_date, to_date, topic_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Study-time totals could not be calculated", error))
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Days::new(u64::from(date.weekday().num_days_from_monday()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(date(2026, 10, 30));
        syllabus::seed(&mut connection).expect("syllabus should be seeded");
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

    fn input(session_date: &str, duration_seconds: i64, notes: &str) -> StudySessionInput {
        StudySessionInput {
            topic_id: None,
            session_date: session_date.to_owned(),
            duration_seconds,
            notes: notes.to_owned(),
        }
    }

    #[test]
    fn manual_session_crud_recalculates_totals() {
        let mut connection = connection();
        let topic = topic_id(&connection, "GA-01");
        let created = create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 1_800,
                notes: "Probability practice".to_owned(),
            },
        )
        .expect("manual session should be created");

        assert_eq!(created.source, "manual");
        assert_eq!(created.duration_seconds, 1_800);
        assert_eq!(
            created.topic.as_ref().map(|topic| topic.code.as_str()),
            Some("GA-01")
        );
        assert!(created.started_at.is_none());
        assert!(created.ended_at.is_none());

        let first_day = totals(
            &connection,
            "2026-11-02",
            "2026-11-02",
            date(2026, 11, 2),
            None,
        )
        .expect("totals should load");
        assert_eq!(first_day.range_seconds, 1_800);
        assert_eq!(first_day.today_seconds, 1_800);

        let updated = update(
            &mut connection,
            created.id,
            StudySessionInput {
                topic_id: None,
                session_date: "2026-11-03".to_owned(),
                duration_seconds: 3_600,
                notes: "Rescheduled".to_owned(),
            },
        )
        .expect("manual session should update");
        assert_eq!(updated.session_date, "2026-11-03");
        assert_eq!(updated.duration_seconds, 3_600);
        assert!(updated.topic.is_none());

        let old_day = totals(
            &connection,
            "2026-11-02",
            "2026-11-02",
            date(2026, 11, 2),
            None,
        )
        .expect("old totals should load");
        let new_day = totals(
            &connection,
            "2026-11-03",
            "2026-11-03",
            date(2026, 11, 3),
            None,
        )
        .expect("new totals should load");
        assert_eq!(old_day.range_seconds, 0);
        assert_eq!(new_day.range_seconds, 3_600);

        delete(&mut connection, created.id).expect("session should delete");
        let after_delete = totals(
            &connection,
            "2026-11-03",
            "2026-11-03",
            date(2026, 11, 3),
            None,
        )
        .expect("post-delete totals should load");
        assert_eq!(after_delete.range_seconds, 0);
        assert!(after_delete.daily.is_empty());
    }

    #[test]
    fn session_filters_apply_date_and_topic() {
        let mut connection = connection();
        let ga = topic_id(&connection, "GA-01");
        let mmp = topic_id(&connection, "MMP-C01");

        create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(ga),
                session_date: "2026-11-01".to_owned(),
                duration_seconds: 600,
                notes: String::new(),
            },
        )
        .unwrap();
        create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(mmp),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 1_200,
                notes: String::new(),
            },
        )
        .unwrap();

        let by_topic = list(
            &connection,
            StudySessionFilter {
                from_date: None,
                to_date: None,
                topic_id: Some(mmp),
            },
        )
        .unwrap();
        assert_eq!(by_topic.len(), 1);
        assert_eq!(by_topic[0].duration_seconds, 1_200);

        let by_date = list(
            &connection,
            StudySessionFilter {
                from_date: Some("2026-11-02".to_owned()),
                to_date: Some("2026-11-02".to_owned()),
                topic_id: None,
            },
        )
        .unwrap();
        assert_eq!(by_date.len(), 1);
        assert_eq!(by_date[0].topic.as_ref().unwrap().code, "MMP-C01");
    }

    #[test]
    fn timer_sessions_keep_their_recorded_date_and_duration() {
        let mut connection = connection();
        let topic = topic_id(&connection, "GA-01");
        let transaction = connection.transaction().expect("transaction should start");
        let session_id = insert_timer_session(
            &transaction,
            Some(topic),
            "2026-11-01",
            "2026-11-01T23:50:00.000Z",
            "2026-11-02T00:15:00.000Z",
            1_500,
            "Cross-midnight focus",
        )
        .expect("timer session should save");
        transaction.commit().expect("transaction should commit");

        let original = load_by_id(&connection, session_id).expect("timer session should load");
        assert_eq!(original.session_date, "2026-11-01");
        assert_eq!(
            original.started_at.as_deref(),
            Some("2026-11-01T23:50:00.000Z")
        );
        assert_eq!(
            original.ended_at.as_deref(),
            Some("2026-11-02T00:15:00.000Z")
        );

        let edited = update(
            &mut connection,
            session_id,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: original.session_date.clone(),
                duration_seconds: original.duration_seconds,
                notes: "Edited context".to_owned(),
            },
        )
        .expect("notes should remain editable");
        assert_eq!(edited.notes, "Edited context");
        assert_eq!(edited.started_at, original.started_at);
        assert_eq!(edited.ended_at, original.ended_at);

        let date_error = update(
            &mut connection,
            session_id,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: original.duration_seconds,
                notes: "Do not save".to_owned(),
            },
        )
        .expect_err("timer session date must stay fixed");
        assert_eq!(date_error.field, Some("sessionDate"));

        let duration_error = update(
            &mut connection,
            session_id,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: original.session_date.clone(),
                duration_seconds: original.duration_seconds + 1,
                notes: "Do not save".to_owned(),
            },
        )
        .expect_err("timer session duration must stay fixed");
        assert_eq!(duration_error.field, Some("durationSeconds"));

        let unchanged = load_by_id(&connection, session_id).expect("timer session should reload");
        assert_eq!(unchanged.session_date, original.session_date);
        assert_eq!(unchanged.duration_seconds, original.duration_seconds);
        assert_eq!(unchanged.started_at, original.started_at);
        assert_eq!(unchanged.ended_at, original.ended_at);
        assert_eq!(unchanged.notes, "Edited context");
    }

    #[test]
    fn invalid_sessions_are_rejected_without_writing() {
        let mut connection = connection();
        let count_before: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .unwrap();

        let invalid_dates = ["", "2026-02-30", "30-10-2026"];
        for invalid_date in invalid_dates {
            let error = create_manual(&mut connection, input(invalid_date, 600, ""))
                .expect_err("invalid date should fail");
            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some("sessionDate"));
        }

        for invalid_duration in [-1, 0, MAX_DURATION_SECONDS + 1] {
            let error = create_manual(&mut connection, input("2026-11-02", invalid_duration, ""))
                .expect_err("invalid duration should fail");
            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some("durationSeconds"));
        }

        let long_notes = "x".repeat(MAX_NOTES_CHARS + 1);
        let error = create_manual(&mut connection, input("2026-11-02", 600, &long_notes))
            .expect_err("overlong notes should fail");
        assert_eq!(error.code, "validation_error");
        assert_eq!(error.field, Some("notes"));

        let count_after: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count_before, count_after);
    }

    #[test]
    fn malformed_filters_and_missing_sessions_are_rejected() {
        let mut connection = connection();

        let error = list(
            &connection,
            StudySessionFilter {
                from_date: Some("2026-11-03".to_owned()),
                to_date: Some("2026-11-01".to_owned()),
                topic_id: None,
            },
        )
        .expect_err("reversed range should fail");
        assert_eq!(error.field, Some("fromDate"));

        let error = list(
            &connection,
            StudySessionFilter {
                from_date: None,
                to_date: None,
                topic_id: Some(999_999),
            },
        )
        .expect_err("unknown topic should fail");
        assert_eq!(error.field, Some("topicId"));

        let error = update(&mut connection, 999_999, input("2026-11-02", 600, ""))
            .expect_err("missing session should fail");
        assert_eq!(error.code, "session_not_found");

        let error = delete(&mut connection, 999_999).expect_err("missing session should fail");
        assert_eq!(error.code, "session_not_found");
    }

    #[test]
    fn archived_topics_keep_historical_sessions_and_allow_note_edits() {
        let mut connection = connection();
        let topic = topic_id(&connection, "GA-01");
        let created = create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 900,
                notes: "Original notes".to_owned(),
            },
        )
        .unwrap();

        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?1",
                [topic],
            )
            .unwrap();

        let history = list(&connection, StudySessionFilter::default()).unwrap();
        assert_eq!(history.len(), 1);
        let archived_topic = history[0].topic.as_ref().expect("topic should remain");
        assert!(archived_topic.archived);
        assert_eq!(archived_topic.code, "GA-01");

        let updated = update(
            &mut connection,
            created.id,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 900,
                notes: "Edited after archiving".to_owned(),
            },
        )
        .expect("existing archived topic should be allowed");
        assert_eq!(updated.notes, "Edited after archiving");

        let error = create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 900,
                notes: String::new(),
            },
        )
        .expect_err("archived topic cannot be used for a new session");
        assert_eq!(error.code, "topic_not_found");
    }

    #[test]
    fn topic_filter_applies_to_range_daily_and_weekly_totals() {
        let mut connection = connection();
        let ga = topic_id(&connection, "GA-01");
        let mmp = topic_id(&connection, "MMP-C01");

        for (topic, session_date, duration) in [
            (ga, "2026-11-02", 600_i64),
            (mmp, "2026-11-02", 1_200_i64),
            (ga, "2026-11-03", 300_i64),
        ] {
            create_manual(
                &mut connection,
                StudySessionInput {
                    topic_id: Some(topic),
                    session_date: session_date.to_owned(),
                    duration_seconds: duration,
                    notes: String::new(),
                },
            )
            .unwrap();
        }

        let filtered = totals(
            &connection,
            "2026-11-02",
            "2026-11-03",
            date(2026, 11, 3),
            Some(ga),
        )
        .expect("topic-filtered totals should load");

        assert_eq!(filtered.range_seconds, 900);
        assert_eq!(filtered.range_session_count, 2);
        assert_eq!(filtered.today_seconds, 300);
        assert_eq!(filtered.week_seconds, 900);
        assert_eq!(filtered.daily.len(), 2);
        assert_eq!(filtered.daily[0].date, "2026-11-02");
        assert_eq!(filtered.daily[0].seconds, 600);
        assert_eq!(filtered.daily[0].session_count, 1);
        assert_eq!(filtered.daily[1].date, "2026-11-03");
        assert_eq!(filtered.daily[1].seconds, 300);

        let all_topics = totals(
            &connection,
            "2026-11-02",
            "2026-11-03",
            date(2026, 11, 3),
            None,
        )
        .expect("unfiltered totals should load");
        assert_eq!(all_topics.range_seconds, 2_100);
    }

    #[test]
    fn archived_topics_with_history_remain_available_for_filters() {
        let mut connection = connection();
        let ga = topic_id(&connection, "GA-01");
        let mmp = topic_id(&connection, "MMP-C01");

        create_manual(
            &mut connection,
            StudySessionInput {
                topic_id: Some(ga),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 900,
                notes: String::new(),
            },
        )
        .unwrap();

        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id IN (?1, ?2)",
                params![ga, mmp],
            )
            .unwrap();

        let topics = list_topics(&connection).expect("session topics should load");
        let ga_entry = topics
            .iter()
            .find(|topic| topic.id == ga)
            .expect("archived topic with history should remain listed");
        assert!(ga_entry.archived);
        assert_eq!(ga_entry.code, "GA-01");
        assert!(
            topics.iter().all(|topic| topic.id != mmp),
            "an archived topic without history should not be offered for new sessions"
        );
        assert!(
            topics.iter().any(|topic| !topic.archived),
            "active topics should still be available"
        );

        let filtered = list(
            &connection,
            StudySessionFilter {
                from_date: None,
                to_date: None,
                topic_id: Some(ga),
            },
        )
        .expect("archived topic history should remain filterable");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].duration_seconds, 900);
        assert!(filtered[0].topic.as_ref().unwrap().archived);
    }

    #[test]
    fn daily_and_weekly_totals_are_grouped_by_local_session_date() {
        let mut connection = connection();
        let dates = [
            ("2026-10-26", 600_i64),   // Monday
            ("2026-10-28", 900_i64),   // Wednesday
            ("2026-11-01", 1_200_i64), // Sunday
            ("2026-11-02", 1_800_i64), // Next Monday
        ];

        for (session_date, duration) in dates {
            create_manual(&mut connection, input(session_date, duration, "")).unwrap();
        }

        let result = totals(
            &connection,
            "2026-10-26",
            "2026-11-02",
            date(2026, 11, 2),
            None,
        )
        .expect("totals should load");
        assert_eq!(result.range_seconds, 4_500);
        assert_eq!(result.range_session_count, 4);
        assert_eq!(result.daily.len(), 4);
        assert_eq!(result.weekly.len(), 2);
        assert_eq!(result.weekly[0].week_start, "2026-10-26");
        assert_eq!(result.weekly[0].week_end, "2026-11-01");
        assert_eq!(result.weekly[0].seconds, 2_700);
        assert_eq!(result.weekly[1].week_start, "2026-11-02");
        assert_eq!(result.weekly[1].seconds, 1_800);
        assert_eq!(result.today_seconds, 1_800);
        assert_eq!(result.week_seconds, 1_800);
    }

    #[test]
    fn timer_sessions_are_append_only_and_keep_break_time_out_of_totals() {
        let connection = connection();
        let topic = topic_id(&connection, "GA-01");
        let mut connection = connection;

        let transaction = connection.transaction().unwrap();
        let session_id = insert_timer_session(
            &transaction,
            Some(topic),
            "2026-10-30",
            "2026-10-30T10:00:00.000Z",
            "2026-10-30T10:25:00.000Z",
            1_500,
            "",
        )
        .expect("timer session should insert");
        transaction.commit().unwrap();

        assert!(session_id > 0);
        let sessions = list(&connection, StudySessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].source, "timer");
        assert_eq!(sessions[0].duration_seconds, 1_500);
        assert_eq!(
            sessions[0].started_at.as_deref(),
            Some("2026-10-30T10:00:00.000Z")
        );
        assert_eq!(
            sessions[0].ended_at.as_deref(),
            Some("2026-10-30T10:25:00.000Z")
        );

        let totals = totals(
            &connection,
            "2026-10-30",
            "2026-10-30",
            date(2026, 10, 30),
            None,
        )
        .unwrap();
        assert_eq!(totals.range_seconds, 1_500);
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM study_sessions", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}
