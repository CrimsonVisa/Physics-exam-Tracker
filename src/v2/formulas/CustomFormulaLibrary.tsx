import { useEffect, useMemo, useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import { deleteV2CustomFormula, listV2CustomFormulas, type V2CustomFormula } from '../api';
import { FormulaEquation } from './FormulaEquation';
import { CustomFormulaDetail } from './CustomFormulaDetail';
import { CustomFormulaEditor } from './CustomFormulaEditor';

function matches(formula: V2CustomFormula, query: string): boolean {
  const term = query.trim().toLowerCase();
  if (!term) {
    return true;
  }
  return [
    formula.name,
    formula.latex,
    formula.plainText,
    formula.category,
    formula.description,
    formula.notes,
    formula.source,
    ...formula.variables.flatMap((variable) => [variable.symbol, variable.meaning, variable.unit]),
  ].some((value) => value.toLowerCase().includes(term));
}

export function CustomFormulaLibrary() {
  const [formulas, setFormulas] = useState<V2CustomFormula[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<V2CustomFormula | null>(null);

  useEffect(() => {
    let cancelled = false;
    listV2CustomFormulas()
      .then((entries) => {
        if (!cancelled) {
          setFormulas(entries);
          setLoadError(null);
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const categories = useMemo(
    () =>
      [...new Set((formulas ?? []).map((formula) => formula.category))].sort((left, right) =>
        left.localeCompare(right),
      ),
    [formulas],
  );

  const filtered = useMemo(
    () =>
      (formulas ?? []).filter(
        (formula) =>
          (category === 'all' || formula.category === category) && matches(formula, query),
      ),
    [category, formulas, query],
  );

  const selected = formulas?.find((formula) => formula.id === selectedId) ?? null;

  function openCreate() {
    setEditing(null);
    setEditorOpen(true);
  }

  function openEdit(formula: V2CustomFormula) {
    setEditing(formula);
    setEditorOpen(true);
  }

  async function remove(formula: V2CustomFormula) {
    if (!window.confirm(`Delete the custom formula "${formula.name}"? This cannot be undone.`)) {
      return;
    }
    try {
      await deleteV2CustomFormula(formula.id);
      setFormulas((current) =>
        current ? current.filter((entry) => entry.id !== formula.id) : current,
      );
      if (selectedId === formula.id) {
        setSelectedId(null);
      }
      setActionError(null);
    } catch (error) {
      setActionError(toCommandError(error).message);
    }
  }

  function mergeSaved(saved: V2CustomFormula) {
    setFormulas((current) => {
      if (!current) {
        return [saved];
      }
      return current.some((entry) => entry.id === saved.id)
        ? current.map((entry) => (entry.id === saved.id ? saved : entry))
        : [...current, saved].sort((left, right) => left.name.localeCompare(right.name));
    });
    setSelectedId(saved.id);
    setEditorOpen(false);
  }

  if (selected) {
    return (
      <div className="v2-reference-library">
        {actionError ? (
          <p className="v2-inline-error" role="alert">
            {actionError}
          </p>
        ) : null}
        <CustomFormulaDetail
          formula={selected}
          onBack={() => setSelectedId(null)}
          onDelete={() => void remove(selected)}
          onEdit={() => openEdit(selected)}
        />
        {editorOpen ? (
          <CustomFormulaEditor
            formula={editing}
            onClose={() => setEditorOpen(false)}
            onSaved={mergeSaved}
          />
        ) : null}
      </div>
    );
  }

  const empty = formulas !== null && formulas.length === 0;
  const noMatches = formulas !== null && formulas.length > 0 && filtered.length === 0;

  return (
    <div className="v2-reference-library">
      <header className="v2-reference-library-header">
        <div>
          <h2>My formulas</h2>
          <p>
            Keep your own equations alongside the built-in library. Both LaTeX and plain-text
            expressions are preserved as reference text.
          </p>
        </div>
        <button className="primary-button" onClick={openCreate} type="button">
          Add formula
        </button>
      </header>

      <section aria-label="My formula filters" className="v2-reference-toolbar">
        <label className="v2-field v2-reference-search">
          <span>Search my formulas</span>
          <input
            aria-label="Search my formulas"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, expression, variable, category or note"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Category</span>
          <select
            aria-label="Formula category filter"
            onChange={(event) => setCategory(event.target.value)}
            value={category}
          >
            <option value="all">All categories</option>
            {categories.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <button
          className="secondary-button"
          onClick={() => {
            setQuery('');
            setCategory('all');
          }}
          type="button"
        >
          Clear filters
        </button>
      </section>

      <p aria-live="polite" className="v2-reference-count">
        {formulas === null
          ? 'Loading my formulas…'
          : `${filtered.length} formula${filtered.length === 1 ? '' : 's'}`}
      </p>

      {loadError ? (
        <p className="v2-inline-error" role="alert">
          {loadError}
        </p>
      ) : null}
      {actionError ? (
        <p className="v2-inline-error" role="alert">
          {actionError}
        </p>
      ) : null}

      {empty ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No custom formulas yet</h2>
          <p>
            Add a formula when you want a personal equation, derivation aid or course-specific
            relation in the same reference workspace.
          </p>
          <button className="primary-button" onClick={openCreate} type="button">
            Add your first formula
          </button>
        </div>
      ) : noMatches ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No custom formulas match these filters</h2>
          <p>Try a shorter search or choose a different category.</p>
          <button
            className="secondary-button"
            onClick={() => {
              setQuery('');
              setCategory('all');
            }}
            type="button"
          >
            Clear filters
          </button>
        </div>
      ) : (
        <ul className="v2-reference-card-list">
          {filtered.map((formula) => (
            <li key={formula.id}>
              <button
                aria-label={`Open custom formula ${formula.name}`}
                className="v2-reference-card"
                onClick={() => setSelectedId(formula.id)}
                type="button"
              >
                <span className="v2-reference-card-heading">
                  <span className="v2-reference-card-title">{formula.name}</span>
                  <span className="v2-reference-badge" data-tone="user">
                    User-defined
                  </span>
                </span>
                <span className="v2-reference-card-equation">
                  <FormulaEquation latex={formula.latex} plainText={formula.plainText} />
                </span>
                <span className="v2-reference-card-description">{formula.description}</span>
                <span className="v2-reference-card-meta">
                  <span className="v2-reference-badge" data-tone="category">
                    {formula.category}
                  </span>
                  <span className="v2-reference-badge" data-tone="muted">
                    {formula.variables.length} variable
                    {formula.variables.length === 1 ? '' : 's'}
                  </span>
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {editorOpen ? (
        <CustomFormulaEditor
          formula={editing}
          onClose={() => setEditorOpen(false)}
          onSaved={mergeSaved}
        />
      ) : null}
    </div>
  );
}
