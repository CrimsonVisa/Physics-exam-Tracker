import { invoke } from '@tauri-apps/api/core';

export const TIMER_MODES = ['focus', 'short_break', 'long_break'] as const;
export type TimerMode = (typeof TIMER_MODES)[number];

export type TimerStatus = 'idle' | 'running' | 'paused';
export type RecoveryReason = 'restart' | 'inactivity';
export type LastTimerEvent = 'completed' | 'discarded';

export type TimerSettings = {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  notificationsEnabled: boolean;
};

export type TimerSettingsInput = TimerSettings;

export type TimerTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
};

export type TimerRecovery = {
  reason: RecoveryReason;
  excludedMs: number;
  recoveredAt: string;
};

export type TimerState = {
  status: TimerStatus;
  mode: TimerMode;
  durationMs: number;
  accumulatedMs: number;
  remainingMs: number;
  firstStartedAt: string | null;
  sessionDate: string | null;
  topic: TimerTopic | null;
  lastEvent: LastTimerEvent | null;
  lastEventAt: string | null;
  lastEventDurationMs: number;
  lastCompletedSessionId: number | null;
  revisionPlanItemId: number | null;
  recovery: TimerRecovery | null;
};

export type TimerSnapshot = {
  state: TimerState;
  settings: TimerSettings;
  serverNowMs: number;
};

export function getTimerSnapshot(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('get_timer_snapshot');
}

export function updateTimerSettings(settings: TimerSettingsInput): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('update_timer_settings', { settings });
}

export function startStudyTimer(
  mode: TimerMode,
  topicId: number | null,
  planItemId: number | null = null,
): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>(
    'start_study_timer',
    planItemId === null ? { mode, topicId } : { mode, topicId, planItemId },
  );
}

export function pauseStudyTimer(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('pause_study_timer');
}

export function resumeStudyTimer(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('resume_study_timer');
}

export function finishStudyTimer(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('finish_study_timer');
}

export function discardStudyTimer(): Promise<TimerSnapshot> {
  return invoke<TimerSnapshot>('discard_study_timer');
}
