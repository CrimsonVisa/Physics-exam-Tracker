use std::collections::HashSet;

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const MAX_FORMULA_VARIABLES: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomFormulaVariable {
    pub id: i64,
    pub position: i64,
    pub symbol: String,
    pub meaning: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomFormula {
    pub id: i64,
    pub is_builtin: bool,
    pub name: String,
    pub latex: String,
    pub plain_text: String,
    pub category: String,
    pub description: String,
    pub notes: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub variables: Vec<V2CustomFormulaVariable>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomFormulaVariableInput {
    pub symbol: String,
    pub meaning: String,
    #[serde(default)]
    pub unit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomFormulaInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub latex: String,
    pub plain_text: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub variables: Vec<V2CustomFormulaVariableInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomUnit {
    pub id: i64,
    pub is_builtin: bool,
    pub name: String,
    pub symbol: String,
    pub quantity: String,
    pub dimension: String,
    pub si_classification: String,
    pub conversion_kind: String,
    pub conversion_factor: String,
    pub conversion_reference_unit: String,
    pub conversion_note: String,
    pub description: String,
    pub notes: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomUnitInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub symbol: String,
    pub quantity: String,
    #[serde(default)]
    pub dimension: String,
    pub si_classification: String,
    pub conversion_kind: String,
    #[serde(default)]
    pub conversion_factor: String,
    #[serde(default)]
    pub conversion_reference_unit: String,
    #[serde(default)]
    pub conversion_note: String,
    pub description: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomConstant {
    pub id: i64,
    pub is_builtin: bool,
    pub name: String,
    pub symbol: String,
    pub value_text: String,
    pub unit: String,
    pub category: String,
    pub uncertainty_text: String,
    pub precision_text: String,
    pub source: String,
    pub description: String,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2CustomConstantInput {
    #[serde(default)]
    pub id: Option<i64>,
    pub name: String,
    pub symbol: String,
    pub value_text: String,
    #[serde(default)]
    pub unit: String,
    pub category: String,
    #[serde(default)]
    pub uncertainty_text: String,
    #[serde(default)]
    pub precision_text: String,
    #[serde(default)]
    pub source: String,
    pub description: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug)]
struct ValidatedVariable {
    symbol: String,
    meaning: String,
    unit: String,
}

#[derive(Debug)]
struct ValidatedFormula {
    name: String,
    latex: String,
    plain_text: String,
    category: String,
    description: String,
    notes: String,
    source: String,
    variables: Vec<ValidatedVariable>,
}

fn required_text(field: &'static str, value: &str, max: usize) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation(field, "This field is required."));
    }
    if trimmed.chars().count() > max {
        return Err(AppError::validation(
            field,
            format!("Keep this field to {max} characters or fewer."),
        ));
    }
    Ok(trimmed.to_owned())
}

fn optional_text(field: &'static str, value: &str, max: usize) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.chars().count() > max {
        return Err(AppError::validation(
            field,
            format!("Keep this field to {max} characters or fewer."),
        ));
    }
    Ok(trimmed.to_owned())
}

/// Validate a required numeric text field without changing the characters the
/// user entered. Surrounding whitespace is ignored for presence and length
/// checks but retained for storage/display.
fn verbatim_text(field: &'static str, value: &str, max: usize) -> Result<String, AppError> {
    if value.trim().is_empty() {
        return Err(AppError::validation(field, "This field is required."));
    }
    if value.chars().count() > max {
        return Err(AppError::validation(
            field,
            format!("Keep this field to {max} characters or fewer."),
        ));
    }
    Ok(value.to_owned())
}

fn required_choice(field: &'static str, value: &str, allowed: &[&str]) -> Result<String, AppError> {
    if !allowed.contains(&value) {
        return Err(AppError::validation(
            field,
            "Choose one of the available options.",
        ));
    }
    Ok(value.to_owned())
}

fn superscript_character(character: char) -> Option<char> {
    match character {
        '⁰' => Some('0'),
        '¹' => Some('1'),
        '²' => Some('2'),
        '³' => Some('3'),
        '⁴' => Some('4'),
        '⁵' => Some('5'),
        '⁶' => Some('6'),
        '⁷' => Some('7'),
        '⁸' => Some('8'),
        '⁹' => Some('9'),
        '⁺' => Some('+'),
        '⁻' => Some('-'),
        _ => None,
    }
}

fn valid_exponent(value: &str) -> Option<String> {
    let (sign, digits) = match value.chars().next() {
        Some('+') => ("+", &value[1..]),
        Some('-') => ("-", &value[1..]),
        _ => ("", value),
    };
    if digits.is_empty() || !digits.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    Some(format!("{sign}{digits}"))
}

fn is_superscript_digit(character: char) -> bool {
    superscript_character(character).is_some_and(|mapped| mapped.is_ascii_digit())
}

fn normalize_exponent(value: &str, allow_unsigned_ascii: bool) -> Option<String> {
    let characters: Vec<char> = value.chars().collect();
    let first = characters.first().copied()?;
    let rest = &characters[1..];
    let all_superscript = characters
        .iter()
        .all(|character| superscript_character(*character).is_some());
    let all_ascii_digits = characters
        .iter()
        .all(|character| character.is_ascii_digit());
    let ascii_sign_then_superscript_digits = (first == '+' || first == '-')
        && rest
            .iter()
            .all(|character| is_superscript_digit(*character));
    let superscript_sign_then_ascii_digits =
        matches!(first, '⁺' | '⁻') && rest.iter().all(|character| character.is_ascii_digit());
    let ascii_sign_then_ascii_digits =
        (first == '+' || first == '-') && rest.iter().all(|character| character.is_ascii_digit());
    if !(all_superscript
        || (allow_unsigned_ascii && all_ascii_digits)
        || ascii_sign_then_superscript_digits
        || superscript_sign_then_ascii_digits
        || ascii_sign_then_ascii_digits)
    {
        return None;
    }

    let normalized: String = characters
        .iter()
        .filter_map(|character| {
            superscript_character(*character).or_else(|| {
                if character.is_ascii_digit() || *character == '+' || *character == '-' {
                    Some(*character)
                } else {
                    None
                }
            })
        })
        .collect();
    valid_exponent(&normalized)
}

fn normalize_caret_exponent(value: &str) -> Option<String> {
    normalize_exponent(value, true)
}

fn normalize_superscript_exponent(value: &str) -> Option<String> {
    // Without a caret, plain ASCII digits are ambiguous multiplication (for
    // example 5×100). Require a superscript exponent or an explicit sign.
    normalize_exponent(value, false)
}

fn is_plain_decimal(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('e')
        && !value.contains('E')
        && value.parse::<f64>().is_ok_and(f64::is_finite)
}

fn normalize_numeric_text(value: &str) -> String {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| match character {
            '−' | '–' => '-',
            _ => character,
        })
        .collect();

    // Accept scientific notation written with a multiplication sign and an
    // explicit caret or Unicode superscript exponent. The base is validated as
    // a plain decimal so arbitrary text and multiple operators are rejected.
    for marker in ["×10", "x10", "X10", "·10", "⋅10", "∙10", "*10"] {
        for (index, _) in compact.match_indices(marker) {
            let base = &compact[..index];
            if !is_plain_decimal(base) {
                continue;
            }
            let rest = &compact[index + marker.len()..];
            let exponent = if let Some(exponent) = rest.strip_prefix('^') {
                normalize_caret_exponent(exponent)
            } else {
                normalize_superscript_exponent(rest)
            };
            if let Some(exponent) = exponent {
                return format!("{base}e{exponent}");
            }
        }
    }
    // Also accept superscript exponents after ordinary e/E notation, such as
    // 6.626e⁻³⁴. An explicit e/E exponent may use unsigned ASCII digits.
    for marker in ["e", "E"] {
        for (index, _) in compact.match_indices(marker) {
            let base = &compact[..index];
            if !is_plain_decimal(base) {
                continue;
            }
            if let Some(exponent) = normalize_exponent(&compact[index + 1..], true) {
                return format!("{base}e{exponent}");
            }
        }
    }
    compact
}

fn finite_number(field: &'static str, value: &str) -> Result<f64, AppError> {
    let normalized = normalize_numeric_text(value);
    let parsed = normalized.parse::<f64>().map_err(|_| {
        AppError::validation(
            field,
            "Enter a finite number, for example 1.602e-19, 6.626×10^-34 or 299792458.",
        )
    })?;
    if !parsed.is_finite() {
        return Err(AppError::validation(
            field,
            "Enter a finite number, for example 1.602e-19, 6.626×10^-34 or 299792458.",
        ));
    }
    Ok(parsed)
}

fn validate_formula(input: &V2CustomFormulaInput) -> Result<ValidatedFormula, AppError> {
    let name = required_text("name", &input.name, 200)?;
    let latex = required_text("latex", &input.latex, 20000)?;
    let plain_text = required_text("plainText", &input.plain_text, 20000)?;
    let category = required_text("category", &input.category, 120)?;
    let description = required_text("description", &input.description, 6000)?;
    let notes = optional_text("notes", &input.notes, 6000)?;
    let source = optional_text("source", &input.source, 1000)?;

    if input.variables.len() > MAX_FORMULA_VARIABLES {
        return Err(AppError::validation(
            "variables",
            format!("A formula can have at most {MAX_FORMULA_VARIABLES} variables."),
        ));
    }
    let mut variables = Vec::with_capacity(input.variables.len());
    let mut seen_symbols = HashSet::new();
    for (index, variable) in input.variables.iter().enumerate() {
        let symbol = required_text("variables", &variable.symbol, 80)?;
        let meaning = required_text("variables", &variable.meaning, 500)?;
        let unit = optional_text("variables", &variable.unit, 120)?;
        if !seen_symbols.insert(symbol.to_lowercase()) {
            return Err(AppError::validation(
                "variables",
                format!(
                    "Variable {} repeats the symbol {symbol}. Each symbol must be unique.",
                    index + 1
                ),
            ));
        }
        variables.push(ValidatedVariable {
            symbol,
            meaning,
            unit,
        });
    }

    Ok(ValidatedFormula {
        name,
        latex,
        plain_text,
        category,
        description,
        notes,
        source,
        variables,
    })
}

fn formula_columns() -> &'static str {
    "id, is_builtin, name, latex, plain_text, category, description, notes, source, created_at, updated_at"
}

