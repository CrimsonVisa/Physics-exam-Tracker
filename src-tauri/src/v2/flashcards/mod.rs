use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

mod schedule;
use schedule::{ALGORITHM_VERSION, INITIAL_EASE, Rating, ScheduleState, schedule_review};

const DEFAULT_LIMIT: i64 = 200;
const MAX_LIMIT: i64 = 500;
const DEFAULT_QUEUE_LIMIT: i64 = 100;
const MAX_QUEUE_LIMIT: i64 = 500;
const PREVIEW_CHARS: usize = 180;
const MAX_QUERY_CHARS: usize = 120;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardExamLinkInput {
    pub exam_id: i64,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub front: String,
    pub back: String,
    pub subject_area: String,
    #[serde(default)]
    pub concept_id: Option<i64>,
    #[serde(default)]
    pub exam_links: Vec<V2FlashcardExamLinkInput>,
    pub today: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardFilter {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub subject_area: Option<String>,
    #[serde(default)]
    pub node_id: Option<i64>,
    #[serde(default)]
    pub concept_slug: Option<String>,
    #[serde(default)]
    pub include_archived: Option<bool>,
    #[serde(default)]
    pub due_only: Option<bool>,
    pub today: String,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardStatsInput {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub subject_area: Option<String>,
    #[serde(default)]
    pub node_id: Option<i64>,
    #[serde(default)]
    pub concept_slug: Option<String>,
    #[serde(default)]
    pub include_archived: Option<bool>,
    pub today: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardQueueInput {
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub subject_area: Option<String>,
    #[serde(default)]
    pub node_id: Option<i64>,
    #[serde(default)]
    pub concept_slug: Option<String>,
    #[serde(default)]
    pub due_only: bool,
    pub today: String,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardReviewInput {
    pub flashcard_id: i64,
    pub rating: String,
    pub today: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardArchiveInput {
    pub flashcard_id: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardExamLink {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub syllabus_node_id: Option<i64>,
    pub node_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardSchedule {
    pub due_on: String,
    pub interval_days: i64,
    pub ease_factor: f64,
    pub review_count: i64,
    pub lapse_count: i64,
    pub last_rating: Option<String>,
    pub last_reviewed_on: Option<String>,
    pub last_reviewed_at: Option<String>,
    pub algorithm_version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardSummary {
    pub id: i64,
    pub front_preview: String,
    pub back_preview: String,
    pub subject_area: String,
    pub concept_id: Option<i64>,
    pub concept_title: Option<String>,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub schedule: V2FlashcardSchedule,
    pub exam_links: Vec<V2FlashcardExamLink>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardReview {
    pub id: i64,
    pub reviewed_on: String,
    pub reviewed_at: String,
    pub rating: String,
    pub scheduled_due_on: String,
    pub next_due_on: String,
    pub interval_before_days: i64,
    pub interval_after_days: i64,
    pub ease_before: f64,
    pub ease_after: f64,
    pub algorithm_version: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardDetail {
    pub flashcard: V2FlashcardSummary,
    pub front: String,
    pub back: String,
    pub reviews: Vec<V2FlashcardReview>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardLibrary {
    pub cards: Vec<V2FlashcardSummary>,
    pub total: i64,
    pub subjects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardReviewCard {
    pub id: i64,
    pub front: String,
    pub back: String,
    pub subject_area: String,
    pub concept_title: Option<String>,
    pub schedule: V2FlashcardSchedule,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardQueue {
    pub cards: Vec<V2FlashcardReviewCard>,
    pub remaining: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardReviewResult {
    pub review: V2FlashcardReview,
    pub schedule: V2FlashcardSchedule,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2FlashcardStats {
    pub total_cards: i64,
    pub due_cards: i64,
    pub reviewed_cards: i64,
    pub new_cards: i64,
    pub total_reviews: i64,
    pub reviews_last_30_days: i64,
    pub successful_last_30_days: i64,
    pub retention_percent: Option<f64>,
}

struct FilterSql {
    where_sql: String,
    values: Vec<Value>,
}

pub fn list_flashcards(
    connection: &Connection,
    filter: V2FlashcardFilter,
) -> Result<V2FlashcardLibrary, AppError> {
    let (_today, common) = build_filter(connection, &filter)?;
    let limit = validate_limit(filter.limit, DEFAULT_LIMIT, MAX_LIMIT, "limit")?;
    let offset = filter.offset.unwrap_or(0);
    if offset < 0 {
        return Err(AppError::validation("offset", "Offset cannot be negative."));
    }
    let sort = filter.sort.as_deref().unwrap_or("due");
    let order = match sort {
        "due" => "s.due_on ASC, f.front COLLATE NOCASE, f.id",
        "created" => "f.created_at DESC, f.id DESC",
        "updated" => "f.updated_at DESC, f.id DESC",
        "front" => "f.front COLLATE NOCASE, f.id",
        "subject" => "f.subject_area COLLATE NOCASE, f.front COLLATE NOCASE, f.id",
        _ => {
            return Err(AppError::validation(
                "sort",
                "Choose due, created, updated, front or subject sorting.",
            ));
        }
    };

    let total: i64 = connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM flashcards f
                 JOIN flashcard_schedule s ON s.flashcard_id=f.id
                 WHERE {}",
                common.where_sql
            ),
            params_from_iter(common.values.iter()),
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The flashcard count could not be read", error))?;

    let mut values = common.values.clone();
    values.push(Value::Integer(limit));
    values.push(Value::Integer(offset));
    let sql = format!(
        "SELECT f.id, f.front, f.back, f.subject_area, f.concept_id, c.title,
                f.is_archived, f.archived_at,
                s.due_on, s.interval_days, s.ease_factor, s.review_count, s.lapse_count,
                s.last_rating, s.last_reviewed_on, s.last_reviewed_at, s.algorithm_version,
                f.created_at, f.updated_at
         FROM flashcards f
         JOIN flashcard_schedule s ON s.flashcard_id=f.id
         LEFT JOIN concepts c ON c.id=f.concept_id
         WHERE {}
         ORDER BY {order}
         LIMIT ? OFFSET ?",
        common.where_sql
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("The flashcard list could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), summary_from_row)
        .map_err(|error| AppError::database("The flashcard list could not run", error))?;
    let mut cards = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A flashcard row is invalid", error))?;
    attach_exam_links(connection, &mut cards)?;

    let subjects = list_subjects(connection)?;
    Ok(V2FlashcardLibrary {
        cards,
        total,
        subjects,
    })
}

pub fn get_flashcard_detail(
    connection: &Connection,
    flashcard_id: i64,
) -> Result<V2FlashcardDetail, AppError> {
    let mut summary = load_summary(connection, flashcard_id)?;
    let mut summaries = vec![summary.clone()];
    attach_exam_links(connection, &mut summaries)?;
    summary = summaries.remove(0);

    let (front, back): (String, String) = connection
        .query_row(
            "SELECT front, back FROM flashcards WHERE id=?1",
            [flashcard_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| AppError::database("The flashcard content could not be read", error))?;

    let mut statement = connection
        .prepare(
            "SELECT id, reviewed_on, reviewed_at, rating, scheduled_due_on, next_due_on,
                    interval_before_days, interval_after_days, ease_before, ease_after,
                    algorithm_version
             FROM flashcard_reviews
             WHERE flashcard_id=?1
             ORDER BY reviewed_at DESC, id DESC
             LIMIT 50",
        )
        .map_err(|error| {
            AppError::database("The flashcard history could not be prepared", error)
        })?;
    let rows = statement
        .query_map([flashcard_id], review_from_row)
        .map_err(|error| AppError::database("The flashcard history could not run", error))?;
    let reviews = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A flashcard history row is invalid", error))?;

    Ok(V2FlashcardDetail {
        flashcard: summary,
        front,
        back,
        reviews,
    })
}

pub fn save_flashcard(
    connection: &mut Connection,
    input: V2FlashcardInput,
    now: DateTime<Utc>,
) -> Result<V2FlashcardDetail, AppError> {
    let front = required_text(&input.front, 20_000, "front", "Card front")?;
    let back = required_text(&input.back, 20_000, "back", "Card back")?;
    let subject = required_text(&input.subject_area, 200, "subjectArea", "Subject")?;
    let today = parse_date(&input.today, "today")?;
    if let Some(concept_id) = input.concept_id {
        validate_concept(connection, concept_id)?;
    }
    validate_exam_links(connection, &input.exam_links)?;

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The flashcard save transaction could not start", error)
    })?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let card_id = if let Some(card_id) = input.id {
        let changed = transaction
            .execute(
                "UPDATE flashcards
                 SET front=?1, back=?2, subject_area=?3, concept_id=?4, updated_at=?5
                 WHERE id=?6",
                params![front, back, subject, input.concept_id, now_text, card_id],
            )
            .map_err(|error| AppError::database("The flashcard could not be updated", error))?;
        if changed != 1 {
            return Err(AppError::flashcard_not_found());
        }
        card_id
    } else {
        transaction
            .execute(
                "INSERT INTO flashcards(
                    front, back, subject_area, concept_id, is_archived, archived_at, created_at, updated_at
                 ) VALUES(?1,?2,?3,?4,0,NULL,?5,?5)",
                params![front, back, subject, input.concept_id, now_text],
            )
            .map_err(|error| AppError::database("The flashcard could not be created", error))?;
        let card_id = transaction.last_insert_rowid();
        transaction
            .execute(
                "INSERT INTO flashcard_schedule(
                    flashcard_id, due_on, interval_days, ease_factor, review_count, lapse_count,
                    last_rating, last_reviewed_on, last_reviewed_at, algorithm_version,
                    created_at, updated_at
                 ) VALUES(?1,?2,0,?3,0,0,NULL,NULL,NULL,?4,?5,?5)",
                params![
                    card_id,
                    today.format("%Y-%m-%d").to_string(),
                    INITIAL_EASE,
                    ALGORITHM_VERSION,
                    now_text
                ],
            )
            .map_err(|error| {
                AppError::database("The flashcard schedule could not be created", error)
            })?;
        card_id
    };

    transaction
        .execute(
            "DELETE FROM flashcard_exam_links WHERE flashcard_id=?1",
            [card_id],
        )
        .map_err(|error| {
            AppError::database("The flashcard exam links could not be replaced", error)
        })?;
    for link in &input.exam_links {
        transaction
            .execute(
                "INSERT INTO flashcard_exam_links(
                    flashcard_id, exam_id, syllabus_node_id, created_at
                 ) VALUES(?1,?2,?3,?4)",
                params![card_id, link.exam_id, link.syllabus_node_id, now_text],
            )
            .map_err(|error| {
                AppError::database("A flashcard exam link could not be saved", error)
            })?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("The flashcard save could not be committed", error))?;

    get_flashcard_detail(connection, card_id)
}

pub fn set_flashcard_archived(
    connection: &mut Connection,
    input: V2FlashcardArchiveInput,
    now: DateTime<Utc>,
) -> Result<V2FlashcardDetail, AppError> {
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let archived_at = if input.archived {
        Some(now_text.clone())
    } else {
        None
    };
    let archived_flag = if input.archived { 1_i64 } else { 0_i64 };
    let changed = connection
        .execute(
            "UPDATE flashcards
             SET is_archived=?1, archived_at=?2, updated_at=?3
             WHERE id=?4",
            params![archived_flag, archived_at, now_text, input.flashcard_id],
        )
        .map_err(|error| {
            AppError::database("The flashcard archive state could not be saved", error)
        })?;
    if changed != 1 {
        return Err(AppError::flashcard_not_found());
    }
    get_flashcard_detail(connection, input.flashcard_id)
}

pub fn review_queue(
    connection: &Connection,
    input: V2FlashcardQueueInput,
) -> Result<V2FlashcardQueue, AppError> {
    let filter = V2FlashcardFilter {
        query: None,
        exam_id: input.exam_id,
        subject_area: input.subject_area,
        node_id: input.node_id,
        concept_slug: input.concept_slug,
        include_archived: Some(false),
        due_only: Some(input.due_only),
        today: input.today,
        sort: None,
        limit: None,
        offset: None,
    };
    let (today, common) = build_filter(connection, &filter)?;
    let limit = validate_limit(input.limit, DEFAULT_QUEUE_LIMIT, MAX_QUEUE_LIMIT, "limit")?;

    let total: i64 = connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM flashcards f
                 JOIN flashcard_schedule s ON s.flashcard_id=f.id
                 WHERE {}",
                common.where_sql
            ),
            params_from_iter(common.values.iter()),
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::database("The flashcard queue count could not be read", error)
        })?;

    let mut values = common.values.clone();
    let order = if input.due_only {
        "s.due_on ASC, f.front COLLATE NOCASE, f.id".to_owned()
    } else {
        values.push(Value::Text(today.format("%Y-%m-%d").to_string()));
        "CASE WHEN s.due_on <= ? THEN 0 ELSE 1 END, s.due_on, f.front COLLATE NOCASE, f.id"
            .to_owned()
    };
    values.push(Value::Integer(limit));
    let sql = format!(
        "SELECT f.id, f.front, f.back, f.subject_area, c.title,
                s.due_on, s.interval_days, s.ease_factor, s.review_count, s.lapse_count,
                s.last_rating, s.last_reviewed_on, s.last_reviewed_at, s.algorithm_version
         FROM flashcards f
         JOIN flashcard_schedule s ON s.flashcard_id=f.id
         LEFT JOIN concepts c ON c.id=f.concept_id
         WHERE {}
         ORDER BY {order}
         LIMIT ?",
        common.where_sql
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("The flashcard queue could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok(V2FlashcardReviewCard {
                id: row.get(0)?,
                front: row.get(1)?,
                back: row.get(2)?,
                subject_area: row.get(3)?,
                concept_title: row.get(4)?,
                schedule: schedule_from_row(row, 5)?,
            })
        })
        .map_err(|error| AppError::database("The flashcard queue could not run", error))?;
    let cards = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A flashcard queue row is invalid", error))?;
    Ok(V2FlashcardQueue {
        remaining: (total - cards.len() as i64).max(0),
        cards,
    })
}

pub fn review_flashcard(
    connection: &mut Connection,
    input: V2FlashcardReviewInput,
    now: DateTime<Utc>,
) -> Result<V2FlashcardReviewResult, AppError> {
    let rating = Rating::parse(&input.rating)
        .ok_or_else(|| AppError::validation("rating", "Choose Again, Hard, Good or Easy."))?;
    let today = parse_date(&input.today, "today")?;
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The review transaction could not start", error))?;
    let row: Option<(bool, V2FlashcardSchedule)> = transaction
        .query_row(
            "SELECT f.is_archived,
                    s.due_on, s.interval_days, s.ease_factor, s.review_count, s.lapse_count,
                    s.last_rating, s.last_reviewed_on, s.last_reviewed_at, s.algorithm_version
             FROM flashcards f
             JOIN flashcard_schedule s ON s.flashcard_id=f.id
             WHERE f.id=?1",
            [input.flashcard_id],
            |row| Ok((row.get::<_, i64>(0)? == 1, schedule_from_row(row, 1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("The flashcard schedule could not be read", error))?;
    let (archived, current) = row.ok_or_else(AppError::flashcard_not_found)?;
    if archived {
        return Err(AppError::validation(
            "flashcardId",
            "Archived flashcards cannot be reviewed.",
        ));
    }
    if let Some(last_reviewed_on) = current.last_reviewed_on.as_deref()
        && today.format("%Y-%m-%d").to_string().as_str() < last_reviewed_on
    {
        return Err(AppError::validation(
            "today",
            "The review date cannot be earlier than the last review date.",
        ));
    }
    let outcome = schedule_review(
        ScheduleState {
            interval_days: current.interval_days,
            ease_factor: current.ease_factor,
            review_count: current.review_count,
            lapse_count: current.lapse_count,
        },
        rating,
        today,
    );
    let reviewed_on = today.format("%Y-%m-%d").to_string();
    let next_due_on = outcome.due_on.format("%Y-%m-%d").to_string();
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction
        .execute(
            "UPDATE flashcard_schedule
             SET due_on=?1, interval_days=?2, ease_factor=?3, review_count=?4, lapse_count=?5,
                 last_rating=?6, last_reviewed_on=?7, last_reviewed_at=?8, algorithm_version=?9,
                 updated_at=?10
             WHERE flashcard_id=?11",
            params![
                next_due_on,
                outcome.interval_days,
                outcome.ease_factor,
                outcome.review_count,
                outcome.lapse_count,
                rating.as_str(),
                reviewed_on,
                now_text,
                ALGORITHM_VERSION,
                now_text,
                input.flashcard_id
            ],
        )
        .map_err(|error| {
            AppError::database("The flashcard schedule could not be updated", error)
        })?;
    transaction
        .execute(
            "INSERT INTO flashcard_reviews(
                flashcard_id, reviewed_on, reviewed_at, rating, scheduled_due_on, next_due_on,
                interval_before_days, interval_after_days, ease_before, ease_after,
                algorithm_version, created_at
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?3)",
            params![
                input.flashcard_id,
                reviewed_on,
                now_text,
                rating.as_str(),
                current.due_on,
                next_due_on,
                current.interval_days,
                outcome.interval_days,
                current.ease_factor,
                outcome.ease_factor,
                ALGORITHM_VERSION
            ],
        )
        .map_err(|error| AppError::database("The flashcard review could not be recorded", error))?;
    let review_id = transaction.last_insert_rowid();
    transaction.commit().map_err(|error| {
        AppError::database("The flashcard review could not be committed", error)
    })?;

    Ok(V2FlashcardReviewResult {
        review: V2FlashcardReview {
            id: review_id,
            reviewed_on: reviewed_on.clone(),
            reviewed_at: now_text.clone(),
            rating: rating.as_str().to_owned(),
            scheduled_due_on: current.due_on,
            next_due_on,
            interval_before_days: current.interval_days,
            interval_after_days: outcome.interval_days,
            ease_before: current.ease_factor,
            ease_after: outcome.ease_factor,
            algorithm_version: ALGORITHM_VERSION,
        },
        schedule: V2FlashcardSchedule {
            due_on: outcome.due_on.format("%Y-%m-%d").to_string(),
            interval_days: outcome.interval_days,
            ease_factor: outcome.ease_factor,
            review_count: outcome.review_count,
            lapse_count: outcome.lapse_count,
            last_rating: Some(rating.as_str().to_owned()),
            last_reviewed_on: Some(reviewed_on),
            last_reviewed_at: Some(now_text),
            algorithm_version: ALGORITHM_VERSION,
        },
    })
}

pub fn flashcard_stats(
    connection: &Connection,
    input: V2FlashcardStatsInput,
) -> Result<V2FlashcardStats, AppError> {
    let filter = V2FlashcardFilter {
        query: input.query,
        exam_id: input.exam_id,
        subject_area: input.subject_area,
        node_id: input.node_id,
        concept_slug: input.concept_slug,
        include_archived: input.include_archived,
        due_only: Some(false),
        today: input.today,
        sort: None,
        limit: None,
        offset: None,
    };
    let (today, common) = build_filter(connection, &filter)?;
    let today_text = today.format("%Y-%m-%d").to_string();
    let mut values = vec![Value::Text(today_text.clone())];
    values.extend(common.values.iter().cloned());
    let stats: (i64, i64, i64, i64, i64) = connection
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN s.due_on <= ?1 THEN 1 ELSE 0 END),0),
                        COALESCE(SUM(CASE WHEN s.review_count > 0 THEN 1 ELSE 0 END),0),
                        COALESCE(SUM(CASE WHEN s.review_count = 0 THEN 1 ELSE 0 END),0),
                        COALESCE(SUM(s.review_count),0)
                 FROM flashcards f
                 JOIN flashcard_schedule s ON s.flashcard_id=f.id
                 WHERE {}",
                common.where_sql
            ),
            params_from_iter(values.iter()),
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|error| AppError::database("The flashcard statistics could not be read", error))?;

    let thirty_days_ago = today
        .checked_sub_days(chrono::Days::new(30))
        .unwrap_or(today)
        .format("%Y-%m-%d")
        .to_string();
    let mut review_values = vec![
        Value::Text(thirty_days_ago),
        Value::Text(today.format("%Y-%m-%d").to_string()),
    ];
    review_values.extend(common.values.iter().cloned());
    let (reviews_last_30_days, successful_last_30_days): (i64, i64) = connection
        .query_row(
            &format!(
                "SELECT COUNT(*),
                        COALESCE(SUM(CASE WHEN r.rating IN ('good','easy') THEN 1 ELSE 0 END),0)
                 FROM flashcard_reviews r
                 JOIN flashcards f ON f.id=r.flashcard_id
                 JOIN flashcard_schedule s ON s.flashcard_id=f.id
                 WHERE r.reviewed_on BETWEEN ?1 AND ?2 AND {}",
                common.where_sql
            ),
            params_from_iter(review_values.iter()),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| AppError::database("The flashcard retention could not be read", error))?;

    let retention_percent = if reviews_last_30_days > 0 {
        Some((successful_last_30_days as f64 / reviews_last_30_days as f64) * 100.0)
    } else {
        None
    };

    Ok(V2FlashcardStats {
        total_cards: stats.0,
        due_cards: stats.1,
        reviewed_cards: stats.2,
        new_cards: stats.3,
        total_reviews: stats.4,
        reviews_last_30_days,
        successful_last_30_days,
        retention_percent,
    })
}

fn build_filter(
    connection: &Connection,
    filter: &V2FlashcardFilter,
) -> Result<(NaiveDate, FilterSql), AppError> {
    let today = parse_date(&filter.today, "today")?;
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    if !filter.include_archived.unwrap_or(false) {
        clauses.push("f.is_archived=0".to_owned());
    }
    if let Some(exam_id) = filter.exam_id {
        validate_exam(connection, exam_id)?;
        clauses.push(
            "(NOT EXISTS (SELECT 1 FROM flashcard_exam_links l WHERE l.flashcard_id=f.id)
              OR EXISTS (SELECT 1 FROM flashcard_exam_links l WHERE l.flashcard_id=f.id AND l.exam_id=?))"
                .to_owned(),
        );
        values.push(Value::Integer(exam_id));
    }
    if let Some(subject) = filter
        .subject_area
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if subject.chars().count() > 200 {
            return Err(AppError::validation(
                "subjectArea",
                "Subject filter must be 200 characters or fewer.",
            ));
        }
        clauses.push("f.subject_area = ? COLLATE NOCASE".to_owned());
        values.push(Value::Text(subject.to_owned()));
    }
    if let Some(node_id) = filter.node_id {
        let exam_id = filter.exam_id.ok_or_else(|| {
            AppError::validation(
                "examId",
                "Choose an examination before filtering by syllabus node.",
            )
        })?;
        validate_node(connection, exam_id, node_id)?;
        clauses.push(
            "EXISTS (
                WITH RECURSIVE subtree(node_id) AS (
                    SELECT id FROM exam_syllabus_nodes WHERE exam_id=? AND id=?
                    UNION ALL
                    SELECT n.id FROM exam_syllabus_nodes n
                    JOIN subtree s ON n.parent_id=s.node_id
                    WHERE n.exam_id=?
                )
                SELECT 1 FROM flashcard_exam_links l
                JOIN subtree s ON s.node_id=l.syllabus_node_id
                WHERE l.flashcard_id=f.id AND l.exam_id=?
            )"
            .to_owned(),
        );
        values.push(Value::Integer(exam_id));
        values.push(Value::Integer(node_id));
        values.push(Value::Integer(exam_id));
        values.push(Value::Integer(exam_id));
    }
    if let Some(slug) = filter
        .concept_slug
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if slug.chars().count() > 120 {
            return Err(AppError::validation(
                "conceptSlug",
                "Concept filter is too long.",
            ));
        }
        clauses.push(
            "EXISTS (SELECT 1 FROM concepts c WHERE c.id=f.concept_id AND c.slug=?)".to_owned(),
        );
        values.push(Value::Text(slug.to_owned()));
    }
    if let Some(query) = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if query.chars().count() > MAX_QUERY_CHARS {
            return Err(AppError::validation(
                "query",
                "Search text must be 120 characters or fewer.",
            ));
        }
        let pattern = format!("%{}%", escape_like(query));
        clauses.push(
            "(f.front LIKE ? ESCAPE '\\' COLLATE NOCASE
              OR f.back LIKE ? ESCAPE '\\' COLLATE NOCASE
              OR f.subject_area LIKE ? ESCAPE '\\' COLLATE NOCASE)"
                .to_owned(),
        );
        for _ in 0..3 {
            values.push(Value::Text(pattern.clone()));
        }
    }
    if filter.due_only.unwrap_or(false) {
        clauses.push("s.due_on <= ?".to_owned());
        values.push(Value::Text(today.format("%Y-%m-%d").to_string()));
    }
    if clauses.is_empty() {
        clauses.push("1=1".to_owned());
    }
    Ok((
        today,
        FilterSql {
            where_sql: clauses.join(" AND "),
            values,
        },
    ))
}

