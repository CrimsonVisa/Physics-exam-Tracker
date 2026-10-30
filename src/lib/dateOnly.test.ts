import { describe, expect, it } from 'vitest';
import {
  addDaysDateOnly,
  calendarDaysBetween,
  countdownFor,
  daysInMonth,
  formatDateLong,
  isLeapYear,
  isValidDateOnly,
  parseDateOnly,
  todayDateOnly,
} from './dateOnly';

describe('date-only parsing', () => {
  it('parses strict calendar dates without losing their civil value', () => {
    expect(parseDateOnly('2026-12-19')).toEqual({ year: 2026, month: 12, day: 19 });
    expect(parseDateOnly('2024-02-29')).toEqual({ year: 2024, month: 2, day: 29 });
    expect(parseDateOnly('2000-02-29')).toEqual({ year: 2000, month: 2, day: 29 });
    expect(parseDateOnly('0001-01-01')).toEqual({ year: 1, month: 1, day: 1 });
    expect(parseDateOnly('9999-12-31')).toEqual({ year: 9999, month: 12, day: 31 });
  });

  it.each([
    '2026-02-30',
    '2026-02-29',
    '1900-02-29',
    '2026-04-31',
    '2026-06-31',
    '2026-13-01',
    '2026-00-10',
    '2026-12-00',
    '2026-2-03',
    '26-12-19',
    '2026/12/19',
    ' 2026-12-19',
    '2026-12-19 ',
    '0000-01-01',
    '10000-01-01',
    '',
  ])('rejects the impossible or malformed value %j', (value) => {
    expect(parseDateOnly(value)).toBeNull();
    expect(isValidDateOnly(value)).toBe(false);
  });

  it('distinguishes leap years using the Gregorian rules', () => {
    expect(isLeapYear(2024)).toBe(true);
    expect(isLeapYear(2000)).toBe(true);
    expect(isLeapYear(2025)).toBe(false);
    expect(isLeapYear(1900)).toBe(false);
    expect(isLeapYear(2100)).toBe(false);
  });

  it('reports the correct length for every month', () => {
    expect(daysInMonth(2024, 2)).toBe(29);
    expect(daysInMonth(2026, 2)).toBe(28);
    expect(daysInMonth(2026, 4)).toBe(30);
    expect(daysInMonth(2026, 12)).toBe(31);
    expect(daysInMonth(2026, 0)).toBe(0);
    expect(daysInMonth(2026, 13)).toBe(0);
  });

  it('formats a human-readable label from the same parsed value', () => {
    expect(formatDateLong('2026-12-19')).toBe('19 December 2026');
    expect(formatDateLong('2024-02-29')).toBe('29 February 2024');
    expect(formatDateLong('2026-02-30')).toBeNull();
  });
});

describe('calendar-day arithmetic', () => {
  it('counts days across a month boundary', () => {
    expect(calendarDaysBetween('2026-01-31', '2026-02-01')).toBe(1);
    expect(calendarDaysBetween('2026-02-28', '2026-03-01')).toBe(1);
    expect(calendarDaysBetween('2024-02-28', '2024-03-01')).toBe(2);
  });

  it('counts days across a year boundary', () => {
    expect(calendarDaysBetween('2026-12-31', '2027-01-01')).toBe(1);
    expect(calendarDaysBetween('2026-12-19', '2027-12-19')).toBe(365);
    expect(calendarDaysBetween('2024-12-19', '2025-12-19')).toBe(365);
  });

  it('treats daylight-saving transitions as whole calendar days', () => {
    expect(calendarDaysBetween('2026-03-07', '2026-03-09')).toBe(2);
    expect(calendarDaysBetween('2026-10-31', '2026-11-02')).toBe(2);
  });

  it('returns zero for the same day and negative values for reversed ranges', () => {
    expect(calendarDaysBetween('2026-12-19', '2026-12-19')).toBe(0);
    expect(calendarDaysBetween('2026-12-20', '2026-12-19')).toBe(-1);
  });

  it('returns null instead of guessing for invalid input', () => {
    expect(calendarDaysBetween('2026-02-30', '2026-12-19')).toBeNull();
    expect(calendarDaysBetween('2026-12-19', 'not-a-date')).toBeNull();
  });
});

describe('date-only arithmetic', () => {
  it('adds and subtracts calendar days without timezone conversion', () => {
    expect(addDaysDateOnly('2026-12-10', -29)).toBe('2026-11-11');
    expect(addDaysDateOnly('2026-12-31', 1)).toBe('2027-01-01');
    expect(addDaysDateOnly('2024-02-28', 1)).toBe('2024-02-29');
    expect(addDaysDateOnly('2024-02-28', 2)).toBe('2024-03-01');
    expect(addDaysDateOnly('2026-01-01', -1)).toBe('2025-12-31');
  });

  it('returns null for an invalid starting date', () => {
    expect(addDaysDateOnly('2026-02-30', 1)).toBeNull();
    expect(addDaysDateOnly('not-a-date', 1)).toBeNull();
  });

  it('returns null when the offset is not a whole day or leaves the supported range', () => {
    expect(addDaysDateOnly('2026-12-10', 1.5)).toBeNull();
    expect(addDaysDateOnly('2026-12-10', Number.NaN)).toBeNull();
    expect(addDaysDateOnly('0001-01-01', -1)).toBeNull();
    expect(addDaysDateOnly('9999-12-31', 1)).toBeNull();
  });
});

describe('countdown states', () => {
  it('reports upcoming days across a month and year boundary', () => {
    expect(countdownFor('2026-12-19', '2026-12-18')).toEqual({
      state: 'upcoming',
      daysRemaining: 1,
    });
    expect(countdownFor('2027-01-02', '2026-12-31')).toEqual({
      state: 'upcoming',
      daysRemaining: 2,
    });
  });

  it('reports zero days on exam day', () => {
    expect(countdownFor('2026-12-19', '2026-12-19')).toEqual({
      state: 'exam-day',
      daysRemaining: 0,
    });
  });

  it('reports elapsed days without a negative countdown after the exam', () => {
    expect(countdownFor('2026-12-19', '2026-12-20')).toEqual({
      state: 'past',
      daysSinceExam: 1,
    });
    expect(countdownFor('2026-12-19', '2027-01-02')).toEqual({
      state: 'past',
      daysSinceExam: 14,
    });
  });

  it('returns null when either side is not a real calendar date', () => {
    expect(countdownFor('2026-02-30', '2026-12-19')).toBeNull();
    expect(countdownFor('2026-12-19', '')).toBeNull();
  });
});

describe('todayDateOnly', () => {
  it('uses the local calendar fields rather than UTC serialization', () => {
    const lateEvening = new Date(2026, 9, 29, 23, 59, 59);
    const earlyMorning = new Date(2026, 0, 5, 0, 30, 0);

    expect(todayDateOnly(lateEvening)).toBe('2026-10-29');
    expect(todayDateOnly(earlyMorning)).toBe('2026-01-05');
  });

  it('pads single-digit months and days', () => {
    expect(todayDateOnly(new Date(2026, 1, 3, 12, 0, 0))).toBe('2026-02-03');
  });
});
