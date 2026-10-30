import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  RevisionDashboard,
  RevisionHistoryEntry,
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
};

const TOPIC_B: RevisionTopic = {
  id: 2,
  code: 'GA-01',
  title: 'Logical reasoning',
  archived: false,
  sectionCode: 'GA',
  sectionTitle: 'General Aptitude',
};

const TOPIC_C: RevisionTopic = {
  id: 3,
  code: 'CM-C01',
  title: 'Newtonian mechanics',
  archived: false,
  sectionCode: 'CM',
  sectionTitle: 'Classical Mechanics',
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
    updatedAt: '2026-10-25T10:00:00.000Z',
    ...overrides,
  };
}

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
  recommendations: [
    {
      topic: TOPIC_A,
      scheduled: true,
      dueState: 'due_today',
      nextRevisionDate: TODAY,
      previousOutcome: 'good',
      masteryEstimate: 60,
      pyqAttempts: 6,
      pyqAccuracy: 33.3,
      pyqSufficient: true,
      priority: 'high',
      score: 100,
      reasons: ['Due today', 'PYQ accuracy 33.3% over 6 scored attempts'],
    },
    {
      topic: TOPIC_C,
      scheduled: false,
      dueState: 'not_scheduled',
      nextRevisionDate: null,
      previousOutcome: null,
      masteryEstimate: null,
      pyqAttempts: 2,
      pyqAccuracy: 0,
      pyqSufficient: false,
      priority: 'medium',
      score: 50,
      reasons: [
        'Not scheduled yet',
        'Only 2 scored PYQ attempts; insufficient evidence for a weak-topic claim',
      ],
    },
  ],
  mockScopeNote:
    'Mock tests are stored at part level and do not map to individual syllabus topics.',
  mockTestCount: 2,
};

function mockRevisionBackend() {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'get_revision_dashboard') {
      return Promise.resolve(DASHBOARD);
    }
    if (command === 'get_revision_history') {
      return Promise.resolve(args?.topicId === TOPIC_A.id ? [HISTORY] : []);
    }
    if (command === 'complete_revision') {
      return Promise.resolve({
        state: STATE_A,
        history: HISTORY,
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
    expect(screen.getAllByText(/PYQ accuracy 33.3%/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/insufficient evidence/).length).toBeGreaterThan(0);
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

  it('shows a loading state while the dashboard command is pending', () => {
    invokeMock.mockReturnValue(new Promise(() => {}));

    render(<RevisionScreen today={TODAY} />);

    expect(screen.getByRole('status')).toHaveTextContent('Loading revision schedule');
  });

  it('shows explicit empty queue and recommendation states', async () => {
    invokeMock.mockResolvedValue({
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