fn load_summary(
    connection: &Connection,
    flashcard_id: i64,
) -> Result<V2FlashcardSummary, AppError> {
    connection
        .query_row(
            "SELECT f.id, f.front, f.back, f.subject_area, f.concept_id, c.title,
                    f.is_archived, f.archived_at,
                    s.due_on, s.interval_days, s.ease_factor, s.review_count, s.lapse_count,
                    s.last_rating, s.last_reviewed_on, s.last_reviewed_at, s.algorithm_version,
                    f.created_at, f.updated_at
             FROM flashcards f
             JOIN flashcard_schedule s ON s.flashcard_id=f.id
             LEFT JOIN concepts c ON c.id=f.concept_id
             WHERE f.id=?1",
            [flashcard_id],
            summary_from_row,
        )
        .optional()
        .map_err(|error| AppError::database("The flashcard could not be read", error))?
        .ok_or_else(AppError::flashcard_not_found)
}

fn summary_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2FlashcardSummary> {
    let front: String = row.get(1)?;
    let back: String = row.get(2)?;
    Ok(V2FlashcardSummary {
        id: row.get(0)?,
        front_preview: preview(&front),
        back_preview: preview(&back),
        subject_area: row.get(3)?,
        concept_id: row.get(4)?,
        concept_title: row.get(5)?,
        is_archived: row.get::<_, i64>(6)? == 1,
        archived_at: row.get(7)?,
        schedule: schedule_from_row(row, 8)?,
        exam_links: Vec::new(),
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

fn schedule_from_row(
    row: &rusqlite::Row<'_>,
    start: usize,
) -> rusqlite::Result<V2FlashcardSchedule> {
    Ok(V2FlashcardSchedule {
        due_on: row.get(start)?,
        interval_days: row.get(start + 1)?,
        ease_factor: row.get(start + 2)?,
        review_count: row.get(start + 3)?,
        lapse_count: row.get(start + 4)?,
        last_rating: row.get(start + 5)?,
        last_reviewed_on: row.get(start + 6)?,
        last_reviewed_at: row.get(start + 7)?,
        algorithm_version: row.get(start + 8)?,
    })
}

fn review_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2FlashcardReview> {
    Ok(V2FlashcardReview {
        id: row.get(0)?,
        reviewed_on: row.get(1)?,
        reviewed_at: row.get(2)?,
        rating: row.get(3)?,
        scheduled_due_on: row.get(4)?,
        next_due_on: row.get(5)?,
        interval_before_days: row.get(6)?,
        interval_after_days: row.get(7)?,
        ease_before: row.get(8)?,
        ease_after: row.get(9)?,
        algorithm_version: row.get(10)?,
    })
}

fn attach_exam_links(
    connection: &Connection,
    summaries: &mut [V2FlashcardSummary],
) -> Result<(), AppError> {
    if summaries.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = summaries.iter().map(|summary| summary.id).collect();
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT l.flashcard_id, l.exam_id, e.code, e.short_name, l.syllabus_node_id, n.title
         FROM flashcard_exam_links l
         JOIN exams e ON e.id=l.exam_id
         LEFT JOIN exam_syllabus_nodes n
            ON n.exam_id=l.exam_id AND n.id=l.syllabus_node_id
         WHERE l.flashcard_id IN ({placeholders})
         ORDER BY e.display_order, l.flashcard_id"
    );
    let values = ids.iter().map(|id| Value::Integer(*id)).collect::<Vec<_>>();
    let mut statement = connection.prepare(&sql).map_err(|error| {
        AppError::database("The flashcard exam links could not be prepared", error)
    })?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                V2FlashcardExamLink {
                    exam_id: row.get(1)?,
                    exam_code: row.get(2)?,
                    exam_short_name: row.get(3)?,
                    syllabus_node_id: row.get(4)?,
                    node_title: row.get(5)?,
                },
            ))
        })
        .map_err(|error| AppError::database("The flashcard exam links could not run", error))?;
    let mut links: HashMap<i64, Vec<V2FlashcardExamLink>> = HashMap::new();
    for row in rows {
        let (card_id, link) =
            row.map_err(|error| AppError::database("A flashcard exam link is invalid", error))?;
        links.entry(card_id).or_default().push(link);
    }
    for summary in summaries.iter_mut() {
        summary.exam_links = links.remove(&summary.id).unwrap_or_default();
    }
    Ok(())
}

