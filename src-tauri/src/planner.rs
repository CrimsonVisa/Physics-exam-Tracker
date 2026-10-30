use chrono::{Days, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{date_only, error::AppError, revision, settings};

pub const TASK_PRIORITIES: [&str; 4] = ["Low", "Normal", "High", "High-Yield"];
pub const TASK_STATUSES: [&str; 2] = ["open", "completed"];
pub const TASK_ANCHORS: [&str; 2] = ["manual", "exam"];
pub const BLOCK_STATUSES: [&str; 3] = ["planned", "completed", "cancelled"];
pub const MAX_TITLE_CHARS: usize = 200;
pub const MAX_NOTES_CHARS: usize = 2_000;
pub const MAX_SEARCH_QUERY_CHARS: usize = 100;
pub const MIN_SCHEDULE_OFFSET_DAYS: i64 = -3_650;
pub const MAX_SCHEDULE_OFFSET_DAYS: i64 = 3_650;

const TASK_SELECT: &str = "SELECT
        t.id,
        t.title,
        t.description,
        t.scheduled_date,
        t.deadline_date,
        t.topic_id,
        top.code,
        top.title,
        top.is_archived,
        t.priority,
        t.status,
        t.schedule_anchor,
        t.schedule_offset_days,
        t.completed_at,
        t.created_at,
        t.updated_at
     FROM checklist_tasks t
     LEFT JOIN syllabus_topics top ON top.id = t.topic_id";

const BLOCK_SELECT: &str = "SELECT
        b.id,
        b.title,
        b.topic_id,
        top.code,
        top.title,
        top.is_archived,
        b.calendar_date,
        b.start_time,
        b.end_time,
        b.status,
        b.notes,
        b.linked_session_id,
        b.created_at,
        b.updated_at
     FROM planned_study_blocks b
     LEFT JOIN syllabus_topics top ON top.id = b.topic_id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerTopic {
    pub id: i64,
    pub code: String,
    pub title: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistTask {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub scheduled_date: String,
    pub deadline_date: Option<String>,
    pub topic: Option<PlannerTopic>,
    pub priority: String,
    pub status: String,
    pub schedule_anchor: String,
    pub schedule_offset_days: Option<i64>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub due_date: String,
    pub is_overdue: bool,
    pub is_upcoming: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskFilter {
    pub status: Option<String>,
    pub query: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scheduled_date: String,
    #[serde(default)]
    pub deadline_date: Option<String>,
    #[serde(default)]
    pub topic_id: Option<i64>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_schedule_anchor")]
    pub schedule_anchor: String,
    #[serde(default)]
    pub schedule_offset_days: Option<i64>,
}

fn default_priority() -> String {
    "Normal".to_owned()
}

fn default_schedule_anchor() -> String {
    "manual".to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedBlock {
    pub id: i64,
    pub title: String,
    pub topic: Option<PlannerTopic>,
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
pub struct PlannedBlockInput {
    pub title: String,
    #[serde(default)]
    pub topic_id: Option<i64>,
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

fn default_block_status() -> String {
    "planned".to_owned()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarData {
    pub from_date: String,
    pub to_date: String,
    pub today: String,
    pub tasks: Vec<ChecklistTask>,
    pub blocks: Vec<PlannedBlock>,
    pub revisions: Vec<revision::RevisionCalendarItem>,
    pub deadlines: Vec<ChecklistTask>,
    pub upcoming_deadlines: Vec<ChecklistTask>,
    pub overdue_tasks: Vec<ChecklistTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Agenda {
    pub today: String,
    pub today_tasks: Vec<ChecklistTask>,
    pub today_blocks: Vec<PlannedBlock>,
    pub upcoming_deadlines: Vec<ChecklistTask>,
    pub overdue_tasks: Vec<ChecklistTask>,
}

struct ValidatedTaskInput {
    title: String,
    description: String,
    scheduled_date: String,
    deadline_date: Option<String>,
    topic_id: Option<i64>,
    priority: String,
    schedule_anchor: String,
    schedule_offset_days: Option<i64>,
}

pub fn list_topics(connection: &Connection) -> Result<Vec<PlannerTopic>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT t.id, t.code, t.title, t.is_archived
             FROM syllabus_topics t
             WHERE t.is_archived = 0
                OR EXISTS (
                    SELECT 1 FROM checklist_tasks c WHERE c.topic_id = t.id
                )
                OR EXISTS (
                    SELECT 1 FROM planned_study_blocks b WHERE b.topic_id = t.id
                )
             ORDER BY t.is_archived, t.code",
        )
        .map_err(|error| AppError::database("Planner topics could not be prepared", error))?;

    let rows = statement
        .query_map([], |row| {
            Ok(PlannerTopic {
                id: row.get(0)?,
                code: row.get(1)?,
                title: row.get(2)?,
                archived: row.get::<_, i64>(3)? != 0,
            })
        })
        .map_err(|error| AppError::database("Planner topics could not be queried", error))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Planner topics could not be read", error))
}

pub fn list_tasks(
    connection: &Connection,
    filter: TaskFilter,
    today: NaiveDate,
) -> Result<Vec<ChecklistTask>, AppError> {
    let status = filter
        .status
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty() && value != "all");
    if let Some(status) = &status
        && !TASK_STATUSES.contains(&status.as_str())
    {
        return Err(AppError::validation(
            "status",
            "Choose open, completed or all tasks.",
        ));
    }

    let query = filter
        .query
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if query
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_SEARCH_QUERY_CHARS)
    {
        return Err(AppError::validation(
            "query",
            format!("Search text must be {MAX_SEARCH_QUERY_CHARS} characters or fewer."),
        ));
    }

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

    let pattern = query.map(|value| format!("%{}%", escape_like(&value)));
    let sql = format!(
        "{TASK_SELECT}
         WHERE (?1 IS NULL OR t.status = ?1)
           AND (
                ?2 IS NULL
                OR t.title LIKE ?2 ESCAPE '!'
                OR t.description LIKE ?2 ESCAPE '!'
           )
           AND (?3 IS NULL OR t.scheduled_date >= ?3)
           AND (?4 IS NULL OR t.scheduled_date <= ?4)
         ORDER BY
            CASE t.status WHEN 'open' THEN 0 ELSE 1 END,
            COALESCE(t.deadline_date, t.scheduled_date),
            t.scheduled_date,
            CASE t.priority
                WHEN 'High-Yield' THEN 0
                WHEN 'High' THEN 1
                WHEN 'Normal' THEN 2
                ELSE 3
            END,
            t.id"
    );

    query_tasks(
        connection,
        &sql,
        params![status, pattern, from_date, to_date],
        today,
    )
}

pub fn create_task(
    connection: &mut Connection,
    input: TaskInput,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;
    let validated = validate_task_input(&transaction, input, None)?;

    transaction
        .execute(
            "INSERT INTO checklist_tasks (
                title, description, scheduled_date, deadline_date, topic_id,
                priority, status, schedule_anchor, schedule_offset_days,
                completed_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, 'open', ?7, ?8,
                NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.title,
                validated.description,
                validated.scheduled_date,
                validated.deadline_date,
                validated.topic_id,
                validated.priority,
                validated.schedule_anchor,
                validated.schedule_offset_days,
            ],
        )
        .map_err(|error| AppError::database("The checklist task could not be saved", error))?;

    let task_id = transaction.last_insert_rowid();
    let task = load_task(&transaction, task_id, today)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The checklist task could not be committed", error))?;

    Ok(task)
}

pub fn update_task(
    connection: &mut Connection,
    task_id: i64,
    input: TaskInput,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    validate_id(task_id, "taskId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;
    let existing = load_task(&transaction, task_id, today)?;
    let existing_topic_id = existing.topic.as_ref().map(|topic| topic.id);
    let validated = validate_task_input(&transaction, input, existing_topic_id)?;

    let changed = transaction
        .execute(
            "UPDATE checklist_tasks
             SET title = ?1,
                 description = ?2,
                 scheduled_date = ?3,
                 deadline_date = ?4,
                 topic_id = ?5,
                 priority = ?6,
                 schedule_anchor = ?7,
                 schedule_offset_days = ?8,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?9",
            params![
                validated.title,
                validated.description,
                validated.scheduled_date,
                validated.deadline_date,
                validated.topic_id,
                validated.priority,
                validated.schedule_anchor,
                validated.schedule_offset_days,
                task_id,
            ],
        )
        .map_err(|error| AppError::database("The checklist task could not be updated", error))?;

    if changed != 1 {
        return Err(AppError::task_not_found());
    }

    let task = load_task(&transaction, task_id, today)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The checklist task could not be committed", error))?;

    Ok(task)
}

pub fn complete_task(
    connection: &mut Connection,
    task_id: i64,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    validate_id(task_id, "taskId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;
    let existing = load_task(&transaction, task_id, today)?;

    if existing.status == "completed" {
        transaction.commit().map_err(|error| {
            AppError::database("The checklist task could not be committed", error)
        })?;
        return Ok(existing);
    }

    transaction
        .execute(
            "UPDATE checklist_tasks
             SET status = 'completed',
                 completed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            [task_id],
        )
        .map_err(|error| AppError::database("The checklist task could not be completed", error))?;

    let task = load_task(&transaction, task_id, today)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The checklist task could not be committed", error))?;

    Ok(task)
}

pub fn reopen_task(
    connection: &mut Connection,
    task_id: i64,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    validate_id(task_id, "taskId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;
    let existing = load_task(&transaction, task_id, today)?;

    if existing.status == "open" {
        transaction.commit().map_err(|error| {
            AppError::database("The checklist task could not be committed", error)
        })?;
        return Ok(existing);
    }

    transaction
        .execute(
            "UPDATE checklist_tasks
             SET status = 'open',
                 completed_at = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            [task_id],
        )
        .map_err(|error| AppError::database("The checklist task could not be reopened", error))?;

    let task = load_task(&transaction, task_id, today)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The checklist task could not be committed", error))?;

    Ok(task)
}

pub fn reschedule_task(
    connection: &mut Connection,
    task_id: i64,
    scheduled_date: &str,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    validate_id(task_id, "taskId")?;
    let scheduled = validate_required_date(scheduled_date, "scheduledDate")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;
    let existing = load_task(&transaction, task_id, today)?;

    if existing.status == "completed" {
        return Err(AppError::task_completed());
    }

    transaction
        .execute(
            "UPDATE checklist_tasks
             SET scheduled_date = ?1,
                 schedule_anchor = 'manual',
                 schedule_offset_days = NULL,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![date_only::format_date_only(scheduled), task_id],
        )
        .map_err(|error| {
            AppError::database("The checklist task could not be rescheduled", error)
        })?;

    let task = load_task(&transaction, task_id, today)?;

    transaction
        .commit()
        .map_err(|error| AppError::database("The checklist task could not be committed", error))?;

    Ok(task)
}

pub fn delete_task(connection: &mut Connection, task_id: i64) -> Result<(), AppError> {
    validate_id(task_id, "taskId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The checklist-task transaction could not be started", error)
    })?;

    let changed = transaction
        .execute("DELETE FROM checklist_tasks WHERE id = ?1", [task_id])
        .map_err(|error| AppError::database("The checklist task could not be deleted", error))?;

    if changed != 1 {
        return Err(AppError::task_not_found());
    }

    transaction.commit().map_err(|error| {
        AppError::database("The checklist-task deletion could not be committed", error)
    })
}

pub fn calendar(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
    today: NaiveDate,
) -> Result<CalendarData, AppError> {
    let from = validate_required_date(from_date, "fromDate")?;
    let to = validate_required_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    let from_text = date_only::format_date_only(from);
    let to_text = date_only::format_date_only(to);
    let today_text = date_only::format_date_only(today);
    let upcoming_end = date_only::format_date_only(today + Days::new(30));

    let tasks = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.scheduled_date >= ?1 AND t.scheduled_date <= ?2
             ORDER BY t.scheduled_date, COALESCE(t.deadline_date, t.scheduled_date), t.id"
        ),
        params![from_text, to_text],
        today,
    )?;

    let blocks = list_blocks(connection, &from_text, &to_text)?;
    let revisions = revision::calendar_items(connection, &from_text, &to_text)?;

    let deadlines = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.status = 'open'
               AND t.deadline_date IS NOT NULL
               AND t.deadline_date >= ?1
               AND t.deadline_date <= ?2
             ORDER BY t.deadline_date, t.id"
        ),
        params![from_text, to_text],
        today,
    )?;

    let upcoming_deadlines = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.status = 'open'
               AND t.deadline_date IS NOT NULL
               AND t.deadline_date >= ?1
               AND t.deadline_date <= ?2
             ORDER BY t.deadline_date, t.id"
        ),
        params![today_text, upcoming_end],
        today,
    )?;

    let overdue_tasks = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.status = 'open'
               AND COALESCE(t.deadline_date, t.scheduled_date) < ?1
             ORDER BY COALESCE(t.deadline_date, t.scheduled_date), t.id
             LIMIT 100"
        ),
        params![today_text],
        today,
    )?;

    Ok(CalendarData {
        from_date: from_text,
        to_date: to_text,
        today: today_text,
        tasks,
        blocks,
        revisions,
        deadlines,
        upcoming_deadlines,
        overdue_tasks,
    })
}

