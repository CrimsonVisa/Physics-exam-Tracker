use rusqlite::types::Value;
use rusqlite::{Connection, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const DEFAULT_LIMIT: i64 = 6;
const MAX_LIMIT: i64 = 20;
const MIN_QUERY_CHARS: usize = 2;
const MAX_QUERY_CHARS: usize = 120;
const SNIPPET_CHARS: usize = 160;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalSearchInput {
    pub query: String,
    #[serde(default)]
    pub exam_id: Option<i64>,
    #[serde(default)]
    pub limit_per_category: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalSyllabusHit {
    pub node_id: i64,
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub exam_is_active: bool,
    pub title: String,
    pub node_code: String,
    pub node_kind: String,
    pub breadcrumb: String,
    pub progress_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalConceptHit {
    pub concept_id: i64,
    pub slug: String,
    pub title: String,
    pub domain: String,
    pub mastery_score: Option<f64>,
    pub evidence_count: i64,
    pub exam_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalPyqHit {
    pub question_id: i64,
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub year: i64,
    pub session_label: String,
    pub question_number: String,
    pub section_title: String,
    pub statement_snippet: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalResourceHit {
    pub resource_id: i64,
    pub title: String,
    pub resource_type: String,
    pub subject_area: String,
    pub exam_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalFlashcardHit {
    pub flashcard_id: i64,
    pub front_snippet: String,
    pub back_snippet: String,
    pub subject_area: String,
    pub exam_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2GlobalSearchResult {
    pub query: String,
    pub exam_id: Option<i64>,
    pub syllabus: Vec<V2GlobalSyllabusHit>,
    pub concepts: Vec<V2GlobalConceptHit>,
    pub pyqs: Vec<V2GlobalPyqHit>,
    pub resources: Vec<V2GlobalResourceHit>,
    pub flashcards: Vec<V2GlobalFlashcardHit>,
}

/// Global search across the V2 records that have a workspace destination today:
/// syllabus nodes, shared concepts, PYQs and study resources. Query text is
/// escaped for LIKE; exact titles rank above prefix matches, which rank above
/// substring matches, with deterministic secondary ordering. Archived rows are
/// excluded and an examination scope is applied in SQL when provided.
pub fn global_search(
    connection: &Connection,
    input: V2GlobalSearchInput,
) -> Result<V2GlobalSearchResult, AppError> {
    let query = input.query.trim().to_owned();
    let query_chars = query.chars().count();
    if query_chars > MAX_QUERY_CHARS {
        return Err(AppError::validation(
            "query",
            "Search text must be 120 characters or fewer.",
        ));
    }
    if let Some(exam_id) = input.exam_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("The search scope could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "examId",
                "Choose one of the four supported examinations.",
            ));
        }
    }
    let limit = input
        .limit_per_category
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    if query_chars < MIN_QUERY_CHARS {
        return Ok(V2GlobalSearchResult {
            query,
            exam_id: input.exam_id,
            syllabus: Vec::new(),
            concepts: Vec::new(),
            pyqs: Vec::new(),
            resources: Vec::new(),
            flashcards: Vec::new(),
        });
    }

    let escaped = escape_like(&query);
    let pattern = format!("%{escaped}%");
    let prefix = format!("{escaped}%");

    Ok(V2GlobalSearchResult {
        query: query.clone(),
        exam_id: input.exam_id,
        syllabus: search_syllabus(connection, &query, &pattern, &prefix, input.exam_id, limit)?,
        concepts: search_concepts(connection, &query, &pattern, &prefix, input.exam_id, limit)?,
        pyqs: search_pyqs(connection, &query, &pattern, &prefix, input.exam_id, limit)?,
        resources: search_resources(connection, &query, &pattern, &prefix, input.exam_id, limit)?,
        flashcards: search_flashcards(connection, &query, &pattern, &prefix, input.exam_id, limit)?,
    })
}

fn search_syllabus(
    connection: &Connection,
    query: &str,
    pattern: &str,
    prefix: &str,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2GlobalSyllabusHit>, AppError> {
    let mut sql = String::from(
        "SELECT n.id, n.exam_id, e.code, e.short_name, p.is_active,
                n.title, n.code, n.node_kind,
                COALESCE(grandparent.title, ''), COALESCE(parent.title, ''),
                COALESCE(progress.status, 'not_started')
         FROM exam_syllabus_nodes n
         JOIN exams e ON e.id = n.exam_id
         JOIN exam_profiles p ON p.exam_id = e.id
         LEFT JOIN exam_syllabus_nodes parent
            ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
         LEFT JOIN exam_syllabus_nodes grandparent
            ON grandparent.exam_id = parent.exam_id AND grandparent.id = parent.parent_id
         LEFT JOIN exam_node_progress progress
            ON progress.exam_id = n.exam_id AND progress.node_id = n.id
         WHERE n.is_archived = 0",
    );
    let mut values: Vec<Value> = Vec::new();
    if let Some(exam_id) = exam_id {
        sql.push_str(" AND n.exam_id = ?");
        values.push(Value::Integer(exam_id));
    }
    sql.push_str(
        " AND (n.title LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR n.code LIKE ? ESCAPE '\\' COLLATE NOCASE)
         ORDER BY CASE
                    WHEN lower(trim(n.title)) = lower(?) THEN 0
                    WHEN n.title LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 1
                    ELSE 2
                  END,
                  e.display_order, n.display_order, n.id
         LIMIT ?",
    );
    values.push(Value::Text(pattern.to_owned()));
    values.push(Value::Text(pattern.to_owned()));
    values.push(Value::Text(query.to_owned()));
    values.push(Value::Text(prefix.to_owned()));
    values.push(Value::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Syllabus search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let grandparent: String = row.get(8)?;
            let parent: String = row.get(9)?;
            let breadcrumb = [grandparent, parent]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" › ");
            Ok(V2GlobalSyllabusHit {
                node_id: row.get(0)?,
                exam_id: row.get(1)?,
                exam_code: row.get(2)?,
                exam_short_name: row.get(3)?,
                exam_is_active: row.get::<_, i64>(4)? == 1,
                title: row.get(5)?,
                node_code: row.get(6)?,
                node_kind: row.get(7)?,
                breadcrumb,
                progress_status: row.get(10)?,
            })
        })
        .map_err(|error| AppError::database("Syllabus search could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A syllabus search row is invalid", error))
}

fn search_concepts(
    connection: &Connection,
    query: &str,
    pattern: &str,
    prefix: &str,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2GlobalConceptHit>, AppError> {
    let mut sql = String::from(
        "SELECT c.id, c.slug, c.title, c.domain,
                mastery.mastery_score, COALESCE(mastery.evidence_count, 0),
                COALESCE((
                    SELECT json_group_array(e2.code)
                    FROM exam_node_concepts nc
                    JOIN exams e2 ON e2.id = nc.exam_id
                    WHERE nc.concept_id = c.id
                ), '[]')
         FROM concepts c
         LEFT JOIN concept_mastery mastery ON mastery.concept_id = c.id
         WHERE c.is_archived = 0",
    );
    let mut values: Vec<Value> = Vec::new();
    if let Some(exam_id) = exam_id {
        sql.push_str(
            " AND EXISTS (
                 SELECT 1 FROM exam_node_concepts nc
                 WHERE nc.concept_id = c.id AND nc.exam_id = ?
              )",
        );
        values.push(Value::Integer(exam_id));
    }
    sql.push_str(
        " AND (c.title LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR c.slug LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR c.description LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR c.domain LIKE ? ESCAPE '\\' COLLATE NOCASE)
         ORDER BY CASE
                    WHEN lower(trim(c.title)) = lower(?) THEN 0
                    WHEN c.title LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 1
                    ELSE 2
                  END,
                  c.title COLLATE NOCASE, c.id
         LIMIT ?",
    );
    for _ in 0..4 {
        values.push(Value::Text(pattern.to_owned()));
    }
    values.push(Value::Text(query.to_owned()));
    values.push(Value::Text(prefix.to_owned()));
    values.push(Value::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Concept search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let codes_json: String = row.get(6)?;
            Ok(V2GlobalConceptHit {
                concept_id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                domain: row.get(3)?,
                mastery_score: row.get(4)?,
                evidence_count: row.get(5)?,
                exam_codes: serde_json::from_str(&codes_json).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database("Concept search could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A concept search row is invalid", error))
}

fn search_pyqs(
    connection: &Connection,
    query: &str,
    pattern: &str,
    prefix: &str,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2GlobalPyqHit>, AppError> {
    let mut sql = String::from(
        "SELECT q.id, q.exam_id, e.code, e.short_name, q.year, q.session_label,
                q.question_number, s.title, q.statement
         FROM pyq_questions q
         JOIN exams e ON e.id = q.exam_id
         JOIN exam_sections s ON s.exam_id = q.exam_id AND s.id = q.section_id
         WHERE q.is_archived = 0",
    );
    let mut values: Vec<Value> = Vec::new();
    if let Some(exam_id) = exam_id {
        sql.push_str(" AND q.exam_id = ?");
        values.push(Value::Integer(exam_id));
    }
    sql.push_str(
        " AND (q.statement LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR q.question_number LIKE ? ESCAPE '\\' COLLATE NOCASE)
         ORDER BY CASE
                    WHEN lower(trim(q.statement)) = lower(?) THEN 0
                    WHEN q.statement LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 1
                    ELSE 2
                  END,
                  q.year DESC, q.id
         LIMIT ?",
    );
    values.push(Value::Text(pattern.to_owned()));
    values.push(Value::Text(pattern.to_owned()));
    values.push(Value::Text(query.to_owned()));
    values.push(Value::Text(prefix.to_owned()));
    values.push(Value::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("PYQ search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let statement_text: String = row.get(8)?;
            Ok(V2GlobalPyqHit {
                question_id: row.get(0)?,
                exam_id: row.get(1)?,
                exam_code: row.get(2)?,
                exam_short_name: row.get(3)?,
                year: row.get(4)?,
                session_label: row.get(5)?,
                question_number: row.get(6)?,
                section_title: row.get(7)?,
                statement_snippet: snippet(&statement_text),
            })
        })
        .map_err(|error| AppError::database("PYQ search could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A PYQ search row is invalid", error))
}

fn search_resources(
    connection: &Connection,
    query: &str,
    pattern: &str,
    prefix: &str,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2GlobalResourceHit>, AppError> {
    let mut sql = String::from(
        "SELECT r.id, r.title, r.resource_type, r.subject_area,
                COALESCE((
                    SELECT json_group_array(e2.code)
                    FROM resource_exam_links rel
                    JOIN exams e2 ON e2.id = rel.exam_id
                    WHERE rel.resource_id = r.id
                ), '[]')
         FROM resources r
         WHERE r.is_archived = 0",
    );
    let mut values: Vec<Value> = Vec::new();
    if let Some(exam_id) = exam_id {
        sql.push_str(
            " AND EXISTS (
                 SELECT 1 FROM resource_exam_links rel
                 WHERE rel.resource_id = r.id AND rel.exam_id = ?
              )",
        );
        values.push(Value::Integer(exam_id));
    }
    sql.push_str(
        " AND (r.title LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR r.description LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR r.subject_area LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR r.source LIKE ? ESCAPE '\\' COLLATE NOCASE)
         ORDER BY CASE
                    WHEN lower(trim(r.title)) = lower(?) THEN 0
                    WHEN r.title LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 1
                    ELSE 2
                  END,
                  r.title COLLATE NOCASE, r.id
         LIMIT ?",
    );
    for _ in 0..4 {
        values.push(Value::Text(pattern.to_owned()));
    }
    values.push(Value::Text(query.to_owned()));
    values.push(Value::Text(prefix.to_owned()));
    values.push(Value::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Resource search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let codes_json: String = row.get(4)?;
            Ok(V2GlobalResourceHit {
                resource_id: row.get(0)?,
                title: row.get(1)?,
                resource_type: row.get(2)?,
                subject_area: row.get(3)?,
                exam_codes: serde_json::from_str(&codes_json).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database("Resource search could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A resource search row is invalid", error))
}

fn search_flashcards(
    connection: &Connection,
    query: &str,
    pattern: &str,
    prefix: &str,
    exam_id: Option<i64>,
    limit: i64,
) -> Result<Vec<V2GlobalFlashcardHit>, AppError> {
    let mut sql = String::from(
        "SELECT f.id, f.front, f.back, f.subject_area,
                COALESCE((
                    SELECT json_group_array(e.code)
                    FROM flashcard_exam_links l
                    JOIN exams e ON e.id = l.exam_id
                    WHERE l.flashcard_id = f.id
                ), '[]')
         FROM flashcards f
         WHERE f.is_archived = 0",
    );
    let mut values: Vec<Value> = Vec::new();
    if let Some(exam_id) = exam_id {
        sql.push_str(
            " AND (
                NOT EXISTS (SELECT 1 FROM flashcard_exam_links l WHERE l.flashcard_id=f.id)
                OR EXISTS (
                    SELECT 1 FROM flashcard_exam_links l
                    WHERE l.flashcard_id=f.id AND l.exam_id=?
                )
              )",
        );
        values.push(Value::Integer(exam_id));
    }
    sql.push_str(
        " AND (f.front LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR f.back LIKE ? ESCAPE '\\' COLLATE NOCASE
               OR f.subject_area LIKE ? ESCAPE '\\' COLLATE NOCASE)
         ORDER BY CASE
                    WHEN lower(trim(f.front)) = lower(?) THEN 0
                    WHEN f.front LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 1
                    WHEN f.front LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 2
                    ELSE 3
                  END,
                  f.front COLLATE NOCASE, f.id
         LIMIT ?",
    );
    for _ in 0..3 {
        values.push(Value::Text(pattern.to_owned()));
    }
    values.push(Value::Text(query.to_owned()));
    values.push(Value::Text(prefix.to_owned()));
    values.push(Value::Text(pattern.to_owned()));
    values.push(Value::Integer(limit));

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| AppError::database("Flashcard search could not be prepared", error))?;
    let rows = statement
        .query_map(params_from_iter(values.iter()), |row| {
            let front: String = row.get(1)?;
            let back: String = row.get(2)?;
            let codes_json: String = row.get(4)?;
            Ok(V2GlobalFlashcardHit {
                flashcard_id: row.get(0)?,
                front_snippet: snippet(&front),
                back_snippet: snippet(&back),
                subject_area: row.get(3)?,
                exam_codes: serde_json::from_str(&codes_json).unwrap_or_default(),
            })
        })
        .map_err(|error| AppError::database("Flashcard search could not run", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A flashcard search row is invalid", error))
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

fn snippet(text: &str) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let mut snippet: String = chars.by_ref().take(SNIPPET_CHARS).collect();
    if chars.next().is_some() {
        snippet.push('…');
    }
    snippet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;
    use chrono::Utc;
    use std::time::Instant;

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        (root, state)
    }

    fn insert_syllabus_node(
        connection: &Connection,
        exam_id: i64,
        code: &str,
        title: &str,
        parent_id: Option<i64>,
        display_order: i64,
    ) -> i64 {
        let now = "2026-11-07T00:00:00Z";
        let content_version_id: i64 = connection
            .query_row(
                "SELECT id FROM exam_content_versions WHERE exam_id=?1 ORDER BY id LIMIT 1",
                [exam_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| {
                connection
                    .execute(
                        "INSERT INTO exam_content_versions(exam_id,content_type,version_label,source_reference,verified_on,review_status,created_at)
                         VALUES(?1,'syllabus','test-fixture','fixture','2026-11-01','verified',?2)",
                        rusqlite::params![exam_id, now],
                    )
                    .expect("content version");
                connection.last_insert_rowid()
            });
        let source_id: i64 = connection
            .query_row(
                "SELECT id FROM exam_syllabus_sources WHERE exam_id=?1 ORDER BY id LIMIT 1",
                [exam_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| {
                connection
                    .execute(
                        "INSERT INTO exam_syllabus_sources(exam_id,content_version_id,code,title,publisher,source_reference,verified_on,created_at,updated_at)
                         VALUES(?1,?2,'TEST-SRC','Test source','Publisher','fixture','2026-11-01',?3,?3)",
                        rusqlite::params![exam_id, content_version_id, now],
                    )
                    .expect("source");
                connection.last_insert_rowid()
            });
        connection
            .execute(
                "INSERT INTO exam_syllabus_nodes(exam_id,parent_id,source_id,code,title,node_kind,display_order,created_at,updated_at)
                 VALUES(?1,?2,?3,?4,?5,'topic',?6,?7,?7)",
                rusqlite::params![exam_id, parent_id, source_id, code, title, display_order, now],
            )
            .expect("syllabus node");
        connection.last_insert_rowid()
    }

    fn insert_concept(connection: &Connection, slug: &str, title: &str, domain: &str) -> i64 {
        let now = "2026-11-07T00:00:00Z";
        connection
            .execute(
                "INSERT INTO concepts(slug,title,domain,difficulty,created_at,updated_at)
                 VALUES(?1,?2,?3,3,?4,?4)",
                rusqlite::params![slug, title, domain, now],
            )
            .expect("concept");
        connection.last_insert_rowid()
    }

    fn insert_pyq(
        connection: &Connection,
        exam_id: i64,
        section_id: i64,
        question_type_id: i64,
        number: &str,
        year: i64,
        statement: &str,
    ) -> i64 {
        let now = "2026-11-07T00:00:00Z";
        connection
            .execute(
                "INSERT INTO pyq_questions(
                    exam_id, section_id, question_type_id, year, session_label, question_number,
                    statement, correct_answer, max_marks, negative_marks, unanswered_marks,
                    difficulty, dedupe_key, is_archived, created_at, updated_at
                 ) VALUES(?1,?2,?3,?4,'test',?5,?6,'A',1,0,0,3,?7,0,?8,?8)",
                rusqlite::params![
                    exam_id,
                    section_id,
                    question_type_id,
                    year,
                    number,
                    statement,
                    format!("dedupe-{exam_id}-{number}"),
                    now
                ],
            )
            .expect("pyq");
        connection.last_insert_rowid()
    }

    fn pyq_fixtures(connection: &Connection, exam_id: i64) -> (i64, i64) {
        let now = "2026-11-07T00:00:00Z";
        let content_version_id: i64 = connection
            .query_row(
                "SELECT id FROM exam_content_versions WHERE exam_id=?1 ORDER BY id LIMIT 1",
                [exam_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| {
                connection
                    .execute(
                        "INSERT INTO exam_content_versions(exam_id,content_type,version_label,source_reference,verified_on,review_status,created_at)
                         VALUES(?1,'patterns','test-fixture','fixture','2026-11-01','verified',?2)",
                        rusqlite::params![exam_id, now],
                    )
                    .expect("content version");
                connection.last_insert_rowid()
            });
        connection
            .execute(
                "INSERT INTO exam_question_types(exam_id,code,title,answer_format,supports_partial_marks,is_archived,notes,created_at,updated_at)
                 VALUES(?1,'TEST-MCQ','Test MCQ','mcq',0,0,'',?2,?2)",
                rusqlite::params![exam_id, now],
            )
            .expect("question type");
        let question_type_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO exam_sections(exam_id,content_version_id,code,title,display_order,created_at,updated_at)
                 VALUES(?1,?2,'TEST-SECTION',?3,900,?4,?4)",
                rusqlite::params![exam_id, content_version_id, format!("Test Section {exam_id}"), now],
            )
            .expect("section");
        (connection.last_insert_rowid(), question_type_id)
    }

    fn insert_resource(connection: &Connection, title: &str, resource_type: &str) -> i64 {
        let now = "2026-11-07T00:00:00Z";
        connection
            .execute(
                "INSERT INTO resources(title,description,subject_area,url,resource_type,source,is_archived,created_at,updated_at)
                 VALUES(?1,'','',NULL,?2,'Curated',0,?3,?3)",
                rusqlite::params![title, resource_type, now],
            )
            .expect("resource");
        connection.last_insert_rowid()
    }

    fn search(state: &V2State, query: &str, exam_id: Option<i64>) -> V2GlobalSearchResult {
        state
            .with_connection(|connection| {
                global_search(
                    connection,
                    V2GlobalSearchInput {
                        query: query.to_owned(),
                        exam_id,
                        limit_per_category: None,
                    },
                )
            })
            .expect("global search")
    }

    #[test]
    fn short_queries_return_no_scans_and_long_queries_are_rejected() {
        let (_root, state) = state();
        let short = search(&state, " a ", None);
        assert!(short.syllabus.is_empty());
        assert!(short.concepts.is_empty());
        assert!(short.pyqs.is_empty());
        assert!(short.resources.is_empty());

        let error = state
            .with_connection(|connection| {
                global_search(
                    connection,
                    V2GlobalSearchInput {
                        query: "x".repeat(MAX_QUERY_CHARS + 1),
                        exam_id: None,
                        limit_per_category: None,
                    },
                )
            })
            .expect_err("long query");
        assert_eq!(error.code, "validation_error");
    }

    #[test]
    fn syllabus_exact_prefix_and_substring_ranking_is_deterministic() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                insert_syllabus_node(connection, 1, "RANK-1", "Zqrank", None, 10);
                insert_syllabus_node(connection, 1, "RANK-2", "Zqrank Beta", None, 11);
                insert_syllabus_node(connection, 1, "RANK-3", "Gamma Zqrank", None, 12);
                Ok::<(), AppError>(())
            })
            .expect("ranking fixtures");

        let first = search(&state, "zqrank", None);
        let second = search(&state, "zqrank", None);
        assert_eq!(first.syllabus, second.syllabus);
        let titles = first
            .syllabus
            .iter()
            .map(|hit| hit.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(titles, ["Zqrank", "Zqrank Beta", "Gamma Zqrank"]);
        assert_eq!(first.syllabus[0].exam_code, "csir-net-physics");
    }

    #[test]
    fn syllabus_breadcrumbs_and_archived_exclusion_work() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let unit = insert_syllabus_node(connection, 2, "BREAD-1", "Breadcrumb Unit", None, 20);
                let section =
                    insert_syllabus_node(connection, 2, "BREAD-2", "Breadcrumb Section", Some(unit), 21);
                insert_syllabus_node(
                    connection,
                    2,
                    "BREAD-3",
                    "Breadcrumb Target Topic",
                    Some(section),
                    22,
                );
                connection
                    .execute(
                        "INSERT INTO exam_syllabus_nodes(exam_id,source_id,code,title,node_kind,display_order,is_archived,archived_at,created_at,updated_at)
                         SELECT exam_id, source_id, 'BREAD-ARCH', 'Breadcrumb Archived', node_kind, 23, 1, '2026-11-01T00:00:00Z', created_at, updated_at
                         FROM exam_syllabus_nodes WHERE code='BREAD-1'",
                        [],
                    )
                    .expect("archived node");
                Ok::<(), AppError>(())
            })
            .expect("breadcrumb fixtures");

        let result = search(&state, "Breadcrumb", None);
        assert!(
            result
                .syllabus
                .iter()
                .all(|hit| hit.title != "Breadcrumb Archived")
        );
        let target = result
            .syllabus
            .iter()
            .find(|hit| hit.title == "Breadcrumb Target Topic")
            .expect("target topic");
        assert_eq!(target.breadcrumb, "Breadcrumb Unit › Breadcrumb Section");
        assert_eq!(target.exam_short_name, "GATE PH");
    }

    #[test]
    fn concepts_return_mastery_exam_badges_and_respect_scope() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let concept = insert_concept(connection, "quantum-scope", "Quantum Scope", "Quantum Mechanics");
                connection
                    .execute(
                        "INSERT INTO concept_mastery(concept_id,mastery_score,confidence,evidence_count,last_recalculated_at,updated_at)
                         VALUES(?1,72.5,3,4,'2026-11-01T00:00:00Z','2026-11-01T00:00:00Z')",
                        [concept],
                    )
                    .expect("mastery");
                let node = insert_syllabus_node(connection, 3, "Q-SCOPE-NODE", "Scope Node", None, 40);
                connection
                    .execute(
                        "INSERT INTO exam_node_concepts(exam_id,node_id,concept_id,weight,created_at)
                         VALUES(3,?1,?2,1,'2026-11-01T00:00:00Z')",
                        rusqlite::params![node, concept],
                    )
                    .expect("mapping");
                Ok::<(), AppError>(())
            })
            .expect("concept fixtures");

        let global = search(&state, "quantum-scope", None);
        let hit = global
            .concepts
            .iter()
            .find(|concept| concept.slug == "quantum-scope")
            .expect("concept hit");
        assert_eq!(hit.mastery_score, Some(72.5));
        assert_eq!(hit.evidence_count, 4);
        assert!(hit.exam_codes.contains(&"tifr-gs-physics".to_owned()));

        let scoped = search(&state, "quantum-scope", Some(2));
        assert!(scoped.concepts.is_empty());
        let tifr = search(&state, "quantum-scope", Some(3));
        assert!(
            tifr.concepts
                .iter()
                .any(|concept| concept.slug == "quantum-scope")
        );
    }

    #[test]
    fn pyqs_and_resources_are_searchable_and_scopeable() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let (section, question_type) = pyq_fixtures(connection, 1);
                let (gate_section, gate_type) = pyq_fixtures(connection, 2);
                insert_pyq(
                    connection,
                    1,
                    section,
                    question_type,
                    "Q1",
                    2024,
                    "Compute the logarithmic decrement for the damped oscillator.",
                );
                insert_pyq(
                    connection,
                    2,
                    gate_section,
                    gate_type,
                    "Q2",
                    2023,
                    "A waveguide problem involving the logarithmic decrement.",
                );
                let resource = insert_resource(connection, "Logarithmic decrement formula sheet", "notes");
                connection
                    .execute(
                        "INSERT INTO resource_exam_links(resource_id,exam_id,category,priority,notes,created_at,updated_at)
                         VALUES(?1,2,'reference','normal','','2026-11-01T00:00:00Z','2026-11-01T00:00:00Z')",
                        [resource],
                    )
                    .expect("resource link");
                Ok::<(), AppError>(())
            })
            .expect("practice fixtures");

        let global = search(&state, "logarithmic", None);
        assert_eq!(global.pyqs.len(), 2);
        assert_eq!(global.pyqs[0].year, 2024);
        let resource_hit = global
            .resources
            .iter()
            .find(|resource| resource.title.contains("Logarithmic"))
            .expect("resource hit");
        assert!(resource_hit.exam_codes.contains(&"gate-physics".to_owned()));

        let scoped = search(&state, "logarithmic", Some(2));
        assert_eq!(scoped.pyqs.len(), 1);
        assert_eq!(scoped.pyqs[0].exam_code, "gate-physics");
        assert_eq!(scoped.resources.len(), 1);
    }

    #[test]
    fn like_wildcards_are_escaped_and_limits_apply() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                insert_concept(connection, "percent-focus", "Percent % Focus", "Methods");
                insert_concept(
                    connection,
                    "underscore-focus",
                    "Underscore _ Focus",
                    "Methods",
                );
                for index in 0..12 {
                    insert_resource(connection, &format!("Limit resource {index}"), "notes");
                }
                Ok::<(), AppError>(())
            })
            .expect("escape fixtures");

        let percent = search(&state, "% foc", None);
        assert!(
            percent
                .concepts
                .iter()
                .any(|hit| hit.title == "Percent % Focus")
        );
        assert!(
            !percent
                .concepts
                .iter()
                .any(|hit| hit.title == "Underscore _ Focus")
        );

        let limited = state
            .with_connection(|connection| {
                global_search(
                    connection,
                    V2GlobalSearchInput {
                        query: "Limit resource".to_owned(),
                        exam_id: None,
                        limit_per_category: Some(3),
                    },
                )
            })
            .expect("limited search");
        assert_eq!(limited.resources.len(), 3);
    }

    #[test]
    fn concept_and_resource_ranking_is_exact_then_prefix_then_substring() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                insert_concept(connection, "orbit-exact", "Orbit", "Mechanics");
                insert_concept(connection, "orbit-prefix", "Orbit dynamics", "Mechanics");
                insert_concept(
                    connection,
                    "orbit-substring",
                    "Stable orbit theory",
                    "Mechanics",
                );
                insert_resource(connection, "Orbit", "notes");
                insert_resource(connection, "Orbit dynamics notes", "notes");
                insert_resource(connection, "Notes on stable orbit theory", "notes");
                Ok::<(), AppError>(())
            })
            .expect("concept and resource ranking fixtures");

        let result = search(&state, "orbit", None);
        let concept_titles = result
            .concepts
            .iter()
            .filter(|hit| hit.slug.starts_with("orbit-"))
            .map(|hit| hit.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            concept_titles,
            ["Orbit", "Orbit dynamics", "Stable orbit theory"]
        );
        let expected_resources = [
            "Orbit",
            "Orbit dynamics notes",
            "Notes on stable orbit theory",
        ];
        let resource_titles = result
            .resources
            .iter()
            .map(|hit| hit.title.as_str())
            .filter(|title| expected_resources.contains(title))
            .collect::<Vec<_>>();
        assert_eq!(resource_titles, expected_resources);
    }

    #[test]
    fn pyq_exact_prefix_and_substring_ranking_is_deterministic() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let (section, question_type) = pyq_fixtures(connection, 1);
                insert_pyq(
                    connection,
                    1,
                    section,
                    question_type,
                    "R1",
                    2024,
                    "zeta eta theta",
                );
                insert_pyq(
                    connection,
                    1,
                    section,
                    question_type,
                    "R2",
                    2025,
                    "zeta eta theta extended",
                );
                insert_pyq(
                    connection,
                    1,
                    section,
                    question_type,
                    "R3",
                    2026,
                    "prefix zeta eta theta",
                );
                Ok::<(), AppError>(())
            })
            .expect("pyq ranking fixtures");

        let first = search(&state, "zeta eta theta", None);
        let second = search(&state, "zeta eta theta", None);
        assert_eq!(first.pyqs, second.pyqs);
        let numbers = first
            .pyqs
            .iter()
            .map(|hit| hit.question_number.as_str())
            .collect::<Vec<_>>();
        assert_eq!(numbers, ["R1", "R2", "R3"]);
    }

    #[test]
    fn flashcard_search_ranks_shared_cards_and_honours_scope() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                for (front, exam_id) in [
                    ("Vector field", None),
                    ("Vector field divergence", Some(1_i64)),
                    ("A vector field note", None),
                ] {
                    crate::v2::flashcards::save_flashcard(
                        connection,
                        crate::v2::flashcards::V2FlashcardInput {
                            id: None,
                            front: front.to_owned(),
                            back: format!("{front} back"),
                            subject_area: "Mathematical Methods".to_owned(),
                            concept_id: None,
                            exam_links: exam_id
                                .map(|exam_id| {
                                    vec![crate::v2::flashcards::V2FlashcardExamLinkInput {
                                        exam_id,
                                        syllabus_node_id: None,
                                    }]
                                })
                                .unwrap_or_default(),
                            today: "2026-11-08".to_owned(),
                        },
                        Utc::now(),
                    )
                    .expect("flashcard fixture");
                }
                Ok::<(), AppError>(())
            })
            .expect("flashcard search fixtures");

        let global = search(&state, "vector field", None);
        let fronts = global
            .flashcards
            .iter()
            .map(|hit| hit.front_snippet.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            fronts,
            [
                "Vector field",
                "Vector field divergence",
                "A vector field note"
            ]
        );

        let gate = search(&state, "vector field", Some(2));
        assert_eq!(gate.flashcards.len(), 2);
        assert!(
            gate.flashcards
                .iter()
                .all(|hit| hit.front_snippet != "Vector field divergence")
        );
    }

    #[test]
    fn global_search_stays_responsive_on_larger_datasets() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let transaction = connection.transaction().expect("transaction");
                let now = "2026-11-07T00:00:00Z";
                let content_version_id: i64 = transaction
                    .query_row(
                        "SELECT id FROM exam_content_versions WHERE exam_id=1 ORDER BY id LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .expect("content version");
                let source_id: i64 = transaction
                    .query_row(
                        "SELECT id FROM exam_syllabus_sources WHERE exam_id=1 ORDER BY id LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .expect("source");
                for index in 0..2_000 {
                    transaction
                        .execute(
                            "INSERT INTO exam_syllabus_nodes(exam_id,source_id,code,title,node_kind,display_order,created_at,updated_at)
                             VALUES(1,?1,?2,?3,'topic',?4,?5,?5)",
                            rusqlite::params![
                                source_id,
                                format!("PERF-{index}"),
                                format!("Performance Topic {index}"),
                                1_000 + index,
                                now
                            ],
                        )
                        .expect("perf node");
                }
                for index in 0..2_000 {
                    transaction
                        .execute(
                            "INSERT INTO concepts(slug,title,domain,difficulty,created_at,updated_at)
                             VALUES(?1,?2,'Physics',3,?3,?3)",
                            rusqlite::params![
                                format!("perf-concept-{index}"),
                                format!("Performance Concept {index}"),
                                now
                            ],
                        )
                        .expect("perf concept");
                }
                for index in 0..1_000 {
                    transaction
                        .execute(
                            "INSERT INTO resources(title,description,subject_area,url,resource_type,source,is_archived,created_at,updated_at)
                             VALUES(?1,'','',NULL,'notes','Curated',0,?2,?2)",
                            rusqlite::params![format!("Performance Resource {index}"), now],
                        )
                        .expect("perf resource");
                }
                let question_type: i64 = {
                    transaction
                        .execute(
                            "INSERT INTO exam_question_types(exam_id,code,title,answer_format,supports_partial_marks,is_archived,notes,created_at,updated_at)
                             VALUES(1,'PERF-MCQ','Perf MCQ','mcq',0,0,'',?1,?1)",
                            [now],
                        )
                        .expect("perf type");
                    transaction.last_insert_rowid()
                };
                transaction
                    .execute(
                        "INSERT INTO exam_sections(exam_id,content_version_id,code,title,display_order,created_at,updated_at)
                         VALUES(1,?1,'PERF-SECTION','Perf Section',900,?2,?2)",
                        rusqlite::params![content_version_id, now],
                    )
                    .expect("perf section");
                let section_id = transaction.last_insert_rowid();
                for index in 0..1_000 {
                    transaction
                        .execute(
                            "INSERT INTO pyq_questions(
                                exam_id, section_id, question_type_id, year, session_label, question_number,
                                statement, correct_answer, max_marks, negative_marks, unanswered_marks,
                                difficulty, dedupe_key, is_archived, created_at, updated_at
                             ) VALUES(1,?1,?2,?3,'perf',?4,?5,'A',1,0,0,3,?6,0,?7,?7)",
                            rusqlite::params![
                                section_id,
                                question_type,
                                2000 + (index % 24),
                                format!("PQ{index}"),
                                format!("Performance PYQ statement {index}"),
                                format!("perf-dedupe-{index}"),
                                now
                            ],
                        )
                        .expect("perf pyq");
                }
                transaction.commit().expect("commit perf fixtures");
                Ok::<(), AppError>(())
            })
            .expect("performance fixtures");

        let started = Instant::now();
        let result = search(&state, "performance", None);
        let elapsed = started.elapsed();
        println!("global search on larger dataset completed in {elapsed:?}");
        assert_eq!(result.syllabus.len(), DEFAULT_LIMIT as usize);
        assert_eq!(result.concepts.len(), DEFAULT_LIMIT as usize);
        assert_eq!(result.pyqs.len(), DEFAULT_LIMIT as usize);
        assert_eq!(result.resources.len(), DEFAULT_LIMIT as usize);
        assert!(
            elapsed.as_millis() < 2_000,
            "global search took {elapsed:?} on a larger dataset"
        );
    }
}
