import { useState } from 'react';
import { DateField } from './DateField';
import { PlannedBlockEditor } from './PlannedBlockEditor';
import { addDaysDateOnly, formatDateLong } from '../lib/dateOnly';
import type { PlannerTopic } from '../lib/plannerApi';
import {
  type RevisionDailyPlan,
  type RevisionDeferralReason,
  type RevisionPlanMembership,
} from '../lib/revisionApi';

const LANE_LABELS = {
  urgent_review: 'Urgent review',
  strengthen: 'Strengthen',
  maintain: 'Maintain',
} as const;

const REASON_LABELS: Record<RevisionDeferralReason, string> = {
  time: 'Not enough time',
  difficulty: 'Too difficult right now',
  energy: 'Low energy',
  life: 'Life interruption',
  other: 'Other',
};

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

type RevisionDailyPlanCardProps = {
  plan: RevisionDailyPlan;
  today: string;
  busy: string | null;
  feedback: Feedback;
  plannerTopics: PlannerTopic[];
  onGenerate: () => void;
  onRebalance: () => void;
  onAdd: (topicId: number) => void;
  onMove: (planItemId: number, target: 'backlog' | 'removed') => void;
  onReorder: (orderedPlanItemIds: number[]) => void;
  onEstimate: (planItemId: number, estimatedMinutes: number) => void;
  onStartTimer: (item: RevisionPlanMembership) => void;
  onLinkBlock: (planItemId: number, blockId: number) => void;
  onDefer: (
    topicId: number,
    newDate: string,
    reason: RevisionDeferralReason,
    notes: string,
  ) => void;
  onOverride: (
    topicId: number,
    overrideDate: string,
    reason: RevisionDeferralReason,
    notes: string,
  ) => void;
  onOpenTopic: (topicId: number, planItemId?: number | null) => void;
};

function minutesLabel(minutes: number): string {
  const rounded = Math.round(minutes * 10) / 10;
  return `${rounded} min`;
}

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function planDateLabel(value: string): string {
  return value || 'today';
}

