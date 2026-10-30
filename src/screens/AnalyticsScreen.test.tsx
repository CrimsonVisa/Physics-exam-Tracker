import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MockAnalytics, PyqAnalytics, StudyPerformance } from '../lib/analyticsApi';
import AnalyticsScreen from './AnalyticsScreen';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

const PYQ_ANALYTICS: PyqAnalytics = {
  totalRecords: 9,
  correct: 4,
  incorrect: 4,
  skipped: 1,
  outcomeUnknown: 0,
  attempts: 8,
  accuracy: 50,
  minimumWeakAttempts: 5,
  weakAccuracyThreshold: 50,
  byTopic: [
    {
      topicId: 5,
      code: 'MMP-C01',
      title: 'Dimensional analysis',
      archived: false,
      sectionCode: 'MMP',
      sectionTitle: 'Mathematical Methods of Physics',
      correct: 2,
      incorrect: 4,
      skipped: 0,
      attempts: 6,
      accuracy: 33.33333333333333,
      sufficientData: true,
    },
    {
      topicId: 1,
      code: 'GA-01',
      title: 'Logical reasoning',
      archived: false,
      sectionCode: 'GA',
      sectionTitle: 'General Aptitude',
      correct: 1,
      incorrect: 1,
      skipped: 1,
      attempts: 2,
      accuracy: 50,
      sufficientData: false,
    },
  ],
  byUnit: [
    {
      code: 'MMP',
      title: 'Mathematical Methods of Physics',
      correct: 2,
      incorrect: 4,
      skipped: 0,
      attempts: 6,
      accuracy: 33.33333333333333,
    },
    {
      code: 'GA',
      title: 'General Aptitude',
      correct: 1,
      incorrect: 1,
      skipped: 1,
      attempts: 2,
      accuracy: 50,
    },
  ],
  byYear: [
    {
      year: 2024,
      correct: 4,
      incorrect: 4,
      skipped: 1,
      attempts: 8,
      accuracy: 50,
    },
  ],
  byPart: [
    {
      part: 'B',
      correct: 4,
      incorrect: 4,
      skipped: 1,
      attempts: 8,
      accuracy: 50,
    },
  ],
  weakTopics: [
    {
      topicId: 5,
      code: 'MMP-C01',
      title: 'Dimensional analysis',
      archived: false,
      sectionCode: 'MMP',
      sectionTitle: 'Mathematical Methods of Physics',
      correct: 2,
      incorrect: 4,
      skipped: 0,
      attempts: 6,
      accuracy: 33.33333333333333,
      sufficientData: true,
    },
  ],
};

const MOCK_ANALYTICS: MockAnalytics = {
  testCount: 2,
  averagePercentage: 60,
  bestPercentage: 70,
  latestPercentage: 70,
  trend: [
    {
      id: 1,
      testName: 'Mock A',
      testDate: '2026-10-01',
      maxMarks: 100,
      computedMarks: 50,
      computedPercentage: 50,
      reportedMarks: null,
    },
    {
      id: 2,
      testName: 'Mock B',
      testDate: '2026-10-15',
      maxMarks: 100,
      computedMarks: 70,
      computedPercentage: 70,
      reportedMarks: 71,
    },
  ],
  byPart: [
    {
      part: 'A',
      questions: 20,
      attempted: 16,
      correct: 12,
      incorrect: 4,
      unanswered: 4,
      accuracy: 75,
    },
    {
      part: 'B',
      questions: 0,
      attempted: 0,
      correct: 0,
      incorrect: 0,
      unanswered: 0,
      accuracy: null,
    },
    {
      part: 'C',
      questions: 0,
      attempted: 0,
      correct: 0,
      incorrect: 0,
      unanswered: 0,
      accuracy: null,
    },
  ],
};

