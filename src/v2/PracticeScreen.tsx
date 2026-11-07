import { useCallback, useEffect, useMemo, useState, type FormEvent } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  applyV2PyqImport,
  archiveV2MockTest,
  archiveV2PyqQuestion,
  getV2PracticeAnalytics,
  getV2PracticeOptions,
  getV2PyqQuestionDetail,
  listV2MockTests,
  listV2PyqQuestions,
  previewV2PyqImport,
  recordV2PyqAttempt,
  saveV2MockTest,
  saveV2PyqQuestion,
  setV2MockTestStatus,
  type V2ExamSummary,
  type V2MockSectionInput,
  type V2MockTest,
  type V2PracticeAnalytics,
  type V2PracticeOptions,
  type V2PracticePattern,
  type V2PracticePatternSection,
  type V2PracticeTopicOption,
  type V2PyqQuestionDetail,
  type V2QuestionOption,
  type V2PyqQuestionInput,
  type V2PyqQuestionSummary,
} from './api';
import { QuestionMockPanel } from './QuestionMockPanel';
import { SourcesPanel } from './SourcesPanel';

const QUESTION_CSV_TEMPLATE = [
  'exam_code,year,session_label,section_code,question_type_code,question_number,statement,correct_answer,correct_range,max_marks,negative_marks,unanswered_marks,difficulty,source_title,source_page,source_url,answer_key_source,answer_key_version,answer_key_verified,verification_status,numeric_tolerance,syllabus_node_code,concept_slugs,dedupe_key,official_explanation,third_party_solution,user_note',
].join('\n');

const QUESTION_CSV_TEMPLATE_V2 = [
  `${QUESTION_CSV_TEMPLATE},options_json,pattern_version_id,source_id,answer_key_id`,
].join('\n');

type PracticeTab = 'pyq' | 'mock' | 'analytics' | 'sources';

type PracticeScreenProps = {
  exams: V2ExamSummary[];
  initialExamCode: string | null;
  initialQuestionId?: number | null;
  today: string;
};

function percent(value: number | null | undefined): string {
  return value === null || value === undefined ? '—' : `${value.toFixed(2)}%`;
}

function minutesLabel(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  return `${(seconds / 60).toFixed(1)} min`;
}

function parseQuestionOptions(text: string): V2QuestionOption[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line, index) => {
      const match = line.match(/^([A-Za-z0-9]{1,8})[).:-]\s*(.+)$/);
      if (match) {
        return { key: match[1].toUpperCase(), text: match[2].trim() };
      }
      return {
        key: String.fromCharCode(65 + index),
        text: line,
      };
    });
}

