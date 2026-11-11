# Physics Tracker V2.0 — Technical Implementation Specification V1.3.2

**Status:** Final technical consistency and validation pass. Specification only. No application source code, database files, migrations, dependencies, configuration, or Git state were modified.

**Scope:** Exactly four supported examinations:

1. CSIR-UGC NET Physical Sciences (`csir-net-physics`)
2. GATE Physics (PH) (`gate-physics`)
3. TIFR Graduate School Physics (`tifr-gs-physics`)
4. JEST Physics (`jest-physics`)

No additional examination is supported. JEST is present in every registry, selector, backup manifest, test matrix, and release gate.

---

## 0. Change log relative to V1.3.1

| Area                    | Correction                                                                                                                                                                           |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Mastery replay ordering | Recalculation run is inserted before replay results. Full transaction pseudocode added; empty evidence set defined.                                                                  |
| Mastery calculation     | Undefined `base_delta` replaced with exact formulas for study, revision, PYQ, mock concept results, manual overrides, difficulty, validation, confidence, caps, and examples.        |
| Revision-plan integrity | Added executable triggers and Rust validation matrix for plan/membership lifecycle, exam ownership, mapping checks, deferral, cancellation, completion, archival, and session links. |
| Mock score history      | Full `marking_context_json` schema and exact completion/reopen/recompletion transaction rules added. Direct insertion as completed is blocked.                                       |
| Backup/restore          | Replaced stage outline with deterministic sidecar state machine, exact file names, durable writes, rollback, and startup recovery.                                                   |
| Readiness               | Added executable component formulas, archived/inactive handling, confidence thresholds, and snapshot rules.                                                                          |
| Schema self-containment | All tables, indexes, triggers, seeds, and creation order included in this document.                                                                                                  |
| Validation              | Added standalone Python schema validation script and complete trigger/FK test matrix.                                                                                                |
| References              | All schema references point to §7; no inherited “V1.3 remains” statements.                                                                                                           |

---

## 1. Executive summary

V2.0 is one Windows-first React/Tauri/Rust/SQLite application with:

- One shared Physics concept library and one mastery state.
- Four independent exam workspaces.
- One shared study timer, session system, planner, revision system, resource library, and settings model.
- Exam-specific syllabus, sections, question types, marking, PYQs, mocks, priorities, readiness, analytics, and dates.
- A new V2 database and `.ptbackup` format.
- No automatic V1 migration, import, restore, or access.

Core invariants:

1. `study_sessions.duration_seconds` is counted once.
2. Exam study time is derived from `session_exam_allocations`.
3. Concept mastery is shared; readiness is computed per exam.
4. Inactive exams are read-only in the Rust backend.
5. Readiness never invents missing evidence.
6. Exam dates are manual, nullable, local-date-only.
7. V1 data and backups remain untouched.

---

## 2. Product requirements

### 2.1 Shared

- Concepts and mastery
- Study timer and sessions
- Checklist/planner/calendar
- Revision schedules and daily plans
- Shared resources
- Global settings
- Combined dashboards/analytics

### 2.2 Exam-specific

- Syllabus and progress
- Sections, question types, marking
- PYQs and attempts
- Mocks and section results
- Priorities
- Readiness
- Analytics
- Manual exam date
- Historical records after deactivation

### 2.3 Quality rules

No placeholders, fake analytics, nonfunctional controls, cross-exam leakage, automatic exam dates, automatic V1 migration, or V1 modification.

---

## 3. Confirmed decisions

| ID    | Decision                                                                                      |
| ----- | --------------------------------------------------------------------------------------------- |
| AD-01 | React + TypeScript + Vite + Tauri 2 + Rust + SQLite/rusqlite.                                 |
| AD-02 | One app, one frontend, one backend, one V2 database.                                          |
| AD-03 | Four exam workspaces are scopes, not databases.                                               |
| AD-04 | Shared concepts/mastery/sessions/planner/timer/revision/resources/settings.                   |
| AD-05 | Exam-specific syllabus/sections/question types/marking/PYQs/mocks/priorities/readiness/dates. |
| AD-06 | Study duration stored once; exam time via allocations.                                        |
| AD-07 | Shared mastery; separate exam readiness.                                                      |
| AD-08 | Inactive exams enforced read-only in Rust.                                                    |
| AD-09 | Exactly four exams seeded and verified.                                                       |
| AD-10 | Exam dates manual, nullable, local-date-only.                                                 |
| AD-11 | Mock scoring uses one section-aggregate model.                                                |
| AD-12 | Stable syllabus codes and content migration mappings.                                         |
| AD-13 | New V2 identity/data path.                                                                    |
| AD-14 | No V1 access/migration/restore.                                                               |
| AD-15 | Readiness components; overall nullable.                                                       |
| AD-16 | Rust authoritative validation.                                                                |
| AD-17 | No new frontend state library.                                                                |
| AD-18 | Build-size changes measured.                                                                  |

---

## 4. Application architecture

### 4.1 Identity

| Item           | V1                                         | V2                                          |
| -------------- | ------------------------------------------ | ------------------------------------------- |
| Identifier     | `com.crimsonvisa.csirnetphysics`           | `com.crimsonvisa.physics-tracker`           |
| Data directory | `%APPDATA%\com.crimsonvisa.csirnetphysics` | `%APPDATA%\com.crimsonvisa.physics-tracker` |
| Database       | `csir-net-tracker.sqlite3`                 | `physics-tracker.sqlite3`                   |
| Backups        | V1 SQLite + manifest                       | `.ptbackup` V2                              |

V2 verifies its data path is not equal to, inside, or an ancestor of the V1 path. V2 never opens V1 files.

### 4.2 Layers

```text
React UI
  ExamContext: Combined | Exam(id)
  Screens/components
        │ typed invoke
        ▼
Tauri commands
  Domain modules; explicit exam scope; inactive guard
        │
        ▼
Rust services
  Transactions, validation, mastery replay, readiness, backup
        │
        ▼
SQLite
  Shared + exam-specific + bridge + audit tables
```

### 4.3 SQLite requirements

Startup checks:

```sql
SELECT sqlite_version();           -- must be >= 3.37
SELECT json_valid('{}');           -- must return 1
PRAGMA foreign_keys = ON;          -- must report 1
```

Required: `STRICT`, JSON1, partial indexes, UPSERT, foreign keys, WAL.

### 4.4 PRAGMAs

- `foreign_keys=ON`
- `journal_mode=WAL`
- `busy_timeout=5000`
- `synchronous=FULL` initially.

All writes transactional. Heavy work on `spawn_blocking`.

### 4.5 Errors

`validation_error`, `not_found`, `conflict`, `forbidden`, `database_error`, `migration_error`, `backup_error`, `restore_error`, `file_error`, `internal_error`.

---

## 5. Examination workspace design

Each workspace contains:

1. Overview
2. Exam date
3. Syllabus/progress
4. Sections/question types/marking
5. PYQs/attempts
6. Mocks/section results
7. Performance analytics
8. Priorities
9. Readiness components/history
10. Historical/inactive access

All four use the same architecture. No exam-specific backend branches for core workflows.

Inactive workspace default: read-only history; reactivation restores editing. Backend enforcement is authoritative.

---

## 6. Official content requirements

Every seeded dataset has an `exam_content_versions` row with source reference/URL, publication/version date, verification date, review status, and reviewer. Production seed data must be `verified`. Missing or ambiguous information is marked pending, never invented.

Required per exam: official syllabus source/version, paper structure, sections, question types, marking/negative/partial rules, duration, and official date reference (manual only; no scraping).

Content updates use stable node codes and `exam_syllabus_node_migrations`; progress is preserved on exact codes or mapped nodes.

---

## 7. Executable SQLite schema

### 7.1 Validation rules

No custom SQL functions are used.

Nullable date:

```sql
CHECK (col IS NULL OR (length(col)=10 AND date(col)=col))
```

Non-null date:

```sql
CHECK (length(col)=10 AND date(col)=col)
```

Valid `HH:MM` time (accepts `09:30`; rejects `9:30`, `24:00`, `09:60`, malformed values):

```sql
CHECK (
    length(col)=5
    AND col GLOB '[0-9][0-9]:[0-9][0-9]'
    AND CAST(substr(col,1,2) AS INTEGER) BETWEEN 0 AND 23
    AND CAST(substr(col,4,2) AS INTEGER) BETWEEN 0 AND 59
)
```

### 7.2 Creation and seed order

1. Open database, apply PRAGMAs, verify SQLite version/JSON1.
2. Create tables in this exact order.
3. Create indexes.
4. Create triggers.
5. Insert seed records.
6. Run `PRAGMA foreign_key_check`.
7. Verify exactly four exams/profiles and exact code set.
8. Commit.

### 7.3 Schema tables

<!-- SCHEMA-TABLES:BEGIN -->

