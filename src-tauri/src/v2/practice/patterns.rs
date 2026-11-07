use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppError;

pub const PATTERN_STATUS_VERIFIED: &str = "verified";
pub const PATTERN_STATUS_PENDING: &str = "pending";

const TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS exam_patterns (
    id INTEGER PRIMARY KEY,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 80),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    variant TEXT NOT NULL DEFAULT '' CHECK (length(variant) <= 120),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (exam_id, code)
) STRICT;

CREATE TABLE IF NOT EXISTS exam_pattern_versions (
    id INTEGER PRIMARY KEY,
    pattern_id INTEGER NOT NULL REFERENCES exam_patterns(id) ON DELETE RESTRICT,
    exam_id INTEGER NOT NULL REFERENCES exams(id) ON DELETE RESTRICT,
    version_label TEXT NOT NULL CHECK (length(trim(version_label)) BETWEEN 1 AND 120),
    effective_from TEXT CHECK (
        effective_from IS NULL OR
        (length(effective_from)=10 AND date(effective_from)=effective_from)
    ),
    source_title TEXT NOT NULL DEFAULT '' CHECK (length(source_title) <= 400),
    source_url TEXT CHECK (source_url IS NULL OR source_url LIKE 'https://%'),
    source_reference TEXT NOT NULL DEFAULT '' CHECK (length(source_reference) <= 2000),
    verified_on TEXT CHECK (
        verified_on IS NULL OR
        (length(verified_on)=10 AND date(verified_on)=verified_on)
    ),
    verification_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (verification_status IN ('pending','verified','rejected')),
    duration_minutes INTEGER CHECK (
        duration_minutes IS NULL OR duration_minutes BETWEEN 1 AND 1440
    ),
    total_marks REAL NOT NULL DEFAULT 0 CHECK (total_marks >= 0),
    total_questions INTEGER NOT NULL DEFAULT 0 CHECK (total_questions >= 0),
    attempt_limit INTEGER CHECK (attempt_limit IS NULL OR attempt_limit > 0),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    provenance TEXT NOT NULL DEFAULT '' CHECK (length(provenance) <= 4000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (pattern_id, version_label),
    UNIQUE (exam_id, id)
) STRICT;

CREATE TABLE IF NOT EXISTS exam_pattern_sections (
    id INTEGER PRIMARY KEY,
    pattern_version_id INTEGER NOT NULL
        REFERENCES exam_pattern_versions(id) ON DELETE CASCADE,
    exam_id INTEGER NOT NULL,
    section_id INTEGER NOT NULL,
    code TEXT NOT NULL CHECK (length(trim(code)) BETWEEN 1 AND 40),
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 200),
    display_order INTEGER NOT NULL CHECK (display_order >= 0),
    question_count INTEGER NOT NULL CHECK (question_count >= 0),
    max_attempts INTEGER CHECK (max_attempts IS NULL OR max_attempts > 0),
    max_marks REAL NOT NULL CHECK (max_marks >= 0),
    correct_marks REAL NOT NULL DEFAULT 1 CHECK (correct_marks > 0),
    incorrect_marks REAL NOT NULL DEFAULT 0 CHECK (incorrect_marks <= 0),
    unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks <= 0),
    question_types_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(question_types_json)),
    marking_variants_json TEXT NOT NULL DEFAULT '[]'
        CHECK (json_valid(marking_variants_json)),
    numeric_tolerance REAL NOT NULL DEFAULT 0 CHECK (numeric_tolerance >= 0),
    partial_rule TEXT NOT NULL DEFAULT '' CHECK (length(partial_rule) <= 1000),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 4000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (pattern_version_id, code),
    UNIQUE (exam_id, id),
    FOREIGN KEY (exam_id, section_id)
        REFERENCES exam_sections(exam_id, id) ON DELETE RESTRICT
) STRICT;

CREATE TABLE IF NOT EXISTS pyq_question_topics (
    exam_id INTEGER NOT NULL,
    question_id INTEGER NOT NULL,
    node_id INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0,1)),
    created_at TEXT NOT NULL,
    PRIMARY KEY (question_id, node_id),
    UNIQUE (exam_id, question_id, node_id),
    FOREIGN KEY (exam_id, question_id)
        REFERENCES pyq_questions(exam_id, id) ON DELETE CASCADE,
    FOREIGN KEY (exam_id, node_id)
        REFERENCES exam_syllabus_nodes(exam_id, id) ON DELETE RESTRICT
) STRICT;

