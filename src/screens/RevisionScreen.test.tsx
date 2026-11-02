import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  RevisionDailyPlan,
  RevisionDashboard,
  RevisionHistoryEntry,
  RevisionIntervalDecision,
  RevisionPlanMembership,
  RevisionSchedule,
  RevisionTopic,
  RevisionTopicState,
} from '../lib/revisionApi';
import RevisionScreen from './RevisionScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(false),
  requestPermission: vi.fn().mockResolvedValue('denied'),
}));

const TODAY = '2026-10-30';

const TOPIC_A: RevisionTopic = {
  id: 1,
  code: 'MMP-C01',
  title: 'Dimensional analysis',
  archived: false,
  sectionCode: 'MMP',
  sectionTitle: 'Mathematical Methods of Physics',
  priority: 'High-Yield',
  confidence: 2,
  syllabusStatus: 'Learning',
  category: 'Core',
};

const TOPIC_B: RevisionTopic = {
  id: 2,
  code: 'GA-01',
  title: 'Logical reasoning',
  archived: false,
  sectionCode: 'GA',
  sectionTitle: 'General Aptitude',
  priority: 'High',
  confidence: 4,
  syllabusStatus: 'Revised',
  category: 'Core',
};

const TOPIC_C: RevisionTopic = {
  id: 3,
  code: 'CM-C01',
  title: 'Newtonian mechanics',
  archived: false,
  sectionCode: 'CM',
  sectionTitle: 'Classical Mechanics',
  priority: 'High',
  confidence: 1,
  syllabusStatus: 'Not started',
  category: 'Core',
};

const ARCHIVED_TOPIC: RevisionTopic = {
  id: 4,
  code: 'EMT-C01',
  title: 'Electrostatics',
  archived: true,
  sectionCode: 'EMT',
  sectionTitle: 'Electromagnetic Theory',
  priority: 'Low',
  confidence: 5,
  syllabusStatus: 'Confident',
  category: 'Optional',
};

function schedule(overrides: Partial<RevisionSchedule> = {}): RevisionSchedule {
  return {
    status: 'active',
    lastRevisionDate: '2026-10-25',
    nextRevisionDate: TODAY,
    currentIntervalDays: 5,
    completedRevisions: 2,
    previousOutcome: 'good',
    masteryEstimate: 60,
    notes: 'Focus on dimensions.',
    deferredUntil: null,
    updatedAt: '2026-10-25T10:00:00.000Z',
    ...overrides,
  };
}

const PLAN_ITEM_A: RevisionPlanMembership = {
  dailyPlanId: 1,
  planItemId: 101,
  topic: TOPIC_A,
  position: 0,
  estimatedMinutes: 25,
  lane: 'urgent_review',
  triageScore: 73.6,
  membershipStatus: 'planned',
  itemStatus: 'planned',
  actualSeconds: 0,
  remainingEstimateMinutes: 25,
  carriedFromDailyPlanId: null,
  plannedBlock: {
    id: null,
    title: null,
    calendarDate: null,
    startTime: null,
    endTime: null,
  },
};

const EMPTY_DAILY_PLAN: RevisionDailyPlan = {
  today: TODAY,
  planId: null,
  planDate: null,
  version: null,
  status: null,
  needsGeneration: true,
  effectiveBudgetMinutes: 90,
  maxTopics: 5,
  plannedMinutes: 0,
  completedMinutes: 0,
  actualMinutesToday: 0,
  remainingWorkMinutes: 0,
  remainingDailyBudgetMinutes: 90,
  overBudget: false,
  overLimit: false,
  scheduled: [],
  completed: [],
  backlog: [],
  deferred: [],
  unscheduledUrgent: [],
};

const GENERATED_DAILY_PLAN: RevisionDailyPlan = {
  ...EMPTY_DAILY_PLAN,
  planId: 1,
  planDate: TODAY,
  version: 1,
  status: 'active',
  needsGeneration: false,
  plannedMinutes: 25,
  remainingWorkMinutes: 25,
  remainingDailyBudgetMinutes: 65,
  scheduled: [PLAN_ITEM_A],
};