```sql
CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY CHECK (version >= 1),
    name TEXT NOT NULL UNIQUE,
    checksum TEXT NOT NULL CHECK (length(checksum)=64 AND checksum=lower(checksum) AND checksum NOT GLOB '*[^0-9a-f]*'),
    applied_at TEXT NOT NULL
) STRICT;

CREATE TABLE app_metadata (
    id INTEGER PRIMARY KEY CHECK (id=1),
    schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
    database_id TEXT NOT NULL UNIQUE CHECK (
        length(database_id)=32 AND database_id=lower(database_id)
        AND database_id NOT GLOB '*[^0-9a-f]*'
    ),
    app_version TEXT NOT NULL CHECK (length(app_version) BETWEEN 1 AND 40),
    exam_config_version INTEGER NOT NULL CHECK (exam_config_version >= 1),
    last_full_integrity_check_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id=1),
    preparation_start_date TEXT CHECK (
        preparation_start_date IS NULL OR
        (length(preparation_start_date)=10 AND date(preparation_start_date)=preparation_start_date)
    ),
    daily_study_target_minutes INTEGER NOT NULL DEFAULT 120
        CHECK (daily_study_target_minutes BETWEEN 1 AND 1440),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exams (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE CHECK (
        code IN ('csir-net-physics','gate-physics','tifr-gs-physics','jest-physics')
    ),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    short_name TEXT NOT NULL CHECK (length(trim(short_name)) BETWEEN 1 AND 80),
    authority TEXT NOT NULL DEFAULT '' CHECK (length(authority) <= 300),
    official_url TEXT NOT NULL DEFAULT ''
        CHECK (official_url='' OR official_url LIKE 'https://%'),
    display_order INTEGER NOT NULL UNIQUE CHECK (display_order BETWEEN 1 AND 4),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_profiles (
    exam_id INTEGER PRIMARY KEY REFERENCES exams(id) ON DELETE RESTRICT,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    exam_date TEXT CHECK (
        exam_date IS NULL OR
        (length(exam_date)=10 AND date(exam_date)=exam_date)
    ),
    date_updated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_date_changes (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    old_date TEXT CHECK (old_date IS NULL OR (length(old_date)=10 AND date(old_date)=old_date)),
    new_date TEXT CHECK (new_date IS NULL OR (length(new_date)=10 AND date(new_date)=new_date)),
    changed_at TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '' CHECK (length(reason) <= 500)
) STRICT;

CREATE TABLE backup_settings (
    id INTEGER PRIMARY KEY CHECK (id=1),
    automatic_enabled INTEGER NOT NULL DEFAULT 0 CHECK (automatic_enabled IN (0,1)),
    frequency TEXT NOT NULL DEFAULT 'daily' CHECK (frequency IN ('daily','weekly','manual_only')),
    destination TEXT CHECK (destination IS NULL OR length(destination) BETWEEN 1 AND 2048),
    retention_count INTEGER NOT NULL DEFAULT 5 CHECK (retention_count BETWEEN 1 AND 100),
    last_success_at TEXT,
    last_success_file TEXT,
    last_failure_at TEXT,
    last_failure_message TEXT CHECK (last_failure_message IS NULL OR length(last_failure_message) <= 1000),
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE dashboard_sections (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER REFERENCES exams(id) ON DELETE CASCADE,
    section_key TEXT NOT NULL CHECK (length(section_key) BETWEEN 1 AND 64),
    visible INTEGER NOT NULL DEFAULT 1 CHECK (visible IN (0,1)),
    display_order INTEGER NOT NULL CHECK (display_order BETWEEN 0 AND 1000),
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_content_versions (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    content_type TEXT NOT NULL CHECK (content_type IN ('syllabus','paper_pattern','marking_scheme')),
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) BETWEEN 1 AND 100),
    source_reference TEXT NOT NULL DEFAULT '' CHECK (length(source_reference) <= 2000),
    source_url TEXT CHECK (source_url IS NULL OR source_url LIKE 'https://%'),
    published_on TEXT CHECK (published_on IS NULL OR (length(published_on)=10 AND date(published_on)=published_on)),
    verified_on TEXT CHECK (verified_on IS NULL OR (length(verified_on)=10 AND date(verified_on)=verified_on)),
    review_status TEXT NOT NULL CHECK (review_status IN ('draft','pending','verified','rejected')),
    reviewed_by TEXT NOT NULL DEFAULT '' CHECK (length(reviewed_by) <= 200),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    created_at TEXT NOT NULL,
    UNIQUE (exam_id, content_type, version_label),
    UNIQUE (exam_id, id),
    CHECK (length(trim(source_reference)) > 0 OR source_url IS NOT NULL)
) STRICT;

CREATE TABLE exam_question_types (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 40),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 120),
    answer_format TEXT NOT NULL CHECK (length(trim(answer_format)) BETWEEN 1 AND 80),
    supports_partial_marks INTEGER NOT NULL DEFAULT 0 CHECK (supports_partial_marks IN (0,1)),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, code),
    UNIQUE (exam_id, id)
) STRICT;

CREATE TABLE exam_sections (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    content_version_id INTEGER NOT NULL,
    default_question_type_id INTEGER,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 40),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    display_order INTEGER NOT NULL CHECK (display_order >= 0),
    max_questions INTEGER CHECK (max_questions IS NULL OR max_questions > 0),
    default_correct_marks REAL NOT NULL DEFAULT 1 CHECK (default_correct_marks >= 0),
    default_incorrect_marks REAL NOT NULL DEFAULT 0 CHECK (default_incorrect_marks <= 0),
    default_unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (default_unanswered_marks <= 0),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, code),
    UNIQUE (exam_id, display_order),
    UNIQUE (exam_id, id),
    FOREIGN KEY (exam_id, content_version_id)
        REFERENCES exam_content_versions(exam_id,id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, default_question_type_id)
        REFERENCES exam_question_types(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE exam_syllabus_sources (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    content_version_id INTEGER NOT NULL,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 64),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    publisher TEXT NOT NULL CHECK (length(trim(publisher)) BETWEEN 1 AND 200),
    url TEXT CHECK (url IS NULL OR url LIKE 'https://%'),
    source_reference TEXT NOT NULL DEFAULT '' CHECK (length(source_reference) <= 2000),
    verified_on TEXT NOT NULL CHECK (length(verified_on)=10 AND date(verified_on)=verified_on),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, code),
    UNIQUE (exam_id, id),
    FOREIGN KEY (exam_id, content_version_id)
        REFERENCES exam_content_versions(exam_id,id) ON DELETE RESTRICT,
    CHECK (url IS NOT NULL OR length(trim(source_reference)) > 0)
) STRICT;

CREATE TABLE exam_syllabus_nodes (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    parent_id INTEGER,
    source_id INTEGER NOT NULL,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 120),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    node_kind TEXT NOT NULL CHECK (node_kind IN ('unit','section','topic')),
    category TEXT NOT NULL DEFAULT '' CHECK (length(category) <= 200),
    display_order INTEGER NOT NULL CHECK (display_order >= 0),
    readiness_weight REAL NOT NULL DEFAULT 1 CHECK (readiness_weight > 0 AND readiness_weight <= 100),
    source_reference TEXT NOT NULL DEFAULT '' CHECK (length(source_reference) <= 500),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    archived_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, code),
    UNIQUE (exam_id, id),
    FOREIGN KEY (exam_id, parent_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, source_id)
        REFERENCES exam_syllabus_sources(exam_id,id) ON DELETE RESTRICT,
    CHECK ((is_archived=0 AND archived_at IS NULL) OR (is_archived=1 AND archived_at IS NOT NULL))
) STRICT;

CREATE TABLE exam_syllabus_node_migrations (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    old_node_id INTEGER NOT NULL,
    new_node_id INTEGER NOT NULL,
    content_version_id INTEGER NOT NULL,
    migration_kind TEXT NOT NULL CHECK (migration_kind IN ('rename','split','merge','replacement')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    UNIQUE (exam_id, old_node_id, new_node_id, content_version_id),
    FOREIGN KEY (exam_id, old_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, new_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, content_version_id)
        REFERENCES exam_content_versions(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE concepts (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE CHECK (
        length(slug) BETWEEN 1 AND 120
        AND slug=lower(slug)
        AND slug NOT GLOB '*[^a-z0-9-]*'
    ),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 4000),
    domain TEXT NOT NULL CHECK (length(trim(domain)) BETWEEN 1 AND 120),
    parent_concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    difficulty INTEGER NOT NULL DEFAULT 3 CHECK (difficulty BETWEEN 1 AND 5),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_node_concepts (
    exam_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    weight REAL NOT NULL DEFAULT 1 CHECK (weight > 0 AND weight <= 100),
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 1000),
    created_at TEXT NOT NULL,
    PRIMARY KEY (node_id, concept_id),
    UNIQUE (exam_id, node_id, concept_id),
    FOREIGN KEY (exam_id, node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE CASCADE
) STRICT;

CREATE TABLE exam_node_progress (
    exam_id INTEGER NOT NULL,
    node_id INTEGER PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'not_started'
        CHECK (status IN ('not_started','learning','revised','confident')),
    priority TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('low','normal','high','high_yield')),
    confidence INTEGER NOT NULL DEFAULT 3 CHECK (confidence BETWEEN 1 AND 5),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    last_studied_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, node_id),
    FOREIGN KEY (exam_id, node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE exam_concept_priorities (
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    priority TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('low','normal','high','high_yield')),
    weight REAL NOT NULL DEFAULT 1 CHECK (weight > 0 AND weight <= 100),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (exam_id, concept_id)
) STRICT;

CREATE TABLE mastery_algorithm_versions (
    id INTEGER PRIMARY KEY,
    version TEXT NOT NULL UNIQUE CHECK (length(trim(version)) BETWEEN 1 AND 40),
    parameters_json TEXT NOT NULL CHECK (json_valid(parameters_json)),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 2000),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE mastery_recalc_runs (
    id INTEGER PRIMARY KEY,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL CHECK (
        reason IN ('source_created','source_edited','source_deleted','manual_override','algorithm_change','repair')
    ),
    algorithm_version_id INTEGER REFERENCES mastery_algorithm_versions(id) ON DELETE RESTRICT,
    evidence_count INTEGER NOT NULL CHECK (evidence_count >= 0),
    evidence_fingerprint TEXT NOT NULL CHECK (
        length(evidence_fingerprint)=64
        AND evidence_fingerprint=lower(evidence_fingerprint)
        AND evidence_fingerprint NOT GLOB '*[^0-9a-f]*'
    ),
    resulting_score REAL NOT NULL CHECK (resulting_score BETWEEN 0 AND 100),
    resulting_confidence INTEGER NOT NULL CHECK (resulting_confidence BETWEEN 1 AND 5),
    started_at TEXT NOT NULL,
    finished_at TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000)
) STRICT;

CREATE TABLE concept_mastery (
    concept_id INTEGER PRIMARY KEY REFERENCES concepts(id) ON DELETE RESTRICT,
    mastery_score REAL NOT NULL DEFAULT 0 CHECK (mastery_score BETWEEN 0 AND 100),
    confidence INTEGER NOT NULL DEFAULT 1 CHECK (confidence BETWEEN 1 AND 5),
    last_reviewed_on TEXT CHECK (last_reviewed_on IS NULL OR (length(last_reviewed_on)=10 AND date(last_reviewed_on)=last_reviewed_on)),
    last_evidence_at TEXT,
    evidence_count INTEGER NOT NULL DEFAULT 0 CHECK (evidence_count >= 0),
    last_algorithm_version_id INTEGER REFERENCES mastery_algorithm_versions(id) ON DELETE RESTRICT,
    last_recalc_run_id INTEGER REFERENCES mastery_recalc_runs(id) ON DELETE RESTRICT,
    last_recalculated_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE concept_evidence (
    id INTEGER PRIMARY KEY,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    source_kind TEXT NOT NULL CHECK (
        source_kind IN ('study_session','revision','pyq_attempt','mock_concept_result','manual')
    ),
    source_id INTEGER,
    source_version INTEGER NOT NULL DEFAULT 1 CHECK (source_version >= 1),
    exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    local_date TEXT NOT NULL CHECK (length(local_date)=10 AND date(local_date)=local_date),
    occurred_at TEXT NOT NULL,
    raw_value_json TEXT NOT NULL CHECK (json_valid(raw_value_json)),
    algorithm_version_id INTEGER NOT NULL REFERENCES mastery_algorithm_versions(id) ON DELETE RESTRICT,
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    UNIQUE (source_kind, source_id, concept_id, source_version)
) STRICT;

CREATE TABLE concept_evidence_voids (
    evidence_id INTEGER PRIMARY KEY REFERENCES concept_evidence(id) ON DELETE RESTRICT,
    voided_at TEXT NOT NULL,
    reason TEXT NOT NULL CHECK (reason IN ('source_edited','source_deleted','manual_correction')),
    replacement_evidence_id INTEGER REFERENCES concept_evidence(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE mastery_replay_results (
    id INTEGER PRIMARY KEY,
    recalc_run_id INTEGER NOT NULL REFERENCES mastery_recalc_runs(id) ON DELETE RESTRICT,
    evidence_id INTEGER NOT NULL REFERENCES concept_evidence(id) ON DELETE RESTRICT,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    algorithm_version_id INTEGER NOT NULL REFERENCES mastery_algorithm_versions(id) ON DELETE RESTRICT,
    applied_delta REAL NOT NULL,
    score_before REAL NOT NULL CHECK (score_before BETWEEN 0 AND 100),
    score_after REAL NOT NULL CHECK (score_after BETWEEN 0 AND 100),
    category TEXT NOT NULL CHECK (category IN ('study','revision','pyq','mock','manual')),
    cap_consumed REAL NOT NULL DEFAULT 0 CHECK (cap_consumed >= 0),
    created_at TEXT NOT NULL,
    UNIQUE (recalc_run_id, evidence_id)
) STRICT;

CREATE TABLE study_sessions (
    id INTEGER PRIMARY KEY,
    session_date TEXT NOT NULL CHECK (length(session_date)=10 AND date(session_date)=session_date),
    started_at TEXT,
    ended_at TEXT,
    duration_seconds INTEGER NOT NULL CHECK (duration_seconds BETWEEN 1 AND 86400),
    source TEXT NOT NULL CHECK (source IN ('timer','manual')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    primary_exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    syllabus_node_id INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((started_at IS NULL AND ended_at IS NULL) OR (started_at IS NOT NULL AND ended_at IS NOT NULL AND started_at < ended_at)),
    CHECK (syllabus_node_id IS NULL OR primary_exam_id IS NOT NULL),
    FOREIGN KEY (primary_exam_id, syllabus_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE session_exam_allocations (
    session_id INTEGER NOT NULL REFERENCES study_sessions(id) ON DELETE CASCADE,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    allocated_seconds INTEGER NOT NULL CHECK (allocated_seconds > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (session_id, exam_id)
) STRICT;

CREATE TABLE checklist_tasks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 2000),
    scheduled_date TEXT NOT NULL CHECK (length(scheduled_date)=10 AND date(scheduled_date)=scheduled_date),
    deadline_date TEXT CHECK (deadline_date IS NULL OR (length(deadline_date)=10 AND date(deadline_date)=deadline_date)),
    exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    syllabus_node_id INTEGER,
    priority TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low','normal','high','high_yield')),
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','completed')),
    schedule_anchor TEXT NOT NULL DEFAULT 'manual' CHECK (schedule_anchor IN ('manual','exam')),
    schedule_offset_days INTEGER CHECK (schedule_offset_days IS NULL OR schedule_offset_days BETWEEN -3650 AND 3650),
    completed_at TEXT,
    linked_session_id INTEGER REFERENCES study_sessions(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((schedule_anchor='manual' AND schedule_offset_days IS NULL) OR (schedule_anchor='exam' AND schedule_offset_days IS NOT NULL AND exam_id IS NOT NULL)),
    CHECK (syllabus_node_id IS NULL OR exam_id IS NOT NULL),
    CHECK ((status='open' AND completed_at IS NULL) OR (status='completed' AND completed_at IS NOT NULL)),
    FOREIGN KEY (exam_id, syllabus_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE planned_study_blocks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    syllabus_node_id INTEGER,
    calendar_date TEXT NOT NULL CHECK (length(calendar_date)=10 AND date(calendar_date)=calendar_date),
    start_time TEXT NOT NULL CHECK (
        length(start_time)=5
        AND start_time GLOB '[0-9][0-9]:[0-9][0-9]'
        AND CAST(substr(start_time,1,2) AS INTEGER) BETWEEN 0 AND 23
        AND CAST(substr(start_time,4,2) AS INTEGER) BETWEEN 0 AND 59
    ),
    end_time TEXT NOT NULL CHECK (
        length(end_time)=5
        AND end_time GLOB '[0-9][0-9]:[0-9][0-9]'
        AND CAST(substr(end_time,1,2) AS INTEGER) BETWEEN 0 AND 23
        AND CAST(substr(end_time,4,2) AS INTEGER) BETWEEN 0 AND 59
    ),
    status TEXT NOT NULL DEFAULT 'planned' CHECK (status IN ('planned','in_progress','completed','cancelled')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    linked_session_id INTEGER REFERENCES study_sessions(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (start_time < end_time),
    CHECK (syllabus_node_id IS NULL OR exam_id IS NOT NULL),
    CHECK (linked_session_id IS NULL OR status IN ('in_progress','completed')),
    FOREIGN KEY (exam_id, syllabus_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE timer_settings (
    id INTEGER PRIMARY KEY CHECK (id=1),
    focus_minutes INTEGER NOT NULL DEFAULT 25 CHECK (focus_minutes BETWEEN 1 AND 240),
    short_break_minutes INTEGER NOT NULL DEFAULT 5 CHECK (short_break_minutes BETWEEN 1 AND 60),
    long_break_minutes INTEGER NOT NULL DEFAULT 15 CHECK (long_break_minutes BETWEEN 1 AND 120),
    notifications_enabled INTEGER NOT NULL DEFAULT 0 CHECK (notifications_enabled IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE revision_settings (
    id INTEGER PRIMARY KEY CHECK (id=1),
    initial_again_days INTEGER NOT NULL DEFAULT 1 CHECK (initial_again_days BETWEEN 1 AND 3650),
    initial_difficult_days INTEGER NOT NULL DEFAULT 2 CHECK (initial_difficult_days BETWEEN 1 AND 3650),
    initial_good_days INTEGER NOT NULL DEFAULT 5 CHECK (initial_good_days BETWEEN 1 AND 3650),
    initial_easy_days INTEGER NOT NULL DEFAULT 10 CHECK (initial_easy_days BETWEEN 1 AND 3650),
    growth_multiplier REAL NOT NULL DEFAULT 1.5 CHECK (growth_multiplier BETWEEN 1.0 AND 10.0),
    minimum_interval_days INTEGER NOT NULL DEFAULT 1 CHECK (minimum_interval_days BETWEEN 1 AND 3650),
    maximum_interval_days INTEGER NOT NULL DEFAULT 3650 CHECK (maximum_interval_days BETWEEN 1 AND 3650),
    include_weekends INTEGER NOT NULL DEFAULT 1 CHECK (include_weekends IN (0,1)),
    daily_revision_target INTEGER NOT NULL DEFAULT 5 CHECK (daily_revision_target BETWEEN 1 AND 100),
    reminders_enabled INTEGER NOT NULL DEFAULT 0 CHECK (reminders_enabled IN (0,1)),
    last_reminder_date TEXT CHECK (last_reminder_date IS NULL OR (length(last_reminder_date)=10 AND date(last_reminder_date)=last_reminder_date)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (minimum_interval_days <= maximum_interval_days),
    CHECK (initial_again_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_difficult_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_good_days BETWEEN minimum_interval_days AND maximum_interval_days),
    CHECK (initial_easy_days BETWEEN minimum_interval_days AND maximum_interval_days)
) STRICT;

CREATE TABLE revision_schedules (
    concept_id INTEGER PRIMARY KEY REFERENCES concepts(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','suspended')),
    last_revision_date TEXT CHECK (last_revision_date IS NULL OR (length(last_revision_date)=10 AND date(last_revision_date)=last_revision_date)),
    next_revision_date TEXT NOT NULL CHECK (length(next_revision_date)=10 AND date(next_revision_date)=next_revision_date),
    current_interval_days INTEGER NOT NULL CHECK (current_interval_days BETWEEN 1 AND 3650),
    completed_revisions INTEGER NOT NULL DEFAULT 0 CHECK (completed_revisions >= 0),
    previous_outcome TEXT CHECK (previous_outcome IS NULL OR previous_outcome IN ('again','difficult','good','easy')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((completed_revisions=0 AND last_revision_date IS NULL AND previous_outcome IS NULL)
        OR (completed_revisions>0 AND last_revision_date IS NOT NULL AND previous_outcome IS NOT NULL))
) STRICT;

CREATE TABLE revision_history (
    id INTEGER PRIMARY KEY,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    trigger_exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    revision_date TEXT NOT NULL CHECK (length(revision_date)=10 AND date(revision_date)=revision_date),
    scheduled_for_date TEXT NOT NULL CHECK (length(scheduled_for_date)=10 AND date(scheduled_for_date)=scheduled_for_date),
    outcome TEXT NOT NULL CHECK (outcome IN ('again','difficult','good','easy')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    interval_before_days INTEGER NOT NULL CHECK (interval_before_days BETWEEN 1 AND 3650),
    interval_after_days INTEGER NOT NULL CHECK (interval_after_days BETWEEN 1 AND 3650),
    mastery_before REAL NOT NULL CHECK (mastery_before BETWEEN 0 AND 100),
    mastery_after REAL NOT NULL CHECK (mastery_after BETWEEN 0 AND 100),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE revision_daily_plans (
    id INTEGER PRIMARY KEY,
    plan_date TEXT NOT NULL CHECK (length(plan_date)=10 AND date(plan_date)=plan_date),
    exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    version INTEGER NOT NULL CHECK (version >= 1),
    effective_budget_minutes INTEGER NOT NULL CHECK (effective_budget_minutes BETWEEN 1 AND 1440),
    max_topics INTEGER NOT NULL CHECK (max_topics BETWEEN 1 AND 100),
    planned_minutes INTEGER NOT NULL DEFAULT 0 CHECK (planned_minutes >= 0),
    completed_minutes REAL NOT NULL DEFAULT 0 CHECK (completed_minutes >= 0),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','superseded','closed')),
    created_at TEXT NOT NULL,
    last_rebalanced_at TEXT,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE revision_plan_items (
    id INTEGER PRIMARY KEY,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    focus_exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    active_daily_plan_id INTEGER REFERENCES revision_daily_plans(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned','in_progress','completed','backlog','deferred','removed','cancelled')),
    estimated_minutes INTEGER NOT NULL CHECK (estimated_minutes BETWEEN 5 AND 240),
    triage_score REAL NOT NULL CHECK (triage_score BETWEEN 0 AND 100),
    lane TEXT NOT NULL CHECK (lane IN ('urgent_review','strengthen','maintain')),
    source_due_date TEXT NOT NULL CHECK (length(source_due_date)=10 AND date(source_due_date)=source_due_date),
    planned_block_id INTEGER REFERENCES planned_study_blocks(id) ON DELETE SET NULL,
    revision_history_id INTEGER REFERENCES revision_history(id) ON DELETE RESTRICT,
    cancelled_at TEXT,
    removed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (status <> 'completed' OR revision_history_id IS NOT NULL),
    CHECK (active_daily_plan_id IS NULL OR status IN ('planned','in_progress','backlog')),
    CHECK ((status='cancelled' AND cancelled_at IS NOT NULL) OR (status<>'cancelled' AND cancelled_at IS NULL))
) STRICT;

CREATE TABLE revision_daily_plan_items (
    daily_plan_id INTEGER NOT NULL REFERENCES revision_daily_plans(id) ON DELETE RESTRICT,
    plan_item_id INTEGER NOT NULL REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    position INTEGER NOT NULL CHECK (position >= 0),
    estimated_minutes INTEGER NOT NULL CHECK (estimated_minutes BETWEEN 5 AND 240),
    lane TEXT NOT NULL CHECK (lane IN ('urgent_review','strengthen','maintain')),
    triage_score REAL NOT NULL CHECK (triage_score BETWEEN 0 AND 100),
    membership_status TEXT NOT NULL DEFAULT 'planned'
        CHECK (membership_status IN ('planned','in_progress','completed','deferred','removed','cancelled')),
    planned_block_id INTEGER,
    planned_block_title TEXT,
    planned_block_date TEXT,
    planned_block_start_time TEXT,
    planned_block_end_time TEXT,
    carried_from_daily_plan_id INTEGER REFERENCES revision_daily_plans(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (daily_plan_id, plan_item_id)
) STRICT;

CREATE TABLE revision_deferrals (
    id INTEGER PRIMARY KEY,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    plan_item_id INTEGER REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    event_kind TEXT NOT NULL CHECK (event_kind IN ('defer','override')),
    overridden_deferral_id INTEGER REFERENCES revision_deferrals(id) ON DELETE RESTRICT,
    previous_due_date TEXT NOT NULL CHECK (length(previous_due_date)=10 AND date(previous_due_date)=previous_due_date),
    new_due_date TEXT NOT NULL CHECK (length(new_due_date)=10 AND date(new_due_date)=new_due_date),
    occurred_on TEXT NOT NULL CHECK (length(occurred_on)=10 AND date(occurred_on)=occurred_on),
    reason TEXT NOT NULL CHECK (reason IN ('time','difficulty','energy','life','other')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    CHECK ((event_kind='defer' AND new_due_date > previous_due_date) OR (event_kind='override' AND new_due_date < previous_due_date))
) STRICT;

CREATE TABLE revision_session_links (
    plan_item_id INTEGER NOT NULL REFERENCES revision_plan_items(id) ON DELETE RESTRICT,
    study_session_id INTEGER NOT NULL REFERENCES study_sessions(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (plan_item_id, study_session_id),
    UNIQUE (study_session_id)
) STRICT;

CREATE TABLE timer_state (
    id INTEGER PRIMARY KEY CHECK (id=1),
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle','running','paused')),
    mode TEXT NOT NULL DEFAULT 'focus' CHECK (mode IN ('focus','short_break','long_break')),
    timer_instance_id TEXT CHECK (timer_instance_id IS NULL OR length(timer_instance_id) BETWEEN 1 AND 64),
    duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (duration_ms BETWEEN 0 AND 86400000),
    accumulated_ms INTEGER NOT NULL DEFAULT 0 CHECK (accumulated_ms >= 0 AND accumulated_ms <= duration_ms),
    segment_started_at TEXT,
    last_heartbeat_at TEXT,
    session_date TEXT CHECK (session_date IS NULL OR (length(session_date)=10 AND date(session_date)=session_date)),
    active_exam_id INTEGER REFERENCES exams(id) ON DELETE RESTRICT,
    concept_id INTEGER REFERENCES concepts(id) ON DELETE RESTRICT,
    syllabus_node_id INTEGER,
    planned_block_id INTEGER REFERENCES planned_study_blocks(id) ON DELETE SET NULL,
    revision_plan_item_id INTEGER REFERENCES revision_plan_items(id) ON DELETE SET NULL,
    recovery_reason TEXT CHECK (recovery_reason IN ('restart','inactivity','clock_change','sleep')),
    recovery_excluded_ms INTEGER NOT NULL DEFAULT 0 CHECK (recovery_excluded_ms >= 0),
    clock_skew_detected INTEGER NOT NULL DEFAULT 0 CHECK (clock_skew_detected IN (0,1)),
    last_event TEXT CHECK (last_event IN ('completed','discarded')),
    last_event_at TEXT,
    last_event_duration_ms INTEGER NOT NULL DEFAULT 0 CHECK (last_event_duration_ms >= 0),
    last_completed_session_id INTEGER REFERENCES study_sessions(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((status='idle' AND accumulated_ms=0 AND segment_started_at IS NULL)
        OR (status='running' AND segment_started_at IS NOT NULL AND last_heartbeat_at IS NOT NULL AND timer_instance_id IS NOT NULL)
        OR (status='paused' AND segment_started_at IS NULL)),
    CHECK (syllabus_node_id IS NULL OR active_exam_id IS NOT NULL),
    FOREIGN KEY (active_exam_id, syllabus_node_id)
        REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE pyq_questions (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    section_id INTEGER NOT NULL,
    question_type_id INTEGER NOT NULL,
    year INTEGER NOT NULL CHECK (year BETWEEN 1900 AND 2100),
    session_label TEXT NOT NULL DEFAULT '' CHECK (length(session_label) <= 120),
    question_number TEXT NOT NULL CHECK (length(trim(question_number)) BETWEEN 1 AND 60),
    statement TEXT NOT NULL DEFAULT '' CHECK (length(statement) <= 20000),
    correct_answer TEXT NOT NULL DEFAULT '' CHECK (length(correct_answer) <= 4000),
    explanation TEXT NOT NULL DEFAULT '' CHECK (length(explanation) <= 20000),
    max_marks REAL NOT NULL CHECK (max_marks > 0 AND max_marks <= 100000),
    negative_marks REAL NOT NULL DEFAULT 0 CHECK (negative_marks <= 0),
    unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks <= 0),
    difficulty INTEGER NOT NULL DEFAULT 3 CHECK (difficulty BETWEEN 1 AND 5),
    source_url TEXT CHECK (source_url IS NULL OR source_url LIKE 'https://%'),
    dedupe_key TEXT CHECK (dedupe_key IS NULL OR length(dedupe_key) <= 200),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, id),
    UNIQUE (exam_id, year, session_label, section_id, question_number),
    UNIQUE (exam_id, dedupe_key),
    FOREIGN KEY (exam_id, section_id) REFERENCES exam_sections(exam_id,id) ON DELETE RESTRICT,
    FOREIGN KEY (exam_id, question_type_id) REFERENCES exam_question_types(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE pyq_question_concepts (
    exam_id INTEGER NOT NULL,
    question_id INTEGER NOT NULL,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    weight REAL NOT NULL DEFAULT 1 CHECK (weight > 0 AND weight <= 100),
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
    created_at TEXT NOT NULL,
    PRIMARY KEY (question_id, concept_id),
    UNIQUE (exam_id, question_id, concept_id),
    FOREIGN KEY (exam_id, question_id) REFERENCES pyq_questions(exam_id,id) ON DELETE CASCADE
) STRICT;

CREATE TABLE pyq_attempts (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL,
    question_id INTEGER NOT NULL,
    attempt_date TEXT NOT NULL CHECK (length(attempt_date)=10 AND date(attempt_date)=attempt_date),
    selected_answer TEXT NOT NULL DEFAULT '' CHECK (length(selected_answer) <= 4000),
    status TEXT NOT NULL CHECK (status IN ('attempted','correct','incorrect','skipped')),
    marks_obtained REAL,
    max_marks_at_attempt REAL NOT NULL CHECK (max_marks_at_attempt > 0),
    negative_marks_at_attempt REAL NOT NULL CHECK (negative_marks_at_attempt <= 0),
    time_spent_seconds INTEGER NOT NULL DEFAULT 0 CHECK (time_spent_seconds BETWEEN 0 AND 86400),
    confidence INTEGER CHECK (confidence BETWEEN 1 AND 5),
    mistakes TEXT NOT NULL DEFAULT '' CHECK (length(mistakes) <= 4000),
    review_status TEXT NOT NULL DEFAULT 'unreviewed' CHECK (review_status IN ('unreviewed','reviewed')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (status <> 'skipped' OR marks_obtained = 0),
    FOREIGN KEY (exam_id, question_id) REFERENCES pyq_questions(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE mock_tests (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    test_name TEXT NOT NULL CHECK (length(trim(test_name)) BETWEEN 1 AND 200),
    test_date TEXT NOT NULL CHECK (length(test_date)=10 AND date(test_date)=test_date),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes BETWEEN 1 AND 1440),
    max_marks REAL NOT NULL CHECK (max_marks > 0),
    marks_obtained REAL CHECK (marks_obtained IS NULL OR marks_obtained BETWEEN -max_marks AND max_marks),
    status TEXT NOT NULL DEFAULT 'planned' CHECK (status IN ('planned','in_progress','completed','cancelled')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    review_observations TEXT NOT NULL DEFAULT '' CHECK (length(review_observations) <= 4000),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, id)
) STRICT;

CREATE TABLE mock_test_sections (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL,
    mock_test_id INTEGER NOT NULL,
    section_id INTEGER NOT NULL,
    question_count INTEGER NOT NULL CHECK (question_count >= 0),
    correct INTEGER NOT NULL DEFAULT 0 CHECK (correct >= 0),
    incorrect INTEGER NOT NULL DEFAULT 0 CHECK (incorrect >= 0),
    unanswered INTEGER NOT NULL DEFAULT 0 CHECK (unanswered >= 0),
    max_marks REAL NOT NULL CHECK (max_marks >= 0),
    marks_obtained REAL NOT NULL DEFAULT 0 CHECK (marks_obtained BETWEEN -max_marks AND max_marks),
    time_spent_minutes INTEGER CHECK (time_spent_minutes IS NULL OR time_spent_minutes BETWEEN 0 AND 1440),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (mock_test_id, section_id),
    CHECK (correct + incorrect + unanswered = question_count),
    FOREIGN KEY (exam_id, mock_test_id) REFERENCES mock_tests(exam_id,id) ON DELETE CASCADE,
    FOREIGN KEY (exam_id, section_id) REFERENCES exam_sections(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE mock_test_concept_results (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL,
    mock_test_id INTEGER NOT NULL,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    correct INTEGER NOT NULL DEFAULT 0 CHECK (correct >= 0),
    incorrect INTEGER NOT NULL DEFAULT 0 CHECK (incorrect >= 0),
    unanswered INTEGER NOT NULL DEFAULT 0 CHECK (unanswered >= 0),
    marks_obtained REAL NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    UNIQUE (mock_test_id, concept_id),
    FOREIGN KEY (exam_id, mock_test_id) REFERENCES mock_tests(exam_id,id) ON DELETE CASCADE
) STRICT;

CREATE TABLE mock_test_score_history (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL,
    mock_test_id INTEGER NOT NULL,
    version INTEGER NOT NULL CHECK (version >= 1),
    max_marks REAL NOT NULL CHECK (max_marks > 0),
    marks_obtained REAL NOT NULL CHECK (marks_obtained BETWEEN -max_marks AND max_marks),
    sections_json TEXT NOT NULL CHECK (json_valid(sections_json)),
    marking_context_json TEXT NOT NULL CHECK (json_valid(marking_context_json)),
    recorded_at TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '' CHECK (length(reason) <= 500),
    UNIQUE (mock_test_id, version),
    FOREIGN KEY (exam_id, mock_test_id) REFERENCES mock_tests(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE readiness_formula_versions (
    id INTEGER PRIMARY KEY,
    version TEXT NOT NULL UNIQUE CHECK (length(trim(version)) BETWEEN 1 AND 40),
    parameters_json TEXT NOT NULL CHECK (json_valid(parameters_json)),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 2000),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_readiness_snapshots (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    as_of_date TEXT NOT NULL CHECK (length(as_of_date)=10 AND date(as_of_date)=as_of_date),
    formula_version_id INTEGER NOT NULL REFERENCES readiness_formula_versions(id) ON DELETE RESTRICT,
    mastery_score REAL CHECK (mastery_score BETWEEN 0 AND 100),
    coverage_score REAL CHECK (coverage_score BETWEEN 0 AND 100),
    pyq_score REAL CHECK (pyq_score BETWEEN 0 AND 100),
    mock_score REAL CHECK (mock_score BETWEEN 0 AND 100),
    revision_score REAL CHECK (revision_score BETWEEN 0 AND 100),
    overall_score REAL CHECK (overall_score BETWEEN 0 AND 100),
    mastery_evidence_count INTEGER NOT NULL DEFAULT 0 CHECK (mastery_evidence_count >= 0),
    coverage_topic_count INTEGER NOT NULL DEFAULT 0 CHECK (coverage_topic_count >= 0),
    pyq_attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (pyq_attempt_count >= 0),
    mock_count INTEGER NOT NULL DEFAULT 0 CHECK (mock_count >= 0),
    revision_event_count INTEGER NOT NULL DEFAULT 0 CHECK (revision_event_count >= 0),
    confidence TEXT NOT NULL CHECK (confidence IN ('insufficient','low','medium','high')),
    components_json TEXT NOT NULL CHECK (json_valid(components_json)),
    computed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK (overall_score IS NULL OR (mastery_score IS NOT NULL AND coverage_score IS NOT NULL AND pyq_score IS NOT NULL AND mock_score IS NOT NULL AND revision_score IS NOT NULL))
) STRICT;

CREATE TABLE resources (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 300),
    description TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 4000),
    subject_area TEXT NOT NULL DEFAULT '' CHECK (length(subject_area) <= 200),
    url TEXT CHECK (url IS NULL OR url LIKE 'http://%' OR url LIKE 'https://%'),
    resource_type TEXT NOT NULL CHECK (resource_type IN ('book','notes','video','paper','website','other')),
    source TEXT NOT NULL DEFAULT '' CHECK (length(source) <= 500),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (url)
) STRICT;

CREATE TABLE resource_concepts (
    resource_id INTEGER NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    concept_id INTEGER NOT NULL REFERENCES concepts(id) ON DELETE RESTRICT,
    weight REAL NOT NULL DEFAULT 1 CHECK (weight > 0 AND weight <= 100),
    created_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, concept_id)
) STRICT;

CREATE TABLE resource_exam_links (
    resource_id INTEGER NOT NULL REFERENCES resources(id) ON DELETE CASCADE,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    category TEXT NOT NULL CHECK (length(trim(category)) BETWEEN 1 AND 80),
    syllabus_node_id INTEGER,
    priority TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low','normal','high','high_yield')),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (resource_id, exam_id),
    CHECK (syllabus_node_id IS NULL OR exam_id IS NOT NULL),
    FOREIGN KEY (exam_id, syllabus_node_id) REFERENCES exam_syllabus_nodes(exam_id,id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE resource_progress (
    resource_id INTEGER PRIMARY KEY REFERENCES resources(id) ON DELETE CASCADE,
    study_status TEXT NOT NULL DEFAULT 'not_started' CHECK (study_status IN ('not_started','studying','completed','skipped')),
    personal_notes TEXT NOT NULL DEFAULT '' CHECK (length(personal_notes) <= 4000),
    last_opened_at TEXT,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE resource_files (
    resource_id INTEGER PRIMARY KEY REFERENCES resources(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL CHECK (length(trim(file_name)) BETWEEN 1 AND 260),
    file_path TEXT NOT NULL UNIQUE CHECK (length(trim(file_path)) BETWEEN 1 AND 1000),
    file_path_normalized TEXT NOT NULL UNIQUE CHECK (length(trim(file_path_normalized)) BETWEEN 1 AND 1000),
    sha256 TEXT NOT NULL CHECK (length(sha256)=64 AND sha256=lower(sha256) AND sha256 NOT GLOB '*[^0-9a-f]*'),
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    storage_mode TEXT NOT NULL CHECK (storage_mode IN ('managed','external')),
    status TEXT NOT NULL DEFAULT 'ok' CHECK (status IN ('ok','missing','changed','permission_denied')),
    imported_at TEXT NOT NULL,
    last_verified_at TEXT,
    missing_since TEXT,
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000),
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE resource_file_history (
    id INTEGER PRIMARY KEY,
    resource_id INTEGER NOT NULL REFERENCES resources(id) ON DELETE RESTRICT,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    storage_mode TEXT NOT NULL CHECK (storage_mode IN ('managed','external')),
    action TEXT NOT NULL CHECK (action IN ('import','replace','relink','remove','missing','changed','restore')),
    occurred_at TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 2000)
) STRICT;
```

