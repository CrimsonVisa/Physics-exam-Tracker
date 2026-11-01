import { type FormEvent, useEffect, useState } from 'react';
import { CsvImportPanel } from '../components/CsvImportPanel';
import { DateField } from '../components/DateField';
import { PyqEditor } from '../components/PyqEditor';
import { toCommandError } from '../lib/commandError';
import { formatDateLong, parseDateOnly } from '../lib/dateOnly';
import {
  applyPyqImport,
  deletePyqAttempt,
  getPyqAttempts,
  getPyqOptions,
  previewPyqImport,
  savePyqCsvTemplate,
  PYQ_PARTS,
  PYQ_STATUSES,
  type PyqAttempt,
  type PyqFilter,
  type PyqOptions,
  type PyqPart,
  type PyqStatus,
} from '../lib/pyqApi';
import { formatStudyDuration } from '../lib/studyTime';

type FilterForm = {
  year: string;
  part: string;
  sectionCode: string;
  topicId: string;
  status: string;
  query: string;
  fromDate: string;
  toDate: string;
};

type PendingDelete = number | null;

const EMPTY_OPTIONS: PyqOptions = { years: [], sections: [], topics: [] };
const MAX_QUERY_CHARS = 100;
const PAGE_SIZE = 20;

function statusLabel(status: PyqStatus): string {
  switch (status) {
    case 'correct':
      return 'Correct';
    case 'incorrect':
      return 'Incorrect';
    case 'skipped':
      return 'Skipped';
    default:
      return 'Attempted';
  }
}

function dateLabel(value: string): string {
  return formatDateLong(value) ?? value;
}

function marksLabel(attempt: PyqAttempt): string {
  if (attempt.marksObtained === null && attempt.maxMarks === null) {
    return 'Marks not recorded';
  }
  if (attempt.marksObtained === null) {
    return `Max ${attempt.maxMarks}`;
  }
  if (attempt.maxMarks === null) {
    return `${attempt.marksObtained} marks`;
  }
  return `${attempt.marksObtained} / ${attempt.maxMarks} marks`;
}

