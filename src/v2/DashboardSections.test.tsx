import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  V2ExamSyllabus,
  V2ExamSummary,
  V2PlannerBlock,
  V2PlannerDay,
  V2PlannerTask,
  V2PracticeAnalytics,
  V2RevisionDashboard,
  V2RevisionHistoryEntry,
  V2RevisionSchedule,
  V2StudySession,
  V2StudyTotals,
  V2TimerSnapshot,
} from './api';
import { TIMER_UPDATED_EVENT } from '../lib/timerEvents';
import {
  useDailyDashboardData,
  type DailyDashboardData,
  latestMock,
  recentStudyConfigs,
  recentTopics,
  summarizePlannerDay,
  summarizeTodaySessions,
  weakestTopics,
} from './dailyDashboardData';
import {
  PerformanceSection,
  QuickStartSection,
  RecentlyStudiedSection,
  RevisionOverviewSection,
  TodayOverviewSection,
  TodayPlanSection,
} from './DashboardSections';
import { StudySection } from './StudySection';
import { TimerProvider } from './timer/TimerProvider';
import { useV2Timer } from './timer/timerContext';

function MutateTimerButton() {
  const { runMutation } = useV2Timer();
  return (
    <button
      onClick={() => void runMutation(() => Promise.resolve(timerSnapshot({ status: 'paused' })))}
      type="button"
    >
      Mutate timer
    </button>
  );
}

function QuickStartHarness(props: {
  exams: V2ExamSummary[];
  sessions: V2StudySession[];
  today: string;
}) {
  return (
    <TimerProvider>
      <QuickStartSection {...props} />
    </TimerProvider>
  );
}

vi.mock('./api', () => ({
  getV2Timer: vi.fn(),
  startV2Timer: vi.fn(),
  pauseV2Timer: vi.fn(),
  resumeV2Timer: vi.fn(),
  finishV2Timer: vi.fn(),
  discardV2Timer: vi.fn(),
  heartbeatV2Timer: vi.fn(),
  setV2TimerAllocations: vi.fn(),
  updateV2TimerSettings: vi.fn(),
  listV2Sessions: vi.fn(),
  getV2ExamSyllabus: vi.fn(),
  getV2PlannerDay: vi.fn(),
  getV2RevisionDashboard: vi.fn(),
  getV2AppSettings: vi.fn(),
  getV2StudyTotals: vi.fn(),
  getV2PracticeAnalytics: vi.fn(),
  listV2QuickStartConfigs: vi.fn(),
  saveV2QuickStartConfig: vi.fn(),
  deleteV2QuickStartConfig: vi.fn(),
}));

import * as api from './api';

const mockedTimer = vi.mocked(api.getV2Timer);
const mockedStartTimer = vi.mocked(api.startV2Timer);
const mockedPause = vi.mocked(api.pauseV2Timer);
const mockedSaveQuickStart = vi.mocked(api.saveV2QuickStartConfig);
const mockedDeleteQuickStart = vi.mocked(api.deleteV2QuickStartConfig);
const mockedQuickStartList = vi.mocked(api.listV2QuickStartConfigs);
const mockedSyllabus = vi.mocked(api.getV2ExamSyllabus);
const mockedSessions = vi.mocked(api.listV2Sessions);
const mockedPlannerDay = vi.mocked(api.getV2PlannerDay);
const mockedRevision = vi.mocked(api.getV2RevisionDashboard);
const mockedSettings = vi.mocked(api.getV2AppSettings);
const mockedTotals = vi.mocked(api.getV2StudyTotals);
const mockedPracticeAnalytics = vi.mocked(api.getV2PracticeAnalytics);

const TODAY = '2026-11-06';
const PAST = '2026-11-05';

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: '',
    officialUrl: '',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
  {
    id: 2,
    code: 'gate-physics',
    name: 'GATE Physics',
    shortName: 'GATE PH',
    authority: '',
    officialUrl: '',
    displayOrder: 2,
    isActive: true,
    examDate: null,
  },
];

function studySession(overrides: Partial<V2StudySession> = {}): V2StudySession {
  return {
    id: 1,
    sessionDate: TODAY,
    startedAt: null,
    endedAt: null,
    durationSeconds: 1_800,
    source: 'timer',
    notes: '',
    primaryExamId: null,
    allocatedSeconds: 1_800,
    unallocatedSeconds: 0,
    allocations: [],
    topicAllocations: [],
    createdAt: '2026-11-06T09:00:00Z',
    updatedAt: '2026-11-06T09:30:00Z',
    ...overrides,
  };
}

function plannerDay(overrides: Partial<V2PlannerDay> = {}): V2PlannerDay {
  return {
    date: TODAY,
    tasks: [],
    blocks: [],
    ...overrides,
  };
}

function plannerTask(overrides: Partial<V2PlannerTask> = {}): V2PlannerTask {
  return {
    id: 1,
    title: 'Planner task',
    description: '',
    scheduledDate: TODAY,
    deadlineDate: null,
    examId: null,
    examCode: null,
    conceptId: null,
    conceptTitle: null,
    syllabusNodeId: null,
    priority: 'normal',
    status: 'open',
    scheduleAnchor: 'manual',
    scheduleOffsetDays: null,
    completedAt: null,
    linkedSessionId: null,
    createdAt: '',
    updatedAt: '',
    ...overrides,
  };
}

function plannerBlock(overrides: Partial<V2PlannerBlock> = {}): V2PlannerBlock {
  return {
    id: 1,
    title: 'Study block',
    examId: null,
    examCode: null,
    conceptId: null,
    conceptTitle: null,
    syllabusNodeId: null,
    calendarDate: TODAY,
    startTime: '09:00',
    endTime: '10:00',
    status: 'planned',
    notes: '',
    linkedSessionId: null,
    createdAt: '',
    updatedAt: '',
    ...overrides,
  };
}

function revisionSchedule(overrides: Partial<V2RevisionSchedule> = {}): V2RevisionSchedule {
  return {
    conceptId: 1,
    conceptSlug: 'kinematics',
    conceptTitle: 'Kinematics',
    conceptDomain: 'Classical Mechanics',
    status: 'active',
    lastRevisionDate: null,
    nextRevisionDate: TODAY,
    currentIntervalDays: 7,
    completedRevisions: 3,
    previousOutcome: null,
    notes: '',
    overdueDays: 0,
    masteryScore: null,
    selectionReason: 'Due by interval',
    ...overrides,
  };
}

