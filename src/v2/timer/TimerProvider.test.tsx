import { act, render, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import type { V2TimerSnapshot, V2TimerState } from '../api';
import { onStudySessionsChanged } from '../../lib/studyEvents';
import { emitTimerUpdated } from '../../lib/timerEvents';
import { TimerProvider } from './TimerProvider';
import { useV2Timer } from './timerContext';

vi.mock('../api', () => ({
  getV2Timer: vi.fn(),
  heartbeatV2Timer: vi.fn(),
  startV2Timer: vi.fn(),
  pauseV2Timer: vi.fn(),
  finishV2Timer: vi.fn(),
  discardV2Timer: vi.fn(),
}));

import {
  discardV2Timer,
  finishV2Timer,
  getV2Timer,
  heartbeatV2Timer,
  pauseV2Timer,
  startV2Timer,
} from '../api';

const mockedGetTimer = vi.mocked(getV2Timer);
const mockedHeartbeat = vi.mocked(heartbeatV2Timer);
const mockedStart = vi.mocked(startV2Timer);
const mockedPause = vi.mocked(pauseV2Timer);
const mockedFinish = vi.mocked(finishV2Timer);
const mockedDiscard = vi.mocked(discardV2Timer);

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function snapshot(state: Partial<V2TimerState> = {}): V2TimerSnapshot {
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
    allocations: [],
    examIds: [],
    serverNowMs: Date.parse('2026-11-04T10:00:00Z'),
  };
}

function wrapper({ children }: { children: ReactNode }) {
  return <TimerProvider>{children}</TimerProvider>;
}

async function flushPromises() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

const START_INPUT = {
  mode: 'focus' as const,
  sessionDate: '2026-11-04',
  activeExamId: null,
};

