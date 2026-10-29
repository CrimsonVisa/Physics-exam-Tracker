-- Phase 2: application-level study planning settings.
-- Date-only values are stored as YYYY-MM-DD strings. The CHECK constraints
-- mirror the authoritative Rust validation as defense in depth.
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    exam_date TEXT NOT NULL
        CHECK (
            length(exam_date) = 10
            AND exam_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND exam_date BETWEEN '0001-01-01' AND '9999-12-31'
            AND date(exam_date) IS exam_date
        ),
    preparation_start_date TEXT NOT NULL
        CHECK (
            length(preparation_start_date) = 10
            AND preparation_start_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND preparation_start_date BETWEEN '0001-01-01' AND '9999-12-31'
            AND date(preparation_start_date) IS preparation_start_date
        ),
    daily_study_target_minutes INTEGER NOT NULL
        CHECK (daily_study_target_minutes BETWEEN 1 AND 1440),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (preparation_start_date <= exam_date)
) STRICT;
