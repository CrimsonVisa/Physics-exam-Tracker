import { useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { formatDateLong } from '../lib/dateOnly';
import { getAgenda, type Agenda } from '../lib/plannerApi';
import { formatTimeRange } from '../lib/timeOfDay';

type TodayPlanCardProps = {
  today: string;
  onOpenChecklist: () => void;
  onOpenCalendar: () => void;
};

function blockStatusLabel(status: 'planned' | 'completed' | 'cancelled'): string {
  return status === 'completed' ? 'Completed' : status === 'cancelled' ? 'Cancelled' : 'Planned';
}

export function TodayPlanCard({ today, onOpenChecklist, onOpenCalendar }: TodayPlanCardProps) {
  const [agenda, setAgenda] = useState<Agenda | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;

    getAgenda()
      .then((loaded) => {
        if (!cancelled) {
          setAgenda(loaded);
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
  }, [reloadKey, today]);

  return (
    <section className="today-plan-card" aria-labelledby="today-plan-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">Today at a glance</p>
          <h2 id="today-plan-heading">Plan for {formatDateLong(today) ?? today}</h2>
        </div>
        <div className="today-plan-actions">
          <button type="button" className="secondary-button" onClick={onOpenChecklist}>
            Open checklist
          </button>
          <button type="button" className="secondary-button" onClick={onOpenCalendar}>
            Open calendar
          </button>
        </div>
      </div>

      {isLoading && (
        <p className="study-today-state" role="status">
          Loading today&apos;s tasks and planned blocks…
        </p>
      )}

      {!isLoading && loadError && (
        <div className="study-today-error" role="alert">
          <p>{loadError}</p>
          <button
            type="button"
            className="secondary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              setReloadKey((key) => key + 1);
            }}
          >
            Retry plan
          </button>
        </div>
      )}

      {!isLoading && !loadError && agenda && (
        <>
          {agenda.todayBlocks.length === 0 && agenda.todayTasks.length === 0 ? (
            <p className="today-plan-empty">
              Nothing scheduled for today yet. Add a checklist task or a planned study block to give
              the day a shape.
            </p>
          ) : (
            <div className="today-plan-grid">
              <div>
                <h3>Planned blocks</h3>
                {agenda.todayBlocks.length === 0 ? (
                  <p className="today-plan-item-empty">No planned blocks today.</p>
                ) : (
                  <ul className="today-plan-list">
                    {agenda.todayBlocks.map((block) => (
                      <li key={`agenda-block-${block.id}`}>
                        <span className="today-plan-time">
                          {formatTimeRange(block.startTime, block.endTime)}
                        </span>
                        <span className="today-plan-item-title">{block.title}</span>
                        <span className={`block-status block-status-${block.status}`}>
                          {blockStatusLabel(block.status)}
                        </span>
                      </li>
                    ))}
                  </ul>
                )}
              </div>

              <div>
                <h3>Checklist tasks</h3>
                {agenda.todayTasks.length === 0 ? (
                  <p className="today-plan-item-empty">No tasks scheduled today.</p>
                ) : (
                  <ul className="today-plan-list">
                    {agenda.todayTasks.map((task) => (
                      <li key={`agenda-task-${task.id}`}>
                        <span
                          className={`today-plan-item-title${task.status === 'completed' ? ' today-plan-item-done' : ''}`}
                        >
                          {task.title}
                        </span>
                        <span className={`task-status task-status-${task.status}`}>
                          {task.status === 'completed' ? 'Completed' : 'Open'}
                        </span>
                        {task.deadlineDate && (
                          <span className="today-plan-deadline">
                            Deadline {formatDateLong(task.deadlineDate) ?? task.deadlineDate}
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            </div>
          )}

          <dl className="today-plan-meta">
            <div>
              <dt>Overdue tasks</dt>
              <dd>{agenda.overdueTasks.length}</dd>
            </div>
            <div>
              <dt>Deadlines next 7 days</dt>
              <dd>{agenda.upcomingDeadlines.length}</dd>
            </div>
          </dl>

          <p className="today-plan-note">
            Planned blocks are intentions, not study time. Only completed focus sessions appear in
            study totals.
          </p>
        </>
      )}
    </section>
  );
}
