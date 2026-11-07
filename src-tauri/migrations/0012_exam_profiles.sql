-- Stage 1: four-examination profiles and exactly one optional date per exam.
--
-- Existing CSIR data remains in the original tables. The legacy singleton
-- settings.exam_date is intentionally not copied into the CSIR profile because
-- the database has no field-level audit proving whether it was user-entered or
-- a built-in default. The profile starts NULL and the legacy value remains in
-- settings for compatibility and explicit user confirmation.

CREATE TABLE exams (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE
        CHECK (length(code) BETWEEN 1 AND 64),
    name TEXT NOT NULL
        CHECK (length(trim(name)) BETWEEN 1 AND 200),
    short_name TEXT NOT NULL
        CHECK (length(trim(short_name)) BETWEEN 1 AND 80),
    authority TEXT NOT NULL DEFAULT ''
        CHECK (length(authority) <= 300),
    official_url TEXT NOT NULL DEFAULT ''
        CHECK (length(official_url) <= 2048),
    display_order INTEGER NOT NULL UNIQUE
        CHECK (display_order >= 0),
    is_builtin INTEGER NOT NULL DEFAULT 1
        CHECK (is_builtin IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE exam_profiles (
    exam_id INTEGER PRIMARY KEY
        REFERENCES exams(id) ON DELETE RESTRICT,
    is_active INTEGER NOT NULL DEFAULT 1
        CHECK (is_active IN (0, 1)),
    exam_date TEXT
        CHECK (
            exam_date IS NULL
            OR (
                length(exam_date) = 10
                AND exam_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
                AND exam_date BETWEEN '0001-01-01' AND '9999-12-31'
                AND date(exam_date) IS exam_date
            )
        ),
    legacy_date_notice_state TEXT NOT NULL DEFAULT 'none'
        CHECK (
            legacy_date_notice_state IN (
                'none',
                'pending',
                'dismissed',
                'transferred'
            )
        ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (legacy_date_notice_state <> 'pending' OR exam_date IS NULL)
) STRICT;

CREATE INDEX idx_exam_profiles_active_date
    ON exam_profiles(is_active, exam_date);

INSERT INTO exams (
    code,
    name,
    short_name,
    authority,
    official_url,
    display_order,
    is_builtin,
    created_at,
    updated_at
) VALUES
    (
        'csir-net-physics',
        'CSIR-UGC NET Physical Sciences',
        'CSIR-NET',
        'CSIR-HRDG / National Testing Agency',
        'https://csirnet.nta.nic.in/',
        1,
        1,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        'gate-physics',
        'GATE Physics (PH)',
        'GATE PH',
        'Indian Institute of Technology Madras',
        'https://gate2027.iitm.ac.in/',
        2,
        1,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        'tifr-gs-physics',
        'TIFR Graduate School Physics',
        'TIFR GS',
        'Tata Institute of Fundamental Research',
        'https://www.tifr.res.in/academics/',
        3,
        1,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        'jest-physics',
        'JEST Physics',
        'JEST',
        'JEST Consortium',
        'https://www.jest.org.in/',
        4,
        1,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    );

INSERT INTO exam_profiles (
    exam_id,
    is_active,
    exam_date,
    legacy_date_notice_state,
    created_at,
    updated_at
)
SELECT
    e.id,
    1,
    NULL,
    CASE
        WHEN e.code = 'csir-net-physics'
             AND EXISTS (SELECT 1 FROM settings WHERE id = 1)
        THEN 'pending'
        ELSE 'none'
    END,
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
    strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
FROM exams e;