fn list_subjects(connection: &Connection) -> Result<Vec<String>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT subject_area FROM flashcards
             WHERE is_archived=0
             ORDER BY subject_area COLLATE NOCASE",
        )
        .map_err(|error| {
            AppError::database("The flashcard subjects could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], |row| row.get(0))
        .map_err(|error| AppError::database("The flashcard subjects could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A flashcard subject is invalid", error))
}

fn validate_exam_links(
    connection: &Connection,
    links: &[V2FlashcardExamLinkInput],
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for link in links {
        if !seen.insert(link.exam_id) {
            return Err(AppError::validation(
                "examLinks",
                "Each examination can appear only once per card.",
            ));
        }
        validate_exam(connection, link.exam_id)?;
        if let Some(node_id) = link.syllabus_node_id {
            validate_node(connection, link.exam_id, node_id)?;
        }
    }
    Ok(())
}

fn validate_exam(connection: &Connection, exam_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id=?1)",
            [exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The examination could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "examId",
            "Choose one of the four supported examinations.",
        ));
    }
    Ok(())
}

fn validate_node(connection: &Connection, exam_id: i64, node_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exam_syllabus_nodes WHERE exam_id=?1 AND id=?2)",
            params![exam_id, node_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The syllabus node could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "syllabusNodeId",
            "Choose a syllabus node from the selected examination.",
        ));
    }
    Ok(())
}

