import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { V2TimerSnapshot, V2TimerState } from '../api';
import { pauseV2Timer, resumeV2Timer } from '../api';
import { GlobalTimerIndicator } from './GlobalTimerIndicator';
import { TimerContext, type TimerContextValue } from './timerContext';

function snapshot(
  state: Partial<V2TimerState> = {},
  allocations: V2TimerSnapshot['allocations'] = [],
): V2TimerSnapshot {
  return {
    state: {
      status: 'idle',
      mode: 'focus',
      durationMs: 25 * 60_000,
      accumulatedMs: 0,
      elapsedMs: 0,
      remainingMs: 25 * 60_000,
      segmentStartedAt: null,
      sessionDate: null,
      activeExamId: null,
      recoveryReason: null,
      recoveryExcludedMs: 0,
      clockSkewDetected: false,
      lastEvent: null,
      lastEventAt: null,
      lastEventDurationMs: 0,
      lastCompletedSessionId: null,
      ...state,
    },
    settings: {
      focusMinutes: 25,
      shortBreakMinutes: 5,
      longBreakMinutes: 15,
      notificationsEnabled: false,
    },
    allocations,
    serverNowMs: Date.parse('2026-11-04T10:00:00Z'),
  };
}

function contextValue(
  timer: V2TimerSnapshot | null,
  error: string | null = null,
  runMutation: TimerContextValue['runMutation'] = vi.fn(),
): TimerContextValue {
  const elapsedMs = timer?.state.elapsedMs ?? 0;
  const remainingMs = timer ? Math.max(0, timer.state.durationMs - elapsedMs) : 0;
  return {
    snapshot: timer,
    loading: false,
    error,
    elapsedMs,
    remainingMs,
    progress: timer && timer.state.durationMs > 0 ? (elapsedMs / timer.state.durationMs) * 100 : 0,
    runMutation,
    refresh: vi.fn().mockResolvedValue(undefined),
  };
}

function renderIndicator(
  timer: V2TimerSnapshot | null,
  onOpenTimer = vi.fn(),
  error: string | null = null,
  runMutation: TimerContextValue['runMutation'] = vi.fn(),
) {
  render(
    <TimerContext.Provider value={contextValue(timer, error, runMutation)}>
      <GlobalTimerIndicator onOpenTimer={onOpenTimer} />
    </TimerContext.Provider>,
  );
  return { onOpenTimer, runMutation };
}

