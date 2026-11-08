use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// One exam/topic allocation inside a saved quick-start configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuickStartAllocation {
    pub exam_id: i64,
    pub exam_code: String,
    pub exam_name: String,
    pub node_id: i64,
    pub node_title: String,
    pub subject_title: String,
    pub allocated_minutes: i64,
}

/// A saved quick-start configuration with all of its allocations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuickStartConfig {
    pub id: i64,
    pub name: String,
    pub duration_minutes: i64,
    pub is_pinned: bool,
    pub created_at: String,
    pub updated_at: String,
    pub allocations: Vec<V2QuickStartAllocation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuickStartAllocationInput {
    pub exam_id: i64,
    pub syllabus_node_id: i64,
    pub allocated_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuickStartConfigInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub duration_minutes: i64,
    #[serde(default)]
    pub is_pinned: Option<bool>,
    #[serde(default)]
    pub allocations: Vec<V2QuickStartAllocationInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2QuickStartConfigDeleteInput {
    pub id: i64,
}

const MAX_CONFIG_AND_ALLOCATION_MINUTES: i64 = 240;

pub fn list_quick_start_configs(
    connection: &Connection,
) -> Result<Vec<V2QuickStartConfig>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT c.id, c.name, c.duration_minutes, c.is_pinned, c.created_at, c.updated_at
             FROM quick_start_configs c
             ORDER BY c.is_pinned DESC, c.updated_at DESC, c.id DESC",
        )
        .map_err(|error| {
            AppError::database("Quick-start configurations could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(V2QuickStartConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                duration_minutes: row.get(2)?,
                is_pinned: row.get::<_, i64>(3)? == 1,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                allocations: Vec::new(),
            })
        })
        .map_err(|error| {
            AppError::database("A quick-start configuration could not be read", error)
        })?;
    let mut configs = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A quick-start configuration is invalid", error))?;
    for config in &mut configs {
        config.allocations = config_allocations(connection, config.id)?;
    }
    Ok(configs)
}

fn config_allocations(
    connection: &Connection,
    config_id: i64,
) -> Result<Vec<V2QuickStartAllocation>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT a.exam_id, e.code, e.name,
                    a.syllabus_node_id, n.title,
                    COALESCE(parent.title, ''), a.allocated_minutes
             FROM quick_start_config_allocations a
             JOIN exams e ON e.id = a.exam_id
             JOIN exam_syllabus_nodes n
                ON n.exam_id = a.exam_id AND n.id = a.syllabus_node_id
             LEFT JOIN exam_syllabus_nodes parent
                ON parent.exam_id = n.exam_id AND parent.id = n.parent_id
             WHERE a.config_id = ?1
             ORDER BY a.position, a.id",
        )
        .map_err(|error| {
            AppError::database(
                "Quick-start configuration allocations could not be prepared",
                error,
            )
        })?;
    let rows = statement
        .query_map([config_id], map_allocation)
        .map_err(|error| {
            AppError::database(
                "A quick-start configuration allocation could not be read",
                error,
            )
        })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| AppError::database("A quick-start allocation is invalid", error))
}

