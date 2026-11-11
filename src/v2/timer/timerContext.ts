import { createContext, useContext } from 'react';
import type { V2TimerSnapshot } from '../api';

export type TimerContextValue = {
  snapshot: V2TimerSnapshot | null;
  loading: boolean;
  error: string | null;
  elapsedMs: number;
  remainingMs: number;
  progress: number;
  runMutation: (operation: () => Promise<V2TimerSnapshot>) => Promise<V2TimerSnapshot>;
  refresh: (showLoading?: boolean) => Promise<void>;
};

export const TimerContext = createContext<TimerContextValue | null>(null);

export function useV2Timer(): TimerContextValue {
  const value = useContext(TimerContext);
  if (!value) {
    throw new Error('useV2Timer must be used inside TimerProvider.');
  }
  return value;
}
