import { useEffect, useRef } from 'react';
import type { V2BuiltinConstant, V2BuiltinConstantCatalog } from '../api';

type BuiltinConstantDetailProps = {
  catalog: V2BuiltinConstantCatalog;
  constant: V2BuiltinConstant;
  onBack: () => void;
};

export function BuiltinConstantDetail({ catalog, constant, onBack }: BuiltinConstantDetailProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <article className="v2-reference-detail" data-builtin-constant-id={constant.id}>
      <div className="v2-reference-detail-header">
        <button className="link-button" onClick={onBack} type="button">
          ← Back to built-in constants
        </button>
        <span className="v2-reference-badge" data-tone="user">
          Read-only
        </span>
      </div>

      <header className="v2-reference-detail-title">
        <div>
          <p className="eyebrow">{constant.category}</p>
          <h2 ref={headingRef} tabIndex={-1}>
            {constant.name}
          </h2>
        </div>
        <div className="v2-reference-badges">
          <span className="v2-reference-badge" data-tone="category">
            {catalog.versionLabel}
          </span>
          {constant.isExact ? (
            <span className="v2-reference-badge" data-tone="user">
              Exact
            </span>
          ) : null}
        </div>
      </header>

      <div className="v2-reference-value-panel">
        <span className="v2-reference-value-symbol">{constant.symbol}</span>
        <span className="v2-reference-value">{constant.valueText}</span>
        {constant.unit ? <span className="v2-reference-value-unit">{constant.unit}</span> : null}
      </div>

      <dl className="v2-reference-definition">
        <div>
          <dt>Symbol</dt>
          <dd>{constant.symbol}</dd>
        </div>
        <div>
          <dt>Value</dt>
          <dd className="v2-reference-verbatim">{constant.valueText}</dd>
        </div>
        <div>
          <dt>Unit</dt>
          <dd>{constant.unit || 'Dimensionless'}</dd>
        </div>
        <div>
          <dt>Uncertainty</dt>
          <dd>{constant.uncertaintyText || 'Not published'}</dd>
        </div>
        <div>
          <dt>Precision</dt>
          <dd>{constant.precisionText || 'Not published'}</dd>
        </div>
        <div>
          <dt>Catalogue</dt>
          <dd>
            {catalog.versionLabel} · edition {catalog.editionYear}
          </dd>
        </div>
        <div>
          <dt>Published</dt>
          <dd>{catalog.publishedOn}</dd>
        </div>
        <div>
          <dt>Retrieved</dt>
          <dd>{catalog.retrievedOn}</dd>
        </div>
        <div>
          <dt>Source locator</dt>
          <dd>{constant.sourceLocator}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{constant.sourceReference}</dd>
        </div>
      </dl>

      <section className="v2-reference-section">
        <h3>Description</h3>
        <p>{constant.description}</p>
      </section>

      {constant.notes ? (
        <section className="v2-reference-section">
          <h3>Notes</h3>
          <p>{constant.notes}</p>
        </section>
      ) : null}

      <section className="v2-reference-section">
        <h3>Catalogue citation</h3>
        <p>{catalog.citation}</p>
        {constant.sourceUrl ? (
          <p>
            <a href={constant.sourceUrl} rel="noreferrer" target="_blank">
              Open authoritative source
            </a>
          </p>
        ) : null}
      </section>

      <p className="v2-reference-disclaimer">
        Built-in values are immutable and shown exactly as published. Physics Tracker does not
        round, convert or recompute them.
      </p>
    </article>
  );
}
