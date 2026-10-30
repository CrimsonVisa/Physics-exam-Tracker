import { type FormEvent, useState } from 'react';
import { addDaysDateOnly, formatDateLong, parseDateOnly } from '../lib/dateOnly';
import { toCommandError } from '../lib/commandError';
import {
  createChecklistTask,
  updateChecklistTask,
  type ChecklistTask,
  type PlannerTopic,
  type ScheduleAnchor,
  type TaskInput,
  type TaskPriority,
} from '../lib/plannerApi';
import { DateField } from './DateField';

const MAX_TITLE_CHARS = 200;
const MAX_DESCRIPTION_CHARS = 2_000;
const MIN_OFFSET_DAYS = -3_650;
const MAX_OFFSET_DAYS = 3_650;

type TaskForm = {
  title: string;
  description: string;
  scheduleAnchor: ScheduleAnchor;
  scheduledDate: string;
  offsetDays: string;
  deadlineDate: string;
  topicId: string;
  priority: TaskPriority;
};

type TaskFormField =
  | 'title'
  | 'description'
  | 'scheduledDate'
  | 'scheduleOffsetDays'
  | 'deadlineDate'
  | 'topicId'
  | 'priority'
  | 'scheduleAnchor';

type TaskEditorProps = {
  task: ChecklistTask | null;
  topics: PlannerTopic[];
  defaultDate: string;
  examDate: string;
  onSaved: (task: ChecklistTask, wasEditing: boolean) => void;
  onCancel: () => void;
};

function formFromTask(task: ChecklistTask | null, defaultDate: string): TaskForm {
  if (!task) {
    return {
      title: '',
      description: '',
      scheduleAnchor: 'manual',
      scheduledDate: defaultDate,
      offsetDays: '-10',
      deadlineDate: '',
      topicId: '',
      priority: 'Normal',
    };
  }

  return {
    title: task.title,
    description: task.description,
    scheduleAnchor: task.scheduleAnchor,
    scheduledDate: task.scheduledDate,
    offsetDays: String(task.scheduleOffsetDays ?? -10),
    deadlineDate: task.deadlineDate ?? '',
    topicId: task.topic ? String(task.topic.id) : '',
    priority: task.priority,
  };
}

function validateTaskForm(
  form: TaskForm,
  examDate: string,
): { input: TaskInput | null; errors: Partial<Record<TaskFormField, string>> } {
  const errors: Partial<Record<TaskFormField, string>> = {};
  const title = form.title.trim();

  if (!title) {
    errors.title = 'Enter a task title.';
  } else if (title.length > MAX_TITLE_CHARS) {
    errors.title = `Task title must be ${MAX_TITLE_CHARS} characters or fewer.`;
  }

  if (form.description.length > MAX_DESCRIPTION_CHARS) {
    errors.description = `Description must be ${MAX_DESCRIPTION_CHARS} characters or fewer.`;
  }

  let offsetDays: number | null = null;
  if (form.scheduleAnchor === 'exam') {
    if (!/^-?\d+$/.test(form.offsetDays.trim())) {
      errors.scheduleOffsetDays = 'Enter a whole number of days from the exam date.';
    } else {
      offsetDays = Number(form.offsetDays.trim());
      if (offsetDays < MIN_OFFSET_DAYS || offsetDays > MAX_OFFSET_DAYS) {
        errors.scheduleOffsetDays = `Exam offset must be between ${MIN_OFFSET_DAYS} and ${MAX_OFFSET_DAYS} days.`;
      } else if (!parseDateOnly(examDate)) {
        errors.scheduleOffsetDays = 'Set a valid exam date before scheduling relative to it.';
      } else if (!addDaysDateOnly(examDate, offsetDays)) {
        errors.scheduleOffsetDays = 'That exam-relative date is outside the supported calendar.';
      }
    }
  } else if (!parseDateOnly(form.scheduledDate)) {
    errors.scheduledDate = 'Choose a real calendar date.';
  }

  if (form.deadlineDate && !parseDateOnly(form.deadlineDate)) {
    errors.deadlineDate = 'Choose a real calendar date or leave the deadline blank.';
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
    errors,
    input: {
      title,
      description: form.description,
      scheduledDate: form.scheduleAnchor === 'manual' ? form.scheduledDate : '',
      deadlineDate: form.deadlineDate || null,
      topicId,
      priority: form.priority,
      scheduleAnchor: form.scheduleAnchor,
      scheduleOffsetDays: form.scheduleAnchor === 'exam' ? offsetDays : null,
    },
  };
}

