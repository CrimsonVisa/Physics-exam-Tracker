import { describe, expect, it } from 'vitest';
import { addDaysDateOnly } from '../lib/dateOnly';
import { examDateDisplay } from './examDate';

const TODAY = '2026-11-04';

describe('V2 exam date display', () => {
  it('shows an explicit not-set state without inventing a date', () => {
    expect(examDateDisplay(null, TODAY)).toMatchObject({
      state: 'not-set',
      label: 'Not set',
    });
  });

  it('shows a future countdown using local calendar dates', () => {
    expect(examDateDisplay('2026-11-06', TODAY)).toMatchObject({
      state: 'upcoming',
      label: '2 days remaining',
      detail: '6 November 2026',
    });
  });

  it('uses the correct singular countdown label', () => {
    expect(examDateDisplay('2026-11-05', TODAY)).toMatchObject({
      state: 'upcoming',
      label: '1 day remaining',
    });
  });

  it('shows exam day for today', () => {
    expect(examDateDisplay(TODAY, TODAY)).toMatchObject({
      state: 'exam-day',
      label: 'Exam day',
      detail: '4 November 2026',
    });
  });

  it('shows a passed state for past dates', () => {
    expect(examDateDisplay('2026-11-01', TODAY)).toMatchObject({
      state: 'passed',
      label: 'Exam date passed',
      detail: '1 November 2026 · 3 days ago',
    });
  });

  it('rejects impossible stored dates instead of guessing', () => {
    expect(examDateDisplay('2026-02-30', TODAY)).toMatchObject({
      state: 'invalid',
      label: 'Invalid date',
    });
  });

  it('classifies countdown urgency at the configured boundaries', () => {
    const at = (days: number) => addDaysDateOnly(TODAY, days)!;

    expect(examDateDisplay(at(31), TODAY)).toMatchObject({
      urgency: 'far',
      daysRemaining: 31,
    });
    expect(examDateDisplay(at(30), TODAY)).toMatchObject({
      urgency: 'approaching',
      daysRemaining: 30,
    });
    expect(examDateDisplay(at(8), TODAY)).toMatchObject({
      urgency: 'approaching',
      daysRemaining: 8,
    });
    expect(examDateDisplay(at(7), TODAY)).toMatchObject({
      urgency: 'imminent',
      daysRemaining: 7,
    });
    expect(examDateDisplay(TODAY, TODAY)).toMatchObject({
      urgency: 'today',
      daysRemaining: 0,
    });
    expect(examDateDisplay(at(-1), TODAY)).toMatchObject({
      urgency: 'passed',
      daysRemaining: null,
      daysSinceExam: 1,
    });
  });

  it('uses calendar days across a daylight-saving transition', () => {
    expect(examDateDisplay('2026-03-30', '2026-03-28')).toMatchObject({
      state: 'upcoming',
      daysRemaining: 2,
      label: '2 days remaining',
    });
  });
});
