use std::collections::HashMap;

use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

const BUILTIN_CONSTANTS_SEED_JSON: &str = include_str!("builtin_constants_seed.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstantSeed {
    version_label: String,
    edition_year: i64,
    published_on: String,
    retrieved_on: String,
    publisher: String,
    source_url: String,
    citation: String,
    seed_sha256: String,
    constants: Vec<ConstantSeedEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstantSeedEntry {
    stable_key: String,
    name: String,
    symbol: String,
    value_text: String,
    unit: String,
    uncertainty_text: String,
    precision_text: String,
    is_exact: bool,
    category: String,
    description: String,
    notes: String,
    source_locator: String,
    source_reference: String,
    source_url: String,
    display_order: i64,
}

type StoredConstant = (String, String, String, String, String, String, i64, i64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BuiltinConstant {
    pub id: i64,
    pub stable_key: String,
    pub name: String,
    pub symbol: String,
    pub value_text: String,
    pub unit: String,
    pub uncertainty_text: String,
    pub precision_text: String,
    pub is_exact: bool,
    pub category: String,
    pub description: String,
    pub notes: String,
    pub source_locator: String,
    pub source_reference: String,
    pub source_url: String,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2BuiltinConstantCatalog {
    pub version_label: String,
    pub edition_year: i64,
    pub published_on: String,
    pub retrieved_on: String,
    pub publisher: String,
    pub source_url: String,
    pub citation: String,
    pub seed_sha256: String,
    pub constants: Vec<V2BuiltinConstant>,
}

fn seed_checksum(seed: &ConstantSeed) -> String {
    let mut canonical = String::new();
    for entry in &seed.constants {
        let line = [
            entry.stable_key.as_str(),
            entry.name.as_str(),
            entry.symbol.as_str(),
            entry.value_text.as_str(),
            entry.unit.as_str(),
            entry.uncertainty_text.as_str(),
            entry.precision_text.as_str(),
            if entry.is_exact { "1" } else { "0" },
            entry.category.as_str(),
            entry.description.as_str(),
            entry.notes.as_str(),
            entry.source_locator.as_str(),
            entry.source_reference.as_str(),
            entry.source_url.as_str(),
            &entry.display_order.to_string(),
        ]
        .join("|");
        if !canonical.is_empty() {
            canonical.push('\n');
        }
        canonical.push_str(&line);
    }
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Seed the immutable built-in constants catalogue and verify that the stored
/// rows still match the shipped canonical payload. This function only inserts;
/// database triggers reject every update or delete of an existing entry.
pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    let seed: ConstantSeed = serde_json::from_str(BUILTIN_CONSTANTS_SEED_JSON)
        .map_err(|error| AppError::internal(format!("The constants seed is invalid: {error}")))?;
    let expected_checksum = seed_checksum(&seed);
    if expected_checksum != seed.seed_sha256 {
        return Err(AppError::schema_integrity(
            "The built-in constants seed checksum does not match its contents.",
        ));
    }

    let transaction = connection.transaction().map_err(|error| {
        AppError::database(
            "The built-in constants seed transaction could not start",
            error,
        )
    })?;
    let token = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    transaction
        .execute(
            "INSERT INTO constant_catalog_versions (
                version_label, edition_year, published_on, retrieved_on, publisher,
                source_url, citation, seed_sha256, is_active, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)
             ON CONFLICT(version_label) DO NOTHING",
            params![
                seed.version_label,
                seed.edition_year,
                seed.published_on,
                seed.retrieved_on,
                seed.publisher,
                seed.source_url,
                seed.citation,
                expected_checksum,
                token
            ],
        )
        .map_err(|error| {
            AppError::database(
                "The built-in constants catalogue version could not be stored",
                error,
            )
        })?;
    let (catalog_id, stored_checksum): (i64, String) = transaction
        .query_row(
            "SELECT id, seed_sha256 FROM constant_catalog_versions WHERE version_label=?1",
            [&seed.version_label],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| {
            AppError::database(
                "The built-in constants catalogue version could not be read",
                error,
            )
        })?;
    if stored_checksum != expected_checksum {
        return Err(AppError::schema_integrity(
            "The stored built-in constants catalogue checksum does not match the shipped seed.",
        ));
    }
    transaction
        .execute(
            "UPDATE constant_catalog_versions SET is_active=1
             WHERE version_label=?1
               AND NOT EXISTS (
                   SELECT 1 FROM constant_catalog_versions
                   WHERE is_active=1 AND version_label<>?1
               )",
            [&seed.version_label],
        )
        .map_err(|error| {
            AppError::database(
                "The built-in constants catalogue could not be activated",
                error,
            )
        })?;

    for entry in &seed.constants {
        transaction
            .execute(
                "INSERT INTO builtin_constants (
                    catalog_version_id, stable_key, name, symbol, value_text, unit,
                    uncertainty_text, precision_text, is_exact, category, description,
                    notes, source_locator, source_reference, source_url, display_order,
                    created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                           ?14, ?15, ?16, ?17)
                 ON CONFLICT(catalog_version_id, stable_key) DO NOTHING",
                params![
                    catalog_id,
                    entry.stable_key,
                    entry.name,
                    entry.symbol,
                    entry.value_text,
                    entry.unit,
                    entry.uncertainty_text,
                    entry.precision_text,
                    if entry.is_exact { 1 } else { 0 },
                    entry.category,
                    entry.description,
                    entry.notes,
                    entry.source_locator,
                    entry.source_reference,
                    entry.source_url,
                    entry.display_order,
                    token
                ],
            )
            .map_err(|error| {
                AppError::database("A built-in constant could not be stored", error)
            })?;
    }

    let stored: HashMap<String, StoredConstant> = {
        let mut statement = transaction
            .prepare(
                "SELECT stable_key, name, symbol, value_text, unit, uncertainty_text,
                            precision_text, is_exact, display_order
                     FROM builtin_constants WHERE catalog_version_id=?1",
            )
            .map_err(|error| {
                AppError::database(
                    "The built-in constants verification could not prepare",
                    error,
                )
            })?;
        let rows = statement
            .query_map([catalog_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ),
                ))
            })
            .map_err(|error| {
                AppError::database("The built-in constants could not be verified", error)
            })?;
        let mut values = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|error| {
                AppError::database("A built-in constant verification row is invalid", error)
            })?;
            values.insert(key, value);
        }
        values
    };
    if stored.len() != seed.constants.len() {
        return Err(AppError::schema_integrity(
            "The built-in constants catalogue does not contain the expected entries.",
        ));
    }
    for entry in &seed.constants {
        let Some((
            name,
            symbol,
            value_text,
            unit,
            uncertainty_text,
            precision_text,
            is_exact,
            display_order,
        )) = stored.get(&entry.stable_key)
        else {
            return Err(AppError::schema_integrity(
                "The built-in constants catalogue is missing an expected entry.",
            ));
        };
        if name != &entry.name
            || symbol != &entry.symbol
            || value_text != &entry.value_text
            || unit != &entry.unit
            || uncertainty_text != &entry.uncertainty_text
            || precision_text != &entry.precision_text
            || *is_exact != if entry.is_exact { 1 } else { 0 }
            || *display_order != entry.display_order
        {
            return Err(AppError::schema_integrity(
                "A stored built-in constant does not match the verified seed.",
            ));
        }
    }

    transaction.commit().map_err(|error| {
        AppError::database("The built-in constants seed could not be committed", error)
    })?;
    Ok(())
}

