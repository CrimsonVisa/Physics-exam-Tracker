-- Phase 7: spaced-repetition revision scheduling.
--
-- revision_settings is a singleton configuration row. revision_schedules holds
-- at most one current schedule per syllabus topic. revision_history is
-- append-only: update and delete triggers protect completed outcomes while
-- keeping the scheduling row independently editable.

CREATE TABLE revision_settings (
    id INTEGER PRIMARY KEY
        CHECK (id = 1),
    initial_again_days INTEGER NOT NULL DEFAULT 1
        CHECK (initial_again_days BETWEEN 1 AND 3650),
    initial_difficult_days INTEGER NOT NULL DEFAULT 2
        CHECK (initial_difficult_days BETWEEN 1 AND 3650),
    initial_good_days INTEGER NOT NULL DEFAULT 5
        CHECK (initial_good_days BETWEEN 1 AND 3650),
    initial_easy_days INTEGER NOT NULL DEFAULT 10
        CHECK (initial_easy_days BETWEEN 1 AND 3650),
    growth_multiplier REAL NOT NULL DEFAULT 1.5
        CHECK (growth_multiplier BETWEEN 1.0 AND 10.0),
    minimum_interval_days INTEGER NOT NULL DEFAULT 1
        CHECK (minimum_interval_days BETWEEN 1 AND 3650),
    maximum_interval_days INTEGER NOT NULL DEFAULT 3650
        CHECK (maximum_interval_days BETWEEN 1 AND 3650),
    include_weekends INTEGER NOT NULL DEFAULT 1
        CHECK (include_weekends IN (0, 1)),
    daily_revision_target INTEGER NOT NULL DEFAULT 5
        CHECK (daily_revision_target BETWEEN 1 AND 100),
    reminders_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (reminders_enabled IN (0, 1)),
    last_reminder_date TEXT
        CHECK (
            last_reminder_date IS NULL
            OR (
                length(last_reminder_date) = 10
                AND last_reminder_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND date(last_reminder_date) IS last_reminder_date
            )
        ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (minimum_interval_days <= maximum_interval_days),
    CHECK (initial_again_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_difficult_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_good_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_easy_days BETWEEN minimum_interval_days AND maximum_interval_days)
) STRICT;

INSERT INTO revision_settings (
    id,
    initial_again_days,
    initial_difficult_days,
    initial_good_days,
    initial_easy_days,
    growth_multiplier,
    minimum_interval_days,
    maximum_interval_days,
    include_weekends,
    daily_revision_target,
    reminders_enabled,
    last_reminder_date,
    created_at,
    updated_at
) VALUES (
    1,
    1,
    2,
    5,
    10,
    1.5,
    1,
    3650,
    1,
    5,
    0,
    NULL,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);

CREATE TABLE revision_schedules (
    topic_id INTEGER PRIMARY KEY
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended')),
    last_revision_date TEXT
        CHECK (
            last_revision_date IS NULL
            OR (
                length(last_revision_date) = 10
                AND last_revision_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND date(last_revision_date) IS last_revision_date
            )
        ),
    next_revision_date TEXT NOT NULL
        CHECK (
            length(next_revision_date) = 10
            AND next_revision_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(next_revision_date) IS next_revision_date
        ),
    current_interval_days INTEGER NOT NULL
        CHECK (current_interval_days BETWEEN 1 AND 3650),
    completed_revisions INTEGER NOT NULL DEFAULT 0
        CHECK (completed_revisions >= 0),
    previous_outcome TEXT
        CHECK (
            previous_outcome IS NULL
            OR previous_outcome IN ('again', 'difficult', 'good', 'easy')
        ),
    mastery_estimate REAL NOT NULL DEFAULT 0
        CHECK (mastery_estimate BETWEEN 0 AND 100),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (
            completed_revisions = 0
            AND last_revision_date IS NULL
            AND previous_outcome IS NULL
        )
        OR (
            completed_revisions > 0
            AND last_revision_date IS NOT NULL
            AND previous_outcome IS NOT NULL
        )
    )
) STRICT;

CREATE INDEX idx_revision_schedules_due
    ON revision_schedules(status, next_revision_date, topic_id);

CREATE INDEX idx_revision_schedules_last
    ON revision_schedules(last_revision_date, topic_id);

CREATE TABLE revision_history (
    id INTEGER PRIMARY KEY,
    topic_id INTEGER NOT NULL
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    revision_date TEXT NOT NULL
        CHECK (
            length(revision_date) = 10
            AND revision_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(revision_date) IS revision_date
        ),
    scheduled_for_date TEXT NOT NULL
        CHECK (
            length(scheduled_for_date) = 10
            AND scheduled_for_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(scheduled_for_date) IS scheduled_for_date
        ),
    outcome TEXT NOT NULL
        CHECK (outcome IN ('again', 'difficult', 'good', 'easy')),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    interval_before_days INTEGER NOT NULL
        CHECK (interval_before_days BETWEEN 1 AND 3650),
    interval_after_days INTEGER NOT NULL
        CHECK (interval_after_days BETWEEN 1 AND 3650),
    mastery_before REAL NOT NULL
        CHECK (mastery_before BETWEEN 0 AND 100),
    mastery_after REAL NOT NULL
        CHECK (mastery_after BETWEEN 0 AND 100),
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX idx_revision_history_topic_date
    ON revision_history(topic_id, revision_date DESC, id DESC);

CREATE INDEX idx_revision_history_date
    ON revision_history(revision_date DESC, id DESC);

CREATE TRIGGER revision_history_no_update
BEFORE UPDATE ON revision_history
BEGIN
    SELECT RAISE(ABORT, 'revision history is append-only');
END;

CREATE TRIGGER revision_history_no_delete
BEFORE DELETE ON revision_history
BEGIN
    SELECT RAISE(ABORT, 'revision history is append-only');
END;
