-- Milestone 3.2: adaptive revision planning, deferral audit and session links.
--
-- Work items (revision_plan_items) are revision attempts that can span days.
-- Daily plan versions own only membership snapshots, so carrying an item to a
-- later plan never rewrites the old plan. Deferrals are append-only, and
-- completed revision history remains append-only and unchanged.

ALTER TABLE revision_settings
ADD COLUMN daily_revision_minutes INTEGER NOT NULL DEFAULT 90
CHECK (daily_revision_minutes BETWEEN 5 AND 1440);

ALTER TABLE revision_schedules
ADD COLUMN deferred_until TEXT
CHECK (
    deferred_until IS NULL
    OR (
        length(deferred_until) = 10
        AND deferred_until GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
        AND date(deferred_until) IS deferred_until
    )
);

CREATE TABLE revision_plan_items (
    id INTEGER PRIMARY KEY,
    topic_id INTEGER NOT NULL
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN (
            'planned', 'in_progress', 'completed',
            'backlog', 'deferred', 'removed'
        )),
    estimated_minutes INTEGER NOT NULL
        CHECK (estimated_minutes BETWEEN 5 AND 240),
    triage_score REAL NOT NULL
        CHECK (triage_score BETWEEN 0 AND 100),
    lane TEXT NOT NULL
        CHECK (lane IN ('urgent_review', 'strengthen', 'maintain')),
    source_due_date TEXT NOT NULL
        CHECK (
            length(source_due_date) = 10
            AND source_due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(source_due_date) IS source_due_date
        ),
    planned_block_id INTEGER
        REFERENCES planned_study_blocks(id) ON DELETE SET NULL,
    revision_history_id INTEGER
        REFERENCES revision_history(id) ON DELETE RESTRICT,
    removed_at TEXT
        CHECK (
            removed_at IS NULL
            OR (
                length(removed_at) = 10
                AND removed_at GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND date(removed_at) IS removed_at
            )
        ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (status <> 'completed' OR revision_history_id IS NOT NULL)
) STRICT;

CREATE UNIQUE INDEX idx_revision_plan_active_topic
    ON revision_plan_items(topic_id)
    WHERE status IN ('planned', 'in_progress', 'backlog', 'deferred');

CREATE UNIQUE INDEX idx_revision_plan_history_once
    ON revision_plan_items(revision_history_id)
    WHERE revision_history_id IS NOT NULL;

CREATE INDEX idx_revision_plan_status
    ON revision_plan_items(status, topic_id);

CREATE INDEX idx_revision_plan_block
    ON revision_plan_items(planned_block_id)
    WHERE planned_block_id IS NOT NULL;

CREATE TRIGGER revision_plan_items_completed_immutable
BEFORE UPDATE ON revision_plan_items
WHEN OLD.status = 'completed'
BEGIN
    SELECT RAISE(ABORT, 'completed revision work items are immutable');
END;

CREATE TRIGGER revision_plan_items_completed_no_delete
BEFORE DELETE ON revision_plan_items
WHEN OLD.status = 'completed'
BEGIN
    SELECT RAISE(ABORT, 'completed revision work items cannot be deleted');
END;

CREATE TABLE revision_daily_plans (
    id INTEGER PRIMARY KEY,
    plan_date TEXT NOT NULL
        CHECK (
            length(plan_date) = 10
            AND plan_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(plan_date) IS plan_date
        ),
    version INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    effective_budget_minutes INTEGER NOT NULL
        CHECK (effective_budget_minutes BETWEEN 1 AND 1440),
    max_topics INTEGER NOT NULL
        CHECK (max_topics BETWEEN 1 AND 100),
    planned_minutes INTEGER NOT NULL DEFAULT 0
        CHECK (planned_minutes >= 0),
    completed_minutes REAL NOT NULL DEFAULT 0
        CHECK (completed_minutes >= 0),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'superseded', 'closed')),
    created_at TEXT NOT NULL,
    last_rebalanced_at TEXT,
    updated_at TEXT NOT NULL,
    UNIQUE (plan_date, version)
) STRICT;

CREATE UNIQUE INDEX idx_revision_daily_plan_active_date
    ON revision_daily_plans(plan_date)
    WHERE status = 'active';

CREATE INDEX idx_revision_daily_plan_date_version
    ON revision_daily_plans(plan_date, version);

