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
}

/// Add the verified official syllabus content idempotently. No existing
/// content version, source, or node is replaced by this function.
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
                    format!(
                        "{} GitHub/official syllabus ingestion; document: {}",
                        seed.source.notes, seed.source.document_year
                    ),
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
                    format!(
                        "{} Verified source; document version: {}.",
                        seed.source.notes, seed.source.document_year
                    ),
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
            "SELECT version_label, review_status
             FROM exam_content_versions
             WHERE exam_id = ?1 AND content_type = 'syllabus'
             ORDER BY id DESC LIMIT 1",
            [exam_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Syllabus content version could not be read", error))?;

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
             ORDER BY n.parent_id, n.display_order, n.id",
        )
        .map_err(|error| AppError::database("Syllabus nodes could not be prepared", error))?;
    let rows = statement
        .query_map([exam_id], |row| {
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
        content_version_label: content.as_ref().map(|value| value.0.clone()),
        content_review_status: content.map(|value| value.1),
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

    fn add_fixture(connection: &Connection) -> i64 {
        let now = "2026-11-04T10:00:00Z";
        connection
            .execute(
                "INSERT INTO exam_content_versions (
                    exam_id, content_type, version_label, source_reference,
                    review_status, created_at
                 ) VALUES (1, 'syllabus', 'v1', 'fixture', 'verified', ?1)",
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