fn row_to_formula(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2CustomFormula> {
    Ok(V2CustomFormula {
        id: row.get(0)?,
        is_builtin: row.get::<_, i64>(1)? != 0,
        name: row.get(2)?,
        latex: row.get(3)?,
        plain_text: row.get(4)?,
        category: row.get(5)?,
        description: row.get(6)?,
        notes: row.get(7)?,
        source: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        variables: Vec::new(),
    })
}

fn load_formula_variables(
    connection: &Connection,
    formula_id: i64,
) -> Result<Vec<V2CustomFormulaVariable>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT id, position, symbol, meaning, unit
             FROM custom_formula_variables
             WHERE formula_id=?1
             ORDER BY position, id",
        )
        .map_err(|error| {
            AppError::database("The custom formula variables could not be prepared", error)
        })?;
    let rows = statement
        .query_map([formula_id], |row| {
            Ok(V2CustomFormulaVariable {
                id: row.get(0)?,
                position: row.get(1)?,
                symbol: row.get(2)?,
                meaning: row.get(3)?,
                unit: row.get(4)?,
            })
        })
        .map_err(|error| {
            AppError::database("The custom formula variables could not be read", error)
        })?;
    let mut variables = Vec::new();
    for row in rows {
        variables.push(row.map_err(|error| {
            AppError::database("A custom formula variable row is invalid", error)
        })?);
    }
    Ok(variables)
}

fn load_formula(connection: &Connection, id: i64) -> Result<Option<V2CustomFormula>, AppError> {
    let mut formula = connection
        .query_row(
            &format!(
                "SELECT {} FROM custom_formulas WHERE id=?1",
                formula_columns()
            ),
            [id],
            row_to_formula,
        )
        .optional()
        .map_err(|error| AppError::database("The custom formula could not be read", error))?;
    if let Some(formula) = formula.as_mut() {
        formula.variables = load_formula_variables(connection, formula.id)?;
    }
    Ok(formula)
}

