import { useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { getStudyTotals, type StudyTotals } from '../lib/sessionsApi';
import { formatStudyDuration } from '../lib/studyTime';

type StudyTodayCardProps = {
  today: string;
  targetMinutes: number;
  onOpenSessions: () => void;
};

export function StudyTodayCard({ today, targetMinutes, onOpenSessions }: StudyTodayCardProps) {
  const [totals, setTotals] = useState<StudyTotals | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;

    getStudyTotals(today, today)
      .then((loaded) => {
        if (!cancelled) {
          setTotals(loaded);
          setLoadError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [today, reloadKey]);

  const targetSeconds = targetMinutes * 60;
  const percentage = totals
    ? Math.min(100, Math.max(0, (totals.todaySeconds / Math.max(1, targetSeconds)) * 100))
    : 0;

  return (
    <section className="study-today-card" aria-labelledby="study-today-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">Daily target</p>
          <h2 id="study-today-heading">Today&apos;s study time</h2>
        </div>
        <button type="button" className="secondary-button" onClick={onOpenSessions}>
          View sessions
        </button>
      </div>

      {isLoading && (
        <p className="study-today-state" role="status">
          Loading study-time totals…
        </p>
      )}

      {!isLoading && loadError && (
        <div className="study-today-error" role="alert">
          <p>{loadError}</p>
          <button
            type="button"
            className="secondary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              setReloadKey((key) => key + 1);
            }}
          >
            Retry totals
          </button>
        </div>
      )}

      {!isLoading && !loadError && totals && (
        <>
          <div className="study-today-body">
            <p className="study-today-value">{formatStudyDuration(totals.todaySeconds)}</p>
            <p className="study-today-target">
              of {formatStudyDuration(targetSeconds)} daily target
            </p>
          </div>

          <div
            className="progress-track study-today-progress"
            role="progressbar"
            aria-label="Today's focus time toward the daily target"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(percentage)}
          >
            <span style={{ width: `${percentage}%` }} />
          </div>

          <dl className="study-today-meta">
            <div>
              <dt>Sessions today</dt>
              <dd>{totals.rangeSessionCount}</dd>
            </div>
            <div>
              <dt>This week</dt>
              <dd>{formatStudyDuration(totals.weekSeconds)}</dd>
            </div>
          </dl>

          <p className="study-today-note">
            Focus sessions only. Paused time and break time are excluded, and manual sessions are
            included after you save them.
          </p>
        </>
      )}
    </section>
  );
}
