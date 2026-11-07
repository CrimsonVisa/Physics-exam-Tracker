use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::{date_only, error::AppError, settings};

pub const CSIR_EXAM_CODE: &str = "csir-net-physics";
#[cfg(test)]
pub const BUILTIN_EXAM_CODES: [&str; 4] = [
    CSIR_EXAM_CODE,
    "gate-physics",
    "tifr-gs-physics",
    "jest-physics",
];

const SELECT_PROFILE: &str = "SELECT
        e.id,
        e.code,
        e.name,
        e.short_name,
        e.authority,
        e.official_url,
        e.display_order,
        e.is_builtin,
        p.is_active,
        p.exam_date,
        p.legacy_date_notice_state,
        p.created_at,
        p.updated_at
     FROM exam_profiles p
     JOIN exams e ON e.id = p.exam_id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamProfile {
    pub exam_id: i64,
    pub code: String,
    pub name: String,
    pub short_name: String,
    pub authority: String,
    pub official_url: String,
    pub display_order: i64,
    pub is_builtin: bool,
    pub is_active: bool,
    pub exam_date: Option<String>,
    pub legacy_date_notice_state: String,
    pub legacy_exam_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list(connection: &Connection) -> Result<Vec<ExamProfile>, AppError> {
    let legacy_date = settings::load(connection)
        .ok()
        .map(|settings| settings.exam_date);
    let mut statement = connection
        .prepare(&format!("{SELECT_PROFILE} ORDER BY e.display_order, e.id"))
        .map_err(|error| AppError::database("Exam profiles could not be prepared", error))?;
    let rows = statement
        .query_map([], map_profile)
        .map_err(|error| AppError::database("Exam profiles could not be queried", error))?;
    let mut profiles = Vec::new();
    for row in rows {
        let mut profile =
            row.map_err(|error| AppError::database("An exam profile could not be read", error))?;
        profile.legacy_exam_date = if profile.code == CSIR_EXAM_CODE
            && matches!(
                profile.legacy_date_notice_state.as_str(),
                "pending" | "dismissed"
            ) {
            legacy_date.clone()
        } else {
            None
        };
        profiles.push(profile);
    }
    Ok(profiles)
}

pub fn load(connection: &Connection, exam_id: i64) -> Result<ExamProfile, AppError> {
    validate_exam_id(exam_id)?;
    let mut profile = connection
        .query_row(
            &format!("{SELECT_PROFILE} WHERE e.id = ?1"),
            [exam_id],
            map_profile,
        )
        .optional()
        .map_err(|error| AppError::database("The exam profile could not be loaded", error))?
        .ok_or_else(|| AppError::validation("examId", "Choose an existing examination."))?;
    profile.legacy_exam_date = legacy_date_for_profile(connection, &profile)?;
    Ok(profile)
}

fn legacy_date_for_profile(
    connection: &Connection,
    profile: &ExamProfile,
) -> Result<Option<String>, AppError> {
    if profile.code != CSIR_EXAM_CODE
        || !matches!(
            profile.legacy_date_notice_state.as_str(),
            "pending" | "dismissed"
        )
    {
        return Ok(None);
    }
    Ok(settings::load(connection).ok().map(|value| value.exam_date))
}

pub fn update_date(
    connection: &mut Connection,
    exam_id: i64,
    exam_date: Option<&str>,
) -> Result<ExamProfile, AppError> {
    let normalized = normalize_optional_date(exam_date)?;
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The exam-date transaction could not start", error))?;
    let profile = load(&transaction, exam_id)?;

    let next_notice_state = if normalized.is_some() && profile.legacy_date_notice_state == "pending"
    {
        "dismissed"
    } else {
        profile.legacy_date_notice_state.as_str()
    };

    transaction
        .execute(
            "UPDATE exam_profiles
             SET exam_date = ?1,
                 legacy_date_notice_state = ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?3",
            params![normalized, next_notice_state, exam_id],
        )
        .map_err(|error| AppError::database("The exam date could not be saved", error))?;

    let updated = load(&transaction, exam_id)?;
    transaction.commit().map_err(|error| {
        AppError::database("The exam-date change could not be committed", error)
    })?;
    Ok(updated)
}

pub fn set_active(
    connection: &mut Connection,
    exam_id: i64,
    is_active: bool,
) -> Result<ExamProfile, AppError> {
    validate_exam_id(exam_id)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The exam-profile transaction could not start", error)
    })?;
    let updated = transaction
        .execute(
            "UPDATE exam_profiles
             SET is_active = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?2",
            params![i64::from(is_active), exam_id],
        )
        .map_err(|error| AppError::database("The exam profile could not be updated", error))?;
    if updated != 1 {
        return Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        ));
    }
    let profile = load(&transaction, exam_id)?;
    transaction.commit().map_err(|error| {
        AppError::database("The exam-profile change could not be committed", error)
    })?;
    Ok(profile)
}

