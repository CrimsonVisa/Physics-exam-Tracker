export const CHART_COLORS = [
  '#6ea8ff',
  '#7bc58a',
  '#e0b84a',
  '#e59ab8',
  '#b49cf0',
  '#e8a45e',
  '#75c9c4',
  '#c4c9d1',
];

export function formatChartDuration(seconds: number): string {
  const safe = Math.max(0, Math.floor(seconds));
  if (safe < 60) {
    return `${safe} s`;
  }
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const remainingSeconds = safe % 60;
  const parts: string[] = [];
  if (hours > 0) {
    parts.push(`${hours} h`);
  }
  if (minutes > 0) {
    parts.push(`${minutes} min`);
  }
  if (remainingSeconds > 0) {
    parts.push(`${remainingSeconds} s`);
  }
  return parts.join(' ');
}

export function formatChartPercent(value: number | null): string {
  return value === null ? 'Not available' : `${value.toFixed(1)}%`;
}

export function truncateLabel(value: string, max = 34): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}

export function shortChartDate(value: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  return match ? `${match[3]}/${match[2]}` : value;
}
