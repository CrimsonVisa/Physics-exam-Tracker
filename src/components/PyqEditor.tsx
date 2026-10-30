import { type FormEvent, useState } from 'react';
import { DateField } from './DateField';
import { toCommandError } from '../lib/commandError';
import { parseDateOnly } from '../lib/dateOnly';
import {
  createPyqAttempt,
  updatePyqAttempt,
  PYQ_PARTS,
  PYQ_STATUSES,
  type PyqAttempt,
  type PyqInput,
  type PyqOptions,
  type PyqPart,
  type PyqStatus,
} from '../lib/pyqApi';
import { durationSecondsFromParts, splitDurationSeconds } from '../lib/studyTime';

const MAX_REFERENCE_CHARS = 200;
const MAX_SESSION_CHARS = 100;
const MAX_NOTES_CHARS = 2_000;

type PyqForm = {
  questionYear: string;
  examSession: string;
  part: string;
  topicId: string;
  questionReference: string;
  attemptDate: string;
  status: PyqStatus;
  marksObtained: string;
  maxMarks: string;
  minutes: string;
  seconds: string;
  notes: string;
  difficulty: string;
};

type PyqFormField =
  | 'questionYear'
  | 'examSession'
  | 'part'
  | 'topicId'
  | 'questionReference'
  | 'attemptDate'
  | 'status'
  | 'marksObtained'
  | 'maxMarks'
  | 'timeSpentSeconds'
  | 'notes'
  | 'difficulty';

type PyqEditorProps = {
  attempt: PyqAttempt | null;
  options: PyqOptions;
  today: string;
  onSaved: (attempt: PyqAttempt, wasEditing: boolean) => void;
  onCancel: () => void;
};

function formFromAttempt(attempt: PyqAttempt | null, today: string): PyqForm {
  if (!attempt) {
    return {
      questionYear: today.slice(0, 4),
      examSession: '',
      part: '',
      topicId: '',
      questionReference: '',
      attemptDate: today,
      status: 'attempted',
      marksObtained: '',
      maxMarks: '',
      minutes: '0',
      seconds: '0',
      notes: '',
      difficulty: '',
    };
  }

  const duration = splitDurationSeconds(attempt.timeSpentSeconds);

  return {
    questionYear: String(attempt.questionYear),
    examSession: attempt.examSession,
    part: attempt.part,
    topicId: String(attempt.topic.id),
    questionReference: attempt.questionReference,
    attemptDate: attempt.attemptDate,
    status: attempt.status,
    marksObtained: attempt.marksObtained === null ? '' : String(attempt.marksObtained),
    maxMarks: attempt.maxMarks === null ? '' : String(attempt.maxMarks),
    minutes: String(duration.minutes),
    seconds: String(duration.seconds),
    notes: attempt.notes,
    difficulty: attempt.difficulty === null ? '' : String(attempt.difficulty),
  };
}

function parseWholeNumber(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) {
    return null;
  }
  return Number(trimmed);
}

function parseOptionalDecimal(value: string): number | null | 'invalid' {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : 'invalid';
}

