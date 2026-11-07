use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AppSettings {
    pub preparation_start_date: Option<String>,
    pub daily_study_target_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2UpdateAppSettingsInput {
    #[serde(default)]
    pub preparation_start_date: Option<String>,
    pub daily_study_target_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamProfile {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub short_name: String,
    pub authority: String,
    pub official_url: String,
    pub display_order: i64,
    pub is_active: bool,
    pub exam_date: Option<String>,
    pub date_updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamDateChangeInput {
    pub exam_id: i64,
    #[serde(default)]
    pub exam_date: Option<String>,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AffectedAnchoredTask {
    pub id: i64,
    pub title: String,
    pub current_date: String,
    pub new_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamDateChangePreview {
    pub exam_id: i64,
    pub old_date: Option<String>,
    pub new_date: Option<String>,
    pub affected_tasks: Vec<V2AffectedAnchoredTask>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamActivationInput {
    pub exam_id: i64,
    pub is_active: bool,
}

pub fn load_settings(connection: &Connection) -> Result<V2AppSettings, AppError> {
    connection
        .query_row(
            "SELECT preparation_start_date, daily_study_target_minutes
             FROM settings WHERE id = 1",
            [],
            |row| {
                Ok(V2AppSettings {
                    preparation_start_date: row.get(0)?,
                    daily_study_target_minutes: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("V2 application settings could not be loaded", error))?
        .ok_or_else(AppError::settings_not_found)
}

pub fn update_settings(
    connection: &mut Connection,
    input: V2UpdateAppSettingsInput,
    now: DateTime<Utc>,
) -> Result<V2AppSettings, AppError> {
    if !(1..=1_440).contains(&input.daily_study_target_minutes) {
        return Err(AppError::validation(
            "dailyStudyTargetMinutes",
            "Daily study target must be between 1 and 1,440 minutes.",
        ));
    }
    let preparation_start = match input.preparation_start_date.as_deref() {
        Some(value) if !value.trim().is_empty() => {
            Some(date_only::parse_date_only(value.trim()).ok_or_else(|| {
                AppError::validation(
                    "preparationStartDate",
                    "Enter a real calendar date in YYYY-MM-DD format.",
                )
            })?)
        }
        _ => None,
    };
    if let Some(start) = preparation_start {
        let first_exam: Option<String> = connection
            .query_row(
                "SELECT MIN(exam_date) FROM exam_profiles WHERE is_active = 1 AND exam_date IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Exam dates could not be checked", error))?;
        if let Some(first_exam) = first_exam.and_then(|value| date_only::parse_date_only(&value))
            && start > first_exam
        {
            return Err(AppError::validation(
                "preparationStartDate",
                "Preparation start cannot be after the earliest configured exam date.",
            ));
        }
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let start_text = preparation_start.map(date_only::format_date_only);
    connection
        .execute(
            "UPDATE settings
             SET preparation_start_date = ?1,
                 daily_study_target_minutes = ?2,
                 updated_at = ?3
             WHERE id = 1",
            params![start_text, input.daily_study_target_minutes, now_text,],
        )
        .map_err(|error| AppError::database("V2 application settings could not be saved", error))?;
    load_settings(connection)
}

pub fn list_exam_profiles(connection: &Connection) -> Result<Vec<V2ExamProfile>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.code, e.name, e.short_name, e.authority, e.official_url,
                    e.display_order, p.is_active, p.exam_date, p.date_updated_at
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("Exam profiles could not be prepared", error))?;
    let rows = statement
        .query_map([], |row| {
            Ok(V2ExamProfile {
                id: row.get(0)?,
                code: row.get(1)?,
                name: row.get(2)?,
                short_name: row.get(3)?,
                authority: row.get(4)?,
                official_url: row.get(5)?,
                display_order: row.get(6)?,
                is_active: row.get::<_, i64>(7)? == 1,
                exam_date: row.get(8)?,
                date_updated_at: row.get(9)?,
            })
        })
        .map_err(|error| AppError::database("Exam profiles could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("An exam profile row is invalid", error))
}

pub fn set_exam_active(
    connection: &mut Connection,
    input: V2ExamActivationInput,
    now: DateTime<Utc>,
) -> Result<V2ExamProfile, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The exam activation transaction could not start", error)
    })?;
    let exists: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
            [input.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Exam could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "examId",
            "The selected examination does not exist.",
        ));
    }
    if !input.is_active {
        let active_timer: i64 = transaction
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM timer_state
                    WHERE id = 1 AND status <> 'idle' AND active_exam_id = ?1
                 )",
                [input.exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Active timer could not be checked", error))?;
        if active_timer == 1 {
            return Err(AppError::validation(
                "examId",
                "Finish or discard the active timer before deactivating its examination.",
            ));
        }
    }
    transaction
        .execute(
            "UPDATE exam_profiles
             SET is_active = ?1, updated_at = ?2
             WHERE exam_id = ?3",
            params![
                i64::from(input.is_active),
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
                input.exam_id
            ],
        )
        .map_err(|error| AppError::database("Exam activation could not be saved", error))?;
    transaction
        .commit()
        .map_err(|error| AppError::database("Exam activation could not be committed", error))?;
    exam_profile(connection, input.exam_id)
}

pub fn preview_exam_date_change(
    connection: &Connection,
    input: V2ExamDateChangeInput,
) -> Result<V2ExamDateChangePreview, AppError> {
    let (old_date, config_version) = exam_date_state(connection, input.exam_id)?;
    let new_date = validate_exam_date(input.exam_date.as_deref())?;
    let affected = anchored_tasks(connection, input.exam_id, new_date.as_deref())?;
    let _ = config_version;
    Ok(V2ExamDateChangePreview {
        exam_id: input.exam_id,
        old_date,
        new_date,
        affected_tasks: affected,
    })
}

pub fn apply_exam_date_change(
    connection: &mut Connection,
    input: V2ExamDateChangeInput,
    now: DateTime<Utc>,
) -> Result<V2ExamProfile, AppError> {
    let preview = preview_exam_date_change(connection, input.clone())?;
    if preview.new_date.is_none() && !preview.affected_tasks.is_empty() {
        return Err(AppError::validation(
            "examDate",
            "Clear or reschedule exam-anchored tasks before removing the exam date.",
        ));
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The exam date transaction could not start", error))?;
    transaction
        .execute(
            "UPDATE exam_profiles
             SET exam_date = ?1, date_updated_at = ?2, updated_at = ?2
             WHERE exam_id = ?3",
            params![preview.new_date, now_text, input.exam_id],
        )
        .map_err(|error| AppError::database("Exam date could not be saved", error))?;
    transaction
        .execute(
            "INSERT INTO exam_date_changes (exam_id, old_date, new_date, changed_at, reason)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.exam_id,
                preview.old_date,
                preview.new_date,
                now_text,
                input.reason.trim(),
            ],
        )
        .map_err(|error| AppError::database("Exam date history could not be saved", error))?;
    for task in &preview.affected_tasks {
        transaction
            .execute(
                "UPDATE checklist_tasks
                 SET scheduled_date = ?1, updated_at = ?2
                 WHERE id = ?3 AND schedule_anchor = 'exam'",
                params![task.new_date, now_text, task.id],
            )
            .map_err(|error| {
                AppError::database("Exam-anchored task could not be rescheduled", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Exam date change could not be committed", error))?;
    exam_profile(connection, input.exam_id)
}

pub fn exam_profile(connection: &Connection, exam_id: i64) -> Result<V2ExamProfile, AppError> {
    list_exam_profiles(connection)?
        .into_iter()
        .find(|exam| exam.id == exam_id)
        .ok_or_else(|| AppError::validation("examId", "The selected examination does not exist."))
}

fn exam_date_state(
    connection: &Connection,
    exam_id: i64,
) -> Result<(Option<String>, i64), AppError> {
    connection
        .query_row(
            "SELECT p.exam_date, m.exam_config_version
             FROM exam_profiles p
             JOIN exams e ON e.id = p.exam_id
             LEFT JOIN app_metadata m ON m.id = 1
             WHERE p.exam_id = ?1",
            [exam_id],
            |row| Ok((row.get(0)?, row.get::<_, Option<i64>>(1)?.unwrap_or(1))),
        )
        .optional()
        .map_err(|error| AppError::database("Exam date could not be read", error))?
        .ok_or_else(|| AppError::validation("examId", "The selected examination does not exist."))
}

fn validate_exam_date(value: Option<&str>) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => {
            let date = date_only::parse_date_only(value.trim()).ok_or_else(|| {
                AppError::validation(
                    "examDate",
                    "Enter a real calendar date in YYYY-MM-DD format.",
                )
            })?;
            Ok(Some(date_only::format_date_only(date)))
        }
    }
}

fn anchored_tasks(
    connection: &Connection,
    exam_id: i64,
    new_date: Option<&str>,
) -> Result<Vec<V2AffectedAnchoredTask>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, scheduled_date, schedule_offset_days
             FROM checklist_tasks
             WHERE exam_id = ?1 AND schedule_anchor = 'exam'
             ORDER BY scheduled_date, id",
        )
        .map_err(|error| AppError::database("Anchored tasks could not be prepared", error))?;
    let rows = statement
        .query_map([exam_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| AppError::database("Anchored tasks could not be read", error))?;
    let mut tasks = Vec::new();
    for row in rows {
        let (id, title, current_date, offset) =
            row.map_err(|error| AppError::database("An anchored task row is invalid", error))?;
        let new_date = new_date
            .and_then(date_only::parse_date_only)
            .and_then(|date| date.checked_add_signed(chrono::Duration::days(offset)))
            .map(date_only::format_date_only)
            .unwrap_or_else(|| current_date.clone());
        if new_date != current_date {
            tasks.push(V2AffectedAnchoredTask {
                id,
                title,
                current_date,
                new_date,
            });
        }
    }
    Ok(tasks)
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
        DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
            + chrono::Duration::seconds(seconds)
    }

    #[test]
    fn application_settings_round_trip_and_validate() {
        let (_root, state) = state();
        let initial = state
            .with_connection(|connection| load_settings(connection))
            .expect("settings");
        assert_eq!(initial.daily_study_target_minutes, 120);
        assert_eq!(initial.preparation_start_date, None);

        let updated = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2UpdateAppSettingsInput {
                        preparation_start_date: Some("2026-10-01".to_owned()),
                        daily_study_target_minutes: 180,
                    },
                    at(0),
                )
            })
            .expect("updated settings");
        assert_eq!(updated.daily_study_target_minutes, 180);
        assert_eq!(
            updated.preparation_start_date.as_deref(),
            Some("2026-10-01")
        );

        let invalid = state
            .with_connection(|connection| {
                update_settings(
                    connection,
                    V2UpdateAppSettingsInput {
                        preparation_start_date: None,
                        daily_study_target_minutes: 0,
                    },
                    at(1),
                )
            })
            .expect_err("invalid target");
        assert_eq!(invalid.field, Some("dailyStudyTargetMinutes"));
    }

    #[test]
    fn exam_date_changes_update_anchors_and_history() {
        let (_root, state) = state();
        let profile = state
            .with_connection(|connection| {
                apply_exam_date_change(
                    connection,
                    V2ExamDateChangeInput {
                        exam_id: 1,
                        exam_date: Some("2026-12-20".to_owned()),
                        reason: "official notice".to_owned(),
                    },
                    at(0),
                )
            })
            .expect("exam date");
        assert_eq!(profile.exam_date.as_deref(), Some("2026-12-20"));

        let task_id = state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO checklist_tasks (
                            title, description, scheduled_date, deadline_date, exam_id,
                            priority, status, schedule_anchor, schedule_offset_days,
                            completed_at, created_at, updated_at
                         ) VALUES ('Revision', '', '2026-12-15', NULL, 1,
                                   'normal', 'open', 'exam', -5, NULL, ?1, ?1)",
                        ["2026-11-04T10:00:00Z"],
                    )
                    .map_err(|error| AppError::database("test task", error))?;
                Ok::<_, AppError>(connection.last_insert_rowid())
            })
            .expect("task");

        let preview = state
            .with_connection(|connection| {
                preview_exam_date_change(
                    connection,
                    V2ExamDateChangeInput {
                        exam_id: 1,
                        exam_date: Some("2026-12-25".to_owned()),
                        reason: String::new(),
                    },
                )
            })
            .expect("preview");
        assert_eq!(preview.affected_tasks.len(), 1);
        assert_eq!(preview.affected_tasks[0].new_date, "2026-12-20");

        state
            .with_connection(|connection| {
                apply_exam_date_change(
                    connection,
                    V2ExamDateChangeInput {
                        exam_id: 1,
                        exam_date: Some("2026-12-25".to_owned()),
                        reason: "changed".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("apply");
        let scheduled: String = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT scheduled_date FROM checklist_tasks WHERE id = ?1",
                        [task_id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("test task date", error))
            })
            .expect("date");
        assert_eq!(scheduled, "2026-12-20");
        let history: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_date_changes WHERE exam_id = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("test history", error))
            })
            .expect("history");
        assert_eq!(history, 2);
    }

    #[test]
    fn activation_is_reversible() {
        let (_root, state) = state();
        let inactive = state
            .with_connection(|connection| {
                set_exam_active(
                    connection,
                    V2ExamActivationInput {
                        exam_id: 2,
                        is_active: false,
                    },
                    at(0),
                )
            })
            .expect("deactivate");
        assert!(!inactive.is_active);
        let active = state
            .with_connection(|connection| {
                set_exam_active(
                    connection,
                    V2ExamActivationInput {
                        exam_id: 2,
                        is_active: true,
                    },
                    at(1),
                )
            })
            .expect("reactivate");
        assert!(active.is_active);
    }
}
