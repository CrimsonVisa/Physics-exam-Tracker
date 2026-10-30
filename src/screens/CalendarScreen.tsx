import { type FormEvent, useEffect, useState } from 'react';
import { DateField } from '../components/DateField';
import { PlannedBlockEditor } from '../components/PlannedBlockEditor';
import { TaskEditor } from '../components/TaskEditor';
import { toCommandError } from '../lib/commandError';
import {
  addDaysDateOnly,
  addMonthsDateOnly,
  calendarDaysBetween,
  formatDateLong,
  formatDateWithWeekday,
  formatMonthYear,
  monthGridRange,
  parseDateOnly,
  startOfWeekDateOnly,
} from '../lib/dateOnly';
import {
  deletePlannedBlock,
  getCalendar,
  getPlannerTopics,
  type CalendarData,
  type ChecklistTask,
  type PlannedBlock,
  type PlannerTopic,
} from '../lib/plannerApi';
import type { RevisionCalendarItem } from '../lib/revisionApi';
import { formatTimeRange } from '../lib/timeOfDay';

type CalendarView = 'month' | 'week' | 'day';

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

const WEEKDAY_LABELS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as const;

function dateRangeForView(
  view: CalendarView,
  anchor: string,
): { start: string; end: string } | null {
  if (view === 'month') {
    return monthGridRange(anchor);
  }

  if (view === 'week') {
    const start = startOfWeekDateOnly(anchor);
    const end = start ? addDaysDateOnly(start, 6) : null;
    return start && end ? { start, end } : null;
  }

  return parseDateOnly(anchor) ? { start: anchor, end: anchor } : null;
}

function datesBetween(start: string, end: string): string[] {
  const count = calendarDaysBetween(start, end);
  if (count === null || count < 0) {
    return [];
  }

  const days: string[] = [];
  for (let index = 0; index <= count; index += 1) {
    const date = addDaysDateOnly(start, index);
    if (!date) {
      break;
    }
    days.push(date);
  }

  return days;
}

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function taskStatusLabel(task: ChecklistTask): string {
  return task.status === 'completed' ? 'Completed' : 'Open';
}

function blockStatusLabel(block: PlannedBlock): string {
  if (block.status === 'completed') {
    return 'Completed';
  }
  if (block.status === 'cancelled') {
    return 'Cancelled';
  }
  return 'Planned';
}

function DayItems({
  tasks,
  blocks,
  revisions,
  limit,
}: {
  tasks: ChecklistTask[];
  blocks: PlannedBlock[];
  revisions: RevisionCalendarItem[];
  limit?: number;
}) {
  const combined = [
    ...blocks.map((block) => ({ kind: 'block' as const, block })),
    ...revisions.map((revision) => ({ kind: 'revision' as const, revision })),
    ...tasks.map((task) => ({ kind: 'task' as const, task })),
  ];
  const visible = limit === undefined ? combined : combined.slice(0, limit);

  return (
    <>
      {visible.map((item) =>
        item.kind === 'block' ? (
          <span className="calendar-chip calendar-chip-block" key={`block-${item.block.id}`}>
            <span className="calendar-chip-time">
              {formatTimeRange(item.block.startTime, item.block.endTime)}
            </span>
            {item.block.title}
          </span>
        ) : item.kind === 'revision' ? (
          <span
            className="calendar-chip calendar-chip-revision"
            key={`revision-${item.revision.topicId}`}
          >
            Revision · {item.revision.topicCode}
          </span>
        ) : (
          <span
            className={`calendar-chip calendar-chip-task${item.task.status === 'completed' ? ' calendar-chip-done' : ''}`}
            key={`task-${item.task.id}`}
          >
            {item.task.title}
          </span>
        ),
      )}
      {limit !== undefined && combined.length > limit && (
        <span className="calendar-chip-more">+{combined.length - limit} more</span>
      )}
    </>
  );
}

