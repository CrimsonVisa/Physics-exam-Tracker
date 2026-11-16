import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { emitRevisionsChanged } from '../lib/revisionEvents';
import {
  completeV2Revision,
  deferV2Revision,
  generateV2RevisionDailyPlan,
  getV2RevisionDailyPlan,
  getV2RevisionDashboard,
  moveV2RevisionPlanItem,
  rebalanceV2RevisionDailyPlan,
  rescheduleV2Revision,
  scheduleV2RevisionConcept,
  searchV2Concepts,
  type V2ExamSummary,
  type V2RevisionDailyPlan,
  type V2RevisionDashboard,
  type V2RevisionSchedule,
} from './api';

type RevisionScreenProps = {
  exams: V2ExamSummary[];
  today: string;
};

const OUTCOMES = [
  ['again', 'Again'],
  ['difficult', 'Difficult'],
  ['good', 'Good'],
  ['easy', 'Easy'],
] as const;

function scheduledNotice(schedule: V2RevisionSchedule): string {
  const prefix = schedule.alreadyScheduled
    ? 'This concept was already in the revision schedule.'
    : 'Concept added to the revision schedule.';
  if (schedule.status !== 'active') {
    return `${prefix} Next revision ${schedule.nextRevisionDate}. The schedule is ${schedule.status}, so it will not appear in revision queues until it is active again.`;
  }
  if (!schedule.eligible) {
    if (schedule.pendingExamCodes.length > 0) {
      return `${prefix} Next revision ${schedule.nextRevisionDate}. It is awaiting approval of its ${schedule.pendingExamCodes.join(', ')} syllabus mapping, so it will not appear in revision queues until that mapping is approved in Concepts.`;
    }
    return `${prefix} Next revision ${schedule.nextRevisionDate}. It has no active exam mapping, so it will not appear in revision queues until it is mapped to a syllabus topic in Concepts.`;
  }
  return `${prefix} Next revision ${schedule.nextRevisionDate}. It is eligible for revision queues because its mapping is approved.`;
}