<!-- SCHEMA-TABLES:END -->

### 7.4 Indexes

<!-- SCHEMA-INDEXES:BEGIN -->

```sql
CREATE INDEX idx_exam_profiles_active ON exam_profiles(is_active, exam_date);
CREATE INDEX idx_exam_date_changes_exam ON exam_date_changes(exam_id, changed_at DESC);

CREATE UNIQUE INDEX ux_dashboard_combined_section ON dashboard_sections(section_key) WHERE exam_id IS NULL;
CREATE UNIQUE INDEX ux_dashboard_exam_section ON dashboard_sections(exam_id, section_key) WHERE exam_id IS NOT NULL;
CREATE UNIQUE INDEX ux_dashboard_combined_order ON dashboard_sections(display_order) WHERE exam_id IS NULL;
CREATE UNIQUE INDEX ux_dashboard_exam_order ON dashboard_sections(exam_id, display_order) WHERE exam_id IS NOT NULL;

CREATE INDEX idx_exam_content_exam_type ON exam_content_versions(exam_id, content_type, review_status);
CREATE INDEX idx_exam_sections_exam_order ON exam_sections(exam_id, display_order);
CREATE INDEX idx_exam_syllabus_nodes_exam_parent_order ON exam_syllabus_nodes(exam_id, parent_id, display_order);
CREATE INDEX idx_exam_syllabus_nodes_exam_kind ON exam_syllabus_nodes(exam_id, node_kind);
CREATE INDEX idx_exam_node_concepts_concept ON exam_node_concepts(concept_id, node_id);
CREATE INDEX idx_exam_node_progress_status ON exam_node_progress(status, priority);
CREATE INDEX idx_exam_concept_priorities_exam_priority ON exam_concept_priorities(exam_id, priority, concept_id);

CREATE INDEX idx_concepts_domain_title ON concepts(domain, title COLLATE NOCASE);
CREATE INDEX idx_concepts_parent ON concepts(parent_concept_id);

CREATE INDEX idx_concept_evidence_concept_time ON concept_evidence(concept_id, occurred_at);
CREATE INDEX idx_concept_evidence_exam_time ON concept_evidence(exam_id, occurred_at);
CREATE INDEX idx_concept_evidence_concept_day ON concept_evidence(concept_id, local_date);
CREATE INDEX idx_concept_evidence_voids_evidence ON concept_evidence_voids(evidence_id);
CREATE INDEX idx_mastery_recalc_concept_time ON mastery_recalc_runs(concept_id, finished_at DESC);
CREATE INDEX idx_mastery_replay_run ON mastery_replay_results(recalc_run_id, evidence_id);
CREATE INDEX idx_mastery_replay_concept ON mastery_replay_results(concept_id, created_at DESC);

CREATE INDEX idx_study_sessions_date ON study_sessions(session_date, id);
CREATE INDEX idx_study_sessions_exam_date ON study_sessions(primary_exam_id, session_date);
CREATE INDEX idx_study_sessions_concept_date ON study_sessions(concept_id, session_date);
CREATE INDEX idx_study_sessions_node_date ON study_sessions(syllabus_node_id, session_date);
CREATE INDEX idx_session_allocations_exam ON session_exam_allocations(exam_id, session_id);

CREATE INDEX idx_checklist_tasks_date_status ON checklist_tasks(scheduled_date, status);
CREATE INDEX idx_checklist_tasks_exam ON checklist_tasks(exam_id, scheduled_date);
CREATE INDEX idx_checklist_tasks_concept ON checklist_tasks(concept_id, scheduled_date);
CREATE INDEX idx_planned_blocks_date ON planned_study_blocks(calendar_date, start_time, id);
CREATE INDEX idx_planned_blocks_exam ON planned_study_blocks(exam_id, calendar_date);
CREATE INDEX idx_planned_blocks_concept ON planned_study_blocks(concept_id, calendar_date);

CREATE INDEX idx_revision_schedules_due ON revision_schedules(status, next_revision_date, concept_id);
CREATE INDEX idx_revision_history_concept_date ON revision_history(concept_id, revision_date DESC, id DESC);
CREATE INDEX idx_revision_history_date ON revision_history(revision_date DESC, id DESC);
CREATE INDEX idx_revision_plan_active_daily ON revision_plan_items(active_daily_plan_id) WHERE active_daily_plan_id IS NOT NULL;
CREATE UNIQUE INDEX ux_revision_plan_active_concept ON revision_plan_items(concept_id)
    WHERE status IN ('planned','in_progress','backlog','deferred');
CREATE UNIQUE INDEX ux_revision_plan_combined_version ON revision_daily_plans(plan_date, version) WHERE exam_id IS NULL;
CREATE UNIQUE INDEX ux_revision_plan_exam_version ON revision_daily_plans(plan_date, exam_id, version) WHERE exam_id IS NOT NULL;
CREATE UNIQUE INDEX ux_revision_plan_combined_active ON revision_daily_plans(plan_date) WHERE exam_id IS NULL AND status='active';
CREATE UNIQUE INDEX ux_revision_plan_exam_active ON revision_daily_plans(plan_date, exam_id) WHERE exam_id IS NOT NULL AND status='active';
CREATE INDEX idx_revision_plan_items_daily ON revision_daily_plan_items(daily_plan_id, position);
CREATE INDEX idx_revision_deferrals_concept_date ON revision_deferrals(concept_id, occurred_on DESC, id DESC);
CREATE INDEX idx_revision_session_links_session ON revision_session_links(study_session_id);

CREATE INDEX idx_pyq_questions_exam_year_section ON pyq_questions(exam_id, year, section_id);
CREATE INDEX idx_pyq_questions_archived ON pyq_questions(exam_id, is_archived);
CREATE INDEX idx_pyq_question_concepts_concept ON pyq_question_concepts(concept_id, question_id);
CREATE INDEX idx_pyq_attempts_question_date ON pyq_attempts(question_id, attempt_date DESC, id DESC);
CREATE INDEX idx_pyq_attempts_exam_date ON pyq_attempts(exam_id, attempt_date DESC, id DESC);

CREATE INDEX idx_mock_tests_exam_date_status ON mock_tests(exam_id, test_date DESC, status);
CREATE INDEX idx_mock_test_sections_mock ON mock_test_sections(mock_test_id, section_id);
CREATE INDEX idx_mock_concept_results_mock ON mock_test_concept_results(mock_test_id, concept_id);
CREATE INDEX idx_mock_score_history_mock ON mock_test_score_history(mock_test_id, version DESC);

CREATE INDEX idx_readiness_snapshots_exam_date ON exam_readiness_snapshots(exam_id, as_of_date DESC, computed_at DESC);

CREATE INDEX idx_resources_title ON resources(title COLLATE NOCASE);
CREATE INDEX idx_resources_type ON resources(resource_type, is_archived);
CREATE INDEX idx_resource_exam_links_exam ON resource_exam_links(exam_id, category);
CREATE INDEX idx_resource_concepts_concept ON resource_concepts(concept_id, resource_id);
CREATE INDEX idx_resource_files_status ON resource_files(status);
CREATE INDEX idx_resource_files_sha ON resource_files(sha256);
CREATE INDEX idx_resource_file_history_resource ON resource_file_history(resource_id, occurred_at DESC);
```

