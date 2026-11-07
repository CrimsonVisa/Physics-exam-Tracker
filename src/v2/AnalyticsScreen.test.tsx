import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2Analytics, V2ExamSummary } from './api';
import { AnalyticsScreen } from './AnalyticsScreen';

vi.mock('./api', () => ({ getV2Analytics: vi.fn() }));

import * as api from './api';

const mockedAnalytics = vi.mocked(api.getV2Analytics);
const TODAY = '2026-11-04';
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
const ANALYTICS: V2Analytics = {
  examId: null,
  rangeFrom: '2026-10-06',
  rangeTo: TODAY,
  totalSeconds: 5400,
  sessionCount: 3,
  allocatedSeconds: 3600,
  unallocatedSeconds: 1800,
  topicAllocatedSeconds: 2400,
  topicUnallocatedSeconds: 3000,
  activeDays: 2,
  consistencyPercent: 6.7,
  daily: [{ label: TODAY, seconds: 3600, sessionCount: 2 }],
  weekly: [{ label: '2026-W44', seconds: 3600, sessionCount: 2 }],
  subjects: [
    {
      examId: 1,
      examCode: 'csir-net-physics',
      subjectTitle: 'Mathematical Methods of Physics',
      allocatedSeconds: 2400,
      sessionCount: 2,
    },
  ],
  topics: [
    {
      examId: 1,
      examCode: 'csir-net-physics',
      subjectTitle: 'Mathematical Methods of Physics',
      nodeId: 3,
      nodeTitle: 'Dimensional analysis',
      allocatedSeconds: 2400,
      sessionCount: 2,
    },
  ],
  mappedConceptCount: 0,
  evidencedConceptCount: 0,
  averageMastery: null,
  masteryBuckets: [
    { label: '0–20', conceptCount: 0 },
    { label: '20–40', conceptCount: 0 },
    { label: '40–60', conceptCount: 0 },
    { label: '60–80', conceptCount: 0 },
    { label: '80–100', conceptCount: 0 },
  ],
  activeRevisionSchedules: 1,
  dueRevisionCount: 1,
  overdueRevisionCount: 0,
  revisionCompletedLast30Days: 2,
};

describe('V2 analytics', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedAnalytics.mockResolvedValue(ANALYTICS);
  });

  it('renders real study, consistency, mastery, and revision values', async () => {
    render(<AnalyticsScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findByText('1 h 30 min')).toBeInTheDocument();
    expect(screen.getByText('6.7%')).toBeInTheDocument();
    expect(screen.getByText('No evidenced concepts')).toBeInTheDocument();
    expect(screen.getByText('Unavailable — formula approval pending')).toBeInTheDocument();
    expect(screen.getByText('2026-W44')).toBeInTheDocument();
  });

  it('applies the exam filter', async () => {
    const user = userEvent.setup();
    render(<AnalyticsScreen exams={EXAMS} today={TODAY} />);
    await screen.findByText('1 h 30 min');

    await user.selectOptions(screen.getByLabelText('Examination scope'), 'csir-net-physics');
    await waitFor(() =>
      expect(mockedAnalytics).toHaveBeenLastCalledWith({
        examId: 1,
        today: TODAY,
        windowDays: 30,
      }),
    );
  });

  it('shows second-level durations for short sessions instead of rounding to zero', async () => {
    mockedAnalytics.mockResolvedValue({
      ...ANALYTICS,
      totalSeconds: 24,
      allocatedSeconds: 20,
      unallocatedSeconds: 4,
      topicAllocatedSeconds: 18,
      topicUnallocatedSeconds: 6,
      daily: [{ label: TODAY, seconds: 24, sessionCount: 2 }],
      weekly: [{ label: '2026-W44', seconds: 24, sessionCount: 2 }],
      subjects: [
        {
          examId: 1,
          examCode: 'csir-net-physics',
          subjectTitle: 'Mathematical Methods of Physics',
          allocatedSeconds: 18,
          sessionCount: 2,
        },
      ],
      topics: [
        {
          examId: 1,
          examCode: 'csir-net-physics',
          subjectTitle: 'Mathematical Methods of Physics',
          nodeId: 3,
          nodeTitle: 'Dimensional analysis',
          allocatedSeconds: 18,
          sessionCount: 2,
        },
      ],
    });
    render(<AnalyticsScreen exams={EXAMS} today={TODAY} />);

    expect(await screen.findAllByText('24 s')).toHaveLength(3);
    expect(screen.getByText('20 s')).toBeInTheDocument();
    expect(screen.getByText('Unallocated: 4 s.')).toBeInTheDocument();
    expect(screen.getByText(/Topic-allocated: 18 s/)).toBeInTheDocument();
    expect(screen.getAllByText('18 s')).toHaveLength(2);
    expect(screen.getAllByText(/Mathematical Methods of Physics/).length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText('0 min')).not.toBeInTheDocument();
  });
});
