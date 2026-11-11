import { BrandLockup } from '../components/BrandMark';
import type { V2DatabaseState, V2DatabaseStatus } from './api';

type DatabaseStateScreenProps = {
  status: V2DatabaseStatus;
  onRetry: () => void;
  onInitialize: () => void;
  busy: boolean;
  loadError?: string | null;
};

const STATE_COPY: Record<
  Exclude<V2DatabaseState, 'healthy'>,
  { title: string; explanation: string }
> = {
  notInitialized: {
    title: 'Database not initialized',
    explanation:
      'The database is not ready. Initialize it once; this operation is idempotent and does not open legacy data.',
  },
  unsupportedSchema: {
    title: 'Database schema is newer than this application',
    explanation:
      'The database uses a schema this build does not support. It was not modified. Use the matching Physics Tracker version.',
  },
  metadataMismatch: {
    title: 'Database metadata mismatch',
    explanation:
      'The database metadata, migration checksum, identity, or exam records do not match the expected foundation. The database was not treated as ready.',
  },
  integrityFailure: {
    title: 'Database integrity check failed',
    explanation:
      'SQLite reported an integrity problem. Physics Tracker will not present the database as ready until it is repaired or restored.',
  },
  foreignKeyFailure: {
    title: 'Database relationship check failed',
    explanation:
      'The database contains foreign-key violations. The shell will not start normal work until these relationships are repaired.',
  },
  inaccessible: {
    title: 'Database inaccessible',
    explanation:
      'The database could not be opened or is locked. Check the application-data folder and close any duplicate application instance.',
  },
};

export function DatabaseStateScreen({
  status,
  onRetry,
  onInitialize,
  busy,
  loadError,
}: DatabaseStateScreenProps) {
  if (status.state === 'healthy') {
    return null;
  }

  const copy = STATE_COPY[status.state];

  return (
    <main className="v2-database-state">
      <div className="v2-database-state-card" role="alert">
        <BrandLockup subtitle="Scientific study workspace" />
        <p className="eyebrow">Physics Tracker · database</p>
        <h1>{copy.title}</h1>
        <p>{copy.explanation}</p>
        {status.errorMessage && <p className="v2-database-error">{status.errorMessage}</p>}
        {loadError && <p className="v2-database-error">{loadError}</p>}

        <dl className="v2-database-facts">
          <div>
            <dt>Reported state</dt>
            <dd>{status.state}</dd>
          </div>
          <div>
            <dt>Schema</dt>
            <dd>
              {status.schemaVersion ?? 'Not initialized'} / expected {status.expectedSchemaVersion}
            </dd>
          </div>
          <div>
            <dt>Database path</dt>
            <dd>{status.databasePath ?? 'Unavailable'}</dd>
          </div>
        </dl>

        <div className="v2-database-actions">
          {status.state === 'notInitialized' ? (
            <>
              <button
                className="primary-button"
                disabled={busy}
                onClick={onInitialize}
                type="button"
              >
                {busy ? 'Initializing…' : 'Initialize database'}
              </button>
              <button className="secondary-button" disabled={busy} onClick={onRetry} type="button">
                Retry status check
              </button>
            </>
          ) : (
            <button className="primary-button" disabled={busy} onClick={onRetry} type="button">
              {busy ? 'Checking…' : 'Check again'}
            </button>
          )}
        </div>
      </div>
    </main>
  );
}
