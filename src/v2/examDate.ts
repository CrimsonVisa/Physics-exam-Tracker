import { countdownFor, formatDateLong, parseDateOnly } from '../lib/dateOnly';

export type ExamDateState = 'not-set' | 'upcoming' | 'exam-day' | 'passed' | 'invalid';

export type ExamCountdownUrgency =
  'not-set' | 'invalid' | 'passed' | 'today' | 'imminent' | 'approaching' | 'far';

/**
 * Central urgency thresholds for every countdown surface. Keep these values in
 * one place so the dashboard and exam workspace never drift.
 */
export const EXAM_COUNTDOWN_THRESHOLDS = {
  approachingDays: 30,
  imminentDays: 7,
} as const;

export type ExamDateDisplay = {
  state: ExamDateState;
  label: string;
  detail: string;
  daysRemaining: number | null;
  daysSinceExam: number | null;
  urgency: ExamCountdownUrgency;
};

function dayLabel(days: number): string {
  return days === 1 ? '1 day remaining' : `${days} days remaining`;
}

function upcomingUrgency(days: number): ExamCountdownUrgency {
  if (days <= EXAM_COUNTDOWN_THRESHOLDS.imminentDays) {
    return 'imminent';
  }
  if (days <= EXAM_COUNTDOWN_THRESHOLDS.approachingDays) {
    return 'approaching';
  }
  return 'far';
}

export function examDateDisplay(examDate: string | null, today: string): ExamDateDisplay {
  if (!examDate) {
    return {
      state: 'not-set',
      label: 'Not set',
      detail: 'No exam date has been set for this examination.',
      daysRemaining: null,
      daysSinceExam: null,
      urgency: 'not-set',
    };
  }

  const formatted = formatDateLong(examDate);
  if (!formatted || !parseDateOnly(examDate)) {
    return {
      state: 'invalid',
      label: 'Invalid date',
      detail: 'The stored exam date is not a valid calendar date.',
      daysRemaining: null,
      daysSinceExam: null,
      urgency: 'invalid',
    };
  }

  const countdown = countdownFor(examDate, today);
  if (!countdown) {
    return {
      state: 'invalid',
      label: 'Invalid date',
      detail: 'The stored exam date cannot be evaluated.',
      daysRemaining: null,
      daysSinceExam: null,
      urgency: 'invalid',
    };
  }

  if (countdown.state === 'upcoming') {
    return {
      state: 'upcoming',
      label: dayLabel(countdown.daysRemaining),
      detail: formatted,
      daysRemaining: countdown.daysRemaining,
      daysSinceExam: null,
      urgency: upcomingUrgency(countdown.daysRemaining),
    };
  }

  if (countdown.state === 'exam-day') {
    return {
      state: 'exam-day',
      label: 'Exam day',
      detail: formatted,
      daysRemaining: 0,
      daysSinceExam: null,
      urgency: 'today',
    };
  }

  return {
    state: 'passed',
    label: 'Exam date passed',
    detail: `${formatted} · ${countdown.daysSinceExam} day${
      countdown.daysSinceExam === 1 ? '' : 's'
    } ago`,
    daysRemaining: null,
    daysSinceExam: countdown.daysSinceExam,
    urgency: 'passed',
  };
}
