import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ChecklistTask, TaskFilter } from '../lib/plannerApi';
import ChecklistScreen from './ChecklistScreen';

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
    description: 'Work through the exercise sheet.',
    scheduledDate: '2026-10-30',
    deadlineDate: null,
    topic: null,
    priority: 'Normal',
    status: 'open',
    scheduleAnchor: 'manual',
    scheduleOffsetDays: null,
    completedAt: null,
    createdAt: '2026-10-29T10:00:00.000Z',
    updatedAt: '2026-10-29T10:00:00.000Z',
    dueDate: '2026-10-30',
    isOverdue: false,
    isUpcoming: true,
    ...overrides,
  };
}

function filterFromArgs(args?: Record<string, unknown>): TaskFilter {
  return (args?.filter as TaskFilter | undefined) ?? {};
}

describe('checklist screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows a loading state while tasks are being read', () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      return new Promise(() => {});
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(screen.getByRole('status')).toHaveTextContent('Loading checklist tasks');
  });

  it('shows distinct empty states for open, completed, and all filters', async () => {
    const tasks: ChecklistTask[] = [];
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === 'get_checklist_tasks') {
        const filter = filterFromArgs(args);
        const status = filter.status ?? 'open';
        return Promise.resolve(
          status === 'all' ? tasks : tasks.filter((task) => task.status === status),
        );
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(await screen.findByText('No open tasks')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Completed' }));
    expect(await screen.findByText('No completed tasks yet')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'All' }));
    expect(await screen.findByText('No checklist tasks yet')).toBeInTheDocument();
  });

  it('renders overdue, upcoming, syllabus, and completed task details', async () => {
    const overdue = makeTask({
      id: 1,
      title: 'Overdue reading',
      scheduledDate: '2026-10-28',
      dueDate: '2026-10-28',
      isOverdue: true,
      isUpcoming: false,
    });
    const upcoming = makeTask({
      id: 2,
      title: 'Upcoming revision',
      deadlineDate: '2026-11-02',
      dueDate: '2026-11-02',
      topic: { id: 7, code: 'GA-01', title: 'Probability', archived: false },
      priority: 'High-Yield',
    });
    const completed = makeTask({
      id: 3,
      title: 'Finished derivation',
      status: 'completed',
      completedAt: '2026-10-30T09:15:00.000Z',
    });

    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === 'get_checklist_tasks') {
        const status = filterFromArgs(args).status ?? 'open';
        return Promise.resolve(
          status === 'all'
            ? [overdue, upcoming, completed]
            : [overdue, upcoming, completed].filter((task) => task.status === status),
        );
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(await screen.findByText('Overdue reading')).toBeInTheDocument();
    expect(screen.getByText('Overdue')).toBeInTheDocument();
    expect(screen.getByText('Upcoming revision')).toBeInTheDocument();
    expect(screen.getByText('Due soon')).toBeInTheDocument();
    expect(screen.getByText('Deadline 2 November 2026')).toBeInTheDocument();
    expect(screen.getByText('GA-01 · Probability')).toBeInTheDocument();
    expect(screen.getByText('High-Yield', { selector: '.priority-badge' })).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'All' }));

    expect(await screen.findByText('Finished derivation')).toBeInTheDocument();
    expect(screen.getByText('Completed', { selector: '.task-status' })).toBeInTheDocument();
    expect(screen.getAllByText(/Completed /).length).toBeGreaterThan(0);
  });

  it('completes and reopens a task without losing the completion timestamp', async () => {
    let tasks = [makeTask()];
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === 'get_checklist_tasks') {
        const status = filterFromArgs(args).status ?? 'open';
        return Promise.resolve(
          status === 'all' ? tasks : tasks.filter((task) => task.status === status),
        );
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'complete_checklist_task') {
        tasks = tasks.map((task) =>
          task.id === 1
            ? {
                ...task,
                status: 'completed',
                completedAt: '2026-10-30T10:00:00.000Z',
                updatedAt: '2026-10-30T10:00:00.000Z',
              }
            : task,
        );
        return Promise.resolve(tasks[0]);
      }
      if (command === 'reopen_checklist_task') {
        tasks = tasks.map((task) =>
          task.id === 1 ? { ...task, status: 'open', completedAt: null } : task,
        );
        return Promise.resolve(tasks[0]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByText('Revise probability');
    await userEvent.click(screen.getByRole('button', { name: 'All' }));
    await screen.findByText('Revise probability');
    await userEvent.click(screen.getByRole('button', { name: 'Complete' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('complete_checklist_task', { taskId: 1 }),
    );
    expect(
      await screen.findByText('Task completed. Its completion timestamp was saved.'),
    ).toBeInTheDocument();
    const reopen = await screen.findByRole('button', { name: 'Reopen' });

    await userEvent.click(reopen);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('reopen_checklist_task', { taskId: 1 }),
    );
    expect(
      await screen.findByText('Task reopened. The previous completion timestamp was cleared.'),
    ).toBeInTheDocument();
  });

  it('reschedules an open task through the inline date form', async () => {
    const task = makeTask();
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === 'get_checklist_tasks') {
        const status = filterFromArgs(args).status ?? 'open';
        return Promise.resolve(status === 'all' || status === 'open' ? [task] : []);
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'reschedule_checklist_task') {
        return Promise.resolve({ ...task, scheduledDate: args?.scheduledDate as string });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByText('Revise probability');
    await userEvent.click(screen.getByRole('button', { name: 'Reschedule' }));

    const dateInput = screen.getByLabelText('New scheduled date') as HTMLInputElement;
    fireEvent.change(dateInput, { target: { value: '2026-11-10' } });
    await userEvent.click(screen.getByRole('button', { name: 'Save new date' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('reschedule_checklist_task', {
        taskId: 1,
        scheduledDate: '2026-11-10',
      }),
    );
    expect(
      await screen.findByText(
        'Open task rescheduled and switched to a manual date. Completed tasks never move.',
      ),
    ).toBeInTheDocument();
  });

  it('requires confirmation before deleting a task', async () => {
    const task = makeTask();
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_checklist_tasks') {
        return Promise.resolve([task]);
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'delete_checklist_task') {
        return Promise.resolve();
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    const title = await screen.findByText('Revise probability');
    const row = title.closest('.task-row');
    expect(row).not.toBeNull();

    await userEvent.click(within(row as HTMLElement).getByRole('button', { name: 'Delete' }));

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent('Delete this task permanently?');
    expect(invokeMock).not.toHaveBeenCalledWith('delete_checklist_task', expect.anything());

    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm delete' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('delete_checklist_task', { taskId: 1 }),
    );
    expect(await screen.findByText('Checklist task deleted.')).toBeInTheDocument();
  });

  it('creates a task with the typed date, priority, description, and topic', async () => {
    let tasks: ChecklistTask[] = [];
    invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
      if (command === 'get_checklist_tasks') {
        return Promise.resolve(tasks);
      }
      if (command === 'get_planner_topics') {
        return Promise.resolve([{ id: 7, code: 'GA-01', title: 'Probability', archived: false }]);
      }
      if (command === 'create_checklist_task') {
        const input = args?.task as {
          title: string;
          description: string;
          scheduledDate: string;
          topicId: number | null;
          priority: string;
        };
        tasks = [
          makeTask({
            id: 9,
            title: input.title,
            description: input.description,
            scheduledDate: input.scheduledDate,
            topic: { id: 7, code: 'GA-01', title: 'Probability', archived: false },
            priority: input.priority as ChecklistTask['priority'],
          }),
        ];
        return Promise.resolve(tasks[0]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    await screen.findByText('No open tasks');
    await screen.findByRole('option', { name: 'GA-01 · Probability' });

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'New probability drill' },
    });
    fireEvent.change(screen.getByLabelText('Description (optional)'), {
      target: { value: 'Twenty questions' },
    });
    fireEvent.change(screen.getByLabelText('Scheduled date'), {
      target: { value: '2026-11-03' },
    });
    fireEvent.change(screen.getByLabelText('Syllabus topic (optional)'), {
      target: { value: '7' },
    });
    fireEvent.change(screen.getByLabelText('Priority'), { target: { value: 'High' } });
    await userEvent.click(screen.getByRole('button', { name: 'Add task' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_checklist_task', {
        task: {
          title: 'New probability drill',
          description: 'Twenty questions',
          scheduledDate: '2026-11-03',
          deadlineDate: null,
          topicId: 7,
          priority: 'High',
          scheduleAnchor: 'manual',
          scheduleOffsetDays: null,
        },
      }),
    );
    expect(
      await screen.findByText('“New probability drill” added to the checklist.'),
    ).toBeInTheDocument();
    expect(await screen.findByText('New probability drill')).toBeInTheDocument();
  });

  it('applies and clears the scheduled-date range filter', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'get_checklist_tasks') {
        return Promise.resolve([makeTask()]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);
    await screen.findByText('Revise probability');

    fireEvent.change(screen.getByLabelText('Scheduled from (optional)'), {
      target: { value: '2026-10-01' },
    });
    fireEvent.change(screen.getByLabelText('Scheduled to (optional)'), {
      target: { value: '2026-10-31' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Apply dates' }));

    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([name]) => name === 'get_checklist_tasks');
      expect(filterFromArgs(calls.at(-1)?.[1])).toEqual({
        status: 'open',
        fromDate: '2026-10-01',
        toDate: '2026-10-31',
      });
    });

    await userEvent.click(screen.getByRole('button', { name: 'Clear dates' }));
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([name]) => name === 'get_checklist_tasks');
      expect(filterFromArgs(calls.at(-1)?.[1])).toEqual({ status: 'open' });
    });
  });

  it('shows a load error and retries with a fresh task fetch', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_planner_topics') {
        return Promise.resolve([]);
      }
      if (command === 'get_checklist_tasks') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<ChecklistScreen today={TODAY} examDate={EXAM_DATE} />);

    expect(await screen.findByText('Checklist tasks could not be loaded')).toBeInTheDocument();
    expect(
      screen.getByText('The local database is unavailable. Open the desktop app and try again.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('No open tasks')).toBeInTheDocument();
    expect(attempts).toBe(2);
  });
});