fn validate_concept(connection: &Connection, concept_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concepts WHERE id=?1)",
            [concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The concept could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "conceptId",
            "Choose a concept from the shared library.",
        ));
    }
    Ok(())
}

fn validate_limit(
    value: Option<i64>,
    default: i64,
    max: i64,
    field: &'static str,
) -> Result<i64, AppError> {
    let limit = value.unwrap_or(default);
    if !(1..=max).contains(&limit) {
        return Err(AppError::validation(
            field,
            format!("Limit must be between 1 and {max}."),
        ));
    }
    Ok(limit)
}

fn parse_date(value: &str, field: &'static str) -> Result<NaiveDate, AppError> {
    let parsed = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::validation(field, "Choose a valid date."))?;
    if parsed.format("%Y-%m-%d").to_string() != value.trim() {
        return Err(AppError::validation(field, "Choose a valid date."));
    }
    Ok(parsed)
}

fn required_text(
    value: &str,
    max: usize,
    field: &'static str,
    label: &'static str,
) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > max {
        return Err(AppError::validation(
            field,
            format!("{label} must be between 1 and {max} characters."),
        ));
    }
    Ok(trimmed.to_owned())
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' | '%' | '_' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn preview(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let mut preview: String = chars.by_ref().take(PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;
    use std::path::Path;

    const TODAY: &str = "2026-11-08";

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn first_topic(connection: &Connection, exam_id: i64) -> i64 {
        connection
            .query_row(
                "SELECT id FROM exam_syllabus_nodes
                 WHERE exam_id=?1 AND node_kind='topic' AND is_archived=0
                 ORDER BY id LIMIT 1",
                [exam_id],
                |row| row.get(0),
            )
            .expect("seeded syllabus topic")
    }

    fn first_unit_with_topic(connection: &Connection, exam_id: i64) -> (i64, i64) {
        connection
            .query_row(
                "SELECT parent.id, child.id
                 FROM exam_syllabus_nodes parent
                 JOIN exam_syllabus_nodes child ON child.parent_id=parent.id
                 WHERE parent.exam_id=?1 AND parent.node_kind='unit'
                   AND child.node_kind='topic' AND child.is_archived=0
                 ORDER BY parent.id, child.id LIMIT 1",
                [exam_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("seeded syllabus unit and topic")
    }

    fn card_input(exam_links: Vec<V2FlashcardExamLinkInput>) -> V2FlashcardInput {
        V2FlashcardInput {
            id: None,
            front: "Explain the physical meaning of the gradient.".to_owned(),
            back: "The gradient points toward the steepest increase of a scalar field.".to_owned(),
            subject_area: "Mathematical Methods".to_owned(),
            concept_id: None,
            exam_links,
            today: TODAY.to_owned(),
        }
    }

    fn save(state: &V2State, input: V2FlashcardInput) -> V2FlashcardDetail {
        state
            .with_connection(|connection| save_flashcard(connection, input, Utc::now()))
            .expect("save flashcard")
    }

    fn list(state: &V2State, filter: V2FlashcardFilter) -> V2FlashcardLibrary {
        state
            .with_connection(|connection| list_flashcards(connection, filter))
            .expect("list flashcards")
    }

    fn default_filter() -> V2FlashcardFilter {
        V2FlashcardFilter {
            query: None,
            exam_id: None,
            subject_area: None,
            node_id: None,
            concept_slug: None,
            include_archived: None,
            due_only: None,
            today: TODAY.to_owned(),
            sort: None,
            limit: None,
            offset: None,
        }
    }

    fn review(
        state: &V2State,
        flashcard_id: i64,
        rating: &str,
        today: &str,
    ) -> V2FlashcardReviewResult {
        state
            .with_connection(|connection| {
                review_flashcard(
                    connection,
                    V2FlashcardReviewInput {
                        flashcard_id,
                        rating: rating.to_owned(),
                        today: today.to_owned(),
                    },
                    Utc::now(),
                )
            })
            .expect("review flashcard")
    }

    #[test]
    fn shared_and_exam_linked_cards_are_scoped_and_archiveable() {
        let (_root, state) = state();
        let exam_one_topic = state
            .with_connection(|connection| Ok(first_topic(connection, 1)))
            .expect("topic");
        let exam_two_topic = state
            .with_connection(|connection| Ok(first_topic(connection, 2)))
            .expect("topic");

        let shared = save(&state, card_input(Vec::new()));
        let exam_one = save(
            &state,
            card_input(vec![V2FlashcardExamLinkInput {
                exam_id: 1,
                syllabus_node_id: Some(exam_one_topic),
            }]),
        );
        let _exam_two = save(
            &state,
            card_input(vec![V2FlashcardExamLinkInput {
                exam_id: 2,
                syllabus_node_id: Some(exam_two_topic),
            }]),
        );

        let all = list(&state, default_filter());
        assert_eq!(all.total, 3);
        assert!(all.subjects.contains(&"Mathematical Methods".to_owned()));

        let mut exam_one_filter = default_filter();
        exam_one_filter.exam_id = Some(1);
        let exam_one_list = list(&state, exam_one_filter);
        assert_eq!(exam_one_list.total, 2);
        assert!(
            exam_one_list
                .cards
                .iter()
                .any(|card| card.id == shared.flashcard.id)
        );
        assert!(
            exam_one_list
                .cards
                .iter()
                .any(|card| card.id == exam_one.flashcard.id)
        );

        let mut exam_two_filter = default_filter();
        exam_two_filter.exam_id = Some(2);
        let exam_two_list = list(&state, exam_two_filter);
        assert_eq!(exam_two_list.total, 2);
        assert!(
            exam_two_list
                .cards
                .iter()
                .any(|card| card.id == shared.flashcard.id)
        );

        let mut node_filter = default_filter();
        node_filter.exam_id = Some(1);
        node_filter.node_id = Some(exam_one_topic);
        let node_list = list(&state, node_filter);
        assert!(
            node_list
                .cards
                .iter()
                .any(|card| card.id == exam_one.flashcard.id)
        );

        let archived = state
            .with_connection(|connection| {
                set_flashcard_archived(
                    connection,
                    V2FlashcardArchiveInput {
                        flashcard_id: exam_one.flashcard.id,
                        archived: true,
                    },
                    Utc::now(),
                )
            })
            .expect("archive");
        assert!(archived.flashcard.is_archived);

        let active = list(&state, default_filter());
        assert_eq!(active.total, 2);
        assert!(
            !active
                .cards
                .iter()
                .any(|card| card.id == exam_one.flashcard.id)
        );

        let mut archived_filter = default_filter();
        archived_filter.include_archived = Some(true);
        assert_eq!(list(&state, archived_filter).total, 3);

        let summary = state
            .with_connection(|connection| {
                let mut summaries = vec![load_summary(connection, exam_one.flashcard.id)?];
                attach_exam_links(connection, &mut summaries)?;
                Ok::<_, AppError>(summaries.remove(0))
            })
            .expect("archived summary");
        assert_eq!(summary.exam_links.len(), 1);
        assert_eq!(summary.exam_links[0].syllabus_node_id, Some(exam_one_topic));
        assert!(summary.exam_links[0].node_title.is_some());
    }

    #[test]
    fn node_filter_includes_descendant_topics() {
        let (_root, state) = state();
        let (unit_id, topic_id) = state
            .with_connection(|connection| Ok(first_unit_with_topic(connection, 1)))
            .expect("unit and topic");
        assert_ne!(unit_id, topic_id);

        let card = save(
            &state,
            card_input(vec![V2FlashcardExamLinkInput {
                exam_id: 1,
                syllabus_node_id: Some(topic_id),
            }]),
        );

        let mut filter = default_filter();
        filter.exam_id = Some(1);
        filter.node_id = Some(unit_id);
        let result = list(&state, filter);
        assert!(result.cards.iter().any(|item| item.id == card.flashcard.id));
    }

    #[test]
    fn review_schedule_history_stats_and_mastery_are_isolated() {
        let (_root, state) = state();
        let concept_id = state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO concepts(slug,title,domain,difficulty,created_at,updated_at)
                         VALUES('flash-test-concept','Flash Test Concept','Physics',3,?1,?1)",
                        [Utc::now().to_rfc3339()],
                    )
                    .map_err(|error| AppError::database("test concept", error))?;
                let concept_id = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO concept_mastery(
                            concept_id,mastery_score,confidence,evidence_count,last_recalculated_at,updated_at
                         ) VALUES(?1,61.5,3,2,?2,?2)",
                        params![concept_id, "2026-11-05T00:00:00Z"],
                    )
                    .map_err(|error| AppError::database("test mastery", error))?;
                Ok::<_, AppError>(concept_id)
            })
            .expect("concept fixture");

        let mut input = card_input(Vec::new());
        input.concept_id = Some(concept_id);
        let card = save(&state, input);
        let schedule = card.flashcard.schedule.clone();
        assert_eq!(schedule.due_on, TODAY);
        assert_eq!(schedule.review_count, 0);

        let first = review(&state, card.flashcard.id, "good", TODAY);
        assert_eq!(first.schedule.due_on, "2026-11-10");
        assert_eq!(first.schedule.interval_days, 2);
        assert_eq!(first.schedule.ease_factor, 2.5);
        assert_eq!(first.review.interval_before_days, 0);
        assert_eq!(first.review.interval_after_days, 2);

        let detail = state
            .with_connection(|connection| get_flashcard_detail(connection, card.flashcard.id))
            .expect("detail");
        assert_eq!(detail.reviews.len(), 1);
        assert_eq!(
            detail.flashcard.concept_title.as_deref(),
            Some("Flash Test Concept")
        );

        let stats = state
            .with_connection(|connection| {
                flashcard_stats(
                    connection,
                    V2FlashcardStatsInput {
                        query: None,
                        exam_id: None,
                        subject_area: None,
                        node_id: None,
                        concept_slug: None,
                        include_archived: None,
                        today: TODAY.to_owned(),
                    },
                )
            })
            .expect("stats");
        assert_eq!(stats.total_cards, 1);
        assert_eq!(stats.due_cards, 0);
        assert_eq!(stats.reviewed_cards, 1);
        assert_eq!(stats.new_cards, 0);
        assert_eq!(stats.total_reviews, 1);
        assert_eq!(stats.reviews_last_30_days, 1);
        assert_eq!(stats.successful_last_30_days, 1);
        assert_eq!(stats.retention_percent, Some(100.0));

        let second = review(&state, card.flashcard.id, "again", "2026-11-10");
        assert_eq!(second.schedule.due_on, "2026-11-11");
        assert_eq!(second.schedule.lapse_count, 1);
        assert_eq!(second.schedule.interval_days, 1);

        let mastery: (f64, i64) = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT mastery_score,evidence_count FROM concept_mastery WHERE concept_id=?1",
                        [concept_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| AppError::database("mastery check", error))
            })
            .expect("mastery");
        assert_eq!(mastery, (61.5, 2));
        let session_count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM study_sessions", [], |row| row.get(0))
                    .map_err(|error| AppError::database("session count", error))
            })
            .expect("sessions");
        assert_eq!(session_count, 0);
    }

    #[test]
    fn validation_and_edit_semantics_are_enforced() {
        let (_root, state) = state();
        let exam_one_topic = state
            .with_connection(|connection| Ok(first_topic(connection, 1)))
            .expect("topic");
        let exam_two_topic = state
            .with_connection(|connection| Ok(first_topic(connection, 2)))
            .expect("topic");

        let mut empty_front = card_input(Vec::new());
        empty_front.front = "   ".to_owned();
        let error = state
            .with_connection(|connection| save_flashcard(connection, empty_front, Utc::now()))
            .expect_err("empty front");
        assert_eq!(error.code, "validation_error");

        let duplicate = state
            .with_connection(|connection| {
                save_flashcard(
                    connection,
                    card_input(vec![
                        V2FlashcardExamLinkInput {
                            exam_id: 1,
                            syllabus_node_id: Some(exam_one_topic),
                        },
                        V2FlashcardExamLinkInput {
                            exam_id: 1,
                            syllabus_node_id: None,
                        },
                    ]),
                    Utc::now(),
                )
            })
            .expect_err("duplicate exams");
        assert_eq!(duplicate.code, "validation_error");

        let wrong_node = state
            .with_connection(|connection| {
                save_flashcard(
                    connection,
                    card_input(vec![V2FlashcardExamLinkInput {
                        exam_id: 1,
                        syllabus_node_id: Some(exam_two_topic),
                    }]),
                    Utc::now(),
                )
            })
            .expect_err("wrong exam node");
        assert_eq!(wrong_node.code, "validation_error");

        let card = save(
            &state,
            card_input(vec![V2FlashcardExamLinkInput {
                exam_id: 1,
                syllabus_node_id: Some(exam_one_topic),
            }]),
        );
        let due_before = card.flashcard.schedule.due_on.clone();
        let mut edit = card_input(Vec::new());
        edit.id = Some(card.flashcard.id);
        edit.front = "Updated front".to_owned();
        edit.back = "Updated back".to_owned();
        edit.subject_area = "Electromagnetism".to_owned();
        let updated = save(&state, edit);
        assert_eq!(updated.flashcard.subject_area, "Electromagnetism");
        assert!(updated.flashcard.exam_links.is_empty());
        assert_eq!(updated.flashcard.schedule.due_on, due_before);
        assert_eq!(updated.flashcard.schedule.review_count, 0);
    }

    #[test]
    fn version_two_database_upgrades_to_three_and_preserves_existing_data() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = root.path().join("v2");
        let state = V2State::initialize(&data_dir);
        let database_path = state.database_path().expect("database path");
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE settings SET daily_study_target_minutes=180 WHERE id=1",
                        [],
                    )
                    .map_err(|error| AppError::database("settings fixture", error))?;
                connection
                    .execute_batch(
                        "DROP TABLE IF EXISTS flashcard_reviews;
                         DROP TABLE IF EXISTS flashcard_schedule;
                         DROP TABLE IF EXISTS flashcard_exam_links;
                         DROP TABLE IF EXISTS flashcards;",
                    )
                    .map_err(|error| AppError::database("drop flashcards", error))?;
                connection
                    .execute(
                        "INSERT OR IGNORE INTO schema_migrations(version,name,checksum,applied_at)
                         VALUES(2,'v2_quick_start_configs',?1,?2)",
                        params![crate::v2::SCHEMA_2_CHECKSUM, "2026-11-05T00:00:00Z"],
                    )
                    .map_err(|error| AppError::database("v2 ledger fixture", error))?;
                connection
                    .execute("UPDATE app_metadata SET schema_version=2 WHERE id=1", [])
                    .map_err(|error| AppError::database("metadata fixture", error))?;
                connection
                    .execute_batch("PRAGMA user_version = 2;")
                    .map_err(|error| AppError::database("version fixture", error))?;
                connection
                    .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(|error| AppError::database("checkpoint fixture", error))?;
                Ok::<(), AppError>(())
            })
            .expect("version two fixture");
        drop(state);

        let reopened = V2State::initialize(&data_dir);
        let status = reopened.status();
        assert_eq!(status.state, crate::v2::V2DatabaseState::Healthy);
        assert_eq!(status.schema_version, Some(crate::v2::V2_SCHEMA_VERSION));
        let (target, flashcard_table): (i64, i64) = reopened
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT (SELECT daily_study_target_minutes FROM settings WHERE id=1),
                                (SELECT COUNT(*) FROM sqlite_master
                                  WHERE type='table' AND name='flashcards')",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| AppError::database("upgrade verification", error))
            })
            .expect("upgraded database");
        assert_eq!(target, 180);
        assert_eq!(flashcard_table, 1);
        assert!(Path::new(&database_path).exists());
    }

    #[test]
    fn due_and_practice_all_queues_are_ordered_and_limited() {
        let (_root, state) = state();
        let first = save(&state, card_input(Vec::new()));
        let second = save(&state, card_input(Vec::new()));
        let third = save(&state, card_input(Vec::new()));

        let due = state
            .with_connection(|connection| {
                review_queue(
                    connection,
                    V2FlashcardQueueInput {
                        exam_id: None,
                        subject_area: None,
                        node_id: None,
                        concept_slug: None,
                        due_only: true,
                        today: TODAY.to_owned(),
                        limit: None,
                    },
                )
            })
            .expect("due queue");
        assert_eq!(due.cards.len(), 3);
        assert_eq!(due.remaining, 0);

        let reviewed = review(&state, first.flashcard.id, "good", TODAY);
        assert_eq!(reviewed.schedule.due_on, "2026-11-10");
        let due_after = state
            .with_connection(|connection| {
                review_queue(
                    connection,
                    V2FlashcardQueueInput {
                        exam_id: None,
                        subject_area: None,
                        node_id: None,
                        concept_slug: None,
                        due_only: true,
                        today: TODAY.to_owned(),
                        limit: None,
                    },
                )
            })
            .expect("due queue after review");
        assert_eq!(due_after.cards.len(), 2);

        let all = state
            .with_connection(|connection| {
                review_queue(
                    connection,
                    V2FlashcardQueueInput {
                        exam_id: None,
                        subject_area: None,
                        node_id: None,
                        concept_slug: None,
                        due_only: false,
                        today: TODAY.to_owned(),
                        limit: Some(2),
                    },
                )
            })
            .expect("all queue");
        assert_eq!(all.cards.len(), 2);
        assert_eq!(all.remaining, 1);
        assert!(all.cards.iter().any(|card| card.id == second.flashcard.id));
        assert!(all.cards.iter().any(|card| card.id == third.flashcard.id));
    }
}
