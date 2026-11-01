import { type FormEvent, useEffect, useState } from 'react';
import { CsvImportPanel } from '../components/CsvImportPanel';
import { DateField } from '../components/DateField';
import { MockTestEditor } from '../components/MockTestEditor';
import { toCommandError } from '../lib/commandError';
import { formatDateLong, parseDateOnly } from '../lib/dateOnly';
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

const PAGE_SIZE = 20;
const PERCENTAGE_TOLERANCE = 1e-9;

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
  const [dateForm, setDateForm] = useState({ fromDate: '', toDate: '' });
  const [appliedDates, setAppliedDates] = useState<{ fromDate?: string; toDate?: string }>({});
  const [dateError, setDateError] = useState<string | null>(null);
  const [page, setPage] = useState(1);

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

  function handleDateFilter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const fromDate = dateForm.fromDate;
    const toDate = dateForm.toDate;

    if (fromDate && !parseDateOnly(fromDate)) {
      setDateError('Choose a real start date or leave it blank.');
      return;
    }
    if (toDate && !parseDateOnly(toDate)) {
      setDateError('Choose a real end date or leave it blank.');
      return;
    }
    if (fromDate && toDate && fromDate > toDate) {
      setDateError('The start date cannot be after the end date.');
      return;
    }

    setDateError(null);
    setPage(1);
    setAppliedDates({
      ...(fromDate ? { fromDate } : {}),
      ...(toDate ? { toDate } : {}),
    });
  }

  function clearDateFilter() {
    setDateForm({ fromDate: '', toDate: '' });
    setDateError(null);
    setPage(1);
    setAppliedDates({});
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

  const filteredTests = tests.filter((test) => {
    if (appliedDates.fromDate && test.testDate < appliedDates.fromDate) {
      return false;
    }
    if (appliedDates.toDate && test.testDate > appliedDates.toDate) {
      return false;
    }
    return true;
  });
  const averagePercentage =
    filteredTests.length === 0
      ? null
      : filteredTests.reduce((total, test) => total + test.computedPercentage, 0) /
        filteredTests.length;
  const bestPercentage =
    filteredTests.length === 0
      ? null
      : Math.max(...filteredTests.map((test) => test.computedPercentage));
  const latest = filteredTests[0] ?? null;
  const invalidConfigurationCount = tests.filter(
    (test) =>
      !test.configurationValid ||
      !Number.isFinite(test.computedPercentage) ||
      test.computedPercentage > 100 + PERCENTAGE_TOLERANCE,
  ).length;
  const pageCount = Math.max(1, Math.ceil(filteredTests.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount);
  const visibleTests = filteredTests.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);
  const hasDateFilter = Boolean(appliedDates.fromDate || appliedDates.toDate);

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
          <span>Tests shown</span>
          <strong>{filteredTests.length}</strong>
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

      {invalidConfigurationCount > 0 && (
        <p className="save-feedback save-feedback-warning" role="status">
          {invalidConfigurationCount} saved mock test
          {invalidConfigurationCount === 1 ? '' : 's'} use
          {invalidConfigurationCount === 1 ? 's' : ''} a marking configuration that would now be
          rejected. Review maximum marks, penalty signs, and part totals for the affected record
          {invalidConfigurationCount === 1 ? '' : 's'}.
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
              <h2 id="mock-history-heading">
                Previous mock tests ({filteredTests.length}
                {hasDateFilter ? ` of ${tests.length}` : ''})
              </h2>
            </div>
          </div>

          <form className="history-date-filter" onSubmit={handleDateFilter} noValidate>
            <DateField
              id="mock-history-from"
              label="Test date from (optional)"
              value={dateForm.fromDate}
              max={dateForm.toDate || undefined}
              onChange={(value) => {
                setDateForm((current) => ({ ...current, fromDate: value }));
                setDateError(null);
              }}
            />
            <DateField
              id="mock-history-to"
              label="Test date to (optional)"
              value={dateForm.toDate}
              min={dateForm.fromDate || undefined}
              onChange={(value) => {
                setDateForm((current) => ({ ...current, toDate: value }));
                setDateError(null);
              }}
            />
            <div className="history-date-actions">
              <button type="submit" className="secondary-button">
                Apply dates
              </button>
              {hasDateFilter && (
                <button type="button" className="link-button" onClick={clearDateFilter}>
                  Clear dates
                </button>
              )}
            </div>
            {dateError && (
              <p className="field-error history-date-error" role="alert">
                {dateError}
              </p>
            )}
          </form>

          {filteredTests.length === 0 ? (
            <div className="task-empty">
              <p className="task-empty-title">
                {tests.length === 0
                  ? 'No mock tests recorded yet'
                  : 'No mock tests match this date range'}
              </p>
              <p>
                {tests.length === 0
                  ? 'Add a completed mock test with its part counts and marking configuration, or import a CSV file using the template below.'
                  : 'Choose a wider test-date range or clear the dates to see every saved mock test.'}
              </p>
            </div>
          ) : (
            <div className="mock-list">
              {visibleTests.map((test) => (
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
                      {(!test.configurationValid ||
                        !Number.isFinite(test.computedPercentage) ||
                        test.computedPercentage > 100 + PERCENTAGE_TOLERANCE) && (
                        <small className="field-error">Review marking configuration</small>
                      )}
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

          {pageCount > 1 && (
            <nav className="history-pagination" aria-label="Mock-test history pages">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setPage((current) => Math.max(1, Math.min(current, pageCount) - 1))}
                disabled={safePage === 1}
              >
                Previous page
              </button>
              <span>
                Page {safePage} of {pageCount} · {filteredTests.length} tests
              </span>
              <button
                type="button"
                className="secondary-button"
                onClick={() =>
                  setPage((current) => Math.min(pageCount, Math.min(current, pageCount) + 1))
                }
                disabled={safePage === pageCount}
              >
                Next page
              </button>
            </nav>
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
