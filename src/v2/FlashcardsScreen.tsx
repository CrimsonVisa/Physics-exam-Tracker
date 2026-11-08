import { useCallback, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2ExamSyllabus,
  getV2FlashcardDetail,
  getV2FlashcardStats,
  listV2Flashcards,
  setV2FlashcardArchived,
  type V2ExamSyllabus,
  type V2ExamSummary,
  type V2FlashcardDetail,
  type V2FlashcardLibrary,
  type V2FlashcardStats,
  type V2FlashcardSummary,
} from './api';
import { FlashcardEditor } from './FlashcardEditor';
import { FlashcardReview } from './FlashcardReview';
import { MathText } from './MathText';

type FlashcardsScreenProps = {
  exams: V2ExamSummary[];
  today: string;
  initialExamCode?: string | null;
  initialNodeId?: number | null;
  initialNodeTitle?: string | null;
  initialConceptSlug?: string | null;
  initialConceptTitle?: string | null;
  initialFlashcardId?: number | null;
  initialQuery?: string | null;
};

type Tab = 'library' | 'review';

function formatPercent(value: number | null): string {
  return value === null ? '—' : `${value.toFixed(1)}%`;
}

function scheduleLabel(card: V2FlashcardSummary, today: string): string {
  if (card.schedule.reviewCount === 0) {
    return 'New · due today';
  }
  if (card.schedule.dueOn <= today) {
    return `Due ${card.schedule.dueOn}`;
  }
  return `Next ${card.schedule.dueOn}`;
}