"#;

const INDEXES: &str = r#"
CREATE INDEX IF NOT EXISTS idx_pyq_questions_exam_status
    ON pyq_questions(exam_id, is_archived, verification_status, year);
CREATE INDEX IF NOT EXISTS idx_pyq_question_topics_node
    ON pyq_question_topics(exam_id, node_id, question_id);
CREATE INDEX IF NOT EXISTS idx_pyq_attempts_exam_date
    ON pyq_attempts(exam_id, attempt_date DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_mock_tests_pattern
    ON mock_tests(exam_id, pattern_version_id, is_archived);
"#;

pub fn prepare(connection: &mut Connection) -> Result<(), AppError> {
    connection
        .execute_batch(TABLES)
        .map_err(|error| AppError::database("Practice tables could not be prepared", error))?;

    let columns: &[(&str, &str, &str)] = &[
        (
            "pyq_questions",
            "pattern_version_id",
            "ALTER TABLE pyq_questions ADD COLUMN pattern_version_id INTEGER REFERENCES exam_pattern_versions(id) ON DELETE RESTRICT",
        ),
        (
            "pyq_questions",
            "syllabus_node_id",
            "ALTER TABLE pyq_questions ADD COLUMN syllabus_node_id INTEGER",
        ),
        (
            "pyq_questions",
            "source_title",
            "ALTER TABLE pyq_questions ADD COLUMN source_title TEXT NOT NULL DEFAULT '' CHECK (length(source_title) <= 400)",
        ),
        (
            "pyq_questions",
            "source_page",
            "ALTER TABLE pyq_questions ADD COLUMN source_page TEXT NOT NULL DEFAULT '' CHECK (length(source_page) <= 80)",
        ),
        (
            "pyq_questions",
            "answer_key_source",
            "ALTER TABLE pyq_questions ADD COLUMN answer_key_source TEXT NOT NULL DEFAULT '' CHECK (length(answer_key_source) <= 400)",
        ),
        (
            "pyq_questions",
            "answer_key_version",
            "ALTER TABLE pyq_questions ADD COLUMN answer_key_version TEXT NOT NULL DEFAULT '' CHECK (length(answer_key_version) <= 120)",
        ),
        (
            "pyq_questions",
            "answer_key_verified",
            "ALTER TABLE pyq_questions ADD COLUMN answer_key_verified INTEGER NOT NULL DEFAULT 0 CHECK (answer_key_verified IN (0,1))",
        ),
        (
            "pyq_questions",
            "verification_status",
            "ALTER TABLE pyq_questions ADD COLUMN verification_status TEXT NOT NULL DEFAULT 'pending' CHECK (verification_status IN ('draft','pending','verified','rejected'))",
        ),
        (
            "pyq_questions",
            "numeric_tolerance",
            "ALTER TABLE pyq_questions ADD COLUMN numeric_tolerance REAL NOT NULL DEFAULT 0 CHECK (numeric_tolerance >= 0)",
        ),
        (
            "pyq_questions",
            "correct_range",
            "ALTER TABLE pyq_questions ADD COLUMN correct_range TEXT NOT NULL DEFAULT '' CHECK (length(correct_range) <= 200)",
        ),
        (
            "pyq_questions",
            "official_explanation",
            "ALTER TABLE pyq_questions ADD COLUMN official_explanation TEXT NOT NULL DEFAULT '' CHECK (length(official_explanation) <= 20000)",
        ),
        (
            "pyq_questions",
            "third_party_solution",
            "ALTER TABLE pyq_questions ADD COLUMN third_party_solution TEXT NOT NULL DEFAULT '' CHECK (length(third_party_solution) <= 20000)",
        ),
        (
            "pyq_questions",
            "user_note",
            "ALTER TABLE pyq_questions ADD COLUMN user_note TEXT NOT NULL DEFAULT '' CHECK (length(user_note) <= 20000)",
        ),
        (
            "pyq_questions",
            "resource_id",
            "ALTER TABLE pyq_questions ADD COLUMN resource_id INTEGER REFERENCES resources(id) ON DELETE SET NULL",
        ),
        (
            "pyq_questions",
            "options_json",
            "ALTER TABLE pyq_questions ADD COLUMN options_json TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(options_json))",
        ),
        (
            "pyq_attempts",
            "pattern_version_id",
            "ALTER TABLE pyq_attempts ADD COLUMN pattern_version_id INTEGER REFERENCES exam_pattern_versions(id) ON DELETE RESTRICT",
        ),
        (
            "pyq_attempts",
            "answer_key_version",
            "ALTER TABLE pyq_attempts ADD COLUMN answer_key_version TEXT NOT NULL DEFAULT '' CHECK (length(answer_key_version) <= 120)",
        ),
        (
            "pyq_attempts",
            "correct_answer_at_attempt",
            "ALTER TABLE pyq_attempts ADD COLUMN correct_answer_at_attempt TEXT NOT NULL DEFAULT '' CHECK (length(correct_answer_at_attempt) <= 4000)",
        ),
        (
            "pyq_attempts",
            "numeric_tolerance_at_attempt",
            "ALTER TABLE pyq_attempts ADD COLUMN numeric_tolerance_at_attempt REAL NOT NULL DEFAULT 0 CHECK (numeric_tolerance_at_attempt >= 0)",
        ),
        (
            "pyq_attempts",
            "difficulty_at_attempt",
            "ALTER TABLE pyq_attempts ADD COLUMN difficulty_at_attempt INTEGER NOT NULL DEFAULT 3 CHECK (difficulty_at_attempt BETWEEN 1 AND 5)",
        ),
        (
            "pyq_attempts",
            "unanswered_marks_at_attempt",
            "ALTER TABLE pyq_attempts ADD COLUMN unanswered_marks_at_attempt REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks_at_attempt <= 0)",
        ),
        (
            "pyq_attempts",
            "scoring_engine",
            "ALTER TABLE pyq_attempts ADD COLUMN scoring_engine TEXT NOT NULL DEFAULT '' CHECK (length(scoring_engine) <= 80)",
        ),
        (
            "mock_tests",
            "pattern_version_id",
            "ALTER TABLE mock_tests ADD COLUMN pattern_version_id INTEGER REFERENCES exam_pattern_versions(id) ON DELETE RESTRICT",
        ),
        (
            "mock_tests",
            "test_mode",
            "ALTER TABLE mock_tests ADD COLUMN test_mode TEXT NOT NULL DEFAULT 'full_length' CHECK (test_mode IN ('full_length','untimed','section','topic','custom','past_paper'))",
        ),
        (
            "mock_tests",
            "scheme_snapshot_json",
            "ALTER TABLE mock_tests ADD COLUMN scheme_snapshot_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(scheme_snapshot_json))",
        ),
        (
            "mock_tests",
            "available_questions",
            "ALTER TABLE mock_tests ADD COLUMN available_questions INTEGER NOT NULL DEFAULT 0 CHECK (available_questions >= 0)",
        ),
        (
            "mock_tests",
            "attempted_questions",
            "ALTER TABLE mock_tests ADD COLUMN attempted_questions INTEGER NOT NULL DEFAULT 0 CHECK (attempted_questions >= 0)",
        ),
        (
            "mock_tests",
            "correct_questions",
            "ALTER TABLE mock_tests ADD COLUMN correct_questions INTEGER NOT NULL DEFAULT 0 CHECK (correct_questions >= 0)",
        ),
        (
            "mock_tests",
            "incorrect_questions",
            "ALTER TABLE mock_tests ADD COLUMN incorrect_questions INTEGER NOT NULL DEFAULT 0 CHECK (incorrect_questions >= 0)",
        ),
        (
            "mock_tests",
            "unanswered_questions",
            "ALTER TABLE mock_tests ADD COLUMN unanswered_questions INTEGER NOT NULL DEFAULT 0 CHECK (unanswered_questions >= 0)",
        ),
        (
            "mock_tests",
            "percentage",
            "ALTER TABLE mock_tests ADD COLUMN percentage REAL",
        ),
        (
            "mock_tests",
            "focus_node_id",
            "ALTER TABLE mock_tests ADD COLUMN focus_node_id INTEGER REFERENCES exam_syllabus_nodes(id) ON DELETE SET NULL",
        ),
        (
            "mock_tests",
            "started_at",
            "ALTER TABLE mock_tests ADD COLUMN started_at TEXT",
        ),
        (
            "mock_tests",
            "ended_at",
            "ALTER TABLE mock_tests ADD COLUMN ended_at TEXT",
        ),
        (
            "mock_test_sections",
            "pattern_section_id",
            "ALTER TABLE mock_test_sections ADD COLUMN pattern_section_id INTEGER REFERENCES exam_pattern_sections(id) ON DELETE RESTRICT",
        ),
        (
            "mock_test_sections",
            "question_type_id",
            "ALTER TABLE mock_test_sections ADD COLUMN question_type_id INTEGER REFERENCES exam_question_types(id) ON DELETE RESTRICT",
        ),
        (
            "mock_test_sections",
            "correct_marks",
            "ALTER TABLE mock_test_sections ADD COLUMN correct_marks REAL NOT NULL DEFAULT 1 CHECK (correct_marks > 0)",
        ),
        (
            "mock_test_sections",
            "incorrect_marks",
            "ALTER TABLE mock_test_sections ADD COLUMN incorrect_marks REAL NOT NULL DEFAULT 0 CHECK (incorrect_marks <= 0)",
        ),
        (
            "mock_test_sections",
            "unanswered_marks",
            "ALTER TABLE mock_test_sections ADD COLUMN unanswered_marks REAL NOT NULL DEFAULT 0 CHECK (unanswered_marks <= 0)",
        ),
        (
            "mock_test_sections",
            "section_snapshot_json",
            "ALTER TABLE mock_test_sections ADD COLUMN section_snapshot_json TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(section_snapshot_json))",
        ),
    ];

    for (table, column, sql) in columns {
        if !has_column(connection, table, column)? {
            connection.execute_batch(sql).map_err(|error| {
                AppError::database(
                    &format!("The practice column {table}.{column} could not be added"),
                    error,
                )
            })?;
        }
    }
    connection
        .execute_batch(INDEXES)
        .map_err(|error| AppError::database("Practice indexes could not be prepared", error))?;

    seed_patterns(connection)
}

pub(crate) fn has_column(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, AppError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| AppError::database("A practice table could not be inspected", error))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| AppError::database("Practice columns could not be read", error))?;
    for row in rows {
        if row.map_err(|error| AppError::database("A practice column is invalid", error))? == column
        {
            return Ok(true);
        }
    }
    Ok(false)
}

