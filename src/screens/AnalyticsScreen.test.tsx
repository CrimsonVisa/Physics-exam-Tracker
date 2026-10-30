import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MockAnalytics, PyqAnalytics, PerformanceAnalytics } from '../lib/analyticsApi';
import type {
  PreparationTimeline,
  StudyAnalytics,
  TopicEvidence,
  TopicIntelligence,
} from '../lib/preparationApi';
import AnalyticsScreen from './AnalyticsScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';
const EXAM = '2026-12-19';

const PYQ_TOPIC = {
  topicId: 5,
  code: 'MMP-C01',
  title: 'Dimensional analysis',
  archived: false,
  sectionCode: 'MMP',
  sectionTitle: 'Mathematical Methods of Physics',
  correct: 2,
  incorrect: 4,
  skipped: 0,
  attempts: 6,
  accuracy: 33.33333333333333,
  sufficientData: true,
};

const PYQ_ANALYTICS: PyqAnalytics = {
  totalRecords: 9,
  correct: 4,
  incorrect: 4,
  skipped: 1,
  outcomeUnknown: 0,
  attempts: 8,
  accuracy: 50,
  minimumWeakAttempts: 5,
  weakAccuracyThreshold: 50,
  byTopic: [
    PYQ_TOPIC,
    {
      topicId: 1,
      code: 'GA-01',
      title: 'Logical reasoning',
      archived: false,
      sectionCode: 'GA',
      sectionTitle: 'General Aptitude',
      correct: 1,
      incorrect: 1,
      skipped: 1,
      attempts: 2,
      accuracy: 50,
      sufficientData: false,
    },
  ],
  byUnit: [
    {
      code: 'MMP',
      title: 'Mathematical Methods of Physics',
      correct: 2,
      incorrect: 4,
      skipped: 0,
      attempts: 6,
      accuracy: 33.33333333333333,
    },
    {
      code: 'GA',
      title: 'General Aptitude',
      correct: 1,
      incorrect: 1,
      skipped: 1,
      attempts: 2,
      accuracy: 50,
    },
  ],
  byYear: [
    {
      year: 2024,
      correct: 4,
      incorrect: 4,
      skipped: 1,
      attempts: 8,
      accuracy: 50,
    },
  ],
  byPart: [
    {
      part: 'A',
      correct: 1,
      incorrect: 2,
      skipped: 0,
      attempts: 3,
      accuracy: 33.33333333333333,
    },
    {
      part: 'B',
      correct: 3,
      incorrect: 2,
      skipped: 1,
      attempts: 5,
      accuracy: 60,
    },
  ],
  weakTopics: [PYQ_TOPIC],
  responseTrend: [
    { date: '2026-10-29', correct: 1, incorrect: 2, skipped: 0, outcomeUnknown: 0 },
    { date: '2026-10-30', correct: 3, incorrect: 2, skipped: 1, outcomeUnknown: 0 },
  ],
};

const MOCK_ANALYTICS: MockAnalytics = {
  testCount: 2,
  averagePercentage: 60,
  bestPercentage: 70,
  latestPercentage: 70,
  totalQuestions: 30,
  totalAttempted: 28,
  totalCorrect: 20,
  totalIncorrect: 8,
  totalUnanswered: 2,
  trend: [
    {
      id: 1,
      testName: 'Mock A',
      testDate: '2026-10-01',
      maxMarks: 100,
      computedMarks: 50,
      computedPercentage: 50,
      reportedMarks: null,
    },
    {
      id: 2,
      testName: 'Mock B',
      testDate: '2026-10-15',
      maxMarks: 100,
      computedMarks: 70,
      computedPercentage: 70,
      reportedMarks: 71,
    },
  ],
  byPart: [
    {
      part: 'A',
      questions: 10,
      attempted: 8,
      correct: 6,
      incorrect: 2,
      unanswered: 2,
      accuracy: 75,
    },
    {
      part: 'B',
      questions: 20,
      attempted: 20,
      correct: 14,
      incorrect: 6,
      unanswered: 0,
      accuracy: 70,
    },
    {
      part: 'C',
      questions: 0,
      attempted: 0,
      correct: 0,
      incorrect: 0,
      unanswered: 0,
      accuracy: null,
    },
  ],
  responseTrend: [
    {
      date: '2026-10-01',
      testCount: 1,
      correct: 10,
      incorrect: 5,
      unanswered: 1,
      averagePercentage: 50,
    },
    {
      date: '2026-10-15',
      testCount: 1,
      correct: 10,
      incorrect: 3,
      unanswered: 1,
      averagePercentage: 70,
    },
  ],
};

