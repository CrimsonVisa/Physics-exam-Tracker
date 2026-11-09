import { useEffect, useMemo, useRef, useState } from 'react';
import type { V2ExamSummary } from '../api';
import { FormulaEquation as MathExpression } from '../formulas/FormulaEquation';
import { EXAM_CODES, EXAM_META } from '../formulas/taxonomy';
import type { ExamCode } from '../formulas/types';
import { TERM_LIBRARY } from './data';
import { FORMULA_REF_TITLES } from './formulaRefs';
import { groupTerms, searchTerms, termExamCodes, termExamRelevance, topicOptions } from './search';
import { TERM_KINDS, TERM_KIND_LABELS, TERM_SUBJECTS } from './taxonomy';
import type { TermKind, TermSubjectId } from './types';
import { TermDetail } from './TermDetail';

type TermReferenceScreenProps = {
  exams: V2ExamSummary[];
  initialExamCode?: string | null;
  initialTermId?: string | null;
  initialSubject?: string | null;
  initialTopic?: string | null;
  initialKind?: string | null;
  initialNodeCode?: string | null;
  initialNodeTitle?: string | null;
  initialFormulaId?: string | null;
  initialFormulaTitle?: string | null;
  initialQuery?: string | null;
  onOpenFormula?: (formulaId: string, formulaTitle: string) => void;
  onOpenSyllabus?: (examCode: ExamCode, nodeCode: string, nodeTitle: string) => void;
};

function isExamCode(value: string | null | undefined): value is ExamCode {
  return value !== null && value !== undefined && EXAM_CODES.includes(value as ExamCode);
}

function isSubjectId(value: string | null | undefined): value is TermSubjectId {
  return (
    value !== null && value !== undefined && TERM_SUBJECTS.some((subject) => subject.id === value)
  );
}

function isKind(value: string | null | undefined): value is TermKind {
  return value !== null && value !== undefined && TERM_KINDS.some((kind) => kind.id === value);
}

function examShortName(exams: V2ExamSummary[], examCode: ExamCode): string {
  return exams.find((exam) => exam.code === examCode)?.shortName ?? EXAM_META[examCode].shortName;
}