pub fn list_builtin_constants(
    connection: &Connection,
) -> Result<V2BuiltinConstantCatalog, AppError> {
    let version = connection
        .query_row(
            "SELECT id, version_label, edition_year, published_on, retrieved_on,
                    publisher, source_url, citation, seed_sha256
             FROM constant_catalog_versions
             WHERE is_active=1
             ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            AppError::database("The built-in constants catalogue could not be read", error)
        })?
        .ok_or_else(|| {
            AppError::schema_integrity("The built-in constants catalogue is not configured.")
        })?;

    let mut statement = connection
        .prepare(
            "SELECT id, stable_key, name, symbol, value_text, unit, uncertainty_text,
                    precision_text, is_exact, category, description, notes,
                    source_locator, source_reference, source_url, display_order
             FROM builtin_constants
             WHERE catalog_version_id=?1
             ORDER BY display_order, id",
        )
        .map_err(|error| {
            AppError::database("The built-in constants list could not be prepared", error)
        })?;
    let rows = statement
        .query_map([version.0], |row| {
            Ok(V2BuiltinConstant {
                id: row.get(0)?,
                stable_key: row.get(1)?,
                name: row.get(2)?,
                symbol: row.get(3)?,
                value_text: row.get(4)?,
                unit: row.get(5)?,
                uncertainty_text: row.get(6)?,
                precision_text: row.get(7)?,
                is_exact: row.get::<_, i64>(8)? == 1,
                category: row.get(9)?,
                description: row.get(10)?,
                notes: row.get(11)?,
                source_locator: row.get(12)?,
                source_reference: row.get(13)?,
                source_url: row.get(14)?,
                display_order: row.get(15)?,
            })
        })
        .map_err(|error| {
            AppError::database("The built-in constants list could not be read", error)
        })?;
    let mut constants = Vec::new();
    for row in rows {
        constants.push(
            row.map_err(|error| AppError::database("A built-in constant row is invalid", error))?,
        );
    }
    Ok(V2BuiltinConstantCatalog {
        version_label: version.1,
        edition_year: version.2,
        published_on: version.3,
        retrieved_on: version.4,
        publisher: version.5,
        source_url: version.6,
        citation: version.7,
        seed_sha256: version.8,
        constants,
    })
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

    #[test]
    fn built_in_catalogue_is_seeded_with_exact_values_and_sources() {
        let (_root, state) = state();
        let catalog = state
            .with_connection(|connection| list_builtin_constants(connection))
            .expect("built-in constants");
        assert_eq!(catalog.version_label, "CODATA 2022");
        assert_eq!(catalog.edition_year, 2022);
        assert_eq!(catalog.constants.len(), 43);
        assert_eq!(
            catalog.seed_sha256,
            "11d7a8f6acfe845d725f659a2c569911b0f22251da6c7638ddb61876bed4cd11"
        );
        let light = catalog
            .constants
            .iter()
            .find(|entry| entry.stable_key == "codata-2022-speed-of-light")
            .expect("speed of light");
        assert_eq!(light.value_text, "299 792 458");
        assert_eq!(light.uncertainty_text, "(exact)");
        assert!(light.is_exact);
        let planck = catalog
            .constants
            .iter()
            .find(|entry| entry.stable_key == "codata-2022-planck-constant")
            .expect("Planck constant");
        assert_eq!(planck.value_text, "6.626 070 15 e-34");
        assert_eq!(planck.unit, "J Hz^-1");
        let au = catalog
            .constants
            .iter()
            .find(|entry| entry.stable_key == "iau-2012-astronomical-unit")
            .expect("astronomical unit");
        assert_eq!(au.value_text, "1.495 978 707 e11");
        assert!(au.notes.contains("not a measured distance"));
    }

    #[test]
    fn built_in_rows_reject_direct_update_and_delete() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let id: i64 = connection
                    .query_row(
                        "SELECT id FROM builtin_constants ORDER BY display_order LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .expect("built-in id");
                let update = connection.execute(
                    "UPDATE builtin_constants SET value_text='tampered' WHERE id=?1",
                    [id],
                );
                assert!(update.is_err(), "built-in update must be blocked");
                let delete = connection.execute("DELETE FROM builtin_constants WHERE id=?1", [id]);
                assert!(delete.is_err(), "built-in delete must be blocked");
                Ok::<(), AppError>(())
            })
            .expect("built-in protection");
    }
}
