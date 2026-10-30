import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CalendarData, ChecklistTask, PlannedBlock } from '../lib/plannerApi';
import CalendarScreen from './CalendarScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';
const EXAM_DATE = '2026-12-19';

function makeTask(overrides: Partial<ChecklistTask> = {}): ChecklistTask {
  return {
    id: 1,
    title: 'Revise probability',
    description: '',
    scheduledDate: TODAY,
    deadlineDate: null,
    topic: null,
    priority: 'Normal',
    status: 'open',
    scheduleAnchor: 'manual',
    scheduleOffsetDays: null,
    completedAt: null,
    createdAt: '2026-10-29T10:00:00.000Z',
    updatedAt: '2026-10-29T10:00:00.000Z',
    dueDate: TODAY,
    isOverdue: false,
    isUpcoming: true,
    ...overrides,
  };
}

function makeBlock(overrides: Partial<PlannedBlock> = {}): PlannedBlock {
  return {
    id: 1,
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
    ...overrides,
  };
}

function makeCalendar(overrides: Partial<CalendarData> = {}): CalendarData {
  return {
    fromDate: '2026-09-28',
    toDate: '2026-11-01',
    today: TODAY,
    tasks: [],
    blocks: [],
    deadlines: [],
    upcomingDeadlines: [],
    overdueTasks: [],
    ...overrides,
  };
}

