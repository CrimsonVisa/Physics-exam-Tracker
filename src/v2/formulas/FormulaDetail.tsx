import type { RefObject } from 'react';
import type { V2ExamSummary } from '../api';
import { MathText } from '../MathText';
import { MathExpression } from '../math/MathExpression';
import { FormulaEquation } from './FormulaEquation';
import { entryExamCodes } from './search';
import {
  EXAM_META,
  EXACTNESS_LABELS,
  FRAMEWORK_LABELS,
  RELEVANCE_LABELS,
  SUBJECT_LABELS,
} from './taxonomy';
import type { ExamCode, FormulaEntry } from './types';

type FormulaDetailProps = {
  entry: FormulaEntry;
  entries: FormulaEntry[];
  exams: V2ExamSummary[];
  onBack: () => void;
  onOpenRelated: (formulaId: string) => void;
  onOpenSyllabus?: (examCode: ExamCode, nodeCode: string, nodeTitle: string) => void;
  onOpenTerms?: (formulaId: string, formulaTitle: string) => void;
  headingRef?: RefObject<HTMLHeadingElement | null>;
};

function examSummary(exams: V2ExamSummary[], examCode: ExamCode): V2ExamSummary | null {
  return exams.find((exam) => exam.code === examCode) ?? null;
}

function examName(exams: V2ExamSummary[], examCode: ExamCode): string {
  return examSummary(exams, examCode)?.shortName ?? EXAM_META[examCode].shortName;
}

