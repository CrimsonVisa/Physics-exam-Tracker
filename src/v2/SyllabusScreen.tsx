import { type ReactNode, useEffect, useMemo, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2ExamSyllabus,
  updateV2SyllabusNodeProgress,
  type V2ExamSyllabus,
  type V2ExamSummary,
  type V2SyllabusNode,
} from './api';

type SyllabusScreenProps = {
  exams: V2ExamSummary[];
  initialExamCode?: string | null;
  initialFocusNodeCode?: string | null;
  initialFocusNodeId?: number | null;
  onOpenFlashcards?: (examCode: string, nodeId: number, nodeTitle: string) => void;
  onOpenFormulas?: (examCode: string, nodeCode: string, nodeTitle: string) => void;
};

const PROGRESS_LABELS: Record<string, string> = {
  not_started: 'Not started',
  learning: 'Learning',
  revised: 'Revised',
  confident: 'Confident',
};

const KIND_LABELS: Record<string, string> = {
  section: 'Section',
  unit: 'Subject / unit',
  topic: 'Topic',
};

export function SyllabusScreen({
  exams,
  initialExamCode = null,
  initialFocusNodeCode = null,
  initialFocusNodeId = null,
  onOpenFlashcards,
  onOpenFormulas,
}: SyllabusScreenProps) {
  const initialExam = exams.find((exam) => exam.code === initialExamCode) ?? exams[0];
  const [examId, setExamId] = useState(String(initialExam?.id ?? ''));
  const [syllabus, setSyllabus] = useState<V2ExamSyllabus | null>(null);
  const [loading, setLoading] = useState(Boolean(initialExam));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState('all');
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  const focusNodeId = useMemo(() => {
    if (initialFocusNodeId !== null) {
      return initialFocusNodeId;
    }
    if (initialFocusNodeCode === null || !syllabus) {
      return null;
    }
    return syllabus.nodes.find((node) => node.code === initialFocusNodeCode)?.id ?? null;
  }, [initialFocusNodeCode, initialFocusNodeId, syllabus]);

  useEffect(() => {
    if (!examId) {
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const next = await getV2ExamSyllabus(Number(examId));
        if (cancelled) {
          return;
        }
        setSyllabus(next);
        const nextExpanded = new Set(
          next.nodes.filter((node) => node.nodeKind !== 'topic').map((node) => node.id),
        );
        if (initialFocusNodeId !== null || initialFocusNodeCode !== null) {
          const parentById = new Map(next.nodes.map((node) => [node.id, node.parentId]));
          const target = next.nodes.find(
            (node) =>
              node.id === initialFocusNodeId ||
              (initialFocusNodeCode !== null && node.code === initialFocusNodeCode),
          );
          if (target) {
            const seen = new Set<number>();
            let parent = target.parentId;
            while (parent !== null && !seen.has(parent)) {
              seen.add(parent);
              nextExpanded.add(parent);
              parent = parentById.get(parent) ?? null;
            }
          }
        }
        setExpanded(nextExpanded);
        setError(null);
      } catch (loadError) {
        if (!cancelled) {
          setError(toCommandError(loadError).message);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [examId, initialFocusNodeCode, initialFocusNodeId]);

  useEffect(() => {
    if (!syllabus || focusNodeId === null) {
      return undefined;
    }
    const timer = window.setTimeout(() => {
      const element = document.getElementById(`v2-syllabus-node-${focusNodeId}`);
      if (!element) {
        return;
      }
      try {
        element.scrollIntoView({ behavior: 'smooth', block: 'center' });
      } catch {
        // jsdom does not implement scrolling; focus is the important part.
      }
      element.focus({ preventScroll: true });
    }, 80);
    return () => {
      window.clearTimeout(timer);
    };
  }, [focusNodeId, syllabus]);

  async function changeProgress(node: V2SyllabusNode, status: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const next = await updateV2SyllabusNodeProgress({
        examId: node.examId,
        nodeId: node.id,
        status,
        priority: node.priority,
        confidence: node.confidence,
        notes: node.notes,
      });
      setSyllabus(next);
      setNotice('Syllabus progress saved.');
    } catch (updateError) {
      setError(toCommandError(updateError).message);
    } finally {
      setBusy(false);
    }
  }

  const children = useMemo(() => {
    const map = new Map<number, V2SyllabusNode[]>();
    for (const node of syllabus?.nodes ?? []) {
      if (node.parentId === null) {
        continue;
      }
      const list = map.get(node.parentId) ?? [];
      list.push(node);
      map.set(node.parentId, list);
    }
    return map;
  }, [syllabus]);

  const roots = useMemo(
    () =>
      (syllabus?.nodes ?? [])
        .filter((node) => node.parentId === null)
        .sort((left, right) => left.displayOrder - right.displayOrder),
    [syllabus],
  );

  const filtering = search.trim().length > 0 || statusFilter !== 'all';
  const filteredNodes = useMemo(() => {
    if (!syllabus || !filtering) {
      return [];
    }
    const query = search.trim().toLowerCase();
    return syllabus.nodes.filter((node) => {
      if (statusFilter !== 'all' && node.progressStatus !== statusFilter) {
        return false;
      }
      if (!query) {
        return true;
      }
      return [node.title, node.code, node.category, ...node.nodePath]
        .join(' ')
        .toLowerCase()
        .includes(query);
    });
  }, [filtering, search, statusFilter, syllabus]);

  function toggle(nodeId: number) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  }

  function expandAll() {
    if (!syllabus) {
      return;
    }
    setExpanded(
      new Set(syllabus.nodes.filter((node) => node.nodeKind !== 'topic').map((node) => node.id)),
    );
  }

  function renderNode(node: V2SyllabusNode, depth: number): ReactNode {
    const nodeChildren = children.get(node.id) ?? [];
    const isExpanded = expanded.has(node.id);
    return (
      <li className="v2-syllabus-row" key={node.id}>
        <div
          className={`v2-syllabus-node${
            focusNodeId === node.id ? ' v2-syllabus-node-focused' : ''
          }`}
          id={`v2-syllabus-node-${node.id}`}
          style={{ paddingLeft: `${Math.min(depth, 5) * 18}px` }}
          tabIndex={focusNodeId === node.id ? -1 : undefined}
        >
          <div className="v2-syllabus-node-main">
            <div className="v2-session-heading">
              {nodeChildren.length > 0 && (
                <button
                  aria-expanded={isExpanded}
                  aria-label={`${isExpanded ? 'Collapse' : 'Expand'} ${node.title}`}
                  className="v2-syllabus-expand"
                  onClick={() => toggle(node.id)}
                  type="button"
                >
                  {isExpanded ? '−' : '+'}
                </button>
              )}
              <strong>{node.title}</strong>
              <span className="v2-status-badge v2-status-inactive">
                {KIND_LABELS[node.nodeKind] ?? node.nodeKind}
              </span>
              <span className="v2-session-date">{node.code}</span>
            </div>
            {node.category && <p className="v2-empty-copy">{node.category}</p>}
            <p className="v2-empty-copy">
              {node.nodePath.join(' → ')} · {node.sourceTitle} ({node.sourceVerifiedOn}) ·{' '}
              {node.contentVersionLabel}
            </p>
            <p className="v2-empty-copy">
              <span className="v2-status-badge v2-status-active">Official verified</span>{' '}
              {node.sourceReference}
            </p>
            {(onOpenFlashcards || onOpenFormulas) && syllabus && node.nodeKind === 'topic' && (
              <div className="v2-syllabus-topic-links">
                {onOpenFlashcards ? (
                  <button
                    aria-label={`Open flashcards for ${node.title}`}
                    className="secondary-button v2-syllabus-flashcard-link"
                    onClick={() => onOpenFlashcards(syllabus.examCode, node.id, node.title)}
                    type="button"
                  >
                    Flashcards for this topic
                  </button>
                ) : null}
                {onOpenFormulas ? (
                  <button
                    aria-label={`Open formulas for ${node.title}`}
                    className="secondary-button v2-syllabus-flashcard-link"
                    onClick={() => onOpenFormulas(syllabus.examCode, node.code, node.title)}
                    type="button"
                  >
                    Formulas for this topic
                  </button>
                ) : null}
              </div>
            )}
          </div>
          {node.nodeKind === 'topic' && (
            <label className="v2-field">
              <span>Progress</span>
              <select
                aria-label={`Progress for ${node.title}`}
                disabled={busy || !syllabus?.examIsActive}
                onChange={(event) => void changeProgress(node, event.target.value)}
                value={node.progressStatus}
              >
                {Object.entries(PROGRESS_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
        {isExpanded && nodeChildren.length > 0 && (
          <ul className="v2-syllabus-list">
            {nodeChildren
              .sort((left, right) => left.displayOrder - right.displayOrder)
              .map((child) => renderNode(child, depth + 1))}
          </ul>
        )}
      </li>
    );
  }

  const progressPercent =
    syllabus && syllabus.summary.topicCount > 0
      ? Math.round((syllabus.summary.revisedOrConfidentCount / syllabus.summary.topicCount) * 100)
      : 0;

  return (
    <div className="page-container v2-syllabus-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · exam coverage</p>
        <h1>Exam syllabus</h1>
        <p className="page-lede">
          Exam-specific official syllabus trees with source attribution, progress, and study
          tracking. Shared concepts remain separate from exam-specific completion.
        </p>
      </header>

      <div className="v2-planner-datebar">
        <label className="v2-field">
          <span>Examination</span>
          <select
            onChange={(event) => {
              setLoading(true);
              setExamId(event.target.value);
            }}
            value={examId}
          >
            {exams.map((exam) => (
              <option key={exam.id} value={exam.id}>
                {exam.shortName} — {exam.name}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Search syllabus</span>
          <input
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Topic, subject, code, or category"
            type="search"
            value={search}
          />
        </label>
        <label className="v2-field">
          <span>Completion filter</span>
          <select onChange={(event) => setStatusFilter(event.target.value)} value={statusFilter}>
            <option value="all">All statuses</option>
            {Object.entries(PROGRESS_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
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

      {loading ? (
        <p className="v2-empty-copy">Loading syllabus…</p>
      ) : !syllabus || !syllabus.configured ? (
        <section className="v2-empty-panel">
          <h2>Official syllabus not configured</h2>
          <p>No verified syllabus nodes are stored for this examination.</p>
        </section>
      ) : (
        <>
          <section className="v2-coverage-grid">
            <article className="v2-coverage-card">
              <span className="v2-status-badge v2-status-active">
                {syllabus.contentReviewStatus ?? 'configured'}
              </span>
              <h2>{syllabus.contentVersionLabel}</h2>
              <dl className="v2-coverage-facts">
                <div>
                  <dt>Total nodes</dt>
                  <dd>{syllabus.summary.nodeCount}</dd>
                </div>
                <div>
                  <dt>Leaf topics</dt>
                  <dd>{syllabus.summary.topicCount}</dd>
                </div>
                <div>
                  <dt>Revised/confident</dt>
                  <dd>{syllabus.summary.revisedOrConfidentCount}</dd>
                </div>
                <div>
                  <dt>Completion</dt>
                  <dd>{progressPercent}%</dd>
                </div>
              </dl>
            </article>
          </section>
          <section className="v2-concept-card" aria-labelledby="syllabus-nodes-heading">
            <div className="v2-section-heading">
              <h2 id="syllabus-nodes-heading">Syllabus tree</h2>
              <div className="v2-syllabus-actions">
                <button className="secondary-button" onClick={expandAll} type="button">
                  Expand all
                </button>
                <button
                  className="secondary-button"
                  onClick={() => setExpanded(new Set())}
                  type="button"
                >
                  Collapse all
                </button>
              </div>
            </div>
            {filtering ? (
              <p className="v2-empty-copy">
                {filteredNodes.length} matching node{filteredNodes.length === 1 ? '' : 's'}.
              </p>
            ) : null}
            <ul className="v2-syllabus-list">
              {filtering
                ? filteredNodes.map((node) => (
                    <li className="v2-syllabus-row" key={node.id}>
                      <div className="v2-syllabus-node">
                        <div className="v2-syllabus-node-main">
                          <div className="v2-session-heading">
                            <strong>{node.title}</strong>
                            <span className="v2-status-badge v2-status-inactive">
                              {KIND_LABELS[node.nodeKind] ?? node.nodeKind}
                            </span>
                            <span className="v2-session-date">{node.code}</span>
                          </div>
                          <p className="v2-empty-copy">{node.nodePath.join(' → ')}</p>
                        </div>
                        {node.nodeKind === 'topic' && (
                          <label className="v2-field">
                            <span>Progress</span>
                            <select
                              aria-label={`Progress for ${node.title}`}
                              disabled={busy || !syllabus.examIsActive}
                              onChange={(event) => void changeProgress(node, event.target.value)}
                              value={node.progressStatus}
                            >
                              {Object.entries(PROGRESS_LABELS).map(([value, label]) => (
                                <option key={value} value={value}>
                                  {label}
                                </option>
                              ))}
                            </select>
                          </label>
                        )}
                      </div>
                    </li>
                  ))
                : roots.map((node) => renderNode(node, 0))}
            </ul>
            {!syllabus.examIsActive && (
              <p className="v2-empty-copy">
                This examination is inactive. Reactivate it before editing syllabus progress.
              </p>
            )}
          </section>
        </>
      )}
    </div>
  );
}
