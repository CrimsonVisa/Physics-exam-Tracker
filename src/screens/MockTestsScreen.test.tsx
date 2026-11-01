import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MockTest } from '../lib/mockTestApi';
import MockTestsScreen from './MockTestsScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const TODAY = '2026-10-30';

function part(overrides: Partial<MockTest['partA']> = {}): MockTest['partA'] {
  return {
    questions: 10,
    attempted: 8,
    correct: 6,
    incorrect: 2,
    unanswered: 2,
    correctMarks: 2,
    incorrectMarks: -0.5,
    unansweredMarks: 0,
    marks: 11,
    ...overrides,
  };
}

function makeTest(overrides: Partial<MockTest> = {}): MockTest {
  return {
    id: 1,
    testName: 'Mock A',
    testDate: '2026-10-01',
    durationMinutes: 180,
    maxMarks: 100,
    marksObtained: null,
    partA: part(),
    partB: part({ questions: 20, attempted: 18, correct: 12, incorrect: 6, unanswered: 2 }),
    partC: part({ questions: 10, attempted: 8, correct: 4, incorrect: 4, unanswered: 2 }),
    totalQuestions: 40,
    totalAttempted: 34,
    totalCorrect: 22,
    totalIncorrect: 12,
    totalUnanswered: 6,
    computedMarks: 50,
    computedPercentage: 50,
    configurationValid: true,
    notes: 'First attempt',
    reviewObservations: 'Slow in Part C',
    createdAt: '2026-10-01T10:00:00.000Z',
    updatedAt: '2026-10-01T10:00:00.000Z',
    ...overrides,
  };
}

const MOCK_B = makeTest({
  id: 2,
  testName: 'Mock B',
  testDate: '2026-10-15',
  computedMarks: 70,
  computedPercentage: 70,
  marksObtained: 71,
  notes: 'Second attempt',
  reviewObservations: 'Better pacing',
});

describe('mock tests screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('shows summary statistics and part-wise history', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve([MOCK_B, makeTest()]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);

    expect(await screen.findByRole('heading', { name: 'Mock B', level: 3 })).toBeInTheDocument();
    expect(screen.getByText('60.0%')).toBeInTheDocument();
    expect(screen.getAllByText('70.0%').length).toBeGreaterThan(0);
    expect(screen.getByText('First attempt')).toBeInTheDocument();
    expect(screen.getByText('Slow in Part C')).toBeInTheDocument();
    expect(screen.getByText('Import mock tests')).toBeInTheDocument();

    const row = screen.getByRole('heading', { name: 'Mock A', level: 3 }).closest('.mock-row');
    expect(row).not.toBeNull();
    expect(within(row as HTMLElement).getByText('34 attempted')).toBeInTheDocument();
    expect(within(row as HTMLElement).getByText('22 correct')).toBeInTheDocument();
    expect(within(row as HTMLElement).getByText('12 incorrect')).toBeInTheDocument();
    expect(within(row as HTMLElement).getByText('6 unanswered')).toBeInTheDocument();
    expect(within(row as HTMLElement).getAllByText('11').length).toBe(3);
  });

  it('scopes the summary cards to the active date filter', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve([MOCK_B, makeTest()]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);
    await screen.findByRole('heading', { name: 'Mock B', level: 3 });

    fireEvent.change(screen.getByLabelText('Test date from (optional)'), {
      target: { value: '2026-10-15' },
    });
    fireEvent.change(screen.getByLabelText('Test date to (optional)'), {
      target: { value: '2026-10-15' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Apply dates' }));

    const summary = screen.getByRole('region', { name: 'Mock test summary' });
    await waitFor(() =>
      expect(within(summary).getByText('Tests shown').parentElement).toHaveTextContent('1'),
    );
    expect(within(summary).getByText('Average calculated score').parentElement).toHaveTextContent(
      '70.0%',
    );
    expect(within(summary).getByText('Best calculated score').parentElement).toHaveTextContent(
      '70.0%',
    );
    expect(within(summary).getByText('Latest calculated score').parentElement).toHaveTextContent(
      '70.0%',
    );
  });

  it('opens the create editor and deletes a test after confirmation', async () => {
    const deleteMockTest = vi.fn().mockResolvedValue(undefined);
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve([MOCK_B]);
      }
      if (command === 'delete_mock_test') {
        return deleteMockTest();
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Mock B', level: 3 });
    await userEvent.click(screen.getByRole('button', { name: 'New mock test' }));
    expect(screen.getByRole('heading', { name: 'Add a mock test' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Close editor' }));

    const row = screen.getByRole('heading', { name: 'Mock B', level: 3 }).closest('.mock-row');
    expect(row).not.toBeNull();
    await userEvent.click(within(row as HTMLElement).getByRole('button', { name: 'Delete' }));

    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent('Delete this mock test permanently?');
    expect(deleteMockTest).not.toHaveBeenCalled();

    await userEvent.click(within(dialog).getByRole('button', { name: 'Confirm delete' }));

    await waitFor(() => expect(deleteMockTest).toHaveBeenCalledTimes(1));
    expect(await screen.findByText('Mock test deleted.')).toBeInTheDocument();
  });

  it('filters and paginates the mock-test history', async () => {
    const tests = Array.from({ length: 21 }, (_, index) =>
      makeTest({
        id: 100 + index,
        testName: `Mock ${index + 1}`,
        testDate: `2026-10-${String(index + 1).padStart(2, '0')}`,
      }),
    );
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve(tests);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);

    await screen.findByRole('heading', { name: 'Mock 1', level: 3 });
    expect(document.querySelectorAll('.mock-row')).toHaveLength(20);
    expect(screen.getByText('Page 1 of 2 · 21 tests')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Next page' }));
    expect(await screen.findByRole('heading', { name: 'Mock 21', level: 3 })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Test date from (optional)'), {
      target: { value: '2026-09-01' },
    });
    fireEvent.change(screen.getByLabelText('Test date to (optional)'), {
      target: { value: '2026-09-30' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Apply dates' }));

    expect(await screen.findByText('No mock tests match this date range')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Clear dates' }));
    expect(await screen.findByRole('heading', { name: 'Mock 1', level: 3 })).toBeInTheDocument();
  });

  it('warns when a saved mock test has an impossible calculated percentage', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve([makeTest({ computedMarks: 120, computedPercentage: 120 })]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);

    expect(
      await screen.findByText(/marking configuration that would now be rejected/i),
    ).toBeInTheDocument();
    expect(screen.getByText('Review marking configuration')).toBeInTheDocument();
  });

  it('warns about a legacy invalid configuration even when the calculated percentage is below 100', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_mock_tests') {
        return Promise.resolve([
          makeTest({
            computedMarks: 50,
            computedPercentage: 50,
            configurationValid: false,
          }),
        ]);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<MockTestsScreen today={TODAY} />);

    expect(
      await screen.findByText(/marking configuration that would now be rejected/i),
    ).toBeInTheDocument();
    expect(screen.getByText('Review marking configuration')).toBeInTheDocument();
    expect(screen.getAllByText('50.0%').length).toBeGreaterThan(0);
  });

  it('shows an empty state when no mock tests are saved', async () => {
    invokeMock.mockResolvedValue([]);

    render(<MockTestsScreen today={TODAY} />);

    expect(await screen.findByText('No mock tests recorded yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'New mock test' })).toBeInTheDocument();
    expect(screen.getByText('Import mock tests')).toBeInTheDocument();
  });
});