struct PatternSeed {
    exam_code: &'static str,
    pattern_code: &'static str,
    pattern_title: &'static str,
    variant: &'static str,
    version_label: &'static str,
    effective_from: Option<&'static str>,
    source_title: &'static str,
    source_url: Option<&'static str>,
    source_reference: &'static str,
    duration_minutes: Option<i64>,
    total_marks: f64,
    total_questions: i64,
    attempt_limit: Option<i64>,
    status: &'static str,
    notes: &'static str,
    sections: &'static [PatternSectionSeed],
}

struct PatternSectionSeed {
    code: &'static str,
    title: &'static str,
    display_order: i64,
    question_count: i64,
    max_attempts: Option<i64>,
    max_marks: f64,
    correct_marks: f64,
    incorrect_marks: f64,
    unanswered_marks: f64,
    question_types: &'static str,
    marking_variants: &'static str,
    numeric_tolerance: f64,
    partial_rule: &'static str,
    notes: &'static str,
    default_question_type: &'static str,
}

const PATTERN_SEEDS: &[PatternSeed] = &[
    PatternSeed {
        exam_code: "csir-net-physics",
        pattern_code: "csir-net-physical-sciences-705",
        pattern_title: "CSIR-UGC NET Physical Sciences (705)",
        variant: "Physical Sciences",
        version_label: "December 2026",
        effective_from: None,
        source_title: "Joint CSIR-UGC NET December 2026 Information Bulletin",
        source_url: Some(
            "https://cdnbbsr.s3waas.gov.in/s3efdf562ce2fb0ad460fd8e9d33e57f57/uploads/2026/10/202610071407707951.pdf",
        ),
        source_reference: "Official NTA information bulletin, Chapter 3 (Scheme of Examination), Physical Sciences (705).",
        duration_minutes: Some(180),
        total_marks: 200.0,
        total_questions: 75,
        attempt_limit: Some(55),
        status: PATTERN_STATUS_VERIFIED,
        notes: "First-N evaluation applies independently to Parts A, B and C. Negative marking is 25% in each part. All Physical Sciences questions are single-correct MCQ.",
        sections: &[
            PatternSectionSeed {
                code: "csir-705-part-a",
                title: "Part A - General Aptitude",
                display_order: 1,
                question_count: 20,
                max_attempts: Some(15),
                max_marks: 30.0,
                correct_marks: 2.0,
                incorrect_marks: -0.5,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":2.0,"incorrect":-0.5,"maxAttempts":15}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Maximum 15 of 20 questions evaluated; first 15 answered questions count.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "csir-705-part-b",
                title: "Part B - Subject",
                display_order: 2,
                question_count: 25,
                max_attempts: Some(20),
                max_marks: 70.0,
                correct_marks: 3.5,
                incorrect_marks: -0.875,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":3.5,"incorrect":-0.875,"maxAttempts":20}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Maximum 20 of 25 questions evaluated; first 20 answered questions count.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "csir-705-part-c",
                title: "Part C - Higher Order",
                display_order: 3,
                question_count: 30,
                max_attempts: Some(20),
                max_marks: 100.0,
                correct_marks: 5.0,
                incorrect_marks: -1.25,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":5.0,"incorrect":-1.25,"maxAttempts":20}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Maximum 20 of 30 questions evaluated; first 20 answered questions count. Physical Sciences Part C is single-correct MCQ.",
                default_question_type: "MCQ",
            },
        ],
    },
    PatternSeed {
        exam_code: "gate-physics",
        pattern_code: "gate-physics-ph",
        pattern_title: "GATE Physics (PH)",
        variant: "PH",
        version_label: "GATE 2027",
        effective_from: None,
        source_title: "GATE 2027 Information Brochure and Question Paper Pattern",
        source_url: Some("https://gate2027ib.iitm.ac.in/GATE2027-IB.pdf"),
        source_reference: "Official GATE 2027 Information Brochure, sections 7.2-7.3; official pattern page.",
        duration_minutes: Some(180),
        total_marks: 100.0,
        total_questions: 65,
        attempt_limit: None,
        status: PATTERN_STATUS_VERIFIED,
        notes: "GA carries 15 marks (5 x 1-mark + 5 x 2-mark questions). PH carries 85 marks across 55 questions with MCQ, MSQ and NAT. The official summary states 1- and 2-mark questions but does not publish a per-type PH count, so no per-type count is asserted.",
        sections: &[
            PatternSectionSeed {
                code: "gate-ga",
                title: "General Aptitude",
                display_order: 1,
                question_count: 10,
                max_attempts: None,
                max_marks: 15.0,
                correct_marks: 1.0,
                incorrect_marks: -0.3333333333333333,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","marks":1,"correct":1.0,"incorrect":-0.3333333333333333},{"questionType":"MCQ","marks":2,"correct":2.0,"incorrect":-0.6666666666666666}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Official GA composition: 5 one-mark and 5 two-mark questions; MCQ wrong answers receive negative marking.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "gate-ph",
                title: "Physics (PH)",
                display_order: 2,
                question_count: 55,
                max_attempts: None,
                max_marks: 85.0,
                correct_marks: 1.0,
                incorrect_marks: -0.3333333333333333,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ","MSQ","NAT"]"#,
                marking_variants: r#"[{"questionType":"MCQ","marks":1,"correct":1.0,"incorrect":-0.3333333333333333},{"questionType":"MCQ","marks":2,"correct":2.0,"incorrect":-0.6666666666666666},{"questionType":"MSQ","marks":1,"correct":1.0,"incorrect":0.0},{"questionType":"MSQ","marks":2,"correct":2.0,"incorrect":0.0},{"questionType":"NAT","marks":1,"correct":1.0,"incorrect":0.0},{"questionType":"NAT","marks":2,"correct":2.0,"incorrect":0.0}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "No partial marking for MSQ. No negative marking for MSQ or NAT.",
                notes: "PH totals are official: 55 questions and 85 marks. Per-type and per-mark counts are not published in the official 2027 pattern; aggregate mock entry requires explicit marks for this mixed section.",
                default_question_type: "MCQ",
            },
        ],
    },
    PatternSeed {
        exam_code: "tifr-gs-physics",
        pattern_code: "tifr-gs-physics-screening",
        pattern_title: "TIFR GS Physics Screening",
        variant: "Physics Screening",
        version_label: "GS 2024",
        effective_from: None,
        source_title: "TIFR GS 2024 Physics Entrance Test - Syllabus and Instructions",
        source_url: Some(
            "https://www.tifr.res.in/academics/docs/Physics_syllabus_exam_instructions.pdf",
        ),
        source_reference: "Official TIFR GS 2024 Physics Screening Test instructions.",
        duration_minutes: None,
        total_marks: 150.0,
        total_questions: 40,
        attempt_limit: None,
        status: PATTERN_STATUS_VERIFIED,
        notes: "Official document is labelled GS 2024. It states the instructions apply to Physics PhD and Integrated PhD screening. Duration is not stated in the supplied document. This is not a GS-2027 pattern.",
        sections: &[
            PatternSectionSeed {
                code: "tifr-gs-section-a",
                title: "Section A - Basic Topics",
                display_order: 1,
                question_count: 25,
                max_attempts: None,
                max_marks: 75.0,
                correct_marks: 3.0,
                incorrect_marks: -1.0,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":3.0,"incorrect":-1.0}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "All questions are single-correct multiple choice.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "tifr-gs-section-b",
                title: "Section B",
                display_order: 2,
                question_count: 15,
                max_attempts: None,
                max_marks: 75.0,
                correct_marks: 5.0,
                incorrect_marks: 0.0,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":5.0,"incorrect":0.0}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Official instructions describe these as questions that may require somewhat more thought/knowledge. All questions are single-correct multiple choice; no negative marking in Section B.",
                default_question_type: "MCQ",
            },
        ],
    },
    PatternSeed {
        exam_code: "jest-physics",
        pattern_code: "jest-physics",
        pattern_title: "JEST Physics",
        variant: "Physics",
        version_label: "JEST 2026",
        effective_from: None,
        source_title: "JEST 2026 Physics sample paper and FAQ",
        source_url: Some(
            "https://www.jest.org.in/sites/default/files/2026-01/sampleqp-jest2026.pdf",
        ),
        source_reference: "Official JEST 2026 FAQ, Exam-Specific item 5; official sample question paper.",
        duration_minutes: Some(180),
        total_marks: 100.0,
        total_questions: 40,
        attempt_limit: None,
        status: PATTERN_STATUS_VERIFIED,
        notes: "Part A: 10 MCQ, +1/-1/3. Part B: 20 MCQ, +3/-1. Part C: 10 numerical, +3, no negative marking. JEST 2027 remains pending.",
        sections: &[
            PatternSectionSeed {
                code: "jest-part-a",
                title: "Part A - 1-Mark MCQ",
                display_order: 1,
                question_count: 10,
                max_attempts: None,
                max_marks: 10.0,
                correct_marks: 1.0,
                incorrect_marks: -0.3333333333333333,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":1.0,"incorrect":-0.3333333333333333}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Single-correct MCQ.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "jest-part-b",
                title: "Part B - 3-Mark MCQ",
                display_order: 2,
                question_count: 20,
                max_attempts: None,
                max_marks: 60.0,
                correct_marks: 3.0,
                incorrect_marks: -1.0,
                unanswered_marks: 0.0,
                question_types: r#"["MCQ"]"#,
                marking_variants: r#"[{"questionType":"MCQ","correct":3.0,"incorrect":-1.0}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Single-correct MCQ.",
                default_question_type: "MCQ",
            },
            PatternSectionSeed {
                code: "jest-part-c",
                title: "Part C - Numerical",
                display_order: 3,
                question_count: 10,
                max_attempts: None,
                max_marks: 30.0,
                correct_marks: 3.0,
                incorrect_marks: 0.0,
                unanswered_marks: 0.0,
                question_types: r#"["NAT"]"#,
                marking_variants: r#"[{"questionType":"NAT","correct":3.0,"incorrect":0.0}]"#,
                numeric_tolerance: 0.0,
                partial_rule: "",
                notes: "Numerical answer; no negative marking.",
                default_question_type: "NAT",
            },
        ],
    },
];

