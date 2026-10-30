import { type FormEvent, useEffect, useState } from 'react';
import { parseDateOnly } from '../lib/dateOnly';
import { toCommandError } from '../lib/commandError';
import {
  createPlannedBlock,
  updatePlannedBlock,
  type PlannedBlock,
  type PlannedBlockInput,
  type PlannedBlockStatus,
  type PlannerTopic,
} from '../lib/plannerApi';
import { getStudySessions, type StudySession } from '../lib/sessionsApi';
import { formatStudyDuration, formatTimeOfDay } from '../lib/studyTime';
import { isValidTimeOfDay, plannedMinutes } from '../lib/timeOfDay';
import { DateField } from './DateField';

const MAX_TITLE_CHARS = 200;
const MAX_NOTES_CHARS = 2_000;

type BlockForm = {
  title: string;
  calendarDate: string;
  startTime: string;
  endTime: string;
  topicId: string;
  status: PlannedBlockStatus;
  notes: string;
  linkedSessionId: string;
};

type BlockFormField =
  | 'title'
  | 'calendarDate'
  | 'startTime'
  | 'endTime'
  | 'topicId'
  | 'status'
  | 'notes'
  | 'linkedSessionId';

type PlannedBlockEditorProps = {
  block: PlannedBlock | null;
  topics: PlannerTopic[];
  defaultDate: string;
  onSaved: (block: PlannedBlock, wasEditing: boolean) => void;
  onCancel: () => void;
};

function formFromBlock(block: PlannedBlock | null, defaultDate: string): BlockForm {
  if (!block) {
    return {
      title: '',
      calendarDate: defaultDate,
      startTime: '09:00',
      endTime: '10:30',
      topicId: '',
      status: 'planned',
      notes: '',
      linkedSessionId: '',
    };
  }

  return {
    title: block.title,
    calendarDate: block.calendarDate,
    startTime: block.startTime,
    endTime: block.endTime,
    topicId: block.topic ? String(block.topic.id) : '',
    status: block.status,
    notes: block.notes,
    linkedSessionId: block.linkedSessionId ? String(block.linkedSessionId) : '',
  };
}

