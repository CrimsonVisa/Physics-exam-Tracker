-- V2 schema upgrade: version 1 -> 2 (QoL-2 quick-start configurations).
--
-- Applied in place by open_or_initialize when an existing database carries
-- user_version 1. Fresh databases receive these tables directly from
-- schema.sql, so every statement here is idempotent. The versioned
-- schema_migrations ledger row and the app_metadata schema version are written
-- by the upgrade routine after this batch succeeds.

CREATE TABLE IF NOT EXISTS quick_start_configs (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
        CHECK (length(trim(name)) BETWEEN 1 AND 80),
    duration_minutes INTEGER NOT NULL
        CHECK (duration_minutes BETWEEN 1 AND 240),
    is_pinned INTEGER NOT NULL DEFAULT 0
        CHECK (is_pinned IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS quick_start_config_allocations (
    id INTEGER PRIMARY KEY,
    config_id INTEGER NOT NULL
        REFERENCES quick_start_configs(id) ON DELETE CASCADE,
    exam_id INTEGER NOT NULL
        REFERENCES exams(id) ON DELETE CASCADE,
    syllabus_node_id INTEGER NOT NULL
        REFERENCES exam_syllabus_nodes(id) ON DELETE CASCADE,
    allocated_minutes INTEGER NOT NULL
        CHECK (allocated_minutes >= 1),
    position INTEGER NOT NULL DEFAULT 0
        CHECK (position >= 0)
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS quick_start_config_allocations_unique
    ON quick_start_config_allocations (config_id, exam_id, syllabus_node_id);

CREATE INDEX IF NOT EXISTS quick_start_config_allocations_config
    ON quick_start_config_allocations (config_id, position);
