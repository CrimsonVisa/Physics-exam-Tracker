import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MockTest } from '../lib/mockTestApi';
import { MockTestEditor } from './MockTestEditor';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';

const PART = {
  questions: 0,
  attempted: 0,
  correct: 0,
  incorrect: 0,
  unanswered: 0,
  correctMarks: 0,
  incorrectMarks: 0,
  unansweredMarks: 0,
  marks: 0,
};

const SAVED: MockTest = {
  id: 11,
  testName: 'Full Mock 1',
  testDate: TODAY,
  durationMinutes: 180,
  maxMarks: 100,
  marksObtained: null,
  partA: {
    ...PART,
    questions: 10,
    attempted: 9,
    correct: 6,
    incorrect: 3,
    unanswered: 1,
    correctMarks: 2,
    incorrectMarks: -0.5,
    marks: 10.5,
  },
  partB: PART,
  partC: PART,
  totalQuestions: 10,
  totalAttempted: 9,
  totalCorrect: 6,
  totalIncorrect: 3,
  totalUnanswered: 1,
  computedMarks: 10.5,
  computedPercentage: 10.5,
  configurationValid: true,
  notes: '',
  reviewObservations: '',
  createdAt: '2026-10-30T10:00:00.000Z',
  updatedAt: '2026-10-30T10:00:00.000Z',
};

describe('mock-test editor', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('previews and saves the score calculated from the editable marking configuration', async () => {
    invokeMock.mockResolvedValue(SAVED);
    const onSaved = vi.fn();

    render(<MockTestEditor test={null} today={TODAY} onSaved={onSaved} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Full Mock 1' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '100' },
    });

    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '10' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '6' } });
    fireEvent.change(within(partA).getByLabelText('Incorrect'), { target: { value: '3' } });
    fireEvent.change(within(partA).getByLabelText('Unanswered'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '2' },
    });
    fireEvent.change(within(partA).getByLabelText('Incorrect marks'), {
      target: { value: '-0.5' },
    });
    fireEvent.change(within(partA).getByLabelText('Unanswered marks'), {
      target: { value: '0' },
    });

    const scorePreview = screen.getByLabelText('Calculated mock score');
    expect(scorePreview).toHaveTextContent('10.50 / 100');
    expect(scorePreview).toHaveTextContent('10.50%');

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_mock_test', {
        test: {
          testName: 'Full Mock 1',
          testDate: TODAY,
          durationMinutes: 180,
          maxMarks: 100,
          marksObtained: null,
          partA: {
            questions: 10,
            correct: 6,
            incorrect: 3,
            unanswered: 1,
            correctMarks: 2,
            incorrectMarks: -0.5,
            unansweredMarks: 0,
          },
          partB: {
            questions: 0,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            correctMarks: null,
            incorrectMarks: null,
            unansweredMarks: null,
          },
          partC: {
            questions: 0,
            correct: 0,
            incorrect: 0,
            unanswered: 0,
            correctMarks: null,
            incorrectMarks: null,
            unansweredMarks: null,
          },
          notes: '',
          reviewObservations: '',
        },
      }),
    );
    expect(onSaved).toHaveBeenCalledWith(SAVED, false);
  });

  it('warns about a legacy invalid configuration when editing an existing record', () => {
    render(
      <MockTestEditor
        test={{
          ...SAVED,
          configurationValid: false,
          partA: { ...SAVED.partA, incorrectMarks: 1 },
        }}
        today={TODAY}
        onSaved={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    expect(screen.getByText(/Saved marking configuration needs review/)).toBeInTheDocument();
    expect(screen.getByText('Part A has positive incorrect-answer marks.')).toBeInTheDocument();
  });

  it('rejects a reported score above the maximum', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Reported score mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '100' },
    });
    fireEvent.change(screen.getByLabelText('Reported score (optional)'), {
      target: { value: '150' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '2' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(screen.getByText('Reported marks cannot exceed maximum marks.')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rejects positive incorrect-answer marks', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Invalid penalty mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '4' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '2' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Incorrect'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '2' },
    });
    fireEvent.change(within(partA).getByLabelText('Incorrect marks'), {
      target: { value: '1' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(
      screen.getByText('Incorrect-answer marks must be zero or negative. Use 0 for no penalty.'),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rejects maximum marks below the configured maximum score', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Impossible percentage mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '10' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '10' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '10' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '2' },
    });

    expect(screen.getByLabelText('Calculated mock score')).toHaveTextContent('200.00%');
    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(
      screen.getByText(
        'Maximum marks must be at least the highest possible score from this marking configuration (20).',
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/calculated score is above maximum marks/i)).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rejects a maximum within the former rounding margin of the possible score', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Tight maximum mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '99.995' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '100' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(
      screen.getByText(
        'Maximum marks must be at least the highest possible score from this marking configuration (100).',
      ),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rejects a tiny maximum that would still produce an impossible percentage', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Tiny maximum mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '0.001' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '1' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '0.001000001' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(
      screen.getByText(
        'The calculated score cannot exceed maximum marks. Review maximum marks or the marking configuration.',
      ),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('rejects an inconsistent part breakdown before invoking the backend', async () => {
    render(<MockTestEditor test={null} today={TODAY} onSaved={vi.fn()} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByLabelText('Test name'), {
      target: { value: 'Broken Mock' },
    });
    fireEvent.change(screen.getByLabelText('Duration (minutes)'), {
      target: { value: '180' },
    });
    fireEvent.change(screen.getByLabelText('Maximum marks'), {
      target: { value: '100' },
    });
    const partA = screen.getByRole('group', { name: 'Part A' });
    fireEvent.change(within(partA).getByLabelText('Questions'), { target: { value: '10' } });
    fireEvent.change(within(partA).getByLabelText('Correct'), { target: { value: '6' } });
    fireEvent.change(within(partA).getByLabelText('Incorrect'), { target: { value: '3' } });
    fireEvent.change(within(partA).getByLabelText('Unanswered'), { target: { value: '0' } });
    fireEvent.change(within(partA).getByLabelText('Correct marks'), {
      target: { value: '2' },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Add mock test' }));

    expect(
      screen.getByText('Correct + incorrect + unanswered must equal the question count.'),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
