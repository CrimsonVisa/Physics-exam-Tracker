import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type {
  Agenda,
  ApplyExamDateChangeInput,
  CalendarData,
  ExamReschedulePreview,
} from './lib/plannerApi';
import type { Settings } from './lib/settingsApi';
import type { StudyTotals } from './lib/sessionsApi';
import type { TimerSnapshot } from './lib/timerApi';

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

const DEFAULT_SETTINGS: Settings = {
  examDate: '2026-12-19',
  preparationStartDate: '2026-10-29',
  dailyStudyTargetMinutes: 240,
};

const TODAY = '2026-12-10';

const EMPTY_TOTALS: StudyTotals = {
  rangeFrom: TODAY,
  rangeTo: TODAY,
  rangeSeconds: 0,
  rangeSessionCount: 0,
  todayDate: TODAY,
  todaySeconds: 0,
  weekStart: '2026-12-07',
  weekEnd: '2026-12-13',
  weekSeconds: 0,
  daily: [],
  weekly: [],
};

const EMPTY_AGENDA: Agenda = {
  today: TODAY,
  todayTasks: [],
  todayBlocks: [],
  upcomingDeadlines: [],
  overdueTasks: [],
};

const EMPTY_CALENDAR: CalendarData = {
  fromDate: TODAY,
  toDate: TODAY,
  today: TODAY,
  tasks: [],
  blocks: [],
  deadlines: [],
  upcomingDeadlines: [],
  overdueTasks: [],
};

function previewFor(settings: Settings): ExamReschedulePreview {
  return {
    currentExamDate: DEFAULT_SETTINGS.examDate,
    proposedExamDate: settings.examDate,
    proposedPreparationStartDate: settings.preparationStartDate,
    proposedDailyStudyTargetMinutes: settings.dailyStudyTargetMinutes,
    changes: [],
    outsideWindow: [],
    completedTasksUntouched: 0,
    manualTasksUntouched: 0,
    plannedBlocksUntouched: 0,
  };
}

const TIMER_SNAPSHOT: TimerSnapshot = {
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
    recovery: null,
  },
  settings: {
    focusMinutes: 25,
    shortBreakMinutes: 5,
    longBreakMinutes: 15,
    notificationsEnabled: false,
  },
  serverNowMs: 1_800_000_000_000,
};

type MockHandler = (args?: Record<string, unknown>) => unknown;

