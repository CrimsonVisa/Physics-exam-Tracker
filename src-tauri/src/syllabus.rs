use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::{
    date_only,
    error::AppError,
    syllabus_catalog::{ADVANCED, CATALOG, CORE, CatalogDef, GENERAL_APTITUDE, SectionDef},
};

pub const TOPIC_STATUSES: [&str; 4] = ["Not started", "Learning", "Revised", "Confident"];
pub const TOPIC_PRIORITIES: [&str; 4] = ["Low", "Normal", "High", "High-Yield"];
pub const TOPIC_CATEGORIES: [&str; 3] = [GENERAL_APTITUDE, CORE, ADVANCED];
pub const MAX_SEARCH_QUERY_CHARS: usize = 100;
pub const MAX_NOTES_CHARS: usize = 2000;

#[cfg(test)]
const DEFAULT_STATUS: &str = "Not started";
#[cfg(test)]
const DEFAULT_PRIORITY: &str = "Normal";
#[cfg(test)]
const DEFAULT_CONFIDENCE: i64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllabusSource {
    pub code: String,
    pub title: String,
    pub publisher: String,
    pub url: String,
    pub landing_url: Option<String>,
    pub verified_on: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllabusSection {
    pub code: String,
    pub title: String,
    pub source_code: String,
    pub source_reference: String,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllabusCatalog {
    pub sources: Vec<SyllabusSource>,
    pub sections: Vec<SyllabusSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicProgress {
    pub status: String,
    pub priority: String,
    pub confidence: i64,
    pub notes: String,
    pub updated_at: Option<String>,
    pub recorded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllabusTopic {
    pub id: i64,
    pub code: String,
    pub section_code: String,
    pub section_title: String,
    pub title: String,
    pub category: String,
    pub source_code: String,
    pub source_title: String,
    pub source_url: String,
    pub source_reference: String,
    pub progress: TopicProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusCounts {
    pub not_started: i64,
    pub learning: i64,
    pub revised: i64,
    pub confident: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionSummary {
    pub code: String,
    pub title: String,
    pub total_topics: i64,
    pub status_counts: StatusCounts,
    pub approximate_progress: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllabusSummary {
    pub total_topics: i64,
    pub status_counts: StatusCounts,
    pub approximate_progress: f64,
    pub sections: Vec<SectionSummary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SyllabusFilter {
    pub query: Option<String>,
    pub section_code: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicProgressInput {
    pub status: String,
    pub priority: String,
    pub confidence: i64,
    pub notes: String,
}

/// Insert or update the built-in catalog. Stable codes are the identity used by
/// progress, so metadata updates never replace a topic row or its study record.
pub fn seed(connection: &mut Connection) -> Result<(), AppError> {
    seed_catalog(connection, &CATALOG)
}

fn seed_catalog(connection: &mut Connection, catalog: &CatalogDef<'_>) -> Result<(), AppError> {
    validate_catalog(catalog).map_err(|message| {
        AppError::internal(format!(
            "The built-in syllabus catalog is invalid: {message}"
        ))
    })?;

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The syllabus catalog transaction could not be started",
            error,
        )
    })?;

    for source in catalog.sources {
        transaction
            .execute(
                "INSERT INTO syllabus_sources (
                    code, title, publisher, url, landing_url, verified_on, notes,
                    created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 ON CONFLICT(code) DO UPDATE SET
                    title = excluded.title,
                    publisher = excluded.publisher,
                    url = excluded.url,
                    landing_url = excluded.landing_url,
                    verified_on = excluded.verified_on,
                    notes = excluded.notes,
                    updated_at = excluded.updated_at",
                params![
                    source.code,
                    source.title,
                    source.publisher,
                    source.url,
                    source.landing_url,
                    source.verified_on,
                    source.notes,
                ],
            )
            .map_err(|error| AppError::database("Syllabus sources could not be stored", error))?;
    }

    let source_ids = load_id_map(&transaction, "syllabus_sources", "Syllabus sources")?;

    for section in catalog.sections {
        let source_id = source_ids.get(section.source_code).ok_or_else(|| {
            AppError::internal(format!(
                "The syllabus section {} references an unknown source.",
                section.code
            ))
        })?;

        transaction
            .execute(
                "INSERT INTO syllabus_sections (
                    code, title, source_id, source_reference, display_order,
                    created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 ON CONFLICT(code) DO UPDATE SET
                    title = excluded.title,
                    source_id = excluded.source_id,
                    source_reference = excluded.source_reference,
                    display_order = excluded.display_order,
                    updated_at = excluded.updated_at",
                params![
                    section.code,
                    section.title,
                    source_id,
                    section.source_reference,
                    section.display_order,
                ],
            )
            .map_err(|error| AppError::database("Syllabus sections could not be stored", error))?;
    }

    let section_ids = load_id_map(&transaction, "syllabus_sections", "Syllabus sections")?;

    for topic in catalog.topics {
        let section_id = section_ids.get(topic.section_code).ok_or_else(|| {
            AppError::internal(format!(
                "The syllabus topic {} references an unknown section.",
                topic.code
            ))
        })?;
        let source_id = source_ids.get(topic.source_code).ok_or_else(|| {
            AppError::internal(format!(
                "The syllabus topic {} references an unknown source.",
                topic.code
            ))
        })?;

        transaction
            .execute(
                "INSERT INTO syllabus_topics (
                    section_id, code, title, category, source_id, source_reference,
                    display_order, is_archived, archived_at, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, NULL,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 )
                 ON CONFLICT(code) DO UPDATE SET
                    section_id = excluded.section_id,
                    title = excluded.title,
                    category = excluded.category,
                    source_id = excluded.source_id,
                    source_reference = excluded.source_reference,
                    display_order = excluded.display_order,
                    is_archived = 0,
                    archived_at = NULL,
                    updated_at = excluded.updated_at",
                params![
                    section_id,
                    topic.code,
                    topic.title,
                    topic.category,
                    source_id,
                    topic.source_reference,
                    topic.display_order,
                ],
            )
            .map_err(|error| AppError::database("Syllabus topics could not be stored", error))?;
    }

    // Topics removed from a newer catalog are hidden rather than deleted. Their
    // progress rows remain valid because topic ids and codes are never reused for
    // a different catalog entry.
    if catalog.topics.is_empty() {
        transaction
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE is_archived = 0",
                [],
            )
            .map_err(|error| {
                AppError::database("Removed syllabus topics could not be archived", error)
            })?;
    } else {
        let codes: Vec<&str> = catalog.topics.iter().map(|topic| topic.code).collect();
        let placeholders = vec!["?"; codes.len()].join(", ");
        let sql = format!(
            "UPDATE syllabus_topics
             SET is_archived = 1,
                 archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE is_archived = 0 AND code NOT IN ({placeholders})"
        );

        transaction
            .execute(&sql, params_from_iter(codes.iter()))
            .map_err(|error| {
                AppError::database("Removed syllabus topics could not be archived", error)
            })?;
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The syllabus catalog could not be committed", error))
}

fn load_id_map(
    connection: &Connection,
    table: &str,
    label: &str,
) -> Result<HashMap<String, i64>, AppError> {
    let sql = format!("SELECT code, id FROM {table}");
    let result = (|| -> rusqlite::Result<HashMap<String, i64>> {
        let mut statement = connection.prepare(&sql)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get(1)?)))?;

        rows.collect()
    })();

    result.map_err(|error| AppError::database(label, error))
}

fn validate_catalog(catalog: &CatalogDef<'_>) -> Result<(), String> {
    let mut source_codes = HashSet::new();
    for source in catalog.sources {
        if !source_codes.insert(source.code) {
            return Err(format!("duplicate source code {}", source.code));
        }
        if !source.url.starts_with("https://") {
            return Err(format!("source {} must use an HTTPS URL", source.code));
        }
        if source
            .landing_url
            .is_some_and(|url| !url.starts_with("https://"))
        {
            return Err(format!("source {} has an invalid landing URL", source.code));
        }
        if date_only::parse_date_only(source.verified_on).is_none() {
            return Err(format!(
                "source {} has an invalid verified date",
                source.code
            ));
        }
    }

    let mut section_codes = HashSet::new();
    let mut section_orders = HashSet::new();
    let mut sections_by_code: HashMap<&str, &SectionDef> = HashMap::new();
    for section in catalog.sections {
        if !section_codes.insert(section.code) {
            return Err(format!("duplicate section code {}", section.code));
        }
        if !section_orders.insert(section.display_order) {
            return Err(format!(
                "duplicate section display order {}",
                section.display_order
            ));
        }
        if !source_codes.contains(section.source_code) {
            return Err(format!(
                "section {} references an unknown source",
                section.code
            ));
        }
        sections_by_code.insert(section.code, section);
    }

    let advanced_only_sections = ["AMP", "CMP", "NPP"];
    let mut topic_codes = HashSet::new();
    let mut section_topic_orders: HashMap<(&str, &str), HashSet<i64>> = HashMap::new();

    for topic in catalog.topics {
        if !topic_codes.insert(topic.code) {
            return Err(format!("duplicate topic code {}", topic.code));
        }

        let section = sections_by_code.get(topic.section_code).ok_or_else(|| {
            format!(
                "topic {} references unknown section {}",
                topic.code, topic.section_code
            )
        })?;

        if !source_codes.contains(topic.source_code) {
            return Err(format!("topic {} references an unknown source", topic.code));
        }
        if topic.source_code != section.source_code {
            return Err(format!(
                "topic {} and its section use different sources",
                topic.code
            ));
        }
        if !TOPIC_CATEGORIES.contains(&topic.category) {
            return Err(format!(
                "topic {} uses unsupported category {}",
                topic.code, topic.category
            ));
        }
        if !topic.code.starts_with(section.code) {
            return Err(format!(
                "topic {} does not start with its section code",
                topic.code
            ));
        }
        if topic.display_order < 1 {
            return Err(format!("topic {} has an invalid order", topic.code));
        }
        if !section_topic_orders
            .entry((topic.section_code, topic.category))
            .or_default()
            .insert(topic.display_order)
        {
            return Err(format!(
                "section {} category {} has duplicate topic order {}",
                topic.section_code, topic.category, topic.display_order
            ));
        }

        match (section.code, topic.category) {
            ("GA", GENERAL_APTITUDE) => {}
            ("GA", _) => {
                return Err(format!(
                    "General Aptitude topic {} must use the General Aptitude category",
                    topic.code
                ));
            }
            (_, CORE | ADVANCED) => {}
            (_, _) => {
                return Err(format!("topic {} must use Core or Advanced", topic.code));
            }
        }

        if topic.category == CORE && advanced_only_sections.contains(&section.code) {
            return Err(format!(
                "advanced-only section {} contains Core topic {}",
                section.code, topic.code
            ));
        }
    }

    Ok(())
}

pub fn load_catalog(connection: &Connection) -> Result<SyllabusCatalog, AppError> {
    let sources = (|| -> rusqlite::Result<Vec<SyllabusSource>> {
        let mut statement = connection.prepare(
            "SELECT code, title, publisher, url, landing_url, verified_on, notes
             FROM syllabus_sources
             ORDER BY id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SyllabusSource {
                code: row.get(0)?,
                title: row.get(1)?,
                publisher: row.get(2)?,
                url: row.get(3)?,
                landing_url: row.get(4)?,
                verified_on: row.get(5)?,
                notes: row.get(6)?,
            })
        })?;

        rows.collect()
    })()
    .map_err(|error| AppError::database("Syllabus sources could not be loaded", error))?;

    let sections = (|| -> rusqlite::Result<Vec<SyllabusSection>> {
        let mut statement = connection.prepare(
            "SELECT s.code, s.title, src.code, s.source_reference, s.display_order
             FROM syllabus_sections s
             JOIN syllabus_sources src ON src.id = s.source_id
             ORDER BY s.display_order, s.id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SyllabusSection {
                code: row.get(0)?,
                title: row.get(1)?,
                source_code: row.get(2)?,
                source_reference: row.get(3)?,
                display_order: row.get(4)?,
            })
        })?;

        rows.collect()
    })()
    .map_err(|error| AppError::database("Syllabus sections could not be loaded", error))?;

    Ok(SyllabusCatalog { sources, sections })
}