function revisionHistoryEntry(
  overrides: Partial<V2RevisionHistoryEntry> = {},
): V2RevisionHistoryEntry {
  return {
    id: 1,
    conceptId: 1,
    conceptTitle: 'Kinematics',
    triggerExamId: null,
    revisionDate: TODAY,
    scheduledForDate: TODAY,
    outcome: 'good',
    notes: '',
    intervalBeforeDays: 7,
    intervalAfterDays: 14,
    masteryBefore: 40,
    masteryAfter: 55,
    ...overrides,
  };
}

function revisionDashboard(overrides: Partial<V2RevisionDashboard> = {}): V2RevisionDashboard {
  return {
    settings: {
      initialAgainDays: 1,
      initialDifficultDays: 2,
      initialGoodDays: 4,
      initialEasyDays: 7,
      growthMultiplier: 2,
      minimumIntervalDays: 1,
      maximumIntervalDays: 180,
      includeWeekends: true,
      dailyRevisionTarget: 5,
      remindersEnabled: false,
    },
    dueToday: [],
    overdue: [],
    upcoming: [],
    recentHistory: [],
    stats: {
      dueToday: 0,
      overdue: 0,
      completedToday: 0,
      completedLast7Days: 0,
      totalCompleted: 0,
    },
    examId: null,
    ...overrides,
  };
}

function studyTotals(topics: Array<Partial<V2StudyTotals['topics'][number]>> = []): V2StudyTotals {
  return {
    totalSeconds: 0,
    sessionCount: 0,
    allocatedSeconds: 0,
    unallocatedSeconds: 0,
    exams: [],
    topics: topics.map((topic, index) => ({
      examId: 1,
      examCode: 'csir-net-physics',
      nodeId: 10 + index,
      nodeTitle: `Topic ${index}`,
      subjectTitle: 'Quantum Mechanics',
      allocatedSeconds: 1_200,
      sessionCount: 1,
      lastStudiedDate: null,
      ...topic,
    })) as V2StudyTotals['topics'],
  };
}

function practiceAnalytics(overrides: Partial<V2PracticeAnalytics> = {}): V2PracticeAnalytics {
  return {
    examId: 1,
    pyqAttempts: 0,
    distinctQuestions: 0,
    correct: 0,
    incorrect: 0,
    unanswered: 0,
    accuracyPercentage: null,
    marksObtained: 0,
    maxMarksAttempted: 0,
    percentage: null,
    timeSpentSeconds: 0,
    averageSecondsPerQuestion: null,
    verifiedQuestions: 0,
    verifiedQuestionsInScope: 0,
    totalQuestionsInScope: 0,
    verificationCoveragePercentage: null,
    byYear: [],
    bySection: [],
    byTopic: [],
    pyqTrend: [],
    mocks: {
      completedCount: 0,
      averagePercentage: null,
      bestPercentage: null,
      lastPercentage: null,
      byPattern: [],
      history: [],
      insufficientData: false,
    },
    insufficientPyqData: true,
    ...overrides,
  };
}

function syllabusScaffold(examId: number, titles: string[]): V2ExamSyllabus {
  const exam = EXAMS.find((candidate) => candidate.id === examId) ?? EXAMS[0];
  return {
    examId,
    examCode: exam.code,
    examName: exam.name,
    examIsActive: exam.isActive,
    configured: true,
    contentVersionLabel: 'official 2026',
    contentReviewStatus: 'verified',
    nodes: titles.map((title, index) => ({
      id: 700 + index,
      examId,
      parentId: null,
      code: `topic-${index}`,
      title,
      nodeKind: 'topic',
      category: '',
      displayOrder: index,
      readinessWeight: 1,
      isArchived: false,
      sourceTitle: 'Synthetic syllabus',
      sourceVerifiedOn: PAST,
      sourceUrl: null,
      sourceReference: 'fixture',
      contentVersionLabel: 'official 2026',
      contentReviewStatus: 'verified',
      progressStatus: 'not_started',
      priority: 'normal',
      confidence: 0,
      notes: '',
      nodePath: [title],
    })),
    summary: {
      nodeCount: titles.length,
      topicCount: titles.length,
      revisedOrConfidentCount: 0,
      mappedConceptCount: 0,
    },
  };
}

function timerSnapshot(
  stateOverrides: Partial<V2TimerSnapshot['state']> = {},
  allocations: V2TimerSnapshot['allocations'] = [],
): V2TimerSnapshot {
  return {
    state: {
      status: 'idle',
      mode: 'focus',
      durationMs: 25 * 60_000,
      accumulatedMs: 0,
      elapsedMs: 0,
      remainingMs: 25 * 60_000,
      segmentStartedAt: null,
      sessionDate: TODAY,
      activeExamId: null,
      recoveryReason: null,
      recoveryExcludedMs: 0,
      clockSkewDetected: false,
      lastEvent: null,
      lastEventAt: null,
      lastEventDurationMs: 0,
      lastCompletedSessionId: null,
      ...stateOverrides,
    },
    settings: {
      focusMinutes: 25,
      shortBreakMinutes: 5,
      longBreakMinutes: 15,
      notificationsEnabled: false,
    },
    allocations,
    serverNowMs: 0,
  };
}

function dailyData(overrides: Partial<DailyDashboardData> = {}): DailyDashboardData {
  return {
    loading: false,
    error: null,
    retry: () => {},
    sessions: [],
    plannerDay: null,
    revision: null,
    settings: null,
    totals: null,
    ...overrides,
  };
}

