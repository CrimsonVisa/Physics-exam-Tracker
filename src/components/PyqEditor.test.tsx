import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PyqAttempt, PyqOptions } from '../lib/pyqApi';
import { PyqEditor } from './PyqEditor';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';
const OPTIONS: PyqOptions = {
  years: [2024],
  sections: [{ code: 'MMP', title: 'Mathematical Methods of Physics' }],
  topics: [
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

const SAVED: PyqAttempt = {
  id: 9,
  questionYear: 2024,
  examSession: 'June 2024',
  part: 'B',
  topic: OPTIONS.topics[0],
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

describe('PYQ editor', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('creates an attempt with all typed details', async () => {
    invokeMock.mockResolvedValue(SAVED);
    const onSaved = vi.fn();

    render(
      <PyqEditor
        attempt={null}
        options={OPTIONS}
        today={TODAY}
        onSaved={onSaved}
        onCancel={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText('Question year'), { target: { value: '2024' } });
    fireEvent.change(screen.getByLabelText('Exam / session (optional)'), {
      target: { value: 'June 2024' },
    });
    fireEvent.change(screen.getByLabelText('Part'), { target: { value: 'B' } });
    fireEvent.change(screen.getByLabelText('Status'), { target: { value: 'correct' } });
    fireEvent.change(screen.getByLabelText('Syllabus topic'), { target: { value: '5' } });
    fireEvent.change(screen.getByLabelText('Question reference'), {
      target: { value: 'Paper B Q42' },
    });
    fireEvent.change(screen.getByLabelText('Marks obtained (optional)'), {
      target: { value: '2' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks (optional)'), {
      target: { value: '2' },
    });
    fireEvent.change(screen.getByLabelText('Minutes'), { target: { value: '1' } });
    fireEvent.change(screen.getByLabelText('Seconds'), { target: { value: '30' } });
    fireEvent.change(screen.getByLabelText('Difficulty (optional)'), {
      target: { value: '4' },
    });
    fireEvent.change(screen.getByLabelText('Personal notes (optional)'), {
      target: { value: 'Use the symmetry argument' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add PYQ attempt' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_pyq_attempt', {
        attempt: {
          questionYear: 2024,
          examSession: 'June 2024',
          part: 'B',
          topicId: 5,
          questionReference: 'Paper B Q42',
          attemptDate: TODAY,
          status: 'correct',
          marksObtained: 2,
          maxMarks: 2,
          timeSpentSeconds: 90,
          notes: 'Use the symmetry argument',
          difficulty: 4,
        },
      }),
    );
    expect(onSaved).toHaveBeenCalledWith(SAVED, false);
  });

  it('blocks missing references and topics locally', async () => {
    render(
      <PyqEditor
        attempt={null}
        options={OPTIONS}
        today={TODAY}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: 'Add PYQ attempt' }));

    expect(screen.getByText('Choose Part A, B or C.')).toBeInTheDocument();
    expect(screen.getByText('Choose a syllabus topic.')).toBeInTheDocument();
    expect(screen.getByText('Enter a question reference or identifier.')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('loads an existing attempt into an editable form', () => {
    render(
      <PyqEditor
        attempt={SAVED}
        options={OPTIONS}
        today={TODAY}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByLabelText('Question year')).toHaveValue(2024);
    expect(screen.getByLabelText('Part')).toHaveValue('B');
    expect(screen.getByLabelText('Syllabus topic')).toHaveValue('5');
    expect(screen.getByLabelText('Question reference')).toHaveValue('Paper B Q42');
    expect(screen.getByLabelText('Minutes')).toHaveValue(1);
    expect(screen.getByLabelText('Seconds')).toHaveValue(30);
    expect(screen.getByRole('button', { name: 'Save PYQ attempt' })).toBeInTheDocument();

    const form = screen.getByRole('button', { name: 'Save PYQ attempt' }).closest('form');
    expect(form).not.toBeNull();
    expect(
      within(form as HTMLElement).getByDisplayValue('Use the symmetry argument'),
    ).toBeInTheDocument();
  });
});
