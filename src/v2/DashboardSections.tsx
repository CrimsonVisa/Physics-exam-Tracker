import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { toCommandError } from '../lib/commandError';
import { emitTimerUpdated } from '../lib/timerEvents';
import { formatStudyDuration } from '../lib/studyTime';
import {
  deleteV2QuickStartConfig,
  getV2ExamSyllabus,
  getV2PracticeAnalytics,
  getV2Timer,
  listV2QuickStartConfigs,
  resumeV2Timer,
  saveV2QuickStartConfig,
  startV2Timer,
  type V2ExamSyllabus,
  type V2ExamSummary,
  type V2PlannerBlock,
  type V2PracticeAnalytics,
  type V2QuickStartConfig,
  type V2StudySession,
  type V2TimerSnapshot,
} from './api';
import {
  latestMock,
  recentTopics,
  summarizePlannerDay,
  summarizeTodaySessions,
  weakestTopics,
  type DailyDashboardData,
} from './dailyDashboardData';
import type { V2View } from './navigation';

type Navigate = (view: V2View) => void;

function shortExamCode(exams: V2ExamSummary[], examCode: string): string {
  return exams.find((exam) => exam.code === examCode)?.shortName ?? examCode;
}

function examExists(exams: V2ExamSummary[], examCode: string): boolean {
  return exams.some((exam) => exam.code === examCode);
}

function masteryLabel(score: number | null): string {
  return score === null ? 'No evidence' : score.toFixed(1);
}

function percentLabel(value: number | null): string {
  return value === null ? '—' : `${Math.round(value)}%`;
}

function blockMinutes(block: V2PlannerBlock): number {
  const [startHour = 0, startMinute = 0] = block.startTime.split(':').map(Number);
  const [endHour = 0, endMinute = 0] = block.endTime.split(':').map(Number);
  const start = startHour * 60 + startMinute;
  const end = endHour * 60 + endMinute;
  return Math.max(0, end - start);
}

const BLOCK_STATUS_LABELS: Record<string, string> = {
  planned: 'Planned',
  in_progress: 'In progress',
  completed: 'Completed',
  cancelled: 'Cancelled',
};

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

type SectionCardProps = {
  ariaLabel: string;
  eyebrow: string;
  title: string;
  className?: string;
  caption?: string;
  loading?: boolean;
  loadingCopy: string;
  error?: string | null;
  onRetry?: () => void;
  children: ReactNode;
};

function SectionCard({
  ariaLabel,
  eyebrow,
  title,
  className = 'v2-concept-card',
  caption,
  loading = false,
  loadingCopy,
  error = null,
  onRetry,
  children,
}: SectionCardProps) {
  return (
    <section aria-busy={loading || undefined} aria-label={ariaLabel} className={className}>
      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2>{title}</h2>
        </div>
        {caption && <p>{caption}</p>}
      </div>
      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}
      {error && onRetry && (
        <div className="v2-form-actions">
          <button className="secondary-button" onClick={onRetry} type="button">
            Retry
          </button>
        </div>
      )}
      {loading ? <p className="v2-empty-copy">{loadingCopy}</p> : children}
    </section>
  );
}

function plannerDetailParts(parts: Array<string | false | null | undefined>): string | null {
  const joined = parts.filter(Boolean).join(' · ');
  return joined.length > 0 ? joined : null;
}

export function TodayOverviewSection({
  data,
  onNavigate,
  today,
}: {
  data: DailyDashboardData;
  onNavigate: Navigate;
  today: string;
}) {
  const { seconds, sessionCount } = summarizeTodaySessions(data.sessions, today);
  const plannerCounts = summarizePlannerDay(data.plannerDay);
  const revisionStats = data.revision?.stats ?? null;
  const targetMinutes = data.settings?.dailyStudyTargetMinutes ?? 0;
  const targetConfigured = targetMinutes > 0;
  const completedMinutes = Math.round(seconds / 60);
  const targetPercent = targetConfigured
    ? Math.min(100, Math.round((seconds / (targetMinutes * 60)) * 100))
    : 0;
  const activeTaskCount = plannerCounts.tasksTotal - plannerCounts.tasksCancelled;

  return (
    <SectionCard
      ariaLabel="Today's overview"
      caption={`${today} · all examinations combined`}
      error={data.error}
      eyebrow="Today"
      loading={data.loading}
      loadingCopy="Loading today's study time, planner and revision activity…"
      onRetry={data.retry}
      title="Today's overview"
    >
      <div className="v2-coverage-grid">
        <article className="v2-coverage-card">
          <span className="v2-status-badge v2-status-active">Study time</span>
          <h2>{formatStudyDuration(seconds)}</h2>
          <p className="v2-empty-copy">
            {sessionCount === 0
              ? 'No study session is recorded for today yet.'
              : `${plural(sessionCount, 'session')} completed today.`}
          </p>
          {targetConfigured ? (
            <div className="v2-day-target">
              <div
                aria-label={`Daily target: ${completedMinutes} of ${targetMinutes} minutes completed`}
                className="v2-timer-progress"
                data-progress={targetPercent}
              >
                <span style={{ width: `${targetPercent}%` }} />
              </div>
              <p className="v2-empty-copy">
                Daily target: {completedMinutes} of {targetMinutes} min ({targetPercent}% complete).
              </p>
            </div>
          ) : (
            <>
              <p className="v2-empty-copy">
                No daily study target is configured. Targets are managed in Settings.
              </p>
              <div className="v2-form-actions">
                <button
                  className="secondary-button"
                  onClick={() => onNavigate({ kind: 'settings' })}
                  type="button"
                >
                  Open settings
                </button>
              </div>
            </>
          )}
        </article>
        <article className="v2-coverage-card">
          <span className="v2-status-badge v2-status-neutral">Planner</span>
          <h2>
            {plannerCounts.tasksTotal === 0
              ? 'No planner tasks'
              : `${plannerCounts.tasksDone} done · ${plannerCounts.tasksRemaining} left`}
          </h2>
          {plannerCounts.tasksTotal === 0 ? (
            <p className="v2-empty-copy">
              Nothing is scheduled on today's checklist. Tasks can be added in the planner.
            </p>
          ) : (
            <p className="v2-empty-copy">
              {plural(plannerCounts.tasksDone, 'task')} completed of {activeTaskCount} active
              {plannerCounts.tasksCancelled > 0
                ? ` · ${plural(plannerCounts.tasksCancelled, 'cancelled task')}`
                : ''}
              .
            </p>
          )}
          {plannerCounts.blocksTotal > 0 && (
            <p className="v2-empty-copy">
              Planned study blocks: {plannerCounts.blocksDone} done ·{' '}
              {plannerCounts.blocksRemaining} left
              {plannerCounts.blocksCancelled > 0
                ? ` · ${plannerCounts.blocksCancelled} cancelled`
                : ''}
              .
            </p>
          )}
        </article>
        <article className="v2-coverage-card">
          <span className="v2-status-badge v2-status-inactive">Revisions</span>
          <h2>
            {revisionStats
              ? `${revisionStats.dueToday} due · ${revisionStats.overdue} overdue`
              : '—'}
          </h2>
          <p className="v2-empty-copy">
            {revisionStats
              ? `${revisionStats.completedToday} completed today · ${revisionStats.completedLast7Days} in the last 7 days.`
              : 'Revision statistics are not available yet.'}
          </p>
        </article>
      </div>
    </SectionCard>
  );
}