CREATE TABLE revision_daily_plan_items (
    daily_plan_id INTEGER NOT NULL
        REFERENCES revision_daily_plans(id) ON DELETE RESTRICT,
    plan_item_id INTEGER NOT NULL
        REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    estimated_minutes INTEGER NOT NULL
        CHECK (estimated_minutes BETWEEN 5 AND 240),
    lane TEXT NOT NULL
        CHECK (lane IN ('urgent_review', 'strengthen', 'maintain')),
    triage_score REAL NOT NULL
        CHECK (triage_score BETWEEN 0 AND 100),
    membership_status TEXT NOT NULL DEFAULT 'planned'
        CHECK (membership_status IN ('planned', 'in_progress', 'completed')),
    planned_block_id INTEGER,
    planned_block_title TEXT,
    planned_block_date TEXT,
    planned_block_start_time TEXT,
    planned_block_end_time TEXT,
    carried_from_daily_plan_id INTEGER
        REFERENCES revision_daily_plans(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (daily_plan_id, plan_item_id)
) STRICT;

CREATE INDEX idx_revision_daily_plan_items_item
    ON revision_daily_plan_items(plan_item_id);

CREATE INDEX idx_revision_daily_plan_items_block
    ON revision_daily_plan_items(planned_block_id)
    WHERE planned_block_id IS NOT NULL;

CREATE TABLE revision_deferrals (
    id INTEGER PRIMARY KEY,
    topic_id INTEGER NOT NULL
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    plan_item_id INTEGER
        REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL
        CHECK (event_kind IN ('defer', 'override')),
    overridden_deferral_id INTEGER
        REFERENCES revision_deferrals(id) ON DELETE RESTRICT,
    previous_due_date TEXT NOT NULL
        CHECK (
            length(previous_due_date) = 10
            AND previous_due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(previous_due_date) IS previous_due_date
        ),
    new_due_date TEXT NOT NULL
        CHECK (
            length(new_due_date) = 10
            AND new_due_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(new_due_date) IS new_due_date
        ),
    occurred_on TEXT NOT NULL
        CHECK (
            length(occurred_on) = 10
            AND occurred_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(occurred_on) IS occurred_on
        ),
    reason TEXT NOT NULL
        CHECK (reason IN ('time', 'difficulty', 'energy', 'life', 'other')),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    CHECK (
        (event_kind = 'defer' AND new_due_date > previous_due_date)
        OR (event_kind = 'override' AND new_due_date < previous_due_date)
    )
) STRICT;

CREATE INDEX idx_revision_deferrals_topic_date
    ON revision_deferrals(topic_id, occurred_on DESC, id DESC);

CREATE TRIGGER revision_deferrals_no_update
BEFORE UPDATE ON revision_deferrals
BEGIN
    SELECT RAISE(ABORT, 'revision deferrals are append-only');
END;

CREATE TRIGGER revision_deferrals_no_delete
BEFORE DELETE ON revision_deferrals
BEGIN
    SELECT RAISE(ABORT, 'revision deferrals are append-only');
END;

CREATE TABLE revision_session_links (
    plan_item_id INTEGER NOT NULL
        REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    study_session_id INTEGER NOT NULL
        REFERENCES study_sessions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (plan_item_id, study_session_id),
    UNIQUE (study_session_id)
) STRICT;

CREATE INDEX idx_revision_session_links_session
    ON revision_session_links(study_session_id);

ALTER TABLE timer_state
ADD COLUMN revision_plan_item_id INTEGER
REFERENCES revision_plan_items(id) ON DELETE SET NULL;

CREATE TRIGGER revision_daily_plan_items_insert_active
BEFORE INSERT ON revision_daily_plan_items
WHEN (
    SELECT status FROM revision_daily_plans
    WHERE id = NEW.daily_plan_id
) <> 'active'
BEGIN
    SELECT RAISE(ABORT, 'daily plan memberships require an active plan');
END;

CREATE TRIGGER revision_daily_plan_items_update_active
BEFORE UPDATE ON revision_daily_plan_items
WHEN (
    (SELECT status FROM revision_daily_plans WHERE id = OLD.daily_plan_id) <> 'active'
    OR OLD.membership_status = 'completed'
    OR NEW.daily_plan_id IS NOT OLD.daily_plan_id
    OR NEW.plan_item_id IS NOT OLD.plan_item_id
    OR NEW.position IS NOT OLD.position
    OR NEW.estimated_minutes IS NOT OLD.estimated_minutes
    OR NEW.lane IS NOT OLD.lane
    OR NEW.triage_score IS NOT OLD.triage_score
    OR NEW.planned_block_id IS NOT OLD.planned_block_id
    OR NEW.planned_block_title IS NOT OLD.planned_block_title
    OR NEW.planned_block_date IS NOT OLD.planned_block_date
    OR NEW.planned_block_start_time IS NOT OLD.planned_block_start_time
    OR NEW.planned_block_end_time IS NOT OLD.planned_block_end_time
    OR NEW.carried_from_daily_plan_id IS NOT OLD.carried_from_daily_plan_id
    OR NEW.created_at IS NOT OLD.created_at
    OR (OLD.membership_status = 'in_progress' AND NEW.membership_status = 'planned')
)
BEGIN
    SELECT RAISE(ABORT, 'daily plan membership snapshots are immutable');
END;

CREATE TRIGGER revision_daily_plan_items_no_delete
BEFORE DELETE ON revision_daily_plan_items
BEGIN
    SELECT RAISE(ABORT, 'daily plan memberships are immutable');
END;

CREATE TRIGGER revision_daily_plans_no_delete
BEFORE DELETE ON revision_daily_plans
BEGIN
    SELECT RAISE(ABORT, 'daily plan versions are immutable');
END;

CREATE TRIGGER revision_daily_plans_snapshot_immutable
BEFORE UPDATE ON revision_daily_plans
WHEN OLD.status <> 'active'
    AND (
        NEW.plan_date <> OLD.plan_date
        OR NEW.version <> OLD.version
        OR NEW.effective_budget_minutes <> OLD.effective_budget_minutes
        OR NEW.max_topics <> OLD.max_topics
        OR NEW.planned_minutes <> OLD.planned_minutes
        OR NEW.completed_minutes <> OLD.completed_minutes
        OR NEW.created_at IS NOT OLD.created_at
        OR NEW.last_rebalanced_at IS NOT OLD.last_rebalanced_at
    )
BEGIN
    SELECT RAISE(ABORT, 'daily plan snapshots are immutable');
END;
