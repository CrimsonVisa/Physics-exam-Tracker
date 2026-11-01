import { invoke } from '@tauri-apps/api/core';

export const REVISION_OUTCOMES = ['again', 'difficult', 'good', 'easy'] as const;
export type RevisionOutcome = (typeof REVISION_OUTCOMES)[number];

export type RevisionSettings = {
  initialAgainDays: number;
  initialDifficultDays: number;
  initialGoodDays: number;
  initialEasyDays: number;
  growthMultiplier: number;
  minimumIntervalDays: number;
  maximumIntervalDays: number;
  includeWeekends: boolean;
  dailyRevisionTarget: number;
  remindersEnabled: boolean;
  updatedAt: string;
};

export type RevisionSettingsInput = Omit<RevisionSettings, 'updatedAt'>;

export type RevisionTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
  sectionCode: string;
  sectionTitle: string;
};

export type RevisionSchedule = {
  status: 'active' | 'suspended';
  lastRevisionDate: string | null;
  nextRevisionDate: string;
  currentIntervalDays: number;
  completedRevisions: number;
  previousOutcome: RevisionOutcome | null;
  masteryEstimate: number;
  notes: string;
  updatedAt: string;
};

export type RevisionTopicState = {
  topic: RevisionTopic;
  schedule: RevisionSchedule | null;
  dueState: 'not_scheduled' | 'suspended' | 'overdue' | 'due_today' | 'upcoming';
  daysOverdue: number;
  daysUntilDue: number | null;
};

export type RevisionStats = {
  scheduledCount: number;
  activeCount: number;
  suspendedCount: number;
  dueTodayCount: number;
  overdueCount: number;
  completedToday: number;
  completedLast7Days: number;
  totalCompleted: number;
};

export type RevisionHistoryEntry = {
  id: number;
  topicId: number;
  revisionDate: string;
  scheduledForDate: string;
  outcome: RevisionOutcome;
  notes: string;
  intervalBeforeDays: number;
  intervalAfterDays: number;
  masteryBefore: number;
  masteryAfter: number;
  createdAt: string;
};

export type RevisionHistoryWithTopic = {
  history: RevisionHistoryEntry;
  topic: RevisionTopic;
};

export type RevisionRecommendation = {
  topic: RevisionTopic;
  scheduled: boolean;
  dueState: RevisionTopicState['dueState'];
  nextRevisionDate: string | null;
  previousOutcome: RevisionOutcome | null;
  masteryEstimate: number | null;
  pyqAttempts: number;
  pyqQuestions: number;
  pyqAccuracy: number | null;
  pyqEvidenceAccuracy: number | null;
  pyqLastAttemptDate: string | null;
  pyqSufficient: boolean;
  priority: 'high' | 'medium' | 'low';
  score: number;
  reasons: string[];
};

export type RevisionDashboard = {
  today: string;
  settings: RevisionSettings;
  stats: RevisionStats;
  dueToday: RevisionTopicState[];
  overdue: RevisionTopicState[];
  upcoming: RevisionTopicState[];
  suspended: RevisionTopicState[];
  completedRecent: RevisionHistoryWithTopic[];
  topics: RevisionTopicState[];
  archivedTopics: RevisionTopicState[];
  recommendations: RevisionRecommendation[];
  mockScopeNote: string;
  mockTestCount: number;
};

export type RevisionSummary = {
  topicId: number;
  status: 'active' | 'suspended' | null;
  dueState: RevisionTopicState['dueState'];
  nextRevisionDate: string | null;
  completedRevisions: number;
  masteryEstimate: number | null;
};

export type RevisionCalendarItem = {
  topicId: number;
  topicCode: string;
  topicTitle: string;
  sectionCode: string;
  sectionTitle: string;
  nextRevisionDate: string;
  currentIntervalDays: number;
  completedRevisions: number;
  masteryEstimate: number;
};

export type CompleteRevisionInput = {
  outcome: RevisionOutcome;
  notes: string;
};

export type RevisionCompletion = {
  state: RevisionTopicState;
  history: RevisionHistoryEntry;
};

export type RevisionReminder = {
  date: string;
  dueToday: number;
  overdue: number;
  total: number;
};

export function getRevisionDashboard(): Promise<RevisionDashboard> {
  return invoke<RevisionDashboard>('get_revision_dashboard');
}

export function updateRevisionSettings(settings: RevisionSettingsInput): Promise<RevisionSettings> {
  return invoke<RevisionSettings>('update_revision_settings', { settings });
}

export function getRevisionHistory(topicId: number): Promise<RevisionHistoryEntry[]> {
  return invoke<RevisionHistoryEntry[]>('get_revision_history', { topicId });
}

export function scheduleRevisionTopic(
  topicId: number,
  nextDate: string,
): Promise<RevisionTopicState> {
  return invoke<RevisionTopicState>('schedule_revision_topic', { topicId, nextDate });
}

export function rescheduleRevision(topicId: number, nextDate: string): Promise<RevisionTopicState> {
  return invoke<RevisionTopicState>('reschedule_revision', { topicId, nextDate });
}

export function updateRevisionNotes(topicId: number, notes: string): Promise<RevisionTopicState> {
  return invoke<RevisionTopicState>('update_revision_notes', { topicId, notes });
}

export function suspendRevision(topicId: number): Promise<RevisionTopicState> {
  return invoke<RevisionTopicState>('suspend_revision', { topicId });
}

export function reactivateRevision(topicId: number): Promise<RevisionTopicState> {
  return invoke<RevisionTopicState>('reactivate_revision', { topicId });
}

export function completeRevision(
  topicId: number,
  input: CompleteRevisionInput,
): Promise<RevisionCompletion> {
  return invoke<RevisionCompletion>('complete_revision', { topicId, input });
}

export function getRevisionSummaries(): Promise<RevisionSummary[]> {
  return invoke<RevisionSummary[]>('get_revision_summaries');
}

export function checkRevisionReminder(): Promise<RevisionReminder | null> {
  return invoke<RevisionReminder | null>('check_revision_reminder');
}
