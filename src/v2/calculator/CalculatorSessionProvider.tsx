import { useCallback, useMemo, useState, type ReactNode } from 'react';
import { addHistoryEntry } from './history';
import { CalculatorSessionContext, type CalculatorSessionValue } from './session';
import type { AngleMode, HistoryEntry } from './types';

type CalculatorSessionProviderProps = {
  children: ReactNode;
};

export function CalculatorSessionProvider({ children }: CalculatorSessionProviderProps) {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [memory, setMemory] = useState<number | null>(null);
  const [angleMode, setAngleMode] = useState<AngleMode>('deg');
  const [answer, setAnswer] = useState(0);

  const addHistory = useCallback((entry: HistoryEntry) => {
    setHistory((entries) => addHistoryEntry(entries, entry));
  }, []);

  const clearHistory = useCallback(() => {
    setHistory([]);
  }, []);

  const value = useMemo<CalculatorSessionValue>(
    () => ({
      history,
      memory,
      angleMode,
      answer,
      addHistory,
      clearHistory,
      setMemory,
      setAngleMode,
      setAnswer,
    }),
    [
      history,
      memory,
      angleMode,
      answer,
      addHistory,
      clearHistory,
      setMemory,
      setAngleMode,
      setAnswer,
    ],
  );

  return (
    <CalculatorSessionContext.Provider value={value}>{children}</CalculatorSessionContext.Provider>
  );
}