export default function PyqScreen({ today }: { today: string }) {
  const [options, setOptions] = useState<PyqOptions>(EMPTY_OPTIONS);
  const [optionsError, setOptionsError] = useState<string | null>(null);
  const [filterForm, setFilterForm] = useState<FilterForm>({
    year: '',
    part: '',
    sectionCode: '',
    topicId: '',
    status: '',
    query: '',
    fromDate: '',
    toDate: '',
  });
  const [appliedFilter, setAppliedFilter] = useState<PyqFilter>({});
  const [filterError, setFilterError] = useState<string | null>(null);
  const [attempts, setAttempts] = useState<PyqAttempt[]>([]);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [editor, setEditor] = useState<{ attempt: PyqAttempt | null } | null>(null);
  const [editorVersion, setEditorVersion] = useState(0);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'error'; message: string } | null>(
    null,
  );
  const [pendingDelete, setPendingDelete] = useState<PendingDelete>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;

    getPyqOptions()
      .then((loaded) => {
        if (!cancelled) {
          setOptions(loaded);
          setOptionsError(null);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setOptionsError(toCommandError(error).message);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [reloadKey]);

  useEffect(() => {
    let cancelled = false;

    getPyqAttempts(appliedFilter)
      .then((loaded) => {
        if (!cancelled) {
          setAttempts(loaded);
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
  }, [appliedFilter, reloadKey]);

  function reload() {
    setIsLoading(true);
    setLoadError(null);
    setReloadKey((key) => key + 1);
  }

  function handleFilter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFilterError(null);

    const query = filterForm.query.trim();
    if (query.length > MAX_QUERY_CHARS) {
      setFilterError(`Search text must be ${MAX_QUERY_CHARS} characters or fewer.`);
      return;
    }

    let year: number | undefined;
    if (filterForm.year) {
      year = Number(filterForm.year);
      if (!Number.isInteger(year) || year < 1900 || year > 2100) {
        setFilterError('Choose a valid question year.');
        return;
      }
    }

    let topicId: number | undefined;
    if (filterForm.topicId) {
      topicId = Number(filterForm.topicId);
      if (!Number.isSafeInteger(topicId) || topicId <= 0) {
        setFilterError('Choose a valid syllabus topic.');
        return;
      }
    }

    if (filterForm.fromDate && !parseDateOnly(filterForm.fromDate)) {
      setFilterError('Choose a real start date or leave it blank.');
      return;
    }
    if (filterForm.toDate && !parseDateOnly(filterForm.toDate)) {
      setFilterError('Choose a real end date or leave it blank.');
      return;
    }
    if (filterForm.fromDate && filterForm.toDate && filterForm.fromDate > filterForm.toDate) {
      setFilterError('The start date cannot be after the end date.');
      return;
    }

    setIsLoading(true);
    setLoadError(null);
    setPage(1);
    setAppliedFilter({
      ...(year ? { year } : {}),
      ...(filterForm.part ? { part: filterForm.part as PyqPart } : {}),
      ...(filterForm.sectionCode ? { sectionCode: filterForm.sectionCode } : {}),
      ...(topicId ? { topicId } : {}),
      ...(filterForm.status ? { status: filterForm.status as PyqStatus } : {}),
      ...(query ? { query } : {}),
      ...(filterForm.fromDate ? { fromDate: filterForm.fromDate } : {}),
      ...(filterForm.toDate ? { toDate: filterForm.toDate } : {}),
    });
  }

  function resetFilter() {
    setFilterForm({
      year: '',
      part: '',
      sectionCode: '',
      topicId: '',
      status: '',
      query: '',
      fromDate: '',
      toDate: '',
    });
    setFilterError(null);
    setIsLoading(true);
    setLoadError(null);
    setPage(1);
    setAppliedFilter({});
  }

  async function handleDelete(attemptId: number) {
    if (deletingId !== null) {
      return;
    }

    setDeletingId(attemptId);
    setFeedback(null);

    try {
      await deletePyqAttempt(attemptId);
      setPendingDelete(null);
      if (editor?.attempt?.id === attemptId) {
        setEditor(null);
      }
      setFeedback({ tone: 'success', message: 'PYQ attempt deleted.' });
      reload();
    } catch (error: unknown) {
      setFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setDeletingId(null);
    }
  }

  const filterTopicOptions = options.topics.filter(
    (topic) => !filterForm.sectionCode || topic.sectionCode === filterForm.sectionCode,
  );
  const activeFilters = Object.keys(appliedFilter).length > 0;
  const pageCount = Math.max(1, Math.ceil(attempts.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount);
  const visibleAttempts = attempts.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);

  return (
    <div className="page-container pyq-page">
      <header className="page-header">
        <p className="eyebrow">Previous-year questions</p>
        <h1>PYQ attempt bank</h1>
        <p className="page-lede">
          Record your own question references, attempts, outcomes, and notes. Nothing is fetched
          online and no real exam questions are invented or preloaded.
        </p>
      </header>

      <section className="pyq-filter-card" aria-labelledby="pyq-filter-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Filter history</p>
            <h2 id="pyq-filter-heading">Find PYQ attempts</h2>
          </div>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setEditor({ attempt: null });
              setEditorVersion((version) => version + 1);
              setFeedback(null);
            }}
          >
            New PYQ attempt
          </button>
        </div>

        <form className="pyq-filter-grid" onSubmit={handleFilter} noValidate>
          <div className="form-field">
            <label htmlFor="pyq-filter-year">Question year</label>
            <select
              id="pyq-filter-year"
              value={filterForm.year}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, year: event.target.value }))
              }
            >
              <option value="">All years</option>
              {options.years.map((year) => (
                <option value={year} key={year}>
                  {year}
                </option>
              ))}
            </select>
          </div>

          <div className="form-field">
            <label htmlFor="pyq-filter-part">Part</label>
            <select
              id="pyq-filter-part"
              value={filterForm.part}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, part: event.target.value }))
              }
            >
              <option value="">All parts</option>
              {PYQ_PARTS.map((part) => (
                <option value={part} key={part}>
                  Part {part}
                </option>
              ))}
            </select>
          </div>

          <div className="form-field">
            <label htmlFor="pyq-filter-unit">Unit</label>
            <select
              id="pyq-filter-unit"
              value={filterForm.sectionCode}
              onChange={(event) =>
                setFilterForm((current) => ({
                  ...current,
                  sectionCode: event.target.value,
                  topicId: '',
                }))
              }
            >
              <option value="">All units</option>
              {options.sections.map((section) => (
                <option value={section.code} key={section.code}>
                  {section.title}
                </option>
              ))}
            </select>
          </div>

          <div className="form-field">
            <label htmlFor="pyq-filter-topic">Topic</label>
            <select
              id="pyq-filter-topic"
              value={filterForm.topicId}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, topicId: event.target.value }))
              }
            >
              <option value="">All topics</option>
              {filterTopicOptions.map((topic) => (
                <option value={topic.id} key={topic.id}>
                  {topic.code} · {topic.title}
                  {topic.archived ? ' (archived)' : ''}
                </option>
              ))}
            </select>
          </div>

          <div className="form-field">
            <label htmlFor="pyq-filter-status">Status</label>
            <select
              id="pyq-filter-status"
              value={filterForm.status}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, status: event.target.value }))
              }
            >
              <option value="">All outcomes</option>
              {PYQ_STATUSES.map((status) => (
                <option value={status} key={status}>
                  {statusLabel(status)}
                </option>
              ))}
            </select>
          </div>

          <div className="pyq-date-range">
            <DateField
              id="pyq-filter-from"
              label="Attempted from (optional)"
              value={filterForm.fromDate}
              max={filterForm.toDate || undefined}
              onChange={(value) => setFilterForm((current) => ({ ...current, fromDate: value }))}
            />
            <DateField
              id="pyq-filter-to"
              label="Attempted to (optional)"
              value={filterForm.toDate}
              min={filterForm.fromDate || undefined}
              onChange={(value) => setFilterForm((current) => ({ ...current, toDate: value }))}
            />
          </div>

          <div className="form-field pyq-filter-search">
            <label htmlFor="pyq-filter-query">Search</label>
            <input
              id="pyq-filter-query"
              type="search"
              value={filterForm.query}
              maxLength={MAX_QUERY_CHARS}
              onChange={(event) =>
                setFilterForm((current) => ({ ...current, query: event.target.value }))
              }
              placeholder="Reference, session, or notes"
            />
          </div>

          <div className="pyq-filter-actions">
            <button type="submit" className="primary-button">
              Apply filters
            </button>
            <button type="button" className="secondary-button" onClick={resetFilter}>
              Reset
            </button>
          </div>
        </form>

        {optionsError && (
          <p className="save-feedback save-feedback-error" role="alert">
            Filter options could not be loaded: {optionsError}
          </p>
        )}
        {filterError && (
          <p className="save-feedback save-feedback-error" role="alert">
            {filterError}
          </p>
        )}
      </section>

      {feedback && (
        <p
          className={`save-feedback save-feedback-${feedback.tone}`}
          role={feedback.tone === 'error' ? 'alert' : 'status'}
        >
          {feedback.message}
        </p>
      )}

      {editor && (
        <section className="pyq-editor-card" aria-labelledby="pyq-editor-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">{editor.attempt ? 'Editing' : 'New record'}</p>
              <h2 id="pyq-editor-heading">
                {editor.attempt ? 'Edit PYQ attempt' : 'Add a PYQ attempt'}
              </h2>
            </div>
            <button type="button" className="secondary-button" onClick={() => setEditor(null)}>
              Close editor
            </button>
          </div>
          <p className="settings-lede">
            Record only information you already have. Historical attempts remain separate records,
            even when they refer to the same question.
          </p>
          <PyqEditor
            key={
              editor.attempt
                ? `pyq-${editor.attempt.id}-${editorVersion}`
                : `pyq-new-${editorVersion}`
            }
            attempt={editor.attempt}
            options={options}
            today={today}
            onSaved={(saved, wasEditing) => {
              setEditor(null);
              setEditorVersion((version) => version + 1);
              setFeedback({
                tone: 'success',
                message: wasEditing
                  ? `PYQ attempt ${saved.questionReference} updated.`
                  : `PYQ attempt ${saved.questionReference} added.`,
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
          <p className="state-title">Loading PYQ attempts.</p>
          <p className="state-copy">Reading your saved question references and outcomes.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">PYQ attempts could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button type="button" className="primary-button" onClick={reload}>
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && (
        <section className="pyq-history-card" aria-labelledby="pyq-history-heading">
          <div className="card-heading">
            <div>
              <p className="eyebrow">Historical attempts</p>
              <h2 id="pyq-history-heading">Attempts ({attempts.length})</h2>
            </div>
            {activeFilters && <span className="task-query-badge">Filters applied</span>}
          </div>

          {attempts.length === 0 ? (
            <div className="task-empty">
              <p className="task-empty-title">
                {activeFilters ? 'No attempts match these filters' : 'No PYQ attempts recorded yet'}
              </p>
              <p>
                {activeFilters
                  ? 'Reset the filters or add a new attempt using your own question reference.'
                  : 'Add your first attempt, or import a CSV file using the template below.'}
              </p>
            </div>
          ) : (
            <div className="pyq-list">
              {visibleAttempts.map((attempt) => (
                <article className="pyq-row" key={attempt.id}>
                  <div className="pyq-row-main">
                    <div className="pyq-row-heading">
                      <h3>{attempt.questionReference}</h3>
                      <div className="task-badges">
                        <span className="pyq-badge">{attempt.questionYear}</span>
                        <span className="pyq-badge">Part {attempt.part}</span>
                        <span className={`pyq-status pyq-status-${attempt.status}`}>
                          {statusLabel(attempt.status)}
                        </span>
                        {attempt.difficulty && (
                          <span className="pyq-badge">Difficulty {attempt.difficulty}/5</span>
                        )}
                      </div>
                    </div>
                    <p className="pyq-meta">
                      <span>
                        {attempt.topic.code} · {attempt.topic.title}
                      </span>
                      <span>{attempt.topic.sectionTitle}</span>
                      {attempt.examSession && <span>{attempt.examSession}</span>}
                      <span>Attempted {dateLabel(attempt.attemptDate)}</span>
                    </p>
                    <p className="pyq-stats">
                      <span>{marksLabel(attempt)}</span>
                      <span>Time spent {formatStudyDuration(attempt.timeSpentSeconds)}</span>
                    </p>
                    {attempt.notes && <p className="pyq-notes">{attempt.notes}</p>}
                  </div>

                  <div className="pyq-row-actions">
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => {
                        setEditor({ attempt });
                        setPendingDelete(null);
                        setFeedback(null);
                      }}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="danger-button"
                      onClick={() => setPendingDelete(attempt.id)}
                      disabled={deletingId !== null}
                    >
                      Delete
                    </button>
                  </div>

                  {pendingDelete === attempt.id && (
                    <div
                      className="task-delete-confirm"
                      role="alertdialog"
                      aria-labelledby={`delete-pyq-${attempt.id}`}
                    >
                      <p className="task-delete-title" id={`delete-pyq-${attempt.id}`}>
                        Delete this PYQ attempt permanently?
                      </p>
                      <p>
                        The attempt and its notes will be removed. Syllabus progress and study
                        sessions are not affected.
                      </p>
                      <div className="form-footer">
                        <button
                          type="button"
                          className="danger-button"
                          onClick={() => void handleDelete(attempt.id)}
                          disabled={deletingId !== null}
                        >
                          {deletingId === attempt.id ? 'Deleting…' : 'Confirm delete'}
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
            <nav className="history-pagination" aria-label="PYQ history pages">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setPage((current) => Math.max(1, Math.min(current, pageCount) - 1))}
                disabled={safePage === 1}
              >
                Previous page
              </button>
              <span>
                Page {safePage} of {pageCount} · {attempts.length} attempts
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
        <summary>Import PYQ records from CSV</summary>
        <CsvImportPanel
          id="pyq-import"
          title="Import PYQ attempts"
          description="Use the template columns exactly. References must match existing syllabus topic codes. Preview validates every row before anything is written."
          saveTemplate={savePyqCsvTemplate}
          previewImport={previewPyqImport}
          applyImport={applyPyqImport}
          onImported={(result) => {
            setFeedback({
              tone: result.rejected > 0 ? 'error' : 'success',
              message: `PYQ import finished: ${result.imported} imported, ${result.skipped} skipped, ${result.rejected} rejected.`,
            });
            reload();
          }}
        />
      </details>
    </div>
  );
}
