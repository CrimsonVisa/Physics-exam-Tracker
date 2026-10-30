import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StudyTotals } from '../lib/sessionsApi';
import { StudyTodayCard } from './StudyTodayCard';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-12-10';

const TOTALS: StudyTotals = {
  rangeFrom: TODAY,
  rangeTo: TODAY,
  rangeSeconds: 5_400,
  rangeSessionCount: 2,
  todayDate: TODAY,
  todaySeconds: 5_400,
  weekStart: '2026-12-07',
  weekEnd: '2026-12-13',
  weekSeconds: 14_400,
  daily: [{ date: TODAY, seconds: 5_400, sessionCount: 2 }],
  weekly: [
    {
      weekStart: '2026-12-07',
      weekEnd: '2026-12-13',
      seconds: 14_400,
      sessionCount: 4,
    },
  ],
};

describe('dashboard study card', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows today’s focus time against the daily target', async () => {
    invokeMock.mockResolvedValue(TOTALS);
    const onOpenSessions = vi.fn();

    render(<StudyTodayCard today={TODAY} targetMinutes={240} onOpenSessions={onOpenSessions} />);

    expect(await screen.findByText('1h 30m')).toBeInTheDocument();
    expect(screen.getByText('of 4h daily target')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '38');
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('4h')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'View sessions' }));
    expect(onOpenSessions).toHaveBeenCalledTimes(1);
  });

  it('shows a loading state while totals are pending', () => {
    invokeMock.mockReturnValue(new Promise<StudyTotals>(() => {}));

    render(<StudyTodayCard today={TODAY} targetMinutes={240} onOpenSessions={vi.fn()} />);

    expect(screen.getByText('Loading study-time totals…')).toBeInTheDocument();
  });

  it('shows a retryable error without hiding the card', async () => {
    invokeMock
      .mockRejectedValueOnce(new Error('database unavailable'))
      .mockResolvedValueOnce(TOTALS);

    render(<StudyTodayCard today={TODAY} targetMinutes={240} onOpenSessions={vi.fn()} />);

    expect(
      await screen.findByText(
        'The local database is unavailable. Open the desktop app and try again.',
      ),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry totals' }));

    expect(await screen.findByText('1h 30m')).toBeInTheDocument();
  });
});
