use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SyllabusNode {
    pub id: i64,
    pub exam_id: i64,
    pub parent_id: Option<i64>,
    pub code: String,
    pub title: String,
    pub node_kind: String,
    pub category: String,
    pub display_order: i64,
    pub readiness_weight: f64,
    pub is_archived: bool,
    pub source_title: String,
    pub source_verified_on: String,
    pub source_url: Option<String>,
    pub source_reference: String,
    pub content_version_label: String,
    pub content_review_status: String,
    pub progress_status: String,
    pub priority: String,
    pub confidence: i64,
    pub notes: String,
    pub node_path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SyllabusSummary {
    pub node_count: i64,
    pub topic_count: i64,
    pub revised_or_confident_count: i64,
    pub mapped_concept_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ExamSyllabus {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub exam_is_active: bool,
    pub configured: bool,
    pub content_version_label: Option<String>,
    pub content_review_status: Option<String>,
    pub nodes: Vec<V2SyllabusNode>,
    pub summary: V2SyllabusSummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SyllabusExamInput {
    pub exam_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2SyllabusProgressInput {
    pub exam_id: i64,
    pub node_id: i64,
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_confidence")]
    pub confidence: i64,
    #[serde(default)]
    pub notes: String,
}

fn default_priority() -> String {
    "normal".to_owned()
}

fn default_confidence() -> i64 {
    3
}

const SYLLABUS_SEED_JSON: &str = include_str!("syllabus_seed.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyllabusSeed {
    exam_code: String,
    content_version_label: String,
    content_review_status: String,
    source: SyllabusSeedSource,
    nodes: Vec<SyllabusSeedNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyllabusSeedSource {
    code: String,
    title: String,
    publisher: String,
    url: String,
    source_reference: String,
    verified_on: String,
    #[serde(default)]
    document_year: String,
    #[serde(default)]
    notes: String,
    /// Apply an approved provenance-note correction to an existing source
    /// row without changing its ID, content version, or node relationships.
    #[serde(default)]
    refresh_notes: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyllabusSeedNode {
    code: String,
    title: String,
    kind: String,
    #[serde(default)]
    parent_code: Option<String>,
    #[serde(default)]
    category: String,
    display_order: i64,
    /// Apply an approved title correction to an existing node in place.
    /// The immutable code and the node ID stay unchanged.
    #[serde(default)]
    refresh_title: bool,
}

/// Add or refresh the verified official syllabus content idempotently.
/// Approved title and provenance corrections are applied only when the seed
/// marks them explicitly; node codes, node IDs and relationships are never
/// replaced by this function.
pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    let seeds: Vec<SyllabusSeed> = serde_json::from_str(SYLLABUS_SEED_JSON)
        .map_err(|error| AppError::internal(format!("The syllabus seed is invalid: {error}")))?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The syllabus seed transaction could not start", error)
    })?;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    for seed in seeds {
        let exam_id: Option<i64> = transaction
            .query_row(
                "SELECT id FROM exams WHERE code = ?1",
                [&seed.exam_code],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| AppError::database("The syllabus exam could not be read", error))?;
        let Some(exam_id) = exam_id else {
            continue;
        };

        let content_notes = format!(
            "{} GitHub/official syllabus ingestion; document: {}",
            seed.source.notes, seed.source.document_year
        );
        let source_notes = format!(
            "{} Verified source; document version: {}.",
            seed.source.notes, seed.source.document_year
        );

        transaction
            .execute(
                "INSERT INTO exam_content_versions (
                    exam_id, content_type, version_label, source_reference, source_url,
                    published_on, verified_on, review_status, reviewed_by, notes,
                    created_at
                 ) VALUES (?1, 'syllabus', ?2, ?3, ?4, NULL, ?5, ?6, 'Official source register',
                           ?7, ?8)
                 ON CONFLICT(exam_id, content_type, version_label) DO NOTHING",
                params![
                    exam_id,
                    seed.content_version_label,
                    seed.source.source_reference,
                    seed.source.url,
                    seed.source.verified_on,
                    seed.content_review_status,
                    &content_notes,
                    now,
                ],
            )
            .map_err(|error| {
                AppError::database("The syllabus content version could not be stored", error)
            })?;
        let content_version_id: i64 = transaction
            .query_row(
                "SELECT id FROM exam_content_versions
                 WHERE exam_id = ?1 AND content_type = 'syllabus' AND version_label = ?2",
                params![exam_id, seed.content_version_label],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("The syllabus content version could not be reloaded", error)
            })?;

        transaction
            .execute(
                "INSERT INTO exam_syllabus_sources (
                    exam_id, content_version_id, code, title, publisher, url,
                    source_reference, verified_on, notes, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                 ON CONFLICT(exam_id, code) DO NOTHING",
                params![
                    exam_id,
                    content_version_id,
                    seed.source.code,
                    seed.source.title,
                    seed.source.publisher,
                    seed.source.url,
                    seed.source.source_reference,
                    seed.source.verified_on,
                    &source_notes,
                    now,
                ],
            )
            .map_err(|error| {
                AppError::database("The syllabus source could not be stored", error)
            })?;
        let source_id: i64 = transaction
            .query_row(
                "SELECT id FROM exam_syllabus_sources WHERE exam_id = ?1 AND code = ?2",
                params![exam_id, seed.source.code],
                |row| row.get(0),
            )
            .map_err(|error| {
                AppError::database("The syllabus source could not be reloaded", error)
            })?;

        if seed.source.refresh_notes {
            transaction
                .execute(
                    "UPDATE exam_content_versions
                     SET notes = ?2
                     WHERE id = ?1 AND notes <> ?2",
                    params![content_version_id, &content_notes],
                )
                .map_err(|error| {
                    AppError::database(
                        "The approved syllabus content-version note could not be refreshed",
                        error,
                    )
                })?;
            transaction
                .execute(
                    "UPDATE exam_syllabus_sources
                     SET notes = ?3, updated_at = ?4
                     WHERE exam_id = ?1 AND code = ?2 AND notes <> ?3",
                    params![exam_id, seed.source.code, &source_notes, now],
                )
                .map_err(|error| {
                    AppError::database(
                        "The approved syllabus source note could not be refreshed",
                        error,
                    )
                })?;
        }

        let mut ids = HashMap::new();
        for node in seed.nodes {
            let parent_id = node
                .parent_code
                .as_deref()
                .and_then(|code| ids.get(code).copied());
            transaction
                .execute(
                    "INSERT INTO exam_syllabus_nodes (
                        exam_id, parent_id, source_id, code, title, node_kind, category,
                        display_order, readiness_weight, source_reference, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, '', ?9, ?9)
                     ON CONFLICT(exam_id, code) DO NOTHING",
                    params![
                        exam_id,
                        parent_id,
                        source_id,
                        node.code,
                        node.title,
                        node.kind,
                        node.category,
                        node.display_order,
                        now,
                    ],
                )
                .map_err(|error| {
                    AppError::database("A syllabus node could not be stored", error)
                })?;
            let node_id: i64 = transaction
                .query_row(
                    "SELECT id FROM exam_syllabus_nodes WHERE exam_id = ?1 AND code = ?2",
                    params![exam_id, node.code],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    AppError::database("A syllabus node could not be reloaded", error)
                })?;
            if node.refresh_title {
                transaction
                    .execute(
                        "UPDATE exam_syllabus_nodes
                         SET title = ?3, updated_at = ?4
                         WHERE exam_id = ?1 AND code = ?2
                           AND is_archived = 0 AND title <> ?3",
                        params![exam_id, node.code, node.title, now],
                    )
                    .map_err(|error| {
                        AppError::database(
                            "The approved syllabus topic title could not be refreshed",
                            error,
                        )
                    })?;
            }
            ids.insert(node.code, node_id);
        }
    }

    transaction
        .commit()
        .map_err(|error| AppError::database("The syllabus seed could not be committed", error))
}