const PERFORMANCE: PerformanceAnalytics = {
  fromDate: '2026-10-03',
  toDate: TODAY,
  pyq: PYQ_ANALYTICS,
  mock: MOCK_ANALYTICS,
  separationNote:
    'PYQ and mock-test results are calculated separately. No combined score is produced because they measure different records and marking rules.',
};

const STUDY: StudyAnalytics = {
  fromDate: '2026-10-03',
  toDate: TODAY,
  today: TODAY,
  preparationStartDate: '2026-10-01',
  dailyTargetMinutes: 60,
  dailyTargetSeconds: 3_600,
  rangeSeconds: 10_800,
  rangeSessionCount: 5,
  totalRecordedSeconds: 18_000,
  totalRecordedSessions: 8,
  eligibleDays: 28,
  activeDays: 4,
  consistentDaysPercent: 14.3,
  targetMetDays: 2,
  missedTargetDays: 25,
  targetCompletionRate: 7.1,
  totalActiveDays: 4,
  currentStreak: 2,
  longestStreak: 3,
  daily: [
    {
      date: '2026-10-29',
      seconds: 3_600,
      sessionCount: 2,
      eligible: true,
      targetSeconds: 3_600,
      targetMet: true,
    },
    {
      date: '2026-10-30',
      seconds: 1_800,
      sessionCount: 1,
      eligible: true,
      targetSeconds: 3_600,
      targetMet: false,
    },
  ],
  weekly: [
    {
      weekStart: '2026-10-26',
      weekEnd: '2026-11-01',
      seconds: 10_800,
      sessionCount: 5,
      eligibleDays: 5,
      targetSeconds: 18_000,
      targetMet: false,
    },
  ],
  monthly: [
    {
      month: '2026-10',
      monthStart: '2026-10-01',
      monthEnd: '2026-10-31',
      seconds: 10_800,
      sessionCount: 5,
    },
  ],
  weeklyTargetsMet: 1,
  weeklyTargetsTotal: 2,
  weeklyTargetCompletionRate: 50,
  durationBuckets: [
    {
      key: 'under_15',
      label: 'Under 15 minutes',
      minimumSeconds: 1,
      maximumSeconds: 899,
      sessionCount: 0,
      seconds: 0,
    },
    {
      key: '15_to_29',
      label: '15-29 minutes',
      minimumSeconds: 900,
      maximumSeconds: 1_799,
      sessionCount: 1,
      seconds: 1_800,
    },
    {
      key: '30_to_59',
      label: '30-59 minutes',
      minimumSeconds: 1_800,
      maximumSeconds: 3_599,
      sessionCount: 2,
      seconds: 3_600,
    },
    {
      key: '60_to_89',
      label: '60-89 minutes',
      minimumSeconds: 3_600,
      maximumSeconds: 5_399,
      sessionCount: 2,
      seconds: 7_200,
    },
    {
      key: '90_to_119',
      label: '90-119 minutes',
      minimumSeconds: 5_400,
      maximumSeconds: 7_199,
      sessionCount: 0,
      seconds: 0,
    },
    {
      key: '120_plus',
      label: '120 minutes or more',
      minimumSeconds: 7_200,
      maximumSeconds: null,
      sessionCount: 0,
      seconds: 0,
    },
  ],
  byUnit: [
    {
      sectionCode: 'MMP',
      sectionTitle: 'Mathematical Methods of Physics',
      seconds: 9_000,
      sessionCount: 4,
    },
    {
      sectionCode: 'GA',
      sectionTitle: 'General Aptitude',
      seconds: 1_800,
      sessionCount: 1,
    },
  ],
  unassignedSeconds: 0,
  unassignedSessionCount: 0,
  focusSeconds: 10_800,
  breakSeconds: null,
  breakTrackingSupported: false,
  previousPeriod: {
    fromDate: '2026-09-05',
    toDate: '2026-10-02',
    seconds: 7_200,
    sessionCount: 4,
    activeDays: 3,
    eligibleDays: 2,
    targetMetDays: 1,
    targetCompletionRate: 50,
    secondsChangePercent: 50,
  },
  revisionConsistency: {
    completedInRange: 3,
    onTimeCount: 2,
    lateCount: 1,
    onTimeRate: 66.7,
    activeScheduledCount: 6,
    currentlyOverdueCount: 2,
    note: 'Revision consistency counts completed revisions on or before their scheduled date.',
  },
};