pub fn agenda(connection: &Connection, today: NaiveDate) -> Result<Agenda, AppError> {
    let today_text = date_only::format_date_only(today);
    let upcoming_end = date_only::format_date_only(today + Days::new(7));

    let today_tasks = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.scheduled_date = ?1
             ORDER BY
                CASE t.status WHEN 'open' THEN 0 ELSE 1 END,
                CASE t.priority
                    WHEN 'High-Yield' THEN 0
                    WHEN 'High' THEN 1
                    WHEN 'Normal' THEN 2
                    ELSE 3
                END,
                t.id"
        ),
        params![today_text],
        today,
    )?;

    let today_blocks = list_blocks(connection, &today_text, &today_text)?;

    let upcoming_deadlines = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.status = 'open'
               AND t.deadline_date IS NOT NULL
               AND t.deadline_date >= ?1
               AND t.deadline_date <= ?2
             ORDER BY t.deadline_date, t.id"
        ),
        params![today_text, upcoming_end],
        today,
    )?;

    let overdue_tasks = query_tasks(
        connection,
        &format!(
            "{TASK_SELECT}
             WHERE t.status = 'open'
               AND COALESCE(t.deadline_date, t.scheduled_date) < ?1
             ORDER BY COALESCE(t.deadline_date, t.scheduled_date), t.id
             LIMIT 50"
        ),
        params![today_text],
        today,
    )?;

    Ok(Agenda {
        today: today_text,
        today_tasks,
        today_blocks,
        upcoming_deadlines,
        overdue_tasks,
    })
}

