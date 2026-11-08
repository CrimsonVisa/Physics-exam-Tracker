import { act, renderHook } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ReactNode } from 'react';
import { CalculatorSessionProvider } from './CalculatorSessionProvider';
import { createHistoryEntry } from './history';
import { useCalculatorSession } from './session';

function wrapper({ children }: { children: ReactNode }) {
  return <CalculatorSessionProvider>{children}</CalculatorSessionProvider>;
}

function entry(index: number) {
  return createHistoryEntry({
    expression: `${index} + 1`,
    result: String(index + 1),
    value: index + 1,
    mode: 'practice',
    timestamp: 1_700_000_000_000 + index,
  });
}

describe('CalculatorSessionProvider', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it('starts with empty history, no memory, degree mode and no previous answer', () => {
    const { result } = renderHook(() => useCalculatorSession(), { wrapper });
    expect(result.current.history).toEqual([]);
    expect(result.current.memory).toBeNull();
    expect(result.current.angleMode).toBe('deg');
    expect(result.current.answer).toBe(0);
  });

  it('retains history, memory, angle mode and answer after state updates', () => {
    const { result } = renderHook(() => useCalculatorSession(), { wrapper });
    act(() => {
      result.current.addHistory(entry(1));
      result.current.setMemory(12.5);
      result.current.setAngleMode('rad');
      result.current.setAnswer(42);
    });
    expect(result.current.history).toHaveLength(1);
    expect(result.current.memory).toBe(12.5);
    expect(result.current.angleMode).toBe('rad');
    expect(result.current.answer).toBe(42);
  });

  it('limits session history to the last 50 entries', () => {
    const { result } = renderHook(() => useCalculatorSession(), { wrapper });
    act(() => {
      for (let index = 0; index < 60; index += 1) {
        result.current.addHistory(entry(index));
      }
    });
    expect(result.current.history).toHaveLength(50);
    expect(result.current.history[0].expression).toBe('59 + 1');
    expect(result.current.history[49].expression).toBe('10 + 1');
  });

  it('clears history without clearing memory, angle mode or answer', () => {
    const { result } = renderHook(() => useCalculatorSession(), { wrapper });
    act(() => {
      result.current.addHistory(entry(1));
      result.current.setMemory(9);
      result.current.setAngleMode('rad');
      result.current.setAnswer(7);
      result.current.clearHistory();
    });
    expect(result.current.history).toEqual([]);
    expect(result.current.memory).toBe(9);
    expect(result.current.angleMode).toBe('rad');
    expect(result.current.answer).toBe(7);
  });

  it('starts a new provider session with cleared state', () => {
    const first = renderHook(() => useCalculatorSession(), { wrapper });
    act(() => {
      first.result.current.addHistory(entry(1));
      first.result.current.setMemory(5);
      first.result.current.setAngleMode('rad');
      first.result.current.setAnswer(8);
    });
    first.unmount();

    const second = renderHook(() => useCalculatorSession(), { wrapper });
    expect(second.result.current.history).toEqual([]);
    expect(second.result.current.memory).toBeNull();
    expect(second.result.current.angleMode).toBe('deg');
    expect(second.result.current.answer).toBe(0);
  });

  it('does not write browser storage when session state changes', () => {
    const { result } = renderHook(() => useCalculatorSession(), { wrapper });
    act(() => {
      result.current.addHistory(entry(1));
      result.current.setMemory(3);
      result.current.setAngleMode('rad');
      result.current.setAnswer(11);
    });
    expect(window.localStorage).toHaveLength(0);
    expect(window.sessionStorage).toHaveLength(0);
  });

  it('contains no browser-storage or database persistence in the calculator session path', () => {
    for (const path of [
      'src/v2/calculator/session.ts',
      'src/v2/calculator/CalculatorSessionProvider.tsx',
      'src/v2/CalculatorScreen.tsx',
    ]) {
      const source = readFileSync(join(process.cwd(), path), 'utf8');
      expect(source).not.toMatch(/localStorage|sessionStorage|@tauri-apps\/api/);
      expect(source).not.toMatch(/invoke\(|\.sqlite3?\b|\.ptbackup\b/);
    }
  });
});
