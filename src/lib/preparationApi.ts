import { invoke } from '@tauri-apps/api/core';
import type { MockTest } from './mockTestApi';
import type { ChecklistTask, PlannedBlock } from './plannerApi';
import type { PyqAttempt } from './pyqApi';
import type { RevisionStats } from './revisionApi';
import type { Settings } from './settingsApi';
import type { SyllabusSummary } from './syllabusApi';

export const DASHBOARD_SECTION_KEYS = [
  'exam-countdown',
  'syllabus-progress',
  'study-totals',
  'today-study',
  'revision-status',
  'deadlines-blocks',
  'recent-pyq',
  'latest-mock',
  'insights',
] as const;

export type DashboardSectionKey = (typeof DASHBOARD_SECTION_KEYS)[number];

export type DashboardSection = {
  key: DashboardSectionKey;
  visible: boolean;
  displayOrder: number;
};

export type DashboardLayout = {
  sections: DashboardSection[];
};

export type DashboardSectionInput = {
  key: DashboardSectionKey;
  visible: boolean;
  displayOrder: number;
};

export type InsightMetric = {
  label: string;
  value: string;
};

export type ActionableInsight = {
  id: string;
  severity: 'info' | 'warning' | 'critical' | string;
  title: string;
  reason: string;
  metrics: InsightMetric[];
  targetView: string;
  targetLabel: string;
};

export type PreparationDashboard = {
  today: string;
  settings: Settings;
  preparationDayNumber: number;
  preparationElapsedDays: number;
  daysUntilExam: number | null;
  syllabus: SyllabusSummary;
  revision: RevisionStats;
  dailyRevisionTarget: number;
  totalStudySeconds: number;
  todayStudySeconds: number;
  todaySessionCount: number;
  weekStart: string;
  weekEnd: string;
  weekStudySeconds: number;
  monthStart: string;
  monthStudySeconds: number;
  dailyStudyTargetMinutes: number;
  todayTargetPercent: number;
  upcomingDeadlines: ChecklistTask[];
  upcomingBlocks: PlannedBlock[];
  overdueTaskCount: number;
  recentPyq: PyqAttempt[];
  latestMock: MockTest | null;
  insights: ActionableInsight[];
  layout: DashboardLayout;
};

export type StudyTrendDay = {
  date: string;
  seconds: number;
  sessionCount: number;
  eligible: boolean;
  targetSeconds: number;
  targetMet: boolean;
};

export type StudyTrendWeek = {
  weekStart: string;
  weekEnd: string;
  seconds: number;
  sessionCount: number;
  eligibleDays: number;
  targetSeconds: number;
  targetMet: boolean;
};

export type StudyTrendMonth = {
  month: string;
  monthStart: string;
  monthEnd: string;
  seconds: number;
  sessionCount: number;
};

export type DurationBucket = {
  key: string;
  label: string;
  minimumSeconds: number;
  maximumSeconds: number | null;
  sessionCount: number;
  seconds: number;
};

export type UnitStudyTime = {
  sectionCode: string;
  sectionTitle: string;
  seconds: number;
  sessionCount: number;
};

export type PreviousPeriodStudy = {
  fromDate: string;
  toDate: string;
  seconds: number;
  sessionCount: number;
  activeDays: number;
  eligibleDays: number;
  targetMetDays: number;
  targetCompletionRate: number | null;
  secondsChangePercent: number | null;
};

export type RevisionConsistency = {
  completedInRange: number;
  onTimeCount: number;
  lateCount: number;
  onTimeRate: number | null;
  activeScheduledCount: number;
  currentlyOverdueCount: number;
  note: string;
};

