-- Phase 5: persistent checklist tasks and planned study blocks.
-- Dates are local calendar dates stored as YYYY-MM-DD strings, matching the
-- existing settings and study-session model. Planned blocks are only plans and
-- are deliberately not linked to study-time totals.

CREATE TABLE checklist_tasks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT ''
        CHECK (length(description) <= 2000),
    scheduled_date TEXT NOT NULL
        CHECK (
            length(scheduled_date) = 10
            AND scheduled_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND scheduled_date BETWEEN '0001-01-01' AND '9999-12-31'
            AND date(scheduled_date) IS scheduled_date
        ),
    deadline_date TEXT
        CHECK (
            deadline_date IS NULL
            OR (
                length(deadline_date) = 10
                AND deadline_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND deadline_date BETWEEN '0001-01-01' AND '9999-12-31'
                AND date(deadline_date) IS deadline_date
            )
        ),
    topic_id INTEGER
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    priority TEXT NOT NULL DEFAULT 'Normal'
        CHECK (priority IN ('Low', 'Normal', 'High', 'High-Yield')),
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'completed')),
    schedule_anchor TEXT NOT NULL DEFAULT 'manual'
        CHECK (schedule_anchor IN ('manual', 'exam')),
    schedule_offset_days INTEGER,
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (schedule_anchor = 'manual' AND schedule_offset_days IS NULL)
        OR (
            schedule_anchor = 'exam'
            AND schedule_offset_days BETWEEN -3650 AND 3650
        )
    ),
    CHECK (
        (status = 'open' AND completed_at IS NULL)
        OR (status = 'completed' AND completed_at IS NOT NULL)
    )
) STRICT;

CREATE INDEX idx_checklist_tasks_scheduled_date
    ON checklist_tasks(scheduled_date, id);

CREATE INDEX idx_checklist_tasks_deadline_date
    ON checklist_tasks(deadline_date, id);

CREATE INDEX idx_checklist_tasks_status_due
    ON checklist_tasks(status, scheduled_date, deadline_date);

CREATE INDEX idx_checklist_tasks_topic
    ON checklist_tasks(topic_id);

CREATE TABLE planned_study_blocks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL
        CHECK (length(trim(title)) BETWEEN 1 AND 200),
    topic_id INTEGER
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    calendar_date TEXT NOT NULL
        CHECK (
            length(calendar_date) = 10
            AND calendar_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND calendar_date BETWEEN '0001-01-01' AND '9999-12-31'
            AND date(calendar_date) IS calendar_date
        ),
    start_time TEXT NOT NULL
        CHECK (
            length(start_time) = 5
            AND start_time GLOB '[0-2][0-9]:[0-5][0-9]'
            AND start_time BETWEEN '00:00' AND '23:59'
        ),
    end_time TEXT NOT NULL
        CHECK (
            length(end_time) = 5
            AND end_time GLOB '[0-2][0-9]:[0-5][0-9]'
            AND end_time BETWEEN '00:00' AND '23:59'
        ),
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'completed', 'cancelled')),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    linked_session_id INTEGER
        REFERENCES study_sessions(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (start_time < end_time),
    CHECK (status = 'completed' OR linked_session_id IS NULL)
) STRICT;

CREATE INDEX idx_planned_blocks_date
    ON planned_study_blocks(calendar_date, start_time, id);

CREATE INDEX idx_planned_blocks_topic
    ON planned_study_blocks(topic_id);

CREATE INDEX idx_planned_blocks_linked_session
    ON planned_study_blocks(linked_session_id);