function HookProbe({ today, enabled }: { today: string; enabled: boolean }) {
  const data = useDailyDashboardData(today, enabled);
  return (
    <div>
      <span aria-live="polite">{data.loading ? 'loading' : 'loaded'}</span>
      <span>{data.error ?? 'no-error'}</span>
      <span data-testid="sessions">{data.sessions.length}</span>
      <button onClick={data.retry} type="button">
        retry
      </button>
    </div>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mockedSessions.mockResolvedValue([]);
  mockedPlannerDay.mockResolvedValue(plannerDay());
  mockedRevision.mockResolvedValue(revisionDashboard());
  mockedSettings.mockResolvedValue({
    preparationStartDate: null,
    dailyStudyTargetMinutes: 120,
  });
  mockedTotals.mockResolvedValue(studyTotals());
  mockedTimer.mockResolvedValue(timerSnapshot());
  mockedQuickStartList.mockResolvedValue([]);
  mockedSaveQuickStart.mockResolvedValue([]);
  mockedDeleteQuickStart.mockResolvedValue([]);
});

describe('today summary helpers', () => {
  it('counts only sessions recorded for today, once per session', () => {
    const sessions = [
      studySession({ id: 1, durationSeconds: 2_400 }),
      studySession({ id: 2, sessionDate: PAST, durationSeconds: 1_800 }),
      studySession({ id: 3, durationSeconds: 600 }),
    ];
    const summary = summarizeTodaySessions(sessions, TODAY);
    expect(summary).toEqual({ seconds: 3_000, sessionCount: 2 });
  });

  it('never adds exam allocations on top of the session duration', () => {
    const session = studySession({
      durationSeconds: 1_800,
      allocations: [
        {
          examId: 2,
          examCode: 'gate-physics',
          examName: 'GATE Physics',
          examIsActive: true,
          allocatedSeconds: 1_800,
        },
      ],
      topicAllocations: [
        {
          examId: 2,
          examCode: 'gate-physics',
          examName: 'GATE Physics',
          nodeId: 4,
          nodeTitle: 'Vector calculus',
          subjectTitle: 'Mathematical Methods',
          allocatedSeconds: 1_800,
        },
      ],
    });
    const summary = summarizeTodaySessions([session], TODAY);
    expect(summary.seconds).toBe(1_800);
  });

  it('counts planner tasks and blocks with cancelled items separated', () => {
    const counts = summarizePlannerDay(
      plannerDay({
        tasks: [
          plannerTask({ id: 1, status: 'completed' }),
          plannerTask({ id: 2, status: 'open' }),
          plannerTask({ id: 3, status: 'open' }),
          plannerTask({ id: 4, status: 'cancelled' }),
          plannerTask({ id: 5, status: 'completed' }),
        ],
        blocks: [
          plannerBlock({ id: 1, status: 'completed' }),
          plannerBlock({ id: 2, status: 'planned' }),
          plannerBlock({ id: 3, status: 'cancelled' }),
        ],
      }),
    );
    expect(counts).toEqual({
      tasksTotal: 5,
      tasksDone: 2,
      tasksRemaining: 2,
      tasksCancelled: 1,
      blocksTotal: 3,
      blocksDone: 1,
      blocksRemaining: 1,
      blocksCancelled: 1,
    });
  });

  it('derives distinct recent study configurations newest first', () => {
    const configs = recentStudyConfigs([
      studySession({
        id: 1,
        sessionDate: PAST,
        topicAllocations: [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics',
            nodeId: 4,
            nodeTitle: 'Vector calculus',
            subjectTitle: 'Mathematical Methods',
            allocatedSeconds: 1_800,
          },
        ],
      }),
      studySession({
        id: 2,
        topicAllocations: [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics',
            nodeId: 4,
            nodeTitle: 'Vector calculus',
            subjectTitle: 'Mathematical Methods',
            allocatedSeconds: 1_800,
          },
        ],
      }),
    ]);
    expect(configs).toHaveLength(1);
    expect(configs[0]).toMatchObject({ sessionDate: TODAY, minutes: 30 });
  });

  it('orders weak topics by ascending accuracy and drops rows without accuracy', () => {
    const topics = weakestTopics(
      practiceAnalytics({
        byTopic: [
          {
            nodeId: 1,
            label: 'Homework 70',
            attempts: 4,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            accuracyPercentage: 70,
          },
          {
            nodeId: 2,
            label: 'Exam 45',
            attempts: 6,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            accuracyPercentage: 45,
          },
          {
            nodeId: 3,
            label: 'No accuracy',
            attempts: 2,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            accuracyPercentage: null,
          },
          {
            nodeId: 4,
            label: 'Exam 30',
            attempts: 3,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            accuracyPercentage: 30,
          },
          {
            nodeId: 5,
            label: 'Exam 55',
            attempts: 8,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            accuracyPercentage: 55,
          },
        ],
      }),
      3,
    );
    expect(topics.map((row) => row.label)).toEqual(['Exam 30', 'Exam 45', 'Exam 55']);
  });

  it('picks the most recent completed mock', () => {
    const mock = latestMock(
      practiceAnalytics({
        mocks: {
          completedCount: 2,
          averagePercentage: 60,
          bestPercentage: 70,
          lastPercentage: 55,
          byPattern: [],
          insufficientData: false,
          history: [
            {
              testId: 1,
              testDate: PAST,
              testName: 'Earlier mock',
              percentage: 70,
              patternVersionId: 1,
              patternVersionLabel: '2025 official',
            },
            {
              testId: 2,
              testDate: TODAY,
              testName: 'Latest mock',
              percentage: 55,
              patternVersionId: 1,
              patternVersionLabel: '2026 official',
            },
          ],
        },
      }),
    );
    expect(mock).toMatchObject({ testName: 'Latest mock', testDate: TODAY });
  });

  it('sorts recently studied topics by last studied date and drops unknown dates', () => {
    const topics = recentTopics(
      studyTotals([
        { nodeId: 1, nodeTitle: 'Old', lastStudiedDate: '2026-11-01' },
        { nodeId: 2, nodeTitle: 'Never studied', lastStudiedDate: null },
        { nodeId: 3, nodeTitle: 'Newest', lastStudiedDate: TODAY },
      ]),
    );
    expect(topics.map((topic) => topic.nodeTitle)).toEqual(['Newest', 'Old']);
  });
});

