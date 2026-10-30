import { describe, expect, it } from 'vitest';
import type { RevisionSettings } from './revisionApi';
import { initialIntervalFor, nextIntervalFor, outcomeLabel } from './revisionScheduling';

const SETTINGS: RevisionSettings = {
  initialAgainDays: 1,
  initialDifficultDays: 2,
  initialGoodDays: 5,
  initialEasyDays: 10,
  growthMultiplier: 1.5,
  minimumIntervalDays: 1,
  maximumIntervalDays: 3650,
  includeWeekends: true,
  dailyRevisionTarget: 5,
  remindersEnabled: false,
  updatedAt: '2026-10-30T10:00:00.000Z',
};

describe('revision scheduling preview helpers', () => {
  it('maps outcomes to initial intervals', () => {
    expect(initialIntervalFor(SETTINGS, 'again')).toBe(1);
    expect(initialIntervalFor(SETTINGS, 'difficult')).toBe(2);
    expect(initialIntervalFor(SETTINGS, 'good')).toBe(5);
    expect(initialIntervalFor(SETTINGS, 'easy')).toBe(10);
    expect(outcomeLabel('again')).toBe('Again / forgot');
    expect(outcomeLabel(null)).toBe('No previous outcome');
  });

  it('grows Good and Easy intervals and resets Again and Difficult', () => {
    expect(nextIntervalFor(SETTINGS, 1, 'good')).toBe(5);
    expect(nextIntervalFor(SETTINGS, 10, 'good')).toBe(15);
    expect(nextIntervalFor(SETTINGS, 20, 'easy')).toBe(30);
    expect(nextIntervalFor(SETTINGS, 30, 'again')).toBe(1);
    expect(nextIntervalFor(SETTINGS, 30, 'difficult')).toBe(2);
  });

  it('clamps intervals to configured bounds', () => {
    const bounded = {
      ...SETTINGS,
      minimumIntervalDays: 7,
      maximumIntervalDays: 20,
      initialAgainDays: 7,
    };
    expect(nextIntervalFor(bounded, 18, 'good')).toBe(20);
    expect(nextIntervalFor(bounded, 18, 'again')).toBe(7);
  });
});
