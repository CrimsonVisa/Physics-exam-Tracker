import { useEffect, useState } from 'react';
import { DateField } from '../components/DateField';
import { HorizontalBarList, MetricBarChart } from '../components/AnalyticsCharts';
import { toCommandError } from '../lib/commandError';
import {
  addDaysDateOnly,
  formatDateLong,
  formatDateWithWeekday,
  parseDateOnly,
  startOfMonthDateOnly,
} from '../lib/dateOnly';
import {
  getPerformanceAnalytics,
  type MockAnalytics,
  type PyqAnalytics,
  type PerformanceAnalytics,
} from '../lib/analyticsApi';
import {
  getPreparationTimeline,
  getStudyAnalytics,
  getTopicIntelligence,
  type PreparationTimeline,
  type StudyAnalytics,
  type TimelineEvent,
  type TopicEvidence,
  type TopicIntelligence,
} from '../lib/preparationApi';
import { formatStudyDuration } from '../lib/studyTime';

export type AnalyticsTab = 'study' | 'performance' | 'topics' | 'timeline';

type Range = {
  fromDate: string;
  toDate: string;
};

type AnalyticsScreenProps = {
  today: string;
  examDate: string;
  initialTab?: AnalyticsTab;
  onTabChange?: (tab: AnalyticsTab) => void;
};

const TABS: Array<{ id: AnalyticsTab; label: string; eyebrow: string }> = [
  { id: 'study', label: 'Study analytics', eyebrow: 'Time, targets and streak' },
  { id: 'performance', label: 'Exam performance', eyebrow: 'PYQ and mock tests, separately' },
  { id: 'topics', label: 'Topic intelligence', eyebrow: 'Evidence and revision signals' },
  { id: 'timeline', label: 'Preparation timeline', eyebrow: 'Consolidated dated record' },
];

const TIMELINE_TYPES: Array<{ kind: string; label: string }> = [
  { kind: 'exam', label: 'Exam date' },
  { kind: 'deadline', label: 'Checklist deadlines' },
  { kind: 'planned_block', label: 'Planned blocks' },
  { kind: 'revision', label: 'Scheduled revisions' },
  { kind: 'study_session', label: 'Study sessions' },
  { kind: 'mock_test', label: 'Mock tests' },
];

function defaultRange(today: string): Range {
  return {
    fromDate: addDaysDateOnly(today, -27) ?? today,
    toDate: today,
  };
}

function defaultTimelineRange(today: string, examDate: string): Range {
  const forward = addDaysDateOnly(today, 90) ?? today;
  const parsedExam = parseDateOnly(examDate);
  const parsedForward = parseDateOnly(forward);
  return {
    fromDate: addDaysDateOnly(today, -30) ?? today,
    toDate: parsedExam && parsedForward && examDate > forward ? examDate : forward,
  };
}

function percentageLabel(value: number | null): string {
  return value === null ? 'No data' : `${value.toFixed(1)}%`;
}

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function durationLabel(value: number): string {
  return formatStudyDuration(value);
}

function statusLabel(value: string): string {
  if (value === 'due_today') {
    return 'Due today';
  }
  return value.replaceAll('_', ' ').replace(/\b\w/g, (character) => character.toUpperCase());
}

