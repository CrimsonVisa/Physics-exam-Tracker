import { useEffect, useRef } from 'react';
import type { V2CustomUnit } from '../api';
import { MathText } from '../MathText';
import { MathExpression } from '../math/MathExpression';
import { CONVERSION_KIND_LABELS, SI_CLASSIFICATION_LABELS } from '../reference/catalog';
import { UnitConversionSummary } from './UnitConversionSummary';

type CustomUnitDetailProps = {
  unit: V2CustomUnit;
  onBack: () => void;
  onDelete: () => void;
  onEdit: () => void;
};

export function CustomUnitDetail({ unit, onBack, onDelete, onEdit }: CustomUnitDetailProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <article className="v2-reference-detail" data-unit-id={unit.id}>
      <div className="v2-reference-detail-header">
        <button className="link-button" onClick={onBack} type="button">
          ← Back to my units
        </button>
        <div className="v2-reference-detail-actions">
          {unit.isBuiltin ? null : (
            <>
              <button className="secondary-button" onClick={onEdit} type="button">
                Edit unit
              </button>
              <button
                className="secondary-button v2-danger-button"
                onClick={onDelete}
                type="button"
              >
                Delete unit
              </button>
            </>
          )}
        </div>
      </div>

      <header className="v2-reference-detail-title">
        <div>
          <p className="eyebrow">{unit.quantity}</p>
          <h2 ref={headingRef} tabIndex={-1}>
            {unit.name}
          </h2>
        </div>
        <div className="v2-reference-badges">
          <span className="v2-reference-badge" data-tone="user">
            {unit.isBuiltin ? 'Built-in' : 'User-defined'}
          </span>
          <span className="v2-reference-badge" data-tone="category">
            {SI_CLASSIFICATION_LABELS[unit.siClassification]}
          </span>
        </div>
      </header>

      <div className="v2-reference-value-panel">
        <span className="v2-reference-value-symbol">
          <MathExpression latex={unit.symbol} plainText={unit.symbol} />
        </span>
        <span className="v2-reference-value">{unit.quantity}</span>
        {unit.dimension ? (
          <span className="v2-reference-value-unit">
            [<MathExpression latex={unit.dimension} plainText={unit.dimension} />]
          </span>
        ) : null}
      </div>

      <dl className="v2-reference-definition">
        <div>
          <dt>Symbol</dt>
          <dd>
            <MathExpression latex={unit.symbol} plainText={unit.symbol} />
          </dd>
        </div>
        <div>
          <dt>Physical quantity</dt>
          <dd>{unit.quantity}</dd>
        </div>
        <div>
          <dt>Dimension</dt>
          <dd>
            {unit.dimension ? (
              <MathExpression latex={unit.dimension} plainText={unit.dimension} />
            ) : (
              'Not specified'
            )}
          </dd>
        </div>
        <div>
          <dt>SI classification</dt>
          <dd>{SI_CLASSIFICATION_LABELS[unit.siClassification]}</dd>
        </div>
        <div>
          <dt>Conversion kind</dt>
          <dd>{CONVERSION_KIND_LABELS[unit.conversionKind]}</dd>
        </div>
        <div>
          <dt>Conversion</dt>
          <dd>
            <UnitConversionSummary unit={unit} />
          </dd>
        </div>
        {unit.conversionKind === 'scalar' ? (
          <>
            <div>
              <dt>Conversion factor</dt>
              <dd className="v2-reference-verbatim">{unit.conversionFactor}</dd>
            </div>
            <div>
              <dt>Reference unit</dt>
              <dd>
                <MathExpression
                  latex={unit.conversionReferenceUnit}
                  plainText={unit.conversionReferenceUnit}
                />
              </dd>
            </div>
          </>
        ) : null}
        {unit.conversionKind === 'contextual' ? (
          <div>
            <dt>Conversion context</dt>
            <dd>
              <MathText text={unit.conversionNote} />
            </dd>
          </div>
        ) : null}
        <div>
          <dt>Source</dt>
          <dd>{unit.source || 'No source recorded'}</dd>
        </div>
        <div>
          <dt>Last updated</dt>
          <dd>{unit.updatedAt}</dd>
        </div>
      </dl>

      <section className="v2-reference-section">
        <h3>Description</h3>
        <p>
          <MathText text={unit.description} />
        </p>
      </section>

      {unit.notes ? (
        <section className="v2-reference-section">
          <h3>Notes</h3>
          <p>
            <MathText text={unit.notes} />
          </p>
        </section>
      ) : null}

      <p className="v2-reference-disclaimer">
        Conversion fields are reference text. This entry does not change official SI definitions,
        and no automatic unit conversion is performed.
      </p>
    </article>
  );
}