export function TodayPlanSection({
  data,
  exams,
  onNavigate,
}: {
  data: DailyDashboardData;
  exams: V2ExamSummary[];
  onNavigate: Navigate;
}) {
  const tasks = data.plannerDay?.tasks ?? [];
  const blocks = data.plannerDay?.blocks ?? [];
  const plannerCounts = summarizePlannerDay(data.plannerDay);

  return (
    <SectionCard
      ariaLabel="Today's study plan"
      caption={`${plural(plannerCounts.tasksTotal, 'task')} · ${plural(
        plannerCounts.blocksTotal,
        'block',
      )} scheduled`}
      error={data.error}
      eyebrow="Plan"
      loading={data.loading}
      loadingCopy="Loading today's planner tasks and blocks…"
      onRetry={data.retry}
      title="Today's study plan"
    >
      {tasks.length + blocks.length === 0 ? (
        <p className="v2-empty-panel-copy">
          No planner tasks or blocks are scheduled for today. Planned topics can be managed in the
          study planner.
        </p>
      ) : (
        <div className="v2-day-plan-list">
          {tasks.length > 0 && (
            <div>
              <p className="eyebrow">Checklist tasks</p>
              <ul className="v2-planner-list">
                {tasks.map((task) => {
                  const detail = plannerDetailParts([
                    task.conceptTitle ? `Concept: ${task.conceptTitle}` : false,
                    task.deadlineDate ? `Deadline ${task.deadlineDate}` : false,
                    `Priority ${task.priority}`,
                  ]);
                  return (
                    <li
                      className={`v2-planner-row${task.status === 'completed' ? ' v2-planner-done' : ''}`}
                      data-task-id={task.id}
                      key={task.id}
                    >
                      <div>
                        <div className="v2-session-heading">
                          <strong>{task.title}</strong>
                          {task.examCode && (
                            <span className="v2-allocation-chip">
                              {shortExamCode(exams, task.examCode)}
                            </span>
                          )}
                          <span
                            className={`v2-status-badge ${
                              task.status === 'completed' ? 'v2-status-active' : 'v2-status-neutral'
                            }`}
                          >
                            {task.status === 'completed' ? 'Completed' : 'Open'}
                          </span>
                        </div>
                        {detail && <p className="v2-empty-copy">{detail}</p>}
                      </div>
                      {task.examCode && examExists(exams, task.examCode) && (
                        <div className="v2-form-actions">
                          <button
                            aria-label={`Open the ${shortExamCode(exams, task.examCode)} workspace for ${task.title}`}
                            className="secondary-button"
                            onClick={() =>
                              onNavigate({ kind: 'exam', examCode: task.examCode as string })
                            }
                            type="button"
                          >
                            Workspace
                          </button>
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
          {blocks.length > 0 && (
            <div>
              <p className="eyebrow">Planned study blocks</p>
              <ul className="v2-planner-list">
                {blocks.map((block) => {
                  const detail = plannerDetailParts([
                    `${block.startTime}–${block.endTime}`,
                    blockMinutes(block) > 0 ? `${blockMinutes(block)} min` : false,
                    block.conceptTitle ? block.conceptTitle : false,
                  ]);
                  return (
                    <li
                      className={`v2-planner-row${block.status === 'completed' ? ' v2-planner-done' : ''}`}
                      data-block-id={block.id}
                      key={block.id}
                    >
                      <div>
                        <div className="v2-session-heading">
                          <strong>{block.title}</strong>
                          {block.examCode && (
                            <span className="v2-allocation-chip">
                              {shortExamCode(exams, block.examCode)}
                            </span>
                          )}
                          <span className="v2-status-badge v2-status-inactive">
                            {BLOCK_STATUS_LABELS[block.status] ?? block.status}
                          </span>
                        </div>
                        {detail && <p className="v2-empty-copy">{detail}</p>}
                      </div>
                      {block.examCode && examExists(exams, block.examCode) && (
                        <div className="v2-form-actions">
                          <button
                            aria-label={`Open the ${shortExamCode(exams, block.examCode)} workspace for ${block.title}`}
                            className="secondary-button"
                            onClick={() =>
                              onNavigate({ kind: 'exam', examCode: block.examCode as string })
                            }
                            type="button"
                          >
                            Workspace
                          </button>
                        </div>
                      )}
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
        </div>
      )}
      <div className="v2-form-actions">
        <button
          className="secondary-button"
          onClick={() => onNavigate({ kind: 'planner' })}
          type="button"
        >
          Manage planner tasks
        </button>
      </div>
    </SectionCard>
  );
}

export function RevisionOverviewSection({
  data,
  onNavigate,
}: {
  data: DailyDashboardData;
  onNavigate: Navigate;
}) {
  const dueToday = data.revision?.dueToday ?? [];
  const overdue = data.revision?.overdue ?? [];
  const recentHistory = data.revision?.recentHistory ?? [];
  const dueRows = dueToday.slice(0, 3);
  const overdueRows = overdue.slice(0, 3);
  const recentRows = recentHistory.slice(0, 3);
  const dueItemCount = dueRows.length + overdueRows.length;

  return (
    <SectionCard
      ariaLabel="Revision overview"
      caption={`${plural(dueToday.length, 'due today')} · ${plural(
        overdue.length,
        'overdue',
      )} · ${plural(recentHistory.length, 'recent record')}`}
      error={data.error}
      eyebrow="Revisions"
      loading={data.loading}
      loadingCopy="Loading the revision schedule…"
      onRetry={data.retry}
      title="Revision overview"
    >
      <div className="v2-day-plan-list">
        {dueItemCount === 0 ? (
          <p className="v2-empty-panel-copy">
            {data.revision
              ? 'No revisions are due today and none are overdue.'
              : 'Revision data is not available.'}
          </p>
        ) : (
          <>
            {overdueRows.length > 0 && (
              <div>
                <p className="eyebrow">Overdue</p>
                <ul className="v2-planner-list">
                  {overdueRows.map((schedule) => (
                    <li
                      className="v2-planner-row"
                      data-concept-id={schedule.conceptId}
                      key={schedule.conceptId}
                    >
                      <div>
                        <div className="v2-session-heading">
                          <strong>{schedule.conceptTitle}</strong>
                          <span className="v2-status-badge v2-status-inactive">
                            {schedule.overdueDays} day{schedule.overdueDays === 1 ? '' : 's'}{' '}
                            overdue
                          </span>
                        </div>
                        <p className="v2-empty-copy">
                          {schedule.conceptDomain} · due {schedule.nextRevisionDate} · mastery{' '}
                          {masteryLabel(schedule.masteryScore)}
                        </p>
                      </div>
                    </li>
                  ))}
                </ul>
                {overdue.length > overdueRows.length && (
                  <p className="v2-empty-copy">
                    {plural(overdue.length - overdueRows.length, 'further overdue revision')} in
                    revision planning.
                  </p>
                )}
              </div>
            )}
            {dueRows.length > 0 && (
              <div>
                <p className="eyebrow">Due today</p>
                <ul className="v2-planner-list">
                  {dueRows.map((schedule) => (
                    <li
                      className="v2-planner-row"
                      data-concept-id={schedule.conceptId}
                      key={schedule.conceptId}
                    >
                      <div>
                        <div className="v2-session-heading">
                          <strong>{schedule.conceptTitle}</strong>
                          <span className="v2-status-badge v2-status-active">Due today</span>
                        </div>
                        <p className="v2-empty-copy">
                          {schedule.conceptDomain} · interval {schedule.currentIntervalDays} days ·
                          mastery {masteryLabel(schedule.masteryScore)}
                        </p>
                      </div>
                    </li>
                  ))}
                </ul>
                {dueToday.length > dueRows.length && (
                  <p className="v2-empty-copy">
                    {plural(dueToday.length - dueRows.length, 'further due revision')} in revision
                    planning.
                  </p>
                )}
              </div>
            )}
          </>
        )}
        <div>
          <p className="eyebrow">Recently completed</p>
          {recentRows.length === 0 ? (
            <p className="v2-empty-panel-copy">No revision history has been recorded yet.</p>
          ) : (
            <ul className="v2-planner-list">
              {recentRows.map((entry) => (
                <li className="v2-planner-row" data-history-id={entry.id} key={entry.id}>
                  <div>
                    <div className="v2-session-heading">
                      <strong>{entry.conceptTitle}</strong>
                      <span className="v2-status-badge v2-status-neutral">{entry.outcome}</span>
                    </div>
                    <p className="v2-empty-copy">
                      {entry.revisionDate} · mastery {entry.masteryBefore.toFixed(1)} →{' '}
                      {entry.masteryAfter.toFixed(1)}
                    </p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <div className="v2-form-actions">
        <button
          className="primary-button"
          onClick={() => onNavigate({ kind: 'revision' })}
          type="button"
        >
          Open revision planning
        </button>
      </div>
    </SectionCard>
  );
}

export function RecentlyStudiedSection({
  data,
  exams,
  onNavigate,
}: {
  data: DailyDashboardData;
  exams: V2ExamSummary[];
  onNavigate: Navigate;
}) {
  const topics = recentTopics(data.totals);

  return (
    <SectionCard
      ariaLabel="Recently studied topics"
      caption={
        topics.length > 0
          ? `${plural(topics.length, 'topic')} with recent study time`
          : 'Topic study time'
      }
      error={data.error}
      eyebrow="History"
      loading={data.loading}
      loadingCopy="Loading recently studied topics…"
      onRetry={data.retry}
      title="Recently studied"
    >
      {topics.length === 0 ? (
        <p className="v2-empty-panel-copy">
          No topic-allocated study time is recorded yet. Allocate timer sessions to syllabus topics
          to build this list.
        </p>
      ) : (
        <>
          <ul className="v2-practice-history">
            {topics.map((topic) => (
              <li key={`${topic.examId}-${topic.nodeId}`}>
                <button
                  className="v2-day-topic-row"
                  onClick={() => onNavigate({ kind: 'syllabus', examCode: topic.examCode })}
                  type="button"
                >
                  <strong>
                    {shortExamCode(exams, topic.examCode)} · {topic.subjectTitle} ·{' '}
                    {topic.nodeTitle}
                  </strong>
                  <span>
                    {formatStudyDuration(topic.allocatedSeconds)} · last studied{' '}
                    {topic.lastStudiedDate}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <p className="v2-empty-copy">
            Each topic opens the syllabus entry inside its examination workspace.
          </p>
        </>
      )}
    </SectionCard>
  );
}

const PERFORM_LIMIT_WEAK = 3;

function performanceEntryHasData(analytics: V2PracticeAnalytics | null): boolean {
  return latestMock(analytics) !== null || weakestTopics(analytics, PERFORM_LIMIT_WEAK).length > 0;
}

export function PerformanceSection({
  exams,
  onNavigate,
}: {
  exams: V2ExamSummary[];
  onNavigate: Navigate;
}) {
  const activeExams = useMemo(() => exams.filter((exam) => exam.isActive), [exams]);
  const [attempt, setAttempt] = useState(0);
  const [entries, setEntries] = useState<Array<{
    exam: V2ExamSummary;
    analytics: V2PracticeAnalytics | null;
    error: string | null;
  }> | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const loaded = await Promise.all(
        activeExams.map(async (exam) => {
          try {
            const analytics = await getV2PracticeAnalytics({ examId: exam.id });
            return { exam, analytics, error: null as string | null };
          } catch (loadError) {
            return { exam, analytics: null, error: toCommandError(loadError).message };
          }
        }),
      );
      if (!cancelled) {
        setEntries(loaded);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [activeExams, attempt]);

  const anyData = entries?.some((entry) => performanceEntryHasData(entry.analytics)) ?? false;

  return (
    <SectionCard
      ariaLabel="Recent performance and weak areas"
      caption="Latest completed mock and lowest-accuracy topics per active examination"
      eyebrow="Performance"
      loading={entries === null && activeExams.length > 0}
      loadingCopy="Loading practice analytics…"
      title="Performance and weak areas"
    >
      {activeExams.length === 0 ? (
        <p className="v2-empty-panel-copy">
          No active examinations. Performance focuses on active examination workspaces.
        </p>
      ) : entries === null ? null : anyData ? (
        <div className="v2-day-plan-list">
          {entries
            .filter(
              (entry) =>
                !entry.error && entry.analytics && performanceEntryHasData(entry.analytics),
            )
            .map((entry) => {
              const mock = latestMock(entry.analytics);
              const weak = weakestTopics(entry.analytics, PERFORM_LIMIT_WEAK);
              return (
                <div key={entry.exam.code}>
                  <p className="eyebrow">
                    {entry.exam.shortName} · {entry.exam.name}
                  </p>
                  <ul className="v2-practice-history">
                    {mock ? (
                      <li>
                        <strong>
                          Last mock — {mock.testDate} · {mock.testName}
                        </strong>
                        <span>
                          {percentLabel(mock.percentage)} ·{' '}
                          {mock.patternVersionLabel ?? 'Unspecified pattern'}
                        </span>
                      </li>
                    ) : (
                      <li>
                        <strong>No completed mock yet.</strong>
                        <span>Aggregate or question-level mocks appear here once recorded.</span>
                      </li>
                    )}
                    {weak.map((row) => (
                      <li key={`${row.nodeId ?? 'none'}-${row.label}`}>
                        <strong>{row.label}</strong>
                        <span>
                          {plural(row.attempts, 'attempt')} · accuracy{' '}
                          {percentLabel(row.accuracyPercentage)}
                        </span>
                      </li>
                    ))}
                  </ul>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      onClick={() => onNavigate({ kind: 'practice', examCode: entry.exam.code })}
                      type="button"
                    >
                      Review in practice systems
                    </button>
                  </div>
                </div>
              );
            })}
        </div>
      ) : (
        <p className="v2-empty-panel-copy">
          No mock or topic accuracy data is recorded yet for the active examinations. PYQ attempts
          and completed mocks build this overview.
        </p>
      )}
      {activeExams.length > 0 && entries !== null && (
        <>
          {entries
            .filter((entry) => entry.error)
            .map((entry) => (
              <p className="v2-study-error" key={entry.exam.code} role="alert">
                Performance data for {shortExamCode(exams, entry.exam.code)} could not be loaded:{' '}
                {entry.error}
              </p>
            ))}
          <div className="v2-form-actions">
            <button
              className="secondary-button"
              onClick={() => setAttempt((value) => value + 1)}
              type="button"
            >
              Refresh performance data
            </button>
          </div>
        </>
      )}
    </SectionCard>
  );
}

type QuickStartDraftRow = {
  examId: string;
  nodeId: string;
  minutes: string;
};

type QuickStartDraft = {
  id: number | null;
  name: string;
  durationMinutes: number;
  allocations: QuickStartDraftRow[];
};

type QuickStartPlan = {
  rows: Array<{ examId: number; nodeId: number; minutes: number }>;
  plannedMinutes: number;
};

const QUICK_START_PRESETS = [25, 45, 60, 90] as const;

function emptyQuickStartDraft(focusMinutes: number): QuickStartDraft {
  return {
    id: null,
    name: '',
    durationMinutes: focusMinutes,
    allocations: [],
  };
}

function validateQuickStartPlan(
  rows: Array<{ examId: number; nodeId: number; minutes: number }>,
  durationMinutes: number,
): string | null {
  if (!Number.isFinite(durationMinutes) || durationMinutes < 1 || durationMinutes > 240) {
    return 'The session duration must be between 1 and 240 minutes.';
  }
  const seen = new Set<string>();
  let total = 0;
  for (const row of rows) {
    if (row.examId <= 0 || row.nodeId <= 0) {
      return 'Choose an examination and topic for every allocation.';
    }
    if (!Number.isFinite(row.minutes) || row.minutes <= 0) {
      return 'Allocated time must be greater than zero minutes.';
    }
    if (seen.has(`${row.examId}:${row.nodeId}`)) {
      return 'Each examination topic can appear only once in a configuration.';
    }
    seen.add(`${row.examId}:${row.nodeId}`);
    total += row.minutes;
  }
  if (rows.length > 0 && total > durationMinutes) {
    return `Allocations total ${total} minutes, which exceeds the ${durationMinutes}-minute session duration.`;
  }
  return null;
}

export function QuickStartSection({
  exams,
  sessions,
  today,
}: {
  exams: V2ExamSummary[];
  sessions: V2StudySession[];
  today: string;
}) {
  const [timer, setTimer] = useState<V2TimerSnapshot | null>(null);
  const [timerError, setTimerError] = useState<string | null>(null);
  const [configs, setConfigs] = useState<V2QuickStartConfig[] | null>(null);
  const [configsError, setConfigsError] = useState<string | null>(null);
  const [syllabi, setSyllabi] = useState<Record<number, V2ExamSyllabus>>({});
  const [draft, setDraft] = useState<QuickStartDraft>(() => emptyQuickStartDraft(25));
  const [busy, setBusy] = useState<'start' | 'configs' | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [scrollNotice, setScrollNotice] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const snapshot = await getV2Timer();
        if (!cancelled) {
          setTimer(snapshot);
        }
      } catch (loadError) {
        if (!cancelled) {
          setTimerError(toCommandError(loadError).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const loadConfigs = useCallback(async () => {
    setBusy((current) => (current === null ? 'configs' : current));
    setConfigsError(null);
    try {
      const loaded = await listV2QuickStartConfigs();
      setConfigs(loaded);
    } catch (loadError) {
      setConfigsError(toCommandError(loadError).message);
    } finally {
      setBusy((current) => (current === 'configs' ? null : current));
    }
  }, []);

  useEffect(() => {
    void (async () => {
      await loadConfigs();
    })();
  }, [loadConfigs]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const loaded: Record<number, V2ExamSyllabus> = {};
      await Promise.all(
        exams
          .filter((exam) => exam.isActive)
          .map(async (exam) => {
            try {
              const value = await getV2ExamSyllabus(exam.id);
              if (value?.configured) {
                loaded[exam.id] = value;
              }
            } catch {
              // A syllabus can be unavailable without blocking quick starts.
            }
          }),
      );
      if (!cancelled) {
        setSyllabi(loaded);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [exams]);

  const activeExams = useMemo(() => exams.filter((exam) => exam.isActive), [exams]);

  const timerStatus = timer?.state.status ?? null;

  // Most recently used configuration, derived from recorded sessions only. It
  // is recomputed from history every render; nothing is stored twice.
  const lastUsed = useMemo(() => {
    const ordered = [...sessions].sort((left, right) => right.id - left.id);
    const session = ordered[0] ?? null;
    if (!session) {
      return null;
    }
    const seen = new Set<string>();
    const rows: Array<{ examId: number; nodeId: number; minutes: number; label: string }> = [];
    for (const allocation of session.topicAllocations) {
      if (seen.has(`${allocation.examId}:${allocation.nodeId}`)) {
        continue;
      }
      seen.add(`${allocation.examId}:${allocation.nodeId}`);
      const minutes = Math.max(1, Math.round(allocation.allocatedSeconds / 60));
      rows.push({
        examId: allocation.examId,
        nodeId: allocation.nodeId,
        minutes,
        label: `${allocation.examCode} · ${allocation.nodeTitle} · ${minutes} min`,
      });
    }
    const plannedMinutes =
      rows.length > 0
        ? Math.min(
            240,
            Math.max(
              1,
              rows.reduce((total, row) => total + row.minutes, 0),
            ),
          )
        : Math.min(240, Math.max(1, Math.round(session.durationSeconds / 60)));
    return {
      sessionDate: session.sessionDate,
      plan: {
        rows: rows.map((row) => ({ examId: row.examId, nodeId: row.nodeId, minutes: row.minutes })),
        plannedMinutes,
      },
      rowsLabel: rows.map((row) => row.label).join(' + '),
      combined: rows.length === 0,
      invalidReason: validateQuickStartPlan(
        rows.map((row) => ({ examId: row.examId, nodeId: row.nodeId, minutes: row.minutes })),
        plannedMinutes,
      ),
    } as const;
  }, [sessions]);

  // Distinct recent dedicated configurations for one-click loading.
  const recentConfigs = useMemo(() => {
    const seen = new Set<string>();
    const items: Array<{
      id: string;
      sessionDate: string;
      plannedMinutes: number;
      parts: string[];
      rows: Array<{ examId: number; nodeId: number; minutes: number }>;
    }> = [];
    for (const session of [...sessions].sort((left, right) => right.id - left.id)) {
      if (items.length >= 3) {
        break;
      }
      if (session.topicAllocations.length === 0) {
        continue;
      }
      const seenInSession = new Set<string>();
      const rows: Array<{ examId: number; nodeId: number; minutes: number; label: string }> = [];
      for (const allocation of session.topicAllocations) {
        if (seenInSession.has(`${allocation.examId}:${allocation.nodeId}`)) {
          continue;
        }
        seenInSession.add(`${allocation.examId}:${allocation.nodeId}`);
        const minutes = Math.max(1, Math.round(allocation.allocatedSeconds / 60));
        rows.push({
          examId: allocation.examId,
          nodeId: allocation.nodeId,
          minutes,
          label: `${allocation.examCode} · ${allocation.nodeTitle} · ${minutes} min`,
        });
      }
      const key = rows
        .map((row) => `${row.examId}:${row.nodeId}:${row.minutes}`)
        .sort()
        .join('|');
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      items.push({
        id: key,
        sessionDate: session.sessionDate,
        plannedMinutes: Math.min(
          240,
          Math.max(
            1,
            rows.reduce((total, row) => total + row.minutes, 0),
          ),
        ),
        parts: rows.map((row) => row.label),
        rows: rows.map((row) => ({ examId: row.examId, nodeId: row.nodeId, minutes: row.minutes })),
      });
    }
    return items;
  }, [sessions]);

  async function startPlan(plan: QuickStartPlan) {
    const validationError = validateQuickStartPlan(plan.rows, plan.plannedMinutes);
    if (validationError) {
      setActionError(validationError);
      return;
    }
    if (timerStatus !== 'idle' || busy !== null) {
      return;
    }
    setBusy('start');
    setActionError(null);
    try {
      const snapshot = await startV2Timer({
        mode: 'focus',
        sessionDate: today,
        activeExamId: null,
        allocations: plan.rows.map((row) => ({
          examId: row.examId,
          nodeId: row.nodeId,
          allocatedSeconds: Math.round(row.minutes * 60),
        })),
        durationMinutes: plan.plannedMinutes,
      });
      setTimer(snapshot);
      setScrollNotice(null);
      emitTimerUpdated();
    } catch (startError) {
      setActionError(toCommandError(startError).message);
      try {
        setTimer(await getV2Timer());
      } catch {
        // Keep the previous snapshot when the follow-up read also fails.
      }
    } finally {
      setBusy(null);
    }
  }

  async function resumePausedTimer() {
    if (busy !== null || timerStatus !== 'paused') {
      return;
    }
    setBusy('start');
    setActionError(null);
    try {
      const snapshot = await resumeV2Timer();
      setTimer(snapshot);
      emitTimerUpdated();
    } catch (resumeError) {
      setActionError(toCommandError(resumeError).message);
    } finally {
      setBusy(null);
    }
  }

  function updateDraft(patch: Partial<QuickStartDraft>) {
    setDraft((current) => ({ ...current, ...patch }));
  }

  function addDraftRow() {
    const firstExam = activeExams[0];
    setDraft((current) => ({
      ...current,
      allocations: [
        ...current.allocations,
        {
          examId: firstExam ? String(firstExam.id) : '',
          nodeId: '',
          minutes: '25',
        },
      ],
    }));
    setActionError(null);
  }

  function updateDraftRow(key: number, patch: Partial<QuickStartDraftRow>) {
    setDraft((current) => ({
      ...current,
      allocations: current.allocations.map((row, index) =>
        index === key ? { ...row, ...patch } : row,
      ),
    }));
    setActionError(null);
  }

  function removeDraftRow(key: number) {
    setDraft((current) => ({
      ...current,
      allocations: current.allocations.filter((_, index) => index !== key),
    }));
    setActionError(null);
  }

  function draftPlan(): QuickStartPlan {
    return {
      rows: draft.allocations.map((row) => ({
        examId: Number(row.examId),
        nodeId: Number(row.nodeId),
        minutes: Number(row.minutes),
      })),
      plannedMinutes: draft.durationMinutes,
    };
  }

  function cancelEditing() {
    setDeletingId(null);
    setDraft(emptyQuickStartDraft(timer?.settings.focusMinutes ?? 25));
  }

  function loadIntoDraft(config: {
    id?: number | null;
    name: string;
    durationMinutes: number;
    rows: Array<{ examId: number; nodeId: number; minutes: number }>;
  }) {
    setDeletingId(null);
    setDraft({
      id: config.id ?? null,
      name: config.name,
      durationMinutes: config.durationMinutes,
      allocations: config.rows
        .filter((row) => exams.some((exam) => exam.isActive && exam.id === row.examId))
        .map((row) => ({
          examId: String(row.examId),
          nodeId: String(row.nodeId),
          minutes: String(row.minutes),
        })),
    });
    setActionError(null);
  }

  function openStudyTimer() {
    const heading = document.getElementById('study-timer-heading');
    if (!heading) {
      setScrollNotice(
        'The study timer section is hidden. Enabling it again under Customize dashboard at the bottom of this page restores the entry point.',
      );
      return;
    }
    try {
      heading.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } catch {
      // Environments without smooth-scroll support simply keep the current
      // scroll position; the timer section is already on this page.
    }
    heading.focus({ preventScroll: true });
    setScrollNotice(null);
  }

  async function saveDraftConfiguration() {
    const name = draft.name.trim();
    if (name.length === 0) {
      setActionError('Give the configuration a name before saving.');
      return;
    }
    const plan = draftPlan();
    const validationError = validateQuickStartPlan(plan.rows, plan.plannedMinutes);
    if (validationError) {
      setActionError(validationError);
      return;
    }
    setBusy('configs');
    setActionError(null);
    try {
      const saved = await saveV2QuickStartConfig({
        id: draft.id,
        name,
        durationMinutes: plan.plannedMinutes,
        isPinned: null,
        allocations: plan.rows.map((row) => ({
          examId: row.examId,
          syllabusNodeId: row.nodeId,
          allocatedMinutes: row.minutes,
        })),
      });
      setConfigs(saved);
      const savedId = draft.id ?? saved.find((config) => config.name === name)?.id ?? null;
      setDraft({ ...draft, id: savedId, name });
    } catch (saveError) {
      setActionError(toCommandError(saveError).message);
    } finally {
      setBusy(null);
    }
  }

  async function deleteConfig(id: number) {
    if (deletingId !== id) {
      setDeletingId(id);
      return;
    }
    setBusy('configs');
    setActionError(null);
    try {
      const remaining = await deleteV2QuickStartConfig({ id });
      setConfigs(remaining);
      setDeletingId(null);
      if (draft.id === id) {
        setDraft(emptyQuickStartDraft(timer?.settings.focusMinutes ?? 25));
      }
    } catch (deleteError) {
      setActionError(toCommandError(deleteError).message);
    } finally {
      setBusy(null);
    }
  }

  const timerState = timer?.state ?? null;
  const statusBadgeClass =
    timerStatus === 'running'
      ? 'v2-status-active'
      : timerStatus === 'paused'
        ? 'v2-status-inactive'
        : 'v2-status-neutral';
  const statusLabel =
    timerStatus === 'running'
      ? 'Timer running'
      : timerStatus === 'paused'
        ? 'Timer paused'
        : 'Timer idle';
  const draftPlanCopy = draftPlan();
  const draftError = validateQuickStartPlan(draftPlanCopy.rows, draftPlanCopy.plannedMinutes);
  const draftTotalMinutes = draftPlanCopy.rows.reduce(
    (total, row) => total + (Number.isFinite(row.minutes) ? Math.max(0, row.minutes) : 0),
    0,
  );

  return (
    <SectionCard
      ariaLabel="Quick start timer"
      caption="Reuses the persisted study timer"
      className="v2-concept-card v2-quickstart"
      eyebrow="Quick start"
      loadingCopy="Loading the persisted study timer…"
      title="Start today's timer"
    >
      {timerError && (
        <p className="v2-study-error" role="alert">
          {timerError}
        </p>
      )}
      {actionError && (
        <p className="v2-study-error" role="alert">
          {actionError}
        </p>
      )}
      {!timer && !timerError && <p className="v2-empty-copy">Loading the persisted study timer…</p>}
      {timerState && (
        <>
          <p className="v2-empty-copy">
            <span className={`v2-status-badge ${statusBadgeClass}`}>{statusLabel}</span>{' '}
            {timerStatus === 'running'
              ? 'The running timer keeps counting where it started. Pause and finish live in the study timer section.'
              : timerStatus === 'paused'
                ? 'The paused timer was not replaced. Resume it here or continue in the study timer section.'
                : 'Quick starts run through the persisted study timer and its backend validation.'}
          </p>
          {timerState.recoveryReason ? (
            <p className="v2-empty-copy">
              {timerState.recoveryReason === 'clockChange'
                ? 'A clock change was detected. Unverified time was not counted.'
                : `Recovery excluded ${formatStudyDuration(
                    Math.round(timerState.recoveryExcludedMs / 1_000),
                  )} of unverified time after ${
                    timerState.recoveryReason === 'restart' ? 'a restart' : 'missing checkpoints'
                  }.`}
            </p>
          ) : null}
        </>
      )}

      {timer?.state.status === 'running' && (
        <div className="v2-day-plan-list">
          <p className="v2-empty-copy">
            {formatStudyDuration(Math.round(timer.state.durationMs / 1_000))} focus timer ·{' '}
            {timer.allocations.length === 0
              ? 'Combined study timer, no topic allocations'
              : `${timer.allocations.length} topic allocation${
                  timer.allocations.length === 1 ? '' : 's'
                }`}
          </p>
          {timer.allocations.length > 0 && (
            <div className="v2-allocation-chips">
              {timer.allocations.map((allocation) => (
                <span
                  className="v2-allocation-chip"
                  key={`${allocation.examId}-${allocation.nodeId}`}
                >
                  {shortExamCode(exams, allocation.examCode)} · {allocation.nodeTitle} ·{' '}
                  {Math.round(allocation.allocatedSeconds / 60)} min
                </span>
              ))}
            </div>
          )}
          <div className="v2-form-actions">
            <button className="primary-button" onClick={openStudyTimer} type="button">
              Open the study timer
            </button>
          </div>
        </div>
      )}

      {timerState?.status === 'paused' && (
        <div className="v2-form-actions">
          <button
            className="primary-button"
            disabled={busy !== null}
            onClick={() => void resumePausedTimer()}
            type="button"
          >
            {busy === 'start' ? 'Resuming…' : 'Resume timer'}
          </button>
          <button className="secondary-button" onClick={openStudyTimer} type="button">
            Open the study timer
          </button>
        </div>
      )}

      {timerState?.status === 'idle' && (
        <>
          <div className="v2-form-actions">
            <button className="primary-button" onClick={openStudyTimer} type="button">
              Open the study timer
            </button>
          </div>
          <div className="v2-day-plan-list">
            <div>
              <p className="eyebrow">Last used</p>
              {lastUsed === null ? (
                <p className="v2-empty-copy">
                  No recorded session yet. Start one below and it becomes the one-click preset.
                </p>
              ) : (
                <>
                  <ul className="v2-practice-history">
                    <li>
                      <div className="v2-day-topic-row">
                        <strong>
                          {lastUsed.combined ? 'Combined study session' : 'Repeat the last topics'}{' '}
                          · {lastUsed.sessionDate} · {lastUsed.plan.plannedMinutes} min
                        </strong>
                        <span>
                          {lastUsed.combined
                            ? 'No topic allocation was recorded with the last session.'
                            : lastUsed.rowsLabel}
                        </span>
                      </div>
                    </li>
                  </ul>
                  <div className="v2-form-actions">
                    <button
                      className="primary-button"
                      disabled={busy !== null}
                      onClick={() => void startPlan(lastUsed.plan)}
                      type="button"
                    >
                      {busy === 'start' ? 'Starting…' : 'Start last used configuration'}
                    </button>
                  </div>
                </>
              )}
            </div>

            <div>
              <p className="eyebrow">{draft.id ? 'Edit configuration' : 'Quick configuration'}</p>
              {draft.id && (
                <p className="v2-empty-copy">
                  Editing a saved configuration. Saving keeps its record; starting uses the values
                  exactly as shown.
                </p>
              )}
              <div className="v2-quickstart-duration">
                <label className="v2-field">
                  <span>Configuration name</span>
                  <input
                    disabled={busy !== null}
                    maxLength={80}
                    onChange={(event) => updateDraft({ name: event.target.value })}
                    value={draft.name}
                  />
                </label>
                <label className="v2-field">
                  <span>Session duration</span>
                  <input
                    max={240}
                    min={1}
                    onChange={(event) =>
                      updateDraft({ durationMinutes: Number(event.target.value) })
                    }
                    type="number"
                    value={Number.isFinite(draft.durationMinutes) ? draft.durationMinutes : ''}
                  />
                </label>
                <div aria-label="Duration presets" className="v2-quickstart-presets">
                  {QUICK_START_PRESETS.map((preset) => (
                    <button
                      aria-label={`Plan a ${preset} minute session`}
                      aria-pressed={draft.durationMinutes === preset}
                      className={`v2-day-preset${
                        draft.durationMinutes === preset ? ' v2-day-preset-active' : ''
                      }`}
                      disabled={busy !== null}
                      key={preset}
                      onClick={() => updateDraft({ durationMinutes: preset })}
                      type="button"
                    >
                      {preset} min
                    </button>
                  ))}
                </div>
              </div>
              {draft.allocations.map((row, index) => {
                const syllabus = syllabi[Number(row.examId)];
                const topics = (syllabus?.nodes ?? []).filter(
                  (node) => node.nodeKind === 'topic' && !node.isArchived,
                );
                return (
                  <div className="v2-timer-allocation-row" key={index}>
                    <label className="v2-field">
                      <span>Examination</span>
                      <select
                        disabled={busy !== null}
                        onChange={(event) =>
                          updateDraftRow(index, { examId: event.target.value, nodeId: '' })
                        }
                        value={row.examId}
                      >
                        <option value="">None (combined time)</option>
                        {activeExams.map((exam) => (
                          <option key={exam.id} value={exam.id}>
                            {exam.shortName} — {exam.code}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Subject / topic</span>
                      <select
                        disabled={busy !== null || topics.length === 0}
                        onChange={(event) => updateDraftRow(index, { nodeId: event.target.value })}
                        value={row.nodeId}
                      >
                        <option value="">
                          {syllabus ? 'No topic (combined time)' : 'Syllabus not loaded'}
                        </option>
                        {topics.map((topic) => (
                          <option key={topic.id} value={topic.id}>
                            {topic.nodePath.slice(-2).join(' › ')}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label className="v2-field">
                      <span>Minutes</span>
                      <input
                        disabled={busy !== null}
                        min={1}
                        onChange={(event) => updateDraftRow(index, { minutes: event.target.value })}
                        type="number"
                        value={row.minutes}
                      />
                    </label>
                    <button
                      aria-label="Remove allocation"
                      className="secondary-button v2-danger-button"
                      disabled={busy !== null}
                      onClick={() => removeDraftRow(index)}
                      type="button"
                    >
                      Remove
                    </button>
                  </div>
                );
              })}
              <div className="v2-form-actions">
                <button
                  className="secondary-button"
                  disabled={busy !== null}
                  onClick={addDraftRow}
                  type="button"
                >
                  Add exam/topic allocation
                </button>
              </div>
              <p
                className={draftError ? 'v2-study-error' : 'v2-empty-copy'}
                role={draftError ? 'alert' : undefined}
              >
                {draftError ??
                  `${draftTotalMinutes} of ${draftPlanCopy.plannedMinutes} min allocated. Unallocated time is counted once as combined study time.`}
              </p>
              <div className="v2-form-actions">
                <button
                  className="primary-button"
                  disabled={busy !== null || draftError !== null}
                  onClick={() => void startPlan(draftPlanCopy)}
                  type="button"
                >
                  {busy === 'start' ? 'Starting…' : 'Start focus session'}
                </button>
                <button
                  className="secondary-button"
                  disabled={busy !== null || draft.name.trim().length === 0}
                  onClick={() => void saveDraftConfiguration()}
                  type="button"
                >
                  {busy === 'configs'
                    ? 'Saving…'
                    : draft.id
                      ? 'Save changes'
                      : 'Save configuration'}
                </button>
                {draft.id && (
                  <button
                    className="secondary-button"
                    disabled={busy !== null}
                    onClick={cancelEditing}
                    type="button"
                  >
                    Cancel editing
                  </button>
                )}
              </div>
            </div>
          </div>

          <div>
            <p className="eyebrow">Saved configurations</p>
            {configsError && (
              <p className="v2-study-error" role="alert">
                {configsError}
              </p>
            )}
            {configsError && (
              <div className="v2-form-actions">
                <button
                  className="secondary-button"
                  disabled={busy === 'configs'}
                  onClick={() => void loadConfigs()}
                  type="button"
                >
                  Retry saved configurations
                </button>
              </div>
            )}
            {configs === null ? (
              <p className="v2-empty-copy">Loading saved configurations…</p>
            ) : configs.length === 0 ? (
              <p className="v2-empty-panel-copy">
                No saved configurations yet. Configure a session above and save it for a one-click
                start.
              </p>
            ) : (
              <ul className="v2-planner-list">
                {configs.map((config) => (
                  <li className="v2-planner-row" data-config-id={config.id} key={config.id}>
                    <div>
                      <div className="v2-session-heading">
                        <strong>{config.name}</strong>
                        <span className="v2-status-badge v2-status-inactive">
                          {config.durationMinutes} min
                        </span>
                        {config.isPinned && (
                          <span className="v2-status-badge v2-status-active">Pinned</span>
                        )}
                      </div>
                      {config.allocations.length === 0 ? (
                        <p className="v2-empty-copy">
                          Combined study session, no topic allocation.
                        </p>
                      ) : (
                        <div className="v2-allocation-chips">
                          {config.allocations.map((allocation) => (
                            <span
                              className="v2-allocation-chip"
                              key={`${config.id}-${allocation.examId}-${allocation.nodeId}`}
                            >
                              {shortExamCode(exams, allocation.examCode)} · {allocation.nodeTitle} ·{' '}
                              {allocation.allocatedMinutes} min
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                    <div className="v2-form-actions">
                      <button
                        aria-label={`Start the ${config.name} configuration`}
                        className="primary-button"
                        disabled={busy !== null}
                        onClick={() =>
                          void startPlan({
                            rows: config.allocations.map((allocation) => ({
                              examId: allocation.examId,
                              nodeId: allocation.nodeId,
                              minutes: allocation.allocatedMinutes,
                            })),
                            plannedMinutes: config.durationMinutes,
                          })
                        }
                        type="button"
                      >
                        Start
                      </button>
                      <button
                        aria-label={`Load ${config.name} into the quick configuration`}
                        className="secondary-button"
                        disabled={busy !== null}
                        onClick={() =>
                          loadIntoDraft({
                            id: config.id,
                            name: config.name,
                            durationMinutes: config.durationMinutes,
                            rows: config.allocations.map((allocation) => ({
                              examId: allocation.examId,
                              nodeId: allocation.nodeId,
                              minutes: allocation.allocatedMinutes,
                            })),
                          })
                        }
                        type="button"
                      >
                        Load
                      </button>
                      <button
                        aria-label={`${
                          config.isPinned ? 'Unpin' : 'Pin'
                        } the ${config.name} configuration`}
                        className="secondary-button"
                        disabled={busy !== null}
                        onClick={() =>
                          void (async () => {
                            setBusy('configs');
                            setActionError(null);
                            try {
                              const updated = await saveV2QuickStartConfig({
                                id: config.id,
                                name: config.name,
                                durationMinutes: config.durationMinutes,
                                isPinned: !config.isPinned,
                                allocations: config.allocations.map((allocation) => ({
                                  examId: allocation.examId,
                                  syllabusNodeId: allocation.nodeId,
                                  allocatedMinutes: allocation.allocatedMinutes,
                                })),
                              });
                              setConfigs(updated);
                            } catch (pinError) {
                              setActionError(toCommandError(pinError).message);
                            } finally {
                              setBusy(null);
                            }
                          })()
                        }
                        type="button"
                      >
                        {config.isPinned ? 'Unpin' : 'Pin'}
                      </button>
                      <button
                        aria-label={
                          deletingId === config.id
                            ? undefined
                            : `Delete the ${config.name} configuration`
                        }
                        className="secondary-button v2-danger-button"
                        disabled={busy !== null}
                        onClick={() => void deleteConfig(config.id)}
                        type="button"
                      >
                        {deletingId === config.id ? 'Confirm delete' : 'Delete'}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {recentConfigs.length > 0 && (
            <div>
              <p className="eyebrow">Recent study configurations</p>
              <ul className="v2-practice-history">
                {recentConfigs.map((config) => (
                  <li key={config.id}>
                    <div className="v2-day-topic-row">
                      <strong>
                        {config.sessionDate} · {config.plannedMinutes} min across{' '}
                        {plural(config.parts.length, 'topic')}
                      </strong>
                      <span>{config.parts.join(' + ')}</span>
                    </div>
                    <button
                      aria-label={`Load the recent configuration of ${config.sessionDate} into the quick configuration`}
                      className="secondary-button"
                      disabled={busy !== null}
                      onClick={() =>
                        loadIntoDraft({
                          id: null,
                          name: '',
                          durationMinutes: config.plannedMinutes,
                          rows: config.rows,
                        })
                      }
                      type="button"
                    >
                      Load
                    </button>
                  </li>
                ))}
              </ul>
              <p className="v2-empty-copy">
                Derived from recorded sessions as they happened; nothing is stored twice.
              </p>
            </div>
          )}
        </>
      )}

      {scrollNotice && (
        <p className="v2-study-notice" role="status">
          {scrollNotice}
        </p>
      )}
    </SectionCard>
  );
}