describe('TodayOverviewSection', () => {
  it('renders today totals, the daily target bar, planner and revision numbers', async () => {
    const onNavigate = vi.fn();
    render(
      <TodayOverviewSection
        data={dailyData({
          sessions: [
            studySession({ id: 1, durationSeconds: 2_400 }),
            studySession({ id: 2, sessionDate: PAST, durationSeconds: 1_200 }),
          ],
          settings: { preparationStartDate: null, dailyStudyTargetMinutes: 120 },
          plannerDay: plannerDay({
            tasks: [
              plannerTask({ id: 1, status: 'completed' }),
              plannerTask({ id: 2, status: 'open' }),
            ],
            blocks: [],
          }),
          revision: revisionDashboard({
            stats: {
              dueToday: 2,
              overdue: 1,
              completedToday: 1,
              completedLast7Days: 4,
              totalCompleted: 12,
            },
          }),
        })}
        onNavigate={onNavigate}
        today={TODAY}
      />,
    );

    expect(screen.getByText('40m')).toBeInTheDocument();
    expect(screen.getByText('1 session completed today.')).toBeInTheDocument();
    const bar = document.querySelector<HTMLInputElement>('.v2-day-target .v2-timer-progress');
    expect(bar).not.toBeNull();
    expect(bar?.dataset.progress).toBe('33');
    expect(screen.getByText('Daily target: 40 of 120 min (33% complete).')).toBeInTheDocument();
    expect(screen.getByText('1 done · 1 left')).toBeInTheDocument();
    expect(screen.getByText('2 due · 1 overdue')).toBeInTheDocument();
    expect(screen.getByText('1 completed today · 4 in the last 7 days.')).toBeInTheDocument();
    expect(onNavigate).not.toHaveBeenCalled();
  });

  it('does not count allocations twice and links to settings when no target exists', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <TodayOverviewSection
        data={dailyData({
          sessions: [
            studySession({
              durationSeconds: 1_800,
              allocations: [
                {
                  examId: 2,
                  examCode: 'gate-physics',
                  examName: 'GATE Physics',
                  examIsActive: true,
                  allocatedSeconds: 1_800,
                },
              ],
            }),
          ],
          settings: { preparationStartDate: null, dailyStudyTargetMinutes: 0 },
        })}
        onNavigate={onNavigate}
        today={TODAY}
      />,
    );
    expect(screen.getByText('30m')).toBeInTheDocument();
    expect(screen.getByText(/No daily study target is configured/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Open settings' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'settings' });
    expect(screen.getByText('No planner tasks')).toBeInTheDocument();
    expect(screen.getByText('Revision statistics are not available yet.')).toBeInTheDocument();
  });

  it('shows loading, error and retry states from the shared data set', () => {
    const retry = vi.fn();
    render(
      <TodayOverviewSection
        data={dailyData({ loading: true, error: 'Session command failed.', retry })}
        onNavigate={vi.fn()}
        today={TODAY}
      />,
    );
    expect(
      screen.getByText("Loading today's study time, planner and revision activity…"),
    ).toBeInTheDocument();
    expect(screen.getByRole('alert').textContent).toContain('Session command failed.');
  });
});

describe('TodayPlanSection', () => {
  it('lists tasks and blocks with exam chips, durations, status and navigation', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <TodayPlanSection
        data={dailyData({
          plannerDay: plannerDay({
            tasks: [
              plannerTask({
                id: 7,
                title: 'Derive Euler formulas',
                examCode: 'csir-net-physics',
                examId: 1,
                deadlineDate: '2026-11-08',
                priority: 'high',
              }),
              plannerTask({
                id: 8,
                title: 'Flash-card pass',
                status: 'completed',
                conceptTitle: 'Ring totals',
              }),
            ],
            blocks: [
              plannerBlock({
                id: 3,
                title: 'Morning block',
                examCode: 'gate-physics',
                examId: 2,
                conceptTitle: 'Ring totals',
                startTime: '09:00',
                endTime: '10:15',
              }),
            ],
          }),
        })}
        onNavigate={onNavigate}
        exams={[
          {
            id: 1,
            code: 'csir-net-physics',
            name: 'CSIR-UGC NET Physical Sciences',
            shortName: 'CSIR-NET',
            authority: '',
            officialUrl: '',
            displayOrder: 1,
            isActive: true,
            examDate: null,
          },
        ]}
      />,
    );

    expect(screen.getByText('Derive Euler formulas')).toBeInTheDocument();
    expect(screen.getByText('Morning block')).toBeInTheDocument();
    expect(screen.getByText('09:00–10:15 · 75 min · Ring totals')).toBeInTheDocument();
    expect(screen.getByText('Deadline 2026-11-08 · Priority high')).toBeInTheDocument();
    expect(screen.getAllByText('CSIR-NET').length).toBe(1);
    expect(screen.getByText('Completed')).toBeInTheDocument();
    expect(screen.getByText('Planned')).toBeInTheDocument();

    const row = document.querySelector<HTMLLIElement>('li[data-task-id="8"]');
    expect(row?.className).toContain('v2-planner-done');

    await user.click(
      screen.getByRole('button', { name: /Open the CSIR-NET workspace for Derive Euler formulas/ }),
    );
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'exam', examCode: 'csir-net-physics' });
  });

  it('navigates to the planner and shows the empty state', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(<TodayPlanSection data={dailyData()} exams={[]} onNavigate={onNavigate} />);
    expect(
      screen.getByText(/No planner tasks or blocks are scheduled for today/),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Manage planner tasks' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'planner' });
  });
});

