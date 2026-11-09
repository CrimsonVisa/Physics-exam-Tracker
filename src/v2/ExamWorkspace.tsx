import type { V2ExamSummary } from './api';
import { examDateDisplay } from './examDate';
import { StudySection } from './StudySection';

type ExamWorkspaceProps = {
  exam: V2ExamSummary;
  exams: V2ExamSummary[];
  today: string;
  onBack: () => void;
  onOpenConcepts: (examCode: string) => void;
  onOpenFormulas: (examCode: string) => void;
  onOpenTerms: (examCode: string) => void;
  onOpenSyllabus: (examCode: string) => void;
};

const EXAM_INITIALS: Record<string, string> = {
  'csir-net-physics': 'CSIR',
  'gate-physics': 'GATE',
  'tifr-gs-physics': 'TIFR',
  'jest-physics': 'JEST',
};

export function ExamWorkspace({
  exam,
  exams,
  today,
  onBack,
  onOpenConcepts,
  onOpenFormulas,
  onOpenTerms,
  onOpenSyllabus,
}: ExamWorkspaceProps) {
  const date = examDateDisplay(exam.examDate, today);

  return (
    <div className="page-container v2-workspace" data-exam-code={exam.code}>
      <button className="secondary-button v2-back-button" onClick={onBack} type="button">
        ← Back to combined dashboard
      </button>

      <header className="page-header v2-workspace-header">
        <div className="v2-workspace-hero">
          <span aria-hidden="true" className="v2-exam-tile" data-exam={exam.code}>
            {EXAM_INITIALS[exam.code] ?? exam.shortName.slice(0, 4)}
          </span>
          <div>
            <p className="eyebrow">
              {exam.shortName} · {exam.isActive ? 'Active examination' : 'Inactive examination'}
            </p>
            <h1>{exam.name}</h1>
            <p className="page-lede">
              This workspace is resolved from the V2 database by its stable exam code. It shows the
              examination identity, activation state, shared timer, and exam-specific syllabus
              progress.
            </p>
          </div>
        </div>
      </header>

      <div className="v2-workspace-actions">
        <button
          className="secondary-button v2-back-button"
          onClick={() => onOpenConcepts(exam.code)}
          type="button"
        >
          Browse concept coverage for this exam
        </button>
        <button
          className="secondary-button v2-back-button"
          onClick={() => onOpenSyllabus(exam.code)}
          type="button"
        >
          Open exam syllabus and progress
        </button>
        <button
          className="secondary-button v2-back-button"
          onClick={() => onOpenFormulas(exam.code)}
          type="button"
        >
          Browse formula reference for this exam
        </button>
        <button
          className="secondary-button v2-back-button"
          onClick={() => onOpenTerms(exam.code)}
          type="button"
        >
          Browse terms and units for this exam
        </button>
      </div>

      {!exam.isActive && (
        <section className="v2-inactive-banner" aria-labelledby="inactive-exam-heading">
          <h2 id="inactive-exam-heading">Inactive examination</h2>
          <p>
            Historical records remain in the database and this workspace stays available for review.
            Editing workflows are disabled until the examination is reactivated.
          </p>
        </section>
      )}

      <StudySection
        exams={exams}
        focusExamCode={exam.code}
        key={exam.code}
        readOnly={!exam.isActive}
        today={today}
      />

      <div className="v2-workspace-grid">
        <section className="v2-info-card" aria-labelledby="exam-identity-heading">
          <p className="eyebrow">Exam identity</p>
          <h2 id="exam-identity-heading">Configured workspace</h2>
          <dl className="v2-fact-list">
            <div>
              <dt>Short name</dt>
              <dd>{exam.shortName}</dd>
            </div>
            <div>
              <dt>Exam code</dt>
              <dd>{exam.code}</dd>
            </div>
            <div>
              <dt>Authority</dt>
              <dd>{exam.authority || 'Not recorded'}</dd>
            </div>
            <div>
              <dt>Status</dt>
              <dd>{exam.isActive ? 'Active' : 'Inactive'}</dd>
            </div>
          </dl>
        </section>

        <section className="v2-info-card" aria-labelledby="exam-date-heading">
          <p className="eyebrow">Exam date</p>
          <h2 id="exam-date-heading">{date.label}</h2>
          <p className="v2-date-detail">{date.detail}</p>
          {exam.examDate === null && (
            <p className="v2-empty-copy">
              No date is stored for this examination. Physics Tracker never guesses or fetches exam
              dates automatically.
            </p>
          )}
        </section>
      </div>

      <section className="v2-empty-panel" aria-labelledby="workspace-foundation-heading">
        <h2 id="workspace-foundation-heading">Workspace foundation</h2>
        <p>
          This workspace exposes the exam identity, activation state, shared timer, and links to the
          official exam-specific syllabus. Syllabus completion is tracked independently for this
          examination.
        </p>
      </section>
    </div>
  );
}