const INTERVAL_PREVIEW: RevisionIntervalDecision = {
  intervalBeforeDays: 5,
  intervalAfterDays: 10,
  timingDays: 0,
  timingFactor: 1,
  evidenceStatus: 'none',
  evidenceFactor: 1,
  reasons: ['Interval changes from 5 to 10 days.', 'Timing adjustment: +0 days.'],
};

const STATE_A: RevisionTopicState = {
  topic: TOPIC_A,
  schedule: schedule(),
  dueState: 'due_today',
  daysOverdue: 0,
  daysUntilDue: 0,
};

const STATE_B: RevisionTopicState = {
  topic: TOPIC_B,
  schedule: schedule({ nextRevisionDate: '2026-10-28', previousOutcome: 'again' }),
  dueState: 'overdue',
  daysOverdue: 2,
  daysUntilDue: null,
};

const STATE_C: RevisionTopicState = {
  topic: TOPIC_C,
  schedule: null,
  dueState: 'not_scheduled',
  daysOverdue: 0,
  daysUntilDue: null,
};

const ARCHIVED_STATE: RevisionTopicState = {
  topic: ARCHIVED_TOPIC,
  schedule: schedule({ lastRevisionDate: '2026-09-20', nextRevisionDate: '2026-10-01' }),
  dueState: 'overdue',
  daysOverdue: 29,
  daysUntilDue: null,
};

const HISTORY: RevisionHistoryEntry = {
  id: 11,
  topicId: 1,
  revisionDate: '2026-10-25',
  scheduledForDate: '2026-10-25',
  outcome: 'good',
  notes: 'Recalled most steps.',
  intervalBeforeDays: 2,
  intervalAfterDays: 5,
  masteryBefore: 45,
  masteryAfter: 60,
  createdAt: '2026-10-25T10:00:00.000Z',
  planItemId: null,
  actualSeconds: null,
  timingDays: 0,
};

const ARCHIVED_HISTORY: RevisionHistoryEntry = {
  ...HISTORY,
  id: 12,
  topicId: ARCHIVED_TOPIC.id,
  revisionDate: '2026-09-20',
  scheduledForDate: '2026-09-20',
  notes: 'Archived revision outcome.',
};