const EVIDENCE_BASE: TopicEvidence = {
  topicId: 5,
  code: 'MMP-C01',
  title: 'Dimensional analysis',
  sectionCode: 'MMP',
  sectionTitle: 'Mathematical Methods of Physics',
  syllabusStatus: 'Confident',
  confidence: 5,
  revisionStatus: 'active',
  dueState: 'overdue',
  nextRevisionDate: '2026-10-28',
  daysOverdue: 2,
  daysUntilDue: null,
  completedRevisions: 2,
  previousOutcome: 'difficult',
  masteryEstimate: 42.5,
  pyqCorrect: 2,
  pyqIncorrect: 4,
  pyqSkipped: 0,
  pyqAttempts: 6,
  pyqAccuracy: 33.3,
  sufficientData: true,
  reasons: [
    'PYQ accuracy 33.3% across 6 scored attempts (weak threshold: below 50% with at least 5 attempts).',
    'Revision is 2 days overdue.',
  ],
};

const TOPICS: TopicIntelligence = {
  thresholds: {
    minimumScoredAttempts: 5,
    weakAccuracyThreshold: 50,
    strongAccuracyThreshold: 75,
    highConfidenceMinimum: 4,
    lowConfidenceMaximum: 2,
  },
  totalTopics: 182,
  scoredTopics: 4,
  noDataTopics: 178,
  strong: [
    {
      ...EVIDENCE_BASE,
      topicId: 1,
      code: 'GA-01',
      title: 'Logical reasoning',
      sectionTitle: 'General Aptitude',
      syllabusStatus: 'Learning',
      confidence: 2,
      dueState: 'not_scheduled',
      nextRevisionDate: null,
      completedRevisions: 0,
      previousOutcome: null,
      masteryEstimate: null,
      pyqCorrect: 5,
      pyqIncorrect: 0,
      pyqAttempts: 5,
      pyqAccuracy: 100,
      reasons: ['PYQ accuracy 100.0% across 5 scored attempts.'],
    },
  ],
  weak: [EVIDENCE_BASE],
  needsRevision: [EVIDENCE_BASE],
  insufficient: [
    {
      ...EVIDENCE_BASE,
      topicId: 2,
      code: 'CM-C01',
      title: 'Newtonian mechanics',
      confidence: 3,
      dueState: 'not_scheduled',
      nextRevisionDate: null,
      completedRevisions: 0,
      previousOutcome: null,
      masteryEstimate: null,
      pyqCorrect: 0,
      pyqIncorrect: 3,
      pyqAttempts: 3,
      pyqAccuracy: 0,
      sufficientData: false,
      reasons: [
        'Only 3 scored PYQ attempts; at least 5 are required before a weak or strong claim.',
      ],
    },
  ],
  overdueRevision: [EVIDENCE_BASE],
  highConfidencePoorPyq: [EVIDENCE_BASE],
  lowConfidenceStrongPyq: [
    {
      ...EVIDENCE_BASE,
      topicId: 1,
      code: 'GA-01',
      confidence: 1,
      pyqAccuracy: 100,
      reasons: ['Confidence is 1/5 but PYQ accuracy is 100.0% across 5 scored attempts.'],
    },
  ],
  notes: [
    'Topic intelligence uses saved scored PYQ attempts and revision records only.',
    'A topic has sufficient performance evidence at five scored attempts.',
  ],
};

const TIMELINE: PreparationTimeline = {
  fromDate: '2026-09-30',
  toDate: '2026-12-19',
  today: TODAY,
  events: [
    {
      id: 'exam:target',
      date: '2026-12-19',
      kind: 'exam',
      title: 'Target exam date',
      subtitle: 'Configured in Settings.',
      status: 'upcoming',
      referenceId: null,
      topicCode: null,
      durationSeconds: null,
      calculatedPercentage: null,
    },
    {
      id: 'deadline:1',
      date: '2026-11-02',
      kind: 'deadline',
      title: 'Submit revision plan',
      subtitle: 'High deadline',
      status: 'open',
      referenceId: 1,
      topicCode: 'MMP-C01',
      durationSeconds: null,
      calculatedPercentage: null,
    },
    {
      id: 'session:1',
      date: TODAY,
      kind: 'study_session',
      title: 'MMP-C01 study session',
      subtitle: 'Dimensional analysis',
      status: 'completed',
      referenceId: 1,
      topicCode: 'MMP-C01',
      durationSeconds: 1_800,
      calculatedPercentage: null,
    },
    {
      id: 'mock:1',
      date: TODAY,
      kind: 'mock_test',
      title: 'Mock A',
      subtitle: 'Calculated 60.0%',
      status: 'completed',
      referenceId: 1,
      topicCode: null,
      durationSeconds: null,
      calculatedPercentage: 60,
    },
  ],
  notes: ['Timeline entries are read-only projections of saved records.'],
};

