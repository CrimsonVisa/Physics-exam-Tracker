-- V2 schema upgrade: version 4 -> 5.
--
-- Adds the immutable built-in constants catalogue and persistent sticky
-- notes. Fresh databases receive the same objects from schema.sql, so every
-- statement here is idempotent. The versioned schema_migrations ledger row and
-- the app_metadata schema version are written by the upgrade routine after
-- this batch succeeds. No existing table or row is altered or dropped.

CREATE TABLE IF NOT EXISTS constant_catalog_versions (
    id INTEGER PRIMARY KEY,
    version_label TEXT NOT NULL UNIQUE
        CHECK (length(trim(version_label)) BETWEEN 1 AND 100),
    edition_year INTEGER NOT NULL CHECK (edition_year BETWEEN 1900 AND 2200),
    published_on TEXT NOT NULL
        CHECK (length(published_on) = 10 AND date(published_on) = published_on),
    retrieved_on TEXT NOT NULL
        CHECK (length(retrieved_on) = 10 AND date(retrieved_on) = retrieved_on),
    publisher TEXT NOT NULL CHECK (length(trim(publisher)) BETWEEN 1 AND 200),
    source_url TEXT NOT NULL CHECK (source_url LIKE 'https://%'),
    citation TEXT NOT NULL CHECK (length(trim(citation)) BETWEEN 1 AND 1000),
    seed_sha256 TEXT NOT NULL CHECK (
        length(seed_sha256) = 64
        AND seed_sha256 = lower(seed_sha256)
        AND seed_sha256 NOT GLOB '*[^0-9a-f]*'
    ),
    is_active INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_constant_catalog_one_active
    ON constant_catalog_versions(is_active)
    WHERE is_active = 1;

CREATE TABLE IF NOT EXISTS builtin_constants (
    id INTEGER PRIMARY KEY,
    catalog_version_id INTEGER NOT NULL
        REFERENCES constant_catalog_versions(id) ON DELETE RESTRICT,
    stable_key TEXT NOT NULL CHECK (length(trim(stable_key)) BETWEEN 1 AND 120),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 300),
    symbol TEXT NOT NULL CHECK (length(trim(symbol)) BETWEEN 1 AND 80),
    value_text TEXT NOT NULL CHECK (length(trim(value_text)) BETWEEN 1 AND 200),
    unit TEXT NOT NULL DEFAULT '' CHECK (length(unit) <= 120),
    uncertainty_text TEXT NOT NULL DEFAULT '' CHECK (length(uncertainty_text) <= 200),
    precision_text TEXT NOT NULL DEFAULT '' CHECK (length(precision_text) <= 120),
    is_exact INTEGER NOT NULL DEFAULT 0 CHECK (is_exact IN (0, 1)),
    category TEXT NOT NULL CHECK (category IN (
        'Fundamental',
        'Electromagnetic',
        'Atomic and nuclear',
        'Thermodynamic',
        'Astrophysical'
    )),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 6000),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 6000),
    source_locator TEXT NOT NULL CHECK (length(trim(source_locator)) BETWEEN 1 AND 300),
    source_reference TEXT NOT NULL CHECK (length(trim(source_reference)) BETWEEN 1 AND 1000),
    source_url TEXT NOT NULL CHECK (source_url LIKE 'https://%'),
    display_order INTEGER NOT NULL CHECK (display_order >= 0),
    created_at TEXT NOT NULL,
    UNIQUE (catalog_version_id, stable_key),
    UNIQUE (catalog_version_id, display_order)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_builtin_constants_category
    ON builtin_constants(catalog_version_id, category, display_order);

