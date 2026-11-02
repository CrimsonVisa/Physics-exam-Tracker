import { type FormEvent, useEffect, useState } from 'react';
import { DateField } from './DateField';
import { PlannedBlockEditor } from './PlannedBlockEditor';
import { toCommandError } from '../lib/commandError';
import { addDaysDateOnly, formatDateLong, parseDateOnly } from '../lib/dateOnly';
import type { PlannerTopic } from '../lib/plannerApi';
import {
  completeRevision,
  deferRevision,
  getRevisionHistory,
  getRevisionIntervalPreview,
  overrideDeferredRevision,
  reactivateRevision,
  rescheduleRevision,
  REVISION_OUTCOMES,
  scheduleRevisionTopic,
  suspendRevision,
  updateRevisionNotes,
  type CompleteRevisionInput,
  type RevisionDeferralReason,
  type RevisionHistoryEntry,
  type RevisionIntervalDecision,
  type RevisionOutcome,
  type RevisionSettings,
  type RevisionTopicState,
} from '../lib/revisionApi';
import { nextIntervalFor, outcomeLabel, OUTCOME_LABELS } from '../lib/revisionScheduling';

type DetailMode = 'idle' | 'reviewing' | 'rescheduling' | 'planning' | 'deferring' | 'overriding';

const DEFERRAL_REASONS: Record<RevisionDeferralReason, string> = {
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

type RevisionTopicDetailProps = {
  state: RevisionTopicState;
  settings: RevisionSettings;
  today: string;
  plannerTopics: PlannerTopic[];
  planItemId?: number | null;
  onChanged: () => void;
  onClose: () => void;
  readOnly?: boolean;
};

function dateLabel(value: string | null): string {
  if (!value) {
    return 'Not recorded';
  }
  return formatDateLong(value) ?? value;
}

function statusLabel(state: RevisionTopicState): string {
  if (!state.schedule) {
    return 'Not scheduled';
  }
  return state.schedule.status === 'suspended' ? 'Suspended' : 'Active';
}

export function RevisionTopicDetail({
  state,
  settings,
  today,
  plannerTopics,
  planItemId = null,
  onChanged,
  onClose,
  readOnly = false,
}: RevisionTopicDetailProps) {
  const [history, setHistory] = useState<RevisionHistoryEntry[]>([]);
  const [historyLoading, setHistoryLoading] = useState(true);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyKey, setHistoryKey] = useState(0);
  const [mode, setMode] = useState<DetailMode>('idle');
  const [outcome, setOutcome] = useState<RevisionOutcome>('good');
  const [revisionNotes, setRevisionNotes] = useState('');
  const [notesDraft, setNotesDraft] = useState(state.schedule?.notes ?? '');
  const [scheduleDate, setScheduleDate] = useState(today);
  const [rescheduleDate, setRescheduleDate] = useState(state.schedule?.nextRevisionDate ?? today);
  const [dateError, setDateError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [intervalPreview, setIntervalPreview] = useState<RevisionIntervalDecision | null>(null);
  const [intervalPreviewError, setIntervalPreviewError] = useState<string | null>(null);
  const [deferDate, setDeferDate] = useState(
    addDaysDateOnly(state.schedule?.nextRevisionDate ?? today, 7) ?? today,
  );
  const [deferReason, setDeferReason] = useState<RevisionDeferralReason>('time');
  const [deferNotes, setDeferNotes] = useState('');
  const [overrideDate, setOverrideDate] = useState(today);
  const [overrideReason, setOverrideReason] = useState<RevisionDeferralReason>('time');
  const [overrideNotes, setOverrideNotes] = useState('');

  useEffect(() => {
    let cancelled = false;

    getRevisionHistory(state.topic.id)
      .then((loaded) => {
        if (!cancelled) {
          setHistory(loaded);
          setHistoryError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setHistoryError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setHistoryLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [state.topic.id, historyKey]);

  useEffect(() => {
    if (mode !== 'reviewing' || !state.schedule) {
      return;
    }
    let cancelled = false;
    getRevisionIntervalPreview(state.topic.id, outcome, today)
      .then((preview) => {
        if (!cancelled) {
          setIntervalPreview(preview);
          setIntervalPreviewError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setIntervalPreview(null);
          setIntervalPreviewError(toCommandError(error).message);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [mode, outcome, state.topic.id, state.schedule, today]);

  function refreshHistory() {
    setHistoryLoading(true);
    setHistoryKey((key) => key + 1);
  }

  function actionError(error: unknown) {
    setFeedback({ tone: 'error', message: toCommandError(error).message });
  }

  async function handleSchedule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (!parseDateOnly(scheduleDate)) {
      setDateError('Choose a real calendar date.');
      return;
    }

    setDateError(null);
    setBusy('schedule');
    setFeedback(null);

    try {
      await scheduleRevisionTopic(state.topic.id, scheduleDate);
      setFeedback({
        tone: 'success',
        message: `Revision scheduling started for ${state.topic.code}.`,
      });
      refreshHistory();
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleComplete(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (revisionNotes.length > 2_000) {
      setFeedback({ tone: 'error', message: 'Revision notes must be 2,000 characters or fewer.' });
      return;
    }

    setBusy('complete');
    setFeedback(null);
    const input: CompleteRevisionInput = {
      outcome,
      notes: revisionNotes,
      ...(planItemId === null ? {} : { planItemId }),
    };

    try {
      const result = await completeRevision(state.topic.id, input);
      setFeedback({
        tone: 'success',
        message: `Revision recorded. Next review is ${dateLabel(
          result.state.schedule?.nextRevisionDate ?? null,
        )} with a ${result.state.schedule?.currentIntervalDays ?? 0}-day interval.${
          result.history.actualSeconds !== null
            ? ` Actual linked time: ${(result.history.actualSeconds / 60).toFixed(1)} minutes.`
            : ''
        }`,
      });
      setMode('idle');
      setRevisionNotes('');
      refreshHistory();
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleReschedule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (!parseDateOnly(rescheduleDate)) {
      setDateError('Choose a real calendar date.');
      return;
    }

    setDateError(null);
    setBusy('reschedule');
    setFeedback(null);

    try {
      await rescheduleRevision(state.topic.id, rescheduleDate);
      setFeedback({
        tone: 'success',
        message: `Next revision manually set to ${dateLabel(rescheduleDate)}.`,
      });
      setMode('idle');
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleDefer(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (!parseDateOnly(deferDate)) {
      setDateError('Choose a real calendar date.');
      return;
    }
    setDateError(null);
    setBusy('defer');
    setFeedback(null);
    try {
      const result = await deferRevision(state.topic.id, deferDate, deferReason, deferNotes);
      setFeedback({
        tone: 'success',
        message: `Revision deferred until ${dateLabel(
          result.schedule.deferredUntil ?? deferDate,
        )}. Deferrals are recorded in an append-only audit trail and do not count as completed work.`,
      });
      setMode('idle');
      setDeferNotes('');
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleOverride(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (!parseDateOnly(overrideDate)) {
      setDateError('Choose a real calendar date.');
      return;
    }
    setDateError(null);
    setBusy('override');
    setFeedback(null);
    try {
      const result = await overrideDeferredRevision(
        state.topic.id,
        overrideDate,
        overrideReason,
        overrideNotes,
      );
      setFeedback({
        tone: 'success',
        message: `Deferral overridden. The topic is eligible again from ${dateLabel(
          result.schedule.deferredUntil ?? overrideDate,
        )}.`,
      });
      setMode('idle');
      setOverrideNotes('');
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleStatusChange(status: 'active' | 'suspended') {
    if (busy) {
      return;
    }
    setBusy(status);
    setFeedback(null);

    try {
      if (status === 'suspended') {
        await suspendRevision(state.topic.id);
        setFeedback({ tone: 'success', message: 'Revision scheduling suspended.' });
      } else {
        await reactivateRevision(state.topic.id);
        setFeedback({ tone: 'success', message: 'Revision scheduling reactivated.' });
      }
      setMode('idle');
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  async function handleSaveNotes(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }
    if (notesDraft.length > 2_000) {
      setFeedback({ tone: 'error', message: 'Notes must be 2,000 characters or fewer.' });
      return;
    }

    setBusy('notes');
    setFeedback(null);
    try {
      await updateRevisionNotes(state.topic.id, notesDraft);
      setFeedback({ tone: 'success', message: 'Revision notes saved.' });
      onChanged();
    } catch (error) {
      actionError(error);
    } finally {
      setBusy(null);
    }
  }

  const schedule = state.schedule;
  const currentInterval = schedule?.currentIntervalDays ?? settings.minimumIntervalDays;
  const nextPreview = nextIntervalFor(settings, currentInterval, outcome);
  const isDeferred = Boolean(schedule?.deferredUntil && schedule.deferredUntil > today);
  const displayedInterval = intervalPreview?.intervalAfterDays ?? nextPreview;

  return (
    <section className="revision-detail-card" aria-labelledby="revision-detail-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">{state.topic.sectionTitle}</p>
          <h2 id="revision-detail-heading">
            {state.topic.code} · {state.topic.title}
          </h2>
          <div className="revision-detail-badges">
            <span
              className={`revision-badge revision-badge-${state.dueState.replaceAll('_', '-')}`}
            >
              {state.dueState === 'overdue'
                ? `${state.daysOverdue} day${state.daysOverdue === 1 ? '' : 's'} overdue`
                : state.dueState === 'due_today'
                  ? 'Due today'
                  : state.dueState === 'upcoming'
                    ? `Upcoming in ${state.daysUntilDue ?? 0} day${state.daysUntilDue === 1 ? '' : 's'}`
                    : state.dueState === 'suspended'
                      ? 'Suspended'
                      : 'Not scheduled'}
            </span>
            <span
              className={`revision-badge revision-badge-${statusLabel(state)
                .toLowerCase()
                .replaceAll(' ', '-')}`}
            >
              {statusLabel(state)}
            </span>
            {state.topic.archived && <span className="revision-badge">Archived topic</span>}
          </div>
        </div>
        <button type="button" className="secondary-button" onClick={onClose}>
          Close details
        </button>
      </div>

      {schedule ? (
        <>
          <dl className="revision-facts">
            <div>
              <dt>Next revision</dt>
              <dd>{dateLabel(schedule.nextRevisionDate)}</dd>
            </div>
            <div>
              <dt>Last revision</dt>
              <dd>{dateLabel(schedule.lastRevisionDate)}</dd>
            </div>
            <div>
              <dt>Current interval</dt>
              <dd>{schedule.currentIntervalDays} days</dd>
            </div>
            <div>
              <dt>Completed revisions</dt>
              <dd>{schedule.completedRevisions}</dd>
            </div>
            <div>
              <dt>Previous outcome</dt>
              <dd>{outcomeLabel(schedule.previousOutcome)}</dd>
            </div>
            <div>
              <dt>Revision mastery estimate</dt>
              <dd>{schedule.masteryEstimate.toFixed(1)}%</dd>
            </div>
            {schedule.deferredUntil && (
              <div>
                <dt>Deferred until</dt>
                <dd>{dateLabel(schedule.deferredUntil)}</dd>
              </div>
            )}
          </dl>

          {readOnly && (
            <p className="revision-note">
              Archived topic. Revision scheduling is read-only until the topic is unarchived.
            </p>
          )}

          {!readOnly && (
            <div className="revision-actions">
              <button
                type="button"
                className="primary-button"
                onClick={() => {
                  setIntervalPreview(null);
                  setIntervalPreviewError(null);
                  setMode('reviewing');
                  setFeedback(null);
                }}
                disabled={busy !== null || schedule.status === 'suspended' || isDeferred}
              >
                Start revision
              </button>
              {isDeferred ? (
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => {
                    setMode('overriding');
                    setOverrideDate(today);
                    setDateError(null);
                    setFeedback(null);
                  }}
                  disabled={busy !== null}
                >
                  Override deferral
                </button>
              ) : (
                <>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => {
                      setMode('deferring');
                      setDeferDate(
                        addDaysDateOnly(schedule.nextRevisionDate, 7) ?? schedule.nextRevisionDate,
                      );
                      setDateError(null);
                      setFeedback(null);
                    }}
                    disabled={busy !== null}
                  >
                    Defer revision
                  </button>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => {
                      setMode('rescheduling');
                      setRescheduleDate(schedule.nextRevisionDate);
                      setDateError(null);
                      setFeedback(null);
                    }}
                    disabled={busy !== null}
                  >
                    Manual reschedule
                  </button>
                </>
              )}
              <button
                type="button"
                className="secondary-button"
                onClick={() => {
                  setMode('planning');
                  setFeedback(null);
                }}
                disabled={busy !== null}
              >
                Plan study block
              </button>
              {schedule.status === 'active' ? (
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => void handleStatusChange('suspended')}
                  disabled={busy !== null}
                >
                  {busy === 'suspended' ? 'Suspending…' : 'Suspend scheduling'}
                </button>
              ) : (
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => void handleStatusChange('active')}
                  disabled={busy !== null}
                >
                  {busy === 'active' ? 'Reactivating…' : 'Reactivate scheduling'}
                </button>
              )}
            </div>
          )}
        </>
      ) : readOnly ? (
        <p className="revision-note">
          This archived topic has no revision schedule. Any saved history remains available below.
        </p>
      ) : (
        <form className="revision-inline-form" onSubmit={handleSchedule} noValidate>
          <p className="revision-note">
            This topic has no schedule yet. Adding one does not change syllabus progress.
          </p>
          <DateField
            id="revision-first-date"
            label="First revision date"
            value={scheduleDate}
            onChange={(value) => {
              setScheduleDate(value);
              setDateError(null);
            }}
            error={dateError ?? undefined}
            disabled={busy !== null}
          />
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy !== null}>
              {busy === 'schedule' ? 'Saving…' : 'Add to revision schedule'}
            </button>
          </div>
        </form>
      )}

      {!readOnly && feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      {!readOnly && schedule && mode === 'reviewing' && (
        <form className="revision-review-form" onSubmit={handleComplete} noValidate>
          <div className="card-heading">
            <div>
              <p className="eyebrow">Revision in progress</p>
              <h3>Record the outcome</h3>
            </div>
            <button
              type="button"
              className="secondary-button"
              onClick={() => setMode('idle')}
              disabled={busy !== null}
            >
              Cancel revision
            </button>
          </div>
          <div className="planner-form-row">
            <div className="form-field">
              <label htmlFor="revision-outcome">Outcome</label>
              <select
                id="revision-outcome"
                value={outcome}
                onChange={(event) => setOutcome(event.target.value as RevisionOutcome)}
                disabled={busy !== null}
              >
                {REVISION_OUTCOMES.map((value) => (
                  <option value={value} key={value}>
                    {OUTCOME_LABELS[value]}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label htmlFor="revision-interval-preview">Next interval preview</label>
              <output id="revision-interval-preview" className="revision-interval-preview">
                {displayedInterval} days
              </output>
            </div>
          </div>
          {intervalPreviewError ? (
            <p className="field-hint">
              Adaptive interval preview is unavailable: {intervalPreviewError}. The saved outcome
              will still use the current rules.
            </p>
          ) : intervalPreview ? (
            <div className="revision-interval-reasons">
              <p className="field-hint">
                Timing {intervalPreview.timingDays >= 0 ? '+' : ''}
                {intervalPreview.timingDays} day(s) · evidence {intervalPreview.evidenceStatus} ·
                factor {intervalPreview.evidenceFactor.toFixed(2)}
              </p>
              <ul className="revision-reasons">
                {intervalPreview.reasons.map((reason) => (
                  <li key={reason}>{reason}</li>
                ))}
              </ul>
            </div>
          ) : null}
          <div className="form-field">
            <label htmlFor="revision-review-notes">Revision notes (optional)</label>
            <textarea
              id="revision-review-notes"
              className="planner-textarea"
              value={revisionNotes}
              maxLength={2_000}
              onChange={(event) => setRevisionNotes(event.target.value)}
              disabled={busy !== null}
            />
            <p className="field-hint">
              Notes are stored with this revision outcome in the append-only history.
            </p>
          </div>
          <p className="revision-note">
            Completing this revision changes only the revision schedule and history. It does not
            mark the syllabus topic as mastered or completed.
          </p>
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy !== null}>
              {busy === 'complete' ? 'Completing…' : 'Complete revision'}
            </button>
          </div>
        </form>
      )}

      {!readOnly && schedule && mode === 'rescheduling' && (
        <form className="revision-inline-form" onSubmit={handleReschedule} noValidate>
          <DateField
            id="revision-reschedule-date"
            label="New next revision date"
            value={rescheduleDate}
            onChange={(value) => {
              setRescheduleDate(value);
              setDateError(null);
            }}
            hint="Manual rescheduling does not change the interval, mastery estimate, or history."
            error={dateError ?? undefined}
            disabled={busy !== null}
          />
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy !== null}>
              {busy === 'reschedule' ? 'Saving…' : 'Save new date'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => setMode('idle')}
              disabled={busy !== null}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {!readOnly && schedule && mode === 'deferring' && (
        <form className="revision-inline-form" onSubmit={handleDefer} noValidate>
          <p className="revision-note">
            Deferral is an explicit audit event. The topic becomes unavailable until the return date
            and is never counted as completed or mastered.
          </p>
          <DateField
            id="revision-defer-date"
            label="Return date"
            value={deferDate}
            onChange={(value) => {
              setDeferDate(value);
              setDateError(null);
            }}
            hint="Must be later than the current next revision date."
            error={dateError ?? undefined}
            disabled={busy !== null}
          />
          <div className="planner-form-row">
            <div className="form-field">
              <label htmlFor="revision-defer-reason">Reason</label>
              <select
                id="revision-defer-reason"
                value={deferReason}
                onChange={(event) => setDeferReason(event.target.value as RevisionDeferralReason)}
                disabled={busy !== null}
              >
                {Object.entries(DEFERRAL_REASONS).map(([value, label]) => (
                  <option value={value} key={value}>
                    {label}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label htmlFor="revision-defer-notes">Notes (optional)</label>
              <input
                id="revision-defer-notes"
                value={deferNotes}
                maxLength={2000}
                onChange={(event) => setDeferNotes(event.target.value)}
                disabled={busy !== null}
              />
            </div>
          </div>
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy !== null}>
              {busy === 'defer' ? 'Deferring…' : 'Defer revision'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => setMode('idle')}
              disabled={busy !== null}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {!readOnly && schedule && mode === 'overriding' && (
        <form className="revision-inline-form" onSubmit={handleOverride} noValidate>
          <p className="revision-note">
            Overriding keeps the original deferral in the append-only audit trail and adds an
            explicit override event. The return date must be earlier than the current return date
            and cannot be before today.
          </p>
          <DateField
            id="revision-override-date"
            label="Bring the revision back on"
            value={overrideDate}
            onChange={(value) => {
              setOverrideDate(value);
              setDateError(null);
            }}
            hint={`Current return date: ${
              schedule.deferredUntil ? dateLabel(schedule.deferredUntil) : 'not deferred'
            }.`}
            error={dateError ?? undefined}
            disabled={busy !== null}
          />
          <div className="planner-form-row">
            <div className="form-field">
              <label htmlFor="revision-override-reason">Reason</label>
              <select
                id="revision-override-reason"
                value={overrideReason}
                onChange={(event) =>
                  setOverrideReason(event.target.value as RevisionDeferralReason)
                }
                disabled={busy !== null}
              >
                {Object.entries(DEFERRAL_REASONS).map(([value, label]) => (
                  <option value={value} key={value}>
                    {label}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label htmlFor="revision-override-notes">Notes (optional)</label>
              <input
                id="revision-override-notes"
                value={overrideNotes}
                maxLength={2000}
                onChange={(event) => setOverrideNotes(event.target.value)}
                disabled={busy !== null}
              />
            </div>
          </div>
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy !== null}>
              {busy === 'override' ? 'Overriding…' : 'Override deferral'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => setMode('idle')}
              disabled={busy !== null}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {!readOnly && schedule && mode === 'planning' && (
        <section className="revision-plan-editor" aria-labelledby="revision-plan-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Study planner</p>
              <h3 id="revision-plan-heading">Plan a revision study block</h3>
            </div>
            <button
              type="button"
              className="secondary-button"
              onClick={() => setMode('idle')}
              disabled={busy !== null}
            >
              Cancel planning
            </button>
          </div>
          <p className="revision-note">
            A planned block is a calendar intention. It does not count as study time unless you
            explicitly record a separate study session.
          </p>
          <PlannedBlockEditor
            key={`revision-plan-${state.topic.id}-${schedule.nextRevisionDate}`}
            block={null}
            topics={plannerTopics}
            defaultDate={schedule.nextRevisionDate}
            initialTitle={`Revision · ${state.topic.code}`}
            initialTopicId={state.topic.id}
            initialNotes={`Revision block for ${state.topic.title}`}
            onSaved={(block) => {
              setMode('idle');
              setFeedback({
                tone: 'success',
                message: `Planned block “${block.title}” saved for ${dateLabel(
                  block.calendarDate,
                )}. Study time is unchanged.`,
              });
            }}
            onCancel={() => setMode('idle')}
          />
        </section>
      )}

      {!readOnly && schedule && (
        <form className="revision-notes-form" onSubmit={handleSaveNotes}>
          <div className="form-field">
            <label htmlFor="revision-topic-notes">Topic revision notes (optional)</label>
            <textarea
              id="revision-topic-notes"
              className="planner-textarea"
              value={notesDraft}
              maxLength={2_000}
              onChange={(event) => setNotesDraft(event.target.value)}
              disabled={busy !== null}
            />
            <p className="field-hint">
              {notesDraft.length}/2,000 characters. These notes belong to the schedule, not to an
              individual revision outcome.
            </p>
          </div>
          <div className="form-footer">
            <button type="submit" className="secondary-button" disabled={busy !== null}>
              {busy === 'notes' ? 'Saving…' : 'Save revision notes'}
            </button>
          </div>
        </form>
      )}

      <section className="revision-history" aria-labelledby="revision-history-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Append-only record</p>
            <h3 id="revision-history-heading">Revision history</h3>
          </div>
        </div>
        {historyLoading && (
          <p className="revision-note" role="status">
            Loading revision history…
          </p>
        )}
        {!historyLoading && historyError && (
          <div className="revision-history-error" role="alert">
            <p>{historyError}</p>
            <button
              type="button"
              className="secondary-button"
              onClick={() => {
                setHistoryLoading(true);
                setHistoryKey((key) => key + 1);
              }}
            >
              Retry history
            </button>
          </div>
        )}
        {!historyLoading && !historyError && history.length === 0 && (
          <p className="revision-note">No completed revisions recorded for this topic yet.</p>
        )}
        {!historyLoading && !historyError && history.length > 0 && (
          <div className="revision-history-table-wrap">
            <table className="revision-history-table">
              <thead>
                <tr>
                  <th scope="col">Revised</th>
                  <th scope="col">Was due</th>
                  <th scope="col">Outcome</th>
                  <th scope="col">Interval</th>
                  <th scope="col">Timing</th>
                  <th scope="col">Actual time</th>
                  <th scope="col">Mastery</th>
                  <th scope="col">Notes</th>
                </tr>
              </thead>
              <tbody>
                {history.map((entry) => (
                  <tr key={entry.id}>
                    <th scope="row">{dateLabel(entry.revisionDate)}</th>
                    <td>{dateLabel(entry.scheduledForDate)}</td>
                    <td>
                      <span className={`revision-outcome revision-outcome-${entry.outcome}`}>
                        {OUTCOME_LABELS[entry.outcome]}
                      </span>
                    </td>
                    <td>
                      {entry.intervalBeforeDays} → {entry.intervalAfterDays} days
                    </td>
                    <td>
                      {entry.timingDays === null
                        ? '—'
                        : `${entry.timingDays > 0 ? '+' : ''}${entry.timingDays} day${
                            Math.abs(entry.timingDays) === 1 ? '' : 's'
                          }`}
                    </td>
                    <td>
                      {entry.actualSeconds === null
                        ? '—'
                        : `${(entry.actualSeconds / 60).toFixed(1)} min`}
                    </td>
                    <td>
                      {entry.masteryBefore.toFixed(1)}% → {entry.masteryAfter.toFixed(1)}%
                    </td>
                    <td>{entry.notes || '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </section>
  );
}