pub fn search_topics(
    connection: &Connection,
    filter: SyllabusFilter,
) -> Result<Vec<SyllabusTopic>, AppError> {
    let query = normalized_text(filter.query);
    if query
        .as_ref()
        .is_some_and(|value| value.chars().count() > MAX_SEARCH_QUERY_CHARS)
    {
        return Err(AppError::validation(
            "query",
            format!("Search text must be {MAX_SEARCH_QUERY_CHARS} characters or fewer."),
        ));
    }

    let section_code = normalized_text(filter.section_code);
    if let Some(code) = &section_code {
        if code.chars().count() > 32 {
            return Err(AppError::validation(
                "sectionCode",
                "Choose a section from the current syllabus.",
            ));
        }

        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM syllabus_sections WHERE code = ?1)",
                [code],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("The syllabus section filter could not be checked", error)
            })?;

        if exists == 0 {
            return Err(AppError::validation(
                "sectionCode",
                "Choose a section from the current syllabus.",
            ));
        }
    }

    let category = normalized_text(filter.category);
    if let Some(value) = &category {
        validate_category(value)?;
    }

    let status = normalized_text(filter.status);
    if let Some(value) = &status {
        validate_status(value)?;
    }

    let priority = normalized_text(filter.priority);
    if let Some(value) = &priority {
        validate_priority(value)?;
    }

    let pattern = query.map(|value| format!("%{}%", escape_like(&value)));

    let result = (|| -> rusqlite::Result<Vec<SyllabusTopic>> {
        let mut statement = connection.prepare(
            "SELECT
                t.id,
                t.code,
                s.code,
                s.title,
                t.title,
                t.category,
                src.code,
                src.title,
                src.url,
                t.source_reference,
                COALESCE(p.status, 'Not started'),
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.notes, ''),
                p.updated_at,
                CASE WHEN p.topic_id IS NULL THEN 0 ELSE 1 END
             FROM syllabus_topics t
             JOIN syllabus_sections s ON s.id = t.section_id
             JOIN syllabus_sources src ON src.id = t.source_id
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             WHERE t.is_archived = 0
               AND (
                   ?1 IS NULL
                   OR t.title LIKE ?1 ESCAPE '!'
                   OR t.code LIKE ?1 ESCAPE '!'
               )
               AND (?2 IS NULL OR s.code = ?2)
               AND (?3 IS NULL OR t.category = ?3)
               AND (?4 IS NULL OR COALESCE(p.status, 'Not started') = ?4)
               AND (?5 IS NULL OR COALESCE(p.priority, 'Normal') = ?5)
             ORDER BY
                s.display_order,
                s.id,
                CASE t.category
                    WHEN 'General Aptitude' THEN 0
                    WHEN 'Core' THEN 1
                    WHEN 'Advanced' THEN 2
                    ELSE 3
                END,
                t.display_order,
                t.id",
        )?;

        let rows = statement.query_map(
            params![pattern, section_code, category, status, priority],
            |row| {
                let recorded: i64 = row.get(15)?;

                Ok(SyllabusTopic {
                    id: row.get(0)?,
                    code: row.get(1)?,
                    section_code: row.get(2)?,
                    section_title: row.get(3)?,
                    title: row.get(4)?,
                    category: row.get(5)?,
                    source_code: row.get(6)?,
                    source_title: row.get(7)?,
                    source_url: row.get(8)?,
                    source_reference: row.get(9)?,
                    progress: TopicProgress {
                        status: row.get(10)?,
                        priority: row.get(11)?,
                        confidence: row.get(12)?,
                        notes: row.get(13)?,
                        updated_at: row.get(14)?,
                        recorded: recorded != 0,
                    },
                })
            },
        )?;

        rows.collect()
    })();

    result.map_err(|error| AppError::database("Syllabus topics could not be searched", error))
}

