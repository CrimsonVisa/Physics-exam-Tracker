use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StickyNote {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub color: String,
    pub is_pinned: bool,
    pub exam_id: Option<i64>,
    pub exam_code: Option<String>,
    pub subject_node_id: Option<i64>,
    pub subject_title: Option<String>,
    pub topic_node_id: Option<i64>,
    pub topic_title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2StickyNoteInput {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    pub color: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub subject_node_id: Option<i64>,
    #[serde(default)]
    pub topic_node_id: Option<i64>,
}

fn note_columns() -> &'static str {
    "n.id, n.title, n.body, n.color, n.is_pinned, n.exam_id, e.code,
     n.subject_node_id, subject.title, n.topic_node_id, topic.title,
     n.created_at, n.updated_at"
}

fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2StickyNote> {
    Ok(V2StickyNote {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        color: row.get(3)?,
        is_pinned: row.get::<_, i64>(4)? == 1,
        exam_id: row.get(5)?,
        exam_code: row.get(6)?,
        subject_node_id: row.get(7)?,
        subject_title: row.get(8)?,
        topic_node_id: row.get(9)?,
        topic_title: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn load_note(connection: &Connection, id: i64) -> Result<Option<V2StickyNote>, AppError> {
    connection
        .query_row(
            &format!(
                "SELECT {} FROM sticky_notes n
                 LEFT JOIN exams e ON e.id = n.exam_id
                 LEFT JOIN exam_syllabus_nodes subject
                    ON subject.exam_id = n.exam_id AND subject.id = n.subject_node_id
                 LEFT JOIN exam_syllabus_nodes topic
                    ON topic.exam_id = n.exam_id AND topic.id = n.topic_node_id
                 WHERE n.id = ?1",
                note_columns()
            ),
            [id],
            row_to_note,
        )
        .optional()
        .map_err(|error| AppError::database("The sticky note could not be read", error))
}

pub fn list_sticky_notes(connection: &Connection) -> Result<Vec<V2StickyNote>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM sticky_notes n
             LEFT JOIN exams e ON e.id = n.exam_id
             LEFT JOIN exam_syllabus_nodes subject
                ON subject.exam_id = n.exam_id AND subject.id = n.subject_node_id
             LEFT JOIN exam_syllabus_nodes topic
                ON topic.exam_id = n.exam_id AND topic.id = n.topic_node_id
             ORDER BY n.is_pinned DESC, n.updated_at DESC, n.id DESC",
            note_columns()
        ))
        .map_err(|error| AppError::database("The sticky note list could not be prepared", error))?;
    let rows = statement
        .query_map([], row_to_note)
        .map_err(|error| AppError::database("The sticky note list could not be read", error))?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(row.map_err(|error| AppError::database("A sticky note row is invalid", error))?);
    }
    Ok(notes)
}

fn validate_associations(
    connection: &Connection,
    input: &V2StickyNoteInput,
) -> Result<(), AppError> {
    if input.subject_node_id.is_some() && input.exam_id.is_none() {
        return Err(AppError::validation(
            "examId",
            "Choose an examination before associating a subject.",
        ));
    }
    if input.topic_node_id.is_some() && input.exam_id.is_none() {
        return Err(AppError::validation(
            "examId",
            "Choose an examination before associating a topic.",
        ));
    }
    let Some(exam_id) = input.exam_id else {
        return Ok(());
    };
    let exam_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id=?1)",
            [exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The note examination could not be checked", error))?;
    if exam_exists == 0 {
        return Err(AppError::validation(
            "examId",
            "The selected examination does not exist.",
        ));
    }
    if let Some(subject_id) = input.subject_node_id {
        let subject: Option<(String, Option<i64>)> = connection
            .query_row(
                "SELECT node_kind, parent_id FROM exam_syllabus_nodes
                 WHERE exam_id=?1 AND id=?2 AND is_archived=0",
                params![exam_id, subject_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("The note subject association could not be checked", error)
            })?;
        let Some((kind, _parent_id)) = subject else {
            return Err(AppError::validation(
                "subjectNodeId",
                "The selected subject is not part of this examination.",
            ));
        };
        if kind != "unit" {
            return Err(AppError::validation(
                "subjectNodeId",
                "Choose a syllabus unit as the subject.",
            ));
        }
    }
    if let Some(topic_id) = input.topic_node_id {
        let topic: Option<(String, Option<i64>)> = connection
            .query_row(
                "SELECT node_kind, parent_id FROM exam_syllabus_nodes
                 WHERE exam_id=?1 AND id=?2 AND is_archived=0",
                params![exam_id, topic_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("The note topic association could not be checked", error)
            })?;
        let Some((kind, parent_id)) = topic else {
            return Err(AppError::validation(
                "topicNodeId",
                "The selected topic is not part of this examination.",
            ));
        };
        if kind != "topic" {
            return Err(AppError::validation(
                "topicNodeId",
                "Choose a syllabus topic for the topic association.",
            ));
        }
        if let Some(subject_id) = input.subject_node_id
            && parent_id != Some(subject_id)
        {
            return Err(AppError::validation(
                "topicNodeId",
                "Choose a topic that belongs to the selected subject.",
            ));
        }
    }
    Ok(())
}

