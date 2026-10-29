import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useTodayDateOnly } from './useTodayDateOnly';

describe('useTodayDateOnly', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('refreshes the countdown date after local midnight', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 11, 18, 23, 59, 30));

    const { result } = renderHook(() => useTodayDateOnly());
    expect(result.current).toBe('2026-12-18');

    vi.setSystemTime(new Date(2026, 11, 19, 0, 0, 1));
    act(() => {
      vi.advanceTimersByTime(60_000);
    });

    expect(result.current).toBe('2026-12-19');
  });

  it('prefers an explicit date and leaves the clock alone', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 11, 18, 23, 59, 30));

    const { result } = renderHook(() => useTodayDateOnly('2026-12-19'));
    expect(result.current).toBe('2026-12-19');

    vi.setSystemTime(new Date(2026, 11, 20, 12, 0, 0));
    act(() => {
      vi.advanceTimersByTime(120_000);
    });

    expect(result.current).toBe('2026-12-19');
  });
});
