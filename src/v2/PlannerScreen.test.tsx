import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2ExamSummary, V2PlannerDay } from './api';
import { PlannerScreen } from './PlannerScreen';

vi.mock('./api', () => ({
  getV2PlannerDay: vi.fn(),
  saveV2Task: vi.fn(),
  setV2TaskStatus: vi.fn(),
  rescheduleV2Task: vi.fn(),
  deleteV2Task: vi.fn(),
  saveV2Block: vi.fn(),
  setV2BlockStatus: vi.fn(),
  linkV2BlockSession: vi.fn(),
  deleteV2Block: vi.fn(),
}));

import * as api from './api';

const mockedDay = vi.mocked(api.getV2PlannerDay);
const mockedSaveTask = vi.mocked(api.saveV2Task);
const mockedSetTaskStatus = vi.mocked(api.setV2TaskStatus);
const mockedDeleteTask = vi.mocked(api.deleteV2Task);
const mockedSaveBlock = vi.mocked(api.saveV2Block);
const mockedSetBlockStatus = vi.mocked(api.setV2BlockStatus);
const mockedDeleteBlock = vi.mocked(api.deleteV2Block);

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG / NTA',
    officialUrl: 'https://csirnet.nta.nic.in/',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
];

const TODAY = '2026-11-04';

const DAY: V2PlannerDay = {
  date: TODAY,
  tasks: [
    {
      id: 1,
      title: 'Revise kinematics',
      description: 'Solve examples',
      scheduledDate: TODAY,
      deadlineDate: null,
      examId: 1,
      examCode: 'csir-net-physics',
      conceptId: null,
      conceptTitle: null,
      syllabusNodeId: null,
      priority: 'high',
      status: 'open',
      scheduleAnchor: 'manual',
      scheduleOffsetDays: null,
      completedAt: null,
      linkedSessionId: null,
      createdAt: '2026-11-04T10:00:00Z',
      updatedAt: '2026-11-04T10:00:00Z',
    },
  ],
  blocks: [
    {
      id: 2,
      title: 'Morning study',
      examId: 1,
      examCode: 'csir-net-physics',
      conceptId: null,
      conceptTitle: null,
      syllabusNodeId: null,
      calendarDate: TODAY,
      startTime: '09:00',
      endTime: '10:00',
      status: 'planned',
      notes: '',
      linkedSessionId: null,
      createdAt: '2026-11-04T10:00:00Z',
      updatedAt: '2026-11-04T10:00:00Z',
    },
  ],
};

describe('V2 planner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedDay.mockResolvedValue(DAY);
  });

  it('loads tasks and blocks for the selected date', async () => {
    render(<PlannerScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('Revise kinematics')).toBeInTheDocument();
    expect(screen.getByText('Morning study')).toBeInTheDocument();
    expect(mockedDay).toHaveBeenCalledWith(TODAY);
  });

  it('adds a task and completes it', async () => {
    const user = userEvent.setup();
    mockedSaveTask.mockResolvedValue(DAY.tasks[0]);
    mockedSetTaskStatus.mockResolvedValue({
      ...DAY.tasks[0],
      status: 'completed',
      completedAt: '2026-11-04T11:00:00Z',
    });
    render(<PlannerScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('Revise kinematics');

    await user.type(screen.getByLabelText('Task title'), 'New task');
    await user.click(screen.getByRole('button', { name: 'Add task' }));
    await waitFor(() =>
      expect(mockedSaveTask).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'New task',
          scheduledDate: TODAY,
          scheduleAnchor: 'manual',
        }),
      ),
    );

    await user.click(screen.getByRole('button', { name: 'Complete' }));
    await waitFor(() =>
      expect(mockedSetTaskStatus).toHaveBeenCalledWith({
        taskId: 1,
        status: 'completed',
      }),
    );
  });

  it('adds blocks and changes their status', async () => {
    const user = userEvent.setup();
    mockedSaveBlock.mockResolvedValue(DAY.blocks[0]);
    mockedSetBlockStatus.mockResolvedValue({ ...DAY.blocks[0], status: 'completed' });
    render(<PlannerScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('Morning study');

    await user.type(screen.getByLabelText('Block title'), 'Evening block');
    await user.click(screen.getByRole('button', { name: 'Add planned block' }));
    await waitFor(() =>
      expect(mockedSaveBlock).toHaveBeenCalledWith(
        expect.objectContaining({
          title: 'Evening block',
          calendarDate: TODAY,
          startTime: '09:00',
          endTime: '10:00',
        }),
      ),
    );

    await user.selectOptions(screen.getByLabelText('Status for Morning study'), 'completed');
    await waitFor(() =>
      expect(mockedSetBlockStatus).toHaveBeenCalledWith({
        blockId: 2,
        status: 'completed',
      }),
    );
  });

  it('deletes tasks and blocks after explicit action', async () => {
    const user = userEvent.setup();
    mockedDeleteTask.mockResolvedValue();
    mockedDeleteBlock.mockResolvedValue();
    render(<PlannerScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('Revise kinematics');

    await user.click(screen.getAllByRole('button', { name: 'Delete' })[0]);
    await waitFor(() => expect(mockedDeleteTask).toHaveBeenCalledWith(1));
    const deleteButtons = screen.getAllByRole('button', { name: 'Delete' });
    await user.click(deleteButtons[deleteButtons.length - 1]);
    await waitFor(() => expect(mockedDeleteBlock).toHaveBeenCalledWith(2));
  });
});