pub fn save_sticky_note(
    connection: &mut Connection,
    input: V2StickyNoteInput,
    now: DateTime<Utc>,
) -> Result<V2StickyNote, AppError> {
    let title = input.title.trim();
    if title.chars().count() > 200 {
        return Err(AppError::validation(
            "title",
            "Note titles must be 200 characters or fewer.",
        ));
    }
    if input.body.chars().count() > 20_000 {
        return Err(AppError::validation(
            "body",
            "Note text must be 20,000 characters or fewer.",
        ));
    }
    if title.is_empty() && input.body.trim().is_empty() {
        return Err(AppError::validation(
            "body",
            "Enter a title or some note text before saving.",
        ));
    }
    if !matches!(
        input.color.as_str(),
        "yellow" | "blue" | "green" | "pink" | "purple" | "orange"
    ) {
        return Err(AppError::validation(
            "color",
            "Choose one of the available note colors.",
        ));
    }
    if let Some(id) = input.id
        && id <= 0
    {
        return Err(AppError::validation("id", "The selected note is invalid."));
    }

    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The sticky note save could not start", error))?;
    validate_associations(&transaction, &input)?;
    let token = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let note_id = match input.id {
        Some(id) => {
            let exists: i64 = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sticky_notes WHERE id=?1)",
                    [id],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    AppError::database("The sticky note could not be checked", error)
                })?;
            if exists == 0 {
                return Err(AppError::validation(
                    "id",
                    "This sticky note no longer exists. Refresh and try again.",
                ));
            }
            transaction
                .execute(
                    "UPDATE sticky_notes
                     SET title=?1, body=?2, color=?3, is_pinned=?4, exam_id=?5,
                         subject_node_id=?6, topic_node_id=?7, updated_at=?8
                     WHERE id=?9",
                    params![
                        title,
                        input.body,
                        input.color,
                        if input.is_pinned { 1 } else { 0 },
                        input.exam_id,
                        input.subject_node_id,
                        input.topic_node_id,
                        token,
                        id
                    ],
                )
                .map_err(|error| {
                    AppError::database("The sticky note could not be updated", error)
                })?;
            id
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO sticky_notes (
                        title, body, color, is_pinned, exam_id, subject_node_id,
                        topic_node_id, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                    params![
                        title,
                        input.body,
                        input.color,
                        if input.is_pinned { 1 } else { 0 },
                        input.exam_id,
                        input.subject_node_id,
                        input.topic_node_id,
                        token
                    ],
                )
                .map_err(|error| {
                    AppError::database("The sticky note could not be created", error)
                })?;
            transaction.last_insert_rowid()
        }
    };
    let saved = load_note(&transaction, note_id)?.ok_or_else(|| {
        AppError::database(
            "The saved sticky note could not be read",
            "the saved row is missing",
        )
    })?;
    transaction.commit().map_err(|error| {
        AppError::database("The sticky note save could not be committed", error)
    })?;
    Ok(saved)
}

