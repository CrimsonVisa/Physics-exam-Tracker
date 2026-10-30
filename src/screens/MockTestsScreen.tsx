import { useEffect, useState } from 'react';
import { CsvImportPanel } from '../components/CsvImportPanel';
import { MockTestEditor } from '../components/MockTestEditor';
import { toCommandError } from '../lib/commandError';
import { formatDateLong } from '../lib/dateOnly';
import {
  applyMockImport,
  deleteMockTest,
  getMockTests,
  previewMockImport,
  saveMockCsvTemplate,
  type MockTest,
} from '../lib/mockTestApi';
import { formatStudyDuration } from '../lib/studyTime';

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function formatScore(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

function percentageLabel(value: number): string {
  return `${value.toFixed(1)}%`;
}

export default function MockTestsScreen({ today }: { today: string }) {
  const [tests, setTests] = useState<MockTest[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [editor, setEditor] = useState<{ test: MockTest | null } | null>(null);
  const [editorVersion, setEditorVersion] = useState(0);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'error'; message: string } | null>(
    null,
  );
  const [pendingDelete, setPendingDelete] = useState<number | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    getMockTests()
      .then((loaded) => {
        if (!cancelled) {
          setTests(loaded);
          setLoadError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setLoadError(toCommandError(error).message);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  function reload() {
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }

  async function handleDelete(testId: number) {
    if (deletingId !== null) {
      return;
    }

    setDeletingId(testId);
    setFeedback(null);

    try {
      await deleteMockTest(testId);
      setPendingDelete(null);
      if (editor?.test?.id === testId) {
        setEditor(null);
      }
      setFeedback({ tone: 'success', message: 'Mock test deleted.' });
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setDeletingId(null);
    }
  }

  const averagePercentage =
    tests.length === 0
      ? null
      : tests.reduce((total, test) => total + test.computedPercentage, 0) / tests.length;
  const bestPercentage =
    tests.length === 0 ? null : Math.max(...tests.map((test) => test.computedPercentage));
  const latest = tests[0] ?? null;

  return (
    <div className="page-container mock-page">
      <header className="page-header">
        <p className="eyebrow">Mock tests</p>
        <h1>Full-test performance</h1>
        <p className="page-lede">
          Keep mock-test records separate from individual PYQ attempts. Scores are calculated from
          each test's saved part counts and marking configuration, so no universal negative-marking
          scheme is assumed.
        </p>
      </header>

      <section className="mock-summary-grid" aria-label="Mock test summary">
        <div className="mock-summary-card">
          <span>Tests recorded</span>
          <strong>{tests.length}</strong>
        </div>
        <div className="mock-summary-card">
          <span>Average calculated score</span>
          <strong>
            {averagePercentage === null ? 'No data' : percentageLabel(averagePercentage)}
          </strong>
        </div>
        <div className="mock-summary-card">
          <span>Best calculated score</span>
          <strong>{bestPercentage === null ? 'No data' : percentageLabel(bestPercentage)}</strong>
        </div>
        <div className="mock-summary-card">
          <span>Latest calculated score</span>
          <strong>
            {latest === null ? 'No data' : percentageLabel(latest.computedPercentage)}
          </strong>
          {latest && <small>{dateLabel(latest.testDate)}</small>}
        </div>
      </section>

      <div className="mock-toolbar">
        <button
          type="button"
          className="primary-button"
          onClick={() => {
            setEditor({ test: null });
            setEditorVersion((version) => version + 1);
            setPendingDelete(null);
            setFeedback(null);
          }}
        >
          New mock test
        </button>
        {latest && (
          <p className="mock-latest-note">
            Latest: <strong>{latest.testName}</strong> on {dateLabel(latest.testDate)} ·{' '}
            {formatScore(latest.computedMarks)} / {formatScore(latest.maxMarks)} calculated
          </p>
        )}
      </div>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      {editor && (
        <section className="mock-editor-card" aria-labelledby="mock-editor-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{editor.test ? 'Editing' : 'New record'}</p>
              <h2 id="mock-editor-heading">{editor.test ? 'Edit mock test' : 'Add a mock test'}</h2>
            </div>
            <button type="button" className="secondary-button" onClick={() => setEditor(null)}>
              Close editor
            </button>
          </div>
          <p className="settings-lede">
            Enter question counts for each part and the marking values used by this test. The
            calculated score updates as you edit and is stored from these values.
          </p>
          <MockTestEditor
            key={
              editor.test ? `mock-${editor.test.id}-${editorVersion}` : `mock-new-${editorVersion}`
            }
            test={editor.test}
            today={today}
            onSaved={(saved, wasEditing) => {
              setEditor(null);
              setEditorVersion((version) => version + 1);
              setFeedback({
                tone: 'success',
                message: wasEditing
                  ? `Mock test “${saved.testName}” updated.`
                  : `Mock test “${saved.testName}” added.`,
              });
              reload();
            }}
            onCancel={() => {
              setEditor(null);
              setEditorVersion((version) => version + 1);
              setFeedback(null);
            }}
          />
        </section>
      )}

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading mock tests.</p>
          <p className="state-copy">Reading saved part counts and marking configurations.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Mock tests could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button type="button" className="primary-button" onClick={reload}>
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && (
        <section className="mock-history-card" aria-labelledby="mock-history-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Review history</p>
              <h2 id="mock-history-heading">Previous mock tests ({tests.length})</h2>
            </div>
          </div>

          {tests.length === 0 ? (
            <div className="task-empty">
              <p className="task-empty-title">No mock tests recorded yet</p>
              <p>
                Add a completed mock test with its part counts and marking configuration, or import
                a CSV file using the template below.
              </p>
            </div>
          ) : (
            <div className="mock-list">
              {tests.map((test) => (
                <article className="mock-row" key={test.id}>
                  <div className="mock-row-heading">
                    <div>
                      <h3>{test.testName}</h3>
                      <p className="mock-meta">
                        {dateLabel(test.testDate)} ·{' '}
                        {formatStudyDuration(test.durationMinutes * 60)} · Reported score{' '}
                        {test.marksObtained === null
                          ? 'not recorded'
                          : `${formatScore(test.marksObtained)} / ${formatScore(test.maxMarks)}`}
                      </p>
                    </div>
                    <div className="mock-score-block">
                      <span>Calculated</span>
                      <strong>
                        {formatScore(test.computedMarks)} / {formatScore(test.maxMarks)}
                      </strong>
                      <small>{percentageLabel(test.computedPercentage)}</small>
                    </div>
                  </div>

                  <div className="mock-total-strip">
                    <span>{test.totalQuestions} questions</span>
                    <span>{test.totalAttempted} attempted</span>
                    <span>{test.totalCorrect} correct</span>
                    <span>{test.totalIncorrect} incorrect</span>
                    <span>{test.totalUnanswered} unanswered</span>
                  </div>

                  <div className="mock-part-table-wrap">
                    <table className="mock-part-table">
                      <thead>
                        <tr>
                          <th scope="col">Part</th>
                          <th scope="col">Questions</th>
                          <th scope="col">Attempted</th>
                          <th scope="col">Correct</th>
                          <th scope="col">Incorrect</th>
                          <th scope="col">Unanswered</th>
                          <th scope="col">Marking</th>
                          <th scope="col">Marks</th>
                        </tr>
                      </thead>
                      <tbody>
                        {[
                          ['A', test.partA],
                          ['B', test.partB],
                          ['C', test.partC],
                        ].map(([label, part]) => {
                          const result = part as MockTest['partA'];
                          return (
                            <tr key={`${test.id}-part-${label as string}`}>
                              <th scope="row">{label as string}</th>
                              <td>{result.questions}</td>
                              <td>{result.attempted}</td>
                              <td>{result.correct}</td>
                              <td>{result.incorrect}</td>
                              <td>{result.unanswered}</td>
                              <td>
                                {formatScore(result.correctMarks)} /{' '}
                                {formatScore(result.incorrectMarks)} /{' '}
                                {formatScore(result.unansweredMarks)}
                              </td>
                              <td>{formatScore(result.marks)}</td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                  </div>

                  {(test.notes || test.reviewObservations) && (
                    <div className="mock-notes-grid">
                      {test.notes && (
                        <div>
                          <h4>Notes</h4>
                          <p>{test.notes}</p>
                        </div>
                      )}
                      {test.reviewObservations && (
                        <div>
                          <h4>Review observations</h4>
                          <p>{test.reviewObservations}</p>
                        </div>
                      )}
                    </div>
                  )}

                  <div className="mock-row-actions">
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => {
                        setEditor({ test });
                        setPendingDelete(null);
                        setFeedback(null);
                      }}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="danger-button"
                      onClick={() => setPendingDelete(test.id)}
                      disabled={deletingId !== null}
                    >
                      Delete
                    </button>
                  </div>

                  {pendingDelete === test.id && (
                    <div
                      className="task-delete-confirm"
                      role="alertdialog"
                      aria-labelledby={`delete-mock-${test.id}`}
                    >
                      <p className="task-delete-title" id={`delete-mock-${test.id}`}>
                        Delete this mock test permanently?
                      </p>
                      <p>
                        The mock-test record and its review notes will be removed. Study sessions
                        and PYQ attempts are not affected.
                      </p>
                      <div className="form-footer">
                        <button
                          type="button"
                          className="danger-button"
                          onClick={() => void handleDelete(test.id)}
                          disabled={deletingId !== null}
                        >
                          {deletingId === test.id ? 'Deleting…' : 'Confirm delete'}
                        </button>
                        <button
                          type="button"
                          className="secondary-button"
                          onClick={() => setPendingDelete(null)}
                          disabled={deletingId !== null}
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  )}
                </article>
              ))}
            </div>
          )}
        </section>
      )}

      <details className="import-disclosure">
        <summary>Import mock tests from CSV</summary>
        <CsvImportPanel
          id="mock-import"
          title="Import mock tests"
          description="The template includes all three parts and their marking values. Preview validates the full row before any transaction is committed."
          saveTemplate={saveMockCsvTemplate}
          previewImport={previewMockImport}
          applyImport={applyMockImport}
          onImported={(result) => {
            setFeedback({
              tone: result.rejected > 0 ? 'error' : 'success',
              message: `Mock-test import finished: ${result.imported} imported, ${result.skipped} skipped, ${result.rejected} rejected.`,
            });
            reload();
          }}
        />
      </details>
    </div>
  );
}
