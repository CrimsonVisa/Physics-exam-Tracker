-- Phase 3: official-source syllabus catalog and per-topic progress.
-- Catalog rows are seeded idempotently from Rust. User progress is stored in a
-- separate table so catalog metadata updates never overwrite study records.

CREATE TABLE syllabus_sources (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE
        CHECK (length(code) BETWEEN 1 AND 64),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 300),
    publisher TEXT NOT NULL
        CHECK (length(trim(publisher)) BETWEEN 1 AND 200),
    url TEXT NOT NULL UNIQUE
        CHECK (url LIKE 'https://%' AND length(url) <= 2048),
    landing_url TEXT
        CHECK (
            landing_url IS NULL
            OR (landing_url LIKE 'https://%' AND length(landing_url) <= 2048)
        ),
    verified_on TEXT NOT NULL
        CHECK (
            length(verified_on) = 10
            AND verified_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(verified_on) IS verified_on
        ),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE syllabus_sections (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE
        CHECK (length(code) BETWEEN 1 AND 32),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 200),
    source_id INTEGER NOT NULL
        REFERENCES syllabus_sources(id) ON DELETE RESTRICT,
    source_reference TEXT NOT NULL DEFAULT ''
        CHECK (length(source_reference) <= 200),
    display_order INTEGER NOT NULL
        CHECK (display_order >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE syllabus_topics (
    id INTEGER PRIMARY KEY,
    section_id INTEGER NOT NULL
        REFERENCES syllabus_sections(id) ON DELETE RESTRICT,
    code TEXT NOT NULL UNIQUE
        CHECK (length(code) BETWEEN 1 AND 64),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 300),
    category TEXT NOT NULL
        CHECK (category IN ('General Aptitude', 'Core', 'Advanced')),
    source_id INTEGER NOT NULL
        REFERENCES syllabus_sources(id) ON DELETE RESTRICT,
    source_reference TEXT NOT NULL DEFAULT ''
        CHECK (length(source_reference) <= 200),
    display_order INTEGER NOT NULL
        CHECK (display_order >= 0),
    is_archived INTEGER NOT NULL DEFAULT 0
        CHECK (is_archived IN (0, 1)),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (is_archived = 0 AND archived_at IS NULL)
        OR (is_archived = 1 AND archived_at IS NOT NULL)
    )
) STRICT;

CREATE TABLE syllabus_progress (
    topic_id INTEGER PRIMARY KEY
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'Not started'
        CHECK (status IN ('Not started', 'Learning', 'Revised', 'Confident')),
    priority TEXT NOT NULL DEFAULT 'Normal'
        CHECK (priority IN ('Low', 'Normal', 'High', 'High-Yield')),
    confidence INTEGER NOT NULL DEFAULT 3
        CHECK (confidence BETWEEN 1 AND 5),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

-- Active-topic listings and filters use these indexes. Archived topics are kept
-- out of the partial indexes because they are intentionally hidden from study
-- views while their progress rows remain available for continuity.
CREATE INDEX idx_syllabus_sections_order
    ON syllabus_sections(display_order);

CREATE INDEX idx_syllabus_topics_section_order
    ON syllabus_topics(section_id, display_order)
    WHERE is_archived = 0;

CREATE INDEX idx_syllabus_topics_category_order
    ON syllabus_topics(category, display_order)
    WHERE is_archived = 0;

CREATE INDEX idx_syllabus_progress_status
    ON syllabus_progress(status);

CREATE INDEX idx_syllabus_progress_priority
    ON syllabus_progress(priority);