describe('RevisionOverviewSection', () => {
  it('shows overdue, due-today and recently completed revisions with navigation', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <RevisionOverviewSection
        data={dailyData({
          revision: revisionDashboard({
            dueToday: [
              revisionSchedule({ conceptId: 1, conceptTitle: 'Kinematics', masteryScore: 41 }),
              revisionSchedule({ conceptId: 2, conceptTitle: 'Ring totals', masteryScore: null }),
              revisionSchedule({ conceptId: 3, conceptTitle: 'Condensate laws' }),
              revisionSchedule({ conceptId: 4, conceptTitle: 'Fourth due' }),
            ],
            overdue: [
              revisionSchedule({
                conceptId: 9,
                conceptTitle: 'Thermodynamics second law',
                nextRevisionDate: '2026-11-03',
                overdueDays: 2,
                masteryScore: 38,
              }),
            ],
            recentHistory: [
              revisionHistoryEntry({
                id: 5,
                conceptTitle: 'Thermodynamics second law',
                outcome: 'good',
              }),
            ],
            stats: {
              dueToday: 4,
              overdue: 1,
              completedToday: 1,
              completedLast7Days: 4,
              totalCompleted: 12,
            },
          }),
        })}
        onNavigate={onNavigate}
      />,
    );

    expect(screen.getAllByText('Thermodynamics second law')).toHaveLength(2);
    expect(screen.getByText('2 days overdue')).toBeInTheDocument();
    expect(screen.getByText('Kinematics')).toBeInTheDocument();
    expect(screen.getAllByText('Due today').length).toBeGreaterThanOrEqual(3);
    expect(screen.getByText('1 further due revision in revision planning.')).toBeInTheDocument();
    expect(screen.getByText('good')).toBeInTheDocument();
    expect(screen.getByText(`${TODAY} · mastery 40.0 → 55.0`)).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Open revision planning' }));
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'revision' });
  });

  it('shows empty states for an idle schedule', () => {
    render(
      <RevisionOverviewSection
        data={dailyData({ revision: revisionDashboard() })}
        onNavigate={vi.fn()}
      />,
    );
    expect(
      screen.getByText('No revisions are due today and none are overdue.'),
    ).toBeInTheDocument();
    expect(screen.getByText('No revision history has been recorded yet.')).toBeInTheDocument();
  });

  it('shows the unavailable copy when revision data is missing', () => {
    render(<RevisionOverviewSection data={dailyData()} onNavigate={vi.fn()} />);
    expect(screen.getByText('Revision data is not available.')).toBeInTheDocument();
  });
});

describe('RecentlyStudiedSection', () => {
  it('lists recently studied topics and navigates to their syllabus', async () => {
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <RecentlyStudiedSection
        data={dailyData({
          totals: studyTotals([
            {
              nodeId: 1,
              nodeTitle: 'Vector calculus',
              lastStudiedDate: PAST,
              allocatedSeconds: 900,
            },
            {
              nodeId: 2,
              nodeTitle: 'Angular momentum',
              lastStudiedDate: TODAY,
              allocatedSeconds: 3_600,
              examCode: 'gate-physics',
            },
          ]),
        })}
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        onNavigate={onNavigate}
      />,
    );

    expect(screen.getByText('2 topics with recent study time')).toBeInTheDocument();
    expect(screen.getByText(/GATE PH · Quantum Mechanics · Angular momentum/)).toBeInTheDocument();
    expect(screen.getByText('1h · last studied 2026-11-06')).toBeInTheDocument();

    await user.click(
      screen.getByRole('button', { name: /GATE PH · Quantum Mechanics · Angular momentum/ }),
    );
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'syllabus', examCode: 'gate-physics' });
  });

  it('shows the empty state when no topic time exists', () => {
    render(<RecentlyStudiedSection data={dailyData()} exams={[]} onNavigate={vi.fn()} />);
    expect(screen.getByText(/No topic-allocated study time is recorded yet/)).toBeInTheDocument();
  });
});