<!-- SCHEMA-INDEXES:END -->

### 7.5 Triggers

<!-- SCHEMA-TRIGGERS:BEGIN -->

```sql
CREATE TRIGGER trg_schema_migrations_no_update BEFORE UPDATE ON schema_migrations
BEGIN SELECT RAISE(ABORT,'schema migrations are immutable'); END;

CREATE TRIGGER trg_schema_migrations_no_delete BEFORE DELETE ON schema_migrations
BEGIN SELECT RAISE(ABORT,'schema migrations are immutable'); END;

CREATE TRIGGER trg_exams_no_delete BEFORE DELETE ON exams
BEGIN SELECT RAISE(ABORT,'built-in exams cannot be deleted'); END;

CREATE TRIGGER trg_exams_code_immutable BEFORE UPDATE OF code ON exams
WHEN OLD.code <> NEW.code
BEGIN SELECT RAISE(ABORT,'exam codes are immutable'); END;

CREATE TRIGGER trg_exam_profiles_no_delete BEFORE DELETE ON exam_profiles
BEGIN SELECT RAISE(ABORT,'exam profiles cannot be deleted; deactivate instead'); END;

CREATE TRIGGER trg_exam_date_changes_no_update BEFORE UPDATE ON exam_date_changes
BEGIN SELECT RAISE(ABORT,'exam date history is append-only'); END;

CREATE TRIGGER trg_exam_date_changes_no_delete BEFORE DELETE ON exam_date_changes
BEGIN SELECT RAISE(ABORT,'exam date history is append-only'); END;

CREATE TRIGGER trg_syllabus_code_immutable BEFORE UPDATE OF code ON exam_syllabus_nodes
WHEN OLD.code <> NEW.code
BEGIN SELECT RAISE(ABORT,'syllabus node codes are stable; create a replacement node'); END;

CREATE TRIGGER trg_session_allocation_insert BEFORE INSERT ON session_exam_allocations
BEGIN
    SELECT CASE WHEN (
        SELECT COALESCE(SUM(a.allocated_seconds),0) + NEW.allocated_seconds
        FROM session_exam_allocations a WHERE a.session_id=NEW.session_id
    ) > (SELECT duration_seconds FROM study_sessions WHERE id=NEW.session_id)
    THEN RAISE(ABORT,'allocation exceeds session duration') END;
END;

CREATE TRIGGER trg_session_allocation_update BEFORE UPDATE ON session_exam_allocations
BEGIN
    SELECT CASE WHEN (
        SELECT COALESCE(SUM(a.allocated_seconds),0) + NEW.allocated_seconds
        FROM session_exam_allocations a
        WHERE a.session_id=NEW.session_id
          AND NOT (a.session_id=OLD.session_id AND a.exam_id=OLD.exam_id)
    ) > (SELECT duration_seconds FROM study_sessions WHERE id=NEW.session_id)
    THEN RAISE(ABORT,'allocation exceeds session duration') END;
END;

CREATE TRIGGER trg_study_session_duration_guard BEFORE UPDATE OF duration_seconds ON study_sessions
WHEN NEW.duration_seconds < (
    SELECT COALESCE(SUM(allocated_seconds),0) FROM session_exam_allocations WHERE session_id=NEW.id
)
BEGIN SELECT RAISE(ABORT,'session duration cannot be less than allocated total'); END;

CREATE TRIGGER trg_revision_history_no_update BEFORE UPDATE ON revision_history
BEGIN SELECT RAISE(ABORT,'revision history is append-only'); END;

CREATE TRIGGER trg_revision_history_no_delete BEFORE DELETE ON revision_history
BEGIN SELECT RAISE(ABORT,'revision history is append-only'); END;

CREATE TRIGGER trg_revision_deferrals_no_update BEFORE UPDATE ON revision_deferrals
BEGIN SELECT RAISE(ABORT,'revision deferrals are append-only'); END;

CREATE TRIGGER trg_revision_deferrals_no_delete BEFORE DELETE ON revision_deferrals
BEGIN SELECT RAISE(ABORT,'revision deferrals are append-only'); END;

CREATE TRIGGER trg_revision_plan_completed_no_update BEFORE UPDATE ON revision_plan_items
WHEN OLD.status='completed'
BEGIN SELECT RAISE(ABORT,'completed revision plan items are immutable'); END;

CREATE TRIGGER trg_revision_plan_completed_no_delete BEFORE DELETE ON revision_plan_items
WHEN OLD.status='completed'
BEGIN SELECT RAISE(ABORT,'completed revision plan items cannot be deleted'); END;

CREATE TRIGGER trg_revision_plan_item_active_plan_guard_insert BEFORE INSERT ON revision_daily_plan_items
WHEN EXISTS (
    SELECT 1 FROM revision_plan_items i
    WHERE i.id=NEW.plan_item_id
      AND i.active_daily_plan_id IS NOT NULL
      AND i.active_daily_plan_id<>NEW.daily_plan_id
)
BEGIN SELECT RAISE(ABORT,'plan item already belongs to an active plan'); END;

CREATE TRIGGER trg_revision_plan_item_active_plan_guard_update BEFORE UPDATE ON revision_daily_plan_items
WHEN EXISTS (
    SELECT 1 FROM revision_plan_items i
    WHERE i.id=NEW.plan_item_id
      AND i.active_daily_plan_id IS NOT NULL
      AND i.active_daily_plan_id<>NEW.daily_plan_id
)
BEGIN SELECT RAISE(ABORT,'plan item already belongs to an active plan'); END;

CREATE TRIGGER trg_revision_daily_plan_items_active_parent BEFORE INSERT ON revision_daily_plan_items
WHEN (SELECT status FROM revision_daily_plans WHERE id=NEW.daily_plan_id) <> 'active'
BEGIN SELECT RAISE(ABORT,'daily plan is not active'); END;

CREATE TRIGGER trg_revision_daily_plan_items_no_delete BEFORE DELETE ON revision_daily_plan_items
BEGIN SELECT RAISE(ABORT,'daily plan memberships are historical; use status'); END;

CREATE TRIGGER trg_revision_daily_plan_items_update_guard BEFORE UPDATE ON revision_daily_plan_items
WHEN (SELECT status FROM revision_daily_plans WHERE id=OLD.daily_plan_id) <> 'active'
  OR OLD.membership_status='completed'
  OR NEW.daily_plan_id<>OLD.daily_plan_id
  OR NEW.plan_item_id<>OLD.plan_item_id
  OR NEW.position<>OLD.position
  OR NEW.estimated_minutes<>OLD.estimated_minutes
  OR NEW.lane<>OLD.lane
  OR NEW.triage_score<>OLD.triage_score
  OR NEW.created_at<>OLD.created_at
BEGIN SELECT RAISE(ABORT,'daily plan membership is immutable or plan is not active'); END;

CREATE TRIGGER trg_revision_plan_close_clear_active AFTER UPDATE OF status ON revision_daily_plans
WHEN OLD.status='active' AND NEW.status<>'active'
BEGIN
    UPDATE revision_plan_items SET active_daily_plan_id=NULL WHERE active_daily_plan_id=OLD.id;
END;

CREATE TRIGGER trg_revision_plan_delete_guard BEFORE DELETE ON revision_daily_plans
WHEN EXISTS (SELECT 1 FROM revision_daily_plan_items WHERE daily_plan_id=OLD.id)
BEGIN SELECT RAISE(ABORT,'daily plans with memberships cannot be deleted'); END;

CREATE TRIGGER trg_revision_deferral_completed_guard BEFORE INSERT ON revision_deferrals
WHEN NEW.plan_item_id IS NOT NULL AND (
    SELECT status FROM revision_plan_items WHERE id=NEW.plan_item_id
)='completed'
BEGIN SELECT RAISE(ABORT,'completed revision item cannot be deferred'); END;

CREATE TRIGGER trg_revision_exam_scope_guard BEFORE INSERT ON revision_daily_plan_items
WHEN (SELECT exam_id FROM revision_daily_plans WHERE id=NEW.daily_plan_id) IS NOT NULL
AND (
    (SELECT focus_exam_id FROM revision_plan_items WHERE id=NEW.plan_item_id)
        IS NOT (SELECT exam_id FROM revision_daily_plans WHERE id=NEW.daily_plan_id)
    OR NOT EXISTS (
        SELECT 1 FROM exam_node_concepts nc
        WHERE nc.exam_id=(SELECT exam_id FROM revision_daily_plans WHERE id=NEW.daily_plan_id)
          AND nc.concept_id=(SELECT concept_id FROM revision_plan_items WHERE id=NEW.plan_item_id)
    )
    OR (SELECT is_active FROM exam_profiles WHERE exam_id=(SELECT exam_id FROM revision_daily_plans WHERE id=NEW.daily_plan_id))<>1
)
BEGIN SELECT RAISE(ABORT,'revision plan item does not belong to the exam scope'); END;

CREATE TRIGGER trg_revision_combined_scope_guard BEFORE INSERT ON revision_daily_plan_items
WHEN (SELECT exam_id FROM revision_daily_plans WHERE id=NEW.daily_plan_id) IS NULL
AND NOT EXISTS (
    SELECT 1 FROM exam_node_concepts nc
    JOIN exam_profiles ep ON ep.exam_id=nc.exam_id
    WHERE nc.concept_id=(SELECT concept_id FROM revision_plan_items WHERE id=NEW.plan_item_id)
      AND ep.is_active=1
)
BEGIN SELECT RAISE(ABORT,'combined plan item is not mapped to an active exam'); END;

CREATE TRIGGER trg_concept_evidence_no_update BEFORE UPDATE ON concept_evidence
BEGIN SELECT RAISE(ABORT,'concept evidence is immutable; void and replace'); END;

CREATE TRIGGER trg_concept_evidence_no_delete BEFORE DELETE ON concept_evidence
BEGIN SELECT RAISE(ABORT,'concept evidence cannot be deleted; void instead'); END;

CREATE TRIGGER trg_concept_evidence_voids_no_update BEFORE UPDATE ON concept_evidence_voids
BEGIN SELECT RAISE(ABORT,'evidence voids are append-only'); END;

CREATE TRIGGER trg_concept_evidence_voids_no_delete BEFORE DELETE ON concept_evidence_voids
BEGIN SELECT RAISE(ABORT,'evidence voids are append-only'); END;

CREATE TRIGGER trg_mastery_recalc_runs_no_update BEFORE UPDATE ON mastery_recalc_runs
BEGIN SELECT RAISE(ABORT,'mastery recalculation history is append-only'); END;

CREATE TRIGGER trg_mastery_recalc_runs_no_delete BEFORE DELETE ON mastery_recalc_runs
BEGIN SELECT RAISE(ABORT,'mastery recalculation history is append-only'); END;

CREATE TRIGGER trg_mastery_replay_results_no_update BEFORE UPDATE ON mastery_replay_results
BEGIN SELECT RAISE(ABORT,'mastery replay results are append-only'); END;

CREATE TRIGGER trg_mastery_replay_results_no_delete BEFORE DELETE ON mastery_replay_results
BEGIN SELECT RAISE(ABORT,'mastery replay results are append-only'); END;

CREATE TRIGGER trg_readiness_snapshot_no_update BEFORE UPDATE ON exam_readiness_snapshots
BEGIN SELECT RAISE(ABORT,'readiness snapshots are immutable'); END;

CREATE TRIGGER trg_readiness_snapshot_no_delete BEFORE DELETE ON exam_readiness_snapshots
BEGIN SELECT RAISE(ABORT,'readiness snapshots are immutable'); END;

CREATE TRIGGER trg_mock_score_history_no_update BEFORE UPDATE ON mock_test_score_history
BEGIN SELECT RAISE(ABORT,'mock score history is append-only'); END;

CREATE TRIGGER trg_mock_score_history_no_delete BEFORE DELETE ON mock_test_score_history
BEGIN SELECT RAISE(ABORT,'mock score history is append-only'); END;

CREATE TRIGGER trg_resource_file_history_no_update BEFORE UPDATE ON resource_file_history
BEGIN SELECT RAISE(ABORT,'resource file history is append-only'); END;

CREATE TRIGGER trg_resource_file_history_no_delete BEFORE DELETE ON resource_file_history
BEGIN SELECT RAISE(ABORT,'resource file history is append-only'); END;

CREATE TRIGGER trg_mock_insert_completed_guard BEFORE INSERT ON mock_tests
WHEN NEW.status='completed'
BEGIN SELECT RAISE(ABORT,'insert mock as planned/in_progress, then complete'); END;

CREATE TRIGGER trg_mock_delete_completed_guard BEFORE DELETE ON mock_tests
WHEN OLD.status='completed'
BEGIN SELECT RAISE(ABORT,'completed mocks cannot be deleted; archive instead'); END;

CREATE TRIGGER trg_mock_completion_validate BEFORE UPDATE OF status ON mock_tests
WHEN NEW.status='completed' AND OLD.status<>'completed'
BEGIN
    SELECT CASE WHEN NEW.marks_obtained IS NULL
        OR (SELECT COUNT(*) FROM mock_test_sections WHERE mock_test_id=NEW.id)=0
        OR (SELECT COALESCE(SUM(max_marks),0) FROM mock_test_sections WHERE mock_test_id=NEW.id)<>NEW.max_marks
        OR (SELECT COALESCE(SUM(marks_obtained),0) FROM mock_test_sections WHERE mock_test_id=NEW.id)<>NEW.marks_obtained
        OR EXISTS (SELECT 1 FROM mock_test_sections WHERE mock_test_id=NEW.id AND question_count=0)
    THEN RAISE(ABORT,'mock totals are incomplete or inconsistent') END;
END;

CREATE TRIGGER trg_mock_section_insert_completed_guard BEFORE INSERT ON mock_test_sections
WHEN (SELECT status FROM mock_tests WHERE id=NEW.mock_test_id)='completed'
BEGIN SELECT RAISE(ABORT,'reopen mock before editing sections'); END;

CREATE TRIGGER trg_mock_section_update_completed_guard BEFORE UPDATE ON mock_test_sections
WHEN (SELECT status FROM mock_tests WHERE id=OLD.mock_test_id)='completed'
BEGIN SELECT RAISE(ABORT,'reopen mock before editing sections'); END;

CREATE TRIGGER trg_mock_section_delete_completed_guard BEFORE DELETE ON mock_test_sections
WHEN (SELECT status FROM mock_tests WHERE id=OLD.mock_test_id)='completed'
BEGIN SELECT RAISE(ABORT,'reopen mock before editing sections'); END;

CREATE TRIGGER trg_revision_plan_item_scope_guard
BEFORE UPDATE OF concept_id, focus_exam_id ON revision_plan_items
WHEN (OLD.concept_id <> NEW.concept_id OR OLD.focus_exam_id IS NOT NEW.focus_exam_id)
  AND (
      OLD.active_daily_plan_id IS NOT NULL
      OR EXISTS (SELECT 1 FROM revision_daily_plan_items WHERE plan_item_id=OLD.id)
  )
BEGIN
    SELECT RAISE(ABORT,'concept and focus exam cannot change while plan membership exists');
END;

CREATE TRIGGER trg_revision_daily_plan_exam_immutable
BEFORE UPDATE OF exam_id ON revision_daily_plans
WHEN OLD.exam_id IS NOT NEW.exam_id
  AND EXISTS (SELECT 1 FROM revision_daily_plan_items WHERE daily_plan_id=OLD.id)
BEGIN
    SELECT RAISE(ABORT,'daily plan exam cannot change while memberships exist');
END;
```