export function RevisionScreen({ exams, today }: RevisionScreenProps) {
  const [examCode, setExamCode] = useState('');
  const [dashboard, setDashboard] = useState<V2RevisionDashboard | null>(null);
  const [plan, setPlan] = useState<V2RevisionDailyPlan | null>(null);
  const [concepts, setConcepts] = useState<Array<{ id: number; title: string }>>([]);
  const [conceptId, setConceptId] = useState('');
  const [outcomes, setOutcomes] = useState<Record<number, string>>({});
  const [deferDates, setDeferDates] = useState<Record<number, string>>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const examId = exams.find((exam) => exam.code === examCode)?.id ?? null;

  const load = useCallback(
    async (selectedExamId: number | null) => {
      try {
        const [nextDashboard, nextPlan] = await Promise.all([
          getV2RevisionDashboard({ examId: selectedExamId, today }),
          getV2RevisionDailyPlan({ planDate: today, examId: selectedExamId }),
        ]);
        setDashboard(nextDashboard);
        setPlan(nextPlan);
        setError(null);
      } catch (loadError) {
        setError(toCommandError(loadError).message);
      }
    },
    [today],
  );

  useEffect(() => {
    void (async () => {
      await load(examId);
    })();
  }, [examId, load]);

  useEffect(() => {
    void (async () => {
      try {
        const result = await searchV2Concepts({ limit: 200 });
        setConcepts(result.concepts.map((concept) => ({ id: concept.id, title: concept.title })));
      } catch {
        // The revision workspace still works for already scheduled concepts.
      }
    })();
  }, []);

  async function run(action: () => Promise<string | void>, message: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const actionMessage = await action();
      await load(examId);
      setNotice(actionMessage ?? message);
    } catch (actionError) {
      setError(toCommandError(actionError).message);
    } finally {
      setBusy(false);
    }
  }

  function scheduleSelectedConcept() {
    if (!conceptId) {
      setError('Choose a concept to schedule.');
      return;
    }
    void run(async () => {
      const schedule = await scheduleV2RevisionConcept({ conceptId: Number(conceptId) });
      setConceptId('');
      emitRevisionsChanged();
      return scheduledNotice(schedule);
    }, 'Concept added to the revision schedule.');
  }

  function complete(schedule: V2RevisionSchedule) {
    void run(async () => {
      await completeV2Revision({
        conceptId: schedule.conceptId,
        outcome: outcomes[schedule.conceptId] ?? 'good',
        revisionDate: today,
        triggerExamId: examId,
      });
      emitRevisionsChanged();
    }, 'Revision completed and mastery evidence recorded.');
  }

  function reschedule(schedule: V2RevisionSchedule) {
    const date = deferDates[schedule.conceptId];
    if (!date) {
      setError('Choose a later revision date.');
      return;
    }
    void run(async () => {
      await deferV2Revision({
        conceptId: schedule.conceptId,
        newRevisionDate: date,
        reason: 'time',
      });
      emitRevisionsChanged();
    }, 'Revision deferred.');
  }

  function moveToToday(schedule: V2RevisionSchedule) {
    void run(async () => {
      await rescheduleV2Revision({
        conceptId: schedule.conceptId,
        nextRevisionDate: today,
      });
      emitRevisionsChanged();
      try {
        const updated = await generateV2RevisionDailyPlan({ planDate: today, examId });
        setPlan(updated);
        emitRevisionsChanged();
        return updated.items.some((item) => item.conceptId === schedule.conceptId)
          ? 'Revision moved to today and added to the daily plan.'
          : 'Revision moved to today. The daily plan is full, so use Rebalance to make room for it.';
      } catch (planError) {
        return `Revision moved to today, but the daily plan could not be refreshed: ${
          toCommandError(planError).message
        }`;
      }
    }, 'Revision scheduled for today.');
  }

  function renderSchedule(schedule: V2RevisionSchedule) {
    return (
      <li className="v2-planner-row" key={schedule.conceptId}>
        <div>
          <div className="v2-session-heading">
            <strong>{schedule.conceptTitle}</strong>
            <span className="v2-status-badge v2-status-inactive">{schedule.status}</span>
            {schedule.overdueDays > 0 && (
              <span className="v2-status-badge v2-status-inactive">
                {schedule.overdueDays} day(s) overdue
              </span>
            )}
          </div>
          <p className="v2-empty-copy">
            {schedule.conceptDomain} · due {schedule.nextRevisionDate} · interval{' '}
            {schedule.currentIntervalDays} days · mastery{' '}
            {schedule.masteryScore === null ? 'No evidence' : schedule.masteryScore.toFixed(1)}
          </p>
          <p className="v2-empty-copy">{schedule.selectionReason}</p>
          <div className="v2-resource-controls">
            <label className="v2-field">
              <span>Outcome</span>
              <select
                onChange={(event) =>
                  setOutcomes((current) => ({
                    ...current,
                    [schedule.conceptId]: event.target.value,
                  }))
                }
                value={outcomes[schedule.conceptId] ?? 'good'}
              >
                {OUTCOMES.map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
            <button
              className="primary-button"
              disabled={busy}
              onClick={() => complete(schedule)}
              type="button"
            >
              Complete
            </button>
            <label className="v2-field">
              <span>Defer to</span>
              <input
                onChange={(event) =>
                  setDeferDates((current) => ({
                    ...current,
                    [schedule.conceptId]: event.target.value,
                  }))
                }
                type="date"
                value={deferDates[schedule.conceptId] ?? ''}
              />
            </label>
            <button
              className="secondary-button"
              disabled={busy}
              onClick={() => reschedule(schedule)}
              type="button"
            >
              Defer
            </button>
            <button
              className="secondary-button"
              disabled={busy}
              onClick={() => moveToToday(schedule)}
              type="button"
            >
              Due today
            </button>
          </div>
        </div>
      </li>
    );
  }

  function renderUpcoming(schedule: V2RevisionSchedule) {
    return (
      <li className="v2-planner-row" key={schedule.conceptId}>
        <div>
          <div className="v2-session-heading">
            <strong>{schedule.conceptTitle}</strong>
            <span className="v2-status-badge v2-status-active">{schedule.nextRevisionDate}</span>
          </div>
          <p className="v2-empty-copy">
            {schedule.conceptDomain} · due {schedule.nextRevisionDate} · interval{' '}
            {schedule.currentIntervalDays} days · mastery{' '}
            {schedule.masteryScore === null ? 'No evidence' : schedule.masteryScore.toFixed(1)}
          </p>
          <p className="v2-empty-copy">
            Approved mapping: {schedule.approvedExamCodes.join(', ') || 'none'}
          </p>
        </div>
        <div className="v2-form-actions">
          <button
            className="secondary-button"
            disabled={busy}
            onClick={() => moveToToday(schedule)}
            type="button"
          >
            Move to today
          </button>
        </div>
      </li>
    );
  }

  function renderAwaitingApproval(schedule: V2RevisionSchedule) {
    return (
      <li className="v2-planner-row" key={schedule.conceptId}>
        <div>
          <div className="v2-session-heading">
            <strong>{schedule.conceptTitle}</strong>
            <span className="v2-status-badge v2-status-inactive">Awaiting approval</span>
          </div>
          <p className="v2-empty-copy">
            {schedule.conceptDomain} · provisional next revision {schedule.nextRevisionDate}
          </p>
          <p className="v2-empty-copy">
            {schedule.pendingExamCodes.length > 0
              ? `Pending mapping: ${schedule.pendingExamCodes.join(', ')}. Approve the concept-to-syllabus mapping in Concepts before this revision can enter a queue or daily plan.`
              : 'No active exam mapping. Map this concept to a syllabus topic in Concepts before this revision can enter a queue or daily plan.'}
          </p>
        </div>
      </li>
    );
  }

  return (
    <div className="page-container v2-revision-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · spaced review</p>
        <h1>Revision planning</h1>
        <p className="page-lede">
          Concept-linked revision schedules, daily plans, deferrals, and completion history.
          Completing a revision records revision evidence and recalculates shared mastery.
        </p>
      </header>

      <div className="v2-planner-datebar">
        <label className="v2-field">
          <span>Examination scope</span>
          <select onChange={(event) => setExamCode(event.target.value)} value={examCode}>
            <option value="">Combined</option>
            {exams.map((exam) => (
              <option key={exam.code} value={exam.code}>
                {exam.shortName}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Schedule a shared concept</span>
          <select onChange={(event) => setConceptId(event.target.value)} value={conceptId}>
            <option value="">Choose a concept…</option>
            {concepts.map((concept) => (
              <option key={concept.id} value={concept.id}>
                {concept.title}
              </option>
            ))}
          </select>
        </label>
        <button
          className="primary-button"
          disabled={busy}
          onClick={scheduleSelectedConcept}
          type="button"
        >
          Schedule concept
        </button>
      </div>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="v2-study-notice" role="status">
          {notice}
        </p>
      )}

      {dashboard && (
        <section className="v2-coverage-grid">
          <article className="v2-coverage-card">
            <span className="v2-status-badge v2-status-inactive">Due today</span>
            <h2>{dashboard.stats.dueToday}</h2>
            <p className="v2-empty-copy">Active schedules due on {today}.</p>
          </article>
          <article className="v2-coverage-card">
            <span className="v2-status-badge v2-status-inactive">Overdue</span>
            <h2>{dashboard.stats.overdue}</h2>
            <p className="v2-empty-copy">Schedules whose due date has passed.</p>
          </article>
          <article className="v2-coverage-card">
            <span className="v2-status-badge v2-status-active">Completed today</span>
            <h2>{dashboard.stats.completedToday}</h2>
            <p className="v2-empty-copy">Last 7 days: {dashboard.stats.completedLast7Days}.</p>
          </article>
          <article className="v2-coverage-card">
            <span className="v2-status-badge v2-status-inactive">All completed</span>
            <h2>{dashboard.stats.totalCompleted}</h2>
            <p className="v2-empty-copy">Append-only revision history.</p>
          </article>
        </section>
      )}

      <div className="v2-planner-grid">
        <section className="v2-concept-card">
          <p className="eyebrow">Due queue</p>
          <h2>Overdue and due today</h2>
          {!dashboard || dashboard.overdue.length + dashboard.dueToday.length === 0 ? (
            <p className="v2-empty-panel-copy">
              No revisions are due. Schedule concepts or wait for the next due date.
            </p>
          ) : (
            <ul className="v2-planner-list">
              {[...dashboard.overdue, ...dashboard.dueToday].map(renderSchedule)}
            </ul>
          )}
        </section>

        <section className="v2-concept-card">
          <p className="eyebrow">Daily plan</p>
          <h2>{today} revision plan</h2>
          <div className="v2-form-actions">
            <button
              className="primary-button"
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  const generated = await generateV2RevisionDailyPlan({
                    planDate: today,
                    examId,
                  });
                  setPlan(generated);
                  return generated.items.length === 0
                    ? 'Daily plan refreshed. No revisions are due today or overdue.'
                    : `Daily plan updated with ${generated.items.length} due revision${
                        generated.items.length === 1 ? '' : 's'
                      }.`;
                }, 'Daily revision plan generated.')
              }
              type="button"
            >
              Generate plan
            </button>
            <button
              className="secondary-button"
              disabled={busy || !plan}
              onClick={() =>
                void run(async () => {
                  const next = await rebalanceV2RevisionDailyPlan({
                    planDate: today,
                    examId,
                  });
                  setPlan(next);
                  return next.items.length === 0
                    ? 'Daily plan rebalanced. No revisions are due today or overdue.'
                    : `Daily plan rebalanced with ${next.items.length} due revision${
                        next.items.length === 1 ? '' : 's'
                      }.`;
                }, 'Daily revision plan rebalanced.')
              }
              type="button"
            >
              Rebalance
            </button>
          </div>
          {!plan ? (
            <p className="v2-empty-panel-copy">
              No active daily plan. Generate a plan to include concepts that are due today or
              overdue.
            </p>
          ) : plan.items.length === 0 ? (
            <p className="v2-empty-panel-copy">
              The active plan contains no eligible due concepts. Upcoming revisions enter the plan
              on their due date, and scheduled concepts need an approved syllabus mapping.
            </p>
          ) : (
            <ul className="v2-planner-list">
              {plan.items.map((item) => (
                <li className="v2-planner-row" key={item.planItemId}>
                  <div>
                    <div className="v2-session-heading">
                      <strong>{item.conceptTitle}</strong>
                      <span className="v2-status-badge v2-status-inactive">{item.lane}</span>
                      <span className="v2-session-date">{item.membershipStatus}</span>
                    </div>
                    <p className="v2-empty-copy">
                      {item.estimatedMinutes} min · triage {item.triageScore} · due{' '}
                      {item.sourceDueDate}
                    </p>
                    <p className="v2-empty-copy">{item.selectionReason}</p>
                  </div>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          const next = await moveV2RevisionPlanItem({
                            planItemId: item.planItemId,
                            target: 'backlog',
                            planDate: today,
                            examId,
                          });
                          setPlan(next);
                        }, 'Revision item moved to backlog.')
                      }
                      type="button"
                    >
                      Backlog
                    </button>
                    <button
                      className="secondary-button v2-danger-button"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          const next = await moveV2RevisionPlanItem({
                            planItemId: item.planItemId,
                            target: 'removed',
                            planDate: today,
                            examId,
                          });
                          setPlan(next);
                        }, 'Revision item removed from the plan.')
                      }
                      type="button"
                    >
                      Remove
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      <section className="v2-concept-card">
        <p className="eyebrow">Upcoming</p>
        <h2>Next scheduled revisions</h2>
        {!dashboard || dashboard.upcoming.length === 0 ? (
          <p className="v2-empty-panel-copy">
            No upcoming revisions. Schedule a concept to see its next review date here.
          </p>
        ) : (
          <ul className="v2-planner-list">{dashboard.upcoming.map(renderUpcoming)}</ul>
        )}
      </section>

      {dashboard && dashboard.awaitingApproval.length > 0 && (
        <section className="v2-concept-card">
          <p className="eyebrow">Awaiting approval</p>
          <h2>Scheduled concepts without an approved mapping</h2>
          <ul className="v2-planner-list">
            {dashboard.awaitingApproval.map(renderAwaitingApproval)}
          </ul>
        </section>
      )}

      <section className="v2-concept-card">
        <p className="eyebrow">History</p>
        <h2>Recent revisions</h2>
        {!dashboard || dashboard.recentHistory.length === 0 ? (
          <p className="v2-empty-panel-copy">No revision history yet.</p>
        ) : (
          <ul className="v2-planner-list">
            {dashboard.recentHistory.map((entry) => (
              <li className="v2-planner-row" key={entry.id}>
                <div>
                  <strong>{entry.conceptTitle}</strong>
                  <p className="v2-empty-copy">
                    {entry.revisionDate} · {entry.outcome} · scheduled {entry.scheduledForDate} ·
                    interval {entry.intervalBeforeDays} → {entry.intervalAfterDays} days
                  </p>
                  <p className="v2-empty-copy">
                    Mastery {entry.masteryBefore.toFixed(1)} → {entry.masteryAfter.toFixed(1)}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