const PENDING_PATTERNS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "tifr-gs-physics",
        "tifr-gs-physics-screening",
        "TIFR GS Physics Screening",
        "Physics Screening",
        "GS 2027 (pending official verification)",
        "GS 2027 pending official pattern verification; GS 2024 remains the latest verified version in this app.",
    ),
    (
        "jest-physics",
        "jest-physics",
        "JEST Physics",
        "Physics",
        "JEST 2027 (pending official announcement)",
        "JEST 2027 pending official announcement; JEST 2026 remains the latest verified version in this app.",
    ),
];

fn seed_patterns(connection: &mut Connection) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    for seed in PATTERN_SEEDS {
        let Some(exam_id) = exam_id(connection, seed.exam_code)? else {
            continue;
        };
        let Some(content_version_id) = syllabus_version(connection, exam_id)? else {
            continue;
        };
        let pattern_id = ensure_pattern(
            connection,
            exam_id,
            seed.pattern_code,
            seed.pattern_title,
            seed.variant,
            &now,
        )?;
        let version_id = ensure_version(
            connection,
            pattern_id,
            exam_id,
            seed.version_label,
            seed.effective_from,
            seed.source_title,
            seed.source_url,
            seed.source_reference,
            seed.duration_minutes,
            seed.total_marks,
            seed.total_questions,
            seed.attempt_limit,
            seed.status,
            seed.notes,
            &now,
        )?;
        for section in seed.sections {
            let declared_types: Vec<String> =
                serde_json::from_str(section.question_types).unwrap_or_default();
            for type_code in &declared_types {
                ensure_question_type(connection, exam_id, type_code, &now)?;
            }
            let type_id =
                ensure_question_type(connection, exam_id, section.default_question_type, &now)?;
            let section_id = ensure_exam_section(
                connection,
                exam_id,
                content_version_id,
                section,
                type_id,
                &now,
            )?;
            ensure_pattern_section(connection, version_id, exam_id, section_id, section, &now)?;
        }
    }

    for (exam_code, pattern_code, title, variant, version_label, note) in PENDING_PATTERNS {
        let Some(exam_id) = exam_id(connection, exam_code)? else {
            continue;
        };
        let pattern_id = ensure_pattern(connection, exam_id, pattern_code, title, variant, &now)?;
        let _ = ensure_version(
            connection,
            pattern_id,
            exam_id,
            version_label,
            None,
            "",
            None,
            "",
            None,
            0.0,
            0,
            None,
            PATTERN_STATUS_PENDING,
            note,
            &now,
        )?;
    }
    Ok(())
}

