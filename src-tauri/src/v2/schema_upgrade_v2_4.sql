-- V2 schema upgrade: version 3 -> 4 (custom physics references).
--
-- Applied in place by open_or_initialize when an existing database carries
-- user_version 3. Fresh databases receive these tables directly from
-- schema.sql, so every statement here is idempotent. The versioned
-- schema_migrations ledger row and the app_metadata schema version are written
-- by the upgrade routine after this batch succeeds. No existing table or row
-- is dropped or altered.

CREATE TABLE IF NOT EXISTS custom_formulas (
    id INTEGER PRIMARY KEY,
    is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    latex TEXT NOT NULL CHECK (length(trim(latex)) BETWEEN 1 AND 20000),
    plain_text TEXT NOT NULL CHECK (length(trim(plain_text)) BETWEEN 1 AND 20000),
    category TEXT NOT NULL CHECK (length(trim(category)) BETWEEN 1 AND 120),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 6000),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 6000),
    source TEXT NOT NULL DEFAULT '' CHECK (length(source) <= 1000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_formulas_name
    ON custom_formulas(name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_custom_formulas_category
    ON custom_formulas(category COLLATE NOCASE, name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS custom_formula_variables (
    id INTEGER PRIMARY KEY,
    formula_id INTEGER NOT NULL
        REFERENCES custom_formulas(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    symbol TEXT NOT NULL CHECK (length(trim(symbol)) BETWEEN 1 AND 80),
    meaning TEXT NOT NULL CHECK (length(trim(meaning)) BETWEEN 1 AND 500),
    unit TEXT NOT NULL DEFAULT '' CHECK (length(unit) <= 120)
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_formula_variables_position
    ON custom_formula_variables(formula_id, position);
CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_formula_variables_symbol
    ON custom_formula_variables(formula_id, symbol COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS custom_units (
    id INTEGER PRIMARY KEY,
    is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 200),
    symbol TEXT NOT NULL CHECK (length(trim(symbol)) BETWEEN 1 AND 80),
    quantity TEXT NOT NULL CHECK (length(trim(quantity)) BETWEEN 1 AND 200),
    dimension TEXT NOT NULL DEFAULT '' CHECK (length(dimension) <= 200),
    si_classification TEXT NOT NULL
        CHECK (si_classification IN ('si_base', 'si_derived', 'non_si')),
    conversion_kind TEXT NOT NULL DEFAULT 'none'
        CHECK (conversion_kind IN ('none', 'scalar', 'contextual')),
    conversion_factor TEXT NOT NULL DEFAULT '' CHECK (length(conversion_factor) <= 200),
    conversion_reference_unit TEXT NOT NULL DEFAULT ''
        CHECK (length(conversion_reference_unit) <= 120),
    conversion_note TEXT NOT NULL DEFAULT '' CHECK (length(conversion_note) <= 2000),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 6000),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 6000),
    source TEXT NOT NULL DEFAULT '' CHECK (length(source) <= 1000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (conversion_kind = 'none'
            AND length(trim(conversion_factor)) = 0
            AND length(trim(conversion_reference_unit)) = 0
            AND length(trim(conversion_note)) = 0)
        OR (conversion_kind = 'scalar'
            AND length(trim(conversion_factor)) > 0
            AND length(trim(conversion_reference_unit)) > 0)
        OR (conversion_kind = 'contextual'
            AND length(trim(conversion_factor)) = 0
            AND length(trim(conversion_note)) > 0)
    )
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_units_name
    ON custom_units(name COLLATE NOCASE);
CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_units_symbol
    ON custom_units(symbol COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_custom_units_quantity
    ON custom_units(quantity COLLATE NOCASE, name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS custom_constants (
    id INTEGER PRIMARY KEY,
    is_builtin INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 300),
    symbol TEXT NOT NULL CHECK (length(trim(symbol)) BETWEEN 1 AND 80),
    value_text TEXT NOT NULL CHECK (length(trim(value_text)) BETWEEN 1 AND 200),
    unit TEXT NOT NULL DEFAULT '' CHECK (length(unit) <= 120),
    category TEXT NOT NULL CHECK (length(trim(category)) BETWEEN 1 AND 120),
    uncertainty_text TEXT NOT NULL DEFAULT '' CHECK (length(uncertainty_text) <= 200),
    precision_text TEXT NOT NULL DEFAULT '' CHECK (length(precision_text) <= 120),
    source TEXT NOT NULL DEFAULT '' CHECK (length(source) <= 1000),
    description TEXT NOT NULL CHECK (length(trim(description)) BETWEEN 1 AND 6000),
    notes TEXT NOT NULL DEFAULT '' CHECK (length(notes) <= 6000),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_constants_name
    ON custom_constants(name COLLATE NOCASE);
CREATE UNIQUE INDEX IF NOT EXISTS idx_custom_constants_symbol
    ON custom_constants(symbol COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_custom_constants_category
    ON custom_constants(category COLLATE NOCASE, name COLLATE NOCASE);

CREATE TRIGGER IF NOT EXISTS trg_custom_formulas_builtin_no_update
BEFORE UPDATE ON custom_formulas
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom formulas are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_formulas_builtin_no_delete
BEFORE DELETE ON custom_formulas
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom formulas are protected'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_formula_variables_builtin_no_insert
BEFORE INSERT ON custom_formula_variables
WHEN (SELECT is_builtin FROM custom_formulas WHERE id = NEW.formula_id) = 1
BEGIN SELECT RAISE(ABORT, 'built-in formula variables are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_formula_variables_builtin_no_update
BEFORE UPDATE ON custom_formula_variables
WHEN (SELECT is_builtin FROM custom_formulas WHERE id = OLD.formula_id) = 1
BEGIN SELECT RAISE(ABORT, 'built-in formula variables are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_formula_variables_builtin_no_delete
BEFORE DELETE ON custom_formula_variables
WHEN (SELECT is_builtin FROM custom_formulas WHERE id = OLD.formula_id) = 1
BEGIN SELECT RAISE(ABORT, 'built-in formula variables are protected'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_units_builtin_no_update
BEFORE UPDATE ON custom_units
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom units are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_units_builtin_no_delete
BEFORE DELETE ON custom_units
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom units are protected'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_constants_builtin_no_update
BEFORE UPDATE ON custom_constants
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom constants are immutable'); END;

CREATE TRIGGER IF NOT EXISTS trg_custom_constants_builtin_no_delete
BEFORE DELETE ON custom_constants
WHEN OLD.is_builtin = 1
BEGIN SELECT RAISE(ABORT, 'built-in custom constants are protected'); END;