pub fn list_custom_formulas(connection: &Connection) -> Result<Vec<V2CustomFormula>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM custom_formulas ORDER BY name COLLATE NOCASE, id",
            formula_columns()
        ))
        .map_err(|error| {
            AppError::database("The custom formula list could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], row_to_formula)
        .map_err(|error| AppError::database("The custom formula list could not be read", error))?;
    let mut formulas = Vec::new();
    for row in rows {
        let mut formula =
            row.map_err(|error| AppError::database("A custom formula row is invalid", error))?;
        formula.variables = load_formula_variables(connection, formula.id)?;
        formulas.push(formula);
    }
    Ok(formulas)
}

pub fn save_custom_formula(
    connection: &mut Connection,
    input: V2CustomFormulaInput,
    now: DateTime<Utc>,
) -> Result<V2CustomFormula, AppError> {
    let ValidatedFormula {
        name,
        latex,
        plain_text,
        category,
        description,
        notes,
        source,
        variables,
    } = validate_formula(&input)?;
    let token = now.to_rfc3339_opts(SecondsFormat::Millis, true);

    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom formula save could not start", error))?;

    let existing_id = input.id;
    if let Some(id) = existing_id {
        if id <= 0 {
            return Err(AppError::validation(
                "id",
                "The selected formula is invalid.",
            ));
        }
        let builtin: Option<i64> = transaction
            .query_row(
                "SELECT is_builtin FROM custom_formulas WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                AppError::database(
                    "The custom formula protection state could not be read",
                    error,
                )
            })?;
        match builtin {
            None => {
                return Err(AppError::validation(
                    "id",
                    "This custom formula no longer exists. Refresh the list and try again.",
                ));
            }
            Some(1) => {
                return Err(AppError::validation(
                    "id",
                    "Built-in formulas cannot be edited.",
                ));
            }
            Some(_) => {}
        }
    }

    let conflict: Option<i64> = transaction
        .query_row(
            "SELECT id FROM custom_formulas
             WHERE name=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)",
            params![name, existing_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The custom formula name could not be checked", error)
        })?;
    if conflict.is_some() {
        return Err(AppError::validation(
            "name",
            "A custom formula with this name already exists.",
        ));
    }

    let formula_id = match existing_id {
        Some(id) => {
            transaction
                .execute(
                    "UPDATE custom_formulas
                     SET name=?1, latex=?2, plain_text=?3, category=?4, description=?5,
                         notes=?6, source=?7, updated_at=?8
                     WHERE id=?9 AND is_builtin=0",
                    params![
                        name,
                        latex,
                        plain_text,
                        category,
                        description,
                        notes,
                        source,
                        token,
                        id
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom formula could not be updated", error)
                })?;
            id
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO custom_formulas
                     (is_builtin, name, latex, plain_text, category, description, notes, source,
                      created_at, updated_at)
                     VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
                    params![
                        name,
                        latex,
                        plain_text,
                        category,
                        description,
                        notes,
                        source,
                        token
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom formula could not be created", error)
                })?;
            transaction.last_insert_rowid()
        }
    };

    transaction
        .execute(
            "DELETE FROM custom_formula_variables WHERE formula_id=?1",
            [formula_id],
        )
        .map_err(|error| {
            AppError::database("The custom formula variables could not be replaced", error)
        })?;
    for (position, variable) in variables.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO custom_formula_variables
                 (formula_id, position, symbol, meaning, unit)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    formula_id,
                    position as i64,
                    variable.symbol,
                    variable.meaning,
                    variable.unit
                ],
            )
            .map_err(|error| {
                AppError::database("A custom formula variable could not be saved", error)
            })?;
    }

    let saved = load_formula(&transaction, formula_id)?.ok_or_else(|| {
        AppError::database(
            "The saved custom formula could not be read",
            "the saved row is missing",
        )
    })?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom formula save could not be committed", error)
    })?;
    Ok(saved)
}

pub fn delete_custom_formula(connection: &mut Connection, id: i64) -> Result<(), AppError> {
    if id <= 0 {
        return Err(AppError::validation(
            "id",
            "The selected formula is invalid.",
        ));
    }
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom formula delete could not start", error))?;
    let builtin: Option<i64> = transaction
        .query_row(
            "SELECT is_builtin FROM custom_formulas WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database(
                "The custom formula protection state could not be read",
                error,
            )
        })?;
    match builtin {
        None => {
            return Err(AppError::validation(
                "id",
                "This custom formula no longer exists. Refresh the list and try again.",
            ));
        }
        Some(1) => {
            return Err(AppError::validation(
                "id",
                "Built-in formulas cannot be deleted.",
            ));
        }
        Some(_) => {}
    }
    transaction
        .execute(
            "DELETE FROM custom_formulas WHERE id=?1 AND is_builtin=0",
            [id],
        )
        .map_err(|error| AppError::database("The custom formula could not be deleted", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom formula delete could not be committed", error)
    })?;
    Ok(())
}

#[derive(Debug)]
struct ValidatedUnit {
    name: String,
    symbol: String,
    quantity: String,
    dimension: String,
    si_classification: String,
    conversion_kind: String,
    conversion_factor: String,
    conversion_reference_unit: String,
    conversion_note: String,
    description: String,
    notes: String,
    source: String,
}

