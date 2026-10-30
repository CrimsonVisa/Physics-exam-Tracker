import { type FormEvent, useEffect, useState } from 'react';
import { DateField } from '../components/DateField';
import { toCommandError } from '../lib/commandError';
import { addDaysDateOnly, formatDateLong, parseDateOnly } from '../lib/dateOnly';
import {
  createManualSession,
  deleteStudySession,
  getSessionTopics,
  getStudySessions,
  getStudyTotals,
  updateStudySession,
  type SessionTopic,
  type StudySession,
  type StudySessionInput,
  type StudyTotals,
} from '../lib/sessionsApi';
import {
  durationSecondsFromParts,
  formatStudyDuration,
  formatTimeOfDay,
  splitDurationSeconds,
} from '../lib/studyTime';

type SessionForm = {
  topicId: string;
  sessionDate: string;
  minutes: string;
  seconds: string;
  notes: string;
};

type SessionFormField = 'topicId' | 'sessionDate' | 'durationSeconds' | 'notes';

type AppliedFilter = {
  fromDate: string;
  toDate: string;
  topicId: string;
};

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

const MAX_NOTES_CHARS = 2000;
const MAX_DURATION_SECONDS = 86_400;

function defaultFilter(today: string): AppliedFilter {
  return {
    fromDate: addDaysDateOnly(today, -29) ?? today,
    toDate: today,
    topicId: '',
  };
}

function parseWholeNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }

  return Number(trimmed);
}

function blankForm(today: string): SessionForm {
  return {
    topicId: '',
    sessionDate: today,
    minutes: '25',
    seconds: '0',
    notes: '',
  };
}

function formFromSession(session: StudySession): SessionForm {
  const duration = splitDurationSeconds(session.durationSeconds);

  return {
    topicId: session.topic ? String(session.topic.id) : '',
    sessionDate: session.sessionDate,
    minutes: String(duration.minutes),
    seconds: String(duration.seconds),
    notes: session.notes,
  };
}

function validateForm(form: SessionForm): {
  input: StudySessionInput | null;
  errors: Partial<Record<SessionFormField, string>>;
} {
  const errors: Partial<Record<SessionFormField, string>> = {};
  if (!parseDateOnly(form.sessionDate)) {
    errors.sessionDate = 'Choose a real calendar date.';
  }

  const minutes = parseWholeNumber(form.minutes);
  const seconds = parseWholeNumber(form.seconds);

  if (minutes === null || minutes > 1440) {
    errors.durationSeconds = 'Enter whole minutes from 0 to 1440.';
  } else if (seconds === null || seconds > 59) {
    errors.durationSeconds = 'Enter whole seconds from 0 to 59.';
  } else {
    const total = durationSecondsFromParts(minutes, seconds);
    if (total < 1 || total > MAX_DURATION_SECONDS) {
      errors.durationSeconds = 'Duration must be between 1 second and 24 hours.';
    }
  }

  if (form.notes.length > MAX_NOTES_CHARS) {
    errors.notes = `Notes must be ${MAX_NOTES_CHARS} characters or fewer.`;
  }

  let topicId: number | null = null;
  if (form.topicId) {
    const parsed = Number(form.topicId);
    if (!Number.isSafeInteger(parsed) || parsed <= 0) {
      errors.topicId = 'Choose a topic from the list.';
    } else {
      topicId = parsed;
    }
  }

  if (Object.keys(errors).length > 0) {
    return { input: null, errors };
  }

  return {
    input: {
      topicId,
      sessionDate: form.sessionDate,
      durationSeconds: durationSecondsFromParts(minutes!, seconds!),
      notes: form.notes,
    },
    errors,
  };
}

function groupByDate(sessions: StudySession[]): { date: string; sessions: StudySession[] }[] {
  const groups = new Map<string, StudySession[]>();

  for (const session of sessions) {
    const list = groups.get(session.sessionDate) ?? [];
    list.push(session);
    groups.set(session.sessionDate, list);
  }

  return [...groups.entries()].map(([date, items]) => ({ date, sessions: items }));
}

