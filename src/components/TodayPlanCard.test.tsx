import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Agenda } from '../lib/plannerApi';
import { TodayPlanCard } from './TodayPlanCard';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';

const AGENDA: Agenda = {
  today: TODAY,
  todayTasks: [
    {
      id: 1,
      title: 'Revise probability',
      description: '',
      scheduledDate: TODAY,
      deadlineDate: '2026-11-02',
      topic: null,
      priority: 'High',
      status: 'open',
      scheduleAnchor: 'manual',
      scheduleOffsetDays: null,
      completedAt: null,
      createdAt: '2026-10-29T10:00:00.000Z',
      updatedAt: '2026-10-29T10:00:00.000Z',
      dueDate: '2026-11-02',
      isOverdue: false,
      isUpcoming: true,
    },
  ],
  todayBlocks: [
    {
      id: 2,
      title: 'Electrodynamics block',
      topic: null,
      calendarDate: TODAY,
      startTime: '10:00',
      endTime: '11:30',
      status: 'planned',
      notes: '',
      linkedSessionId: null,
      createdAt: '2026-10-29T10:00:00.000Z',
      updatedAt: '2026-10-29T10:00:00.000Z',
    },
  ],
  upcomingDeadlines: [],
  overdueTasks: [],
};

describe('dashboard today plan card', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows today’s blocks and tasks with a clear planned-time warning', async () => {
    invokeMock.mockResolvedValue(AGENDA);
    const onOpenChecklist = vi.fn();
    const onOpenCalendar = vi.fn();

    render(
      <TodayPlanCard
        today={TODAY}
        onOpenChecklist={onOpenChecklist}
        onOpenCalendar={onOpenCalendar}
      />,
    );

    expect(await screen.findByText('Electrodynamics block')).toBeInTheDocument();
    expect(screen.getByText('10:00–11:30')).toBeInTheDocument();
    expect(screen.getByText('Revise probability')).toBeInTheDocument();
    expect(screen.getByText('Deadline 2 November 2026')).toBeInTheDocument();
    expect(screen.getByText(/Planned blocks are intentions, not study time/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Open checklist' }));
    await userEvent.click(screen.getByRole('button', { name: 'Open calendar' }));

    expect(onOpenChecklist).toHaveBeenCalledTimes(1);
    expect(onOpenCalendar).toHaveBeenCalledTimes(1);
  });

  it('shows an empty plan and reports a load error with retry', async () => {
    let attempts = 0;
    invokeMock.mockImplementation(() => {
      attempts += 1;
      return attempts === 1
        ? Promise.reject(new Error('database unavailable'))
        : Promise.resolve({ ...AGENDA, todayTasks: [], todayBlocks: [] });
    });

    const view = render(
      <TodayPlanCard today={TODAY} onOpenChecklist={vi.fn()} onOpenCalendar={vi.fn()} />,
    );

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The local database is unavailable. Open the desktop app and try again.',
    );

    await userEvent.click(screen.getByRole('button', { name: 'Retry plan' }));

    expect(await screen.findByText(/Nothing scheduled for today yet/)).toBeInTheDocument();
    expect(attempts).toBe(2);

    view.unmount();
  });

  it('shows a loading state while the agenda command is pending', () => {
    invokeMock.mockReturnValue(new Promise(() => {}));

    render(<TodayPlanCard today={TODAY} onOpenChecklist={vi.fn()} onOpenCalendar={vi.fn()} />);

    expect(screen.getByRole('status')).toHaveTextContent(
      "Loading today's tasks and planned blocks",
    );
  });

  it('updates the error state when the agenda command fails after retry', async () => {
    invokeMock.mockRejectedValue(new Error('offline'));

    render(<TodayPlanCard today={TODAY} onOpenChecklist={vi.fn()} onOpenCalendar={vi.fn()} />);

    await waitFor(() => expect(screen.getByRole('alert')).toBeInTheDocument());
    expect(screen.queryByText('Electrodynamics block')).not.toBeInTheDocument();
  });
});
