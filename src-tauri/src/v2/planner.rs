use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerTask {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub scheduled_date: String,
    pub deadline_date: Option<String>,
    pub exam_id: Option<i64>,
    pub exam_code: Option<String>,
    pub concept_id: Option<i64>,
    pub concept_title: Option<String>,
    pub syllabus_node_id: Option<i64>,
    pub priority: String,
    pub status: String,
    pub schedule_anchor: String,
    pub schedule_offset_days: Option<i64>,
    pub completed_at: Option<String>,
    pub linked_session_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerTaskInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub scheduled_date: String,
    #[serde(default)]
    pub deadline_date: Option<String>,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub concept_id: Option<i64>,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_anchor")]
    pub schedule_anchor: String,
    #[serde(default)]
    pub schedule_offset_days: Option<i64>,
    #[serde(default)]
    pub linked_session_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerBlock {
    pub id: i64,
    pub title: String,
    pub exam_id: Option<i64>,
    pub exam_code: Option<String>,
    pub concept_id: Option<i64>,
    pub concept_title: Option<String>,
    pub syllabus_node_id: Option<i64>,
    pub calendar_date: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
    pub notes: String,
    pub linked_session_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerBlockInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub concept_id: Option<i64>,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    pub calendar_date: String,
    pub start_time: String,
    pub end_time: String,
    #[serde(default = "default_block_status")]
    pub status: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub linked_session_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerDay {
    pub date: String,
    pub tasks: Vec<V2PlannerTask>,
    pub blocks: Vec<V2PlannerBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2PlannerDayInput {
    pub date: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2TaskStatusInput {
    pub task_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2RescheduleTaskInput {
    pub task_id: i64,
    pub scheduled_date: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BlockStatusInput {
    pub block_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2LinkBlockSessionInput {
    pub block_id: i64,
    pub session_id: Option<i64>,
}

fn default_priority() -> String {
    "normal".to_owned()
}

fn default_anchor() -> String {
    "manual".to_owned()
}

fn default_block_status() -> String {
    "planned".to_owned()
}

pub fn get_planner_day(
    connection: &Connection,
    input: V2PlannerDayInput,
) -> Result<V2PlannerDay, AppError> {
    let date = validate_date(&input.date, "date")?;
    let date_text = date_only::format_date_only(date);
    Ok(V2PlannerDay {
        date: date_text.clone(),
        tasks: list_tasks_for_date(connection, &date_text)?,
        blocks: list_blocks_for_date(connection, &date_text)?,
    })
}

pub fn list_tasks_for_date(
    connection: &Connection,
    date: &str,
) -> Result<Vec<V2PlannerTask>, AppError> {
    list_tasks(connection, "t.scheduled_date = ?1", params![date])
}

pub fn list_blocks_for_date(
    connection: &Connection,
    date: &str,
) -> Result<Vec<V2PlannerBlock>, AppError> {
    list_blocks(connection, "b.calendar_date = ?1", params![date])
}

pub fn get_agenda(
    connection: &Connection,
    input: V2PlannerDayInput,
) -> Result<V2PlannerDay, AppError> {
    get_planner_day(connection, input)
}

pub fn save_task(
    connection: &mut Connection,
    input: V2PlannerTaskInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerTask, AppError> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::validation(
            "title",
            "Task title must be between 1 and 200 characters.",
        ));
    }
    if input.description.chars().count() > 2_000 {
        return Err(AppError::validation(
            "description",
            "Task description must be 2,000 characters or fewer.",
        ));
    }
    let scheduled = validate_date(&input.scheduled_date, "scheduledDate")?;
    let deadline = input
        .deadline_date
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| validate_date(value, "deadlineDate"))
        .transpose()?;
    if let Some(deadline) = deadline
        && deadline < scheduled
    {
        return Err(AppError::validation(
            "deadlineDate",
            "Deadline cannot be before the scheduled date.",
        ));
    }
    validate_priority(&input.priority)?;
    if !matches!(input.schedule_anchor.as_str(), "manual" | "exam") {
        return Err(AppError::validation(
            "scheduleAnchor",
            "Schedule anchor must be manual or exam.",
        ));
    }
    if input.schedule_anchor == "exam" {
        if input.exam_id.is_none() || input.schedule_offset_days.is_none() {
            return Err(AppError::validation(
                "scheduleOffsetDays",
                "Exam-anchored tasks require an exam and a day offset.",
            ));
        }
    } else if input.schedule_offset_days.is_some() {
        return Err(AppError::validation(
            "scheduleOffsetDays",
            "Manual tasks do not use an exam day offset.",
        ));
    }
    validate_exam(connection, input.exam_id)?;
    validate_concept(connection, input.concept_id)?;
    validate_node(connection, input.exam_id, input.syllabus_node_id)?;
    validate_session(connection, input.linked_session_id)?;
    if input.schedule_anchor == "exam"
        && let (Some(exam_id), Some(offset)) = (input.exam_id, input.schedule_offset_days)
    {
        let exam_date: Option<String> = connection
            .query_row(
                "SELECT exam_date FROM exam_profiles WHERE exam_id = ?1",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Exam date could not be checked", error))?;
        let expected = exam_date
            .and_then(|value| date_only::parse_date_only(&value))
            .and_then(|value| value.checked_add_signed(chrono::Duration::days(offset)))
            .ok_or_else(|| {
                AppError::validation(
                    "scheduledDate",
                    "Set the exam date before adding an exam-anchored task.",
                )
            })?;
        if scheduled != expected {
            return Err(AppError::validation(
                "scheduledDate",
                format!(
                    "An exam-anchored task must be scheduled on {}.",
                    date_only::format_date_only(expected)
                ),
            ));
        }
    }

    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let scheduled_text = date_only::format_date_only(scheduled);
    let deadline_text = deadline.map(date_only::format_date_only);
    let task_id = if let Some(task_id) = input.id {
        let changed = connection
            .execute(
                "UPDATE checklist_tasks
                 SET title = ?1, description = ?2, scheduled_date = ?3, deadline_date = ?4,
                     exam_id = ?5, concept_id = ?6, syllabus_node_id = ?7, priority = ?8,
                     schedule_anchor = ?9, schedule_offset_days = ?10,
                     linked_session_id = ?11, updated_at = ?12
                 WHERE id = ?13",
                params![
                    title,
                    input.description.trim(),
                    scheduled_text,
                    deadline_text,
                    input.exam_id,
                    input.concept_id,
                    input.syllabus_node_id,
                    input.priority,
                    input.schedule_anchor,
                    input.schedule_offset_days,
                    input.linked_session_id,
                    now_text,
                    task_id,
                ],
            )
            .map_err(|error| AppError::database("Checklist task could not be updated", error))?;
        if changed != 1 {
            return Err(AppError::task_not_found());
        }
        task_id
    } else {
        connection
            .execute(
                "INSERT INTO checklist_tasks (
                    title, description, scheduled_date, deadline_date, exam_id, concept_id,
                    syllabus_node_id, priority, status, schedule_anchor, schedule_offset_days,
                    completed_at, linked_session_id, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'open', ?9, ?10, NULL, ?11, ?12, ?12)",
                params![
                    title,
                    input.description.trim(),
                    scheduled_text,
                    deadline_text,
                    input.exam_id,
                    input.concept_id,
                    input.syllabus_node_id,
                    input.priority,
                    input.schedule_anchor,
                    input.schedule_offset_days,
                    input.linked_session_id,
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Checklist task could not be saved", error))?;
        connection.last_insert_rowid()
    };
    load_task(connection, task_id)
}

pub fn set_task_status(
    connection: &mut Connection,
    input: V2TaskStatusInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerTask, AppError> {
    if !matches!(input.status.as_str(), "open" | "completed") {
        return Err(AppError::validation(
            "status",
            "Task status must be open or completed.",
        ));
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let completed_at = (input.status == "completed").then_some(now_text.clone());
    let changed = connection
        .execute(
            "UPDATE checklist_tasks
             SET status = ?1, completed_at = ?2, updated_at = ?3
             WHERE id = ?4",
            params![input.status, completed_at, now_text, input.task_id],
        )
        .map_err(|error| AppError::database("Task status could not be saved", error))?;
    if changed != 1 {
        return Err(AppError::task_not_found());
    }
    load_task(connection, input.task_id)
}

pub fn reschedule_task(
    connection: &mut Connection,
    input: V2RescheduleTaskInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerTask, AppError> {
    let date = validate_date(&input.scheduled_date, "scheduledDate")?;
    let deadline: Option<String> = connection
        .query_row(
            "SELECT deadline_date FROM checklist_tasks WHERE id = ?1",
            [input.task_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Task could not be loaded", error))?
        .ok_or_else(AppError::task_not_found)?;
    if let Some(deadline) = deadline.and_then(|value| date_only::parse_date_only(&value))
        && date > deadline
    {
        return Err(AppError::validation(
            "scheduledDate",
            "The new scheduled date cannot be after the task deadline.",
        ));
    }
    let changed = connection
        .execute(
            "UPDATE checklist_tasks
             SET scheduled_date = ?1, schedule_anchor = 'manual', schedule_offset_days = NULL,
                 updated_at = ?2
             WHERE id = ?3 AND status = 'open'",
            params![
                date_only::format_date_only(date),
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
                input.task_id
            ],
        )
        .map_err(|error| AppError::database("Task could not be rescheduled", error))?;
    if changed != 1 {
        return Err(AppError::validation(
            "taskId",
            "Only an open task can be rescheduled.",
        ));
    }
    load_task(connection, input.task_id)
}

pub fn delete_task(connection: &mut Connection, task_id: i64) -> Result<(), AppError> {
    let changed = connection
        .execute("DELETE FROM checklist_tasks WHERE id = ?1", [task_id])
        .map_err(|error| AppError::database("Task could not be deleted", error))?;
    if changed != 1 {
        return Err(AppError::task_not_found());
    }
    Ok(())
}

pub fn save_block(
    connection: &mut Connection,
    input: V2PlannerBlockInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerBlock, AppError> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::validation(
            "title",
            "Block title must be between 1 and 200 characters.",
        ));
    }
    if input.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Block notes must be 2,000 characters or fewer.",
        ));
    }
    let date = validate_date(&input.calendar_date, "calendarDate")?;
    validate_time(&input.start_time, "startTime")?;
    validate_time(&input.end_time, "endTime")?;
    if input.start_time >= input.end_time {
        return Err(AppError::validation(
            "endTime",
            "End time must be after start time.",
        ));
    }
    validate_block_status(&input.status)?;
    validate_exam(connection, input.exam_id)?;
    validate_concept(connection, input.concept_id)?;
    validate_node(connection, input.exam_id, input.syllabus_node_id)?;
    validate_session(connection, input.linked_session_id)?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let block_id = if let Some(block_id) = input.id {
        let changed = connection
            .execute(
                "UPDATE planned_study_blocks
                 SET title = ?1, exam_id = ?2, concept_id = ?3, syllabus_node_id = ?4,
                     calendar_date = ?5, start_time = ?6, end_time = ?7, status = ?8,
                     notes = ?9, linked_session_id = ?10, updated_at = ?11
                 WHERE id = ?12",
                params![
                    title,
                    input.exam_id,
                    input.concept_id,
                    input.syllabus_node_id,
                    date_only::format_date_only(date),
                    input.start_time,
                    input.end_time,
                    input.status,
                    input.notes.trim(),
                    input.linked_session_id,
                    now_text,
                    block_id,
                ],
            )
            .map_err(|error| AppError::database("Planned block could not be updated", error))?;
        if changed != 1 {
            return Err(AppError::block_not_found());
        }
        block_id
    } else {
        connection
            .execute(
                "INSERT INTO planned_study_blocks (
                    title, exam_id, concept_id, syllabus_node_id, calendar_date, start_time,
                    end_time, status, notes, linked_session_id, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                params![
                    title,
                    input.exam_id,
                    input.concept_id,
                    input.syllabus_node_id,
                    date_only::format_date_only(date),
                    input.start_time,
                    input.end_time,
                    input.status,
                    input.notes.trim(),
                    input.linked_session_id,
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Planned block could not be saved", error))?;
        connection.last_insert_rowid()
    };
    load_block(connection, block_id)
}

pub fn set_block_status(
    connection: &mut Connection,
    input: V2BlockStatusInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerBlock, AppError> {
    if !matches!(
        input.status.as_str(),
        "planned" | "in_progress" | "completed" | "cancelled"
    ) {
        return Err(AppError::validation(
            "status",
            "Choose planned, in progress, completed, or cancelled.",
        ));
    }
    let changed = connection
        .execute(
            "UPDATE planned_study_blocks
             SET status = ?1, updated_at = ?2
             WHERE id = ?3",
            params![
                input.status,
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
                input.block_id
            ],
        )
        .map_err(|error| AppError::database("Block status could not be saved", error))?;
    if changed != 1 {
        return Err(AppError::block_not_found());
    }
    load_block(connection, input.block_id)
}

pub fn link_block_session(
    connection: &mut Connection,
    input: V2LinkBlockSessionInput,
    now: DateTime<Utc>,
) -> Result<V2PlannerBlock, AppError> {
    validate_session(connection, input.session_id)?;
    let status = connection
        .query_row(
            "SELECT status FROM planned_study_blocks WHERE id = ?1",
            [input.block_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| AppError::database("Block could not be loaded", error))?
        .ok_or_else(AppError::block_not_found)?;
    if input.session_id.is_some() && !matches!(status.as_str(), "in_progress" | "completed") {
        return Err(AppError::validation(
            "sessionId",
            "Mark the planned block in progress or completed before linking a session.",
        ));
    }
    connection
        .execute(
            "UPDATE planned_study_blocks
             SET linked_session_id = ?1, updated_at = ?2
             WHERE id = ?3",
            params![
                input.session_id,
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
                input.block_id
            ],
        )
        .map_err(|error| AppError::database("Block session link could not be saved", error))?;
    load_block(connection, input.block_id)
}

pub fn delete_block(connection: &mut Connection, block_id: i64) -> Result<(), AppError> {
    let changed = connection
        .execute("DELETE FROM planned_study_blocks WHERE id = ?1", [block_id])
        .map_err(|error| AppError::database("Planned block could not be deleted", error))?;
    if changed != 1 {
        return Err(AppError::block_not_found());
    }
    Ok(())
}

fn list_tasks(
    connection: &Connection,
    condition: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<V2PlannerTask>, AppError> {
    let sql = format!(
        "SELECT t.id, t.title, t.description, t.scheduled_date, t.deadline_date,
                t.exam_id, e.code, t.concept_id, c.title, t.syllabus_node_id,
                t.priority, t.status, t.schedule_anchor, t.schedule_offset_days,
                t.completed_at, t.linked_session_id, t.created_at, t.updated_at
         FROM checklist_tasks t
         LEFT JOIN exams e ON e.id = t.exam_id
         LEFT JOIN concepts c ON c.id = t.concept_id
         WHERE {condition}
         ORDER BY t.scheduled_date, CASE t.priority
             WHEN 'high_yield' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
             t.id"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Checklist tasks could not be prepared", error))?;
    let rows = statement
        .query_map(params, map_task)
        .map_err(|error| AppError::database("Checklist tasks could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A checklist task row is invalid", error))
}

fn load_task(connection: &Connection, task_id: i64) -> Result<V2PlannerTask, AppError> {
    list_tasks(connection, "t.id = ?1", [task_id])?
        .into_iter()
        .next()
        .ok_or_else(AppError::task_not_found)
}

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2PlannerTask> {
    Ok(V2PlannerTask {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        scheduled_date: row.get(3)?,
        deadline_date: row.get(4)?,
        exam_id: row.get(5)?,
        exam_code: row.get(6)?,
        concept_id: row.get(7)?,
        concept_title: row.get(8)?,
        syllabus_node_id: row.get(9)?,
        priority: row.get(10)?,
        status: row.get(11)?,
        schedule_anchor: row.get(12)?,
        schedule_offset_days: row.get(13)?,
        completed_at: row.get(14)?,
        linked_session_id: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn list_blocks(
    connection: &Connection,
    condition: &str,
    params: impl rusqlite::Params,
) -> Result<Vec<V2PlannerBlock>, AppError> {
    let sql = format!(
        "SELECT b.id, b.title, b.exam_id, e.code, b.concept_id, c.title,
                b.syllabus_node_id, b.calendar_date, b.start_time, b.end_time,
                b.status, b.notes, b.linked_session_id, b.created_at, b.updated_at
         FROM planned_study_blocks b
         LEFT JOIN exams e ON e.id = b.exam_id
         LEFT JOIN concepts c ON c.id = b.concept_id
         WHERE {condition}
         ORDER BY b.calendar_date, b.start_time, b.id"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Planned blocks could not be prepared", error))?;
    let rows = statement
        .query_map(params, |row| {
            Ok(V2PlannerBlock {
                id: row.get(0)?,
                title: row.get(1)?,
                exam_id: row.get(2)?,
                exam_code: row.get(3)?,
                concept_id: row.get(4)?,
                concept_title: row.get(5)?,
                syllabus_node_id: row.get(6)?,
                calendar_date: row.get(7)?,
                start_time: row.get(8)?,
                end_time: row.get(9)?,
                status: row.get(10)?,
                notes: row.get(11)?,
                linked_session_id: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })
        .map_err(|error| AppError::database("Planned blocks could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A planned block row is invalid", error))
}

fn load_block(connection: &Connection, block_id: i64) -> Result<V2PlannerBlock, AppError> {
    list_blocks(connection, "b.id = ?1", [block_id])?
        .into_iter()
        .next()
        .ok_or_else(AppError::block_not_found)
}

fn validate_date(value: &str, field: &'static str) -> Result<chrono::NaiveDate, AppError> {
    date_only::parse_date_only(value.trim()).ok_or_else(|| {
        AppError::validation(field, "Enter a real calendar date in YYYY-MM-DD format.")
    })
}

fn validate_time(value: &str, field: &'static str) -> Result<(), AppError> {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 5
        && bytes[2] == b':'
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && value[..2]
            .parse::<u32>()
            .map(|hour| hour <= 23)
            .unwrap_or(false)
        && value[3..]
            .parse::<u32>()
            .map(|minute| minute <= 59)
            .unwrap_or(false);
    if !valid {
        return Err(AppError::validation(field, "Enter a time in HH:MM format."));
    }
    Ok(())
}

fn validate_priority(value: &str) -> Result<(), AppError> {
    if matches!(value, "low" | "normal" | "high" | "high_yield") {
        Ok(())
    } else {
        Err(AppError::validation(
            "priority",
            "Choose low, normal, high, or high-yield priority.",
        ))
    }
}

fn validate_block_status(value: &str) -> Result<(), AppError> {
    if matches!(value, "planned" | "in_progress" | "completed" | "cancelled") {
        Ok(())
    } else {
        Err(AppError::validation(
            "status",
            "Choose planned, in progress, completed, or cancelled.",
        ))
    }
}

fn validate_exam(connection: &Connection, exam_id: Option<i64>) -> Result<(), AppError> {
    if let Some(exam_id) = exam_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Exam could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "examId",
                "The selected examination does not exist.",
            ));
        }
    }
    Ok(())
}

fn validate_concept(connection: &Connection, concept_id: Option<i64>) -> Result<(), AppError> {
    if let Some(concept_id) = concept_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1 AND is_archived = 0)",
                [concept_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Concept could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "conceptId",
                "Choose an active concept from the shared library.",
            ));
        }
    }
    Ok(())
}

fn validate_node(
    connection: &Connection,
    exam_id: Option<i64>,
    node_id: Option<i64>,
) -> Result<(), AppError> {
    if let Some(node_id) = node_id {
        let exam_id = exam_id.ok_or_else(|| {
            AppError::validation("examId", "A syllabus node requires an examination.")
        })?;
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM exam_syllabus_nodes
                    WHERE exam_id = ?1 AND id = ?2 AND is_archived = 0
                 )",
                params![exam_id, node_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Syllabus node could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "syllabusNodeId",
                "Choose an active syllabus node from the selected examination.",
            ));
        }
    }
    Ok(())
}

fn validate_session(connection: &Connection, session_id: Option<i64>) -> Result<(), AppError> {
    if let Some(session_id) = session_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM study_sessions WHERE id = ?1)",
                [session_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Study session could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "sessionId",
                "The selected study session does not exist.",
            ));
        }
    }
    Ok(())
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

    #[test]
    fn task_crud_status_and_rescheduling_persist() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        let task = state
            .with_connection(|connection| {
                save_task(
                    connection,
                    V2PlannerTaskInput {
                        id: None,
                        title: "Review kinematics".to_owned(),
                        description: "Work through core examples".to_owned(),
                        scheduled_date: "2026-11-05".to_owned(),
                        deadline_date: Some("2026-11-10".to_owned()),
                        exam_id: Some(1),
                        concept_id: Some(concept),
                        syllabus_node_id: None,
                        priority: "high".to_owned(),
                        schedule_anchor: "manual".to_owned(),
                        schedule_offset_days: None,
                        linked_session_id: None,
                    },
                    at(0),
                )
            })
            .expect("task");
        assert_eq!(task.status, "open");
        assert_eq!(task.concept_title.as_deref(), Some("Kinematics"));

        let completed = state
            .with_connection(|connection| {
                set_task_status(
                    connection,
                    V2TaskStatusInput {
                        task_id: task.id,
                        status: "completed".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("complete");
        assert_eq!(completed.status, "completed");
        assert!(completed.completed_at.is_some());

        let reschedule_error = state
            .with_connection(|connection| {
                reschedule_task(
                    connection,
                    V2RescheduleTaskInput {
                        task_id: task.id,
                        scheduled_date: "2026-11-06".to_owned(),
                    },
                    at(2),
                )
            })
            .expect_err("completed task cannot be rescheduled");
        assert_eq!(reschedule_error.code, "validation_error");

        let reopened = state
            .with_connection(|connection| {
                set_task_status(
                    connection,
                    V2TaskStatusInput {
                        task_id: task.id,
                        status: "open".to_owned(),
                    },
                    at(3),
                )
            })
            .expect("reopen");
        assert_eq!(reopened.status, "open");
        assert!(reopened.completed_at.is_none());

        let rescheduled = state
            .with_connection(|connection| {
                reschedule_task(
                    connection,
                    V2RescheduleTaskInput {
                        task_id: task.id,
                        scheduled_date: "2026-11-06".to_owned(),
                    },
                    at(4),
                )
            })
            .expect("reschedule");
        assert_eq!(rescheduled.scheduled_date, "2026-11-06");

        let day = state
            .with_connection(|connection| {
                get_planner_day(
                    connection,
                    V2PlannerDayInput {
                        date: "2026-11-06".to_owned(),
                    },
                )
            })
            .expect("day");
        assert_eq!(day.tasks.len(), 1);

        state
            .with_connection(|connection| delete_task(connection, task.id))
            .expect("delete");
        let empty = state
            .with_connection(|connection| {
                get_planner_day(
                    connection,
                    V2PlannerDayInput {
                        date: "2026-11-06".to_owned(),
                    },
                )
            })
            .expect("empty day");
        assert!(empty.tasks.is_empty());
    }

    #[test]
    fn planned_blocks_validate_times_and_link_sessions() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        let session_id = state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO study_sessions (
                            session_date, duration_seconds, source, concept_id,
                            created_at, updated_at
                         ) VALUES ('2026-11-04', 1800, 'manual', ?1,
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [concept],
                    )
                    .map_err(|error| AppError::database("test session", error))?;
                Ok::<_, AppError>(connection.last_insert_rowid())
            })
            .expect("session");

        let invalid = state
            .with_connection(|connection| {
                save_block(
                    connection,
                    V2PlannerBlockInput {
                        id: None,
                        title: "Bad block".to_owned(),
                        exam_id: Some(1),
                        concept_id: Some(concept),
                        syllabus_node_id: None,
                        calendar_date: "2026-11-05".to_owned(),
                        start_time: "14:00".to_owned(),
                        end_time: "13:00".to_owned(),
                        status: "planned".to_owned(),
                        notes: String::new(),
                        linked_session_id: None,
                    },
                    at(0),
                )
            })
            .expect_err("invalid time");
        assert_eq!(invalid.field, Some("endTime"));

        let block = state
            .with_connection(|connection| {
                save_block(
                    connection,
                    V2PlannerBlockInput {
                        id: None,
                        title: "Study block".to_owned(),
                        exam_id: Some(1),
                        concept_id: Some(concept),
                        syllabus_node_id: None,
                        calendar_date: "2026-11-05".to_owned(),
                        start_time: "13:00".to_owned(),
                        end_time: "14:00".to_owned(),
                        status: "planned".to_owned(),
                        notes: String::new(),
                        linked_session_id: None,
                    },
                    at(1),
                )
            })
            .expect("block");
        assert_eq!(block.status, "planned");

        let link_error = state
            .with_connection(|connection| {
                link_block_session(
                    connection,
                    V2LinkBlockSessionInput {
                        block_id: block.id,
                        session_id: Some(session_id),
                    },
                    at(2),
                )
            })
            .expect_err("planned block cannot link");
        assert_eq!(link_error.code, "validation_error");

        let in_progress = state
            .with_connection(|connection| {
                set_block_status(
                    connection,
                    V2BlockStatusInput {
                        block_id: block.id,
                        status: "in_progress".to_owned(),
                    },
                    at(3),
                )
            })
            .expect("in progress");
        assert_eq!(in_progress.status, "in_progress");

        let linked = state
            .with_connection(|connection| {
                link_block_session(
                    connection,
                    V2LinkBlockSessionInput {
                        block_id: block.id,
                        session_id: Some(session_id),
                    },
                    at(4),
                )
            })
            .expect("link");
        assert_eq!(linked.linked_session_id, Some(session_id));

        state
            .with_connection(|connection| delete_block(connection, block.id))
            .expect("delete block");
    }
}
