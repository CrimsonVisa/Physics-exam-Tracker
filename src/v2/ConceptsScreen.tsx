import { type FormEvent, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2ConceptDetail,
  replayV2ConceptMastery,
  searchV2Concepts,
  updateV2ConceptMapping,
  updateV2ConceptMastery,
  verifyV2MasteryRun,
  type V2ConceptDetail,
  type V2ConceptEvidence,
  type V2ConceptMasterySummary,
  type V2ConceptSearchFilter,
  type V2ConceptSearchResult,
  type V2ConceptSummary,
  type V2ConceptSyllabusMapping,
  type V2ExamConceptCoverage,
  type V2ExamSummary,
  type V2MasteryReplayVerification,
  type V2MasteryRunSummary,
} from './api';

type ConceptsScreenProps = {
  exams: V2ExamSummary[];
  today: string;
  initialExamCode?: string | null;
  initialConceptSlug?: string | null;
  onOpenFlashcards?: (conceptSlug: string, conceptTitle: string) => void;
};

const COVERAGE_LABELS: Record<string, string> = {
  not_configured: 'Official syllabus not configured',
  unmapped: 'No concept mappings',
  unverified: 'Unverified mappings',
  partially_verified: 'Partially verified',
  verified: 'Approved mappings',
};

const MAPPING_STATUS_LABELS: Record<string, string> = {
  proposed: 'Proposed',
  reviewed: 'Reviewed',
  approved: 'Approved',
  rejected: 'Rejected',
};

const MAPPING_ORIGIN_LABELS: Record<string, string> = {
  curated: 'Curated',
  official: 'Officially documented',
};

const REASON_LABELS: Record<string, string> = {
  source_created: 'New evidence',
  source_edited: 'Evidence edited',
  source_deleted: 'Evidence removed',
  manual_override: 'Manual override',
  algorithm_change: 'Algorithm change',
  repair: 'Recalculation',
};

const SOURCE_LABELS: Record<string, string> = {
  study_session: 'Study session',
  revision: 'Revision',
  pyq_attempt: 'PYQ attempt',
  mock_concept_result: 'Mock concept result',
  manual: 'Manual override',
};

const PROGRESS_LABELS: Record<string, string> = {
  not_started: 'Not started',
  learning: 'Learning',
  revised: 'Revised',
  confident: 'Confident',
};

function formatScore(score: number): string {
  return score.toFixed(1);
}

function examShortName(exams: V2ExamSummary[], code: string): string {
  return exams.find((exam) => exam.code === code)?.shortName ?? code;
}

function masteryLabel(mastery: V2ConceptMasterySummary | null): string {
  if (!mastery || mastery.evidenceCount === 0) {
    return 'No mastery evidence';
  }
  return `Mastery ${formatScore(mastery.masteryScore)} / 100`;
}

function rawValueText(evidence: V2ConceptEvidence): string {
  if (typeof evidence.rawValue === 'object' && evidence.rawValue !== null) {
    return JSON.stringify(evidence.rawValue);
  }
  return String(evidence.rawValue);
}

function coverageDescription(coverage: V2ExamConceptCoverage): string {
  if (coverage.status === 'not_configured') {
    return 'No verified official syllabus content is configured for this examination. No mappings were guessed.';
  }
  if (coverage.status === 'unmapped') {
    return 'Syllabus nodes exist, but no shared concepts are mapped to them yet.';
  }
  if (coverage.status === 'unverified') {
    return 'Mappings are proposed or their syllabus content version is not yet verified.';
  }
  if (coverage.status === 'partially_verified') {
    return 'Some mappings are approved with verified syllabus content and others still need review.';
  }
  return 'Mapped concepts come from approved mappings on a verified syllabus content version.';
}