export function RevisionDailyPlanCard({
  plan,
  today,
  busy,
  feedback,
  plannerTopics,
  onGenerate,
  onRebalance,
  onAdd,
  onMove,
  onReorder,
  onEstimate,
  onStartTimer,
  onLinkBlock,
  onDefer,
  onOverride,
  onOpenTopic,
}: RevisionDailyPlanCardProps) {
  const [estimateDrafts, setEstimateDrafts] = useState<Record<number, string>>({});
  const [planningItemId, setPlanningItemId] = useState<number | null>(null);
  const [deferringTopicId, setDeferringTopicId] = useState<number | null>(null);
  const [deferDate, setDeferDate] = useState(addDaysDateOnly(today, 7) ?? today);
  const [deferReason, setDeferReason] = useState<RevisionDeferralReason>('time');
  const [deferNotes, setDeferNotes] = useState('');
  const [overridingTopicId, setOverridingTopicId] = useState<number | null>(null);
  const [overrideDate, setOverrideDate] = useState(today);
  const [overrideReason, setOverrideReason] = useState<RevisionDeferralReason>('time');
  const [overrideNotes, setOverrideNotes] = useState('');

  const usedMinutes = Math.max(0, plan.effectiveBudgetMinutes - plan.remainingDailyBudgetMinutes);
  const usedPercent = Math.min(100, (usedMinutes / Math.max(1, plan.effectiveBudgetMinutes)) * 100);
  const overBoth = plan.overBudget && plan.overLimit;

  function saveEstimate(item: RevisionPlanMembership) {
    const parsed = Number(estimateDrafts[item.planItemId]);
    if (!Number.isInteger(parsed) || parsed < 5 || parsed > 240) {
      return;
    }
    if (parsed !== item.estimatedMinutes) {
      setEstimateDrafts((current) => {
        const next = { ...current };
        delete next[item.planItemId];
        return next;
      });
      onEstimate(item.planItemId, parsed);
    }
  }

  function reorder(itemId: number, direction: -1 | 1) {
    const ids = plan.scheduled.map((item) => item.planItemId);
    const index = ids.indexOf(itemId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= ids.length) {
      return;
    }
    [ids[index], ids[target]] = [ids[target], ids[index]];
    onReorder(ids);
  }

  function openDeferral(topicId: number, suggestedDate: string | null) {
    setOverridingTopicId(null);
    setDeferringTopicId(topicId);
    setDeferDate(suggestedDate ?? addDaysDateOnly(today, 7) ?? today);
    setDeferReason('time');
    setDeferNotes('');
  }

  function openOverride(topicId: number) {
    setDeferringTopicId(null);
    setOverridingTopicId(topicId);
    setOverrideDate(today);
    setOverrideReason('time');
    setOverrideNotes('');
  }

  return (
    <section className="revision-daily-plan-card" aria-labelledby="revision-daily-plan-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">Adaptive daily workload</p>
          <h2 id="revision-daily-plan-heading">Today&apos;s revision plan</h2>
          <p className="revision-plan-subtitle">
            {plan.needsGeneration
              ? 'No plan has been generated for today. Review recommendations stay advisory until you generate a plan.'
              : `Version ${plan.version ?? 1} · ${planDateLabel(plan.planDate ?? '')} · ${
                  plan.status ?? 'active'
                }`}
          </p>
        </div>
        <div className="revision-plan-heading-actions">
          {plan.needsGeneration ? (
            <button
              type="button"
              className="primary-button"
              onClick={onGenerate}
              disabled={busy !== null}
            >
              {busy === 'generate' ? 'Generating…' : "Generate today's plan"}
            </button>
          ) : (
            <>
              <button
                type="button"
                className="secondary-button"
                onClick={onRebalance}
                disabled={busy !== null}
              >
                {busy === 'rebalance' ? 'Rebalancing…' : 'Rebalance remaining work'}
              </button>
            </>
          )}
        </div>
      </div>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      <div className="revision-plan-stats" aria-label="Daily plan workload">
        <div>
          <span>Budget</span>
          <strong>{minutesLabel(plan.effectiveBudgetMinutes)}</strong>
          <small>{plan.maxTopics} topic maximum</small>
        </div>
        <div>
          <span>Planned</span>
          <strong>{minutesLabel(plan.plannedMinutes)}</strong>
          <small>{plan.scheduled.length} scheduled</small>
        </div>
        <div>
          <span>Completed today</span>
          <strong>{minutesLabel(plan.completedMinutes)}</strong>
          <small>{plan.completed.length} topic(s)</small>
        </div>
        <div>
          <span>Actual revision time</span>
          <strong>{minutesLabel(plan.actualMinutesToday)}</strong>
          <small>{minutesLabel(plan.remainingWorkMinutes)} work remaining</small>
        </div>
      </div>

      {!plan.needsGeneration && (
        <div
          className="progress-track revision-plan-progress"
          role="progressbar"
          aria-label="Daily revision budget used"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(usedPercent)}
        >
          <span style={{ width: `${usedPercent}%` }} />
        </div>
      )}

      {plan.needsGeneration ? (
        <p className="revision-empty">
          Generate today&apos;s plan when you are ready to commit to a workload. Deferrals are never
          counted as completed work.
        </p>
      ) : (
        <>
          {(plan.overBudget || plan.overLimit) && (
            <p className="revision-plan-warning" role="status">
              {overBoth
                ? 'This plan is over budget and over the topic limit because of explicit manual additions. Reduce, move to backlog, or keep it as a deliberate overload.'
                : plan.overBudget
                  ? 'This plan is over the daily revision budget. You can reduce or move work to the backlog.'
                  : 'This plan is over the configured topic limit. You can reduce or move work to the backlog.'}
            </p>
          )}
          <p className="revision-plan-remaining">
            {minutesLabel(plan.remainingDailyBudgetMinutes)} of the daily budget remains after
            completed and committed work.
          </p>

          <div className="revision-plan-list-heading">
            <h3>Scheduled work</h3>
            <span>{plan.scheduled.length} topic(s)</span>
          </div>
          {plan.scheduled.length === 0 ? (
            <p className="revision-empty">No scheduled work remains in this plan version.</p>
          ) : (
            <ul className="revision-plan-list">
              {plan.scheduled.map((item, index) => (
                <li
                  key={`plan-item-${item.planItemId}`}
                  className={`revision-plan-item revision-plan-item-${item.membershipStatus}`}
                >
                  <div className="revision-plan-item-heading">
                    <div>
                      <p className="revision-plan-topic">
                        <span className="revision-plan-position">{index + 1}</span>
                        {item.topic.code} · {item.topic.title}
                      </p>
                      <p className="revision-plan-meta">
                        {item.topic.sectionTitle} · {LANE_LABELS[item.lane]} · triage{' '}
                        {item.triageScore.toFixed(1)}
                      </p>
                    </div>
                    <div className="revision-plan-item-badges">
                      <span className={`revision-badge revision-badge-${item.itemStatus}`}>
                        {item.membershipStatus === 'in_progress' ? 'In progress' : 'Planned'}
                      </span>
                      <span className="revision-plan-time">
                        {minutesLabel(item.actualSeconds / 60)} actual ·{' '}
                        {minutesLabel(item.remainingEstimateMinutes)} remaining
                      </span>
                    </div>
                  </div>

                  {item.plannedBlock.title && (
                    <p className="revision-plan-block">
                      Planner block: {item.plannedBlock.title}
                      {item.plannedBlock.startTime
                        ? ` · ${item.plannedBlock.startTime}–${item.plannedBlock.endTime ?? ''}`
                        : ''}
                    </p>
                  )}

                  <div className="revision-plan-item-controls">
                    <label htmlFor={`plan-estimate-${item.planItemId}`}>Estimate</label>
                    <input
                      id={`plan-estimate-${item.planItemId}`}
                      type="number"
                      min={5}
                      max={240}
                      step={5}
                      value={estimateDrafts[item.planItemId] ?? String(item.estimatedMinutes)}
                      onChange={(event) =>
                        setEstimateDrafts((current) => ({
                          ...current,
                          [item.planItemId]: event.target.value,
                        }))
                      }
                      disabled={busy !== null}
                    />
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => saveEstimate(item)}
                      disabled={busy !== null}
                      aria-label={`Save estimate for ${item.topic.code}`}
                    >
                      Save estimate
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => reorder(item.planItemId, -1)}
                      disabled={busy !== null || index === 0}
                      aria-label={`Move ${item.topic.code} up`}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => reorder(item.planItemId, 1)}
                      disabled={busy !== null || index === plan.scheduled.length - 1}
                      aria-label={`Move ${item.topic.code} down`}
                    >
                      ↓
                    </button>
                  </div>

                  <div className="revision-plan-item-actions">
                    <button
                      type="button"
                      className="primary-button"
                      onClick={() => onStartTimer(item)}
                      disabled={busy !== null}
                      aria-label={`Start focus timer for ${item.topic.code}`}
                    >
                      {busy === `timer-${item.planItemId}` ? 'Starting…' : 'Start focus timer'}
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => {
                        setPlanningItemId(
                          planningItemId === item.planItemId ? null : item.planItemId,
                        );
                        setDeferringTopicId(null);
                      }}
                      disabled={busy !== null}
                      aria-label={
                        planningItemId === item.planItemId
                          ? `Cancel planned block for ${item.topic.code}`
                          : `Plan block for ${item.topic.code}`
                      }
                    >
                      {planningItemId === item.planItemId ? 'Cancel block' : 'Plan block'}
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => openDeferral(item.topic.id, null)}
                      disabled={busy !== null}
                      aria-label={`Defer revision for ${item.topic.code}`}
                    >
                      Defer
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onMove(item.planItemId, 'backlog')}
                      disabled={busy !== null}
                      aria-label={`Move ${item.topic.code} to backlog`}
                    >
                      Move to backlog
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onMove(item.planItemId, 'removed')}
                      disabled={busy !== null}
                      aria-label={`Remove ${item.topic.code} from today's plan`}
                    >
                      Remove
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onOpenTopic(item.topic.id, item.planItemId)}
                      aria-label={`Open topic ${item.topic.code}`}
                    >
                      Open topic
                    </button>
                  </div>

                  {planningItemId === item.planItemId && (
                    <div className="revision-plan-block-editor">
                      <PlannedBlockEditor
                        key={`plan-block-${item.planItemId}`}
                        block={null}
                        topics={plannerTopics}
                        defaultDate={today}
                        initialTitle={`Revision · ${item.topic.code}`}
                        initialTopicId={item.topic.id}
                        initialNotes={`Revision block for ${item.topic.title}`}
                        onSaved={(block) => {
                          setPlanningItemId(null);
                          onLinkBlock(item.planItemId, block.id);
                        }}
                        onCancel={() => setPlanningItemId(null)}
                      />
                    </div>
                  )}

                  {deferringTopicId === item.topic.id && (
                    <form
                      className="revision-plan-inline-form"
                      onSubmit={(event) => {
                        event.preventDefault();
                        setDeferringTopicId(null);
                        onDefer(item.topic.id, deferDate, deferReason, deferNotes);
                      }}
                    >
                      <DateField
                        id={`defer-date-${item.planItemId}`}
                        label="Return date"
                        value={deferDate}
                        onChange={setDeferDate}
                        disabled={busy !== null}
                      />
                      <div className="form-field">
                        <label htmlFor={`defer-reason-${item.planItemId}`}>Reason</label>
                        <select
                          id={`defer-reason-${item.planItemId}`}
                          value={deferReason}
                          onChange={(event) =>
                            setDeferReason(event.target.value as RevisionDeferralReason)
                          }
                          disabled={busy !== null}
                        >
                          {Object.entries(REASON_LABELS).map(([value, label]) => (
                            <option value={value} key={value}>
                              {label}
                            </option>
                          ))}
                        </select>
                      </div>
                      <div className="form-field">
                        <label htmlFor={`defer-notes-${item.planItemId}`}>Notes (optional)</label>
                        <input
                          id={`defer-notes-${item.planItemId}`}
                          value={deferNotes}
                          maxLength={2000}
                          onChange={(event) => setDeferNotes(event.target.value)}
                          disabled={busy !== null}
                        />
                      </div>
                      <div className="form-footer">
                        <button
                          type="submit"
                          className="primary-button"
                          disabled={busy !== null}
                          aria-label={`Defer revision for ${item.topic.code}`}
                        >
                          Defer revision
                        </button>
                        <button
                          type="button"
                          className="secondary-button"
                          onClick={() => setDeferringTopicId(null)}
                          disabled={busy !== null}
                        >
                          Cancel
                        </button>
                      </div>
                    </form>
                  )}
                </li>
              ))}
            </ul>
          )}

          {plan.completed.length > 0 && (
            <>
              <div className="revision-plan-list-heading">
                <h3>Completed on this plan</h3>
                <span>{plan.completed.length} topic(s)</span>
              </div>
              <ul className="revision-plan-completed-list">
                {plan.completed.map((item) => (
                  <li key={`completed-plan-item-${item.planItemId}`}>
                    <div>
                      <p className="revision-plan-topic">
                        {item.topic.code} · {item.topic.title}
                      </p>
                      <p className="revision-plan-meta">
                        {minutesLabel(item.actualSeconds / 60)} actual revision time
                      </p>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onOpenTopic(item.topic.id, item.planItemId)}
                      aria-label={`Open outcome for ${item.topic.code}`}
                    >
                      Open outcome
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}

          {plan.unscheduledUrgent.length > 0 && (
            <>
              <div className="revision-plan-list-heading">
                <h3>Urgent work left out</h3>
                <span>{plan.unscheduledUrgent.length} topic(s)</span>
              </div>
              <ul className="revision-plan-compact-list">
                {plan.unscheduledUrgent.map((item) => (
                  <li key={`urgent-${item.topic.id}`}>
                    <div>
                      <p className="revision-plan-topic">
                        {item.topic.code} · {item.topic.title}
                      </p>
                      <p className="revision-plan-meta">
                        {item.reason} · {minutesLabel(item.estimateMinutes)} ·{' '}
                        {LANE_LABELS[item.lane]}
                      </p>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onAdd(item.topic.id)}
                      disabled={busy !== null}
                      aria-label={`Add ${item.topic.code} to today's plan`}
                    >
                      Add anyway
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}

          {plan.backlog.length > 0 && (
            <>
              <div className="revision-plan-list-heading">
                <h3>Backlog</h3>
                <span>{plan.backlog.length} topic(s)</span>
              </div>
              <ul className="revision-plan-compact-list">
                {plan.backlog.map((item) => (
                  <li key={`backlog-${item.planItemId}`}>
                    <div>
                      <p className="revision-plan-topic">
                        {item.topic.code} · {item.topic.title}
                      </p>
                      <p className="revision-plan-meta">
                        {item.status === 'in_progress' ? 'In progress' : 'Backlog'} ·{' '}
                        {minutesLabel(item.remainingEstimateMinutes)} remaining ·{' '}
                        {LANE_LABELS[item.lane]}
                        {item.deferredUntil
                          ? ` · deferred until ${dateLabel(item.deferredUntil)}`
                          : ''}
                      </p>
                    </div>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => onAdd(item.topic.id)}
                      disabled={busy !== null}
                      aria-label={`Add ${item.topic.code} to today's plan`}
                    >
                      Add to today
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}

          {plan.deferred.length > 0 && (
            <>
              <div className="revision-plan-list-heading">
                <h3>Deferred</h3>
                <span>{plan.deferred.length} topic(s)</span>
              </div>
              <ul className="revision-plan-compact-list">
                {plan.deferred.map((item) => (
                  <li key={`deferred-${item.topic.id}`}>
                    <div>
                      <p className="revision-plan-topic">
                        {item.topic.code} · {item.topic.title}
                      </p>
                      <p className="revision-plan-meta">
                        Returns {dateLabel(item.deferredUntil)} · {item.deferralCount} deferral
                        {item.deferralCount === 1 ? '' : 's'} · original due{' '}
                        {dateLabel(item.originalDueDate)} · {REASON_LABELS[item.latestReason]}
                        {item.latestNotes ? ` · ${item.latestNotes}` : ''}
                      </p>
                    </div>
                    <div className="revision-plan-item-actions">
                      <button
                        type="button"
                        className="secondary-button"
                        onClick={() => openOverride(item.topic.id)}
                        disabled={busy !== null}
                        aria-label={`Override deferral for ${item.topic.code}`}
                      >
                        Override
                      </button>
                      <button
                        type="button"
                        className="secondary-button"
                        onClick={() => onOpenTopic(item.topic.id, item.planItemId)}
                        aria-label={`Open topic ${item.topic.code}`}
                      >
                        Open topic
                      </button>
                    </div>
                    {overridingTopicId === item.topic.id && (
                      <form
                        className="revision-plan-inline-form"
                        onSubmit={(event) => {
                          event.preventDefault();
                          setOverridingTopicId(null);
                          onOverride(item.topic.id, overrideDate, overrideReason, overrideNotes);
                        }}
                      >
                        <DateField
                          id={`override-date-${item.topic.id}`}
                          label="Bring back on"
                          value={overrideDate}
                          onChange={setOverrideDate}
                          hint="Must be earlier than the current return date and cannot be before today."
                          disabled={busy !== null}
                        />
                        <div className="form-field">
                          <label htmlFor={`override-reason-${item.topic.id}`}>Reason</label>
                          <select
                            id={`override-reason-${item.topic.id}`}
                            value={overrideReason}
                            onChange={(event) =>
                              setOverrideReason(event.target.value as RevisionDeferralReason)
                            }
                            disabled={busy !== null}
                          >
                            {Object.entries(REASON_LABELS).map(([value, label]) => (
                              <option value={value} key={value}>
                                {label}
                              </option>
                            ))}
                          </select>
                        </div>
                        <div className="form-field">
                          <label htmlFor={`override-notes-${item.topic.id}`}>
                            Notes (optional)
                          </label>
                          <input
                            id={`override-notes-${item.topic.id}`}
                            value={overrideNotes}
                            maxLength={2000}
                            onChange={(event) => setOverrideNotes(event.target.value)}
                            disabled={busy !== null}
                          />
                        </div>
                        <div className="form-footer">
                          <button
                            type="submit"
                            className="primary-button"
                            disabled={busy !== null}
                            aria-label={`Override deferral for ${item.topic.code}`}
                          >
                            Override deferral
                          </button>
                          <button
                            type="button"
                            className="secondary-button"
                            onClick={() => setOverridingTopicId(null)}
                            disabled={busy !== null}
                          >
                            Cancel
                          </button>
                        </div>
                      </form>
                    )}
                  </li>
                ))}
              </ul>
            </>
          )}
        </>
      )}
    </section>
  );
}
