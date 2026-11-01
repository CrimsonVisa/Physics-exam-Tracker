import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PyqAttempt, PyqOptions } from '../lib/pyqApi';
import PyqScreen from './PyqScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';
const OPTIONS: PyqOptions = {
  years: [2024, 2023],
  sections: [
    { code: 'GA', title: 'General Aptitude' },
    { code: 'MMP', title: 'Mathematical Methods of Physics' },
  ],
  topics: [
    {
      id: 1,
      code: 'GA-01',
      title: 'Logical reasoning',
      archived: false,
      sectionCode: 'GA',
      sectionTitle: 'General Aptitude',
    },
    {
      id: 5,
      code: 'MMP-C01',
      title: 'Dimensional analysis',
      archived: false,
      sectionCode: 'MMP',
      sectionTitle: 'Mathematical Methods of Physics',
    },
  ],
};

const ATTEMPT: PyqAttempt = {
  id: 21,
  questionYear: 2024,
  examSession: 'June 2024',
  part: 'B',
  topic: OPTIONS.topics[1],
  questionReference: 'Paper B Q42',
  attemptDate: TODAY,
  status: 'correct',
  marksObtained: 2,
  maxMarks: 2,
  timeSpentSeconds: 90,
  notes: 'Use the symmetry argument',
  difficulty: 4,
  createdAt: '2026-10-30T10:00:00.000Z',
  updatedAt: '2026-10-30T10:00:00.000Z',
};

function mockPyqBackend({
  attempts = [],
  options = OPTIONS,
  deleteAttempt = vi.fn().mockResolvedValue(undefined),
}: {
  attempts?: PyqAttempt[];
  options?: PyqOptions;
  deleteAttempt?: () => Promise<unknown>;
} = {}) {
  invokeMock.mockImplementation((command: string) => {
    if (command === 'get_pyq_options') {
      return Promise.resolve(options);
    }
    if (command === 'get_pyq_attempts') {
      return Promise.resolve(attempts);
    }
    if (command === 'delete_pyq_attempt') {
      return deleteAttempt();
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
  return { deleteAttempt };
}

describe('PYQ screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows an empty history and a usable filter form', async () => {
    mockPyqBackend();

    render(<PyqScreen today={TODAY} />);

    expect(await screen.findByText('No PYQ attempts recorded yet')).toBeInTheDocument();
    expect(screen.getByText('Import PYQ attempts')).toBeInTheDocument();
    expect(screen.getByLabelText('Question year')).toBeInTheDocument();
    expect(screen.getByLabelText('Unit')).toBeInTheDocument();
  });

  it('renders saved attempts and applies all filter dimensions', async () => {
    mockPyqBackend({ attempts: [ATTEMPT] });

    render(<PyqScreen today={TODAY} />);

    expect(await screen.findByText('Paper B Q42')).toBeInTheDocument();
    expect(screen.getByText('Correct', { selector: '.pyq-status' })).toBeInTheDocument();
    expect(screen.getByText('2024', { selector: '.pyq-badge' })).toBeInTheDocument();
    expect(screen.getByText('Part B', { selector: '.pyq-badge' })).toBeInTheDocument();
    expect(screen.getAllByText('MMP-C01 · Dimensional analysis').length).toBeGreaterThan(0);
    expect(screen.getByText('2 / 2 marks')).toBeInTheDocument();
    expect(screen.getByText('Time spent 1m 30s')).toBeInTheDocument();
    expect(screen.getByText('Use the symmetry argument')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Question year'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('Part'), { target: { value: 'B' } });
    fireEvent.change(screen.getByLabelText('Unit'), { target: { value: 'MMP' } });
    fireEvent.change(screen.getByLabelText('Topic'), { target: { value: '5' } });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'correct' } });
    fireEvent.change(screen.getByLabelText('Attempted from (optional)'), {
      target: { value: '2026-10-01' },
    });
    fireEvent.change(screen.getByLabelText('Attempted to (optional)'), {
      target: { value: '2026-10-31' },
    });
    fireEvent.change(screen.getByLabelText('Search'), { target: { value: 'symmetry' } });
    await userEvent.click(screen.getByRole('button', { name: 'Apply filters' }));

    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter(([command]) => command === 'get_pyq_attempts');
      expect(calls.at(-1)?.[1]).toEqual({
        filter: {
          year: 2024,
          part: 'B',
          sectionCode: 'MMP',
          topicId: 5,
          status: 'correct',
          query: 'symmetry',
          fromDate: '2026-10-01',
          toDate: '2026-10-31',
        },
      });
    });
  });

  it('paginates long filtered histories', async () => {
    const attempts = Array.from({ length: 21 }, (_, index) => ({
      ...ATTEMPT,
      id: 100 + index,
      questionReference: `Paper B Q${index + 1}`,
    }));
    mockPyqBackend({ attempts });

    render(<PyqScreen today={TODAY} />);

    await screen.findByText('Paper B Q1');
    expect(document.querySelectorAll('.pyq-row')).toHaveLength(20);
    expect(screen.getByText('Page 1 of 2 · 21 attempts')).toBeInTheDocument();
    expect(screen.queryByText('Paper B Q21')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Next page' }));

    expect(document.querySelectorAll('.pyq-row')).toHaveLength(1);
    expect(screen.getByText('Paper B Q21')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  });

  it('opens the editor and deletes an attempt only after confirmation', async () => {
    const deleteAttempt = vi.fn().mockResolvedValue(undefined);
    mockPyqBackend({ attempts: [ATTEMPT], deleteAttempt });

    render(<PyqScreen today={TODAY} />);

    await screen.findByText('Paper B Q42');
    await userEvent.click(screen.getByRole('button', { name: 'Edit' }));

    expect(screen.getByRole('heading', { name: 'Edit PYQ attempt' })).toBeInTheDocument();
    expect(screen.getByLabelText('Question reference')).toHaveValue('Paper B Q42');
    await userEvent.click(screen.getByRole('button', { name: 'Close editor' }));

    const row = screen.getByText('Paper B Q42').closest('.pyq-row');
    expect(row).not.toBeNull();
    await userEvent.click(within(row as HTMLElement).getByRole('button', { name: 'Delete' }));

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent('Delete this PYQ attempt permanently?');
    expect(deleteAttempt).not.toHaveBeenCalled();

    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm delete' }));

    await waitFor(() => expect(deleteAttempt).toHaveBeenCalledTimes(1));
    expect(await screen.findByText('PYQ attempt deleted.')).toBeInTheDocument();
  });

  it('shows a load error and retries the attempt query', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_pyq_options') {
        return Promise.resolve(OPTIONS);
      }
      if (command === 'get_pyq_attempts') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve([]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<PyqScreen today={TODAY} />);

    expect(await screen.findByText('PYQ attempts could not be loaded')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('No PYQ attempts recorded yet')).toBeInTheDocument();
    expect(attempts).toBe(2);
  });
});