export function ConceptsScreen({
  exams,
  today,
  initialExamCode = null,
  initialConceptSlug = null,
  onOpenFlashcards,
}: ConceptsScreenProps) {
  const [query, setQuery] = useState('');
  const [domain, setDomain] = useState('');
  const [examCode, setExamCode] = useState(initialExamCode ?? '');
  const [includeArchived, setIncludeArchived] = useState(false);
  const [filter, setFilter] = useState<V2ConceptSearchFilter>(() => ({
    examCode: initialExamCode ?? undefined,
    limit: 100,
  }));
  const [result, setResult] = useState<V2ConceptSearchResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);
  const [detail, setDetail] = useState<V2ConceptDetail | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);

  const [manualScore, setManualScore] = useState('70');
  const [manualNotes, setManualNotes] = useState('');
  const [manualError, setManualError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [verifyingRunId, setVerifyingRunId] = useState<number | null>(null);
  const [verification, setVerification] = useState<V2MasteryReplayVerification | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const next = await searchV2Concepts(filter);
        if (!cancelled) {
          setResult(next);
          setError(null);
        }
      } catch (searchError) {
        if (!cancelled) {
          setError(toCommandError(searchError).message);
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
  }, [filter]);

  useEffect(() => {
    const slug = initialConceptSlug?.trim();
    if (!slug) {
      return undefined;
    }
    let cancelled = false;
    void (async () => {
      setSelectedSlug(slug);
      setDetail(null);
      setDetailError(null);
      setDetailLoading(true);
      try {
        const next = await getV2ConceptDetail({ slug });
        if (cancelled) {
          return;
        }
        setDetail(next);
        setManualScore(
          next.concept.mastery && next.concept.mastery.evidenceCount > 0
            ? formatScore(next.concept.mastery.masteryScore)
            : '70',
        );
        setManualNotes('');
      } catch (loadError) {
        if (!cancelled) {
          setDetailError(toCommandError(loadError).message);
        }
      } finally {
        if (!cancelled) {
          setDetailLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [initialConceptSlug]);

  async function refreshResult() {
    try {
      setResult(await searchV2Concepts(filter));
      setError(null);
    } catch (refreshError) {
      setError(toCommandError(refreshError).message);
    }
  }

  function submitSearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setLoading(true);
    setResult(null);
    setFilter({
      query: query.trim() || undefined,
      domain: domain || undefined,
      examCode: examCode || undefined,
      includeArchived,
      limit: 100,
    });
  }

  function updateExamFilter(value: string) {
    setExamCode(value);
    setLoading(true);
    setFilter({
      query: query.trim() || undefined,
      domain: domain || undefined,
      examCode: value || undefined,
      includeArchived,
      limit: 100,
    });
  }

  function updateDomainFilter(value: string) {
    setDomain(value);
    setLoading(true);
    setFilter({
      query: query.trim() || undefined,
      domain: value || undefined,
      examCode: examCode || undefined,
      includeArchived,
      limit: 100,
    });
  }

  function updateArchivedFilter(value: boolean) {
    setIncludeArchived(value);
    setLoading(true);
    setFilter({
      query: query.trim() || undefined,
      domain: domain || undefined,
      examCode: examCode || undefined,
      includeArchived: value,
      limit: 100,
    });
  }

  async function openConcept(slug: string) {
    setSelectedSlug(slug);
    setDetail(null);
    setDetailError(null);
    setDetailLoading(true);
    setNotice(null);
    setManualError(null);
    setVerification(null);
    try {
      const next = await getV2ConceptDetail({ slug });
      setDetail(next);
      setManualScore(
        next.concept.mastery && next.concept.mastery.evidenceCount > 0
          ? formatScore(next.concept.mastery.masteryScore)
          : '70',
      );
      setManualNotes('');
    } catch (loadError) {
      setDetailError(toCommandError(loadError).message);
    } finally {
      setDetailLoading(false);
    }
  }

  async function refreshDetail(slug: string) {
    const next = await getV2ConceptDetail({ slug });
    setDetail(next);
    await refreshResult();
    return next;
  }

  async function reviewMapping(
    mapping: V2ConceptSyllabusMapping,
    status: 'proposed' | 'reviewed' | 'approved' | 'rejected',
    notes: string,
  ) {
    setBusy(true);
    setDetailError(null);
    setNotice(null);
    try {
      const next = await updateV2ConceptMapping({
        examId: mapping.examId,
        nodeId: mapping.nodeId,
        conceptId: mapping.conceptId,
        status,
        notes,
      });
      setDetail(next);
      await refreshResult();
      setNotice(
        status === 'approved'
          ? 'Mapping approved. It is now eligible for revision planning.'
          : 'Mapping review saved.',
      );
    } catch (mappingError) {
      setDetailError(toCommandError(mappingError).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleManualMastery(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!detail) {
      return;
    }
    const targetScore = Number(manualScore);
    if (!Number.isFinite(targetScore) || targetScore < 0 || targetScore > 100) {
      setManualError('Enter a manual mastery target between 0 and 100.');
      return;
    }

    setBusy(true);
    setManualError(null);
    setNotice(null);
    try {
      await updateV2ConceptMastery({
        conceptId: detail.concept.id,
        targetScore,
        localDate: today,
        notes: manualNotes.trim(),
      });
      await refreshDetail(detail.concept.slug);
      setNotice(
        'Manual evidence recorded. Shared mastery was recalculated from the full active evidence set.',
      );
      setManualNotes('');
    } catch (updateError) {
      setManualError(toCommandError(updateError).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleReplay() {
    if (!detail) {
      return;
    }
    setBusy(true);
    setNotice(null);
    setError(null);
    try {
      await replayV2ConceptMastery({ conceptId: detail.concept.id });
      await refreshDetail(detail.concept.slug);
      setNotice('Mastery was recalculated from the current active evidence.');
    } catch (replayError) {
      setError(toCommandError(replayError).message);
    } finally {
      setBusy(false);
    }
  }

  async function handleVerifyRun(run: V2MasteryRunSummary) {
    setVerifyingRunId(run.id);
    setVerification(null);
    try {
      setVerification(await verifyV2MasteryRun({ runId: run.id }));
    } catch (verifyError) {
      setError(toCommandError(verifyError).message);
    } finally {
      setVerifyingRunId(null);
    }
  }

  if (selectedSlug) {
    if (detailLoading) {
      return (
        <div className="page-container v2-concepts" aria-busy="true">
          <p className="v2-empty-copy">Loading concept detail…</p>
        </div>
      );
    }
    if (!detail) {
      return (
        <div className="page-container v2-concepts">
          <button
            className="secondary-button v2-back-button"
            onClick={() => setSelectedSlug(null)}
            type="button"
          >
            ← Back to concept library
          </button>
          <p className="v2-study-error" role="alert">
            {detailError ?? 'The concept detail could not be loaded.'}
          </p>
        </div>
      );
    }

    return (
      <div className="page-container v2-concepts v2-concept-detail-page">
        <button
          className="secondary-button v2-back-button"
          onClick={() => setSelectedSlug(null)}
          type="button"
        >
          ← Back to concept library
        </button>

        <header className="page-header v2-concept-header">
          <p className="eyebrow">
            {detail.concept.domain} · Difficulty {detail.concept.difficulty} / 5
          </p>
          <h1>{detail.concept.title}</h1>
          <p className="page-lede">{detail.concept.description}</p>
          <div className="v2-concept-badges">
            {detail.concept.isArchived && (
              <span className="v2-status-badge v2-status-inactive">Archived concept</span>
            )}
            {detail.concept.parentTitle && (
              <span className="v2-allocation-chip">Parent: {detail.concept.parentTitle}</span>
            )}
            <span className="v2-allocation-chip">
              {detail.mappings.length} exam mapping
              {detail.mappings.length === 1 ? '' : 's'}
            </span>
          </div>
          {onOpenFlashcards && (
            <button
              aria-label={`Open flashcards for ${detail.concept.title}`}
              className="secondary-button v2-concept-flashcard-link"
              onClick={() => onOpenFlashcards(detail.concept.slug, detail.concept.title)}
              type="button"
            >
              Flashcards for this concept
            </button>
          )}
        </header>

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

        <div className="v2-concept-detail-grid">
          <section className="v2-concept-card" aria-labelledby="concept-mastery-heading">
            <div className="v2-timer-topline">
              <div>
                <p className="eyebrow">Shared mastery</p>
                <h2 id="concept-mastery-heading">Evidence-based mastery</h2>
              </div>
              <span className="v2-status-badge v2-status-active">One shared record</span>
            </div>

            {detail.concept.mastery && detail.concept.mastery.evidenceCount > 0 ? (
              <>
                <p className="v2-mastery-score">
                  {formatScore(detail.concept.mastery.masteryScore)}
                  <span> / 100</span>
                </p>
                <dl className="v2-fact-list">
                  <div>
                    <dt>Confidence</dt>
                    <dd>{detail.concept.mastery.confidence} / 5</dd>
                  </div>
                  <div>
                    <dt>Active evidence</dt>
                    <dd>{detail.concept.mastery.evidenceCount}</dd>
                  </div>
                  <div>
                    <dt>Last evidence</dt>
                    <dd>{detail.concept.mastery.lastEvidenceAt ?? 'Not recorded'}</dd>
                  </div>
                  <div>
                    <dt>Algorithm</dt>
                    <dd>{detail.concept.mastery.algorithmVersion ?? 'Not recorded'}</dd>
                  </div>
                </dl>
              </>
            ) : (
              <div className="v2-mastery-missing">
                <h3>No mastery evidence yet</h3>
                <p>
                  Absence of evidence is not the same as low mastery. No score is fabricated for
                  this concept until explicit evidence is recorded.
                </p>
              </div>
            )}

            <div className="v2-form-actions">
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => void handleReplay()}
                type="button"
              >
                Recalculate from evidence
              </button>
            </div>

            <details className="v2-timer-settings">
              <summary>Record manual mastery evidence</summary>
              <form className="v2-manual-mastery-form" onSubmit={handleManualMastery}>
                <p className="v2-empty-copy">
                  Manual evidence sets the exact target score at this point in the replay. It does
                  not bypass later evidence or cap rules.
                </p>
                <label className="v2-field">
                  <span>Manual mastery target (0–100)</span>
                  <input
                    max={100}
                    min={0}
                    onChange={(event) => setManualScore(event.target.value)}
                    step={0.1}
                    type="number"
                    value={manualScore}
                  />
                </label>
                <label className="v2-field">
                  <span>Evidence notes</span>
                  <textarea
                    maxLength={2000}
                    onChange={(event) => setManualNotes(event.target.value)}
                    rows={3}
                    value={manualNotes}
                  />
                </label>
                {manualError && (
                  <p className="v2-study-error" role="alert">
                    {manualError}
                  </p>
                )}
                <button className="primary-button" disabled={busy} type="submit">
                  {busy ? 'Saving evidence…' : 'Record manual evidence'}
                </button>
              </form>
            </details>
          </section>

          <section className="v2-concept-card" aria-labelledby="concept-mappings-heading">
            <p className="eyebrow">Exam mappings</p>
            <h2 id="concept-mappings-heading">Syllabus coverage</h2>
            {detail.mappings.length === 0 ? (
              <p className="v2-empty-panel-copy">
                No exam-specific syllabus mapping is configured for this concept. No official
                mapping has been guessed or inferred.
              </p>
            ) : (
              <ul className="v2-mapping-list">
                {detail.mappings.map((mapping) => (
                  <MappingRow
                    busy={busy}
                    exams={exams}
                    key={`${mapping.examId}-${mapping.nodeId}-${mapping.updatedAt}`}
                    mapping={mapping}
                    onReview={reviewMapping}
                  />
                ))}
              </ul>
            )}
          </section>
        </div>

        {detail.children.length > 0 && (
          <section className="v2-concept-card" aria-labelledby="concept-children-heading">
            <p className="eyebrow">Hierarchy</p>
            <h2 id="concept-children-heading">Child concepts</h2>
            <div className="v2-allocation-chips">
              {detail.children.map((child) => (
                <button
                  className="v2-allocation-chip v2-concept-child-link"
                  key={child.slug}
                  onClick={() => void openConcept(child.slug)}
                  type="button"
                >
                  {child.title}
                  {child.isArchived && ' (archived)'}
                </button>
              ))}
            </div>
          </section>
        )}

        <section className="v2-concept-card" aria-labelledby="concept-evidence-heading">
          <div className="v2-section-heading">
            <div>
              <p className="eyebrow">Provenance</p>
              <h2 id="concept-evidence-heading">Evidence</h2>
            </div>
            <p>
              {detail.evidence.length} recorded event{detail.evidence.length === 1 ? '' : 's'}
            </p>
          </div>
          {detail.evidence.length === 0 ? (
            <p className="v2-empty-panel-copy">
              No evidence has been recorded for this concept. Mastery remains unassessed.
            </p>
          ) : (
            <ul className="v2-evidence-list">
              {detail.evidence.map((evidence) => (
                <li
                  className={`v2-evidence-row${evidence.isVoided ? ' v2-evidence-voided' : ''}`}
                  key={evidence.id}
                >
                  <div className="v2-session-heading">
                    <strong>{SOURCE_LABELS[evidence.sourceKind] ?? evidence.sourceKind}</strong>
                    <span className="v2-session-date">{evidence.localDate}</span>
                    {evidence.examShortName && (
                      <span className="v2-allocation-chip">{evidence.examShortName}</span>
                    )}
                    {evidence.isVoided && (
                      <span className="v2-status-badge v2-status-inactive">
                        Voided: {evidence.voidReason ?? 'historical'}
                      </span>
                    )}
                  </div>
                  <code className="v2-evidence-value">{rawValueText(evidence)}</code>
                  <p className="v2-empty-copy">
                    Algorithm {evidence.algorithmVersion} · source #{evidence.sourceId ?? '—'} ·
                    version {evidence.sourceVersion} · occurred {evidence.occurredAt}
                  </p>
                  {evidence.notes && <p className="v2-evidence-note">{evidence.notes}</p>}
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="v2-concept-card" aria-labelledby="concept-history-heading">
          <div className="v2-section-heading">
            <div>
              <p className="eyebrow">Replay</p>
              <h2 id="concept-history-heading">Mastery history</h2>
            </div>
            <p>
              {detail.masteryHistory.length} calculation
              {detail.masteryHistory.length === 1 ? '' : 's'}
            </p>
          </div>
          {detail.masteryHistory.length === 0 ? (
            <p className="v2-empty-panel-copy">No mastery calculations have been recorded yet.</p>
          ) : (
            <ul className="v2-history-list">
              {detail.masteryHistory.map((run) => (
                <li className="v2-history-row" key={run.id}>
                  <div>
                    <strong>{formatScore(run.resultingScore)} / 100</strong>
                    <span className="v2-session-date">{run.finishedAt}</span>
                  </div>
                  <p className="v2-empty-copy">
                    {REASON_LABELS[run.reason] ?? run.reason} · confidence {run.resultingConfidence}{' '}
                    · {run.evidenceCount} evidence event{run.evidenceCount === 1 ? '' : 's'} ·
                    algorithm {run.algorithmVersion ?? 'unknown'}
                  </p>
                  <code className="v2-fingerprint">{run.evidenceFingerprint}</code>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      disabled={verifyingRunId === run.id}
                      onClick={() => void handleVerifyRun(run)}
                      type="button"
                    >
                      {verifyingRunId === run.id ? 'Verifying…' : 'Verify replay'}
                    </button>
                  </div>
                  {verification?.runId === run.id && (
                    <p
                      className={verification.matches ? 'v2-study-notice' : 'v2-study-error'}
                      role="status"
                    >
                      {verification.matches
                        ? 'Replay verified: stored score, confidence, fingerprint, and ordered results match.'
                        : `Replay mismatch: ${verification.mismatches.join(' ')}`}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    );
  }

  return (
    <div className="page-container v2-concepts">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · shared library</p>
        <h1>Concept library</h1>
        <p className="page-lede">
          One shared Physics concept library across all four examinations. Mastery is evidence-based
          and shared; syllabus mappings remain exam-specific and are never inferred.
        </p>
      </header>

      <section aria-label="Exam syllabus coverage" className="v2-coverage-grid">
        {(result?.examCoverage ?? []).map((coverage) => (
          <article
            className="v2-coverage-card"
            data-exam-code={coverage.examCode}
            key={coverage.examCode}
          >
            <div className="v2-exam-card-topline">
              <span
                className={`v2-status-badge ${
                  coverage.status === 'verified' ? 'v2-status-active' : 'v2-status-inactive'
                }`}
              >
                {COVERAGE_LABELS[coverage.status] ?? coverage.status}
              </span>
              <span className="v2-exam-code">{coverage.examShortName}</span>
            </div>
            <h2>{coverage.examShortName}</h2>
            <p className="v2-empty-copy">{coverageDescription(coverage)}</p>
            <dl className="v2-coverage-facts">
              <div>
                <dt>Mapped concepts</dt>
                <dd>{coverage.mappedConceptCount}</dd>
              </div>
              <div>
                <dt>Verified mappings</dt>
                <dd>{coverage.verifiedMappedConceptCount}</dd>
              </div>
              <div>
                <dt>Syllabus nodes</dt>
                <dd>{coverage.nodeCount}</dd>
              </div>
              <div>
                <dt>Revised nodes</dt>
                <dd>{coverage.revisedNodeCount}</dd>
              </div>
            </dl>
          </article>
        ))}
      </section>

      <form className="v2-concept-filters" onSubmit={submitSearch}>
        <label className="v2-field v2-concept-search">
          <span>Search concepts</span>
          <input
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Title, slug, domain, or description"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Domain</span>
          <select onChange={(event) => updateDomainFilter(event.target.value)} value={domain}>
            <option value="">All domains</option>
            {(result?.domains ?? []).map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Examination</span>
          <select onChange={(event) => updateExamFilter(event.target.value)} value={examCode}>
            <option value="">Combined view</option>
            {exams.map((exam) => (
              <option key={exam.code} value={exam.code}>
                {exam.shortName}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-checkbox-field">
          <input
            checked={includeArchived}
            onChange={(event) => updateArchivedFilter(event.target.checked)}
            type="checkbox"
          />
          <span>Include archived concepts</span>
        </label>
        <button className="primary-button" disabled={loading} type="submit">
          {loading ? 'Searching…' : 'Search'}
        </button>
      </form>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}
      {error && !result && (
        <button
          className="secondary-button"
          onClick={() => {
            setLoading(true);
            setError(null);
            setFilter({ ...filter });
          }}
          type="button"
        >
          Retry concept search
        </button>
      )}

      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">Curated concepts</p>
          <h2>{result ? `${result.total} matching concepts` : 'Loading concepts'}</h2>
        </div>
        <p>
          Concepts are curated and exam-neutral. Exam mappings appear only when the database
          contains the corresponding syllabus records.
        </p>
      </div>

      {loading && !result ? (
        <p className="v2-empty-copy">Loading the shared concept library…</p>
      ) : result && result.concepts.length === 0 ? (
        <p className="v2-empty-panel-copy">
          No concepts match these filters. Clear the search or choose a different examination.
        </p>
      ) : (
        <ul className="v2-concept-list">
          {(result?.concepts ?? []).map((concept) => (
            <ConceptRow
              concept={concept}
              exams={exams}
              key={concept.slug}
              onOpen={() => void openConcept(concept.slug)}
            />
          ))}
        </ul>
      )}
    </div>
  );
}

function ConceptRow({
  concept,
  exams,
  onOpen,
}: {
  concept: V2ConceptSummary;
  exams: V2ExamSummary[];
  onOpen: () => void;
}) {
  return (
    <li>
      <button className="v2-concept-row" onClick={onOpen} type="button">
        <div className="v2-concept-row-main">
          <p className="eyebrow">{concept.domain}</p>
          <h3>{concept.title}</h3>
          <p>{concept.description}</p>
          <div className="v2-allocation-chips">
            {concept.examCodes.length === 0 ? (
              <span className="v2-allocation-chip v2-allocation-unallocated">No exam mappings</span>
            ) : (
              concept.examCodes.map((code) => {
                const verified = concept.verifiedExamCodes.includes(code);
                return (
                  <span
                    className={`v2-allocation-chip${
                      verified ? '' : ' v2-allocation-chip-inactive'
                    }`}
                    key={code}
                  >
                    {examShortName(exams, code)} {verified ? 'verified' : 'unverified'}
                  </span>
                );
              })
            )}
          </div>
        </div>
        <div className="v2-concept-row-side">
          <span
            className={`v2-status-badge ${
              concept.mastery && concept.mastery.evidenceCount > 0
                ? 'v2-status-active'
                : 'v2-status-inactive'
            }`}
          >
            {masteryLabel(concept.mastery)}
          </span>
          <span className="v2-concept-difficulty">Difficulty {concept.difficulty} / 5</span>
          {concept.parentTitle && (
            <span className="v2-concept-parent">Parent · {concept.parentTitle}</span>
          )}
        </div>
      </button>
    </li>
  );
}

function MappingRow({
  mapping,
  exams,
  busy,
  onReview,
}: {
  mapping: V2ConceptSyllabusMapping;
  exams: V2ExamSummary[];
  busy: boolean;
  onReview: (
    mapping: V2ConceptSyllabusMapping,
    status: 'proposed' | 'reviewed' | 'approved' | 'rejected',
    notes: string,
  ) => Promise<void>;
}) {
  const [status, setStatus] = useState(mapping.status);
  const [notes, setNotes] = useState(mapping.notes);

  const changed = status !== mapping.status || notes !== mapping.notes;

  return (
    <li className="v2-mapping-row">
      <div className="v2-session-heading">
        <strong>{examShortName(exams, mapping.examCode)}</strong>
        <span
          className={`v2-status-badge ${
            mapping.status === 'approved'
              ? 'v2-status-active'
              : mapping.status === 'rejected' || mapping.nodeIsArchived
                ? 'v2-status-inactive'
                : 'v2-status-neutral'
          }`}
        >
          {MAPPING_STATUS_LABELS[mapping.status] ?? mapping.status}
        </span>
        <span className="v2-allocation-chip">
          {MAPPING_ORIGIN_LABELS[mapping.origin] ?? mapping.origin}
        </span>
        {mapping.isVerified ? (
          <span className="v2-allocation-chip">Syllabus content verified</span>
        ) : (
          <span className="v2-allocation-chip v2-allocation-chip-inactive">
            Syllabus content unverified
          </span>
        )}
        {mapping.isPrimary && <span className="v2-allocation-chip">Primary mapping</span>}
        {!mapping.examIsActive && (
          <span className="v2-allocation-chip v2-allocation-chip-inactive">Exam inactive</span>
        )}
      </div>
      <p className="v2-mapping-path">{mapping.nodePath.join(' → ')}</p>
      <p className="v2-empty-copy">
        {mapping.nodeKind} · weight {mapping.weight} · progress{' '}
        {PROGRESS_LABELS[mapping.progressStatus] ?? mapping.progressStatus}
      </p>
      <p className="v2-empty-copy">
        Source: {mapping.sourceTitle} ({mapping.sourcePublisher}) · verified{' '}
        {mapping.sourceVerifiedOn} · content {mapping.contentVersionLabel} (
        {mapping.contentReviewStatus})
      </p>
      <p className="v2-empty-copy">
        {mapping.provenance || 'No provenance recorded.'}
        {mapping.sourceReference ? ` · ${mapping.sourceReference}` : ''}
      </p>
      {mapping.reviewedAt && (
        <p className="v2-empty-copy">
          Reviewed {mapping.reviewedAt}
          {mapping.reviewedBy ? ` by ${mapping.reviewedBy}` : ''}.
        </p>
      )}
      <div className="v2-mapping-review">
        <label className="v2-field">
          <span>Mapping status</span>
          <select
            disabled={busy}
            onChange={(event) => setStatus(event.target.value)}
            value={status}
          >
            {Object.entries(MAPPING_STATUS_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field v2-mapping-review-notes">
          <span>Mapping notes</span>
          <textarea
            disabled={busy}
            maxLength={1000}
            onChange={(event) => setNotes(event.target.value)}
            value={notes}
          />
        </label>
        <button
          className="secondary-button"
          disabled={busy || !changed}
          onClick={() =>
            void onReview(
              mapping,
              status as 'proposed' | 'reviewed' | 'approved' | 'rejected',
              notes,
            )
          }
          type="button"
        >
          Save review
        </button>
      </div>
    </li>
  );
}
