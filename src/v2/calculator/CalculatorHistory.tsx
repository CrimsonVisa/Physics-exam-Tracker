import { formatHistoryTime } from './history';
import { CALCULATOR_PROFILES } from './profiles';
import type { HistoryEntry } from './types';

type CalculatorHistoryProps = {
  entries: HistoryEntry[];
  onClear: () => void;
  onCopy: (entry: HistoryEntry) => void;
  onReuse: (entry: HistoryEntry) => void;
};

export function CalculatorHistory({ entries, onClear, onCopy, onReuse }: CalculatorHistoryProps) {
  return (
    <section
      aria-labelledby="calculator-history-heading"
      className="v2-concept-card v2-calculator-history"
    >
      <div className="v2-calculator-section-heading">
        <div>
          <p className="eyebrow">Session</p>
          <h2 id="calculator-history-heading">History</h2>
        </div>
        <button
          className="link-button"
          disabled={entries.length === 0}
          onClick={onClear}
          type="button"
        >
          Clear history
        </button>
      </div>

      {entries.length === 0 ? (
        <p className="v2-empty-panel-copy">No calculations yet.</p>
      ) : (
        <ol className="v2-calculator-history-list">
          {entries.map((entry) => (
            <li className="v2-calculator-history-item" key={entry.id}>
              <div className="v2-calculator-history-body">
                <button
                  aria-label={`Reuse expression ${entry.expression}`}
                  className="v2-calculator-history-expression"
                  onClick={() => onReuse(entry)}
                  type="button"
                >
                  {entry.expression}
                </button>
                <p className="v2-calculator-history-result">{entry.result}</p>
                <p className="v2-calculator-history-meta">
                  {CALCULATOR_PROFILES[entry.mode].shortLabel} ·{' '}
                  {formatHistoryTime(entry.timestamp)}
                </p>
              </div>
              <button
                aria-label={`Copy result ${entry.result}`}
                className="secondary-button v2-calculator-history-copy"
                onClick={() => onCopy(entry)}
                type="button"
              >
                Copy
              </button>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
