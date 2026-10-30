-- Phase 10: import-batch identity for repeat-safe JSON merges.
--
-- The table stores only import provenance and result counts. It never stores a
-- copy of imported study records. Merge imports check this table before writing
-- so the same JSON export cannot be appended twice silently.

CREATE TABLE json_import_batches (
    id INTEGER PRIMARY KEY,
    batch_id TEXT NOT NULL UNIQUE
        CHECK (
            length(batch_id) = 32
            AND batch_id = lower(batch_id)
            AND batch_id NOT GLOB '*[^0-9a-f]*'
        ),
    source_database_id TEXT NOT NULL
        CHECK (
            length(source_database_id) = 32
            AND source_database_id = lower(source_database_id)
            AND source_database_id NOT GLOB '*[^0-9a-f]*'
        ),
    file_sha256 TEXT NOT NULL
        CHECK (
            length(file_sha256) = 64
            AND file_sha256 = lower(file_sha256)
            AND file_sha256 NOT GLOB '*[^0-9a-f]*'
        ),
    strategy TEXT NOT NULL
        CHECK (strategy IN ('merge', 'replace')),
    imported_at TEXT NOT NULL,
    imported_count INTEGER NOT NULL DEFAULT 0
        CHECK (imported_count >= 0),
    skipped_count INTEGER NOT NULL DEFAULT 0
        CHECK (skipped_count >= 0),
    conflicting_count INTEGER NOT NULL DEFAULT 0
        CHECK (conflicting_count >= 0),
    rejected_count INTEGER NOT NULL DEFAULT 0
        CHECK (rejected_count >= 0)
) STRICT;

CREATE INDEX idx_json_import_batches_imported_at
    ON json_import_batches(imported_at DESC, id DESC);