<!-- SCHEMA-TRIGGERS:END -->

### 7.6 Seed records

<!-- SEED-SQL:BEGIN -->

```sql
INSERT INTO schema_migrations(version,name,checksum,applied_at)
VALUES (1,'v2_initial_schema','0000000000000000000000000000000000000000000000000000000000000000','2026-01-01T00:00:00Z');

INSERT INTO app_metadata(id,schema_version,database_id,app_version,exam_config_version,created_at,updated_at)
VALUES (1,1,'00000000000000000000000000000000','2.0.0',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO settings(id,preparation_start_date,daily_study_target_minutes,created_at,updated_at)
VALUES (1,NULL,120,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO timer_settings(id,focus_minutes,short_break_minutes,long_break_minutes,notifications_enabled,created_at,updated_at)
VALUES (1,25,5,15,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO revision_settings(id,created_at,updated_at)
VALUES (1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO backup_settings(id,updated_at)
VALUES (1,'2026-01-01T00:00:00Z');

INSERT INTO exams(id,code,name,short_name,authority,official_url,display_order,created_at,updated_at) VALUES
(1,'csir-net-physics','CSIR-UGC NET Physical Sciences','CSIR-NET','CSIR-HRDG / NTA','https://csirnet.nta.nic.in/',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(2,'gate-physics','GATE Physics (PH)','GATE PH','IIT','https://gate.iitm.ac.in/',2,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(3,'tifr-gs-physics','TIFR Graduate School Physics','TIFR GS','TIFR','https://www.tifr.res.in/academics/',3,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(4,'jest-physics','JEST Physics','JEST','JEST Consortium','https://www.jest.org.in/',4,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO exam_profiles(exam_id,is_active,exam_date,created_at,updated_at) VALUES
(1,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(2,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(3,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
(4,1,NULL,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');

INSERT INTO mastery_algorithm_versions(id,version,parameters_json,description,is_active,created_at)
VALUES (1,'1.0','{"studyCap":10,"revisionCap":10,"pyqCap":10,"mockCap":8,"totalPositiveCap":20,"totalNegativeCap":15,"studyPerMinuteDivisor":15,"studyMaxBase":8,"revisionDeltas":{"again":-6,"difficult":2,"good":5,"easy":8},"pyqCorrect":4,"pyqIncorrect":-3,"mockCorrect":3,"mockIncorrect":-2,"difficultyMultipliers":{"1":0.75,"2":0.75,"3":1.0,"4":1.25,"5":1.25}}','initial mastery algorithm',1,'2026-01-01T00:00:00Z');

INSERT INTO readiness_formula_versions(id,version,parameters_json,description,is_active,created_at)
VALUES (1,'1.0','{"weights":{"mastery":0.35,"coverage":0.25,"pyq":0.20,"mock":0.15,"revision":0.05},"minSamples":{"pyq":10,"mock":1,"revision":5},"recencyDays":{"pyq":365,"mock":365,"revision":30}}','initial readiness formula',1,'2026-01-01T00:00:00Z');
```

