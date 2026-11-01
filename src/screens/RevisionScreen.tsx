import { useEffect, useState } from 'react';
import { RevisionSettingsPanel } from '../components/RevisionSettingsPanel';
import { RevisionTopicDetail } from '../components/RevisionTopicDetail';
import { toCommandError } from '../lib/commandError';
import { formatDateLong } from '../lib/dateOnly';
import type { PlannerTopic } from '../lib/plannerApi';
import {
  getRevisionDashboard,
  scheduleRevisionTopic,
  type RevisionDashboard,
  type RevisionHistoryWithTopic,
  type RevisionRecommendation,
  type RevisionSettings,
  type RevisionTopicState,
} from '../lib/revisionApi';
import { OUTCOME_LABELS } from '../lib/revisionScheduling';

type RevisionFilter = {
  query: string;
  sectionCode: string;
  schedulingStatus: string;
  dueState: string;
};

const EMPTY_FILTER: RevisionFilter = {
  query: '',
  sectionCode: '',
  schedulingStatus: '',
  dueState: '',
};

function dateLabel(value: string | null): string {
  if (!value) {
    return 'Not scheduled';
  }
  return formatDateLong(value) ?? value;
}

function statusLabel(state: RevisionTopicState): string {
  if (!state.schedule) {
    return 'Not scheduled';
  }
  return state.schedule.status === 'suspended' ? 'Suspended' : 'Active';
}

function dueLabel(state: RevisionTopicState): string {
  switch (state.dueState) {
    case 'overdue':
      return `${state.daysOverdue} day${state.daysOverdue === 1 ? '' : 's'} overdue`;
    case 'due_today':
      return 'Due today';
    case 'upcoming':
      return `In ${state.daysUntilDue ?? 0} day${state.daysUntilDue === 1 ? '' : 's'}`;
    case 'suspended':
      return 'Suspended';
    default:
      return 'Not scheduled';
  }
}