export type StudyAnalytics = {
  fromDate: string;
  toDate: string;
  today: string;
  preparationStartDate: string;
  dailyTargetMinutes: number;
  dailyTargetSeconds: number;
  rangeSeconds: number;
  rangeSessionCount: number;
  totalRecordedSeconds: number;
  totalRecordedSessions: number;
  eligibleDays: number;
  activeDays: number;
  consistentDaysPercent: number | null;
  targetMetDays: number;
  missedTargetDays: number;
  targetCompletionRate: number | null;
  totalActiveDays: number;
  currentStreak: number;
  longestStreak: number;
  daily: StudyTrendDay[];
  weekly: StudyTrendWeek[];
  monthly: StudyTrendMonth[];
  weeklyTargetsMet: number;
  weeklyTargetsTotal: number;
  weeklyTargetCompletionRate: number | null;
  durationBuckets: DurationBucket[];
  byUnit: UnitStudyTime[];
  unassignedSeconds: number;
  unassignedSessionCount: number;
  focusSeconds: number;
  breakSeconds: number | null;
  breakTrackingSupported: boolean;
  previousPeriod: PreviousPeriodStudy;
  revisionConsistency: RevisionConsistency;
};

export type TopicIntelligenceThresholds = {
  minimumScoredAttempts: number;
  weakAccuracyThreshold: number;
  strongAccuracyThreshold: number;
  highConfidenceMinimum: number;
  lowConfidenceMaximum: number;
};

export type TopicEvidence = {
  topicId: number;
  code: string;
  title: string;
  sectionCode: string;
  sectionTitle: string;
  syllabusStatus: string;
  confidence: number;
  revisionStatus: string | null;
  dueState: 'not_scheduled' | 'suspended' | 'overdue' | 'due_today' | 'upcoming';
  nextRevisionDate: string | null;
  daysOverdue: number;
  daysUntilDue: number | null;
  completedRevisions: number;
  previousOutcome: string | null;
  masteryEstimate: number | null;
  pyqCorrect: number;
  pyqIncorrect: number;
  pyqSkipped: number;
  pyqAttempts: number;
  pyqAccuracy: number | null;
  sufficientData: boolean;
  reasons: string[];
};

export type TopicIntelligence = {
  thresholds: TopicIntelligenceThresholds;
  totalTopics: number;
  scoredTopics: number;
  noDataTopics: number;
  strong: TopicEvidence[];
  weak: TopicEvidence[];
  needsRevision: TopicEvidence[];
  insufficient: TopicEvidence[];
  overdueRevision: TopicEvidence[];
  highConfidencePoorPyq: TopicEvidence[];
  lowConfidenceStrongPyq: TopicEvidence[];
  notes: string[];
};

export type TimelineEvent = {
  id: string;
  date: string;
  kind: 'exam' | 'deadline' | 'planned_block' | 'revision' | 'study_session' | 'mock_test' | string;
  title: string;
  subtitle: string;
  status: string;
  referenceId: number | null;
  topicCode: string | null;
  durationSeconds: number | null;
  calculatedPercentage: number | null;
};

export type PreparationTimeline = {
  fromDate: string;
  toDate: string;
  today: string;
  events: TimelineEvent[];
  notes: string[];
};

export function getPreparationDashboard(): Promise<PreparationDashboard> {
  return invoke<PreparationDashboard>('get_preparation_dashboard');
}

export function updateDashboardLayout(sections: DashboardSectionInput[]): Promise<DashboardLayout> {
  return invoke<DashboardLayout>('update_dashboard_layout', { sections });
}

export function resetDashboardLayout(): Promise<DashboardLayout> {
  return invoke<DashboardLayout>('reset_dashboard_layout');
}

export function getStudyAnalytics(fromDate: string, toDate: string): Promise<StudyAnalytics> {
  return invoke<StudyAnalytics>('get_study_analytics', { fromDate, toDate });
}

export function getTopicIntelligence(): Promise<TopicIntelligence> {
  return invoke<TopicIntelligence>('get_topic_intelligence');
}

export function getPreparationTimeline(
  fromDate: string,
  toDate: string,
): Promise<PreparationTimeline> {
  return invoke<PreparationTimeline>('get_preparation_timeline', { fromDate, toDate });
}
