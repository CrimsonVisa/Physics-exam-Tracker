import { useEffect, useState } from 'react';
import { DashboardCustomizer } from './DashboardCustomizer';
import { toCommandError } from '../lib/commandError';
import { calendarDaysBetween, countdownFor, formatDateLong, type Countdown } from '../lib/dateOnly';
import {
  getPreparationDashboard,
  type ActionableInsight,
  type DashboardLayout,
  type DashboardSectionKey,
  type PreparationDashboard as PreparationDashboardData,
} from '../lib/preparationApi';
import { formatStudyDuration } from '../lib/studyTime';
import { formatTimeRange } from '../lib/timeOfDay';
type PreparationDashboardProps = {
  today: string;
  previewExamDate: string;
  previewPreparationStartDate: string;
  previewTargetMinutes: number;
  refreshKey: string;
  onNavigate: (view: string) => void;
};
function CountdownBody({ countdown }: { countdown: Countdown | null }) {
  if (!countdown) {
    return (
      <div className="countdown-empty">
        <p className="countdown-empty-title">Choose a valid exam date</p>
        <p className="countdown-caption">The countdown updates as you edit the date.</p>
      </div>
    );
  }
  if (countdown.state === 'past') {
    return (
      <>
        <p className="countdown-past-title">Exam date has passed</p>
        <p className="countdown-past-days">
          {countdown.daysSinceExam} {countdown.daysSinceExam === 1 ? 'day' : 'days'} ago
        </p>
        <p className="countdown-caption">
          Update the exam date when you know the date for the next attempt.
        </p>
      </>
    );
  }
  const days = countdown.daysRemaining;
  return (
    <>
      <p className="countdown-value">{days}</p>
      <p className="countdown-label">{days === 1 ? 'day remaining' : 'days remaining'}</p>
      <p className="countdown-caption">
        {countdown.state === 'exam-day'
          ? 'Today is exam day. Focus on the paper.'
          : 'Calendar days on this device. Take it one focused block at a time.'}
      </p>
    </>
  );
}
function dateLabel(value: string | null): string {
  if (!value) {
    return 'Not set';
  }
  return formatDateLong(value) ?? value;
}
function statusLabel(value: string): string {
  if (value === 'due_today') {
    return 'Due today';
  }
  return value.replaceAll('_', ' ').replace(/\b\w/g, (character) => character.toUpperCase());
}
function InsightCard({
  insight,
  onNavigate,
}: {
  insight: ActionableInsight;
  onNavigate: (view: string) => void;
}) {
  return (
    <article className={`dashboard-insight dashboard-insight-${insight.severity}`}>
      <div className="dashboard-insight-heading">
        <span className="dashboard-insight-severity">{insight.severity}</span>
        <h3>{insight.title}</h3>
      </div>
      <p className="dashboard-insight-reason">{insight.reason}</p>
      <dl className="dashboard-insight-metrics">
        {insight.metrics.map((metric) => (
          <div key={`${insight.id}-${metric.label}`}>
            <dt>{metric.label}</dt> <dd>{metric.value}</dd>
          </div>
        ))}
      </dl>
      <button
        type="button"
        className="secondary-button"
        onClick={() => onNavigate(insight.targetView)}
      >
        {insight.targetLabel}
      </button>
    </article>
  );
}
const SECTION_TITLES: Record<DashboardSectionKey, string> = {
  'exam-countdown': 'Exam countdown',
  'syllabus-progress': 'Syllabus progress',
  'study-totals': 'Study-time totals',
  'today-study': "Today's study time",
  'revision-status': 'Revision status',
  'deadlines-blocks': 'Deadlines and planned blocks',
  'recent-pyq': 'Recent PYQ attempts',
  'latest-mock': 'Latest mock test',
  insights: 'Actionable insights',
};
export function PreparationDashboard({
  today,
  previewExamDate,
  previewPreparationStartDate,
  previewTargetMinutes,
  refreshKey,
  onNavigate,
}: PreparationDashboardProps) {
  const [data, setData] = useState<PreparationDashboardData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [isCustomizing, setIsCustomizing] = useState(false);
  useEffect(() => {
    let cancelled = false;
    getPreparationDashboard()
      .then((loaded) => {
        if (!cancelled) {
          setData(loaded);
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
  }, [reloadKey, refreshKey, today]);
  function retry() {
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }
  function applyLayout(layout: DashboardLayout) {
    setData((current) => (current ? { ...current, layout } : current));
  }
  const countdown = previewExamDate ? countdownFor(previewExamDate, today) : null;
  const elapsedDays = calendarDaysBetween(previewPreparationStartDate, today);
  const preparationDayNumber = elapsedDays !== null && elapsedDays >= 0 ? elapsedDays + 1 : 0;
  const targetSeconds = Math.max(1, previewTargetMinutes * 60);
  const targetProgress = data
    ? Math.min(100, Math.max(0, (data.todayStudySeconds / targetSeconds) * 100))
    : 0;
  const revisionTarget = data?.dailyRevisionTarget ?? 0;
  const revisionProgress = data
    ? Math.min(100, (data.revision.completedToday / Math.max(1, revisionTarget)) * 100)
    : 0;
  if (isLoading) {
    return (
      <section className="state-card" role="status" aria-live="polite">
        <p className="state-title">Loading preparation overview…</p>
        <p className="state-copy">
          Reading saved syllabus progress, study sessions, revisions, deadlines, PYQs, and mock
          tests.
        </p>
      </section>
    );
  }
  if (loadError || !data) {
    return (
      <section className="state-card state-card-error" role="alert">
        <p className="state-title">Preparation overview could not be loaded</p>
        <p className="state-copy">{loadError ?? 'The local database returned no overview data.'}</p>
        <button type="button" className="primary-button" onClick={retry}>
          Retry overview
        </button>
      </section>
    );
  }
  const dashboard = data;
  const ordered = [...dashboard.layout.sections].sort(
    (left, right) => left.displayOrder - right.displayOrder,
  );
  const visible = ordered.filter((section) => section.visible);
  function renderSection(key: DashboardSectionKey) {
    switch (key) {
      case 'exam-countdown':
        return (
          <section className="countdown-card prep-section" aria-labelledby="countdown-heading">
            <h2 className="sr-only" id="countdown-heading">
              Countdown details
            </h2>
            <div className="countdown-topline">
              <p className="eyebrow eyebrow-light">Target exam</p>
              <p className="exam-date-line">
                {previewExamDate ? (
                  <time dateTime={previewExamDate}>{dateLabel(previewExamDate)}</time>
                ) : (
                  'Choose a valid date'
                )}
              </p>
            </div>
            <div className="countdown-body" aria-live="polite">
              <CountdownBody countdown={countdown} />
            </div>
            <dl className="countdown-meta">
              <div>
                <dt>Preparation start</dt> <dd>{dateLabel(previewPreparationStartDate)}</dd>
              </div>
              <div>
                <dt>Preparation day</dt>
                <dd>{preparationDayNumber > 0 ? preparationDayNumber : 'Not started'}</dd>
              </div>
              <div>
                <dt>Elapsed days</dt>
                <dd>{elapsedDays !== null && elapsedDays >= 0 ? elapsedDays : 'Not started'}</dd>
              </div>
              <div>
                <dt>Daily target</dt> <dd>{formatStudyDuration(targetSeconds)}</dd>
              </div>
            </dl>
            <p className="provisional-note">
              <strong>Verify the exam date.</strong> 19 December is a provisional default, not an
              official schedule. Confirm the actual subject date on your admit card before relying
              on this countdown.
            </p>
          </section>
        );
      case 'syllabus-progress':
        return (
          <section className="prep-card" aria-labelledby="syllabus-overview-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Equal-weight approximation</p>
                <h2 id="syllabus-overview-heading">Syllabus progress</h2>
              </div>
              <span className="prep-card-value">
                {dashboard.syllabus.approximateProgress.toFixed(1)}%
              </span>
            </div>
            <div
              className="progress-track progress-track-large"
              role="progressbar"
              aria-label="Approximate syllabus progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(dashboard.syllabus.approximateProgress)}
            >
              <span
                style={{ width: `${Math.min(100, dashboard.syllabus.approximateProgress)}%` }}
              />
            </div>
            <dl className="prep-stat-grid">
              <div>
                <dt>Not started</dt> <dd>{dashboard.syllabus.statusCounts.notStarted}</dd>
              </div>
              <div>
                <dt>Learning</dt> <dd>{dashboard.syllabus.statusCounts.learning}</dd>
              </div>
              <div>
                <dt>Revised</dt> <dd>{dashboard.syllabus.statusCounts.revised}</dd>
              </div>
              <div>
                <dt>Confident</dt> <dd>{dashboard.syllabus.statusCounts.confident}</dd>
              </div>
            </dl>
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('syllabus')}
              >
                Open syllabus
              </button>
            </div>
          </section>
        );
      case 'study-totals':
        return (
          <section className="prep-card" aria-labelledby="study-totals-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Saved focus sessions only</p>
                <h2 id="study-totals-heading">Study-time totals</h2>
              </div>
            </div>
            <dl className="prep-stat-grid prep-stat-grid-large">
              <div>
                <dt>All recorded time</dt>
                <dd>{formatStudyDuration(dashboard.totalStudySeconds)}</dd>
              </div>
              <div>
                <dt>This week</dt> <dd>{formatStudyDuration(dashboard.weekStudySeconds)}</dd>
              </div>
              <div>
                <dt>This month</dt> <dd>{formatStudyDuration(dashboard.monthStudySeconds)}</dd>
              </div>
            </dl>
            <p className="prep-note">
              Weekly totals use Monday-based local calendar weeks. Planned blocks and breaks do not
              contribute.
            </p>
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('sessions')}
              >
                Open sessions
              </button>
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('analytics')}
              >
                Open analytics
              </button>
            </div>
          </section>
        );
      case 'today-study':
        return (
          <section className="prep-card" aria-labelledby="study-today-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Daily target</p>
                <h2 id="study-today-heading">Today&apos;s study time</h2>
              </div>
              <span className="prep-card-value">
                {formatStudyDuration(dashboard.todayStudySeconds)}
              </span>
            </div>
            <p className="prep-target-copy">of {formatStudyDuration(targetSeconds)} daily target</p>
            <div
              className="progress-track study-today-progress"
              role="progressbar"
              aria-label="Today's focus time toward the daily target"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(targetProgress)}
            >
              <span style={{ width: `${targetProgress}%` }} />
            </div>
            <dl className="prep-stat-grid">
              <div>
                <dt>Today&apos;s sessions</dt> <dd>{dashboard.todaySessionCount}</dd>
              </div>
              <div>
                <dt>Target progress</dt>
                <dd> {((dashboard.todayStudySeconds / targetSeconds) * 100).toFixed(1)}% </dd>
              </div>
            </dl>
            <p className="prep-note">
              Only completed focus sessions appear in study totals. A running or paused timer is in
              progress and is not counted until the session is completed. Planned blocks and
              revision dates are not counted.
            </p>
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('sessions')}
              >
                View sessions
              </button>
            </div>
          </section>
        );
      case 'revision-status':
        return (
          <section className="prep-card" aria-labelledby="revision-overview-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Saved schedule and outcomes</p>
                <h2 id="revision-overview-heading">Revision status</h2>
              </div>
              <span className="prep-card-value">
                {dashboard.revision.completedToday}/{revisionTarget}
              </span>
            </div>
            <dl className="prep-stat-grid">
              <div>
                <dt>Overdue</dt>
                <dd className={dashboard.revision.overdueCount > 0 ? 'prep-danger' : undefined}>
                  {dashboard.revision.overdueCount}
                </dd>
              </div>
              <div>
                <dt>Due today</dt> <dd>{dashboard.revision.dueTodayCount}</dd>
              </div>
              <div>
                <dt>Completed today</dt> <dd>{dashboard.revision.completedToday}</dd>
              </div>
              <div>
                <dt>Active schedules</dt> <dd>{dashboard.revision.activeCount}</dd>
              </div>
            </dl>
            <div
              className="progress-track"
              role="progressbar"
              aria-label="Today's revision target progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(revisionProgress)}
            >
              <span style={{ width: `${revisionProgress}%` }} />
            </div>
            <p className="prep-note">
              Revision completion changes only the revision schedule. It never marks a syllabus
              topic completed or adds study time automatically.
            </p>
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('revision')}
              >
                Open revision queue
              </button>
            </div>
          </section>
        );
      case 'deadlines-blocks':
        return (
          <section className="prep-card" aria-labelledby="plan-overview-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Open commitments</p>
                <h2 id="plan-overview-heading"> Plan for {formatDateLong(today) ?? today} </h2>
              </div>
              <span className="prep-card-value">{dashboard.overdueTaskCount} overdue</span>
            </div>
            <div className="prep-plan-grid">
              <div>
                <h3>Upcoming deadlines</h3>
                {dashboard.upcomingDeadlines.length === 0 ? (
                  <p className="prep-empty">No open deadlines in the next 7 days.</p>
                ) : (
                  <ul className="prep-compact-list">
                    {dashboard.upcomingDeadlines.map((task) => (
                      <li key={`dashboard-deadline-${task.id}`}>
                        <div>
                          <p>{task.title}</p>
                          <small>
                            {dateLabel(task.deadlineDate)} · {task.priority}
                            {task.topic ? ` · ${task.topic.code}` : ''}
                          </small>
                        </div>
                        <span className={`prep-status prep-status-${task.status}`}>
                          {statusLabel(task.status)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div>
                <h3>Upcoming planned blocks</h3>
                {dashboard.upcomingBlocks.length === 0 ? (
                  <p className="prep-empty">No planned blocks in the next 7 days.</p>
                ) : (
                  <ul className="prep-compact-list">
                    {dashboard.upcomingBlocks.map((block) => (
                      <li key={`dashboard-block-${block.id}`}>
                        <div>
                          <p>{block.title}</p>
                          <small>
                            {dateLabel(block.calendarDate)} ·
                            {formatTimeRange(block.startTime, block.endTime)}
                          </small>
                        </div>
                        <span className={`prep-status prep-status-${block.status}`}>
                          {statusLabel(block.status)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
            <p className="prep-note">
              Deadlines and planned blocks are intentions or reminders, not completed study. Only
              completed focus sessions appear in study totals.
            </p>
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('checklist')}
              >
                Open checklist
              </button>
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('calendar')}
              >
                Open calendar
              </button>
            </div>
          </section>
        );
      case 'recent-pyq':
        return (
          <section className="prep-card" aria-labelledby="recent-pyq-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Latest saved attempts</p>
                <h2 id="recent-pyq-heading">Recent PYQ attempts</h2>
              </div>
            </div>
            {dashboard.recentPyq.length === 0 ? (
              <p className="prep-empty">No PYQ attempts recorded yet.</p>
            ) : (
              <ul className="prep-activity-list">
                {dashboard.recentPyq.map((attempt) => (
                  <li key={`dashboard-pyq-${attempt.id}`}>
                    <div>
                      <p>
                        {attempt.topic.code} · {attempt.topic.title}
                      </p>
                      <small>
                        {dateLabel(attempt.attemptDate)} · Part {attempt.part} ·
                        {attempt.questionReference}
                      </small>
                    </div>
                    <span className={`prep-status prep-status-${attempt.status}`}>
                      {statusLabel(attempt.status)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
            <div className="prep-section-actions">
              <button type="button" className="secondary-button" onClick={() => onNavigate('pyq')}>
                Open PYQ bank
              </button>
            </div>
          </section>
        );
      case 'latest-mock':
        return (
          <section className="prep-card" aria-labelledby="latest-mock-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Calculated from saved marking</p>
                <h2 id="latest-mock-heading">Latest mock test</h2>
              </div>
            </div>
            {dashboard.latestMock ? (
              <>
                <p className="prep-mock-title">{dashboard.latestMock.testName}</p>
                <p className="prep-target-copy">{dateLabel(dashboard.latestMock.testDate)}</p>
                <dl className="prep-stat-grid prep-stat-grid-large">
                  <div>
                    <dt>Calculated score</dt>
                    <dd>{dashboard.latestMock.computedPercentage.toFixed(1)}%</dd>
                  </div>
                  <div>
                    <dt>Calculated marks</dt>
                    <dd>
                      {dashboard.latestMock.computedMarks.toFixed(2)} /
                      {dashboard.latestMock.maxMarks.toFixed(2)}
                    </dd>
                  </div>
                  <div>
                    <dt>Reported marks</dt>
                    <dd>
                      {dashboard.latestMock.marksObtained === null
                        ? 'Not recorded'
                        : dashboard.latestMock.marksObtained.toFixed(2)}
                    </dd>
                  </div>
                </dl>
                <p className="prep-note">
                  The calculated score uses this test&apos;s saved marking configuration. PYQ and
                  mock-test results remain separate; no combined performance score is produced.
                </p>
              </>
            ) : (
              <p className="prep-empty">No mock tests recorded yet.</p>
            )}
            <div className="prep-section-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => onNavigate('mock-tests')}
              >
                Open mock tests
              </button>
            </div>
          </section>
        );
      case 'insights':
        return (
          <section className="prep-card" aria-labelledby="dashboard-insights-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Evidence-linked suggestions</p>
                <h2 id="dashboard-insights-heading">Actionable insights</h2>
              </div>
            </div>
            {dashboard.insights.length === 0 ? (
              <p className="prep-empty">
                No actionable insight currently meets the evidence thresholds.
              </p>
            ) : (
              <div className="dashboard-insight-list">
                {dashboard.insights.map((insight) => (
                  <InsightCard key={insight.id} insight={insight} onNavigate={onNavigate} />
                ))}
              </div>
            )}
            <p className="prep-note">
              Insights describe saved data and thresholds. They do not predict rank, selection,
              readiness, or success, and comparisons do not establish cause.
            </p>
          </section>
        );
      default:
        return null;
    }
  }
  return (
    <>
      <section className="prep-toolbar" aria-label="Dashboard controls">
        <div>
          <p className="eyebrow">Preparation overview</p>
          <p className="prep-toolbar-copy">
            Every value below comes from saved records. Sections can be reordered, hidden, or
            restored.
          </p>
        </div>
        <button
          type="button"
          className="secondary-button"
          onClick={() => setIsCustomizing((current) => !current)}
          aria-expanded={isCustomizing}
        >
          {isCustomizing ? 'Close customizer' : 'Customize dashboard'}
        </button>
      </section>
      {isCustomizing && (
        <DashboardCustomizer
          sections={dashboard.layout.sections}
          onChanged={applyLayout}
          onClose={() => setIsCustomizing(false)}
        />
      )}
      {visible.length === 0 ? (
        <section className="state-card">
          <p className="state-title">All dashboard sections are hidden</p>
          <p className="state-copy">
            Open the customizer or restore the default layout to show saved preparation information.
          </p>
          <button type="button" className="primary-button" onClick={() => setIsCustomizing(true)}>
            Customize dashboard
          </button>
        </section>
      ) : (
        <div className="preparation-dashboard-grid">
          {visible.map((section) => (
            <div
              className={`dashboard-section dashboard-section-${section.key}`}
              key={section.key}
              aria-label={SECTION_TITLES[section.key]}
            >
              {renderSection(section.key)}
            </div>
          ))}
        </div>
      )}
    </>
  );
}