function QueueCard({
  title,
  eyebrow,
  items,
  empty,
  onOpen,
}: {
  title: string;
  eyebrow: string;
  items: RevisionTopicState[];
  empty: string;
  onOpen: (topicId: number) => void;
}) {
  const headingId = `revision-queue-${title.toLowerCase().replaceAll(' ', '-')}`;

  return (
    <section className="revision-queue-card" aria-labelledby={headingId}>
      <div>
        <p className="eyebrow">{eyebrow}</p>
        <h2 id={headingId}>{title}</h2>
      </div>
      {items.length === 0 ? (
        <p className="revision-empty">{empty}</p>
      ) : (
        <ul className="revision-queue-list">
          {items.map((state) => (
            <li key={`${title}-${state.topic.id}`}>
              <div>
                <p className="revision-queue-topic">
                  {state.topic.code} · {state.topic.title}
                </p>
                <p className="revision-queue-meta">
                  {state.topic.sectionTitle} · {dueLabel(state)}
                </p>
              </div>
              <button
                type="button"
                className="secondary-button"
                onClick={() => onOpen(state.topic.id)}
              >
                Review
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function RecommendationRow({
  recommendation,
  busy,
  onOpen,
  onSchedule,
}: {
  recommendation: RevisionRecommendation;
  busy: boolean;
  onOpen: (topicId: number) => void;
  onSchedule: (topicId: number) => void;
}) {
  return (
    <article className={`revision-recommendation revision-priority-${recommendation.priority}`}>
      <div className="revision-recommendation-heading">
        <div>
          <h3>
            {recommendation.topic.code} · {recommendation.topic.title}
          </h3>
          <p className="revision-recommendation-meta">
            {recommendation.topic.sectionTitle} ·{' '}
            {recommendation.scheduled
              ? dateLabel(recommendation.nextRevisionDate)
              : 'Not scheduled'}
          </p>
        </div>
        <span className={`revision-priority revision-priority-${recommendation.priority}`}>
          {recommendation.priority} priority
        </span>
      </div>
      <ul className="revision-reasons">
        {recommendation.reasons.map((reason) => (
          <li key={reason}>{reason}</li>
        ))}
      </ul>
      <div className="revision-recommendation-actions">
        {recommendation.scheduled ? (
          <button
            type="button"
            className="secondary-button"
            onClick={() => onOpen(recommendation.topic.id)}
          >
            Open topic
          </button>
        ) : (
          <button
            type="button"
            className="primary-button"
            onClick={() => onSchedule(recommendation.topic.id)}
            disabled={busy}
          >
            {busy ? 'Scheduling…' : 'Add to schedule'}
          </button>
        )}
      </div>
    </article>
  );
}

function RecentRevision({
  item,
  onOpen,
}: {
  item: RevisionHistoryWithTopic;
  onOpen: (topicId: number) => void;
}) {
  return (
    <li>
      <div>
        <p className="revision-queue-topic">
          {item.topic.code} · {item.topic.title}
        </p>
        <p className="revision-queue-meta">
          {dateLabel(item.history.revisionDate)} · {OUTCOME_LABELS[item.history.outcome]} · interval{' '}
          {item.history.intervalAfterDays} days
          {item.history.notes ? ` · ${item.history.notes}` : ''}
        </p>
      </div>
      <button type="button" className="secondary-button" onClick={() => onOpen(item.topic.id)}>
        Open topic
      </button>
    </li>
  );
}

export default function RevisionScreen({ today }: { today: string }) {
  const [dashboard, setDashboard] = useState<RevisionDashboard | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [selectedTopicId, setSelectedTopicId] = useState<number | null>(null);
  const [filter, setFilter] = useState<RevisionFilter>(EMPTY_FILTER);
  const [feedback, setFeedback] = useState<{
    tone: 'success' | 'error';
    message: string;
  } | null>(null);
  const [schedulingTopicId, setSchedulingTopicId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    getRevisionDashboard()
      .then((loaded) => {
        if (!cancelled) {
          setDashboard(loaded);
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
  }, [reloadKey]);

  function reload() {
    setReloadKey((key) => key + 1);
  }

  function openTopic(topicId: number) {
    setSelectedTopicId(topicId);
    setFeedback(null);
  }

  async function handleSchedule(topicId: number) {
    if (schedulingTopicId !== null) {
      return;
    }
    setSchedulingTopicId(topicId);
    setFeedback(null);

    try {
      await scheduleRevisionTopic(topicId, today);
      setSelectedTopicId(topicId);
      setFeedback({
        tone: 'success',
        message: 'Topic added to the revision schedule with today as its first review date.',
      });
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setSchedulingTopicId(null);
    }
  }

  const sections = dashboard
    ? [
        ...new Map(
          dashboard.topics.map((state) => [
            state.topic.sectionCode,
            { code: state.topic.sectionCode, title: state.topic.sectionTitle },
          ]),
        ).values(),
      ]
    : [];

  const filteredTopics =
    dashboard?.topics.filter((state) => {
      const query = filter.query.trim().toLowerCase();
      if (
        query &&
        !state.topic.code.toLowerCase().includes(query) &&
        !state.topic.title.toLowerCase().includes(query)
      ) {
        return false;
      }
      if (filter.sectionCode && state.topic.sectionCode !== filter.sectionCode) {
        return false;
      }
      if (filter.schedulingStatus) {
        const status = state.schedule?.status ?? 'not_scheduled';
        if (status !== filter.schedulingStatus) {
          return false;
        }
      }
      if (filter.dueState && state.dueState !== filter.dueState) {
        return false;
      }
      return true;
    }) ?? [];

  const archivedTopics = dashboard?.archivedTopics ?? [];
  const selectedState =
    dashboard?.topics.find((state) => state.topic.id === selectedTopicId) ??
    archivedTopics.find((state) => state.topic.id === selectedTopicId) ??
    null;
  const plannerTopics: PlannerTopic[] =
    dashboard?.topics.map((state) => ({
      id: state.topic.id,
      code: state.topic.code,
      title: state.topic.title,
      archived: state.topic.archived,
    })) ?? [];

  const targetProgress = dashboard
    ? Math.min(
        100,
        (dashboard.stats.completedToday / Math.max(1, dashboard.settings.dailyRevisionTarget)) *
          100,
      )
    : 0;

  return (
    <div className="page-container revision-page">
      <header className="page-header">
        <p className="eyebrow">Spaced repetition</p>
        <h1>Revision scheduler</h1>
        <p className="page-lede">
          Review topics on a transparent, configurable schedule. Recommendations are advisory:
          nothing is rescheduled or marked mastered without your explicit action.
        </p>
      </header>

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading revision schedule.</p>
          <p className="state-copy">
            Reading saved revision history, due dates, and available PYQ evidence.
          </p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Revision scheduling could not be loaded</p>
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

      {!isLoading && !loadError && dashboard && (
        <>
          <section className="revision-stats" aria-label="Revision summary">
            <div className="revision-stat-card revision-stat-overdue">
              <span>Overdue</span>
              <strong>{dashboard.stats.overdueCount}</strong>
              <small>Needs attention</small>
            </div>
            <div className="revision-stat-card">
              <span>Due today</span>
              <strong>{dashboard.stats.dueTodayCount}</strong>
              <small>Daily target {dashboard.settings.dailyRevisionTarget}</small>
            </div>
            <div className="revision-stat-card">
              <span>Completed today</span>
              <strong>{dashboard.stats.completedToday}</strong>
              <small>{dashboard.stats.completedLast7Days} in the last 7 days</small>
            </div>
            <div className="revision-stat-card">
              <span>Active schedules</span>
              <strong>{dashboard.stats.activeCount}</strong>
              <small>{dashboard.stats.suspendedCount} suspended</small>
            </div>
          </section>

          <div className="revision-target-progress">
            <div className="revision-target-heading">
              <p>Today&apos;s revision target</p>
              <span>
                {dashboard.stats.completedToday} / {dashboard.settings.dailyRevisionTarget}
              </span>
            </div>
            <div
              className="progress-track"
              role="progressbar"
              aria-label="Today's revision target progress"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(targetProgress)}
            >
              <span style={{ width: `${targetProgress}%` }} />
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

          <div className="revision-queue-grid">
            <QueueCard
              title="Overdue revisions"
              eyebrow="Catch up"
              items={dashboard.overdue}
              empty="No overdue revisions."
              onOpen={openTopic}
            />
            <QueueCard
              title="Due today"
              eyebrow="Today's queue"
              items={dashboard.dueToday}
              empty="Nothing due today."
              onOpen={openTopic}
            />
            <QueueCard
              title="Upcoming revisions"
              eyebrow="Next 30 days"
              items={dashboard.upcoming}
              empty="No revisions scheduled in the next 30 days."
              onOpen={openTopic}
            />
          </div>

          <section
            className="revision-recommendations-card"
            aria-labelledby="revision-recommendations-heading"
          >
            <div className="card-heading">
              <div>
                <p className="eyebrow">Advisory only</p>
                <h2 id="revision-recommendations-heading">Recommendations</h2>
              </div>
            </div>
            <p className="settings-lede">
              Recommendations use due dates, previous outcomes, mastery, and PYQ outcomes with
              enough data. They never change the schedule automatically.
            </p>
            {dashboard.recommendations.length === 0 ? (
              <p className="revision-empty">
                No recommendation currently meets the transparent thresholds.
              </p>
            ) : (
              <div className="revision-recommendation-list">
                {dashboard.recommendations.map((recommendation) => (
                  <RecommendationRow
                    key={`recommendation-${recommendation.topic.id}`}
                    recommendation={recommendation}
                    busy={schedulingTopicId === recommendation.topic.id}
                    onOpen={openTopic}
                    onSchedule={(topicId) => void handleSchedule(topicId)}
                  />
                ))}
              </div>
            )}
            <p className="revision-scope-note">{dashboard.mockScopeNote}</p>
          </section>

          <section className="revision-recent-card" aria-labelledby="revision-recent-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Actual saved outcomes</p>
                <h2 id="revision-recent-heading">Completed recently</h2>
              </div>
            </div>
            {dashboard.completedRecent.length === 0 ? (
              <p className="revision-empty">No completed revisions in the last 14 days.</p>
            ) : (
              <ul className="revision-recent-list">
                {dashboard.completedRecent.map((item) => (
                  <RecentRevision
                    key={`recent-${item.history.id}`}
                    item={item}
                    onOpen={openTopic}
                  />
                ))}
              </ul>
            )}
          </section>

          <details className="revision-settings-disclosure">
            <summary>Revision settings and algorithm rules</summary>
            <RevisionSettingsPanel
              settings={dashboard.settings}
              onSaved={(saved: RevisionSettings) => {
                setDashboard((current) => (current ? { ...current, settings: saved } : current));
              }}
            />
          </details>

          <section className="revision-topics-card" aria-labelledby="revision-topics-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Every syllabus topic</p>
                <h2 id="revision-topics-heading">Topic schedules</h2>
              </div>
              <span className="revision-filter-count">
                {filteredTopics.length} of {dashboard.topics.length}
              </span>
            </div>

            <div className="revision-filters">
              <div className="form-field">
                <label htmlFor="revision-filter-query">Search</label>
                <input
                  id="revision-filter-query"
                  type="search"
                  value={filter.query}
                  onChange={(event) =>
                    setFilter((current) => ({ ...current, query: event.target.value }))
                  }
                  placeholder="Topic code or title"
                />
              </div>
              <div className="form-field">
                <label htmlFor="revision-filter-unit">Unit</label>
                <select
                  id="revision-filter-unit"
                  value={filter.sectionCode}
                  onChange={(event) =>
                    setFilter((current) => ({ ...current, sectionCode: event.target.value }))
                  }
                >
                  <option value="">All units</option>
                  {sections.map((section) => (
                    <option value={section.code} key={section.code}>
                      {section.title}
                    </option>
                  ))}
                </select>
              </div>
              <div className="form-field">
                <label htmlFor="revision-filter-status">Scheduling status</label>
                <select
                  id="revision-filter-status"
                  value={filter.schedulingStatus}
                  onChange={(event) =>
                    setFilter((current) => ({
                      ...current,
                      schedulingStatus: event.target.value,
                    }))
                  }
                >
                  <option value="">All scheduling states</option>
                  <option value="active">Active</option>
                  <option value="suspended">Suspended</option>
                  <option value="not_scheduled">Not scheduled</option>
                </select>
              </div>
              <div className="form-field">
                <label htmlFor="revision-filter-due">Due state</label>
                <select
                  id="revision-filter-due"
                  value={filter.dueState}
                  onChange={(event) =>
                    setFilter((current) => ({ ...current, dueState: event.target.value }))
                  }
                >
                  <option value="">Any due state</option>
                  <option value="overdue">Overdue</option>
                  <option value="due_today">Due today</option>
                  <option value="upcoming">Upcoming</option>
                  <option value="suspended">Suspended</option>
                  <option value="not_scheduled">Not scheduled</option>
                </select>
              </div>
              <button
                type="button"
                className="secondary-button"
                onClick={() => setFilter(EMPTY_FILTER)}
              >
                Clear filters
              </button>
            </div>

            {filteredTopics.length === 0 ? (
              <div className="task-empty">
                <p className="task-empty-title">No topics match these filters</p>
                <p>Clear the filters or search for another syllabus topic.</p>
              </div>
            ) : (
              <div className="revision-topic-table-wrap">
                <table className="revision-topic-table">
                  <thead>
                    <tr>
                      <th scope="col">Topic</th>
                      <th scope="col">Unit</th>
                      <th scope="col">Status</th>
                      <th scope="col">Next revision</th>
                      <th scope="col">Interval</th>
                      <th scope="col">Mastery</th>
                      <th scope="col">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredTopics.map((state) => (
                      <tr key={`revision-topic-${state.topic.id}`}>
                        <th scope="row">
                          {state.topic.code} · {state.topic.title}
                          {state.topic.archived ? ' (archived)' : ''}
                        </th>
                        <td>{state.topic.sectionTitle}</td>
                        <td>
                          <span
                            className={`revision-badge revision-badge-${statusLabel(state)
                              .toLowerCase()
                              .replaceAll(' ', '-')}`}
                          >
                            {statusLabel(state)}
                          </span>
                          <span
                            className={`revision-due revision-due-${state.dueState.replaceAll('_', '-')}`}
                          >
                            {dueLabel(state)}
                          </span>
                        </td>
                        <td>{dateLabel(state.schedule?.nextRevisionDate ?? null)}</td>
                        <td>
                          {state.schedule ? `${state.schedule.currentIntervalDays} days` : '—'}
                        </td>
                        <td>
                          {state.schedule ? `${state.schedule.masteryEstimate.toFixed(1)}%` : '—'}
                        </td>
                        <td>
                          <div className="revision-table-actions">
                            <button
                              type="button"
                              className="secondary-button"
                              onClick={() => openTopic(state.topic.id)}
                            >
                              Open
                            </button>
                            {!state.schedule && (
                              <button
                                type="button"
                                className="primary-button"
                                onClick={() => void handleSchedule(state.topic.id)}
                                disabled={schedulingTopicId !== null}
                              >
                                {schedulingTopicId === state.topic.id ? 'Scheduling…' : 'Schedule'}
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          {archivedTopics.length > 0 && (
            <details className="revision-settings-disclosure">
              <summary>Archived revision history ({archivedTopics.length})</summary>
              <p className="settings-lede">
                Archived topics keep their schedules and revision history. Open a topic to view it
                read-only; unarchive the topic in Syllabus to schedule new revisions.
              </p>
              <div className="revision-topic-table-wrap">
                <table className="revision-topic-table">
                  <thead>
                    <tr>
                      <th scope="col">Topic</th>
                      <th scope="col">Unit</th>
                      <th scope="col">Status</th>
                      <th scope="col">Last revision</th>
                      <th scope="col">Completed</th>
                      <th scope="col">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    {archivedTopics.map((state) => (
                      <tr key={`archived-revision-topic-${state.topic.id}`}>
                        <th scope="row">
                          {state.topic.code} · {state.topic.title}
                        </th>
                        <td>{state.topic.sectionTitle}</td>
                        <td>{statusLabel(state)}</td>
                        <td>{dateLabel(state.schedule?.lastRevisionDate ?? null)}</td>
                        <td>{state.schedule?.completedRevisions ?? 0}</td>
                        <td>
                          <button
                            type="button"
                            className="secondary-button"
                            onClick={() => openTopic(state.topic.id)}
                          >
                            View history
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </details>
          )}

          {selectedState && (
            <RevisionTopicDetail
              key={`revision-detail-${selectedState.topic.id}`}
              state={selectedState}
              settings={dashboard.settings}
              today={today}
              plannerTopics={plannerTopics}
              onChanged={reload}
              onClose={() => setSelectedTopicId(null)}
              readOnly={selectedState.topic.archived}
            />
          )}
        </>
      )}
    </div>
  );
}
