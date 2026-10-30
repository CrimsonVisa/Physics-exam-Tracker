import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PlannedBlock } from '../lib/plannerApi';
import { PlannedBlockEditor } from './PlannedBlockEditor';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';

const SAVED_BLOCK: PlannedBlock = {
  id: 8,
  title: 'Electrodynamics block',
  topic: null,
  calendarDate: TODAY,
  startTime: '10:00',
  endTime: '11:30',
  status: 'completed',
  notes: '',
  linkedSessionId: 12,
  createdAt: '2026-10-30T10:00:00.000Z',
  updatedAt: '2026-10-30T10:00:00.000Z',
};

describe('planned-block editor', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('links a completed block to an existing session only when the form is saved', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_study_sessions') {
        return Promise.resolve([
          {
            id: 12,
            topic: null,
            sessionDate: TODAY,
            startedAt: '2026-10-30T09:00:00.000Z',
            endedAt: '2026-10-30T09:30:00.000Z',
            durationSeconds: 1_800,
            source: 'manual',
            notes: 'Warm-up set',
            createdAt: '2026-10-30T09:30:00.000Z',
            updatedAt: '2026-10-30T09:30:00.000Z',
          },
        ]);
      }
      if (command === 'create_planned_block') {
        return Promise.resolve(SAVED_BLOCK);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    const onSaved = vi.fn();

    render(
      <PlannedBlockEditor
        block={null}
        topics={[]}
        defaultDate={TODAY}
        onSaved={onSaved}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Planned-block title'), {
      target: { value: 'Electrodynamics block' },
    });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'completed' } });

    const sessionSelect = screen.getByLabelText(
      'Link an actual study session (optional)',
    ) as HTMLSelectElement;
    await waitFor(() => expect(sessionSelect).not.toBeDisabled());
    await waitFor(() =>
      expect(
        screen.getByRole('option', { name: /30m · General study — Warm-up set/ }),
      ).toBeInTheDocument(),
    );
    fireEvent.change(sessionSelect, { target: { value: '12' } });
    expect(invokeMock).not.toHaveBeenCalledWith('create_planned_block', expect.anything());

    await userEvent.click(screen.getByRole('button', { name: 'Add planned block' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_planned_block', {
        block: {
          title: 'Electrodynamics block',
          topicId: null,
          calendarDate: TODAY,
          startTime: '09:00',
          endTime: '10:30',
          status: 'completed',
          notes: '',
          linkedSessionId: 12,
        },
      }),
    );
    expect(onSaved).toHaveBeenCalledWith(SAVED_BLOCK, false);
  });

  it('rejects an end time that is not later than the start time', async () => {
    invokeMock.mockResolvedValue({ ...SAVED_BLOCK, status: 'planned', linkedSessionId: null });

    render(
      <PlannedBlockEditor
        block={null}
        topics={[]}
        defaultDate={TODAY}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Planned-block title'), {
      target: { value: 'Bad block' },
    });
    fireEvent.change(screen.getByLabelText('Start time'), { target: { value: '11:00' } });
    fireEvent.change(screen.getByLabelText('End time'), { target: { value: '10:00' } });
    await userEvent.click(screen.getByRole('button', { name: 'Add planned block' }));

    expect(
      screen.getByText('The end time must be later than the start time on the same date.'),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith('create_planned_block', expect.anything());
  });
});
