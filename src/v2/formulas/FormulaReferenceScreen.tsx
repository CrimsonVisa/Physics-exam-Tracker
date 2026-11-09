import { useEffect, useMemo, useRef, useState } from 'react';
import type { V2ExamSummary } from '../api';
import { FORMULA_LIBRARY } from './data';
import { FormulaDetail } from './FormulaDetail';
import { FormulaEquation } from './FormulaEquation';
import {
  entryExamCodes,
  entryExamRelevance,
  groupFormulas,
  searchFormulas,
  topicOptions,
} from './search';
import { EXAM_META, EXACTNESS_LABELS, FORMULA_SUBJECTS, FRAMEWORK_LABELS } from './taxonomy';
import type { ExamCode, FormulaSubjectId } from './types';

type FormulaReferenceScreenProps = {
  exams: V2ExamSummary[];
  initialExamCode?: string | null;
  initialFormulaId?: string | null;
  initialSubject?: string | null;
  initialTopic?: string | null;
  initialNodeCode?: string | null;
  initialNodeTitle?: string | null;
  initialQuery?: string | null;
  onOpenSyllabus?: (examCode: ExamCode, nodeCode: string, nodeTitle: string) => void;
  onOpenTerms?: (formulaId: string, formulaTitle: string) => void;
};

const EXAM_CODES = Object.keys(EXAM_META) as ExamCode[];

function isExamCode(value: string | null | undefined): value is ExamCode {
  return value !== null && value !== undefined && EXAM_CODES.includes(value as ExamCode);
}

function isSubjectId(value: string | null | undefined): value is FormulaSubjectId {
  return (
    value !== null &&
    value !== undefined &&
    FORMULA_SUBJECTS.some((subject) => subject.id === value)
  );
}

function examShortName(exams: V2ExamSummary[], examCode: ExamCode): string {
  return exams.find((exam) => exam.code === examCode)?.shortName ?? EXAM_META[examCode].shortName;
}