fn validate_unit(input: &V2CustomUnitInput) -> Result<ValidatedUnit, AppError> {
    let name = required_text("name", &input.name, 200)?;
    let symbol = required_text("symbol", &input.symbol, 80)?;
    let quantity = required_text("quantity", &input.quantity, 200)?;
    let dimension = optional_text("dimension", &input.dimension, 200)?;
    let si_classification = required_choice(
        "siClassification",
        &input.si_classification,
        &["si_base", "si_derived", "non_si"],
    )?;
    let conversion_kind = required_choice(
        "conversionKind",
        &input.conversion_kind,
        &["none", "scalar", "contextual"],
    )?;
    let conversion_factor = optional_text("conversionFactor", &input.conversion_factor, 200)?;
    let conversion_reference_unit = optional_text(
        "conversionReferenceUnit",
        &input.conversion_reference_unit,
        120,
    )?;
    let conversion_note = optional_text("conversionNote", &input.conversion_note, 2000)?;
    let description = required_text("description", &input.description, 6000)?;
    let notes = optional_text("notes", &input.notes, 6000)?;
    let source = optional_text("source", &input.source, 1000)?;

    match conversion_kind.as_str() {
        "none" => {
            if !conversion_factor.is_empty()
                || !conversion_reference_unit.is_empty()
                || !conversion_note.is_empty()
            {
                return Err(AppError::validation(
                    "conversionKind",
                    "Choose scalar or contextual conversion before entering conversion details.",
                ));
            }
        }
        "scalar" => {
            if conversion_factor.is_empty() {
                return Err(AppError::validation(
                    "conversionFactor",
                    "Enter the scalar conversion factor.",
                ));
            }
            finite_number("conversionFactor", &conversion_factor)?;
            if conversion_reference_unit.is_empty() {
                return Err(AppError::validation(
                    "conversionReferenceUnit",
                    "Enter the reference unit for this conversion.",
                ));
            }
        }
        "contextual" => {
            if !conversion_factor.is_empty() {
                return Err(AppError::validation(
                    "conversionFactor",
                    "Contextual conversions must not use a scalar factor.",
                ));
            }
            if conversion_note.is_empty() {
                return Err(AppError::validation(
                    "conversionNote",
                    "Describe the context required for this conversion.",
                ));
            }
        }
        _ => unreachable!("conversion kind was validated"),
    }

    Ok(ValidatedUnit {
        name,
        symbol,
        quantity,
        dimension,
        si_classification,
        conversion_kind,
        conversion_factor,
        conversion_reference_unit,
        conversion_note,
        description,
        notes,
        source,
    })
}

fn unit_columns() -> &'static str {
    "id, is_builtin, name, symbol, quantity, dimension, si_classification, conversion_kind,
     conversion_factor, conversion_reference_unit, conversion_note, description, notes, source,
     created_at, updated_at"
}

fn row_to_unit(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2CustomUnit> {
    Ok(V2CustomUnit {
        id: row.get(0)?,
        is_builtin: row.get::<_, i64>(1)? != 0,
        name: row.get(2)?,
        symbol: row.get(3)?,
        quantity: row.get(4)?,
        dimension: row.get(5)?,
        si_classification: row.get(6)?,
        conversion_kind: row.get(7)?,
        conversion_factor: row.get(8)?,
        conversion_reference_unit: row.get(9)?,
        conversion_note: row.get(10)?,
        description: row.get(11)?,
        notes: row.get(12)?,
        source: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

pub fn list_custom_units(connection: &Connection) -> Result<Vec<V2CustomUnit>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM custom_units ORDER BY name COLLATE NOCASE, id",
            unit_columns()
        ))
        .map_err(|error| AppError::database("The custom unit list could not be prepared", error))?;
    let rows = statement
        .query_map([], row_to_unit)
        .map_err(|error| AppError::database("The custom unit list could not be read", error))?;
    let mut units = Vec::new();
    for row in rows {
        units.push(row.map_err(|error| AppError::database("A custom unit row is invalid", error))?);
    }
    Ok(units)
}

pub fn save_custom_unit(
    connection: &mut Connection,
    input: V2CustomUnitInput,
    now: DateTime<Utc>,
) -> Result<V2CustomUnit, AppError> {
    let unit = validate_unit(&input)?;
    let token = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom unit save could not start", error))?;

    if let Some(id) = input.id {
        if id <= 0 {
            return Err(AppError::validation("id", "The selected unit is invalid."));
        }
        let builtin: Option<i64> = transaction
            .query_row(
                "SELECT is_builtin FROM custom_units WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                AppError::database("The custom unit protection state could not be read", error)
            })?;
        match builtin {
            None => {
                return Err(AppError::validation(
                    "id",
                    "This custom unit no longer exists. Refresh the list and try again.",
                ));
            }
            Some(1) => {
                return Err(AppError::validation(
                    "id",
                    "Built-in units cannot be edited.",
                ));
            }
            Some(_) => {}
        }
    }

    let conflict: Option<i64> = transaction
        .query_row(
            "SELECT id FROM custom_units
             WHERE name=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)",
            params![unit.name, input.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("The custom unit name could not be checked", error))?;
    if conflict.is_some() {
        return Err(AppError::validation(
            "name",
            "A custom unit with this name already exists.",
        ));
    }
    let symbol_conflict: Option<i64> = transaction
        .query_row(
            "SELECT id FROM custom_units
             WHERE symbol=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)",
            params![unit.symbol, input.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The custom unit symbol could not be checked", error)
        })?;
    if symbol_conflict.is_some() {
        return Err(AppError::validation(
            "symbol",
            "A custom unit with this symbol already exists.",
        ));
    }

    let unit_id = match input.id {
        Some(id) => {
            transaction
                .execute(
                    "UPDATE custom_units
                     SET name=?1, symbol=?2, quantity=?3, dimension=?4, si_classification=?5,
                         conversion_kind=?6, conversion_factor=?7,
                         conversion_reference_unit=?8, conversion_note=?9, description=?10,
                         notes=?11, source=?12, updated_at=?13
                     WHERE id=?14 AND is_builtin=0",
                    params![
                        unit.name,
                        unit.symbol,
                        unit.quantity,
                        unit.dimension,
                        unit.si_classification,
                        unit.conversion_kind,
                        unit.conversion_factor,
                        unit.conversion_reference_unit,
                        unit.conversion_note,
                        unit.description,
                        unit.notes,
                        unit.source,
                        token,
                        id
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom unit could not be updated", error)
                })?;
            id
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO custom_units
                     (is_builtin, name, symbol, quantity, dimension, si_classification,
                      conversion_kind, conversion_factor, conversion_reference_unit,
                      conversion_note, description, notes, source, created_at, updated_at)
                     VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
                    params![
                        unit.name,
                        unit.symbol,
                        unit.quantity,
                        unit.dimension,
                        unit.si_classification,
                        unit.conversion_kind,
                        unit.conversion_factor,
                        unit.conversion_reference_unit,
                        unit.conversion_note,
                        unit.description,
                        unit.notes,
                        unit.source,
                        token
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom unit could not be created", error)
                })?;
            transaction.last_insert_rowid()
        }
    };

    let saved = transaction
        .query_row(
            &format!("SELECT {} FROM custom_units WHERE id=?1", unit_columns()),
            [unit_id],
            row_to_unit,
        )
        .optional()
        .map_err(|error| AppError::database("The saved custom unit could not be read", error))?
        .ok_or_else(|| {
            AppError::database(
                "The saved custom unit could not be read",
                "the saved row is missing",
            )
        })?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom unit save could not be committed", error)
    })?;
    Ok(saved)
}

