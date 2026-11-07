import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  getV2QuestionMockSession,
  listV2QuestionMockCandidates,
  saveV2QuestionMockState,
  startV2QuestionMock,
  submitV2QuestionMock,
  type V2MockTest,
  type V2PracticePattern,
  type V2QuestionMockCandidate,
  type V2QuestionMockQuestion,
  type V2QuestionMockSession,
} from './api';

type QuestionMockPanelProps = {
  examId: number;
  pattern: V2PracticePattern | null;
  mocks: V2MockTest[];
  today: string;
  refresh: () => Promise<void>;
};

function formatSeconds(value: number | null): string {
  if (value === null) return 'Untimed';
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  const seconds = value % 60;
  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

function percent(value: number | null | undefined): string {
  return value === null || value === undefined ? '—' : `${value.toFixed(2)}%`;
}

function optionsList(answer: string): string[] {
  return answer
    .split(',')
    .map((value) => value.trim().toUpperCase())
    .filter(Boolean)
    .sort();
}

export function QuestionMockPanel({
  examId,
  pattern,
  mocks,
  today,
  refresh,
}: QuestionMockPanelProps) {
  const [candidates, setCandidates] = useState<V2QuestionMockCandidate[]>([]);
  const [candidatesLoading, setCandidatesLoading] = useState(true);
  const [candidatesError, setCandidatesError] = useState<string | null>(null);
  const [mode, setMode] = useState<'full_length' | 'section' | 'custom'>('custom');
  const [sectionId, setSectionId] = useState(String(pattern?.sections[0]?.sectionId ?? ''));
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [testName, setTestName] = useState(`${pattern?.versionLabel ?? 'Question-level'} practice`);
  const [testDate, setTestDate] = useState(today);
  const [duration, setDuration] = useState(
    pattern?.durationMinutes ? String(pattern.durationMinutes) : '60',
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeSession, setActiveSession] = useState<V2QuestionMockSession | null>(null);
  const [loadingSession, setLoadingSession] = useState<number | null>(null);

  const patternId = pattern?.id ?? null;
  useEffect(() => {
    if (patternId === null) return;
    let cancelled = false;
    void (async () => {
      try {
        const list = await listV2QuestionMockCandidates(examId, patternId);
        if (!cancelled) setCandidates(list);
      } catch (caught) {
        if (!cancelled) setCandidatesError(toCommandError(caught).message);
      } finally {
        if (!cancelled) setCandidatesLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [examId, patternId]);

  const questionMocks = useMemo(
    () => mocks.filter((mock) => mock.mockKind === 'question_level'),
    [mocks],
  );
  const sectionCandidates = useMemo(
    () => candidates.filter((candidate) => String(candidate.question.sectionId) === sectionId),
    [candidates, sectionId],
  );
  const visibleCandidates = mode === 'section' ? sectionCandidates : candidates;
  const selectedCandidates = candidates.filter((candidate) =>
    selectedIds.includes(candidate.question.id),
  );
  const selectedCounts = useMemo(() => {
    const counts = new Map<number, number>();
    for (const candidate of selectedCandidates) {
      counts.set(candidate.patternSectionId, (counts.get(candidate.patternSectionId) ?? 0) + 1);
    }
    return counts;
  }, [selectedCandidates]);

  const fullLengthValid =
    pattern?.sections.every(
      (section) => (selectedCounts.get(section.id) ?? 0) === section.questionCount,
    ) ?? false;
  const selectionValid =
    selectedCandidates.length > 0 &&
    (mode !== 'full_length' || fullLengthValid) &&
    (mode !== 'section' ||
      selectedCandidates.every(
        (candidate) =>
          candidate.patternSectionId === Number(selectedCandidates[0]?.patternSectionId),
      ));
  const sectionLimitValid =
    mode !== 'section' ||
    selectedCandidates.length <=
      (pattern?.sections.find((section) => String(section.sectionId) === sectionId)
        ?.questionCount ?? 0);

  function toggleQuestion(id: number) {
    setSelectedIds((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
  }

  function selectAllVisible() {
    setSelectedIds(visibleCandidates.map((candidate) => candidate.question.id));
  }

  function changeSection(value: string) {
    setSectionId(value);
    setSelectedIds(
      candidates
        .filter((candidate) => String(candidate.question.sectionId) === value)
        .map((candidate) => candidate.question.id),
    );
  }

  async function startMock() {
    if (!pattern || !selectionValid || !sectionLimitValid) return;
    setBusy(true);
    setError(null);
    try {
      const session = await startV2QuestionMock({
        examId,
        patternVersionId: pattern.id,
        testName,
        testDate,
        testMode: mode,
        durationMinutes: duration ? Number(duration) : null,
        focusNodeId: null,
        notes: '',
        questionIds: selectedCandidates.map((candidate) => candidate.question.id),
      });
      setActiveSession(session);
      await refresh();
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(false);
    }
  }

  async function openMock(mockId: number) {
    setLoadingSession(mockId);
    setError(null);
    try {
      setActiveSession(await getV2QuestionMockSession(examId, mockId));
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setLoadingSession(null);
    }
  }

  if (activeSession) {
    if (activeSession.status === 'completed') {
      return (
        <QuestionMockResults
          onExit={() => {
            setActiveSession(null);
            void refresh();
          }}
          session={activeSession}
        />
      );
    }
    return (
      <QuestionMockRunner
        onChange={setActiveSession}
        onExit={() => {
          setActiveSession(null);
          void refresh();
        }}
        onRefresh={refresh}
        session={activeSession}
      />
    );
  }

  return (
    <section className="v2-practice-panel" aria-label="Question-level mock exams">
      <div className="v2-section-heading">
        <div>
          <h2>Question-level examination</h2>
          <p>
            A real question-by-question mock using verified PYQs, server-authoritative timing,
            autosave and restart recovery. Existing aggregate mocks remain available separately.
          </p>
        </div>
      </div>
      {!pattern ? (
        <div className="v2-inactive-banner">
          <h2>Verified pattern required</h2>
          <p>Select a verified pattern version before starting a question-level mock.</p>
        </div>
      ) : (
        <>
          <div className="v2-question-mock-builder">
            <div className="v2-practice-form-grid">
              <label className="v2-field">
                <span>Mode</span>
                <select
                  onChange={(event) => {
                    setMode(event.target.value as 'full_length' | 'section' | 'custom');
                    setSelectedIds([]);
                  }}
                  value={mode}
                >
                  <option value="full_length">Full-length simulation</option>
                  <option value="section">Single-section test</option>
                  <option value="custom">Custom question set</option>
                </select>
              </label>
              {mode === 'section' && (
                <label className="v2-field">
                  <span>Section</span>
                  <select onChange={(event) => changeSection(event.target.value)} value={sectionId}>
                    {pattern.sections.map((section) => (
                      <option key={section.id} value={section.sectionId}>
                        {section.title}
                      </option>
                    ))}
                  </select>
                </label>
              )}
              <label className="v2-field">
                <span>Name</span>
                <input onChange={(event) => setTestName(event.target.value)} value={testName} />
              </label>
              <label className="v2-field">
                <span>Date</span>
                <input
                  onChange={(event) => setTestDate(event.target.value)}
                  type="date"
                  value={testDate}
                />
              </label>
              <label className="v2-field">
                <span>Time limit (minutes)</span>
                <input
                  min={1}
                  onChange={(event) => setDuration(event.target.value)}
                  type="number"
                  value={duration}
                />
              </label>
            </div>
            <p className="v2-empty-copy">
              Pattern: {pattern.patternTitle} · {pattern.versionLabel}. Full-length mode requires
              the official question count in every section. Only verified questions with verified
              answer keys and recorded choices are listed. Sections with an official attempt limit
              evaluate only the first N answered questions.
            </p>
            <div className="v2-question-mock-actions">
              <button className="secondary-button" onClick={selectAllVisible} type="button">
                Select all listed
              </button>
              <button className="secondary-button" onClick={() => setSelectedIds([])} type="button">
                Clear selection
              </button>
              <span>
                Selected {selectedCandidates.length} of {visibleCandidates.length} listed
              </span>
            </div>
            {candidatesError && (
              <p className="v2-inline-error" role="alert">
                {candidatesError}
              </p>
            )}
            {candidatesLoading ? (
              <p role="status">Loading eligible questions…</p>
            ) : candidates.length === 0 ? (
              <div className="v2-empty-panel">
                <h2>No eligible question-level questions</h2>
                <p>
                  Add verified PYQ questions with verified answer keys and at least two answer
                  choices for MCQ/MSQ before starting a question-level mock.
                </p>
              </div>
            ) : (
              <>
                {mode === 'full_length' && (
                  <div className="v2-question-mock-counts">
                    {pattern.sections.map((section) => (
                      <div key={section.id}>
                        <strong>
                          {selectedCounts.get(section.id) ?? 0} / {section.questionCount}
                        </strong>
                        <span>{section.title}</span>
                      </div>
                    ))}
                  </div>
                )}
                <div className="v2-question-mock-candidates">
                  {visibleCandidates.map((candidate) => (
                    <label className="v2-question-mock-candidate" key={candidate.question.id}>
                      <input
                        checked={selectedIds.includes(candidate.question.id)}
                        onChange={() => toggleQuestion(candidate.question.id)}
                        type="checkbox"
                      />
                      <span>
                        <strong>
                          {candidate.question.year} · Q{candidate.question.questionNumber} ·{' '}
                          {candidate.question.questionTypeCode}
                        </strong>
                        <span>{candidate.patternSectionTitle}</span>
                        <span>
                          {candidate.question.statement.slice(0, 160) || 'Question text stored'}
                        </span>
                        <span>
                          +{candidate.question.maxMarks} / {candidate.question.negativeMarks} ·{' '}
                          {candidate.question.options.length} choices
                        </span>
                      </span>
                    </label>
                  ))}
                </div>
                {!selectionValid && (
                  <p className="v2-empty-copy">
                    {mode === 'full_length'
                      ? 'Select exactly the required question count for every pattern section.'
                      : 'Select at least one eligible question from one section.'}
                  </p>
                )}
                {!sectionLimitValid && (
                  <p className="v2-inline-error">
                    The selected section has more questions than the pattern allows.
                  </p>
                )}
                {error && (
                  <p className="v2-inline-error" role="alert">
                    {error}
                  </p>
                )}
                <div className="v2-form-actions">
                  <button
                    className="primary-button"
                    disabled={
                      busy ||
                      !selectionValid ||
                      !sectionLimitValid ||
                      !testName.trim() ||
                      !testDate ||
                      !duration
                    }
                    onClick={() => void startMock()}
                    type="button"
                  >
                    {busy ? 'Starting…' : 'Start timed mock'}
                  </button>
                </div>
              </>
            )}
          </div>
        </>
      )}

      <h3>Question-level history</h3>
      {questionMocks.length === 0 ? (
        <p className="v2-empty-copy">No question-level mocks recorded for this examination.</p>
      ) : (
        <ul className="v2-practice-mock-list">
          {questionMocks.map((mock) => (
            <li className="v2-practice-card" key={mock.id}>
              <div className="v2-section-heading">
                <div>
                  <strong>{mock.testName}</strong>
                  <p className="v2-empty-copy">
                    {mock.testDate} · {mock.patternVersionLabel ?? 'Pattern not stated'} ·{' '}
                    {mock.testMode} · {mock.status}
                    {mock.submissionKind === 'automatic' ? ' · auto-submitted' : ''}
                  </p>
                </div>
                <strong>
                  {mock.status === 'completed' ? percent(mock.percentage) : mock.status}
                </strong>
              </div>
              <p className="v2-empty-copy">
                {mock.availableQuestions} questions · {mock.correctQuestions} correct ·{' '}
                {mock.incorrectQuestions} incorrect · {mock.unansweredQuestions} unanswered
              </p>
              <div className="v2-form-actions">
                {(mock.status === 'in_progress' || mock.status === 'completed') && (
                  <button
                    className="secondary-button"
                    disabled={loadingSession === mock.id}
                    onClick={() => void openMock(mock.id)}
                    type="button"
                  >
                    {loadingSession === mock.id
                      ? 'Loading…'
                      : mock.status === 'in_progress'
                        ? 'Resume'
                        : 'Review results'}
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

type QuestionMockRunnerProps = {
  session: V2QuestionMockSession;
  onChange: (session: V2QuestionMockSession) => void;
  onExit: () => void;
  onRefresh: () => Promise<void>;
};

function QuestionMockRunner({ session, onChange, onExit, onRefresh }: QuestionMockRunnerProps) {
  const [remaining, setRemaining] = useState<number | null>(session.remainingSeconds);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [activeSectionId, setActiveSectionId] = useState(
    session.sections[0]?.sectionId ?? session.questions[0]?.sectionId ?? 0,
  );
  const [currentQuestionId, setCurrentQuestionId] = useState<number>(
    session.currentQuestionId ?? session.questions[0]?.id ?? 0,
  );
  const saveQueue = useRef<Promise<void>>(Promise.resolve());
  const pendingSaves = useRef(0);
  const autoSubmitted = useCallback(
    async (automatic: boolean) => {
      setSubmitting(true);
      setError(null);
      try {
        await saveQueue.current;
        const completed = await submitV2QuestionMock({
          examId: session.examId,
          mockId: session.mockId,
          confirmation: true,
          automatic,
        });
        onChange(completed);
        await onRefresh();
      } catch (caught) {
        setError(toCommandError(caught).message);
      } finally {
        setSubmitting(false);
      }
    },
    [onChange, onRefresh, session.examId, session.mockId],
  );

  const autoSubmitRef = useRef(false);
  useEffect(() => {
    const timer = window.setInterval(() => {
      setRemaining((current) => (current === null ? null : Math.max(0, current - 1)));
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (remaining === 0 && !autoSubmitRef.current && !submitting) {
      autoSubmitRef.current = true;
      void autoSubmitted(true);
    }
  }, [autoSubmitted, remaining, submitting]);

  const currentQuestion =
    session.questions.find((question) => question.id === currentQuestionId) ?? session.questions[0];
  const sectionQuestions = session.questions
    .filter((question) => question.sectionId === activeSectionId)
    .sort((left, right) => left.displayOrder - right.displayOrder);
  const currentSectionIndex = sectionQuestions.findIndex(
    (question) => question.id === currentQuestion?.id,
  );

  function persist(patch: {
    mockQuestionId?: number;
    selectedAnswer?: string;
    markedForReview?: boolean;
    currentQuestionId?: number;
  }) {
    pendingSaves.current += 1;
    setSaving(true);
    setError(null);
    saveQueue.current = saveQueue.current
      .then(async () => {
        const saved = await saveV2QuestionMockState({
          examId: session.examId,
          mockId: session.mockId,
          ...patch,
        });
        setRemaining(saved.remainingSeconds);
        if (saved.status === 'completed') {
          const completed = await getV2QuestionMockSession(session.examId, session.mockId);
          onChange(completed);
        }
      })
      .catch((caught) => {
        setError(toCommandError(caught).message);
      })
      .finally(() => {
        pendingSaves.current -= 1;
        if (pendingSaves.current === 0) setSaving(false);
      });
  }

  function updateAnswer(value: string) {
    if (!currentQuestion) return;
    onChange({
      ...session,
      currentQuestionId: currentQuestion.id,
      questions: session.questions.map((question) =>
        question.id === currentQuestion.id
          ? { ...question, selectedAnswer: value, visited: true }
          : question,
      ),
    });
    void persist({
      mockQuestionId: currentQuestion.id,
      selectedAnswer: value,
      currentQuestionId: currentQuestion.id,
    });
  }

  function toggleMarked() {
    if (!currentQuestion) return;
    const next = !currentQuestion.markedForReview;
    onChange({
      ...session,
      questions: session.questions.map((question) =>
        question.id === currentQuestion.id ? { ...question, markedForReview: next } : question,
      ),
    });
    void persist({
      mockQuestionId: currentQuestion.id,
      markedForReview: next,
      currentQuestionId: currentQuestion.id,
    });
  }

  function clearResponse() {
    if (!currentQuestion) return;
    updateAnswer('');
  }

  function goTo(question: V2QuestionMockQuestion) {
    setCurrentQuestionId(question.id);
    setActiveSectionId(question.sectionId);
    onChange({ ...session, currentQuestionId: question.id });
    void persist({ currentQuestionId: question.id });
  }

  function switchSection(sectionIdValue: number) {
    setActiveSectionId(sectionIdValue);
    const first = session.questions
      .filter((question) => question.sectionId === sectionIdValue)
      .sort((left, right) => left.displayOrder - right.displayOrder)[0];
    if (first) {
      goTo(first);
    }
  }

  const answered = session.questions.filter(
    (question) => question.selectedAnswer.trim() !== '',
  ).length;
  const marked = session.questions.filter((question) => question.markedForReview).length;
  const notVisited = session.questions.filter((question) => !question.visited).length;

  return (
    <section
      aria-label="Question-level mock exam"
      className="v2-question-mock-runner"
      data-workspace="mock"
    >
      <header className="v2-question-mock-header">
        <div>
          <p className="eyebrow">
            {session.patternTitle} · {session.patternVersionLabel}
          </p>
          <h2>{session.testName}</h2>
        </div>
        <div className="v2-question-mock-timer">
          <span>{remaining === 0 ? 'Time expired' : formatSeconds(remaining)}</span>
          <small>{saving ? 'Saving…' : submitting ? 'Submitting…' : 'Saved automatically'}</small>
        </div>
      </header>
      {error && (
        <p className="v2-inline-error" role="alert">
          {error}
        </p>
      )}
      <div className="v2-question-mock-section-tabs">
        {session.sections.map((section) => (
          <button
            className={`nav-button${activeSectionId === section.sectionId ? ' nav-item-active' : ''}`}
            key={section.sectionId}
            onClick={() => switchSection(section.sectionId)}
            type="button"
          >
            {section.sectionCode} · {section.sectionTitle}
            {section.maxAttempts ? ` · first ${section.maxAttempts} evaluated` : ''}
          </button>
        ))}
      </div>
      {currentQuestion && (
        <div className="v2-question-mock-body">
          <article className="v2-question-mock-card">
            <div className="v2-question-mock-meta">
              <span>
                Question {currentQuestion.displayOrder} of {session.questions.length}
              </span>
              <span>
                {currentQuestion.questionTypeCode} · +{currentQuestion.maxMarks} /{' '}
                {currentQuestion.negativeMarks}
              </span>
              {currentQuestion.markedForReview && (
                <span className="v2-qpalette-marked">Marked</span>
              )}
            </div>
            {currentQuestion.topicTitle && (
              <p className="v2-empty-copy">Topic: {currentQuestion.topicTitle}</p>
            )}
            <p className="v2-question-mock-statement">{currentQuestion.statement}</p>
            {currentQuestion.questionTypeCode === 'MCQ' && (
              <div className="v2-question-mock-options">
                {currentQuestion.options.map((option) => (
                  <label key={option.key}>
                    <input
                      checked={currentQuestion.selectedAnswer === option.key}
                      name={`question-${currentQuestion.id}`}
                      onChange={() => updateAnswer(option.key)}
                      type="radio"
                    />
                    <span>
                      <strong>{option.key}</strong> {option.text}
                    </span>
                  </label>
                ))}
              </div>
            )}
            {currentQuestion.questionTypeCode === 'MSQ' && (
              <div className="v2-question-mock-options">
                {currentQuestion.options.map((option) => {
                  const selected = optionsList(currentQuestion.selectedAnswer);
                  return (
                    <label key={option.key}>
                      <input
                        checked={selected.includes(option.key)}
                        onChange={() => {
                          const next = selected.includes(option.key)
                            ? selected.filter((value) => value !== option.key)
                            : [...selected, option.key].sort();
                          updateAnswer(next.join(','));
                        }}
                        type="checkbox"
                      />
                      <span>
                        <strong>{option.key}</strong> {option.text}
                      </span>
                    </label>
                  );
                })}
              </div>
            )}
            {currentQuestion.questionTypeCode === 'NAT' && (
              <label className="v2-field">
                <span>Numerical answer</span>
                <input
                  onChange={(event) => updateAnswer(event.target.value)}
                  placeholder="Enter your answer"
                  value={currentQuestion.selectedAnswer}
                />
              </label>
            )}
            <div className="v2-question-mock-controls">
              <button
                className="secondary-button"
                disabled={currentSectionIndex <= 0}
                onClick={() => {
                  const previous = sectionQuestions[currentSectionIndex - 1];
                  if (previous) goTo(previous);
                }}
                type="button"
              >
                Previous
              </button>
              <button className="secondary-button" onClick={toggleMarked} type="button">
                {currentQuestion.markedForReview ? 'Unmark review' : 'Mark for review'}
              </button>
              <button
                className="secondary-button"
                disabled={!currentQuestion.selectedAnswer}
                onClick={clearResponse}
                type="button"
              >
                Clear response
              </button>
              <button
                className="secondary-button"
                disabled={
                  currentSectionIndex < 0 || currentSectionIndex >= sectionQuestions.length - 1
                }
                onClick={() => {
                  const next = sectionQuestions[currentSectionIndex + 1];
                  if (next) goTo(next);
                }}
                type="button"
              >
                Next
              </button>
            </div>
          </article>
          <aside className="v2-question-mock-palette">
            <div className="v2-section-heading">
              <h3>Question palette</h3>
              <button
                className="secondary-button"
                onClick={() => setConfirming(true)}
                type="button"
              >
                Submit
              </button>
            </div>
            <div className="v2-qpalette-grid">
              {session.questions.map((question) => {
                const classes = ['v2-qpalette-item'];
                if (question.markedForReview) classes.push('v2-qpalette-marked');
                else if (question.selectedAnswer.trim()) classes.push('v2-qpalette-answered');
                else if (question.visited) classes.push('v2-qpalette-visited');
                else classes.push('v2-qpalette-unvisited');
                if (question.id === currentQuestion.id) classes.push('v2-qpalette-current');
                return (
                  <button
                    className={classes.join(' ')}
                    key={question.id}
                    onClick={() => goTo(question)}
                    title={`Question ${question.displayOrder} · ${question.sectionCode}`}
                    type="button"
                  >
                    {question.displayOrder}
                  </button>
                );
              })}
            </div>
            <div className="v2-qpalette-legend">
              <span className="v2-qpalette-answered">Answered {answered}</span>
              <span className="v2-qpalette-visited">
                Visited {session.questions.length - notVisited - answered}
              </span>
              <span className="v2-qpalette-unvisited">Not visited {notVisited}</span>
              <span className="v2-qpalette-marked">Marked {marked}</span>
            </div>
            <button className="secondary-button" onClick={onExit} type="button">
              Leave without submitting
            </button>
          </aside>
        </div>
      )}
      {confirming && (
        <div className="v2-question-mock-confirm" role="dialog" aria-modal="true">
          <h3>Submit the mock?</h3>
          <p>
            {answered} answered · {session.questions.length - answered} unanswered · {marked} marked
            for review. Submission closes the exam and cannot be undone.
          </p>
          <div className="v2-form-actions">
            <button
              className="primary-button"
              disabled={submitting}
              onClick={() => {
                setConfirming(false);
                void autoSubmitted(false);
              }}
              type="button"
            >
              {submitting ? 'Submitting…' : 'Submit now'}
            </button>
            <button
              className="secondary-button"
              disabled={submitting}
              onClick={() => setConfirming(false)}
              type="button"
            >
              Continue exam
            </button>
          </div>
        </div>
      )}
    </section>
  );
}

function QuestionMockResults({
  session,
  onExit,
}: {
  session: V2QuestionMockSession;
  onExit: () => void;
}) {
  const notEvaluated = session.questions.filter((question) => question.notEvaluated).length;
  return (
    <section className="v2-practice-panel" aria-label="Question-level mock results">
      <div className="v2-section-heading">
        <div>
          <p className="eyebrow">
            {session.patternTitle} · {session.patternVersionLabel}
          </p>
          <h2>{session.testName} results</h2>
          <p>
            Submitted {session.submissionKind === 'automatic' ? 'automatically' : 'manually'} on{' '}
            {session.endedAt ?? session.testDate}. The scoring scheme is the pattern snapshot
            captured when the mock started.
          </p>
        </div>
        <button className="secondary-button" onClick={onExit} type="button">
          Back to question-level exams
        </button>
      </div>
      <div className="v2-practice-stats">
        <div>
          <strong>{session.marksObtained?.toFixed(2) ?? '—'}</strong>
          <span>Marks / {session.maxMarks.toFixed(2)}</span>
        </div>
        <div>
          <strong>{percent(session.percentage)}</strong>
          <span>Percentage</span>
        </div>
        <div>
          <strong>{session.correctQuestions}</strong>
          <span>Correct</span>
        </div>
        <div>
          <strong>{session.incorrectQuestions}</strong>
          <span>Incorrect</span>
        </div>
        <div>
          <strong>{session.unansweredQuestions}</strong>
          <span>Unanswered</span>
        </div>
        <div>
          <strong>{session.averageTimeSeconds?.toFixed(1) ?? '—'}s</strong>
          <span>Average time / question</span>
        </div>
        <div>
          <strong>{session.totalTimeSeconds}s</strong>
          <span>Total question time</span>
        </div>
      </div>
      {notEvaluated > 0 && (
        <p className="v2-empty-copy">
          {notEvaluated} answered question(s) were outside the official section attempt limit and
          were not evaluated.
        </p>
      )}
      <h3>Section results</h3>
      <div className="v2-question-mock-table-wrap">
        <table className="v2-question-mock-table">
          <thead>
            <tr>
              <th>Section</th>
              <th>Questions</th>
              <th>Attempt limit</th>
              <th>Correct</th>
              <th>Incorrect</th>
              <th>Unanswered</th>
              <th>Marks</th>
              <th>Accuracy</th>
              <th>Avg time</th>
            </tr>
          </thead>
          <tbody>
            {session.sections.map((section) => (
              <tr key={section.id}>
                <td>{section.sectionTitle}</td>
                <td>{section.questionCount}</td>
                <td>{section.maxAttempts ?? '—'}</td>
                <td>{section.correct}</td>
                <td>{section.incorrect}</td>
                <td>{section.unanswered}</td>
                <td>
                  {section.marksObtained.toFixed(2)} / {section.maxMarks.toFixed(2)}
                </td>
                <td>{percent(section.accuracyPercentage)}</td>
                <td>{section.averageTimeSeconds?.toFixed(1) ?? '—'}s</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {session.topics.length > 0 && (
        <>
          <h3>Topic results</h3>
          <div className="v2-question-mock-table-wrap">
            <table className="v2-question-mock-table">
              <thead>
                <tr>
                  <th>Topic</th>
                  <th>Questions</th>
                  <th>Correct</th>
                  <th>Incorrect</th>
                  <th>Unanswered</th>
                  <th>Accuracy</th>
                  <th>Avg time</th>
                </tr>
              </thead>
              <tbody>
                {session.topics.map((topic) => (
                  <tr key={topic.topicId}>
                    <td>{topic.topicTitle}</td>
                    <td>{topic.questions}</td>
                    <td>{topic.correct}</td>
                    <td>{topic.incorrect}</td>
                    <td>{topic.unanswered}</td>
                    <td>{percent(topic.accuracyPercentage)}</td>
                    <td>{topic.averageTimeSeconds?.toFixed(1) ?? '—'}s</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {session.compatibleAttempts.length > 0 && (
        <>
          <h3>Compatible pattern attempts</h3>
          <ul className="v2-practice-history">
            {session.compatibleAttempts.map((attempt) => (
              <li key={attempt.mockId}>
                <strong>
                  {attempt.testDate} · {attempt.testName}
                  {attempt.current ? ' · current' : ''}
                </strong>
                <span>
                  {percent(attempt.percentage)} · {attempt.marksObtained?.toFixed(2) ?? '—'} /{' '}
                  {attempt.maxMarks.toFixed(2)}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
      <h3>Question review</h3>
      <ul className="v2-question-mock-review">
        {session.questions.map((question) => (
          <li key={question.id}>
            <div className="v2-question-mock-review-head">
              <strong>
                Q{question.displayOrder} · {question.sectionCode} · {question.questionTypeCode}
              </strong>
              <span
                className={`v2-qpalette-${question.responseStatus === 'correct' ? 'answered' : 'marked'}`}
              >
                {question.notEvaluated ? 'not evaluated' : question.responseStatus}
              </span>
              <span>
                {question.marksObtained.toFixed(2)} / {question.maxMarks.toFixed(2)} ·{' '}
                {question.timeSpentSeconds}s
              </span>
            </div>
            <p>{question.statement}</p>
            {question.options.length > 0 && (
              <ul className="v2-question-mock-review-options">
                {question.options.map((option) => {
                  const selected = optionsList(question.selectedAnswer).includes(option.key);
                  const correct = optionsList(question.correctAnswer ?? '').includes(option.key);
                  return (
                    <li
                      className={
                        correct
                          ? 'v2-question-mock-option-correct'
                          : selected
                            ? 'v2-question-mock-option-selected'
                            : ''
                      }
                      key={option.key}
                    >
                      {option.key}. {option.text}
                      {selected ? ' · selected' : ''}
                      {correct ? ' · correct' : ''}
                    </li>
                  );
                })}
              </ul>
            )}
            {question.questionTypeCode === 'NAT' && (
              <p>
                Selected {question.selectedAnswer || '—'} · correct{' '}
                {question.correctAnswer || question.correctRange || '—'}
              </p>
            )}
            {question.officialExplanation && (
              <>
                <h4>Official explanation</h4>
                <p>{question.officialExplanation}</p>
              </>
            )}
            {question.thirdPartySolution && (
              <>
                <h4>Third-party solution</h4>
                <p>{question.thirdPartySolution}</p>
              </>
            )}
            {question.userNote && (
              <>
                <h4>User note</h4>
                <p>{question.userNote}</p>
              </>
            )}
            <p className="v2-empty-copy">
              {question.sourceTitle || 'Source not stated'}
              {question.sourcePage ? ` · page ${question.sourcePage}` : ''}
              {question.topicTitle ? ` · topic ${question.topicTitle}` : ''}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}