CREATE TRIGGER IF NOT EXISTS trg_builtin_constants_no_update
BEFORE UPDATE ON builtin_constants
BEGIN SELECT RAISE(ABORT, 'built-in constants are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_builtin_constants_no_delete
BEFORE DELETE ON builtin_constants
BEGIN SELECT RAISE(ABORT, 'built-in constants are protected'); END;

CREATE TRIGGER IF NOT EXISTS trg_constant_catalog_no_delete
BEFORE DELETE ON constant_catalog_versions
BEGIN SELECT RAISE(ABORT, 'constant catalogue versions are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_constant_catalog_no_metadata_update
BEFORE UPDATE ON constant_catalog_versions
WHEN NEW.id <> OLD.id
  OR NEW.version_label <> OLD.version_label
  OR NEW.edition_year <> OLD.edition_year
  OR NEW.published_on <> OLD.published_on
  OR NEW.retrieved_on <> OLD.retrieved_on
  OR NEW.publisher <> OLD.publisher
  OR NEW.source_url <> OLD.source_url
  OR NEW.citation <> OLD.citation
  OR NEW.seed_sha256 <> OLD.seed_sha256
  OR NEW.created_at <> OLD.created_at
BEGIN SELECT RAISE(ABORT, 'constant catalogue metadata is immutable'); END;

CREATE TABLE IF NOT EXISTS sticky_notes (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL DEFAULT '' CHECK (length(title) <= 200),
    body TEXT NOT NULL DEFAULT '' CHECK (length(body) <= 20000),
    color TEXT NOT NULL DEFAULT 'yellow'
        CHECK (color IN ('yellow', 'blue', 'green', 'pink', 'purple', 'orange')),
    is_pinned INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0, 1)),
    exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    subject_node_id INTEGER,
    topic_node_id INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (length(trim(title)) > 0 OR length(trim(body)) > 0),
    CHECK (subject_node_id IS NULL OR exam_id IS NOT NULL),
    CHECK (topic_node_id IS NULL OR exam_id IS NOT NULL),
    CHECK (subject_node_id IS NULL OR topic_node_id IS NULL OR subject_node_id <> topic_node_id),
    FOREIGN KEY (exam_id, subject_node_id)
        REFERENCES exam_syllabus_nodes(exam_id, id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, topic_node_id)
        REFERENCES exam_syllabus_nodes(exam_id, id) ON DELETE RESTRICT
) STRICT;

CREATE INDEX IF NOT EXISTS idx_sticky_notes_pinned_updated
    ON sticky_notes(is_pinned DESC, updated_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_sticky_notes_exam
    ON sticky_notes(exam_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_sticky_notes_topic
    ON sticky_notes(topic_node_id, updated_at DESC);

CREATE TRIGGER IF NOT EXISTS trg_sticky_notes_associations_insert
BEFORE INSERT ON sticky_notes
WHEN (NEW.subject_node_id IS NOT NULL AND (
        SELECT node_kind FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.subject_node_id
    ) IS NOT 'unit')
  OR (NEW.topic_node_id IS NOT NULL AND (
        SELECT node_kind FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.topic_node_id
    ) IS NOT 'topic')
  OR (NEW.subject_node_id IS NOT NULL AND NEW.topic_node_id IS NOT NULL AND (
        SELECT parent_id FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.topic_node_id
    ) IS NOT NEW.subject_node_id)
BEGIN SELECT RAISE(ABORT, 'invalid sticky note syllabus association'); END;

CREATE TRIGGER IF NOT EXISTS trg_sticky_notes_associations_update
BEFORE UPDATE OF exam_id, subject_node_id, topic_node_id ON sticky_notes
WHEN (NEW.subject_node_id IS NOT NULL AND (
        SELECT node_kind FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.subject_node_id
    ) IS NOT 'unit')
  OR (NEW.topic_node_id IS NOT NULL AND (
        SELECT node_kind FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.topic_node_id
    ) IS NOT 'topic')
  OR (NEW.subject_node_id IS NOT NULL AND NEW.topic_node_id IS NOT NULL AND (
        SELECT parent_id FROM exam_syllabus_nodes
        WHERE exam_id = NEW.exam_id AND id = NEW.topic_node_id
    ) IS NOT NEW.subject_node_id)
BEGIN SELECT RAISE(ABORT, 'invalid sticky note syllabus association'); END;
