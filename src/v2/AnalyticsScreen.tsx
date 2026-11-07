import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { getV2Analytics, type V2Analytics, type V2ExamSummary } from './api';

type AnalyticsScreenProps = {
  exams: V2ExamSummary[];
  today: string;
};

function duration(seconds: number): string {
  const safeSeconds = Math.max(0, Math.floor(seconds));
  if (safeSeconds === 0) {
    return '0 min';
  }
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);
  const remainingSeconds = safeSeconds % 60;
  if (hours > 0) {
    return minutes > 0 ? `${hours} h ${minutes} min` : `${hours} h`;
  }
  if (minutes > 0) {
    return remainingSeconds > 0 ? `${minutes} min ${remainingSeconds} s` : `${minutes} min`;
  }
  return `${remainingSeconds} s`;
}

function examShortName(exams: V2ExamSummary[], code: string): string {
  return exams.find((exam) => exam.code === code)?.shortName ?? code;
}

export function AnalyticsScreen({ exams, today }: AnalyticsScreenProps) {
  const [examCode, setExamCode] = useState('');
  const [windowDays, setWindowDays] = useState('30');
  const [analytics, setAnalytics] = useState<V2Analytics | null>(null);
  const [error, setError] = useState<string | null>(null);
  const examId = exams.find((exam) => exam.code === examCode)?.id ?? null;

  const load = useCallback(
    async (selectedExamId: number | null, days: number) => {
      try {
        setAnalytics(await getV2Analytics({ examId: selectedExamId, today, windowDays: days }));
        setError(null);
      } catch (loadError) {
        setError(toCommandError(loadError).message);
      }
    },
    [today],
  );

  useEffect(() => {
    void (async () => {
      await load(examId, Number(windowDays));
    })();
  }, [examId, load, windowDays]);

  return (
    <div className="page-container v2-analytics-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · real data</p>
        <h1>Analytics</h1>
        <p className="page-lede">
          Recorded study time, daily and weekly trends, concept coverage, mastery distribution, and
          revision completion. Readiness is not shown because its weights and minimum samples are
          still unapproved.
        </p>
      </header>

      <div className="v2-planner-datebar">
        <label className="v2-field">
          <span>Examination scope</span>
          <select onChange={(event) => setExamCode(event.target.value)} value={examCode}>
            <option value="">Combined</option>
            {exams.map((exam) => (
              <option key={exam.code} value={exam.code}>
                {exam.shortName}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Window</span>
          <select onChange={(event) => setWindowDays(event.target.value)} value={windowDays}>
            <option value="7">7 days</option>
            <option value="30">30 days</option>
            <option value="90">90 days</option>
            <option value="365">365 days</option>
          </select>
        </label>
      </div>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}

      {analytics && (
        <>
          <section className="v2-coverage-grid">
            <article className="v2-coverage-card">
              <span className="v2-status-badge v2-status-active">Study time</span>
              <h2>{duration(analytics.totalSeconds)}</h2>
              <p className="v2-empty-copy">
                {analytics.sessionCount} session{analytics.sessionCount === 1 ? '' : 's'} in{' '}
                {analytics.rangeFrom} to {analytics.rangeTo}.
              </p>
            </article>
            <article className="v2-coverage-card">
              <span className="v2-status-badge v2-status-inactive">Allocated</span>
              <h2>{duration(analytics.allocatedSeconds)}</h2>
              <p className="v2-empty-copy">
                Unallocated: {duration(analytics.unallocatedSeconds)}.
              </p>
              <p className="v2-empty-copy">
                Topic-allocated: {duration(analytics.topicAllocatedSeconds)} · without topic
                allocation: {duration(analytics.topicUnallocatedSeconds)}.
              </p>
            </article>
            <article className="v2-coverage-card">
              <span className="v2-status-badge v2-status-inactive">Consistency</span>
              <h2>{analytics.consistencyPercent.toFixed(1)}%</h2>
              <p className="v2-empty-copy">
                {analytics.activeDays} active day{analytics.activeDays === 1 ? '' : 's'} in the
                selected window.
              </p>
            </article>
            <article className="v2-coverage-card">
              <span className="v2-status-badge v2-status-inactive">Revision</span>
              <h2>{analytics.revisionCompletedLast30Days}</h2>
              <p className="v2-empty-copy">
                Due {analytics.dueRevisionCount} · overdue {analytics.overdueRevisionCount} · active
                schedules {analytics.activeRevisionSchedules}.
              </p>
            </article>
          </section>

          <div className="v2-planner-grid">
            <section className="v2-concept-card">
              <p className="eyebrow">Daily trend</p>
              <h2>Recorded study by day</h2>
              {analytics.daily.length === 0 ? (
                <p className="v2-empty-panel-copy">No study sessions in this window.</p>
              ) : (
                <ul className="v2-trend-list">
                  {analytics.daily.map((point) => (
                    <li key={point.label}>
                      <span>{point.label}</span>
                      <strong>{duration(point.seconds)}</strong>
                      <span className="v2-session-date">{point.sessionCount} session(s)</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <section className="v2-concept-card">
              <p className="eyebrow">Weekly trend</p>
              <h2>Recorded study by week</h2>
              {analytics.weekly.length === 0 ? (
                <p className="v2-empty-panel-copy">No weekly study data yet.</p>
              ) : (
                <ul className="v2-trend-list">
                  {analytics.weekly.map((point) => (
                    <li key={point.label}>
                      <span>{point.label}</span>
                      <strong>{duration(point.seconds)}</strong>
                      <span className="v2-session-date">{point.sessionCount} session(s)</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>

          <div className="v2-planner-grid">
            <section className="v2-concept-card">
              <p className="eyebrow">Subject breakdown</p>
              <h2>Study duration by subject</h2>
              {analytics.subjects.length === 0 ? (
                <p className="v2-empty-panel-copy">
                  No topic-allocated study time in this window. Sessions without topic allocations
                  remain included in combined study time.
                </p>
              ) : (
                <ul className="v2-trend-list">
                  {analytics.subjects.map((subject) => (
                    <li key={`${subject.examId}-${subject.subjectTitle}`}>
                      <span>
                        {examShortName(exams, subject.examCode)} · {subject.subjectTitle}
                      </span>
                      <strong>{duration(subject.allocatedSeconds)}</strong>
                      <span className="v2-session-date">{subject.sessionCount} session(s)</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
            <section className="v2-concept-card">
              <p className="eyebrow">Topic breakdown</p>
              <h2>Study duration by topic</h2>
              {analytics.topics.length === 0 ? (
                <p className="v2-empty-panel-copy">
                  No topic-allocated study time in this window. Unallocated study remains counted
                  once in combined totals.
                </p>
              ) : (
                <ul className="v2-trend-list">
                  {analytics.topics.map((topic) => (
                    <li key={`${topic.examId}-${topic.nodeId}`}>
                      <span>
                        {examShortName(exams, topic.examCode)} · {topic.subjectTitle} ·{' '}
                        {topic.nodeTitle}
                      </span>
                      <strong>{duration(topic.allocatedSeconds)}</strong>
                      <span className="v2-session-date">{topic.sessionCount} session(s)</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>

          <div className="v2-planner-grid">
            <section className="v2-concept-card">
              <p className="eyebrow">Coverage and mastery</p>
              <h2>Concept state</h2>
              <dl className="v2-fact-list">
                <div>
                  <dt>Approved mapped concepts</dt>
                  <dd>{analytics.mappedConceptCount}</dd>
                </div>
                <div>
                  <dt>With evidence</dt>
                  <dd>{analytics.evidencedConceptCount}</dd>
                </div>
                <div>
                  <dt>Average mastery</dt>
                  <dd>
                    {analytics.averageMastery === null
                      ? 'No evidenced concepts'
                      : analytics.averageMastery.toFixed(1)}
                  </dd>
                </div>
                <div>
                  <dt>Readiness</dt>
                  <dd>Unavailable — formula approval pending</dd>
                </div>
              </dl>
            </section>
            <section className="v2-concept-card">
              <p className="eyebrow">Mastery distribution</p>
              <h2>Evidenced concepts</h2>
              {analytics.evidencedConceptCount === 0 ? (
                <p className="v2-empty-panel-copy">
                  No concept has mastery evidence yet. No zero values are fabricated.
                </p>
              ) : (
                <ul className="v2-trend-list">
                  {analytics.masteryBuckets.map((bucket) => (
                    <li key={bucket.label}>
                      <span>{bucket.label}</span>
                      <strong>{bucket.conceptCount}</strong>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>
        </>
      )}
    </div>
  );
}