function RangeControls({
  range,
  today,
  isLoading,
  onApply,
}: {
  range: Range;
  today: string;
  isLoading: boolean;
  onApply: (range: Range) => void;
}) {
  const [draft, setDraft] = useState<Range>(range);
  const [error, setError] = useState<string | null>(null);

  function apply(next: Range) {
    const from = parseDateOnly(next.fromDate);
    const to = parseDateOnly(next.toDate);
    if (!from || !to) {
      setError('Choose real calendar dates for both fields.');
      return;
    }
    if (next.fromDate > next.toDate) {
      setError('The start date cannot be after the end date.');
      return;
    }
    if (next.toDate > today) {
      setError('Study analytics cannot include future dates.');
      return;
    }
    setError(null);
    setDraft(next);
    onApply(next);
  }

  function quick(days: number) {
    apply({
      fromDate: addDaysDateOnly(today, -(days - 1)) ?? today,
      toDate: today,
    });
  }

  function thisMonth() {
    apply({
      fromDate: startOfMonthDateOnly(today) ?? today,
      toDate: today,
    });
  }

  return (
    <section className="analytics-range-card" aria-labelledby="analytics-range-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">Local calendar dates</p>
          <h2 id="analytics-range-heading">Date range</h2>
        </div>
        <span className="analytics-range-summary">
          {dateLabel(range.fromDate)} to {dateLabel(range.toDate)}
        </span>
      </div>
      <div className="analytics-range-controls">
        <DateField
          id="analytics-from-date"
          label="From date"
          value={draft.fromDate}
          max={today}
          onChange={(value) => {
            setDraft((current) => ({ ...current, fromDate: value }));
            setError(null);
          }}
          disabled={isLoading}
        />
        <DateField
          id="analytics-to-date"
          label="To date"
          value={draft.toDate}
          max={today}
          onChange={(value) => {
            setDraft((current) => ({ ...current, toDate: value }));
            setError(null);
          }}
          disabled={isLoading}
        />
        <div className="analytics-quick-ranges" aria-label="Quick ranges">
          <button type="button" className="secondary-button" onClick={() => quick(7)}>
            Last 7 days
          </button>
          <button type="button" className="secondary-button" onClick={() => quick(28)}>
            Last 28 days
          </button>
          <button type="button" className="secondary-button" onClick={() => quick(90)}>
            Last 90 days
          </button>
          <button type="button" className="secondary-button" onClick={thisMonth}>
            This month
          </button>
          <button
            type="button"
            className="primary-button"
            onClick={() => apply(draft)}
            disabled={isLoading}
          >
            {isLoading ? 'Applying…' : 'Apply range'}
          </button>
        </div>
      </div>
      {error && (
        <p className="field-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}

function StudyTab({ study }: { study: StudyAnalytics }) {
  const daily = study.daily.map((day) => ({
    key: day.date,
    label: day.date.slice(5),
    title: `${dateLabel(day.date)} · ${durationLabel(day.seconds)}${day.eligible ? '' : ' · before preparation start'}`,
    value: day.seconds,
  }));
  const weekly = study.weekly.map((week) => ({
    key: week.weekStart,
    label: week.weekStart.slice(5),
    title: `${dateLabel(week.weekStart)} to ${dateLabel(week.weekEnd)} · ${durationLabel(week.seconds)}`,
    value: week.seconds,
  }));
  const monthly = study.monthly.map((month) => ({
    key: month.month,
    label: month.month,
    title: `${dateLabel(month.monthStart)} to ${dateLabel(month.monthEnd)} · ${durationLabel(month.seconds)}`,
    value: month.seconds,
  }));
  const units = study.byUnit.map((unit) => ({
    key: unit.sectionCode,
    label: unit.sectionTitle,
    title: `${unit.sessionCount} sessions · ${durationLabel(unit.seconds)}`,
    value: unit.seconds,
  }));
  const durations = study.durationBuckets.map((bucket) => ({
    key: bucket.key,
    label: bucket.label,
    title: `${bucket.sessionCount} sessions · ${durationLabel(bucket.seconds)}`,
    value: bucket.sessionCount,
  }));
  const change = study.previousPeriod.secondsChangePercent;
  const hasSessions = study.rangeSessionCount > 0;

  return (
    <>
      <section className="analytics-overview" aria-label="Study analytics overview">
        <div className="analytics-stat-card">
          <span>Recorded study time</span>
          <strong>{durationLabel(study.rangeSeconds)}</strong>
          <small>{study.rangeSessionCount} focused sessions in this range</small>
        </div>
        <div className="analytics-stat-card">
          <span>Daily target completion</span>
          <strong>{percentageLabel(study.targetCompletionRate)}</strong>
          <small>
            {study.targetMetDays} of {study.eligibleDays} eligible days
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>Active-day consistency</span>
          <strong>{percentageLabel(study.consistentDaysPercent)}</strong>
          <small>
            {study.activeDays} active days · {study.missedTargetDays} completed days missed target
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>Current study streak</span>
          <strong>{study.currentStreak}</strong>
          <small>
            Longest {study.longestStreak} days · {study.totalActiveDays} total active days
          </small>
        </div>
      </section>

      <section className="analytics-card" aria-labelledby="comparison-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Same-length comparison</p>
            <h2 id="comparison-heading">Current versus previous period</h2>
          </div>
          <span className="analytics-range-summary">
            Previous: {dateLabel(study.previousPeriod.fromDate)} to{' '}
            {dateLabel(study.previousPeriod.toDate)}
          </span>
        </div>
        <div className="analytics-comparison">
          <div>
            <span>Current period</span>
            <strong>{durationLabel(study.rangeSeconds)}</strong>
            <small>
              {study.activeDays} active days · target rate{' '}
              {percentageLabel(study.targetCompletionRate)}
            </small>
          </div>
          <div>
            <span>Previous period</span>
            <strong>{durationLabel(study.previousPeriod.seconds)}</strong>
            <small>
              {study.previousPeriod.activeDays} active days · target rate{' '}
              {percentageLabel(study.previousPeriod.targetCompletionRate)}
            </small>
          </div>
          <div>
            <span>Study-time change</span>
            <strong className={change !== null && change < 0 ? 'analytics-negative' : undefined}>
              {change === null ? 'Not comparable' : `${change > 0 ? '+' : ''}${change.toFixed(1)}%`}
            </strong>
            <small>
              A saved-total comparison only. It does not establish why study time changed.
            </small>
          </div>
        </div>
        <p className="analytics-note">
          The previous period has the same number of calendar days and ends the day before the
          current range starts. Days before the preparation start date never count toward targets or
          consistency. A current streak may end yesterday while today has no session yet; a full
          missed calendar day ends it.
        </p>
      </section>

      <section className="analytics-card" aria-labelledby="daily-trend-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Daily trend · seconds converted to readable durations</p>
            <h2 id="daily-trend-heading">Study time by day</h2>
          </div>
          <span className="analytics-range-summary">
            Daily target {durationLabel(study.dailyTargetSeconds)}
          </span>
        </div>
        {!hasSessions && (
          <p className="analytics-empty">
            No study sessions in this range. Chart values are real saved zeros, not fabricated
            activity.
          </p>
        )}
        {study.fromDate < study.preparationStartDate && (
          <p className="analytics-note">
            This range starts before the preparation start date. Any saved sessions on those earlier
            dates remain in recorded-time totals, but those dates are not eligible for targets,
            consistency, or streaks.
          </p>
        )}
        <MetricBarChart
          data={daily}
          ariaLabel={`Daily study time from ${study.fromDate} to ${study.toDate}. Maximum ${durationLabel(Math.max(study.dailyTargetSeconds, ...daily.map((point) => point.value)))}.`}
          formatValue={durationLabel}
          referenceValue={study.dailyTargetSeconds}
          referenceLabel="Daily target"
          emptyMessage="No dates in this range."
        />
      </section>

      <div className="analytics-two-column">
        <section className="analytics-card" aria-labelledby="weekly-trend-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Monday-based weeks</p>
              <h2 id="weekly-trend-heading">Weekly study time</h2>
            </div>
          </div>
          <MetricBarChart
            data={weekly}
            ariaLabel="Weekly study time for the selected range."
            formatValue={durationLabel}
            emptyMessage="No weeks in this range."
          />
          <p className="analytics-note">
            Weekly target completion: {study.weeklyTargetsMet} of {study.weeklyTargetsTotal} weeks (
            {percentageLabel(study.weeklyTargetCompletionRate)}). Partial weeks use their eligible
            day count for the target.
          </p>
        </section>

        <section className="analytics-card" aria-labelledby="monthly-trend-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Calendar months</p>
              <h2 id="monthly-trend-heading">Monthly study time</h2>
            </div>
          </div>
          <MetricBarChart
            data={monthly}
            ariaLabel="Monthly study time for the selected range."
            formatValue={durationLabel}
            emptyMessage="No months in this range."
          />
          <p className="analytics-note">
            Totals include only recorded focus sessions. Calendar months and leap days use local
            date strings without timezone conversion.
          </p>
        </section>
      </div>

      <div className="analytics-two-column">
        <section className="analytics-card" aria-labelledby="duration-buckets-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Session duration</p>
              <h2 id="duration-buckets-heading">Sessions by duration</h2>
            </div>
          </div>
          <HorizontalBarList
            data={durations}
            formatValue={(value) => `${value} session${value === 1 ? '' : 's'}`}
            emptyMessage="No sessions in this range."
            ariaLabel="Session counts by duration bucket."
          />
          <table className="analytics-table analytics-table-compact analytics-inline-table">
            <thead>
              <tr>
                <th scope="col">Bucket</th>
                <th scope="col">Sessions</th>
                <th scope="col">Recorded time</th>
              </tr>
            </thead>
            <tbody>
              {study.durationBuckets.map((bucket) => (
                <tr key={`duration-row-${bucket.key}`}>
                  <th scope="row">{bucket.label}</th>
                  <td>{bucket.sessionCount}</td>
                  <td>{durationLabel(bucket.seconds)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section className="analytics-card" aria-labelledby="unit-time-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Syllabus attribution</p>
              <h2 id="unit-time-heading">Study time by unit</h2>
            </div>
          </div>
          <HorizontalBarList
            data={units}
            formatValue={durationLabel}
            emptyMessage="No syllabus-linked study time in this range."
            ariaLabel="Study time by syllabus unit."
          />
          {study.unassignedSessionCount > 0 && (
            <p className="analytics-note">
              Unassigned sessions: {study.unassignedSessionCount} ·{' '}
              {durationLabel(study.unassignedSeconds)}. These were recorded without a syllabus
              topic.
            </p>
          )}
          {study.unassignedSessionCount === 0 && (
            <p className="analytics-note">
              Every session in this range has a syllabus topic or there are no sessions.
            </p>
          )}
        </section>
      </div>

      <div className="analytics-two-column">
        <section className="analytics-card" aria-labelledby="focus-break-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">What the database supports</p>
              <h2 id="focus-break-heading">Focus versus break time</h2>
            </div>
          </div>
          <div className="analytics-comparison analytics-comparison-compact">
            <div>
              <span>Recorded focus time</span>
              <strong>{durationLabel(study.focusSeconds)}</strong>
              <small>Completed focus sessions in the selected range.</small>
            </div>
            <div>
              <span>Break time</span>
              <strong>Not tracked</strong>
              <small>
                Timer breaks are not saved as study sessions, so break totals cannot be calculated
                honestly.
              </small>
            </div>
          </div>
          <p className="analytics-note">
            Break tracking is marked unsupported rather than estimated. Planned blocks also remain
            outside study totals.
          </p>
        </section>

        <section className="analytics-card" aria-labelledby="revision-consistency-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Saved revision outcomes</p>
              <h2 id="revision-consistency-heading">Revision consistency</h2>
            </div>
          </div>
          <dl className="analytics-facts">
            <div>
              <dt>Completed in range</dt>
              <dd>{study.revisionConsistency.completedInRange}</dd>
            </div>
            <div>
              <dt>On time</dt>
              <dd>{study.revisionConsistency.onTimeCount}</dd>
            </div>
            <div>
              <dt>Late</dt>
              <dd>{study.revisionConsistency.lateCount}</dd>
            </div>
            <div>
              <dt>On-time rate</dt>
              <dd>{percentageLabel(study.revisionConsistency.onTimeRate)}</dd>
            </div>
            <div>
              <dt>Currently overdue</dt>
              <dd>{study.revisionConsistency.currentlyOverdueCount}</dd>
            </div>
            <div>
              <dt>Active schedules</dt>
              <dd>{study.revisionConsistency.activeScheduledCount}</dd>
            </div>
          </dl>
          <p className="analytics-note">{study.revisionConsistency.note}</p>
        </section>
      </div>
    </>
  );
}

function PyqSection({ pyq }: { pyq: PyqAnalytics }) {
  const weakIds = new Set(pyq.weakTopics.map((topic) => topic.topicId));
  return (
    <>
      <section className="analytics-overview" aria-label="PYQ performance overview">
        <div className="analytics-stat-card">
          <span>PYQ records</span>
          <strong>{pyq.totalRecords}</strong>
          <small>
            {pyq.correct} correct · {pyq.incorrect} incorrect · {pyq.skipped} skipped
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>PYQ accuracy</span>
          <strong>{percentageLabel(pyq.accuracy)}</strong>
          <small>
            {pyq.attempts} scored attempts
            {pyq.outcomeUnknown > 0 ? ` · ${pyq.outcomeUnknown} without an outcome` : ''}
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>Weak topics</span>
          <strong>{pyq.weakTopics.length}</strong>
          <small>
            At least {pyq.minimumWeakAttempts} scored attempts and below{' '}
            {pyq.weakAccuracyThreshold.toFixed(0)}%
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>Units with PYQ data</span>
          <strong>{pyq.byUnit.length}</strong>
          <small>{pyq.byTopic.length} topics with saved scored or skipped attempts</small>
        </div>
      </section>

      {pyq.totalRecords === 0 ? (
        <section className="analytics-card">
          <p className="analytics-empty">No PYQ attempts in this date range.</p>
        </section>
      ) : (
        <>
          <section className="analytics-card" aria-labelledby="pyq-topic-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Correct ÷ (correct + incorrect)</p>
                <h2 id="pyq-topic-heading">PYQ accuracy by topic</h2>
              </div>
            </div>
            <p className="analytics-note">
              Skipped and outcome-unknown attempts are shown but excluded from scored accuracy. A
              weak label requires at least {pyq.minimumWeakAttempts} scored attempts.
            </p>
            <div className="analytics-table-wrap">
              <table className="analytics-table">
                <thead>
                  <tr>
                    <th scope="col">Topic</th>
                    <th scope="col">Unit</th>
                    <th scope="col">Scored</th>
                    <th scope="col">Correct</th>
                    <th scope="col">Incorrect</th>
                    <th scope="col">Skipped</th>
                    <th scope="col">Accuracy</th>
                    <th scope="col">Evidence</th>
                  </tr>
                </thead>
                <tbody>
                  {pyq.byTopic.map((topic) => (
                    <tr key={`pyq-topic-${topic.topicId}`}>
                      <th scope="row">
                        {topic.code} · {topic.title}
                      </th>
                      <td>{topic.sectionTitle}</td>
                      <td>{topic.attempts}</td>
                      <td>{topic.correct}</td>
                      <td>{topic.incorrect}</td>
                      <td>{topic.skipped}</td>
                      <td>{percentageLabel(topic.accuracy)}</td>
                      <td>
                        {!topic.sufficientData ? (
                          <span className="analytics-tag analytics-tag-insufficient">
                            Insufficient data
                          </span>
                        ) : weakIds.has(topic.topicId) ? (
                          <span className="analytics-tag analytics-tag-weak">Weak</span>
                        ) : (
                          <span className="analytics-tag analytics-tag-ok">Sufficient</span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <div className="analytics-two-column">
            <section className="analytics-card" aria-labelledby="pyq-unit-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">PYQ only</p>
                  <h2 id="pyq-unit-heading">Accuracy by unit</h2>
                </div>
              </div>
              <table className="analytics-table analytics-table-compact">
                <thead>
                  <tr>
                    <th scope="col">Unit</th>
                    <th scope="col">Scored</th>
                    <th scope="col">Accuracy</th>
                  </tr>
                </thead>
                <tbody>
                  {pyq.byUnit.map((unit) => (
                    <tr key={`pyq-unit-${unit.code}`}>
                      <th scope="row">{unit.title}</th>
                      <td>{unit.attempts}</td>
                      <td>{percentageLabel(unit.accuracy)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>

            <section className="analytics-card" aria-labelledby="pyq-year-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">PYQ only</p>
                  <h2 id="pyq-year-heading">Performance by examination year</h2>
                </div>
              </div>
              <table className="analytics-table analytics-table-compact">
                <thead>
                  <tr>
                    <th scope="col">Year</th>
                    <th scope="col">Scored</th>
                    <th scope="col">Correct</th>
                    <th scope="col">Incorrect</th>
                    <th scope="col">Accuracy</th>
                  </tr>
                </thead>
                <tbody>
                  {pyq.byYear.map((year) => (
                    <tr key={`pyq-year-${year.year}`}>
                      <th scope="row">{year.year}</th>
                      <td>{year.attempts}</td>
                      <td>{year.correct}</td>
                      <td>{year.incorrect}</td>
                      <td>{percentageLabel(year.accuracy)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          </div>

          <section className="analytics-card" aria-labelledby="pyq-parts-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">PYQ only</p>
                <h2 id="pyq-parts-heading">Part A, B and C performance</h2>
              </div>
            </div>
            <div className="analytics-part-grid">
              {pyq.byPart.map((part) => (
                <div className="analytics-part-card" key={`pyq-part-${part.part}`}>
                  <span>Part {part.part}</span>
                  <strong>{percentageLabel(part.accuracy)}</strong>
                  <small>
                    {part.correct} correct · {part.incorrect} incorrect · {part.skipped} skipped
                  </small>
                </div>
              ))}
            </div>
          </section>

          <section className="analytics-card" aria-labelledby="pyq-response-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Saved PYQ response trend</p>
                <h2 id="pyq-response-heading">Correct and incorrect responses by attempt date</h2>
              </div>
            </div>
            <div className="analytics-table-wrap analytics-table-scroll">
              <table className="analytics-table analytics-table-compact">
                <thead>
                  <tr>
                    <th scope="col">Date</th>
                    <th scope="col">Correct</th>
                    <th scope="col">Incorrect</th>
                    <th scope="col">Skipped</th>
                    <th scope="col">No outcome</th>
                  </tr>
                </thead>
                <tbody>
                  {pyq.responseTrend.map((point) => (
                    <tr key={`pyq-response-${point.date}`}>
                      <th scope="row">{dateLabel(point.date)}</th>
                      <td>{point.correct}</td>
                      <td>{point.incorrect}</td>
                      <td>{point.skipped}</td>
                      <td>{point.outcomeUnknown}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}
    </>
  );
}

function MockSection({ mock }: { mock: MockAnalytics }) {
  return (
    <>
      <section className="analytics-overview" aria-label="Mock-test performance overview">
        <div className="analytics-stat-card">
          <span>Mock tests</span>
          <strong>{mock.testCount}</strong>
          <small>{mock.totalQuestions} questions across saved tests</small>
        </div>
        <div className="analytics-stat-card">
          <span>Average calculated score</span>
          <strong>{percentageLabel(mock.averagePercentage)}</strong>
          <small>Best {percentageLabel(mock.bestPercentage)}</small>
        </div>
        <div className="analytics-stat-card">
          <span>Latest calculated score</span>
          <strong>{percentageLabel(mock.latestPercentage)}</strong>
          <small>Each test uses its own saved marking configuration</small>
        </div>
        <div className="analytics-stat-card">
          <span>Attempted / unanswered</span>
          <strong>
            {mock.totalAttempted} / {mock.totalUnanswered}
          </strong>
          <small>
            {mock.totalCorrect} correct · {mock.totalIncorrect} incorrect
          </small>
        </div>
      </section>

      {mock.testCount === 0 ? (
        <section className="analytics-card">
          <p className="analytics-empty">No mock tests in this date range.</p>
        </section>
      ) : (
        <>
          <section className="analytics-card" aria-labelledby="mock-trend-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Calculated from saved marking rules</p>
                <h2 id="mock-trend-heading">Mock-test score trend</h2>
              </div>
            </div>
            <div className="analytics-table-wrap">
              <table className="analytics-table">
                <thead>
                  <tr>
                    <th scope="col">Date</th>
                    <th scope="col">Test</th>
                    <th scope="col">Calculated marks</th>
                    <th scope="col">Calculated percentage</th>
                    <th scope="col">Reported marks</th>
                  </tr>
                </thead>
                <tbody>
                  {mock.trend.map((point) => (
                    <tr key={`mock-trend-${point.id}`}>
                      <th scope="row">{dateLabel(point.testDate)}</th>
                      <td>{point.testName}</td>
                      <td>
                        {point.computedMarks.toFixed(2)} / {point.maxMarks.toFixed(2)}
                      </td>
                      <td>
                        <div className="analytics-score-cell">
                          <span>{point.computedPercentage.toFixed(1)}%</span>
                          <div className="analytics-bar" aria-hidden="true">
                            <span
                              style={{
                                width: `${Math.max(0, Math.min(100, point.computedPercentage))}%`,
                              }}
                            />
                          </div>
                        </div>
                      </td>
                      <td>
                        {point.reportedMarks === null
                          ? 'Not recorded'
                          : point.reportedMarks.toFixed(2)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <div className="analytics-two-column">
            <section className="analytics-card" aria-labelledby="mock-part-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Mock tests only</p>
                  <h2 id="mock-part-heading">Part-wise results</h2>
                </div>
              </div>
              <table className="analytics-table analytics-table-compact">
                <thead>
                  <tr>
                    <th scope="col">Part</th>
                    <th scope="col">Attempted</th>
                    <th scope="col">Unanswered</th>
                    <th scope="col">Correct</th>
                    <th scope="col">Incorrect</th>
                    <th scope="col">Accuracy</th>
                  </tr>
                </thead>
                <tbody>
                  {mock.byPart.map((part) => (
                    <tr key={`mock-part-${part.part}`}>
                      <th scope="row">Part {part.part}</th>
                      <td>{part.attempted}</td>
                      <td>{part.unanswered}</td>
                      <td>{part.correct}</td>
                      <td>{part.incorrect}</td>
                      <td>{percentageLabel(part.accuracy)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>

            <section className="analytics-card" aria-labelledby="mock-response-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Mock tests only</p>
                  <h2 id="mock-response-heading">Response trend by test date</h2>
                </div>
              </div>
              <div className="analytics-table-wrap analytics-table-scroll">
                <table className="analytics-table analytics-table-compact">
                  <thead>
                    <tr>
                      <th scope="col">Date</th>
                      <th scope="col">Tests</th>
                      <th scope="col">Correct</th>
                      <th scope="col">Incorrect</th>
                      <th scope="col">Unanswered</th>
                      <th scope="col">Avg score</th>
                    </tr>
                  </thead>
                  <tbody>
                    {mock.responseTrend.map((point) => (
                      <tr key={`mock-response-${point.date}`}>
                        <th scope="row">{dateLabel(point.date)}</th>
                        <td>{point.testCount}</td>
                        <td>{point.correct}</td>
                        <td>{point.incorrect}</td>
                        <td>{point.unanswered}</td>
                        <td>{percentageLabel(point.averagePercentage)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          </div>
        </>
      )}
    </>
  );
}

function PerformanceTab({ performance }: { performance: PerformanceAnalytics }) {
  return (
    <>
      <section className="analytics-separation-card" role="note">
        <strong>Separate domains:</strong> {performance.separationNote}
      </section>
      <PyqSection pyq={performance.pyq} />
      <div className="analytics-domain-divider" aria-hidden="true" />
      <MockSection mock={performance.mock} />
    </>
  );
}

function EvidenceRow({ evidence }: { evidence: TopicEvidence }) {
  return (
    <article className="topic-evidence-card">
      <div className="topic-evidence-heading">
        <div>
          <h4>
            {evidence.code} · {evidence.title}
          </h4>
          <p className="topic-evidence-meta">
            {evidence.sectionTitle} · syllabus status {evidence.syllabusStatus} · confidence{' '}
            {evidence.confidence}/5
          </p>
        </div>
        <span className={`revision-badge revision-badge-${evidence.dueState.replaceAll('_', '-')}`}>
          {statusLabel(evidence.dueState)}
        </span>
      </div>
      <ul className="revision-reasons">
        {evidence.reasons.map((reason) => (
          <li key={`${evidence.topicId}-${reason}`}>{reason}</li>
        ))}
      </ul>
      <dl className="topic-evidence-facts">
        <div>
          <dt>PYQ accuracy</dt>
          <dd>{percentageLabel(evidence.pyqAccuracy)}</dd>
        </div>
        <div>
          <dt>Scored attempts</dt>
          <dd>{evidence.pyqAttempts}</dd>
        </div>
        <div>
          <dt>Correct / incorrect</dt>
          <dd>
            {evidence.pyqCorrect} / {evidence.pyqIncorrect}
          </dd>
        </div>
        <div>
          <dt>Revision completed</dt>
          <dd>{evidence.completedRevisions}</dd>
        </div>
        <div>
          <dt>Revision mastery</dt>
          <dd>
            {evidence.masteryEstimate === null
              ? 'Not available'
              : `${evidence.masteryEstimate.toFixed(1)}%`}
          </dd>
        </div>
        <div>
          <dt>Next revision</dt>
          <dd>
            {evidence.nextRevisionDate ? dateLabel(evidence.nextRevisionDate) : 'Not scheduled'}
          </dd>
        </div>
      </dl>
    </article>
  );
}

function EvidenceGroup({
  title,
  eyebrow,
  items,
  emptyMessage,
  open = false,
}: {
  title: string;
  eyebrow: string;
  items: TopicEvidence[];
  emptyMessage: string;
  open?: boolean;
}) {
  return (
    <details className="analytics-evidence-group" open={open}>
      <summary>
        <span>
          <span className="eyebrow">{eyebrow}</span>
          <strong>{title}</strong>
        </span>
        <span className="analytics-evidence-count">{items.length}</span>
      </summary>
      {items.length === 0 ? (
        <p className="analytics-empty">{emptyMessage}</p>
      ) : (
        <div className="topic-evidence-list">
          {items.map((evidence) => (
            <EvidenceRow evidence={evidence} key={`${title}-${evidence.topicId}`} />
          ))}
        </div>
      )}
    </details>
  );
}

function TopicsTab({ intelligence }: { intelligence: TopicIntelligence }) {
  return (
    <>
      <section className="analytics-overview" aria-label="Topic intelligence overview">
        <div className="analytics-stat-card">
          <span>Active topics</span>
          <strong>{intelligence.totalTopics}</strong>
          <small>{intelligence.scoredTopics} have at least one scored PYQ attempt</small>
        </div>
        <div className="analytics-stat-card">
          <span>No scored PYQ data</span>
          <strong>{intelligence.noDataTopics}</strong>
          <small>No weak or strong claim is inferred from silence</small>
        </div>
        <div className="analytics-stat-card">
          <span>Weak with evidence</span>
          <strong>{intelligence.weak.length}</strong>
          <small>
            Below {intelligence.thresholds.weakAccuracyThreshold.toFixed(0)}% with at least{' '}
            {intelligence.thresholds.minimumScoredAttempts} scored attempts
          </small>
        </div>
        <div className="analytics-stat-card">
          <span>Overdue revisions</span>
          <strong>{intelligence.overdueRevision.length}</strong>
          <small>From active saved revision schedules</small>
        </div>
      </section>

      <section className="analytics-card" aria-labelledby="topic-thresholds-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Transparent rules</p>
            <h2 id="topic-thresholds-heading">Evidence thresholds</h2>
          </div>
        </div>
        <p className="analytics-note">
          Strong: at least {intelligence.thresholds.minimumScoredAttempts} scored attempts and
          accuracy of {intelligence.thresholds.strongAccuracyThreshold.toFixed(0)}% or more. Weak:
          at least {intelligence.thresholds.minimumScoredAttempts} scored attempts and accuracy
          below {intelligence.thresholds.weakAccuracyThreshold.toFixed(0)}%. High-confidence
          mismatch uses confidence {intelligence.thresholds.highConfidenceMinimum}/5 or more; the
          reverse mismatch uses confidence {intelligence.thresholds.lowConfidenceMaximum}/5 or less.
        </p>
        {intelligence.notes.map((note) => (
          <p className="analytics-note" key={note}>
            {note}
          </p>
        ))}
      </section>

      <EvidenceGroup
        title="Weak topics with sufficient evidence"
        eyebrow="Needs targeted practice"
        items={intelligence.weak}
        emptyMessage="No topic currently meets the weak-topic evidence threshold."
        open
      />
      <EvidenceGroup
        title="Revision overdue"
        eyebrow="Schedule attention"
        items={intelligence.overdueRevision}
        emptyMessage="No saved revision schedule is overdue."
        open
      />
      <EvidenceGroup
        title="Needs revision"
        eyebrow="Due dates and previous outcomes"
        items={intelligence.needsRevision}
        emptyMessage="No saved revision signal currently needs attention."
        open
      />
      <EvidenceGroup
        title="High confidence, weak PYQ evidence"
        eyebrow="Confidence mismatch"
        items={intelligence.highConfidencePoorPyq}
        emptyMessage="No high-confidence topic has sufficient weak PYQ evidence."
        open
      />
      <EvidenceGroup
        title="Strong topics with sufficient evidence"
        eyebrow="Saved PYQ outcomes"
        items={intelligence.strong}
        emptyMessage="No topic currently meets the strong-topic evidence threshold."
      />
      <EvidenceGroup
        title="Insufficient performance data"
        eyebrow="Evidence threshold not reached"
        items={intelligence.insufficient}
        emptyMessage="No attempted topic is below the evidence threshold."
      />
      <EvidenceGroup
        title="Low confidence, strong PYQ evidence"
        eyebrow="Reverse confidence mismatch"
        items={intelligence.lowConfidenceStrongPyq}
        emptyMessage="No low-confidence topic has sufficient strong PYQ evidence."
      />
    </>
  );
}

function TimelineTab({
  today,
  initialRange,
  timeline,
  loading,
  error,
  onApply,
}: {
  today: string;
  initialRange: Range;
  timeline: PreparationTimeline | null;
  loading: boolean;
  error: string | null;
  onApply: (range: Range) => void;
}) {
  const [draft, setDraft] = useState<Range>(initialRange);
  const [rangeError, setRangeError] = useState<string | null>(null);
  const [includedKinds, setIncludedKinds] = useState<Set<string>>(
    () => new Set(TIMELINE_TYPES.map((type) => type.kind)),
  );

  function apply() {
    const from = parseDateOnly(draft.fromDate);
    const to = parseDateOnly(draft.toDate);
    if (!from || !to) {
      setRangeError('Choose real calendar dates for both fields.');
      return;
    }
    if (draft.fromDate > draft.toDate) {
      setRangeError('The start date cannot be after the end date.');
      return;
    }
    setRangeError(null);
    onApply(draft);
  }

  function toggleKind(kind: string) {
    setIncludedKinds((current) => {
      const next = new Set(current);
      if (next.has(kind)) {
        next.delete(kind);
      } else {
        next.add(kind);
      }
      return next;
    });
  }

  const filteredEvents = timeline?.events.filter((event) => includedKinds.has(event.kind)) ?? [];
  const grouped = filteredEvents.reduce<Array<{ date: string; events: TimelineEvent[] }>>(
    (groups, event) => {
      const last = groups.at(-1);
      if (last && last.date === event.date) {
        last.events.push(event);
      } else {
        groups.push({ date: event.date, events: [event] });
      }
      return groups;
    },
    [],
  );

  return (
    <>
      <section className="analytics-range-card" aria-labelledby="timeline-range-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Past and upcoming records</p>
            <h2 id="timeline-range-heading">Timeline date range</h2>
          </div>
          <span className="analytics-range-summary">
            {dateLabel(timeline?.fromDate ?? initialRange.fromDate)} to{' '}
            {dateLabel(timeline?.toDate ?? initialRange.toDate)}
          </span>
        </div>
        <div className="analytics-range-controls">
          <DateField
            id="timeline-from-date"
            label="Timeline from"
            value={draft.fromDate}
            onChange={(value) => {
              setDraft((current) => ({ ...current, fromDate: value }));
              setRangeError(null);
            }}
            disabled={loading}
          />
          <DateField
            id="timeline-to-date"
            label="Timeline to"
            value={draft.toDate}
            onChange={(value) => {
              setDraft((current) => ({ ...current, toDate: value }));
              setRangeError(null);
            }}
            disabled={loading}
          />
          <div className="analytics-quick-ranges">
            <button type="button" className="primary-button" onClick={apply} disabled={loading}>
              {loading ? 'Loading…' : 'Apply timeline range'}
            </button>
          </div>
        </div>
        {rangeError && (
          <p className="field-error" role="alert">
            {rangeError}
          </p>
        )}
      </section>

      <section className="analytics-card" aria-labelledby="timeline-filter-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Filter visible activity types</p>
            <h2 id="timeline-filter-heading">Timeline activity filter</h2>
          </div>
          <span className="analytics-range-summary">{filteredEvents.length} events</span>
        </div>
        <fieldset className="timeline-type-filters">
          <legend className="sr-only">Timeline activity types</legend>
          {TIMELINE_TYPES.map((type) => (
            <label key={type.kind}>
              <input
                type="checkbox"
                checked={includedKinds.has(type.kind)}
                onChange={() => toggleKind(type.kind)}
              />
              {type.label}
            </label>
          ))}
        </fieldset>
        <p className="analytics-note">
          Today is {dateLabel(today)}. A linked planned block is represented once by its completed
          study session when both records fall in the selected range.
        </p>
      </section>

      {loading && (
        <section className="state-card" role="status">
          <p className="state-title">Loading preparation timeline…</p>
        </section>
      )}
      {!loading && error && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Timeline could not be loaded</p>
          <p className="state-copy">{error}</p>
          <button type="button" className="primary-button" onClick={apply}>
            Retry timeline
          </button>
        </section>
      )}
      {!loading && !error && filteredEvents.length === 0 && (
        <section className="analytics-card">
          <p className="analytics-empty">
            No timeline events match this range and activity filter.
          </p>
        </section>
      )}
      {!loading && !error && grouped.length > 0 && (
        <section className="analytics-card" aria-labelledby="timeline-events-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Read-only consolidated record</p>
              <h2 id="timeline-events-heading">Preparation timeline</h2>
            </div>
          </div>
          <div className="timeline-groups">
            {grouped.map((group) => (
              <section className="timeline-day" key={`timeline-day-${group.date}`}>
                <h3>{formatDateWithWeekday(group.date) ?? group.date}</h3>
                <ol className="timeline-list">
                  {group.events.map((event) => (
                    <li className={`timeline-event timeline-event-${event.kind}`} key={event.id}>
                      <div>
                        <span className="timeline-kind">{statusLabel(event.kind)}</span>
                        <p className="timeline-title">{event.title}</p>
                        <p className="timeline-subtitle">
                          {event.subtitle}
                          {event.durationSeconds !== null
                            ? ` · ${durationLabel(event.durationSeconds)}`
                            : ''}
                        </p>
                      </div>
                      <span className={`prep-status prep-status-${event.status}`}>
                        {statusLabel(event.status)}
                      </span>
                    </li>
                  ))}
                </ol>
              </section>
            ))}
          </div>
          {timeline?.notes.map((note) => (
            <p className="analytics-note" key={note}>
              {note}
            </p>
          ))}
        </section>
      )}
    </>
  );
}

export default function AnalyticsScreen({
  today,
  examDate,
  initialTab = 'study',
  onTabChange,
}: AnalyticsScreenProps) {
  const [activeTab, setActiveTab] = useState<AnalyticsTab>(initialTab);
  const [range, setRange] = useState<Range>(() => defaultRange(today));
  const [performance, setPerformance] = useState<PerformanceAnalytics | null>(null);
  const [study, setStudy] = useState<StudyAnalytics | null>(null);
  const [topics, setTopics] = useState<TopicIntelligence | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  const [timelineRange, setTimelineRange] = useState<Range>(() =>
    defaultTimelineRange(today, examDate),
  );
  const [timeline, setTimeline] = useState<PreparationTimeline | null>(null);
  const [timelineLoading, setTimelineLoading] = useState(true);
  const [timelineError, setTimelineError] = useState<string | null>(null);
  const [timelineReloadKey, setTimelineReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;

    Promise.all([
      getPerformanceAnalytics(range.fromDate, range.toDate),
      getStudyAnalytics(range.fromDate, range.toDate),
      getTopicIntelligence(),
    ])
      .then(([loadedPerformance, loadedStudy, loadedTopics]) => {
        if (!cancelled) {
          setPerformance(loadedPerformance);
          setStudy(loadedStudy);
          setTopics(loadedTopics);
          setLoadError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [range, reloadKey]);

  useEffect(() => {
    let cancelled = false;

    getPreparationTimeline(timelineRange.fromDate, timelineRange.toDate)
      .then((loaded) => {
        if (!cancelled) {
          setTimeline(loaded);
          setTimelineError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setTimelineError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setTimelineLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [timelineRange, timelineReloadKey]);

  const activeTabMeta = TABS.find((tab) => tab.id === activeTab) ?? TABS[0];

  return (
    <div className="page-container analytics-page">
      <header className="page-header">
        <p className="eyebrow">Analytics</p>
        <h1>Study performance</h1>
        <p className="page-lede">
          Study totals, PYQ outcomes, mock scores, topic evidence, and the preparation timeline use
          only saved local records. PYQ and mock-test results stay separate, and no readiness, rank,
          selection probability, or success percentage is invented.
        </p>
      </header>

      <div className="analytics-tabs" role="tablist" aria-label="Analytics views">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            id={`analytics-tab-${tab.id}`}
            type="button"
            role="tab"
            aria-selected={activeTab === tab.id}
            aria-controls={`analytics-panel-${tab.id}`}
            className={`analytics-tab${activeTab === tab.id ? ' analytics-tab-active' : ''}`}
            onClick={() => {
              setActiveTab(tab.id);
              onTabChange?.(tab.id);
            }}
          >
            <span>{tab.label}</span>
            <small>{tab.eyebrow}</small>
          </button>
        ))}
      </div>

      {(activeTab === 'study' || activeTab === 'performance') && (
        <RangeControls
          key={`${range.fromDate}:${range.toDate}`}
          range={range}
          today={today}
          isLoading={isLoading}
          onApply={(next) => {
            setIsLoading(true);
            setRange(next);
          }}
        />
      )}

      {isLoading && activeTab !== 'timeline' && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Calculating {activeTabMeta.label.toLowerCase()}…</p>
          <p className="state-copy">
            Reading saved sessions, PYQ attempts, mock tests, revision outcomes, and syllabus
            evidence for the selected dates.
          </p>
        </section>
      )}

      {!isLoading && loadError && activeTab !== 'timeline' && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Analytics could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              setReloadKey((key) => key + 1);
            }}
          >
            Retry analytics
          </button>
        </section>
      )}

      {!isLoading && !loadError && performance && study && topics && activeTab !== 'timeline' && (
        <div
          id={`analytics-panel-${activeTab}`}
          role="tabpanel"
          aria-labelledby={`analytics-tab-${activeTab}`}
          className="analytics-tab-panel"
        >
          {activeTab === 'study' && <StudyTab study={study} />}
          {activeTab === 'performance' && <PerformanceTab performance={performance} />}
          {activeTab === 'topics' && <TopicsTab intelligence={topics} />}
        </div>
      )}

      {activeTab === 'timeline' && (
        <div
          id="analytics-panel-timeline"
          role="tabpanel"
          aria-labelledby="analytics-tab-timeline"
          className="analytics-tab-panel"
        >
          <TimelineTab
            key={`${timelineRange.fromDate}:${timelineRange.toDate}`}
            today={today}
            initialRange={timelineRange}
            timeline={timeline}
            loading={timelineLoading}
            error={timelineError}
            onApply={(next) => {
              setTimelineLoading(true);
              setTimelineRange(next);
              setTimelineReloadKey((key) => key + 1);
            }}
          />
        </div>
      )}
    </div>
  );
}
