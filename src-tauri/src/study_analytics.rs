use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Days, NaiveDate};
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::{date_only, error::AppError, settings};

pub const MAX_ANALYTICS_RANGE_DAYS: i64 = 3_660;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTrendDay {
    pub date: String,
    pub seconds: i64,
    pub session_count: i64,
    pub eligible: bool,
    pub target_seconds: i64,
    pub target_met: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTrendWeek {
    pub week_start: String,
    pub week_end: String,
    pub seconds: i64,
    pub session_count: i64,
    pub eligible_days: i64,
    pub target_seconds: i64,
    pub target_met: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyTrendMonth {
    pub month: String,
    pub month_start: String,
    pub month_end: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DurationBucket {
    pub key: String,
    pub label: String,
    pub minimum_seconds: i64,
    pub maximum_seconds: Option<i64>,
    pub session_count: i64,
    pub seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitStudyTime {
    pub section_code: String,
    pub section_title: String,
    pub seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousPeriodStudy {
    pub from_date: String,
    pub to_date: String,
    pub seconds: i64,
    pub session_count: i64,
    pub active_days: i64,
    pub eligible_days: i64,
    pub target_met_days: i64,
    pub target_completion_rate: Option<f64>,
    pub seconds_change_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionConsistency {
    pub completed_in_range: i64,
    pub on_time_count: i64,
    pub late_count: i64,
    pub on_time_rate: Option<f64>,
    pub active_scheduled_count: i64,
    pub currently_overdue_count: i64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyAnalytics {
    pub from_date: String,
    pub to_date: String,
    pub today: String,
    pub preparation_start_date: String,
    pub daily_target_minutes: i64,
    pub daily_target_seconds: i64,
    pub range_seconds: i64,
    pub range_session_count: i64,
    pub total_recorded_seconds: i64,
    pub total_recorded_sessions: i64,
    pub eligible_days: i64,
    pub active_days: i64,
    pub consistent_days_percent: Option<f64>,
    pub target_met_days: i64,
    pub missed_target_days: i64,
    pub target_completion_rate: Option<f64>,
    pub total_active_days: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub daily: Vec<StudyTrendDay>,
    pub weekly: Vec<StudyTrendWeek>,
    pub monthly: Vec<StudyTrendMonth>,
    pub weekly_targets_met: i64,
    pub weekly_targets_total: i64,
    pub weekly_target_completion_rate: Option<f64>,
    pub duration_buckets: Vec<DurationBucket>,
    pub by_unit: Vec<UnitStudyTime>,
    pub unassigned_seconds: i64,
    pub unassigned_session_count: i64,
    pub focus_seconds: i64,
    pub break_seconds: Option<i64>,
    pub break_tracking_supported: bool,
    pub previous_period: PreviousPeriodStudy,
    pub revision_consistency: RevisionConsistency,
}

#[derive(Default)]
struct DailyAccumulator {
    seconds: i64,
    session_count: i64,
}

#[derive(Default)]
struct WeekAccumulator {
    seconds: i64,
    eligible_seconds: i64,
    session_count: i64,
    eligible_days: i64,
    target_seconds: i64,
}

#[derive(Default)]
struct MonthAccumulator {
    seconds: i64,
    session_count: i64,
}

pub fn study_analytics(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    today: NaiveDate,
) -> Result<StudyAnalytics, AppError> {
    let from = parse_date(from_date, "fromDate")?;
    let to = parse_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }
    if to > today {
        return Err(AppError::validation(
            "toDate",
            "Study analytics cannot include future dates.",
        ));
    }
    if (to - from).num_days() + 1 > MAX_ANALYTICS_RANGE_DAYS {
        return Err(AppError::validation(
            "fromDate",
            format!("Choose a range of {MAX_ANALYTICS_RANGE_DAYS} days or fewer."),
        ));
    }

    let settings = settings::load(connection)?;
    let preparation_start = parse_date(&settings.preparation_start_date, "preparationStartDate")?;
    let daily_target_seconds = settings.daily_study_target_minutes * 60;
    let range_from = date_only::format_date_only(from);
    let range_to = date_only::format_date_only(to);
    let today_text = date_only::format_date_only(today);

    let mut daily_sessions: BTreeMap<String, DailyAccumulator> = BTreeMap::new();
    let mut statement = connection
        .prepare(
            "SELECT session_date, SUM(duration_seconds), COUNT(*)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
             GROUP BY session_date
             ORDER BY session_date",
        )
        .map_err(|error| {
            AppError::database("Study analytics sessions could not be prepared", error)
        })?;
    let rows = statement
        .query_map(params![range_from, range_to], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| {
            AppError::database("Study analytics sessions could not be queried", error)
        })?;
    for row in rows {
        let (date, seconds, session_count) = row.map_err(|error| {
            AppError::database("A study analytics day could not be read", error)
        })?;
        daily_sessions.insert(
            date,
            DailyAccumulator {
                seconds,
                session_count,
            },
        );
    }

    let (total_recorded_seconds, total_recorded_sessions): (i64, i64) = connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*) FROM study_sessions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            AppError::database("Total recorded study time could not be calculated", error)
        })?;

    let active_dates = active_dates_since(connection, preparation_start, today)?;
    let current_streak = current_streak(&active_dates, today);
    let longest_streak = longest_streak(&active_dates);
    let total_active_days = active_dates.len() as i64;

    let mut daily = Vec::new();
    let mut weekly: BTreeMap<String, (NaiveDate, WeekAccumulator)> = BTreeMap::new();
    let mut monthly: BTreeMap<String, (NaiveDate, MonthAccumulator)> = BTreeMap::new();
    let mut range_seconds = 0_i64;
    let mut range_session_count = 0_i64;
    let mut eligible_days = 0_i64;
    let mut active_days = 0_i64;
    let mut target_met_days = 0_i64;
    let mut missed_target_days = 0_i64;

    let mut cursor = from;
    while cursor <= to {
        let date_text = date_only::format_date_only(cursor);
        let data = daily_sessions.get(&date_text);
        let seconds = data.map_or(0, |entry| entry.seconds);
        let session_count = data.map_or(0, |entry| entry.session_count);
        let eligible = cursor >= preparation_start && cursor <= today;
        let target_seconds = if eligible { daily_target_seconds } else { 0 };
        let target_met = eligible && seconds >= daily_target_seconds;

        range_seconds += seconds;
        range_session_count += session_count;
        if eligible {
            eligible_days += 1;
            if seconds > 0 {
                active_days += 1;
            }
            if target_met {
                target_met_days += 1;
            }
            if cursor < today && seconds < daily_target_seconds {
                missed_target_days += 1;
            }
        }

        daily.push(StudyTrendDay {
            date: date_text,
            seconds,
            session_count,
            eligible,
            target_seconds,
            target_met,
        });

        let week_start = monday_of(cursor);
        let week_key = date_only::format_date_only(week_start);
        let week_end = week_start + Days::new(6);
        let week_entry = weekly
            .entry(week_key)
            .or_insert((week_end, WeekAccumulator::default()));
        week_entry.1.seconds += seconds;
        week_entry.1.session_count += session_count;
        if eligible {
            week_entry.1.eligible_seconds += seconds;
            week_entry.1.eligible_days += 1;
            week_entry.1.target_seconds += daily_target_seconds;
        }

        let month_key = format!("{:04}-{:02}", cursor.year(), cursor.month());
        let month_start = NaiveDate::from_ymd_opt(cursor.year(), cursor.month(), 1)
            .ok_or_else(|| AppError::internal("A calendar month could not be represented."))?;
        let month_entry = monthly
            .entry(month_key)
            .or_insert((month_start, MonthAccumulator::default()));
        month_entry.1.seconds += seconds;
        month_entry.1.session_count += session_count;

        cursor = cursor
            .checked_add_days(Days::new(1))
            .ok_or_else(|| AppError::validation("toDate", "The date range is too large."))?;
    }

    let weekly = weekly
        .into_iter()
        .map(|(week_start, (week_end, data))| {
            let target_met = data.eligible_days > 0 && data.eligible_seconds >= data.target_seconds;
            StudyTrendWeek {
                week_start,
                week_end: date_only::format_date_only(week_end),
                seconds: data.seconds,
                session_count: data.session_count,
                eligible_days: data.eligible_days,
                target_seconds: data.target_seconds,
                target_met,
            }
        })
        .collect::<Vec<_>>();
    let weekly_targets_total = weekly.iter().filter(|week| week.eligible_days > 0).count() as i64;
    let weekly_targets_met = weekly.iter().filter(|week| week.target_met).count() as i64;

    let monthly = monthly
        .into_iter()
        .map(|(month, (month_start, data))| {
            let month_end = end_of_month(month_start)
                .ok_or_else(|| AppError::internal("A calendar month could not be represented."))?;
            Ok(StudyTrendMonth {
                month,
                month_start: date_only::format_date_only(month_start),
                month_end: date_only::format_date_only(month_end),
                seconds: data.seconds,
                session_count: data.session_count,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let duration_buckets = duration_buckets(connection, &range_from, &range_to)?;
    let (by_unit, unassigned_seconds, unassigned_session_count) =
        study_by_unit(connection, &range_from, &range_to)?;

    let previous_period = previous_period(
        connection,
        from,
        to,
        range_seconds,
        preparation_start,
        daily_target_seconds,
    )?;
    let revision_consistency = revision_consistency(connection, from, to, today)?;

    Ok(StudyAnalytics {
        from_date: range_from,
        to_date: range_to,
        today: today_text,
        preparation_start_date: settings.preparation_start_date,
        daily_target_minutes: settings.daily_study_target_minutes,
        daily_target_seconds,
        range_seconds,
        range_session_count,
        total_recorded_seconds,
        total_recorded_sessions,
        eligible_days,
        active_days,
        consistent_days_percent: percentage(active_days, eligible_days),
        target_met_days,
        missed_target_days,
        target_completion_rate: percentage(target_met_days, eligible_days),
        total_active_days,
        current_streak,
        longest_streak,
        daily,
        weekly,
        monthly,
        weekly_targets_met,
        weekly_targets_total,
        weekly_target_completion_rate: percentage(weekly_targets_met, weekly_targets_total),
        duration_buckets,
        by_unit,
        unassigned_seconds,
        unassigned_session_count,
        focus_seconds: range_seconds,
        break_seconds: None,
        break_tracking_supported: false,
        previous_period,
        revision_consistency,
    })
}

fn active_dates_since(
    connection: &Connection,
    preparation_start: NaiveDate,
    today: NaiveDate,
) -> Result<BTreeSet<NaiveDate>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT session_date
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
             ORDER BY session_date",
        )
        .map_err(|error| AppError::database("Study streak dates could not be prepared", error))?;
    let rows = statement
        .query_map(
            params![
                date_only::format_date_only(preparation_start),
                date_only::format_date_only(today)
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| AppError::database("Study streak dates could not be queried", error))?;

    let mut dates = BTreeSet::new();
    for row in rows {
        let value = row
            .map_err(|error| AppError::database("A study streak date could not be read", error))?;
        let date = parse_date(&value, "sessionDate")?;
        dates.insert(date);
    }
    Ok(dates)
}

/// A current streak keeps today's grace period: if today has no recorded study,
/// the streak may still end yesterday. A full missed calendar day ends it.
fn current_streak(active_dates: &BTreeSet<NaiveDate>, today: NaiveDate) -> i64 {
    let mut cursor = today;
    if !active_dates.contains(&cursor) {
        let Some(yesterday) = cursor.checked_sub_days(Days::new(1)) else {
            return 0;
        };
        cursor = yesterday;
    }

    let mut streak = 0_i64;
    while active_dates.contains(&cursor) {
        streak += 1;
        let Some(previous) = cursor.checked_sub_days(Days::new(1)) else {
            break;
        };
        cursor = previous;
    }
    streak
}

fn longest_streak(active_dates: &BTreeSet<NaiveDate>) -> i64 {
    let mut longest = 0_i64;
    let mut running = 0_i64;
    let mut previous: Option<NaiveDate> = None;

    for date in active_dates {
        running = if previous.is_some_and(|value| value + Days::new(1) == *date) {
            running + 1
        } else {
            1
        };
        longest = longest.max(running);
        previous = Some(*date);
    }

    longest
}

fn duration_buckets(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<DurationBucket>, AppError> {
    let definitions = [
        ("under_15", "Under 15 minutes", 1_i64, Some(899_i64)),
        ("15_to_29", "15-29 minutes", 900, Some(1_799)),
        ("30_to_59", "30-59 minutes", 1_800, Some(3_599)),
        ("60_to_89", "60-89 minutes", 3_600, Some(5_399)),
        ("90_to_119", "90-119 minutes", 5_400, Some(7_199)),
        ("120_plus", "120 minutes or more", 7_200, None),
    ];
    let mut buckets = definitions
        .iter()
        .map(|(key, label, minimum, maximum)| DurationBucket {
            key: (*key).to_owned(),
            label: (*label).to_owned(),
            minimum_seconds: *minimum,
            maximum_seconds: *maximum,
            session_count: 0,
            seconds: 0,
        })
        .collect::<Vec<_>>();

    let mut statement = connection
        .prepare(
            "SELECT duration_seconds
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
             ORDER BY duration_seconds",
        )
        .map_err(|error| {
            AppError::database("Session duration buckets could not be prepared", error)
        })?;
    let rows = statement
        .query_map(params![from_date, to_date], |row| row.get::<_, i64>(0))
        .map_err(|error| {
            AppError::database("Session duration buckets could not be queried", error)
        })?;

    for row in rows {
        let seconds =
            row.map_err(|error| AppError::database("A session duration could not be read", error))?;
        if let Some(bucket) = buckets.iter_mut().find(|bucket| {
            seconds >= bucket.minimum_seconds
                && bucket
                    .maximum_seconds
                    .is_none_or(|maximum| seconds <= maximum)
        }) {
            bucket.session_count += 1;
            bucket.seconds += seconds;
        }
    }

    Ok(buckets)
}

fn study_by_unit(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
) -> Result<(Vec<UnitStudyTime>, i64, i64), AppError> {
    let mut statement = connection
        .prepare(
            "SELECT s.code, s.title, COALESCE(SUM(ss.duration_seconds), 0), COUNT(*)
             FROM study_sessions ss
             JOIN syllabus_topics t ON t.id = ss.topic_id
             JOIN syllabus_sections s ON s.id = t.section_id
             WHERE ss.session_date >= ?1 AND ss.session_date <= ?2
             GROUP BY s.id, s.code, s.title
             ORDER BY COALESCE(SUM(ss.duration_seconds), 0) DESC, s.title",
        )
        .map_err(|error| AppError::database("Study time by unit could not be prepared", error))?;
    let rows = statement
        .query_map(params![from_date, to_date], |row| {
            Ok(UnitStudyTime {
                section_code: row.get(0)?,
                section_title: row.get(1)?,
                seconds: row.get(2)?,
                session_count: row.get(3)?,
            })
        })
        .map_err(|error| AppError::database("Study time by unit could not be queried", error))?;
    let by_unit = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Study time by unit could not be read", error))?;

    let (unassigned_seconds, unassigned_session_count): (i64, i64) = connection
        .query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0), COUNT(*)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2 AND topic_id IS NULL",
            params![from_date, to_date],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            AppError::database("Unassigned study time could not be calculated", error)
        })?;

    Ok((by_unit, unassigned_seconds, unassigned_session_count))
}

fn previous_period(
    connection: &Connection,
    from: NaiveDate,
    to: NaiveDate,
    current_seconds: i64,
    preparation_start: NaiveDate,
    daily_target_seconds: i64,
) -> Result<PreviousPeriodStudy, AppError> {
    let days = (to - from).num_days() + 1;
    let previous_to = from.checked_sub_days(Days::new(1)).ok_or_else(|| {
        AppError::validation("fromDate", "The comparison period is outside the calendar.")
    })?;
    let previous_from = from
        .checked_sub_days(Days::new(days as u64))
        .ok_or_else(|| {
            AppError::validation("fromDate", "The comparison period is outside the calendar.")
        })?;
    let previous_from_text = date_only::format_date_only(previous_from);
    let previous_to_text = date_only::format_date_only(previous_to);

    let mut statement = connection
        .prepare(
            "SELECT session_date, SUM(duration_seconds), COUNT(*)
             FROM study_sessions
             WHERE session_date >= ?1 AND session_date <= ?2
             GROUP BY session_date",
        )
        .map_err(|error| {
            AppError::database("The comparison period could not be prepared", error)
        })?;
    let rows = statement
        .query_map(params![previous_from_text, previous_to_text], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| AppError::database("The comparison period could not be queried", error))?;
    let mut daily = BTreeMap::new();
    for row in rows {
        let (date, seconds, session_count) =
            row.map_err(|error| AppError::database("A comparison day could not be read", error))?;
        daily.insert(date, (seconds, session_count));
    }

    let mut seconds = 0_i64;
    let mut session_count = 0_i64;
    let mut active_days = 0_i64;
    let mut eligible_days = 0_i64;
    let mut target_met_days = 0_i64;
    let mut cursor = previous_from;
    while cursor <= previous_to {
        let date_text = date_only::format_date_only(cursor);
        let (day_seconds, day_sessions) = daily.get(&date_text).copied().unwrap_or((0, 0));
        seconds += day_seconds;
        session_count += day_sessions;
        if day_seconds > 0 {
            active_days += 1;
        }
        if cursor >= preparation_start {
            eligible_days += 1;
            if day_seconds >= daily_target_seconds {
                target_met_days += 1;
            }
        }
        cursor = cursor
            .checked_add_days(Days::new(1))
            .ok_or_else(|| AppError::internal("The comparison period could not advance."))?;
    }

    Ok(PreviousPeriodStudy {
        from_date: previous_from_text,
        to_date: previous_to_text,
        seconds,
        session_count,
        active_days,
        eligible_days,
        target_met_days,
        target_completion_rate: percentage(target_met_days, eligible_days),
        seconds_change_percent: if seconds == 0 {
            None
        } else {
            Some(round_one(
                (current_seconds - seconds) as f64 / seconds as f64 * 100.0,
            ))
        },
    })
}

fn revision_consistency(
    connection: &Connection,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
) -> Result<RevisionConsistency, AppError> {
    let (completed_in_range, on_time_count): (i64, i64) = connection
        .query_row(
            "SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN revision_date <= scheduled_for_date THEN 1 ELSE 0 END), 0)
             FROM revision_history
             WHERE revision_date >= ?1 AND revision_date <= ?2",
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            AppError::database("Revision consistency could not be calculated", error)
        })?;
    let (active_scheduled_count, currently_overdue_count): (i64, i64) = connection
        .query_row(
            "SELECT
                COUNT(*),
                COALESCE(SUM(CASE WHEN next_revision_date < ?1 THEN 1 ELSE 0 END), 0)
             FROM revision_schedules
             WHERE status = 'active'",
            [date_only::format_date_only(today)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            AppError::database(
                "Current revision consistency could not be calculated",
                error,
            )
        })?;

    Ok(RevisionConsistency {
        completed_in_range,
        on_time_count,
        late_count: completed_in_range - on_time_count,
        on_time_rate: percentage(on_time_count, completed_in_range),
        active_scheduled_count,
        currently_overdue_count,
        note: "Revision consistency counts completed revisions in this range whose completion date was on or before the scheduled date. Currently overdue active schedules are shown separately.".to_owned(),
    })
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Days::new(date.weekday().num_days_from_monday() as u64)
}

fn end_of_month(date: NaiveDate) -> Option<NaiveDate> {
    let next = if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)?
    };
    next.checked_sub_days(Days::new(1))
}

fn percentage(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator <= 0 {
        None
    } else {
        Some(round_one(numerator as f64 / denominator as f64 * 100.0))
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
    use std::collections::BTreeSet;

    use chrono::NaiveDate;
    use rusqlite::Connection;

    use super::*;
    use crate::{
        db, revision, sessions,
        settings::{self, SettingsInput},
        syllabus,
    };

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection(today: NaiveDate, preparation_start: &str, target_minutes: i64) -> Connection {
        let mut connection = db::test_connection_with_defaults(today);
        syllabus::seed(&mut connection).expect("syllabus should seed");
        settings::update(
            &mut connection,
            SettingsInput {
                exam_date: "2027-12-19".to_owned(),
                preparation_start_date: preparation_start.to_owned(),
                daily_study_target_minutes: target_minutes,
            },
            today,
        )
        .expect("settings should save");
        connection
    }

    fn topic(connection: &Connection, code: &str) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = ?1",
                [code],
                |row| row.get(0),
            )
            .expect("topic should exist")
    }

    fn session(connection: &mut Connection, topic_id: i64, day: &str, seconds: i64) {
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

    #[test]
    fn totals_targets_streaks_duration_and_comparison_are_calculated_from_saved_sessions() {
        let today = date(2026, 10, 30);
        let mut connection = connection(today, "2026-10-01", 60);
        let topic_id = topic(&connection, "MMP-C01");

        for (day, seconds) in [
            ("2026-09-20", 7_200),
            ("2026-10-25", 3_600),
            ("2026-10-26", 1_800),
            ("2026-10-27", 3_600),
            ("2026-10-28", 3_600),
            ("2026-10-29", 3_600),
            ("2026-10-30", 1_800),
        ] {
            session(&mut connection, topic_id, day, seconds);
        }

        let analytics = study_analytics(&connection, "2026-10-27", "2026-10-30", today)
            .expect("analytics should load");

        assert_eq!(analytics.range_seconds, 12_600);
        assert_eq!(analytics.range_session_count, 4);
        assert_eq!(analytics.total_recorded_seconds, 25_200);
        assert_eq!(analytics.total_recorded_sessions, 7);
        assert_eq!(analytics.eligible_days, 4);
        assert_eq!(analytics.active_days, 4);
        assert_eq!(analytics.consistent_days_percent, Some(100.0));
        assert_eq!(analytics.target_met_days, 3);
        assert_eq!(analytics.missed_target_days, 0);
        assert_eq!(analytics.target_completion_rate, Some(75.0));
        assert_eq!(analytics.total_active_days, 6);
        assert_eq!(analytics.current_streak, 6);
        assert_eq!(analytics.longest_streak, 6);
        assert_eq!(analytics.daily.len(), 4);
        assert!(analytics.daily[0].eligible);
        assert_eq!(analytics.weekly.len(), 1);
        assert_eq!(analytics.weekly[0].eligible_days, 4);
        assert_eq!(analytics.weekly[0].seconds, 12_600);
        assert!(!analytics.weekly[0].target_met);
        assert_eq!(analytics.monthly.len(), 1);
        assert_eq!(analytics.monthly[0].month, "2026-10");
        assert_eq!(analytics.monthly[0].seconds, 12_600);

        let under_30 = analytics
            .duration_buckets
            .iter()
            .find(|bucket| bucket.key == "30_to_59")
            .expect("bucket should exist");
        let one_hour = analytics
            .duration_buckets
            .iter()
            .find(|bucket| bucket.key == "60_to_89")
            .expect("bucket should exist");
        assert_eq!(under_30.session_count, 1);
        assert_eq!(under_30.seconds, 1_800);
        assert_eq!(one_hour.session_count, 3);
        assert_eq!(one_hour.seconds, 10_800);

        assert_eq!(analytics.by_unit.len(), 1);
        assert_eq!(analytics.by_unit[0].seconds, 12_600);
        assert_eq!(analytics.unassigned_session_count, 0);
        assert_eq!(analytics.focus_seconds, 12_600);
        assert_eq!(analytics.break_seconds, None);
        assert!(!analytics.break_tracking_supported);

        assert_eq!(analytics.previous_period.from_date, "2026-10-23");
        assert_eq!(analytics.previous_period.to_date, "2026-10-26");
        assert_eq!(analytics.previous_period.seconds, 5_400);
        assert_eq!(analytics.previous_period.active_days, 2);
        assert_eq!(analytics.previous_period.eligible_days, 4);
        assert_eq!(analytics.previous_period.target_met_days, 1);
        assert_eq!(
            analytics.previous_period.seconds_change_percent,
            Some(133.3)
        );
    }

    #[test]
    fn pre_preparation_days_do_not_count_for_targets_streaks_or_consistency() {
        let today = date(2026, 10, 30);
        let mut connection = connection(today, "2026-10-28", 60);
        let topic_id = topic(&connection, "MMP-C01");
        for (day, seconds) in [
            ("2026-10-01", 7_200),
            ("2026-10-28", 3_600),
            ("2026-10-29", 3_600),
            ("2026-10-30", 3_600),
        ] {
            session(&mut connection, topic_id, day, seconds);
        }

        let analytics = study_analytics(&connection, "2026-10-27", "2026-10-30", today)
            .expect("analytics should load");

        assert_eq!(analytics.eligible_days, 3);
        assert_eq!(analytics.active_days, 3);
        assert_eq!(analytics.total_active_days, 3);
        assert_eq!(analytics.current_streak, 3);
        assert!(!analytics.daily[0].eligible);
        assert_eq!(analytics.daily[0].seconds, 0);
        assert_eq!(analytics.monthly[0].seconds, 10_800);
        assert_eq!(analytics.total_recorded_seconds, 18_000);
    }

    #[test]
    fn current_streak_has_one_grace_day_and_longest_streak_spans_sorted_dates() {
        let today = date(2026, 10, 30);
        let mut dates = BTreeSet::new();
        for day in [27, 28, 29] {
            dates.insert(date(2026, 10, day));
        }
        assert_eq!(current_streak(&dates, today), 3);
        assert_eq!(longest_streak(&dates), 3);

        dates.remove(&date(2026, 10, 27));
        assert_eq!(current_streak(&dates, today), 2);
        assert_eq!(longest_streak(&dates), 2);

        let no_active_today = BTreeSet::from([date(2026, 10, 27), date(2026, 10, 28)]);
        assert_eq!(current_streak(&no_active_today, today), 0);
        assert_eq!(longest_streak(&no_active_today), 2);
    }

    #[test]
    fn leap_day_and_month_boundaries_use_local_date_strings() {
        let today = date(2024, 3, 1);
        let mut connection = connection(today, "2024-02-28", 50);
        let topic_id = topic(&connection, "MMP-C01");
        for (day, seconds) in [
            ("2024-02-28", 3_000),
            ("2024-02-29", 3_000),
            ("2024-03-01", 3_000),
        ] {
            session(&mut connection, topic_id, day, seconds);
        }

        let analytics = study_analytics(&connection, "2024-02-28", "2024-03-01", today)
            .expect("analytics should load");
        assert_eq!(analytics.daily.len(), 3);
        assert_eq!(analytics.daily[1].date, "2024-02-29");
        assert_eq!(analytics.monthly.len(), 2);
        assert_eq!(analytics.monthly[0].month, "2024-02");
        assert_eq!(analytics.monthly[0].month_end, "2024-02-29");
        assert_eq!(analytics.monthly[1].month, "2024-03");
        assert_eq!(analytics.current_streak, 3);
        assert_eq!(analytics.target_met_days, 3);
    }

    #[test]
    fn weekly_targets_missed_days_and_year_transition_use_eligible_local_days() {
        let today = date(2027, 1, 3);
        let mut connection = connection(today, "2026-12-28", 20);
        let topic_id = topic(&connection, "MMP-C01");
        for day in [
            "2026-12-28",
            "2026-12-29",
            "2026-12-30",
            "2026-12-31",
            "2027-01-01",
            "2027-01-03",
        ] {
            session(&mut connection, topic_id, day, 1_800);
        }

        let analytics = study_analytics(&connection, "2026-12-28", "2027-01-03", today)
            .expect("analytics should load");
        assert_eq!(analytics.eligible_days, 7);
        assert_eq!(analytics.target_met_days, 6);
        assert_eq!(analytics.missed_target_days, 1);
        assert_eq!(analytics.target_completion_rate, Some(85.7));
        assert_eq!(analytics.weekly.len(), 1);
        assert_eq!(analytics.weekly[0].week_start, "2026-12-28");
        assert_eq!(analytics.weekly[0].week_end, "2027-01-03");
        assert_eq!(analytics.weekly[0].eligible_days, 7);
        assert_eq!(analytics.weekly[0].target_seconds, 8_400);
        assert!(analytics.weekly[0].target_met);
        assert_eq!(analytics.weekly_targets_met, 1);
        assert_eq!(analytics.weekly_targets_total, 1);
        assert_eq!(analytics.weekly_target_completion_rate, Some(100.0));
        assert_eq!(analytics.monthly.len(), 2);
        assert_eq!(analytics.monthly[0].month, "2026-12");
        assert_eq!(analytics.monthly[0].seconds, 7_200);
        assert_eq!(analytics.monthly[1].month, "2027-01");
        assert_eq!(analytics.monthly[1].seconds, 3_600);
        assert_eq!(analytics.current_streak, 1);
        assert_eq!(analytics.longest_streak, 5);
        assert_eq!(analytics.previous_period.from_date, "2026-12-21");
        assert_eq!(analytics.previous_period.to_date, "2026-12-27");
    }

    #[test]
    fn revision_consistency_counts_on_time_completions_and_live_overdue_schedules() {
        let today = date(2026, 10, 30);
        let mut connection = connection(today, "2026-10-01", 60);
        let late_topic = topic(&connection, "MMP-C01");
        let on_time_topic = topic(&connection, "GA-01");
        let overdue_topic = topic(&connection, "CM-C01");

        revision::schedule_topic(&mut connection, late_topic, "2026-10-28", today)
            .expect("schedule");
        revision::complete(
            &mut connection,
            late_topic,
            revision::CompleteRevisionInput {
                outcome: "good".to_owned(),
                notes: String::new(),
            },
            date(2026, 10, 29),
        )
        .expect("late completion");
        revision::schedule_topic(&mut connection, on_time_topic, "2026-10-29", today)
            .expect("schedule");
        revision::complete(
            &mut connection,
            on_time_topic,
            revision::CompleteRevisionInput {
                outcome: "easy".to_owned(),
                notes: String::new(),
            },
            date(2026, 10, 29),
        )
        .expect("on-time completion");
        revision::schedule_topic(&mut connection, overdue_topic, "2026-10-27", today)
            .expect("overdue schedule");

        let analytics = study_analytics(&connection, "2026-10-01", "2026-10-30", today)
            .expect("analytics should load");
        assert_eq!(analytics.revision_consistency.completed_in_range, 2);
        assert_eq!(analytics.revision_consistency.on_time_count, 1);
        assert_eq!(analytics.revision_consistency.late_count, 1);
        assert_eq!(analytics.revision_consistency.on_time_rate, Some(50.0));
        assert_eq!(analytics.revision_consistency.active_scheduled_count, 3);
        assert_eq!(analytics.revision_consistency.currently_overdue_count, 1);
    }

    #[test]
    fn range_validation_rejects_future_dates_reversed_ranges_and_oversized_ranges() {
        let today = date(2026, 10, 30);
        let connection = connection(today, "2026-10-01", 60);

        assert_eq!(
            study_analytics(&connection, "2026-10-30", "2026-10-31", today)
                .expect_err("future range should fail")
                .field,
            Some("toDate")
        );
        assert_eq!(
            study_analytics(&connection, "2026-10-30", "2026-10-29", today)
                .expect_err("reversed range should fail")
                .field,
            Some("fromDate")
        );
        assert_eq!(
            study_analytics(&connection, "2010-01-01", "2026-10-30", today)
                .expect_err("oversized range should fail")
                .field,
            Some("fromDate")
        );
    }
}
