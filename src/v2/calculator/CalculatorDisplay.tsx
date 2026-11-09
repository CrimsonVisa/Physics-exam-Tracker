import type { AngleMode } from './types';

type CalculatorDisplayProps = {
  label: string;
  angleMode: AngleMode;
  expression: string;
  previousExpression: string | null;
  resultText: string | null;
  approximate: boolean;
  error: string | null;
  memory: number | null;
  onCopy: () => void;
};

export function CalculatorDisplay({
  label,
  angleMode,
  expression,
  previousExpression,
  resultText,
  approximate,
  error,
  memory,
  onCopy,
}: CalculatorDisplayProps) {
  return (
    <section className="v2-calculator-display" aria-label="Calculator display">
      <div className="v2-calculator-display-header">
        <span className="v2-calculator-display-mode">{label}</span>
        <span className="v2-calculator-display-angle" aria-label="Display angle mode">
          {angleMode === 'deg' ? 'DEG' : 'RAD'}
        </span>
        {memory !== null && (
          <span className="v2-calculator-memory-indicator" aria-label="Memory stored">
            M
          </span>
        )}
      </div>

      <p className="v2-calculator-previous" aria-label="Previous expression">
        {previousExpression ?? ''}
      </p>

      <div
        className="v2-calculator-expression"
        aria-label={`Current expression: ${expression || 'empty'}`}
      >
        {expression || '0'}
      </div>

      <div className="v2-calculator-result-row">
        {error ? (
          <p className="v2-calculator-error" role="alert">
            {error}
          </p>
        ) : (
          <p
            className="v2-calculator-result"
            aria-atomic="true"
            aria-label={
              resultText
                ? `${approximate ? 'Approximately ' : 'Result '}${resultText}`
                : 'No result'
            }
            aria-live="polite"
          >
            {resultText ? (
              <>
                <span className="sr-only">{approximate ? 'Approximately ' : 'Result '}</span>
                {approximate && (
                  <span aria-hidden="true" className="v2-calculator-approx">
                    ≈
                  </span>
                )}
                <span>{resultText}</span>
              </>
            ) : (
              <span className="v2-calculator-result-placeholder">—</span>
            )}
          </p>
        )}
        <button
          className="secondary-button v2-calculator-copy"
          disabled={!resultText || error !== null}
          onClick={onCopy}
          type="button"
        >
          Copy result
        </button>
      </div>
    </section>
  );
}