<!-- SEED-SQL:END -->

**Implementation note:** the 64-zero checksum in the seed is a placeholder. Initialization replaces it with the SHA-256 of the schema payload, with CRLF/CR normalized to LF, before commit.

---

## 8. Mastery calculation and replay transaction

### 8.1 Complete base delta

All values come from `raw_value_json` and the evidence row’s algorithm version.

```text
STUDY:
  require duration_seconds integer 1..86400
  base = min(studyMaxBase, max(1, floor(duration_seconds / 900)))
  category = study

REVISION:
  require outcome in {again,difficult,good,easy}
  base = revisionDeltas[outcome]
  category = revision

PYQ:
  require status in {correct,incorrect,skipped}
  base = status==correct ? pyqCorrect : status==incorrect ? pyqIncorrect : 0
  multiplier = difficultyMultipliers[difficulty 1..5]
  base = base * multiplier
  if question has multiple concept mappings:
     factor = mapping.weight / sum(weights for that question)
     base = base * factor
  category = pyq

MOCK CONCEPT RESULT:
  require correct >=0, incorrect >=0
  base = correct*mockCorrect + incorrect*mockIncorrect
  category = mock

MANUAL:
  require target_score number 0..100
  set score to target_score; bypass caps
  category = manual
```

Validation:

