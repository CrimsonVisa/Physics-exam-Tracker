import { invoke } from '@tauri-apps/api/core';

export type SessionSource = 'timer' | 'manual';

export type SessionTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
};

export type StudySession = {
  id: number;
  topic: SessionTopic | null;
  sessionDate: string;
  startedAt: string | null;
  endedAt: string | null;
  durationSeconds: number;
  source: SessionSource;
  notes: string;
  createdAt: string;
  updatedAt: string;
};

export type StudySessionFilter = {
  fromDate?: string;
  toDate?: string;
  topicId?: number;
};

export type StudySessionInput = {
  topicId: number | null;
  sessionDate: string;
  durationSeconds: number;
  notes: string;
};

export type DailyStudyTotal = {
  date: string;
  seconds: number;
  sessionCount: number;
};

export type WeeklyStudyTotal = {
  weekStart: string;
  weekEnd: string;
  seconds: number;
  sessionCount: number;
};

export type StudyTotals = {
  rangeFrom: string;
  rangeTo: string;
  rangeSeconds: number;
  rangeSessionCount: number;
  todayDate: string;
  todaySeconds: number;
  weekStart: string;
  weekEnd: string;
  weekSeconds: number;
  daily: DailyStudyTotal[];
  weekly: WeeklyStudyTotal[];
};

export function getStudySessions(filter: StudySessionFilter): Promise<StudySession[]> {
  return invoke<StudySession[]>('get_study_sessions', { filter });
}

export function getSessionTopics(): Promise<SessionTopic[]> {
  return invoke<SessionTopic[]>('get_session_topics');
}

export function createManualSession(session: StudySessionInput): Promise<StudySession> {
  return invoke<StudySession>('create_manual_session', { session });
}

export function updateStudySession(
  sessionId: number,
  session: StudySessionInput,
): Promise<StudySession> {
  return invoke<StudySession>('update_study_session', { sessionId, session });
}

export function deleteStudySession(sessionId: number): Promise<void> {
  return invoke<void>('delete_study_session', { sessionId });
}

export function getStudyTotals(
  fromDate: string,
  toDate: string,
  topicId: number | null = null,
): Promise<StudyTotals> {
  return invoke<StudyTotals>('get_study_totals', { fromDate, toDate, topicId });
}