const DASHBOARD: RevisionDashboard = {
  today: TODAY,
  settings: {
    initialAgainDays: 1,
    initialDifficultDays: 2,
    initialGoodDays: 5,
    initialEasyDays: 10,
    growthMultiplier: 1.5,
    minimumIntervalDays: 1,
    maximumIntervalDays: 3650,
    includeWeekends: true,
    dailyRevisionTarget: 5,
    dailyRevisionMinutes: 90,
    remindersEnabled: false,
    updatedAt: '2026-10-25T10:00:00.000Z',
  },
  stats: {
    scheduledCount: 2,
    activeCount: 2,
    suspendedCount: 0,
    dueTodayCount: 1,
    overdueCount: 1,
    completedToday: 0,
    completedLast7Days: 1,
    totalCompleted: 3,
  },
  dueToday: [STATE_A],
  overdue: [STATE_B],
  upcoming: [],
  suspended: [],
  completedRecent: [{ history: HISTORY, topic: TOPIC_A }],
  topics: [STATE_A, STATE_B, STATE_C],
  archivedTopics: [ARCHIVED_STATE],
  recommendations: [
    {
      topic: TOPIC_A,
      scheduled: true,
      dueState: 'due_today',
      nextRevisionDate: TODAY,
      previousOutcome: 'good',
      masteryEstimate: 60,
      pyqAttempts: 6,
      pyqQuestions: 6,
      pyqAccuracy: 33.3,
      pyqEvidenceAccuracy: 33.3,
      pyqLastAttemptDate: TODAY,
      pyqSufficient: true,
      lane: 'urgent_review',
      triageScore: 73.6,
      daysOverdue: 0,
      daysUntilDue: 0,
      dueUrgency: 80,
      priorityNeed: 100,
      confidenceNeed: 75,
      statusNeed: 75,
      confidenceStatusNeed: 75,
      pyqEvidenceNeed: 60,
      revisionRisk: 31,
      evidenceStatus: 'weak',
      reasons: [
        'Due today.',
        'Syllabus priority: High-Yield.',
        'Confidence marked 2/5.',
        'Syllabus status: Learning.',
        'Recent PYQ accuracy is weak (33.3%) across 6 distinct questions.',
        'Previous revision outcome was Good.',
        'Revision mastery is 60.0%.',
      ],
    },
    {
      topic: TOPIC_C,
      scheduled: false,
      dueState: 'not_scheduled',
      nextRevisionDate: null,
      previousOutcome: null,
      masteryEstimate: null,
      pyqAttempts: 2,
      pyqQuestions: 2,
      pyqAccuracy: 0,
      pyqEvidenceAccuracy: 0,
      pyqLastAttemptDate: TODAY,
      pyqSufficient: false,
      lane: 'strengthen',
      triageScore: 17.5,
      daysOverdue: 0,
      daysUntilDue: null,
      dueUrgency: 0,
      priorityNeed: 75,
      confidenceNeed: 100,
      statusNeed: 100,
      confidenceStatusNeed: 100,
      pyqEvidenceNeed: 0,
      revisionRisk: 0,
      evidenceStatus: 'insufficient',
      reasons: [
        'Not scheduled.',
        'Syllabus priority: High.',
        'Confidence marked 1/5.',
        'Syllabus status: Not started.',
        'Only 2 distinct scored PYQ questions; at least 5 are required for weak or strong evidence.',
        'High-priority topic has insufficient recent PYQ evidence.',
      ],
    },
  ],
  mockScopeNote:
    'Mock tests are stored at part level and do not map to individual syllabus topics.',
  mockTestCount: 2,
};
function mockDashboardBackend(dashboard: RevisionDashboard) {
  invokeMock.mockImplementation((command: string) => {
    if (command === 'get_revision_dashboard') {
      return Promise.resolve(dashboard);
    }
    if (command === 'get_revision_daily_plan') {
      return Promise.resolve(EMPTY_DAILY_PLAN);
    }
    if (command === 'get_revision_interval_preview') {
      return Promise.resolve(INTERVAL_PREVIEW);
    }
    if (command === 'get_revision_history') {
      return Promise.resolve([]);
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

function mockRevisionBackend() {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'get_revision_dashboard') {
      return Promise.resolve(DASHBOARD);
    }
    if (command === 'get_revision_daily_plan') {
      return Promise.resolve(EMPTY_DAILY_PLAN);
    }
    if (command === 'get_revision_interval_preview') {
      return Promise.resolve(INTERVAL_PREVIEW);
    }
    if (command === 'get_revision_history') {
      if (args?.topicId === ARCHIVED_TOPIC.id) {
        return Promise.resolve([ARCHIVED_HISTORY]);
      }
      return Promise.resolve(args?.topicId === TOPIC_A.id ? [HISTORY] : []);
    }
    if (command === 'complete_revision') {
      return Promise.resolve({
        state: STATE_A,
        history: HISTORY,
        planItemId: null,
      });
    }
    if (command === 'reschedule_revision') {
      return Promise.resolve({
        ...STATE_A,
        schedule: schedule({ nextRevisionDate: '2026-11-20' }),
      });
    }
    if (command === 'suspend_revision') {
      return Promise.resolve({
        ...STATE_A,
        schedule: schedule({ status: 'suspended' }),
        dueState: 'suspended',
      });
    }
    if (command === 'reactivate_revision') {
      return Promise.resolve(STATE_A);
    }
    if (command === 'schedule_revision_topic') {
      return Promise.resolve({
        ...STATE_C,
        schedule: schedule({ nextRevisionDate: TODAY }),
        dueState: 'due_today',
        daysUntilDue: 0,
      });
    }
    if (command === 'update_revision_notes') {
      return Promise.resolve(STATE_A);
    }
    if (command === 'defer_revision') {
      return Promise.resolve({
        plan: EMPTY_DAILY_PLAN,
        event: {
          id: 77,
          topicId: TOPIC_A.id,
          planItemId: null,
          eventKind: 'defer',
          overriddenDeferralId: null,
          previousDueDate: TODAY,
          newDueDate: '2026-11-10',
          occurredOn: TODAY,
          reason: 'time',
          notes: 'Conference',
          createdAt: '2026-10-30T10:00:00.000Z',
        },
        schedule: schedule({
          nextRevisionDate: '2026-11-10',
          deferredUntil: '2026-11-10',
        }),
      });
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}
describe('revision screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders queues, recommendations, recent history, and filters topic schedules', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);
    expect(await screen.findByRole('heading', { name: 'Overdue revisions' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Due today' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Upcoming revisions' })).toBeInTheDocument();
    expect(screen.getByText('Recommendations')).toBeInTheDocument();
    expect(screen.getAllByText('Urgent review').length).toBeGreaterThan(0);
    expect(screen.getByText('Strengthen')).toBeInTheDocument();
    expect(screen.getByText('Triage score 73.6')).toBeInTheDocument();
    expect(screen.getByText('Triage score 17.5')).toBeInTheDocument();
    expect(screen.getAllByText(/PYQ accuracy is weak \(33.3%\)/).length).toBeGreaterThan(0);
    expect(
      screen.getAllByText(/at least 5 are required for weak or strong evidence/).length,
    ).toBeGreaterThan(0);
    expect(screen.getByText('Completed recently')).toBeInTheDocument();
    expect(screen.getByText(/Recalled most steps/)).toBeInTheDocument();
    expect(screen.getByText(DASHBOARD.mockScopeNote)).toBeInTheDocument();
    expect(screen.getByText('3 of 3')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Search'), { target: { value: 'GA-01' } });
    await waitFor(() => expect(screen.getByText('1 of 3')).toBeInTheDocument());
    fireEvent.change(screen.getByLabelText('Scheduling status'), {
      target: { value: 'not_scheduled' },
    });
    await waitFor(() =>
      expect(screen.getByText('No topics match these filters')).toBeInTheDocument(),
    );
    await userEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    expect(await screen.findByText('3 of 3')).toBeInTheDocument();
  });

  it('shows an unscheduled strengthen recommendation without fabricating a due state', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);

    const heading = await screen.findByRole('heading', {
      name: 'CM-C01 · Newtonian mechanics',
      level: 3,
    });
    const card = heading.closest('article');
    expect(card).not.toBeNull();
    expect(within(card as HTMLElement).getByText('Strengthen')).toBeInTheDocument();
    expect(within(card as HTMLElement).getByText('Triage score 17.5')).toBeInTheDocument();
    expect(within(card as HTMLElement).getAllByText(/Not scheduled/).length).toBeGreaterThan(0);
    expect(within(card as HTMLElement).queryByText(/overdue/i)).not.toBeInTheDocument();
    expect(
      within(card as HTMLElement).getByText(/insufficient recent PYQ evidence/),
    ).toBeInTheDocument();
  });

  it('renders the maintain lane with explicit stale and skipped-only evidence copy', async () => {
    mockDashboardBackend({
      ...DASHBOARD,
      recommendations: [
        {
          ...DASHBOARD.recommendations[0],
          topic: TOPIC_B,
          lane: 'maintain',
          triageScore: 38.4,
          dueState: 'upcoming',
          nextRevisionDate: '2026-11-11',
          daysOverdue: 0,
          daysUntilDue: 12,
          dueUrgency: 46,
          priorityNeed: 75,
          confidenceNeed: 25,
          statusNeed: 50,
          confidenceStatusNeed: 35,
          revisionRisk: 21,
          masteryEstimate: 85,
          pyqAttempts: 8,
          pyqQuestions: 8,
          pyqAccuracy: 80,
          pyqEvidenceAccuracy: 80,
          pyqSufficient: true,
          evidenceStatus: 'strong',
          pyqEvidenceNeed: 0,
          reasons: [
            'Due in 12 days.',
            'Recent PYQ accuracy is strong (80.0%) across 8 distinct questions.',
            'Revision mastery is high (85.0%).',
            'Strong evidence and high revision mastery.',
          ],
        },
        {
          ...DASHBOARD.recommendations[0],
          topic: TOPIC_A,
          lane: 'strengthen',
          triageScore: 38.1,
          dueState: 'upcoming',
          nextRevisionDate: '2026-11-19',
          daysUntilDue: 20,
          dueUrgency: 30,
          revisionRisk: 41,
          masteryEstimate: 35,
          pyqAttempts: 6,
          pyqQuestions: 0,
          pyqAccuracy: 33.3,
          pyqEvidenceAccuracy: null,
          pyqLastAttemptDate: '2025-01-01',
          pyqSufficient: false,
          evidenceStatus: 'stale',
          pyqEvidenceNeed: 0,
          reasons: [
            'Due in 20 days.',
            'No scored PYQ evidence is within the last 365 calendar days.',
            'Revision mastery is low (35.0%).',
          ],
        },
        {
          ...DASHBOARD.recommendations[1],
          topic: TOPIC_C,
          lane: 'strengthen',
          pyqAttempts: 0,
          pyqQuestions: 0,
          pyqAccuracy: null,
          pyqEvidenceAccuracy: null,
          pyqLastAttemptDate: null,
          pyqSufficient: false,
          evidenceStatus: 'skipped_only',
          reasons: [
            'Not scheduled.',
            'PYQ evidence is skipped-only; skipped attempts do not count as scored evidence.',
          ],
        },
      ],
    });
    render(<RevisionScreen today={TODAY} />);
    expect(await screen.findByText('Maintain')).toBeInTheDocument();
    expect(screen.getByText('Triage score 38.4')).toBeInTheDocument();
    expect(screen.getByText('Strong evidence and high revision mastery.')).toBeInTheDocument();
    expect(
      screen.getByText('No scored PYQ evidence is within the last 365 calendar days.'),
    ).toBeInTheDocument();
    expect(screen.getByText(/Stale PYQ evidence/)).toBeInTheDocument();
    expect(
      screen.getByText(
        'PYQ evidence is skipped-only; skipped attempts do not count as scored evidence.',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/Skipped-only PYQ evidence/)).toBeInTheDocument();
  });

  it('opens a topic, shows history, and completes a revision with typed outcome and notes', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);

    const dueTodayQueue = await screen.findByRole('region', { name: 'Due today' });
    await userEvent.click(within(dueTodayQueue).getByRole('button', { name: 'Review' }));
    expect(
      await screen.findByRole('heading', {
        name: 'MMP-C01 · Dimensional analysis',
        level: 2,
      }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Topic revision notes (optional)')).toHaveValue(
      'Focus on dimensions.',
    );
    expect(await screen.findByText('Recalled most steps.')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Start revision' }));
    fireEvent.change(screen.getByLabelText('Outcome'), { target: { value: 'easy' } });
    fireEvent.change(screen.getByLabelText('Revision notes (optional)'), {
      target: { value: 'Confident derivation' },
    });
    expect(screen.getByLabelText('Next interval preview')).toHaveTextContent('10 days');
    await userEvent.click(screen.getByRole('button', { name: 'Complete revision' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('complete_revision', {
        topicId: TOPIC_A.id,
        input: { outcome: 'easy', notes: 'Confident derivation' },
      }),
    );
    expect(await screen.findByText(/Revision recorded/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('get_revision_history', {
      topicId: TOPIC_A.id,
    });
  });

  it('opens archived revision history read-only without scheduling actions', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);
    await screen.findByText('Archived revision history (1)');
    await userEvent.click(screen.getByRole('button', { name: 'View history' }));
    expect(
      await screen.findByRole('heading', {
        name: 'EMT-C01 · Electrostatics',
        level: 2,
      }),
    ).toBeInTheDocument();
    expect(await screen.findByText('Archived revision outcome.')).toBeInTheDocument();
    expect(
      screen.getByText(/Archived topic. Revision scheduling is read-only/),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Start revision' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Manual reschedule' })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Topic revision notes (optional)')).not.toBeInTheDocument();
  });

  it('manually reschedules, suspends, and schedules an unscheduled topic explicitly', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);

    const dueTodayQueue = await screen.findByRole('region', { name: 'Due today' });
    await userEvent.click(within(dueTodayQueue).getByRole('button', { name: 'Review' }));
    await screen.findByRole('heading', {
      name: 'MMP-C01 · Dimensional analysis',
      level: 2,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Manual reschedule' }));
    fireEvent.change(screen.getByLabelText('New next revision date'), {
      target: { value: '2026-11-20' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Save new date' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('reschedule_revision', {
        topicId: TOPIC_A.id,
        nextDate: '2026-11-20',
      }),
    );
    expect(screen.getByText(/manually set/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Suspend scheduling' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('suspend_revision', { topicId: TOPIC_A.id }),
    );
    expect(screen.getByText('Revision scheduling suspended.')).toBeInTheDocument();
    const topicTable = screen.getAllByRole('table')[0];
    const row = within(topicTable)
      .getByText(/CM-C01 · Newtonian mechanics/)
      .closest('tr');
    expect(row).not.toBeNull();
    await userEvent.click(within(row as HTMLElement).getByRole('button', { name: 'Schedule' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('schedule_revision_topic', {
        topicId: TOPIC_C.id,
        nextDate: TODAY,
      }),
    );
    expect(screen.getByText(/Topic added to the revision schedule/)).toBeInTheDocument();
  });

  it('generates a daily plan and wires plan items to the timer and backlog', async () => {
    let currentPlan = EMPTY_DAILY_PLAN;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_revision_dashboard') {
        return Promise.resolve(DASHBOARD);
      }
      if (command === 'get_revision_daily_plan') {
        return Promise.resolve(currentPlan);
      }
      if (command === 'get_revision_interval_preview') {
        return Promise.resolve(INTERVAL_PREVIEW);
      }
      if (command === 'generate_revision_daily_plan') {
        currentPlan = GENERATED_DAILY_PLAN;
        return Promise.resolve(currentPlan);
      }
      if (command === 'start_study_timer') {
        return Promise.resolve({});
      }
      if (command === 'move_revision_plan_item') {
        currentPlan = {
          ...GENERATED_DAILY_PLAN,
          plannedMinutes: 0,
          remainingWorkMinutes: 0,
          remainingDailyBudgetMinutes: 90,
          scheduled: [],
        };
        return Promise.resolve(currentPlan);
      }
      if (command === 'get_revision_history') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<RevisionScreen today={TODAY} />);
    const planRegion = await screen.findByRole('region', { name: "Today's revision plan" });
    expect(
      within(planRegion).getByText(/No plan has been generated for today/),
    ).toBeInTheDocument();
    await userEvent.click(
      within(planRegion).getByRole('button', { name: "Generate today's plan" }),
    );
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('generate_revision_daily_plan'));
    expect(
      await within(planRegion).findByRole('heading', { name: 'Scheduled work' }),
    ).toBeInTheDocument();
    await userEvent.click(
      within(planRegion).getByRole('button', {
        name: /Start focus timer for MMP-C01/,
      }),
    );
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('start_study_timer', {
        mode: 'focus',
        topicId: TOPIC_A.id,
        planItemId: PLAN_ITEM_A.planItemId,
      }),
    );
    await userEvent.click(
      within(planRegion).getByRole('button', {
        name: /Move MMP-C01 to backlog/,
      }),
    );
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('move_revision_plan_item', {
        planItemId: PLAN_ITEM_A.planItemId,
        target: 'backlog',
      }),
    );
  });

  it('defers a revision with an explicit return date and reason', async () => {
    mockRevisionBackend();

    render(<RevisionScreen today={TODAY} />);
    const dueTodayQueue = await screen.findByRole('region', { name: 'Due today' });
    await userEvent.click(within(dueTodayQueue).getByRole('button', { name: 'Review' }));
    await screen.findByRole('heading', {
      name: 'MMP-C01 · Dimensional analysis',
      level: 2,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Defer revision' }));
    fireEvent.change(screen.getByLabelText('Return date'), {
      target: { value: '2026-11-10' },
    });
    fireEvent.change(screen.getByLabelText('Reason'), { target: { value: 'time' } });
    fireEvent.change(screen.getByLabelText('Notes (optional)'), {
      target: { value: 'Conference' },
    });
    const deferButtons = screen.getAllByRole('button', { name: 'Defer revision' });
    await userEvent.click(deferButtons[deferButtons.length - 1]);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('defer_revision', {
        topicId: TOPIC_A.id,
        newDate: '2026-11-10',
        reason: 'time',
        notes: 'Conference',
      }),
    );
    expect(await screen.findByText(/Revision deferred until/)).toBeInTheDocument();
  });

  it('loads and expands immutable plan history versions', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_revision_dashboard') {
        return Promise.resolve(DASHBOARD);
      }
      if (command === 'get_revision_daily_plan') {
        return Promise.resolve(EMPTY_DAILY_PLAN);
      }
      if (command === 'get_revision_interval_preview') {
        return Promise.resolve(INTERVAL_PREVIEW);
      }
      if (command === 'get_revision_plan_history') {
        return Promise.resolve([
          {
            planId: 1,
            planDate: TODAY,
            version: 1,
            status: 'superseded',
            effectiveBudgetMinutes: 90,
            maxTopics: 5,
            plannedMinutes: 25,
            completedMinutes: 10,
            createdAt: '2026-10-30T09:00:00.000Z',
            lastRebalancedAt: '2026-10-30T10:00:00.000Z',
            memberships: [PLAN_ITEM_A],
          },
        ]);
      }
      if (command === 'get_revision_history') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<RevisionScreen today={TODAY} />);
    await screen.findByText('Revision plan history and workload versions');
    await userEvent.click(screen.getByText('Revision plan history and workload versions'));
    await userEvent.click(screen.getByRole('button', { name: 'Load last 30 days' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('get_revision_plan_history', {
        fromDate: '2026-09-30',
        toDate: TODAY,
      }),
    );
    expect(await screen.findByText(/version 1/)).toBeInTheDocument();
    await userEvent.click(screen.getByText('Memberships (1)'));
    expect(screen.getAllByText(/MMP-C01 · Dimensional analysis/).length).toBeGreaterThan(0);
  });

  it('refreshes the dashboard and daily plan when the local date changes', async () => {
    const tomorrow = '2026-10-31';
    let dashboardCalls = 0;
    let planCalls = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_revision_dashboard') {
        dashboardCalls += 1;
        return Promise.resolve(
          dashboardCalls === 1 ? DASHBOARD : { ...DASHBOARD, today: tomorrow },
        );
      }
      if (command === 'get_revision_daily_plan') {
        planCalls += 1;
        return Promise.resolve(
          planCalls === 1 ? GENERATED_DAILY_PLAN : { ...EMPTY_DAILY_PLAN, today: tomorrow },
        );
      }
      if (command === 'get_revision_interval_preview') {
        return Promise.resolve(INTERVAL_PREVIEW);
      }
      if (command === 'get_revision_history') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    const { rerender } = render(<RevisionScreen today={TODAY} />);
    const planRegion = await screen.findByRole('region', { name: "Today's revision plan" });
    expect(within(planRegion).getByText(/Version 1/)).toBeInTheDocument();

    rerender(<RevisionScreen today={tomorrow} />);

    await waitFor(() => expect(dashboardCalls).toBe(2));
    await waitFor(() => expect(planCalls).toBe(2));
    expect(await screen.findByText(/No plan has been generated for today/)).toBeInTheDocument();
    expect(screen.queryByText(/Version 1/)).not.toBeInTheDocument();
  });

  it('shows a loading state while the dashboard command is pending', () => {
    invokeMock.mockReturnValue(new Promise(() => {}));
    render(<RevisionScreen today={TODAY} />);
    expect(screen.getByRole('status')).toHaveTextContent('Loading revision schedule');
  });

  it('shows explicit empty queue and recommendation states', async () => {
    mockDashboardBackend({
      ...DASHBOARD,
      stats: {
        scheduledCount: 0,
        activeCount: 0,
        suspendedCount: 0,
        dueTodayCount: 0,
        overdueCount: 0,
        completedToday: 0,
        completedLast7Days: 0,
        totalCompleted: 0,
      },
      dueToday: [],
      overdue: [],
      upcoming: [],
      suspended: [],
      completedRecent: [],
      topics: [],
      archivedTopics: [],
      recommendations: [],
    });
    render(<RevisionScreen today={TODAY} />);
    expect(await screen.findByText('No overdue revisions.')).toBeInTheDocument();
    expect(screen.getByText('Nothing due today.')).toBeInTheDocument();
    expect(screen.getByText('No revisions scheduled in the next 30 days.')).toBeInTheDocument();
    expect(
      screen.getByText('No recommendation currently meets the transparent thresholds.'),
    ).toBeInTheDocument();
    expect(screen.getByText('No completed revisions in the last 14 days.')).toBeInTheDocument();
    expect(screen.getByText('No topics match these filters')).toBeInTheDocument();
  });

  it('shows an error state and retries the dashboard', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_revision_dashboard') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve(DASHBOARD);
      }
      if (command === 'get_revision_history') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    render(<RevisionScreen today={TODAY} />);
    expect(await screen.findByText('Revision scheduling could not be loaded')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('Overdue revisions')).toBeInTheDocument();
    expect(attempts).toBe(2);
  });
});
