import { describe, expect, it } from 'vitest';
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
});
