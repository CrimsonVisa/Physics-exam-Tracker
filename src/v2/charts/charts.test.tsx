import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { AnalyticsCharts } from '../AnalyticsCharts';
import type { V2AnalyticsCharts, V2ExamSummary } from '../api';
import { BarChart } from './BarChart';
import { ChartFrame } from './ChartFrame';
import { DonutChart } from './DonutChart';
import { LineChart } from './LineChart';

const EXAMS: V2ExamSummary[] = [
  {
    id: 1,
    code: 'csir-net-physics',
    name: 'CSIR-UGC NET Physical Sciences',
    shortName: 'CSIR-NET',
    authority: 'CSIR-HRDG',
    officialUrl: 'https://example.com/csir',
    displayOrder: 1,
    isActive: true,
    examDate: null,
  },
];

function emptyCharts(): V2AnalyticsCharts {
  return {
    rangeFrom: '2026-10-12',
    rangeTo: '2026-11-10',
    totalSeconds: 0,
    allocatedSeconds: 0,
    unallocatedSeconds: 0,
    studyByExam: [],
    studyBySubject: [],
    dailyStudy: [],
    weeklyConsistency: [],
    questionOutcomes: {
      attempts: 0,
      correct: 0,
      incorrect: 0,
      unanswered: 0,
      gradedAttempts: 0,
      accuracyPercentage: null,
      insufficientSample: true,
    },
    outcomesByExam: [],
    topicAccuracy: [],
    syllabusCompletion: [],
    mockTrend: [],
    mockByExam: [],
  };
}

describe('chart primitives', () => {
  it('renders an accessible data table from ChartFrame', async () => {
    const user = userEvent.setup();
    render(
      <ChartFrame
        columns={['Label', 'Value']}
        description="A tested chart."
        rows={[['Alpha', 10]]}
        title="Test chart"
      >
        <DonutChart
          ariaLabel="Test donut"
          data={[{ label: 'Alpha', value: 10, valueLabel: '10 min' }]}
        />
      </ChartFrame>,
    );

    expect(screen.getByRole('heading', { name: 'Test chart' })).toBeInTheDocument();
    await user.click(screen.getByText('View data table'));
    expect(screen.getByRole('columnheader', { name: 'Label' })).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: 'Alpha' })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: 'Test donut' })).toBeInTheDocument();
  });

  it('renders bar and line charts without fabricating empty data', () => {
    const { rerender } = render(
      <BarChart ariaLabel="Test bars" data={[{ label: 'One', value: 10 }]} />,
    );
    expect(screen.getByRole('img', { name: 'Test bars' })).toBeInTheDocument();

    rerender(<BarChart ariaLabel="Empty bars" data={[]} />);
    expect(screen.getByText('No chartable data in this range.')).toBeInTheDocument();

    rerender(
      <LineChart
        ariaLabel="Mock scores"
        series={[
          {
            label: 'CSIR-NET',
            points: [
              { x: '2026-11-01', y: 42.5 },
              { x: '2026-11-08', y: -12.5 },
            ],
          },
        ]}
        valueSuffix="%"
      />,
    );
    expect(screen.getByRole('img', { name: 'Mock scores' })).toBeInTheDocument();
  });

  it('shows sub-minute durations in seconds instead of zero minutes', () => {
    const charts = emptyCharts();
    charts.totalSeconds = 23;
    charts.unallocatedSeconds = 23;
    charts.dailyStudy = [{ label: '2026-11-10', seconds: 23, sessionCount: 1 }];

    render(<AnalyticsCharts charts={charts} exams={EXAMS} />);

    const studyCard = screen
      .getByRole('heading', { name: 'Study time by examination' })
      .closest('figure');
    expect(studyCard).not.toBeNull();
    expect(within(studyCard as HTMLElement).getAllByText('23 s').length).toBeGreaterThan(0);
    expect(within(studyCard as HTMLElement).queryByText('0 min')).not.toBeInTheDocument();

    const dailyCard = screen.getByRole('heading', { name: 'Daily study trend' }).closest('figure');
    expect(dailyCard).not.toBeNull();
    expect(within(dailyCard as HTMLElement).getAllByText('23 s').length).toBeGreaterThan(0);
  });

  it('shows honest empty and small-sample analytics states', () => {
    const charts = emptyCharts();
    charts.topicAccuracy = [
      {
        examId: 1,
        examCode: 'csir-net-physics',
        nodeId: 3,
        topicTitle: 'Dimensional analysis',
        attempts: 2,
        correct: 1,
        incorrect: 1,
        unanswered: 0,
        gradedAttempts: 2,
        accuracyPercentage: 50,
        insufficientSample: true,
      },
    ];
    charts.mockTrend = [
      {
        examId: 1,
        examCode: 'csir-net-physics',
        testId: 1,
        testDate: '2026-11-01',
        testName: 'Mock 1',
        percentage: 42.5,
      },
    ];
    charts.syllabusCompletion = [
      {
        examId: 1,
        examCode: 'csir-net-physics',
        examShortName: 'CSIR-NET',
        topicCount: 141,
        completedTopicCount: 0,
        completionPercentage: 0,
      },
    ];

    render(<AnalyticsCharts charts={charts} exams={EXAMS} />);

    expect(screen.getAllByText('No study sessions in this range.').length).toBeGreaterThan(0);
    expect(screen.getByText('No PYQ attempts in this range.')).toBeInTheDocument();
    expect(screen.getByText(/No topic has at least five graded attempts/)).toBeInTheDocument();
    expect(screen.getByText(/Only one completed mock in this range/)).toBeInTheDocument();
    expect(screen.getByText('Insufficient sample')).toBeInTheDocument();
    expect(screen.getByText(/Only one examination has data/)).toBeInTheDocument();
  });
});
