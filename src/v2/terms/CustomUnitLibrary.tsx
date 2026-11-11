import { useEffect, useMemo, useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  deleteV2CustomUnit,
  listV2CustomUnits,
  type V2CustomUnit,
  type V2CustomUnitSiClassification,
} from '../api';
import {
  CONVERSION_KIND_LABELS,
  SI_CLASSIFICATION_LABELS,
  conversionSummary,
} from '../reference/catalog';
import { CustomUnitDetail } from './CustomUnitDetail';
import { CustomUnitEditor } from './CustomUnitEditor';

function matches(unit: V2CustomUnit, query: string): boolean {
  const term = query.trim().toLowerCase();
  if (!term) {
    return true;
  }
  return [
    unit.name,
    unit.symbol,
    unit.quantity,
    unit.dimension,
    unit.description,
    unit.notes,
    unit.source,
    unit.conversionFactor,
    unit.conversionReferenceUnit,
    unit.conversionNote,
  ].some((value) => value.toLowerCase().includes(term));
}

export function CustomUnitLibrary() {
  const [units, setUnits] = useState<V2CustomUnit[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [quantity, setQuantity] = useState('all');
  const [siClassification, setSiClassification] = useState<'all' | V2CustomUnitSiClassification>(
    'all',
  );
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [editing, setEditing] = useState<V2CustomUnit | null>(null);

  useEffect(() => {
    let cancelled = false;
    listV2CustomUnits()
      .then((entries) => {
        if (!cancelled) {
          setUnits(entries);
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

  const quantities = useMemo(
    () =>
      [...new Set((units ?? []).map((unit) => unit.quantity))].sort((left, right) =>
        left.localeCompare(right),
      ),
    [units],
  );

  const filtered = useMemo(
    () =>
      (units ?? []).filter(
        (unit) =>
          (quantity === 'all' || unit.quantity === quantity) &&
          (siClassification === 'all' || unit.siClassification === siClassification) &&
          matches(unit, query),
      ),
    [query, quantity, siClassification, units],
  );

  const selected = units?.find((unit) => unit.id === selectedId) ?? null;

  function clearFilters() {
    setQuery('');
    setQuantity('all');
    setSiClassification('all');
  }

  function openCreate() {
    setEditing(null);
    setEditorOpen(true);
  }

  function openEdit(unit: V2CustomUnit) {
    setEditing(unit);
    setEditorOpen(true);
  }

  async function remove(unit: V2CustomUnit) {
    if (!window.confirm(`Delete the custom unit "${unit.name}"? This cannot be undone.`)) {
      return;
    }
    try {
      await deleteV2CustomUnit(unit.id);
      setUnits((current) => (current ? current.filter((entry) => entry.id !== unit.id) : current));
      if (selectedId === unit.id) {
        setSelectedId(null);
      }
      setActionError(null);
    } catch (error) {
      setActionError(toCommandError(error).message);
    }
  }

  function mergeSaved(saved: V2CustomUnit) {
    setUnits((current) => {
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
        <CustomUnitDetail
          unit={selected}
          onBack={() => setSelectedId(null)}
          onDelete={() => void remove(selected)}
          onEdit={() => openEdit(selected)}
        />
        {editorOpen ? (
          <CustomUnitEditor
            unit={editing}
            onClose={() => setEditorOpen(false)}
            onSaved={mergeSaved}
          />
        ) : null}
      </div>
    );
  }

  const empty = units !== null && units.length === 0;
  const noMatches = units !== null && units.length > 0 && filtered.length === 0;

  return (
    <div className="v2-reference-library">
      <header className="v2-reference-library-header">
        <div>
          <h2>My units</h2>
          <p>
            Add units that are not in the built-in catalogue. Conversion details are stored as
            reference information and are never applied automatically.
          </p>
        </div>
        <button className="primary-button" onClick={openCreate} type="button">
          Add unit
        </button>
      </header>

      <section aria-label="My unit filters" className="v2-reference-toolbar">
        <label className="v2-field v2-reference-search">
          <span>Search my units</span>
          <input
            aria-label="Search my units"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, symbol, quantity, dimension or note"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Physical quantity</span>
          <select
            aria-label="Unit quantity filter"
            onChange={(event) => setQuantity(event.target.value)}
            value={quantity}
          >
            <option value="all">All quantities</option>
            {quantities.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>SI classification</span>
          <select
            aria-label="Unit SI classification filter"
            onChange={(event) =>
              setSiClassification(event.target.value as 'all' | V2CustomUnitSiClassification)
            }
            value={siClassification}
          >
            <option value="all">All classifications</option>
            {Object.entries(SI_CLASSIFICATION_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <button className="secondary-button" onClick={clearFilters} type="button">
          Clear filters
        </button>
      </section>

      <p aria-live="polite" className="v2-reference-count">
        {units === null
          ? 'Loading my units…'
          : `${filtered.length} unit${filtered.length === 1 ? '' : 's'}`}
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
          <h2>No custom units yet</h2>
          <p>
            Add a derived, non-SI or course-specific unit without changing the built-in SI
            reference.
          </p>
          <button className="primary-button" onClick={openCreate} type="button">
            Add your first unit
          </button>
        </div>
      ) : noMatches ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No custom units match these filters</h2>
          <p>Try a shorter search or choose a different quantity or SI classification.</p>
          <button className="secondary-button" onClick={clearFilters} type="button">
            Clear filters
          </button>
        </div>
      ) : (
        <ul className="v2-reference-card-list">
          {filtered.map((unit) => (
            <li key={unit.id}>
              <button
                aria-label={`Open custom unit ${unit.name}`}
                className="v2-reference-card"
                onClick={() => setSelectedId(unit.id)}
                type="button"
              >
                <span className="v2-reference-card-heading">
                  <span className="v2-reference-card-title">
                    {unit.name} <span className="v2-reference-card-symbol">({unit.symbol})</span>
                  </span>
                  <span className="v2-reference-badge" data-tone="user">
                    User-defined
                  </span>
                </span>
                <span className="v2-reference-card-value">
                  {unit.quantity}
                  {unit.dimension ? ` · [${unit.dimension}]` : ''}
                </span>
                <span className="v2-reference-card-description">{unit.description}</span>
                <span className="v2-reference-card-meta">
                  <span className="v2-reference-badge" data-tone="category">
                    {SI_CLASSIFICATION_LABELS[unit.siClassification]}
                  </span>
                  <span className="v2-reference-badge" data-tone="muted">
                    {CONVERSION_KIND_LABELS[unit.conversionKind]}
                  </span>
                  {unit.conversionKind !== 'none' ? (
                    <span className="v2-reference-badge" data-tone="conversion">
                      {conversionSummary(unit)}
                    </span>
                  ) : null}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {editorOpen ? (
        <CustomUnitEditor
          unit={editing}
          onClose={() => setEditorOpen(false)}
          onSaved={mergeSaved}
        />
      ) : null}
    </div>
  );
}