export function FormulaDetail({
  entry,
  entries,
  exams,
  onBack,
  onOpenRelated,
  onOpenSyllabus,
  onOpenTerms,
  headingRef,
}: FormulaDetailProps) {
  const relatedEntries = (entry.relatedIds ?? [])
    .map((relatedId) => entries.find((candidate) => candidate.id === relatedId))
    .filter((candidate): candidate is FormulaEntry => candidate !== undefined);

  return (
    <div className="page-container v2-formula-detail" data-formula-id={entry.id}>
      <button className="secondary-button v2-back-button" onClick={onBack} type="button">
        ← Back to formulas
      </button>

      <header className="page-header v2-formula-detail-header">
        <p className="eyebrow">
          {SUBJECT_LABELS[entry.subject]} · {entry.topic}
          {entry.subtopic ? ` · ${entry.subtopic}` : ''}
        </p>
        <h1 ref={headingRef} tabIndex={-1}>
          {entry.title}
        </h1>
        <div className="v2-formula-badges" aria-label="Formula classifications">
          <span className="v2-formula-badge" data-tone="exactness">
            {EXACTNESS_LABELS[entry.exactness]}
          </span>
          <span className="v2-formula-badge" data-tone="framework">
            {FRAMEWORK_LABELS[entry.framework]}
          </span>
          {entryExamCodes(entry).map((examCode) => {
            const exam = examSummary(exams, examCode);
            return (
              <span
                className="v2-formula-badge"
                data-exam-code={examCode}
                data-tone={exam?.isActive === false ? 'inactive' : 'exam'}
                key={examCode}
              >
                {examName(exams, examCode)}
                {exam?.isActive === false ? ' · inactive' : ''}
              </span>
            );
          })}
        </div>
      </header>

      <section className="v2-concept-card v2-formula-equation-card" aria-label="Formula">
        <FormulaEquation display latex={entry.latex} plainText={entry.plainText} />
      </section>

      <div className="v2-formula-detail-grid">
        <div className="v2-formula-detail-main">
          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-meaning-heading"
          >
            <h2 id="formula-meaning-heading">Physical meaning</h2>
            <p className="v2-formula-summary">
              <MathText text={entry.summary} />
            </p>
          </section>

          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-symbols-heading"
          >
            <h2 id="formula-symbols-heading">Symbols, units and dimensions</h2>
            <dl className="v2-formula-symbols">
              {entry.symbols.map((symbol, index) => (
                <div key={`${symbol.symbol}-${index}`}>
                  <dt>
                    <FormulaEquation
                      latex={symbol.symbol}
                      plainText={symbol.symbol}
                      className="v2-formula-symbol"
                    />
                  </dt>
                  <dd>
                    <span>
                      <MathText text={symbol.meaning} />
                    </span>
                    {symbol.unit ? (
                      <span>
                        Unit: <MathExpression latex={symbol.unit} plainText={symbol.unit} />
                      </span>
                    ) : null}
                    {symbol.dimensions ? (
                      <span>
                        Dimensions:{' '}
                        <MathExpression latex={symbol.dimensions} plainText={symbol.dimensions} />
                      </span>
                    ) : null}
                  </dd>
                </div>
              ))}
            </dl>
          </section>

          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-conditions-heading"
          >
            <h2 id="formula-conditions-heading">Conditions and assumptions</h2>
            <ul className="v2-formula-list">
              {entry.conditions.map((condition) => (
                <li key={condition}>
                  <MathText text={condition} />
                </li>
              ))}
            </ul>
          </section>

          {entry.specialCases && entry.specialCases.length > 0 ? (
            <section
              className="v2-concept-card v2-formula-detail-card"
              aria-labelledby="formula-special-heading"
            >
              <h2 id="formula-special-heading">Special cases and limits</h2>
              <ul className="v2-formula-list">
                {entry.specialCases.map((specialCase) => (
                  <li key={specialCase}>
                    <MathText text={specialCase} />
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {entry.conventions && entry.conventions.length > 0 ? (
            <section
              className="v2-concept-card v2-formula-detail-card"
              aria-labelledby="formula-conventions-heading"
            >
              <h2 id="formula-conventions-heading">Conventions</h2>
              <ul className="v2-formula-list">
                {entry.conventions.map((convention) => (
                  <li key={convention}>
                    <MathText text={convention} />
                  </li>
                ))}
              </ul>
            </section>
          ) : null}
        </div>

        <aside className="v2-formula-detail-side">
          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-syllabus-heading"
          >
            <h2 id="formula-syllabus-heading">Examination and syllabus links</h2>
            <ul className="v2-formula-syllabus-list">
              {entry.syllabusLinks.map((syllabusLink, linkIndex) => {
                const exam = examSummary(exams, syllabusLink.examCode);
                return (
                  <li key={`${syllabusLink.examCode}-${linkIndex}`}>
                    <div className="v2-formula-syllabus-head">
                      <strong>{examName(exams, syllabusLink.examCode)}</strong>
                      <span
                        className="v2-formula-relevance"
                        data-relevance={syllabusLink.relevance}
                      >
                        {RELEVANCE_LABELS[syllabusLink.relevance]}
                      </span>
                      {exam?.isActive === false ? (
                        <span className="v2-status-badge v2-status-inactive">Inactive</span>
                      ) : null}
                    </div>
                    {syllabusLink.nodes.map((node) => (
                      <div className="v2-formula-syllabus-node" key={node.code}>
                        <span>{node.title}</span>
                        {onOpenSyllabus ? (
                          <button
                            aria-label={`Open syllabus topic ${node.title}`}
                            className="link-button"
                            onClick={() =>
                              onOpenSyllabus(syllabusLink.examCode, node.code, node.title)
                            }
                            type="button"
                          >
                            Open syllabus topic
                          </button>
                        ) : null}
                      </div>
                    ))}
                    {syllabusLink.note ? (
                      <p className="v2-empty-copy">{syllabusLink.note}</p>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          </section>

          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-related-heading"
          >
            <h2 id="formula-related-heading">Related formulas</h2>
            {relatedEntries.length === 0 ? (
              <p className="v2-empty-copy">No related formulas are listed.</p>
            ) : (
              <div className="v2-formula-related-list">
                {relatedEntries.map((related) => (
                  <button
                    className="secondary-button v2-formula-related-button"
                    key={related.id}
                    onClick={() => onOpenRelated(related.id)}
                    type="button"
                  >
                    {related.title}
                  </button>
                ))}
              </div>
            )}
          </section>

          {onOpenTerms ? (
            <section
              className="v2-concept-card v2-formula-detail-card"
              aria-labelledby="formula-terms-heading"
            >
              <h2 id="formula-terms-heading">Related terminology</h2>
              <button
                className="secondary-button v2-formula-related-button"
                onClick={() => onOpenTerms(entry.id, entry.title)}
                type="button"
              >
                Browse related terms and units
              </button>
            </section>
          ) : null}

          <section
            className="v2-concept-card v2-formula-detail-card"
            aria-labelledby="formula-sources-heading"
          >
            <h2 id="formula-sources-heading">Sources and provenance</h2>
            <ul className="v2-formula-sources">
              {entry.sources.map((source, index) => (
                <li key={`${source.citation}-${index}`}>
                  <strong>{source.citation}</strong>
                  {source.locator ? (
                    <>
                      <span> · {source.locator}</span>
                      <span
                        className="v2-formula-source-status"
                        data-check={source.locatorCheck ?? 'unverified'}
                        title={
                          source.locatorCheck === 'chapter-checked'
                            ? 'Chapter-level check only; the exact formula content was not independently verified.'
                            : 'This locator has not been checked against the cited source.'
                        }
                      >
                        {source.locatorCheck === 'chapter-checked'
                          ? 'chapter checked'
                          : 'unverified locator'}
                      </span>
                    </>
                  ) : (
                    <span
                      className="v2-formula-source-status"
                      data-check="unverified"
                      title="No locator was supplied for this source."
                    >
                      locator not provided
                    </span>
                  )}
                  {source.url ? <span> · {source.url}</span> : null}
                </li>
              ))}
            </ul>
            <p className="v2-empty-copy">
              “Chapter checked” means the cited chapter or section was inspected, not that the exact
              formula content was independently verified. Sources with no check remain explicitly
              unverified.
            </p>
            <p className="v2-empty-copy">
              Syllabus links record curated relevance. They are not an official endorsement of a
              formula by the examination authority.
            </p>
          </section>
        </aside>
      </div>
    </div>
  );
}
