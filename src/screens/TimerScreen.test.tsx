import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SyllabusTopic } from '../lib/syllabusApi';
import type { TimerSettings, TimerSnapshot, TimerState } from '../lib/timerApi';
import TimerScreen from './TimerScreen';

const { invokeMock, isPermissionGrantedMock, requestPermissionMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isPermissionGrantedMock: vi.fn(),
  requestPermissionMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: isPermissionGrantedMock,
  requestPermission: requestPermissionMock,
}));

const TOPIC: SyllabusTopic = {
  id: 1,
  code: 'GA-01',
  sectionCode: 'GA',
  sectionTitle: 'General Aptitude',
  title: 'Logical reasoning',
  category: 'General Aptitude',
  sourceCode: 'nta-csir-net-dec-2026',
  sourceTitle: 'Joint CSIR-UGC NET December 2026 Information Bulletin',
  sourceUrl: 'https://example.gov.in/bulletin.pdf',
  sourceReference: 'NTA December 2026 bulletin — General Aptitude',
  progress: {
    status: 'Not started',
    priority: 'Normal',
    confidence: 3,
    notes: '',
    updatedAt: null,
    recorded: false,
  },
};

const SETTINGS: TimerSettings = {
  focusMinutes: 25,
  shortBreakMinutes: 5,
  longBreakMinutes: 15,
  notificationsEnabled: false,
};

const SERVER_NOW = 1_800_000_000_000;

function makeSnapshot(
  state: Partial<TimerState> = {},
  settings: TimerSettings = SETTINGS,
): TimerSnapshot {
  return {
    state: {
      status: 'idle',
      mode: 'focus',
      durationMs: 25 * 60_000,
      accumulatedMs: 0,
      remainingMs: 25 * 60_000,
      firstStartedAt: null,
      sessionDate: null,
      topic: null,
      lastEvent: null,
      lastEventAt: null,
      lastEventDurationMs: 0,
      lastCompletedSessionId: null,
      revisionPlanItemId: null,
      recovery: null,
      ...state,
    },
    settings,
    serverNowMs: SERVER_NOW,
  };
}

const RUNNING: Partial<TimerState> = {
  status: 'running',
  firstStartedAt: '2026-12-10T04:00:00.000Z',
  sessionDate: '2026-12-10',
  topic: { id: 1, code: 'GA-01', title: 'Logical reasoning', archived: false },
};

const PAUSED: Partial<TimerState> = {
  status: 'paused',
  accumulatedMs: 10 * 60_000,
  remainingMs: 15 * 60_000,
  firstStartedAt: '2026-12-10T04:00:00.000Z',
  sessionDate: '2026-12-10',
};

type MockHandler = (args?: Record<string, unknown>) => unknown;