function SessionRow({
  session,
  pendingDelete,
  busy,
  onEdit,
  onRequestDelete,
  onCancelDelete,
  onConfirmDelete,
}: {
  session: StudySession;
  pendingDelete: boolean;
  busy: boolean;
  onEdit: () => void;
  onRequestDelete: () => void;
  onCancelDelete: () => void;
  onConfirmDelete: () => void;
}) {
  const start = formatTimeOfDay(session.startedAt);
  const end = formatTimeOfDay(session.endedAt);

  return (
    <article className="session-row">
      <div className="session-row-main">
        <div className="session-row-heading">
          <p className="session-topic">
            {session.topic ? (
              <>
                <span className="session-topic-code">{session.topic.code}</span>
                {session.topic.title}
              </>
            ) : (
              'General study'
            )}
          </p>
          <span className={`session-source session-source-${session.source}`}>
            {session.source === 'timer' ? 'Timer' : 'Manual'}
          </span>
          {session.topic?.archived && <span className="session-archived">Archived topic</span>}
        </div>

        <p className="session-meta">
          <strong>{formatStudyDuration(session.durationSeconds)}</strong>
          {start && end && (
            <>
              {' '}
              · {start}–{end}
            </>
          )}
          {!start && ' · no clock time recorded'}
        </p>

        {session.notes && <p className="session-notes">{session.notes}</p>}
      </div>

      <div className="session-row-actions">
        <button type="button" className="secondary-button" onClick={onEdit} disabled={busy}>
          Edit
        </button>
        <button type="button" className="danger-button" onClick={onRequestDelete} disabled={busy}>
          Delete
        </button>
      </div>

      {pendingDelete && (
        <div className="session-delete-confirm" role="alertdialog" aria-labelledby="delete-title">
          <p className="session-delete-title" id="delete-title">
            Delete this session permanently?
          </p>
          <p>Daily and weekly totals will be recalculated. The syllabus topic is not affected.</p>
          <div className="form-footer">
            <button
              type="button"
              className="danger-button"
              onClick={onConfirmDelete}
              disabled={busy}
            >
              {busy ? 'Deleting…' : 'Confirm delete'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={onCancelDelete}
              disabled={busy}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </article>
  );
}

export default function SessionsScreen({ today }: { today: string }) {
  const initialFilter = defaultFilter(today);
  const [filterForm, setFilterForm] = useState<AppliedFilter>(initialFilter);
  const [appliedFilter, setAppliedFilter] = useState<AppliedFilter>(initialFilter);
  const [filterError, setFilterError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<StudySession[]>([]);
  const [totals, setTotals] = useState<StudyTotals | null>(null);
  const [topics, setTopics] = useState<SessionTopic[]>([]);
  const [topicsError, setTopicsError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [form, setForm] = useState<SessionForm>(() => blankForm(today));
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingSource, setEditingSource] = useState<StudySession['source'] | null>(null);
  const [formErrors, setFormErrors] = useState<Partial<Record<SessionFormField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [formFeedback, setFormFeedback] = useState<Feedback>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    getSessionTopics()
      .then((loadedTopics) => {
        if (!cancelled) {
          setTopics(loadedTopics);
          setTopicsError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setTopicsError(toCommandError(error).message);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  useEffect(() => {
    let cancelled = false;

    const topicId = appliedFilter.topicId ? Number(appliedFilter.topicId) : null;

    Promise.all([
      getStudySessions({
        fromDate: appliedFilter.fromDate,
        toDate: appliedFilter.toDate,
        ...(topicId ? { topicId } : {}),
      }),
      getStudyTotals(appliedFilter.fromDate, appliedFilter.toDate, topicId),
    ])
      .then(([loadedSessions, loadedTotals]) => {
        if (cancelled) {
          return;
        }

        setSessions(loadedSessions);
        setTotals(loadedTotals);
        setLoadError(null);
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
  }, [appliedFilter, reloadKey]);

  function applyFilter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFilterError(null);

    const from = parseDateOnly(filterForm.fromDate);
    const to = parseDateOnly(filterForm.toDate);

    if (!from || !to) {
      setFilterError('Choose real calendar dates for both ends of the range.');
      return;
    }

    if (filterForm.fromDate > filterForm.toDate) {
      setFilterError('The start date cannot be after the end date.');
      return;
    }

    setIsLoading(true);
    setLoadError(null);
    setAppliedFilter({ ...filterForm });
  }

  function resetFilter() {
    const next = defaultFilter(today);
    setFilterForm(next);
    setIsLoading(true);
    setLoadError(null);
    setAppliedFilter(next);
    setFilterError(null);
  }

  function resetForm() {
    setForm(blankForm(today));
    setEditingId(null);
    setEditingSource(null);
    setFormErrors({});
  }

  function handleEdit(session: StudySession) {
    setForm(formFromSession(session));
    setEditingId(session.id);
    setEditingSource(session.source);
    setFormErrors({});
    setFormFeedback(null);
    setPendingDeleteId(null);
  }

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSaving) {
      return;
    }

    setFormFeedback(null);
    const validation = validateForm(form);
    setFormErrors(validation.errors);

    if (!validation.input) {
      return;
    }

    setIsSaving(true);

    try {
      if (editingId !== null) {
        await updateStudySession(editingId, validation.input);
        setFormFeedback({
          tone: 'success',
          message: 'Study session updated. Daily and weekly totals were recalculated.',
        });
      } else {
        await createManualSession(validation.input);
        setFormFeedback({ tone: 'success', message: 'Manual study session saved.' });
      }

      resetForm();
      setReloadKey((key) => key + 1);
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as SessionFormField | undefined;

      if (field && ['topicId', 'sessionDate', 'durationSeconds', 'notes'].includes(field)) {
        setFormErrors({ [field]: commandError.message });
      }

      setFormFeedback({ tone: 'error', message: commandError.message });
    } finally {
      setIsSaving(false);
    }
  }

  async function handleDelete(sessionId: number) {
    if (deletingId !== null) {
      return;
    }

    setDeletingId(sessionId);
    setFormFeedback(null);

    try {
      await deleteStudySession(sessionId);
      setPendingDeleteId(null);
      if (editingId === sessionId) {
        resetForm();
      }
      setFormFeedback({
        tone: 'success',
        message: 'Study session deleted. Daily and weekly totals were recalculated.',
      });
      setReloadKey((key) => key + 1);
    } catch (error: unknown) {
      setFormFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setDeletingId(null);
    }
  }

  function handleFormField(field: keyof SessionForm, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    const errorField: SessionFormField | null =
      field === 'minutes' || field === 'seconds'
        ? 'durationSeconds'
        : field === 'topicId'
          ? 'topicId'
          : field === 'sessionDate'
            ? 'sessionDate'
            : 'notes';

    setFormErrors((current) => {
      if (!current[errorField]) {
        return current;
      }

      const next = { ...current };
      delete next[errorField];
      return next;
    });
    setFormFeedback(null);
  }

  const groupedSessions = groupByDate(sessions);
  const selectedTopic = appliedFilter.topicId
    ? topics.find((topic) => String(topic.id) === appliedFilter.topicId)
    : null;
  const formTopics = topics.filter(
    (topic) => !topic.archived || (editingId !== null && String(topic.id) === form.topicId),
  );

  return (
    <div className="page-container sessions-page">
      <header className="page-header">
        <p className="eyebrow">Study sessions</p>
        <h1>Study-time history</h1>
        <p className="page-lede">
          Timer and manual focus sessions live here. Breaks are never recorded. A timer session is
          assigned to the local date when its focus block started, even when it crosses midnight.
        </p>
      </header>

      <section className="sessions-filter-card" aria-labelledby="session-filter-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">History filters</p>
            <h2 id="session-filter-heading">Find study sessions</h2>
          </div>
          <button type="button" className="secondary-button" onClick={resetFilter}>
            Reset to last 30 days
          </button>
        </div>

        <form className="sessions-filter-form" onSubmit={applyFilter} noValidate>
          <DateField
            id="session-filter-from"
            label="From date"
            value={filterForm.fromDate}
            onChange={(value) => setFilterForm((current) => ({ ...current, fromDate: value }))}
            max={filterForm.toDate}
          />
          <DateField
            id="session-filter-to"
            label="To date"
            value={filterForm.toDate}
            onChange={(value) => setFilterForm((current) => ({ ...current, toDate: value }))}
            min={filterForm.fromDate}
          />
          <div className="form-field">
            <label htmlFor="session-filter-topic">Topic</label>
            <select
              id="session-filter-topic"
              value={filterForm.topicId}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, topicId: event.target.value }))
              }
            >
              <option value="">All topics</option>
              {topics.map((topic) => (
                <option value={topic.id} key={topic.id}>
                  {topic.code} · {topic.title}
                  {topic.archived ? ' (archived)' : ''}
                </option>
              ))}
            </select>
            {topicsError && (
              <p className="field-error" role="alert">
                Topic filters could not be loaded: {topicsError}
              </p>
            )}
          </div>
          <div className="sessions-filter-action">
            <button type="submit" className="primary-button">
              Apply filters
            </button>
          </div>
        </form>

        {filterError && (
          <p className="save-feedback save-feedback-error" role="alert">
            {filterError}
          </p>
        )}
      </section>

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading study sessions.</p>
          <p className="state-copy">
            Reading local sessions and calculating daily and weekly totals.
          </p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Study sessions could not be loaded</p>
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
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && totals && (
        <>
          <section className="session-totals" aria-labelledby="session-totals-heading">
            <div className="session-totals-heading">
              <div>
                <p className="eyebrow">Focus time only</p>
                <h2 id="session-totals-heading">Daily and weekly totals</h2>
              </div>
              {selectedTopic && (
                <p className="session-filter-note">
                  Filtered to {selectedTopic.code} · {selectedTopic.title}
                </p>
              )}
            </div>

            <div className="session-total-cards">
              <div className="session-total-card">
                <p className="session-total-label">Selected range</p>
                <p className="session-total-value">{formatStudyDuration(totals.rangeSeconds)}</p>
                <p className="session-total-meta">
                  {totals.rangeSessionCount}{' '}
                  {totals.rangeSessionCount === 1 ? 'session' : 'sessions'}
                </p>
              </div>
              <div className="session-total-card">
                <p className="session-total-label">Today</p>
                <p className="session-total-value">{formatStudyDuration(totals.todaySeconds)}</p>
                <p className="session-total-meta">{formatDateLong(totals.todayDate)}</p>
              </div>
              <div className="session-total-card">
                <p className="session-total-label">This week</p>
                <p className="session-total-value">{formatStudyDuration(totals.weekSeconds)}</p>
                <p className="session-total-meta">
                  {formatDateLong(totals.weekStart)} – {formatDateLong(totals.weekEnd)}
                </p>
              </div>
            </div>

            <div className="session-total-breakdowns">
              <details className="session-breakdown" open>
                <summary>Daily totals ({totals.daily.length})</summary>
                {totals.daily.length === 0 ? (
                  <p className="session-breakdown-empty">No focus time in this range.</p>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th scope="col">Date</th>
                        <th scope="col">Sessions</th>
                        <th scope="col">Focus time</th>
                      </tr>
                    </thead>
                    <tbody>
                      {totals.daily.map((day) => (
                        <tr key={day.date}>
                          <th scope="row">{formatDateLong(day.date)}</th>
                          <td>{day.sessionCount}</td>
                          <td>{formatStudyDuration(day.seconds)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </details>

              <details className="session-breakdown" open>
                <summary>Weekly totals ({totals.weekly.length})</summary>
                {totals.weekly.length === 0 ? (
                  <p className="session-breakdown-empty">No weekly focus time in this range.</p>
                ) : (
                  <table>
                    <thead>
                      <tr>
                        <th scope="col">Week</th>
                        <th scope="col">Sessions</th>
                        <th scope="col">Focus time</th>
                      </tr>
                    </thead>
                    <tbody>
                      {totals.weekly.map((week) => (
                        <tr key={week.weekStart}>
                          <th scope="row">
                            {formatDateLong(week.weekStart)} – {formatDateLong(week.weekEnd)}
                          </th>
                          <td>{week.sessionCount}</td>
                          <td>{formatStudyDuration(week.seconds)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </details>
            </div>
          </section>

          <div className="sessions-workspace">
            <section className="session-history-card" aria-labelledby="session-history-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Local history</p>
                  <h2 id="session-history-heading">Sessions ({sessions.length})</h2>
                </div>
              </div>

              {formFeedback && (
                <p
                  className={`save-feedback save-feedback-${formFeedback.tone}`}
                  role={formFeedback.tone === 'error' ? 'alert' : 'status'}
                >
                  {formFeedback.message}
                </p>
              )}

              {sessions.length === 0 ? (
                <div className="session-empty">
                  <p className="session-empty-title">No study sessions in this range</p>
                  <p>
                    Start a focus timer on the Timer page, or add a manual session using the form.
                    Completed timer sessions appear here automatically.
                  </p>
                </div>
              ) : (
                <div className="session-days">
                  {groupedSessions.map((group) => (
                    <section className="session-day-group" key={group.date}>
                      <div className="session-day-heading">
                        <h3>{formatDateLong(group.date)}</h3>
                        <p>
                          {formatStudyDuration(
                            group.sessions.reduce(
                              (total, session) => total + session.durationSeconds,
                              0,
                            ),
                          )}
                        </p>
                      </div>
                      {group.sessions.map((session) => (
                        <SessionRow
                          key={session.id}
                          session={session}
                          pendingDelete={pendingDeleteId === session.id}
                          busy={deletingId === session.id}
                          onEdit={() => handleEdit(session)}
                          onRequestDelete={() => {
                            setPendingDeleteId(session.id);
                            setFormFeedback(null);
                          }}
                          onCancelDelete={() => setPendingDeleteId(null)}
                          onConfirmDelete={() => void handleDelete(session.id)}
                        />
                      ))}
                    </section>
                  ))}
                </div>
              )}
            </section>

            <section className="session-editor-card" aria-labelledby="session-editor-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">{editingId === null ? 'Manual entry' : 'Editing'}</p>
                  <h2 id="session-editor-heading">
                    {editingId === null ? 'Add a study session' : 'Edit study session'}
                  </h2>
                </div>
                {editingSource && (
                  <span className={`session-source session-source-${editingSource}`}>
                    {editingSource === 'timer' ? 'Timer session' : 'Manual session'}
                  </span>
                )}
              </div>

              <p className="settings-lede">
                {editingId === null
                  ? 'Use manual entry for study that happened away from the timer. Only focus time belongs here.'
                  : editingSource === 'timer'
                    ? 'Timer start and end timestamps are kept. Changing the date or duration recalculates study totals.'
                    : 'Editing the date or duration recalculates the affected daily and weekly totals.'}
              </p>

              <form className="session-form" onSubmit={handleSave} noValidate>
                <div className="session-form-row">
                  <DateField
                    id="session-date"
                    label="Session date"
                    value={form.sessionDate}
                    onChange={(value) => handleFormField('sessionDate', value)}
                    hint="The local calendar date this focus time belongs to."
                    error={formErrors.sessionDate}
                    disabled={isSaving}
                  />

                  <div className="form-field">
                    <label htmlFor="session-topic">Syllabus topic (optional)</label>
                    <select
                      id="session-topic"
                      value={form.topicId}
                      onChange={(event) => handleFormField('topicId', event.target.value)}
                      disabled={isSaving}
                    >
                      <option value="">No specific topic</option>
                      {formTopics.map((topic) => (
                        <option value={topic.id} key={topic.id}>
                          {topic.code} · {topic.title}
                          {topic.archived ? ' (archived)' : ''}
                        </option>
                      ))}
                    </select>
                    {formErrors.topicId && (
                      <p className="field-error" role="alert">
                        {formErrors.topicId}
                      </p>
                    )}
                  </div>
                </div>

                <fieldset
                  className={`session-duration${formErrors.durationSeconds ? ' form-field-invalid' : ''}`}
                >
                  <legend>Duration</legend>
                  <div className="session-duration-inputs">
                    <div className="form-field">
                      <label htmlFor="session-minutes">Minutes</label>
                      <input
                        id="session-minutes"
                        type="number"
                        min={0}
                        max={1440}
                        step={1}
                        value={form.minutes}
                        onChange={(event) => handleFormField('minutes', event.target.value)}
                        aria-invalid={formErrors.durationSeconds ? true : undefined}
                        aria-describedby={
                          formErrors.durationSeconds
                            ? 'session-duration-hint session-duration-error'
                            : 'session-duration-hint'
                        }
                        disabled={isSaving}
                      />
                    </div>
                    <div className="form-field">
                      <label htmlFor="session-seconds">Seconds</label>
                      <input
                        id="session-seconds"
                        type="number"
                        min={0}
                        max={59}
                        step={1}
                        value={form.seconds}
                        onChange={(event) => handleFormField('seconds', event.target.value)}
                        aria-invalid={formErrors.durationSeconds ? true : undefined}
                        aria-describedby={
                          formErrors.durationSeconds
                            ? 'session-duration-hint session-duration-error'
                            : 'session-duration-hint'
                        }
                        disabled={isSaving}
                      />
                    </div>
                  </div>
                  <p className="field-hint" id="session-duration-hint">
                    1 second to 24 hours. Breaks should not be entered as study sessions.
                  </p>
                  {formErrors.durationSeconds && (
                    <p className="field-error" id="session-duration-error" role="alert">
                      {formErrors.durationSeconds}
                    </p>
                  )}
                </fieldset>

                <div className={`form-field${formErrors.notes ? ' form-field-invalid' : ''}`}>
                  <label htmlFor="session-notes">Notes (optional)</label>
                  <textarea
                    id="session-notes"
                    value={form.notes}
                    maxLength={MAX_NOTES_CHARS}
                    onChange={(event) => handleFormField('notes', event.target.value)}
                    aria-invalid={formErrors.notes ? true : undefined}
                    aria-describedby={
                      formErrors.notes ? 'session-notes-error' : 'session-notes-hint'
                    }
                    disabled={isSaving}
                  />
                  <p className="field-hint" id="session-notes-hint">
                    {form.notes.length}/{MAX_NOTES_CHARS} characters.
                  </p>
                  {formErrors.notes && (
                    <p className="field-error" id="session-notes-error" role="alert">
                      {formErrors.notes}
                    </p>
                  )}
                </div>

                <div className="form-footer">
                  <button type="submit" className="primary-button" disabled={isSaving}>
                    {isSaving
                      ? 'Saving…'
                      : editingId === null
                        ? 'Save manual session'
                        : 'Save changes'}
                  </button>
                  {editingId !== null && (
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={resetForm}
                      disabled={isSaving}
                    >
                      Cancel edit
                    </button>
                  )}
                </div>
              </form>
            </section>
          </div>
        </>
      )}
    </div>
  );
}
