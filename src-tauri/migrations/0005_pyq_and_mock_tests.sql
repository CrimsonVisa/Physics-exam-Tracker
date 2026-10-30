-- Phase 6: previous-year question attempts, mock tests, and duplicate-safe CSV
-- imports. PYQ records and mock tests are separate domains: a mock test never
-- creates individual PYQ attempts, and neither table contributes to study-time
-- totals.
--
-- import_key is nullable for manually entered rows and unique for imported
-- rows. It is a stable fingerprint of the source row so repeated imports can be
-- skipped without silently discarding legitimate manual records.

CREATE TABLE pyq_attempts (
    id INTEGER PRIMARY KEY,
    question_year INTEGER NOT NULL
        CHECK (question_year BETWEEN 1900 AND 2100),
    exam_session TEXT NOT NULL DEFAULT ''
        CHECK (length(exam_session) <= 100),
    part TEXT NOT NULL
        CHECK (part IN ('A', 'B', 'C')),
    topic_id INTEGER NOT NULL
        REFERENCES syllabus_topics(id) ON DELETE RESTRICT,
    question_reference TEXT NOT NULL
        CHECK (length(trim(question_reference)) BETWEEN 1 AND 200),
    attempt_date TEXT NOT NULL
        CHECK (
            length(attempt_date) = 10
            AND attempt_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(attempt_date) IS attempt_date
        ),
    status TEXT NOT NULL
        CHECK (status IN ('attempted', 'correct', 'incorrect', 'skipped')),
    marks_obtained REAL
        CHECK (
            marks_obtained IS NULL
            OR (marks_obtained BETWEEN -100000 AND 100000)
        ),
    max_marks REAL
        CHECK (
            max_marks IS NULL
            OR (max_marks > 0 AND max_marks <= 100000)
        ),
    time_spent_seconds INTEGER NOT NULL DEFAULT 0
        CHECK (time_spent_seconds BETWEEN 0 AND 86400),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    difficulty INTEGER
        CHECK (difficulty IS NULL OR difficulty BETWEEN 1 AND 5),
    import_key TEXT UNIQUE
        CHECK (import_key IS NULL OR length(import_key) <= 200),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        marks_obtained IS NULL
        OR max_marks IS NULL
        OR marks_obtained <= max_marks
    )
) STRICT;

CREATE INDEX idx_pyq_attempts_year
    ON pyq_attempts(question_year, attempt_date, id);

CREATE INDEX idx_pyq_attempts_part
    ON pyq_attempts(part, attempt_date, id);

CREATE INDEX idx_pyq_attempts_topic
    ON pyq_attempts(topic_id, attempt_date, id);

CREATE INDEX idx_pyq_attempts_date
    ON pyq_attempts(attempt_date, id);

CREATE INDEX idx_pyq_attempts_status
    ON pyq_attempts(status, attempt_date, id);

CREATE TABLE mock_tests (
    id INTEGER PRIMARY KEY,
    test_name TEXT NOT NULL
        CHECK (length(trim(test_name)) BETWEEN 1 AND 200),
    test_date TEXT NOT NULL
        CHECK (
            length(test_date) = 10
            AND test_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
            AND date(test_date) IS test_date
        ),
    duration_minutes INTEGER NOT NULL
        CHECK (duration_minutes BETWEEN 1 AND 1440),
    max_marks REAL NOT NULL
        CHECK (max_marks > 0 AND max_marks <= 100000),
    marks_obtained REAL
        CHECK (
            marks_obtained IS NULL
            OR (marks_obtained BETWEEN -100000 AND 100000)
        ),
    part_a_questions INTEGER NOT NULL DEFAULT 0
        CHECK (part_a_questions BETWEEN 0 AND 10000),
    part_a_correct INTEGER NOT NULL DEFAULT 0
        CHECK (part_a_correct BETWEEN 0 AND 10000),
    part_a_incorrect INTEGER NOT NULL DEFAULT 0
        CHECK (part_a_incorrect BETWEEN 0 AND 10000),
    part_a_unanswered INTEGER NOT NULL DEFAULT 0
        CHECK (part_a_unanswered BETWEEN 0 AND 10000),
    part_a_correct_marks REAL NOT NULL DEFAULT 0
        CHECK (part_a_correct_marks BETWEEN 0 AND 1000),
    part_a_incorrect_marks REAL NOT NULL DEFAULT 0
        CHECK (part_a_incorrect_marks BETWEEN -1000 AND 1000),
    part_a_unanswered_marks REAL NOT NULL DEFAULT 0
        CHECK (part_a_unanswered_marks BETWEEN -1000 AND 1000),
    part_b_questions INTEGER NOT NULL DEFAULT 0
        CHECK (part_b_questions BETWEEN 0 AND 10000),
    part_b_correct INTEGER NOT NULL DEFAULT 0
        CHECK (part_b_correct BETWEEN 0 AND 10000),
    part_b_incorrect INTEGER NOT NULL DEFAULT 0
        CHECK (part_b_incorrect BETWEEN 0 AND 10000),
    part_b_unanswered INTEGER NOT NULL DEFAULT 0
        CHECK (part_b_unanswered BETWEEN 0 AND 10000),
    part_b_correct_marks REAL NOT NULL DEFAULT 0
        CHECK (part_b_correct_marks BETWEEN 0 AND 1000),
    part_b_incorrect_marks REAL NOT NULL DEFAULT 0
        CHECK (part_b_incorrect_marks BETWEEN -1000 AND 1000),
    part_b_unanswered_marks REAL NOT NULL DEFAULT 0
        CHECK (part_b_unanswered_marks BETWEEN -1000 AND 1000),
    part_c_questions INTEGER NOT NULL DEFAULT 0
        CHECK (part_c_questions BETWEEN 0 AND 10000),
    part_c_correct INTEGER NOT NULL DEFAULT 0
        CHECK (part_c_correct BETWEEN 0 AND 10000),
    part_c_incorrect INTEGER NOT NULL DEFAULT 0
        CHECK (part_c_incorrect BETWEEN 0 AND 10000),
    part_c_unanswered INTEGER NOT NULL DEFAULT 0
        CHECK (part_c_unanswered BETWEEN 0 AND 10000),
    part_c_correct_marks REAL NOT NULL DEFAULT 0
        CHECK (part_c_correct_marks BETWEEN 0 AND 1000),
    part_c_incorrect_marks REAL NOT NULL DEFAULT 0
        CHECK (part_c_incorrect_marks BETWEEN -1000 AND 1000),
    part_c_unanswered_marks REAL NOT NULL DEFAULT 0
        CHECK (part_c_unanswered_marks BETWEEN -1000 AND 1000),
    notes TEXT NOT NULL DEFAULT ''
        CHECK (length(notes) <= 2000),
    review_observations TEXT NOT NULL DEFAULT ''
        CHECK (length(review_observations) <= 2000),
    import_key TEXT UNIQUE
        CHECK (import_key IS NULL OR length(import_key) <= 200),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (part_a_correct + part_a_incorrect + part_a_unanswered = part_a_questions),
    CHECK (part_b_correct + part_b_incorrect + part_b_unanswered = part_b_questions),
    CHECK (part_c_correct + part_c_incorrect + part_c_unanswered = part_c_questions),
    CHECK (part_a_questions + part_b_questions + part_c_questions > 0)
) STRICT;

CREATE INDEX idx_mock_tests_date
    ON mock_tests(test_date DESC, id DESC);

CREATE INDEX idx_mock_tests_name
    ON mock_tests(test_name, test_date DESC);