function formatCompletedAt(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return parsed.toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

export function TaskEditor({
  task,
  topics,
  defaultDate,
  examDate,
  onSaved,
  onCancel,
}: TaskEditorProps) {
  const [form, setForm] = useState<TaskForm>(() => formFromTask(task, defaultDate));
  const [errors, setErrors] = useState<Partial<Record<TaskFormField, string>>>({});
  const [isSaving, setIsSaving] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  function handleField(field: keyof TaskForm, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    const errorField: TaskFormField | null =
      field === 'title'
        ? 'title'
        : field === 'description'
          ? 'description'
          : field === 'scheduledDate'
            ? 'scheduledDate'
            : field === 'offsetDays'
              ? 'scheduleOffsetDays'
              : field === 'deadlineDate'
                ? 'deadlineDate'
                : field === 'topicId'
                  ? 'topicId'
                  : field === 'priority'
                    ? 'priority'
                    : 'scheduleAnchor';

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
    const validation = validateTaskForm(form, examDate);
    setErrors(validation.errors);

    if (!validation.input) {
      return;
    }

    setIsSaving(true);

    try {
      const saved =
        task === null
          ? await createChecklistTask(validation.input)
          : await updateChecklistTask(task.id, validation.input);
      onSaved(saved, task !== null);
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as TaskFormField | undefined;

      if (
        field &&
        [
          'title',
          'description',
          'scheduledDate',
          'scheduleOffsetDays',
          'deadlineDate',
          'topicId',
          'priority',
          'scheduleAnchor',
        ].includes(field)
      ) {
        setErrors({ [field]: commandError.message });
      }

      setFeedback(commandError.message);
    } finally {
      setIsSaving(false);
    }
  }

  const anchorPreview =
    form.scheduleAnchor === 'exam' && /^-?\d+$/.test(form.offsetDays.trim())
      ? addDaysDateOnly(examDate, Number(form.offsetDays.trim()))
      : null;

  return (
    <form className="planner-form" onSubmit={handleSubmit} noValidate>
      {task && (
        <div className="task-editor-status">
          <span className={`task-status task-status-${task.status}`}>
            {task.status === 'completed' ? 'Completed' : 'Open'}
          </span>
          {task.completedAt && (
            <span className="task-completed-at">
              Completed {formatCompletedAt(task.completedAt)}
            </span>
          )}
          <span className="task-editor-preserve">
            Editing fields never changes the completion timestamp.
          </span>
        </div>
      )}

      <div className={`form-field${errors.title ? ' form-field-invalid' : ''}`}>
        <label htmlFor="task-title">Task title</label>
        <input
          id="task-title"
          value={form.title}
          maxLength={MAX_TITLE_CHARS}
          onChange={(event) => handleField('title', event.target.value)}
          aria-invalid={errors.title ? true : undefined}
          aria-describedby={errors.title ? 'task-title-error' : undefined}
          disabled={isSaving}
          required
        />
        {errors.title && (
          <p className="field-error" id="task-title-error" role="alert">
            {errors.title}
          </p>
        )}
      </div>

      <div className="planner-form-row">
        <div className={`form-field${errors.description ? ' form-field-invalid' : ''}`}>
          <label htmlFor="task-description">Description (optional)</label>
          <textarea
            id="task-description"
            className="planner-textarea"
            value={form.description}
            maxLength={MAX_DESCRIPTION_CHARS}
            onChange={(event) => handleField('description', event.target.value)}
            aria-invalid={errors.description ? true : undefined}
            aria-describedby={
              errors.description ? 'task-description-error' : 'task-description-hint'
            }
            disabled={isSaving}
          />
          <p className="field-hint" id="task-description-hint">
            {form.description.length}/{MAX_DESCRIPTION_CHARS} characters.
          </p>
          {errors.description && (
            <p className="field-error" id="task-description-error" role="alert">
              {errors.description}
            </p>
          )}
        </div>

        <div className="form-field">
          <label htmlFor="task-anchor">Schedule type</label>
          <select
            id="task-anchor"
            value={form.scheduleAnchor}
            onChange={(event) => handleField('scheduleAnchor', event.target.value)}
            disabled={isSaving}
          >
            <option value="manual">Manual calendar date</option>
            <option value="exam">Exam-relative date</option>
          </select>
          <p className="field-hint">
            Exam-relative dates adjust only after you confirm a new exam date. Manual dates never
            move automatically.
          </p>
        </div>
      </div>

      <div className="planner-form-row">
        {form.scheduleAnchor === 'manual' ? (
          <DateField
            id="task-scheduled-date"
            label="Scheduled date"
            value={form.scheduledDate}
            onChange={(value) => handleField('scheduledDate', value)}
            hint="The local calendar date this task first appears on."
            error={errors.scheduledDate}
            disabled={isSaving}
          />
        ) : (
          <div className={`form-field${errors.scheduleOffsetDays ? ' form-field-invalid' : ''}`}>
            <label htmlFor="task-offset-days">Days from the exam date</label>
            <div className="input-with-suffix">
              <input
                id="task-offset-days"
                type="number"
                step={1}
                min={MIN_OFFSET_DAYS}
                max={MAX_OFFSET_DAYS}
                value={form.offsetDays}
                onChange={(event) => handleField('offsetDays', event.target.value)}
                aria-invalid={errors.scheduleOffsetDays ? true : undefined}
                aria-describedby="task-offset-hint"
                disabled={isSaving}
              />
              <span aria-hidden="true">days</span>
            </div>
            <p className="field-hint" id="task-offset-hint">
              {anchorPreview
                ? `Currently resolves to ${formatDateLong(anchorPreview)}. Negative values are before the exam.`
                : 'Negative values are before the exam; positive values are after it.'}
            </p>
            {errors.scheduleOffsetDays && (
              <p className="field-error" role="alert">
                {errors.scheduleOffsetDays}
              </p>
            )}
          </div>
        )}

        <DateField
          id="task-deadline-date"
          label="Deadline date (optional)"
          value={form.deadlineDate}
          onChange={(value) => handleField('deadlineDate', value)}
          hint="Leave blank when the scheduled date is the only date that matters."
          error={errors.deadlineDate}
          disabled={isSaving}
        />
      </div>

      <div className="planner-form-row">
        <div className={`form-field${errors.topicId ? ' form-field-invalid' : ''}`}>
          <label htmlFor="task-topic">Syllabus topic (optional)</label>
          <select
            id="task-topic"
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
          <label htmlFor="task-priority">Priority</label>
          <select
            id="task-priority"
            value={form.priority}
            onChange={(event) => handleField('priority', event.target.value)}
            disabled={isSaving}
          >
            <option value="Low">Low</option>
            <option value="Normal">Normal</option>
            <option value="High">High</option>
            <option value="High-Yield">High-Yield</option>
          </select>
        </div>
      </div>

      <div className="form-footer">
        <button type="submit" className="primary-button" disabled={isSaving}>
          {isSaving ? 'Saving…' : task === null ? 'Add task' : 'Save task'}
        </button>
        <button type="button" className="secondary-button" onClick={onCancel} disabled={isSaving}>
          {task === null ? 'Clear' : 'Cancel edit'}
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
