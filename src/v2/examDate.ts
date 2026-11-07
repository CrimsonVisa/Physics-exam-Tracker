import { countdownFor, formatDateLong, parseDateOnly } from '../lib/dateOnly';

export type ExamDateState = 'not-set' | 'upcoming' | 'exam-day' | 'passed' | 'invalid';

export type ExamDateDisplay = {
  state: ExamDateState;
  label: string;
  detail: string;
};

function dayLabel(days: number): string {
  return days === 1 ? '1 day remaining' : `${days} days remaining`;
}

export function examDateDisplay(examDate: string | null, today: string): ExamDateDisplay {
  if (!examDate) {
    return {
      state: 'not-set',
      label: 'Not set',
      detail: 'No exam date has been set for this examination.',
    };
  }

  const formatted = formatDateLong(examDate);
  if (!formatted || !parseDateOnly(examDate)) {
    return {
      state: 'invalid',
      label: 'Invalid date',
      detail: 'The stored exam date is not a valid calendar date.',
    };
  }

  const countdown = countdownFor(examDate, today);
  if (!countdown) {
    return {
      state: 'invalid',
      label: 'Invalid date',
      detail: 'The stored exam date cannot be evaluated.',
    };
  }

  if (countdown.state === 'upcoming') {
    return {
      state: 'upcoming',
      label: dayLabel(countdown.daysRemaining),
      detail: formatted,
    };
  }

  if (countdown.state === 'exam-day') {
    return {
      state: 'exam-day',
      label: 'Exam day',
      detail: formatted,
    };
  }

  return {
    state: 'passed',
    label: 'Exam date passed',
    detail: `${formatted} · ${countdown.daysSinceExam} day${
      countdown.daysSinceExam === 1 ? '' : 's'
    } ago`,
  };
}
