import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  SessionTopic,
  StudySession,
  StudySessionInput,
  StudyTotals,
} from '../lib/sessionsApi';
import SessionsScreen from './SessionsScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-12-10';
const RANGE_FROM = '2026-11-11';

const TOPIC: SessionTopic = {
  id: 1,
  code: 'GA-01',
  title: 'Logical reasoning',
  archived: false,
};

const ARCHIVED_TOPIC: SessionTopic = {
  id: 2,
  code: 'MMP-C01',
  title: 'Dimensional analysis',
  archived: true,
};

const TIMER_SESSION: StudySession = {
  id: 10,
  topic: TOPIC,
  sessionDate: '2026-12-10',
  startedAt: '2026-12-10T04:00:00.000Z',
  endedAt: '2026-12-10T04:25:00.000Z',
  durationSeconds: 1_500,
  source: 'timer',
  notes: 'Derived units practice',
  createdAt: '2026-12-10T04:25:01.000Z',
  updatedAt: '2026-12-10T04:25:01.000Z',
};

const ARCHIVED_SESSION: StudySession = {
  id: 11,
  topic: ARCHIVED_TOPIC,
  sessionDate: '2026-12-09',
  startedAt: null,
  endedAt: null,
  durationSeconds: 600,
  source: 'manual',
  notes: '',
  createdAt: '2026-12-09T12:00:00.000Z',
  updatedAt: '2026-12-09T12:00:00.000Z',
};

const SESSIONS = [TIMER_SESSION, ARCHIVED_SESSION];

const TOTALS: StudyTotals = {
  rangeFrom: RANGE_FROM,
  rangeTo: TODAY,
  rangeSeconds: 2_100,
  rangeSessionCount: 2,
  todayDate: TODAY,
  todaySeconds: 1_500,
  weekStart: '2026-12-07',
  weekEnd: '2026-12-13',
  weekSeconds: 2_100,
  daily: [
    { date: '2026-12-09', seconds: 600, sessionCount: 1 },
    { date: '2026-12-10', seconds: 1_500, sessionCount: 1 },
  ],
  weekly: [
    {
      weekStart: '2026-12-07',
      weekEnd: '2026-12-13',
      seconds: 2_100,
      sessionCount: 2,
    },
  ],
};

const EMPTY_TOTALS: StudyTotals = {
  ...TOTALS,
  rangeSeconds: 0,
  rangeSessionCount: 0,
  todaySeconds: 0,
  weekSeconds: 0,
  daily: [],
  weekly: [],
};

type MockHandler = (args?: Record<string, unknown>) => unknown;

function manualSessionFromInput(
  id: number,
  input: StudySessionInput,
  topic: SessionTopic | null,
): StudySession {
  return {
    id,
    topic,
    sessionDate: input.sessionDate,
    startedAt: null,
    endedAt: null,
    durationSeconds: input.durationSeconds,
    source: 'manual',
    notes: input.notes,
    createdAt: '2026-12-10T12:00:00.000Z',
    updatedAt: '2026-12-10T12:00:00.000Z',
  };
}

function mockCommands(overrides: Record<string, MockHandler> = {}) {
  const handlers: Record<string, MockHandler> = {
    get_session_topics: () => [TOPIC, ARCHIVED_TOPIC],
    get_study_sessions: () => SESSIONS,
    get_study_totals: () => TOTALS,
    create_manual_session: (args) => {
      const input = args?.session as StudySessionInput;
      const topic = input.topicId === TOPIC.id ? TOPIC : null;
      return manualSessionFromInput(20, input, topic);
    },
    update_study_session: (args) => {
      const input = args?.session as StudySessionInput;
      const topic = input.topicId === TOPIC.id ? TOPIC : null;
      return manualSessionFromInput(Number(args?.sessionId), input, topic);
    },
    delete_study_session: () => undefined,
    ...overrides,
  };

  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    const handler = handlers[command];
    if (!handler) {
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    }

    return Promise.resolve(handler(args));
  });
}

function callsFor(command: string) {
  return invokeMock.mock.calls.filter(([name]) => name === command);
}

