import type { RefObject } from 'react';
import type { V2ExamSummary } from '../api';
import { FormulaEquation as MathExpression } from '../formulas/FormulaEquation';
import { EXAM_META, RELEVANCE_LABELS } from '../formulas/taxonomy';
import type { ExamCode } from '../formulas/types';
import { FORMULA_REF_TITLES } from './formulaRefs';
import { TERM_KIND_LABELS, TERM_SUBJECT_LABELS } from './taxonomy';
import type { TermEntry } from './types';

type TermDetailProps = {
  entry: TermEntry;
  entries: TermEntry[];
  exams: V2ExamSummary[];
  onBack: () => void;
  onOpenRelated: (termId: string) => void;
  onOpenFormula?: (formulaId: string, formulaTitle: string) => void;
  onOpenSyllabus?: (examCode: ExamCode, nodeCode: string, nodeTitle: string) => void;
  headingRef?: RefObject<HTMLHeadingElement | null>;
};

function examSummary(exams: V2ExamSummary[], examCode: ExamCode): V2ExamSummary | null {
  return exams.find((exam) => exam.code === examCode) ?? null;
}

function examName(exams: V2ExamSummary[], examCode: ExamCode): string {
  return examSummary(exams, examCode)?.shortName ?? EXAM_META[examCode].shortName;
}