export function PracticeScreen({
  exams,
  initialExamCode,
  initialQuestionId = null,
  today,
}: PracticeScreenProps) {
  const defaultExam = exams.find((exam) => exam.code === initialExamCode) ?? exams[0];
  const [examId, setExamId] = useState(defaultExam?.id ?? 0);
  const [tab, setTab] = useState<PracticeTab>('pyq');
  const [mockView, setMockView] = useState<'question' | 'aggregate'>('question');
  const [options, setOptions] = useState<V2PracticeOptions | null>(null);
  const [questions, setQuestions] = useState<V2PyqQuestionSummary[]>([]);
  const [mocks, setMocks] = useState<V2MockTest[]>([]);
  const [analytics, setAnalytics] = useState<V2PracticeAnalytics | null>(null);
  const [selectedQuestion, setSelectedQuestion] = useState<V2PyqQuestionDetail | null>(null);
  const [selectedQuestionId, setSelectedQuestionId] = useState<number | null>(
    initialQuestionId ?? null,
  );
  const [selectedPatternId, setSelectedPatternId] = useState<number | null>(null);
  const [showQuestionForm, setShowQuestionForm] = useState(false);
  const [editingQuestion, setEditingQuestion] = useState<V2PyqQuestionDetail | null>(null);
  const [showImport, setShowImport] = useState(false);
  const [csv, setCsv] = useState('');
  const [importPreview, setImportPreview] = useState<Awaited<
    ReturnType<typeof previewV2PyqImport>
  > | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [yearFilter, setYearFilter] = useState('');
  const [sectionFilter, setSectionFilter] = useState('');
  const [topicFilter, setTopicFilter] = useState('');
  const [typeFilter, setTypeFilter] = useState('');
  const [verificationFilter, setVerificationFilter] = useState('');
  const [showArchived, setShowArchived] = useState(false);

  const verifiedPatterns = useMemo(
    () => options?.patterns.filter((pattern) => pattern.verificationStatus === 'verified') ?? [],
    [options],
  );
  const selectedPattern =
    verifiedPatterns.find((pattern) => pattern.id === selectedPatternId) ??
    verifiedPatterns[0] ??
    null;

  const refresh = useCallback(async () => {
    if (!examId) return;
    setLoading(true);
    setError(null);
    try {
      const nextOptions = await getV2PracticeOptions(examId);
      const [nextQuestions, nextMocks, nextAnalytics] = await Promise.all([
        listV2PyqQuestions({ examId, includeArchived: showArchived }),
        listV2MockTests(examId, showArchived),
        getV2PracticeAnalytics({
          examId,
          year: yearFilter ? Number(yearFilter) : null,
          sectionId: sectionFilter ? Number(sectionFilter) : null,
          syllabusNodeId: topicFilter ? Number(topicFilter) : null,
        }),
      ]);
      setOptions(nextOptions);
      setQuestions(nextQuestions);
      setMocks(nextMocks);
      setAnalytics(nextAnalytics);
      setSelectedPatternId((current) => {
        const verified = nextOptions.patterns.filter(
          (pattern) => pattern.verificationStatus === 'verified',
        );
        return verified.some((pattern) => pattern.id === current)
          ? current
          : (verified[0]?.id ?? null);
      });
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setLoading(false);
    }
  }, [examId, yearFilter, sectionFilter, topicFilter, showArchived]);

  useEffect(() => {
    void (async () => {
      await refresh();
    })();
  }, [refresh]);

  useEffect(() => {
    if (selectedQuestionId === null) return;
    let cancelled = false;
    void (async () => {
      try {
        const detail = await getV2PyqQuestionDetail(examId, selectedQuestionId);
        if (!cancelled) setSelectedQuestion(detail);
      } catch (caught) {
        if (!cancelled) setError(toCommandError(caught).message);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [examId, selectedQuestionId]);

  const filteredQuestions = useMemo(() => {
    const term = search.trim().toLowerCase();
    return questions.filter((question) => {
      if (yearFilter && question.year !== Number(yearFilter)) return false;
      if (sectionFilter && question.sectionId !== Number(sectionFilter)) return false;
      if (
        topicFilter &&
        question.syllabusNodeId !== Number(topicFilter) &&
        !question.topicIds.includes(Number(topicFilter))
      ) {
        return false;
      }
      if (typeFilter && question.questionTypeId !== Number(typeFilter)) return false;
      if (verificationFilter && question.verificationStatus !== verificationFilter) return false;
      if (!term) return true;
      return [
        question.statement,
        question.sourceTitle,
        question.sourcePage,
        question.officialExplanation,
        question.thirdPartySolution,
        question.userNote,
        question.syllabusNodeTitle ?? '',
      ]
        .join(' ')
        .toLowerCase()
        .includes(term);
    });
  }, [questions, search, yearFilter, sectionFilter, topicFilter, typeFilter, verificationFilter]);

  async function runAction(action: () => Promise<string>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      setNotice(await action());
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(false);
    }
  }

  const patternBanner = selectedPattern ? (
    <div className="v2-practice-pattern-strip">
      <div>
        <p className="eyebrow">Verified pattern</p>
        <strong>
          {selectedPattern.patternTitle} · {selectedPattern.versionLabel}
        </strong>
        <p className="v2-empty-copy">
          {selectedPattern.durationMinutes
            ? `${selectedPattern.durationMinutes} minutes · `
            : 'Duration not stated · '}
          {selectedPattern.totalQuestions} questions · {selectedPattern.totalMarks} marks
        </p>
      </div>
      <div className="v2-practice-pattern-source">
        <span>{selectedPattern.verificationStatus.toUpperCase()}</span>
        {selectedPattern.sourceUrl && (
          <a href={selectedPattern.sourceUrl} rel="noreferrer" target="_blank">
            Official source
          </a>
        )}
      </div>
    </div>
  ) : (
    <div className="v2-inactive-banner">
      <h2>Verified practice pattern unavailable</h2>
      <p>
        This examination has no verified production pattern. Practice questions can still be stored,
        but official scoring and mock creation are disabled until a pattern is reviewed.
      </p>
    </div>
  );

  const practiceTabs: Array<[PracticeTab, string]> = [
    ['pyq', 'PYQ bank & attempts'],
    ['mock', 'Mock tests'],
    ['analytics', 'Practice analytics'],
  ];
  if (options?.exam.code === 'gate-physics') {
    practiceTabs.push(['sources', 'Official sources']);
  }

  return (
    <div className="page-container v2-practice">
      <header className="page-header">
        <div>
          <p className="eyebrow">Physics Tracker · Practice</p>
          <h1>Practice systems</h1>
          <p>
            Exam-specific PYQs, attempts, mock-test history and persisted practice analytics. Shared
            concepts are linked explicitly; no question or marking rule is generated.
          </p>
        </div>
      </header>

      <div className="v2-practice-toolbar">
        <label className="v2-field">
          <span>Examination</span>
          <select
            aria-label="Practice examination"
            onChange={(event) => {
              const next = Number(event.target.value);
              setExamId(next);
              setSelectedQuestionId(null);
              setSelectedQuestion(null);
              setShowQuestionForm(false);
              setEditingQuestion(null);
              setSelectedPatternId(null);
              setSearch('');
              setYearFilter('');
              setSectionFilter('');
              setTopicFilter('');
              setTypeFilter('');
              setVerificationFilter('');
              setTab('pyq');
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
          <span>Pattern version</span>
          <select
            aria-label="Practice pattern version"
            disabled={!options || verifiedPatterns.length === 0}
            onChange={(event) => setSelectedPatternId(Number(event.target.value))}
            value={selectedPattern?.id ?? ''}
          >
            {!selectedPattern && <option value="">No verified pattern</option>}
            {options?.patterns.map((pattern) => (
              <option
                disabled={pattern.verificationStatus !== 'verified'}
                key={pattern.id}
                value={pattern.id}
              >
                {pattern.versionLabel} —{' '}
                {pattern.verificationStatus === 'verified'
                  ? `${pattern.totalMarks} marks`
                  : 'pending review'}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-practice-check">
          <input
            checked={showArchived}
            onChange={(event) => setShowArchived(event.target.checked)}
            type="checkbox"
          />
          <span>Show archived</span>
        </label>
        <div className="v2-practice-tabs" role="tablist" aria-label="Practice views">
          {practiceTabs.map(([key, label]) => (
            <button
              aria-selected={tab === key}
              className={`nav-button${tab === key ? ' nav-item-active' : ''}`}
              key={key}
              onClick={() => setTab(key)}
              role="tab"
              type="button"
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {loading && <p role="status">Loading practice data…</p>}
      {error && (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="v2-study-notice" role="status">
          {notice}
        </p>
      )}
      {!loading && options && patternBanner}

      {!loading && options && tab === 'pyq' && (
        <section className="v2-practice-panel" aria-label="PYQ question bank">
          <div className="v2-section-heading">
            <div>
              <h2>PYQ bank</h2>
              <p>
                {filteredQuestions.length} of {questions.length} questions shown. Answer keys,
                source verification and explanations are stored separately.
              </p>
            </div>
            <div className="v2-form-actions">
              <button
                className="secondary-button"
                disabled={busy || !selectedPattern}
                onClick={() => {
                  setEditingQuestion(null);
                  setShowQuestionForm((value) => !value);
                }}
                type="button"
              >
                {showQuestionForm ? 'Close entry form' : 'Add question'}
              </button>
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => setShowImport((value) => !value)}
                type="button"
              >
                {showImport ? 'Close CSV import' : 'Import CSV'}
              </button>
            </div>
          </div>

          <div className="v2-practice-filters">
            <label className="v2-field">
              <span>Search</span>
              <input
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Statement, source, explanation or note"
                value={search}
              />
            </label>
            <label className="v2-field">
              <span>Year</span>
              <select onChange={(event) => setYearFilter(event.target.value)} value={yearFilter}>
                <option value="">All years</option>
                {[...new Set(questions.map((question) => question.year))]
                  .sort((left, right) => right - left)
                  .map((year) => (
                    <option key={year} value={year}>
                      {year}
                    </option>
                  ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Section</span>
              <select
                onChange={(event) => setSectionFilter(event.target.value)}
                value={sectionFilter}
              >
                <option value="">All sections</option>
                {options.sections.map((section) => (
                  <option key={section.id} value={section.id}>
                    {section.title}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Syllabus topic</span>
              <select onChange={(event) => setTopicFilter(event.target.value)} value={topicFilter}>
                <option value="">All topics</option>
                {options.topics
                  .filter((topic) => topic.nodeKind === 'topic')
                  .map((topic) => (
                    <option key={topic.id} value={topic.id}>
                      {topic.title}
                    </option>
                  ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Question type</span>
              <select onChange={(event) => setTypeFilter(event.target.value)} value={typeFilter}>
                <option value="">All types</option>
                {options.questionTypes.map((type) => (
                  <option key={type.id} value={type.id}>
                    {type.code}
                  </option>
                ))}
              </select>
            </label>
            <label className="v2-field">
              <span>Verification</span>
              <select
                onChange={(event) => setVerificationFilter(event.target.value)}
                value={verificationFilter}
              >
                <option value="">All verification states</option>
                <option value="verified">Verified</option>
                <option value="pending">Pending</option>
                <option value="draft">Draft</option>
                <option value="rejected">Rejected</option>
              </select>
            </label>
          </div>

          {showQuestionForm && selectedPattern && (
            <QuestionForm
              examId={examId}
              initial={editingQuestion}
              onCancel={() => {
                setShowQuestionForm(false);
                setEditingQuestion(null);
              }}
              onSaved={(detail) => {
                setShowQuestionForm(false);
                setEditingQuestion(null);
                setSelectedQuestion(detail);
                setSelectedQuestionId(detail.question.id);
                setNotice('Question saved. Historical attempts were not rewritten.');
                void refresh();
              }}
              options={options}
              pattern={selectedPattern}
              today={today}
            />
          )}

          {showImport && (
            <div className="v2-practice-card">
              <h3>CSV import</h3>
              <p className="v2-empty-copy">
                Import only verified key/source data you have the right to store locally. Preview
                validates the full header contract and duplicate identity before writing.
              </p>
              <p className="v2-empty-copy">
                The original 27-column file remains valid. The v2 header adds
                <code>options_json</code> (a JSON array of choice key/text objects),
                <code>pattern_version_id</code>, <code>source_id</code> and
                <code>answer_key_id</code>.
              </p>
              <label className="v2-field">
                <span>CSV file</span>
                <input
                  accept=".csv,text/csv"
                  onChange={(event) => {
                    const file = event.target.files?.[0];
                    if (!file) return;
                    void file.text().then((text) => {
                      setCsv(text);
                      setImportPreview(null);
                    });
                  }}
                  type="file"
                />
              </label>
              <label className="v2-field">
                <span>CSV content</span>
                <textarea
                  onChange={(event) => {
                    setCsv(event.target.value);
                    setImportPreview(null);
                  }}
                  rows={5}
                  value={csv}
                />
              </label>
              <div className="v2-form-actions">
                <button
                  className="secondary-button"
                  onClick={() => {
                    setCsv(QUESTION_CSV_TEMPLATE);
                    setImportPreview(null);
                  }}
                  type="button"
                >
                  Load header template
                </button>
                <button
                  className="secondary-button"
                  onClick={() => {
                    setCsv(QUESTION_CSV_TEMPLATE_V2);
                    setImportPreview(null);
                  }}
                  type="button"
                >
                  Load v2 header (choices + provenance)
                </button>
                <button
                  className="secondary-button"
                  disabled={busy || !csv.trim()}
                  onClick={() =>
                    void runAction(async () => {
                      const preview = await previewV2PyqImport(csv);
                      setImportPreview(preview);
                      return `${preview.preview.validRows} valid, ${preview.preview.duplicateRows} duplicate, ${preview.preview.invalidRows} invalid.`;
                    })
                  }
                  type="button"
                >
                  Preview import
                </button>
                <button
                  className="primary-button"
                  disabled={busy || !importPreview || importPreview.preview.validRows === 0}
                  onClick={() =>
                    void runAction(async () => {
                      const result = await applyV2PyqImport(csv);
                      setImportPreview(null);
                      setCsv('');
                      return `${result.result.imported} imported, ${result.result.skipped} skipped, ${result.result.rejected} rejected.`;
                    })
                  }
                  type="button"
                >
                  Apply valid rows
                </button>
              </div>
              {importPreview && (
                <div className="v2-practice-import-preview">
                  <p>
                    {importPreview.preview.totalRows} rows · {importPreview.preview.validRows} valid
                    · {importPreview.preview.duplicateRows} duplicate ·{' '}
                    {importPreview.preview.invalidRows} invalid
                  </p>
                  <ul>
                    {importPreview.preview.rows.slice(0, 20).map((row) => (
                      <li key={row.rowNumber}>
                        <strong>
                          Row {row.rowNumber}: {row.status}
                        </strong>
                        {row.errors.length > 0 &&
                          ` — ${row.errors.map((error) => error.message).join('; ')}`}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          <div className="v2-practice-split">
            <div className="v2-practice-list">
              {filteredQuestions.length === 0 ? (
                <div className="v2-empty-panel">
                  <h2>No PYQ questions match</h2>
                  <p>
                    {questions.length === 0
                      ? 'No question content is stored for this examination. Add a question or import a verified local CSV.'
                      : 'Adjust the filters or clear the search.'}
                  </p>
                </div>
              ) : (
                filteredQuestions.map((question) => (
                  <button
                    aria-current={selectedQuestionId === question.id ? 'true' : undefined}
                    className={`v2-practice-question${selectedQuestionId === question.id ? ' v2-practice-question-active' : ''}`}
                    key={question.id}
                    onClick={() => setSelectedQuestionId(question.id)}
                    type="button"
                  >
                    <div className="v2-practice-question-topline">
                      <span>
                        {question.year} · {question.sessionLabel || 'Session not stated'} · Q
                        {question.questionNumber}
                        {question.isArchived ? ' · archived' : ''}
                      </span>
                      <span
                        className={`v2-practice-state v2-practice-state-${question.verificationStatus}`}
                      >
                        {question.verificationStatus}
                      </span>
                    </div>
                    <strong>
                      {question.statement || 'Statement held in local source reference'}
                    </strong>
                    <p>
                      {question.sectionCode} · {question.questionTypeCode} · +{question.maxMarks} /{' '}
                      {question.negativeMarks} · {question.attemptCount} attempts
                    </p>
                    <p className="v2-empty-copy">
                      {question.sourceTitle || 'Source not stated'}
                      {question.sourcePage ? ` · page ${question.sourcePage}` : ''} ·{' '}
                      {question.conceptCount} concept link(s)
                    </p>
                  </button>
                ))
              )}
            </div>
            <div className="v2-practice-detail">
              {!selectedQuestion ? (
                <div className="v2-empty-panel">
                  <h2>Select a question</h2>
                  <p>Open a question to inspect provenance, attempt history and scoring.</p>
                </div>
              ) : (
                <QuestionDetailPanel
                  busy={busy}
                  detail={selectedQuestion}
                  onArchive={() =>
                    void runAction(async () => {
                      await archiveV2PyqQuestion(
                        examId,
                        selectedQuestion.question.id,
                        !selectedQuestion.question.isArchived,
                      );
                      return selectedQuestion.question.isArchived
                        ? 'Question restored.'
                        : 'Question archived; attempt history preserved.';
                    })
                  }
                  onEdit={() => {
                    const patternId = selectedQuestion.question.patternVersionId;
                    if (
                      patternId &&
                      options.patterns.some(
                        (candidate) =>
                          candidate.id === patternId && candidate.verificationStatus === 'verified',
                      )
                    ) {
                      setSelectedPatternId(patternId);
                    }
                    setEditingQuestion(selectedQuestion);
                    setShowQuestionForm(true);
                  }}
                  onRefresh={refresh}
                  today={today}
                />
              )}
            </div>
          </div>
        </section>
      )}

      {!loading && options && tab === 'mock' && (
        <div className="v2-practice-stack">
          <div className="v2-practice-tabs" role="tablist" aria-label="Mock entry mode">
            <button
              aria-selected={mockView === 'question'}
              className={`nav-button${mockView === 'question' ? ' nav-item-active' : ''}`}
              onClick={() => setMockView('question')}
              role="tab"
              type="button"
            >
              Question-level exam
            </button>
            <button
              aria-selected={mockView === 'aggregate'}
              className={`nav-button${mockView === 'aggregate' ? ' nav-item-active' : ''}`}
              onClick={() => setMockView('aggregate')}
              role="tab"
              type="button"
            >
              Aggregate result entry
            </button>
          </div>
          {mockView === 'question' ? (
            <QuestionMockPanel
              examId={examId}
              key={selectedPattern?.id ?? 'no-pattern'}
              mocks={mocks}
              pattern={selectedPattern}
              refresh={refresh}
              today={today}
            />
          ) : (
            <MockPanel
              busy={busy}
              examId={examId}
              key={selectedPattern?.id ?? 'no-pattern'}
              mocks={mocks.filter((mock) => mock.mockKind === 'aggregate')}
              onAction={(action) =>
                void runAction(async () => {
                  await action();
                  return 'Mock state saved.';
                })
              }
              pattern={selectedPattern}
              patterns={options.patterns}
              topics={options.topics}
              refresh={refresh}
              selectedPatternId={selectedPatternId}
              setSelectedPatternId={setSelectedPatternId}
              today={today}
            />
          )}
        </div>
      )}

      {!loading && options && tab === 'analytics' && (
        <PracticeAnalyticsPanel
          analytics={analytics}
          options={options}
          sectionFilter={sectionFilter}
          setSectionFilter={setSectionFilter}
          setTopicFilter={setTopicFilter}
          setYearFilter={setYearFilter}
          topicFilter={topicFilter}
          yearFilter={yearFilter}
          years={[...new Set(questions.map((question) => question.year))].sort(
            (left, right) => right - left,
          )}
        />
      )}

      {!loading && options && tab === 'sources' && options.exam.code === 'gate-physics' && (
        <SourcesPanel examId={examId} options={options} refresh={refresh} today={today} />
      )}
    </div>
  );
}

type QuestionFormProps = {
  examId: number;
  options: V2PracticeOptions;
  pattern: V2PracticePattern;
  initial: V2PyqQuestionDetail | null;
  today: string;
  onSaved: (detail: V2PyqQuestionDetail) => void;
  onCancel: () => void;
};

function markingDefaults(
  section: V2PracticePatternSection,
  typeCode: string,
): { correct: number; incorrect: number } {
  const variant = section.markingVariants.find(
    (candidate) => String(candidate.questionType ?? '').toUpperCase() === typeCode.toUpperCase(),
  );
  return {
    correct: Number(variant?.correct ?? section.correctMarks),
    incorrect: Number(variant?.incorrect ?? section.incorrectMarks),
  };
}

function QuestionForm({
  examId,
  options,
  pattern,
  initial,
  today,
  onSaved,
  onCancel,
}: QuestionFormProps) {
  const patternSections = pattern.sections;
  const initialSection =
    patternSections.find((section) => section.sectionId === initial?.question.sectionId) ??
    patternSections[0];
  const initialTypes = options.questionTypes.filter((type) =>
    initialSection?.questionTypes.some((code) => code.toUpperCase() === type.code.toUpperCase()),
  );
  const initialType =
    initialTypes.find((type) => type.id === initial?.question.questionTypeId) ?? initialTypes[0];
  const initialMarking =
    initialSection && initialType
      ? markingDefaults(initialSection, initialType.code)
      : { correct: 1, incorrect: 0 };
  const [year, setYear] = useState(initial?.question.year ?? (Number(today.slice(0, 4)) || 2026));
  const [sessionLabel, setSessionLabel] = useState(initial?.question.sessionLabel ?? '');
  const [sectionId, setSectionId] = useState(initialSection?.sectionId ?? 0);
  const [questionTypeId, setQuestionTypeId] = useState(initialType?.id ?? 0);
  const [questionNumber, setQuestionNumber] = useState(initial?.question.questionNumber ?? '');
  const [statement, setStatement] = useState(initial?.question.statement ?? '');
  const [correctAnswer, setCorrectAnswer] = useState(initial?.correctAnswer ?? '');
  const [correctRange, setCorrectRange] = useState(initial?.question.correctRange ?? '');
  const [optionsText, setOptionsText] = useState(
    initial?.question.options.map((option) => `${option.key}) ${option.text}`).join('\n') ?? '',
  );
  const [maxMarks, setMaxMarks] = useState(
    String(initial?.question.maxMarks ?? initialMarking.correct),
  );
  const [negativeMarks, setNegativeMarks] = useState(
    String(initial?.question.negativeMarks ?? initialMarking.incorrect),
  );
  const [difficulty, setDifficulty] = useState(String(initial?.question.difficulty ?? 3));
  const [sourceTitle, setSourceTitle] = useState(initial?.question.sourceTitle ?? '');
  const [sourcePage, setSourcePage] = useState(initial?.question.sourcePage ?? '');
  const [sourceUrl, setSourceUrl] = useState(initial?.question.sourceUrl ?? '');
  const [answerKeySource, setAnswerKeySource] = useState(initial?.question.answerKeySource ?? '');
  const [answerKeyVersion, setAnswerKeyVersion] = useState(
    initial?.question.answerKeyVersion ?? '',
  );
  const [answerKeyVerified, setAnswerKeyVerified] = useState(
    initial?.question.answerKeyVerified ?? false,
  );
  const [verificationStatus, setVerificationStatus] = useState(
    initial?.question.verificationStatus ?? 'pending',
  );
  const [numericTolerance, setNumericTolerance] = useState(
    String(initial?.question.numericTolerance ?? 0),
  );
  const [topicIds, setTopicIds] = useState<number[]>(
    initial?.topics.map((topic) => topic.nodeId) ??
      (initial?.question.syllabusNodeId ? [initial.question.syllabusNodeId] : []),
  );
  const [conceptIds, setConceptIds] = useState<number[]>(
    initial?.concepts.map((concept) => concept.conceptId) ?? [],
  );
  const [resourceId, setResourceId] = useState(
    initial?.question.resourceId ? String(initial.question.resourceId) : '',
  );
  const [officialExplanation, setOfficialExplanation] = useState(
    initial?.question.officialExplanation ?? '',
  );
  const [thirdPartySolution, setThirdPartySolution] = useState(
    initial?.question.thirdPartySolution ?? '',
  );
  const [userNote, setUserNote] = useState(initial?.question.userNote ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const currentSection =
    patternSections.find((section) => section.sectionId === sectionId) ?? patternSections[0];
  const currentTypes = options.questionTypes.filter((type) =>
    currentSection?.questionTypes.some((code) => code.toUpperCase() === type.code.toUpperCase()),
  );
  const typeOptions = currentTypes.length > 0 ? currentTypes : options.questionTypes;

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const input: V2PyqQuestionInput = {
        id: initial?.question.id ?? null,
        examId,
        year: Number(year),
        sessionLabel,
        sectionId,
        questionTypeId,
        questionNumber,
        statement,
        correctAnswer,
        correctRange,
        maxMarks: Number(maxMarks),
        negativeMarks: Number(negativeMarks),
        unansweredMarks: 0,
        difficulty: Number(difficulty),
        sourceTitle,
        sourcePage,
        sourceUrl: sourceUrl.trim() || null,
        answerKeySource,
        answerKeyVersion,
        answerKeyVerified,
        verificationStatus,
        numericTolerance: Number(numericTolerance),
        options: parseQuestionOptions(optionsText),
        patternVersionId: pattern.id,
        syllabusNodeId: topicIds[0] ?? null,
        topicIds,
        conceptIds,
        resourceId: resourceId ? Number(resourceId) : null,
        dedupeKey: '',
        officialExplanation,
        thirdPartySolution,
        userNote,
      };
      onSaved(await saveV2PyqQuestion(input));
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="v2-practice-card" onSubmit={submit}>
      <div className="v2-section-heading">
        <div>
          <h3>{initial ? 'Edit question' : 'Add question'}</h3>
          <p>
            Pattern: {pattern.patternTitle} · {pattern.versionLabel}. Marks default to the verified
            marking variant for the selected section and question type; the backend rejects marks
            that do not match a verified variant.
          </p>
        </div>
      </div>
      {error && (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      )}
      <div className="v2-practice-form-grid">
        <label className="v2-field">
          <span>Year</span>
          <input
            min={1900}
            max={2100}
            onChange={(event) => setYear(Number(event.target.value))}
            required
            type="number"
            value={year}
          />
        </label>
        <label className="v2-field">
          <span>Session / paper</span>
          <input onChange={(event) => setSessionLabel(event.target.value)} value={sessionLabel} />
        </label>
        <label className="v2-field">
          <span>Section</span>
          <select
            onChange={(event) => {
              const nextSectionId = Number(event.target.value);
              const nextSection = patternSections.find(
                (section) => section.sectionId === nextSectionId,
              );
              setSectionId(nextSectionId);
              if (!nextSection) return;
              const nextTypes = options.questionTypes.filter((type) =>
                nextSection.questionTypes.some(
                  (code) => code.toUpperCase() === type.code.toUpperCase(),
                ),
              );
              const nextType = nextTypes[0];
              setQuestionTypeId(nextType?.id ?? 0);
              if (nextType) {
                const defaults = markingDefaults(nextSection, nextType.code);
                setMaxMarks(String(defaults.correct));
                setNegativeMarks(String(defaults.incorrect));
              }
            }}
            required
            value={sectionId}
          >
            {patternSections.map((section) => (
              <option key={section.id} value={section.sectionId}>
                {section.title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Question type</span>
          <select
            onChange={(event) => {
              const nextTypeId = Number(event.target.value);
              setQuestionTypeId(nextTypeId);
              const nextType = options.questionTypes.find((type) => type.id === nextTypeId);
              if (currentSection && nextType) {
                const defaults = markingDefaults(currentSection, nextType.code);
                setMaxMarks(String(defaults.correct));
                setNegativeMarks(String(defaults.incorrect));
              }
            }}
            required
            value={questionTypeId}
          >
            {typeOptions.map((type) => (
              <option key={type.id} value={type.id}>
                {type.code} — {type.title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Question number</span>
          <input
            onChange={(event) => setQuestionNumber(event.target.value)}
            required
            value={questionNumber}
          />
        </label>
        <label className="v2-field">
          <span>Difficulty (1-5)</span>
          <input
            max={5}
            min={1}
            onChange={(event) => setDifficulty(event.target.value)}
            type="number"
            value={difficulty}
          />
        </label>
      </div>
      <label className="v2-field">
        <span>Question statement</span>
        <textarea
          onChange={(event) => setStatement(event.target.value)}
          rows={4}
          value={statement}
        />
      </label>
      <label className="v2-field">
        <span>Answer choices (one per line; optional A) prefix)</span>
        <textarea
          onChange={(event) => setOptionsText(event.target.value)}
          placeholder={'A) First choice\nB) Second choice\nC) Third choice'}
          rows={4}
          value={optionsText}
        />
      </label>
      <p className="v2-empty-copy">
        MCQ and MSQ questions need at least two recorded choices before they can appear in a
        question-level mock. Numerical questions leave choices blank.
      </p>
      <div className="v2-practice-form-grid">
        <label className="v2-field">
          <span>Correct answer</span>
          <input
            onChange={(event) => setCorrectAnswer(event.target.value)}
            placeholder="A or A,C or numeric value"
            value={correctAnswer}
          />
        </label>
        <label className="v2-field">
          <span>Correct range (numeric)</span>
          <input
            onChange={(event) => setCorrectRange(event.target.value)}
            placeholder="min,max"
            value={correctRange}
          />
        </label>
        <label className="v2-field">
          <span>Maximum marks</span>
          <input
            min={0.01}
            onChange={(event) => setMaxMarks(event.target.value)}
            required
            step="0.01"
            type="number"
            value={maxMarks}
          />
        </label>
        <label className="v2-field">
          <span>Negative marks</span>
          <input
            max={0}
            onChange={(event) => setNegativeMarks(event.target.value)}
            step="0.001"
            type="number"
            value={negativeMarks}
          />
        </label>
        <label className="v2-field">
          <span>Numeric tolerance</span>
          <input
            min={0}
            onChange={(event) => setNumericTolerance(event.target.value)}
            step="0.001"
            type="number"
            value={numericTolerance}
          />
        </label>
        <label className="v2-field">
          <span>Verification</span>
          <select
            onChange={(event) => setVerificationStatus(event.target.value)}
            value={verificationStatus}
          >
            <option value="draft">Draft</option>
            <option value="pending">Pending</option>
            <option value="verified">Verified</option>
            <option value="rejected">Rejected</option>
          </select>
        </label>
      </div>
      <div className="v2-practice-form-grid">
        <label className="v2-field">
          <span>Source title</span>
          <input onChange={(event) => setSourceTitle(event.target.value)} value={sourceTitle} />
        </label>
        <label className="v2-field">
          <span>Source page</span>
          <input onChange={(event) => setSourcePage(event.target.value)} value={sourcePage} />
        </label>
        <label className="v2-field">
          <span>Source URL</span>
          <input
            onChange={(event) => setSourceUrl(event.target.value)}
            placeholder="https://…"
            value={sourceUrl}
          />
        </label>
        <label className="v2-field">
          <span>Local managed resource</span>
          <select onChange={(event) => setResourceId(event.target.value)} value={resourceId}>
            <option value="">No local resource</option>
            {options.resources.map((resource) => (
              <option key={resource.id} value={resource.id}>
                {resource.title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Answer-key source</span>
          <input
            onChange={(event) => setAnswerKeySource(event.target.value)}
            value={answerKeySource}
          />
        </label>
        <label className="v2-field">
          <span>Answer-key version</span>
          <input
            onChange={(event) => setAnswerKeyVersion(event.target.value)}
            value={answerKeyVersion}
          />
        </label>
      </div>
      <label className="v2-practice-check">
        <input
          checked={answerKeyVerified}
          onChange={(event) => setAnswerKeyVerified(event.target.checked)}
          type="checkbox"
        />
        <span>Answer key verified for this question</span>
      </label>
      <label className="v2-field">
        <span>Syllabus topics (first selected is primary)</span>
        <select
          multiple
          onChange={(event) =>
            setTopicIds(Array.from(event.target.selectedOptions, (option) => Number(option.value)))
          }
          value={topicIds.map(String)}
        >
          {options.topics.map((topic) => (
            <option key={topic.id} value={topic.id}>
              {topic.title} · {topic.nodeKind}
            </option>
          ))}
        </select>
      </label>
      <label className="v2-field">
        <span>Shared concepts</span>
        <select
          multiple
          onChange={(event) =>
            setConceptIds(
              Array.from(event.target.selectedOptions, (option) => Number(option.value)),
            )
          }
          value={conceptIds.map(String)}
        >
          {options.concepts.map((concept) => (
            <option key={concept.id} value={concept.id}>
              {concept.title} · {concept.domain}
            </option>
          ))}
        </select>
      </label>
      <div className="v2-practice-form-grid">
        <label className="v2-field">
          <span>Official explanation</span>
          <textarea
            onChange={(event) => setOfficialExplanation(event.target.value)}
            rows={3}
            value={officialExplanation}
          />
        </label>
        <label className="v2-field">
          <span>Third-party solution</span>
          <textarea
            onChange={(event) => setThirdPartySolution(event.target.value)}
            rows={3}
            value={thirdPartySolution}
          />
        </label>
        <label className="v2-field">
          <span>User note</span>
          <textarea
            onChange={(event) => setUserNote(event.target.value)}
            rows={3}
            value={userNote}
          />
        </label>
      </div>
      <div className="v2-form-actions">
        <button className="primary-button" disabled={busy} type="submit">
          {busy ? 'Saving…' : 'Save question'}
        </button>
        <button className="secondary-button" onClick={onCancel} type="button">
          Cancel
        </button>
      </div>
    </form>
  );
}

type QuestionDetailPanelProps = {
  detail: V2PyqQuestionDetail;
  options?: V2PracticeOptions;
  today: string;
  busy: boolean;
  onEdit: () => void;
  onArchive: () => void;
  onRefresh: () => Promise<void>;
};

function QuestionDetailPanel({
  detail,
  today,
  busy,
  onEdit,
  onArchive,
  onRefresh,
}: QuestionDetailPanelProps) {
  const [selectedAnswer, setSelectedAnswer] = useState('');
  const [attemptDate, setAttemptDate] = useState(today);
  const [seconds, setSeconds] = useState('0');
  const [confidence, setConfidence] = useState('');
  const [notes, setNotes] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);

  async function submitAttempt(event: FormEvent) {
    event.preventDefault();
    setWorking(true);
    setError(null);
    try {
      await recordV2PyqAttempt({
        examId: detail.question.examId,
        questionId: detail.question.id,
        attemptDate,
        selectedAnswer,
        timeSpentSeconds: Number(seconds),
        confidence: confidence ? Number(confidence) : null,
        mistakes: '',
        reviewStatus: 'unreviewed',
        notes,
      });
      setSelectedAnswer('');
      setNotes('');
      await onRefresh();
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setWorking(false);
    }
  }

  return (
    <div className="v2-practice-card">
      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">
            {detail.question.sectionTitle} · {detail.question.year}
          </p>
          <h2>Question {detail.question.questionNumber}</h2>
        </div>
        <span
          className={`v2-practice-state v2-practice-state-${detail.question.verificationStatus}`}
        >
          {detail.question.verificationStatus}
        </span>
      </div>
      <p className="v2-practice-statement">
        {detail.question.statement || 'Statement held in local source reference.'}
      </p>
      <dl className="v2-fact-list">
        <div>
          <dt>Pattern</dt>
          <dd>{detail.question.patternVersionLabel ?? 'Not linked'}</dd>
        </div>
        <div>
          <dt>Question type</dt>
          <dd>{detail.question.questionTypeCode}</dd>
        </div>
        <div>
          <dt>Marks</dt>
          <dd>
            +{detail.question.maxMarks} / {detail.question.negativeMarks} / unanswered{' '}
            {detail.question.unansweredMarks}
          </dd>
        </div>
        <div>
          <dt>Correct answer</dt>
          <dd>
            {detail.correctAnswer || 'Not recorded'}
            {detail.question.correctRange ? ` · range ${detail.question.correctRange}` : ''}
            {detail.question.numericTolerance > 0
              ? ` · tolerance ${detail.question.numericTolerance}`
              : ''}
          </dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>
            {detail.question.sourceTitle || 'Not stated'}
            {detail.question.sourcePage ? ` · page ${detail.question.sourcePage}` : ''}
            {detail.question.sourceUrl && (
              <>
                {' '}
                ·{' '}
                <a href={detail.question.sourceUrl} rel="noreferrer" target="_blank">
                  Open source
                </a>
              </>
            )}
          </dd>
        </div>
        <div>
          <dt>Answer key</dt>
          <dd>
            {detail.question.answerKeySource || 'Not stated'} ·{' '}
            {detail.question.answerKeyVersion || 'unversioned'} ·{' '}
            {detail.question.answerKeyVerified ? 'verified' : 'not verified'}
          </dd>
        </div>
        <div>
          <dt>Difficulty</dt>
          <dd>{detail.question.difficulty} / 5</dd>
        </div>
      </dl>
      {detail.topics.length > 0 && (
        <p>
          <strong>Topics:</strong> {detail.topics.map((topic) => topic.nodeTitle).join(' · ')}
        </p>
      )}
      {detail.concepts.length > 0 && (
        <p>
          <strong>Shared concepts:</strong>{' '}
          {detail.concepts.map((concept) => concept.title).join(' · ')}
        </p>
      )}
      {detail.question.officialExplanation && (
        <div>
          <h3>Official explanation</h3>
          <p>{detail.question.officialExplanation}</p>
        </div>
      )}
      {detail.question.thirdPartySolution && (
        <div>
          <h3>Third-party solution</h3>
          <p>{detail.question.thirdPartySolution}</p>
        </div>
      )}
      {detail.question.userNote && (
        <div>
          <h3>User note</h3>
          <p>{detail.question.userNote}</p>
        </div>
      )}
      <div className="v2-form-actions">
        <button className="secondary-button" disabled={busy} onClick={onEdit} type="button">
          Edit question
        </button>
        <button className="secondary-button" disabled={busy} onClick={onArchive} type="button">
          {detail.question.isArchived ? 'Restore question' : 'Archive question'}
        </button>
      </div>
      <form className="v2-practice-attempt" onSubmit={submitAttempt}>
        <h3>Record attempt</h3>
        {error && (
          <p className="v2-inline-error" role="alert">
            {error}
          </p>
        )}
        <div className="v2-practice-form-grid">
          <label className="v2-field">
            <span>Answer</span>
            <input
              onChange={(event) => setSelectedAnswer(event.target.value)}
              placeholder="Leave blank to skip"
              value={selectedAnswer}
            />
          </label>
          <label className="v2-field">
            <span>Attempt date</span>
            <input
              onChange={(event) => setAttemptDate(event.target.value)}
              type="date"
              value={attemptDate}
            />
          </label>
          <label className="v2-field">
            <span>Time spent (seconds)</span>
            <input
              min={0}
              onChange={(event) => setSeconds(event.target.value)}
              type="number"
              value={seconds}
            />
          </label>
          <label className="v2-field">
            <span>Confidence</span>
            <select onChange={(event) => setConfidence(event.target.value)} value={confidence}>
              <option value="">Not set</option>
              {[1, 2, 3, 4, 5].map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label className="v2-field">
          <span>Attempt notes</span>
          <textarea onChange={(event) => setNotes(event.target.value)} rows={2} value={notes} />
        </label>
        <div className="v2-form-actions">
          <button className="primary-button" disabled={working || busy} type="submit">
            {working ? 'Saving…' : 'Save attempt'}
          </button>
        </div>
        <p className="v2-empty-copy">
          The backend determines correct/incorrect/skipped and marks. A verified answer key and
          verified question are required for scored evidence.
        </p>
      </form>
      <h3>Attempt history</h3>
      {detail.attempts.length === 0 ? (
        <p className="v2-empty-copy">No attempts recorded.</p>
      ) : (
        <ul className="v2-practice-history">
          {detail.attempts.map((attempt) => (
            <li key={attempt.id}>
              <strong>
                {attempt.attemptDate} · {attempt.status}
              </strong>
              <span>
                {attempt.marksObtained} marks · {minutesLabel(attempt.timeSpentSeconds)} · key{' '}
                {attempt.answerKeyVersion || 'unversioned'}
              </span>
              {attempt.selectedAnswer && <span>Answer: {attempt.selectedAnswer}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

type MockPanelProps = {
  examId: number;
  patterns: V2PracticePattern[];
  pattern: V2PracticePattern | null;
  topics: V2PracticeTopicOption[];
  selectedPatternId: number | null;
  setSelectedPatternId: (id: number) => void;
  mocks: V2MockTest[];
  today: string;
  busy: boolean;
  refresh: () => Promise<void>;
  onAction: (action: () => Promise<void>) => void;
};

function MockPanel({
  examId,
  patterns,
  pattern,
  topics,
  selectedPatternId,
  setSelectedPatternId,
  mocks,
  today,
  busy,
  refresh,
  onAction,
}: MockPanelProps) {
  const [name, setName] = useState('Full-length mock');
  const [date, setDate] = useState(today);
  const [duration, setDuration] = useState(
    pattern?.durationMinutes ? String(pattern.durationMinutes) : '',
  );
  const [mode, setMode] = useState('full_length');
  const [focusNodeId, setFocusNodeId] = useState('');
  const [notes, setNotes] = useState('');
  const [drafts, setDrafts] = useState<
    Record<
      number,
      {
        count: string;
        correct: string;
        incorrect: string;
        unanswered: string;
        max: string;
        marks: string;
      }
    >
  >(() =>
    Object.fromEntries(
      (pattern?.sections ?? []).map((section) => [
        section.id,
        {
          count: String(section.questionCount),
          correct: '0',
          incorrect: '0',
          unanswered: String(section.questionCount),
          max: '',
          marks: '',
        },
      ]),
    ),
  );
  const [error, setError] = useState<string | null>(null);
  const [working, setWorking] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!pattern) return;
    setWorking(true);
    setError(null);
    try {
      const sections: V2MockSectionInput[] = pattern.sections.map((section) => {
        const draft = drafts[section.id] ?? {
          count: '0',
          correct: '0',
          incorrect: '0',
          unanswered: '0',
          max: '',
          marks: '',
        };
        return {
          patternSectionId: section.id,
          questionCount: Number(draft.count),
          correct: Number(draft.correct),
          incorrect: Number(draft.incorrect),
          unanswered: Number(draft.unanswered),
          maxMarks: draft.max ? Number(draft.max) : null,
          marksObtained: draft.marks ? Number(draft.marks) : null,
          timeSpentMinutes: null,
          notes: '',
        };
      });
      await saveV2MockTest({
        examId,
        patternVersionId: pattern.id,
        testName: name,
        testDate: date,
        durationMinutes: Number(duration),
        testMode: mode,
        focusNodeId: focusNodeId ? Number(focusNodeId) : null,
        notes,
        reviewObservations: '',
        sections,
      });
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setWorking(false);
    }
  }

  return (
    <section className="v2-practice-panel" aria-label="Mock tests">
      <div className="v2-section-heading">
        <div>
          <h2>Mock tests</h2>
          <p>
            Aggregate section results are validated against the selected verified pattern. Mock
            duration is never added to study totals.
          </p>
        </div>
      </div>
      <div className="v2-practice-patterns">
        {patterns.map((candidate) => (
          <button
            className={`v2-practice-pattern-choice${candidate.id === selectedPatternId ? ' v2-practice-pattern-choice-active' : ''}`}
            disabled={candidate.verificationStatus !== 'verified'}
            key={candidate.id}
            onClick={() => setSelectedPatternId(candidate.id)}
            type="button"
          >
            <strong>{candidate.versionLabel}</strong>
            <span>
              {candidate.verificationStatus === 'verified'
                ? `${candidate.totalMarks} marks · ${candidate.durationMinutes ?? 'duration not stated'} min`
                : 'Pending — not selectable'}
            </span>
          </button>
        ))}
      </div>
      {pattern ? (
        <form className="v2-practice-card" onSubmit={submit}>
          <h3>New mock from {pattern.versionLabel}</h3>
          {error && (
            <p className="v2-inline-error" role="alert">
              {error}
            </p>
          )}
          <div className="v2-practice-form-grid">
            <label className="v2-field">
              <span>Name</span>
              <input onChange={(event) => setName(event.target.value)} required value={name} />
            </label>
            <label className="v2-field">
              <span>Date</span>
              <input
                onChange={(event) => setDate(event.target.value)}
                required
                type="date"
                value={date}
              />
            </label>
            <label className="v2-field">
              <span>Duration used (minutes)</span>
              <input
                min={1}
                onChange={(event) => setDuration(event.target.value)}
                placeholder={pattern.durationMinutes ? undefined : 'Actual time used'}
                required
                type="number"
                value={duration}
              />
            </label>
            <label className="v2-field">
              <span>Mode</span>
              <select onChange={(event) => setMode(event.target.value)} value={mode}>
                <option value="full_length">Full length</option>
                <option value="untimed">Untimed</option>
                <option value="section">Section-specific</option>
                <option value="topic">Topic-based</option>
                <option value="custom">Custom</option>
                <option value="past_paper">Past paper</option>
              </select>
            </label>
          </div>
          {mode === 'topic' && (
            <label className="v2-field">
              <span>Focus syllabus topic</span>
              <select
                onChange={(event) => setFocusNodeId(event.target.value)}
                required
                value={focusNodeId}
              >
                <option value="">Select a syllabus topic</option>
                {topics.map((topic) => (
                  <option key={topic.id} value={topic.id}>
                    {topic.title} · {topic.nodeKind}
                  </option>
                ))}
              </select>
            </label>
          )}
          {pattern.durationMinutes === null && (
            <p className="v2-empty-copy">
              This verified pattern does not state an official duration. Enter the time you actually
              used; it is recorded as your duration, not as an official time limit.
            </p>
          )}
          <label className="v2-field">
            <span>Notes</span>
            <textarea onChange={(event) => setNotes(event.target.value)} rows={2} value={notes} />
          </label>
          <div className="v2-practice-mock-sections">
            {pattern.sections.map((section) => {
              const draft = drafts[section.id] ?? {
                count: String(section.questionCount),
                correct: '0',
                incorrect: '0',
                unanswered: String(section.questionCount),
                max: '',
                marks: '',
              };
              const update = (
                key: 'count' | 'correct' | 'incorrect' | 'unanswered' | 'max' | 'marks',
                value: string,
              ) =>
                setDrafts((current) => ({ ...current, [section.id]: { ...draft, [key]: value } }));
              return (
                <fieldset className="v2-practice-mock-section" key={section.id}>
                  <legend>{section.title}</legend>
                  <p className="v2-empty-copy">
                    {section.code} · max {section.questionCount} questions · {section.maxMarks}{' '}
                    marks
                    {section.maxAttempts
                      ? ` · first ${section.maxAttempts} answered evaluated`
                      : ''}
                  </p>
                  <div className="v2-practice-form-grid">
                    <label className="v2-field">
                      <span>Questions</span>
                      <input
                        min={0}
                        onChange={(event) => update('count', event.target.value)}
                        type="number"
                        value={draft.count}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Correct</span>
                      <input
                        min={0}
                        onChange={(event) => update('correct', event.target.value)}
                        type="number"
                        value={draft.correct}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Incorrect</span>
                      <input
                        min={0}
                        onChange={(event) => update('incorrect', event.target.value)}
                        type="number"
                        value={draft.incorrect}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Unanswered</span>
                      <input
                        min={0}
                        onChange={(event) => update('unanswered', event.target.value)}
                        type="number"
                        value={draft.unanswered}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Max marks (blank = pattern total)</span>
                      <input
                        min={0}
                        onChange={(event) => update('max', event.target.value)}
                        step="0.01"
                        type="number"
                        value={draft.max}
                      />
                    </label>
                    <label className="v2-field">
                      <span>Marks obtained (required for mixed sections)</span>
                      <input
                        onChange={(event) => update('marks', event.target.value)}
                        step="0.01"
                        type="number"
                        value={draft.marks}
                      />
                    </label>
                  </div>
                </fieldset>
              );
            })}
          </div>
          <p className="v2-empty-copy">
            For mixed-marking sections such as GATE GA and PH, enter maximum and obtained marks
            explicitly; aggregate counts alone cannot determine 1-mark and 2-mark question scores.
          </p>
          <div className="v2-form-actions">
            <button className="primary-button" disabled={working || busy} type="submit">
              {working ? 'Saving…' : 'Save planned mock'}
            </button>
          </div>
        </form>
      ) : (
        <div className="v2-inactive-banner">
          <h2>Verified pattern required</h2>
          <p>Select a verified pattern version before creating a mock.</p>
        </div>
      )}
      <h3>Mock history</h3>
      {mocks.length === 0 ? (
        <p className="v2-empty-copy">No mock tests recorded for this examination.</p>
      ) : (
        <ul className="v2-practice-mock-list">
          {mocks.map((mock) => (
            <li className="v2-practice-card" key={mock.id}>
              <div className="v2-section-heading">
                <div>
                  <strong>{mock.testName}</strong>
                  <p className="v2-empty-copy">
                    {mock.testDate} · {mock.patternVersionLabel ?? 'No pattern label'} ·{' '}
                    {mock.testMode} · {mock.status}
                    {mock.isArchived ? ' · archived' : ''}
                    {mock.focusNodeTitle ? ` · focus: ${mock.focusNodeTitle}` : ''}
                  </p>
                </div>
                <strong>
                  {mock.status === 'completed' ? percent(mock.percentage) : 'Not completed'}
                </strong>
              </div>
              <p>
                {mock.sections
                  .map(
                    (section) =>
                      `${section.sectionCode}: ${section.marksObtained}/${section.maxMarks}`,
                  )
                  .join(' · ')}
              </p>
              <p className="v2-empty-copy">
                {mock.availableQuestions} available · {mock.attemptedQuestions} attempted ·{' '}
                {mock.correctQuestions} correct · {mock.incorrectQuestions} incorrect ·{' '}
                {mock.unansweredQuestions} unanswered · {mock.scoreHistory.length} score version(s)
              </p>
              <div className="v2-form-actions">
                {mock.status === 'planned' && (
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() =>
                      onAction(async () => {
                        await setV2MockTestStatus(examId, mock.id, 'start');
                      })
                    }
                    type="button"
                  >
                    Start
                  </button>
                )}
                {(mock.status === 'planned' || mock.status === 'in_progress') && (
                  <button
                    className="primary-button"
                    disabled={busy}
                    onClick={() =>
                      onAction(async () => {
                        await setV2MockTestStatus(examId, mock.id, 'complete');
                      })
                    }
                    type="button"
                  >
                    Complete
                  </button>
                )}
                {mock.status === 'completed' && (
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() =>
                      onAction(async () => {
                        await setV2MockTestStatus(examId, mock.id, 'reopen');
                      })
                    }
                    type="button"
                  >
                    Reopen
                  </button>
                )}
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() =>
                    onAction(async () => {
                      await archiveV2MockTest(examId, mock.id, !mock.isArchived);
                    })
                  }
                  type="button"
                >
                  {mock.isArchived ? 'Restore' : 'Archive'}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function PracticeAnalyticsPanel({
  analytics,
  options,
  years,
  yearFilter,
  sectionFilter,
  topicFilter,
  setYearFilter,
  setSectionFilter,
  setTopicFilter,
}: {
  analytics: V2PracticeAnalytics | null;
  options: V2PracticeOptions;
  years: number[];
  yearFilter: string;
  sectionFilter: string;
  topicFilter: string;
  setYearFilter: (value: string) => void;
  setSectionFilter: (value: string) => void;
  setTopicFilter: (value: string) => void;
}) {
  if (!analytics) {
    return (
      <div className="v2-empty-panel">
        <h2>No practice analytics</h2>
        <p>Record a PYQ attempt or complete a mock to populate analytics.</p>
      </div>
    );
  }
  return (
    <section className="v2-practice-panel" aria-label="Practice analytics">
      <div className="v2-section-heading">
        <div>
          <h2>Practice analytics</h2>
          <p>
            Persisted data only. Raw marks are shown within this examination; comparisons across
            examinations use percentages.
          </p>
        </div>
      </div>
      <div className="v2-practice-filters">
        <label className="v2-field">
          <span>Year scope</span>
          <select onChange={(event) => setYearFilter(event.target.value)} value={yearFilter}>
            <option value="">All years</option>
            {years.map((year) => (
              <option key={year} value={year}>
                {year}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Section scope</span>
          <select onChange={(event) => setSectionFilter(event.target.value)} value={sectionFilter}>
            <option value="">All sections</option>
            {options.sections.map((section) => (
              <option key={section.id} value={section.id}>
                {section.title}
              </option>
            ))}
          </select>
        </label>
        <label className="v2-field">
          <span>Topic scope</span>
          <select onChange={(event) => setTopicFilter(event.target.value)} value={topicFilter}>
            <option value="">All topics</option>
            {options.topics
              .filter((topic) => topic.nodeKind === 'topic')
              .map((topic) => (
                <option key={topic.id} value={topic.id}>
                  {topic.title}
                </option>
              ))}
          </select>
        </label>
      </div>
      <p className="v2-empty-copy">
        Year, section and topic scope apply to PYQ analytics. The mock summary remains
        examination-wide because mock results are not tied to individual PYQ topics.
      </p>
      <div className="v2-practice-stats">
        <div>
          <strong>{analytics.pyqAttempts}</strong>
          <span>PYQ attempts</span>
        </div>
        <div>
          <strong>{analytics.distinctQuestions}</strong>
          <span>Distinct questions</span>
        </div>
        <div>
          <strong>{percent(analytics.accuracyPercentage)}</strong>
          <span>Accuracy</span>
        </div>
        <div>
          <strong>{percent(analytics.percentage)}</strong>
          <span>Marks percentage</span>
        </div>
        <div>
          <strong>{analytics.timeSpentSeconds}s</strong>
          <span>Time on questions</span>
        </div>
        <div>
          <strong>{percent(analytics.verificationCoveragePercentage)}</strong>
          <span>Answer/source verified coverage</span>
        </div>
        <div>
          <strong>{analytics.mocks.completedCount}</strong>
          <span>Completed mocks</span>
        </div>
        <div>
          <strong>{percent(analytics.mocks.averagePercentage)}</strong>
          <span>Average mock percentage</span>
        </div>
      </div>
      <div className="v2-practice-split">
        <div className="v2-practice-card">
          <h3>PYQ distribution</h3>
          <p>
            {analytics.correct} correct · {analytics.incorrect} incorrect · {analytics.unanswered}{' '}
            unanswered/skipped
          </p>
          <p className="v2-empty-copy">
            Maximum marks attempted: {analytics.maxMarksAttempted}. Average time per attempt:{' '}
            {analytics.averageSecondsPerQuestion === null
              ? '—'
              : `${analytics.averageSecondsPerQuestion}s`}
            .
          </p>
        </div>
        <div className="v2-practice-card">
          <h3>Source/answer verification</h3>
          <p>
            {analytics.verifiedQuestions} verified questions in the attempted set ·{' '}
            {analytics.verifiedQuestionsInScope} of {analytics.totalQuestionsInScope} questions in
            scope have verified answers.
          </p>
          {analytics.insufficientPyqData && (
            <p className="v2-empty-copy">
              Trend requires at least two attempts; no invented trend is shown.
            </p>
          )}
        </div>
      </div>
      <h3>Attempt trend</h3>
      {analytics.insufficientPyqData || analytics.pyqTrend.length < 2 ? (
        <p className="v2-empty-copy">
          No trend is shown until at least two persisted attempts exist for this examination.
        </p>
      ) : (
        <ul className="v2-practice-history">
          {analytics.pyqTrend.map((point) => (
            <li key={point.date}>
              <strong>{point.date}</strong>
              <span>
                {point.attempts} attempt(s) · accuracy {percent(point.accuracyPercentage)}
              </span>
            </li>
          ))}
        </ul>
      )}
      <h3>By year</h3>
      {analytics.byYear.length === 0 ? (
        <p className="v2-empty-copy">No attempts.</p>
      ) : (
        <ul className="v2-practice-history">
          {analytics.byYear.map((row) => (
            <li key={row.key}>
              <strong>{row.label}</strong>
              <span>
                {row.attempts} attempts · accuracy {percent(row.accuracyPercentage)} · marks{' '}
                {percent(row.percentage)}
              </span>
            </li>
          ))}
        </ul>
      )}
      <h3>By section</h3>
      {analytics.bySection.length === 0 ? (
        <p className="v2-empty-copy">No attempts.</p>
      ) : (
        <ul className="v2-practice-history">
          {analytics.bySection.map((row) => (
            <li key={row.key}>
              <strong>{row.label}</strong>
              <span>
                {row.attempts} attempts · accuracy {percent(row.accuracyPercentage)}
              </span>
            </li>
          ))}
        </ul>
      )}
      <h3>By topic</h3>
      {analytics.byTopic.length === 0 ? (
        <p className="v2-empty-copy">No topic-linked attempts.</p>
      ) : (
        <ul className="v2-practice-history">
          {analytics.byTopic.map((row) => (
            <li key={`${row.nodeId ?? 'none'}-${row.label}`}>
              <strong>{row.label}</strong>
              <span>
                {row.attempts} attempts · accuracy {percent(row.accuracyPercentage)}
              </span>
            </li>
          ))}
        </ul>
      )}
      <h3>Mock history</h3>
      {analytics.mocks.history.length === 0 ? (
        <p className="v2-empty-copy">No completed mocks.</p>
      ) : (
        <ul className="v2-practice-history">
          {analytics.mocks.history.map((point) => (
            <li key={point.testId}>
              <strong>
                {point.testDate} · {point.testName}
              </strong>
              <span>
                {percent(point.percentage)} · {point.patternVersionLabel ?? 'Unspecified pattern'}
              </span>
            </li>
          ))}
        </ul>
      )}
      <p className="v2-empty-copy">
        Pattern:{' '}
        {options.patterns.find((pattern) => pattern.verificationStatus === 'verified')
          ?.versionLabel ?? 'Pending'}
        . Readiness is not calculated here; no formula is invented.
      </p>
    </section>
  );
}
