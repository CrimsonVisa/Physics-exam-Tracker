use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::error::AppError;

pub const MAX_CSV_BYTES: usize = 5_000_000;
pub const MAX_CSV_ROWS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFieldError {
    pub field: String,
    pub message: String,
}

impl ImportFieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportRowStatus {
    Valid,
    Duplicate,
    Invalid,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRow {
    pub row_number: usize,
    pub status: ImportRowStatus,
    pub summary: String,
    pub errors: Vec<ImportFieldError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub total_rows: i64,
    pub valid_rows: i64,
    pub duplicate_rows: i64,
    pub invalid_rows: i64,
    pub rows: Vec<ImportRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: i64,
    pub skipped: i64,
    pub rejected: i64,
    pub rows: Vec<ImportRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRecord {
    pub line: usize,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<CsvRecord>,
}

/// Parse a strict comma-separated document. Quoted fields may contain commas,
/// doubled quotes, and embedded newlines. A UTF-8 BOM is accepted at the start.
pub fn parse_csv(input: &str) -> Result<Vec<CsvRecord>, AppError> {
    if input.len() > MAX_CSV_BYTES {
        return Err(AppError::validation(
            "file",
            format!(
                "CSV files must be {} MB or smaller.",
                MAX_CSV_BYTES / 1_000_000
            ),
        ));
    }

    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    if input.trim().is_empty() {
        return Err(AppError::validation("file", "The CSV file is empty."));
    }

    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut closed_quote = false;
    let mut line = 1usize;
    let mut row_start_line = 1usize;
    let mut chars = input.chars().peekable();

    while let Some(character) = chars.next() {
        if in_quotes {
            match character {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                        closed_quote = true;
                    }
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    field.push('\n');
                    line += 1;
                }
                '\n' => {
                    field.push('\n');
                    line += 1;
                }
                _ => field.push(character),
            }

            continue;
        }

        match character {
            '"' if !field.is_empty() => {
                return Err(AppError::validation(
                    "file",
                    format!("Malformed CSV at line {line}: a quote must begin a field."),
                ));
            }
            '"' if closed_quote => {
                return Err(AppError::validation(
                    "file",
                    format!("Malformed CSV at line {line}: unexpected text after a closing quote."),
                ));
            }
            '"' => {
                in_quotes = true;
            }
            ',' => {
                fields.push(std::mem::take(&mut field));
                closed_quote = false;
            }
            '\r' | '\n' => {
                if character == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                fields.push(std::mem::take(&mut field));
                records.push(CsvRecord {
                    line: row_start_line,
                    fields: std::mem::take(&mut fields),
                });
                line += 1;
                row_start_line = line;
                closed_quote = false;
            }
            _ if closed_quote => {
                return Err(AppError::validation(
                    "file",
                    format!("Malformed CSV at line {line}: unexpected text after a closing quote."),
                ));
            }
            _ => field.push(character),
        }
    }

    if in_quotes {
        return Err(AppError::validation(
            "file",
            format!("Malformed CSV at line {row_start_line}: a quoted field is not closed."),
        ));
    }

    if !fields.is_empty() || !field.is_empty() || closed_quote {
        fields.push(field);
        records.push(CsvRecord {
            line: row_start_line,
            fields,
        });
    }

    Ok(records)
}

