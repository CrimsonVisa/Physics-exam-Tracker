import type { HistoryEntry } from './types';

export const MAX_HISTORY_ENTRIES = 50;

export function createHistoryEntry(entry: Omit<HistoryEntry, 'id'>): HistoryEntry {
  return {
    ...entry,
    id: `calc-${entry.timestamp}-${Math.random().toString(36).slice(2, 8)}`,
  };
}

export function addHistoryEntry(entries: HistoryEntry[], entry: HistoryEntry): HistoryEntry[] {
  return [entry, ...entries].slice(0, MAX_HISTORY_ENTRIES);
}

export function formatHistoryTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });
}