pub fn load_progress(connection: &Connection, topic_id: i64) -> Result<TopicProgress, AppError> {
    validate_topic_id(topic_id)?;

    match load_progress_optional(connection, topic_id)? {
        Some(progress) => Ok(progress),
        None => Err(AppError::topic_not_found()),
    }
}

fn load_progress_optional(
    connection: &Connection,
    topic_id: i64,
) -> Result<Option<TopicProgress>, AppError> {
    let result = connection
        .query_row(
            "SELECT
                COALESCE(p.status, 'Not started'),
                COALESCE(p.priority, 'Normal'),
                COALESCE(p.confidence, 3),
                COALESCE(p.notes, ''),
                p.updated_at,
                CASE WHEN p.topic_id IS NULL THEN 0 ELSE 1 END
             FROM syllabus_topics t
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             WHERE t.id = ?1 AND t.is_archived = 0",
            [topic_id],
            |row| {
                let recorded: i64 = row.get(5)?;

                Ok(TopicProgress {
                    status: row.get(0)?,
                    priority: row.get(1)?,
                    confidence: row.get(2)?,
                    notes: row.get(3)?,
                    updated_at: row.get(4)?,
                    recorded: recorded != 0,
                })
            },
        )
        .optional();

    result.map_err(|error| AppError::database("Topic progress could not be loaded", error))
}