- `raw_value_json` must match the source kind schema.
- Invalid evidence is rejected before insert; replay never silently skips invalid rows.
- If corrupt invalid evidence is found, enter recovery mode; do not compute.
- Zero-delta events consume no cap.
- Manual override sets exact score and does not consume caps; later evidence applies from the new score.

### 8.2 Caps

Per concept/local date/category:

| Category | Positive cap | Negative cap |
| -------- | -----------: | -----------: |
| Study    |           10 |            0 |
| Revision |           10 |           10 |
| PYQ      |           10 |           10 |
| Mock     |            8 |            8 |
| Total    |           20 |           15 |

Positive and negative caps are tracked independently. Daily reset is by stored `local_date`. Events crossing midnight use each event’s stored local date. Timezone changes do not rewrite stored dates.

Partial cap:

```text
positive applied = min(base, category_positive_remaining, total_positive_remaining)
negative applied = max(base, -category_negative_remaining, -total_negative_remaining)
```

If base exceeds remaining, only remaining is applied. Zero delta consumes no cap.

### 8.3 Formula

```text
positive: score = score + applied * (1 - score/100)
negative: score = score + applied * (score/100)
manual:   score = target
clamp score to 0..100
```

### 8.4 Confidence

- 1: 1 active evidence event
- 2: 2–3 events
- 3: 4–6 events
- 4: 7–12 events
- 5: >12 events and at least one event within 90 days

No evidence: score 0, confidence 1.

### 8.5 Replay transaction

Run inserted before results:

```text
BEGIN IMMEDIATE
  active_evidence =
    SELECT e.* FROM concept_evidence e
    WHERE e.concept_id = :concept_id
      AND NOT EXISTS (SELECT 1 FROM concept_evidence_voids v WHERE v.evidence_id=e.id)
    ORDER BY e.occurred_at ASC, e.id ASC
  fingerprint = sha256(concat ordered active evidence fields)
  compute score, confidence, and ordered results in memory
  INSERT INTO mastery_recalc_runs(..., evidence_count, evidence_fingerprint,
      resulting_score, resulting_confidence, started_at, finished_at)
  run_id = last_insert_rowid()
  INSERT INTO mastery_replay_results(recalc_run_id, evidence_id, concept_id,
      algorithm_version_id, applied_delta, score_before, score_after, category,
      cap_consumed, created_at) for each result
  INSERT INTO concept_mastery(concept_id, mastery_score, confidence, evidence_count,
      last_algorithm_version_id, last_recalc_run_id, last_recalculated_at, updated_at)
    VALUES (...) ON CONFLICT(concept_id) DO UPDATE SET ...
COMMIT
```

Empty evidence: run with `evidence_count=0`, fingerprint of empty string, score 0, confidence 1, no replay results; `concept_mastery` points to that run. Failure rolls back run, results, and mastery update.

### 8.6 Source edits/deletions

- Edit: void old evidence, insert new evidence with `source_version+1`, replay.
- Delete: void evidence, replay.
- Manual: insert manual evidence, replay.
- Voids are append-only; raw evidence is never updated/deleted.
- Replay is deterministic from active evidence + per-row algorithm version.

### 8.7 Worked examples

**Study cap:** Start 0. Study 60 min base 4 → 4.00. Study 120 min base 8, study remaining 6 → `4+6×0.96=9.76`. Study 60 min, study cap 0 → 9.76.

**Positive/negative:** Start 50. Revision good +5 → 52.50. PYQ incorrect −3 → 50.925. PYQ correct +4 → 52.888.

**Total cap:** Study 120 base 8 applied 8. PYQ correct +10 applied 10 (total 18). Revision good +5: total positive remaining 2, applied 2.

**Manual:** Start 20, manual target 70 → 70. Study 30 base 2 → 70.60.

**Replay:** The same ordered evidence and algorithm versions produce 9.76, 52.888, etc. within 1e-9.

---

## 9. Revision-plan integrity

Enforcement types: **C** = SQLite constraint, **T** = SQLite trigger, **R** = Rust transaction validation.

| Rule                                                 | Enforcement                            |
| ---------------------------------------------------- | -------------------------------------- |
| One active plan per date/scope                       | Partial unique indexes (C)             |
| Membership base order/estimated/lane/score immutable | Trigger (T)                            |
| One active plan per plan item                        | `active_daily_plan_id` + trigger (C/T) |
| Exam plan item focus exam = plan exam                | Trigger + Rust (T/R)                   |
| Concept mapped to exam                               | Trigger + Rust (T/R)                   |
| Combined item mapped to active exam                  | Trigger + Rust (T/R)                   |
| Completed item immutable                             | Trigger (T)                            |
| Completed membership immutable                       | Trigger (T)                            |
| No deletion of membership history                    | Trigger (T)                            |
| Deferral only for non-completed item                 | Trigger + Rust (T/R)                   |
| Defer/override date direction                        | CHECK (C)                              |
| Active plan close clears active memberships          | Trigger + Rust (T/R)                   |
| Plan deletion with memberships blocked               | Trigger (T)                            |
| Active exam write blocked                            | Rust inactive guard (R)                |
| Session link unique                                  | Unique (C)                             |
| Session link after completion blocked                | Rust (R)                               |
| Exam deactivation closes plans                       | Rust (R)                               |
| Archived concept/node excluded from new plans        | Rust (R)                               |

Lifecycle:

- Insert plan: status active, unique scope/date.
- Insert membership: parent active, item free, exam ownership/mapping valid.
- Update membership: only active plan, not completed; status transitions planned/in_progress/deferred/removed/cancelled/completed.
- Defer: deferral row, item deferred, membership deferred, clear active plan, schedule date updated.
- Override: deferral row, item backlog, membership removed, clear active plan.
- Cancel: item cancelled, membership cancelled, clear active plan, `cancelled_at`.
- Complete: history row, item completed, membership completed, clear active plan.
- Reassign: only before completion; new exam mapping valid; active plan cleared.
- Session link: insert only for non-completed item; delete allowed only by session cascade or backend unlink before completion.
- Deactivation: close active exam plans; clear active memberships; history remains.
- Archival: no new plans; existing items can complete/remove.

---

## 10. Mock score history

`marking_context_json`:

```json
{
  "schemaVersion": 1,
  "examCode": "gate-physics",
  "examContentVersionId": 1,
  "examContentVersionLabel": "2026",
  "sections": [
    {
      "sectionId": 10,
      "sectionCode": "GA",
      "questionTypeId": 3,
      "questionTypeCode": "MCQ",
      "correctMarks": 1.0,
      "incorrectMarks": -0.33,
      "unansweredMarks": 0.0,
      "partialMarksAllowed": false,
      "partialMarksRule": "",
      "maxMarks": 10.0,
      "questionCount": 10,
      "correct": 8,
      "incorrect": 1,
      "unanswered": 1
    }
  ]
}
```

Rules:

- Initial completion: version 1, snapshot inserted in the same transaction.
- Direct insert as completed: forbidden by trigger; insert planned/in_progress then complete.
- Reopen: completed → in_progress; no history row yet.
- Edit sections: only when not completed.
- Recompletion: compute new snapshot; if identical to latest history, do not insert duplicate; else insert next version.
- Failed completion: no history; rollback.
- Marking changes: history context remains; current defaults may change; recompletion uses current context.
- Completed mocks archive, never hard-delete; drafts may delete.
- History rows immutable.

---

## 11. Backup/restore state machine

Files:

- live: `physics-tracker.sqlite3`, `-wal`, `-shm`
- sidecar: `restore-operation.json`, temp `restore-operation.json.tmp`
- candidate: `restore-candidate.sqlite3`
- previous live: `physics-tracker.pre-restore-<opid>.sqlite3`
- safety: `backups/safety/pre-restore-<opid>.ptbackup`

Sidecar fields: `opId`, `stage`, `createdAt`, `updatedAt`, `candidatePath`, `candidateSha`, `safetyPath`, `safetySha`, `previousLivePath`, `livePath`, `lastError`.

Stages:

| Stage                 | Files / DB state                                                | Authoritative DB       | Recovery                                  |
| --------------------- | --------------------------------------------------------------- | ---------------------- | ----------------------------------------- |
| `validated`           | candidate verified, safety verified, live untouched             | live                   | delete candidate/sidecar; normal          |
| `safety_created`      | safety durable, live untouched                                  | live                   | keep safety; normal                       |
| `db_closed`           | connections closed, WAL checkpointed                            | live                   | reopen live; verify                       |
| `live_renamed`        | live moved to previous path; WAL/SHM removed; candidate present | none                   | if candidate absent, rename previous back |
| `candidate_installed` | candidate at live path; previous exists                         | candidate (unverified) | verify; if fail restore previous/safety   |
| `verified`            | DB reopened and checks pass                                     | candidate              | clear sidecar; normal                     |
| `cleanup_complete`    | sidecar removed; previous retained                              | live                   | normal                                    |

Interruptions:

- Before sidecar: live untouched; temp files removed.
- During sidecar write: use temp + flush + atomic replace; old sidecar remains.
- After candidate validation: live untouched; delete candidate/sidecar if process stops.
- During safety backup: live untouched; incomplete safety file removed.
- After safety: live untouched; safety authoritative fallback.
- During DB close/WAL checkpoint: reopen live; verify.
- After live rename: if candidate missing, restore previous.
- During candidate install: if partial, restore previous/safety.
- After install before verification: verify; rollback if invalid.
- After verification before sidecar removal: clear sidecar; live valid.
- During sidecar removal: live valid; stale sidecar can be ignored after verification.