function validateForm(form: PyqForm): {
  input: PyqInput | null;
  errors: Partial<Record<PyqFormField, string>>;
} {
  const errors: Partial<Record<PyqFormField, string>> = {};
  const year = parseWholeNumber(form.questionYear);
  if (year === null || year < 1900 || year > 2100) {
    errors.questionYear = 'Enter a question year from 1900 to 2100.';
  }

  if (form.examSession.length > MAX_SESSION_CHARS) {
    errors.examSession = `Exam session must be ${MAX_SESSION_CHARS} characters or fewer.`;
  }

  if (!PYQ_PARTS.includes(form.part as PyqPart)) {
    errors.part = 'Choose Part A, B or C.';
  }

  const topicId = Number(form.topicId);
  if (!form.topicId || !Number.isSafeInteger(topicId) || topicId <= 0) {
    errors.topicId = 'Choose a syllabus topic.';
  }

  const reference = form.questionReference.trim();
  if (!reference) {
    errors.questionReference = 'Enter a question reference or identifier.';
  } else if (reference.length > MAX_REFERENCE_CHARS) {
    errors.questionReference = `Question reference must be ${MAX_REFERENCE_CHARS} characters or fewer.`;
  }

  if (!parseDateOnly(form.attemptDate)) {
    errors.attemptDate = 'Choose a real calendar date.';
  }

  if (!PYQ_STATUSES.includes(form.status)) {
    errors.status = 'Choose a valid status.';
  }

  const marks = parseOptionalDecimal(form.marksObtained);
  if (marks === 'invalid' || (marks !== null && (marks < -100_000 || marks > 100_000))) {
    errors.marksObtained = 'Enter marks between -100000 and 100000, or leave blank.';
  }
  const maxMarks = parseOptionalDecimal(form.maxMarks);
  if (maxMarks === 'invalid' || (maxMarks !== null && (maxMarks <= 0 || maxMarks > 100_000))) {
    errors.maxMarks = 'Enter maximum marks greater than 0 and at most 100000, or leave blank.';
  }
  if (typeof marks === 'number' && typeof maxMarks === 'number' && marks > maxMarks) {
    errors.marksObtained = 'Marks obtained cannot exceed maximum marks.';
  }

  const minutes = parseWholeNumber(form.minutes);
  const seconds = parseWholeNumber(form.seconds);
  if (minutes === null || minutes > 1440) {
    errors.timeSpentSeconds = 'Enter whole minutes from 0 to 1440.';
  } else if (seconds === null || seconds > 59) {
    errors.timeSpentSeconds = 'Enter whole seconds from 0 to 59.';
  } else if (durationSecondsFromParts(minutes, seconds) > 86_400) {
    errors.timeSpentSeconds = 'Time spent must be 24 hours or less.';
  }

  if (form.notes.length > MAX_NOTES_CHARS) {
    errors.notes = `Notes must be ${MAX_NOTES_CHARS} characters or fewer.`;
  }

  let difficulty: number | null = null;
  if (form.difficulty) {
    const parsed = Number(form.difficulty);
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 5) {
      errors.difficulty = 'Choose a difficulty from 1 to 5.';
    } else {
      difficulty = parsed;
    }
  }

  if (Object.keys(errors).length > 0) {
    return { input: null, errors };
  }

  return {
    errors,
    input: {
      questionYear: year!,
      examSession: form.examSession.trim(),
      part: form.part as PyqPart,
      topicId,
      questionReference: reference,
      attemptDate: form.attemptDate,
      status: form.status,
      marksObtained: typeof marks === 'number' ? marks : null,
      maxMarks: typeof maxMarks === 'number' ? maxMarks : null,
      timeSpentSeconds: durationSecondsFromParts(minutes!, seconds!),
      notes: form.notes,
      difficulty,
    },
  };
}