pub fn delete_custom_unit(connection: &mut Connection, id: i64) -> Result<(), AppError> {
    if id <= 0 {
        return Err(AppError::validation("id", "The selected unit is invalid."));
    }
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom unit delete could not start", error))?;
    let builtin: Option<i64> = transaction
        .query_row(
            "SELECT is_builtin FROM custom_units WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The custom unit protection state could not be read", error)
        })?;
    match builtin {
        None => {
            return Err(AppError::validation(
                "id",
                "This custom unit no longer exists. Refresh the list and try again.",
            ));
        }
        Some(1) => {
            return Err(AppError::validation(
                "id",
                "Built-in units cannot be deleted.",
            ));
        }
        Some(_) => {}
    }
    transaction
        .execute(
            "DELETE FROM custom_units WHERE id=?1 AND is_builtin=0",
            [id],
        )
        .map_err(|error| AppError::database("The custom unit could not be deleted", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom unit delete could not be committed", error)
    })?;
    Ok(())
}

#[derive(Debug)]
struct ValidatedConstant {
    name: String,
    symbol: String,
    value_text: String,
    unit: String,
    category: String,
    uncertainty_text: String,
    precision_text: String,
    source: String,
    description: String,
    notes: String,
}

fn validate_constant(input: &V2CustomConstantInput) -> Result<ValidatedConstant, AppError> {
    let name = required_text("name", &input.name, 300)?;
    let symbol = required_text("symbol", &input.symbol, 80)?;
    let value_text = verbatim_text("valueText", &input.value_text, 200)?;
    finite_number("valueText", &value_text)?;
    let unit = optional_text("unit", &input.unit, 120)?;
    let category = required_text("category", &input.category, 120)?;
    let uncertainty_text = optional_text("uncertaintyText", &input.uncertainty_text, 200)?;
    let precision_text = optional_text("precisionText", &input.precision_text, 120)?;
    let source = optional_text("source", &input.source, 1000)?;
    let description = required_text("description", &input.description, 6000)?;
    let notes = optional_text("notes", &input.notes, 6000)?;

    Ok(ValidatedConstant {
        name,
        symbol,
        value_text,
        unit,
        category,
        uncertainty_text,
        precision_text,
        source,
        description,
        notes,
    })
}

fn constant_columns() -> &'static str {
    "id, is_builtin, name, symbol, value_text, unit, category, uncertainty_text, precision_text,
     source, description, notes, created_at, updated_at"
}

fn row_to_constant(row: &rusqlite::Row<'_>) -> rusqlite::Result<V2CustomConstant> {
    Ok(V2CustomConstant {
        id: row.get(0)?,
        is_builtin: row.get::<_, i64>(1)? != 0,
        name: row.get(2)?,
        symbol: row.get(3)?,
        value_text: row.get(4)?,
        unit: row.get(5)?,
        category: row.get(6)?,
        uncertainty_text: row.get(7)?,
        precision_text: row.get(8)?,
        source: row.get(9)?,
        description: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn list_custom_constants(connection: &Connection) -> Result<Vec<V2CustomConstant>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM custom_constants ORDER BY name COLLATE NOCASE, id",
            constant_columns()
        ))
        .map_err(|error| {
            AppError::database("The custom constant list could not be prepared", error)
        })?;
    let rows = statement
        .query_map([], row_to_constant)
        .map_err(|error| AppError::database("The custom constant list could not be read", error))?;
    let mut constants = Vec::new();
    for row in rows {
        constants.push(
            row.map_err(|error| AppError::database("A custom constant row is invalid", error))?,
        );
    }
    Ok(constants)
}

