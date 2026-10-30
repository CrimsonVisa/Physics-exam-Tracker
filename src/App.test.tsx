import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import type { Settings } from './lib/settingsApi';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const DEFAULT_SETTINGS: Settings = {
  examDate: '2026-12-19',
  preparationStartDate: '2026-10-29',
  dailyStudyTargetMinutes: 240,
};

const TODAY = '2026-12-10';

function renderApp(today = TODAY) {
  return render(<App today={today} />);
}

describe('settings dashboard', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('loads persisted settings and shows the countdown, dates, target, and provisional warning', async () => {
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS);

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
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS);

    renderApp('2026-12-19');

    expect(await screen.findByText('Today is exam day. Focus on the paper.')).toBeInTheDocument();
    expect(screen.getByText('0')).toHaveClass('countdown-value');
    expect(screen.getByText('days remaining')).toBeInTheDocument();
  });

  it('describes a passed exam date without a negative countdown', async () => {
    invokeMock.mockResolvedValueOnce({ ...DEFAULT_SETTINGS, examDate: '2026-12-01' });

    renderApp();

    expect(await screen.findByText('Exam date has passed')).toBeInTheDocument();
    expect(screen.getByText('9 days ago')).toBeInTheDocument();
    expect(
      screen.getByText(/Update the exam date when you know the date for the next attempt/),
    ).toBeInTheDocument();
  });

  it('describes a single elapsed day correctly', async () => {
    invokeMock.mockResolvedValueOnce({ ...DEFAULT_SETTINGS, examDate: '2026-12-09' });

    renderApp();

    expect(await screen.findByText('1 day ago')).toBeInTheDocument();
  });

  it('updates the preview as the exam date changes and saves the typed settings payload', async () => {
    const updated: Settings = { ...DEFAULT_SETTINGS, examDate: '2026-12-20' };
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS).mockResolvedValueOnce(updated);

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });

    expect(screen.getByText('10')).toHaveClass('countdown-value');
    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(await screen.findByText('Settings saved to the local database.')).toBeInTheDocument();
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'update_settings', {
      settings: {
        examDate: '2026-12-20',
        preparationStartDate: '2026-10-29',
        dailyStudyTargetMinutes: 240,
      },
    });
    expect(screen.getByText('All changes saved')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save settings' })).toBeDisabled();
  });

  it('saves an updated daily target with its hours hint', async () => {
    const updated: Settings = { ...DEFAULT_SETTINGS, dailyStudyTargetMinutes: 300 };
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS).mockResolvedValueOnce(updated);

    renderApp();

    const dailyTarget = (await screen.findByLabelText('Daily study target')) as HTMLInputElement;
    fireEvent.change(dailyTarget, { target: { value: '300' } });

    expect(screen.getByText('300 minutes is 5 hours per day.')).toBeInTheDocument();
    expect(screen.getByText('5 hours per day')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenNthCalledWith(2, 'update_settings', {
        settings: {
          examDate: '2026-12-19',
          preparationStartDate: '2026-10-29',
          dailyStudyTargetMinutes: 300,
        },
      }),
    );
    expect(await screen.findByText('5 hours per day')).toBeInTheDocument();
  });

  it('disables the controls and shows progress while a save is in flight', async () => {
    let resolveSave!: (settings: Settings) => void;
    const pendingSave = new Promise<Settings>((resolve) => {
      resolveSave = resolve;
    });
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS).mockReturnValueOnce(pendingSave);

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(screen.getByRole('button', { name: 'Saving…' })).toBeDisabled();
    expect(examDate).toBeDisabled();
    expect(screen.getByText('Writing to SQLite…')).toBeInTheDocument();

    await act(async () => {
      resolveSave({ ...DEFAULT_SETTINGS, examDate: '2026-12-20' });
      await pendingSave;
    });

    expect(await screen.findByText('Settings saved to the local database.')).toBeInTheDocument();
    expect(examDate).toBeEnabled();
  });

  it('keeps the form open and reports a backend validation error on the responsible field', async () => {
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS).mockRejectedValueOnce({
      code: 'validation_error',
      message: 'Enter a real calendar date in YYYY-MM-DD format.',
      field: 'examDate',
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
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS);

    renderApp();

    const examDate = (await screen.findByLabelText('Exam date')) as HTMLInputElement;
    fireEvent.change(examDate, { target: { value: '' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(await screen.findByText('Choose a real calendar date.')).toBeInTheDocument();
    expect(examDate).toHaveAttribute('aria-invalid', 'true');
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('does not allow a preparation start date after the exam date', async () => {
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS);

    renderApp();

    const preparationStart = (await screen.findByLabelText(
      'Preparation start date',
    )) as HTMLInputElement;
    fireEvent.change(preparationStart, { target: { value: '2026-12-20' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save settings' }));

    expect(
      await screen.findByText('Preparation start date cannot be after the exam date.'),
    ).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it('navigates between the dashboard and the interactive syllabus screen', async () => {
    invokeMock.mockImplementation((command: string) => {
      switch (command) {
        case 'get_settings':
          return Promise.resolve(DEFAULT_SETTINGS);
        case 'get_syllabus_catalog':
          return Promise.resolve({ sources: [], sections: [] });
        case 'get_syllabus_summary':
          return Promise.resolve({
            totalTopics: 0,
            statusCounts: { notStarted: 0, learning: 0, revised: 0, confident: 0 },
            approximateProgress: 0,
            sections: [],
          });
        case 'search_syllabus_topics':
          return Promise.resolve([]);
        default:
          return Promise.reject(new Error(`Unexpected command: ${command}`));
      }
    });

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
    expect(invokeMock.mock.calls.filter(([name]) => name === 'get_settings')).toHaveLength(1);
  });

  it('shows a load error and retries with a fresh command call', async () => {
    invokeMock.mockRejectedValueOnce(new Error('boom')).mockResolvedValueOnce(DEFAULT_SETTINGS);

    renderApp();

    expect(await screen.findByText('Settings could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('days remaining')).toBeInTheDocument();
    expect(invokeMock).toHaveBeenNthCalledWith(1, 'get_settings');
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'get_settings');
  });
});