fn map_allocation(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2QuickStartAllocation> {
    Ok(V2QuickStartAllocation {
        exam_id: row.get(0)?,
        exam_code: row.get(1)?,
        exam_name: row.get(2)?,
        node_id: row.get(3)?,
        node_title: row.get(4)?,
        subject_title: row.get(5)?,
        allocated_minutes: row.get(6)?,
    })
}

pub fn save_quick_start_config(
    connection: &mut Connection,
    input: V2QuickStartConfigInput,
    now: DateTime<Utc>,
) -> Result<Vec<V2QuickStartConfig>, AppError> {
    let name = validate_config(connection, &input, input.id)?;
    let is_pinned = input.is_pinned;
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The quick-start save could not start", error))?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let config_id = match input.id {
        Some(id) => {
            let existing = transaction
                .query_row(
                    "SELECT is_pinned, created_at FROM quick_start_configs WHERE id = ?1",
                    [id],
                    |row| Ok((row.get::<_, i64>(0)? == 1, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(|error| {
                    AppError::database("The quick-start configuration could not be checked", error)
                })?;
            let Some((current_pinned, _created_at)) = existing else {
                return Err(AppError::validation(
                    "id",
                    "The configuration being saved no longer exists.",
                ));
            };
            transaction
                .execute(
                    "UPDATE quick_start_configs
                     SET name = ?1, duration_minutes = ?2, is_pinned = ?3, updated_at = ?4
                     WHERE id = ?5",
                    params![
                        name,
                        input.duration_minutes,
                        i64::from(is_pinned.unwrap_or(current_pinned)),
                        now_text,
                        id
                    ],
                )
                .map_err(|error| {
                    AppError::database("The quick-start configuration could not be updated", error)
                })?;
            transaction
                .execute(
                    "DELETE FROM quick_start_config_allocations WHERE config_id = ?1",
                    [id],
                )
                .map_err(|error| {
                    AppError::database("The quick-start allocations could not be replaced", error)
                })?;
            id
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO quick_start_configs (
                        name, duration_minutes, is_pinned, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        name,
                        input.duration_minutes,
                        i64::from(is_pinned.unwrap_or(false)),
                        now_text,
                        now_text
                    ],
                )
                .map_err(|error| {
                    AppError::database("The quick-start configuration could not be saved", error)
                })?;
            transaction.last_insert_rowid()
        }
    };
    save_allocations(&transaction, config_id, &input.allocations)?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The quick-start save could not commit", error))?;
    list_quick_start_configs(connection)
}

fn save_allocations(
    transaction: &rusqlite::Transaction<'_>,
    config_id: i64,
    allocations: &[V2QuickStartAllocationInput],
) -> Result<(), AppError> {
    for (position, allocation) in allocations.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO quick_start_config_allocations (
                    config_id, exam_id, syllabus_node_id, allocated_minutes, position
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    config_id,
                    allocation.exam_id,
                    allocation.syllabus_node_id,
                    allocation.allocated_minutes,
                    position as i64
                ],
            )
            .map_err(|error| {
                AppError::database("A quick-start allocation could not be saved", error)
            })?;
    }
    Ok(())
}

pub fn delete_quick_start_config(
    connection: &mut Connection,
    input: V2QuickStartConfigDeleteInput,
    _now: DateTime<Utc>,
) -> Result<Vec<V2QuickStartConfig>, AppError> {
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The quick-start delete could not start", error))?;
    let exists: Option<i64> = transaction
        .query_row(
            "SELECT id FROM quick_start_configs WHERE id = ?1",
            [input.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The quick-start configuration could not be checked", error)
        })?;
    if exists.is_none() {
        return Err(AppError::validation(
            "id",
            "The configuration has already been removed or does not exist.",
        ));
    }
    transaction
        .execute("DELETE FROM quick_start_configs WHERE id = ?1", [input.id])
        .map_err(|error| {
            AppError::database("The quick-start configuration could not be deleted", error)
        })?;
    transaction
        .commit()
        .map_err(|error| AppError::database("The quick-start delete could not commit", error))?;
    list_quick_start_configs(connection)
}

