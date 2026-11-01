import { type FormEvent, useEffect, useState } from 'react';
import { DateField } from '../components/DateField';
import { TaskEditor } from '../components/TaskEditor';
import { toCommandError } from '../lib/commandError';
import { formatDateLong, parseDateOnly } from '../lib/dateOnly';
import {
  completeChecklistTask,
  deleteChecklistTask,
  getChecklistTasks,
  getPlannerTopics,
  reopenChecklistTask,
  rescheduleChecklistTask,
  type ChecklistTask,
  type PlannerTopic,
  type TaskStatus,
} from '../lib/plannerApi';

type StatusFilter = TaskStatus | 'all';

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

type PendingAction = {
  taskId: number;
  kind: 'complete' | 'reopen' | 'delete' | 'reschedule';
} | null;

const MAX_QUERY_CHARS = 100;

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function completedLabel(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return parsed.toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

function emptyMessage(
  filter: StatusFilter,
  query: string,
  hasDateRange: boolean,
): {
  title: string;
  copy: string;
} {
  if (query) {
    return {
      title: 'No tasks match this search',
      copy: 'Try a shorter title or description search, or clear the search to see every task.',
    };
  }

  if (hasDateRange) {
    return {
      title: 'No tasks match this date range',
      copy: 'Choose a wider scheduled-date range or clear the dates to see the full checklist.',
    };
  }

  if (filter === 'completed') {
    return {
      title: 'No completed tasks yet',
      copy: 'Completed tasks keep their original completion timestamp and move out of the open list.',
    };
  }

  if (filter === 'all') {
    return {
      title: 'No checklist tasks yet',
      copy: 'Add the next small action, give it a date, and it will appear here and on the calendar.',
    };
  }

  return {
    title: 'No open tasks',
    copy: 'Everything in this view is complete. Add the next action or switch to Completed to review history.',
  };
}

function TaskRow({
  task,
  busy,
  pendingDelete,
  rescheduleOpen,
  rescheduleDate,
  rescheduleError,
  onComplete,
  onReopen,
  onEdit,
  onOpenReschedule,
  onRescheduleDate,
  onCancelReschedule,
  onReschedule,
  onRequestDelete,
  onCancelDelete,
  onConfirmDelete,
}: {
  task: ChecklistTask;
  busy: boolean;
  pendingDelete: boolean;
  rescheduleOpen: boolean;
  rescheduleDate: string;
  rescheduleError: string | null;
  onComplete: () => void;
  onReopen: () => void;
  onEdit: () => void;
  onOpenReschedule: () => void;
  onRescheduleDate: (value: string) => void;
  onCancelReschedule: () => void;
  onReschedule: (event: FormEvent<HTMLFormElement>) => void;
  onRequestDelete: () => void;
  onCancelDelete: () => void;
  onConfirmDelete: () => void;
}) {
  return (
    <article className={`task-row${task.status === 'completed' ? ' task-row-completed' : ''}`}>
      <div className="task-row-heading">
        <h3>{task.title}</h3>
        <div className="task-badges">
          <span className={`priority-badge priority-${task.priority.toLowerCase()}`}>
            {task.priority}
          </span>
          {task.isOverdue && <span className="task-flag task-flag-overdue">Overdue</span>}
          {task.isUpcoming && <span className="task-flag task-flag-upcoming">Due soon</span>}
          <span className={`task-status task-status-${task.status}`}>
            {task.status === 'completed' ? 'Completed' : 'Open'}
          </span>
        </div>
      </div>

      {task.description && <p className="task-description">{task.description}</p>}

      <p className="task-meta">
        <span>Scheduled {dateLabel(task.scheduledDate)}</span>
        {task.deadlineDate && (
          <span className={task.isOverdue ? 'task-meta-overdue' : undefined}>
            Deadline {dateLabel(task.deadlineDate)}
          </span>
        )}
        {task.topic && (
          <span>
            {task.topic.code} · {task.topic.title}
            {task.topic.archived ? ' (archived)' : ''}
          </span>
        )}
        {task.scheduleAnchor === 'exam' && (
          <span>Exam-relative · {task.scheduleOffsetDays ?? 0} days from exam date</span>
        )}
      </p>

      {task.completedAt && (
        <p className="task-completed-at">Completed {completedLabel(task.completedAt)}</p>
      )}

      <div className="task-row-actions">
        {task.status === 'open' ? (
          <button
            type="button"
            className="primary-button task-action"
            onClick={onComplete}
            disabled={busy}
          >
            Complete
          </button>
        ) : (
          <button type="button" className="secondary-button" onClick={onReopen} disabled={busy}>
            Reopen
          </button>
        )}
        <button type="button" className="secondary-button" onClick={onEdit} disabled={busy}>
          Edit
        </button>
        {task.status === 'open' && (
          <button
            type="button"
            className="secondary-button"
            onClick={onOpenReschedule}
            disabled={busy}
          >
            Reschedule
          </button>
        )}
        <button type="button" className="danger-button" onClick={onRequestDelete} disabled={busy}>
          Delete
        </button>
      </div>

      {rescheduleOpen && (
        <form className="task-reschedule" onSubmit={onReschedule} noValidate>
          <DateField
            id={`task-reschedule-${task.id}`}
            label="New scheduled date"
            value={rescheduleDate}
            onChange={onRescheduleDate}
            hint="Rescheduling an open task makes its date manual; exam-relative tasks stop following the exam."
            error={rescheduleError ?? undefined}
            disabled={busy}
          />
          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={busy}>
              {busy ? 'Saving…' : 'Save new date'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={onCancelReschedule}
              disabled={busy}
            >
              Cancel
            </button>
          </div>
        </form>
      )}

      {pendingDelete && (
        <div
          className="task-delete-confirm"
          role="alertdialog"
          aria-labelledby={`delete-task-title-${task.id}`}
        >
          <p className="task-delete-title" id={`delete-task-title-${task.id}`}>
            Delete this task permanently?
          </p>
          <p>
            The checklist record will be removed. Linked syllabus progress and study sessions are
            not affected.
          </p>
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

export default function ChecklistScreen({ today, examDate }: { today: string; examDate: string }) {
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('open');
  const [queryInput, setQueryInput] = useState('');
  const [appliedQuery, setAppliedQuery] = useState('');
  const [queryError, setQueryError] = useState<string | null>(null);
  const [dateForm, setDateForm] = useState({ fromDate: '', toDate: '' });
  const [appliedDates, setAppliedDates] = useState<{ fromDate?: string; toDate?: string }>({});
  const [dateError, setDateError] = useState<string | null>(null);
  const [tasks, setTasks] = useState<ChecklistTask[]>([]);
  const [topics, setTopics] = useState<PlannerTopic[]>([]);
  const [topicsError, setTopicsError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [editingTask, setEditingTask] = useState<ChecklistTask | null>(null);
  const [editorVersion, setEditorVersion] = useState(0);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null);
  const [rescheduleTaskId, setRescheduleTaskId] = useState<number | null>(null);
  const [rescheduleDate, setRescheduleDate] = useState(today);
  const [rescheduleError, setRescheduleError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getChecklistTasks({
      status: statusFilter,
      ...(appliedQuery ? { query: appliedQuery } : {}),
      ...(appliedDates.fromDate ? { fromDate: appliedDates.fromDate } : {}),
      ...(appliedDates.toDate ? { toDate: appliedDates.toDate } : {}),
    })
      .then((loaded) => {
        if (!cancelled) {
          setTasks(loaded);
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
  }, [statusFilter, appliedQuery, appliedDates, reloadKey]);

  useEffect(() => {
    let cancelled = false;

    getPlannerTopics()
      .then((loaded) => {
        if (!cancelled) {
          setTopics(loaded);
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
  }, []);

  function reload() {
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }

  function handleSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = queryInput.trim();
    if (trimmed.length > MAX_QUERY_CHARS) {
      setQueryError(`Search text must be ${MAX_QUERY_CHARS} characters or fewer.`);
      return;
    }

    setQueryError(null);
    setFeedback(null);
    if (trimmed !== appliedQuery) {
      setIsLoading(true);
      setLoadError(null);
      setAppliedQuery(trimmed);
    }
  }

  function clearSearch() {
    setQueryInput('');
    setQueryError(null);
    if (appliedQuery) {
      setIsLoading(true);
      setLoadError(null);
      setAppliedQuery('');
    }
  }

  function handleDateFilter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const fromDate = dateForm.fromDate;
    const toDate = dateForm.toDate;

    if (fromDate && !parseDateOnly(fromDate)) {
      setDateError('Choose a real scheduled start date or leave it blank.');
      return;
    }
    if (toDate && !parseDateOnly(toDate)) {
      setDateError('Choose a real scheduled end date or leave it blank.');
      return;
    }
    if (fromDate && toDate && fromDate > toDate) {
      setDateError('The scheduled start date cannot be after the end date.');
      return;
    }

    setDateError(null);
    setFeedback(null);
    setIsLoading(true);
    setLoadError(null);
    setAppliedDates({
      ...(fromDate ? { fromDate } : {}),
      ...(toDate ? { toDate } : {}),
    });
  }

  function clearDateFilter() {
    setDateForm({ fromDate: '', toDate: '' });
    setDateError(null);
    if (appliedDates.fromDate || appliedDates.toDate) {
      setFeedback(null);
      setIsLoading(true);
      setLoadError(null);
      setAppliedDates({});
    }
  }

  function handleFilter(next: StatusFilter) {
    setFeedback(null);
    if (next !== statusFilter) {
      setIsLoading(true);
      setLoadError(null);
      setStatusFilter(next);
    }
  }

  async function handleStatusChange(task: ChecklistTask) {
    if (pendingAction !== null) {
      return;
    }

    setPendingAction({ taskId: task.id, kind: task.status === 'open' ? 'complete' : 'reopen' });
    setFeedback(null);

    try {
      if (task.status === 'open') {
        await completeChecklistTask(task.id);
        setFeedback({
          tone: 'success',
          message: 'Task completed. Its completion timestamp was saved.',
        });
        if (editingTask?.id === task.id) {
          setEditingTask(null);
        }
      } else {
        await reopenChecklistTask(task.id);
        setFeedback({
          tone: 'success',
          message: 'Task reopened. The previous completion timestamp was cleared.',
        });
        if (editingTask?.id === task.id) {
          setEditingTask(null);
        }
      }
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setPendingAction(null);
    }
  }

  async function handleDelete(taskId: number) {
    if (pendingAction !== null) {
      return;
    }

    setPendingAction({ taskId, kind: 'delete' });
    setFeedback(null);

    try {
      await deleteChecklistTask(taskId);
      setPendingDeleteId(null);
      if (editingTask?.id === taskId) {
        setEditingTask(null);
      }
      if (rescheduleTaskId === taskId) {
        setRescheduleTaskId(null);
      }
      setFeedback({ tone: 'success', message: 'Checklist task deleted.' });
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setPendingAction(null);
    }
  }

  async function handleReschedule(event: FormEvent<HTMLFormElement>, task: ChecklistTask) {
    event.preventDefault();
    if (pendingAction !== null) {
      return;
    }

    if (!parseDateOnly(rescheduleDate)) {
      setRescheduleError('Choose a real calendar date.');
      return;
    }

    if (task.deadlineDate && rescheduleDate > task.deadlineDate) {
      setRescheduleError(
        `The task deadline is ${dateLabel(task.deadlineDate)}; choose a date on or before it.`,
      );
      return;
    }

    setRescheduleError(null);
    setPendingAction({ taskId: task.id, kind: 'reschedule' });
    setFeedback(null);

    try {
      await rescheduleChecklistTask(task.id, rescheduleDate);
      setRescheduleTaskId(null);
      setFeedback({
        tone: 'success',
        message: 'Open task rescheduled and switched to a manual date. Completed tasks never move.',
      });
      reload();
    } catch (error: unknown) {
      setRescheduleError(toCommandError(error).message);
    } finally {
      setPendingAction(null);
    }
  }

  function openReschedule(task: ChecklistTask) {
    setRescheduleTaskId(task.id);
    setRescheduleDate(task.scheduledDate);
    setRescheduleError(null);
    setPendingDeleteId(null);
    setFeedback(null);
  }

  const hasDateRange = Boolean(appliedDates.fromDate || appliedDates.toDate);
  const empty = emptyMessage(statusFilter, appliedQuery, hasDateRange);

  return (
    <div className="page-container checklist-page">
      <header className="page-header">
        <p className="eyebrow">Checklist</p>
        <h1>Tasks and next actions</h1>
        <p className="page-lede">
          Keep small study actions dated and visible. Open tasks can be completed, reopened, and
          rescheduled; completed tasks keep their completion timestamp and never move when the exam
          date changes.
        </p>
      </header>

      <section className="checklist-filter-card" aria-labelledby="checklist-filter-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Views</p>
            <h2 id="checklist-filter-heading">Find tasks</h2>
          </div>
          <span className="local-badge">SQLite · local</span>
        </div>

        <div className="checklist-filter-controls">
          <div className="task-filter-group" role="group" aria-label="Task status filter">
            {(['open', 'completed', 'all'] as const).map((filter) => (
              <button
                type="button"
                className={`task-filter-button${statusFilter === filter ? ' task-filter-button-active' : ''}`}
                aria-pressed={statusFilter === filter}
                key={filter}
                onClick={() => handleFilter(filter)}
              >
                {filter === 'open' ? 'Open' : filter === 'completed' ? 'Completed' : 'All'}
              </button>
            ))}
          </div>

          <form className="task-search-form" onSubmit={handleSearch} noValidate>
            <div className="form-field">
              <label htmlFor="task-search">Search</label>
              <input
                id="task-search"
                type="search"
                value={queryInput}
                maxLength={MAX_QUERY_CHARS}
                onChange={(event) => {
                  setQueryInput(event.target.value);
                  setQueryError(null);
                }}
                aria-invalid={queryError ? true : undefined}
                aria-describedby={queryError ? 'task-search-error' : undefined}
              />
              {queryError && (
                <p className="field-error" id="task-search-error" role="alert">
                  {queryError}
                </p>
              )}
            </div>
            <div className="task-search-actions">
              <button type="submit" className="secondary-button">
                Search
              </button>
              {appliedQuery && (
                <button type="button" className="link-button" onClick={clearSearch}>
                  Clear search
                </button>
              )}
            </div>
          </form>
        </div>

        <form className="history-date-filter" onSubmit={handleDateFilter} noValidate>
          <DateField
            id="checklist-date-from"
            label="Scheduled from (optional)"
            value={dateForm.fromDate}
            onChange={(value) => {
              setDateForm((current) => ({ ...current, fromDate: value }));
              setDateError(null);
            }}
            max={dateForm.toDate || undefined}
          />
          <DateField
            id="checklist-date-to"
            label="Scheduled to (optional)"
            value={dateForm.toDate}
            onChange={(value) => {
              setDateForm((current) => ({ ...current, toDate: value }));
              setDateError(null);
            }}
            min={dateForm.fromDate || undefined}
          />
          <div className="history-date-actions">
            <button type="submit" className="secondary-button">
              Apply dates
            </button>
            {(appliedDates.fromDate || appliedDates.toDate) && (
              <button type="button" className="link-button" onClick={clearDateFilter}>
                Clear dates
              </button>
            )}
          </div>
          {dateError && (
            <p className="field-error history-date-error" role="alert">
              {dateError}
            </p>
          )}
        </form>
      </section>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading checklist tasks.</p>
          <p className="state-copy">Reading dated actions and their syllabus links from SQLite.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Checklist tasks could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              reload();
            }}
          >
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && (
        <div className="checklist-workspace">
          <section className="checklist-list-card" aria-labelledby="checklist-list-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Dated actions</p>
                <h2 id="checklist-list-heading">
                  {statusFilter === 'open'
                    ? 'Open tasks'
                    : statusFilter === 'completed'
                      ? 'Completed tasks'
                      : 'All tasks'}{' '}
                  ({tasks.length})
                </h2>
              </div>
              {appliedQuery && <span className="task-query-badge">Search: “{appliedQuery}”</span>}
            </div>

            {tasks.length === 0 ? (
              <div className="task-empty">
                <p className="task-empty-title">{empty.title}</p>
                <p>{empty.copy}</p>
              </div>
            ) : (
              <div className="task-list">
                {tasks.map((task) => (
                  <TaskRow
                    key={task.id}
                    task={task}
                    busy={pendingAction !== null}
                    pendingDelete={pendingDeleteId === task.id}
                    rescheduleOpen={rescheduleTaskId === task.id}
                    rescheduleDate={rescheduleDate}
                    rescheduleError={rescheduleTaskId === task.id ? rescheduleError : null}
                    onComplete={() => void handleStatusChange(task)}
                    onReopen={() => void handleStatusChange(task)}
                    onEdit={() => {
                      setEditingTask(task);
                      setPendingDeleteId(null);
                      setRescheduleTaskId(null);
                      setFeedback(null);
                    }}
                    onOpenReschedule={() => openReschedule(task)}
                    onRescheduleDate={(value) => {
                      setRescheduleDate(value);
                      setRescheduleError(null);
                    }}
                    onCancelReschedule={() => {
                      setRescheduleTaskId(null);
                      setRescheduleError(null);
                    }}
                    onReschedule={(event) => void handleReschedule(event, task)}
                    onRequestDelete={() => {
                      setPendingDeleteId(task.id);
                      setRescheduleTaskId(null);
                      setFeedback(null);
                    }}
                    onCancelDelete={() => setPendingDeleteId(null)}
                    onConfirmDelete={() => void handleDelete(task.id)}
                  />
                ))}
              </div>
            )}
          </section>

          <section className="task-editor-card" aria-labelledby="task-editor-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">{editingTask ? 'Editing' : 'New task'}</p>
                <h2 id="task-editor-heading">
                  {editingTask ? 'Edit checklist task' : 'Add a checklist task'}
                </h2>
              </div>
            </div>

            <p className="settings-lede">
              {editingTask
                ? 'Changes update the task content and dates. Completion state is changed only with the Complete or Reopen actions.'
                : 'A task can use either a manual calendar date or an exam-relative offset. Only you can move a manual task.'}
            </p>

            {topicsError && (
              <p className="save-feedback save-feedback-error" role="alert">
                Syllabus topics could not be loaded: {topicsError}
              </p>
            )}

            <TaskEditor
              key={
                editingTask
                  ? `task-${editingTask.id}-${editorVersion}`
                  : `task-new-${editorVersion}`
              }
              task={editingTask}
              topics={topics}
              defaultDate={today}
              examDate={examDate}
              onSaved={(saved, wasEditing) => {
                setEditingTask(null);
                setEditorVersion((version) => version + 1);
                setFeedback({
                  tone: 'success',
                  message: wasEditing
                    ? `“${saved.title}” updated.`
                    : `“${saved.title}” added to the checklist.`,
                });
                reload();
              }}
              onCancel={() => {
                setEditingTask(null);
                setEditorVersion((version) => version + 1);
                setFeedback(null);
              }}
            />
          </section>
        </div>
      )}
    </div>
  );
}