pub fn read_table(input: &str, required_headers: &[&str]) -> Result<CsvTable, AppError> {
    let records = parse_csv(input)?;
    let header_record = records
        .iter()
        .find(|record| !is_blank_record(record))
        .ok_or_else(|| AppError::validation("file", "The CSV file has no header row."))?;

    let headers = header_record
        .fields
        .iter()
        .map(|header| header.trim().to_lowercase())
        .collect::<Vec<_>>();

    if headers.iter().any(String::is_empty) {
        return Err(AppError::validation(
            "file",
            "Every CSV column must have a header name.",
        ));
    }

    let mut seen = HashSet::new();
    for header in &headers {
        if !seen.insert(header.as_str()) {
            return Err(AppError::validation(
                "file",
                format!("The CSV header “{header}” appears more than once."),
            ));
        }
    }

    let required: HashSet<String> = required_headers
        .iter()
        .map(|header| header.to_lowercase())
        .collect::<HashSet<_>>();
    let provided: HashSet<&str> = headers.iter().map(String::as_str).collect();
    let missing = required
        .iter()
        .filter(|header| !provided.contains(header.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let unknown = provided
        .iter()
        .filter(|header| !required.contains(**header))
        .copied()
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        let mut missing = missing;
        missing.sort_unstable();
        return Err(AppError::validation(
            "file",
            format!("Missing required CSV columns: {}.", missing.join(", ")),
        ));
    }

    if !unknown.is_empty() {
        let mut unknown = unknown;
        unknown.sort_unstable();
        return Err(AppError::validation(
            "file",
            format!(
                "Unknown CSV columns: {}. Use the downloadable template unchanged.",
                unknown.join(", ")
            ),
        ));
    }

    let header_line = header_record.line;
    let rows = records
        .into_iter()
        .filter(|record| record.line != header_line && !is_blank_record(record))
        .collect::<Vec<_>>();

    if rows.len() > MAX_CSV_ROWS {
        return Err(AppError::validation(
            "file",
            format!("A CSV import may contain at most {MAX_CSV_ROWS} data rows."),
        ));
    }

    Ok(CsvTable { headers, rows })
}

fn is_blank_record(record: &CsvRecord) -> bool {
    record.fields.iter().all(|field| field.trim().is_empty())
}

pub fn values_by_header<'a>(
    table: &'a CsvTable,
    record: &'a CsvRecord,
) -> Result<HashMap<&'a str, &'a str>, ImportFieldError> {
    if record.fields.len() != table.headers.len() {
        return Err(ImportFieldError::new(
            "row",
            format!(
                "Expected {} columns but found {}.",
                table.headers.len(),
                record.fields.len()
            ),
        ));
    }

    Ok(table
        .headers
        .iter()
        .map(String::as_str)
        .zip(record.fields.iter().map(String::as_str))
        .collect())
}

pub fn stable_import_key(prefix: &str, parts: &[&str]) -> String {
    // FNV-1a keeps duplicate detection stable across runs and app restarts.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0x1f;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{prefix}:{hash:016x}")
}

pub fn csv_template(headers: &[&str]) -> String {
    format!("{}\r\n", headers.join(","))
}

pub fn preview_from_rows(rows: Vec<ImportRow>) -> ImportPreview {
    let total_rows = rows.len() as i64;
    let valid_rows = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Valid)
        .count() as i64;
    let duplicate_rows = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Duplicate)
        .count() as i64;
    let invalid_rows = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Invalid)
        .count() as i64;

    ImportPreview {
        total_rows,
        valid_rows,
        duplicate_rows,
        invalid_rows,
        rows,
    }
}

