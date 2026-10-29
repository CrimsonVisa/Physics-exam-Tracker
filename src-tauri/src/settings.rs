use chrono::NaiveDate;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError};

pub const DEFAULT_EXAM_DATE: &str = "2026-12-19";
pub const DEFAULT_DAILY_STUDY_TARGET_MINUTES: i64 = 240;
pub const MIN_DAILY_STUDY_TARGET_MINUTES: i64 = 1;
pub const MAX_DAILY_STUDY_TARGET_MINUTES: i64 = 1440;

const SELECT_SETTINGS: &str = "SELECT exam_date, preparation_start_date, daily_study_target_minutes \
     FROM settings WHERE id = 1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub exam_date: String,
    pub preparation_start_date: String,
    pub daily_study_target_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsInput {
    pub exam_date: String,
    pub preparation_start_date: String,
    pub daily_study_target_minutes: i64,
}

struct ValidatedSettings {
    exam_date: NaiveDate,
    preparation_start_date: NaiveDate,
    daily_study_target_minutes: i64,
}

impl ValidatedSettings {
    fn into_input(self) -> SettingsInput {
        SettingsInput {
            exam_date: date_only::format_date_only(self.exam_date),
            preparation_start_date: date_only::format_date_only(self.preparation_start_date),
            daily_study_target_minutes: self.daily_study_target_minutes,
        }
    }
}

fn validate(input: &SettingsInput) -> Result<ValidatedSettings, AppError> {
    let exam_date = date_only::parse_date_only(&input.exam_date).ok_or_else(|| {
        AppError::validation(
            "examDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    let preparation_start_date = date_only::parse_date_only(&input.preparation_start_date)
        .ok_or_else(|| {
            AppError::validation(
                "preparationStartDate",
                "Enter a real calendar date in YYYY-MM-DD format.",
            )
        })?;

    if preparation_start_date > exam_date {
        return Err(AppError::validation(
            "preparationStartDate",
            "Preparation start date cannot be after the exam date.",
        ));
    }

    if !(MIN_DAILY_STUDY_TARGET_MINUTES..=MAX_DAILY_STUDY_TARGET_MINUTES)
        .contains(&input.daily_study_target_minutes)
    {
        return Err(AppError::validation(
            "dailyStudyTargetMinutes",
            "Daily study target must be between 1 and 1440 minutes.",
        ));
    }

    Ok(ValidatedSettings {
        exam_date,
        preparation_start_date,
        daily_study_target_minutes: input.daily_study_target_minutes,
    })
}

pub fn load(connection: &Connection) -> Result<Settings, AppError> {
    match connection.query_row(SELECT_SETTINGS, [], |row| {
        Ok(Settings {
            exam_date: row.get(0)?,
            preparation_start_date: row.get(1)?,
            daily_study_target_minutes: row.get(2)?,
        })
    }) {
        Ok(settings) => Ok(settings),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(AppError::settings_not_found()),
        Err(error) => Err(AppError::database(
            "Settings could not be loaded from the local database",
            error,
        )),
    }
}

/// Create the initial settings row without ever overwriting an existing one.
pub fn ensure_defaults(connection: &mut Connection, today: NaiveDate) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The settings transaction could not be started", error)
    })?;

    let exists: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE id = 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Existing settings could not be checked", error))?;

    if exists == 0 {
        let exam_date = date_only::parse_date_only(DEFAULT_EXAM_DATE)
            .ok_or_else(|| AppError::internal("The built-in default exam date is invalid."))?;
        let preparation_start_date = date_only::default_preparation_start(today, exam_date);

        transaction
            .execute(
                "INSERT INTO settings (
                    id,
                    exam_date,
                    preparation_start_date,
                    daily_study_target_minutes,
                    created_at,
                    updated_at
                 ) VALUES (
                    1,
                    ?1,
                    ?2,
                    ?3,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )",
                params![
                    date_only::format_date_only(exam_date),
                    date_only::format_date_only(preparation_start_date),
                    DEFAULT_DAILY_STUDY_TARGET_MINUTES,
                ],
            )
            .map_err(|error| AppError::database("Default settings could not be stored", error))?;
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("Default settings could not be committed", error))
}

