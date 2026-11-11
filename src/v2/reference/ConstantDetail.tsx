import { useEffect, useRef } from 'react';
import type { V2CustomConstant } from '../api';

type ConstantDetailProps = {
  constant: V2CustomConstant;
  onBack: () => void;
  onDelete: () => void;
  onEdit: () => void;
};

export function ConstantDetail({ constant, onBack, onDelete, onEdit }: ConstantDetailProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <article className="v2-reference-detail" data-constant-id={constant.id}>
      <div className="v2-reference-detail-header">
        <button className="link-button" onClick={onBack} type="button">
          ← Back to constants
        </button>
        <div className="v2-reference-detail-actions">
          {constant.isBuiltin ? null : (
            <>
              <button className="secondary-button" onClick={onEdit} type="button">
                Edit constant
              </button>
              <button
                className="secondary-button v2-danger-button"
                onClick={onDelete}
                type="button"
              >
                Delete constant
              </button>
            </>
          )}
        </div>
      </div>

      <header className="v2-reference-detail-title">
        <div>
          <p className="eyebrow">{constant.category}</p>
          <h2 ref={headingRef} tabIndex={-1}>
            {constant.name}
          </h2>
        </div>
        <div className="v2-reference-badges">
          <span className="v2-reference-badge" data-tone="user">
            User-defined
          </span>
          <span className="v2-reference-badge" data-tone="category">
            {constant.category}
          </span>
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
          <dd>{constant.unit || 'Dimensionless / not specified'}</dd>
        </div>
        <div>
          <dt>Category</dt>
          <dd>{constant.category}</dd>
        </div>
        <div>
          <dt>Uncertainty</dt>
          <dd>{constant.uncertaintyText || 'Not specified'}</dd>
        </div>
        <div>
          <dt>Precision</dt>
          <dd>{constant.precisionText || 'Not specified'}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{constant.source || 'No source recorded'}</dd>
        </div>
        <div>
          <dt>Last updated</dt>
          <dd>{constant.updatedAt}</dd>
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

      <p className="v2-reference-disclaimer">
        Numerical text is preserved verbatim. Physics Tracker does not silently round or convert
        this value.
      </p>
    </article>
  );
}