export function FormulaReferenceScreen({
  exams,
  initialExamCode = null,
  initialFormulaId = null,
  initialSubject = null,
  initialTopic = null,
  initialNodeCode = null,
  initialNodeTitle = null,
  initialQuery = null,
  onOpenSyllabus,
  onOpenTerms,
}: FormulaReferenceScreenProps) {
  const [query, setQuery] = useState(initialQuery ?? '');
  const [examCode, setExamCode] = useState<ExamCode | 'all'>(() =>
    isExamCode(initialExamCode) && exams.some((exam) => exam.code === initialExamCode)
      ? initialExamCode
      : 'all',
  );
  const [subject, setSubject] = useState<FormulaSubjectId | 'all'>(
    isSubjectId(initialSubject) ? initialSubject : 'all',
  );
  const [topic, setTopic] = useState(initialTopic ?? 'all');
  const [nodeCode, setNodeCode] = useState<string | null>(initialNodeCode);
  const [nodeTitle, setNodeTitle] = useState<string | null>(initialNodeTitle);
  const [selectedFormulaId, setSelectedFormulaId] = useState<string | null>(initialFormulaId);
  const detailHeadingRef = useRef<HTMLHeadingElement>(null);
  const listHeadingRef = useRef<HTMLHeadingElement>(null);

  const filters = useMemo(
    () => ({ examCode, subject, topic, nodeCode }),
    [examCode, subject, topic, nodeCode],
  );

  const filtered = useMemo(
    () => searchFormulas(FORMULA_LIBRARY.entries, query, filters),
    [query, filters],
  );

  const groups = useMemo(() => groupFormulas(filtered), [filtered]);

  const topics = useMemo(
    () => topicOptions(FORMULA_LIBRARY.entries, { examCode, subject, nodeCode }),
    [examCode, subject, nodeCode],
  );

  const subjectScopeEntries = useMemo(
    () =>
      searchFormulas(FORMULA_LIBRARY.entries, query, {
        examCode,
        subject: 'all',
        topic,
        nodeCode,
      }),
    [query, examCode, topic, nodeCode],
  );

  const subjectCounts = useMemo(() => {
    const counts = {} as Record<FormulaSubjectId, number>;
    for (const formulaSubject of FORMULA_SUBJECTS) {
      counts[formulaSubject.id] = 0;
    }
    for (const entry of subjectScopeEntries) {
      counts[entry.subject] += 1;
    }
    return counts;
  }, [subjectScopeEntries]);

  const selectedFormula =
    selectedFormulaId === null
      ? null
      : (FORMULA_LIBRARY.entries.find((entry) => entry.id === selectedFormulaId) ?? null);

  const selectedExam =
    examCode === 'all' ? null : (exams.find((exam) => exam.code === examCode) ?? null);

  useEffect(() => {
    if (selectedFormula) {
      detailHeadingRef.current?.focus();
    }
  }, [selectedFormula]);

  function clearFilters() {
    setQuery('');
    setExamCode('all');
    setSubject('all');
    setTopic('all');
    setNodeCode(null);
    setNodeTitle(null);
  }

  function openFormula(formulaId: string) {
    setSelectedFormulaId(formulaId);
  }

  function backToList() {
    setSelectedFormulaId(null);
    window.setTimeout(() => listHeadingRef.current?.focus(), 0);
  }

  if (selectedFormula) {
    return (
      <FormulaDetail
        entries={FORMULA_LIBRARY.entries}
        entry={selectedFormula}
        exams={exams}
        headingRef={detailHeadingRef}
        onBack={backToList}
        onOpenRelated={openFormula}
        onOpenSyllabus={onOpenSyllabus}
        onOpenTerms={onOpenTerms}
      />
    );
  }

  const activeExamLabel =
    examCode === 'all' ? null : (selectedExam?.shortName ?? EXAM_META[examCode].shortName);

  return (
    <div className="page-container v2-formula-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · formula reference</p>
        <h1 ref={listHeadingRef} tabIndex={-1}>
          Physics Formula Reference
        </h1>
        <p className="page-lede">
          A curated, searchable reference of physics equations with physical meaning, symbol
          definitions, assumptions, conventions, and syllabus relevance. This is a reference
          library, not an equation solver or formula sheet generator.
        </p>
      </header>

      {activeExamLabel || nodeCode ? (
        <div className="v2-formula-context" role="note">
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
                <strong>Showing formulas linked to:</strong> {nodeTitle ?? nodeCode}
              </p>
            ) : null}
          </div>
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
        </div>
      ) : null}

      <section className="v2-formula-toolbar" aria-label="Formula filters">
        <label className="v2-field v2-formula-search">
          <span>Search formulas</span>
          <input
            aria-label="Search formulas"
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Title, equation, symbol, or keyword"
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
        <button className="secondary-button v2-formula-clear" onClick={clearFilters} type="button">
          Clear filters
        </button>
      </section>

      <p className="v2-formula-count" aria-live="polite">
        {filtered.length} formula{filtered.length === 1 ? '' : 's'}
      </p>

      <div className="v2-formula-layout">
        <nav aria-label="Filter by subject" className="v2-formula-subjects">
          <button
            aria-pressed={subject === 'all'}
            className="v2-formula-subject-button"
            onClick={() => {
              setSubject('all');
              setTopic('all');
            }}
            type="button"
          >
            <span>All subjects</span>
            <span>{subjectScopeEntries.length}</span>
          </button>
          {FORMULA_SUBJECTS.map((formulaSubject) => (
            <button
              aria-pressed={subject === formulaSubject.id}
              className="v2-formula-subject-button"
              key={formulaSubject.id}
              onClick={() => {
                setSubject(formulaSubject.id);
                setTopic('all');
              }}
              type="button"
            >
              <span>{formulaSubject.label}</span>
              <span>{subjectCounts[formulaSubject.id]}</span>
            </button>
          ))}
        </nav>

        <section aria-label="Formula results" className="v2-formula-results">
          {filtered.length === 0 ? (
            <div className="v2-empty-panel v2-formula-empty">
              <h2>No formulas match these filters</h2>
              <p>
                Try a shorter search, a different examination or subject, or clear the syllabus
                filter.
              </p>
              <button className="secondary-button" onClick={clearFilters} type="button">
                Clear filters
              </button>
            </div>
          ) : (
            groups.map((group) => (
              <section className="v2-formula-group" key={group.subject}>
                <h2>{group.label}</h2>
                {group.topics.map((topicGroup) => (
                  <div className="v2-formula-topic" key={topicGroup.topic}>
                    <h3>{topicGroup.topic}</h3>
                    <ul className="v2-formula-card-list">
                      {topicGroup.entries.map((entry) => (
                        <li key={entry.id}>
                          <button
                            aria-label={`Open formula ${entry.title}`}
                            className="v2-formula-card"
                            data-formula-id={entry.id}
                            onClick={() => openFormula(entry.id)}
                            type="button"
                          >
                            <span className="v2-formula-card-title">{entry.title}</span>
                            <span className="v2-formula-card-equation">
                              <FormulaEquation latex={entry.latex} plainText={entry.plainText} />
                            </span>
                            <span className="v2-formula-card-summary">{entry.summary}</span>
                            <span className="v2-formula-card-meta">
                              <span className="v2-formula-badge" data-tone="exactness">
                                {EXACTNESS_LABELS[entry.exactness]}
                              </span>
                              <span className="v2-formula-badge" data-tone="framework">
                                {FRAMEWORK_LABELS[entry.framework]}
                              </span>
                              {entryExamCodes(entry).map((entryExamCode) => {
                                const relevance = entryExamRelevance(entry, entryExamCode);
                                return (
                                  <span
                                    className="v2-formula-badge"
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
                              })}
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
