-- Phase 4: persisted study timer and append-only focus-session records.
-- Timer state is a singleton row. Study sessions are independent records: timer
-- breaks never create sessions, and deleting a session never deletes a topic.

CREATE TABLE timer_settings (
    id INTEGER PRIMARY KEY
        CHECK (id = 1),
    focus_minutes INTEGER NOT NULL DEFAULT 25
        CHECK (focus_minutes BETWEEN 1 AND 240),
    short_break_minutes INTEGER NOT NULL DEFAULT 5
        CHECK (short_break_minutes BETWEEN 1 AND 60),
    long_break_minutes INTEGER NOT NULL DEFAULT 15
        CHECK (long_break_minutes BETWEEN 1 AND 120),
    notifications_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (notifications_enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

INSERT INTO timer_settings (
    id,
    focus_minutes,
    short_break_minutes,
    long_break_minutes,
    notifications_enabled,
    created_at,
    updated_at
) VALUES (
    1,
    25,
    5,
    15,
    0,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);

CREATE TABLE study_sessions (
    id INTEGER PRIMARY KEY,
    topic_id INTEGER
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    session_date TEXT NOT NULL
        CHECK (
            length(session_date) = 10
            AND session_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(session_date) IS session_date
        ),
    started_at TEXT,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL
        CHECK (duration_seconds BETWEEN 1 AND 86400),
    source TEXT NOT NULL
        CHECK (source IN ('timer', 'manual')),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (started_at IS NULL AND ended_at IS NULL)
        OR (started_at IS NOT NULL AND ended_at IS NOT NULL AND started_at < ended_at)
    )
) STRICT;

CREATE INDEX idx_study_sessions_date
    ON study_sessions(session_date, id);

CREATE INDEX idx_study_sessions_topic_date
    ON study_sessions(topic_id, session_date);

CREATE INDEX idx_study_sessions_source
    ON study_sessions(source);

CREATE TABLE timer_state (
    id INTEGER PRIMARY KEY
        CHECK (id = 1),
    status TEXT NOT NULL DEFAULT 'idle'
        CHECK (status IN ('idle', 'running', 'paused')),
    mode TEXT NOT NULL DEFAULT 'focus'
        CHECK (mode IN ('focus', 'short_break', 'long_break')),
    duration_ms INTEGER NOT NULL DEFAULT 0
        CHECK (duration_ms BETWEEN 0 AND 86400000),
    accumulated_ms INTEGER NOT NULL DEFAULT 0
        CHECK (accumulated_ms >= 0 AND accumulated_ms <= duration_ms),
    first_started_at TEXT,
    segment_started_at TEXT,
    last_heartbeat_at TEXT,
    topic_id INTEGER
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    session_date TEXT
        CHECK (
            session_date IS NULL
            OR (
                length(session_date) = 10
                AND session_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND date(session_date) IS session_date
            )
        ),
    recovery_reason TEXT
        CHECK (recovery_reason IN ('restart', 'inactivity')),
    recovery_excluded_ms INTEGER NOT NULL DEFAULT 0
        CHECK (recovery_excluded_ms >= 0),
    recovery_at TEXT,
    last_event TEXT
        CHECK (last_event IN ('completed', 'discarded')),
    last_event_at TEXT,
    last_event_duration_ms INTEGER NOT NULL DEFAULT 0
        CHECK (last_event_duration_ms >= 0),
    last_completed_session_id INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (
            status = 'idle'
            AND accumulated_ms = 0
            AND first_started_at IS NULL
            AND segment_started_at IS NULL
        )
        OR (
            status = 'running'
            AND first_started_at IS NOT NULL
            AND segment_started_at IS NOT NULL
            AND last_heartbeat_at IS NOT NULL
        )
        OR (
            status = 'paused'
            AND first_started_at IS NOT NULL
            AND segment_started_at IS NULL
        )
    ),
    CHECK (recovery_reason IS NULL OR status = 'paused')
) STRICT;

INSERT INTO timer_state (
    id,
    status,
    mode,
    duration_ms,
    accumulated_ms,
    last_event_duration_ms,
    created_at,
    updated_at
) VALUES (
    1,
    'idle',
    'focus',
    0,
    0,
    0,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);