pub fn update(connection: &mut Connection, input: SettingsInput) -> Result<Settings, AppError> {
    let validated = validate(&input)?.into_input();
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The settings transaction could not be started", error)
    })?;

    let changed = transaction
        .execute(
            "UPDATE settings
             SET exam_date = ?1,
                 preparation_start_date = ?2,
                 daily_study_target_minutes = ?3,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![
                validated.exam_date,
                validated.preparation_start_date,
                validated.daily_study_target_minutes,
            ],
        )
        .map_err(|error| AppError::database("Settings could not be saved", error))?;

    if changed != 1 {
        return Err(AppError::settings_not_found());
    }

    let settings = load(&transaction)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("Settings could not be committed", error))?;

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn input(exam_date: &str, preparation_start_date: &str, minutes: i64) -> SettingsInput {
        SettingsInput {
            exam_date: exam_date.to_owned(),
            preparation_start_date: preparation_start_date.to_owned(),
            daily_study_target_minutes: minutes,
        }
    }

    fn test_connection() -> Connection {
        db::test_connection_with_defaults(date(2026, 10, 29))
    }

    #[test]
    fn first_run_creates_the_documented_defaults() {
        let mut connection = db::test_connection();

        ensure_defaults(&mut connection, date(2026, 10, 29)).expect("defaults should be created");

        let settings = load(&connection).expect("defaults should load");
        assert_eq!(
            settings,
            Settings {
                exam_date: "2026-12-19".to_owned(),
                preparation_start_date: "2026-10-29".to_owned(),
                daily_study_target_minutes: 240,
            }
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM settings", [], |row| row
                    .get::<_, i64>(0))
                .expect("settings count should be readable"),
            1
        );
    }

    #[test]
    fn ensuring_defaults_never_overwrites_existing_settings() {
        let mut connection = test_connection();
        let saved = update(&mut connection, input("2027-01-10", "2026-11-01", 300))
            .expect("settings should update");

        ensure_defaults(&mut connection, date(2027, 2, 1))
            .expect("ensuring defaults should be idempotent");

        assert_eq!(load(&connection).unwrap(), saved);
    }

    #[test]
    fn update_round_trips_through_the_database() {
        let mut connection = test_connection();
        let saved = update(&mut connection, input("2027-01-10", "2026-11-01", 300))
            .expect("settings should update");

        assert_eq!(saved.exam_date, "2027-01-10");
        assert_eq!(saved.preparation_start_date, "2026-11-01");
        assert_eq!(saved.daily_study_target_minutes, 300);
        assert_eq!(load(&connection).unwrap(), saved);
    }

    #[test]
    fn settings_persist_after_reopening_the_database_file() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join("tracker.sqlite3");
        let saved;

        {
            let mut connection =
                db::open_database(&path, date(2026, 10, 29)).expect("database should open");
            saved = update(&mut connection, input("2027-01-10", "2026-11-01", 315))
                .expect("settings should update");
        }

        let connection = db::open_database(&path, date(2026, 10, 29))
            .expect("the existing database should reopen");

        assert_eq!(load(&connection).unwrap(), saved);
    }

    #[test]
    fn impossible_calendar_dates_are_rejected() {
        let mut connection = test_connection();
        let before = load(&connection).unwrap();

        for invalid_exam_date in ["2026-02-30", "2026-02-29", "2026-04-31", "2026-13-01"] {
            let error = update(&mut connection, input(invalid_exam_date, "2026-01-01", 240))
                .expect_err("invalid exam date should be rejected");

            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some("examDate"));
        }

        let error = update(&mut connection, input("2026-12-19", "2026-11-31", 240))
            .expect_err("invalid preparation date should be rejected");
        assert_eq!(error.field, Some("preparationStartDate"));

        assert_eq!(load(&connection).unwrap(), before);
    }

    #[test]
    fn leap_day_is_valid_when_it_exists_in_the_calendar() {
        let mut connection = test_connection();

        let saved = update(&mut connection, input("2024-02-29", "2024-02-01", 240))
            .expect("a leap day should be accepted");

        assert_eq!(saved.exam_date, "2024-02-29");
    }

    #[test]
    fn preparation_start_cannot_be_after_the_exam() {
        let mut connection = test_connection();
        let before = load(&connection).unwrap();

        let error = update(&mut connection, input("2026-12-19", "2026-12-20", 240))
            .expect_err("a later preparation start should be rejected");

        assert_eq!(error.code, "validation_error");
        assert_eq!(error.field, Some("preparationStartDate"));
        assert_eq!(load(&connection).unwrap(), before);
    }

    #[test]
    fn daily_target_must_be_within_supported_minutes() {
        let mut connection = test_connection();

        for minutes in [0, -1, 1441] {
            let error = update(&mut connection, input("2026-12-19", "2026-10-29", minutes))
                .expect_err("out-of-range target should be rejected");

            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some("dailyStudyTargetMinutes"));
        }
    }

    #[test]
    fn database_checks_reject_invalid_values_written_directly() {
        let connection = test_connection();

        for sql in [
            "UPDATE settings SET exam_date = '2026-02-30' WHERE id = 1",
            "UPDATE settings SET exam_date = '2026-13-01' WHERE id = 1",
            "UPDATE settings SET exam_date = '2026-00-10' WHERE id = 1",
            "UPDATE settings SET preparation_start_date = '2026-11-31' WHERE id = 1",
            "UPDATE settings SET preparation_start_date = '2026-12-20' WHERE id = 1",
            "UPDATE settings SET daily_study_target_minutes = 0 WHERE id = 1",
            "INSERT INTO settings (
                id, exam_date, preparation_start_date, daily_study_target_minutes, created_at, updated_at
             ) VALUES (
                2, '2026-12-19', '2026-10-29', 240, '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z'
             )",
        ] {
            assert!(
                connection.execute(sql, []).is_err(),
                "constraint should reject: {sql}"
            );
        }
    }

    #[test]
    fn rolled_back_transaction_restores_previous_settings() {
        let mut connection = test_connection();
        let before = load(&connection).unwrap();

        {
            let transaction = connection.transaction().expect("transaction should start");
            transaction
                .execute(
                    "UPDATE settings SET daily_study_target_minutes = 300 WHERE id = 1",
                    [],
                )
                .expect("first update should succeed");
            transaction
                .execute(
                    "INSERT INTO settings (
                        id, exam_date, preparation_start_date, daily_study_target_minutes, created_at, updated_at
                     ) VALUES (
                        1, '2026-12-19', '2026-10-29', 240, '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z'
                     )",
                    [],
                )
                .expect_err("the duplicate singleton row should fail");
            transaction
                .rollback()
                .expect("transaction should roll back");
        }

        assert_eq!(load(&connection).unwrap(), before);
    }
}
