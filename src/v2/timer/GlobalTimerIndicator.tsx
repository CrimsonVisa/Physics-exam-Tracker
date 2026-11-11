import { useState } from 'react';
import { toCommandError } from '../../lib/commandError';
import { pauseV2Timer, resumeV2Timer } from '../api';
import { formatTimerClock, TIMER_MODE_LABELS } from './format';
import { useV2Timer } from './timerContext';

type GlobalTimerIndicatorProps = {
  onOpenTimer: (examIds: number[]) => void;
};

export function GlobalTimerIndicator({ onOpenTimer }: GlobalTimerIndicatorProps) {
  const { snapshot, elapsedMs, remainingMs, error, runMutation } = useV2Timer();
  const [pending, setPending] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  if (!snapshot || snapshot.state.status === 'idle') {
    return null;
  }

  const paused = snapshot.state.status === 'paused';
  const completing = snapshot.state.status === 'running' && remainingMs <= 0;
  const modeLabel = TIMER_MODE_LABELS[snapshot.state.mode];
  const syncError = error !== null;
  const baseStateLabel = completing ? 'Completing…' : paused ? 'Paused' : 'Running';
  const stateLabel = syncError ? `${baseStateLabel} · sync issue` : baseStateLabel;
  const remainingText = formatTimerClock(remainingMs);
  const elapsedText = formatTimerClock(elapsedMs);
  const examIds = [
    ...new Set(
      [
        snapshot.state.activeExamId,
        ...snapshot.allocations.map((allocation) => allocation.examId),
      ].filter((id): id is number => typeof id === 'number'),
    ),
  ];
  const completionText = completing ? ' Awaiting backend completion.' : '';
  const timerStateWord = completing ? 'completing' : paused ? 'paused' : 'running';
  const accessibleLabel = `${modeLabel} timer ${timerStateWord}, ${remainingText} remaining, ${elapsedText} elapsed.${completionText}${
    syncError ? ' Synchronization failed; showing the last known timer state.' : ''
  } Open study timer.`;
  const toggleDisabled = pending || completing || syncError;

  async function toggleTimer() {
    if (toggleDisabled) {
      return;
    }
    setPending(true);
    setActionError(null);
    try {
      await runMutation(paused ? resumeV2Timer : pauseV2Timer);
    } catch (mutationError) {
      setActionError(toCommandError(mutationError).message);
    } finally {
      setPending(false);
    }
  }

  return (
    <section aria-label="Active study timer" className="v2-global-timer-dock">
      <span aria-live="polite" className="sr-only">
        {stateLabel}
      </span>
      <button
        aria-label={accessibleLabel}
        className="v2-global-timer"
        data-completing={completing ? 'true' : undefined}
        data-status={snapshot.state.status}
        data-sync={syncError ? 'error' : undefined}
        onClick={() => onOpenTimer(examIds)}
        title={accessibleLabel}
        type="button"
      >
        <span aria-hidden="true" className="v2-global-timer-dot" />
        <span className="v2-global-timer-mode">{modeLabel}</span>
        <span className="v2-global-timer-clock" role="timer">
          {remainingText}
        </span>
        <span className="v2-global-timer-elapsed">{elapsedText}</span>
        <span className="v2-global-timer-state">{stateLabel}</span>
      </button>
      <button
        aria-label={paused ? 'Resume study timer' : 'Pause study timer'}
        className="v2-global-timer-action"
        disabled={toggleDisabled}
        onClick={() => void toggleTimer()}
        type="button"
      >
        {pending ? 'Updating…' : paused ? 'Resume' : 'Pause'}
      </button>
      {actionError && (
        <p className="v2-global-timer-action-error" role="alert">
          {actionError}
        </p>
      )}
    </section>
  );
}