pub fn list_blocks(
    connection: &Connection,
    from_date: &str,
    to_date: &str,
) -> Result<Vec<PlannedBlock>, AppError> {
    let from = validate_required_date(from_date, "fromDate")?;
    let to = validate_required_date(to_date, "toDate")?;
    if from > to {
        return Err(AppError::validation(
            "fromDate",
            "The start date cannot be after the end date.",
        ));
    }

    let sql = format!(
        "{BLOCK_SELECT}
         WHERE b.calendar_date >= ?1 AND b.calendar_date <= ?2
         ORDER BY b.calendar_date, b.start_time, b.id"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Planned study blocks could not be prepared", error))?;
    let rows = statement
        .query_map(
            params![
                date_only::format_date_only(from),
                date_only::format_date_only(to)
            ],
            map_block_row,
        )
        .map_err(|error| AppError::database("Planned study blocks could not be queried", error))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Planned study blocks could not be read", error))
}

pub fn create_block(
    connection: &mut Connection,
    input: PlannedBlockInput,
) -> Result<PlannedBlock, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The planned-block transaction could not be started", error)
    })?;
    let validated = validate_block_input(&transaction, input, None)?;

    transaction
        .execute(
            "INSERT INTO planned_study_blocks (
                title, topic_id, calendar_date, start_time, end_time,
                status, notes, linked_session_id, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                validated.title,
                validated.topic_id,
                validated.calendar_date,
                validated.start_time,
                validated.end_time,
                validated.status,
                validated.notes,
                validated.linked_session_id,
            ],
        )
        .map_err(|error| AppError::database("The planned study block could not be saved", error))?;

    let block_id = transaction.last_insert_rowid();
    let block = load_block(&transaction, block_id)?;

    transaction.commit().map_err(|error| {
        AppError::database("The planned study block could not be committed", error)
    })?;

    Ok(block)
}

pub fn update_block(
    connection: &mut Connection,
    block_id: i64,
    input: PlannedBlockInput,
) -> Result<PlannedBlock, AppError> {
    validate_id(block_id, "blockId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The planned-block transaction could not be started", error)
    })?;
    let existing = load_block(&transaction, block_id)?;
    let existing_topic_id = existing.topic.as_ref().map(|topic| topic.id);
    let validated = validate_block_input(&transaction, input, existing_topic_id)?;

    let changed = transaction
        .execute(
            "UPDATE planned_study_blocks
             SET title = ?1,
                 topic_id = ?2,
                 calendar_date = ?3,
                 start_time = ?4,
                 end_time = ?5,
                 status = ?6,
                 notes = ?7,
                 linked_session_id = ?8,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?9",
            params![
                validated.title,
                validated.topic_id,
                validated.calendar_date,
                validated.start_time,
                validated.end_time,
                validated.status,
                validated.notes,
                validated.linked_session_id,
                block_id,
            ],
        )
        .map_err(|error| {
            AppError::database("The planned study block could not be updated", error)
        })?;

    if changed != 1 {
        return Err(AppError::block_not_found());
    }

    let block = load_block(&transaction, block_id)?;

    transaction.commit().map_err(|error| {
        AppError::database("The planned study block could not be committed", error)
    })?;

    Ok(block)
}

pub fn update_block_status(
    connection: &mut Connection,
    block_id: i64,
    status: &str,
) -> Result<PlannedBlock, AppError> {
    validate_id(block_id, "blockId")?;
    validate_block_status(status)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The planned-block transaction could not be started", error)
    })?;
    load_block(&transaction, block_id)?;

    transaction
        .execute(
            "UPDATE planned_study_blocks
             SET status = ?1,
                 linked_session_id = CASE WHEN ?1 = 'completed' THEN linked_session_id ELSE NULL END,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![status, block_id],
        )
        .map_err(|error| AppError::database("The planned block status could not be updated", error))?;

    let block = load_block(&transaction, block_id)?;

    transaction.commit().map_err(|error| {
        AppError::database("The planned block status could not be committed", error)
    })?;

    Ok(block)
}

pub fn link_block_session(
    connection: &mut Connection,
    block_id: i64,
    session_id: Option<i64>,
) -> Result<PlannedBlock, AppError> {
    validate_id(block_id, "blockId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The planned-block transaction could not be started", error)
    })?;
    let block = load_block(&transaction, block_id)?;

    if let Some(session_id) = session_id {
        validate_id(session_id, "linkedSessionId")?;
        if block.status != "completed" {
            return Err(AppError::block_not_completed());
        }
        validate_linked_session(&transaction, &block.calendar_date, session_id)?;
    }

    transaction
        .execute(
            "UPDATE planned_study_blocks
             SET linked_session_id = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![session_id, block_id],
        )
        .map_err(|error| {
            AppError::database("The planned block link could not be updated", error)
        })?;

    let block = load_block(&transaction, block_id)?;
    transaction.commit().map_err(|error| {
        AppError::database("The planned block link could not be committed", error)
    })?;

    Ok(block)
}