describe('QuickStartSection', () => {
  it('uses the shared provider snapshot and reflects timer mutations without remounting', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    render(
      <TimerProvider>
        <QuickStartSection exams={[]} sessions={[]} today={TODAY} />
        <MutateTimerButton />
      </TimerProvider>,
    );

    expect(await screen.findByText('Timer running')).toBeInTheDocument();
    expect(mockedTimer).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('button', { name: 'Mutate timer' }));
    expect(await screen.findByText('Timer paused')).toBeInTheDocument();
    expect(mockedTimer).toHaveBeenCalledTimes(1);
  });

  it('reflects a StudySection pause without remounting Quick Start', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    mockedPause.mockResolvedValue(timerSnapshot({ status: 'paused' }));

    render(
      <TimerProvider>
        <QuickStartSection exams={[]} sessions={[]} today={TODAY} />
        <StudySection exams={[]} today={TODAY} />
      </TimerProvider>,
    );

    expect(await screen.findByText('Timer running')).toBeInTheDocument();
    await user.click(await screen.findByRole('button', { name: 'Pause timer' }));
    expect(await screen.findByText('Timer paused')).toBeInTheDocument();
  });

  it('shows and starts the last-used configuration from recorded sessions', async () => {
    const dispatchSpy = vi.spyOn(window, 'dispatchEvent');
    const user = userEvent.setup();
    mockedQuickStartList.mockResolvedValue([]);
    mockedStartTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    render(
      <QuickStartHarness
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[
          studySession({
            id: 5,
            topicAllocations: [
              {
                examId: 2,
                examCode: 'gate-physics',
                examName: 'GATE Physics',
                nodeId: 4,
                nodeTitle: 'Vector calculus',
                subjectTitle: 'Mathematical Methods',
                allocatedSeconds: 1_800,
              },
            ],
          }),
        ]}
        today={TODAY}
      />,
    );

    expect(
      await screen.findByText('Repeat the last topics · 2026-11-06 · 30 min'),
    ).toBeInTheDocument();
    expect(screen.getAllByText('gate-physics · Vector calculus · 30 min')).toHaveLength(2);

    await user.click(screen.getByRole('button', { name: 'Start last used configuration' }));
    expect(mockedStartTimer).toHaveBeenCalledTimes(1);
    expect(mockedStartTimer).toHaveBeenCalledWith({
      mode: 'focus',
      sessionDate: TODAY,
      activeExamId: null,
      allocations: [{ examId: 2, nodeId: 4, allocatedSeconds: 1_800 }],
      durationMinutes: 30,
    });
    expect(
      dispatchSpy.mock.calls.some((call) => (call[0] as Event).type === TIMER_UPDATED_EVENT),
    ).toBe(true);
    dispatchSpy.mockRestore();
  });

  it('starts a combined session when the last record carried no topics', async () => {
    const user = userEvent.setup();
    mockedStartTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    render(
      <QuickStartHarness
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[
          studySession({
            id: 5,
            durationSeconds: 1_500,
            topicAllocations: [],
          }),
        ]}
        today={TODAY}
      />,
    );
    expect(
      await screen.findByText('Combined study session · 2026-11-06 · 25 min'),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Start last used configuration' }));
    expect(mockedStartTimer).toHaveBeenCalledWith(expect.objectContaining({ durationMinutes: 25 }));
    expect(mockedStartTimer.mock.calls[0][0]).toMatchObject({ allocations: [] });
  });

  it('explains how to restore the timer when the study section is hidden', async () => {
    render(<QuickStartHarness exams={[]} sessions={[]} today={TODAY} />);
    await screen.findByText('Timer idle');
    await userEvent.setup().click(screen.getByRole('button', { name: 'Open the study timer' }));
    expect(screen.getByRole('status').textContent).toMatch(/study timer section is hidden/u);
  });

  it('does not touch the timer lifecycle through navigation or the builder', async () => {
    const heading = document.createElement('h2');
    heading.id = 'study-timer-heading';
    heading.scrollIntoView = vi.fn() as unknown as (arg?: boolean | ScrollIntoViewOptions) => void;
    document.body.appendChild(heading);
    const user = userEvent.setup();
    try {
      render(<QuickStartHarness exams={[]} sessions={[]} today={TODAY} />);
      await screen.findByText('Timer idle');
      await user.click(screen.getByRole('button', { name: 'Open the study timer' }));
      await user.click(screen.getByRole('button', { name: 'Start focus session' }));
      expect(api.startV2Timer).toHaveBeenCalledTimes(1);
      expect(api.startV2Timer).toHaveBeenCalledWith({
        mode: 'focus',
        sessionDate: TODAY,
        activeExamId: null,
        allocations: [],
        durationMinutes: 25,
      });
      expect(api.pauseV2Timer).not.toHaveBeenCalled();
      expect(api.resumeV2Timer).not.toHaveBeenCalled();
      expect(api.finishV2Timer).not.toHaveBeenCalled();
      expect(api.discardV2Timer).not.toHaveBeenCalled();
      expect(api.setV2TimerAllocations).not.toHaveBeenCalled();
      expect(api.updateV2TimerSettings).not.toHaveBeenCalled();
    } finally {
      heading.remove();
    }
  });

  it('guards against rapid repeated starts with a single backend call', async () => {
    const user = userEvent.setup();
    let releaseStart: (value: V2TimerSnapshot) => void = () => {};
    mockedStartTimer.mockImplementationOnce(
      () =>
        new Promise<V2TimerSnapshot>((resolve) => {
          releaseStart = resolve;
        }),
    );
    render(
      <QuickStartHarness
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[
          studySession({
            id: 5,
            topicAllocations: [
              {
                examId: 2,
                examCode: 'gate-physics',
                examName: 'GATE Physics',
                nodeId: 4,
                nodeTitle: 'Vector calculus',
                subjectTitle: 'Mathematical Methods',
                allocatedSeconds: 1_800,
              },
            ],
          }),
        ]}
        today={TODAY}
      />,
    );
    const startButton = await screen.findByRole('button', {
      name: 'Start last used configuration',
    });
    await user.click(startButton);
    await user.click(startButton);
    expect(mockedStartTimer).toHaveBeenCalledTimes(1);
    releaseStart(timerSnapshot());
  });

  it('hides start controls and shows the running session while a timer runs', async () => {
    mockedTimer.mockResolvedValue(
      timerSnapshot(
        { status: 'running', elapsedMs: 0, remainingMs: 30 * 60_000, durationMs: 30 * 60_000 },
        [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics',
            nodeId: 4,
            nodeTitle: 'Vector calculus',
            subjectTitle: 'Mathematical Methods',
            allocatedSeconds: 1_800,
          },
        ],
      ),
    );
    render(<QuickStartHarness exams={[]} sessions={[]} today={TODAY} />);
    await screen.findByText('Timer running');
    expect(screen.queryByRole('button', { name: 'Start last used configuration' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Start focus session' })).toBeNull();
    expect(screen.getByText('30m focus timer · 1 topic allocation')).toBeInTheDocument();
    expect(screen.getAllByText('gate-physics · Vector calculus · 30 min')).toHaveLength(1);
    expect(screen.getByRole('button', { name: 'Open the study timer' })).toBeInTheDocument();
  });

  it('offers resume for a paused timer without starting another session', async () => {
    const user = userEvent.setup();
    mockedTimer.mockResolvedValue(timerSnapshot({ status: 'paused' }));
    render(<QuickStartHarness exams={[]} sessions={[]} today={TODAY} />);
    await screen.findByText('Timer paused');
    expect(screen.queryByRole('button', { name: 'Start focus session' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Resume timer' }));
    expect(api.resumeV2Timer).toHaveBeenCalledTimes(1);
    expect(api.startV2Timer).not.toHaveBeenCalled();
  });

  it('surfaces recovery information without starting a second timer', async () => {
    mockedTimer.mockResolvedValue(
      timerSnapshot({
        status: 'running',
        recoveryReason: 'restart',
        recoveryExcludedMs: 90_000,
      }),
    );
    render(<QuickStartHarness exams={[]} sessions={[]} today={TODAY} />);
    expect(
      await screen.findByText(/Recovery excluded .* of unverified time after a restart/u),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Start last used configuration' })).toBeNull();
  });

  it('rejects invalid builder allocations before any backend command runs', async () => {
    const user = userEvent.setup();
    render(
      <QuickStartHarness
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[]}
        today={TODAY}
      />,
    );
    await screen.findByText('Timer idle');
    await user.click(screen.getByRole('button', { name: 'Add exam/topic allocation' }));
    expect(
      screen.getByText('Choose an examination and topic for every allocation.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start focus session' })).toBeDisabled();
    expect(api.startV2Timer).not.toHaveBeenCalled();
  });

  it('rejects allocations that exceed the session duration', async () => {
    const user = userEvent.setup();
    mockedSyllabus.mockResolvedValue(syllabusScaffold(2, ['Electromagnetism']));
    mockedQuickStartList.mockResolvedValue([]);
    render(<QuickStartHarness exams={EXAMS} sessions={[]} today={TODAY} />);
    await screen.findByText('Timer idle');
    await user.click(screen.getByRole('button', { name: 'Plan a 25 minute session' }));
    await user.click(screen.getByRole('button', { name: 'Add exam/topic allocation' }));
    await user.selectOptions(screen.getByRole('combobox', { name: 'Subject / topic' }), ['700']);
    const minutesInput = screen.getByLabelText('Minutes', { selector: 'input' });
    await user.clear(minutesInput);
    await user.type(minutesInput, '45');
    expect(
      screen.getByText(
        'Allocations total 45 minutes, which exceeds the 25-minute session duration.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start focus session' })).toBeDisabled();
    expect(api.startV2Timer).not.toHaveBeenCalled();
  });

  it('starts, loads and pins saved configurations from the database', async () => {
    const user = userEvent.setup();
    mockedSyllabus.mockResolvedValue(syllabusScaffold(2, ['Electromagnetism']));
    mockedQuickStartList.mockResolvedValue([
      {
        id: 3,
        name: 'Morning drill',
        durationMinutes: 45,
        isPinned: true,
        createdAt: '2026-11-01T00:00:00Z',
        updatedAt: '2026-11-05T00:00:00Z',
        allocations: [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics',
            nodeId: 7,
            nodeTitle: 'Electromagnetism',
            subjectTitle: 'Physics (PH)',
            allocatedMinutes: 45,
          },
        ],
      },
    ]);
    mockedStartTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    mockedSaveQuickStart.mockResolvedValue([
      {
        id: 3,
        name: 'Morning drill',
        durationMinutes: 45,
        isPinned: true,
        createdAt: '2026-11-01T00:00:00Z',
        updatedAt: '2026-11-05T00:00:00Z',
        allocations: [
          {
            examId: 2,
            examCode: 'gate-physics',
            examName: 'GATE Physics',
            nodeId: 7,
            nodeTitle: 'Electromagnetism',
            subjectTitle: 'Physics (PH)',
            allocatedMinutes: 45,
          },
        ],
      },
    ]);
    render(<QuickStartHarness exams={EXAMS} sessions={[]} today={TODAY} />);
    expect(await screen.findByText('Morning drill')).toBeInTheDocument();
    expect(
      screen.getAllByText('GATE PH · Electromagnetism · 45 min').length,
    ).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Pinned')).toBeInTheDocument();

    await user.click(
      screen.getByRole('button', {
        name: 'Load Morning drill into the quick configuration',
      }),
    );
    expect(screen.getByText(/Editing a saved configuration/u)).toBeInTheDocument();
    expect(screen.getByDisplayValue('Morning drill')).toBeInTheDocument();
    expect(screen.getByLabelText('Session duration', { selector: 'input' })).toHaveValue(45);

    await user.click(screen.getByRole('button', { name: 'Unpin the Morning drill configuration' }));
    expect(mockedSaveQuickStart).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 3,
        name: 'Morning drill',
        isPinned: false,
        allocations: [
          expect.objectContaining({
            examId: 2,
            syllabusNodeId: 7,
            allocatedMinutes: 45,
          }),
        ],
      }),
    );

    await user.click(screen.getByRole('button', { name: 'Start the Morning drill configuration' }));
    expect(mockedStartTimer).toHaveBeenCalledWith({
      mode: 'focus',
      sessionDate: TODAY,
      activeExamId: null,
      allocations: [{ examId: 2, nodeId: 7, allocatedSeconds: 2_700 }],
      durationMinutes: 45,
    });
  });

  it('saves a named configuration, then edits and deletes it with confirmation', async () => {
    const user = userEvent.setup();
    mockedSaveQuickStart.mockResolvedValue([
      {
        id: 9,
        name: 'Evening GATE push',
        durationMinutes: 90,
        isPinned: false,
        createdAt: '2026-11-06T00:00:00Z',
        updatedAt: '2026-11-06T00:00:00Z',
        allocations: [],
      },
    ]);
    render(
      <QuickStartHarness
        exams={[
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[]}
        today={TODAY}
      />,
    );
    await screen.findByText('Timer idle');

    const saveButton = screen.getByRole('button', { name: 'Save configuration' });
    expect(saveButton).toBeDisabled();
    await user.type(screen.getByLabelText('Configuration name'), 'Evening GATE push');
    await user.click(screen.getByRole('button', { name: 'Plan a 90 minute session' }));

    await user.click(saveButton);
    expect(mockedSaveQuickStart).toHaveBeenCalledWith({
      id: null,
      name: 'Evening GATE push',
      durationMinutes: 90,
      isPinned: null,
      allocations: [],
    });

    mockedDeleteQuickStart.mockResolvedValueOnce([]);
    await user.click(
      screen.getByRole('button', { name: 'Delete the Evening GATE push configuration' }),
    );
    expect(screen.getByRole('button', { name: 'Confirm delete' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Confirm delete' }));
    expect(mockedDeleteQuickStart).toHaveBeenCalledWith({ id: 9 });
  });

  it('per-exam allocations load from recent sessions and start without duplication', async () => {
    const user = userEvent.setup();
    mockedQuickStartList.mockResolvedValue([]);
    mockedStartTimer.mockResolvedValue(timerSnapshot({ status: 'running' }));
    render(
      <QuickStartHarness
        exams={[
          {
            id: 1,
            code: 'csir-net-physics',
            name: 'CSIR-UGC NET Physical Sciences',
            shortName: 'CSIR-NET',
            authority: '',
            officialUrl: '',
            displayOrder: 1,
            isActive: true,
            examDate: null,
          },
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        sessions={[
          studySession({
            id: 5,
            topicAllocations: [
              {
                examId: 1,
                examCode: 'csir-net-physics',
                examName: 'CSIR-UGC NET Physical Sciences',
                nodeId: 2,
                nodeTitle: 'Vector calculus',
                subjectTitle: 'Mathematical Methods',
                allocatedSeconds: 1_200,
              },
            ],
          }),
          studySession({
            id: 6,
            topicAllocations: [
              {
                examId: 1,
                examCode: 'csir-net-physics',
                examName: 'CSIR-UGC NET Physical Sciences',
                nodeId: 2,
                nodeTitle: 'Vector calculus',
                subjectTitle: 'Mathematical Methods',
                allocatedSeconds: 1_200,
              },
              {
                examId: 2,
                examCode: 'gate-physics',
                examName: 'GATE Physics',
                nodeId: 7,
                nodeTitle: 'Electromagnetism',
                subjectTitle: 'Physics (PH)',
                allocatedSeconds: 900,
              },
            ],
          }),
        ]}
        today={TODAY}
      />,
    );

    expect(await screen.findByText(/2026-11-06 · 35 min across 2 topics/u)).toBeInTheDocument();
    await user.click(
      screen.getAllByRole('button', {
        name: /Load the recent configuration of 2026-11-06 into the quick configuration/u,
      })[0],
    );
    await user.click(screen.getByRole('button', { name: 'Start focus session' }));
    expect(mockedStartTimer).toHaveBeenCalledWith({
      mode: 'focus',
      sessionDate: TODAY,
      activeExamId: null,
      allocations: [
        { examId: 1, nodeId: 2, allocatedSeconds: 1_200 },
        { examId: 2, nodeId: 7, allocatedSeconds: 900 },
      ],
      durationMinutes: 35,
    });
  });
});

describe('PerformanceSection', () => {
  it('shows the latest mock and lowest-accuracy topics per active exam', async () => {
    mockedPracticeAnalytics.mockImplementation((input: { examId: number }) => {
      if (input.examId === 2) {
        return Promise.resolve(
          practiceAnalytics({
            mocks: {
              completedCount: 1,
              averagePercentage: 70,
              bestPercentage: 70,
              lastPercentage: 70,
              byPattern: [],
              insufficientData: false,
              history: [
                {
                  testId: 3,
                  testDate: '2026-11-01',
                  testName: 'GATE sectional Part A',
                  percentage: 70,
                  patternVersionId: 2,
                  patternVersionLabel: '2026 official',
                },
              ],
            },
          }),
        );
      }
      return Promise.resolve(
        practiceAnalytics({
          byTopic: [
            {
              nodeId: 1,
              label: 'Homework 70',
              attempts: 4,
              correct: 0,
              incorrect: 0,
              unanswered: 0,
              accuracyPercentage: 70,
            },
            {
              nodeId: 2,
              label: 'Exam 45',
              attempts: 6,
              correct: 0,
              incorrect: 0,
              unanswered: 0,
              accuracyPercentage: 45,
            },
            {
              nodeId: 3,
              label: 'No accuracy',
              attempts: 2,
              correct: 0,
              incorrect: 0,
              unanswered: 0,
              accuracyPercentage: null,
            },
            {
              nodeId: 4,
              label: 'Exam 30',
              attempts: 3,
              correct: 0,
              incorrect: 0,
              unanswered: 0,
              accuracyPercentage: 30,
            },
            {
              nodeId: 5,
              label: 'Exam 55',
              attempts: 8,
              correct: 0,
              incorrect: 0,
              unanswered: 0,
              accuracyPercentage: 55,
            },
          ],
        }),
      );
    });
    const onNavigate = vi.fn();
    const user = userEvent.setup();
    render(
      <PerformanceSection
        exams={[
          {
            id: 1,
            code: 'csir-net-physics',
            name: 'CSIR-UGC NET Physical Sciences',
            shortName: 'CSIR-NET',
            authority: '',
            officialUrl: '',
            displayOrder: 1,
            isActive: true,
            examDate: null,
          },
          {
            id: 2,
            code: 'gate-physics',
            name: 'GATE Physics',
            shortName: 'GATE PH',
            authority: '',
            officialUrl: '',
            displayOrder: 2,
            isActive: true,
            examDate: null,
          },
        ]}
        onNavigate={onNavigate}
      />,
    );

    expect(await screen.findByText('Exam 30')).toBeInTheDocument();
    expect(screen.getByText('Last mock — 2026-11-01 · GATE sectional Part A')).toBeInTheDocument();
    expect(screen.queryByText('No accuracy')).not.toBeInTheDocument();

    await user.click(screen.getAllByRole('button', { name: 'Review in practice systems' })[1]);
    expect(onNavigate).toHaveBeenCalledWith({ kind: 'practice', examCode: 'gate-physics' });
  });

  it('surfaces per-exam errors and the section-level empty state', async () => {
    mockedPracticeAnalytics.mockRejectedValue(new Error('analytics unavailable'));
    render(
      <PerformanceSection
        exams={[
          {
            id: 1,
            code: 'csir-net-physics',
            name: 'CSIR-UGC NET Physical Sciences',
            shortName: 'CSIR-NET',
            authority: '',
            officialUrl: '',
            displayOrder: 1,
            isActive: true,
            examDate: null,
          },
        ]}
        onNavigate={vi.fn()}
      />,
    );
    expect(await screen.findByRole('alert')).toHaveTextContent(/could not be loaded/u);
    expect(screen.getByText(/No mock or topic accuracy data is recorded yet/u)).toBeInTheDocument();
  });
});

describe('useDailyDashboardData', () => {
  it('fetches the shared set once when enabled and again when today changes', async () => {
    mockedSessions.mockResolvedValue([studySession({ id: 42 })]);
    const probe = render(<HookProbe today={TODAY} enabled={false} />);
    expect(screen.getByText('loaded')).toBeInTheDocument();
    expect(mockedSessions).not.toHaveBeenCalled();

    probe.rerender(<HookProbe today={TODAY} enabled={true} />);
    expect(screen.getByText('loading')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByTestId('sessions').textContent).toBe('1'));
    expect(mockedSessions).toHaveBeenCalledTimes(1);

    probe.rerender(<HookProbe today={PAST} enabled={true} />);
    await waitFor(() => expect(mockedSessions).toHaveBeenCalledTimes(2));
  });

  it('exposes failures through one retry action', async () => {
    mockedSessions.mockRejectedValueOnce(
      Object.assign(new Error('Session list failed.'), { code: 'command_failed' }),
    );
    render(<HookProbe today={TODAY} enabled={true} />);
    await waitFor(() => expect(screen.getByText('Session list failed.')).toBeInTheDocument());
    mockedSessions.mockResolvedValueOnce([studySession({ id: 99 })]);
    await userEvent.setup().click(screen.getByRole('button', { name: 'retry' }));
    await waitFor(() => expect(screen.getByTestId('sessions').textContent).toBe('1'));
    expect(screen.getByText('no-error')).toBeInTheDocument();
  });
});
