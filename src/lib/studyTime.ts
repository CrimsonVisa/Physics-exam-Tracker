export function formatStudyDuration(totalSeconds: number): string {
  const seconds = Math.max(0, Math.round(totalSeconds));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const remainder = seconds % 60;

  if (hours > 0) {
    return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
  }

  if (minutes > 0) {
    return remainder > 0 ? `${minutes}m ${remainder}s` : `${minutes}m`;
  }

  return `${remainder}s`;
}

export function formatTimerClock(totalMilliseconds: number): string {
  const totalSeconds = Math.max(0, Math.ceil(totalMilliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }

  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

export function splitDurationSeconds(totalSeconds: number): { minutes: number; seconds: number } {
  const safe = Math.max(0, Math.round(totalSeconds));
  return {
    minutes: Math.floor(safe / 60),
    seconds: safe % 60,
  };
}

export function durationSecondsFromParts(minutes: number, seconds: number): number {
  return minutes * 60 + seconds;
}

export function formatTimeOfDay(value: string | null): string | null {
  if (!value) {
    return null;
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}
