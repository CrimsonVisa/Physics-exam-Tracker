import { useEffect, useMemo, useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import {
  getV2BuiltinConstants,
  type V2BuiltinConstant,
  type V2BuiltinConstantCatalog,
} from '../api';
import { BuiltinConstantDetail } from './BuiltinConstantDetail';

function matches(constant: V2BuiltinConstant, query: string): boolean {
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
    constant.description,
    constant.notes,
    constant.sourceLocator,
    constant.sourceReference,
  ].some((value) => value.toLowerCase().includes(term));
}

export function BuiltinConstantsLibrary() {
  const [catalog, setCatalog] = useState<V2BuiltinConstantCatalog | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('all');
  const [selectedId, setSelectedId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    getV2BuiltinConstants()
      .then((next) => {
        if (!cancelled) {
          setCatalog(next);
          setError(null);
        }
      })
      .catch((loadError) => {
        if (!cancelled) {
          setError(toCommandError(loadError).message);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const categories = useMemo(
    () =>
      [...new Set((catalog?.constants ?? []).map((constant) => constant.category))].sort(
        (left, right) => left.localeCompare(right),
      ),
    [catalog],
  );
  const filtered = useMemo(
    () =>
      (catalog?.constants ?? []).filter(
        (constant) =>
          (category === 'all' || constant.category === category) && matches(constant, query),
      ),
    [catalog, category, query],
  );
  const selected = catalog?.constants.find((constant) => constant.id === selectedId) ?? null;

  if (selected && catalog) {
    return (
      <div className="v2-reference-library">
        <BuiltinConstantDetail
          catalog={catalog}
          constant={selected}
          onBack={() => setSelectedId(null)}
        />
      </div>
    );
  }

  return (
    <div className="v2-reference-library">
      <header className="v2-reference-library-header">
        <div>
          <h2>Built-in constants</h2>
          <p>
            Read-only physical constants from {catalog?.versionLabel ?? 'the shipped catalogue'}.
            Values are preserved exactly as published by the authoritative source.
          </p>
        </div>
      </header>

      <section aria-label="Built-in constant filters" className="v2-reference-toolbar">
        <label className="v2-field v2-reference-search">
          <span>Search built-in constants</span>
          <input
            aria-label="Search built-in constants"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, symbol, value, unit, category or source"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Category</span>
          <select
            aria-label="Built-in constant category filter"
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
        {catalog === null
          ? 'Loading built-in constants…'
          : `${filtered.length} constant${filtered.length === 1 ? '' : 's'}`}
      </p>

      {error ? (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      ) : null}

      {catalog && filtered.length === 0 ? (
        <div className="v2-empty-panel v2-reference-empty">
          <h2>No built-in constants match these filters</h2>
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
                aria-label={`Open built-in constant ${constant.name}`}
                className="v2-reference-card"
                onClick={() => setSelectedId(constant.id)}
                type="button"
              >
                <span className="v2-reference-card-heading">
                  <span className="v2-reference-card-title">{constant.name}</span>
                  <span className="v2-reference-badge" data-tone="user">
                    Built-in
                  </span>
                </span>
                <span className="v2-reference-card-value">
                  {constant.symbol} = {constant.valueText}
                  {constant.unit ? ` ${constant.unit}` : ''}
                </span>
                <span className="v2-reference-card-description">{constant.description}</span>
                <span className="v2-reference-card-meta">
                  <span className="v2-reference-badge" data-tone="category">
                    {constant.category}
                  </span>
                  {constant.isExact ? (
                    <span className="v2-reference-badge" data-tone="muted">
                      exact
                    </span>
                  ) : (
                    <span className="v2-reference-badge" data-tone="muted">
                      uncertainty {constant.uncertaintyText}
                    </span>
                  )}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {catalog ? (
        <p className="v2-reference-disclaimer">
          {catalog.publisher} · {catalog.citation}
        </p>
      ) : null}
    </div>
  );
}