pub fn get_exam_syllabus(
    connection: &Connection,
    input: V2SyllabusExamInput,
) -> Result<V2ExamSyllabus, AppError> {
    let (exam_id, exam_code, exam_name, exam_is_active) = connection
        .query_row(
            "SELECT e.id, e.code, e.name, p.is_active
             FROM exams e JOIN exam_profiles p ON p.exam_id = e.id
             WHERE e.id = ?1",
            [input.exam_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? == 1,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("Examination could not be loaded", error))?
        .ok_or_else(|| {
            AppError::validation("examId", "The selected examination does not exist.")
        })?;

    let content = connection
        .query_row(
            "SELECT cv.id, cv.version_label, cv.review_status
             FROM exam_content_versions cv
             WHERE cv.exam_id = ?1 AND cv.content_type = 'syllabus'
               AND EXISTS (
                   SELECT 1
                   FROM exam_syllabus_sources s
                   JOIN exam_syllabus_nodes n
                     ON n.exam_id = s.exam_id AND n.source_id = s.id
                   WHERE s.exam_id = cv.exam_id
                     AND s.content_version_id = cv.id
                     AND n.is_archived = 0
               )
             ORDER BY (cv.review_status = 'verified') DESC,
                      cv.verified_on DESC,
                      cv.id DESC
             LIMIT 1",
            [exam_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| AppError::database("Syllabus content version could not be read", error))?;
    let content_version_id = content.as_ref().map(|value| value.0);

    let mut statement = connection
        .prepare(
            "SELECT n.id, n.parent_id, n.code, n.title, n.node_kind, n.category,
                    n.display_order, n.readiness_weight, n.is_archived,
                    s.title, s.verified_on, s.url, s.source_reference, cv.version_label,
                    cv.review_status, COALESCE(prog.status, 'not_started'),
                    COALESCE(prog.priority, 'normal'),
                    COALESCE(prog.confidence, 3),
                    COALESCE(prog.notes, '')
             FROM exam_syllabus_nodes n
             JOIN exam_syllabus_sources s
                ON s.exam_id = n.exam_id AND s.id = n.source_id
             JOIN exam_content_versions cv
                ON cv.exam_id = s.exam_id AND cv.id = s.content_version_id
             LEFT JOIN exam_node_progress prog
                ON prog.exam_id = n.exam_id AND prog.node_id = n.id
             WHERE n.exam_id = ?1
               AND n.is_archived = 0
               AND (?2 IS NULL OR s.content_version_id = ?2)
             ORDER BY n.parent_id, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("Syllabus nodes could not be prepared", error))?;
    let rows = statement
        .query_map(params![exam_id, content_version_id], |row| {
            Ok(V2SyllabusNode {
                id: row.get(0)?,
                exam_id,
                parent_id: row.get(1)?,
                code: row.get(2)?,
                title: row.get(3)?,
                node_kind: row.get(4)?,
                category: row.get(5)?,
                display_order: row.get(6)?,
                readiness_weight: row.get(7)?,
                is_archived: row.get::<_, i64>(8)? == 1,
                source_title: row.get(9)?,
                source_verified_on: row.get(10)?,
                source_url: row.get(11)?,
                source_reference: row.get(12)?,
                content_version_label: row.get(13)?,
                content_review_status: row.get(14)?,
                progress_status: row.get(15)?,
                priority: row.get(16)?,
                confidence: row.get(17)?,
                notes: row.get(18)?,
                node_path: Vec::new(),
            })
        })
        .map_err(|error| AppError::database("Syllabus nodes could not be read", error))?;
    let mut nodes = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A syllabus node row is invalid", error))?;
    drop(statement);

    let node_map: HashMap<i64, (Option<i64>, String)> = nodes
        .iter()
        .map(|node| (node.id, (node.parent_id, node.title.clone())))
        .collect();
    for node in &mut nodes {
        let mut path = Vec::new();
        let mut current = Some(node.id);
        let mut seen = HashSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            if let Some((parent, title)) = node_map.get(&id) {
                path.push(title.clone());
                current = *parent;
            } else {
                break;
            }
        }
        path.reverse();
        node.node_path = path;
    }

    let topic_count = nodes
        .iter()
        .filter(|node| node.node_kind == "topic")
        .count() as i64;
    let revised_or_confident = nodes
        .iter()
        .filter(|node| matches!(node.progress_status.as_str(), "revised" | "confident"))
        .count() as i64;
    let mapped_concept_count: i64 = connection
        .query_row(
            "SELECT COUNT(DISTINCT nc.concept_id)
             FROM exam_node_concepts nc
             JOIN exam_syllabus_nodes n ON n.exam_id = nc.exam_id AND n.id = nc.node_id
             WHERE nc.exam_id = ?1 AND n.is_archived = 0",
            [exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Syllabus mapping count could not be read", error))?;

    Ok(V2ExamSyllabus {
        exam_id,
        exam_code,
        exam_name,
        exam_is_active,
        configured: content.is_some() && !nodes.is_empty(),
        content_version_label: content.as_ref().map(|value| value.1.clone()),
        content_review_status: content.map(|value| value.2),
        nodes,
        summary: V2SyllabusSummary {
            node_count: node_map.len() as i64,
            topic_count,
            revised_or_confident_count: revised_or_confident,
            mapped_concept_count,
        },
    })
}

pub fn update_node_progress(
    connection: &mut Connection,
    input: V2SyllabusProgressInput,
    now: DateTime<Utc>,
) -> Result<V2ExamSyllabus, AppError> {
    if !matches!(
        input.status.as_str(),
        "not_started" | "learning" | "revised" | "confident"
    ) {
        return Err(AppError::validation(
            "status",
            "Choose not started, learning, revised, or confident.",
        ));
    }
    if !matches!(
        input.priority.as_str(),
        "low" | "normal" | "high" | "high_yield"
    ) {
        return Err(AppError::validation(
            "priority",
            "Choose low, normal, high, or high-yield priority.",
        ));
    }
    if !(1..=5).contains(&input.confidence) {
        return Err(AppError::validation(
            "confidence",
            "Confidence must be between 1 and 5.",
        ));
    }
    if input.notes.chars().count() > 4_000 {
        return Err(AppError::validation(
            "notes",
            "Syllabus notes must be 4,000 characters or fewer.",
        ));
    }
    let active: i64 = connection
        .query_row(
            "SELECT is_active FROM exam_profiles WHERE exam_id = ?1",
            [input.exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Examination could not be checked", error))?
        .ok_or_else(|| {
            AppError::validation("examId", "The selected examination does not exist.")
        })?;
    if active != 1 {
        return Err(AppError::forbidden(
            "examId",
            "Inactive examinations cannot receive syllabus progress updates.",
        ));
    }
    let node_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM exam_syllabus_nodes
                WHERE exam_id = ?1 AND id = ?2 AND is_archived = 0
             )",
            params![input.exam_id, input.node_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Syllabus node could not be checked", error))?;
    if node_exists == 0 {
        return Err(AppError::validation(
            "nodeId",
            "Choose an active syllabus node from this examination.",
        ));
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    connection
        .execute(
            "INSERT INTO exam_node_progress (
                exam_id, node_id, status, priority, confidence, notes,
                last_studied_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7)
             ON CONFLICT(node_id) DO UPDATE SET
                status = excluded.status,
                priority = excluded.priority,
                confidence = excluded.confidence,
                notes = excluded.notes,
                last_studied_at = excluded.last_studied_at,
                updated_at = excluded.updated_at",
            params![
                input.exam_id,
                input.node_id,
                input.status,
                input.priority,
                input.confidence,
                input.notes.trim(),
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Syllabus progress could not be saved", error))?;
    get_exam_syllabus(
        connection,
        V2SyllabusExamInput {
            exam_id: input.exam_id,
        },
    )
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

    fn node_ids(connection: &Connection, exam_id: i64) -> Result<HashMap<String, i64>, AppError> {
        let mut statement = connection
            .prepare("SELECT code, id FROM exam_syllabus_nodes WHERE exam_id = ?1")
            .map_err(|error| AppError::database("syllabus node IDs", error))?;
        let rows = statement
            .query_map([exam_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|error| AppError::database("syllabus node IDs", error))?;
        rows.collect::<rusqlite::Result<HashMap<_, _>>>()
            .map_err(|error| AppError::database("syllabus node IDs", error))
    }

    fn node_titles(
        connection: &Connection,
        exam_id: i64,
    ) -> Result<HashMap<String, String>, AppError> {
        let mut statement = connection
            .prepare("SELECT code, title FROM exam_syllabus_nodes WHERE exam_id = ?1")
            .map_err(|error| AppError::database("syllabus node titles", error))?;
        let rows = statement
            .query_map([exam_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| AppError::database("syllabus node titles", error))?;
        rows.collect::<rusqlite::Result<HashMap<_, _>>>()
            .map_err(|error| AppError::database("syllabus node titles", error))
    }

    fn add_session_allocation(
        connection: &Connection,
        exam_id: i64,
        node_id: i64,
    ) -> Result<i64, AppError> {
        let now = "2026-11-04T10:00:00Z";
        connection
            .execute(
                "INSERT INTO study_sessions (
                    session_date, duration_seconds, source, notes, created_at, updated_at
                 ) VALUES ('2026-11-04', 4, 'manual', '', ?1, ?1)",
                [now],
            )
            .map_err(|error| AppError::database("fixture session", error))?;
        let session_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO session_topic_allocations (
                    session_id, exam_id, node_id, allocated_seconds, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 4, ?4, ?4)",
                params![session_id, exam_id, node_id, now],
            )
            .map_err(|error| AppError::database("fixture session allocation", error))?;
        Ok(session_id)
    }

    fn add_fixture(connection: &Connection) -> i64 {
        let now = "2026-11-04T10:00:00Z";
        connection
            .execute(
                "INSERT INTO exam_content_versions (
                    exam_id, content_type, version_label, source_reference,
                    verified_on, review_status, created_at
                 ) VALUES (1, 'syllabus', 'v1', 'fixture', '2026-12-31', 'verified', ?1)",
                [now],
            )
            .unwrap();
        let version = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_sources (
                    exam_id, content_version_id, code, title, publisher,
                    source_reference, verified_on, created_at, updated_at
                 ) VALUES (1, ?1, 'S1', 'Fixture source', 'Fixture',
                           'fixture', '2026-11-04', ?2, ?2)",
                params![version, now],
            )
            .unwrap();
        let source = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes (
                    exam_id, source_id, code, title, node_kind, category,
                    display_order, created_at, updated_at
                 ) VALUES (1, ?1, 'U1', 'Mechanics', 'unit', 'Mechanics', 1, ?2, ?2)",
                params![source, now],
            )
            .unwrap();
        let unit = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes (
                    exam_id, parent_id, source_id, code, title, node_kind, category,
                    display_order, created_at, updated_at
                 ) VALUES (1, ?1, ?2, 'T1', 'Kinematics', 'topic', 'Mechanics', 1, ?3, ?3)",
                params![unit, source, now],
            )
            .unwrap();
        connection.last_insert_rowid()
    }

    #[test]
    fn official_syllabi_are_seeded_for_all_four_exams() {
        let (_root, state) = state();
        for (exam_id, node_count, topic_count, root_title) in [
            (1, 158, 141, "Part A - Core"),
            (2, 104, 92, "GATE 2027 Physics (PH) Syllabus"),
            (3, 17, 8, "GS 2024 Physics Entrance Test"),
            (4, 49, 42, "JEST 2026 Physics Syllabus"),
        ] {
            let syllabus = state
                .with_connection(|connection| {
                    get_exam_syllabus(connection, V2SyllabusExamInput { exam_id })
                })
                .expect("seeded syllabus");
            assert!(syllabus.configured);
            assert_eq!(syllabus.summary.node_count, node_count);
            assert_eq!(syllabus.summary.topic_count, topic_count);
            assert_eq!(syllabus.content_review_status.as_deref(), Some("verified"));
            assert!(syllabus.content_version_label.is_some());
            assert_eq!(syllabus.nodes[0].title, root_title);
            assert!(
                syllabus
                    .nodes
                    .iter()
                    .all(|node| node.content_review_status == "verified")
            );
        }
        let csir = state
            .with_connection(|connection| {
                get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 1 })
            })
            .expect("CSIR syllabus");
        let dimensional = csir
            .nodes
            .iter()
            .find(|node| node.title == "Dimensional analysis")
            .expect("known official topic");
        assert_eq!(
            dimensional.node_path,
            vec![
                "Part A - Core",
                "Mathematical Methods of Physics",
                "Dimensional analysis"
            ]
        );
        assert_eq!(dimensional.source_verified_on, "2026-11-05");
        assert!(dimensional.source_url.is_some());
    }

    #[test]
    fn syllabus_version_selection_ignores_empty_versions_and_never_mixes_nodes() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let now = "2026-11-10T00:00:00Z";
                connection
                    .execute(
                        "INSERT INTO exam_content_versions (
                            exam_id, content_type, version_label, source_reference,
                            verified_on, review_status, created_at
                         ) VALUES (1, 'syllabus', 'Newer empty fixture', 'fixture',
                                   '2026-12-01', 'verified', ?1)",
                        [now],
                    )
                    .expect("empty version");
                let with_empty = get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 1 })
                    .expect("syllabus with empty newer version");
                assert_eq!(with_empty.summary.node_count, 158);
                assert_eq!(
                    with_empty.content_version_label.as_deref(),
                    Some("CSIR-HRDG Physical Sciences syllabus (2020 PDF, verified 2026-11-05)")
                );

                connection
                    .execute(
                        "INSERT INTO exam_content_versions (
                            exam_id, content_type, version_label, source_reference,
                            verified_on, review_status, created_at
                         ) VALUES (1, 'syllabus', 'Exclusive fixture version', 'fixture',
                                   '2026-12-02', 'verified', ?1)",
                        [now],
                    )
                    .expect("exclusive version");
                let version_id = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO exam_syllabus_sources (
                            exam_id, content_version_id, code, title, publisher,
                            source_reference, verified_on, created_at, updated_at
                         ) VALUES (1, ?1, 'EXCLUSIVE', 'Exclusive source', 'Fixture',
                                   'fixture', '2026-12-02', ?2, ?2)",
                        params![version_id, now],
                    )
                    .expect("exclusive source");
                let source_id = connection.last_insert_rowid();
                connection
                    .execute(
                        "INSERT INTO exam_syllabus_nodes (
                            exam_id, source_id, code, title, node_kind,
                            category, display_order, created_at, updated_at
                         ) VALUES (1, ?1, 'exclusive-topic', 'Exclusive topic',
                                   'topic', 'Fixture', 1, ?2, ?2)",
                        params![source_id, now],
                    )
                    .expect("exclusive node");

                let exclusive = get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 1 })
                    .expect("exclusive syllabus");
                assert_eq!(
                    exclusive.content_version_label.as_deref(),
                    Some("Exclusive fixture version")
                );
                assert_eq!(exclusive.summary.node_count, 1);
                assert_eq!(exclusive.nodes[0].code, "exclusive-topic");
                Ok::<(), AppError>(())
            })
            .expect("version selection");
    }

    #[test]
    fn jest_syllabus_is_independent_and_source_verified() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let jest = get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 4 })
                    .expect("JEST syllabus");
                assert!(jest.configured);
                assert_eq!(jest.exam_code, "jest-physics");
                assert_eq!(jest.summary.node_count, 49);
                assert_eq!(jest.summary.topic_count, 42);
                assert_eq!(
                    jest.nodes
                        .iter()
                        .filter(|node| node.node_kind == "unit")
                        .count(),
                    6
                );
                assert!(jest.nodes.iter().all(|node| node.code.starts_with("jest-")));
                assert!(jest.nodes.iter().all(|node| {
                    !node.code.starts_with("csir-")
                        && !node.code.starts_with("gate-")
                        && !node.code.starts_with("tifr-")
                }));
                assert!(jest.nodes.iter().all(|node| {
                    node.source_url.as_deref()
                        == Some(
                            "https://www.jest.org.in/sites/default/files/2025-12/JEST-Syllabus.pdf",
                        )
                }));
                assert!(
                    jest.nodes
                        .iter()
                        .all(|node| { node.content_version_label == "JEST 2026 Physics Syllabus" })
                );

                let topic = jest
                    .nodes
                    .iter()
                    .find(|node| node.node_kind == "topic")
                    .expect("JEST topic");
                let updated = update_node_progress(
                    connection,
                    V2SyllabusProgressInput {
                        exam_id: 4,
                        node_id: topic.id,
                        status: "revised".to_owned(),
                        priority: "high".to_owned(),
                        confidence: 4,
                        notes: "JEST-only progress".to_owned(),
                    },
                    at(10),
                )
                .expect("JEST progress");
                assert_eq!(
                    updated
                        .nodes
                        .iter()
                        .find(|node| node.id == topic.id)
                        .map(|node| node.progress_status.as_str()),
                    Some("revised")
                );
                let csir = get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 1 })
                    .expect("CSIR syllabus");
                assert!(
                    csir.nodes
                        .iter()
                        .all(|node| node.progress_status == "not_started")
                );
                Ok::<(), AppError>(())
            })
            .expect("JEST integrity");
    }

    #[test]
    fn approved_csir_title_corrections_are_refreshed_without_changing_identity() {
        let (_root, state) = state();
        let corrections = [
            (
                "csir-part-a-core-electronics-and-experimental-meth-t001-semiconductor-devices-diodes-junctions-transistors",
                "Semiconductor devices: diodes, junctions, transistors and field effect devices",
                "Semiconductor devices: diodes, junctions, transistors, field-effect devices, device structure, characteristics, frequency dependence and applications",
            ),
            (
                "csir-part-b-advanced-electronics-and-experimental--t002-transducers-for-temperature-pressure-magnetic-fiel",
                "Transducers for temperature, pressure, magnetic fields, vibration, optical and particle measurements",
                "Transducers for temperature, pressure/vacuum, magnetic fields, vibration, optical measurements and particle detectors",
            ),
            (
                "csir-part-b-advanced-electronics-and-experimental--t004-impedance-matching-amplification-filtering-and-noi",
                "Impedance matching, amplification, filtering and noise reduction",
                "Impedance matching, amplification (op-amp based, instrumentation amplifiers, feedback), filtering and noise reduction",
            ),
            (
                "csir-part-b-advanced-nuclear-and-particle-physics-t006-evidence-of-shell-structure-and-single-particle-sh",
                "Evidence of shell structure and single-particle shell model",
                "Evidence of shell structure, single-particle shell model, its validity and limitations",
            ),
            (
                "csir-part-b-advanced-nuclear-and-particle-physics-t012-quantum-numbers-gell-mann-nishijima-formula-and-qu",
                "Quantum numbers, Gell-Mann-Nishijima formula and quark model",
                "Quantum numbers (charge, spin, parity, isospin, strangeness), Gell-Mann-Nishijima formula and quark model",
            ),
            (
                "csir-part-b-advanced-condensed-matter-physics-t014-ordered-phases-of-matter-and-liquid-crystalline-or",
                "Ordered phases of matter and liquid crystalline order",
                "Ordered phases of matter: translational and orientational order, and kinds of liquid crystalline order",
            ),
        ];

        let before = state
            .with_connection(|connection| {
                for (code, old_title, _) in corrections {
                    connection
                        .execute(
                            "UPDATE exam_syllabus_nodes
                             SET title = ?2
                             WHERE exam_id = 1 AND code = ?1 AND is_archived = 0",
                            params![code, old_title],
                        )
                        .map_err(|error| AppError::database("old CSIR title fixture", error))?;
                }
                let ids = node_ids(connection, 1)?;
                let tracked_node = ids
                    .get(corrections[0].0)
                    .copied()
                    .expect("tracked CSIR correction node");
                connection
                    .execute(
                        "INSERT INTO exam_node_progress (
                            exam_id, node_id, status, priority, confidence, notes,
                            created_at, updated_at
                         ) VALUES (1, ?1, 'confident', 'high', 5, 'preserve CSIR progress',
                                   '2026-11-04T10:00:00Z', '2026-11-04T10:00:00Z')",
                        [tracked_node],
                    )
                    .map_err(|error| AppError::database("CSIR progress fixture", error))?;
                let topic_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 1 AND node_kind = 'topic'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("CSIR topic count", error))?;
                let section_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 1 AND node_kind = 'section'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("CSIR section count", error))?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("CSIR mapping count", error))?;
                Ok((ids, topic_count, section_count, mapping_count))
            })
            .expect("CSIR before");

        state
            .with_connection(prepare)
            .expect("approved CSIR preparation");

        let after = state
            .with_connection(|connection| {
                let ids = node_ids(connection, 1)?;
                let titles = node_titles(connection, 1)?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("CSIR mapping count", error))?;
                Ok((ids, titles, mapping_count))
            })
            .expect("CSIR after");

        let preserved_progress = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT p.status, p.priority, p.confidence, p.notes
                         FROM exam_node_progress p
                         JOIN exam_syllabus_nodes n ON n.id = p.node_id
                         WHERE n.exam_id = 1 AND n.code = ?1",
                        [corrections[0].0],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, i64>(2)?,
                                row.get::<_, String>(3)?,
                            ))
                        },
                    )
                    .map_err(|error| AppError::database("CSIR progress preservation", error))
            })
            .expect("CSIR progress preservation");
        assert_eq!(
            preserved_progress,
            (
                "confident".to_owned(),
                "high".to_owned(),
                5,
                "preserve CSIR progress".to_owned()
            )
        );
        assert_eq!(before.0, after.0);
        assert_eq!(before.1, 141);
        assert_eq!(before.2, 2);
        assert_eq!(before.3, 42);
        assert_eq!(after.2, 42);
        for (code, _, new_title) in corrections {
            assert_eq!(after.1.get(code).map(String::as_str), Some(new_title));
        }
        assert!(after.0.contains_key("csir-part-a-core"));
        assert!(after.0.contains_key("csir-part-b-advanced"));
        assert_eq!(
            state
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT COUNT(*) FROM exam_node_concepts
                             WHERE exam_id = 1
                               AND concept_id = (SELECT id FROM concepts WHERE slug = 'classical-mechanics')
                               AND node_id = (SELECT id FROM exam_syllabus_nodes
                                              WHERE exam_id = 1 AND code = 'csir-part-a-core-classical-mechanics')",
                            [],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| AppError::database("CSIR mapping preservation", error))
                })
                .expect("CSIR mapping preservation"),
            1
        );
    }

    #[test]
    fn approved_jest_provenance_notes_refresh_without_touching_syllabus_nodes() {
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| {
                let ids = node_ids(connection, 4)?;
                let titles = node_titles(connection, 4)?;
                let source: (i64, String) = connection
                    .query_row(
                        "SELECT id, notes FROM exam_syllabus_sources
                         WHERE exam_id = 4 AND code = 'jest-2026-physics'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| AppError::database("JEST source", error))?;
                let content: (i64, String, String) = connection
                    .query_row(
                        "SELECT id, version_label, notes FROM exam_content_versions
                         WHERE exam_id = 4 AND content_type = 'syllabus'
                         ORDER BY id DESC LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|error| AppError::database("JEST content version", error))?;
                connection
                    .execute(
                        "UPDATE exam_syllabus_sources SET notes = 'outdated JEST provenance'
                         WHERE id = ?1",
                        [source.0],
                    )
                    .map_err(|error| AppError::database("old JEST source notes", error))?;
                connection
                    .execute(
                        "UPDATE exam_content_versions SET notes = 'outdated JEST provenance'
                         WHERE id = ?1",
                        [content.0],
                    )
                    .map_err(|error| AppError::database("old JEST content notes", error))?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 4",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("JEST mapping count", error))?;
                Ok((ids, titles, source.0, content.0, content.1, mapping_count))
            })
            .expect("JEST before");

        state
            .with_connection(prepare)
            .expect("approved JEST preparation");

        state
            .with_connection(|connection| {
                let ids = node_ids(connection, 4)?;
                let titles = node_titles(connection, 4)?;
                let source: (i64, String, String, String) = connection
                    .query_row(
                        "SELECT id, title, url, notes FROM exam_syllabus_sources
                         WHERE exam_id = 4 AND code = 'jest-2026-physics'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .map_err(|error| AppError::database("JEST source after", error))?;
                let content: (i64, String, String) = connection
                    .query_row(
                        "SELECT id, version_label, notes FROM exam_content_versions
                         WHERE exam_id = 4 AND content_type = 'syllabus'
                         ORDER BY id DESC LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|error| AppError::database("JEST content after", error))?;
                let units: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 4 AND node_kind = 'unit'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("JEST unit count", error))?;
                let topics: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 4 AND node_kind = 'topic'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("JEST topic count", error))?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 4",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("JEST mapping count after", error))?;

                assert_eq!(before.0, ids);
                assert_eq!(before.1, titles);
                assert_eq!(source.0, before.2);
                assert_eq!(content.0, before.3);
                assert_eq!(source.1, "JEST 2026 Physics Syllabus");
                assert_eq!(
                    source.2,
                    "https://www.jest.org.in/sites/default/files/2025-12/JEST-Syllabus.pdf"
                );
                assert_eq!(before.4, "JEST 2026 Physics Syllabus");
                assert_eq!(content.1, before.4);
                assert!(source.3.contains("undated"));
                assert!(source.3.contains("does not independently verify"));
                assert!(content.2.contains("undated"));
                assert!(content.2.contains("does not independently verify"));
                assert_eq!(ids.len(), 49);
                assert_eq!(units, 6);
                assert_eq!(topics, 42);
                assert_eq!(mapping_count, 28);
                assert_eq!(before.5, 28);
                Ok::<(), AppError>(())
            })
            .expect("JEST integrity after provenance refresh");
    }

    #[test]
    fn gate_2027_remains_exclusive_and_preserves_mappings_and_history() {
        const GATE_TRACKER_ONLY: [&str; 19] = [
            "gate-ph-2027-measurements-and-error-analysis-t001-units-and-dimensions-dimensional-analysis",
            "gate-ph-2027-measurements-and-error-analysis-t002-least-count-and-significant-figures",
            "gate-ph-2027-measurements-and-error-analysis-t003-methods-of-measurement-and-error-analysis",
            "gate-ph-2027-measurements-and-error-analysis-t004-2-probe-and-4-probe-methods-for-resistance-measure",
            "gate-ph-2027-measurements-and-error-analysis-t005-grounding-for-electrical-circuits-and-ground-loops",
            "gate-ph-2027-measurements-and-error-analysis-t006-design-of-dc-power-supply",
            "gate-ph-2027-measurements-and-error-analysis-t007-signal-processing-through-lock-in-amplifiers",
            "gate-ph-2027-optical-physics-t001-wave-equation-plane-and-spherical-waves-superposit",
            "gate-ph-2027-optical-physics-t002-interference-spatial-and-temporal-coherence-dielec",
            "gate-ph-2027-optical-physics-t003-michelson-interferometer-fabry-perot-interferomete",
            "gate-ph-2027-optical-physics-t004-diffraction-fresnel-and-fraunhofer-diffraction-rec",
            "gate-ph-2027-optical-physics-t005-diffraction-from-double-slit-and-many-slits",
            "gate-ph-2027-optical-physics-t006-dispersion-by-a-grating",
            "gate-ph-2027-optical-physics-t007-polarization-jones-vectors-and-matrices-birefringe",
            "gate-ph-2027-mathematical-physics-t003-linear-differential-equations-first-and-second-ord",
            "gate-ph-2027-quantum-mechanics-t005-particle-in-single-and-double-delta-function-poten",
            "gate-ph-2027-atomic-and-molecular-physics-t004-zeeman-paschen-back-and-stark-effects",
            "gate-ph-2027-atomic-and-molecular-physics-t009-nmr-esr-x-ray-and-mossbauer-spectroscopies",
            "gate-ph-2027-electronics-t004-waveform-generators-sine-square-and-triangular-wav",
        ];

        let (_root, state) = state();
        let before = state
            .with_connection(|connection| {
                let ids = node_ids(connection, 2)?;
                let titles = node_titles(connection, 2)?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 2",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE mapping count", error))?;
                let root_title: String = connection
                    .query_row(
                        "SELECT title FROM exam_syllabus_nodes
                         WHERE exam_id = 2 AND code = 'gate-ph-2027'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE root", error))?;
                let source_url: String = connection
                    .query_row(
                        "SELECT url FROM exam_syllabus_sources
                         WHERE exam_id = 2 AND code = 'gate-2027-ph'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE source", error))?;
                let version_label: String = connection
                    .query_row(
                        "SELECT version_label FROM exam_content_versions
                         WHERE exam_id = 2 AND content_type = 'syllabus'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE version", error))?;
                let allocation_node = ids
                    .get(GATE_TRACKER_ONLY[0])
                    .copied()
                    .expect("GATE measurement node");
                let session_id = add_session_allocation(connection, 2, allocation_node)?;
                Ok((
                    ids,
                    titles,
                    mapping_count,
                    root_title,
                    source_url,
                    version_label,
                    session_id,
                    allocation_node,
                ))
            })
            .expect("GATE before");

        state
            .with_connection(prepare)
            .expect("GATE preservation preparation");

        state
            .with_connection(|connection| {
                let ids = node_ids(connection, 2)?;
                let titles = node_titles(connection, 2)?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 2",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE mapping count after", error))?;
                let node_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes WHERE exam_id = 2",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE node count", error))?;
                let topic_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 2 AND node_kind = 'topic'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE topic count", error))?;
                let gate_2025_nodes: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 2 AND code LIKE 'gate-ph-2025%'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE 2025 node check", error))?;
                let gate_2025_versions: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_content_versions
                         WHERE exam_id = 2 AND version_label LIKE '%2025%'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE 2025 version check", error))?;
                let root_title: String = connection
                    .query_row(
                        "SELECT title FROM exam_syllabus_nodes
                         WHERE exam_id = 2 AND code = 'gate-ph-2027'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE root after", error))?;
                let source_url: String = connection
                    .query_row(
                        "SELECT url FROM exam_syllabus_sources
                         WHERE exam_id = 2 AND code = 'gate-2027-ph'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE source after", error))?;
                let version_label: String = connection
                    .query_row(
                        "SELECT version_label FROM exam_content_versions
                         WHERE exam_id = 2 AND content_type = 'syllabus'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE version after", error))?;
                let allocation: (i64, i64) = connection
                    .query_row(
                        "SELECT session_id, node_id FROM session_topic_allocations
                         WHERE session_id = ?1 AND exam_id = 2",
                        [before.6],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| AppError::database("GATE allocation after", error))?;

                assert_eq!(before.0, ids);
                assert_eq!(before.1, titles);
                assert_eq!(before.2, 36);
                assert_eq!(mapping_count, 36);
                assert_eq!(before.3, root_title);
                assert_eq!(before.4, source_url);
                assert_eq!(before.5, version_label);
                assert_eq!(node_count, 104);
                assert_eq!(topic_count, 92);
                assert_eq!(gate_2025_nodes, 0);
                assert_eq!(gate_2025_versions, 0);
                assert_eq!(allocation, (before.6, before.7));
                for code in GATE_TRACKER_ONLY {
                    assert!(
                        ids.contains_key(code),
                        "missing preserved GATE entry {code}"
                    );
                }
                Ok::<(), AppError>(())
            })
            .expect("GATE preservation");
    }

    #[test]
    fn tifr_broad_area_placeholders_are_relabelled_without_removal_or_reference_loss() {
        let child_codes = [
            "tifr-gs-2024-classical-mechanics-t001-broad-area-classical-mechanics",
            "tifr-gs-2024-mathematics-relevant-to-physics-t001-broad-area-mathematics-relevant-to-physics",
            "tifr-gs-2024-electricity-and-magnetism-t001-broad-area-electricity-and-magnetism",
            "tifr-gs-2024-quantum-mechanics-t001-broad-area-quantum-mechanics",
            "tifr-gs-2024-heat-thermodynamics-and-statistical-p-t001-broad-area-heat-thermodynamics-and-statistical-phy",
            "tifr-gs-2024-general-physics-t001-broad-area-general-physics",
            "tifr-gs-2024-modern-physics-t001-broad-area-modern-physics",
            "tifr-gs-2024-electronics-and-experimental-physics-t001-broad-area-electronics-and-experimental-physics",
        ];
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| {
                let ids = node_ids(connection, 3)?;
                let titles = node_titles(connection, 3)?;
                let classical_id = ids
                    .get(child_codes[0])
                    .copied()
                    .expect("TIFR classical placeholder");
                for code in child_codes {
                    let old_title = format!(
                        "Broad area: {}",
                        titles
                            .get(code)
                            .expect("TIFR placeholder title")
                            .trim_start_matches("Coverage note: ")
                    );
                    connection
                        .execute(
                            "UPDATE exam_syllabus_nodes SET title = ?2
                             WHERE exam_id = 3 AND code = ?1",
                            params![code, old_title],
                        )
                        .map_err(|error| AppError::database("old TIFR title fixture", error))?;
                }
                let session_id = add_session_allocation(connection, 3, classical_id)?;
                Ok((ids, titles, classical_id, session_id))
            })
            .expect("TIFR before");

        state
            .with_connection(prepare)
            .expect("approved TIFR preparation");

        state
            .with_connection(|connection| {
                let ids = node_ids(connection, 3)?;
                let titles = node_titles(connection, 3)?;
                let node_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes WHERE exam_id = 3",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("TIFR node count", error))?;
                let unit_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 3 AND node_kind = 'unit'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("TIFR unit count", error))?;
                let topic_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_syllabus_nodes
                         WHERE exam_id = 3 AND node_kind = 'topic'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("TIFR topic count", error))?;
                let progress_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_progress WHERE exam_id = 3",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("TIFR progress", error))?;
                let mapping_count: i64 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM exam_node_concepts WHERE exam_id = 3",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("TIFR mapping count", error))?;
                let allocation: (i64, i64) = connection
                    .query_row(
                        "SELECT session_id, node_id FROM session_topic_allocations
                         WHERE session_id = ?1 AND exam_id = 3",
                        [before.3],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| AppError::database("TIFR allocation after", error))?;

                assert_eq!(before.0, ids);
                assert_eq!(node_count, 17);
                assert_eq!(unit_count, 8);
                assert_eq!(topic_count, 8);
                assert_eq!(progress_count, 0);
                assert_eq!(mapping_count, 2);
                assert_eq!(allocation, (before.3, before.2));
                for code in child_codes {
                    let title = titles.get(code).expect("TIFR child after");
                    assert!(title.starts_with("Coverage note: "), "{code} -> {title}");
                    assert!(!title.starts_with("Broad area:"), "{code} -> {title}");
                }
                for unit in [
                    "tifr-gs-2024-classical-mechanics",
                    "tifr-gs-2024-mathematics-relevant-to-physics",
                    "tifr-gs-2024-electricity-and-magnetism",
                    "tifr-gs-2024-quantum-mechanics",
                    "tifr-gs-2024-heat-thermodynamics-and-statistical-physics",
                    "tifr-gs-2024-general-physics",
                    "tifr-gs-2024-modern-physics",
                    "tifr-gs-2024-electronics-and-experimental-physics",
                ] {
                    assert_eq!(before.1.get(unit), titles.get(unit));
                }
                Ok::<(), AppError>(())
            })
            .expect("TIFR preservation");
    }

    #[test]
    fn official_syllabus_seed_is_idempotent_and_progress_is_exam_specific() {
        let (_root, state) = state();
        let before = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM exam_syllabus_nodes", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|error| AppError::database("node count", error))
            })
            .expect("node count");
        state
            .with_connection(prepare)
            .expect("second syllabus prepare");
        let after = state
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM exam_syllabus_nodes", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|error| AppError::database("node count", error))
            })
            .expect("node count");
        assert_eq!(before, after);

        let (csir_topic, gate_topic) = state
            .with_connection(|connection| {
                let csir_topic: i64 = connection
                    .query_row(
                        "SELECT id FROM exam_syllabus_nodes
                         WHERE exam_id = 1 AND title = 'Dimensional analysis'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("CSIR topic", error))?;
                let gate_topic: i64 = connection
                    .query_row(
                        "SELECT id FROM exam_syllabus_nodes
                         WHERE exam_id = 2 AND title LIKE 'Units and dimensions%'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("GATE topic", error))?;
                Ok((csir_topic, gate_topic))
            })
            .expect("topics");
        state
            .with_connection(|connection| {
                update_node_progress(
                    connection,
                    V2SyllabusProgressInput {
                        exam_id: 1,
                        node_id: csir_topic,
                        status: "confident".to_owned(),
                        priority: "high".to_owned(),
                        confidence: 5,
                        notes: "CSIR-only completion".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("CSIR progress");
        let gate = state
            .with_connection(|connection| {
                get_exam_syllabus(connection, V2SyllabusExamInput { exam_id: 2 })
            })
            .expect("GATE syllabus");
        let gate_topic = gate
            .nodes
            .iter()
            .find(|node| node.id == gate_topic)
            .expect("GATE topic");
        assert_eq!(gate_topic.progress_status, "not_started");
        assert_eq!(gate_topic.notes, "");
    }

    #[test]
    fn shared_concepts_can_map_to_topics_in_different_exams() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let concept_id: i64 = connection
                    .query_row("SELECT id FROM concepts WHERE slug='kinematics'", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| AppError::database("concept", error))?;
                for (exam_id, title) in [
                    (1, "Dimensional analysis"),
                    (2, "Units and dimensions%"),
                ] {
                    let node_id: i64 = connection
                        .query_row(
                            "SELECT id FROM exam_syllabus_nodes
                             WHERE exam_id = ?1 AND title LIKE ?2 LIMIT 1",
                            params![exam_id, title],
                            |row| row.get(0),
                        )
                        .map_err(|error| AppError::database("topic", error))?;
                    connection
                        .execute(
                            "INSERT INTO exam_node_concepts (
                                exam_id, node_id, concept_id, weight, is_primary, notes, created_at
                             ) VALUES (?1, ?2, ?3, 1, 1, 'Unverified shared mapping', '2026-11-05T00:00:00Z')",
                            params![exam_id, node_id, concept_id],
                        )
                        .map_err(|error| AppError::database("mapping", error))?;
                }
                Ok(())
            })
            .expect("shared mappings");
        let mapped = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(DISTINCT exam_id) FROM exam_node_concepts WHERE concept_id = (
                            SELECT id FROM concepts WHERE slug='kinematics'
                         )",
                        [],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(|error| AppError::database("mapping count", error))
            })
            .expect("mapping count");
        assert_eq!(mapped, 2);
    }

    #[test]
    fn syllabus_progress_updates_and_rejects_invalid_values() {
        let (_root, state) = state();
        let topic_id = state
            .with_connection(|connection| Ok::<_, AppError>(add_fixture(connection)))
            .expect("fixture");
        let updated = state
            .with_connection(|connection| {
                update_node_progress(
                    connection,
                    V2SyllabusProgressInput {
                        exam_id: 1,
                        node_id: topic_id,
                        status: "revised".to_owned(),
                        priority: "high".to_owned(),
                        confidence: 4,
                        notes: "Reviewed".to_owned(),
                    },
                    at(0),
                )
            })
            .expect("progress");
        let topic = updated
            .nodes
            .iter()
            .find(|node| node.id == topic_id)
            .expect("topic");
        assert_eq!(topic.progress_status, "revised");
        assert_eq!(topic.priority, "high");
        assert_eq!(topic.confidence, 4);

        let invalid = state
            .with_connection(|connection| {
                update_node_progress(
                    connection,
                    V2SyllabusProgressInput {
                        exam_id: 1,
                        node_id: topic_id,
                        status: "unknown".to_owned(),
                        priority: "normal".to_owned(),
                        confidence: 3,
                        notes: String::new(),
                    },
                    at(1),
                )
            })
            .expect_err("invalid status");
        assert_eq!(invalid.field, Some("status"));
    }
}
