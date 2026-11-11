import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { toCommandError } from '../../lib/commandError';
import { onTimerUpdated } from '../../lib/timerEvents';
import { getV2Timer, heartbeatV2Timer, type V2TimerSnapshot } from '../api';
import { TimerContext, type TimerContextValue } from './timerContext';

type TimerProviderProps = {
  children: ReactNode;
};

function monotonicNow(): number {
  return typeof performance !== 'undefined' ? performance.now() : Date.now();
}

export function TimerProvider({ children }: TimerProviderProps) {
  const [snapshot, setSnapshot] = useState<V2TimerSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [receivedAt, setReceivedAt] = useState(() => monotonicNow());
  const [clock, setClock] = useState(() => monotonicNow());
  const mountedRef = useRef(true);
  const queueRef = useRef<Promise<void>>(Promise.resolve());
  const mutationSequenceRef = useRef(0);
  const reconciliationScheduledRef = useRef(false);
  const heartbeatPendingRef = useRef(false);
  const backgroundInFlightRef = useRef(0);
  const eventRefreshPendingRef = useRef(false);
  const eventRefreshDrainingRef = useRef(false);
  const drainEventRefreshesRef = useRef<() => void>(() => undefined);
  const lastResumeRefreshRef = useRef<number | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const applySnapshot = useCallback((next: V2TimerSnapshot) => {
    setSnapshot(next);
    setReceivedAt(monotonicNow());
    setClock(monotonicNow());
    setError(null);
  }, []);

  const enqueueOperation = useCallback(function enqueue<T>(
    operation: () => Promise<T>,
  ): Promise<T> {
    const queued = queueRef.current.then(operation);
    queueRef.current = queued.then(
      () => undefined,
      () => undefined,
    );
    return queued;
  }, []);

  const fetchSnapshot = useCallback(
    async (showLoading = false) => {
      const generation = mutationSequenceRef.current;
      backgroundInFlightRef.current += 1;
      if (showLoading) {
        setLoading(true);
      }
      try {
        const next = await enqueueOperation(getV2Timer);
        if (mountedRef.current && generation === mutationSequenceRef.current) {
          applySnapshot(next);
        }
      } catch (loadError) {
        if (mountedRef.current && generation === mutationSequenceRef.current) {
          setError(toCommandError(loadError).message);
        }
      } finally {
        backgroundInFlightRef.current -= 1;
        if (mountedRef.current) {
          setLoading(false);
          drainEventRefreshesRef.current();
        }
      }
    },
    [applySnapshot, enqueueOperation],
  );

  const scheduleReconciliation = useCallback(() => {
    if (reconciliationScheduledRef.current) {
      return;
    }
    reconciliationScheduledRef.current = true;
    void fetchSnapshot().finally(() => {
      reconciliationScheduledRef.current = false;
    });
  }, [fetchSnapshot]);

  const runMutation = useCallback(
    async (operation: () => Promise<V2TimerSnapshot>) => {
      const generation = ++mutationSequenceRef.current;
      try {
        const next = await enqueueOperation(operation);
        if (mountedRef.current && generation === mutationSequenceRef.current) {
          applySnapshot(next);
        }
        return next;
      } catch (actionError) {
        if (mountedRef.current && generation === mutationSequenceRef.current) {
          scheduleReconciliation();
        }
        throw actionError;
      }
    },
    [applySnapshot, enqueueOperation, scheduleReconciliation],
  );

  const refresh = useCallback((showLoading = false) => fetchSnapshot(showLoading), [fetchSnapshot]);

  const runHeartbeat = useCallback(async () => {
    if (heartbeatPendingRef.current || backgroundInFlightRef.current > 0) {
      return;
    }
    heartbeatPendingRef.current = true;
    backgroundInFlightRef.current += 1;
    const generation = mutationSequenceRef.current;
    try {
      const next = await enqueueOperation(heartbeatV2Timer);
      if (mountedRef.current && generation === mutationSequenceRef.current) {
        applySnapshot(next);
      }
    } catch (heartbeatError) {
      if (mountedRef.current && generation === mutationSequenceRef.current) {
        setError(toCommandError(heartbeatError).message);
      }
    } finally {
      backgroundInFlightRef.current -= 1;
      heartbeatPendingRef.current = false;
      if (mountedRef.current) {
        drainEventRefreshesRef.current();
      }
    }
  }, [applySnapshot, enqueueOperation]);

  useEffect(() => {
    let cancelled = false;
    backgroundInFlightRef.current += 1;
    void (async () => {
      const generation = mutationSequenceRef.current;
      try {
        const next = await enqueueOperation(getV2Timer);
        if (!cancelled && generation === mutationSequenceRef.current) {
          applySnapshot(next);
        }
      } catch (loadError) {
        if (!cancelled && generation === mutationSequenceRef.current) {
          setError(toCommandError(loadError).message);
        }
      } finally {
        backgroundInFlightRef.current -= 1;
        if (!cancelled) {
          setLoading(false);
          drainEventRefreshesRef.current();
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [applySnapshot, enqueueOperation]);

  useEffect(() => {
    const interval = window.setInterval(() => setClock(monotonicNow()), 1_000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    if (snapshot?.state.status !== 'running') {
      return;
    }
    const interval = window.setInterval(() => {
      void runHeartbeat();
    }, 5_000);
    return () => window.clearInterval(interval);
  }, [runHeartbeat, snapshot?.state.status]);

  const drainEventRefreshes = useCallback(() => {
    if (eventRefreshDrainingRef.current || backgroundInFlightRef.current > 0) {
      return;
    }
    eventRefreshDrainingRef.current = true;
    void (async () => {
      try {
        while (eventRefreshPendingRef.current) {
          eventRefreshPendingRef.current = false;
          await fetchSnapshot();
        }
      } finally {
        eventRefreshDrainingRef.current = false;
      }
    })();
  }, [fetchSnapshot]);

  useEffect(() => {
    drainEventRefreshesRef.current = drainEventRefreshes;
  }, [drainEventRefreshes]);

  const requestEventRefresh = useCallback(() => {
    eventRefreshPendingRef.current = true;
    drainEventRefreshes();
  }, [drainEventRefreshes]);

  useEffect(() => onTimerUpdated(requestEventRefresh), [requestEventRefresh]);

  const requestResumeRefresh = useCallback(() => {
    const now = monotonicNow();
    const previous = lastResumeRefreshRef.current;
    if (previous !== null && now - previous < 500) {
      return;
    }
    lastResumeRefreshRef.current = now;
    requestEventRefresh();
  }, [requestEventRefresh]);

  useEffect(() => {
    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        requestResumeRefresh();
      }
    };
    const onPageShow = () => requestResumeRefresh();
    document.addEventListener('visibilitychange', onVisibilityChange);
    window.addEventListener('pageshow', onPageShow);
    return () => {
      document.removeEventListener('visibilitychange', onVisibilityChange);
      window.removeEventListener('pageshow', onPageShow);
    };
  }, [requestResumeRefresh]);

  const elapsedMs = useMemo(() => {
    if (!snapshot) {
      return 0;
    }
    if (snapshot.state.status === 'running') {
      return Math.min(
        snapshot.state.durationMs,
        snapshot.state.elapsedMs + Math.max(0, clock - receivedAt),
      );
    }
    return snapshot.state.elapsedMs;
  }, [clock, receivedAt, snapshot]);

  const remainingMs = snapshot ? Math.max(0, snapshot.state.durationMs - elapsedMs) : 0;
  const progress =
    snapshot && snapshot.state.durationMs > 0
      ? Math.min(100, (elapsedMs / snapshot.state.durationMs) * 100)
      : 0;

  const value = useMemo<TimerContextValue>(
    () => ({
      snapshot,
      loading,
      error,
      elapsedMs,
      remainingMs,
      progress,
      runMutation,
      refresh,
    }),
    [elapsedMs, error, loading, progress, refresh, remainingMs, runMutation, snapshot],
  );

  return <TimerContext.Provider value={value}>{children}</TimerContext.Provider>;
}