pub fn result_from_rows(rows: Vec<ImportRow>) -> ImportResult {
    let imported = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Imported)
        .count() as i64;
    let skipped = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Duplicate)
        .count() as i64;
    let rejected = rows
        .iter()
        .filter(|row| row.status == ImportRowStatus::Invalid)
        .count() as i64;

    ImportResult {
        imported,
        skipped,
        rejected,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utf8_bom_quoted_commas_quotes_and_newlines() {
        let csv = "\u{feff}a,b,c\r\n1,\"two, three\",\"line one\nline two\"\r\n4,\"say \"\"hi\"\"\",6\r\n";
        let records = parse_csv(csv).expect("CSV should parse");

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].fields, ["a", "b", "c"]);
        assert_eq!(records[1].fields[0], "1");
        assert_eq!(records[1].fields[1], "two, three");
        assert_eq!(records[1].fields[2], "line one\nline two");
        assert_eq!(records[2].fields[1], "say \"hi\"");
        assert_eq!(records[2].line, 4);
    }

    #[test]
    fn preserves_empty_fields_and_ignores_blank_rows_in_tables() {
        let records = parse_csv("a,b,c\n,,\n\n1,,3\n").expect("CSV should parse");

        assert_eq!(records.len(), 4);
        assert_eq!(records[1].fields, ["", "", ""]);
        assert_eq!(records[2].fields, [""]);
        assert_eq!(records[3].fields, ["1", "", "3"]);

        let table = read_table("a,b,c\n,,\n\n1,,3\n", &["a", "b", "c"]).expect("table should load");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].fields, ["1", "", "3"]);
    }

    #[test]
    fn rejects_malformed_quoted_fields() {
        let unclosed = parse_csv("a,b\n1,\"open\n").expect_err("unclosed quote should fail");
        assert_eq!(unclosed.field, Some("file"));

        let embedded = parse_csv("a,b\n1,pre\"quote\"\n").expect_err("embedded quote should fail");
        assert_eq!(embedded.field, Some("file"));

        let trailing = parse_csv("a,b\n1,\"quote\"tail\n").expect_err("trailing text should fail");
        assert_eq!(trailing.field, Some("file"));
    }

    #[test]
    fn validates_headers_and_row_widths() {
        let table = read_table(
            "year,part,status\n2024,B,correct\n",
            &["year", "part", "status"],
        )
        .expect("table should load");
        assert_eq!(table.headers, ["year", "part", "status"]);
        assert_eq!(table.rows.len(), 1);

        let missing = read_table("year,part\n2024,B\n", &["year", "part", "status"])
            .expect_err("missing header should fail");
        assert!(missing.message.contains("status"));

        let unknown = read_table(
            "year,part,status,extra\n2024,B,correct,x\n",
            &["year", "part", "status"],
        )
        .expect_err("unknown header should fail");
        assert!(unknown.message.contains("extra"));

        let duplicate = read_table("year,part,part\n2024,B,C\n", &["year", "part"])
            .expect_err("duplicate header should fail");
        assert!(duplicate.message.contains("more than once"));

        let row = CsvRecord {
            line: 2,
            fields: vec!["2024".to_owned()],
        };
        let error = values_by_header(&table, &row).expect_err("short row should fail");
        assert_eq!(error.field, "row");
    }

    #[test]
    fn counts_preview_and_result_statuses() {
        let rows = vec![
            ImportRow {
                row_number: 2,
                status: ImportRowStatus::Valid,
                summary: "one".to_owned(),
                errors: vec![],
            },
            ImportRow {
                row_number: 3,
                status: ImportRowStatus::Duplicate,
                summary: "two".to_owned(),
                errors: vec![ImportFieldError::new("duplicate", "already imported")],
            },
            ImportRow {
                row_number: 4,
                status: ImportRowStatus::Invalid,
                summary: String::new(),
                errors: vec![ImportFieldError::new("part", "bad part")],
            },
        ];

        let preview = preview_from_rows(rows);
        assert_eq!(preview.total_rows, 3);
        assert_eq!(preview.valid_rows, 1);
        assert_eq!(preview.duplicate_rows, 1);
        assert_eq!(preview.invalid_rows, 1);

        let result = result_from_rows(vec![
            ImportRow {
                row_number: 2,
                status: ImportRowStatus::Imported,
                summary: "one".to_owned(),
                errors: vec![],
            },
            ImportRow {
                row_number: 3,
                status: ImportRowStatus::Duplicate,
                summary: "two".to_owned(),
                errors: vec![],
            },
            ImportRow {
                row_number: 4,
                status: ImportRowStatus::Invalid,
                summary: String::new(),
                errors: vec![],
            },
        ]);
        assert_eq!(
            (result.imported, result.skipped, result.rejected),
            (1, 1, 1)
        );
    }

    #[test]
    fn stable_keys_are_deterministic_and_distinct() {
        let first = stable_import_key("pyq", &["2024", "B", "Q1"]);
        let second = stable_import_key("pyq", &["2024", "B", "Q1"]);
        let third = stable_import_key("pyq", &["2024", "B", "Q2"]);

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert!(first.starts_with("pyq:"));
    }
}