describe('study sessions screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('loads history grouped by date with daily, weekly, and range totals', async () => {
    mockCommands();

    render(<SessionsScreen today={TODAY} />);

    expect(
      await screen.findByRole('heading', { name: 'Study-time history', level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Sessions (2)' })).toBeInTheDocument();
    expect(screen.getByText('Derived units practice')).toBeInTheDocument();
    expect(screen.getByText('Archived topic')).toBeInTheDocument();
    expect(screen.getByText('GA-01')).toBeInTheDocument();
    expect(screen.getByText('MMP-C01')).toBeInTheDocument();
    expect(screen.getByText('Timer', { selector: '.session-source' })).toBeInTheDocument();
    expect(screen.getByText('Manual', { selector: '.session-source' })).toBeInTheDocument();

    const rangeCard = screen
      .getByText('Selected range')
      .closest('.session-total-card') as HTMLElement | null;
    const todayCard = screen
      .getByText('Today')
      .closest('.session-total-card') as HTMLElement | null;
    const weekCard = screen
      .getByText('This week')
      .closest('.session-total-card') as HTMLElement | null;

    expect(rangeCard).not.toBeNull();
    expect(todayCard).not.toBeNull();
    expect(weekCard).not.toBeNull();
    expect(within(rangeCard!).getByText('35m')).toBeInTheDocument();
    expect(within(todayCard!).getByText('25m')).toBeInTheDocument();
    expect(within(weekCard!).getByText('35m')).toBeInTheDocument();
    expect(screen.getByText('Daily totals (2)')).toBeInTheDocument();
    expect(screen.getByText('Weekly totals (1)')).toBeInTheDocument();
  });

  it('shows the topic filter and history in an honest empty state', async () => {
    mockCommands({
      get_study_sessions: () => [],
      get_study_totals: () => EMPTY_TOTALS,
      get_session_topics: () => [],
    });

    render(<SessionsScreen today={TODAY} />);

    expect(await screen.findByText('No study sessions in this range')).toBeInTheDocument();
    expect(screen.getByText('Daily totals (0)')).toBeInTheDocument();
    expect(screen.getByText('No focus time in this range.')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'All topics' })).toBeInTheDocument();
  });

  it('keeps archived topics out of new entries but preserves one while editing its history', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });

    const editorTopic = screen.getByLabelText('Syllabus topic (optional)');
    expect(
      within(editorTopic).queryByRole('option', { name: /Dimensional analysis/ }),
    ).not.toBeInTheDocument();
    expect(
      within(editorTopic).getByRole('option', { name: /Logical reasoning/ }),
    ).toBeInTheDocument();

    await user.click(screen.getAllByRole('button', { name: 'Edit' })[1]);

    expect(await screen.findByRole('heading', { name: 'Edit study session' })).toBeInTheDocument();
    expect(screen.getByLabelText('Syllabus topic (optional)')).toHaveValue('2');
    expect(screen.getByLabelText('Session date')).toBeEnabled();
    expect(screen.getByLabelText('Minutes')).toBeEnabled();
    expect(screen.getByLabelText('Seconds')).toBeEnabled();
    expect(
      within(screen.getByLabelText('Syllabus topic (optional)')).getByRole('option', {
        name: /Dimensional analysis/,
      }),
    ).toBeInTheDocument();
  });

  it('shows a loading state while sessions are pending', () => {
    mockCommands({
      get_study_sessions: () => new Promise<StudySession[]>(() => {}),
    });

    render(<SessionsScreen today={TODAY} />);

    expect(screen.getByText('Loading study sessions.')).toBeInTheDocument();
    expect(screen.queryByText('No study sessions in this range')).not.toBeInTheDocument();
  });

  it('shows a retryable load error', async () => {
    let attempts = 0;
    mockCommands({
      get_study_sessions: () => {
        attempts += 1;
        return attempts === 1 ? Promise.reject(new Error('database unavailable')) : SESSIONS;
      },
    });

    render(<SessionsScreen today={TODAY} />);

    expect(await screen.findByText('Study sessions could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByRole('heading', { name: 'Sessions (2)' })).toBeInTheDocument();
    expect(callsFor('get_study_sessions')).toHaveLength(2);
  });

  it('creates a manual session with a topic, duration, and notes', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    await user.selectOptions(screen.getByLabelText('Syllabus topic (optional)'), '1');
    fireEvent.change(screen.getByLabelText('Minutes'), { target: { value: '45' } });
    fireEvent.change(screen.getByLabelText('Seconds'), { target: { value: '30' } });
    fireEvent.change(screen.getByLabelText('Notes (optional)'), {
      target: { value: 'Rotational motion revision' },
    });

    await user.click(screen.getByRole('button', { name: 'Save manual session' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_manual_session', {
        session: {
          topicId: 1,
          sessionDate: TODAY,
          durationSeconds: 2_730,
          notes: 'Rotational motion revision',
        },
      }),
    );
    expect(await screen.findByText('Manual study session saved.')).toBeInTheDocument();
    expect(screen.getByLabelText('Minutes')).toHaveValue(25);
    expect(screen.getByLabelText('Notes (optional)')).toHaveValue('');
  });

  it('blocks a zero-length manual session before invoking the backend', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    fireEvent.change(screen.getByLabelText('Minutes'), { target: { value: '0' } });
    await user.click(screen.getByRole('button', { name: 'Save manual session' }));

    expect(screen.getByText('Duration must be between 1 second and 24 hours.')).toBeInTheDocument();
    expect(callsFor('create_manual_session')).toHaveLength(0);
  });

  it('reports a backend validation error on the duration field', async () => {
    mockCommands({
      create_manual_session: () =>
        Promise.reject({
          code: 'validation_error',
          message: 'Duration must be between 1 second and 24 hours.',
          field: 'durationSeconds',
        }),
    });
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    await user.click(screen.getByRole('button', { name: 'Save manual session' }));

    const alerts = await screen.findAllByRole('alert');
    expect(alerts.length).toBeGreaterThanOrEqual(2);
    for (const alert of alerts) {
      expect(alert).toHaveTextContent('Duration must be between 1 second and 24 hours.');
    }
  });

  it('edits an existing session and keeps its topic', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    await user.click(screen.getAllByRole('button', { name: 'Edit' })[0]);

    expect(screen.getByRole('heading', { name: 'Edit study session' })).toBeInTheDocument();
    expect(screen.getByLabelText('Session date')).toHaveValue(TODAY);
    expect(screen.getByLabelText('Session date')).toBeDisabled();
    expect(screen.getByLabelText('Minutes')).toHaveValue(25);
    expect(screen.getByLabelText('Minutes')).toBeDisabled();
    expect(screen.getByLabelText('Seconds')).toHaveValue(0);
    expect(screen.getByLabelText('Seconds')).toBeDisabled();
    expect(screen.getByLabelText('Syllabus topic (optional)')).toHaveValue('1');

    fireEvent.change(screen.getByLabelText('Notes (optional)'), {
      target: { value: 'Updated note' },
    });
    await user.click(screen.getByRole('button', { name: 'Save changes' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_study_session', {
        sessionId: 10,
        session: {
          topicId: 1,
          sessionDate: TODAY,
          durationSeconds: 1_500,
          notes: 'Updated note',
        },
      }),
    );
    expect(
      await screen.findByText('Study session updated. Daily and weekly totals were recalculated.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Add a study session' })).toBeInTheDocument();
  });

  it('requires confirmation before deleting and recalculates the history', async () => {
    let sessions = [...SESSIONS];
    mockCommands({
      get_study_sessions: () => sessions,
      delete_study_session: (args) => {
        sessions = sessions.filter((session) => session.id !== args?.sessionId);
        return undefined;
      },
    });
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    await user.click(screen.getAllByRole('button', { name: 'Delete' })[0]);

    expect(screen.getByRole('alertdialog')).toHaveTextContent('Delete this session permanently?');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(callsFor('delete_study_session')).toHaveLength(0);

    await user.click(screen.getAllByRole('button', { name: 'Delete' })[0]);
    await user.click(screen.getByRole('button', { name: 'Confirm delete' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('delete_study_session', { sessionId: 10 }),
    );
    expect(
      await screen.findByText('Study session deleted. Daily and weekly totals were recalculated.'),
    ).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Sessions (1)' })).toBeInTheDocument();
    expect(screen.queryByText('Derived units practice')).not.toBeInTheDocument();
    expect(screen.getByText('Dimensional analysis')).toBeInTheDocument();
  });

  it('applies date and topic filters to both history and totals', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    expect(invokeMock).toHaveBeenCalledWith('get_study_sessions', {
      filter: { fromDate: RANGE_FROM, toDate: TODAY },
    });
    expect(invokeMock).toHaveBeenCalledWith('get_study_totals', {
      fromDate: RANGE_FROM,
      toDate: TODAY,
      topicId: null,
    });

    fireEvent.change(screen.getByLabelText('From date'), {
      target: { value: '2026-12-01' },
    });
    await user.selectOptions(screen.getByLabelText('Topic'), '2');
    await user.click(screen.getByRole('button', { name: 'Apply filters' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('get_study_sessions', {
        filter: { fromDate: '2026-12-01', toDate: TODAY, topicId: 2 },
      }),
    );
    expect(invokeMock).toHaveBeenCalledWith('get_study_totals', {
      fromDate: '2026-12-01',
      toDate: TODAY,
      topicId: 2,
    });
    expect(
      await screen.findByText('Filtered to MMP-C01 · Dimensional analysis'),
    ).toBeInTheDocument();
  });

  it('rejects an inverted date range without reloading history', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<SessionsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Sessions (2)' });
    fireEvent.change(screen.getByLabelText('From date'), {
      target: { value: '2026-12-20' },
    });
    await user.click(screen.getByRole('button', { name: 'Apply filters' }));

    expect(screen.getByText('The start date cannot be after the end date.')).toBeInTheDocument();
    expect(callsFor('get_study_sessions')).toHaveLength(1);
  });
});