function mockBackend() {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'get_performance_analytics') {
      return Promise.resolve({
        ...PERFORMANCE,
        fromDate: String(args?.fromDate ?? PERFORMANCE.fromDate),
        toDate: String(args?.toDate ?? PERFORMANCE.toDate),
      });
    }
    if (command === 'get_study_analytics') {
      return Promise.resolve({
        ...STUDY,
        fromDate: String(args?.fromDate ?? STUDY.fromDate),
        toDate: String(args?.toDate ?? STUDY.toDate),
      });
    }
    if (command === 'get_topic_intelligence') {
      return Promise.resolve(TOPICS);
    }
    if (command === 'get_preparation_timeline') {
      return Promise.resolve({
        ...TIMELINE,
        fromDate: String(args?.fromDate ?? TIMELINE.fromDate),
        toDate: String(args?.toDate ?? TIMELINE.toDate),
      });
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

describe('analytics screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders study trends, targets, comparison, streaks, duration, units, and revision consistency', async () => {
    mockBackend();

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);

    expect(await screen.findByText('Recorded study time')).toBeInTheDocument();
    expect(screen.getByText('Daily target completion')).toBeInTheDocument();
    expect(screen.getByText('Active-day consistency')).toBeInTheDocument();
    expect(screen.getByText('Current study streak')).toBeInTheDocument();
    expect(screen.getByText('Current versus previous period')).toBeInTheDocument();
    expect(screen.getByText('Study time by day')).toBeInTheDocument();
    expect(screen.getByText('Weekly study time')).toBeInTheDocument();
    expect(screen.getByText('Monthly study time')).toBeInTheDocument();
    expect(screen.getByText('Sessions by duration')).toBeInTheDocument();
    expect(screen.getByText('Study time by unit')).toBeInTheDocument();
    expect(screen.getByText('Focus versus break time')).toBeInTheDocument();
    expect(screen.getByText('Not tracked')).toBeInTheDocument();
    expect(screen.getByText('Revision consistency')).toBeInTheDocument();
    expect(screen.getByText('+50.0%')).toBeInTheDocument();
    expect(screen.getAllByText('60-89 minutes').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Mathematical Methods of Physics').length).toBeGreaterThan(0);
    expect(screen.queryByText(/No study sessions in this range/)).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('From date'), { target: { value: '2026-10-24' } });
    await userEvent.click(screen.getByRole('button', { name: 'Last 7 days' }));

    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_study_analytics');
      expect(calls.at(-1)?.[1]).toEqual({
        fromDate: '2026-10-24',
        toDate: TODAY,
      });
    });
  });

  it('keeps PYQ and mock-test summaries separate and shows response and marking data', async () => {
    mockBackend();

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    await screen.findByText('Recorded study time');

    await userEvent.click(screen.getByRole('tab', { name: /Exam performance/ }));

    expect(await screen.findByText('PYQ records')).toBeInTheDocument();
    expect(screen.getByText(/No combined score is produced/)).toBeInTheDocument();
    expect(screen.getByText('PYQ accuracy by topic')).toBeInTheDocument();
    expect(screen.getByText('Performance by examination year')).toBeInTheDocument();
    expect(screen.getByText('Part A, B and C performance')).toBeInTheDocument();
    expect(screen.getAllByText('Weak').length).toBeGreaterThan(0);
    expect(screen.getByText('Mock-test score trend')).toBeInTheDocument();
    expect(screen.getByText('Attempted / unanswered')).toBeInTheDocument();
    expect(screen.getByText('28 / 2')).toBeInTheDocument();
    expect(screen.getByText('Part-wise results')).toBeInTheDocument();
    expect(screen.getByText('Response trend by test date')).toBeInTheDocument();
    expect(screen.getAllByText('Mock B').length).toBeGreaterThan(0);
    expect(screen.getByText('Calculated from saved marking rules')).toBeInTheDocument();
  });

  it('shows evidence groups and reasons for topic intelligence without unsupported claims', async () => {
    mockBackend();

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    await screen.findByText('Recorded study time');

    await userEvent.click(screen.getByRole('tab', { name: /Topic intelligence/ }));

    expect(await screen.findByText('Evidence thresholds')).toBeInTheDocument();
    expect(screen.getByText('Weak topics with sufficient evidence')).toBeInTheDocument();
    expect(screen.getByText('Revision overdue')).toBeInTheDocument();
    expect(
      screen.getAllByText(/PYQ accuracy 33.3% across 6 scored attempts/).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText(/Revision is 2 days overdue/).length).toBeGreaterThan(0);
    expect(
      screen.getAllByText(/Only 3 scored PYQ attempts; at least 5 are required/).length,
    ).toBeGreaterThan(0);
    expect(screen.getByText('High confidence, weak PYQ evidence')).toBeInTheDocument();
    expect(screen.getByText('Low confidence, strong PYQ evidence')).toBeInTheDocument();
    expect(screen.getByText('No scored PYQ data')).toBeInTheDocument();
    expect(
      screen.getByText(/Topic intelligence uses saved scored PYQ attempts/),
    ).toBeInTheDocument();
  });

  it('filters timeline events by activity type and reloads a selected date range', async () => {
    mockBackend();

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    await screen.findByText('Recorded study time');

    await userEvent.click(screen.getByRole('tab', { name: /Preparation timeline/ }));

    expect(await screen.findByText('Timeline activity filter')).toBeInTheDocument();
    expect(await screen.findByText('Target exam date')).toBeInTheDocument();
    expect(screen.getByText('Submit revision plan')).toBeInTheDocument();
    expect(screen.getByText('MMP-C01 study session')).toBeInTheDocument();
    expect(screen.getByText('Mock A')).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText('Mock tests'));
    await waitFor(() => expect(screen.queryByText('Mock A')).not.toBeInTheDocument());

    fireEvent.change(screen.getByLabelText('Timeline from'), {
      target: { value: '2026-10-01' },
    });
    fireEvent.change(screen.getByLabelText('Timeline to'), {
      target: { value: '2026-12-31' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Apply timeline range' }));

    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(
        ([command]) => command === 'get_preparation_timeline',
      );
      expect(calls.at(-1)?.[1]).toEqual({
        fromDate: '2026-10-01',
        toDate: '2026-12-31',
      });
    });
  });

  it('shows loading, empty, validation, and retry states without inventing data', async () => {
    invokeMock.mockReturnValue(new Promise(() => {}));
    const loadingView = render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    expect(screen.getByRole('status')).toHaveTextContent('Calculating study analytics');
    loadingView.unmount();

    invokeMock.mockReset();
    mockBackend();
    const validView = render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    await screen.findByText('Recorded study time');

    fireEvent.change(screen.getByLabelText('From date'), {
      target: { value: '2026-11-01' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Apply range' }));
    expect(
      await screen.findByText('The start date cannot be after the end date.'),
    ).toBeInTheDocument();
    validView.unmount();

    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_performance_analytics') {
        return Promise.resolve({
          ...PERFORMANCE,
          pyq: { ...PYQ_ANALYTICS, totalRecords: 0, attempts: 0, accuracy: null, byTopic: [] },
          mock: {
            ...MOCK_ANALYTICS,
            testCount: 0,
            averagePercentage: null,
            bestPercentage: null,
            latestPercentage: null,
            trend: [],
            byPart: [],
            responseTrend: [],
          },
        });
      }
      if (command === 'get_study_analytics') {
        return Promise.resolve({
          ...STUDY,
          rangeSeconds: 0,
          rangeSessionCount: 0,
          daily: [],
          weekly: [],
          monthly: [],
          durationBuckets: [],
          byUnit: [],
          activeDays: 0,
          currentStreak: 0,
          longestStreak: 0,
        });
      }
      if (command === 'get_topic_intelligence') {
        return Promise.resolve(TOPICS);
      }
      if (command === 'get_preparation_timeline') {
        return Promise.resolve({ ...TIMELINE, events: [] });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);
    expect(await screen.findByText(/No study sessions in this range/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('tab', { name: /Exam performance/ }));
    expect(await screen.findByText('No PYQ attempts in this date range.')).toBeInTheDocument();
    expect(screen.getByText('No mock tests in this date range.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('tab', { name: /Preparation timeline/ }));
    expect(
      await screen.findByText('No timeline events match this range and activity filter.'),
    ).toBeInTheDocument();
  });

  it('shows a load error and retries analytics', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_performance_analytics') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve(PERFORMANCE);
      }
      if (command === 'get_study_analytics') {
        return Promise.resolve(STUDY);
      }
      if (command === 'get_topic_intelligence') {
        return Promise.resolve(TOPICS);
      }
      if (command === 'get_preparation_timeline') {
        return Promise.resolve(TIMELINE);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<AnalyticsScreen today={TODAY} examDate={EXAM} />);

    expect(await screen.findByText('Analytics could not be loaded')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry analytics' }));
    expect(await screen.findByText('Recorded study time')).toBeInTheDocument();
    await waitFor(() => expect(attempts).toBe(2));
  });
});
