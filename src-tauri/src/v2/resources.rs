use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

const MAX_RESOURCE_PDF_BYTES: u64 = 512 * 1024 * 1024;
const PDF_HEADER_WINDOW_BYTES: usize = 1_024;
const PDF_EOF_WINDOW_BYTES: u64 = 8_192;
const PDF_READ_BUFFER_BYTES: usize = 64 * 1_024;

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
    #[serde(skip_serializing)]
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
            "(r.title LIKE ? OR r.description LIKE ? OR r.subject_area LIKE ? OR r.source LIKE ?
              OR EXISTS (
                SELECT 1 FROM resource_files rf
                WHERE rf.resource_id = r.id AND rf.file_name LIKE ?
              ))"
            .to_owned(),
        );
        let pattern = format!("%{query}%");
        for _ in 0..5 {
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

fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; PDF_READ_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn managed_pdf_file_name(sha256: &str, original_file_name: &str) -> String {
    let stem = Path::new(original_file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("resource");
    let safe_stem: String = stem
        .chars()
        .filter(|value| {
            !value.is_control()
                && !matches!(value, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        .take(80)
        .collect();
    let safe_stem = safe_stem.trim();
    if safe_stem.is_empty() {
        format!("{sha256}.pdf")
    } else {
        format!("{sha256}-{safe_stem}.pdf")
    }
}

/// Validate the PDF container without parsing the full document. The header
/// must occur within the first 1 KB and the final EOF marker must be present in
/// the last 8 KB, which catches renamed non-PDFs, empty files, and truncated
/// downloads while keeping large files out of memory.
fn validate_pdf_file(path: &Path) -> Result<(), AppError> {
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    if metadata.len() == 0 {
        return Err(AppError::validation(
            "filePath",
            "The selected PDF file is empty.",
        ));
    }
    if metadata.len() > MAX_RESOURCE_PDF_BYTES {
        return Err(AppError::validation(
            "filePath",
            "PDF size must be between 1 byte and 512 MB.",
        ));
    }
    let mut file = fs::File::open(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    let header_length = PDF_HEADER_WINDOW_BYTES.min(metadata.len() as usize);
    let mut header = vec![0_u8; header_length];
    file.read_exact(&mut header)
        .map_err(|error| AppError::file_operation("The PDF header could not be read", error))?;
    if !header.windows(5).any(|window| window == b"%PDF-") {
        return Err(AppError::validation(
            "filePath",
            "The selected file is not a valid PDF.",
        ));
    }
    let tail_length = PDF_EOF_WINDOW_BYTES.min(metadata.len());
    file.seek(SeekFrom::End(-(tail_length as i64)))
        .map_err(|error| AppError::file_operation("The PDF footer could not be read", error))?;
    let mut tail = vec![0_u8; tail_length as usize];
    file.read_exact(&mut tail)
        .map_err(|error| AppError::file_operation("The PDF footer could not be read", error))?;
    if !tail.windows(5).any(|window| window == b"%%EOF") {
        return Err(AppError::validation(
            "filePath",
            "The selected PDF is incomplete or corrupted.",
        ));
    }
    Ok(())
}

pub fn validate_pdf_source(path: &Path) -> Result<(), AppError> {
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
    validate_pdf_file(path)
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
    validate_pdf_source(path)?;
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::file_operation("The PDF could not be read", error))?;
    let resource_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resources WHERE id = ?1)",
            [input.resource_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Resource could not be checked", error))?;
    if resource_exists == 0 {
        return Err(AppError::resource_not_found());
    }
    let sha256 = sha256_file(path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::validation("filePath", "The PDF file name is invalid."))?
        .to_owned();
    if file_name.chars().count() > 260 {
        return Err(AppError::validation(
            "filePath",
            "The PDF file name must be 260 characters or fewer.",
        ));
    }
    let mut created_managed_file = None;
    let (file_path, normalized, size_bytes) = if input.storage_mode == "managed" {
        let directory = data_dir
            .join("resources")
            .join(input.resource_id.to_string());
        fs::create_dir_all(&directory).map_err(|error| {
            AppError::v2_file_operation("The managed resource folder could not be created", error)
        })?;
        let destination = directory.join(managed_pdf_file_name(&sha256, &file_name));
        if !destination.exists() {
            if let Err(error) = fs::copy(path, &destination) {
                let _ = fs::remove_file(&destination);
                return Err(AppError::v2_file_operation(
                    "The managed PDF copy could not be created",
                    error,
                ));
            }
            created_managed_file = Some(destination.clone());
        }
        let copied_hash = sha256_file(&destination)?;
        let copied_size = fs::metadata(&destination)
            .map_err(|error| {
                AppError::file_operation("The managed PDF could not be verified", error)
            })?
            .len();
        if copied_hash != sha256 || copied_size != metadata.len() {
            if created_managed_file.is_some() {
                let _ = fs::remove_file(&destination);
            }
            return Err(AppError::file_operation(
                "The managed PDF copy did not match the source file",
                "checksum mismatch",
            ));
        }
        (
            destination.display().to_string(),
            destination.to_string_lossy().to_lowercase(),
            copied_size as i64,
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

    let mut previous_for_cleanup = None;
    let save_result: Result<(), AppError> = (|| {
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
            .map_err(|error| {
                AppError::database("Existing resource file could not be read", error)
            })?;
        previous_for_cleanup = previous.clone();
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
            .map_err(|error| {
                AppError::database("Resource PDF metadata could not be saved", error)
            })?;
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
            .map_err(|error| {
                AppError::database("Resource file history could not be saved", error)
            })?;
        transaction.commit().map_err(|error| {
            AppError::database("Resource PDF metadata could not be committed", error)
        })?;
        Ok(())
    })();
    if let Err(error) = save_result {
        if let Some(created) = created_managed_file {
            let _ = fs::remove_file(created);
        }
        return Err(error);
    }
    if let Some((_, old_path, _, _, old_mode)) = previous_for_cleanup
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

/// Resolve a managed PDF path for opening. Only files that are still inside
/// the application's managed resources folder are accepted; no frontend path is
/// trusted or returned across IPC.
pub fn managed_file_path_for_open(
    connection: &Connection,
    data_dir: &Path,
    resource_id: i64,
) -> Result<std::path::PathBuf, AppError> {
    let file: Option<(String, String, i64)> = connection
        .query_row(
            "SELECT file_path, storage_mode, size_bytes
             FROM resource_files WHERE resource_id = ?1",
            [resource_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| AppError::database("Resource file could not be read", error))?;
    let Some((file_path, storage_mode, size_bytes)) = file else {
        return Err(AppError::validation(
            "resourceId",
            "This resource does not have an imported PDF.",
        ));
    };
    if storage_mode != "managed" {
        return Err(AppError::validation(
            "resourceId",
            "Only imported managed PDFs can be opened from the resource library.",
        ));
    }
    let managed_root = data_dir.join("resources").canonicalize().map_err(|error| {
        AppError::v2_file_operation("The managed resource folder could not be accessed", error)
    })?;
    let candidate = Path::new(&file_path).canonicalize().map_err(|error| {
        AppError::file_operation(
            "The imported PDF is missing. Relink or replace the local copy.",
            error,
        )
    })?;
    if !candidate.is_file() || !candidate.starts_with(&managed_root) {
        return Err(AppError::validation(
            "resourceId",
            "The imported PDF is outside the application's managed resource storage.",
        ));
    }
    let metadata = fs::metadata(&candidate)
        .map_err(|error| AppError::file_operation("The imported PDF could not be read", error))?;
    if metadata.len() as i64 != size_bytes {
        return Err(AppError::file_operation(
            "The imported PDF has changed size. Relink or replace the local copy.",
            "size mismatch",
        ));
    }
    Ok(candidate)
}

pub fn mark_resource_opened(
    connection: &mut Connection,
    resource_id: i64,
    now: DateTime<Utc>,
) -> Result<V2Resource, AppError> {
    let exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM resources WHERE id = ?1)",
            [resource_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Resource could not be checked", error))?;
    if exists == 0 {
        return Err(AppError::resource_not_found());
    }
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    connection
        .execute(
            "INSERT INTO resource_progress (
                resource_id, study_status, personal_notes, last_opened_at, updated_at
             ) VALUES (?1, 'not_started', '', ?2, ?2)
             ON CONFLICT(resource_id) DO UPDATE SET
                last_opened_at = excluded.last_opened_at,
                updated_at = excluded.updated_at",
            params![resource_id, now_text],
        )
        .map_err(|error| {
            AppError::database("Resource opened timestamp could not be saved", error)
        })?;
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

    fn pdf_bytes(body: &str) -> Vec<u8> {
        format!("%PDF-1.4\n{body}\n%%EOF\n").into_bytes()
    }

    fn new_resource(state: &V2State, title: &str) -> V2Resource {
        state
            .with_connection(|connection| {
                save_resource(
                    connection,
                    V2ResourceInput {
                        id: None,
                        title: title.to_owned(),
                        description: String::new(),
                        subject_area: String::new(),
                        url: None,
                        resource_type: "book".to_owned(),
                        source: "Local PDF".to_owned(),
                        concept_ids: Vec::new(),
                        exam_links: Vec::new(),
                    },
                    at(0),
                )
            })
            .expect("resource")
    }

    fn import_managed(
        state: &V2State,
        resource_id: i64,
        path: &Path,
    ) -> Result<V2Resource, AppError> {
        let data_dir = state.data_directory().expect("data dir");
        state.with_connection(|connection| {
            import_resource_file(
                connection,
                &data_dir,
                V2ResourceFileInput {
                    resource_id,
                    file_path: path.display().to_string(),
                    storage_mode: "managed".to_owned(),
                },
                at(1),
            )
        })
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
        fs::write(pdf.path(), pdf_bytes("test resource")).expect("write pdf");
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
        fs::write(source.path(), pdf_bytes("managed source")).expect("source");

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
        fs::write(replacement.path(), pdf_bytes("replacement source")).expect("replacement");
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
        fs::write(&outside, pdf_bytes("outside managed guard")).expect("outside pdf");
        let source = tempfile::Builder::new()
            .suffix(".pdf")
            .tempfile()
            .expect("source pdf");
        fs::write(source.path(), pdf_bytes("first managed")).expect("source");
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
        fs::write(replacement.path(), pdf_bytes("replacement managed")).expect("replacement");
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

    #[test]
    fn pdf_validation_rejects_extension_signature_empty_truncated_and_oversized_files() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Validation");
        let directory = tempfile::tempdir().expect("source directory");

        let text_file = directory.path().join("notes.txt");
        fs::write(&text_file, pdf_bytes("renamed text file")).expect("text file");
        let wrong_extension =
            import_managed(&state, resource.id, &text_file).expect_err("extension");
        assert_eq!(wrong_extension.field, Some("filePath"));
        assert!(wrong_extension.message.contains("Only PDF files"));

        let fake_pdf = directory.path().join("fake.pdf");
        fs::write(&fake_pdf, b"not a PDF").expect("fake pdf");
        let fake = import_managed(&state, resource.id, &fake_pdf).expect_err("signature");
        assert_eq!(fake.field, Some("filePath"));
        assert!(fake.message.contains("not a valid PDF"));

        let empty_pdf = directory.path().join("empty.pdf");
        fs::write(&empty_pdf, []).expect("empty pdf");
        let empty = import_managed(&state, resource.id, &empty_pdf).expect_err("empty");
        assert!(empty.message.contains("empty"));

        let truncated_pdf = directory.path().join("truncated.pdf");
        fs::write(&truncated_pdf, b"%PDF-1.4\nmissing footer\n").expect("truncated pdf");
        let truncated = import_managed(&state, resource.id, &truncated_pdf).expect_err("truncated");
        assert!(truncated.message.contains("incomplete or corrupted"));

        let oversized_pdf = directory.path().join("oversized.pdf");
        let oversized = fs::File::create(&oversized_pdf).expect("oversized pdf");
        oversized
            .set_len(MAX_RESOURCE_PDF_BYTES + 1)
            .expect("oversized length");
        let large = import_managed(&state, resource.id, &oversized_pdf).expect_err("oversized");
        assert!(large.message.contains("512 MB"));

        let managed_dir = state
            .data_directory()
            .expect("data dir")
            .join("resources")
            .join(resource.id.to_string());
        if managed_dir.exists() {
            assert_eq!(
                fs::read_dir(managed_dir)
                    .expect("managed directory")
                    .count(),
                0
            );
        }
        let file_rows: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM resource_files WHERE resource_id = ?1",
                        [resource.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("invalid file count", error))
            })
            .expect("invalid file count");
        assert_eq!(file_rows, 0);
    }

    #[test]
    fn managed_folder_failure_does_not_create_file_metadata() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Folder failure");
        let directory = tempfile::tempdir().expect("source directory");
        let source = directory.path().join("notes.pdf");
        fs::write(&source, pdf_bytes("folder failure")).expect("pdf");
        let data_dir = state.data_directory().expect("data dir");
        let resources_root = data_dir.join("resources");
        fs::create_dir_all(&resources_root).expect("resources root");
        fs::write(
            resources_root.join(resource.id.to_string()),
            b"not a directory",
        )
        .expect("blocking file");

        let error = import_managed(&state, resource.id, &source).expect_err("folder failure");
        assert_eq!(error.code, "file_operation_error");
        let file_rows: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM resource_files WHERE resource_id = ?1",
                        [resource.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("file count", error))
            })
            .expect("file count");
        assert_eq!(file_rows, 0);
        assert!(source.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn unreadable_pdf_is_rejected_without_creating_metadata() {
        use std::os::windows::fs::OpenOptionsExt;

        let (_root, state) = state();
        let resource = new_resource(&state, "Unreadable PDF");
        let directory = tempfile::tempdir().expect("source directory");
        let source = directory.path().join("locked.pdf");
        fs::write(&source, pdf_bytes("locked content")).expect("pdf");
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&source)
            .expect("lock file");

        let error = import_managed(&state, resource.id, &source).expect_err("unreadable");
        assert_eq!(error.code, "file_operation_error");
        drop(lock);
        let file_rows: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM resource_files WHERE resource_id = ?1",
                        [resource.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("file count", error))
            })
            .expect("file count");
        assert_eq!(file_rows, 0);
    }

    #[test]
    fn managed_destination_names_are_bounded_and_sanitized() {
        let sha = "a".repeat(64);
        let long_name = format!("{}.pdf", "Very Long Physics Notes ".repeat(20));
        let stored = managed_pdf_file_name(&sha, &long_name);
        assert!(stored.len() <= 64 + 1 + 80 + 4);
        assert!(stored.ends_with(".pdf"));

        let unsafe_name = "Physics/Notes?<>|*.pdf";
        let sanitized = managed_pdf_file_name(&sha, unsafe_name);
        assert!(!sanitized.contains(['/', '?', '<', '>', '|', '*']));
        assert!(sanitized.starts_with(&sha));
        assert!(sanitized.ends_with(".pdf"));
    }

    #[test]
    fn serialized_resource_files_do_not_expose_managed_paths() {
        let serialized = serde_json::to_value(V2ResourceFile {
            file_name: "notes.pdf".to_owned(),
            file_path: "C:\\private\\resources\\1\\notes.pdf".to_owned(),
            sha256: "a".repeat(64),
            size_bytes: 120,
            storage_mode: "managed".to_owned(),
            status: "ok".to_owned(),
            imported_at: "2026-11-04T10:00:00Z".to_owned(),
            last_verified_at: Some("2026-11-04T10:00:00Z".to_owned()),
            missing_since: None,
        })
        .expect("serialized resource file");
        assert!(serialized.get("filePath").is_none());
        assert_eq!(
            serialized.get("fileName").and_then(|value| value.as_str()),
            Some("notes.pdf")
        );
    }

    #[test]
    fn unicode_and_space_filenames_are_preserved_in_managed_storage() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Unicode PDF");
        let source_directory = tempfile::tempdir().expect("source directory");
        let file_name = "Physics Notes – Ω (Final).pdf";
        let source = source_directory.path().join(file_name);
        fs::write(&source, pdf_bytes("unicode content")).expect("unicode pdf");

        let imported = import_managed(&state, resource.id, &source).expect("unicode import");
        let file = imported.file.expect("file");
        assert_eq!(file.file_name, file_name);
        assert!(Path::new(&file.file_path).is_file());
        assert!(
            file.file_path
                .split(['\\', '/'])
                .any(|part| part.ends_with(file_name))
        );
        assert!(source.is_file());

        let by_file_name = state
            .with_connection(|connection| {
                list_resources(
                    connection,
                    V2ResourceSearchFilter {
                        query: Some("Ω".to_owned()),
                        exam_code: None,
                        include_archived: Some(true),
                    },
                )
            })
            .expect("search by filename");
        assert!(by_file_name.iter().any(|item| item.id == resource.id));
    }

    #[test]
    fn same_named_replacements_use_unique_paths_and_remove_the_old_managed_file() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Replacements");
        let first_directory = tempfile::tempdir().expect("first directory");
        let first = first_directory.path().join("notes.pdf");
        fs::write(&first, pdf_bytes("first version")).expect("first pdf");
        let first_result = import_managed(&state, resource.id, &first).expect("first import");
        let first_path = first_result.file.expect("first file").file_path;

        let second_directory = tempfile::tempdir().expect("second directory");
        let second = second_directory.path().join("notes.pdf");
        fs::write(&second, pdf_bytes("second version")).expect("second pdf");
        let second_result = import_managed(&state, resource.id, &second).expect("second import");
        let second_path = second_result.file.expect("second file").file_path;

        assert_ne!(first_path, second_path);
        assert!(!Path::new(&first_path).is_file());
        assert!(Path::new(&second_path).is_file());
        assert!(first.is_file());
        assert!(second.is_file());
    }

    #[test]
    fn duplicate_imports_do_not_duplicate_metadata_or_overwrite_the_managed_file() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Duplicate import");
        let directory = tempfile::tempdir().expect("source directory");
        let source = directory.path().join("notes.pdf");
        fs::write(&source, pdf_bytes("duplicate content")).expect("pdf");

        let first = import_managed(&state, resource.id, &source).expect("first import");
        let first_path = first.file.expect("first file").file_path;
        let second = import_managed(&state, resource.id, &source).expect("second import");
        let second_path = second.file.expect("second file").file_path;

        assert_eq!(first_path, second_path);
        assert!(Path::new(&first_path).is_file());
        let count: i64 = state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM resource_files WHERE resource_id = ?1",
                        [resource.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("file count", error))
            })
            .expect("file count");
        assert_eq!(count, 1);
        let managed_dir = state
            .data_directory()
            .expect("data dir")
            .join("resources")
            .join(resource.id.to_string());
        assert_eq!(
            fs::read_dir(managed_dir)
                .expect("managed directory")
                .count(),
            1
        );
    }

    #[test]
    fn metadata_failure_removes_a_newly_copied_managed_file() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Metadata failure");
        let directory = tempfile::tempdir().expect("source directory");
        let source = directory.path().join("notes.pdf");
        fs::write(&source, pdf_bytes("metadata failure cleanup")).expect("pdf");
        state
            .with_connection(|connection| {
                connection
                    .execute_batch(
                        "CREATE TRIGGER test_resource_file_insert_failure
                         BEFORE INSERT ON resource_files
                         BEGIN SELECT RAISE(ABORT,'forced metadata failure'); END;",
                    )
                    .map_err(|error| AppError::database("failure trigger", error))
            })
            .expect("failure trigger");

        let error = import_managed(&state, resource.id, &source).expect_err("metadata failure");
        assert_eq!(error.code, "database_error");

        let managed_dir = state
            .data_directory()
            .expect("data dir")
            .join("resources")
            .join(resource.id.to_string());
        if managed_dir.exists() {
            assert_eq!(
                fs::read_dir(managed_dir)
                    .expect("managed directory")
                    .count(),
                0,
                "failed metadata writes must not leave an orphaned PDF"
            );
        }
        assert!(source.is_file());
        state
            .with_connection(|connection| {
                connection
                    .execute_batch("DROP TRIGGER test_resource_file_insert_failure;")
                    .map_err(|error| AppError::database("drop failure trigger", error))
            })
            .expect("drop trigger");
        let retried = import_managed(&state, resource.id, &source).expect("retry import");
        assert_eq!(retried.file.expect("file").status, "ok");
    }

    #[test]
    fn managed_open_and_last_opened_metadata_are_validated() {
        let (_root, state) = state();
        let resource = new_resource(&state, "Open guard");
        let directory = tempfile::tempdir().expect("source directory");
        let source = directory.path().join("notes.pdf");
        fs::write(&source, pdf_bytes("open content")).expect("pdf");
        import_managed(&state, resource.id, &source).expect("import");
        let data_dir = state.data_directory().expect("data dir");

        let path = state
            .with_connection(|connection| {
                managed_file_path_for_open(connection, &data_dir, resource.id)
            })
            .expect("managed path");
        assert!(path.is_file());
        let managed_root = data_dir.join("resources").canonicalize().expect("root");
        assert!(path.starts_with(managed_root));

        let opened = state
            .with_connection(|connection| mark_resource_opened(connection, resource.id, at(2)))
            .expect("mark opened");
        assert!(opened.progress.last_opened_at.is_some());

        let outside = data_dir.join("outside.pdf");
        fs::write(&outside, pdf_bytes("outside open guard")).expect("outside pdf");
        let outside_size = fs::metadata(&outside).expect("outside metadata").len() as i64;
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE resource_files
                         SET file_path = ?1, file_path_normalized = ?2, size_bytes = ?3
                         WHERE resource_id = ?4",
                        rusqlite::params![
                            outside.display().to_string(),
                            outside.to_string_lossy().to_lowercase(),
                            outside_size,
                            resource.id
                        ],
                    )
                    .map_err(|error| AppError::database("outside open path", error))
            })
            .expect("outside open path");
        let outside_error = state
            .with_connection(|connection| {
                managed_file_path_for_open(connection, &data_dir, resource.id)
            })
            .expect_err("outside open");
        assert!(outside_error.message.contains("outside the application's"));
        assert!(outside.is_file());

        let external_source = directory.path().join("external.pdf");
        fs::write(&external_source, pdf_bytes("external content")).expect("external pdf");
        state
            .with_connection(|connection| {
                import_resource_file(
                    connection,
                    &data_dir,
                    V2ResourceFileInput {
                        resource_id: resource.id,
                        file_path: external_source.display().to_string(),
                        storage_mode: "external".to_owned(),
                    },
                    at(3),
                )
            })
            .expect("external import");
        let external_error = state
            .with_connection(|connection| {
                managed_file_path_for_open(connection, &data_dir, resource.id)
            })
            .expect_err("external open");
        assert!(
            external_error
                .message
                .contains("Only imported managed PDFs")
        );
    }

    #[test]
    fn imported_pdf_survives_reopening_and_metadata_edits() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_directory = root.path().join("v2");
        let source_directory = tempfile::tempdir().expect("source directory");
        let source = source_directory.path().join("persistent notes.pdf");
        fs::write(&source, pdf_bytes("persistent content")).expect("pdf");
        let (resource_id, managed_path) = {
            let state = V2State::initialize(&data_directory);
            let resource = new_resource(&state, "Persistent PDF");
            let imported = import_managed(&state, resource.id, &source).expect("import");
            let managed_path = imported.file.expect("file").file_path;
            let edited = state
                .with_connection(|connection| {
                    save_resource(
                        connection,
                        V2ResourceInput {
                            id: Some(resource.id),
                            title: "Renamed PDF".to_owned(),
                            description: "Edited description".to_owned(),
                            subject_area: "Optics".to_owned(),
                            url: None,
                            resource_type: "book".to_owned(),
                            source: "Local PDF".to_owned(),
                            concept_ids: Vec::new(),
                            exam_links: Vec::new(),
                        },
                        at(2),
                    )
                })
                .expect("metadata edit");
            assert_eq!(edited.title, "Renamed PDF");
            assert!(edited.file.as_ref().is_some_and(|file| file.status == "ok"));
            (resource.id, managed_path)
        };

        let reopened = V2State::initialize(&data_directory);
        let resources = reopened
            .with_connection(|connection| {
                list_resources(
                    connection,
                    V2ResourceSearchFilter {
                        query: Some("Renamed PDF".to_owned()),
                        exam_code: None,
                        include_archived: Some(true),
                    },
                )
            })
            .expect("reopened resources");
        let restored = resources
            .iter()
            .find(|item| item.id == resource_id)
            .expect("restored resource");
        assert_eq!(restored.description, "Edited description");
        let file = restored.file.as_ref().expect("restored file");
        assert_eq!(file.status, "ok");
        assert_eq!(file.file_path, managed_path);
        assert!(Path::new(&managed_path).is_file());
    }
}
