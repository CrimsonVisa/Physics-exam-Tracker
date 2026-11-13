# Historical Schema Validation — V2.0 Specification V1.3.2

**Status: historical.** The specification and Python validator target the original schema version 1.
The current active schema is version 5 at `src-tauri/src/v2/schema.sql`. These files are retained as
audit evidence and are not run in CI. Do not use this tool to validate a current release. A current
validator, if needed, should be generated from the active schema and added with dedicated tests in a
separate change.

## Files

- `Physics_Tracker_V2.0_Specification_V1.3.2.md` — original specification, including the embedded
  schema-version-1 SQL blocks.
- `Physics_Tracker_V2.0_Schema_Validation.py` — standalone validator that extracts those SQL blocks
  into a temporary SQLite database. It does not open or modify application data.

Both files are preserved unchanged so the historical schema-version-1 design and its validation
matrix remain auditable.
