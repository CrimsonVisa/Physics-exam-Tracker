import { useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { formatDateLong } from '../lib/dateOnly';
import {
  getMockAnalytics,
  getPyqAnalytics,
  getStudyPerformance,
  type MockAnalytics,
  type PyqAnalytics,
  type StudyPerformance,
} from '../lib/analyticsApi';
import { formatStudyDuration } from '../lib/studyTime';

type AnalyticsData = {
  pyq: PyqAnalytics;
  mock: MockAnalytics;
  study: StudyPerformance;
};

function accuracyLabel(value: number | null): string {
  return value === null ? '—' : `${value.toFixed(1)}%`;
}

function percentageLabel(value: number | null): string {
  return value === null ? 'No data' : `${value.toFixed(1)}%`;
}

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

export default function AnalyticsScreen() {
  const [data, setData] = useState<AnalyticsData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);

  useEffect(() => {
    let cancelled = false;

    Promise.all([getPyqAnalytics(), getMockAnalytics(), getStudyPerformance()])
      .then(([pyq, mock, study]) => {
        if (!cancelled) {
          setData({ pyq, mock, study });
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
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }

  const weakTopicIds = new Set(data?.pyq.weakTopics.map((topic) => topic.topicId) ?? []);
  const insufficientTopics = data?.pyq.byTopic.filter((topic) => !topic.sufficientData) ?? [];

  return (
    <div className="page-container analytics-page">
      <header className="page-header">
        <p className="eyebrow">Analytics</p>
        <h1>Study performance</h1>
        <p className="page-lede">
          These views use only saved PYQ attempts, mock tests, and study sessions. Accuracy is
          correct divided by correct plus incorrect; skipped and outcome-unknown attempts are shown
          separately.
        </p>
      </header>

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Calculating analytics.</p>
          <p className="state-copy">Reading saved outcomes, mock scores, and study time.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Analytics could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button type="button" className="primary-button" onClick={reload}>
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && data && (
        <>
          <section className="analytics-overview" aria-label="Performance overview">
            <div className="analytics-stat-card">
              <span>PYQ records</span>
              <strong>{data.pyq.totalRecords}</strong>
              <small>
                {data.pyq.correct} correct · {data.pyq.incorrect} incorrect · {data.pyq.skipped}{' '}
                skipped
              </small>
            </div>
            <div className="analytics-stat-card">
              <span>PYQ accuracy</span>
              <strong>{accuracyLabel(data.pyq.accuracy)}</strong>
              <small>
                {data.pyq.attempts} scored attempts
                {data.pyq.outcomeUnknown > 0
                  ? ` · ${data.pyq.outcomeUnknown} without a recorded outcome`
                  : ''}
              </small>
            </div>
            <div className="analytics-stat-card">
              <span>Mock tests</span>
              <strong>{data.mock.testCount}</strong>
              <small>Average calculated score {percentageLabel(data.mock.averagePercentage)}</small>
            </div>
            <div className="analytics-stat-card">
              <span>Latest mock score</span>
              <strong>{percentageLabel(data.mock.latestPercentage)}</strong>
              <small>Best {percentageLabel(data.mock.bestPercentage)}</small>
            </div>
          </section>

          <section className="analytics-card" aria-labelledby="weak-topic-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Screening, not a verdict</p>
                <h2 id="weak-topic-heading">Weak-topic identification</h2>
              </div>
            </div>
            <p className="settings-lede">
              A topic is labelled weak only after at least {data.pyq.minimumWeakAttempts}{' '}
              correct-or-incorrect attempts and accuracy below{' '}
              {data.pyq.weakAccuracyThreshold.toFixed(0)}%. Topics with fewer attempts are shown as
              insufficient data, not as poor performance.
            </p>

            {data.pyq.byTopic.length === 0 ? (
              <p className="analytics-empty">No PYQ topic outcomes have been recorded.</p>
            ) : (
              <div className="weak-topic-grid">
                <div>
                  <h3>Weak topics with sufficient data</h3>
                  {data.pyq.weakTopics.length === 0 ? (
                    <p className="analytics-empty">
                      No topic has enough recorded attempts to be labelled weak.
                    </p>
                  ) : (
                    <ul className="analytics-list">
                      {data.pyq.weakTopics.map((topic) => (
                        <li key={`weak-${topic.topicId}`}>
                          <span>
                            {topic.code} · {topic.title}
                          </span>
                          <strong>
                            {accuracyLabel(topic.accuracy)} ({topic.attempts} attempts)
                          </strong>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
                <div>
                  <h3>Insufficient data</h3>
                  {insufficientTopics.length === 0 ? (
                    <p className="analytics-empty">
                      Every attempted topic has at least {data.pyq.minimumWeakAttempts} scored
                      attempts.
                    </p>
                  ) : (
                    <ul className="analytics-list analytics-list-muted">
                      {insufficientTopics.map((topic) => (
                        <li key={`insufficient-${topic.topicId}`}>
                          <span>
                            {topic.code} · {topic.title}
                          </span>
                          <strong>
                            {topic.attempts} scored {topic.attempts === 1 ? 'attempt' : 'attempts'}
                            {topic.accuracy === null ? '' : ` · ${accuracyLabel(topic.accuracy)}`}
                          </strong>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            )}
          </section>

          <section className="analytics-card" aria-labelledby="topic-performance-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">PYQ outcomes</p>
                <h2 id="topic-performance-heading">Accuracy by topic</h2>
              </div>
            </div>
            {data.pyq.byTopic.length === 0 ? (
              <p className="analytics-empty">No topic-level PYQ data yet.</p>
            ) : (
              <div className="analytics-table-wrap">
                <table className="analytics-table">
                  <thead>
                    <tr>
                      <th scope="col">Topic</th>
                      <th scope="col">Unit</th>
                      <th scope="col">Attempts</th>
                      <th scope="col">Correct</th>
                      <th scope="col">Incorrect</th>
                      <th scope="col">Skipped</th>
                      <th scope="col">Accuracy</th>
                      <th scope="col">Data status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.pyq.byTopic.map((topic) => (
                      <tr key={`topic-${topic.topicId}`}>
                        <th scope="row">
                          {topic.code} · {topic.title}
                          {topic.archived ? ' (archived)' : ''}
                        </th>
                        <td>{topic.sectionTitle}</td>
                        <td>{topic.attempts}</td>
                        <td>{topic.correct}</td>
                        <td>{topic.incorrect}</td>
                        <td>{topic.skipped}</td>
                        <td>{accuracyLabel(topic.accuracy)}</td>
                        <td>
                          {!topic.sufficientData ? (
                            <span className="analytics-tag analytics-tag-insufficient">
                              Insufficient data
                            </span>
                          ) : weakTopicIds.has(topic.topicId) ? (
                            <span className="analytics-tag analytics-tag-weak">Weak</span>
                          ) : (
                            <span className="analytics-tag analytics-tag-ok">Sufficient</span>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          <div className="analytics-two-column">
            <section className="analytics-card" aria-labelledby="unit-performance-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">PYQ outcomes</p>
                  <h2 id="unit-performance-heading">By unit</h2>
                </div>
              </div>
              {data.pyq.byUnit.length === 0 ? (
                <p className="analytics-empty">No unit-level PYQ data yet.</p>
              ) : (
                <table className="analytics-table analytics-table-compact">
                  <thead>
                    <tr>
                      <th scope="col">Unit</th>
                      <th scope="col">Attempts</th>
                      <th scope="col">Accuracy</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.pyq.byUnit.map((unit) => (
                      <tr key={`unit-${unit.code}`}>
                        <th scope="row">{unit.title}</th>
                        <td>{unit.attempts}</td>
                        <td>{accuracyLabel(unit.accuracy)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            <section className="analytics-card" aria-labelledby="year-performance-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">PYQ outcomes</p>
                  <h2 id="year-performance-heading">By examination year</h2>
                </div>
              </div>
              {data.pyq.byYear.length === 0 ? (
                <p className="analytics-empty">No PYQ year data yet.</p>
              ) : (
                <table className="analytics-table analytics-table-compact">
                  <thead>
                    <tr>
                      <th scope="col">Year</th>
                      <th scope="col">Attempts</th>
                      <th scope="col">Correct</th>
                      <th scope="col">Incorrect</th>
                      <th scope="col">Accuracy</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.pyq.byYear.map((year) => (
                      <tr key={`year-${year.year}`}>
                        <th scope="row">{year.year}</th>
                        <td>{year.attempts}</td>
                        <td>{year.correct}</td>
                        <td>{year.incorrect}</td>
                        <td>{accuracyLabel(year.accuracy)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>
          </div>

          <section className="analytics-card" aria-labelledby="pyq-part-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">PYQ outcomes</p>
                <h2 id="pyq-part-heading">Part-wise accuracy</h2>
              </div>
            </div>
            {data.pyq.byPart.length === 0 ? (
              <p className="analytics-empty">No PYQ part data yet.</p>
            ) : (
              <div className="analytics-part-grid">
                {data.pyq.byPart.map((part) => (
                  <div className="analytics-part-card" key={`pyq-part-${part.part}`}>
                    <span>Part {part.part}</span>
                    <strong>{accuracyLabel(part.accuracy)}</strong>
                    <small>
                      {part.correct} correct · {part.incorrect} incorrect · {part.skipped} skipped
                    </small>
                  </div>
                ))}
              </div>
            )}
          </section>

          <section className="analytics-card" aria-labelledby="mock-trend-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Mock tests</p>
                <h2 id="mock-trend-heading">Calculated-score trend</h2>
              </div>
            </div>
            {data.mock.trend.length === 0 ? (
              <p className="analytics-empty">No mock-test scores yet.</p>
            ) : (
              <div className="analytics-table-wrap">
                <table className="analytics-table">
                  <thead>
                    <tr>
                      <th scope="col">Date</th>
                      <th scope="col">Test</th>
                      <th scope="col">Calculated</th>
                      <th scope="col">Percentage</th>
                      <th scope="col">Reported</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.mock.trend.map((point) => (
                      <tr key={`trend-${point.id}`}>
                        <th scope="row">{dateLabel(point.testDate)}</th>
                        <td>{point.testName}</td>
                        <td>
                          {point.computedMarks.toFixed(2)} / {point.maxMarks.toFixed(2)}
                        </td>
                        <td>
                          <div className="analytics-score-cell">
                            <span>{point.computedPercentage.toFixed(1)}%</span>
                            <div className="analytics-bar" aria-hidden="true">
                              <span
                                style={{
                                  width: `${Math.max(0, Math.min(100, point.computedPercentage))}%`,
                                }}
                              />
                            </div>
                          </div>
                        </td>
                        <td>
                          {point.reportedMarks === null
                            ? 'Not recorded'
                            : point.reportedMarks.toFixed(2)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </section>

          <div className="analytics-two-column">
            <section className="analytics-card" aria-labelledby="mock-part-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Mock tests</p>
                  <h2 id="mock-part-heading">Part-wise performance</h2>
                </div>
              </div>
              {data.mock.byPart.every((part) => part.attempted === 0) ? (
                <p className="analytics-empty">No attempted mock-test questions yet.</p>
              ) : (
                <table className="analytics-table analytics-table-compact">
                  <thead>
                    <tr>
                      <th scope="col">Part</th>
                      <th scope="col">Attempted</th>
                      <th scope="col">Correct</th>
                      <th scope="col">Incorrect</th>
                      <th scope="col">Accuracy</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.mock.byPart.map((part) => (
                      <tr key={`mock-part-${part.part}`}>
                        <th scope="row">Part {part.part}</th>
                        <td>{part.attempted}</td>
                        <td>{part.correct}</td>
                        <td>{part.incorrect}</td>
                        <td>{accuracyLabel(part.accuracy)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </section>

            <section className="analytics-card" aria-labelledby="study-performance-heading">
              <div className="card-heading">
                <div>
                  <p className="eyebrow">Descriptive only</p>
                  <h2 id="study-performance-heading">Study time and outcomes</h2>
                </div>
              </div>
              <p className="analytics-note">
                Paired by local date. This is not a causal claim, and days without enough outcomes
                remain visibly incomplete.
              </p>
              {data.study.days.length === 0 ? (
                <p className="analytics-empty">
                  No dates have both saved outcomes and study-time context yet.
                </p>
              ) : (
                <div className="analytics-table-wrap analytics-table-scroll">
                  <table className="analytics-table analytics-table-compact">
                    <thead>
                      <tr>
                        <th scope="col">Date</th>
                        <th scope="col">Study time</th>
                        <th scope="col">PYQ outcomes</th>
                        <th scope="col">Mock tests</th>
                      </tr>
                    </thead>
                    <tbody>
                      {data.study.days.map((day) => (
                        <tr key={`study-${day.date}`}>
                          <th scope="row">{dateLabel(day.date)}</th>
                          <td>{formatStudyDuration(day.studySeconds)}</td>
                          <td>
                            {day.pyqCorrect + day.pyqIncorrect === 0
                              ? 'No PYQ outcomes'
                              : `${day.pyqCorrect} correct · ${day.pyqIncorrect} incorrect · ${accuracyLabel(day.pyqAccuracy)}`}
                          </td>
                          <td>
                            {day.mockCount === 0
                              ? 'None'
                              : `${day.mockCount} · avg ${percentageLabel(day.mockAveragePercentage)}`}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>
          </div>
        </>
      )}
    </div>
  );
}