pub fn save_custom_constant(
    connection: &mut Connection,
    input: V2CustomConstantInput,
    now: DateTime<Utc>,
) -> Result<V2CustomConstant, AppError> {
    let constant = validate_constant(&input)?;
    let token = now.to_rfc3339_opts(SecondsFormat::Millis, true);
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom constant save could not start", error))?;

    if let Some(id) = input.id {
        if id <= 0 {
            return Err(AppError::validation(
                "id",
                "The selected constant is invalid.",
            ));
        }
        let builtin: Option<i64> = transaction
            .query_row(
                "SELECT is_builtin FROM custom_constants WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                AppError::database(
                    "The custom constant protection state could not be read",
                    error,
                )
            })?;
        match builtin {
            None => {
                return Err(AppError::validation(
                    "id",
                    "This custom constant no longer exists. Refresh the list and try again.",
                ));
            }
            Some(1) => {
                return Err(AppError::validation(
                    "id",
                    "Built-in constants cannot be edited.",
                ));
            }
            Some(_) => {}
        }
    }

    let conflict: Option<i64> = transaction
        .query_row(
            "SELECT id FROM custom_constants
             WHERE name=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)",
            params![constant.name, input.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The custom constant name could not be checked", error)
        })?;
    if conflict.is_some() {
        return Err(AppError::validation(
            "name",
            "A custom constant with this name already exists.",
        ));
    }
    let symbol_conflict: Option<i64> = transaction
        .query_row(
            "SELECT id FROM custom_constants
             WHERE symbol=?1 COLLATE NOCASE AND (?2 IS NULL OR id<>?2)",
            params![constant.symbol, input.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database("The custom constant symbol could not be checked", error)
        })?;
    if symbol_conflict.is_some() {
        return Err(AppError::validation(
            "symbol",
            "A custom constant with this symbol already exists.",
        ));
    }

    let constant_id = match input.id {
        Some(id) => {
            transaction
                .execute(
                    "UPDATE custom_constants
                     SET name=?1, symbol=?2, value_text=?3, unit=?4, category=?5,
                         uncertainty_text=?6, precision_text=?7, source=?8, description=?9,
                         notes=?10, updated_at=?11
                     WHERE id=?12 AND is_builtin=0",
                    params![
                        constant.name,
                        constant.symbol,
                        constant.value_text,
                        constant.unit,
                        constant.category,
                        constant.uncertainty_text,
                        constant.precision_text,
                        constant.source,
                        constant.description,
                        constant.notes,
                        token,
                        id
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom constant could not be updated", error)
                })?;
            id
        }
        None => {
            transaction
                .execute(
                    "INSERT INTO custom_constants
                     (is_builtin, name, symbol, value_text, unit, category, uncertainty_text,
                      precision_text, source, description, notes, created_at, updated_at)
                     VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
                    params![
                        constant.name,
                        constant.symbol,
                        constant.value_text,
                        constant.unit,
                        constant.category,
                        constant.uncertainty_text,
                        constant.precision_text,
                        constant.source,
                        constant.description,
                        constant.notes,
                        token
                    ],
                )
                .map_err(|error| {
                    AppError::database("The custom constant could not be created", error)
                })?;
            transaction.last_insert_rowid()
        }
    };

    let saved = transaction
        .query_row(
            &format!(
                "SELECT {} FROM custom_constants WHERE id=?1",
                constant_columns()
            ),
            [constant_id],
            row_to_constant,
        )
        .optional()
        .map_err(|error| AppError::database("The saved custom constant could not be read", error))?
        .ok_or_else(|| {
            AppError::database(
                "The saved custom constant could not be read",
                "the saved row is missing",
            )
        })?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom constant save could not be committed", error)
    })?;
    Ok(saved)
}