function mockBackend(overrides: Record<string, MockHandler> = {}) {
  const handlers: Record<string, MockHandler> = {
    get_settings: () => DEFAULT_SETTINGS,
    update_settings: (args) => args?.settings,
    preview_exam_date_change: (args) => previewFor(args?.settings as Settings),
    apply_exam_date_change: (args) => {
      const change = args?.change as ApplyExamDateChangeInput;
      return {
        settings: change.settings,
        appliedTaskCount: change.acceptedChanges.length,
        preservedOutsideWindowCount: 0,
      };
    },
    get_agenda: () => EMPTY_AGENDA,
    get_calendar: () => EMPTY_CALENDAR,
    get_checklist_tasks: () => [],
    get_planner_topics: () => [],
    get_study_totals: () => EMPTY_TOTALS,
    get_syllabus_catalog: () => ({ sources: [], sections: [] }),
    get_syllabus_summary: () => ({
      totalTopics: 0,
      statusCounts: { notStarted: 0, learning: 0, revised: 0, confident: 0 },
      approximateProgress: 0,
      sections: [],
    }),
    search_syllabus_topics: () => [],
    get_timer_snapshot: () => TIMER_SNAPSHOT,
    get_study_sessions: () => [],
    get_session_topics: () => [],
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

function renderApp(today = TODAY) {
  return render(<App today={today} />);
}

function callsFor(command: string) {
  return invokeMock.mock.calls.filter(([name]) => name === command);
}

describe('settings dashboard', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('loads persisted settings and shows the countdown, dates, target, provisional warning, and study total', async () => {
    mockBackend();

    renderApp();

    expect(await screen.findByText('days remaining')).toBeInTheDocument();
    expect(screen.getByText('9')).toHaveClass('countdown-value');
    expect(screen.getByText('19 December 2026')).toBeInTheDocument();
    expect(screen.getByText('29 October 2026')).toBeInTheDocument();
    expect(screen.getByText('4 hours per day')).toBeInTheDocument();
    expect(screen.getByText(/19 December is a provisional default/)).toBeInTheDocument();
    expect(
      screen.getByText(/Confirm the actual subject date on your admit card/),
    ).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('get_settings');
    expect(await screen.findByText("Today's study time")).toBeInTheDocument();
    expect(await screen.findByText('of 4h daily target')).toBeInTheDocument();
    expect(
      await screen.findByRole('heading', { name: /Plan for 10 December 2026/ }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Only completed focus sessions appear in study totals/),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Exam countdown', level: 1 })).toBeInTheDocument();
    expect(screen.getByText('CSIR-NET')).toBeInTheDocument();
    expect(screen.getByText('Physics Tracker')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Primary navigation' })).toBeInTheDocument();
  });

  it('shows a loading state while the SQLite-backed settings command is pending', () => {
    invokeMock.mockReturnValue(new Promise<Settings>(() => {}));

    renderApp();

    expect(screen.getByRole('status')).toHaveTextContent('Loading local settings');
    expect(screen.queryByRole('button', { name: 'Save settings' })).not.toBeInTheDocument();
  });

  it('shows a zero-day countdown on exam day', async () => {
    mockBackend({ get_settings: () => ({ ...DEFAULT_SETTINGS, examDate: '2026-12-19' }) });

    renderApp('2026-12-19');

    expect(await screen.findByText('Today is exam day. Focus on the paper.')).toBeInTheDocument();
    expect(screen.getByText('0', { selector: '.countdown-value' })).toBeInTheDocument();
    expect(screen.getByText('days remaining')).toBeInTheDocument();
  });

  it('describes a passed exam date without a negative countdown', async () => {
    mockBackend({ get_settings: () => ({ ...DEFAULT_SETTINGS, examDate: '2026-12-01' }) });

    renderApp();

    expect(await screen.findByText('Exam date has passed')).toBeInTheDocument();
    expect(screen.getByText('9 days ago')).toBeInTheDocument();
    expect(
      screen.getByText(/Update the exam date when you know the date for the next attempt/),
    ).toBeInTheDocument();
  });

  it('describes a single elapsed day correctly', async () => {
    mockBackend({ get_settings: () => ({ ...DEFAULT_SETTINGS, examDate: '2026-12-09' }) });

    renderApp();

    expect(await screen.findByText('1 day ago')).toBeInTheDocument();
  });

  it('previews an exam-date change and applies the accepted payload only after confirmation', async () => {
    mockBackend();

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });

    expect(screen.getByText('10')).toHaveClass('countdown-value');
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(
      await screen.findByRole('heading', { name: 'Review the exam-date change' }),
    ).toBeInTheDocument();
    expect(screen.getByText('Nothing saved yet')).toBeInTheDocument();
    expect(callsFor('preview_exam_date_change')).toHaveLength(1);
    expect(callsFor('update_settings')).toHaveLength(0);
    expect(callsFor('apply_exam_date_change')).toHaveLength(0);

    await userEvent.click(screen.getByRole('button', { name: 'Apply exam-date change' }));

    expect(
      await screen.findByText('Exam date updated. No exam-anchored task dates needed to move.'),
    ).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('apply_exam_date_change', {
      change: {
        settings: {
          examDate: '2026-12-20',
          preparationStartDate: '2026-10-29',
          dailyStudyTargetMinutes: 240,
        },
        expectedExamDate: '2026-12-19',
        acceptedChanges: [],
      },
    });
    expect(screen.getByText('All changes saved')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save settings' })).toBeDisabled();
  });

  it('cancels an exam-date preview without saving anything', async () => {
    mockBackend();

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));
    await screen.findByRole('heading', { name: 'Review the exam-date change' });

    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.getByText('Exam-date change cancelled. Nothing was saved.')).toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Review the exam-date change' }),
    ).not.toBeInTheDocument();
    expect(callsFor('apply_exam_date_change')).toHaveLength(0);
    expect(callsFor('update_settings')).toHaveLength(0);
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
    expect(screen.getByText('10')).toHaveClass('countdown-value');
  });

  it('lists anchored changes and warnings before applying them', async () => {
    mockBackend({
      preview_exam_date_change: (args) => ({
        ...previewFor(args?.settings as Settings),
        changes: [
          {
            taskId: 7,
            title: 'Revision plan',
            currentScheduledDate: '2026-12-09',
            proposedScheduledDate: '2027-01-05',
            offsetDays: -10,
            outsideWindow: false,
          },
        ],
        outsideWindow: [
          {
            kind: 'planned_block',
            id: 4,
            title: 'Late block',
            date: '2027-02-01',
            reason: 'block_after_exam_date',
          },
        ],
        completedTasksUntouched: 2,
        manualTasksUntouched: 3,
        plannedBlocksUntouched: 1,
      }),
    });

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2027-01-15' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(await screen.findByText('Revision plan')).toBeInTheDocument();
    expect(screen.getByText('Late block')).toBeInTheDocument();
    expect(screen.getByText('Completed tasks preserved').parentElement).toHaveTextContent('2');
    expect(screen.getByText('Manual tasks preserved').parentElement).toHaveTextContent('3');

    await userEvent.click(screen.getByRole('button', { name: 'Apply exam-date change' }));

    expect(
      await screen.findByText(
        'Exam date updated. 1 exam-anchored task was moved after confirmation.',
      ),
    ).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('apply_exam_date_change', {
      change: {
        settings: {
          examDate: '2027-01-15',
          preparationStartDate: '2026-10-29',
          dailyStudyTargetMinutes: 240,
        },
        expectedExamDate: '2026-12-19',
        acceptedChanges: [{ taskId: 7, expectedScheduledDate: '2026-12-09' }],
      },
    });
  });

  it('reports a stale preview without leaving a half-applied confirmation open', async () => {
    mockBackend({
      apply_exam_date_change: () =>
        Promise.reject({
          code: 'reschedule_stale',
          message: 'The exam date or an anchored task changed after the preview.',
        }),
    });

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));
    await screen.findByRole('heading', { name: 'Review the exam-date change' });
    await userEvent.click(screen.getByRole('button', { name: 'Apply exam-date change' }));

    expect(
      await screen.findByText('The exam date or an anchored task changed after the preview.'),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Review the exam-date change' }),
    ).not.toBeInTheDocument();
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
  });

  it('saves an updated daily target with its hours hint', async () => {
    mockBackend();

    renderApp();

    const dailyTarget = (await screen.findByLabelText('Daily study target')) as HTMLInputElement;
    fireEvent.change(dailyTarget, { target: { value: '300' } });

    expect(screen.getByText('300 minutes is 5 hours per day.')).toBeInTheDocument();
    expect(screen.getByText('5 hours per day')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_settings', {
        settings: {
          examDate: '2026-12-19',
          preparationStartDate: '2026-10-29',
          dailyStudyTargetMinutes: 300,
        },
      }),
    );
    expect(await screen.findByText('5 hours per day')).toBeInTheDocument();
  });

  it('disables the controls and shows progress while a settings save is in flight', async () => {
    let resolveSave!: (settings: Settings) => void;
    const pendingSave = new Promise<Settings>((resolve) => {
      resolveSave = resolve;
    });
    mockBackend({ update_settings: () => pendingSave });

    renderApp();

    const dailyTarget = (await screen.findByLabelText('Daily study target')) as HTMLInputElement;
    fireEvent.change(dailyTarget, { target: { value: '300' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(screen.getByRole('button', { name: 'Saving…' })).toBeDisabled();
    expect(dailyTarget).toBeDisabled();
    expect(screen.getByText('Writing to SQLite…')).toBeInTheDocument();

    await act(async () => {
      resolveSave({ ...DEFAULT_SETTINGS, dailyStudyTargetMinutes: 300 });
      await pendingSave;
    });

    expect(await screen.findByText('Settings saved to the local database.')).toBeInTheDocument();
    expect(dailyTarget).toBeEnabled();
  });

  it('keeps the form open and reports a backend preview error on the responsible field', async () => {
    mockBackend({
      preview_exam_date_change: () =>
        Promise.reject({
          code: 'validation_error',
          message: 'Enter a real calendar date in YYYY-MM-DD format.',
          field: 'examDate',
        }),
    });

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    const alerts = await screen.findAllByRole('alert');
    expect(alerts).toHaveLength(2);
    for (const alert of alerts) {
      expect(alert).toHaveTextContent('Enter a real calendar date in YYYY-MM-DD format.');
    }
    expect(examDate).toHaveAttribute('aria-invalid', 'true');
    expect(examDate).toHaveFocus();
  });

  it('blocks an empty date locally before invoking the backend', async () => {
    mockBackend();

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(await screen.findByText('Choose a real calendar date.')).toBeInTheDocument();
    expect(examDate).toHaveAttribute('aria-invalid', 'true');
    expect(callsFor('update_settings')).toHaveLength(0);
  });

  it('does not allow a preparation start date after the exam date', async () => {
    mockBackend();

    renderApp();

    const preparationStart = (await screen.findByLabelText(
      'Preparation start date',
    )) as HTMLInputElement;
    fireEvent.change(preparationStart, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(
      await screen.findByText('Preparation start date cannot be after the exam date.'),
    ).toBeInTheDocument();
    expect(callsFor('update_settings')).toHaveLength(0);
  });

  it('navigates between the dashboard and the interactive syllabus screen', async () => {
    mockBackend();

    renderApp();

    expect(await screen.findByText('days remaining')).toBeInTheDocument();
    const overviewNav = screen.getByRole('button', { name: 'Overview' });
    const syllabusNav = screen.getByRole('button', { name: 'Syllabus' });
    expect(overviewNav).toHaveAttribute('aria-current', 'page');
    expect(syllabusNav).not.toHaveAttribute('aria-current');

    await userEvent.click(syllabusNav);

    expect(
      await screen.findByRole('heading', { name: 'Physical Sciences syllabus', level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Syllabus' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('button', { name: 'Overview' })).not.toHaveAttribute('aria-current');

    await userEvent.click(screen.getByRole('button', { name: 'Overview' }));

    expect(
      await screen.findByRole('heading', { name: 'Exam countdown', level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Overview' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(callsFor('get_settings')).toHaveLength(1);
  });

  it('navigates to the timer and sessions screens', async () => {
    mockBackend();

    renderApp();

    expect(await screen.findByText('days remaining')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Timer' }));
    expect(
      await screen.findByRole('heading', { name: 'Focused study blocks', level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Timer' })).toHaveAttribute('aria-current', 'page');

    await userEvent.click(screen.getByRole('button', { name: 'Sessions' }));
    expect(
      await screen.findByRole('heading', { name: 'Study-time history', level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByText('No study sessions in this range')).toBeInTheDocument();
  });

  it('navigates to the checklist and calendar screens', async () => {
    mockBackend();

    renderApp();

    expect(await screen.findByText('days remaining')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Checklist' }));
    expect(
      await screen.findByRole('heading', { name: 'Tasks and next actions', level: 1 }),
    ).toBeInTheDocument();
    expect(await screen.findByText('No open tasks')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Checklist' })).toHaveAttribute(
      'aria-current',
      'page',
    );

    await userEvent.click(screen.getByRole('button', { name: 'Calendar' }));
    expect(
      await screen.findByRole('heading', { name: 'Plan the study week', level: 1 }),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Calendar' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(callsFor('get_calendar').length).toBeGreaterThan(0);
  });

  it('shows a load error and retries with a fresh command call', async () => {
    let attempts = 0;
    mockBackend({
      get_settings: () => {
        attempts += 1;
        return attempts === 1 ? Promise.reject(new Error('boom')) : DEFAULT_SETTINGS;
      },
    });

    renderApp();

    expect(await screen.findByText('Settings could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('days remaining')).toBeInTheDocument();
    expect(callsFor('get_settings')).toHaveLength(2);
  });
});