function notationPlainText(value: string): string {
  const greek: Record<string, string> = {
    Omega: 'Ω',
    omega: 'ω',
    mu: 'μ',
    nu: 'ν',
    Theta: 'Θ',
    theta: 'θ',
    varepsilon: 'ε',
    epsilon: 'ε',
    phi: 'φ',
    Psi: 'Ψ',
    psi: 'ψ',
    Delta: 'Δ',
    delta: 'δ',
    sigma: 'σ',
    tau: 'τ',
    lambda: 'λ',
    rho: 'ρ',
    pi: 'π',
  };
  let text = value
    .replace(/\\\{/g, '\u0001')
    .replace(/\\\}/g, '\u0002')
    .replace(/\\langle/g, '⟨')
    .replace(/\\rangle/g, '⟩')
    .replace(/\\(?:lvert|rvert|vert)/g, '|');
  for (const [command, symbol] of Object.entries(greek)) {
    text = text.split(`\\${command}`).join(symbol);
  }
  return text
    .replace(/\\operatorname\{([^{}]*)\}/g, '$1')
    .replace(/\\frac\{([^{}]*)\}\{([^{}]*)\}/g, '$1/$2')
    .replace(/\\sqrt\{([^{}]*)\}/g, 'sqrt($1)')
    .replace(/\\(?:mathrm|mathbf|vec|hat|bar|tilde|mathcal|left|right|quad|,|;|!| )/g, '')
    .replace(/\\partial/g, '∂')
    .replace(/\\nabla/g, '∇')
    .replace(/[{}]/g, '')
    .replace(/\\/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .split('\u0001')
    .join('{')
    .split('\u0002')
    .join('}');
}

function SourceStatus({ source }: { source: TermEntry['sources'][number] }) {
  if (!source.locator) {
    return (
      <span
        className="v2-terms-source-status"
        data-check="unverified"
        title="No locator was supplied for this source."
      >
        locator not provided
      </span>
    );
  }
  return (
    <span
      className="v2-terms-source-status"
      data-check={source.locatorCheck ?? 'unverified'}
      title={
        source.locatorCheck === 'chapter-checked'
          ? 'Chapter-level check only; the exact definition content was not independently verified.'
          : 'This locator has not been checked against the cited source.'
      }
    >
      {source.locatorCheck === 'chapter-checked' ? 'chapter checked' : 'unverified locator'}
    </span>
  );
}

export function TermDetail({
  entry,
  entries,
  exams,
  onBack,
  onOpenRelated,
  onOpenFormula,
  onOpenSyllabus,
  headingRef,
}: TermDetailProps) {
  const relatedTerms = (entry.relatedTermIds ?? [])
    .map((id) => entries.find((candidate) => candidate.id === id))
    .filter((candidate): candidate is TermEntry => candidate !== undefined);
  const relatedFormulas = (entry.relatedFormulaIds ?? []).map((id) => ({
    id,
    title: FORMULA_REF_TITLES[id] ?? id,
  }));

  return (
    <div className="page-container v2-terms-detail" data-term-id={entry.id}>
      <button className="secondary-button v2-back-button" onClick={onBack} type="button">
        ← Back to terms
      </button>

      <header className="page-header v2-terms-detail-header">
        <p className="eyebrow">
          {TERM_SUBJECT_LABELS[entry.subject]} · {entry.topic}
          {entry.subtopic ? ` · ${entry.subtopic}` : ''}
        </p>
        <h1 ref={headingRef} tabIndex={-1}>
          {entry.term}
        </h1>
        {entry.aliases && entry.aliases.length > 0 ? (
          <p className="v2-terms-aliases">Also: {entry.aliases.join(', ')}</p>
        ) : null}
        <div className="v2-terms-badges" aria-label="Term classifications">
          <span className="v2-terms-badge" data-tone="kind">
            {TERM_KIND_LABELS[entry.kind]}
          </span>
          {entry.syllabusLinks.length === 0 ? (
            <span className="v2-terms-badge" data-tone="general">
              General reference
            </span>
          ) : null}
          {[...new Set(entry.syllabusLinks.map((link) => link.examCode))].map((examCode) => {
            const exam = examSummary(exams, examCode);
            return (
              <span
                className="v2-terms-badge"
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

      {entry.symbol ||
      entry.siUnit ||
      entry.dimensions ||
      (entry.notationVariants && entry.notationVariants.length > 0) ? (
        <section
          className="v2-concept-card v2-terms-reference-card"
          aria-label="Symbol, unit and dimensions"
        >
          <h2>Symbol, unit and dimensions</h2>
          <dl className="v2-terms-reference-grid">
            {entry.symbol ? (
              <div>
                <dt>Symbol</dt>
                <dd>
                  <MathExpression
                    latex={entry.symbol}
                    plainText={entry.symbolPlainText ?? entry.symbol}
                    className="v2-terms-symbol"
                  />
                </dd>
              </div>
            ) : null}
            {entry.quantity ? (
              <div>
                <dt>Quantity</dt>
                <dd>{entry.quantity}</dd>
              </div>
            ) : null}
            {entry.siUnit ? (
              <div>
                <dt>SI unit</dt>
                <dd>
                  {entry.siUnitLatex ? (
                    <MathExpression
                      latex={entry.siUnitLatex}
                      plainText={entry.siUnit}
                      className="v2-terms-symbol"
                    />
                  ) : (
                    entry.siUnit
                  )}
                </dd>
              </div>
            ) : null}
            {entry.dimensions ? (
              <div>
                <dt>Dimensions</dt>
                <dd>{entry.dimensions === '1' ? 'Dimensionless (1)' : entry.dimensions}</dd>
              </div>
            ) : null}
            {entry.dimensionFormula ? (
              <div>
                <dt>Dimensional formula</dt>
                <dd>
                  <MathExpression
                    latex={entry.dimensionFormula}
                    plainText={entry.dimensionFormula.replace(/\\/g, '').replace(/[{}]/g, '')}
                    className="v2-terms-symbol"
                  />
                </dd>
              </div>
            ) : null}
            {entry.notationVariants && entry.notationVariants.length > 0 ? (
              <div>
                <dt>Notation variants</dt>
                <dd className="v2-terms-notation-list">
                  {entry.notationVariants.map((variant) => (
                    <span className="v2-terms-notation-variant" key={variant}>
                      <MathExpression latex={variant} plainText={notationPlainText(variant)} />
                    </span>
                  ))}
                </dd>
              </div>
            ) : null}
          </dl>
        </section>
      ) : null}

      <div className="v2-terms-detail-grid">
        <div className="v2-terms-detail-main">
          <section
            className="v2-concept-card v2-terms-detail-card"
            aria-labelledby="term-definition-heading"
          >
            <h2 id="term-definition-heading">Definition</h2>
            <p className="v2-terms-definition">{entry.definition}</p>
          </section>

          {entry.conditions && entry.conditions.length > 0 ? (
            <section
              className="v2-concept-card v2-terms-detail-card"
              aria-labelledby="term-conditions-heading"
            >
              <h2 id="term-conditions-heading">Conditions and conventions</h2>
              <ul className="v2-terms-list">
                {entry.conditions.map((condition) => (
                  <li key={condition}>{condition}</li>
                ))}
              </ul>
            </section>
          ) : null}

          <section
            className="v2-concept-card v2-terms-detail-card"
            aria-labelledby="term-related-heading"
          >
            <h2 id="term-related-heading">Related terms</h2>
            {relatedTerms.length === 0 ? (
              <p className="v2-empty-copy">No related terms are listed.</p>
            ) : (
              <div className="v2-terms-related-list">
                {relatedTerms.map((related) => (
                  <button
                    className="secondary-button v2-terms-related-button"
                    key={related.id}
                    onClick={() => onOpenRelated(related.id)}
                    type="button"
                  >
                    {related.term}
                  </button>
                ))}
              </div>
            )}
          </section>
        </div>

        <aside className="v2-terms-detail-side">
          <section
            className="v2-concept-card v2-terms-detail-card"
            aria-labelledby="term-formulas-heading"
          >
            <h2 id="term-formulas-heading">Related formulas</h2>
            {relatedFormulas.length === 0 ? (
              <p className="v2-empty-copy">No related formulas are listed.</p>
            ) : (
              <div className="v2-terms-related-list">
                {relatedFormulas.map((formula) => (
                  <button
                    className="secondary-button v2-terms-related-button"
                    key={formula.id}
                    onClick={() => onOpenFormula?.(formula.id, formula.title)}
                    type="button"
                  >
                    {formula.title}
                  </button>
                ))}
              </div>
            )}
          </section>

          <section
            className="v2-concept-card v2-terms-detail-card"
            aria-labelledby="term-syllabus-heading"
          >
            <h2 id="term-syllabus-heading">Examination and syllabus links</h2>
            {entry.syllabusLinks.length === 0 ? (
              <p className="v2-empty-copy">
                General reference. No defensible examination-syllabus mapping is claimed for this
                entry.
              </p>
            ) : (
              <ul className="v2-terms-syllabus-list">
                {entry.syllabusLinks.map((syllabusLink, linkIndex) => {
                  const exam = examSummary(exams, syllabusLink.examCode);
                  return (
                    <li key={`${syllabusLink.examCode}-${linkIndex}`}>
                      <div className="v2-terms-syllabus-head">
                        <strong>{examName(exams, syllabusLink.examCode)}</strong>
                        <span
                          className="v2-terms-relevance"
                          data-relevance={syllabusLink.relevance}
                        >
                          {RELEVANCE_LABELS[syllabusLink.relevance]}
                        </span>
                        {exam?.isActive === false ? (
                          <span className="v2-status-badge v2-status-inactive">Inactive</span>
                        ) : null}
                      </div>
                      {syllabusLink.nodes.map((node) => (
                        <div className="v2-terms-syllabus-node" key={node.code}>
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
            )}
          </section>

          <section
            className="v2-concept-card v2-terms-detail-card"
            aria-labelledby="term-sources-heading"
          >
            <h2 id="term-sources-heading">Sources and provenance</h2>
            <ul className="v2-terms-sources">
              {entry.sources.map((source, index) => (
                <li key={`${source.citation}-${index}`}>
                  <strong>{source.citation}</strong>
                  {source.locator ? <span> · {source.locator}</span> : null}
                  <SourceStatus source={source} />
                  {source.url ? <span> · {source.url}</span> : null}
                </li>
              ))}
            </ul>
            <p className="v2-empty-copy">
              “Chapter checked” means the cited chapter or section was inspected, not that the exact
              definition content was independently verified. Sources without a check remain
              explicitly unverified.
            </p>
          </section>
        </aside>
      </div>
    </div>
  );
}