pub fn update_progress(
    connection: &mut Connection,
    topic_id: i64,
    input: TopicProgressInput,
) -> Result<TopicProgress, AppError> {
    validate_topic_id(topic_id)?;
    validate_status(&input.status)?;
    validate_priority(&input.priority)?;
    validate_confidence(input.confidence)?;
    validate_notes(&input.notes)?;

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The topic progress transaction could not be started", error)
    })?;

    let active: i64 = transaction
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM syllabus_topics WHERE id = ?1 AND is_archived = 0
             )",
            [topic_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("The syllabus topic could not be checked", error))?;

    if active == 0 {
        return Err(AppError::topic_not_found());
    }

    transaction
        .execute(
            "INSERT INTO syllabus_progress (
                topic_id, status, priority, confidence, notes, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )
             ON CONFLICT(topic_id) DO UPDATE SET
                status = excluded.status,
                priority = excluded.priority,
                confidence = excluded.confidence,
                notes = excluded.notes,
                updated_at = excluded.updated_at",
            params![
                topic_id,
                input.status,
                input.priority,
                input.confidence,
                input.notes,
            ],
        )
        .map_err(|error| AppError::database("Topic progress could not be saved", error))?;

    let progress = load_progress_optional(&transaction, topic_id)?
        .ok_or_else(|| AppError::internal("Saved topic progress could not be reloaded."))?;

    transaction
        .commit()
        .map_err(|error| AppError::database("Topic progress could not be committed", error))?;

    Ok(progress)
}

pub fn summary(connection: &Connection) -> Result<SyllabusSummary, AppError> {
    let sections = (|| -> rusqlite::Result<Vec<SectionSummary>> {
        let mut statement = connection.prepare(
            "SELECT
                s.code,
                s.title,
                COUNT(t.id),
                COALESCE(SUM(CASE WHEN t.id IS NOT NULL AND COALESCE(p.status, 'Not started') = 'Not started' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN t.id IS NOT NULL AND COALESCE(p.status, 'Not started') = 'Learning' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN t.id IS NOT NULL AND COALESCE(p.status, 'Not started') = 'Revised' THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN t.id IS NOT NULL AND COALESCE(p.status, 'Not started') = 'Confident' THEN 1 ELSE 0 END), 0)
             FROM syllabus_sections s
             LEFT JOIN syllabus_topics t
                ON t.section_id = s.id AND t.is_archived = 0
             LEFT JOIN syllabus_progress p ON p.topic_id = t.id
             GROUP BY s.id, s.code, s.title, s.display_order
             ORDER BY s.display_order, s.id",
        )?;

        let rows = statement.query_map([], |row| {
            let total_topics: i64 = row.get(2)?;
            let status_counts = StatusCounts {
                not_started: row.get(3)?,
                learning: row.get(4)?,
                revised: row.get(5)?,
                confident: row.get(6)?,
            };

            Ok(SectionSummary {
                code: row.get(0)?,
                title: row.get(1)?,
                total_topics,
                approximate_progress: progress_percentage(&status_counts, total_topics),
                status_counts,
            })
        })?;

        rows.collect()
    })()
    .map_err(|error| AppError::database("The syllabus summary could not be loaded", error))?;

    let mut total_topics = 0;
    let mut status_counts = StatusCounts {
        not_started: 0,
        learning: 0,
        revised: 0,
        confident: 0,
    };

    for section in &sections {
        total_topics += section.total_topics;
        status_counts.not_started += section.status_counts.not_started;
        status_counts.learning += section.status_counts.learning;
        status_counts.revised += section.status_counts.revised;
        status_counts.confident += section.status_counts.confident;
    }

    Ok(SyllabusSummary {
        total_topics,
        approximate_progress: progress_percentage(&status_counts, total_topics),
        status_counts,
        sections,
    })
}

/// Equal topic weights with explicit status scores. This is a study-tracking
/// approximation, never an official examination score or marks prediction.
fn progress_percentage(counts: &StatusCounts, total_topics: i64) -> f64 {
    if total_topics <= 0 {
        return 0.0;
    }

    let score =
        counts.learning as f64 * 0.5 + counts.revised as f64 * 0.75 + counts.confident as f64;

    score / total_topics as f64 * 100.0
}

fn normalized_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn validate_topic_id(topic_id: i64) -> Result<(), AppError> {
    if topic_id <= 0 {
        return Err(AppError::validation(
            "topicId",
            "Choose a valid syllabus topic.",
        ));
    }

    Ok(())
}

