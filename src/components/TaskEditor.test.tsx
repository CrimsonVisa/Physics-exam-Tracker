import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ChecklistTask } from '../lib/plannerApi';
import { TaskEditor } from './TaskEditor';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';
const EXAM_DATE = '2026-12-19';

const SAVED_TASK: ChecklistTask = {
  id: 5,
  title: 'Exam-relative revision',
  description: '',
  scheduledDate: '2026-12-09',
  deadlineDate: null,
  topic: null,
  priority: 'Normal',
  status: 'open',
  scheduleAnchor: 'exam',
  scheduleOffsetDays: -10,
  completedAt: null,
  createdAt: '2026-10-30T10:00:00.000Z',
  updatedAt: '2026-10-30T10:00:00.000Z',
  dueDate: '2026-12-09',
  isOverdue: false,
  isUpcoming: false,
};

describe('task editor', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('previews and saves an exam-relative offset using the shared date helpers', async () => {
    invokeMock.mockResolvedValue(SAVED_TASK);
    const onSaved = vi.fn();

    render(
      <TaskEditor
        task={null}
        topics={[]}
        defaultDate={TODAY}
        examDate={EXAM_DATE}
        onSaved={onSaved}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'Exam-relative revision' },
    });
    fireEvent.change(screen.getByLabelText('Schedule type'), { target: { value: 'exam' } });

    expect(
      screen.getByText(
        'Currently resolves to 9 December 2026. Negative values are before the exam.',
      ),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Days from the exam date'), {
      target: { value: '-10' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Add task' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_checklist_task', {
        task: {
          title: 'Exam-relative revision',
          description: '',
          scheduledDate: '',
          deadlineDate: null,
          topicId: null,
          priority: 'Normal',
          scheduleAnchor: 'exam',
          scheduleOffsetDays: -10,
        },
      }),
    );
    expect(onSaved).toHaveBeenCalledWith(SAVED_TASK, false);
  });

  it('rejects a deadline earlier than the scheduled date', async () => {
    render(
      <TaskEditor
        task={null}
        topics={[]}
        defaultDate={TODAY}
        examDate={EXAM_DATE}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'Deadline ordering' },
    });
    fireEvent.change(screen.getByLabelText('Scheduled date'), {
      target: { value: '2026-11-10' },
    });
    fireEvent.change(screen.getByLabelText('Deadline date (optional)'), {
      target: { value: '2026-11-09' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add task' }));

    expect(
      screen.getByText('The deadline cannot be earlier than the scheduled date.'),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('blocks a missing title before invoking the backend and shows field errors', async () => {
    invokeMock.mockRejectedValue({
      code: 'validation_error',
      message: 'Enter a task title.',
      field: 'title',
    });

    render(
      <TaskEditor
        task={null}
        topics={[]}
        defaultDate={TODAY}
        examDate={EXAM_DATE}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: 'Add task' }));

    expect(screen.getByText('Enter a task title.')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText('Task title'), {
      target: { value: 'Valid title' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Add task' }));

    const alerts = await screen.findAllByRole('alert');
    expect(alerts.some((alert) => alert.textContent === 'Enter a task title.')).toBe(true);
    expect(screen.getByLabelText('Task title')).toHaveAttribute('aria-invalid', 'true');
  });
});