describe('GlobalTimerIndicator', () => {
  it('shows a completion-pending state at zero before backend confirmation', () => {
    renderIndicator(
      snapshot({
        status: 'running',
        durationMs: 25 * 60_000,
        elapsedMs: 25 * 60_000,
        remainingMs: 0,
      }),
    );

    const button = screen.getByRole('button', {
      name: /Focus timer completing, 00:00 remaining, 25:00 elapsed\. Awaiting backend completion/,
    });
    expect(button).toHaveAccessibleName(/timer completing/);
    expect(button).not.toHaveAccessibleName(/timer running/);
    expect(button).toHaveTextContent('Completing…');
    expect(button).toHaveTextContent('00:00');
    expect(button).toHaveAttribute('data-completing', 'true');
  });

  it('keeps a paused timer at zero labelled Paused', () => {
    renderIndicator(
      snapshot({
        status: 'paused',
        durationMs: 25 * 60_000,
        elapsedMs: 25 * 60_000,
        remainingMs: 0,
      }),
    );

    const button = screen.getByRole('button', { name: /Focus timer paused/ });
    expect(button).toHaveTextContent('Paused');
    expect(button).not.toHaveTextContent('Completing');
  });

  it('renders nothing for idle or completed timers', () => {
    const idle = renderIndicator(snapshot());
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    idle.onOpenTimer.mockClear();

    renderIndicator(snapshot({ status: 'idle', lastEvent: 'completed' }));
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('renders the running focus timer with remaining and elapsed semantics', () => {
    renderIndicator(
      snapshot({
        status: 'running',
        durationMs: 25 * 60_000,
        elapsedMs: 60_000,
        remainingMs: 24 * 60_000,
      }),
    );

    const button = screen.getByRole('button', {
      name: /Focus timer running, 24:00 remaining, 01:00 elapsed/,
    });
    expect(button).toHaveAttribute('data-status', 'running');
    expect(button).toHaveTextContent('Focus');
    expect(button).toHaveTextContent('24:00');
    expect(button).toHaveTextContent('Running');
  });

  it('keeps a paused timer visible with frozen time and paused state', () => {
    renderIndicator(
      snapshot({
        status: 'paused',
        durationMs: 25 * 60_000,
        elapsedMs: 90_000,
        remainingMs: 23 * 60_000 + 30_000,
      }),
    );

    const button = screen.getByRole('button', {
      name: /Focus timer paused, 23:30 remaining, 01:30 elapsed/,
    });
    expect(button).toHaveAttribute('data-status', 'paused');
    expect(button).toHaveTextContent('Paused');
    expect(button).toHaveTextContent('23:30');
  });

  it('uses the correct break labels', () => {
    renderIndicator(snapshot({ status: 'running', mode: 'shortBreak' }));
    expect(screen.getByRole('button', { name: /Short break timer running/ })).toBeInTheDocument();
  });

  it('passes the active exam to the timer navigation callback', async () => {
    const user = userEvent.setup();
    const { onOpenTimer } = renderIndicator(
      snapshot(
        {
          status: 'running',
          activeExamId: 2,
        },
        [
          {
            examId: 3,
            examCode: 'tifr-gs-physics',
            examName: 'TIFR Graduate School Physics',
            nodeId: 10,
            nodeTitle: 'Topic',
            subjectTitle: 'Subject',
            allocatedSeconds: 600,
          },
        ],
      ),
    );

    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));
    expect(onOpenTimer).toHaveBeenCalledWith([2, 3]);
  });

  it('falls back to the first allocation exam when no active exam is set', async () => {
    const user = userEvent.setup();
    const { onOpenTimer } = renderIndicator(
      snapshot(
        {
          status: 'running',
          activeExamId: null,
        },
        [
          {
            examId: 3,
            examCode: 'tifr-gs-physics',
            examName: 'TIFR Graduate School Physics',
            nodeId: 10,
            nodeTitle: 'Topic',
            subjectTitle: 'Subject',
            allocatedSeconds: 600,
          },
        ],
      ),
    );

    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));
    expect(onOpenTimer).toHaveBeenCalledWith([3]);
  });

  it('tries allocations when the active exam id is stale', async () => {
    const user = userEvent.setup();
    const { onOpenTimer } = renderIndicator(
      snapshot(
        {
          status: 'running',
          activeExamId: 99,
        },
        [
          {
            examId: 3,
            examCode: 'tifr-gs-physics',
            examName: 'TIFR Graduate School Physics',
            nodeId: 10,
            nodeTitle: 'Topic',
            subjectTitle: 'Subject',
            allocatedSeconds: 600,
          },
        ],
      ),
    );

    await user.click(screen.getByRole('button', { name: /Focus timer running/ }));
    expect(onOpenTimer).toHaveBeenCalledWith([99, 3]);
  });

  it('shows a synchronization warning while preserving the last known clock', () => {
    renderIndicator(
      snapshot({
        status: 'running',
        elapsedMs: 60_000,
        remainingMs: 24 * 60_000,
      }),
      undefined,
      'The timer refresh failed.',
    );

    const button = screen.getByRole('button', {
      name: /Focus timer running, 24:00 remaining, 01:00 elapsed\. Synchronization failed; showing the last known timer state/,
    });
    expect(button).toHaveAttribute('data-sync', 'error');
    expect(button).toHaveTextContent('24:00');
    expect(button).toHaveTextContent('Running · sync issue');
  });

  it('pauses a running timer through the provider mutation queue', async () => {
    const user = userEvent.setup();
    const runMutation = vi.fn().mockResolvedValue(snapshot({ status: 'paused' }));
    renderIndicator(
      snapshot({
        status: 'running',
        elapsedMs: 60_000,
        remainingMs: 24 * 60_000,
      }),
      undefined,
      null,
      runMutation,
    );

    await user.click(screen.getByRole('button', { name: 'Pause study timer' }));

    expect(runMutation).toHaveBeenCalledTimes(1);
    expect(runMutation).toHaveBeenCalledWith(pauseV2Timer);
    expect(screen.getByRole('button', { name: /Focus timer running/ })).toHaveTextContent('01:00');
  });

  it('resumes a paused timer through the provider mutation queue', async () => {
    const user = userEvent.setup();
    const runMutation = vi.fn().mockResolvedValue(snapshot({ status: 'running' }));
    renderIndicator(
      snapshot({ status: 'paused', elapsedMs: 90_000 }),
      undefined,
      null,
      runMutation,
    );

    await user.click(screen.getByRole('button', { name: 'Resume study timer' }));

    expect(runMutation).toHaveBeenCalledTimes(1);
    expect(runMutation).toHaveBeenCalledWith(resumeV2Timer);
  });

  it('disables timer mutations while completing or synchronizing', () => {
    renderIndicator(
      snapshot({
        status: 'running',
        elapsedMs: 25 * 60_000,
        remainingMs: 0,
      }),
    );

    expect(screen.getByRole('button', { name: 'Pause study timer' })).toBeDisabled();
  });

  it('disables timer mutations while the last state has a sync error', () => {
    renderIndicator(
      snapshot({ status: 'running', elapsedMs: 60_000 }),
      undefined,
      'The timer refresh failed.',
    );

    expect(screen.getByRole('button', { name: 'Pause study timer' })).toBeDisabled();
  });

  it('reports a failed pause without hiding the timer', async () => {
    const user = userEvent.setup();
    const runMutation = vi
      .fn()
      .mockRejectedValue({ code: 'timer_error', message: 'The timer could not be paused.' });
    renderIndicator(snapshot({ status: 'running' }), undefined, null, runMutation);

    await user.click(screen.getByRole('button', { name: 'Pause study timer' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('The timer could not be paused.');
    expect(screen.getByRole('button', { name: /Focus timer running/ })).toBeInTheDocument();
  });
});