/// Trimmed name for persistence after enforcing every configuration rule:
/// bounded name and duration, active examinations, real unarchived syllabus
/// topics owned by that examination, no duplicated exam/topic pair, and a
/// total that never exceeds the planned session duration. The rules mirror
/// the timer's allocation validation so every saved configuration can be
/// started by the unified timer without reworking.
fn validate_config(
    connection: &Connection,
    input: &V2QuickStartConfigInput,
    current_id: Option<i64>,
) -> Result<String, AppError> {
    let name = input.name.trim().to_owned();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::validation(
            "name",
            "Give the configuration a name between 1 and 80 characters.",
        ));
    }
    if !(1..=MAX_CONFIG_AND_ALLOCATION_MINUTES).contains(&input.duration_minutes) {
        return Err(AppError::validation(
            "durationMinutes",
            "The session duration must be between 1 and 240 minutes.",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut total = 0_i64;
    for allocation in &input.allocations {
        if allocation.exam_id <= 0 || allocation.syllabus_node_id <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Choose an examination and topic for every allocation.",
            ));
        }
        if allocation.allocated_minutes <= 0 {
            return Err(AppError::validation(
                "allocations",
                "Allocated time must be greater than zero minutes.",
            ));
        }
        match exam_active(connection, allocation.exam_id)? {
            None => {
                return Err(AppError::validation(
                    "allocations",
                    "The selected examination does not exist.",
                ));
            }
            Some(false) => {
                return Err(AppError::forbidden(
                    "allocations",
                    "Inactive examinations cannot receive new study time.",
                ));
            }
            Some(true) => {}
        }
        let node: Option<(String, i64)> = connection
            .query_row(
                "SELECT node_kind, is_archived FROM exam_syllabus_nodes
                 WHERE exam_id = ?1 AND id = ?2",
                params![allocation.exam_id, allocation.syllabus_node_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| {
                AppError::database("The selected syllabus topic could not be read", error)
            })?;
        let Some((kind, archived)) = node else {
            return Err(AppError::validation(
                "allocations",
                "The selected topic is not part of the chosen syllabus.",
            ));
        };
        if kind != "topic" || archived == 1 {
            return Err(AppError::validation(
                "allocations",
                "Choose an active topic from the examination syllabus.",
            ));
        }
        if !seen.insert((allocation.exam_id, allocation.syllabus_node_id)) {
            return Err(AppError::validation(
                "allocations",
                "Each examination topic can appear only once in a configuration.",
            ));
        }
        total = total.saturating_add(allocation.allocated_minutes);
    }
    if total > input.duration_minutes {
        return Err(AppError::validation(
            "allocations",
            format!(
                "Allocations total {total} minutes, which exceeds the {}-minute session duration.",
                input.duration_minutes
            ),
        ));
    }
    let duplicate: Option<i64> = connection
        .query_row(
            "SELECT id FROM quick_start_configs
             WHERE trim(name) = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, current_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The quick-start name could not be checked", error))?;
    if duplicate.is_some() {
        return Err(AppError::validation(
            "name",
            "A configuration with this name already exists.",
        ));
    }
    Ok(name)
}

fn exam_active(connection: &Connection, exam_id: i64) -> Result<Option<bool>, AppError> {
    connection
        .query_row(
            "SELECT p.is_active
             FROM exams e
             JOIN exam_profiles p ON p.exam_id = e.id
             WHERE e.id = ?1",
            [exam_id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .optional()
        .map_err(|error| AppError::database("The examination could not be checked", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{V2_DATABASE_FILE_NAME, V2State};
    use std::path::Path;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-06T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
    }

    fn ids_in_db(connection: &Connection) -> (i64, i64) {
        let exam_id: i64 = connection
            .query_row(
                "SELECT id FROM exams WHERE code='csir-net-physics'",
                [],
                |row| row.get(0),
            )
            .expect("csir exam");
        let node_id: i64 = connection
            .query_row(
                "SELECT n.id FROM exam_syllabus_nodes n
                 JOIN exams e ON e.id = n.exam_id
                 WHERE e.code='csir-net-physics' AND n.node_kind='topic' AND n.is_archived=0
                 ORDER BY n.display_order LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("csir topic");
        (exam_id, node_id)
    }

    fn ids(state: &V2State) -> (i64, i64) {
        state.with_connection(|c| Ok(ids_in_db(c))).expect("ids")
    }

    fn input(
        name: &str,
        duration: i64,
        exam_id: i64,
        node_id: i64,
        minutes: i64,
    ) -> V2QuickStartConfigInput {
        V2QuickStartConfigInput {
            id: None,
            name: name.to_owned(),
            duration_minutes: duration,
            is_pinned: None,
            allocations: vec![V2QuickStartAllocationInput {
                exam_id,
                syllabus_node_id: node_id,
                allocated_minutes: minutes,
            }],
        }
    }

    fn save(state: &V2State, input: V2QuickStartConfigInput) -> Vec<V2QuickStartConfig> {
        state
            .with_connection(|connection| {
                save_quick_start_config(connection, input, now()).map(|mut list| {
                    list.sort_by_key(|config| config.id);
                    list
                })
            })
            .expect("save configuration")
    }

    fn count_allocations(state: &V2State) -> i64 {
        state
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM quick_start_config_allocations",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| AppError::database("allocations", error))
            })
            .expect("allocation count")
    }

    #[test]
    fn saved_configurations_persist_and_cascade_on_delete() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let (exam_id, node_id) = ids(&state);

        let saved = save(
            &state,
            input("Classical mechanics drill", 45, exam_id, node_id, 45),
        );
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].name, "Classical mechanics drill");
        assert_eq!(saved[0].duration_minutes, 45);
        assert!(!saved[0].is_pinned);
        assert_eq!(saved[0].allocations.len(), 1);
        assert_eq!(saved[0].allocations[0].allocated_minutes, 45);
        assert_eq!(saved[0].allocations[0].exam_code, "csir-net-physics");
        assert!(!saved[0].allocations[0].node_title.is_empty());
        drop(state);

        let reopened = V2State::initialize(&root.path().join("v2"));
        let loaded = reopened
            .with_connection(|connection| list_quick_start_configs(connection))
            .expect("list configurations");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Classical mechanics drill");
        assert_eq!(loaded[0].allocations.len(), 1);

        let deleted = reopened
            .with_connection(|connection| {
                delete_quick_start_config(
                    connection,
                    V2QuickStartConfigDeleteInput { id: loaded[0].id },
                    now(),
                )
            })
            .expect("delete configuration");
        assert!(deleted.is_empty());
        assert_eq!(count_allocations(&reopened), 0);
    }

    #[test]
    fn pinned_configurations_sort_before_others_and_updates_keep_the_id() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let (exam_id, node_id) = ids(&state);

        let first = save(&state, input("Alpha", 25, exam_id, node_id, 25))[0].id;
        let second = save(&state, input("Beta", 60, exam_id, node_id, 60))[0].id;
        // Pin the older configuration and edit the newer one in place.
        let pinned = state
            .with_connection(|connection| {
                save_quick_start_config(
                    connection,
                    V2QuickStartConfigInput {
                        id: Some(first),
                        is_pinned: Some(true),
                        ..input("Alpha", 40, exam_id, node_id, 40)
                    },
                    now() + chrono::Duration::seconds(5),
                )
            })
            .expect("pin");
        let ordered = pinned
            .iter()
            .map(|config| {
                (
                    config.name.as_str(),
                    config.is_pinned,
                    config.duration_minutes,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec![("Alpha", true, 40), ("Beta", false, 60),]);
        assert_eq!(pinned.len(), 2);
        assert!(pinned.iter().any(|config| config.id == second));
        assert_eq!(count_allocations(&state), 2);
    }

    #[test]
    fn duplicate_names_and_invalid_allocations_are_rejected() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let (exam_id, node_id) = ids(&state);

        save(&state, input("Morning drill", 45, exam_id, node_id, 45));
        let duplicate = state
            .with_connection(|connection| {
                save_quick_start_config(
                    connection,
                    input("  morning drill  ", 45, exam_id, node_id, 45),
                    now(),
                )
            })
            .expect_err("duplicate name");
        assert_eq!(duplicate.code, "validation_error");

        let under_file = save(&state, input("Below total", 60, exam_id, node_id, 45));
        assert_eq!(under_file[0].allocations[0].allocated_minutes, 45);

        let over_total = state
            .with_connection(|connection| {
                save_quick_start_config(
                    connection,
                    input("Over total", 30, exam_id, node_id, 45),
                    now(),
                )
            })
            .expect_err("allocation above duration");
        assert_eq!(over_total.code, "validation_error");

        let invalid_node = state
            .with_connection(|connection| {
                save_quick_start_config(
                    connection,
                    input("Wrong node owner", 25, exam_id, node_id + 100_000, 25),
                    now(),
                )
            })
            .expect_err("foreign node");
        assert_eq!(invalid_node.code, "validation_error");

        let zero_minutes = state
            .with_connection(|connection| {
                save_quick_start_config(
                    connection,
                    input("Zero minutes", 25, exam_id, node_id, 0),
                    now(),
                )
            })
            .expect_err("zero minutes");
        assert_eq!(zero_minutes.code, "validation_error");

        let empty_name = state
            .with_connection(|connection| {
                save_quick_start_config(connection, input("   ", 25, exam_id, node_id, 25), now())
            })
            .expect_err("empty name");
        assert_eq!(empty_name.code, "validation_error");

        // Nothing invalid was persisted.
        let loaded = state
            .with_connection(|connection| list_quick_start_configs(connection))
            .expect("list after failures");
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn schema_upgrade_from_version_one_recreates_the_tables() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = root.path().join("v2");
        let state = V2State::initialize(&data_dir);
        state
            .with_connection(|connection| -> Result<(), AppError> {
                connection
                    .execute("DELETE FROM quick_start_config_allocations", [])
                    .map_err(|error| AppError::database("drop allocations", error))?;
                connection
                    .execute("DROP TABLE quick_start_config_allocations", [])
                    .map_err(|error| AppError::database("drop allocations", error))?;
                connection
                    .execute("DROP TABLE quick_start_configs", [])
                    .map_err(|error| AppError::database("drop configs", error))?;
                connection
                    .execute(
                        "UPDATE app_metadata SET schema_version = 1 WHERE id = 1",
                        [],
                    )
                    .map_err(|error| AppError::database("metadata", error))?;
                connection
                    .execute_batch("PRAGMA user_version = 1;")
                    .map_err(|error| AppError::database("user_version", error))?;
                Ok(())
            })
            .expect("legacy downgrade");
        drop(state);

        let reopened = V2State::initialize(&data_dir);
        let status = reopened.status();
        assert_eq!(status.state, crate::v2::V2DatabaseState::Healthy);
        assert_eq!(status.schema_version, Some(crate::v2::V2_SCHEMA_VERSION));
        assert_eq!(
            status.metadata_schema_version,
            Some(crate::v2::V2_SCHEMA_VERSION)
        );
        assert!(status.migration_checksum_valid);
        let (exam_id, node_id) = ids(&reopened);
        let saved = save(&reopened, input("After upgrade", 30, exam_id, node_id, 30));
        assert_eq!(saved.len(), 1);
        // The append-only ledger keeps the seeded initial-schema row and the
        // database is verified healthy against the canonical payload.
        let ledger: Vec<(i64, String)> = reopened
            .with_connection(|connection| {
                let mut statement = connection
                    .prepare("SELECT version, name FROM schema_migrations ORDER BY version")
                    .map_err(|error| AppError::database("ledger", error))?;
                let rows = statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|error| AppError::database("ledger", error))?;
                let collected = rows
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| AppError::database("ledger", error))?;
                Ok(collected)
            })
            .expect("ledger rows");
        assert!(ledger.iter().any(|(version, _)| *version == 2));
        assert!(ledger.iter().any(|(version, _)| *version == 3));
        let _ = V2_DATABASE_FILE_NAME;
        let database_path = data_dir.join(V2_DATABASE_FILE_NAME);
        assert!(Path::new(&database_path).exists());
    }
}