export function PyqEditor({ attempt, options, today, onSaved, onCancel }: PyqEditorProps) {
  const [form, setForm] = useState<PyqForm>(() => formFromAttempt(attempt, today));
  const [errors, setErrors] = useState<Partial<Record<PyqFormField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  function handleField(field: keyof PyqForm, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    const errorField: PyqFormField | null =
      field === 'questionYear'
        ? 'questionYear'
        : field === 'examSession'
          ? 'examSession'
          : field === 'part'
            ? 'part'
            : field === 'topicId'
              ? 'topicId'
              : field === 'questionReference'
                ? 'questionReference'
                : field === 'attemptDate'
                  ? 'attemptDate'
                  : field === 'status'
                    ? 'status'
                    : field === 'marksObtained'
                      ? 'marksObtained'
                      : field === 'maxMarks'
                        ? 'maxMarks'
                        : field === 'minutes' || field === 'seconds'
                          ? 'timeSpentSeconds'
                          : field === 'notes'
                            ? 'notes'
                            : field === 'difficulty'
                              ? 'difficulty'
                              : null;

    if (errorField) {
      setErrors((current) => {
        if (!current[errorField]) {
          return current;
        }
        const next = { ...current };
        delete next[errorField];
        return next;
      });
    }
    setFeedback(null);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSaving) {
      return;
    }

    setFeedback(null);
    const validation = validateForm(form);
    setErrors(validation.errors);

    if (!validation.input) {
      return;
    }

    setIsSaving(true);

    try {
      const saved =
        attempt === null
          ? await createPyqAttempt(validation.input)
          : await updatePyqAttempt(attempt.id, validation.input);
      onSaved(saved, attempt !== null);
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as PyqFormField | undefined;

      if (
        field &&
        [
          'questionYear',
          'examSession',
          'part',
          'topicId',
          'questionReference',
          'attemptDate',
          'status',
          'marksObtained',
          'maxMarks',
          'timeSpentSeconds',
          'notes',
          'difficulty',
        ].includes(field)
      ) {
        setErrors({ [field]: commandError.message });
      }
      setFeedback(commandError.message);
    } finally {
      setIsSaving(false);
    }
  }

  const topicOptions = options.topics.filter(
    (topic) => !topic.archived || (attempt !== null && String(topic.id) === form.topicId),
  );

  return (
    <form className="planner-form" onSubmit={handleSubmit} noValidate>
      <div className="planner-form-row">
        <div className={`form-field${errors.questionYear ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-year">Question year</label>
          <input
            id="pyq-year"
            type="number"
            min={1900}
            max={2100}
            step={1}
            value={form.questionYear}
            onChange={(event) => handleField('questionYear', event.target.value)}
            aria-invalid={errors.questionYear ? true : undefined}
            disabled={isSaving}
            required
          />
          {errors.questionYear && (
            <p className="field-error" role="alert">
              {errors.questionYear}
            </p>
          )}
        </div>

        <div className={`form-field${errors.examSession ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-session">Exam / session (optional)</label>
          <input
            id="pyq-session"
            value={form.examSession}
            maxLength={MAX_SESSION_CHARS}
            onChange={(event) => handleField('examSession', event.target.value)}
            placeholder="e.g. June 2024"
            aria-invalid={errors.examSession ? true : undefined}
            disabled={isSaving}
          />
          {errors.examSession && (
            <p className="field-error" role="alert">
              {errors.examSession}
            </p>
          )}
        </div>
      </div>

      <div className="planner-form-row">
        <div className={`form-field${errors.part ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-part">Part</label>
          <select
            id="pyq-part"
            value={form.part}
            onChange={(event) => handleField('part', event.target.value)}
            aria-invalid={errors.part ? true : undefined}
            disabled={isSaving}
            required
          >
            <option value="">Choose part</option>
            <option value="A">Part A</option>
            <option value="B">Part B</option>
            <option value="C">Part C</option>
          </select>
          {errors.part && (
            <p className="field-error" role="alert">
              {errors.part}
            </p>
          )}
        </div>

        <div className={`form-field${errors.status ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-status">Status</label>
          <select
            id="pyq-status"
            value={form.status}
            onChange={(event) => handleField('status', event.target.value)}
            aria-invalid={errors.status ? true : undefined}
            disabled={isSaving}
          >
            <option value="attempted">Attempted</option>
            <option value="correct">Correct</option>
            <option value="incorrect">Incorrect</option>
            <option value="skipped">Skipped</option>
          </select>
          {errors.status && (
            <p className="field-error" role="alert">
              {errors.status}
            </p>
          )}
        </div>
      </div>

      <div className={`form-field${errors.topicId ? ' form-field-invalid' : ''}`}>
        <label htmlFor="pyq-topic">Syllabus topic</label>
        <select
          id="pyq-topic"
          value={form.topicId}
          onChange={(event) => handleField('topicId', event.target.value)}
          aria-invalid={errors.topicId ? true : undefined}
          disabled={isSaving}
          required
        >
          <option value="">Choose topic</option>
          {topicOptions.map((topic) => (
            <option value={topic.id} key={topic.id}>
              {topic.code} · {topic.title} · {topic.sectionTitle}
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

      <div className={`form-field${errors.questionReference ? ' form-field-invalid' : ''}`}>
        <label htmlFor="pyq-reference">Question reference</label>
        <input
          id="pyq-reference"
          value={form.questionReference}
          maxLength={MAX_REFERENCE_CHARS}
          onChange={(event) => handleField('questionReference', event.target.value)}
          placeholder="e.g. Paper B Q42 or your own ID"
          aria-invalid={errors.questionReference ? true : undefined}
          disabled={isSaving}
          required
        />
        <p className="field-hint">
          Only your identifier is stored. Question text is not imported or invented.
        </p>
        {errors.questionReference && (
          <p className="field-error" role="alert">
            {errors.questionReference}
          </p>
        )}
      </div>

      <DateField
        id="pyq-attempt-date"
        label="Attempt date"
        value={form.attemptDate}
        onChange={(value) => handleField('attemptDate', value)}
        error={errors.attemptDate}
        disabled={isSaving}
      />

      <div className="planner-form-row">
        <div className={`form-field${errors.marksObtained ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-marks">Marks obtained (optional)</label>
          <input
            id="pyq-marks"
            type="number"
            step="0.01"
            value={form.marksObtained}
            onChange={(event) => handleField('marksObtained', event.target.value)}
            aria-invalid={errors.marksObtained ? true : undefined}
            disabled={isSaving}
          />
          {errors.marksObtained && (
            <p className="field-error" role="alert">
              {errors.marksObtained}
            </p>
          )}
        </div>

        <div className={`form-field${errors.maxMarks ? ' form-field-invalid' : ''}`}>
          <label htmlFor="pyq-max-marks">Maximum marks (optional)</label>
          <input
            id="pyq-max-marks"
            type="number"
            min={0}
            step="0.01"
            value={form.maxMarks}
            onChange={(event) => handleField('maxMarks', event.target.value)}
            aria-invalid={errors.maxMarks ? true : undefined}
            disabled={isSaving}
          />
          {errors.maxMarks && (
            <p className="field-error" role="alert">
              {errors.maxMarks}
            </p>
          )}
        </div>
      </div>

      <fieldset
        className={`session-duration${errors.timeSpentSeconds ? ' form-field-invalid' : ''}`}
      >
        <legend>Time spent</legend>
        <div className="session-duration-inputs">
          <div className="form-field">
            <label htmlFor="pyq-minutes">Minutes</label>
            <input
              id="pyq-minutes"
              type="number"
              min={0}
              max={1440}
              step={1}
              value={form.minutes}
              onChange={(event) => handleField('minutes', event.target.value)}
              aria-invalid={errors.timeSpentSeconds ? true : undefined}
              disabled={isSaving}
            />
          </div>
          <div className="form-field">
            <label htmlFor="pyq-seconds">Seconds</label>
            <input
              id="pyq-seconds"
              type="number"
              min={0}
              max={59}
              step={1}
              value={form.seconds}
              onChange={(event) => handleField('seconds', event.target.value)}
              aria-invalid={errors.timeSpentSeconds ? true : undefined}
              disabled={isSaving}
            />
          </div>
        </div>
        {errors.timeSpentSeconds && (
          <p className="field-error" role="alert">
            {errors.timeSpentSeconds}
          </p>
        )}
      </fieldset>

      <div className="form-field">
        <label htmlFor="pyq-difficulty">Difficulty (optional)</label>
        <select
          id="pyq-difficulty"
          value={form.difficulty}
          onChange={(event) => handleField('difficulty', event.target.value)}
          disabled={isSaving}
        >
          <option value="">Not rated</option>
          <option value="1">1 · Very easy</option>
          <option value="2">2 · Easy</option>
          <option value="3">3 · Moderate</option>
          <option value="4">4 · Hard</option>
          <option value="5">5 · Very hard</option>
        </select>
        {errors.difficulty && (
          <p className="field-error" role="alert">
            {errors.difficulty}
          </p>
        )}
      </div>

      <div className={`form-field${errors.notes ? ' form-field-invalid' : ''}`}>
        <label htmlFor="pyq-notes">Personal notes (optional)</label>
        <textarea
          id="pyq-notes"
          className="planner-textarea"
          value={form.notes}
          maxLength={MAX_NOTES_CHARS}
          onChange={(event) => handleField('notes', event.target.value)}
          aria-invalid={errors.notes ? true : undefined}
          disabled={isSaving}
        />
        <p className="field-hint">
          {form.notes.length}/{MAX_NOTES_CHARS} characters.
        </p>
        {errors.notes && (
          <p className="field-error" role="alert">
            {errors.notes}
          </p>
        )}
      </div>

      <div className="form-footer">
        <button type="submit" className="primary-button" disabled={isSaving}>
          {isSaving ? 'Saving…' : attempt === null ? 'Add PYQ attempt' : 'Save PYQ attempt'}
        </button>
        <button type="button" className="secondary-button" onClick={onCancel} disabled={isSaving}>
          {attempt === null ? 'Close' : 'Cancel edit'}
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