pub fn confirm_legacy_date(
    connection: &mut Connection,
    exam_id: i64,
) -> Result<ExamProfile, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The legacy exam-date transaction could not start", error)
    })?;
    let profile = load(&transaction, exam_id)?;
    if profile.code != CSIR_EXAM_CODE {
        return Err(AppError::validation(
            "examId",
            "Only the CSIR profile has a legacy date available for confirmation.",
        ));
    }
    let legacy = settings::load(&transaction)?;
    let legacy_date = normalize_optional_date(Some(&legacy.exam_date))?.ok_or_else(|| {
        AppError::validation("examDate", "The legacy CSIR date is not available.")
    })?;

    transaction
        .execute(
            "UPDATE exam_profiles
             SET exam_date = ?1,
                 legacy_date_notice_state = 'transferred',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?2",
            params![legacy_date, exam_id],
        )
        .map_err(|error| {
            AppError::database("The legacy exam date could not be confirmed", error)
        })?;

    let updated = load(&transaction, exam_id)?;
    transaction.commit().map_err(|error| {
        AppError::database(
            "The legacy exam-date confirmation could not be committed",
            error,
        )
    })?;
    Ok(updated)
}

pub fn dismiss_legacy_date_notice(
    connection: &mut Connection,
    exam_id: i64,
) -> Result<ExamProfile, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The legacy-date notice transaction could not start", error)
    })?;
    let profile = load(&transaction, exam_id)?;
    if profile.code != CSIR_EXAM_CODE || profile.legacy_date_notice_state == "none" {
        return Ok(profile);
    }

    transaction
        .execute(
            "UPDATE exam_profiles
             SET legacy_date_notice_state = 'dismissed',
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE exam_id = ?1",
            [exam_id],
        )
        .map_err(|error| {
            AppError::database("The legacy-date notice could not be dismissed", error)
        })?;

    let updated = load(&transaction, exam_id)?;
    transaction.commit().map_err(|error| {
        AppError::database(
            "The legacy-date notice dismissal could not be committed",
            error,
        )
    })?;
    Ok(updated)
}

fn normalize_optional_date(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(
            "examDate",
            "Enter a real calendar date in YYYY-MM-DD format, or clear the date.",
        ));
    }
    let parsed = date_only::parse_date_only(trimmed).ok_or_else(|| {
        AppError::validation(
            "examDate",
            "Enter a real calendar date in YYYY-MM-DD format.",
        )
    })?;
    Ok(Some(date_only::format_date_only(parsed)))
}

fn validate_exam_id(value: i64) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::validation(
            "examId",
            "Choose an existing examination.",
        ));
    }
    Ok(())
}

