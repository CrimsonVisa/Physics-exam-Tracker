-- Phase 11: study-resource catalogue, independent study tracking, and local PDFs.
--
-- Resource metadata is append/preserve oriented. A source refresh may mark a
-- resource removed or update its metadata, but never deletes the row or the
-- user's study progress.

CREATE TABLE study_resources (
    id INTEGER PRIMARY KEY,
    resource_key TEXT NOT NULL UNIQUE
        CHECK (
            length(resource_key) = 64
            AND resource_key = lower(resource_key)
            AND resource_key NOT GLOB '*[^0-9a-f]*'
        ),
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 300),
    description TEXT NOT NULL DEFAULT ''
        CHECK (length(description) <= 2000),
    category TEXT NOT NULL
        CHECK (
            category IN (
                'handwritten_notes',
                'classroom_notes',
                'assignments',
                'formula_sheets',
                'previous_year_papers',
                'important_topics',
                'recommended_books',
                'nptel_courses',
                'youtube_playlists',
                'other'
            )
        ),
    unit TEXT NOT NULL DEFAULT 'Unmapped'
        CHECK (
            unit IN (
                'General Aptitude',
                'Mathematical Methods of Physics',
                'Classical Mechanics',
                'Electromagnetic Theory',
                'Quantum Mechanics',
                'Thermodynamic and Statistical Physics',
                'Electronics and Experimental Methods',
                'Atomic & Molecular Physics',
                'Condensed Matter Physics',
                'Nuclear and Particle Physics',
                'All Units',
                'Unmapped'
            )
        ),
    topic_code TEXT,
    source_page_url TEXT,
    original_url TEXT NOT NULL,
    external_id TEXT,
    external_kind TEXT NOT NULL
        CHECK (
            external_kind IN (
                'drive_file',
                'drive_folder',
                'nptel_course',
                'youtube_playlist',
                'web_page'
            )
        ),
    mime_type TEXT,
    availability TEXT NOT NULL DEFAULT 'unverified'
        CHECK (availability IN ('online', 'removed', 'unverified', 'invalid')),
    discovered_at TEXT NOT NULL,
    last_seen_at TEXT,
    last_verified_at TEXT,
    removed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (removed_at IS NULL AND availability != 'removed')
        OR (removed_at IS NOT NULL AND availability = 'removed')
    )
) STRICT;

CREATE INDEX idx_study_resources_category
    ON study_resources(category, title);
CREATE INDEX idx_study_resources_unit
    ON study_resources(unit, category);
CREATE INDEX idx_study_resources_topic
    ON study_resources(topic_code);
CREATE INDEX idx_study_resources_source
    ON study_resources(source_page_url);
CREATE INDEX idx_study_resources_availability
    ON study_resources(availability);
CREATE INDEX idx_study_resources_title
    ON study_resources(title COLLATE NOCASE);

CREATE TABLE resource_progress (
    resource_id INTEGER PRIMARY KEY
        REFERENCES study_resources(id) ON DELETE CASCADE,
    study_status TEXT NOT NULL DEFAULT 'not_started'
        CHECK (study_status IN ('not_started', 'studying', 'completed', 'skipped')),
    personal_notes TEXT NOT NULL DEFAULT ''
        CHECK (length(personal_notes) <= 4000),
    last_opened_at TEXT,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE resource_local_files (
    resource_id INTEGER PRIMARY KEY
        REFERENCES study_resources(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL
        CHECK (length(trim(file_name)) BETWEEN 1 AND 260),
    file_path TEXT NOT NULL UNIQUE
        CHECK (length(trim(file_path)) BETWEEN 1 AND 1000),
    sha256 TEXT NOT NULL
        CHECK (
            length(sha256) = 64
            AND sha256 = lower(sha256)
            AND sha256 NOT GLOB '*[^0-9a-f]*'
        ),
    size_bytes INTEGER NOT NULL
        CHECK (size_bytes > 0),
    imported_at TEXT NOT NULL
) STRICT;

CREATE TABLE resource_discovery_runs (
    id INTEGER PRIMARY KEY,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL
        CHECK (status IN ('running', 'success', 'partial', 'blocked', 'failed')),
    pages_ok INTEGER NOT NULL DEFAULT 0
        CHECK (pages_ok >= 0),
    pages_failed INTEGER NOT NULL DEFAULT 0
        CHECK (pages_failed >= 0),
    resources_seen INTEGER NOT NULL DEFAULT 0
        CHECK (resources_seen >= 0),
    resources_added INTEGER NOT NULL DEFAULT 0
        CHECK (resources_added >= 0),
    resources_updated INTEGER NOT NULL DEFAULT 0
        CHECK (resources_updated >= 0),
    resources_removed INTEGER NOT NULL DEFAULT 0
        CHECK (resources_removed >= 0),
    message TEXT NOT NULL DEFAULT ''
        CHECK (length(message) <= 2000)
) STRICT;

CREATE INDEX idx_resource_discovery_runs_finished
    ON resource_discovery_runs(finished_at DESC, id DESC);