pub fn delete_custom_constant(connection: &mut Connection, id: i64) -> Result<(), AppError> {
    if id <= 0 {
        return Err(AppError::validation(
            "id",
            "The selected constant is invalid.",
        ));
    }
    let transaction = connection
        .transaction()
        .map_err(|error| AppError::database("The custom constant delete could not start", error))?;
    let builtin: Option<i64> = transaction
        .query_row(
            "SELECT is_builtin FROM custom_constants WHERE id=?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            AppError::database(
                "The custom constant protection state could not be read",
                error,
            )
        })?;
    match builtin {
        None => {
            return Err(AppError::validation(
                "id",
                "This custom constant no longer exists. Refresh the list and try again.",
            ));
        }
        Some(1) => {
            return Err(AppError::validation(
                "id",
                "Built-in constants cannot be deleted.",
            ));
        }
        Some(_) => {}
    }
    transaction
        .execute(
            "DELETE FROM custom_constants WHERE id=?1 AND is_builtin=0",
            [id],
        )
        .map_err(|error| AppError::database("The custom constant could not be deleted", error))?;
    transaction.commit().map_err(|error| {
        AppError::database("The custom constant delete could not be committed", error)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::V2State;
    use chrono::TimeZone;

    fn at(minutes: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 10, 12, minutes as u32, 0)
            .single()
            .expect("test timestamp")
    }

    fn state() -> (tempfile::TempDir, V2State) {
        let root = tempfile::tempdir().expect("temporary directory");
        let state = V2State::initialize(root.path().join("v2").as_path());
        (root, state)
    }

    fn formula_input(name: &str) -> V2CustomFormulaInput {
        V2CustomFormulaInput {
            id: None,
            name: name.to_owned(),
            latex: String::new(),
            plain_text: String::new(),
            category: String::new(),
            description: String::new(),
            notes: String::new(),
            source: String::new(),
            variables: Vec::new(),
        }
    }

    fn valid_formula(name: &str) -> V2CustomFormulaInput {
        V2CustomFormulaInput {
            latex: r"E = mc^2".to_owned(),
            plain_text: "E = m c^2".to_owned(),
            category: "Special relativity".to_owned(),
            description: "Mass-energy equivalence for a body at rest.".to_owned(),
            variables: vec![
                V2CustomFormulaVariableInput {
                    symbol: "E".to_owned(),
                    meaning: "rest energy".to_owned(),
                    unit: "J".to_owned(),
                },
                V2CustomFormulaVariableInput {
                    symbol: "m".to_owned(),
                    meaning: "mass".to_owned(),
                    unit: "kg".to_owned(),
                },
            ],
            ..formula_input(name)
        }
    }

    #[test]
    fn formula_crud_replaces_variables_and_reports_duplicates() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let saved = save_custom_formula(connection, valid_formula("Mass-energy"), at(1))?;
                assert_eq!(saved.name, "Mass-energy");
                assert_eq!(saved.variables.len(), 2);
                assert_eq!(saved.variables[0].symbol, "E");
                assert!(!saved.is_builtin);

                let mut update = valid_formula("Mass-energy relation");
                update.id = Some(saved.id);
                update.variables = vec![V2CustomFormulaVariableInput {
                    symbol: "c".to_owned(),
                    meaning: "speed of light".to_owned(),
                    unit: "m s^-1".to_owned(),
                }];
                let updated = save_custom_formula(connection, update, at(2))?;
                assert_eq!(updated.id, saved.id);
                assert_eq!(updated.variables.len(), 1);
                assert_eq!(updated.variables[0].symbol, "c");

                let mut duplicate = valid_formula("mass-energy relation");
                duplicate.description = "Duplicate name check.".to_owned();
                let error = save_custom_formula(connection, duplicate, at(3)).unwrap_err();
                assert_eq!(error.code, "validation_error");
                assert_eq!(error.field, Some("name"));

                let listed = list_custom_formulas(connection)?;
                assert_eq!(listed.len(), 1);
                assert_eq!(listed[0].variables[0].symbol, "c");

                delete_custom_formula(connection, saved.id)?;
                assert!(list_custom_formulas(connection)?.is_empty());
                Ok::<(), AppError>(())
            })
            .expect("formula CRUD");
    }

    #[test]
    fn formula_update_rolls_back_when_a_later_write_fails() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let saved =
                    save_custom_formula(connection, valid_formula("Rollback formula"), at(1))?;
                connection
                    .execute_batch(
                        "CREATE TRIGGER test_formula_variable_failure
                         BEFORE INSERT ON custom_formula_variables
                         BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
                    )
                    .expect("failure trigger");

                let mut update = valid_formula("Changed formula");
                update.id = Some(saved.id);
                let error = save_custom_formula(connection, update, at(2)).unwrap_err();
                assert_eq!(error.code, "database_error");

                let stored = list_custom_formulas(connection)?;
                assert_eq!(stored.len(), 1);
                assert_eq!(stored[0].name, "Rollback formula");
                assert_eq!(stored[0].variables.len(), 2);
                Ok::<(), AppError>(())
            })
            .expect("rollback");
    }

    fn valid_unit(name: &str, symbol: &str) -> V2CustomUnitInput {
        V2CustomUnitInput {
            id: None,
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            quantity: "Length".to_owned(),
            dimension: "L".to_owned(),
            si_classification: "non_si".to_owned(),
            conversion_kind: "scalar".to_owned(),
            conversion_factor: "0.3048".to_owned(),
            conversion_reference_unit: "m".to_owned(),
            conversion_note: String::new(),
            description: "A unit of length.".to_owned(),
            notes: String::new(),
            source: String::new(),
        }
    }

    #[test]
    fn unit_crud_validates_conversion_kinds_and_duplicate_symbols() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let saved = save_custom_unit(connection, valid_unit("Foot", "ft"), at(1))?;
                assert_eq!(saved.conversion_kind, "scalar");
                assert_eq!(saved.conversion_factor, "0.3048");

                let mut contextual = valid_unit("Smoot", "smoot");
                contextual.conversion_kind = "contextual".to_owned();
                contextual.conversion_factor = String::new();
                contextual.conversion_reference_unit = String::new();
                contextual.conversion_note =
                    "Defined relative to the Harvard Bridge length.".to_owned();
                let contextual = save_custom_unit(connection, contextual, at(2))?;
                assert_eq!(contextual.conversion_kind, "contextual");

                let mut invalid_factor = valid_unit("Pace", "pace");
                invalid_factor.conversion_factor = "not-a-number".to_owned();
                let error = save_custom_unit(connection, invalid_factor, at(3)).unwrap_err();
                assert_eq!(error.field, Some("conversionFactor"));

                let mut no_note = valid_unit("Cubit", "cubit");
                no_note.conversion_kind = "contextual".to_owned();
                no_note.conversion_factor = String::new();
                no_note.conversion_reference_unit = String::new();
                let error = save_custom_unit(connection, no_note, at(4)).unwrap_err();
                assert_eq!(error.field, Some("conversionNote"));

                let mut none_with_details = valid_unit("Metre copy", "mcopy");
                none_with_details.conversion_kind = "none".to_owned();
                let error = save_custom_unit(connection, none_with_details, at(5)).unwrap_err();
                assert_eq!(error.field, Some("conversionKind"));

                let mut duplicate_symbol = valid_unit("Foot alias", "FT");
                duplicate_symbol.description = "Duplicate symbol check.".to_owned();
                let error = save_custom_unit(connection, duplicate_symbol, at(6)).unwrap_err();
                assert_eq!(error.field, Some("symbol"));

                let listed = list_custom_units(connection)?;
                assert_eq!(listed.len(), 2);
                delete_custom_unit(connection, saved.id)?;
                assert_eq!(list_custom_units(connection)?.len(), 1);
                Ok::<(), AppError>(())
            })
            .expect("unit CRUD");
    }

    fn valid_constant(name: &str, symbol: &str) -> V2CustomConstantInput {
        V2CustomConstantInput {
            id: None,
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            value_text: "6.62607015e-34".to_owned(),
            unit: "J s".to_owned(),
            category: "Fundamental".to_owned(),
            uncertainty_text: "exact".to_owned(),
            precision_text: "9 significant figures".to_owned(),
            source: "User reference".to_owned(),
            description: "A user-recorded fundamental constant.".to_owned(),
            notes: String::new(),
        }
    }

    #[test]
    fn constant_crud_preserves_value_text_and_optional_metadata() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                let mut input = valid_constant("Planck constant (local)", "h_local");
                input.value_text = "6.62607015×10^-34".to_owned();
                let saved = save_custom_constant(connection, input, at(1))?;
                assert_eq!(saved.value_text, "6.62607015×10^-34");
                assert_eq!(saved.uncertainty_text, "exact");
                assert_eq!(saved.precision_text, "9 significant figures");

                let mut no_metadata = valid_constant("Reduced Planck constant (local)", "ħ_local");
                no_metadata.value_text = "1.054571817e-34".to_owned();
                no_metadata.uncertainty_text = String::new();
                no_metadata.precision_text = String::new();
                let no_metadata = save_custom_constant(connection, no_metadata, at(2))?;
                assert_eq!(no_metadata.uncertainty_text, "");
                assert_eq!(no_metadata.precision_text, "");

                let mut invalid_value = valid_constant("Invalid value", "invalid");
                invalid_value.value_text = "not a number".to_owned();
                let error = save_custom_constant(connection, invalid_value, at(3)).unwrap_err();
                assert_eq!(error.field, Some("valueText"));

                let mut duplicate_name = valid_constant("planck constant (local)", "h_other");
                duplicate_name.value_text = "1".to_owned();
                let error = save_custom_constant(connection, duplicate_name, at(4)).unwrap_err();
                assert_eq!(error.field, Some("name"));

                let mut duplicate_symbol = valid_constant("Another local constant", "H_LOCAL");
                duplicate_symbol.value_text = "1".to_owned();
                let error = save_custom_constant(connection, duplicate_symbol, at(5)).unwrap_err();
                assert_eq!(error.field, Some("symbol"));

                assert_eq!(list_custom_constants(connection)?.len(), 2);
                delete_custom_constant(connection, saved.id)?;
                assert_eq!(list_custom_constants(connection)?.len(), 1);
                Ok::<(), AppError>(())
            })
            .expect("constant CRUD");
    }

    #[test]
    fn numeric_text_accepts_scientific_notation_without_changing_storage() {
        let valid: [(&str, f64); 23] = [
            ("6.62607015×10⁻³⁴", 6.62607015e-34),
            ("−6.62607015×10⁻³⁴", -6.62607015e-34),
            (".5×10⁻³", 5e-4),
            ("1.380649 × 10^-23", 1.380649e-23),
            ("2.99792458·10⁸", 2.99792458e8),
            ("2.99792458⋅10⁸", 2.99792458e8),
            ("6.626*10^-34", 6.626e-34),
            ("6.626x10^-34", 6.626e-34),
            ("6.626X10^+34", 6.626e34),
            ("6.626×10^-34", 6.626e-34),
            ("6.626×10^34", 6.626e34),
            ("6.626×10^⁻³⁴", 6.626e-34),
            ("6.626×10-34", 6.626e-34),
            ("6.626×10⁻34", 6.626e-34),
            ("6.626×10-³⁴", 6.626e-34),
            ("6.626×10⁺³⁴", 6.626e34),
            ("6.62607015e-34", 6.62607015e-34),
            ("6.626e⁻³⁴", 6.626e-34),
            ("6.626E-³⁴", 6.626e-34),
            ("-1.2E+3", -1.2e3),
            ("0", 0.0),
            ("1.", 1.0),
            ("+.5", 0.5),
        ];
        for (value, expected) in valid {
            let parsed = finite_number("valueText", value)
                .unwrap_or_else(|error| panic!("{value} should be accepted: {error:?}"));
            let tolerance = expected.abs() * 1e-12 + 1e-300;
            assert!(
                (parsed - expected).abs() <= tolerance,
                "{value} parsed as {parsed}, expected {expected}"
            );
        }

        for value in [
            "",
            "not a number",
            "1.2.3",
            "1e",
            "1e+",
            "1e2e3",
            "6.626e2³",
            "6.626E2.5",
            "NaN",
            "inf",
            "5×100",
            "6.626×10",
            "6.626×10^",
            "6.626×10^-",
            "6.626×10--34",
            "6.626×10^--34",
            "6.626×10^abc",
            "6.626×10^2.5",
            "6.626×10^2e3",
            "6.626×10^2³",
            "6.626×10^2⁴",
            "6.626×10³4",
            "6.626×10⁻3⁴",
            "6.626×10^2×10^3",
            "6.626×10³⁴extra",
            "abc×10^2",
            "1×10^2+3",
        ] {
            assert!(
                finite_number("valueText", value).is_err(),
                "{value} should be rejected"
            );
        }

        let (_root, state) = state();
        state
            .with_connection(|connection| {
                for (name, symbol, value) in [
                    ("Superscript constant", "sup", "6.62607015×10⁻³⁴"),
                    ("Spaced constant", "sp", "1.380649 × 10^-23"),
                    ("Dot constant", "dot", "2.99792458·10⁸"),
                    ("Negative constant", "neg", "−6.62607015×10⁻³⁴"),
                    ("Padded constant", "pad", " 6.62607015×10⁻³⁴ "),
                ] {
                    let saved = save_custom_constant(
                        connection,
                        V2CustomConstantInput {
                            id: None,
                            name: name.to_owned(),
                            symbol: symbol.to_owned(),
                            value_text: value.to_owned(),
                            unit: String::new(),
                            category: "Fundamental".to_owned(),
                            uncertainty_text: String::new(),
                            precision_text: String::new(),
                            source: String::new(),
                            description: "Scientific notation preservation.".to_owned(),
                            notes: String::new(),
                        },
                        at(1),
                    )?;
                    assert_eq!(saved.value_text, value);
                }

                let mut superscript_factor = valid_unit("Superscript unit", "supu");
                superscript_factor.conversion_factor = "3.048×10⁻¹".to_owned();
                let saved = save_custom_unit(connection, superscript_factor, at(2))?;
                assert_eq!(saved.conversion_factor, "3.048×10⁻¹");
                Ok::<(), AppError>(())
            })
            .expect("numeric notation storage");
    }

    #[test]
    fn built_in_rows_are_protected_at_the_backend() {
        let (_root, state) = state();
        state
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO custom_formulas
                         (is_builtin, name, latex, plain_text, category, description, notes, source,
                          created_at, updated_at)
                         VALUES (1, 'Protected formula', 'x', 'x', 'General', 'Protected.', '', '',
                                 '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                        [],
                    )
                    .expect("protected formula");
                let formula_id = connection.last_insert_rowid();
                let mut formula = valid_formula("Protected formula");
                formula.id = Some(formula_id);
                assert!(save_custom_formula(connection, formula, at(1)).is_err());
                assert!(delete_custom_formula(connection, formula_id).is_err());

                connection
                    .execute(
                        "INSERT INTO custom_units
                         (is_builtin, name, symbol, quantity, dimension, si_classification,
                          conversion_kind, conversion_factor, conversion_reference_unit,
                          conversion_note, description, notes, source, created_at, updated_at)
                         VALUES (1, 'Protected unit', 'prot', 'Length', 'L', 'non_si', 'none', '',
                                 '', '', 'Protected.', '', '', '2026-01-01T00:00:00Z',
                                 '2026-01-01T00:00:00Z')",
                        [],
                    )
                    .expect("protected unit");
                let unit_id = connection.last_insert_rowid();
                assert!(delete_custom_unit(connection, unit_id).is_err());

                connection
                    .execute(
                        "INSERT INTO custom_constants
                         (is_builtin, name, symbol, value_text, unit, category, uncertainty_text,
                          precision_text, source, description, notes, created_at, updated_at)
                         VALUES (1, 'Protected constant', 'pc', '1', '', 'Fundamental', '', '', '',
                                 'Protected.', '', '2026-01-01T00:00:00Z',
                                 '2026-01-01T00:00:00Z')",
                        [],
                    )
                    .expect("protected constant");
                let constant_id = connection.last_insert_rowid();
                assert!(delete_custom_constant(connection, constant_id).is_err());
                Ok::<(), AppError>(())
            })
            .expect("built-in protection");
    }
}