fn validate_status(status: &str) -> Result<(), AppError> {
    if TOPIC_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::validation(
            "status",
            "Choose Not started, Learning, Revised or Confident.",
        ))
    }
}

fn validate_priority(priority: &str) -> Result<(), AppError> {
    if TOPIC_PRIORITIES.contains(&priority) {
        Ok(())
    } else {
        Err(AppError::validation(
            "priority",
            "Choose Low, Normal, High or High-Yield.",
        ))
    }
}

fn validate_category(category: &str) -> Result<(), AppError> {
    if TOPIC_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(AppError::validation(
            "category",
            "Choose General Aptitude, Core or Advanced.",
        ))
    }
}

fn validate_confidence(confidence: i64) -> Result<(), AppError> {
    if (1..=5).contains(&confidence) {
        Ok(())
    } else {
        Err(AppError::validation(
            "confidence",
            "Confidence must be a whole number from 1 to 5.",
        ))
    }
}

fn validate_notes(notes: &str) -> Result<(), AppError> {
    if notes.chars().count() <= MAX_NOTES_CHARS {
        Ok(())
    } else {
        Err(AppError::validation(
            "notes",
            format!("Notes must be {MAX_NOTES_CHARS} characters or fewer."),
        ))
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
    use crate::syllabus_catalog::{self, TopicDef};

    fn test_connection() -> Connection {
        let mut connection = crate::db::test_connection();
        seed(&mut connection).expect("catalog should seed");
        connection
    }

    fn empty_connection() -> Connection {
        crate::db::test_connection()
    }

    fn topic_id(connection: &Connection, code: &str) -> i64 {
        connection
            .query_row(
                "SELECT id FROM syllabus_topics WHERE code = ?1",
                [code],
                |row| row.get(0),
            )
            .expect("test topic should exist")
    }

    fn progress_input(
        status: &str,
        priority: &str,
        confidence: i64,
        notes: &str,
    ) -> TopicProgressInput {
        TopicProgressInput {
            status: status.to_owned(),
            priority: priority.to_owned(),
            confidence,
            notes: notes.to_owned(),
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
    fn catalog_definition_is_consistent_and_complete() {
        validate_catalog(&CATALOG).expect("the built-in catalog should be valid");

        assert_eq!(CATALOG.sources.len(), 2);
        assert_eq!(CATALOG.sections.len(), 10);
        assert_eq!(CATALOG.topics.len(), 182);

        let csir = CATALOG
            .sources
            .iter()
            .find(|source| source.code == syllabus_catalog::SOURCE_CSIR)
            .expect("CSIR source should exist");
        assert_eq!(csir.title, "Proposed Syllabus for Physical Sciences");
        assert!(
            csir.url
                .ends_with("20201221140202824physicalscience_syllbus.pdf")
        );

        let nta = CATALOG
            .sources
            .iter()
            .find(|source| source.code == syllabus_catalog::SOURCE_NTA)
            .expect("NTA source should exist");
        assert_eq!(
            nta.title,
            "Joint CSIR-UGC NET December 2026 Information Bulletin"
        );

        let general_aptitude_titles: Vec<&str> = CATALOG
            .topics
            .iter()
            .filter(|topic| topic.section_code == "GA")
            .map(|topic| topic.title)
            .collect();
        assert_eq!(
            general_aptitude_titles,
            vec![
                "Logical reasoning",
                "Graphical analysis",
                "Analytical and numerical ability",
                "Quantitative comparison",
                "Series formation",
                "Puzzles",
            ]
        );

        let mut categories_by_section: HashMap<&str, HashSet<&str>> = HashMap::new();
        for topic in CATALOG.topics {
            categories_by_section
                .entry(topic.section_code)
                .or_default()
                .insert(topic.category);
        }

        for section in ["MMP", "CM", "EMT", "QM", "TSP", "EEM"] {
            assert_eq!(
                categories_by_section.get(section),
                Some(&HashSet::from([CORE, ADVANCED])),
                "section {section} should contain Core and Advanced topics"
            );
        }

        for section in ["AMP", "CMP", "NPP"] {
            assert_eq!(
                categories_by_section.get(section),
                Some(&HashSet::from([ADVANCED])),
                "section {section} should be Advanced only"
            );
        }
    }

    #[test]
    fn seeding_is_idempotent_and_creates_no_progress_rows() {
        let mut connection = empty_connection();

        seed(&mut connection).expect("first seed should succeed");
        let topic_codes: Vec<String> = {
            let mut statement = connection
                .prepare("SELECT code FROM syllabus_topics ORDER BY code")
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };

        seed(&mut connection).expect("second seed should be idempotent");

        let repeat_codes: Vec<String> = {
            let mut statement = connection
                .prepare("SELECT code FROM syllabus_topics ORDER BY code")
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };

        assert_eq!(count(&connection, "syllabus_sources"), 2);
        assert_eq!(count(&connection, "syllabus_sections"), 10);
        assert_eq!(count(&connection, "syllabus_topics"), 182);
        assert_eq!(count(&connection, "syllabus_progress"), 0);
        assert_eq!(topic_codes, repeat_codes);
    }

    #[test]
    fn opening_a_database_seeds_the_catalog_without_fabricated_progress() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        let path = directory.path().join("tracker.sqlite3");
        let connection = crate::db::open_database(
            &path,
            chrono::NaiveDate::from_ymd_opt(2026, 10, 29).unwrap(),
        )
        .expect("database should open");

        assert_eq!(count(&connection, "syllabus_sources"), 2);
        assert_eq!(count(&connection, "syllabus_sections"), 10);
        assert_eq!(count(&connection, "syllabus_topics"), 182);
        assert_eq!(count(&connection, "syllabus_progress"), 0);
    }

    #[test]
    fn search_supports_text_code_category_section_and_progress_filters() {
        let mut connection = test_connection();
        let topic_id = topic_id(&connection, "GA-01");

        update_progress(
            &mut connection,
            topic_id,
            progress_input("Learning", "High-Yield", 4, "Practice daily."),
        )
        .expect("progress should save");

        let all = search_topics(&connection, SyllabusFilter::default()).unwrap();
        assert_eq!(all.len(), 182);
        assert_eq!(all.first().unwrap().code, "GA-01");
        assert_eq!(all.last().unwrap().code, "NPP-A22");

        let text = search_topics(
            &connection,
            SyllabusFilter {
                query: Some("phase space".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(text.len(), 3);
        assert!(
            text.iter()
                .all(|topic| topic.title.to_lowercase().contains("phase space"))
        );
        let text_codes: Vec<&str> = text.iter().map(|topic| topic.code.as_str()).collect();
        assert_eq!(text_codes, vec!["CM-C02", "CM-A01", "TSP-C05"]);

        let code = search_topics(
            &connection,
            SyllabusFilter {
                query: Some("  QM-A03  ".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].code, "QM-A03");

        let wildcard = search_topics(
            &connection,
            SyllabusFilter {
                query: Some("%".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert!(wildcard.is_empty(), "LIKE metacharacters must be escaped");

        let section = search_topics(
            &connection,
            SyllabusFilter {
                section_code: Some("MMP".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(section.len(), 20);

        let core = search_topics(
            &connection,
            SyllabusFilter {
                category: Some(CORE.to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(core.len(), 78);
        assert!(core.iter().all(|topic| topic.category == CORE));

        let learning = search_topics(
            &connection,
            SyllabusFilter {
                status: Some("Learning".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(learning.len(), 1);
        assert_eq!(learning[0].code, "GA-01");
        assert_eq!(learning[0].progress.priority, "High-Yield");
        assert!(learning[0].progress.recorded);

        let high_yield = search_topics(
            &connection,
            SyllabusFilter {
                priority: Some("High-Yield".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(high_yield.len(), 1);

        let combined = search_topics(
            &connection,
            SyllabusFilter {
                section_code: Some("GA".to_owned()),
                status: Some("Learning".to_owned()),
                priority: Some("High-Yield".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].code, "GA-01");

        let not_started = search_topics(
            &connection,
            SyllabusFilter {
                status: Some("Not started".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap();
        assert_eq!(not_started.len(), 181);
    }

    #[test]
    fn search_rejects_invalid_filters() {
        let connection = test_connection();

        let error = search_topics(
            &connection,
            SyllabusFilter {
                query: Some("x".repeat(MAX_SEARCH_QUERY_CHARS + 1)),
                ..SyllabusFilter::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.field, Some("query"));

        let error = search_topics(
            &connection,
            SyllabusFilter {
                section_code: Some("UNKNOWN".to_owned()),
                ..SyllabusFilter::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.field, Some("sectionCode"));

        for (filter, expected_field) in [
            (
                SyllabusFilter {
                    category: Some("Part A".to_owned()),
                    ..SyllabusFilter::default()
                },
                "category",
            ),
            (
                SyllabusFilter {
                    status: Some("Done".to_owned()),
                    ..SyllabusFilter::default()
                },
                "status",
            ),
            (
                SyllabusFilter {
                    priority: Some("Urgent".to_owned()),
                    ..SyllabusFilter::default()
                },
                "priority",
            ),
        ] {
            let error = search_topics(&connection, filter).unwrap_err();
            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some(expected_field));
        }
    }

    #[test]
    fn progress_round_trips_and_updates_in_place() {
        let mut connection = test_connection();
        let id = topic_id(&connection, "GA-01");

        let first = update_progress(
            &mut connection,
            id,
            progress_input("Learning", "High", 4, "Read chapter 1."),
        )
        .expect("progress should save");

        assert_eq!(first.status, "Learning");
        assert_eq!(first.priority, "High");
        assert_eq!(first.confidence, 4);
        assert_eq!(first.notes, "Read chapter 1.");
        assert!(first.recorded);
        assert!(first.updated_at.is_some());
        assert_eq!(load_progress(&connection, id).unwrap(), first);

        let second = update_progress(
            &mut connection,
            id,
            progress_input("Revised", "High-Yield", 5, "Revised and practised."),
        )
        .expect("progress should update");

        assert_eq!(second.status, "Revised");
        assert_eq!(count(&connection, "syllabus_progress"), 1);
        assert_eq!(load_progress(&connection, id).unwrap(), second);

        let untouched = load_progress(&connection, topic_id(&connection, "GA-02")).unwrap();
        assert_eq!(untouched.status, DEFAULT_STATUS);
        assert_eq!(untouched.priority, DEFAULT_PRIORITY);
        assert_eq!(untouched.confidence, DEFAULT_CONFIDENCE);
        assert!(!untouched.recorded);
        assert_eq!(count(&connection, "syllabus_progress"), 1);
    }

    #[test]
    fn progress_validation_rejects_invalid_values_without_writing() {
        let mut connection = test_connection();
        let id = topic_id(&connection, "GA-01");

        for (input, expected_field) in [
            (progress_input("Done", "Normal", 3, ""), "status"),
            (progress_input("Learning", "Urgent", 3, ""), "priority"),
            (progress_input("Learning", "Normal", 0, ""), "confidence"),
            (progress_input("Learning", "Normal", 6, ""), "confidence"),
            (
                progress_input("Learning", "Normal", 3, &"a".repeat(MAX_NOTES_CHARS + 1)),
                "notes",
            ),
        ] {
            let error = update_progress(&mut connection, id, input).unwrap_err();
            assert_eq!(error.code, "validation_error");
            assert_eq!(error.field, Some(expected_field));
        }

        let error = update_progress(
            &mut connection,
            0,
            progress_input("Learning", "Normal", 3, ""),
        )
        .unwrap_err();
        assert_eq!(error.field, Some("topicId"));

        let error = update_progress(
            &mut connection,
            999_999,
            progress_input("Learning", "Normal", 3, ""),
        )
        .unwrap_err();
        assert_eq!(error.code, "topic_not_found");
        assert_eq!(count(&connection, "syllabus_progress"), 0);
    }

    #[test]
    fn archived_topics_cannot_be_updated_from_the_active_catalog() {
        let mut connection = test_connection();
        let id = topic_id(&connection, "GA-01");

        connection
            .execute(
                "UPDATE syllabus_topics
                 SET is_archived = 1,
                     archived_at = '2026-10-29T00:00:00.000Z'
                 WHERE id = ?1",
                [id],
            )
            .unwrap();

        let error = update_progress(
            &mut connection,
            id,
            progress_input("Learning", "Normal", 3, ""),
        )
        .unwrap_err();
        assert_eq!(error.code, "topic_not_found");

        let error = load_progress(&connection, id).unwrap_err();
        assert_eq!(error.code, "topic_not_found");
    }

    #[test]
    fn summary_uses_equal_weights_and_the_documented_status_scores() {
        let mut connection = empty_connection();
        let reduced_topics = &CATALOG.topics[..4];
        let reduced = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: reduced_topics,
        };
        seed_catalog(&mut connection, &reduced).expect("reduced catalog should seed");

        let statuses = [
            ("GA-01", "Learning"),
            ("GA-02", "Revised"),
            ("GA-03", "Confident"),
            ("GA-04", "Not started"),
        ];

        for (code, status) in statuses {
            let id = topic_id(&connection, code);
            update_progress(&mut connection, id, progress_input(status, "Normal", 3, "")).unwrap();
        }

        let summary = summary(&connection).unwrap();

        assert_eq!(summary.total_topics, 4);
        assert_eq!(
            summary.status_counts,
            StatusCounts {
                not_started: 1,
                learning: 1,
                revised: 1,
                confident: 1,
            }
        );
        assert!((summary.approximate_progress - 56.25).abs() < 1e-9);

        let general_aptitude = summary
            .sections
            .iter()
            .find(|section| section.code == "GA")
            .unwrap();
        assert_eq!(general_aptitude.total_topics, 4);
        assert!((general_aptitude.approximate_progress - 56.25).abs() < 1e-9);

        let empty_section = summary
            .sections
            .iter()
            .find(|section| section.code == "MMP")
            .unwrap();
        assert_eq!(empty_section.total_topics, 0);
        assert_eq!(empty_section.approximate_progress, 0.0);
        assert_eq!(
            empty_section.status_counts,
            StatusCounts {
                not_started: 0,
                learning: 0,
                revised: 0,
                confident: 0,
            }
        );
    }

    #[test]
    fn empty_catalog_summary_is_zero_not_undefined() {
        let mut connection = empty_connection();
        let empty = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: &CATALOG.topics[..0],
        };
        seed_catalog(&mut connection, &empty).expect("empty catalog should seed");

        let summary = summary(&connection).unwrap();
        assert_eq!(summary.total_topics, 0);
        assert_eq!(summary.approximate_progress, 0.0);
        assert_eq!(summary.status_counts.not_started, 0);
        assert_eq!(summary.sections.len(), 10);
    }

    #[test]
    fn catalog_changes_preserve_progress_and_archive_removed_topics() {
        let mut connection = test_connection();
        let id = topic_id(&connection, "QM-C03");

        update_progress(
            &mut connection,
            id,
            progress_input(
                "Confident",
                "High-Yield",
                5,
                "Worked through oscillator problems.",
            ),
        )
        .expect("progress should save");

        let reduced = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: &CATALOG.topics[..4],
        };
        seed_catalog(&mut connection, &reduced).expect("reduced catalog should seed");

        let archived: (i64, Option<String>) = connection
            .query_row(
                "SELECT is_archived, archived_at FROM syllabus_topics WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(archived.0, 1);
        assert!(archived.1.is_some());

        let preserved: (String, String, i64, String) = connection
            .query_row(
                "SELECT status, priority, confidence, notes
                 FROM syllabus_progress WHERE topic_id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            preserved,
            (
                "Confident".to_owned(),
                "High-Yield".to_owned(),
                5,
                "Worked through oscillator problems.".to_owned(),
            )
        );

        let active = search_topics(&connection, SyllabusFilter::default()).unwrap();
        assert_eq!(active.len(), 4);
        assert!(!active.iter().any(|topic| topic.id == id));
        assert_eq!(summary(&connection).unwrap().total_topics, 4);

        seed(&mut connection).expect("full catalog should restore removed topics");

        let restored: (i64, Option<String>) = connection
            .query_row(
                "SELECT is_archived, archived_at FROM syllabus_topics WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(restored, (0, None));
        assert_eq!(load_progress(&connection, id).unwrap().status, "Confident");
        assert_eq!(
            search_topics(&connection, SyllabusFilter::default())
                .unwrap()
                .len(),
            182
        );
    }

    #[test]
    fn database_constraints_reject_invalid_progress_and_catalog_values() {
        let connection = test_connection();
        let id = topic_id(&connection, "GA-01");

        for sql in [
            format!(
                "INSERT INTO syllabus_progress (
                    topic_id, status, priority, confidence, notes, created_at, updated_at
                 ) VALUES ({id}, 'Done', 'Normal', 3, '', '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z')"
            ),
            format!(
                "INSERT INTO syllabus_progress (
                    topic_id, status, priority, confidence, notes, created_at, updated_at
                 ) VALUES ({id}, 'Learning', 'Urgent', 3, '', '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z')"
            ),
            format!(
                "INSERT INTO syllabus_progress (
                    topic_id, status, priority, confidence, notes, created_at, updated_at
                 ) VALUES ({id}, 'Learning', 'Normal', 6, '', '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z')"
            ),
            format!(
                "INSERT INTO syllabus_progress (
                    topic_id, status, priority, confidence, notes, created_at, updated_at
                 ) VALUES ({id}, 'Learning', 'Normal', 3, '{}', '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z')",
                "a".repeat(MAX_NOTES_CHARS + 1)
            ),
            "UPDATE syllabus_topics SET is_archived = 1 WHERE code = 'GA-01'".to_owned(),
            "UPDATE syllabus_topics SET category = 'Part A' WHERE code = 'GA-01'".to_owned(),
            "INSERT INTO syllabus_progress (
                topic_id, status, priority, confidence, notes, created_at, updated_at
             ) VALUES (999999, 'Learning', 'Normal', 3, '', '2026-10-29T00:00:00.000Z', '2026-10-29T00:00:00.000Z')".to_owned(),
        ] {
            assert!(
                connection.execute(&sql, []).is_err(),
                "constraint should reject: {sql}"
            );
        }
    }

    #[test]
    fn catalog_validation_rejects_duplicate_codes_and_cross_source_topics() {
        let duplicate_topics = [
            TopicDef {
                code: "GA-01",
                section_code: "GA",
                title: "Duplicate",
                category: GENERAL_APTITUDE,
                source_code: syllabus_catalog::SOURCE_NTA,
                source_reference: "NTA December 2026 bulletin — General Aptitude",
                display_order: 1,
            },
            TopicDef {
                code: "GA-01",
                section_code: "GA",
                title: "Duplicate again",
                category: GENERAL_APTITUDE,
                source_code: syllabus_catalog::SOURCE_NTA,
                source_reference: "NTA December 2026 bulletin — General Aptitude",
                display_order: 2,
            },
        ];
        let duplicate_catalog = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: &duplicate_topics,
        };
        assert!(validate_catalog(&duplicate_catalog).is_err());

        let cross_source_topics = [TopicDef {
            code: "MMP-C01",
            section_code: "MMP",
            title: "Cross source",
            category: CORE,
            source_code: syllabus_catalog::SOURCE_NTA,
            source_reference: "NTA December 2026 bulletin — General Aptitude",
            display_order: 1,
        }];
        let cross_source_catalog = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: &cross_source_topics,
        };
        assert!(validate_catalog(&cross_source_catalog).is_err());
    }

    #[test]
    fn catalog_validation_rejects_core_topics_in_advanced_only_units() {
        let invalid_topics = [TopicDef {
            code: "AMP-C01",
            section_code: "AMP",
            title: "Invalid core topic",
            category: CORE,
            source_code: syllabus_catalog::SOURCE_CSIR,
            source_reference: "CSIR syllabus PDF — Core",
            display_order: 1,
        }];
        let invalid_catalog = CatalogDef {
            sources: CATALOG.sources,
            sections: CATALOG.sections,
            topics: &invalid_topics,
        };

        assert!(validate_catalog(&invalid_catalog).is_err());
    }

    #[test]
    fn source_metadata_is_seeded_verbatim() {
        let connection = test_connection();
        let catalog = load_catalog(&connection).unwrap();

        assert_eq!(catalog.sources.len(), 2);
        assert_eq!(catalog.sections.len(), 10);
        assert_eq!(catalog.sections[0].code, "GA");
        assert_eq!(
            catalog.sources[0].url,
            "https://csirhrdg.res.in/SiteContent/ManagedContent/ContentFiles/20201221140202824physicalscience_syllbus.pdf"
        );
        assert_eq!(catalog.sources[0].verified_on, "2026-10-29");
        assert_eq!(
            catalog.sources[1].url,
            "https://cdnbbsr.s3waas.gov.in/s3efdf562ce2fb0ad460fd8e9d33e57f57/uploads/2026/10/202610071407707951.pdf"
        );
    }

    #[test]
    fn the_nine_physical_sciences_units_keep_their_documented_order() {
        let connection = test_connection();
        let catalog = load_catalog(&connection).unwrap();
        let titles: Vec<&str> = catalog
            .sections
            .iter()
            .skip(1)
            .map(|section| section.title.as_str())
            .collect();

        assert_eq!(
            titles,
            vec![
                "Mathematical Methods of Physics",
                "Classical Mechanics",
                "Electromagnetic Theory",
                "Quantum Mechanics",
                "Thermodynamic and Statistical Physics",
                "Electronics and Experimental Methods",
                "Atomic & Molecular Physics",
                "Condensed Matter Physics",
                "Nuclear and Particle Physics",
            ]
        );
    }
}