fn map_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExamProfile> {
    Ok(ExamProfile {
        exam_id: row.get(0)?,
        code: row.get(1)?,
        name: row.get(2)?,
        short_name: row.get(3)?,
        authority: row.get(4)?,
        official_url: row.get(5)?,
        display_order: row.get(6)?,
        is_builtin: row.get::<_, i64>(7)? != 0,
        is_active: row.get::<_, i64>(8)? != 0,
        exam_date: row.get(9)?,
        legacy_date_notice_state: row.get(10)?,
        legacy_exam_date: None,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use chrono::NaiveDate;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn profile_by_code(connection: &Connection, code: &str) -> ExamProfile {
        list(connection)
            .expect("profiles should load")
            .into_iter()
            .find(|profile| profile.code == code)
            .expect("profile should exist")
    }

    #[test]
    fn fresh_install_has_all_four_active_profiles_and_no_dates() {
        let connection = db::test_connection();
        let profiles = list(&connection).expect("profiles should load");

        assert_eq!(profiles.len(), 4);
        assert_eq!(
            profiles.iter().filter(|profile| profile.is_active).count(),
            4
        );
        assert!(profiles.iter().all(|profile| profile.exam_date.is_none()));
        assert!(
            profiles
                .iter()
                .all(|profile| profile.legacy_date_notice_state == "none")
        );
        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.code.as_str())
                .collect::<Vec<_>>(),
            BUILTIN_EXAM_CODES
        );
    }

    #[test]
    fn date_can_be_set_edited_and_cleared() {
        let mut connection = db::test_connection();
        let csir = profile_by_code(&connection, CSIR_EXAM_CODE);

        let saved = update_date(&mut connection, csir.exam_id, Some("2027-03-15"))
            .expect("date should save");
        assert_eq!(saved.exam_date.as_deref(), Some("2027-03-15"));

        let edited = update_date(&mut connection, csir.exam_id, Some("2027-04-20"))
            .expect("date should edit");
        assert_eq!(edited.exam_date.as_deref(), Some("2027-04-20"));

        let cleared = update_date(&mut connection, csir.exam_id, None).expect("date should clear");
        assert_eq!(cleared.exam_date, None);
    }

    #[test]
    fn invalid_date_is_rejected() {
        let mut connection = db::test_connection();
        let csir = profile_by_code(&connection, CSIR_EXAM_CODE);

        for invalid in ["2026-02-30", "2026-13-01", "not-a-date", ""] {
            let error = update_date(&mut connection, csir.exam_id, Some(invalid))
                .expect_err("invalid date should fail");
            assert_eq!(error.field, Some("examDate"));
        }
    }

    #[test]
    fn activation_is_reversible_and_does_not_delete_history() {
        let mut connection = db::test_connection_with_defaults(date(2026, 11, 2));
        let csir = profile_by_code(&connection, CSIR_EXAM_CODE);
        connection
            .execute(
                "INSERT INTO study_sessions (
                    topic_id, session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES (
                    NULL, '2026-11-02', 1800, 'manual', 'preserve me',
                    '2026-11-02T00:00:00Z', '2026-11-02T00:00:00Z'
                 )",
                [],
            )
            .expect("session should save");

        let deactivated =
            set_active(&mut connection, csir.exam_id, false).expect("profile should deactivate");
        assert!(!deactivated.is_active);
        let session_count: i64 = connection
            .query_row("SELECT count(*) FROM study_sessions", [], |row| row.get(0))
            .expect("sessions should count");
        assert_eq!(session_count, 1);

        let reactivated =
            set_active(&mut connection, csir.exam_id, true).expect("profile should reactivate");
        assert!(reactivated.is_active);
        assert_eq!(
            profile_by_code(&connection, CSIR_EXAM_CODE).exam_date,
            csir.exam_date
        );
    }

    #[test]
    fn legacy_date_is_confirmed_or_dismissed_without_changing_settings() {
        let mut connection = db::test_connection();
        settings::ensure_defaults(&mut connection, date(2026, 11, 2))
            .expect("settings should seed");
        connection
            .execute(
                "UPDATE exam_profiles
                 SET legacy_date_notice_state = 'pending'
                 WHERE exam_id = (SELECT id FROM exams WHERE code = ?1)",
                [CSIR_EXAM_CODE],
            )
            .expect("legacy notice should seed");
        let legacy = settings::load(&connection).expect("settings should load");
        let csir = profile_by_code(&connection, CSIR_EXAM_CODE);
        assert_eq!(csir.exam_date, None);
        assert_eq!(
            csir.legacy_exam_date.as_deref(),
            Some(legacy.exam_date.as_str())
        );

        let confirmed =
            confirm_legacy_date(&mut connection, csir.exam_id).expect("legacy date should confirm");
        assert_eq!(
            confirmed.exam_date.as_deref(),
            Some(legacy.exam_date.as_str())
        );
        assert_eq!(confirmed.legacy_date_notice_state, "transferred");
        assert_eq!(
            settings::load(&connection)
                .expect("settings should remain")
                .exam_date,
            legacy.exam_date
        );
    }

    #[test]
    fn dismissing_notice_preserves_legacy_value() {
        let mut connection = db::test_connection();
        settings::ensure_defaults(&mut connection, date(2026, 11, 2))
            .expect("settings should seed");
        connection
            .execute(
                "UPDATE exam_profiles
                 SET legacy_date_notice_state = 'pending'
                 WHERE exam_id = (SELECT id FROM exams WHERE code = ?1)",
                [CSIR_EXAM_CODE],
            )
            .expect("legacy notice should seed");
        let legacy = settings::load(&connection).expect("settings should load");
        let csir = profile_by_code(&connection, CSIR_EXAM_CODE);

        let dismissed = dismiss_legacy_date_notice(&mut connection, csir.exam_id)
            .expect("notice should dismiss");
        assert_eq!(dismissed.exam_date, None);
        assert_eq!(dismissed.legacy_date_notice_state, "dismissed");
        assert_eq!(
            settings::load(&connection)
                .expect("settings should remain")
                .exam_date,
            legacy.exam_date
        );
    }
}