pub fn delete_block(connection: &mut Connection, block_id: i64) -> Result<(), AppError> {
    validate_id(block_id, "blockId")?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The planned-block transaction could not be started", error)
    })?;

    let changed = transaction
        .execute("DELETE FROM planned_study_blocks WHERE id = ?1", [block_id])
        .map_err(|error| AppError::database("The planned block could not be deleted", error))?;

    if changed != 1 {
        return Err(AppError::block_not_found());
    }

    transaction.commit().map_err(|error| {
        AppError::database("The planned-block deletion could not be committed", error)
    })
}

pub(crate) fn load_task(
    connection: &Connection,
    task_id: i64,
    today: NaiveDate,
) -> Result<ChecklistTask, AppError> {
    let sql = format!("{TASK_SELECT} WHERE t.id = ?1");
    query_tasks(connection, &sql, [task_id], today)?
        .into_iter()
        .next()
        .ok_or_else(AppError::task_not_found)
}

fn query_tasks<P: rusqlite::Params>(
    connection: &Connection,
    sql: &str,
    params: P,
    today: NaiveDate,
) -> Result<Vec<ChecklistTask>, AppError> {
    let today_text = date_only::format_date_only(today);
    let upcoming_end = date_only::format_date_only(today + Days::new(7));

    let mut statement = connection
        .prepare(sql)
        .map_err(|error| AppError::database("Checklist tasks could not be prepared", error))?;
    let rows = statement
        .query_map(params, |row| map_task_row(row, &today_text, &upcoming_end))
        .map_err(|error| AppError::database("Checklist tasks could not be queried", error))?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("Checklist tasks could not be read", error))
}

fn map_task_row(
    row: &rusqlite::Row<'_>,
    today_text: &str,
    upcoming_end: &str,
) -> rusqlite::Result<ChecklistTask> {
    let topic_id: Option<i64> = row.get(5)?;
    let topic = match topic_id {
        Some(id) => Some(PlannerTopic {
            id,
            code: row.get(6)?,
            title: row.get(7)?,
            archived: row.get::<_, i64>(8)? != 0,
        }),
        None => None,
    };

    let scheduled_date: String = row.get(3)?;
    let deadline_date: Option<String> = row.get(4)?;
    let status: String = row.get(10)?;
    let due_date = deadline_date
        .clone()
        .unwrap_or_else(|| scheduled_date.clone());
    let is_overdue = status == "open" && due_date.as_str() < today_text;
    let is_upcoming =
        status == "open" && due_date.as_str() >= today_text && due_date.as_str() <= upcoming_end;

    Ok(ChecklistTask {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        scheduled_date,
        deadline_date,
        topic,
        priority: row.get(9)?,
        status,
        schedule_anchor: row.get(11)?,
        schedule_offset_days: row.get(12)?,
        completed_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        due_date,
        is_overdue,
        is_upcoming,
    })
}

fn validate_task_input(
    connection: &Connection,
    input: TaskInput,
    existing_topic_id: Option<i64>,
) -> Result<ValidatedTaskInput, AppError> {
    let title = input.title.trim().to_owned();
    if title.is_empty() {
        return Err(AppError::validation("title", "Enter a task title."));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::validation(
            "title",
            format!("Task title must be {MAX_TITLE_CHARS} characters or fewer."),
        ));
    }
    if input.description.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "description",
            format!("Description must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    if !TASK_PRIORITIES.contains(&input.priority.as_str()) {
        return Err(AppError::validation(
            "priority",
            "Choose Low, Normal, High or High-Yield.",
        ));
    }

    if !TASK_ANCHORS.contains(&input.schedule_anchor.as_str()) {
        return Err(AppError::validation(
            "scheduleAnchor",
            "Choose a manual date or an exam-relative date.",
        ));
    }

    let (scheduled_date, schedule_offset_days) = if input.schedule_anchor == "exam" {
        let offset = input.schedule_offset_days.ok_or_else(|| {
            AppError::validation(
                "scheduleOffsetDays",
                "Enter how many days from the exam date this task should be scheduled.",
            )
        })?;
        if !(MIN_SCHEDULE_OFFSET_DAYS..=MAX_SCHEDULE_OFFSET_DAYS).contains(&offset) {
            return Err(AppError::validation(
                "scheduleOffsetDays",
                format!(
                    "Exam offset must be between {MIN_SCHEDULE_OFFSET_DAYS} and {MAX_SCHEDULE_OFFSET_DAYS} days."
                ),
            ));
        }
        let exam_date = date_only::parse_date_only(&settings::load(connection)?.exam_date)
            .ok_or_else(|| AppError::internal("The saved exam date is invalid."))?;
        let scheduled = add_days(exam_date, offset).ok_or_else(|| {
            AppError::validation(
                "scheduleOffsetDays",
                "That exam-relative date falls outside the supported calendar.",
            )
        })?;
        (date_only::format_date_only(scheduled), Some(offset))
    } else {
        if input.schedule_offset_days.is_some() {
            return Err(AppError::validation(
                "scheduleOffsetDays",
                "Manual dates cannot also have an exam offset.",
            ));
        }
        let scheduled = validate_required_date(&input.scheduled_date, "scheduledDate")?;
        (date_only::format_date_only(scheduled), None)
    };

    let deadline_date = input
        .deadline_date
        .map(|value| validate_required_date(&value, "deadlineDate"))
        .transpose()?
        .map(date_only::format_date_only);

    if let Some(topic_id) = input.topic_id {
        validate_id(topic_id, "topicId")?;
        validate_topic_reference(connection, topic_id, existing_topic_id)?;
    }

    Ok(ValidatedTaskInput {
        title,
        description: input.description,
        scheduled_date,
        deadline_date,
        topic_id: input.topic_id,
        priority: input.priority,
        schedule_anchor: input.schedule_anchor,
        schedule_offset_days,
    })
}

struct ValidatedBlockInput {
    title: String,
    topic_id: Option<i64>,
    calendar_date: String,
    start_time: String,
    end_time: String,
    status: String,
    notes: String,
    linked_session_id: Option<i64>,
}

