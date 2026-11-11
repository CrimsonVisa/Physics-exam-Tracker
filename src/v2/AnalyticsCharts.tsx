import type { V2AnalyticsCharts, V2ExamSummary } from './api';
import { BarChart } from './charts/BarChart';
import { ChartFrame } from './charts/ChartFrame';
import { DonutChart } from './charts/DonutChart';
import { LineChart } from './charts/LineChart';
import { formatChartDuration, formatChartPercent } from './charts/chartMath';

type AnalyticsChartsProps = {
  charts: V2AnalyticsCharts;
  exams: V2ExamSummary[];
};

function examName(exams: V2ExamSummary[], code: string): string {
  return exams.find((exam) => exam.code === code)?.shortName ?? code;
}

export function AnalyticsCharts({ charts, exams }: AnalyticsChartsProps) {
  const studyDonut = [
    ...charts.studyByExam.map((entry) => ({
      label: entry.examShortName,
      value: entry.allocatedSeconds,
      valueLabel: formatChartDuration(entry.allocatedSeconds),
    })),
    {
      label: 'Unallocated',
      value: charts.unallocatedSeconds,
      valueLabel: formatChartDuration(charts.unallocatedSeconds),
    },
  ].filter((entry) => entry.value > 0);

  const subjectData = charts.studyBySubject.map((entry) => ({
    label: `${examName(exams, entry.examCode)} · ${entry.subjectTitle || 'Unclassified'}`,
    value: entry.allocatedSeconds,
    valueLabel: formatChartDuration(entry.allocatedSeconds),
  }));
  const subjectTop = subjectData.slice(0, 8);
  const subjectOther = subjectData.slice(8).reduce((sum, entry) => sum + entry.value, 0);
  if (subjectOther > 0) {
    subjectTop.push({
      label: 'Other subjects',
      value: subjectOther,
      valueLabel: formatChartDuration(subjectOther),
    });
  }

  const outcomes = charts.questionOutcomes;
  const outcomeDonut = [
    { label: 'Correct', value: outcomes.correct },
    { label: 'Incorrect', value: outcomes.incorrect },
    { label: 'Unanswered', value: outcomes.unanswered },
  ].filter((entry) => entry.value > 0);

  const topicRows = charts.topicAccuracy.map((entry) => [
    `${examName(exams, entry.examCode)} · ${entry.topicTitle}`,
    entry.attempts,
    entry.gradedAttempts,
    formatChartPercent(entry.accuracyPercentage),
    entry.insufficientSample ? 'Insufficient sample' : 'Reportable',
  ]);
  const topicBars = charts.topicAccuracy
    .filter((entry) => !entry.insufficientSample && entry.accuracyPercentage !== null)
    .sort((left, right) => right.gradedAttempts - left.gradedAttempts)
    .slice(0, 10)
    .map((entry) => ({
      label: `${examName(exams, entry.examCode)} · ${entry.topicTitle}`,
      value: entry.accuracyPercentage ?? 0,
      valueLabel: `${(entry.accuracyPercentage ?? 0).toFixed(1)}% (n=${entry.gradedAttempts})`,
    }));

  const dailySeries = [
    {
      label: 'Study time',
      points: charts.dailyStudy.map((point) => ({
        x: point.label,
        y: point.seconds,
      })),
    },
  ];

  const consistencyBars = charts.weeklyConsistency.map((week) => ({
    label: week.weekStart,
    value: week.eligibleDays > 0 ? (week.activeDays / week.eligibleDays) * 100 : 0,
    valueLabel: `${week.activeDays}/${week.eligibleDays} days`,
  }));

  const syllabusBars = charts.syllabusCompletion
    .filter((entry) => entry.topicCount > 0 && entry.completionPercentage !== null)
    .map((entry) => ({
      label: entry.examShortName,
      value: entry.completionPercentage ?? 0,
      valueLabel: `${entry.completedTopicCount}/${entry.topicCount} topics`,
    }));

  const mockSeries = [...new Set(charts.mockTrend.map((point) => point.examCode))].map((code) => ({
    label: examName(exams, code),
    points: charts.mockTrend
      .filter((point) => point.examCode === code)
      .map((point) => ({ x: point.testDate, y: point.percentage })),
  }));
  const mockRows = charts.mockTrend.map((point) => [
    examName(exams, point.examCode),
    point.testDate,
    point.testName,
    formatChartPercent(point.percentage),
  ]);

  const comparisonRows = charts.syllabusCompletion.map((completion) => {
    const study = charts.studyByExam.find((entry) => entry.examId === completion.examId);
    const accuracy = charts.outcomesByExam.find((entry) => entry.examId === completion.examId);
    const mocks = charts.mockByExam.find((entry) => entry.examId === completion.examId);
    return [
      completion.examShortName,
      formatChartDuration(study?.allocatedSeconds ?? 0),
      formatChartPercent(completion.completionPercentage),
      formatChartPercent(accuracy?.outcomes.accuracyPercentage ?? null),
      formatChartPercent(mocks?.averagePercentage ?? null),
      `${accuracy?.outcomes.gradedAttempts ?? 0} graded · ${mocks?.completedCount ?? 0} mocks`,
    ];
  });
  const comparisonCount = new Set([
    ...charts.studyByExam
      .filter((entry) => entry.allocatedSeconds > 0)
      .map((entry) => entry.examId),
    ...charts.outcomesByExam
      .filter((entry) => entry.outcomes.attempts > 0)
      .map((entry) => entry.examId),
    ...charts.mockByExam.map((entry) => entry.examId),
  ]).size;

  return (
    <div className="v2-chart-grid">
      <ChartFrame
        columns={['Group', 'Allocated time', 'Sessions']}
        description="Allocated study seconds grouped by examination. Time not allocated to an exam is shown separately so the composition remains honest."
        rows={[
          ...charts.studyByExam.map((entry) => [
            entry.examShortName,
            formatChartDuration(entry.allocatedSeconds),
            entry.sessionCount,
          ]),
          ['Unallocated', formatChartDuration(charts.unallocatedSeconds), ''],
        ]}
        title="Study time by examination"
      >
        {charts.totalSeconds === 0 ? (
          <p className="v2-empty-panel-copy">No study sessions in this range.</p>
        ) : (
          <DonutChart ariaLabel="Study time by examination" data={studyDonut} />
        )}
      </ChartFrame>

      <ChartFrame
        columns={['Subject', 'Study time']}
        description="Topic-allocated study time grouped by the parent syllabus unit. Combined totals still include unallocated time."
        rows={charts.studyBySubject.map((entry) => [
          `${examName(exams, entry.examCode)} · ${entry.subjectTitle || 'Unclassified'}`,
          formatChartDuration(entry.allocatedSeconds),
        ])}
        title="Study time by subject"
      >
        {subjectTop.length === 0 ? (
          <p className="v2-empty-panel-copy">
            No topic-allocated study time in this range. Sessions without topic allocations remain
            counted in total study time.
          </p>
        ) : (
          <BarChart ariaLabel="Study time by subject" data={subjectTop} />
        )}
      </ChartFrame>

      <ChartFrame
        columns={['Outcome', 'Count']}
        description="PYQ attempt outcomes. Accuracy excludes unanswered attempts and uses correct divided by correct plus incorrect."
        rows={[
          ['Correct', outcomes.correct],
          ['Incorrect', outcomes.incorrect],
          ['Unanswered', outcomes.unanswered],
        ]}
        title="Question outcomes"
      >
        {outcomes.attempts === 0 ? (
          <p className="v2-empty-panel-copy">No PYQ attempts in this range.</p>
        ) : (
          <>
            <DonutChart
              ariaLabel="Correct, incorrect and unanswered questions"
              data={outcomeDonut}
            />
            <p className="v2-chart-note">
              Accuracy {formatChartPercent(outcomes.accuracyPercentage)} from{' '}
              {outcomes.gradedAttempts} graded attempt
              {outcomes.gradedAttempts === 1 ? '' : 's'}.
              {outcomes.insufficientSample
                ? ' Fewer than five graded attempts: treat this percentage as provisional.'
                : ''}
            </p>
          </>
        )}
      </ChartFrame>

      <ChartFrame
        columns={['Topic', 'Attempts', 'Graded', 'Accuracy', 'Sample']}
        description="Primary syllabus topic per question, so multi-topic questions are counted once. Topic percentages are charted only after five graded attempts."
        rows={topicRows}
        title="Topic-wise accuracy"
      >
        {topicBars.length === 0 ? (
          <p className="v2-empty-panel-copy">
            No topic has at least five graded attempts yet. Counts remain available in the data
            table.
          </p>
        ) : (
          <BarChart ariaLabel="Topic accuracy percentages" data={topicBars} maxValue={100} />
        )}
      </ChartFrame>

      <ChartFrame
        className="v2-chart-card-wide"
        columns={['Date', 'Study time', 'Sessions']}
        description="Daily study time from recorded sessions. Missing dates are not zero-filled."
        rows={charts.dailyStudy.map((point) => [
          point.label,
          formatChartDuration(point.seconds),
          point.sessionCount,
        ])}
        title="Daily study trend"
      >
        {charts.dailyStudy.length === 0 ? (
          <p className="v2-empty-panel-copy">No study sessions in this range.</p>
        ) : (
          <LineChart
            ariaLabel="Daily study time"
            formatValue={formatChartDuration}
            series={dailySeries}
          />
        )}
      </ChartFrame>

      <ChartFrame
        columns={['Week starting', 'Active days', 'Eligible days', 'Study time']}
        description="Active study days divided by the calendar days available in each displayed week."
        rows={charts.weeklyConsistency.map((week) => [
          week.weekStart,
          week.activeDays,
          week.eligibleDays,
          formatChartDuration(week.studySeconds),
        ])}
        title="Weekly study consistency"
      >
        {consistencyBars.length === 0 ? (
          <p className="v2-empty-panel-copy">No weekly range data.</p>
        ) : (
          <BarChart ariaLabel="Active days per week" data={consistencyBars} maxValue={100} />
        )}
      </ChartFrame>

      <ChartFrame
        columns={['Examination', 'Completed topics', 'Total topics', 'Completion']}
        description="A topic counts as completed when its existing progress status is revised or confident."
        rows={charts.syllabusCompletion.map((entry) => [
          entry.examShortName,
          entry.completedTopicCount,
          entry.topicCount,
          formatChartPercent(entry.completionPercentage),
        ])}
        title="Syllabus completion"
      >
        {syllabusBars.length === 0 ? (
          <p className="v2-empty-panel-copy">No verified syllabus topics are stored.</p>
        ) : (
          <BarChart
            ariaLabel="Syllabus completion percentages"
            data={syllabusBars}
            maxValue={100}
          />
        )}
      </ChartFrame>

      <ChartFrame
        className="v2-chart-card-wide"
        columns={['Examination', 'Date', 'Mock', 'Percentage']}
        description="Completed, non-archived mock tests with a published percentage. Negative percentages are preserved."
        rows={mockRows}
        title="Mock-test score trend"
      >
        {charts.mockTrend.length === 0 ? (
          <p className="v2-empty-panel-copy">No completed mocks in this range.</p>
        ) : charts.mockTrend.length < 2 ? (
          <p className="v2-empty-panel-copy">
            Only one completed mock in this range. A trend line needs at least two completed mocks;
            the point is shown in the data table.
          </p>
        ) : (
          <LineChart ariaLabel="Mock-test score percentages" series={mockSeries} valueSuffix="%" />
        )}
      </ChartFrame>

      <ChartFrame
        className="v2-chart-card-wide"
        columns={[
          'Examination',
          'Study time',
          'Syllabus',
          'PYQ accuracy',
          'Mock average',
          'Samples',
        ]}
        description="Study time, syllabus completion, PYQ accuracy and mock averages are compared separately; percentages are never plotted on the seconds axis."
        rows={comparisonRows}
        title="Exam-wise comparison"
      >
        {comparisonRows.length === 0 ? (
          <p className="v2-empty-panel-copy">No examination data in this range.</p>
        ) : (
          <>
            <BarChart
              ariaLabel="Allocated study time by examination"
              data={charts.studyByExam.map((entry) => ({
                label: entry.examShortName,
                value: entry.allocatedSeconds,
                valueLabel: formatChartDuration(entry.allocatedSeconds),
              }))}
            />
            {comparisonCount < 2 ? (
              <p className="v2-chart-note">
                Only one examination has data in this range, so no comparison should be inferred.
              </p>
            ) : null}
          </>
        )}
      </ChartFrame>
    </div>
  );
}