Failure rules:

- Disk full: abort before live rename; remove temp; live untouched.
- Sharing violation/open handle: abort before live rename; close/reopen live.
- Candidate invalid: never install; live untouched.
- Safety invalid: abort before live rename.
- Damaged sidecar: inspect files; choose valid live, else candidate, else previous; never delete only good copy.
- Missing live/candidate: use previous or safety; enter recovery mode if none valid.
- Success only after `integrity_check`, `foreign_key_check`, schema version, four exams/profiles, metadata.

---

## 12. Readiness formulas

Weights: 0.35 mastery, 0.25 coverage, 0.20 PYQ, 0.15 mock, 0.05 revision. All components 0–100; missing = null; overall null unless all five non-null. No weight redistribution.

- **Mastery:** for each concept mapped to exam, `weight = max(exam_node_concepts.weight)` for that exam. Score = weighted average of `concept_mastery.mastery_score`; unseen concepts count 0. Count = active evidence for mapped concepts. Requires ≥1 mapped concept and ≥1 evidence; else null. Archived concepts/nodes excluded.
- **Coverage:** leaf topic nodes (`node_kind='topic'`, not archived). Status values: not_started 0, learning 0.5, revised 0.8, confident 1.0. Weighted by `readiness_weight`. Requires ≥1 leaf; else null.
- **PYQ:** non-skipped attempts in last 365 days. Recency weight `0.5^(days/180)`. Accuracy = Σ(weight × correct_indicator) / Σ(weight). Requires ≥10 attempts; else null.
- **Mock:** completed mocks in last 365 days, last 3. Percentage = marks_obtained / max_marks. Recency weight same. Requires ≥1; else null.
- **Revision:** revision history rows in last 30 days. On-time = revision_date <= scheduled_for_date; late = otherwise. Adherence = on-time / total rows. Requires ≥5; else null.
- Archived topics/concepts excluded. Inactive exams excluded from active readiness; snapshots remain historical. Edited/deleted data changes current calculations; snapshots immutable.
- Overall = sum(weight × component) when all present. Confidence:
  - insufficient: any component null.
  - low: all present but any at minimum or last evidence >180 days.
  - medium: all present, ≥3 components above minimum, last evidence ≤90 days.
  - high: all present, all components ≥2× minimum, last evidence ≤30 days.
- Snapshots: one row per computation with exam, date, formula version, component scores/counts, confidence, nullable overall, `components_json`, `computed_at`. Multiple snapshots allowed; latest displayed. Old snapshots immutable.

---

## 13. Startup integrity

- Every startup: schema version, exactly four exams/profiles, `quick_check(1)`, `foreign_key_check`.
- Full `integrity_check` + `foreign_key_check`: weekly, before backup, before/after restore, after recovery.
- Missing exam/profile: recover in transaction.
- Extra/unsupported exam: fail closed; preserve DB.
- `foreign_key_check` rows: recovery mode, no writes; log offending tables; require restore/repair.
- `integrity_check` failure: recovery mode.
- Content missing: exam “Official content not configured”; no fake data.
- Full-check timestamp stored in `app_metadata`.

---

## 14. Frontend, Rust modules, and API boundaries

### 14.1 Screens

| Screen     | Purpose                                      |
| ---------- | -------------------------------------------- |
| Overview   | Combined/exam readiness and activity summary |
| Exams      | Four profiles, manual dates, activation      |
| Syllabus   | Exam-specific tree and progress              |
| Concepts   | Shared library, mastery, evidence            |
| PYQs       | Exam-specific question bank and attempts     |
| Mock Tests | Exam-specific mocks and section results      |
| Timer      | Shared timer with exam/concept context       |
| Sessions   | Shared session history and allocations       |
| Checklist  | Shared/exam tasks                            |
| Calendar   | Tasks and planned blocks                     |
| Revision   | Combined/exam daily plans and history        |
| Analytics  | Combined/exam real-data metrics              |
| Resources  | Shared library and exam links                |
| Backup     | V2 backup/restore/recovery                   |

Exam switcher: Combined + four exams. Shared data remains shared; combined totals count sessions once.

### 14.2 Frontend state

- `ExamContext`: `Combined | Exam(id)`, persisted as a UI preference.
- `useApiQuery` with loading/success/error/cancel/refresh.
- Local form state; Rust is authoritative.
- Mutations refetch affected data.
- No new state/cache library.
- No placeholder controls, fake charts, or silent failures.

### 14.3 Rust modules

```text
src-tauri/src/
  main.rs
  lib.rs                 builder/state/plugin/command registration only
  db/                    connection, PRAGMAs, migrations, seed, transactions
  exams/                 profiles, dates, activation, sections
  concepts/              library, mastery replay, evidence
  syllabus/              nodes, mappings, progress, content migrations
  study/                 timer, sessions, allocations, planner, calendar
  pyq/                   questions, attempts, analytics
  mocks/                 tests, section scoring, score history
  revision/              schedules, plans, deferrals, links
  analytics/             combined/exam metrics, readiness
  resources/             library, links, files, history
  backup/                format, restore state machine, retention
  error.rs
  date_only.rs
  performance_audit.rs   test-only
```

### 14.4 Tauri command groups

| Domain    | Representative commands                                                                                                                                                                                                                        |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Exams     | `list_exams`, `get_exam_profile`, `preview_exam_date_change`, `apply_exam_date_change`, `set_exam_active`                                                                                                                                      |
| Syllabus  | `get_exam_syllabus`, `update_syllabus_node_progress`, `get_syllabus_summary`                                                                                                                                                                   |
| Concepts  | `search_concepts`, `get_concept_detail`, `update_concept_mastery`                                                                                                                                                                              |
| Timer     | `get_timer`, `start_timer`, `pause_timer`, `resume_timer`, `finish_timer`, `discard_timer`                                                                                                                                                     |
| Sessions  | `list_sessions`, `create_manual_session`, `update_study_session`, `delete_study_session`, `set_session_allocations`, `get_study_totals`                                                                                                        |
| Planner   | `list_tasks`, `save_task`, `set_task_status`, `reschedule_task`, `delete_task`, `get_calendar`, `get_agenda`, `save_block`, `set_block_status`, `link_block_session`, `delete_block`                                                           |
| PYQ       | `get_pyq_options`, `list_pyq_questions`, `save_pyq_question`, `list_pyq_attempts`, `save_pyq_attempt`, `delete_pyq_attempt`, `get_pyq_analytics`                                                                                               |
| Mocks     | `list_mock_tests`, `save_mock_test`, `delete_mock_test`, `get_mock_analytics`                                                                                                                                                                  |
| Revision  | `get_revision_dashboard`, `update_revision_settings`, `complete_revision`, `defer_revision`, `get_revision_daily_plan`, `generate_revision_daily_plan`, `rebalance_revision_daily_plan`, `move_revision_plan_item`, `link_revision_plan_block` |
| Analytics | `get_combined_dashboard`, `get_exam_dashboard`, `get_combined_analytics`, `get_exam_analytics`                                                                                                                                                 |
| Resources | `list_resources`, `save_resource`, `archive_resource`, `link_resource_exam`, `update_resource_progress`, `import_resource_pdf`, `remove_resource_pdf`, `relink_resource_pdf`, `replace_resource_pdf`                                           |
| Backup    | `get_backup_overview`, `update_backup_settings`, `create_manual_backup`, `run_automatic_backup_if_due`, `preview_restore_backup`, `apply_restore_backup`, `get_restore_recovery`, `keep_restore_recovery`, `restore_recovery_safety`           |

All mutating exam commands take explicit `exam_id`; inactive guard returns `forbidden`. Session totals: combined uses duration once; exam uses allocations. Mock/revision/backup use the transaction/state rules above. Rust owns validation; frontend DTO fields must match returned backend fields.

---

## 15. Implementation roadmap and acceptance gates

| Milestone              | Deliverables                                                       | Exit gate                                                  |
| ---------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------- |
| M0 Foundation          | Schema, seed, four exams/profiles, app identity, error model       | Fresh DB, four exams, null dates, no V1 access             |
| M1 Integrity/backup    | Migrations, triggers, backup/restore state machine, startup checks | Schema/trigger/FK tests, backup round-trip, recovery tests |
| M2 Workspaces          | Exam switcher, dates, syllabus, sections, marking, progress        | Four independent workspaces with reviewed content          |
| M3 Concepts            | Concept library, mappings, mastery replay/evidence                 | Deterministic replay, mastery tests                        |
| M4 Study               | Timer, sessions, allocations, planner, calendar                    | Timer recovery, no double counting, allocation limits      |
| M5 Practice            | PYQs, attempts, mocks, section scoring, history                    | Four-exam PYQ/mock/marking tests                           |
| M6 Revision/readiness  | Plan lifecycle, deferrals, readiness formulas/snapshots            | Plan integrity, readiness nullability, snapshot tests      |
| M7 Analytics/resources | Combined/exam analytics, resources, file history                   | Real-data metrics, file-safe operations                    |
| M8 Release             | Integration, accessibility, Windows build, E2E                     | Full acceptance and release checklist                      |

No milestone is complete based only on component tests; its user workflow must work end to end.

---

## 16. Validation plan

The standalone script `Physics_Tracker_V2.0_Schema_Validation.py` uses the actual SQL blocks from this document and a temporary database. It does not touch application data.

### 16.1 Schema tests (SQLite/Python)

- Fresh schema, indexes, triggers.
- Seed exactly four exams/profiles.
- Valid time `09:30`; invalid `9:30`, `24:00`, `09:60`.
- Date constraints.
- FK integrity and cross-exam rejection.
- Trigger behavior for allocation caps, evidence immutability, mock completion, plan scope, etc.
- Unique/partial unique indexes.
- Readiness snapshot constraints.
- `integrity_check` and `foreign_key_check`.

### 16.2 Rust integration tests

- Inactive exam guard across commands.
- Mastery replay transaction with run/results ordering.
- Timer recovery on restart/clock/sleep.
- Mock completion/reopen/recomplete.
- Revision lifecycle transitions.
- Backup/restore.

### 16.3 Windows-specific tests

- Atomic rename, sharing violations, open handles, WAL/SHM, crash recovery at every stage.

### 16.4 Executed validation record

`Physics_Tracker_V2.0_Schema_Validation.py` was executed against a temporary SQLite database using the actual SQL blocks in this document.

- SQLite version/JSON1/foreign-keys checks: passed.
- Fresh schema, all indexes, all triggers, exact four-exam seed: passed.
- 86 schema/constraint tests: **86 passed, 0 failed**.
- `PRAGMA integrity_check`: `ok`.
- `PRAGMA foreign_key_check`: empty.
- The script did not touch any application database.

Rust integration and Windows-specific recovery tests remain to be implemented and executed during the relevant milestones.

---

## 17. Remaining blockers

| ID   | Blocker                                                | Default                                   | Blocking |
| ---- | ------------------------------------------------------ | ----------------------------------------- | -------- |
| B-01 | Official content/section/marking sources and reviewers | Official review; no invented content      | Yes      |
| B-02 | Readiness weights/min samples approval                 | 35/25/20/15/5; all components for overall | Yes      |
| B-03 | Inactive workspace read-only vs editable               | Read-only; reactivate to edit             | Yes      |
| B-04 | One current date vs historical cycles                  | One nullable current + audit              | Yes      |

Safe defaults: new app identity, no V1 import, no scraper, no CSV release scope, no encryption, curated concepts, metadata-only PDF backup.

---

## 18. Release checklist

- [ ] Four exams; JEST included; no extra exam.
- [ ] Fresh schema/seed/triggers validate.
- [ ] Mastery replay deterministic; run inserted before results.
- [ ] Timer recovery never counts unverified time.
- [ ] Inactive exam writes rejected in Rust.
- [ ] Revision-plan lifecycle and exam scope enforced.
- [ ] Mock completion/history reproducible.
- [ ] Backup/restore state machine tested.
- [ ] Readiness formulas and confidence tested.
- [ ] Every trigger/FK/important constraint has a mechanical test.
- [ ] V1 data untouched.
- [ ] No placeholders/fake data/dead controls.
- [ ] Official content verified for all four exams.
