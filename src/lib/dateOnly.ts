export type DateParts = Readonly<{
  year: number;
  month: number;
  day: number;
}>;

export type Countdown =
  | { state: 'upcoming'; daysRemaining: number }
  | { state: 'exam-day'; daysRemaining: 0 }
  | { state: 'past'; daysSinceExam: number };

const DATE_ONLY_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;
const MONTH_NAMES = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
] as const;

export function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

export function daysInMonth(year: number, month: number): number {
  if (month < 1 || month > 12) {
    return 0;
  }

  if (month === 2) {
    return isLeapYear(year) ? 29 : 28;
  }

  return [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1];
}

/**
 * Parse a strict date-only value without using `Date` or any UTC conversion.
 * Impossible dates such as 2026-02-30 are rejected.
 */
export function parseDateOnly(value: string): DateParts | null {
  const match = DATE_ONLY_PATTERN.exec(value);
  if (!match) {
    return null;
  }

  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);

  if (year < 1 || year > 9999 || month < 1 || month > 12 || day < 1) {
    return null;
  }

  if (day > daysInMonth(year, month)) {
    return null;
  }

  return { year, month, day };
}

export function isValidDateOnly(value: string): boolean {
  return parseDateOnly(value) !== null;
}

export function formatDateLong(value: string): string | null {
  const parts = parseDateOnly(value);
  if (!parts) {
    return null;
  }

  return `${parts.day} ${MONTH_NAMES[parts.month - 1]} ${parts.year}`;
}

/**
 * Days since 1970-01-01 using the proleptic Gregorian calendar. This integer
 * algorithm is intentionally independent of local time and timezone offsets.
 */
function daysFromCivil({ year, month, day }: DateParts): number {
  const adjustedYear = year - (month <= 2 ? 1 : 0);
  const era = Math.floor(adjustedYear / 400);
  const yearOfEra = adjustedYear - era * 400;
  const dayOfYear = Math.floor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
  const dayOfEra =
    yearOfEra * 365 + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100) + dayOfYear;

  return era * 146097 + dayOfEra - 719468;
}

export function calendarDaysBetween(startDate: string, endDate: string): number | null {
  const start = parseDateOnly(startDate);
  const end = parseDateOnly(endDate);

  if (!start || !end) {
    return null;
  }

  return daysFromCivil(end) - daysFromCivil(start);
}

function civilFromDays(days: number): DateParts {
  const z = days + 719468;
  const era = Math.floor(z / 146097);
  const dayOfEra = z - era * 146097;
  const yearOfEra = Math.floor(
    (dayOfEra -
      Math.floor(dayOfEra / 1460) +
      Math.floor(dayOfEra / 36524) -
      Math.floor(dayOfEra / 146096)) /
      365,
  );
  const year = yearOfEra + era * 400;
  const dayOfYear =
    dayOfEra - (365 * yearOfEra + Math.floor(yearOfEra / 4) - Math.floor(yearOfEra / 100));
  const monthPrime = Math.floor((5 * dayOfYear + 2) / 153);
  const day = dayOfYear - Math.floor((153 * monthPrime + 2) / 5) + 1;
  const month = monthPrime + (monthPrime < 10 ? 3 : -9);

  return {
    year: year + (month <= 2 ? 1 : 0),
    month,
    day,
  };
}

/** Add a whole number of calendar days without involving time zones. */
export function addDaysDateOnly(value: string, offset: number): string | null {
  const parts = parseDateOnly(value);
  if (!parts || !Number.isSafeInteger(offset)) {
    return null;
  }

  const shifted = civilFromDays(daysFromCivil(parts) + offset);
  if (shifted.year < 1 || shifted.year > 9999) {
    return null;
  }

  const year = String(shifted.year).padStart(4, '0');
  const month = String(shifted.month).padStart(2, '0');
  const day = String(shifted.day).padStart(2, '0');

  return `${year}-${month}-${day}`;
}

export function countdownFor(examDate: string, today: string): Countdown | null {
  const days = calendarDaysBetween(today, examDate);
  if (days === null) {
    return null;
  }

  if (days > 0) {
    return { state: 'upcoming', daysRemaining: days };
  }

  if (days === 0) {
    return { state: 'exam-day', daysRemaining: 0 };
  }

  return { state: 'past', daysSinceExam: Math.abs(days) };
}

/** Current local calendar date, derived from local date fields rather than UTC. */
export function todayDateOnly(now: Date = new Date()): string {
  const year = String(now.getFullYear()).padStart(4, '0');
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');

  return `${year}-${month}-${day}`;
}
