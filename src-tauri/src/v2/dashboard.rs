use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const DEFAULT_SECTIONS: [(&str, &str); 10] = [
    ("quickstart", "Quick start timer"),
    ("today", "Today's overview"),
    ("plan", "Today's study plan"),
    ("revision", "Revision overview"),
    ("recent", "Recently studied"),
    ("performance", "Performance and weak areas"),
    ("overview", "Database overview"),
    ("study", "Study timer and sessions"),
    ("exams", "Examination workspaces"),
    ("database", "V2 database details"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DashboardSection {
    pub section_key: String,
    pub title: String,
    pub visible: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DashboardSectionInput {
    pub section_key: String,
    pub visible: bool,
    pub display_order: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DashboardLayoutInput {
    #[serde(default)]
    pub exam_id: Option<i64>,
    pub sections: Vec<V2DashboardSectionInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2DashboardLayoutScope {
    #[serde(default)]
    pub exam_id: Option<i64>,
}

pub fn load_layout(
    connection: &Connection,
    input: V2DashboardLayoutScope,
) -> Result<Vec<V2DashboardSection>, AppError> {
    validate_scope(connection, input.exam_id)?;
    let mut statement = connection
        .prepare(
            "SELECT section_key, visible, display_order
             FROM dashboard_sections
             WHERE exam_id IS ?1
             ORDER BY display_order, id",
        )
        .map_err(|error| AppError::database("Dashboard sections could not be prepared", error))?;
    let rows = statement
        .query_map([input.exam_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)? == 1,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| AppError::database("Dashboard sections could not be read", error))?;
    let mut overrides = std::collections::HashMap::new();
    for row in rows {
        let (key, visible, order) =
            row.map_err(|error| AppError::database("A dashboard section row is invalid", error))?;
        overrides.insert(key, (visible, order));
    }
    let mut sections = DEFAULT_SECTIONS
        .iter()
        .enumerate()
        .map(|(index, (key, title))| {
            let (visible, display_order) = overrides.remove(*key).unwrap_or((true, index as i64));
            V2DashboardSection {
                section_key: (*key).to_owned(),
                title: (*title).to_owned(),
                visible,
                display_order,
            }
        })
        .collect::<Vec<_>>();
    sections.sort_by_key(|section| section.display_order);
    Ok(sections)
}

pub fn save_layout(
    connection: &mut Connection,
    input: V2DashboardLayoutInput,
    now: DateTime<Utc>,
) -> Result<Vec<V2DashboardSection>, AppError> {
    validate_scope(connection, input.exam_id)?;
    validate_sections(&input.sections)?;
    let transaction = connection.transaction().map_err(|error| {
        AppError::database("The dashboard layout transaction could not start", error)
    })?;
    transaction
        .execute(
            "DELETE FROM dashboard_sections WHERE exam_id IS ?1",
            [input.exam_id],
        )
        .map_err(|error| AppError::database("Dashboard layout could not be replaced", error))?;
    let now_text = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    for section in &input.sections {
        transaction
            .execute(
                "INSERT INTO dashboard_sections (
                    exam_id, section_key, visible, display_order, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    input.exam_id,
                    section.section_key,
                    i64::from(section.visible),
                    section.display_order,
                    now_text,
                ],
            )
            .map_err(|error| AppError::database("Dashboard section could not be saved", error))?;
    }
    transaction
        .commit()
        .map_err(|error| AppError::database("Dashboard layout could not be committed", error))?;
    load_layout(
        connection,
        V2DashboardLayoutScope {
            exam_id: input.exam_id,
        },
    )
}

pub fn reset_layout(
    connection: &mut Connection,
    input: V2DashboardLayoutScope,
    _now: DateTime<Utc>,
) -> Result<Vec<V2DashboardSection>, AppError> {
    validate_scope(connection, input.exam_id)?;
    connection
        .execute(
            "DELETE FROM dashboard_sections WHERE exam_id IS ?1",
            [input.exam_id],
        )
        .map_err(|error| AppError::database("Dashboard layout could not be reset", error))?;
    load_layout(connection, input)
}

fn validate_scope(connection: &Connection, exam_id: Option<i64>) -> Result<(), AppError> {
    if let Some(exam_id) = exam_id {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM exams WHERE id = ?1)",
                [exam_id],
                |row| row.get(0),
            )
            .map_err(|error| AppError::database("Examination could not be checked", error))?;
        if exists == 0 {
            return Err(AppError::validation(
                "examId",
                "The selected examination does not exist.",
            ));
        }
    }
    Ok(())
}

fn validate_sections(sections: &[V2DashboardSectionInput]) -> Result<(), AppError> {
    let supported = DEFAULT_SECTIONS
        .iter()
        .map(|(key, _)| *key)
        .collect::<std::collections::HashSet<_>>();
    let mut seen = std::collections::HashSet::new();
    let mut orders = std::collections::HashSet::new();
    for section in sections {
        if !supported.contains(section.section_key.as_str()) {
            return Err(AppError::validation(
                "sectionKey",
                "Choose a supported dashboard section.",
            ));
        }
        if !seen.insert(section.section_key.as_str()) {
            return Err(AppError::validation(
                "sectionKey",
                "Each dashboard section may appear only once.",
            ));
        }
        if !(0..=1_000).contains(&section.display_order) {
            return Err(AppError::validation(
                "displayOrder",
                "Dashboard section order must be between 0 and 1,000.",
            ));
        }
        if !orders.insert(section.display_order) {
            return Err(AppError::validation(
                "displayOrder",
                "Each dashboard section must have a distinct order.",
            ));
        }
    }
    if sections.len() != DEFAULT_SECTIONS.len() {
        return Err(AppError::validation(
            "sections",
            "All supported dashboard sections must be provided.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-04T10:00:00Z")
            .expect("timestamp")
            .with_timezone(&Utc)
            + chrono::Duration::seconds(seconds)
    }

    fn layout(pairs: &[(&str, bool, i64)]) -> V2DashboardLayoutInput {
        let sections = pairs
            .iter()
            .map(|(key, visible, order)| V2DashboardSectionInput {
                section_key: (*key).to_owned(),
                visible: *visible,
                display_order: *order,
            })
            .collect();
        V2DashboardLayoutInput {
            exam_id: None,
            sections,
        }
    }

    const DEFAULT_LAYOUT: [(&str, bool, i64); 10] = [
        ("quickstart", true, 0),
        ("today", true, 1),
        ("plan", true, 2),
        ("revision", true, 3),
        ("recent", true, 4),
        ("performance", true, 5),
        ("overview", true, 6),
        ("study", true, 7),
        ("exams", true, 8),
        ("database", true, 9),
    ];

    fn order_of(sections: &[V2DashboardSection]) -> Vec<&str> {
        sections
            .iter()
            .map(|section| section.section_key.as_str())
            .collect()
    }

    #[test]
    fn default_layout_lists_all_sections_in_default_order() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let loaded = state
            .with_connection(|connection| {
                load_layout(connection, V2DashboardLayoutScope { exam_id: None })
            })
            .expect("load layout");
        assert_eq!(
            order_of(&loaded),
            [
                "quickstart",
                "today",
                "plan",
                "revision",
                "recent",
                "performance",
                "overview",
                "study",
                "exams",
                "database",
            ]
        );
        assert!(loaded.iter().all(|section| section.visible));
    }

    #[test]
    fn dashboard_visibility_and_order_survive_restart_and_reset() {
        let root = tempfile::tempdir().expect("temporary directory");
        let data_dir = root.path().join("v2");
        let state = V2State::initialize(&data_dir);
        let saved = state
            .with_connection(|connection| {
                save_layout(
                    connection,
                    layout(&[
                        ("overview", false, 30),
                        ("study", true, 10),
                        ("exams", false, 40),
                        ("database", true, 20),
                        ("quickstart", true, 0),
                        ("today", true, 1),
                        ("plan", true, 2),
                        ("revision", true, 3),
                        ("recent", true, 4),
                        ("performance", true, 5),
                    ]),
                    at(0),
                )
            })
            .expect("save layout");
        assert_eq!(
            order_of(&saved),
            [
                "quickstart",
                "today",
                "plan",
                "revision",
                "recent",
                "performance",
                "study",
                "database",
                "overview",
                "exams",
            ]
        );
        assert_eq!(saved.iter().filter(|section| !section.visible).count(), 2);
        assert!(!saved[8].visible);
        assert!(!saved[9].visible);
        drop(state);

        let reopened = V2State::initialize(&data_dir);
        let loaded = reopened
            .with_connection(|connection| {
                load_layout(connection, V2DashboardLayoutScope { exam_id: None })
            })
            .expect("load layout");
        assert_eq!(
            order_of(&loaded),
            [
                "quickstart",
                "today",
                "plan",
                "revision",
                "recent",
                "performance",
                "study",
                "database",
                "overview",
                "exams",
            ]
        );
        assert!(!loaded[8].visible);
        assert!(!loaded[9].visible);

        let reset = reopened
            .with_connection(|connection| {
                reset_layout(connection, V2DashboardLayoutScope { exam_id: None }, at(1))
            })
            .expect("reset layout");
        assert_eq!(
            order_of(&reset),
            [
                "quickstart",
                "today",
                "plan",
                "revision",
                "recent",
                "performance",
                "overview",
                "study",
                "exams",
                "database",
            ]
        );
        assert!(reset.iter().all(|section| section.visible));
    }

    #[test]
    fn new_default_sections_merge_into_a_saved_legacy_layout() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        // Emulate a layout that a build before the smart-dashboard sections
        // persisted: only the four legacy keys exist as rows.
        state
            .with_connection(|connection| -> Result<(), AppError> {
                let legacy: [(&str, bool, i64); 4] = [
                    ("overview", true, 0),
                    ("study", false, 1),
                    ("exams", true, 2),
                    ("database", false, 3),
                ];
                for (key, visible, order) in legacy {
                    connection
                        .execute(
                            "INSERT INTO dashboard_sections (
                                exam_id, section_key, visible, display_order, updated_at
                             ) VALUES (NULL, ?1, ?2, ?3, ?4)",
                            params![key, i64::from(visible), order, "2026-11-01T00:00:00.000Z"],
                        )
                        .map_err(|error| {
                            AppError::database("legacy section row could not be inserted", error)
                        })?;
                }
                Ok(())
            })
            .expect("legacy rows inserted");
        let loaded = state
            .with_connection(|connection| {
                load_layout(connection, V2DashboardLayoutScope { exam_id: None })
            })
            .expect("load layout");
        assert_eq!(loaded.len(), 10);
        // Saved visibility for legacy keys is preserved; new sections default to
        // visible so an existing customization is not overwritten.
        for section in &loaded {
            match section.section_key.as_str() {
                "study" | "database" => assert!(!section.visible),
                _ => assert!(section.visible),
            }
        }
        assert!(
            loaded
                .iter()
                .any(|section| section.section_key == "quickstart")
        );
        assert!(
            loaded
                .iter()
                .any(|section| section.section_key == "performance")
        );

        let saved = state
            .with_connection(|connection| save_layout(connection, layout(&DEFAULT_LAYOUT), at(0)))
            .expect("full layout save");
        assert_eq!(saved.len(), 10);
        assert!(saved.iter().all(|section| section.visible));
    }

    #[test]
    fn unsupported_sections_and_duplicate_orders_are_rejected() {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(&root.path().join("v2"));
        let invalid = state
            .with_connection(|connection| {
                save_layout(connection, layout(&[("unsupported", true, 0)]), at(0))
            })
            .expect_err("unsupported section");
        assert_eq!(invalid.code, "validation_error");

        let mut duplicate_pairs = DEFAULT_LAYOUT
            .iter()
            .map(|(key, visible, order)| (*key, *visible, *order))
            .collect::<Vec<_>>();
        duplicate_pairs[1] = ("today", true, 0);
        let duplicate = state
            .with_connection(|connection| save_layout(connection, layout(&duplicate_pairs), at(1)))
            .expect_err("duplicate order");
        assert_eq!(duplicate.code, "validation_error");
    }
}
