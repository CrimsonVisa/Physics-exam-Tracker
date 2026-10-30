const TIME_PATTERN = /^([01]\d|2[0-3]):([0-5]\d)$/;

export function isValidTimeOfDay(value: string): boolean {
  return TIME_PATTERN.test(value);
}

export function timeOfDayMinutes(value: string): number | null {
  const match = TIME_PATTERN.exec(value);
  if (!match) {
    return null;
  }

  return Number(match[1]) * 60 + Number(match[2]);
}

export function formatTimeRange(startTime: string, endTime: string): string {
  return `${startTime}–${endTime}`;
}

export function plannedMinutes(startTime: string, endTime: string): number | null {
  const start = timeOfDayMinutes(startTime);
  const end = timeOfDayMinutes(endTime);
  if (start === null || end === null || end <= start) {
    return null;
  }

  return end - start;
}