fn validate_block_input(
    connection: &Connection,
    input: PlannedBlockInput,
    existing_topic_id: Option<i64>,
) -> Result<ValidatedBlockInput, AppError> {
    let title = input.title.trim().to_owned();
    if title.is_empty() {
        return Err(AppError::validation(
            "title",
            "Enter a planned-block title.",
        ));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(AppError::validation(
            "title",
            format!("Block title must be {MAX_TITLE_CHARS} characters or fewer."),
        ));
    }

    let calendar_date = validate_required_date(&input.calendar_date, "calendarDate")?;
    let calendar_date_text = date_only::format_date_only(calendar_date);
    let start_minutes = validate_time_of_day(&input.start_time, "startTime")?;
    let end_minutes = validate_time_of_day(&input.end_time, "endTime")?;

    if start_minutes >= end_minutes {
        return Err(AppError::validation(
            "endTime",
            "The end time must be later than the start time on the same date.",
        ));
    }

    validate_block_status(&input.status)?;

    if input.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ));
    }

    if let Some(topic_id) = input.topic_id {
        validate_id(topic_id, "topicId")?;
        validate_topic_reference(connection, topic_id, existing_topic_id)?;
    }

    if let Some(session_id) = input.linked_session_id {
        if input.status != "completed" {
            return Err(AppError::validation(
                "linkedSessionId",
                "Only a completed planned block can link to an actual study session.",
            ));
        }
        validate_id(session_id, "linkedSessionId")?;
        validate_linked_session(connection, &calendar_date_text, session_id)?;
    }

    Ok(ValidatedBlockInput {
        title,
        topic_id: input.topic_id,
        calendar_date: calendar_date_text,
        start_time: input.start_time,
        end_time: input.end_time,
        status: input.status,
        notes: input.notes,
        linked_session_id: input.linked_session_id,
    })
}

fn load_block(connection: &Connection, block_id: i64) -> Result<PlannedBlock, AppError> {
    let sql = format!("{BLOCK_SELECT} WHERE b.id = ?1");
    connection
        .query_row(&sql, [block_id], map_block_row)
        .optional()
        .map_err(|error| AppError::database("The planned study block could not be loaded", error))?
        .ok_or_else(AppError::block_not_found)
}