export function FlashcardsScreen({
  exams,
  today,
  initialExamCode = null,
  initialNodeId = null,
  initialNodeTitle = null,
  initialConceptSlug = null,
  initialConceptTitle = null,
  initialFlashcardId = null,
  initialQuery = null,
}: FlashcardsScreenProps) {
  const initialExam = exams.find((exam) => exam.code === initialExamCode);
  const [examId, setExamId] = useState<number | null>(initialExam?.id ?? null);
  const [subjectArea, setSubjectArea] = useState('');
  const [nodeId, setNodeId] = useState<number | null>(initialNodeId);
  const [nodeTitle, setNodeTitle] = useState(initialNodeTitle ?? '');
  const [conceptSlug, setConceptSlug] = useState<string | null>(initialConceptSlug);
  const [conceptTitle, setConceptTitle] = useState(initialConceptTitle ?? '');
  const [query, setQuery] = useState(initialQuery ?? '');
  const [includeArchived, setIncludeArchived] = useState(false);
  const [dueOnly, setDueOnly] = useState(false);
  const [sort, setSort] = useState('due');
  const [library, setLibrary] = useState<V2FlashcardLibrary | null>(null);
  const [stats, setStats] = useState<V2FlashcardStats | null>(null);
  const [syllabus, setSyllabus] = useState<V2ExamSyllabus | null>(null);
  const [detail, setDetail] = useState<V2FlashcardDetail | null>(null);
  const [tab, setTab] = useState<Tab>('library');
  const [editorOpen, setEditorOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadLibrary = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextLibrary, nextStats] = await Promise.all([
        listV2Flashcards({
          query: query.trim() || undefined,
          examId,
          subjectArea: subjectArea || undefined,
          nodeId,
          conceptSlug: conceptSlug ?? undefined,
          includeArchived,
          dueOnly,
          sort,
          today,
        }),
        getV2FlashcardStats({
          query: query.trim() || undefined,
          examId,
          subjectArea: subjectArea || undefined,
          nodeId,
          conceptSlug: conceptSlug ?? undefined,
          includeArchived,
          today,
        }),
      ]);
      setLibrary(nextLibrary);
      setStats(nextStats);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setLoading(false);
    }
  }, [conceptSlug, dueOnly, examId, includeArchived, query, sort, subjectArea, nodeId, today]);

  useEffect(() => {
    void (async () => {
      await loadLibrary();
    })();
  }, [loadLibrary]);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (examId === null) {
        setSyllabus(null);
        return;
      }
      void (async () => {
        try {
          const next = await getV2ExamSyllabus(examId);
          if (!cancelled) {
            setSyllabus(next);
          }
        } catch {
          if (!cancelled) {
            setSyllabus(null);
          }
        }
      })();
    }, 0);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [examId]);

  useEffect(() => {
    if (initialFlashcardId === null) {
      return undefined;
    }
    let cancelled = false;
    void (async () => {
      try {
        const next = await getV2FlashcardDetail(initialFlashcardId);
        if (!cancelled) {
          setDetail(next);
          setTab('library');
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(toCommandError(loadError).message);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [initialFlashcardId]);

  function clearNodeFilter() {
    setNodeId(null);
    setNodeTitle('');
  }

  function clearConceptFilter() {
    setConceptSlug(null);
    setConceptTitle('');
  }

  async function openCard(card: V2FlashcardSummary) {
    setBusy(true);
    setError(null);
    try {
      setDetail(await getV2FlashcardDetail(card.id));
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setBusy(false);
    }
  }

  async function setArchived(card: V2FlashcardDetail, archived: boolean) {
    const action = archived ? 'Archive' : 'Restore';
    if (
      !window.confirm(
        `${action} “${card.flashcard.frontPreview}”?${
          archived ? ' Review history is preserved while the card is archived.' : ''
        }`,
      )
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const next = await setV2FlashcardArchived({
        flashcardId: card.flashcard.id,
        archived,
      });
      setDetail(next);
      await loadLibrary();
    } catch (archiveError) {
      setError(toCommandError(archiveError).message);
    } finally {
      setBusy(false);
    }
  }

  const selectedSubjects = library?.subjects ?? [];
  const nodeOptions = syllabus?.nodes ?? [];

  return (
    <div className="page-container v2-flashcards-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · active recall</p>
        <h1>Flashcards</h1>
        <p className="page-lede">
          Shared cards with one schedule and review history. Cards can be shared across all four
          examinations or linked to a specific examination and syllabus topic. Reviews never change
          the shared concept mastery records.
        </p>
      </header>

      <section className="v2-flashcard-stats" aria-label="Flashcard counts">
        <div>
          <span>Total cards</span>
          <strong>{stats?.totalCards ?? 0}</strong>
        </div>
        <div>
          <span>Due today</span>
          <strong>{stats?.dueCards ?? 0}</strong>
        </div>
        <div>
          <span>Reviewed</span>
          <strong>{stats?.reviewedCards ?? 0}</strong>
        </div>
        <div>
          <span>Retention · 30 days</span>
          <strong>{formatPercent(stats?.retentionPercent ?? null)}</strong>
        </div>
      </section>

      <div className="v2-flashcard-toolbar">
        <div className="v2-flashcard-tabs" role="tablist" aria-label="Flashcard workspace">
          <button
            aria-selected={tab === 'library'}
            className="nav-button"
            onClick={() => setTab('library')}
            role="tab"
            type="button"
          >
            Library
          </button>
          <button
            aria-selected={tab === 'review'}
            className="nav-button"
            onClick={() => setTab('review')}
            role="tab"
            type="button"
          >
            Review
          </button>
        </div>
        <button
          className="primary-button"
          onClick={() => {
            setDetail(null);
            setEditorOpen(true);
          }}
          type="button"
        >
          New card
        </button>
      </div>

      {tab === 'review' ? (
        <FlashcardReview
          conceptSlug={conceptSlug}
          examId={examId}
          exams={exams}
          nodeId={nodeId}
          onBack={() => setTab('library')}
          onReviewed={() => void loadLibrary()}
          subjectArea={subjectArea}
          today={today}
        />
      ) : (
        <>
          <section className="v2-flashcard-filterbar" aria-label="Flashcard filters">
            <label className="v2-field">
              <span>Search</span>
              <input
                aria-label="Search flashcards"
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Front, back or subject"
                value={query}
              />
            </label>
            <label className="v2-field">
              <span>Examination</span>
              <select
                aria-label="Flashcard examination"
                onChange={(event) => {
                  setExamId(event.target.value ? Number(event.target.value) : null);
                  clearNodeFilter();
                }}
                value={examId ?? ''}
              >
                <option value="">All exams</option>
                {exams.map((exam) => (
                  <option key={exam.id} value={exam.id}>
                    {exam.shortName}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Subject</span>
              <select
                aria-label="Flashcard subject"
                onChange={(event) => setSubjectArea(event.target.value)}
                value={subjectArea}
              >
                <option value="">All subjects</option>
                {selectedSubjects.map((subject) => (
                  <option key={subject} value={subject}>
                    {subject}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Syllabus topic</span>
              <select
                aria-label="Flashcard syllabus topic"
                disabled={examId === null}
                onChange={(event) => {
                  const selected = nodeOptions.find(
                    (node) => node.id === Number(event.target.value),
                  );
                  setNodeId(event.target.value ? Number(event.target.value) : null);
                  setNodeTitle(selected?.title ?? '');
                }}
                value={nodeId ?? ''}
              >
                <option value="">All topics</option>
                {nodeOptions.map((node) => (
                  <option key={node.id} value={node.id}>
                    {node.nodePath.join(' › ')} · {node.title}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Sort</span>
              <select
                aria-label="Flashcard sort"
                onChange={(event) => setSort(event.target.value)}
                value={sort}
              >
                <option value="due">Due soonest</option>
                <option value="created">Newest</option>
                <option value="updated">Recently updated</option>
                <option value="front">Front A–Z</option>
                <option value="subject">Subject A–Z</option>
              </select>
            </label>
            <label className="v2-flashcard-check">
              <input
                checked={dueOnly}
                onChange={(event) => setDueOnly(event.target.checked)}
                type="checkbox"
              />
              <span>Due today only</span>
            </label>
            <label className="v2-flashcard-check">
              <input
                checked={includeArchived}
                onChange={(event) => setIncludeArchived(event.target.checked)}
                type="checkbox"
              />
              <span>Show archived</span>
            </label>
          </section>

          {(conceptSlug || nodeId !== null) && (
            <div className="v2-flashcard-filter-chips">
              {conceptSlug && (
                <span className="v2-allocation-chip">
                  Concept: {conceptTitle || conceptSlug}
                  <button
                    aria-label="Clear concept filter"
                    className="link-button"
                    onClick={clearConceptFilter}
                    type="button"
                  >
                    Clear
                  </button>
                </span>
              )}
              {nodeId !== null && (
                <span className="v2-allocation-chip">
                  Syllabus: {nodeTitle || `node ${nodeId}`}
                  <button
                    aria-label="Clear syllabus filter"
                    className="link-button"
                    onClick={clearNodeFilter}
                    type="button"
                  >
                    Clear
                  </button>
                </span>
              )}
            </div>
          )}

          {error && (
            <p className="v2-study-error" role="alert">
              {error}
            </p>
          )}

          <div className={`v2-flashcard-layout${detail ? ' has-detail' : ''}`}>
            <section aria-label="Flashcard library" className="v2-flashcard-library">
              {loading ? (
                <p className="v2-empty-panel-copy">Loading flashcards…</p>
              ) : (library?.cards.length ?? 0) === 0 ? (
                <div className="v2-empty-panel">
                  <h2>No flashcards found</h2>
                  <p className="v2-empty-panel-copy">
                    Create the first card, or clear the current filters.
                  </p>
                  <button
                    className="primary-button"
                    onClick={() => {
                      setDetail(null);
                      setEditorOpen(true);
                    }}
                    type="button"
                  >
                    New card
                  </button>
                </div>
              ) : (
                <>
                  <p className="v2-empty-copy">
                    {library?.total ?? 0} card{(library?.total ?? 0) === 1 ? '' : 's'} in this scope
                  </p>
                  <ul className="v2-flashcard-list">
                    {library?.cards.map((card) => (
                      <li key={card.id}>
                        <button
                          className={`v2-flashcard-row${
                            detail?.flashcard.id === card.id ? ' is-active' : ''
                          }`}
                          onClick={() => void openCard(card)}
                          type="button"
                        >
                          <span className="v2-flashcard-row-main">
                            <strong>{card.frontPreview}</strong>
                            <span>{card.backPreview}</span>
                          </span>
                          <span className="v2-flashcard-row-meta">
                            <span>{card.subjectArea}</span>
                            <span>{scheduleLabel(card, today)}</span>
                            {card.conceptTitle && <span>{card.conceptTitle}</span>}
                            {card.isArchived && (
                              <span className="v2-status-badge v2-status-inactive">Archived</span>
                            )}
                            {card.examLinks.length === 0 ? (
                              <span className="v2-status-badge v2-status-active">Shared</span>
                            ) : (
                              card.examLinks.map((link) => (
                                <span className="v2-status-badge" key={link.examId}>
                                  {link.examShortName}
                                </span>
                              ))
                            )}
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </section>

            {detail && (
              <aside aria-label="Flashcard detail" className="v2-flashcard-detail">
                <div className="v2-flashcard-detail-head">
                  <div>
                    <p className="eyebrow">{detail.flashcard.subjectArea}</p>
                    <h2>{detail.flashcard.isArchived ? 'Archived card' : 'Card detail'}</h2>
                  </div>
                  <button
                    aria-label="Close card detail"
                    className="secondary-button"
                    onClick={() => setDetail(null)}
                    type="button"
                  >
                    Close
                  </button>
                </div>
                <div className="v2-flashcard-detail-content">
                  <MathText text={detail.front} />
                </div>
                <div className="v2-flashcard-detail-divider" />
                <div className="v2-flashcard-detail-content">
                  <MathText text={detail.back} />
                </div>
                <dl className="v2-fact-list">
                  <div>
                    <dt>Subject</dt>
                    <dd>{detail.flashcard.subjectArea}</dd>
                  </div>
                  <div>
                    <dt>Concept</dt>
                    <dd>{detail.flashcard.conceptTitle ?? 'None'}</dd>
                  </div>
                  <div>
                    <dt>Schedule</dt>
                    <dd>
                      Due {detail.flashcard.schedule.dueOn} · interval{' '}
                      {detail.flashcard.schedule.intervalDays} d · ease{' '}
                      {detail.flashcard.schedule.easeFactor.toFixed(2)}
                    </dd>
                  </div>
                  <div>
                    <dt>Reviews</dt>
                    <dd>
                      {detail.flashcard.schedule.reviewCount} · lapses{' '}
                      {detail.flashcard.schedule.lapseCount}
                    </dd>
                  </div>
                  <div>
                    <dt>Examinations</dt>
                    <dd>
                      {detail.flashcard.examLinks.length === 0
                        ? 'Shared across all examinations'
                        : detail.flashcard.examLinks
                            .map((link) =>
                              link.nodeTitle
                                ? `${link.examShortName}: ${link.nodeTitle}`
                                : link.examShortName,
                            )
                            .join(' · ')}
                    </dd>
                  </div>
                </dl>
                <div className="v2-form-actions">
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => setEditorOpen(true)}
                    type="button"
                  >
                    Edit
                  </button>
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => void setArchived(detail, !detail.flashcard.isArchived)}
                    type="button"
                  >
                    {detail.flashcard.isArchived ? 'Restore card' : 'Archive card'}
                  </button>
                  <button className="primary-button" onClick={() => setTab('review')} type="button">
                    Review
                  </button>
                </div>

                <section aria-label="Review history" className="v2-flashcard-history">
                  <h3>Review history</h3>
                  {detail.reviews.length === 0 ? (
                    <p className="v2-empty-copy">No reviews yet.</p>
                  ) : (
                    <ul className="v2-flashcard-history-list">
                      {detail.reviews.map((review) => (
                        <li key={review.id}>
                          <strong>{review.rating}</strong>
                          <span>{review.reviewedOn}</span>
                          <span>
                            {review.intervalBeforeDays} d → {review.intervalAfterDays} d · next{' '}
                            {review.nextDueOn}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              </aside>
            )}
          </div>
        </>
      )}

      {editorOpen && (
        <FlashcardEditor
          exams={exams}
          flashcard={detail}
          initialExamCode={exams.find((exam) => exam.id === examId)?.code ?? null}
          initialNodeId={nodeId}
          onClose={() => setEditorOpen(false)}
          onSaved={(saved) => {
            setDetail(saved);
            void loadLibrary();
          }}
          subjects={selectedSubjects}
          today={today}
        />
      )}
    </div>
  );
}
