import { type FormEvent, useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  deleteV2Block,
  deleteV2Task,
  getV2PlannerDay,
  saveV2Block,
  saveV2Task,
  setV2BlockStatus,
  setV2TaskStatus,
  type V2ExamSummary,
  type V2PlannerBlock,
  type V2PlannerDay,
  type V2PlannerTask,
} from './api';

type PlannerScreenProps = {
  today: string;
  exams: V2ExamSummary[];
};

export function PlannerScreen({ today, exams }: PlannerScreenProps) {
  const [date, setDate] = useState(today);
  const [day, setDay] = useState<V2PlannerDay | null>(null);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [deadline, setDeadline] = useState('');
  const [priority, setPriority] = useState('normal');
  const [examId, setExamId] = useState('');
  const [blockTitle, setBlockTitle] = useState('');
  const [blockStart, setBlockStart] = useState('09:00');
  const [blockEnd, setBlockEnd] = useState('10:00');
  const [blockExamId, setBlockExamId] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const loadDay = useCallback(async (target: string) => {
    try {
      setDay(await getV2PlannerDay(target));
      setError(null);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      await loadDay(date);
    })();
  }, [date, loadDay]);

  async function addTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await saveV2Task({
        title,
        description,
        scheduledDate: date,
        deadlineDate: deadline || null,
        examId: examId ? Number(examId) : null,
        conceptId: null,
        priority,
        scheduleAnchor: 'manual',
        scheduleOffsetDays: null,
      });
      setTitle('');
      setDescription('');
      setDeadline('');
      await loadDay(date);
      setNotice('Task added.');
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function addBlock(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await saveV2Block({
        title: blockTitle,
        examId: blockExamId ? Number(blockExamId) : null,
        conceptId: null,
        calendarDate: date,
        startTime: blockStart,
        endTime: blockEnd,
        status: 'planned',
        notes: '',
      });
      setBlockTitle('');
      await loadDay(date);
      setNotice('Planned block added.');
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function toggleTask(task: V2PlannerTask) {
    setBusy(true);
    setError(null);
    try {
      await setV2TaskStatus({
        taskId: task.id,
        status: task.status === 'completed' ? 'open' : 'completed',
      });
      await loadDay(date);
    } catch (taskError) {
      setError(toCommandError(taskError).message);
    } finally {
      setBusy(false);
    }
  }

  async function removeTask(task: V2PlannerTask) {
    setBusy(true);
    setError(null);
    try {
      await deleteV2Task(task.id);
      await loadDay(date);
    } catch (taskError) {
      setError(toCommandError(taskError).message);
    } finally {
      setBusy(false);
    }
  }

  async function changeBlockStatus(block: V2PlannerBlock, status: string) {
    setBusy(true);
    setError(null);
    try {
      await setV2BlockStatus({ blockId: block.id, status });
      await loadDay(date);
    } catch (blockError) {
      setError(toCommandError(blockError).message);
    } finally {
      setBusy(false);
    }
  }

  async function removeBlock(block: V2PlannerBlock) {
    setBusy(true);
    setError(null);
    try {
      await deleteV2Block(block.id);
      await loadDay(date);
    } catch (blockError) {
      setError(toCommandError(blockError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page-container v2-planner-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · study plan</p>
        <h1>Planner and calendar</h1>
        <p className="page-lede">
          Daily checklist tasks and planned study blocks. Completed tasks and blocks persist, and
          planned time never counts as recorded study time.
        </p>
      </header>

      <div className="v2-planner-datebar">
        <label className="v2-field">
          <span>Planning date</span>
          <input onChange={(event) => setDate(event.target.value)} type="date" value={date} />
        </label>
        <button className="secondary-button" onClick={() => setDate(today)} type="button">
          Today
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

      <div className="v2-planner-grid">
        <section className="v2-concept-card" aria-labelledby="tasks-heading">
          <p className="eyebrow">Checklist</p>
          <h2 id="tasks-heading">Tasks for {date}</h2>
          <form className="v2-planner-form" onSubmit={addTask}>
            <label className="v2-field">
              <span>Task title</span>
              <input
                maxLength={200}
                onChange={(event) => setTitle(event.target.value)}
                required
                value={title}
              />
            </label>
            <label className="v2-field">
              <span>Description</span>
              <textarea
                maxLength={2000}
                onChange={(event) => setDescription(event.target.value)}
                rows={2}
                value={description}
              />
            </label>
            <div className="v2-planner-form-row">
              <label className="v2-field">
                <span>Deadline</span>
                <input
                  onChange={(event) => setDeadline(event.target.value)}
                  type="date"
                  value={deadline}
                />
              </label>
              <label className="v2-field">
                <span>Priority</span>
                <select onChange={(event) => setPriority(event.target.value)} value={priority}>
                  <option value="low">Low</option>
                  <option value="normal">Normal</option>
                  <option value="high">High</option>
                  <option value="high_yield">High yield</option>
                </select>
              </label>
              <label className="v2-field">
                <span>Examination</span>
                <select onChange={(event) => setExamId(event.target.value)} value={examId}>
                  <option value="">Shared / no exam</option>
                  {exams.map((exam) => (
                    <option key={exam.id} value={exam.id}>
                      {exam.shortName}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <button className="primary-button" disabled={busy} type="submit">
              Add task
            </button>
          </form>

          {!day || day.tasks.length === 0 ? (
            <p className="v2-empty-panel-copy">No tasks are scheduled for this date.</p>
          ) : (
            <ul className="v2-planner-list">
              {day.tasks.map((task) => (
                <li
                  className={`v2-planner-row${task.status === 'completed' ? ' v2-planner-done' : ''}`}
                  key={task.id}
                >
                  <div>
                    <div className="v2-session-heading">
                      <strong>{task.title}</strong>
                      <span className="v2-status-badge v2-status-inactive">
                        {task.priority === 'high_yield' ? 'High yield' : task.priority}
                      </span>
                      {task.examCode && <span className="v2-allocation-chip">{task.examCode}</span>}
                    </div>
                    {task.description && <p className="v2-empty-copy">{task.description}</p>}
                    <p className="v2-empty-copy">
                      Scheduled {task.scheduledDate}
                      {task.deadlineDate ? ` · deadline ${task.deadlineDate}` : ''}
                    </p>
                  </div>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={() => void toggleTask(task)}
                      type="button"
                    >
                      {task.status === 'completed' ? 'Reopen' : 'Complete'}
                    </button>
                    <button
                      className="secondary-button v2-danger-button"
                      disabled={busy}
                      onClick={() => void removeTask(task)}
                      type="button"
                    >
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="v2-concept-card" aria-labelledby="blocks-heading">
          <p className="eyebrow">Calendar</p>
          <h2 id="blocks-heading">Planned blocks</h2>
          <form className="v2-planner-form" onSubmit={addBlock}>
            <label className="v2-field">
              <span>Block title</span>
              <input
                maxLength={200}
                onChange={(event) => setBlockTitle(event.target.value)}
                required
                value={blockTitle}
              />
            </label>
            <div className="v2-planner-form-row">
              <label className="v2-field">
                <span>Start</span>
                <input
                  onChange={(event) => setBlockStart(event.target.value)}
                  type="time"
                  value={blockStart}
                />
              </label>
              <label className="v2-field">
                <span>End</span>
                <input
                  onChange={(event) => setBlockEnd(event.target.value)}
                  type="time"
                  value={blockEnd}
                />
              </label>
              <label className="v2-field">
                <span>Examination</span>
                <select
                  onChange={(event) => setBlockExamId(event.target.value)}
                  value={blockExamId}
                >
                  <option value="">Shared / no exam</option>
                  {exams.map((exam) => (
                    <option key={exam.id} value={exam.id}>
                      {exam.shortName}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            <button className="primary-button" disabled={busy} type="submit">
              Add planned block
            </button>
          </form>

          {!day || day.blocks.length === 0 ? (
            <p className="v2-empty-panel-copy">No planned blocks are scheduled for this date.</p>
          ) : (
            <ul className="v2-planner-list">
              {day.blocks.map((block) => (
                <li className="v2-planner-row" key={block.id}>
                  <div>
                    <div className="v2-session-heading">
                      <strong>{block.title}</strong>
                      <span className="v2-status-badge v2-status-inactive">{block.status}</span>
                      {block.examCode && (
                        <span className="v2-allocation-chip">{block.examCode}</span>
                      )}
                    </div>
                    <p className="v2-empty-copy">
                      {block.startTime}–{block.endTime}
                    </p>
                  </div>
                  <div className="v2-form-actions">
                    <select
                      aria-label={`Status for ${block.title}`}
                      disabled={busy}
                      onChange={(event) => void changeBlockStatus(block, event.target.value)}
                      value={block.status}
                    >
                      <option value="planned">Planned</option>
                      <option value="in_progress">In progress</option>
                      <option value="completed">Completed</option>
                      <option value="cancelled">Cancelled</option>
                    </select>
                    <button
                      className="secondary-button v2-danger-button"
                      disabled={busy}
                      onClick={() => void removeBlock(block)}
                      type="button"
                    >
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