describe('TimerProvider', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-11-04T10:00:00Z'));
    mockedGetTimer.mockResolvedValue(snapshot());
    mockedHeartbeat.mockResolvedValue(snapshot());
    mockedStart.mockResolvedValue(snapshot({ status: 'running' }));
    mockedPause.mockResolvedValue(snapshot({ status: 'paused' }));
    mockedFinish.mockResolvedValue(snapshot({ status: 'idle', lastEvent: 'completed' }));
    mockedDiscard.mockResolvedValue(snapshot({ status: 'idle', lastEvent: 'discarded' }));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('loads the persisted snapshot once with derived timer values', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({ status: 'running', elapsedMs: 60_000, remainingMs: 24 * 60_000 }),
    );

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    expect(mockedGetTimer).toHaveBeenCalledTimes(1);
    expect(result.current.snapshot?.state.status).toBe('running');
    expect(result.current.elapsedMs).toBe(60_000);
    expect(result.current.remainingMs).toBe(24 * 60_000);
    expect(result.current.progress).toBeCloseTo(4);
  });

  it('uses one 1-second clock interval and preserves elapsed/remaining semantics', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({
        status: 'running',
        elapsedMs: 10_000,
        remainingMs: 25 * 60_000 - 10_000,
      }),
    );

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(result.current.elapsedMs).toBe(10_000);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(result.current.elapsedMs).toBe(11_000);
    expect(result.current.remainingMs).toBe(25 * 60_000 - 11_000);
    expect(result.current.progress).toBeCloseTo((11_000 / (25 * 60_000)) * 100);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(result.current.elapsedMs).toBe(12_000);
  });

  it('runs exactly one 5-second heartbeat across multiple consumers', async () => {
    const running = snapshot({
      status: 'running',
      elapsedMs: 1_000,
      remainingMs: 25 * 60_000 - 1_000,
    });
    mockedGetTimer.mockResolvedValue(running);
    mockedHeartbeat.mockResolvedValue(running);

    function Consumer() {
      useV2Timer();
      return null;
    }

    render(
      <TimerProvider>
        <Consumer />
        <Consumer />
      </TimerProvider>,
    );
    await flushPromises();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    expect(mockedHeartbeat).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    expect(mockedHeartbeat).toHaveBeenCalledTimes(2);
  });

  it('does not follow wall-clock jumps during interpolation', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({ status: 'running', elapsedMs: 10_000, remainingMs: 25 * 60_000 - 10_000 }),
    );

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(result.current.elapsedMs).toBe(10_000);

    const dateSpy = vi.spyOn(Date, 'now').mockReturnValue(Date.now() + 3_600_000);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(result.current.elapsedMs).toBe(11_000);
    dateSpy.mockRestore();
  });

  it('uses the latest backend snapshot to override interpolated elapsed time', async () => {
    mockedGetTimer.mockResolvedValueOnce(
      snapshot({ status: 'running', elapsedMs: 10_000, remainingMs: 25 * 60_000 - 10_000 }),
    );
    const heartbeat = deferred<V2TimerSnapshot>();
    mockedHeartbeat.mockReturnValueOnce(heartbeat.promise);

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(result.current.elapsedMs).toBe(11_000);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await act(async () => {
      heartbeat.resolve(
        snapshot({ status: 'running', elapsedMs: 30_000, remainingMs: 25 * 60_000 - 30_000 }),
      );
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.elapsedMs).toBe(30_000);
  });

  it('coalesces visibility and pageshow resume refreshes', async () => {
    mockedGetTimer.mockResolvedValue(snapshot());
    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    Object.defineProperty(document, 'visibilityState', {
      configurable: true,
      value: 'visible',
    });

    await act(async () => {
      document.dispatchEvent(new Event('visibilitychange'));
      window.dispatchEvent(new Event('pageshow'));
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(mockedGetTimer).toHaveBeenCalledTimes(2);
    expect(result.current.error).toBeNull();
  });

  it('does not heartbeat while paused or idle', async () => {
    mockedGetTimer.mockResolvedValue(snapshot({ status: 'paused', elapsedMs: 60_000 }));

    const paused = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15_000);
    });
    expect(mockedHeartbeat).not.toHaveBeenCalled();
    expect(paused.result.current.elapsedMs).toBe(60_000);
    paused.unmount();

    mockedGetTimer.mockResolvedValue(snapshot());
    renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(15_000);
    });
    expect(mockedHeartbeat).not.toHaveBeenCalled();
  });

  it('refreshes on the existing TIMER_UPDATED_EVENT', async () => {
    renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    await act(async () => {
      emitTimerUpdated();
    });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(2);
  });

  it('applies heartbeat completion to the shared snapshot', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({ status: 'running', elapsedMs: 25 * 60_000, remainingMs: 0 }),
    );
    mockedHeartbeat.mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastEventDurationMs: 25 * 60_000,
        lastCompletedSessionId: 9,
      }),
    );

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });

    expect(mockedHeartbeat).toHaveBeenCalledTimes(1);
    expect(result.current.snapshot?.state.status).toBe('idle');
    expect(result.current.snapshot?.state.lastEvent).toBe('completed');
  });

  it('announces a newly completed session for dashboard refresh', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({ status: 'running', elapsedMs: 25 * 60_000, remainingMs: 0 }),
    );
    mockedHeartbeat.mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastEventDurationMs: 25 * 60_000,
        lastCompletedSessionId: 25,
      }),
    );
    const listener = vi.fn();
    const unsubscribe = onStudySessionsChanged(listener);

    try {
      renderHook(() => useV2Timer(), { wrapper });
      await flushPromises();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });

      expect(listener).toHaveBeenCalledTimes(1);
    } finally {
      unsubscribe();
    }
  });

  it('does not re-announce the same completed session after renders or refresh events', async () => {
    mockedGetTimer.mockResolvedValueOnce(snapshot({ status: 'running' })).mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastCompletedSessionId: 25,
      }),
    );
    mockedHeartbeat.mockResolvedValue(
      snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastCompletedSessionId: 25,
      }),
    );
    const listener = vi.fn();
    const unsubscribe = onStudySessionsChanged(listener);

    try {
      const { result, rerender } = renderHook(() => useV2Timer(), { wrapper });
      await flushPromises();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(listener).toHaveBeenCalledTimes(1);

      rerender();
      await act(async () => {
        emitTimerUpdated();
      });
      await flushPromises();

      expect(listener).toHaveBeenCalledTimes(1);
      expect(result.current.snapshot?.state.lastCompletedSessionId).toBe(25);
    } finally {
      unsubscribe();
    }
  });

  it('cleans up the clock interval, heartbeat and event listener on unmount', async () => {
    mockedGetTimer.mockResolvedValue(
      snapshot({ status: 'running', elapsedMs: 1_000, remainingMs: 25 * 60_000 - 1_000 }),
    );

    const { unmount } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    unmount();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
      emitTimerUpdated();
    });
    await flushPromises();

    expect(mockedHeartbeat).not.toHaveBeenCalled();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);
  });

  it('surfaces a snapshot load error', async () => {
    mockedGetTimer.mockRejectedValue(new Error('timer unavailable'));

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    expect(result.current.error).toBeTruthy();
    expect(result.current.loading).toBe(false);
  });

  it('clears a synchronization error after a successful refresh', async () => {
    mockedGetTimer
      .mockRejectedValueOnce(new Error('temporary failure'))
      .mockResolvedValueOnce(snapshot({ status: 'running' }));

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(result.current.error).toBeTruthy();

    await act(async () => {
      emitTimerUpdated();
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(result.current.error).toBeNull();
    expect(result.current.snapshot?.state.status).toBe('running');
  });

  it('serializes an initial read before a requested start and ignores the stale read', async () => {
    const initial = deferred<V2TimerSnapshot>();
    const start = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockReturnValueOnce(initial.promise);
    mockedStart.mockReturnValueOnce(start.promise);

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    let mutation!: Promise<V2TimerSnapshot>;
    await act(async () => {
      mutation = result.current.runMutation(() => startV2Timer(START_INPUT));
    });
    expect(mockedStart).not.toHaveBeenCalled();

    await act(async () => {
      initial.resolve(snapshot());
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedStart).toHaveBeenCalledTimes(1);

    await act(async () => {
      start.resolve(snapshot({ status: 'running' }));
      await mutation;
    });

    expect(result.current.snapshot?.state.status).toBe('running');
  });

  it('queues a pause behind an event refresh and ignores the stale refresh snapshot', async () => {
    const running = snapshot({ status: 'running' });
    const refresh = deferred<V2TimerSnapshot>();
    const pause = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockResolvedValueOnce(running).mockReturnValueOnce(refresh.promise);
    mockedPause.mockReturnValueOnce(pause.promise);

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    await act(async () => {
      emitTimerUpdated();
      await vi.advanceTimersByTimeAsync(0);
    });

    let mutation!: Promise<V2TimerSnapshot>;
    await act(async () => {
      mutation = result.current.runMutation(pauseV2Timer);
    });
    expect(mockedPause).not.toHaveBeenCalled();

    await act(async () => {
      refresh.resolve(running);
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedPause).toHaveBeenCalledTimes(1);

    await act(async () => {
      pause.resolve(snapshot({ status: 'paused' }));
      await mutation;
    });

    expect(result.current.snapshot?.state.status).toBe('paused');
  });

  it.each(['pause', 'finish', 'discard'] as const)(
    'never invokes %s concurrently with an executing heartbeat',
    async (kind) => {
      const heartbeat = deferred<V2TimerSnapshot>();
      const action = deferred<V2TimerSnapshot>();
      const actionMock =
        kind === 'pause' ? mockedPause : kind === 'finish' ? mockedFinish : mockedDiscard;
      const actionOperation =
        kind === 'pause' ? pauseV2Timer : kind === 'finish' ? finishV2Timer : discardV2Timer;
      const finalSnapshot =
        kind === 'pause'
          ? snapshot({ status: 'paused' })
          : snapshot({ status: 'idle', lastEvent: kind === 'finish' ? 'completed' : 'discarded' });
      mockedGetTimer.mockResolvedValue(snapshot({ status: 'running' }));
      mockedHeartbeat.mockReturnValueOnce(heartbeat.promise);
      actionMock.mockReturnValueOnce(action.promise);

      const { result } = renderHook(() => useV2Timer(), { wrapper });
      await flushPromises();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });
      expect(mockedHeartbeat).toHaveBeenCalledTimes(1);

      let mutation!: Promise<V2TimerSnapshot>;
      await act(async () => {
        mutation = result.current.runMutation(actionOperation);
      });
      expect(actionMock).not.toHaveBeenCalled();

      await act(async () => {
        heartbeat.resolve(snapshot({ status: 'running' }));
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(actionMock).toHaveBeenCalledTimes(1);

      await act(async () => {
        action.resolve(finalSnapshot);
        await mutation;
      });
      expect(result.current.snapshot?.state.status).toBe(finalSnapshot.state.status);
    },
  );

  it.each(['pause', 'finish', 'discard'] as const)(
    'preserves completion when heartbeat completes before queued %s',
    async (kind) => {
      const heartbeat = deferred<V2TimerSnapshot>();
      const action = deferred<V2TimerSnapshot>();
      const actionMock =
        kind === 'pause' ? mockedPause : kind === 'finish' ? mockedFinish : mockedDiscard;
      const actionOperation =
        kind === 'pause' ? pauseV2Timer : kind === 'finish' ? finishV2Timer : discardV2Timer;
      const completed = snapshot({
        status: 'idle',
        lastEvent: 'completed',
        lastCompletedSessionId: 7,
      });
      mockedGetTimer.mockResolvedValue(snapshot({ status: 'running' }));
      mockedHeartbeat.mockReturnValueOnce(heartbeat.promise);
      actionMock.mockReturnValueOnce(action.promise);

      const { result } = renderHook(() => useV2Timer(), { wrapper });
      await flushPromises();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(5_000);
      });

      let mutation!: Promise<V2TimerSnapshot>;
      await act(async () => {
        mutation = result.current.runMutation(actionOperation);
      });

      await act(async () => {
        heartbeat.resolve(completed);
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(actionMock).toHaveBeenCalledTimes(1);

      await act(async () => {
        action.resolve(completed);
        await mutation;
      });

      expect(result.current.snapshot?.state.status).toBe('idle');
      expect(result.current.snapshot?.state.lastEvent).toBe('completed');
      expect(result.current.snapshot?.state.lastCompletedSessionId).toBe(7);
    },
  );

  it('does not start another heartbeat while one is unresolved', async () => {
    const heartbeat = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockResolvedValue(snapshot({ status: 'running' }));
    mockedHeartbeat
      .mockReturnValueOnce(heartbeat.promise)
      .mockResolvedValue(snapshot({ status: 'running' }));

    renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(mockedHeartbeat).toHaveBeenCalledTimes(1);

    await act(async () => {
      heartbeat.resolve(snapshot({ status: 'running' }));
      await vi.advanceTimersByTimeAsync(0);
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    expect(mockedHeartbeat).toHaveBeenCalledTimes(2);
  });

  it('coalesces event refreshes that arrive during an active background read', async () => {
    const initial = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockReturnValueOnce(initial.promise);

    renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    await act(async () => {
      emitTimerUpdated();
      emitTimerUpdated();
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedGetTimer).toHaveBeenCalledTimes(1);

    await act(async () => {
      initial.resolve(snapshot());
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedGetTimer).toHaveBeenCalledTimes(2);
  });

  it('executes two requested mutations in FIFO order and applies the latest snapshot', async () => {
    const pause = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockResolvedValue(snapshot());
    mockedPause.mockReturnValueOnce(pause.promise);
    mockedStart.mockResolvedValueOnce(snapshot({ status: 'running' }));

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    let first!: Promise<V2TimerSnapshot>;
    let second!: Promise<V2TimerSnapshot>;
    await act(async () => {
      first = result.current.runMutation(pauseV2Timer);
      second = result.current.runMutation(() => startV2Timer(START_INPUT));
    });

    expect(mockedPause).toHaveBeenCalledTimes(1);
    expect(mockedStart).not.toHaveBeenCalled();

    await act(async () => {
      pause.resolve(snapshot({ status: 'paused' }));
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(mockedStart).toHaveBeenCalledTimes(1);

    await act(async () => {
      await Promise.all([first, second]);
    });
    expect(result.current.snapshot?.state.status).toBe('running');
  });

  it('reconciles after a failed mutation while preserving the original error', async () => {
    const actionError = new Error('response lost after commit');
    mockedGetTimer.mockResolvedValue(snapshot());
    mockedStart.mockRejectedValueOnce(actionError);

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();

    const action = result.current.runMutation(() => startV2Timer(START_INPUT));
    await act(async () => {
      await expect(action).rejects.toBe(actionError);
    });
    await flushPromises();

    expect(mockedGetTimer).toHaveBeenCalledTimes(2);

    mockedPause.mockResolvedValueOnce(snapshot({ status: 'paused' }));
    let next!: Promise<V2TimerSnapshot>;
    await act(async () => {
      next = result.current.runMutation(pauseV2Timer);
      await next;
    });
    expect(result.current.snapshot?.state.status).toBe('paused');
  });

  it('does not let reconciliation overwrite a newer mutation', async () => {
    const reconciliation = deferred<V2TimerSnapshot>();
    mockedGetTimer.mockResolvedValueOnce(snapshot());
    mockedStart.mockRejectedValueOnce(new Error('lost response'));
    mockedPause.mockResolvedValueOnce(snapshot({ status: 'paused' }));

    const { result } = renderHook(() => useV2Timer(), { wrapper });
    await flushPromises();
    mockedGetTimer.mockReturnValueOnce(reconciliation.promise);

    const failed = result.current.runMutation(() => startV2Timer(START_INPUT));
    await act(async () => {
      await expect(failed).rejects.toThrow('lost response');
    });

    let newer!: Promise<V2TimerSnapshot>;
    await act(async () => {
      newer = result.current.runMutation(pauseV2Timer);
    });
    expect(mockedPause).not.toHaveBeenCalled();

    await act(async () => {
      reconciliation.resolve(snapshot({ status: 'running' }));
      await vi.advanceTimersByTimeAsync(0);
      await newer;
    });

    expect(result.current.snapshot?.state.status).toBe('paused');
  });
});
