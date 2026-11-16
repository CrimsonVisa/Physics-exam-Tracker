import { useEffect, useMemo, useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import { deleteV2CustomConstant, listV2CustomConstants, type V2CustomConstant } from '../api';
import { MathText } from '../MathText';
import { MathExpression } from '../math/MathExpression';
import { CONSTANT_CATEGORIES } from './catalog';
import { ConstantDetail } from './ConstantDetail';
import { ConstantEditor } from './ConstantEditor';
import { BuiltinConstantsLibrary } from './BuiltinConstantsLibrary';
import { ReferencePanel, ReferenceTabs } from './ReferenceTabs';

function matches(constant: V2CustomConstant, query: string): boolean {
  const term = query.trim().toLowerCase();
  if (!term) {
    return true;
  }
  return [
    constant.name,
    constant.symbol,
    constant.valueText,
    constant.unit,
    constant.category,
    constant.uncertaintyText,
    constant.precisionText,
    constant.source,
    constant.description,
    constant.notes,
  ].some((value) => value.toLowerCase().includes(term));
}

function CustomConstantsLibrary() {
  const [constants, setConstants] = useState<V2CustomConstant[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<V2CustomConstant | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    listV2CustomConstants()
      .then((entries) => {
        if (!cancelled) {
          setConstants(entries);
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

  const categories = useMemo(() => {
    const values = new Set<string>(CONSTANT_CATEGORIES);
    for (const constant of constants ?? []) {
      values.add(constant.category);
    }
    return [...values].sort((left, right) => left.localeCompare(right));
  }, [constants]);

  const filtered = useMemo(
    () =>
      (constants ?? []).filter(
        (constant) =>
          (category === 'all' || constant.category === category) && matches(constant, query),
      ),
    [category, constants, query],
  );

  const selected = constants?.find((constant) => constant.id === selectedId) ?? null;

  function openCreate() {
    setEditing(null);
    setEditorOpen(true);
    setNotice(null);
    setActionError(null);
  }

  function openEdit(constant: V2CustomConstant) {
    setEditing(constant);
    setEditorOpen(true);
    setNotice(null);
    setActionError(null);
  }

  async function remove(constant: V2CustomConstant) {
    if (!window.confirm(`Delete the custom constant "${constant.name}"? This cannot be undone.`)) {
      return;
    }
    try {
      await deleteV2CustomConstant(constant.id);
      setConstants((current) =>
        current ? current.filter((entry) => entry.id !== constant.id) : current,
      );
      if (selectedId === constant.id) {
        setSelectedId(null);
      }
      setActionError(null);
      setNotice(`Constant "${constant.name}" deleted.`);
    } catch (error) {
      setNotice(null);
      setActionError(toCommandError(error).message);
    }
  }

  function handleSaved(saved: V2CustomConstant) {
    setConstants((current) => {
      if (!current) {
        return [saved];
      }
      return current.some((entry) => entry.id === saved.id)
        ? current.map((entry) => (entry.id === saved.id ? saved : entry))
        : [...current, saved].sort((left, right) => left.name.localeCompare(right.name));
    });
    setSelectedId(saved.id);
    setEditorOpen(false);
    setQuery('');
    setCategory('all');
    setActionError(null);
    setNotice(`Constant "${saved.name}" saved.`);
  }

  if (selected) {
    return (
      <div className="v2-reference-library">
        {actionError ? (
          <p className="v2-inline-error" role="alert">
            {actionError}
          </p>
        ) : null}
        {notice ? (
          <p className="v2-study-notice" role="status">
            {notice}
          </p>
        ) : null}
        <ConstantDetail
          constant={selected}
          onBack={() => setSelectedId(null)}
          onDelete={() => void remove(selected)}
          onEdit={() => openEdit(selected)}
        />
        {editorOpen ? (
          <ConstantEditor
            constant={editing}
            onClose={() => setEditorOpen(false)}
            onSaved={handleSaved}
          />
        ) : null}
      </div>
    );
  }

  return (
    <div className="v2-reference-library">
      <header className="v2-reference-library-header">
        <div>
          <h2>Constants</h2>
          <p>
            Store your own physical constants and numerical reference values. Values remain exactly
            as written, including scientific notation and precision.
          </p>
        </div>
        <button className="primary-button" onClick={openCreate} type="button">
          Add constant
        </button>
      </header>

      <section aria-label="Constant filters" className="v2-reference-toolbar">
        <label className="v2-field v2-reference-search">
          <span>Search constants</span>
          <input
            aria-label="Search constants"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, symbol, value, unit, category or note"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Category</span>
          <select
            aria-label="Constant category filter"
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
        {constants === null
          ? 'Loading constants…'
          : `${filtered.length} constant${filtered.length === 1 ? '' : 's'}`}
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
      {notice ? (
        <p className="v2-study-notice" role="status">
          {notice}
        </p>
      ) : null}

      {constants !== null && constants.length === 0 ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No custom constants yet</h2>
          <p>
            Add a constant or reference value to keep it alongside your other physics reference
            material.
          </p>
          <button className="primary-button" onClick={openCreate} type="button">
            Add your first constant
          </button>
        </div>
      ) : constants !== null && filtered.length === 0 ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No constants match these filters</h2>
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
          {filtered.map((constant) => (
            <li key={constant.id}>
              <button
                aria-label={`Open constant ${constant.name}`}
                className="v2-reference-card"
                onClick={() => setSelectedId(constant.id)}
                type="button"
              >
                <span className="v2-reference-card-heading">
                  <span className="v2-reference-card-title">{constant.name}</span>
                  <span className="v2-reference-badge" data-tone="user">
                    User-defined
                  </span>
                </span>
                <span className="v2-reference-card-value">
                  <MathExpression latex={constant.symbol} plainText={constant.symbol} /> ={' '}
                  {constant.valueText}
                  {constant.unit ? (
                    <>
                      {' '}
                      <MathExpression latex={constant.unit} plainText={constant.unit} />
                    </>
                  ) : null}
                </span>
                <span className="v2-reference-card-description">
                  <MathText text={constant.description} />
                </span>
                <span className="v2-reference-card-meta">
                  <span className="v2-reference-badge" data-tone="category">
                    {constant.category}
                  </span>
                  {constant.uncertaintyText ? (
                    <span className="v2-reference-badge" data-tone="muted">
                      uncertainty {constant.uncertaintyText}
                    </span>
                  ) : null}
                  {constant.precisionText ? (
                    <span className="v2-reference-badge" data-tone="muted">
                      {constant.precisionText}
                    </span>
                  ) : null}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {editorOpen ? (
        <ConstantEditor
          constant={editing}
          onClose={() => setEditorOpen(false)}
          onSaved={handleSaved}
        />
      ) : null}
    </div>
  );
}

export function ConstantsLibrary() {
  const [tab, setTab] = useState<'constants-builtin' | 'constants-custom'>('constants-builtin');
  return (
    <div className="v2-reference-library">
      <ReferenceTabs
        active={tab}
        ariaLabel="Constants sections"
        onChange={setTab}
        options={[
          { id: 'constants-builtin', label: 'Built-in Constants' },
          { id: 'constants-custom', label: 'My Constants' },
        ]}
      />
      {tab === 'constants-builtin' ? (
        <ReferencePanel tab="constants-builtin">
          <BuiltinConstantsLibrary />
        </ReferencePanel>
      ) : (
        <ReferencePanel tab="constants-custom">
          <CustomConstantsLibrary />
        </ReferencePanel>
      )}
    </div>
  );
}