export function TermReferenceScreen({
  exams,
  initialExamCode = null,
  initialTermId = null,
  initialSubject = null,
  initialTopic = null,
  initialKind = null,
  initialNodeCode = null,
  initialNodeTitle = null,
  initialFormulaId = null,
  initialFormulaTitle = null,
  initialQuery = null,
  onOpenFormula,
  onOpenSyllabus,
}: TermReferenceScreenProps) {
  const [query, setQuery] = useState(initialQuery ?? '');
  const [examCode, setExamCode] = useState<ExamCode | 'all'>(() =>
    isExamCode(initialExamCode) && exams.some((exam) => exam.code === initialExamCode)
      ? initialExamCode
      : 'all',
  );
  const [subject, setSubject] = useState<TermSubjectId | 'all'>(
    isSubjectId(initialSubject) ? initialSubject : 'all',
  );
  const [topic, setTopic] = useState(initialTopic ?? 'all');
  const [kind, setKind] = useState<TermKind | 'all'>(isKind(initialKind) ? initialKind : 'all');
  const [nodeCode, setNodeCode] = useState<string | null>(initialNodeCode);
  const [nodeTitle, setNodeTitle] = useState<string | null>(initialNodeTitle);
  const [formulaId, setFormulaId] = useState<string | null>(initialFormulaId);
  const [formulaTitle, setFormulaTitle] = useState<string | null>(
    initialFormulaTitle ??
      (initialFormulaId ? (FORMULA_REF_TITLES[initialFormulaId] ?? initialFormulaId) : null),
  );
  const [selectedTermId, setSelectedTermId] = useState<string | null>(initialTermId);
  const detailHeadingRef = useRef<HTMLHeadingElement>(null);
  const listHeadingRef = useRef<HTMLHeadingElement>(null);

  const filters = useMemo(
    () => ({ examCode, subject, topic, kind, nodeCode, formulaId }),
    [examCode, subject, topic, kind, nodeCode, formulaId],
  );

  const filtered = useMemo(
    () => searchTerms(TERM_LIBRARY.entries, query, filters),
    [query, filters],
  );

  const groups = useMemo(() => groupTerms(filtered), [filtered]);

  const topics = useMemo(
    () =>
      topicOptions(TERM_LIBRARY.entries, {
        examCode,
        subject,
        kind,
        nodeCode,
        formulaId,
      }),
    [examCode, subject, kind, nodeCode, formulaId],
  );

  const subjectScopeEntries = useMemo(
    () =>
      searchTerms(TERM_LIBRARY.entries, query, {
        examCode,
        subject: 'all',
        topic,
        kind,
        nodeCode,
        formulaId,
      }),
    [query, examCode, topic, kind, nodeCode, formulaId],
  );

  const counts = useMemo(() => {
    const next = Object.fromEntries(
      TERM_SUBJECTS.map((termSubject) => [termSubject.id, 0]),
    ) as Record<TermSubjectId, number>;
    for (const entry of subjectScopeEntries) {
      next[entry.subject] += 1;
    }
    return next;
  }, [subjectScopeEntries]);

  const selectedTerm =
    selectedTermId === null
      ? null
      : (TERM_LIBRARY.entries.find((entry) => entry.id === selectedTermId) ?? null);

  const selectedExam =
    examCode === 'all' ? null : (exams.find((exam) => exam.code === examCode) ?? null);

  useEffect(() => {
    if (selectedTerm) {
      detailHeadingRef.current?.focus();
    }
  }, [selectedTerm]);

  function clearFilters() {
    setQuery('');
    setExamCode('all');
    setSubject('all');
    setTopic('all');
    setKind('all');
    setNodeCode(null);
    setNodeTitle(null);
    setFormulaId(null);
    setFormulaTitle(null);
  }

  function openTerm(termId: string) {
    setSelectedTermId(termId);
  }

  function backToList() {
    setSelectedTermId(null);
    window.setTimeout(() => listHeadingRef.current?.focus(), 0);
  }

  if (selectedTerm) {
    return (
      <TermDetail
        entries={TERM_LIBRARY.entries}
        entry={selectedTerm}
        exams={exams}
        headingRef={detailHeadingRef}
        onBack={backToList}
        onOpenFormula={onOpenFormula}
        onOpenRelated={openTerm}
        onOpenSyllabus={onOpenSyllabus}
      />
    );
  }

  const activeExamLabel =
    examCode === 'all' ? null : (selectedExam?.shortName ?? EXAM_META[examCode].shortName);

  return (
    <div className="page-container v2-terms-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · terms and units</p>
        <h1 ref={listHeadingRef} tabIndex={-1}>
          Physics Terms &amp; SI Units
        </h1>
        <p className="page-lede">
          A curated reference of physics terminology, standard symbols and notation, SI units and
          dimensional formulas for CSIR-NET, GATE Physics, TIFR GS and JEST preparation.
        </p>
        <p className="v2-empty-copy">
          Library reviewed {TERM_LIBRARY.lastReviewed}. {TERM_LIBRARY.sourceVerificationNote}
        </p>
      </header>

      {activeExamLabel || nodeCode || formulaId ? (
        <div className="v2-terms-context" role="note">
          <div>
            {activeExamLabel ? (
              <p>
                <strong>{activeExamLabel}</strong>
                {selectedExam?.isActive === false ? (
                  <span className="v2-status-badge v2-status-inactive">Inactive examination</span>
                ) : null}
              </p>
            ) : null}
            {nodeCode ? (
              <p>
                <strong>Showing terms linked to:</strong> {nodeTitle ?? nodeCode}
              </p>
            ) : null}
            {formulaId ? (
              <p>
                <strong>Showing terms related to:</strong>{' '}
                {formulaTitle ?? FORMULA_REF_TITLES[formulaId] ?? formulaId}
              </p>
            ) : null}
          </div>
          <div className="v2-terms-context-actions">
            {nodeCode ? (
              <button
                className="link-button"
                onClick={() => {
                  setNodeCode(null);
                  setNodeTitle(null);
                }}
                type="button"
              >
                Clear syllabus filter
              </button>
            ) : null}
            {formulaId ? (
              <button
                className="link-button"
                onClick={() => {
                  setFormulaId(null);
                  setFormulaTitle(null);
                }}
                type="button"
              >
                Clear formula filter
              </button>
            ) : null}
          </div>
        </div>
      ) : null}

      <section className="v2-terms-toolbar" aria-label="Term filters">
        <label className="v2-field v2-terms-search">
          <span>Search terms</span>
          <input
            aria-label="Search terms"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Term, alias, symbol, unit, dimension or keyword"
            type="search"
            value={query}
          />
        </label>
        <label className="v2-field">
          <span>Examination</span>
          <select
            aria-label="Examination filter"
            onChange={(event) => {
              setExamCode(event.target.value as ExamCode | 'all');
              setTopic('all');
              setNodeCode(null);
              setNodeTitle(null);
            }}
            value={examCode}
          >
            <option value="all">All examinations</option>
            {exams.map((exam) => (
              <option key={exam.code} value={exam.code}>
                {exam.shortName}
                {exam.isActive ? '' : ' (inactive)'}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Topic</span>
          <select
            aria-label="Topic filter"
            onChange={(event) => setTopic(event.target.value)}
            value={topic}
          >
            <option value="all">All topics</option>
            {topics.map((topicOption) => (
              <option key={topicOption} value={topicOption}>
                {topicOption}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Kind</span>
          <select
            aria-label="Kind filter"
            onChange={(event) => setKind(event.target.value as TermKind | 'all')}
            value={kind}
          >
            <option value="all">All kinds</option>
            {TERM_KINDS.map((termKind) => (
              <option key={termKind.id} value={termKind.id}>
                {termKind.label}
              </option>
            ))}
          </select>
        </label>
        <button className="secondary-button v2-terms-clear" onClick={clearFilters} type="button">
          Clear filters
        </button>
      </section>

      <p className="v2-terms-count" aria-live="polite">
        {filtered.length} term{filtered.length === 1 ? '' : 's'}
      </p>

      <div className="v2-terms-layout">
        <nav aria-label="Filter by subject" className="v2-terms-subjects">
          <button
            aria-pressed={subject === 'all'}
            className="v2-terms-subject-button"
            onClick={() => {
              setSubject('all');
              setTopic('all');
            }}
            type="button"
          >
            <span>All subjects</span>
            <span>{subjectScopeEntries.length}</span>
          </button>
          {TERM_SUBJECTS.map((termSubject) => (
            <button
              aria-pressed={subject === termSubject.id}
              className="v2-terms-subject-button"
              key={termSubject.id}
              onClick={() => {
                setSubject(termSubject.id);
                setTopic('all');
              }}
              type="button"
            >
              <span>{termSubject.label}</span>
              <span>{counts[termSubject.id]}</span>
            </button>
          ))}
        </nav>

        <section aria-label="Term results" className="v2-terms-results">
          {filtered.length === 0 ? (
            <div className="v2-empty-panel v2-terms-empty">
              <h2>No terms match these filters</h2>
              <p>
                Try a shorter search, a different examination, subject or kind, or clear the
                syllabus/formula filter.
              </p>
              <button className="secondary-button" onClick={clearFilters} type="button">
                Clear filters
              </button>
            </div>
          ) : (
            groups.map((group) => (
              <section className="v2-terms-group" key={group.subject}>
                <h2>{group.label}</h2>
                {group.topics.map((topicGroup) => (
                  <div className="v2-terms-topic" key={topicGroup.topic}>
                    <h3>{topicGroup.topic}</h3>
                    <ul className="v2-terms-card-list">
                      {topicGroup.entries.map((entry) => (
                        <li key={entry.id}>
                          <button
                            aria-label={`Open term ${entry.term}`}
                            className="v2-terms-card"
                            data-term-id={entry.id}
                            onClick={() => openTerm(entry.id)}
                            type="button"
                          >
                            <span className="v2-terms-card-title">{entry.term}</span>
                            {entry.symbol ? (
                              <span className="v2-terms-card-symbol">
                                <MathExpression
                                  latex={entry.symbol}
                                  plainText={entry.symbolPlainText ?? entry.symbol}
                                />
                              </span>
                            ) : null}
                            <span className="v2-terms-card-definition">{entry.definition}</span>
                            <span className="v2-terms-card-meta">
                              <span className="v2-terms-badge" data-tone="kind">
                                {TERM_KIND_LABELS[entry.kind]}
                              </span>
                              {entry.siUnit ? (
                                <span className="v2-terms-badge" data-tone="unit">
                                  {entry.siUnit}
                                </span>
                              ) : null}
                              {entry.dimensions ? (
                                <span className="v2-terms-badge" data-tone="dimension">
                                  {entry.dimensions === '1' ? 'dimensionless' : entry.dimensions}
                                </span>
                              ) : null}
                              {termExamCodes(entry).length === 0 ? (
                                <span className="v2-terms-badge" data-tone="general">
                                  General reference
                                </span>
                              ) : (
                                termExamCodes(entry).map((entryExamCode) => {
                                  const relevance = termExamRelevance(entry, entryExamCode);
                                  return (
                                    <span
                                      className="v2-terms-badge"
                                      data-exam-code={entryExamCode}
                                      data-relevance={relevance ?? undefined}
                                      data-tone="exam"
                                      key={entryExamCode}
                                    >
                                      {examShortName(exams, entryExamCode)}
                                      {entryExamCode === 'tifr-gs-physics' && relevance
                                        ? ` · ${relevance}`
                                        : ''}
                                    </span>
                                  );
                                })
                              )}
                            </span>
                          </button>
                        </li>
                      ))}
                    </ul>
                  </div>
                ))}
              </section>
            ))
          )}
        </section>
      </div>
    </div>
  );
}