const STUDY: StudyPerformance = {
  days: [
    {
      date: '2026-10-30',
      studySeconds: 3_600,
      pyqCorrect: 1,
      pyqIncorrect: 1,
      pyqAccuracy: 50,
      mockCount: 1,
      mockAveragePercentage: 70,
    },
    {
      date: '2026-10-29',
      studySeconds: 0,
      pyqCorrect: 1,
      pyqIncorrect: 0,
      pyqAccuracy: 100,
      mockCount: 0,
      mockAveragePercentage: null,
    },
  ],
};

function mockAnalytics() {
  invokeMock.mockImplementation((command: string) => {
    if (command === 'get_pyq_analytics') {
      return Promise.resolve(PYQ_ANALYTICS);
    }
    if (command === 'get_mock_analytics') {
      return Promise.resolve(MOCK_ANALYTICS);
    }
    if (command === 'get_study_performance') {
      return Promise.resolve(STUDY);
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

describe('analytics screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('distinguishes weak topics from insufficient data and renders all analytics tables', async () => {
    mockAnalytics();

    render(<AnalyticsScreen />);

    expect(await screen.findByText('PYQ records')).toBeInTheDocument();
    expect(screen.getAllByText('50.0%').length).toBeGreaterThan(0);
    expect(screen.getByText('Weak', { selector: '.analytics-tag' })).toBeInTheDocument();
    expect(
      screen.getByText('Insufficient data', { selector: '.analytics-tag' }),
    ).toBeInTheDocument();
    expect(screen.getAllByText('MMP-C01 · Dimensional analysis').length).toBeGreaterThan(0);
    expect(screen.getByText('By unit')).toBeInTheDocument();
    expect(screen.getByText('By examination year')).toBeInTheDocument();
    expect(screen.getByText('Part-wise accuracy')).toBeInTheDocument();
    expect(screen.getByText('Calculated-score trend')).toBeInTheDocument();
    expect(screen.getByText('Mock A')).toBeInTheDocument();
    expect(screen.getByText('Mock B')).toBeInTheDocument();
    expect(screen.getByText('Study time and outcomes')).toBeInTheDocument();
    expect(screen.getByText('1h')).toBeInTheDocument();
    expect(screen.getByText('1 · avg 70.0%')).toBeInTheDocument();
    expect(screen.getByText(/not a causal claim/)).toBeInTheDocument();
  });

  it('shows explicit empty states without inventing performance', async () => {
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_pyq_analytics') {
        return Promise.resolve({
          ...PYQ_ANALYTICS,
          totalRecords: 0,
          correct: 0,
          incorrect: 0,
          skipped: 0,
          outcomeUnknown: 0,
          attempts: 0,
          accuracy: null,
          byTopic: [],
          byUnit: [],
          byYear: [],
          byPart: [],
          weakTopics: [],
        });
      }
      if (command === 'get_mock_analytics') {
        return Promise.resolve({
          testCount: 0,
          averagePercentage: null,
          bestPercentage: null,
          latestPercentage: null,
          trend: [],
          byPart: [],
        });
      }
      if (command === 'get_study_performance') {
        return Promise.resolve({ days: [] });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<AnalyticsScreen />);

    expect(
      await screen.findByText('No PYQ topic outcomes have been recorded.'),
    ).toBeInTheDocument();
    expect(screen.getByText('No mock-test scores yet.')).toBeInTheDocument();
    expect(screen.getByText(/No dates have both saved outcomes/)).toBeInTheDocument();
    expect(screen.getAllByText('No data').length).toBeGreaterThan(0);
  });

  it('shows a load error and retries all analytics queries', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_pyq_analytics') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve({
              ...PYQ_ANALYTICS,
              byTopic: [],
              weakTopics: [],
              byUnit: [],
              byYear: [],
              byPart: [],
            });
      }
      if (command === 'get_mock_analytics') {
        return Promise.resolve(MOCK_ANALYTICS);
      }
      if (command === 'get_study_performance') {
        return Promise.resolve(STUDY);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    render(<AnalyticsScreen />);

    expect(await screen.findByText('Analytics could not be loaded')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByText('PYQ records')).toBeInTheDocument();
    await waitFor(() => expect(attempts).toBe(2));
  });
});
