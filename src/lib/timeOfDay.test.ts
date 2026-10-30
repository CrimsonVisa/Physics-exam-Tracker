import { describe, expect, it } from 'vitest';
import { formatTimeRange, isValidTimeOfDay, plannedMinutes, timeOfDayMinutes } from './timeOfDay';

describe('time-of-day helpers', () => {
  it('accepts strict 24-hour HH:MM values', () => {
    expect(isValidTimeOfDay('00:00')).toBe(true);
    expect(isValidTimeOfDay('09:05')).toBe(true);
    expect(isValidTimeOfDay('23:59')).toBe(true);
    expect(isValidTimeOfDay('24:00')).toBe(false);
    expect(isValidTimeOfDay('9:05')).toBe(false);
    expect(isValidTimeOfDay('10:60')).toBe(false);
    expect(isValidTimeOfDay('')).toBe(false);
  });

  it('converts times to minutes and calculates planned durations', () => {
    expect(timeOfDayMinutes('10:30')).toBe(630);
    expect(timeOfDayMinutes('24:00')).toBeNull();
    expect(plannedMinutes('10:00', '11:30')).toBe(90);
    expect(plannedMinutes('11:30', '10:00')).toBeNull();
    expect(plannedMinutes('10:00', '10:00')).toBeNull();
    expect(formatTimeRange('10:00', '11:30')).toBe('10:00–11:30');
  });
});
