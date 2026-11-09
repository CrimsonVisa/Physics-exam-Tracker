import { describe, expect, it } from 'vitest';
import {
  MAX_HISTORY_ENTRIES,
  addHistoryEntry,
  createHistoryEntry,
  formatHistoryTime,
} from './history';
import type { HistoryEntry } from './types';

function entry(index: number): HistoryEntry {
  return {
    id: `entry-${index}`,
    expression: `${index} + 1`,
    result: String(index + 1),
    value: index + 1,
    timestamp: 1_700_000_000_000 + index,
  };
}

describe('calculator history', () => {
  it('keeps newest entries first and caps the list at 50', () => {
    let entries: HistoryEntry[] = [];
    for (let index = 0; index < MAX_HISTORY_ENTRIES + 5; index += 1) {
      entries = addHistoryEntry(entries, entry(index));
    }
    expect(entries).toHaveLength(MAX_HISTORY_ENTRIES);
    expect(entries[0].expression).toBe(`${MAX_HISTORY_ENTRIES + 4} + 1`);
    expect(entries.at(-1)?.expression).toBe('5 + 1');
  });

  it('creates stable, non-empty history identifiers', () => {
    const first = createHistoryEntry({
      expression: '2 + 3',
      result: '5',
      value: 5,
      timestamp: 1_700_000_000_000,
    });
    const second = createHistoryEntry({
      expression: '2 + 3',
      result: '5',
      value: 5,
      timestamp: 1_700_000_000_000,
    });
    expect(first.id).toMatch(/^calc-/);
    expect(second.id).toMatch(/^calc-/);
    expect(first.id).not.toBe(second.id);
  });

  it('formats a local session timestamp', () => {
    expect(formatHistoryTime(1_700_000_000_000)).toMatch(/\d{1,2}:\d{2}/);
  });
});
