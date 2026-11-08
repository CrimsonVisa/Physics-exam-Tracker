-- V2 schema upgrade: version 2 -> 3 (QoL-4 flashcards).
--
-- Applied in place by open_or_initialize when an existing database carries
-- user_version 2. Fresh databases receive these tables directly from
-- schema.sql, so every statement here is idempotent. The versioned
-- schema_migrations ledger row and the app_metadata schema version are written
-- by the upgrade routine after this batch succeeds.

CREATE TABLE IF NOT EXISTS flashcards (
    id INTEGER PRIMARY KEY,
    front TEXT NOT NULL CHECK (length(trim(front)) BETWEEN 1 AND 20000),
    back TEXT NOT NULL CHECK (length(trim(back)) BETWEEN 1 AND 20000),
    subject_area TEXT NOT NULL CHECK (length(trim(subject_area)) BETWEEN 1 AND 200),
    concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (is_archived=0 AND archived_at IS NULL)
        OR (is_archived=1 AND archived_at IS NOT NULL)
    )
) STRICT;

CREATE TABLE IF NOT EXISTS flashcard_exam_links (
    flashcard_id INTEGER NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    syllabus_node_id INTEGER,
    created_at TEXT NOT NULL,
    PRIMARY KEY (flashcard_id, exam_id),
    FOREIGN KEY (exam_id, syllabus_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE IF NOT EXISTS flashcard_schedule (
    flashcard_id INTEGER PRIMARY KEY REFERENCES flashcards(id) ON DELETE CASCADE,
    due_on TEXT NOT NULL CHECK (length(due_on)=10 AND date(due_on)=due_on),
    interval_days INTEGER NOT NULL DEFAULT 0
        CHECK (interval_days BETWEEN 0 AND 3650),
    ease_factor REAL NOT NULL DEFAULT 2.5
        CHECK (ease_factor BETWEEN 1.3 AND 3.0),
    review_count INTEGER NOT NULL DEFAULT 0 CHECK (review_count >= 0),
    lapse_count INTEGER NOT NULL DEFAULT 0 CHECK (lapse_count >= 0 AND lapse_count <= review_count),
    last_rating TEXT CHECK (last_rating IS NULL OR last_rating IN ('again','hard','good','easy')),
    last_reviewed_on TEXT CHECK (
        last_reviewed_on IS NULL
        OR (length(last_reviewed_on)=10 AND date(last_reviewed_on)=last_reviewed_on)
    ),
    last_reviewed_at TEXT,
    algorithm_version INTEGER NOT NULL DEFAULT 1 CHECK (algorithm_version >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (review_count=0 AND interval_days=0 AND last_rating IS NULL
            AND last_reviewed_on IS NULL AND last_reviewed_at IS NULL)
        OR (review_count>0 AND interval_days BETWEEN 1 AND 3650
            AND last_rating IS NOT NULL AND last_reviewed_on IS NOT NULL
            AND last_reviewed_at IS NOT NULL)
    ),
    CHECK (last_reviewed_on IS NULL OR due_on > last_reviewed_on)
) STRICT;

CREATE TABLE IF NOT EXISTS flashcard_reviews (
    id INTEGER PRIMARY KEY,
    flashcard_id INTEGER NOT NULL REFERENCES flashcards(id) ON DELETE CASCADE,
    reviewed_on TEXT NOT NULL CHECK (length(reviewed_on)=10 AND date(reviewed_on)=reviewed_on),
    reviewed_at TEXT NOT NULL,
    rating TEXT NOT NULL CHECK (rating IN ('again','hard','good','easy')),
    scheduled_due_on TEXT NOT NULL CHECK (
        length(scheduled_due_on)=10 AND date(scheduled_due_on)=scheduled_due_on
    ),
    next_due_on TEXT NOT NULL CHECK (
        length(next_due_on)=10 AND date(next_due_on)=next_due_on
    ),
    interval_before_days INTEGER NOT NULL CHECK (interval_before_days BETWEEN 0 AND 3650),
    interval_after_days INTEGER NOT NULL CHECK (interval_after_days BETWEEN 1 AND 3650),
    ease_before REAL NOT NULL CHECK (ease_before BETWEEN 1.3 AND 3.0),
    ease_after REAL NOT NULL CHECK (ease_after BETWEEN 1.3 AND 3.0),
    algorithm_version INTEGER NOT NULL CHECK (algorithm_version >= 1),
    created_at TEXT NOT NULL,
    CHECK (next_due_on > reviewed_on)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_flashcards_subject ON flashcards(subject_area COLLATE NOCASE, id);
CREATE INDEX IF NOT EXISTS idx_flashcards_concept ON flashcards(concept_id, id);
CREATE INDEX IF NOT EXISTS idx_flashcards_archived ON flashcards(is_archived, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_flashcard_exam_links_exam ON flashcard_exam_links(exam_id, flashcard_id);
CREATE INDEX IF NOT EXISTS idx_flashcard_exam_links_node ON flashcard_exam_links(exam_id, syllabus_node_id, flashcard_id);
CREATE INDEX IF NOT EXISTS idx_flashcard_schedule_due ON flashcard_schedule(due_on, flashcard_id);
CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_card_time ON flashcard_reviews(flashcard_id, reviewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_flashcard_reviews_day ON flashcard_reviews(reviewed_on, id);

CREATE TRIGGER IF NOT EXISTS trg_flashcard_reviews_no_update BEFORE UPDATE ON flashcard_reviews
BEGIN SELECT RAISE(ABORT,'flashcard review history is append-only'); END;
