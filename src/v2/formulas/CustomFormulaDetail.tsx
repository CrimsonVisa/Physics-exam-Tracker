import { useEffect, useRef } from 'react';
import type { V2CustomFormula } from '../api';
import { MathText } from '../MathText';
import { MathExpression } from '../math/MathExpression';
import { FormulaEquation } from './FormulaEquation';

type CustomFormulaDetailProps = {
  formula: V2CustomFormula;
  onBack: () => void;
  onDelete: () => void;
  onEdit: () => void;
};

export function CustomFormulaDetail({
  formula,
  onBack,
  onDelete,
  onEdit,
}: CustomFormulaDetailProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <article className="v2-reference-detail" data-formula-id={formula.id}>
      <div className="v2-reference-detail-header">
        <button className="link-button" onClick={onBack} type="button">
          ← Back to my formulas
        </button>
        <div className="v2-reference-detail-actions">
          {formula.isBuiltin ? null : (
            <>
              <button className="secondary-button" onClick={onEdit} type="button">
                Edit formula
              </button>
              <button
                className="secondary-button v2-danger-button"
                onClick={onDelete}
                type="button"
              >
                Delete formula
              </button>
            </>
          )}
        </div>
      </div>

      <header className="v2-reference-detail-title">
        <div>
          <p className="eyebrow">{formula.category}</p>
          <h2 ref={headingRef} tabIndex={-1}>
            {formula.name}
          </h2>
        </div>
        <div className="v2-reference-badges">
          <span className="v2-reference-badge" data-tone="user">
            {formula.isBuiltin ? 'Built-in' : 'User-defined'}
          </span>
          <span className="v2-reference-badge" data-tone="category">
            {formula.category}
          </span>
        </div>
      </header>

      <div className="v2-reference-equation-panel">
        <FormulaEquation latex={formula.latex} plainText={formula.plainText} />
      </div>

      <dl className="v2-reference-definition">
        <div>
          <dt>Plain-text expression</dt>
          <dd className="v2-reference-verbatim">{formula.plainText}</dd>
        </div>
        <div>
          <dt>Category</dt>
          <dd>{formula.category}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{formula.source || 'No source recorded'}</dd>
        </div>
        <div>
          <dt>Last updated</dt>
          <dd>{formula.updatedAt}</dd>
        </div>
      </dl>

      <section className="v2-reference-section">
        <h3>Description</h3>
        <p>
          <MathText text={formula.description} />
        </p>
      </section>

      <section className="v2-reference-section">
        <h3>Variables</h3>
        {formula.variables.length === 0 ? (
          <p className="v2-empty-copy">No variables were recorded for this formula.</p>
        ) : (
          <div className="v2-reference-table-scroll">
            <table className="v2-reference-table">
              <thead>
                <tr>
                  <th scope="col">Symbol</th>
                  <th scope="col">Meaning</th>
                  <th scope="col">Unit</th>
                </tr>
              </thead>
              <tbody>
                {formula.variables.map((variable) => (
                  <tr key={variable.id}>
                    <td>
                      <MathExpression latex={variable.symbol} plainText={variable.symbol} />
                    </td>
                    <td>
                      <MathText text={variable.meaning} />
                    </td>
                    <td>
                      {variable.unit ? (
                        <MathExpression latex={variable.unit} plainText={variable.unit} />
                      ) : (
                        '—'
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {formula.notes ? (
        <section className="v2-reference-section">
          <h3>Notes</h3>
          <p>
            <MathText text={formula.notes} />
          </p>
        </section>
      ) : null}

      <p className="v2-reference-disclaimer">
        This entry is reference material only. Expressions are not evaluated or parsed by the
        application.
      </p>
    </article>
  );
}
