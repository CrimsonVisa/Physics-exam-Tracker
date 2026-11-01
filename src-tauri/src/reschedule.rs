use std::collections::{HashMap, HashSet};

use chrono::{Days, NaiveDate};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError, settings};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDateChange {
    pub task_id: i64,
    pub title: String,
    pub current_scheduled_date: String,
    pub proposed_scheduled_date: String,
    pub offset_days: i64,
    pub outside_window: bool,
    pub deadline_date: Option<String>,
    pub deadline_conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutsideWindowItem {
    pub kind: String,
    pub id: i64,
    pub title: String,
    pub date: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamReschedulePreview {
    pub current_exam_date: String,
    pub proposed_exam_date: String,
    pub proposed_preparation_start_date: String,
    pub proposed_daily_study_target_minutes: i64,
    pub changes: Vec<TaskDateChange>,
    pub outside_window: Vec<OutsideWindowItem>,
    pub completed_tasks_untouched: i64,
    pub manual_tasks_untouched: i64,
    pub planned_blocks_untouched: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedTaskChange {
    pub task_id: i64,
    pub expected_scheduled_date: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyExamDateChangeInput {
    pub settings: settings::SettingsInput,
    pub expected_exam_date: String,
    #[serde(default)]
    pub accepted_changes: Vec<AcceptedTaskChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyExamDateChangeResult {
    pub settings: settings::Settings,
    pub applied_task_count: i64,
    pub preserved_outside_window_count: i64,
}

pub fn preview(
    connection: &Connection,
    input: settings::SettingsInput,
    today: NaiveDate,
) -> Result<ExamReschedulePreview, AppError> {
    let validated = settings::validate(&input)?;
    let current = settings::load(connection)?;

    let changes = proposal_changes(
        connection,
        validated.exam_date,
        validated.preparation_start_date,
        today,
    )?;
    let outside_window = outside_window_items(
        connection,
        validated.exam_date,
        validated.preparation_start_date,
        &changes,
    )?;

    Ok(ExamReschedulePreview {
        current_exam_date: current.exam_date,
        proposed_exam_date: date_only::format_date_only(validated.exam_date),
        proposed_preparation_start_date: date_only::format_date_only(
            validated.preparation_start_date,
        ),
        proposed_daily_study_target_minutes: validated.daily_study_target_minutes,
        changes,
        outside_window,
        completed_tasks_untouched: count_tasks(connection, "status = 'completed'")?,
        manual_tasks_untouched: count_tasks(
            connection,
            "status = 'open' AND schedule_anchor = 'manual'",
        )?,
        planned_blocks_untouched: count_blocks(connection)?,
    })
}

pub fn apply(
    connection: &mut Connection,
    input: ApplyExamDateChangeInput,
    today: NaiveDate,
) -> Result<ApplyExamDateChangeResult, AppError> {
    let validated = settings::validate(&input.settings)?;
    let expected_exam_date =
        date_only::parse_date_only(&input.expected_exam_date).ok_or_else(|| {
            AppError::validation(
                "expectedExamDate",
                "The preview's original exam date is invalid. Refresh and preview again.",
            )
        })?;
    let expected_exam_date = date_only::format_date_only(expected_exam_date);

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The exam-date transaction could not be started", error)
    })?;

    let current = settings::load(&transaction)?;
    if current.exam_date != expected_exam_date {
        return Err(AppError::reschedule_stale());
    }

    let changes = proposal_changes(
        &transaction,
        validated.exam_date,
        validated.preparation_start_date,
        today,
    )?;
    let change_map: HashMap<i64, &TaskDateChange> = changes
        .iter()
        .map(|change| (change.task_id, change))
        .collect();

    if let Some(conflict) = changes.iter().find(|change| change.deadline_conflict) {
        return Err(AppError::validation(
            "acceptedChanges",
            format!(
                "The exam-date change would move \"{}\" past its deadline. Resolve that deadline before applying the new exam date.",
                conflict.title
            ),
        ));
    }

    let mut accepted_ids = HashSet::new();
    for accepted in &input.accepted_changes {
        if !accepted_ids.insert(accepted.task_id) {
            return Err(AppError::reschedule_stale());
        }

        let change = change_map
            .get(&accepted.task_id)
            .ok_or_else(AppError::reschedule_stale)?;

        if accepted.expected_scheduled_date != change.current_scheduled_date {
            return Err(AppError::reschedule_stale());
        }

        let changed = transaction
            .execute(
                "UPDATE checklist_tasks
                 SET scheduled_date = ?1,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?2
                   AND status = 'open'
                   AND schedule_anchor = 'exam'
                   AND scheduled_date = ?3
                   AND (deadline_date IS NULL OR deadline_date >= ?4)",
                params![
                    change.proposed_scheduled_date,
                    change.task_id,
                    change.current_scheduled_date,
                    change.proposed_scheduled_date,
                ],
            )
            .map_err(|error| {
                AppError::database("An exam-anchored task could not be rescheduled", error)
            })?;

        if changed != 1 {
            if change.deadline_conflict {
                return Err(AppError::validation(
                    "acceptedChanges",
                    format!(
                        "The exam-date change would move \"{}\" past its deadline.",
                        change.title
                    ),
                ));
            }
            return Err(AppError::reschedule_stale());
        }
    }

    if accepted_ids.len() != change_map.len() {
        return Err(AppError::validation(
            "acceptedChanges",
            "Accept every previewed exam-anchored task change before applying the new exam date.",
        ));
    }

    let changed = transaction
        .execute(
            "UPDATE settings
             SET exam_date = ?1,
                 preparation_start_date = ?2,
                 daily_study_target_minutes = ?3,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = 1",
            params![
                date_only::format_date_only(validated.exam_date),
                date_only::format_date_only(validated.preparation_start_date),
                validated.daily_study_target_minutes,
            ],
        )
        .map_err(|error| AppError::database("The exam date could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::settings_not_found());
    }

    let outside_count = outside_window_items(
        &transaction,
        validated.exam_date,
        validated.preparation_start_date,
        &[],
    )?
    .len() as i64;
    let settings = settings::load(&transaction)?;

    transaction.commit().map_err(|error| {
        AppError::database("The exam-date change could not be committed", error)
    })?;

    Ok(ApplyExamDateChangeResult {
        settings,
        applied_task_count: accepted_ids.len() as i64,
        preserved_outside_window_count: outside_count,
    })
}

fn proposal_changes(
    connection: &Connection,
    new_exam_date: NaiveDate,
    new_preparation_start_date: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<TaskDateChange>, AppError> {
    let today_text = date_only::format_date_only(today);
    let mut statement = connection
        .prepare(
            "SELECT id, title, scheduled_date, schedule_offset_days, deadline_date
             FROM checklist_tasks
             WHERE status = 'open' AND schedule_anchor = 'exam'
             ORDER BY id",
        )
        .map_err(|error| AppError::database("Exam-anchored tasks could not be prepared", error))?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|error| AppError::database("Exam-anchored tasks could not be queried", error))?;

    let mut changes = Vec::new();
    for row in rows {
        let (task_id, title, current_scheduled_date, offset, deadline_date) =
            row.map_err(|error| AppError::database("An anchored task could not be read", error))?;

        if current_scheduled_date < today_text {
            continue;
        }

        let offset = offset.ok_or_else(|| {
            AppError::internal("An exam-anchored task is missing its schedule offset.")
        })?;
        let proposed = add_days(new_exam_date, offset).ok_or_else(|| {
            AppError::validation(
                "examDate",
                "This exam date would move an anchored task outside the supported calendar.",
            )
        })?;
        let proposed_scheduled_date = date_only::format_date_only(proposed);

        if proposed_scheduled_date == current_scheduled_date {
            continue;
        }

        let deadline_conflict = deadline_date
            .as_deref()
            .is_some_and(|deadline| proposed_scheduled_date.as_str() > deadline);

        changes.push(TaskDateChange {
            task_id,
            title,
            current_scheduled_date,
            proposed_scheduled_date,
            offset_days: offset,
            outside_window: proposed < new_preparation_start_date || proposed > new_exam_date,
            deadline_date,
            deadline_conflict,
        });
    }

    Ok(changes)
}

fn outside_window_items(
    connection: &Connection,
    new_exam_date: NaiveDate,
    new_preparation_start_date: NaiveDate,
    changes: &[TaskDateChange],
) -> Result<Vec<OutsideWindowItem>, AppError> {
    let changed_ids: HashSet<i64> = changes.iter().map(|change| change.task_id).collect();
    let new_exam_text = date_only::format_date_only(new_exam_date);
    let new_start_text = date_only::format_date_only(new_preparation_start_date);
    let mut items = Vec::new();

    let mut task_statement = connection
        .prepare(
            "SELECT id, title, scheduled_date, deadline_date
             FROM checklist_tasks
             WHERE status = 'open'
             ORDER BY id",
        )
        .map_err(|error| AppError::database("Open checklist tasks could not be prepared", error))?;
    let tasks = task_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|error| AppError::database("Open checklist tasks could not be queried", error))?;

    for task in tasks {
        let (id, title, scheduled_date, deadline_date) =
            task.map_err(|error| AppError::database("A checklist task could not be read", error))?;

        if !changed_ids.contains(&id)
            && let Some((date, reason)) = outside_date(
                &scheduled_date,
                &new_exam_text,
                &new_start_text,
                "after_exam_date",
                "before_preparation_start",
            )
        {
            items.push(OutsideWindowItem {
                kind: "task".to_owned(),
                id,
                title: title.clone(),
                date,
                reason,
            });
        }

        if let Some(deadline) = deadline_date
            && let Some((date, reason)) = outside_date(
                &deadline,
                &new_exam_text,
                &new_start_text,
                "deadline_after_exam_date",
                "deadline_before_preparation_start",
            )
        {
            items.push(OutsideWindowItem {
                kind: "task_deadline".to_owned(),
                id,
                title,
                date,
                reason,
            });
        }
    }

    let mut block_statement = connection
        .prepare(
            "SELECT id, title, calendar_date
             FROM planned_study_blocks
             WHERE status = 'planned'
             ORDER BY id",
        )
        .map_err(|error| AppError::database("Planned study blocks could not be prepared", error))?;
    let blocks = block_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Planned study blocks could not be queried", error))?;

    for block in blocks {
        let (id, title, calendar_date) = block
            .map_err(|error| AppError::database("A planned block could not be read", error))?;
        if let Some((date, reason)) = outside_date(
            &calendar_date,
            &new_exam_text,
            &new_start_text,
            "block_after_exam_date",
            "block_before_preparation_start",
        ) {
            items.push(OutsideWindowItem {
                kind: "planned_block".to_owned(),
                id,
                title,
                date,
                reason,
            });
        }
    }

    Ok(items)
}

fn outside_date(
    date: &str,
    exam_date: &str,
    preparation_start_date: &str,
    after_reason: &str,
    before_reason: &str,
) -> Option<(String, String)> {
    if date > exam_date {
        Some((date.to_owned(), after_reason.to_owned()))
    } else if date < preparation_start_date {
        Some((date.to_owned(), before_reason.to_owned()))
    } else {
        None
    }
}

fn count_tasks(connection: &Connection, predicate: &str) -> Result<i64, AppError> {
    let sql = format!("SELECT count(*) FROM checklist_tasks WHERE {predicate}");
    connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(|error| AppError::database("Checklist task counts could not be calculated", error))
}

fn count_blocks(connection: &Connection) -> Result<i64, AppError> {
    connection
        .query_row("SELECT count(*) FROM planned_study_blocks", [], |row| {
            row.get(0)
        })
        .map_err(|error| AppError::database("Planned block counts could not be calculated", error))
}

fn add_days(date: NaiveDate, offset: i64) -> Option<NaiveDate> {
    if offset >= 0 {
        date.checked_add_days(Days::new(offset as u64))
    } else {
        date.checked_sub_days(Days::new((-offset) as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, planner, sessions, syllabus};

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date should be valid")
    }

    fn connection() -> Connection {
        let mut connection = db::test_connection_with_defaults(date(2026, 10, 30));
        syllabus::seed(&mut connection).expect("syllabus should be seeded");
        connection
    }

    fn settings_input(exam_date: &str, preparation_start_date: &str) -> settings::SettingsInput {
        settings::SettingsInput {
            exam_date: exam_date.to_owned(),
            preparation_start_date: preparation_start_date.to_owned(),
            daily_study_target_minutes: 300,
        }
    }

    fn anchored_input(title: &str, offset: i64) -> planner::TaskInput {
        planner::TaskInput {
            title: title.to_owned(),
            description: String::new(),
            scheduled_date: String::new(),
            deadline_date: None,
            topic_id: None,
            priority: "Normal".to_owned(),
            schedule_anchor: "exam".to_owned(),
            schedule_offset_days: Some(offset),
        }
    }

    fn manual_input(title: &str, scheduled_date: &str) -> planner::TaskInput {
        planner::TaskInput {
            title: title.to_owned(),
            description: String::new(),
            scheduled_date: scheduled_date.to_owned(),
            deadline_date: None,
            topic_id: None,
            priority: "Normal".to_owned(),
            schedule_anchor: "manual".to_owned(),
            schedule_offset_days: None,
        }
    }

    fn block_input(title: &str, calendar_date: &str) -> planner::PlannedBlockInput {
        planner::PlannedBlockInput {
            title: title.to_owned(),
            topic_id: None,
            calendar_date: calendar_date.to_owned(),
            start_time: "10:00".to_owned(),
            end_time: "11:30".to_owned(),
            status: "planned".to_owned(),
            notes: String::new(),
            linked_session_id: None,
        }
    }

    #[test]
    fn preview_recalculates_future_anchors_and_warns_about_outside_items() {
        let mut connection = connection();
        let today = date(2026, 10, 30);

        let future =
            planner::create_task(&mut connection, anchored_input("Future anchor", -10), today)
                .expect("future anchor should save");
        let completed = planner::create_task(
            &mut connection,
            anchored_input("Completed anchor", -5),
            today,
        )
        .expect("completed anchor should save");
        planner::complete_task(&mut connection, completed.id, today)
            .expect("anchor should complete");
        let manual = planner::create_task(
            &mut connection,
            manual_input("Manual after exam", "2027-01-20"),
            today,
        )
        .expect("manual task should save");
        let past = planner::create_task(&mut connection, anchored_input("Past anchor", -80), today)
            .expect("past anchor should save");
        let block =
            planner::create_block(&mut connection, block_input("Outside block", "2027-02-01"))
                .expect("block should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("preview should load");

        assert_eq!(preview.current_exam_date, "2026-12-19");
        assert_eq!(preview.changes.len(), 1);
        assert_eq!(preview.changes[0].task_id, future.id);
        assert_eq!(preview.changes[0].current_scheduled_date, "2026-12-09");
        assert_eq!(preview.changes[0].proposed_scheduled_date, "2027-01-05");
        assert!(!preview.changes[0].outside_window);
        assert_eq!(preview.completed_tasks_untouched, 1);
        assert_eq!(preview.manual_tasks_untouched, 1);
        assert_eq!(preview.planned_blocks_untouched, 1);

        assert!(preview.outside_window.iter().any(|item| {
            item.kind == "task" && item.id == manual.id && item.reason == "after_exam_date"
        }));
        assert!(preview.outside_window.iter().any(|item| {
            item.kind == "task" && item.id == past.id && item.reason == "before_preparation_start"
        }));
        assert!(
            preview
                .outside_window
                .iter()
                .any(|item| { item.kind == "planned_block" && item.id == block.id })
        );

        let settings_after = settings::load(&connection).expect("settings should load");
        assert_eq!(settings_after.exam_date, "2026-12-19");
        assert_eq!(
            planner::load_task(&connection, future.id, today)
                .expect("task should reload")
                .scheduled_date,
            "2026-12-09"
        );
        assert_eq!(
            planner::load_task(&connection, completed.id, today)
                .expect("completed task should reload")
                .scheduled_date,
            "2026-12-14"
        );
    }

    #[test]
    fn exam_date_changes_reject_anchored_tasks_that_would_pass_their_deadline() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let mut input = anchored_input("Deadline anchor", -10);
        input.deadline_date = Some("2026-12-31".to_owned());
        let task = planner::create_task(&mut connection, input, today).expect("task should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-19", "2026-12-01"),
            today,
        )
        .expect("preview should load");

        let change = preview
            .changes
            .iter()
            .find(|change| change.task_id == task.id)
            .expect("deadline task should move");
        assert_eq!(change.proposed_scheduled_date, "2027-01-09");
        assert_eq!(change.deadline_date.as_deref(), Some("2026-12-31"));
        assert!(change.deadline_conflict);
        assert!(!change.outside_window);
        assert!(
            !preview
                .outside_window
                .iter()
                .any(|item| { item.kind == "task_deadline" && item.id == task.id })
        );

        let accepted = preview
            .changes
            .iter()
            .map(|change| AcceptedTaskChange {
                task_id: change.task_id,
                expected_scheduled_date: change.current_scheduled_date.clone(),
            })
            .collect::<Vec<_>>();
        let error = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-19", "2026-12-01"),
                expected_exam_date: "2026-12-19".to_owned(),
                accepted_changes: accepted,
            },
            today,
        )
        .expect_err("apply should reject the deadline conflict");

        assert_eq!(error.field, Some("acceptedChanges"));
        assert_eq!(
            settings::load(&connection)
                .expect("settings should load")
                .exam_date,
            "2026-12-19"
        );
        assert_eq!(
            planner::load_task(&connection, task.id, today)
                .expect("task should reload")
                .scheduled_date,
            "2026-12-09"
        );
    }

    #[test]
    fn applying_a_preview_updates_settings_and_accepted_tasks_only() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let session = sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: None,
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 1_800,
                notes: "History stays put".to_owned(),
            },
        )
        .expect("session should save");

        let future =
            planner::create_task(&mut connection, anchored_input("Future anchor", -10), today)
                .expect("future anchor should save");
        let completed = planner::create_task(
            &mut connection,
            anchored_input("Completed anchor", -5),
            today,
        )
        .expect("completed anchor should save");
        planner::complete_task(&mut connection, completed.id, today)
            .expect("anchor should complete");
        let manual = planner::create_task(
            &mut connection,
            manual_input("Manual after exam", "2027-01-20"),
            today,
        )
        .expect("manual task should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("preview should load");
        let accepted: Vec<AcceptedTaskChange> = preview
            .changes
            .iter()
            .map(|change| AcceptedTaskChange {
                task_id: change.task_id,
                expected_scheduled_date: change.current_scheduled_date.clone(),
            })
            .collect();

        let result = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-15", "2026-11-01"),
                expected_exam_date: "2026-12-19".to_owned(),
                accepted_changes: accepted,
            },
            today,
        )
        .expect("apply should succeed");

        assert_eq!(result.applied_task_count, 1);
        assert!(result.preserved_outside_window_count >= 1);
        assert_eq!(result.settings.exam_date, "2027-01-15");
        assert_eq!(result.settings.preparation_start_date, "2026-11-01");
        assert_eq!(result.settings.daily_study_target_minutes, 300);

        assert_eq!(
            planner::load_task(&connection, future.id, today)
                .expect("future task should load")
                .scheduled_date,
            "2027-01-05"
        );
        assert_eq!(
            planner::load_task(&connection, completed.id, today)
                .expect("completed task should load")
                .scheduled_date,
            "2026-12-14"
        );
        assert_eq!(
            planner::load_task(&connection, manual.id, today)
                .expect("manual task should load")
                .scheduled_date,
            "2027-01-20"
        );

        let sessions_after = sessions::list(&connection, sessions::StudySessionFilter::default())
            .expect("sessions should load");
        assert_eq!(sessions_after.len(), 1);
        assert_eq!(sessions_after[0].id, session.id);
        assert_eq!(sessions_after[0].session_date, "2026-11-02");
        assert_eq!(sessions_after[0].duration_seconds, 1_800);

        let totals = sessions::totals(&connection, "2026-11-02", "2026-11-02", today, None)
            .expect("totals should load");
        assert_eq!(totals.range_seconds, 1_800);
    }

    #[test]
    fn stale_previews_are_rejected_without_writing() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let future =
            planner::create_task(&mut connection, anchored_input("Future anchor", -10), today)
                .expect("future anchor should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("preview should load");
        let accepted = preview
            .changes
            .iter()
            .map(|change| AcceptedTaskChange {
                task_id: change.task_id,
                expected_scheduled_date: change.current_scheduled_date.clone(),
            })
            .collect::<Vec<_>>();

        planner::update_task(
            &mut connection,
            future.id,
            manual_input("Moved by the user", "2026-12-10"),
            today,
        )
        .expect("task should update");

        let error = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-15", "2026-11-01"),
                expected_exam_date: "2026-12-19".to_owned(),
                accepted_changes: accepted,
            },
            today,
        )
        .expect_err("stale preview should fail");
        assert_eq!(error.code, "reschedule_stale");

        let settings_after = settings::load(&connection).expect("settings should load");
        assert_eq!(settings_after.exam_date, "2026-12-19");
        assert_eq!(
            planner::load_task(&connection, future.id, today)
                .expect("task should load")
                .scheduled_date,
            "2026-12-10"
        );

        let error = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-15", "2026-11-01"),
                expected_exam_date: "2026-12-18".to_owned(),
                accepted_changes: Vec::new(),
            },
            today,
        )
        .expect_err("wrong expected exam should fail");
        assert_eq!(error.code, "reschedule_stale");
    }

    #[test]
    fn partial_acceptance_is_rejected_without_writing() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let first =
            planner::create_task(&mut connection, anchored_input("First anchor", -10), today)
                .expect("first anchor should save");
        let second =
            planner::create_task(&mut connection, anchored_input("Second anchor", -5), today)
                .expect("second anchor should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("preview should load");
        assert_eq!(preview.changes.len(), 2);

        let error = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-15", "2026-11-01"),
                expected_exam_date: "2026-12-19".to_owned(),
                accepted_changes: vec![AcceptedTaskChange {
                    task_id: first.id,
                    expected_scheduled_date: first.scheduled_date.clone(),
                }],
            },
            today,
        )
        .expect_err("a partial acceptance should fail");
        assert_eq!(error.code, "validation_error");
        assert_eq!(error.field, Some("acceptedChanges"));

        assert_eq!(
            settings::load(&connection)
                .expect("settings should load")
                .exam_date,
            "2026-12-19"
        );
        assert_eq!(
            planner::load_task(&connection, first.id, today)
                .expect("first task should load")
                .scheduled_date,
            first.scheduled_date
        );
        assert_eq!(
            planner::load_task(&connection, second.id, today)
                .expect("second task should load")
                .scheduled_date,
            second.scheduled_date
        );
    }

    #[test]
    fn settings_update_requires_confirmation_when_future_anchors_exist() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let future =
            planner::create_task(&mut connection, anchored_input("Future anchor", -10), today)
                .expect("future anchor should save");

        let error = settings::update(
            &mut connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect_err("direct exam change should be blocked");
        assert_eq!(error.code, "reschedule_confirmation_required");
        assert_eq!(
            settings::load(&connection)
                .expect("settings should load")
                .exam_date,
            "2026-12-19"
        );

        planner::delete_task(&mut connection, future.id).expect("task should delete");
        settings::update(
            &mut connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("direct exam change should work without future anchors");
    }

    #[test]
    fn apply_without_accepted_changes_still_preserves_all_tasks_and_blocks() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let manual = planner::create_task(
            &mut connection,
            manual_input("Manual task", "2027-01-20"),
            today,
        )
        .expect("manual task should save");
        let block =
            planner::create_block(&mut connection, block_input("Planned block", "2027-02-01"))
                .expect("block should save");

        let preview = preview(
            &connection,
            settings_input("2027-01-15", "2026-11-01"),
            today,
        )
        .expect("preview should load");
        assert!(preview.changes.is_empty());
        assert!(!preview.outside_window.is_empty());

        let result = apply(
            &mut connection,
            ApplyExamDateChangeInput {
                settings: settings_input("2027-01-15", "2026-11-01"),
                expected_exam_date: "2026-12-19".to_owned(),
                accepted_changes: Vec::new(),
            },
            today,
        )
        .expect("apply should succeed");
        assert_eq!(result.applied_task_count, 0);
        assert!(result.preserved_outside_window_count >= 2);
        assert_eq!(
            planner::load_task(&connection, manual.id, today)
                .expect("manual task should load")
                .scheduled_date,
            "2027-01-20"
        );
        assert_eq!(
            planner::list_blocks(&connection, "2027-02-01", "2027-02-01")
                .expect("block should load")[0]
                .id,
            block.id
        );
    }
}