function mockCommands(overrides: Record<string, MockHandler> = {}) {
  const handlers: Record<string, MockHandler> = {
    get_timer_snapshot: () => makeSnapshot(),
    search_syllabus_topics: () => [TOPIC],
    start_study_timer: () => makeSnapshot(RUNNING),
    pause_study_timer: () => makeSnapshot(PAUSED),
    resume_study_timer: () => makeSnapshot(RUNNING),
    finish_study_timer: () =>
      makeSnapshot({
        lastEvent: 'completed',
        lastEventAt: '2026-12-10T04:25:00.000Z',
        lastEventDurationMs: 25 * 60_000,
        lastCompletedSessionId: 7,
      }),
    discard_study_timer: () =>
      makeSnapshot({
        lastEvent: 'discarded',
        lastEventAt: '2026-12-10T04:05:00.000Z',
        lastEventDurationMs: 5 * 60_000,
      }),
    update_timer_settings: (args) => makeSnapshot({}, args?.settings as TimerSettings),
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

describe('study timer screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isPermissionGrantedMock.mockReset().mockResolvedValue(false);
    requestPermissionMock.mockReset().mockResolvedValue('denied');
  });

  it('loads the persisted timer, topic list, and duration settings', async () => {
    mockCommands();

    render(<TimerScreen />);

    expect(
      await screen.findByRole('heading', { name: 'Focused study blocks', level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByRole('timer')).toHaveTextContent('25:00');
    expect(screen.getByText('Ready')).toHaveClass('timer-status');
    expect(screen.getByRole('button', { name: 'Start focus' })).toBeInTheDocument();
    expect(screen.getByLabelText('Syllabus topic (optional)')).toHaveValue('');
    expect(screen.getByRole('option', { name: 'GA-01 · Logical reasoning' })).toBeInTheDocument();
    expect(screen.getByLabelText('Focus (minutes)')).toHaveValue(25);
    expect(screen.getByLabelText('Short break (minutes)')).toHaveValue(5);
    expect(screen.getByLabelText('Long break (minutes)')).toHaveValue(15);
    expect(screen.getByRole('button', { name: 'Save timer settings' })).toBeDisabled();
  });

  it('shows the configured duration for a fresh idle timer and when switching modes', async () => {
    mockCommands({
      get_timer_snapshot: () => makeSnapshot({ durationMs: 0, remainingMs: 0 }),
    });
    const user = userEvent.setup();

    render(<TimerScreen />);

    expect(await screen.findByRole('timer')).toHaveTextContent('25:00');

    await user.click(screen.getByRole('radio', { name: /Short break/ }));
    expect(screen.getByRole('timer')).toHaveTextContent('05:00');

    await user.click(screen.getByRole('radio', { name: /Long break/ }));
    expect(screen.getByRole('timer')).toHaveTextContent('15:00');
  });

  it('shows a loading state while the persisted timer is pending', () => {
    mockCommands({
      get_timer_snapshot: () => new Promise<TimerSnapshot>(() => {}),
    });

    render(<TimerScreen />);

    expect(screen.getByText('Loading the study timer.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Start focus' })).not.toBeInTheDocument();
  });

  it('shows a retryable load error', async () => {
    let attempts = 0;
    mockCommands({
      get_timer_snapshot: () => {
        attempts += 1;
        return attempts === 1 ? Promise.reject(new Error('database unavailable')) : makeSnapshot();
      },
    });

    render(<TimerScreen />);

    expect(await screen.findByText('The study timer could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByRole('button', { name: 'Start focus' })).toBeInTheDocument();
    expect(callsFor('get_timer_snapshot')).toHaveLength(2);
  });

  it('keeps the timer usable when the topic list fails and retries it', async () => {
    let attempts = 0;
    mockCommands({
      search_syllabus_topics: () => {
        attempts += 1;
        return attempts === 1 ? Promise.reject(new Error('topic failure')) : [TOPIC];
      },
    });

    render(<TimerScreen />);

    expect(await screen.findByText(/Topics could not be loaded/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Start focus' })).toBeEnabled();

    await userEvent.click(screen.getByRole('button', { name: 'Retry topics' }));

    expect(
      await screen.findByRole('option', { name: 'GA-01 · Logical reasoning' }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Topics could not be loaded/)).not.toBeInTheDocument();
  });

  it('starts a focus timer with the selected topic, pauses, resumes, and finishes it', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<TimerScreen />);

    await screen.findByRole('option', { name: 'GA-01 · Logical reasoning' });
    await user.selectOptions(screen.getByLabelText('Syllabus topic (optional)'), '1');
    await user.click(screen.getByRole('button', { name: 'Start focus' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('start_study_timer', {
        mode: 'focus',
        topicId: 1,
      }),
    );
    expect(await screen.findByRole('button', { name: 'Pause' })).toBeInTheDocument();
    expect(screen.getByText('Running')).toHaveClass('timer-status');

    await user.click(screen.getByRole('button', { name: 'Pause' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('pause_study_timer'));
    expect(await screen.findByRole('button', { name: 'Resume' })).toBeInTheDocument();
    expect(screen.getByText('Paused')).toHaveClass('timer-status');
    expect(screen.getByRole('timer')).toHaveTextContent('15:00');

    await user.click(screen.getByRole('button', { name: 'Resume' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('resume_study_timer'));
    expect(await screen.findByRole('button', { name: 'Pause' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Finish & save' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('finish_study_timer'));
    expect(await screen.findByText('Focus session saved')).toBeInTheDocument();
    expect(screen.getByText(/25m is now in Study sessions/)).toBeInTheDocument();
  });

  it('requires confirmation before discarding a timer and never saves discarded time', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<TimerScreen />);

    await screen.findByRole('button', { name: 'Start focus' });
    await user.click(screen.getByRole('button', { name: 'Start focus' }));
    await screen.findByRole('button', { name: 'Discard' });

    await user.click(screen.getByRole('button', { name: 'Discard' }));
    expect(screen.getByRole('alertdialog')).toHaveTextContent('Discard this timer?');

    await user.click(screen.getByRole('button', { name: 'Keep timer' }));
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
    expect(callsFor('discard_study_timer')).toHaveLength(0);

    await user.click(screen.getByRole('button', { name: 'Discard' }));
    await user.click(screen.getByRole('button', { name: 'Confirm discard' }));

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('discard_study_timer'));
    expect(await screen.findByText('Timer discarded')).toBeInTheDocument();
    expect(screen.getByText(/5m was not added to study totals/)).toBeInTheDocument();
  });

  it('shows restart recovery, excluded time, and saved focus time', async () => {
    mockCommands({
      get_timer_snapshot: () =>
        makeSnapshot({
          ...PAUSED,
          accumulatedMs: 10 * 60_000,
          recovery: {
            reason: 'restart',
            excludedMs: 2 * 60_000,
            recoveredAt: '2026-12-10T04:10:00.000Z',
          },
        }),
    });

    render(<TimerScreen />);

    expect(await screen.findByText('Timer recovered after restart')).toBeInTheDocument();
    expect(screen.getByText(/2m was excluded because no timer heartbeat/)).toBeInTheDocument();
    expect(screen.getByText(/Saved focus time: 10m/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Resume' })).toBeInTheDocument();
  });

  it('shows inactivity recovery as a paused timer', async () => {
    mockCommands({
      get_timer_snapshot: () =>
        makeSnapshot({
          ...PAUSED,
          recovery: {
            reason: 'inactivity',
            excludedMs: 60_000,
            recoveredAt: '2026-12-10T04:10:00.000Z',
          },
        }),
    });

    render(<TimerScreen />);

    expect(await screen.findByText('Timer paused after inactivity')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Resume' })).toBeInTheDocument();
  });

  it('validates and saves timer settings', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<TimerScreen />);

    const focusMinutes = await screen.findByLabelText('Focus (minutes)');
    fireEvent.change(focusMinutes, { target: { value: '0' } });
    await user.click(screen.getByRole('button', { name: 'Save timer settings' }));

    expect(screen.getByText('Enter a whole number of minutes from 1 to 240.')).toBeInTheDocument();
    expect(callsFor('update_timer_settings')).toHaveLength(0);

    fireEvent.change(focusMinutes, { target: { value: '30' } });
    await user.click(screen.getByRole('button', { name: 'Save timer settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_timer_settings', {
        settings: {
          focusMinutes: 30,
          shortBreakMinutes: 5,
          longBreakMinutes: 15,
          notificationsEnabled: false,
        },
      }),
    );
    expect(
      await screen.findByText('Timer settings saved. They apply to the next timer you start.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save timer settings' })).toBeDisabled();
  });

  it('does not save notification settings when Windows denies permission', async () => {
    mockCommands();
    const user = userEvent.setup();

    render(<TimerScreen />);

    await user.click(await screen.findByRole('checkbox', { name: /Desktop notifications/ }));
    await user.click(screen.getByRole('button', { name: 'Save timer settings' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Windows did not grant notification permission.',
    );
    expect(callsFor('update_timer_settings')).toHaveLength(0);
    expect(requestPermissionMock).toHaveBeenCalled();
  });

  it('saves notification settings after Windows grants permission', async () => {
    isPermissionGrantedMock.mockResolvedValue(true);
    mockCommands({
      update_timer_settings: (args) => makeSnapshot({}, args?.settings as TimerSettings),
    });
    const user = userEvent.setup();

    render(<TimerScreen />);

    await user.click(await screen.findByRole('checkbox', { name: /Desktop notifications/ }));
    await user.click(screen.getByRole('button', { name: 'Save timer settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_timer_settings', {
        settings: {
          focusMinutes: 25,
          shortBreakMinutes: 5,
          longBreakMinutes: 15,
          notificationsEnabled: true,
        },
      }),
    );
    expect(requestPermissionMock).not.toHaveBeenCalled();
    expect(await screen.findByText(/Timer settings saved/)).toBeInTheDocument();
  });
});
