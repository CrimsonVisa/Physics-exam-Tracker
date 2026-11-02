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
  dailyRevisionMinutes: number;
  remindersEnabled: boolean;
  updatedAt: string;
};

export type RevisionSettingsInput = Omit<RevisionSettings, 'updatedAt'>;

export type RevisionPriority = 'High-Yield' | 'High' | 'Normal' | 'Low';
export type RevisionSyllabusStatus = 'Not started' | 'Learning' | 'Revised' | 'Confident';
export type RevisionLane = 'urgent_review' | 'strengthen' | 'maintain';
export type RevisionEvidenceStatus =
  'weak' | 'strong' | 'moderate' | 'insufficient' | 'stale' | 'skipped_only' | 'unscored' | 'none';

export type RevisionTopic = {
  id: number;
  code: string;
  title: string;
  archived: boolean;
  sectionCode: string;
  sectionTitle: string;
  priority: RevisionPriority;
  confidence: number;
  syllabusStatus: RevisionSyllabusStatus;
  category: string;
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
  deferredUntil: string | null;
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
  planItemId: number | null;
  actualSeconds: number | null;
  timingDays: number | null;
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
  lane: RevisionLane;
  triageScore: number;
  daysOverdue: number;
  daysUntilDue: number | null;
  dueUrgency: number;
  priorityNeed: number;
  confidenceNeed: number;
  statusNeed: number;
  confidenceStatusNeed: number;
  pyqEvidenceNeed: number;
  revisionRisk: number;
  evidenceStatus: RevisionEvidenceStatus;
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
  planItemId?: number | null;
};

export type RevisionCompletion = {
  state: RevisionTopicState;
  history: RevisionHistoryEntry;
  planItemId: number | null;
};

export type RevisionDeferralReason = 'time' | 'difficulty' | 'energy' | 'life' | 'other';

export type RevisionPlanBlockSnapshot = {
  id: number | null;
  title: string | null;
  calendarDate: string | null;
  startTime: string | null;
  endTime: string | null;
};

export type RevisionPlanMembership = {
  dailyPlanId: number;
  planItemId: number;
  topic: RevisionTopic;
  position: number;
  estimatedMinutes: number;
  lane: RevisionLane;
  triageScore: number;
  membershipStatus: 'planned' | 'in_progress' | 'completed';
  itemStatus: 'planned' | 'in_progress' | 'completed' | 'backlog' | 'deferred' | 'removed';
  actualSeconds: number;
  remainingEstimateMinutes: number;
  carriedFromDailyPlanId: number | null;
  plannedBlock: RevisionPlanBlockSnapshot;
};

export type RevisionBacklogItem = {
  planItemId: number;
  topic: RevisionTopic;
  status: string;
  estimatedMinutes: number;
  lane: RevisionLane;
  triageScore: number;
  sourceDueDate: string;
  actualSeconds: number;
  remainingEstimateMinutes: number;
  deferredUntil: string | null;
};

export type RevisionDeferredItem = {
  topic: RevisionTopic;
  planItemId: number | null;
  originalDueDate: string;
  previousDueDate: string;
  deferredUntil: string;
  deferralCount: number;
  latestReason: RevisionDeferralReason;
  latestNotes: string;
  latestOccurredOn: string;
};

export type RevisionUnscheduledUrgent = {
  topic: RevisionTopic;
  estimateMinutes: number;
  reason: string;
  lane: RevisionLane;
  triageScore: number;
};

export type RevisionDailyPlan = {
  today: string;
  planId: number | null;
  planDate: string | null;
  version: number | null;
  status: string | null;
  needsGeneration: boolean;
  effectiveBudgetMinutes: number;
  maxTopics: number;
  plannedMinutes: number;
  completedMinutes: number;
  actualMinutesToday: number;
  remainingWorkMinutes: number;
  remainingDailyBudgetMinutes: number;
  overBudget: boolean;
  overLimit: boolean;
  scheduled: RevisionPlanMembership[];
  completed: RevisionPlanMembership[];
  backlog: RevisionBacklogItem[];
  deferred: RevisionDeferredItem[];
  unscheduledUrgent: RevisionUnscheduledUrgent[];
};

export type RevisionPlanHistoryEntry = {
  planId: number;
  planDate: string;
  version: number;
  status: string;
  effectiveBudgetMinutes: number;
  maxTopics: number;
  plannedMinutes: number;
  completedMinutes: number;
  createdAt: string;
  lastRebalancedAt: string | null;
  memberships: RevisionPlanMembership[];
};

export type RevisionDeferralEvent = {
  id: number;
  topicId: number;
  planItemId: number | null;
  eventKind: 'defer' | 'override';
  overriddenDeferralId: number | null;
  previousDueDate: string;
  newDueDate: string;
  occurredOn: string;
  reason: RevisionDeferralReason;
  notes: string;
  createdAt: string;
};

export type RevisionDeferralResult = {
  plan: RevisionDailyPlan;
  event: RevisionDeferralEvent;
  schedule: RevisionSchedule;
};

export type RevisionIntervalDecision = {
  intervalBeforeDays: number;
  intervalAfterDays: number;
  timingDays: number;
  timingFactor: number;
  evidenceStatus: RevisionEvidenceStatus;
  evidenceFactor: number;
  reasons: string[];
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

export function getRevisionDailyPlan(): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('get_revision_daily_plan');
}

export function generateRevisionDailyPlan(): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('generate_revision_daily_plan');
}

export function rebalanceRevisionDailyPlan(): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('rebalance_revision_daily_plan');
}

export function getRevisionPlanHistory(
  fromDate: string,
  toDate: string,
): Promise<RevisionPlanHistoryEntry[]> {
  return invoke<RevisionPlanHistoryEntry[]>('get_revision_plan_history', { fromDate, toDate });
}

export function addRevisionPlanItem(topicId: number): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('add_revision_plan_item', { topicId });
}

export function moveRevisionPlanItem(
  planItemId: number,
  target: 'backlog' | 'removed',
): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('move_revision_plan_item', { planItemId, target });
}

export function reorderRevisionPlanItems(
  planId: number,
  orderedPlanItemIds: number[],
): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('reorder_revision_plan_items', {
    planId,
    orderedPlanItemIds,
  });
}

export function updateRevisionPlanEstimate(
  planItemId: number,
  estimatedMinutes: number,
): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('update_revision_plan_estimate', {
    planItemId,
    estimatedMinutes,
  });
}

export function linkRevisionPlanBlock(
  planItemId: number,
  blockId: number,
): Promise<RevisionDailyPlan> {
  return invoke<RevisionDailyPlan>('link_revision_plan_block', { planItemId, blockId });
}

export function deferRevision(
  topicId: number,
  newDate: string,
  reason: RevisionDeferralReason,
  notes: string,
): Promise<RevisionDeferralResult> {
  return invoke<RevisionDeferralResult>('defer_revision', { topicId, newDate, reason, notes });
}

export function overrideDeferredRevision(
  topicId: number,
  overrideDate: string,
  reason: RevisionDeferralReason,
  notes: string,
): Promise<RevisionDeferralResult> {
  return invoke<RevisionDeferralResult>('override_deferred_revision', {
    topicId,
    overrideDate,
    reason,
    notes,
  });
}

export function getRevisionIntervalPreview(
  topicId: number,
  outcome: RevisionOutcome,
  revisionDate?: string,
): Promise<RevisionIntervalDecision> {
  return invoke<RevisionIntervalDecision>('get_revision_interval_preview', {
    topicId,
    outcome,
    revisionDate: revisionDate ?? null,
  });
}