export default function CalendarScreen({ today, examDate }: { today: string; examDate: string }) {
  const [view, setView] = useState<CalendarView>('month');
  const [anchor, setAnchor] = useState(today);
  const [selectedDate, setSelectedDate] = useState(today);
  const [jumpDate, setJumpDate] = useState(today);
  const [jumpError, setJumpError] = useState<string | null>(null);
  const [data, setData] = useState<CalendarData | null>(null);
  const [topics, setTopics] = useState<PlannerTopic[]>([]);
  const [topicsError, setTopicsError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [taskEditor, setTaskEditor] = useState<{ task: ChecklistTask | null } | null>(null);
  const [blockEditor, setBlockEditor] = useState<{ block: PlannedBlock | null } | null>(null);
  const [pendingDeleteBlockId, setPendingDeleteBlockId] = useState<number | null>(null);
  const [deletingBlockId, setDeletingBlockId] = useState<number | null>(null);

  const range = dateRangeForView(view, anchor);
  const rangeStart = range?.start;
  const rangeEnd = range?.end;

  useEffect(() => {
    let cancelled = false;

    if (!rangeStart || !rangeEnd) {
      return () => {
        cancelled = true;
      };
    }

    getCalendar(rangeStart, rangeEnd)
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
  }, [rangeStart, rangeEnd, reloadKey]);

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

  function selectDate(value: string) {
    setSelectedDate(value);
    setJumpDate(value);
    setJumpError(null);
    setPendingDeleteBlockId(null);
  }

  async function handleDeleteBlock(blockId: number) {
    if (deletingBlockId !== null) {
      return;
    }

    setDeletingBlockId(blockId);
    setFeedback(null);

    try {
      await deletePlannedBlock(blockId);
      setPendingDeleteBlockId(null);
      if (blockEditor?.block?.id === blockId) {
        setBlockEditor(null);
      }
      setFeedback({ tone: 'success', message: 'Planned study block deleted.' });
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setDeletingBlockId(null);
    }
  }

  function move(direction: -1 | 1) {
    const next =
      view === 'month'
        ? addMonthsDateOnly(anchor, direction)
        : addDaysDateOnly(anchor, direction * (view === 'week' ? 7 : 1));

    if (!next) {
      return;
    }

    setIsLoading(true);
    setLoadError(null);
    setAnchor(next);
    selectDate(next);
    setFeedback(null);
    setTaskEditor(null);
    setBlockEditor(null);
  }

  function goToToday() {
    setIsLoading(true);
    setLoadError(null);
    setAnchor(today);
    selectDate(today);
    setFeedback(null);
    setTaskEditor(null);
    setBlockEditor(null);
  }

  function changeView(next: CalendarView) {
    setIsLoading(true);
    setLoadError(null);
    setView(next);
    setAnchor(selectedDate);
    setTaskEditor(null);
    setBlockEditor(null);
    setFeedback(null);
  }

  function handleJump(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!parseDateOnly(jumpDate)) {
      setJumpError('Choose a real calendar date.');
      return;
    }

    setJumpError(null);
    setIsLoading(true);
    setLoadError(null);
    setAnchor(jumpDate);
    selectDate(jumpDate);
    setFeedback(null);
    setTaskEditor(null);
    setBlockEditor(null);
  }

  const rangeError = range ? null : 'The selected calendar range is invalid.';
  const days = range ? datesBetween(range.start, range.end) : [];
  const rangeLabel =
    view === 'month'
      ? (formatMonthYear(anchor) ?? anchor)
      : view === 'week' && range
        ? `${dateLabel(range.start)} – ${dateLabel(range.end)}`
        : (formatDateWithWeekday(anchor) ?? anchor);

  const tasksFor = (date: string) =>
    data ? data.tasks.filter((task) => task.scheduledDate === date) : [];
  const blocksFor = (date: string) =>
    data ? data.blocks.filter((block) => block.calendarDate === date) : [];
  const revisionsFor = (date: string) =>
    data ? data.revisions.filter((revision) => revision.nextRevisionDate === date) : [];

  const selectedTasks = tasksFor(selectedDate);
  const selectedBlocks = blocksFor(selectedDate);
  const selectedRevisions = revisionsFor(selectedDate);
  const selectedDeadlines = data
    ? data.deadlines.filter((task) => task.deadlineDate === selectedDate && task.status === 'open')
    : [];
  const calendarIsEmpty = Boolean(
    data &&
    data.tasks.length === 0 &&
    data.blocks.length === 0 &&
    data.revisions.length === 0 &&
    data.deadlines.length === 0,
  );
  const serverToday = data?.today ?? today;

  function dayClasses(date: string, extra = ''): string {
    const monthPrefix = anchor.slice(0, 7);
    const outside = view === 'month' && !date.startsWith(monthPrefix);
    return [
      extra,
      outside ? 'calendar-day-outside' : '',
      date === serverToday ? 'calendar-day-today' : '',
      date === selectedDate ? 'calendar-day-selected' : '',
    ]
      .filter(Boolean)
      .join(' ');
  }

  function renderMonth() {
    return (
      <div className="calendar-month" aria-label="Month calendar">
        <div className="calendar-weekdays" aria-hidden="true">
          {WEEKDAY_LABELS.map((label) => (
            <span key={label}>{label}</span>
          ))}
        </div>
        <div className="calendar-month-grid">
          {days.map((date) => {
            const tasks = tasksFor(date);
            const blocks = blocksFor(date);
            const revisions = revisionsFor(date);
            return (
              <button
                type="button"
                className={dayClasses(date, 'calendar-day-cell')}
                key={date}
                aria-label={`${formatDateWithWeekday(date) ?? date}, ${tasks.length} tasks, ${blocks.length} planned blocks, ${revisions.length} revisions`}
                aria-pressed={date === selectedDate}
                onClick={() => selectDate(date)}
              >
                <span className="calendar-day-number">{Number(date.slice(8, 10))}</span>
                <span className="calendar-day-items">
                  <DayItems tasks={tasks} blocks={blocks} revisions={revisions} limit={3} />
                </span>
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  function renderWeek() {
    return (
      <div className="calendar-week" aria-label="Week calendar">
        {days.map((date) => {
          const tasks = tasksFor(date);
          const blocks = blocksFor(date);
          const revisions = revisionsFor(date);
          return (
            <button
              type="button"
              className={dayClasses(date, 'calendar-week-day')}
              key={date}
              aria-label={`${formatDateWithWeekday(date) ?? date}, ${tasks.length} tasks, ${blocks.length} planned blocks, ${revisions.length} revisions`}
              aria-pressed={date === selectedDate}
              onClick={() => selectDate(date)}
            >
              <span className="calendar-week-day-heading">
                <span>{formatDateLong(date) ?? date}</span>
                <span className="calendar-week-day-count">
                  {tasks.length + blocks.length + revisions.length}
                </span>
              </span>
              <span className="calendar-day-items">
                <DayItems tasks={tasks} blocks={blocks} revisions={revisions} limit={5} />
              </span>
            </button>
          );
        })}
      </div>
    );
  }

  function renderDay() {
    return (
      <div className="calendar-day-view" aria-label="Day calendar">
        <div className="calendar-day-view-heading">
          <div>
            <p className="eyebrow">Daily schedule</p>
            <h3>{formatDateWithWeekday(anchor) ?? anchor}</h3>
          </div>
          <span className="calendar-day-total">
            {selectedTasks.length + selectedBlocks.length + selectedRevisions.length} items
          </span>
        </div>

        {selectedBlocks.length === 0 &&
        selectedTasks.length === 0 &&
        selectedRevisions.length === 0 ? (
          <div className="calendar-empty">
            <p className="calendar-empty-title">Nothing scheduled on this date</p>
            <p>Add a planned block or a checklist task from the day panel below.</p>
          </div>
        ) : (
          <div className="calendar-day-schedule">
            {selectedRevisions.length > 0 && (
              <section>
                <h4>Revisions</h4>
                {selectedRevisions.map((revision) => (
                  <article
                    className="calendar-schedule-row"
                    key={`day-revision-${revision.topicId}`}
                  >
                    <span className="calendar-schedule-time">Revision</span>
                    <div>
                      <p className="calendar-schedule-title">
                        {revision.topicCode} · {revision.topicTitle}
                      </p>
                      <p className="calendar-schedule-meta">
                        {revision.sectionTitle} · {revision.currentIntervalDays}-day interval ·{' '}
                        {revision.completedRevisions} completed
                      </p>
                    </div>
                  </article>
                ))}
              </section>
            )}

            {selectedBlocks.length > 0 && (
              <section>
                <h4>Planned blocks</h4>
                {selectedBlocks.map((block) => (
                  <article className="calendar-schedule-row" key={`day-block-${block.id}`}>
                    <span className="calendar-schedule-time">
                      {formatTimeRange(block.startTime, block.endTime)}
                    </span>
                    <div>
                      <p className="calendar-schedule-title">{block.title}</p>
                      <p className="calendar-schedule-meta">
                        {blockStatusLabel(block)}
                        {block.topic ? ` · ${block.topic.code} · ${block.topic.title}` : ''}
                        {block.linkedSessionId ? ` · linked session #${block.linkedSessionId}` : ''}
                      </p>
                    </div>
                  </article>
                ))}
              </section>
            )}

            {selectedTasks.length > 0 && (
              <section>
                <h4>Checklist tasks</h4>
                {selectedTasks.map((task) => (
                  <article className="calendar-schedule-row" key={`day-task-${task.id}`}>
                    <span className={`task-status task-status-${task.status}`}>
                      {taskStatusLabel(task)}
                    </span>
                    <div>
                      <p
                        className={`calendar-schedule-title${task.status === 'completed' ? ' calendar-schedule-title-done' : ''}`}
                      >
                        {task.title}
                      </p>
                      <p className="calendar-schedule-meta">
                        {task.priority}
                        {task.deadlineDate ? ` · deadline ${dateLabel(task.deadlineDate)}` : ''}
                        {task.topic ? ` · ${task.topic.code} · ${task.topic.title}` : ''}
                      </p>
                    </div>
                  </article>
                ))}
              </section>
            )}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="page-container calendar-page">
      <header className="page-header">
        <p className="eyebrow">Calendar</p>
        <h1>Plan the study week</h1>
        <p className="page-lede">
          See dated checklist tasks and planned study blocks in month, week, or day view. Planned
          blocks are commitments only; recorded study time continues to come from the Timer and
          Sessions pages.
        </p>
      </header>

      <section className="calendar-toolbar" aria-label="Calendar controls">
        <div className="calendar-nav">
          <button
            type="button"
            className="secondary-button"
            aria-label={`Previous ${view}`}
            onClick={() => move(-1)}
          >
            ←
          </button>
          <h2>{rangeLabel}</h2>
          <button
            type="button"
            className="secondary-button"
            aria-label={`Next ${view}`}
            onClick={() => move(1)}
          >
            →
          </button>
          <button type="button" className="secondary-button" onClick={goToToday}>
            Today
          </button>
        </div>

        <div className="calendar-view-switch" role="group" aria-label="Calendar view">
          {(['month', 'week', 'day'] as const).map((option) => (
            <button
              type="button"
              className={`calendar-view-button${view === option ? ' calendar-view-button-active' : ''}`}
              aria-pressed={view === option}
              key={option}
              onClick={() => changeView(option)}
            >
              {option === 'month' ? 'Month' : option === 'week' ? 'Week' : 'Day'}
            </button>
          ))}
        </div>

        <form className="calendar-jump" onSubmit={handleJump} noValidate>
          <DateField
            id="calendar-jump-date"
            label="Jump to date"
            value={jumpDate}
            onChange={(value) => {
              setJumpDate(value);
              setJumpError(null);
            }}
            error={jumpError ?? undefined}
          />
          <button type="submit" className="secondary-button">
            Go
          </button>
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

      {isLoading && !rangeError && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading the calendar.</p>
          <p className="state-copy">Reading dated tasks, planned blocks, and upcoming deadlines.</p>
        </section>
      )}

      {(rangeError || (!isLoading && loadError)) && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">The calendar could not be loaded</p>
          <p className="state-copy">{rangeError ?? loadError}</p>
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

      {!isLoading && !rangeError && !loadError && data && (
        <>
          {calendarIsEmpty && (
            <p className="calendar-empty-note" role="status">
              No tasks, planned blocks, or revisions in this calendar range. Use the day panel to
              add one.
            </p>
          )}

          <div className="calendar-layout">
            <div className="calendar-main">
              {view === 'month' ? renderMonth() : view === 'week' ? renderWeek() : renderDay()}

              <section className="selected-day-card" aria-labelledby="selected-day-heading">
                <div className="card-heading">
                  <div>
                    <p className="eyebrow">Selected day</p>
                    <h2 id="selected-day-heading">
                      {formatDateWithWeekday(selectedDate) ?? selectedDate}
                    </h2>
                  </div>
                  <div className="selected-day-actions">
                    <button
                      type="button"
                      className="primary-button"
                      onClick={() => {
                        setTaskEditor({ task: null });
                        setBlockEditor(null);
                        setPendingDeleteBlockId(null);
                        setFeedback(null);
                      }}
                    >
                      Add task here
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => {
                        setBlockEditor({ block: null });
                        setTaskEditor(null);
                        setPendingDeleteBlockId(null);
                        setFeedback(null);
                      }}
                    >
                      Add planned block
                    </button>
                  </div>
                </div>

                <div className="selected-day-columns">
                  <div>
                    <h3>Tasks</h3>
                    {selectedTasks.length === 0 ? (
                      <p className="selected-day-empty">No checklist tasks scheduled.</p>
                    ) : (
                      <div className="selected-day-list">
                        {selectedTasks.map((task) => (
                          <article
                            className={`selected-day-item${task.status === 'completed' ? ' selected-day-item-done' : ''}`}
                            key={`selected-task-${task.id}`}
                          >
                            <div>
                              <p className="selected-day-item-title">{task.title}</p>
                              <p className="selected-day-item-meta">
                                {taskStatusLabel(task)} · {task.priority}
                                {task.deadlineDate
                                  ? ` · deadline ${dateLabel(task.deadlineDate)}`
                                  : ''}
                              </p>
                            </div>
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => {
                                setTaskEditor({ task });
                                setBlockEditor(null);
                                setFeedback(null);
                              }}
                            >
                              Edit task
                            </button>
                          </article>
                        ))}
                      </div>
                    )}
                  </div>

                  <div>
                    <h3>Planned blocks</h3>
                    {selectedBlocks.length === 0 ? (
                      <p className="selected-day-empty">No planned study blocks.</p>
                    ) : (
                      <div className="selected-day-list">
                        {selectedBlocks.map((block) => (
                          <article className="selected-day-item" key={`selected-block-${block.id}`}>
                            <div>
                              <p className="selected-day-item-title">{block.title}</p>
                              <p className="selected-day-item-meta">
                                {formatTimeRange(block.startTime, block.endTime)} ·{' '}
                                {blockStatusLabel(block)}
                                {block.linkedSessionId
                                  ? ` · linked session #${block.linkedSessionId}`
                                  : ''}
                              </p>
                            </div>
                            <div className="selected-day-item-actions">
                              <button
                                type="button"
                                className="secondary-button"
                                onClick={() => {
                                  setBlockEditor({ block });
                                  setTaskEditor(null);
                                  setPendingDeleteBlockId(null);
                                  setFeedback(null);
                                }}
                              >
                                Edit block
                              </button>
                              <button
                                type="button"
                                className="danger-button"
                                onClick={() => {
                                  setPendingDeleteBlockId(block.id);
                                  setFeedback(null);
                                }}
                                disabled={deletingBlockId !== null}
                              >
                                Delete block
                              </button>
                            </div>

                            {pendingDeleteBlockId === block.id && (
                              <div
                                className="task-delete-confirm"
                                role="alertdialog"
                                aria-labelledby={`delete-block-title-${block.id}`}
                              >
                                <p
                                  className="task-delete-title"
                                  id={`delete-block-title-${block.id}`}
                                >
                                  Delete this planned block permanently?
                                </p>
                                <p>
                                  The plan will be removed. Linked study sessions and recorded study
                                  time are not affected.
                                </p>
                                <div className="form-footer">
                                  <button
                                    type="button"
                                    className="danger-button"
                                    onClick={() => void handleDeleteBlock(block.id)}
                                    disabled={deletingBlockId !== null}
                                  >
                                    {deletingBlockId === block.id
                                      ? 'Deleting…'
                                      : 'Confirm delete block'}
                                  </button>
                                  <button
                                    type="button"
                                    className="secondary-button"
                                    onClick={() => setPendingDeleteBlockId(null)}
                                    disabled={deletingBlockId !== null}
                                  >
                                    Cancel
                                  </button>
                                </div>
                              </div>
                            )}
                          </article>
                        ))}
                      </div>
                    )}
                  </div>

                  <div>
                    <h3>Revisions</h3>
                    {selectedRevisions.length === 0 ? (
                      <p className="selected-day-empty">No revisions scheduled.</p>
                    ) : (
                      <div className="selected-day-list">
                        {selectedRevisions.map((revision) => (
                          <article
                            className="selected-day-item"
                            key={`selected-revision-${revision.topicId}`}
                          >
                            <div>
                              <p className="selected-day-item-title">
                                {revision.topicCode} · {revision.topicTitle}
                              </p>
                              <p className="selected-day-item-meta">
                                {revision.sectionTitle} · interval {revision.currentIntervalDays}{' '}
                                days · {revision.completedRevisions} completed
                              </p>
                            </div>
                          </article>
                        ))}
                      </div>
                    )}
                  </div>
                </div>

                {selectedDeadlines.length > 0 && (
                  <div className="selected-day-deadlines">
                    <h3>Deadlines on this day</h3>
                    <ul>
                      {selectedDeadlines.map((task) => (
                        <li key={`selected-deadline-${task.id}`}>{task.title}</li>
                      ))}
                    </ul>
                  </div>
                )}
              </section>
            </div>

            <aside className="calendar-side" aria-label="Deadline summary">
              <section className="calendar-side-card" aria-labelledby="upcoming-deadlines-heading">
                <div className="card-heading">
                  <div>
                    <p className="eyebrow">Next 30 days</p>
                    <h2 id="upcoming-deadlines-heading">Upcoming deadlines</h2>
                  </div>
                </div>
                {data.upcomingDeadlines.length === 0 ? (
                  <p className="selected-day-empty">No open tasks with upcoming deadlines.</p>
                ) : (
                  <ul className="deadline-list">
                    {data.upcomingDeadlines.map((task) => (
                      <li key={`upcoming-${task.id}`}>
                        <p className="deadline-title">{task.title}</p>
                        <p className="deadline-meta">
                          {task.deadlineDate ? dateLabel(task.deadlineDate) : 'No deadline'}
                          {task.topic ? ` · ${task.topic.code}` : ''}
                        </p>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              <section className="calendar-side-card" aria-labelledby="overdue-heading">
                <div className="card-heading">
                  <div>
                    <p className="eyebrow">Needs attention</p>
                    <h2 id="overdue-heading">Overdue tasks</h2>
                  </div>
                </div>
                {data.overdueTasks.length === 0 ? (
                  <p className="selected-day-empty">No overdue open tasks.</p>
                ) : (
                  <ul className="deadline-list">
                    {data.overdueTasks.map((task) => (
                      <li key={`overdue-${task.id}`}>
                        <p className="deadline-title">{task.title}</p>
                        <p className="deadline-meta">
                          Due {dateLabel(task.dueDate)}
                          {task.topic ? ` · ${task.topic.code}` : ''}
                        </p>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              <p className="calendar-side-note">
                Planned blocks and revision dates never add to study totals. A completed block can
                link to an actual session only when you choose one in its editor and save.
              </p>
            </aside>
          </div>

          {taskEditor && (
            <section
              className="calendar-editor-card"
              aria-labelledby="calendar-task-editor-heading"
            >
              <div className="card-heading">
                <div>
                  <p className="eyebrow">{taskEditor.task ? 'Editing task' : 'New task'}</p>
                  <h2 id="calendar-task-editor-heading">
                    {taskEditor.task ? 'Edit checklist task' : 'Add a task for this day'}
                  </h2>
                </div>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setTaskEditor(null)}
                >
                  Close editor
                </button>
              </div>
              {topicsError && (
                <p className="save-feedback save-feedback-error" role="alert">
                  Syllabus topics could not be loaded: {topicsError}
                </p>
              )}
              <TaskEditor
                key={
                  taskEditor.task
                    ? `calendar-task-${taskEditor.task.id}`
                    : `calendar-task-new-${selectedDate}`
                }
                task={taskEditor.task}
                topics={topics}
                defaultDate={selectedDate}
                examDate={examDate}
                onSaved={(saved, wasEditing) => {
                  setTaskEditor(null);
                  setFeedback({
                    tone: 'success',
                    message: wasEditing
                      ? `“${saved.title}” updated.`
                      : `“${saved.title}” added for ${dateLabel(saved.scheduledDate)}.`,
                  });
                  reload();
                }}
                onCancel={() => setTaskEditor(null)}
              />
            </section>
          )}

          {blockEditor && (
            <section
              className="calendar-editor-card"
              aria-labelledby="calendar-block-editor-heading"
            >
              <div className="card-heading">
                <div>
                  <p className="eyebrow">{blockEditor.block ? 'Editing block' : 'New plan'}</p>
                  <h2 id="calendar-block-editor-heading">
                    {blockEditor.block ? 'Edit planned study block' : 'Plan a study block'}
                  </h2>
                </div>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setBlockEditor(null)}
                >
                  Close editor
                </button>
              </div>
              <PlannedBlockEditor
                key={
                  blockEditor.block
                    ? `calendar-block-${blockEditor.block.id}`
                    : `calendar-block-new-${selectedDate}`
                }
                block={blockEditor.block}
                topics={topics}
                defaultDate={selectedDate}
                onSaved={(saved, wasEditing) => {
                  setBlockEditor(null);
                  setFeedback({
                    tone: 'success',
                    message: wasEditing
                      ? `“${saved.title}” updated.`
                      : `“${saved.title}” planned for ${dateLabel(saved.calendarDate)}.`,
                  });
                  reload();
                }}
                onCancel={() => setBlockEditor(null)}
              />
            </section>
          )}
        </>
      )}
    </div>
  );
}