fn exam_id(connection: &Connection, code: &str) -> Result<Option<i64>, AppError> {
    connection
        .query_row("SELECT id FROM exams WHERE code = ?1", [code], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| AppError::database("Exam code could not be resolved", error))
}

fn syllabus_version(connection: &Connection, exam_id: i64) -> Result<Option<i64>, AppError> {
    connection
        .query_row(
            "SELECT id FROM exam_content_versions
             WHERE exam_id = ?1 AND content_type = 'syllabus'
             ORDER BY CASE review_status WHEN 'verified' THEN 0 ELSE 1 END, id DESC
             LIMIT 1",
            [exam_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| AppError::database("Syllabus version could not be resolved", error))
}

fn ensure_pattern(
    connection: &Connection,
    exam_id: i64,
    code: &str,
    title: &str,
    variant: &str,
    now: &str,
) -> Result<i64, AppError> {
    connection
        .execute(
            "INSERT INTO exam_patterns(exam_id, code, title, variant, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)
             ON CONFLICT(exam_id, code) DO NOTHING",
            params![exam_id, code, title, variant, now],
        )
        .map_err(|error| AppError::database("Exam pattern could not be seeded", error))?;
    connection
        .query_row(
            "SELECT id FROM exam_patterns WHERE exam_id = ?1 AND code = ?2",
            params![exam_id, code],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Exam pattern could not be read", error))
}

#[allow(clippy::too_many_arguments)]
fn ensure_version(
    connection: &Connection,
    pattern_id: i64,
    exam_id: i64,
    label: &str,
    effective_from: Option<&str>,
    source_title: &str,
    source_url: Option<&str>,
    source_reference: &str,
    duration_minutes: Option<i64>,
    total_marks: f64,
    total_questions: i64,
    attempt_limit: Option<i64>,
    status: &str,
    notes: &str,
    now: &str,
) -> Result<i64, AppError> {
    connection
        .execute(
            "INSERT INTO exam_pattern_versions(
                pattern_id, exam_id, version_label, effective_from, source_title, source_url,
                source_reference, verified_on, verification_status, duration_minutes,
                total_marks, total_questions, attempt_limit, notes, provenance, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
             ON CONFLICT(pattern_id, version_label) DO NOTHING",
            params![
                pattern_id,
                exam_id,
                label,
                effective_from,
                source_title,
                source_url,
                source_reference,
                if status == PATTERN_STATUS_VERIFIED {
                    Some("2026-11-05")
                } else {
                    None
                },
                status,
                duration_minutes,
                total_marks,
                total_questions,
                attempt_limit,
                notes,
                if status == PATTERN_STATUS_VERIFIED {
                    "Verified against the official source document during Milestone 10 implementation on 2026-11-05."
                } else {
                    "No production configuration has been approved for this version."
                },
                now,
            ],
        )
        .map_err(|error| AppError::database("Exam pattern version could not be seeded", error))?;
    connection
        .query_row(
            "SELECT id FROM exam_pattern_versions WHERE pattern_id = ?1 AND version_label = ?2",
            params![pattern_id, label],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Exam pattern version could not be read", error))
}

fn ensure_question_type(
    connection: &Connection,
    exam_id: i64,
    code: &str,
    now: &str,
) -> Result<i64, AppError> {
    let (title, format) = match code {
        "MCQ" => ("Multiple Choice (single correct)", "single_option"),
        "MSQ" => ("Multiple Select", "multiple_options"),
        "NAT" => ("Numerical Answer Type", "numeric"),
        other => (other, "single_option"),
    };
    connection
        .execute(
            "INSERT INTO exam_question_types(
                exam_id, code, title, answer_format, supports_partial_marks, is_archived, notes,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 0, 0, '', ?5, ?5)
             ON CONFLICT(exam_id, code) DO NOTHING",
            params![exam_id, code, title, format, now],
        )
        .map_err(|error| AppError::database("Question type could not be seeded", error))?;
    connection
        .query_row(
            "SELECT id FROM exam_question_types WHERE exam_id = ?1 AND code = ?2",
            params![exam_id, code],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Question type could not be read", error))
}

fn ensure_exam_section(
    connection: &Connection,
    exam_id: i64,
    content_version_id: i64,
    seed: &PatternSectionSeed,
    type_id: i64,
    now: &str,
) -> Result<i64, AppError> {
    connection
        .execute(
            "INSERT INTO exam_sections(
                exam_id, content_version_id, default_question_type_id, code, title,
                display_order, max_questions, default_correct_marks, default_incorrect_marks,
                default_unanswered_marks, is_archived, notes, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, ?12)
             ON CONFLICT(exam_id, code) DO NOTHING",
            params![
                exam_id,
                content_version_id,
                type_id,
                seed.code,
                seed.title,
                seed.display_order,
                seed.question_count,
                seed.correct_marks,
                seed.incorrect_marks,
                seed.unanswered_marks,
                seed.notes,
                now,
            ],
        )
        .map_err(|error| AppError::database("Exam section could not be seeded", error))?;
    connection
        .query_row(
            "SELECT id FROM exam_sections WHERE exam_id = ?1 AND code = ?2",
            params![exam_id, seed.code],
            |row| row.get(0),
        )
        .map_err(|error| AppError::database("Exam section could not be read", error))
}

fn ensure_pattern_section(
    connection: &Connection,
    version_id: i64,
    exam_id: i64,
    section_id: i64,
    seed: &PatternSectionSeed,
    now: &str,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO exam_pattern_sections(
                pattern_version_id, exam_id, section_id, code, title, display_order,
                question_count, max_attempts, max_marks, correct_marks, incorrect_marks,
                unanswered_marks, question_types_json, marking_variants_json,
                numeric_tolerance, partial_rule, notes, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)
             ON CONFLICT(pattern_version_id, code) DO NOTHING",
            params![
                version_id,
                exam_id,
                section_id,
                seed.code,
                seed.title,
                seed.display_order,
                seed.question_count,
                seed.max_attempts,
                seed.max_marks,
                seed.correct_marks,
                seed.incorrect_marks,
                seed.unanswered_marks,
                seed.question_types,
                seed.marking_variants,
                seed.numeric_tolerance,
                seed.partial_rule,
                seed.notes,
                now,
            ],
        )
        .map_err(|error| AppError::database("Pattern section could not be seeded", error))?;
    Ok(())
}
