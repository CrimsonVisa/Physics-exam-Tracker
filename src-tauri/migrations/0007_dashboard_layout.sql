-- Phase 8: persisted dashboard section preferences.
--
-- The dashboard layout is intentionally stored in SQLite rather than browser
-- storage so it survives restarts and follows the user's local data. Each row
-- controls one known dashboard section. Analytics data itself remains derived
-- from the existing source tables and is never duplicated here.

CREATE TABLE dashboard_sections (
    section_key TEXT PRIMARY KEY
        CHECK (
            section_key IN (
                'exam-countdown',
                'syllabus-progress',
                'study-totals',
                'today-study',
                'revision-status',
                'deadlines-blocks',
                'recent-pyq',
                'latest-mock',
                'insights'
            )
        ),
    visible INTEGER NOT NULL DEFAULT 1
        CHECK (visible IN (0, 1)),
    display_order INTEGER NOT NULL UNIQUE
        CHECK (display_order BETWEEN 0 AND 1000),
    updated_at TEXT NOT NULL
) STRICT;

INSERT INTO dashboard_sections (
    section_key,
    visible,
    display_order,
    updated_at
) VALUES
    ('exam-countdown', 1, 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('syllabus-progress', 1, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('study-totals', 1, 2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('today-study', 1, 3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('revision-status', 1, 4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('deadlines-blocks', 1, 5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('recent-pyq', 1, 6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('latest-mock', 1, 7, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('insights', 1, 8, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