function validateBlockForm(form: BlockForm): {
  input: PlannedBlockInput | null;
  errors: Partial<Record<BlockFormField, string>>;
} {
  const errors: Partial<Record<BlockFormField, string>> = {};
  const title = form.title.trim();

  if (!title) {
    errors.title = 'Enter a planned-block title.';
  } else if (title.length > MAX_TITLE_CHARS) {
    errors.title = `Block title must be ${MAX_TITLE_CHARS} characters or fewer.`;
  }

  if (!parseDateOnly(form.calendarDate)) {
    errors.calendarDate = 'Choose a real calendar date.';
  }

  if (!isValidTimeOfDay(form.startTime)) {
    errors.startTime = 'Enter a 24-hour start time in HH:MM format.';
  }

  if (!isValidTimeOfDay(form.endTime)) {
    errors.endTime = 'Enter a 24-hour end time in HH:MM format.';
  } else if (
    isValidTimeOfDay(form.startTime) &&
    (plannedMinutes(form.startTime, form.endTime) ?? 0) <= 0
  ) {
    errors.endTime = 'The end time must be later than the start time on the same date.';
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

  let linkedSessionId: number | null = null;
  if (form.status === 'completed' && form.linkedSessionId) {
    const parsed = Number(form.linkedSessionId);
    if (!Number.isSafeInteger(parsed) || parsed <= 0) {
      errors.linkedSessionId = 'Choose a study session from this date.';
    } else {
      linkedSessionId = parsed;
    }
  }

  if (Object.keys(errors).length > 0) {
    return { input: null, errors };
  }

  return {
    errors,
    input: {
      title,
      topicId,
      calendarDate: form.calendarDate,
      startTime: form.startTime,
      endTime: form.endTime,
      status: form.status,
      notes: form.notes,
      linkedSessionId,
    },
  };
}

function sessionOptionLabel(session: StudySession): string {
  const time = formatTimeOfDay(session.startedAt);
  const topic = session.topic ? `${session.topic.code} · ${session.topic.title}` : 'General study';
  const notes = session.notes ? ` — ${session.notes}` : '';

  return `${time ? `${time} · ` : ''}${formatStudyDuration(session.durationSeconds)} · ${topic}${notes}`;
}

export function PlannedBlockEditor({
  block,
  topics,
  defaultDate,
  onSaved,
  onCancel,
}: PlannedBlockEditorProps) {
  const [form, setForm] = useState<BlockForm>(() => formFromBlock(block, defaultDate));
  const [errors, setErrors] = useState<Partial<Record<BlockFormField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [sessionResult, setSessionResult] = useState<{
    key: string;
    sessions: StudySession[];
    error: string | null;
  } | null>(null);

  const sessionKey =
    form.status === 'completed' && parseDateOnly(form.calendarDate) ? form.calendarDate : null;

  useEffect(() => {
    let cancelled = false;

    if (!sessionKey) {
      return () => {
        cancelled = true;
      };
    }

    getStudySessions({ fromDate: sessionKey, toDate: sessionKey })
      .then((loaded) => {
        if (!cancelled) {
          setSessionResult({ key: sessionKey, sessions: loaded, error: null });
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setSessionResult({
            key: sessionKey,
            sessions: [],
            error: toCommandError(error).message,
          });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [sessionKey]);

  const currentSessionResult = sessionResult?.key === sessionKey ? sessionResult : null;
  const sessions = currentSessionResult?.sessions ?? [];
  const sessionsError = currentSessionResult?.error ?? null;
  const isLoadingSessions = sessionKey !== null && currentSessionResult === null;

  function handleField(field: keyof BlockForm, value: string) {
    setForm((current) => {
      if (field === 'calendarDate') {
        return { ...current, calendarDate: value, linkedSessionId: '' };
      }

      if (field === 'status') {
        return {
          ...current,
          status: value as PlannedBlockStatus,
          linkedSessionId: value === 'completed' ? current.linkedSessionId : '',
        };
      }

      return { ...current, [field]: value };
    });

    const errorField = field as BlockFormField;
    setErrors((current) => {
      if (!current[errorField]) {
        return current;
      }

      const next = { ...current };
      delete next[errorField];
      return next;
    });
    setFeedback(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSaving) {
      return;
    }

    setFeedback(null);
    const validation = validateBlockForm(form);
    setErrors(validation.errors);

    if (!validation.input) {
      return;
    }

    setIsSaving(true);

    try {
      const saved =
        block === null
          ? await createPlannedBlock(validation.input)
          : await updatePlannedBlock(block.id, validation.input);
      onSaved(saved, block !== null);
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as BlockFormField | undefined;

      if (
        field &&
        [
          'title',
          'calendarDate',
          'startTime',
          'endTime',
          'topicId',
          'status',
          'notes',
          'linkedSessionId',
        ].includes(field)
      ) {
        setErrors({ [field]: commandError.message });
      }

      setFeedback(commandError.message);
    } finally {
      setIsSaving(false);
    }
  }

  const durationMinutes =
    isValidTimeOfDay(form.startTime) && isValidTimeOfDay(form.endTime)
      ? plannedMinutes(form.startTime, form.endTime)
      : null;
  const selectedSession = sessions.find((session) => String(session.id) === form.linkedSessionId);

  return (
    <form className="planner-form" onSubmit={handleSubmit} noValidate>
      <div className={`form-field${errors.title ? ' form-field-invalid' : ''}`}>
        <label htmlFor="block-title">Planned-block title</label>
        <input
          id="block-title"
          value={form.title}
          maxLength={MAX_TITLE_CHARS}
          onChange={(event) => handleField('title', event.target.value)}
          aria-invalid={errors.title ? true : undefined}
          aria-describedby={errors.title ? 'block-title-error' : undefined}
          disabled={isSaving}
          required
        />
        {errors.title && (
          <p className="field-error" id="block-title-error" role="alert">
            {errors.title}
          </p>
        )}
      </div>

      <DateField
        id="block-date"
        label="Calendar date"
        value={form.calendarDate}
        onChange={(value) => handleField('calendarDate', value)}
        hint="Planned blocks are manual commitments and never move with the exam date."
        error={errors.calendarDate}
        disabled={isSaving}
      />

      <div className="planner-form-row">
        <div className={`form-field${errors.startTime ? ' form-field-invalid' : ''}`}>
          <label htmlFor="block-start-time">Start time</label>
          <input
            id="block-start-time"
            type="time"
            value={form.startTime}
            onChange={(event) => handleField('startTime', event.target.value)}
            aria-invalid={errors.startTime ? true : undefined}
            disabled={isSaving}
            required
          />
          {errors.startTime && (
            <p className="field-error" role="alert">
              {errors.startTime}
            </p>
          )}
        </div>

        <div className={`form-field${errors.endTime ? ' form-field-invalid' : ''}`}>
          <label htmlFor="block-end-time">End time</label>
          <input
            id="block-end-time"
            type="time"
            value={form.endTime}
            onChange={(event) => handleField('endTime', event.target.value)}
            aria-invalid={errors.endTime ? true : undefined}
            disabled={isSaving}
            required
          />
          {errors.endTime && (
            <p className="field-error" role="alert">
              {errors.endTime}
            </p>
          )}
        </div>
      </div>

      <p className="field-hint">
        {durationMinutes === null
          ? 'Use strict 24-hour times; the end must be later than the start on the same date.'
          : `Planned duration: ${formatStudyDuration(durationMinutes * 60)}. This is not recorded study time.`}
      </p>

      <div className="planner-form-row">
        <div className={`form-field${errors.topicId ? ' form-field-invalid' : ''}`}>
          <label htmlFor="block-topic">Syllabus topic (optional)</label>
          <select
            id="block-topic"
            value={form.topicId}
            onChange={(event) => handleField('topicId', event.target.value)}
            disabled={isSaving}
          >
            <option value="">No specific topic</option>
            {topics.map((topic) => (
              <option value={topic.id} key={topic.id}>
                {topic.code} · {topic.title}
                {topic.archived ? ' (archived)' : ''}
              </option>
            ))}
          </select>
          {errors.topicId && (
            <p className="field-error" role="alert">
              {errors.topicId}
            </p>
          )}
        </div>

        <div className="form-field">
          <label htmlFor="block-status">Status</label>
          <select
            id="block-status"
            value={form.status}
            onChange={(event) => handleField('status', event.target.value)}
            disabled={isSaving}
          >
            <option value="planned">Planned</option>
            <option value="completed">Completed</option>
            <option value="cancelled">Cancelled</option>
          </select>
        </div>
      </div>

      <div className={`form-field${errors.notes ? ' form-field-invalid' : ''}`}>
        <label htmlFor="block-notes">Notes (optional)</label>
        <textarea
          id="block-notes"
          className="planner-textarea"
          value={form.notes}
          maxLength={MAX_NOTES_CHARS}
          onChange={(event) => handleField('notes', event.target.value)}
          aria-invalid={errors.notes ? true : undefined}
          aria-describedby={errors.notes ? 'block-notes-error' : 'block-notes-hint'}
          disabled={isSaving}
        />
        <p className="field-hint" id="block-notes-hint">
          {form.notes.length}/{MAX_NOTES_CHARS} characters.
        </p>
        {errors.notes && (
          <p className="field-error" id="block-notes-error" role="alert">
            {errors.notes}
          </p>
        )}
      </div>

      {form.status === 'completed' && (
        <div className={`form-field${errors.linkedSessionId ? ' form-field-invalid' : ''}`}>
          <label htmlFor="block-session">Link an actual study session (optional)</label>
          <select
            id="block-session"
            value={form.linkedSessionId}
            onChange={(event) => handleField('linkedSessionId', event.target.value)}
            aria-invalid={errors.linkedSessionId ? true : undefined}
            aria-describedby="block-session-hint"
            disabled={isSaving || isLoadingSessions}
          >
            <option value="">No linked session</option>
            {sessions.map((session) => (
              <option value={session.id} key={session.id}>
                {sessionOptionLabel(session)}
              </option>
            ))}
          </select>
          <p className="field-hint" id="block-session-hint">
            {isLoadingSessions
              ? 'Loading sessions recorded on this date…'
              : sessionsError
                ? `Sessions could not be loaded: ${sessionsError}`
                : sessions.length === 0
                  ? 'No study sessions are recorded on this date. Add one on the Sessions page first.'
                  : 'This link is created only when you save. It never creates or changes study time.'}
          </p>
          {selectedSession && (
            <p className="block-session-preview">
              Linked session: {formatStudyDuration(selectedSession.durationSeconds)} ·{' '}
              {selectedSession.topic ? selectedSession.topic.title : 'General study'}
            </p>
          )}
          {errors.linkedSessionId && (
            <p className="field-error" role="alert">
              {errors.linkedSessionId}
            </p>
          )}
        </div>
      )}

      <div className="form-footer">
        <button type="submit" className="primary-button" disabled={isSaving}>
          {isSaving ? 'Saving…' : block === null ? 'Add planned block' : 'Save planned block'}
        </button>
        <button type="button" className="secondary-button" onClick={onCancel} disabled={isSaving}>
          {block === null ? 'Clear' : 'Cancel edit'}
        </button>
      </div>

      {feedback && (
        <p className="save-feedback save-feedback-error" role="alert">
          {feedback}
        </p>
      )}
    </form>
  );
}
