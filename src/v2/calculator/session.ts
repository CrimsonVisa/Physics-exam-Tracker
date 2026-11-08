import { createContext, useContext } from 'react';
import type { Dispatch, SetStateAction } from 'react';
import type { AngleMode, HistoryEntry } from './types';

export type CalculatorSessionValue = {
  history: HistoryEntry[];
  memory: number | null;
  angleMode: AngleMode;
  answer: number;
  addHistory: (entry: HistoryEntry) => void;
  clearHistory: () => void;
  setMemory: Dispatch<SetStateAction<number | null>>;
  setAngleMode: Dispatch<SetStateAction<AngleMode>>;
  setAnswer: Dispatch<SetStateAction<number>>;
};

export const CalculatorSessionContext = createContext<CalculatorSessionValue | null>(null);

export function useCalculatorSession(): CalculatorSessionValue {
  const value = useContext(CalculatorSessionContext);
  if (!value) {
    throw new Error('useCalculatorSession must be used inside CalculatorSessionProvider.');
  }
  return value;
}