pub fn delete_sticky_note(connection: &mut Connection, id: i64) -> Result<(), AppError> {
    if id <= 0 {
        return Err(AppError::validation("id", "The selected note is invalid."));
    }
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The sticky note delete could not start", error))?;
    let deleted = transaction
        .execute("DELETE FROM sticky_notes WHERE id=?1", [id])
        .map_err(|error| AppError::database("The sticky note could not be deleted", error))?;
    if deleted == 0 {
        return Err(AppError::validation(
            "id",
            "This sticky note no longer exists. Refresh and try again.",
        ));
    }
    transaction.commit().map_err(|error| {
        AppError::database("The sticky note delete could not be committed", error)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2State, syllabus};
    use chrono::TimeZone;

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 10, 10, 0, 0)
            .single()
            .expect("timestamp")
            + chrono::Duration::seconds(seconds)
    }

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn input(title: &str, body: &str) -> V2StickyNoteInput {
        V2StickyNoteInput {
            id: None,
            title: title.to_owned(),
            body: body.to_owned(),
            color: "yellow".to_owned(),
            is_pinned: false,
            exam_id: None,
            subject_node_id: None,
            topic_node_id: None,
        }
    }

    #[test]
    fn note_crud_pinning_and_filter_order_are_persistent() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let first = save_sticky_note(connection, input("First", "Body"), at(1))?;
                let second = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        is_pinned: true,
                        color: "blue".to_owned(),
                        ..input("Pinned", "Pinned body")
                    },
                    at(2),
                )?;
                let listed = list_sticky_notes(connection)?;
                assert_eq!(listed.len(), 2);
                assert_eq!(listed[0].id, second.id);
                assert!(listed[0].is_pinned);
                assert_eq!(listed[0].color, "blue");
                assert_eq!(listed[1].id, first.id);

                let updated = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        id: Some(first.id),
                        is_pinned: true,
                        body: "Updated body".to_owned(),
                        ..input("First", "Body")
                    },
                    at(3),
                )?;
                assert_eq!(updated.body, "Updated body");
                assert!(updated.is_pinned);
                assert_eq!(list_sticky_notes(connection)?.len(), 2);

                delete_sticky_note(connection, second.id)?;
                assert_eq!(list_sticky_notes(connection)?.len(), 1);
                Ok::<(), AppError>(())
            })
            .expect("note CRUD");
    }

    #[test]
    fn note_associations_are_validated_and_titles_are_returned() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let jest = syllabus::get_exam_syllabus(
                    connection,
                    syllabus::V2SyllabusExamInput { exam_id: 4 },
                )?;
                let unit = jest
                    .nodes
                    .iter()
                    .find(|node| node.node_kind == "unit")
                    .expect("unit");
                let topic = jest
                    .nodes
                    .iter()
                    .find(|node| node.node_kind == "topic" && node.parent_id == Some(unit.id))
                    .expect("topic");
                let saved = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        exam_id: Some(4),
                        subject_node_id: Some(unit.id),
                        topic_node_id: Some(topic.id),
                        ..input("JEST note", "Associated note")
                    },
                    at(1),
                )?;
                assert_eq!(saved.exam_code.as_deref(), Some("jest-physics"));
                assert_eq!(saved.subject_title.as_deref(), Some(unit.title.as_str()));
                assert_eq!(saved.topic_title.as_deref(), Some(topic.title.as_str()));

                let invalid = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        exam_id: Some(1),
                        subject_node_id: Some(unit.id),
                        ..input("Wrong exam", "Mismatch")
                    },
                    at(2),
                )
                .unwrap_err();
                assert_eq!(invalid.field, Some("subjectNodeId"));

                let missing_exam = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        topic_node_id: Some(topic.id),
                        ..input("No exam", "Missing exam")
                    },
                    at(3),
                )
                .unwrap_err();
                assert_eq!(missing_exam.field, Some("examId"));
                Ok::<(), AppError>(())
            })
            .expect("note associations");
    }

    #[test]
    fn empty_notes_and_invalid_colors_are_rejected() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let empty = save_sticky_note(connection, input("", "   "), at(1)).unwrap_err();
                assert_eq!(empty.field, Some("body"));
                let color = save_sticky_note(
                    connection,
                    V2StickyNoteInput {
                        color: "red".to_owned(),
                        ..input("Title", "Body")
                    },
                    at(2),
                )
                .unwrap_err();
                assert_eq!(color.field, Some("color"));
                Ok::<(), AppError>(())
            })
            .expect("note validation");
    }
}
