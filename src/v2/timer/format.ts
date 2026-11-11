import type { V2TimerMode } from '../api';

export const TIMER_MODE_LABELS: Record<V2TimerMode, string> = {
  focus: 'Focus',
  shortBreak: 'Short break',
  longBreak: 'Long break',
};

export const TIMER_CARD_MODE_LABELS: Record<V2TimerMode, string> = {
  focus: 'Focus session',
  shortBreak: 'Short break',
  longBreak: 'Long break',
};

function pad(value: number): string {
  return String(value).padStart(2, '0');
}

/** Existing V2 timer readout semantics: whole seconds, floored. */
export function formatTimerClock(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1_000));
  const hours = Math.floor(totalSeconds / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${pad(minutes)}:${pad(seconds)}`;
}