fn map_block_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlannedBlock> {
    let topic_id: Option<i64> = row.get(2)?;
    let topic = match topic_id {
        Some(id) => Some(PlannerTopic {
            id,
            code: row.get(3)?,
            title: row.get(4)?,
            archived: row.get::<_, i64>(5)? != 0,
        }),
        None => None,
    };

    Ok(PlannedBlock {
        id: row.get(0)?,
        title: row.get(1)?,
        topic,
        calendar_date: row.get(6)?,
        start_time: row.get(7)?,
        end_time: row.get(8)?,
        status: row.get(9)?,
        notes: row.get(10)?,
        linked_session_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn validate_topic_reference(
    connection: &Connection,
    topic_id: i64,
    existing_topic_id: Option<i64>,
) -> Result<(), AppError> {
    let archived: Option<i64> = connection
        .query_row(
            "SELECT is_archived FROM syllabus_topics WHERE id = ?1",
            [topic_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The syllabus topic could not be checked", error))?;

    match archived {
        None => Err(AppError::topic_not_found()),
        Some(1) if existing_topic_id != Some(topic_id) => Err(AppError::topic_not_found()),
        _ => Ok(()),
    }
}

fn validate_linked_session(
    connection: &Connection,
    calendar_date: &str,
    session_id: i64,
) -> Result<(), AppError> {
    let session_date: Option<String> = connection
        .query_row(
            "SELECT session_date FROM study_sessions WHERE id = ?1",
            [session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The study session could not be checked", error))?;

    match session_date {
        None => Err(AppError::validation(
            "linkedSessionId",
            "Choose an existing study session from this planned block's date.",
        )),
        Some(date) if date != calendar_date => Err(AppError::validation(
            "linkedSessionId",
            "Choose a study session recorded on the same local date as this planned block.",
        )),
        Some(_) => Ok(()),
    }
}

fn validate_block_status(status: &str) -> Result<(), AppError> {
    if BLOCK_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::validation(
            "status",
            "Choose planned, completed or cancelled.",
        ))
    }
}

fn validate_time_of_day(value: &str, field: &'static str) -> Result<i64, AppError> {
    let bytes = value.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return Err(AppError::validation(
            field,
            "Enter a 24-hour time in HH:MM format.",
        ));
    }
    if !bytes[..2].iter().all(u8::is_ascii_digit) || !bytes[3..].iter().all(u8::is_ascii_digit) {
        return Err(AppError::validation(
            field,
            "Enter a 24-hour time in HH:MM format.",
        ));
    }

    let hours = i64::from((bytes[0] - b'0') * 10 + (bytes[1] - b'0'));
    let minutes = i64::from((bytes[3] - b'0') * 10 + (bytes[4] - b'0'));
    if hours > 23 || minutes > 59 {
        return Err(AppError::validation(
            field,
            "Enter a real 24-hour time in HH:MM format.",
        ));
    }

    Ok(hours * 60 + minutes)
}

fn validate_id(value: i64, field: &'static str) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::validation(
            field,
            "The selected record is invalid.",
        ));
    }

    Ok(())
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

fn add_days(date: NaiveDate, offset: i64) -> Option<NaiveDate> {
    if offset >= 0 {
        date.checked_add_days(Days::new(offset as u64))
    } else {
        date.checked_sub_days(Days::new((-offset) as u64))
    }
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '!' | '%' | '_') {
            escaped.push('!');
        }
        escaped.push(character);
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, sessions, syllabus};

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

    fn task_input(title: &str, scheduled_date: &str) -> TaskInput {
        TaskInput {
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

    fn block_input(title: &str, calendar_date: &str) -> PlannedBlockInput {
        PlannedBlockInput {
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

    fn count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table count should be readable")
    }

    #[test]
    fn task_crud_preserves_completion_timestamps() {
        let mut connection = connection();
        let topic = topic_id(&connection, "GA-01");
        let mut input = task_input("Revise probability", "2026-11-05");
        input.description = "Work through the exercise sheet.".to_owned();
        input.deadline_date = Some("2026-11-07".to_owned());
        input.topic_id = Some(topic);
        input.priority = "High".to_owned();

        let created = create_task(&mut connection, input, date(2026, 10, 30))
            .expect("task should be created");
        assert_eq!(created.status, "open");
        assert_eq!(created.due_date, "2026-11-07");
        assert_eq!(
            created.topic.as_ref().map(|topic| topic.code.as_str()),
            Some("GA-01")
        );
        assert!(created.completed_at.is_none());

        let updated = update_task(
            &mut connection,
            created.id,
            TaskInput {
                title: "Revise probability and statistics".to_owned(),
                description: "Updated notes".to_owned(),
                scheduled_date: "2026-11-06".to_owned(),
                deadline_date: Some("2026-11-08".to_owned()),
                topic_id: None,
                priority: "High-Yield".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: None,
            },
            date(2026, 10, 30),
        )
        .expect("task should update");
        assert_eq!(updated.title, "Revise probability and statistics");
        assert_eq!(updated.priority, "High-Yield");
        assert_eq!(updated.status, "open");

        let completed = complete_task(&mut connection, created.id, date(2026, 10, 30))
            .expect("task should complete");
        let completed_at = completed
            .completed_at
            .clone()
            .expect("completion timestamp should be stored");
        assert_eq!(completed.status, "completed");

        let completed_again = complete_task(&mut connection, created.id, date(2026, 10, 30))
            .expect("completing twice should be idempotent");
        assert_eq!(
            completed_again.completed_at.as_deref(),
            Some(completed_at.as_str())
        );

        let reopened = reopen_task(&mut connection, created.id, date(2026, 10, 30))
            .expect("task should reopen");
        assert_eq!(reopened.status, "open");
        assert!(reopened.completed_at.is_none());

        delete_task(&mut connection, created.id).expect("task should delete");
        assert_eq!(count(&connection, "checklist_tasks"), 0);
        let error = delete_task(&mut connection, created.id).expect_err("missing task should fail");
        assert_eq!(error.code, "task_not_found");
    }

    #[test]
    fn task_overdue_and_upcoming_indicators_use_deadline_or_scheduled_date() {
        let mut connection = connection();
        let today = date(2026, 10, 30);

        let mut overdue = task_input("Overdue task", "2026-10-29");
        overdue.priority = "High".to_owned();
        create_task(&mut connection, overdue, today).expect("overdue task should save");

        let mut upcoming = task_input("Upcoming task", "2026-11-01");
        upcoming.deadline_date = Some("2026-11-02".to_owned());
        create_task(&mut connection, upcoming, today).expect("upcoming task should save");

        create_task(
            &mut connection,
            task_input("Later task", "2026-11-15"),
            today,
        )
        .expect("later task should save");

        let mut completed_overdue = task_input("Completed old task", "2026-10-28");
        completed_overdue.priority = "Normal".to_owned();
        let completed = create_task(&mut connection, completed_overdue, today)
            .expect("completed task should save");
        complete_task(&mut connection, completed.id, today).expect("task should complete");

        let tasks =
            list_tasks(&connection, TaskFilter::default(), today).expect("tasks should list");
        let overdue_task = tasks
            .iter()
            .find(|task| task.title == "Overdue task")
            .expect("overdue task should exist");
        assert!(overdue_task.is_overdue);
        assert!(!overdue_task.is_upcoming);
        assert_eq!(overdue_task.due_date, "2026-10-29");

        let upcoming_task = tasks
            .iter()
            .find(|task| task.title == "Upcoming task")
            .expect("upcoming task should exist");
        assert!(!upcoming_task.is_overdue);
        assert!(upcoming_task.is_upcoming);
        assert_eq!(upcoming_task.due_date, "2026-11-02");

        let later_task = tasks
            .iter()
            .find(|task| task.title == "Later task")
            .expect("later task should exist");
        assert!(!later_task.is_overdue);
        assert!(!later_task.is_upcoming);

        let completed_task = tasks
            .iter()
            .find(|task| task.title == "Completed old task")
            .expect("completed task should exist");
        assert!(!completed_task.is_overdue);
        assert!(!completed_task.is_upcoming);

        let open = list_tasks(
            &connection,
            TaskFilter {
                status: Some("open".to_owned()),
                ..TaskFilter::default()
            },
            today,
        )
        .expect("open tasks should list");
        assert_eq!(open.len(), 3);
        assert!(open.iter().all(|task| task.status == "open"));

        let searched = list_tasks(
            &connection,
            TaskFilter {
                query: Some("upcoming".to_owned()),
                ..TaskFilter::default()
            },
            today,
        )
        .expect("search should work");
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].title, "Upcoming task");
    }

    #[test]
    fn exam_anchored_task_dates_are_derived_from_saved_settings() {
        let mut connection = connection();
        let input = TaskInput {
            title: "Exam-relative revision".to_owned(),
            description: String::new(),
            scheduled_date: "ignored".to_owned(),
            deadline_date: None,
            topic_id: None,
            priority: "Normal".to_owned(),
            schedule_anchor: "exam".to_owned(),
            schedule_offset_days: Some(-10),
        };

        let task = create_task(&mut connection, input, date(2026, 10, 30))
            .expect("exam-anchored task should save");
        assert_eq!(task.scheduled_date, "2026-12-09");
        assert_eq!(task.schedule_anchor, "exam");
        assert_eq!(task.schedule_offset_days, Some(-10));

        let manual = update_task(
            &mut connection,
            task.id,
            task_input("Manual revision", "2026-11-20"),
            date(2026, 10, 30),
        )
        .expect("task should switch to a manual date");
        assert_eq!(manual.scheduled_date, "2026-11-20");
        assert_eq!(manual.schedule_anchor, "manual");
        assert_eq!(manual.schedule_offset_days, None);

        let error = create_task(
            &mut connection,
            TaskInput {
                title: "Out of range".to_owned(),
                description: String::new(),
                scheduled_date: String::new(),
                deadline_date: None,
                topic_id: None,
                priority: "Normal".to_owned(),
                schedule_anchor: "exam".to_owned(),
                schedule_offset_days: Some(MAX_SCHEDULE_OFFSET_DAYS + 1),
            },
            date(2026, 10, 30),
        )
        .expect_err("out-of-range offset should fail");
        assert_eq!(error.field, Some("scheduleOffsetDays"));

        let error = create_task(
            &mut connection,
            TaskInput {
                title: "Manual with offset".to_owned(),
                description: String::new(),
                scheduled_date: "2026-11-20".to_owned(),
                deadline_date: None,
                topic_id: None,
                priority: "Normal".to_owned(),
                schedule_anchor: "manual".to_owned(),
                schedule_offset_days: Some(3),
            },
            date(2026, 10, 30),
        )
        .expect_err("manual offset should fail");
        assert_eq!(error.field, Some("scheduleOffsetDays"));
    }

    #[test]
    fn task_validation_rejects_invalid_values_without_writing() {
        let mut connection = connection();
        let today = date(2026, 10, 30);

        let invalid = [
            (
                TaskInput {
                    title: String::new(),
                    ..task_input("", "2026-11-01")
                },
                "title",
            ),
            (
                TaskInput {
                    title: "x".repeat(MAX_TITLE_CHARS + 1),
                    ..task_input("", "2026-11-01")
                },
                "title",
            ),
            (task_input("Bad date", "2026-02-30"), "scheduledDate"),
            (
                TaskInput {
                    deadline_date: Some("2026-04-31".to_owned()),
                    ..task_input("Bad deadline", "2026-11-01")
                },
                "deadlineDate",
            ),
            (
                TaskInput {
                    priority: "Urgent".to_owned(),
                    ..task_input("Bad priority", "2026-11-01")
                },
                "priority",
            ),
            (
                TaskInput {
                    schedule_anchor: "relative".to_owned(),
                    ..task_input("Bad anchor", "2026-11-01")
                },
                "scheduleAnchor",
            ),
            (
                TaskInput {
                    topic_id: Some(999_999),
                    ..task_input("Bad topic", "2026-11-01")
                },
                "topicId",
            ),
            (
                TaskInput {
                    title: "Missing offset".to_owned(),
                    schedule_anchor: "exam".to_owned(),
                    schedule_offset_days: None,
                    ..task_input("", "2026-11-01")
                },
                "scheduleOffsetDays",
            ),
        ];

        for (input, expected_field) in invalid {
            let error = create_task(&mut connection, input, today)
                .expect_err("invalid task should be rejected");
            if expected_field == "topicId" {
                assert_eq!(error.code, "topic_not_found");
            } else {
                assert_eq!(error.code, "validation_error");
            }
            assert_eq!(error.field, Some(expected_field));
        }

        assert_eq!(count(&connection, "checklist_tasks"), 0);
    }

    #[test]
    fn rescheduling_only_applies_to_incomplete_tasks_and_clears_the_anchor() {
        let mut connection = connection();
        let today = date(2026, 10, 30);
        let task = create_task(
            &mut connection,
            TaskInput {
                title: "Anchored task".to_owned(),
                description: String::new(),
                scheduled_date: String::new(),
                deadline_date: None,
                topic_id: None,
                priority: "High".to_owned(),
                schedule_anchor: "exam".to_owned(),
                schedule_offset_days: Some(-5),
            },
            today,
        )
        .expect("task should be created");
        assert_eq!(task.scheduled_date, "2026-12-14");

        let moved = reschedule_task(&mut connection, task.id, "2026-11-20", today)
            .expect("open task should be rescheduled");
        assert_eq!(moved.scheduled_date, "2026-11-20");
        assert_eq!(moved.schedule_anchor, "manual");
        assert_eq!(moved.schedule_offset_days, None);

        complete_task(&mut connection, task.id, today).expect("task should complete");
        let error = reschedule_task(&mut connection, task.id, "2026-11-21", today)
            .expect_err("completed task should not reschedule");
        assert_eq!(error.code, "task_completed");
        assert_eq!(
            load_task(&connection, task.id, today)
                .expect("task should reload")
                .scheduled_date,
            "2026-11-20"
        );
    }

    #[test]
    fn planned_block_crud_status_and_explicit_session_link() {
        let mut connection = connection();
        let topic = topic_id(&connection, "MMP-C01");
        let mut input = block_input("Electrodynamics revision", "2026-11-02");
        input.topic_id = Some(topic);
        input.notes = "Work through boundary conditions.".to_owned();

        let created = create_block(&mut connection, input).expect("block should be created");
        assert_eq!(created.status, "planned");
        assert_eq!(
            created.topic.as_ref().map(|topic| topic.code.as_str()),
            Some("MMP-C01")
        );
        assert!(created.linked_session_id.is_none());

        let updated = update_block(
            &mut connection,
            created.id,
            PlannedBlockInput {
                title: "Electrodynamics revision".to_owned(),
                topic_id: Some(topic),
                calendar_date: "2026-11-02".to_owned(),
                start_time: "14:00".to_owned(),
                end_time: "15:30".to_owned(),
                status: "completed".to_owned(),
                notes: "Completed the problem set.".to_owned(),
                linked_session_id: None,
            },
        )
        .expect("block should update");
        assert_eq!(updated.status, "completed");
        assert_eq!(updated.start_time, "14:00");

        let session = sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: Some(topic),
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 5_400,
                notes: String::new(),
            },
        )
        .expect("study session should be created");

        let linked = link_block_session(&mut connection, created.id, Some(session.id))
            .expect("completed block should link");
        assert_eq!(linked.linked_session_id, Some(session.id));

        let cancelled = update_block_status(&mut connection, created.id, "cancelled")
            .expect("status should update");
        assert_eq!(cancelled.status, "cancelled");
        assert!(cancelled.linked_session_id.is_none());

        let error = link_block_session(&mut connection, created.id, Some(session.id))
            .expect_err("a cancelled block cannot link");
        assert_eq!(error.code, "block_not_completed");

        update_block_status(&mut connection, created.id, "completed")
            .expect("status should update");
        link_block_session(&mut connection, created.id, Some(session.id))
            .expect("completed block should link again");
        link_block_session(&mut connection, created.id, None).expect("link should clear");
        assert_eq!(count(&connection, "planned_study_blocks"), 1);

        delete_block(&mut connection, created.id).expect("block should delete");
        assert_eq!(count(&connection, "planned_study_blocks"), 0);
        let error =
            delete_block(&mut connection, created.id).expect_err("missing block should fail");
        assert_eq!(error.code, "block_not_found");
    }

    #[test]
    fn planned_block_validation_rejects_bad_dates_times_status_and_links() {
        let mut connection = connection();
        let session = sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: None,
                session_date: "2026-11-01".to_owned(),
                duration_seconds: 1_800,
                notes: String::new(),
            },
        )
        .expect("study session should be created");

        let invalid = [
            (
                PlannedBlockInput {
                    calendar_date: "2026-02-30".to_owned(),
                    ..block_input("Bad date", "2026-11-02")
                },
                "calendarDate",
            ),
            (
                PlannedBlockInput {
                    start_time: "25:00".to_owned(),
                    ..block_input("Bad start", "2026-11-02")
                },
                "startTime",
            ),
            (
                PlannedBlockInput {
                    end_time: "10:00".to_owned(),
                    ..block_input("Bad end", "2026-11-02")
                },
                "endTime",
            ),
            (
                PlannedBlockInput {
                    status: "done".to_owned(),
                    ..block_input("Bad status", "2026-11-02")
                },
                "status",
            ),
            (
                PlannedBlockInput {
                    topic_id: Some(999_999),
                    ..block_input("Bad topic", "2026-11-02")
                },
                "topicId",
            ),
            (
                PlannedBlockInput {
                    notes: "x".repeat(MAX_NOTES_CHARS + 1),
                    ..block_input("Long notes", "2026-11-02")
                },
                "notes",
            ),
            (
                PlannedBlockInput {
                    linked_session_id: Some(session.id),
                    ..block_input("Planned link", "2026-11-02")
                },
                "linkedSessionId",
            ),
            (
                PlannedBlockInput {
                    status: "completed".to_owned(),
                    linked_session_id: Some(session.id),
                    ..block_input("Wrong date link", "2026-11-02")
                },
                "linkedSessionId",
            ),
        ];

        for (input, expected_field) in invalid {
            let error =
                create_block(&mut connection, input).expect_err("invalid block should be rejected");
            if expected_field == "topicId" {
                assert_eq!(error.code, "topic_not_found");
            } else {
                assert_eq!(error.code, "validation_error");
            }
            assert_eq!(error.field, Some(expected_field));
        }

        assert_eq!(count(&connection, "planned_study_blocks"), 0);
    }

    #[test]
    fn calendar_returns_leap_day_tasks_blocks_and_deadline_lists() {
        let mut connection = connection();
        let today = date(2024, 2, 29);

        create_task(
            &mut connection,
            task_input("Leap-day task", "2024-02-29"),
            today,
        )
        .expect("task should save");
        let mut deadline = task_input("Deadline task", "2024-02-28");
        deadline.deadline_date = Some("2024-02-29".to_owned());
        create_task(&mut connection, deadline, today).expect("deadline task should save");
        create_task(
            &mut connection,
            task_input("Outside task", "2024-03-02"),
            today,
        )
        .expect("outside task should save");
        create_block(&mut connection, block_input("Leap-day block", "2024-02-29"))
            .expect("block should save");

        let data =
            calendar(&connection, "2024-02-28", "2024-03-01", today).expect("calendar should load");
        assert_eq!(data.tasks.len(), 2);
        assert_eq!(data.tasks[0].scheduled_date, "2024-02-28");
        assert_eq!(data.tasks[1].scheduled_date, "2024-02-29");
        assert_eq!(data.blocks.len(), 1);
        assert_eq!(data.blocks[0].calendar_date, "2024-02-29");
        assert_eq!(data.deadlines.len(), 1);
        assert_eq!(
            data.deadlines[0].deadline_date.as_deref(),
            Some("2024-02-29")
        );
        assert_eq!(data.upcoming_deadlines.len(), 1);
        assert!(data.overdue_tasks.is_empty());
    }

    #[test]
    fn agenda_separates_today_tasks_blocks_deadlines_and_overdue_items() {
        let mut connection = connection();
        let today = date(2026, 10, 30);

        create_task(
            &mut connection,
            task_input("Today task", "2026-10-30"),
            today,
        )
        .expect("today task should save");
        let mut upcoming = task_input("Upcoming deadline", "2026-11-01");
        upcoming.deadline_date = Some("2026-11-02".to_owned());
        create_task(&mut connection, upcoming, today).expect("upcoming task should save");
        create_task(
            &mut connection,
            task_input("Overdue task", "2026-10-25"),
            today,
        )
        .expect("overdue task should save");
        create_block(&mut connection, block_input("Today block", "2026-10-30"))
            .expect("today block should save");
        create_block(&mut connection, block_input("Tomorrow block", "2026-10-31"))
            .expect("tomorrow block should save");

        let agenda = agenda(&connection, today).expect("agenda should load");
        assert_eq!(agenda.today_tasks.len(), 1);
        assert_eq!(agenda.today_tasks[0].title, "Today task");
        assert_eq!(agenda.today_blocks.len(), 1);
        assert_eq!(agenda.today_blocks[0].title, "Today block");
        assert_eq!(agenda.upcoming_deadlines.len(), 1);
        assert_eq!(agenda.upcoming_deadlines[0].title, "Upcoming deadline");
        assert_eq!(agenda.overdue_tasks.len(), 1);
        assert_eq!(agenda.overdue_tasks[0].title, "Overdue task");
    }

    #[test]
    fn planned_blocks_never_inflate_recorded_study_time() {
        let mut connection = connection();
        let today = date(2026, 11, 2);

        sessions::create_manual(
            &mut connection,
            sessions::StudySessionInput {
                topic_id: None,
                session_date: "2026-11-02".to_owned(),
                duration_seconds: 1_800,
                notes: String::new(),
            },
        )
        .expect("study session should save");

        let before = sessions::totals(&connection, "2026-11-02", "2026-11-02", today, None)
            .expect("totals should load");
        assert_eq!(before.range_seconds, 1_800);

        let block = create_block(&mut connection, block_input("Two-hour plan", "2026-11-02"))
            .expect("planned block should save");
        update_block_status(&mut connection, block.id, "completed").expect("block should complete");

        let after = sessions::totals(&connection, "2026-11-02", "2026-11-02", today, None)
            .expect("totals should load again");
        assert_eq!(after.range_seconds, 1_800);
        assert_eq!(after.range_session_count, 1);
    }

    #[test]
    fn database_constraints_reject_invalid_task_and_block_rows() {
        let connection = connection();
        let sql = [
            "INSERT INTO checklist_tasks (
                title, description, scheduled_date, deadline_date, priority, status,
                schedule_anchor, schedule_offset_days, completed_at, created_at, updated_at
             ) VALUES (
                '', '', '2026-11-01', NULL, 'Normal', 'open',
                'manual', NULL, NULL, '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z'
             )",
            "INSERT INTO checklist_tasks (
                title, description, scheduled_date, deadline_date, priority, status,
                schedule_anchor, schedule_offset_days, completed_at, created_at, updated_at
             ) VALUES (
                'Bad date', '', '2026-02-30', NULL, 'Normal', 'open',
                'manual', NULL, NULL, '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z'
             )",
            "INSERT INTO checklist_tasks (
                title, description, scheduled_date, deadline_date, priority, status,
                schedule_anchor, schedule_offset_days, completed_at, created_at, updated_at
             ) VALUES (
                'Bad completion', '', '2026-11-01', NULL, 'Normal', 'completed',
                'manual', NULL, NULL, '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z'
             )",
            "INSERT INTO planned_study_blocks (
                title, calendar_date, start_time, end_time, status, notes, created_at, updated_at
             ) VALUES (
                'Bad time', '2026-11-01', '14:00', '13:00', 'planned', '',
                '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z'
             )",
            "INSERT INTO planned_study_blocks (
                title, calendar_date, start_time, end_time, status, notes,
                linked_session_id, created_at, updated_at
             ) VALUES (
                'Bad link', '2026-11-01', '10:00', '11:00', 'planned', '',
                999999, '2026-10-30T00:00:00.000Z', '2026-10-30T00:00:00.000Z'
             )",
        ];

        for statement in sql {
            assert!(
                connection.execute(statement, []).is_err(),
                "constraint should reject: {statement}"
            );
        }
    }
}
