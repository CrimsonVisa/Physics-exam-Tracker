use std::fs;
use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceProgress {
    pub study_status: String,
    pub personal_notes: String,
    pub last_opened_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceExamLink {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_short_name: String,
    pub category: String,
    pub syllabus_node_id: Option<i64>,
    pub priority: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceFile {
    pub file_name: String,
    pub file_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub storage_mode: String,
    pub status: String,
    pub imported_at: String,
    pub last_verified_at: Option<String>,
    pub missing_since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2Resource {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub subject_area: String,
    pub url: Option<String>,
    pub resource_type: String,
    pub source: String,
    pub is_archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub progress: V2ResourceProgress,
    pub concept_ids: Vec<i64>,
    pub exam_links: Vec<V2ResourceExamLink>,
    pub file: Option<V2ResourceFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceExamLinkInput {
    pub exam_id: i64,
    pub category: String,
    #[serde(default)]
    pub syllabus_node_id: Option<i64>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub subject_area: String,
    #[serde(default)]
    pub url: Option<String>,
    pub resource_type: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub concept_ids: Vec<i64>,
    #[serde(default)]
    pub exam_links: Vec<V2ResourceExamLinkInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceSearchFilter {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub exam_code: Option<String>,
    #[serde(default)]
    pub include_archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceProgressInput {
    pub resource_id: i64,
    pub study_status: String,
    #[serde(default)]
    pub personal_notes: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceExamLinkCommandInput {
    pub resource_id: i64,
    pub link: V2ResourceExamLinkInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceFileInput {
    pub resource_id: i64,
    pub file_path: String,
    #[serde(default = "default_storage_mode")]
    pub storage_mode: String,
}

fn default_storage_mode() -> String {
    "managed".to_owned()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2ResourceArchiveInput {
    pub resource_id: i64,
    pub archived: bool,
}

fn default_priority() -> String {
    "normal".to_owned()
}

pub fn list_resources(
    connection: &Connection,
    filter: V2ResourceSearchFilter,
) -> Result<Vec<V2Resource>, AppError> {
    let mut where_parts = Vec::new();
    let mut values: Vec<String> = Vec::new();
    if !filter.include_archived.unwrap_or(false) {
        where_parts.push("r.is_archived = 0".to_owned());
    }
    if let Some(query) = filter
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        where_parts.push(
            "(r.title LIKE ? OR r.description LIKE ? OR r.subject_area LIKE ? OR r.source LIKE ?)"
                .to_owned(),
        );
        let pattern = format!("%{query}%");
        for _ in 0..4 {
            values.push(pattern.clone());
        }
    }
    if let Some(exam_code) = filter
        .exam_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        where_parts.push(
            "EXISTS (
                SELECT 1 FROM resource_exam_links rel
                JOIN exams e ON e.id = rel.exam_id
                WHERE rel.resource_id = r.id AND e.code = ?
            )"
            .to_owned(),
        );
        values.push(exam_code.to_owned());
    }
    let where_sql = if where_parts.is_empty() {
        "1 = 1".to_owned()
    } else {
        where_parts.join(" AND ")
    };
    let mut statement = connection
        .prepare(&format!(
            "SELECT r.id, r.title, r.description, r.subject_area, r.url, r.resource_type,
                    r.source, r.is_archived, r.created_at, r.updated_at
             FROM resources r
             WHERE {where_sql}
             ORDER BY r.is_archived, r.title COLLATE NOCASE, r.id"
        ))
        .map_err(|error| AppError::database("Resources could not be prepared", error))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)? == 1,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .map_err(|error| AppError::database("Resources could not be read", error))?;
    let mut resources = Vec::new();
    for row in rows {
        let (
            id,
            title,
            description,
            subject_area,
            url,
            resource_type,
            source,
            is_archived,
            created_at,
            updated_at,
        ) = row.map_err(|error| AppError::database("A resource row is invalid", error))?;
        resources.push(V2Resource {
            id,
            title,
            description,
            subject_area,
            url,
            resource_type,
            source,
            is_archived,
            created_at,
            updated_at,
            progress: load_progress(connection, id)?,
            concept_ids: load_concept_ids(connection, id)?,
            exam_links: load_exam_links(connection, id)?,
            file: load_file(connection, id)?,
        });
    }
    Ok(resources)
}

pub fn save_resource(
    connection: &mut Connection,
    input: V2ResourceInput,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    let title = input.title.trim();
    if title.is_empty() || title.chars().count() > 300 {
        return Err(AppError::validation(
            "title",
            "Resource title must be between 1 and 300 characters.",
        ));
    }
    if input.description.chars().count() > 4_000 {
        return Err(AppError::validation(
            "description",
            "Resource description must be 4,000 characters or fewer.",
        ));
    }
    if input.subject_area.chars().count() > 200 {
        return Err(AppError::validation(
            "subjectArea",
            "Subject area must be 200 characters or fewer.",
        ));
    }
    if input.source.chars().count() > 500 {
        return Err(AppError::validation(
            "source",
            "Resource source must be 500 characters or fewer.",
        ));
    }
    if !matches!(
        input.resource_type.as_str(),
        "book" | "notes" | "video" | "paper" | "website" | "other"
    ) {
        return Err(AppError::validation(
            "resourceType",
            "Choose book, notes, video, paper, website, or other.",
        ));
    }
    let url = input
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(url) = url
        && !(url.starts_with("https://") || url.starts_with("http://"))
    {
        return Err(AppError::validation(
            "url",
            "Resource URL must start with http:// or https://.",
        ));
    }
    for concept_id in &input.concept_ids {
        validate_concept(connection, *concept_id)?;
    }
    for link in &input.exam_links {
        validate_exam_link(connection, link)?;
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The resource save transaction could not start", error)
    })?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let resource_id = if let Some(resource_id) = input.id {
        let changed = transaction
            .execute(
                "UPDATE resources
                 SET title = ?1, description = ?2, subject_area = ?3, url = ?4,
                     resource_type = ?5, source = ?6, updated_at = ?7
                 WHERE id = ?8",
                params![
                    title,
                    input.description.trim(),
                    input.subject_area.trim(),
                    url,
                    input.resource_type,
                    input.source.trim(),
                    now_text,
                    resource_id,
                ],
            )
            .map_err(|error| AppError::database("Resource could not be updated", error))?;
        if changed != 1 {
            return Err(AppError::resource_not_found());
        }
        resource_id
    } else {
        transaction
            .execute(
                "INSERT INTO resources (
                    title, description, subject_area, url, resource_type, source,
                    is_archived, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
                params![
                    title,
                    input.description.trim(),
                    input.subject_area.trim(),
                    url,
                    input.resource_type,
                    input.source.trim(),
                    now_text,
                ],
            )
            .map_err(|error| {
                if matches!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::ConstraintViolation)
                ) {
                    AppError::validation("url", "A resource with this URL already exists.")
                } else {
                    AppError::database("Resource could not be saved", error)
                }
            })?;
        transaction.last_insert_rowid()
    };

    transaction
        .execute(
            "DELETE FROM resource_concepts WHERE resource_id = ?1",
            [resource_id],
        )
        .map_err(|error| AppError::database("Resource concepts could not be replaced", error))?;
    for concept_id in &input.concept_ids {
        transaction
            .execute(
                "INSERT INTO resource_concepts (resource_id, concept_id, weight, created_at)
                 VALUES (?1, ?2, 1, ?3)",
                params![resource_id, concept_id, now_text],
            )
            .map_err(|error| AppError::database("Resource concept could not be saved", error))?;
    }
    transaction
        .execute(
            "DELETE FROM resource_exam_links WHERE resource_id = ?1",
            [resource_id],
        )
        .map_err(|error| AppError::database("Resource exam links could not be replaced", error))?;
    for link in &input.exam_links {
        transaction
            .execute(
                "INSERT INTO resource_exam_links (
                    resource_id, exam_id, category, syllabus_node_id, priority, notes,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    resource_id,
                    link.exam_id,
                    link.category.trim(),
                    link.syllabus_node_id,
                    link.priority,
                    link.notes.trim(),
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Resource exam link could not be saved", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Resource save could not be committed", error))?;
    get_resource(connection, resource_id)
}

pub fn archive_resource(
    connection: &mut Connection,
    input: V2ResourceArchiveInput,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    let changed = connection
        .execute(
            "UPDATE resources SET is_archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                i64::from(input.archived),
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
                input.resource_id
            ],
        )
        .map_err(|error| AppError::database("Resource archive state could not be saved", error))?;
    if changed != 1 {
        return Err(AppError::resource_not_found());
    }
    get_resource(connection, input.resource_id)
}

pub fn update_resource_progress(
    connection: &mut Connection,
    input: V2ResourceProgressInput,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    if !matches!(
        input.study_status.as_str(),
        "not_started" | "studying" | "completed" | "skipped"
    ) {
        return Err(AppError::validation(
            "studyStatus",
            "Choose not started, studying, completed, or skipped.",
        ));
    }
    if input.personal_notes.chars().count() > 4_000 {
        return Err(AppError::validation(
            "personalNotes",
            "Personal notes must be 4,000 characters or fewer.",
        ));
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let changed = connection
        .execute(
            "INSERT INTO resource_progress (
                resource_id, study_status, personal_notes, last_opened_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(resource_id) DO UPDATE SET
                study_status = excluded.study_status,
                personal_notes = excluded.personal_notes,
                last_opened_at = excluded.last_opened_at,
                updated_at = excluded.updated_at",
            params![
                input.resource_id,
                input.study_status,
                input.personal_notes.trim(),
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Resource progress could not be saved", error))?;
    if changed == 0 {
        return Err(AppError::resource_not_found());
    }
    get_resource(connection, input.resource_id)
}

pub fn link_resource_exam(
    connection: &mut Connection,
    input: V2ResourceExamLinkCommandInput,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    validate_exam_link(connection, &input.link)?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    connection
        .execute(
            "INSERT INTO resource_exam_links (
                resource_id, exam_id, category, syllabus_node_id, priority, notes,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(resource_id, exam_id) DO UPDATE SET
                category = excluded.category,
                syllabus_node_id = excluded.syllabus_node_id,
                priority = excluded.priority,
                notes = excluded.notes,
                updated_at = excluded.updated_at",
            params![
                input.resource_id,
                input.link.exam_id,
                input.link.category.trim(),
                input.link.syllabus_node_id,
                input.link.priority,
                input.link.notes.trim(),
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Resource exam link could not be saved", error))?;
    get_resource(connection, input.resource_id)
}

pub fn import_resource_file(
    connection: &mut Connection,
    data_dir: &Path,
    input: V2ResourceFileInput,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    if !matches!(input.storage_mode.as_str(), "managed" | "external") {
        return Err(AppError::validation(
            "storageMode",
            "Choose managed or external storage.",
        ));
    }
    let path = Path::new(input.file_path.trim());
    if !path.is_file() {
        return Err(AppError::validation(
            "filePath",
            "Choose an existing PDF file.",
        ));
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
    {
        return Err(AppError::validation(
            "filePath",
            "Only PDF files are supported.",
        ));
    }
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    if metadata.len() == 0 || metadata.len() > 512 * 1024 * 1024 {
        return Err(AppError::validation(
            "filePath",
            "PDF size must be between 1 byte and 512 MB.",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::validation("filePath", "The PDF file name is invalid."))?
        .to_owned();
    let (file_path, normalized, size_bytes) = if input.storage_mode == "managed" {
        let directory = data_dir
            .join("resources")
            .join(input.resource_id.to_string());
        fs::create_dir_all(&directory).map_err(|error| {
            AppError::v2_file_operation("The managed resource folder could not be created", error)
        })?;
        let destination = directory.join(format!("{sha256}-{file_name}"));
        if !destination.exists() {
            fs::copy(path, &destination).map_err(|error| {
                AppError::v2_file_operation("The managed PDF copy could not be created", error)
            })?;
        }
        let copied = fs::read(&destination).map_err(|error| {
            AppError::file_operation("The managed PDF could not be verified", error)
        })?;
        let mut copied_hasher = Sha256::new();
        copied_hasher.update(&copied);
        if format!("{:x}", copied_hasher.finalize()) != sha256
            || copied.len() as u64 != metadata.len()
        {
            return Err(AppError::file_operation(
                "The managed PDF copy did not match the source file",
                "checksum mismatch",
            ));
        }
        (
            destination.display().to_string(),
            destination.to_string_lossy().to_lowercase(),
            copied.len() as i64,
        )
    } else {
        let canonical = path.canonicalize().map_err(|error| {
            AppError::file_operation("The PDF path could not be resolved", error)
        })?;
        let file_path = canonical.display().to_string();
        let normalized = file_path.to_lowercase();
        (file_path, normalized, metadata.len() as i64)
    };
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);

    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The resource file transaction could not start", error)
    })?;
    let exists: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resources WHERE id = ?1)",
            [input.resource_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Resource could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::resource_not_found());
    }
    let previous: Option<(String, String, String, i64, String)> = transaction
        .query_row(
            "SELECT file_name, file_path, sha256, size_bytes, storage_mode
             FROM resource_files WHERE resource_id = ?1",
            [input.resource_id],
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
        .optional()
        .map_err(|error| AppError::database("Existing resource file could not be read", error))?;
    transaction
        .execute(
            "INSERT INTO resource_files (
                resource_id, file_name, file_path, file_path_normalized, sha256, size_bytes,
                storage_mode, status, imported_at, last_verified_at, missing_since, notes,
                updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'ok', ?8, ?8, NULL, '', ?8)
             ON CONFLICT(resource_id) DO UPDATE SET
                file_name = excluded.file_name,
                file_path = excluded.file_path,
                file_path_normalized = excluded.file_path_normalized,
                sha256 = excluded.sha256,
                size_bytes = excluded.size_bytes,
                storage_mode = excluded.storage_mode,
                status = 'ok',
                imported_at = excluded.imported_at,
                last_verified_at = excluded.last_verified_at,
                missing_since = NULL,
                updated_at = excluded.updated_at",
            params![
                input.resource_id,
                file_name,
                file_path,
                normalized,
                sha256,
                size_bytes,
                input.storage_mode,
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Resource PDF metadata could not be saved", error))?;
    let action = match &previous {
        None => "import",
        Some((_, old_path, _, _, _)) if !Path::new(old_path).is_file() => "relink",
        Some(_) => "replace",
    };
    transaction
        .execute(
            "INSERT INTO resource_file_history (
                resource_id, file_name, file_path, sha256, size_bytes, storage_mode,
                action, occurred_at, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '')",
            params![
                input.resource_id,
                file_name,
                file_path,
                sha256,
                size_bytes,
                input.storage_mode,
                action,
                now_text,
            ],
        )
        .map_err(|error| AppError::database("Resource file history could not be saved", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("Resource PDF metadata could not be committed", error)
    })?;
    if let Some((_, old_path, _, _, old_mode)) = previous
        && old_mode == "managed"
        && old_path != file_path
        && let Some(old_file) = managed_file_for_deletion(data_dir, &old_path)
    {
        let _ = fs::remove_file(old_file);
    }
    get_resource(connection, input.resource_id)
}

pub fn remove_resource_file(
    connection: &mut Connection,
    data_dir: &Path,
    resource_id: i64,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The resource file removal transaction could not start",
            error,
        )
    })?;
    let file: Option<(String, String, String, i64, String)> = transaction
        .query_row(
            "SELECT file_name, file_path, sha256, size_bytes, storage_mode
             FROM resource_files WHERE resource_id = ?1",
            [resource_id],
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
        .optional()
        .map_err(|error| AppError::database("Resource file could not be read", error))?;
    let Some((file_name, file_path, sha256, size_bytes, storage_mode)) = file else {
        return Err(AppError::validation(
            "resourceId",
            "This resource does not have a linked PDF.",
        ));
    };
    transaction
        .execute(
            "DELETE FROM resource_files WHERE resource_id = ?1",
            [resource_id],
        )
        .map_err(|error| {
            AppError::database("Resource file metadata could not be removed", error)
        })?;
    transaction
        .execute(
            "INSERT INTO resource_file_history (
                resource_id, file_name, file_path, sha256, size_bytes, storage_mode,
                action, occurred_at, notes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'remove', ?7, 'Metadata removed; original file untouched.')",
            params![
                resource_id,
                file_name,
                file_path,
                sha256,
                size_bytes,
                storage_mode,
                now.to_rfc3339_opts(SecondsFormat::Millis, true),
            ],
        )
        .map_err(|error| AppError::database("Resource file history could not be saved", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("Resource file removal could not be committed", error)
    })?;
    if storage_mode == "managed"
        && let Some(managed_file) = managed_file_for_deletion(data_dir, &file_path)
    {
        let _ = fs::remove_file(managed_file);
    }
    get_resource(connection, resource_id)
}

/// Resolve a managed PDF path and confirm that it really lies inside the
/// application's managed resources folder. Canonicalization prevents lexical
/// traversal such as `resources/../outside.pdf` from deleting external files.
fn managed_file_for_deletion(data_dir: &Path, file_path: &str) -> Option<std::path::PathBuf> {
    let managed_root = data_dir.join("resources").canonicalize().ok()?;
    let candidate = Path::new(file_path).canonicalize().ok()?;
    if candidate.is_file() && candidate.starts_with(&managed_root) {
        Some(candidate)
    } else {
        None
    }
}

fn get_resource(connection: &Connection, resource_id: i64) -> Result<V2Resource, AppError> {
    list_resources(
        connection,
        V2ResourceSearchFilter {
            query: None,
            exam_code: None,
            include_archived: Some(true),
        },
    )?
    .into_iter()
    .find(|resource| resource.id == resource_id)
    .ok_or_else(AppError::resource_not_found)
}

fn load_progress(
    connection: &Connection,
    resource_id: i64,
) -> Result<V2ResourceProgress, AppError> {
    connection
        .query_row(
            "SELECT study_status, personal_notes, last_opened_at, updated_at
             FROM resource_progress WHERE resource_id = ?1",
            [resource_id],
            |row| {
                Ok(V2ResourceProgress {
                    study_status: row.get(0)?,
                    personal_notes: row.get(1)?,
                    last_opened_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Resource progress could not be read", error))?
        .map_or_else(
            || {
                Ok(V2ResourceProgress {
                    study_status: "not_started".to_owned(),
                    personal_notes: String::new(),
                    last_opened_at: None,
                    updated_at: String::new(),
                })
            },
            Ok,
        )
}

fn load_concept_ids(connection: &Connection, resource_id: i64) -> Result<Vec<i64>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT concept_id FROM resource_concepts WHERE resource_id = ?1 ORDER BY concept_id",
        )
        .map_err(|error| AppError::database("Resource concepts could not be prepared", error))?;
    let rows = statement
        .query_map([resource_id], |row| row.get::<_, i64>(0))
        .map_err(|error| AppError::database("Resource concepts could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A resource concept row is invalid", error))
}

fn load_exam_links(
    connection: &Connection,
    resource_id: i64,
) -> Result<Vec<V2ResourceExamLink>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT rel.exam_id, e.code, e.short_name, rel.category, rel.syllabus_node_id,
                    rel.priority, rel.notes
             FROM resource_exam_links rel
             JOIN exams e ON e.id = rel.exam_id
             WHERE rel.resource_id = ?1
             ORDER BY e.display_order",
        )
        .map_err(|error| AppError::database("Resource exam links could not be prepared", error))?;
    let rows = statement
        .query_map([resource_id], |row| {
            Ok(V2ResourceExamLink {
                exam_id: row.get(0)?,
                exam_code: row.get(1)?,
                exam_short_name: row.get(2)?,
                category: row.get(3)?,
                syllabus_node_id: row.get(4)?,
                priority: row.get(5)?,
                notes: row.get(6)?,
            })
        })
        .map_err(|error| AppError::database("Resource exam links could not be read", error))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A resource exam link row is invalid", error))
}

fn load_file(
    connection: &Connection,
    resource_id: i64,
) -> Result<Option<V2ResourceFile>, AppError> {
    let file = connection
        .query_row(
            "SELECT file_name, file_path, sha256, size_bytes, storage_mode, status,
                    imported_at, last_verified_at, missing_since
             FROM resource_files WHERE resource_id = ?1",
            [resource_id],
            |row| {
                Ok(V2ResourceFile {
                    file_name: row.get(0)?,
                    file_path: row.get(1)?,
                    sha256: row.get(2)?,
                    size_bytes: row.get(3)?,
                    storage_mode: row.get(4)?,
                    status: row.get(5)?,
                    imported_at: row.get(6)?,
                    last_verified_at: row.get(7)?,
                    missing_since: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::database("Resource file could not be read", error))?;
    Ok(file.map(|mut file| {
        match fs::metadata(&file.file_path) {
            Ok(metadata) if metadata.len() as i64 == file.size_bytes => {
                file.status = "ok".to_owned();
            }
            Ok(_) => {
                file.status = "changed".to_owned();
            }
            Err(_) => {
                file.status = "missing".to_owned();
            }
        }
        file
    }))
}

fn validate_concept(connection: &Connection, concept_id: i64) -> Result<(), AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM concepts WHERE id = ?1 AND is_archived = 0)",
            [concept_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Concept could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::validation(
            "conceptIds",
            "Choose active concepts from the shared library.",
        ));
    }
    Ok(())
}

fn validate_exam_link(
    connection: &Connection,
    link: &V2ResourceExamLinkInput,
) -> Result<(), AppError> {
    if link.category.trim().is_empty() || link.category.chars().count() > 80 {
        return Err(AppError::validation(
            "category",
            "Resource exam category must be between 1 and 80 characters.",
        ));
    }
    if link.notes.chars().count() > 2_000 {
        return Err(AppError::validation(
            "notes",
            "Resource exam notes must be 2,000 characters or fewer.",
        ));
    }
    if !matches!(
        link.priority.as_str(),
        "low" | "normal" | "high" | "high_yield"
    ) {
        return Err(AppError::validation(
            "priority",
            "Choose low, normal, high, or high-yield priority.",
        ));
    }
    let exam_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
            [link.exam_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Exam could not be checked", error))?;
    if exam_exists == 0 {
        return Err(AppError::validation(
            "examId",
            "The selected examination does not exist.",
        ));
    }
    if let Some(node_id) = link.syllabus_node_id {
        let node_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM exam_syllabus_nodes
                    WHERE exam_id = ?1 AND id = ?2 AND is_archived = 0
                 )",
                params![link.exam_id, node_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Syllabus node could not be checked", error))?;
        if node_exists == 0 {
            return Err(AppError::validation(
                "syllabusNodeId",
                "Choose an active syllabus node from the selected examination.",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2State, backup::create_manual_backup};

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
    fn resource_metadata_progress_links_and_pdf_are_persistent() {
        let (_root, state) = state();
        let concept = concept_id(&state);
        let resource = state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: "Mechanics notes".to_owned(),
                        description: "Curated notes".to_owned(),
                        subject_area: "Mechanics".to_owned(),
                        url: Some("https://example.com/mechanics".to_owned()),
                        resource_type: "notes".to_owned(),
                        source: "Curated".to_owned(),
                        concept_ids: vec![concept],
                        exam_links: vec![V2ResourceExamLinkInput {
                            exam_id: 1,
                            category: "core".to_owned(),
                            syllabus_node_id: None,
                            priority: "high".to_owned(),
                            notes: String::new(),
                        }],
                    },
                    at(0),
                )
            })
            .expect("resource");
        assert_eq!(resource.concept_ids, vec![concept]);
        assert_eq!(resource.exam_links.len(), 1);

        let progress = state
            .with_connection(|connection| {
                update_resource_progress(
                    connection,
                    V2ResourceProgressInput {
                        resource_id: resource.id,
                        study_status: "studying".to_owned(),
                        personal_notes: "Read chapter 2".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("progress");
        assert_eq!(progress.progress.study_status, "studying");

        let pdf = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("pdf");
        fs::write(pdf.path(), b"%PDF-1.4\ntest resource\n").expect("write pdf");
        let data_dir = state.data_directory().expect("data dir");
        let with_file = state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: pdf.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(2),
                )
            })
            .expect("file");
        assert!(with_file.file.is_some());
        assert_eq!(with_file.file.as_ref().expect("file").status, "ok");
        assert_eq!(with_file.file.as_ref().expect("file").sha256.len(), 64);
        assert_eq!(
            with_file.file.as_ref().expect("file").storage_mode,
            "managed"
        );
        let managed_path = with_file.file.as_ref().expect("file").file_path.clone();
        assert!(Path::new(&managed_path).is_file());

        let removed = state
            .with_connection(|connection| {
                remove_resource_file(connection, &data_dir, resource.id, at(3))
            })
            .expect("remove file");
        assert!(removed.file.is_none());
        assert!(!Path::new(&managed_path).is_file());
        assert!(pdf.path().is_file(), "external source must not be deleted");

        let archived = state
            .with_connection(|connection| {
                archive_resource(
                    connection,
                    V2ResourceArchiveInput {
                        resource_id: resource.id,
                        archived: true,
                    },
                    at(4),
                )
            })
            .expect("archive");
        assert!(archived.is_archived);
        let visible = state
            .with_connection(|connection| {
                list_resources(
                    connection,
                    V2ResourceSearchFilter {
                        query: None,
                        exam_code: None,
                        include_archived: Some(false),
                    },
                )
            })
            .expect("visible");
        assert!(visible.is_empty());
    }

    #[test]
    fn managed_pdf_missing_detection_relinking_and_restore_metadata_are_consistent() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let resource = state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: "Managed PDF".to_owned(),
                        description: String::new(),
                        subject_area: "Optics".to_owned(),
                        url: None,
                        resource_type: "book".to_owned(),
                        source: String::new(),
                        concept_ids: Vec::new(),
                        exam_links: Vec::new(),
                    },
                    at(0),
                )
            })
            .expect("resource");
        let source = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("source pdf");
        fs::write(source.path(), b"%PDF-1.4\nmanaged source\n").expect("source");

        let imported = state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: source.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("managed import");
        let first_path = imported.file.as_ref().expect("file").file_path.clone();
        assert!(Path::new(&first_path).is_file());
        assert!(source.path().is_file());

        fs::remove_file(&first_path).expect("remove managed copy");
        let missing = state
            .with_connection(|connection| {
                list_resources(
                    connection,
                    V2ResourceSearchFilter {
                        query: Some("Managed PDF".to_owned()),
                        exam_code: None,
                        include_archived: Some(false),
                    },
                )
            })
            .expect("missing check")
            .into_iter()
            .find(|candidate| candidate.id == resource.id)
            .expect("resource");
        assert_eq!(missing.file.as_ref().expect("file").status, "missing");

        let replacement = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("replacement pdf");
        fs::write(replacement.path(), b"%PDF-1.4\nreplacement source\n").expect("replacement");
        let relinked = state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: replacement.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(2),
                )
            })
            .expect("relink");
        let second_path = relinked.file.as_ref().expect("file").file_path.clone();
        assert_ne!(first_path, second_path);
        assert!(Path::new(&second_path).is_file());
        assert_eq!(relinked.file.as_ref().expect("file").status, "ok");
        assert!(source.path().is_file());
        assert!(replacement.path().is_file());

        let backup = state
            .with_connection(|connection| create_manual_backup(connection, &data_dir, at(3)))
            .expect("backup");
        state
            .with_connection(|connection| {
                archive_resource(
                    connection,
                    V2ResourceArchiveInput {
                        resource_id: resource.id,
                        archived: true,
                    },
                    at(4),
                )
            })
            .expect("archive");
        state
            .restore_backup(
                crate::v2::backup::V2RestoreInput {
                    path: backup.path.clone(),
                },
                at(5),
            )
            .expect("restore");
        let restored = state
            .with_connection(|connection| {
                list_resources(
                    connection,
                    V2ResourceSearchFilter {
                        query: Some("Managed PDF".to_owned()),
                        exam_code: None,
                        include_archived: Some(true),
                    },
                )
            })
            .expect("restored resource")
            .into_iter()
            .find(|candidate| candidate.id == resource.id)
            .expect("restored resource");
        assert!(!restored.is_archived);
        let restored_file = restored.file.as_ref().expect("file");
        assert_eq!(restored_file.status, "ok");
        assert_eq!(restored_file.file_path, second_path);
        assert!(Path::new(&restored_file.file_path).is_file());

        let external = state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: source.path().display().to_string(),
                        storage_mode: "external".to_owned(),
                    },
                    at(6),
                )
            })
            .expect("external relink");
        assert_eq!(
            external.file.as_ref().expect("file").storage_mode,
            "external"
        );
        let removed = state
            .with_connection(|connection| {
                remove_resource_file(connection, &data_dir, resource.id, at(7))
            })
            .expect("remove external");
        assert!(removed.file.is_none());
        assert!(
            source.path().is_file(),
            "external source files must never be deleted"
        );
    }

    #[test]
    fn managed_deletion_never_follows_path_traversal_outside_the_managed_folder() {
        let (_root, state) = state();
        let data_dir = state.data_directory().expect("data dir");
        let resource = state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: "Traversal guard".to_owned(),
                        description: String::new(),
                        subject_area: String::new(),
                        url: None,
                        resource_type: "book".to_owned(),
                        source: String::new(),
                        concept_ids: Vec::new(),
                        exam_links: Vec::new(),
                    },
                    at(0),
                )
            })
            .expect("resource");
        let outside = data_dir.join("outside.pdf");
        fs::write(&outside, b"%PDF-1.4\noutside managed guard\n").expect("outside pdf");
        let source = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("source pdf");
        fs::write(source.path(), b"%PDF-1.4\nfirst managed\n").expect("source");
        state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: source.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(1),
                )
            })
            .expect("managed import");
        let traversal = data_dir
            .join("resources")
            .join("..")
            .join("outside.pdf")
            .display()
            .to_string();
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE resource_files
                         SET file_path = ?1, file_path_normalized = ?2
                         WHERE resource_id = ?3",
                        rusqlite::params![traversal, traversal.to_lowercase(), resource.id],
                    )
                    .map_err(|error| {
                        AppError::database("The traversal test path could not be saved", error)
                    })
            })
            .expect("traversal path");
        let removed = state
            .with_connection(|connection| {
                remove_resource_file(connection, &data_dir, resource.id, at(2))
            })
            .expect("remove traversal");
        assert!(removed.file.is_none());
        assert!(
            outside.is_file(),
            "deletion must not escape the managed resources folder"
        );

        state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: source.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(3),
                )
            })
            .expect("second managed import");
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE resource_files
                         SET file_path = ?1, file_path_normalized = ?2
                         WHERE resource_id = ?3",
                        rusqlite::params![traversal, traversal.to_lowercase(), resource.id],
                    )
                    .map_err(|error| {
                        AppError::database("The traversal test path could not be saved", error)
                    })
            })
            .expect("traversal path again");
        let replacement = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("replacement pdf");
        fs::write(replacement.path(), b"%PDF-1.4\nreplacement managed\n").expect("replacement");
        let replaced = state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: replacement.path().display().to_string(),
                        storage_mode: "managed".to_owned(),
                    },
                    at(4),
                )
            })
            .expect("replacement");
        assert!(replaced.file.is_some());
        assert!(
            outside.is_file(),
            "replacement cleanup must not escape the managed resources folder"
        );
        let replacement_path = replaced.file.as_ref().expect("file").file_path.clone();
        assert!(Path::new(&replacement_path).is_file());
        state
            .with_connection(|connection| {
                remove_resource_file(connection, &data_dir, resource.id, at(5))
            })
            .expect("remove replacement");
        assert!(!Path::new(&replacement_path).is_file());
        assert!(outside.is_file());
    }

    #[test]
    fn invalid_resource_urls_and_exam_links_are_rejected() {
        let (_root, state) = state();
        let invalid_url = state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: "Bad".to_owned(),
                        description: String::new(),
                        subject_area: String::new(),
                        url: Some("ftp://example.com".to_owned()),
                        resource_type: "website".to_owned(),
                        source: String::new(),
                        concept_ids: Vec::new(),
                        exam_links: Vec::new(),
                    },
                    at(0),
                )
            })
            .expect_err("invalid url");
        assert_eq!(invalid_url.field, Some("url"));

        let invalid_link = state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: "Link".to_owned(),
                        description: String::new(),
                        subject_area: String::new(),
                        url: None,
                        resource_type: "other".to_owned(),
                        source: String::new(),
                        concept_ids: Vec::new(),
                        exam_links: vec![V2ResourceExamLinkInput {
                            exam_id: 99,
                            category: "core".to_owned(),
                            syllabus_node_id: None,
                            priority: "normal".to_owned(),
                            notes: String::new(),
                        }],
                    },
                    at(1),
                )
            })
            .expect_err("invalid exam");
        assert_eq!(invalid_link.field, Some("examId"));
    }
}
