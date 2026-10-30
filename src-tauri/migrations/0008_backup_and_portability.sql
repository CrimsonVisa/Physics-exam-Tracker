-- Phase 9: backup configuration, database identity, and automatic-backup state.
--
-- Backup contents themselves are never copied into the database. This migration
-- stores only configuration and last-result metadata. database_metadata is
-- populated by Rust after a successful migration so a pre-migration safety
-- backup can carry the same identifier.

CREATE TABLE database_metadata (
    id INTEGER PRIMARY KEY
        CHECK (id = 1),
    database_id TEXT NOT NULL UNIQUE
        CHECK (
            length(database_id) = 32
            AND database_id = lower(database_id)
            AND database_id NOT GLOB '*[^0-9a-f]*'
        ),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE backup_settings (
    id INTEGER PRIMARY KEY
        CHECK (id = 1),
    automatic_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (automatic_enabled IN (0, 1)),
    frequency TEXT NOT NULL DEFAULT 'daily'
        CHECK (frequency IN ('daily', 'weekly', 'before_important_operations')),
    destination TEXT
        CHECK (destination IS NULL OR length(destination) BETWEEN 1 AND 2048),
    retention_count INTEGER NOT NULL DEFAULT 5
        CHECK (retention_count BETWEEN 1 AND 100),
    last_success_at TEXT,
    last_success_file TEXT
        CHECK (last_success_file IS NULL OR length(last_success_file) <= 300),
    last_failure_at TEXT,
    last_failure_message TEXT
        CHECK (last_failure_message IS NULL OR length(last_failure_message) <= 1000),
    updated_at TEXT NOT NULL
) STRICT;

INSERT INTO backup_settings (
    id,
    automatic_enabled,
    frequency,
    destination,
    retention_count,
    last_success_at,
    last_success_file,
    last_failure_at,
    last_failure_message,
    updated_at
) VALUES (
    1,
    0,
    'daily',
    NULL,
    5,
    NULL,
    NULL,
    NULL,
    NULL,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);
