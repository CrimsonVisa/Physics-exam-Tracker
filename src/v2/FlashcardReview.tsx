import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2FlashcardReviewQueue,
  reviewV2Flashcard,
  type V2FlashcardQueue,
  type V2FlashcardReviewCard,
  type V2ExamSummary,
} from './api';
import { MathText } from './MathText';

type ReviewMode = 'due' | 'all';
type Rating = 'again' | 'hard' | 'good' | 'easy';

type FlashcardReviewProps = {
  exams: V2ExamSummary[];
  today: string;
  examId: number | null;
  subjectArea: string;
  nodeId: number | null;
  conceptSlug: string | null;
  onBack: () => void;
  onReviewed: () => void;
};

const RATINGS: Array<{ value: Rating; label: string; hint: string }> = [
  { value: 'again', label: 'Again', hint: '1 day' },
  { value: 'hard', label: 'Hard', hint: 'shorter step' },
  { value: 'good', label: 'Good', hint: 'normal step' },
  { value: 'easy', label: 'Easy', hint: 'longer step' },
];

function examName(exams: V2ExamSummary[], examId: number | null): string {
  if (examId === null) {
    return 'All examinations';
  }
  return exams.find((exam) => exam.id === examId)?.shortName ?? 'Examination';
}

export function FlashcardReview({
  exams,
  today,
  examId,
  subjectArea,
  nodeId,
  conceptSlug,
  onBack,
  onReviewed,
}: FlashcardReviewProps) {
  const [mode, setMode] = useState<ReviewMode>('due');
  const [queue, setQueue] = useState<V2FlashcardQueue | null>(null);
  const [index, setIndex] = useState(0);
  const [revealed, setRevealed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [counts, setCounts] = useState<Record<Rating, number>>({
    again: 0,
    hard: 0,
    good: 0,
    easy: 0,
  });
  const [finished, setFinished] = useState(false);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const next = await getV2FlashcardReviewQueue({
        examId,
        subjectArea: subjectArea.trim() || undefined,
        nodeId,
        conceptSlug: conceptSlug ?? undefined,
        dueOnly: mode === 'due',
        today,
        limit: 100,
      });
      setQueue(next);
      setIndex(0);
      setRevealed(false);
      setFinished(next.cards.length === 0);
      setCounts({ again: 0, hard: 0, good: 0, easy: 0 });
    } catch (loadError) {
      setError(toCommandError(loadError).message);
      setQueue(null);
      setFinished(true);
    } finally {
      setBusy(false);
    }
  }, [conceptSlug, examId, mode, nodeId, subjectArea, today]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (!cancelled) {
        void load();
      }
    }, 0);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [load]);

  const current: V2FlashcardReviewCard | null = queue?.cards[index] ?? null;

  async function rate(rating: Rating) {
    if (!current || busy) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await reviewV2Flashcard({ flashcardId: current.id, rating, today });
      setCounts((currentCounts) => ({
        ...currentCounts,
        [rating]: currentCounts[rating] + 1,
      }));
      onReviewed();
      if (index + 1 >= (queue?.cards.length ?? 0)) {
        setFinished(true);
      } else {
        setIndex((value) => value + 1);
        setRevealed(false);
      }
    } catch (reviewError) {
      setError(toCommandError(reviewError).message);
    } finally {
      setBusy(false);
    }
  }

  const totalRated = counts.again + counts.hard + counts.good + counts.easy;

  return (
    <section className="v2-flashcard-review-page" aria-labelledby="flashcard-review-heading">
      <header className="v2-flashcard-review-head">
        <div>
          <p className="eyebrow">Review</p>
          <h2 id="flashcard-review-heading">Flashcard review</h2>
          <p className="v2-empty-copy">
            {examName(exams, examId)}
            {subjectArea ? ` · ${subjectArea}` : ''}
            {conceptSlug ? ' · concept filter active' : ''}
          </p>
        </div>
        <div className="v2-flashcard-mode-toggle" role="group" aria-label="Review mode">
          <button
            aria-pressed={mode === 'due'}
            className="nav-button"
            onClick={() => setMode('due')}
            type="button"
          >
            Due today
          </button>
          <button
            aria-pressed={mode === 'all'}
            className="nav-button"
            onClick={() => setMode('all')}
            type="button"
          >
            Practice all
          </button>
        </div>
      </header>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}

      {busy && !queue ? (
        <p className="v2-empty-panel-copy">Loading cards…</p>
      ) : finished ? (
        <div className="v2-flashcard-session-summary">
          <p className="eyebrow">Session complete</p>
          <h3>{totalRated === 0 ? 'Nothing to review right now' : 'Review saved'}</h3>
          <p className="v2-empty-copy">
            {mode === 'due'
              ? 'No cards are due in this scope.'
              : 'No cards match the current scope.'}
          </p>
          {totalRated > 0 && (
            <dl className="v2-fact-list">
              {RATINGS.map((rating) => (
                <div key={rating.value}>
                  <dt>{rating.label}</dt>
                  <dd>{counts[rating.value]}</dd>
                </div>
              ))}
            </dl>
          )}
          <div className="v2-form-actions">
            {mode === 'due' && (
              <button className="secondary-button" onClick={() => setMode('all')} type="button">
                Practice all cards
              </button>
            )}
            {queue !== null && queue.remaining > 0 && totalRated > 0 && (
              <button className="secondary-button" onClick={() => void load()} type="button">
                Continue reviewing
              </button>
            )}
            <button className="primary-button" onClick={onBack} type="button">
              Back to library
            </button>
          </div>
        </div>
      ) : current ? (
        <article className="v2-flashcard-review-card">
          <div className="v2-flashcard-review-meta">
            <span>
              Card {index + 1} of {queue?.cards.length ?? 0}
              {(queue?.remaining ?? 0) > 0 ? ` · ${queue?.remaining} more queued` : ''}
            </span>
            <span>{current.subjectArea}</span>
            {current.conceptTitle && <span>{current.conceptTitle}</span>}
          </div>
          <div className="v2-flashcard-review-content">
            <MathText text={current.front} />
          </div>
          {!revealed ? (
            <button
              className="primary-button v2-flashcard-reveal"
              onClick={() => setRevealed(true)}
              type="button"
            >
              Reveal answer
            </button>
          ) : (
            <>
              <div className="v2-flashcard-review-divider" />
              <div className="v2-flashcard-review-content v2-flashcard-review-back">
                <MathText text={current.back} />
              </div>
              <div className="v2-flashcard-ratings" role="group" aria-label="Rate this review">
                {RATINGS.map((rating) => (
                  <button
                    className={`secondary-button v2-flashcard-rating v2-flashcard-rating-${rating.value}`}
                    disabled={busy}
                    key={rating.value}
                    onClick={() => void rate(rating.value)}
                    type="button"
                  >
                    <strong>{rating.label}</strong>
                    <span>{rating.hint}</span>
                  </button>
                ))}
              </div>
            </>
          )}
          <p className="v2-empty-copy">
            Good and Easy count as successful recall for the retention figure. Again and Hard are
            reported separately; these counts are never a mastery score.
          </p>
        </article>
      ) : (
        <p className="v2-empty-panel-copy">Loading cards…</p>
      )}

      <div className="v2-form-actions">
        <button className="secondary-button" onClick={onBack} type="button">
          End review
        </button>
      </div>
    </section>
  );
}