function mockCalendar(calendar: CalendarData | (() => CalendarData) = makeCalendar()) {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'get_calendar') {
      const data = typeof calendar === 'function' ? calendar() : calendar;
      return Promise.resolve({
        ...data,
        fromDate: args?.fromDate as string,
        toDate: args?.toDate as string,
      });
    }
    if (command === 'get_planner_topics') {
      return Promise.resolve([{ id: 7, code: 'GA-01', title: 'Probability', archived: false }]);
    }
    if (command === 'get_study_sessions') {
      return Promise.resolve([]);
    }
    if (command === 'create_checklist_task') {
      const task = args?.task as { title: string; scheduledDate: string };
      return Promise.resolve(
        makeTask({ id: 20, title: task.title, scheduledDate: task.scheduledDate }),
      );
    }
    if (command === 'create_planned_block') {
      return Promise.resolve(makeBlock({ id: 20, title: 'New calendar block' }));
    }
    if (command === 'delete_planned_block') {
      return Promise.resolve();
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

describe('calendar screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows a month view with selected-day empty states and deadline summaries', async () => {
    mockCalendar();

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(await screen.findByRole('heading', { name: 'October 2026' })).toBeInTheDocument();
    expect(
      screen.getByText(/No tasks or planned blocks in this calendar range/),
    ).toBeInTheDocument();
    expect(screen.getByText('No open tasks with upcoming deadlines.')).toBeInTheDocument();
    expect(screen.getByText('No overdue open tasks.')).toBeInTheDocument();
    expect(
      screen.getByRole('button', {
        name: 'Friday, 30 October 2026, 0 tasks, 0 planned blocks',
      }),
    ).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('heading', { name: 'Friday, 30 October 2026' })).toBeInTheDocument();
    expect(screen.getByText('No checklist tasks scheduled.')).toBeInTheDocument();
    expect(screen.getByText('No planned study blocks.')).toBeInTheDocument();

    await userEvent.click(
      screen.getByRole('button', {
        name: 'Saturday, 31 October 2026, 0 tasks, 0 planned blocks',
      }),
    );
    expect(screen.getByRole('heading', { name: 'Saturday, 31 October 2026' })).toBeInTheDocument();
  });

  it('shows a loading state while the calendar range is being read', () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      return new Promise(() => {});
    });

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(screen.getByRole('status')).toHaveTextContent('Loading the calendar');
  });

  it('jumps to a manually entered date', async () => {
    mockCalendar();

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);
    await screen.findByRole('heading', { name: 'October 2026' });

    fireEvent.change(screen.getByLabelText('Jump to date'), {
      target: { value: '2026-12-19' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Go' }));

    expect(await screen.findByRole('heading', { name: 'December 2026' })).toBeInTheDocument();
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_calendar');
      expect(calls.at(-1)?.[1]).toEqual({ fromDate: '2026-11-30', toDate: '2027-01-03' });
    });
  });

  it('navigates months, weeks, days, and back to today with local date boundaries', async () => {
    mockCalendar();

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByRole('heading', { name: 'October 2026' });

    await userEvent.click(screen.getByRole('button', { name: 'Next month' }));
    expect(await screen.findByRole('heading', { name: 'November 2026' })).toBeInTheDocument();
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_calendar');
      expect(calls.at(-1)?.[1]).toEqual({ fromDate: '2026-10-26', toDate: '2026-12-06' });
    });

    await userEvent.click(screen.getByRole('button', { name: 'Today' }));
    expect(await screen.findByRole('heading', { name: 'October 2026' })).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Week' }));
    expect(
      await screen.findByRole('heading', { name: '26 October 2026 – 1 November 2026' }),
    ).toBeInTheDocument();
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_calendar');
      expect(calls.at(-1)?.[1]).toEqual({ fromDate: '2026-10-26', toDate: '2026-11-01' });
    });

    await userEvent.click(screen.getByRole('button', { name: 'Day' }));
    expect(
      (await screen.findAllByRole('heading', { name: 'Friday, 30 October 2026' })).length,
    ).toBeGreaterThan(0);
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_calendar');
      expect(calls.at(-1)?.[1]).toEqual({ fromDate: TODAY, toDate: TODAY });
    });
  });

  it('renders tasks, blocks, deadlines, and overdue items on the selected day', async () => {
    const task = makeTask({
      title: 'Revise probability',
      deadlineDate: '2026-11-05',
      dueDate: '2026-11-05',
      topic: { id: 7, code: 'GA-01', title: 'Probability', archived: false },
    });
    const todayDeadline = makeTask({
      id: 3,
      title: 'Today deadline task',
      deadlineDate: TODAY,
      dueDate: TODAY,
    });
    const block = makeBlock({ title: 'Electrodynamics block' });
    const overdue = makeTask({
      id: 2,
      title: 'Late reading',
      scheduledDate: '2026-10-25',
      dueDate: '2026-10-25',
      isOverdue: true,
      isUpcoming: false,
    });

    mockCalendar(
      makeCalendar({
        tasks: [task, todayDeadline, overdue],
        blocks: [block],
        deadlines: [todayDeadline, task],
        upcomingDeadlines: [todayDeadline, task],
        overdueTasks: [overdue],
      }),
    );

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByRole('heading', { name: 'October 2026' });
    const selectedHeading = await screen.findByRole('heading', {
      name: 'Friday, 30 October 2026',
    });
    const selectedDay = selectedHeading.closest('section');
    expect(selectedDay).not.toBeNull();

    expect(within(selectedDay as HTMLElement).getByText('Revise probability')).toBeInTheDocument();
    expect(
      within(selectedDay as HTMLElement).getByText('Electrodynamics block'),
    ).toBeInTheDocument();
    expect(
      within(selectedDay as HTMLElement).getByText('10:00–11:30 · Planned'),
    ).toBeInTheDocument();
    expect(
      within(selectedDay as HTMLElement).getByText('Deadlines on this day'),
    ).toBeInTheDocument();
    expect(
      within(selectedDay as HTMLElement).getAllByText('Today deadline task').length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText('Late reading').length).toBeGreaterThan(0);
    expect(screen.getByText('Due 25 October 2026')).toBeInTheDocument();
    expect(screen.getAllByText('Revise probability').length).toBeGreaterThan(1);
    expect(screen.getByText('5 November 2026 · GA-01')).toBeInTheDocument();
  });

  it('opens the task editor for the selected date and creates a task', async () => {
    mockCalendar();

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByRole('heading', { name: 'October 2026' });
    await userEvent.click(screen.getByRole('button', { name: 'Add task here' }));

    expect(screen.getByRole('heading', { name: 'Add a task for this day' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'New calendar task' },
    });
    fireEvent.change(screen.getByLabelText('Scheduled date'), {
      target: { value: '2026-11-01' },
    });

    const editor = screen
      .getByRole('heading', { name: 'Add a task for this day' })
      .closest('section') as HTMLElement;
    await userEvent.click(within(editor).getByRole('button', { name: 'Add task' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_checklist_task', {
        task: {
          title: 'New calendar task',
          description: '',
          scheduledDate: '2026-11-01',
          deadlineDate: null,
          topicId: null,
          priority: 'Normal',
          scheduleAnchor: 'manual',
          scheduleOffsetDays: null,
        },
      }),
    );
    expect(
      await screen.findByText('“New calendar task” added for 1 November 2026.'),
    ).toBeInTheDocument();
  });

  it('opens the planned-block editor and saves a same-day time range', async () => {
    mockCalendar();

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByRole('heading', { name: 'October 2026' });
    await userEvent.click(screen.getByRole('button', { name: 'Add planned block' }));

    expect(screen.getByRole('heading', { name: 'Plan a study block' })).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Planned-block title'), {
      target: { value: 'New calendar block' },
    });
    fireEvent.change(screen.getByLabelText('Start time'), { target: { value: '13:00' } });
    fireEvent.change(screen.getByLabelText('End time'), { target: { value: '14:30' } });
    fireEvent.change(screen.getByLabelText('Notes (optional)'), {
      target: { value: 'Two chapters' },
    });

    const editor = screen
      .getByRole('heading', { name: 'Plan a study block' })
      .closest('section') as HTMLElement;
    await userEvent.click(within(editor).getByRole('button', { name: 'Add planned block' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_planned_block', {
        block: {
          title: 'New calendar block',
          topicId: null,
          calendarDate: TODAY,
          startTime: '13:00',
          endTime: '14:30',
          status: 'planned',
          notes: 'Two chapters',
          linkedSessionId: null,
        },
      }),
    );
    expect(
      await screen.findByText('“New calendar block” planned for 30 October 2026.'),
    ).toBeInTheDocument();
  });

  it('requires confirmation before deleting a planned block', async () => {
    mockCalendar(makeCalendar({ blocks: [makeBlock({ title: 'Delete me block' })] }));

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByRole('heading', { name: 'October 2026' });
    await userEvent.click(screen.getByRole('button', { name: 'Delete block' }));

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent('Delete this planned block permanently?');
    expect(invokeMock).not.toHaveBeenCalledWith('delete_planned_block', expect.anything());

    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm delete block' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('delete_planned_block', { blockId: 1 }),
    );
    expect(await screen.findByText('Planned study block deleted.')).toBeInTheDocument();
  });

  it('shows an error and retries the calendar fetch', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'get_calendar') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve(makeCalendar());
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<CalendarScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(await screen.findByText('The calendar could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    await waitFor(() => expect(attempts).toBe(2));
    await waitFor(() =>
      expect(screen.queryByText('The calendar could not be loaded')).not.toBeInTheDocument(),
    );
    expect(screen.getByRole('heading', { name: 'October 2026' })).toBeInTheDocument();
  });
});
