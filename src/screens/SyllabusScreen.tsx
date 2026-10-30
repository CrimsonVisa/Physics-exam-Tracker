import { openUrl } from '@tauri-apps/plugin-opener';
import { type FormEvent, type ReactNode, useEffect, useState } from 'react';
import { formatDateLong } from '../lib/dateOnly';
import { toCommandError } from '../lib/commandError';
import {
  getSyllabusCatalog,
  getSyllabusSummary,
  searchSyllabusTopics,
  TOPIC_CATEGORIES,
  TOPIC_PRIORITIES,
  TOPIC_STATUSES,
  updateTopicProgress,
  type SectionSummary,
  type SyllabusCatalog,
  type SyllabusFilter,
  type SyllabusSummary,
  type SyllabusTopic,
  type TopicCategory,
  type TopicPriority,
  type TopicProgress,
  type TopicProgressInput,
  type TopicStatus,
} from '../lib/syllabusApi';

type FilterState = {
  query: string;
  sectionCode: string;
  category: string;
  status: string;
  priority: string;
};

type DraftValues = {
  status: TopicStatus;
  priority: TopicPriority;
  confidence: number;
  notes: string;
};

type Draft = {
  topicId: number;
  values: DraftValues;
};

type EditorField = keyof DraftValues;

type EditorErrors = Partial<Record<EditorField, string>>;

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

const EMPTY_FILTERS: FilterState = {
  query: '',
  sectionCode: '',
  category: '',
  status: '',
  priority: '',
};

const CONFIDENCE_VALUES = [1, 2, 3, 4, 5] as const;

const FORM_IDS: Record<EditorField, string> = {
  status: 'topic-status',
  priority: 'topic-priority',
  confidence: 'topic-confidence',
  notes: 'topic-notes',
};

const FILTER_IDS: Record<keyof FilterState, string> = {
  query: 'syllabus-search',
  sectionCode: 'syllabus-section-filter',
  category: 'syllabus-category-filter',
  status: 'syllabus-status-filter',
  priority: 'syllabus-priority-filter',
};

function isTopicStatus(value: string): value is TopicStatus {
  return (TOPIC_STATUSES as readonly string[]).includes(value);
}

function isTopicPriority(value: string): value is TopicPriority {
  return (TOPIC_PRIORITIES as readonly string[]).includes(value);
}

function isEditorField(value: string): value is EditorField {
  return value === 'status' || value === 'priority' || value === 'confidence' || value === 'notes';
}

function draftFromProgress(progress: TopicProgress): DraftValues {
  return {
    status: progress.status,
    priority: progress.priority,
    confidence: progress.confidence,
    notes: progress.notes,
  };
}

function buildFilter(filters: FilterState): SyllabusFilter {
  const filter: SyllabusFilter = {};
  const query = filters.query.trim();

  if (query) {
    filter.query = query;
  }
  if (filters.sectionCode) {
    filter.sectionCode = filters.sectionCode;
  }
  if (filters.category) {
    filter.category = filters.category as TopicCategory;
  }
  if (filters.status) {
    filter.status = filters.status as TopicStatus;
  }
  if (filters.priority) {
    filter.priority = filters.priority as TopicPriority;
  }

  return filter;
}

function draftIsDirty(draft: DraftValues, saved: TopicProgress): boolean {
  return (
    draft.status !== saved.status ||
    draft.priority !== saved.priority ||
    draft.confidence !== saved.confidence ||
    draft.notes !== saved.notes
  );
}

function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}

function clampPercent(value: number): number {
  return Math.min(100, Math.max(0, value));
}

function statusClass(status: TopicStatus): string {
  return `status-${status.toLowerCase().replaceAll(' ', '-')}`;
}

function categoryClass(category: TopicCategory): string {
  return `category-${category.toLowerCase().replaceAll(' ', '-')}`;
}

function priorityClass(priority: TopicPriority): string {
  return `priority-${priority.toLowerCase().replaceAll(' ', '-')}`;
}

function formatSavedAt(value: string | null): string | null {
  if (!value) {
    return null;
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function ExternalLink({ href, children }: { href: string; children: ReactNode }) {
  const [failed, setFailed] = useState(false);

  return (
    <>
      <a
        href={href}
        onClick={(event) => {
          event.preventDefault();
          setFailed(false);
          void openUrl(href).catch(() => setFailed(true));
        }}
      >
        {children}
      </a>
      {failed && (
        <span className="source-link-error" role="alert">
          Could not open this link in your browser.
        </span>
      )}
    </>
  );
}

function FieldError({ id, message }: { id: string; message?: string }) {
  if (!message) {
    return null;
  }

  return (
    <p className="field-error" id={id} role="alert">
      {message}
    </p>
  );
}

function SummaryCard({ summary, isFetching }: { summary: SyllabusSummary; isFetching: boolean }) {
  const counts = summary.statusCounts;

  return (
    <section className="syllabus-summary-card" aria-labelledby="syllabus-summary-heading">
      <div className="summary-topline">
        <div>
          <p className="eyebrow">Approximate tracker metric</p>
          <h2 id="syllabus-summary-heading">Syllabus progress</h2>
        </div>
        <p className="summary-percent">
          <span>{formatPercent(summary.approximateProgress)}</span>
          <small>
            {summary.totalTopics} active {summary.totalTopics === 1 ? 'topic' : 'topics'}
          </small>
        </p>
      </div>

      <div
        className="progress-track progress-track-large"
        role="progressbar"
        aria-label="Approximate syllabus progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(summary.approximateProgress)}
      >
        <span style={{ width: `${clampPercent(summary.approximateProgress)}%` }} />
      </div>

      <dl className="summary-counts">
        <div className="status-count status-count-not-started">
          <dt>Not started</dt>
          <dd>{counts.notStarted}</dd>
        </div>
        <div className="status-count status-count-learning">
          <dt>Learning</dt>
          <dd>{counts.learning}</dd>
        </div>
        <div className="status-count status-count-revised">
          <dt>Revised</dt>
          <dd>{counts.revised}</dd>
        </div>
        <div className="status-count status-count-confident">
          <dt>Confident</dt>
          <dd>{counts.confident}</dd>
        </div>
      </dl>

      <div className="summary-notes">
        <p>
          <strong>How this is calculated:</strong> every active topic has equal weight. Status
          scores are Not started 0, Learning 0.5, Revised 0.75 and Confident 1. This is an
          approximate tracker metric, not an official CSIR score or a marks prediction.
        </p>
        <p>
          <strong>Priority labels:</strong> Low, Normal, High and High-Yield are your own planning
          labels, not official CSIR weightings.
        </p>
      </div>

      <p className="summary-refresh" role="status" aria-live="polite">
        {isFetching ? 'Refreshing syllabus data…' : 'Local SQLite · saved on this PC'}
      </p>

      <details className="section-breakdown">
        <summary>Section breakdown</summary>
        <ul>
          {summary.sections.map((section) => (
            <SectionBreakdownRow key={section.code} section={section} />
          ))}
        </ul>
      </details>
    </section>
  );
}

function SectionBreakdownRow({ section }: { section: SectionSummary }) {
  return (
    <li>
      <div className="section-breakdown-heading">
        <span>{section.title}</span>
        <span>
          {formatPercent(section.approximateProgress)} · {section.totalTopics} topics
        </span>
      </div>
      <div className="progress-track progress-track-small" aria-hidden="true">
        <span style={{ width: `${clampPercent(section.approximateProgress)}%` }} />
      </div>
    </li>
  );
}

function SourcePanel({ catalog }: { catalog: SyllabusCatalog }) {
  return (
    <section className="source-panel" aria-labelledby="syllabus-sources-heading">
      <div className="card-heading">
        <div>
          <p className="eyebrow">Provenance</p>
          <h2 id="syllabus-sources-heading">Official sources</h2>
        </div>
        <span className="local-badge">Checked 29 Oct 2026</span>
      </div>

      <p className="source-lede">
        The CSIR document is titled <strong>Proposed Syllabus for Physical Sciences</strong>. It
        organizes content into <strong>Core</strong> and <strong>Advanced</strong> categories. Those
        categories are separate from the exam question paper&apos;s Parts A, B and C.
      </p>

      <div className="source-grid">
        {catalog.sources.map((source) => (
          <article className="source-card" key={source.code}>
            <h3>{source.title}</h3>
            <p className="source-meta">
              {source.publisher} · Verified {formatDateLong(source.verifiedOn) ?? source.verifiedOn}
            </p>
            <p className="source-copy">{source.notes}</p>
            <p className="source-links">
              <ExternalLink href={source.url}>Official document</ExternalLink>
              {source.landingUrl && (
                <ExternalLink href={source.landingUrl}>Source page</ExternalLink>
              )}
            </p>
          </article>
        ))}
      </div>

      <p className="source-footnote">
        General Aptitude topics use the NTA December 2026 bulletin wording. Titles are transcribed
        with typographic normalization; no topic weightings or guaranteed appearances are implied.
      </p>
    </section>
  );
}

export default function SyllabusScreen() {
  const [catalog, setCatalog] = useState<SyllabusCatalog | null>(null);
  const [topics, setTopics] = useState<SyllabusTopic[]>([]);
  const [summary, setSummary] = useState<SyllabusSummary | null>(null);
  const [filters, setFilters] = useState<FilterState>(EMPTY_FILTERS);
  const [refreshKey, setRefreshKey] = useState(0);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [isFetching, setIsFetching] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedTopicId, setSelectedTopicId] = useState<number | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [editorErrors, setEditorErrors] = useState<EditorErrors>({});
  const [isSaving, setIsSaving] = useState(false);
  const [saveFeedback, setSaveFeedback] = useState<Feedback>(null);

  useEffect(() => {
    let cancelled = false;
    const delay = filters.query.trim() ? 200 : 0;

    const timer = window.setTimeout(() => {
      setIsFetching(true);

      Promise.all([
        getSyllabusCatalog(),
        getSyllabusSummary(),
        searchSyllabusTopics(buildFilter(filters)),
      ])
        .then(([nextCatalog, nextSummary, nextTopics]) => {
          if (cancelled) {
            return;
          }

          setCatalog(nextCatalog);
          setSummary(nextSummary);
          setTopics(nextTopics);
          setLoadError(null);
          setHasLoaded(true);
        })
        .catch((error: unknown) => {
          if (!cancelled) {
            setLoadError(toCommandError(error).message);
          }
        })
        .finally(() => {
          if (!cancelled) {
            setIsFetching(false);
          }
        });
    }, delay);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [filters, refreshKey]);

  const resolvedSelectedTopicId =
    selectedTopicId !== null && topics.some((topic) => topic.id === selectedTopicId)
      ? selectedTopicId
      : (topics[0]?.id ?? null);
  const selectedTopic = topics.find((topic) => topic.id === resolvedSelectedTopicId) ?? null;
  const activeDraft = selectedTopic
    ? draft?.topicId === selectedTopic.id
      ? draft.values
      : draftFromProgress(selectedTopic.progress)
    : null;
  const dirty = Boolean(
    activeDraft && selectedTopic && draftIsDirty(activeDraft, selectedTopic.progress),
  );

  const activeFilterCount =
    [filters.sectionCode, filters.category, filters.status, filters.priority].filter(Boolean)
      .length + (filters.query.trim() ? 1 : 0);

  const savedAt = selectedTopic ? formatSavedAt(selectedTopic.progress.updatedAt) : null;

  function resetEditorFeedback() {
    setEditorErrors({});
    setSaveFeedback(null);
  }

  function handleFilterChange(field: keyof FilterState, value: string) {
    setFilters((current) => ({ ...current, [field]: value }));
    resetEditorFeedback();
  }

  function handleClearFilters() {
    setFilters(EMPTY_FILTERS);
    resetEditorFeedback();
  }

  function handleSelectTopic(topic: SyllabusTopic) {
    setSelectedTopicId(topic.id);
    setDraft({ topicId: topic.id, values: draftFromProgress(topic.progress) });
    setEditorErrors({});
    setSaveFeedback(null);
  }

  function updateDraft<K extends EditorField>(field: K, value: DraftValues[K]) {
    if (!selectedTopic) {
      return;
    }

    setDraft((current) => {
      const base =
        current?.topicId === selectedTopic.id
          ? current.values
          : draftFromProgress(selectedTopic.progress);
      const values: DraftValues = { ...base, [field]: value };

      return { topicId: selectedTopic.id, values };
    });
    setEditorErrors((current) => {
      if (!current[field]) {
        return current;
      }

      const next = { ...current };
      delete next[field];
      return next;
    });
    setSaveFeedback(null);
  }

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!selectedTopic || !activeDraft || isSaving || !dirty) {
      return;
    }

    const nextErrors: EditorErrors = {};
    if (
      !Number.isInteger(activeDraft.confidence) ||
      activeDraft.confidence < 1 ||
      activeDraft.confidence > 5
    ) {
      nextErrors.confidence = 'Confidence must be a whole number from 1 to 5.';
    }
    if ([...activeDraft.notes].length > 2000) {
      nextErrors.notes = 'Notes must be 2000 characters or fewer.';
    }

    setEditorErrors(nextErrors);

    if (Object.keys(nextErrors).length > 0) {
      setSaveFeedback({ tone: 'error', message: 'Check the highlighted fields and try again.' });
      return;
    }

    const input: TopicProgressInput = {
      status: activeDraft.status,
      priority: activeDraft.priority,
      confidence: activeDraft.confidence,
      notes: activeDraft.notes,
    };
    const topicId = selectedTopic.id;
    const topicCode = selectedTopic.code;

    setIsSaving(true);
    setSaveFeedback(null);

    try {
      const saved = await updateTopicProgress(topicId, input);

      setTopics((current) =>
        current.map((topic) => (topic.id === topicId ? { ...topic, progress: saved } : topic)),
      );
      setDraft((current) =>
        current?.topicId === topicId ? { topicId, values: draftFromProgress(saved) } : current,
      );
      setSaveFeedback({ tone: 'success', message: `Progress saved for ${topicCode}.` });

      try {
        setSummary(await getSyllabusSummary());
      } catch {
        // The progress write succeeded; keep the last summary if it cannot refresh.
      }

      const statusMatches = !filters.status || filters.status === saved.status;
      const priorityMatches = !filters.priority || filters.priority === saved.priority;
      if (!statusMatches || !priorityMatches) {
        setRefreshKey((key) => key + 1);
      }
    } catch (error: unknown) {
      const commandError = toCommandError(error);

      if (commandError.field && isEditorField(commandError.field)) {
        setEditorErrors({ [commandError.field]: commandError.message });
      }

      setSaveFeedback({ tone: 'error', message: commandError.message });
    } finally {
      setIsSaving(false);
    }
  }

  function handleRetry() {
    setLoadError(null);
    setRefreshKey((key) => key + 1);
  }

  const initialLoading = !hasLoaded && !loadError;

  return (
    <div className="page-container syllabus-page">
      <header className="page-header">
        <p className="eyebrow">Syllabus</p>
        <h1>Physical Sciences syllabus</h1>
        <p className="page-lede">
          Work through the CSIR Physical Sciences units and record a personal status for each topic.
          Catalog definitions and your progress are stored separately in the local SQLite database.
        </p>
      </header>

      {initialLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading syllabus…</p>
          <p className="state-copy">
            Reading the official-source catalog and any progress saved on this PC.
          </p>
        </section>
      )}

      {!hasLoaded && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Syllabus could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button type="button" className="primary-button" onClick={handleRetry}>
            Retry
          </button>
        </section>
      )}

      {hasLoaded && (
        <>
          {loadError && (
            <section className="state-card state-card-error inline-error" role="alert">
              <p className="state-title">The syllabus could not refresh</p>
              <p className="state-copy">{loadError}</p>
              <button type="button" className="secondary-button" onClick={handleRetry}>
                Retry
              </button>
            </section>
          )}

          {summary && <SummaryCard summary={summary} isFetching={isFetching} />}

          <section className="syllabus-controls" aria-labelledby="syllabus-filters-heading">
            <div className="card-heading">
              <div>
                <p className="eyebrow">Find a topic</p>
                <h2 id="syllabus-filters-heading">Search and filters</h2>
              </div>
              <button
                type="button"
                className="secondary-button"
                onClick={handleClearFilters}
                disabled={activeFilterCount === 0}
              >
                Clear filters
              </button>
            </div>

            <div className="filter-grid">
              <div className="form-field filter-search">
                <label htmlFor={FILTER_IDS.query}>Search topics</label>
                <input
                  id={FILTER_IDS.query}
                  type="search"
                  value={filters.query}
                  onChange={(event) => handleFilterChange('query', event.target.value)}
                  placeholder="Topic name or code"
                  autoComplete="off"
                />
              </div>

              <div className="form-field">
                <label htmlFor={FILTER_IDS.sectionCode}>Filter by section</label>
                <select
                  id={FILTER_IDS.sectionCode}
                  value={filters.sectionCode}
                  onChange={(event) => handleFilterChange('sectionCode', event.target.value)}
                >
                  <option value="">All sections</option>
                  {catalog?.sections.map((section) => (
                    <option key={section.code} value={section.code}>
                      {section.title}
                    </option>
                  ))}
                </select>
              </div>

              <div className="form-field">
                <label htmlFor={FILTER_IDS.category}>Filter by category</label>
                <select
                  id={FILTER_IDS.category}
                  value={filters.category}
                  onChange={(event) => handleFilterChange('category', event.target.value)}
                >
                  <option value="">All categories</option>
                  {TOPIC_CATEGORIES.map((category) => (
                    <option key={category} value={category}>
                      {category}
                    </option>
                  ))}
                </select>
              </div>

              <div className="form-field">
                <label htmlFor={FILTER_IDS.status}>Filter by status</label>
                <select
                  id={FILTER_IDS.status}
                  value={filters.status}
                  onChange={(event) => handleFilterChange('status', event.target.value)}
                >
                  <option value="">All statuses</option>
                  {TOPIC_STATUSES.map((status) => (
                    <option key={status} value={status}>
                      {status}
                    </option>
                  ))}
                </select>
              </div>

              <div className="form-field">
                <label htmlFor={FILTER_IDS.priority}>Filter by priority</label>
                <select
                  id={FILTER_IDS.priority}
                  value={filters.priority}
                  onChange={(event) => handleFilterChange('priority', event.target.value)}
                >
                  <option value="">All priorities</option>
                  {TOPIC_PRIORITIES.map((priority) => (
                    <option key={priority} value={priority}>
                      {priority}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          </section>

          <div className="syllabus-workspace">
            <section className="topic-list-card" aria-labelledby="topic-list-heading">
              <div className="card-heading topic-list-heading">
                <div>
                  <p className="eyebrow">Catalog</p>
                  <h2 id="topic-list-heading">Topics</h2>
                </div>
                <span className="result-count" role="status" aria-live="polite">
                  {isFetching ? 'Updating…' : `${topics.length} matching`}
                </span>
              </div>

              {topics.length === 0 ? (
                <div className="empty-state">
                  <p className="empty-title">No topics match these filters</p>
                  <p className="empty-copy">
                    Try a different search or clear the filters to see the full syllabus.
                  </p>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={handleClearFilters}
                    disabled={activeFilterCount === 0}
                  >
                    Clear filters
                  </button>
                </div>
              ) : (
                <ul className="topic-list" id="topic-results">
                  {topics.map((topic) => {
                    const selected = topic.id === resolvedSelectedTopicId;

                    return (
                      <li key={topic.id}>
                        <button
                          type="button"
                          className={`topic-row${selected ? ' topic-row-selected' : ''}`}
                          aria-pressed={selected}
                          onClick={() => handleSelectTopic(topic)}
                        >
                          <span className="topic-row-top">
                            <code>{topic.code}</code>
                            <span className={`badge ${statusClass(topic.progress.status)}`}>
                              {topic.progress.status}
                            </span>
                          </span>
                          <span className="topic-row-title">{topic.title}</span>
                          <span className="topic-row-meta">
                            <span className="topic-row-section">{topic.sectionTitle}</span>
                            <span className={`badge ${categoryClass(topic.category)}`}>
                              {topic.category}
                            </span>
                            {topic.progress.priority !== 'Normal' && (
                              <span className={`badge ${priorityClass(topic.progress.priority)}`}>
                                {topic.progress.priority}
                              </span>
                            )}
                          </span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </section>

            <section className="topic-detail-card" aria-labelledby="topic-detail-heading">
              {selectedTopic && activeDraft ? (
                <>
                  <div className="topic-detail-header">
                    <div>
                      <p className="eyebrow">{selectedTopic.sectionTitle}</p>
                      <h2 id="topic-detail-heading">{selectedTopic.title}</h2>
                      <p className="topic-code-line">
                        <code>{selectedTopic.code}</code>
                        <span className={`badge ${categoryClass(selectedTopic.category)}`}>
                          {selectedTopic.category}
                        </span>
                      </p>
                    </div>
                  </div>

                  <p className="topic-source">
                    <span>{selectedTopic.sourceReference}</span>
                    <ExternalLink href={selectedTopic.sourceUrl}>
                      {selectedTopic.sourceTitle}
                    </ExternalLink>
                  </p>

                  <p className="topic-save-state" role="status">
                    {selectedTopic.progress.recorded
                      ? savedAt
                        ? `Last saved ${savedAt}`
                        : 'Saved locally'
                      : 'No saved progress yet. Defaults are shown until you save.'}
                  </p>

                  <form className="topic-progress-form" onSubmit={handleSave} noValidate>
                    <div
                      className={`form-field${editorErrors.status ? ' form-field-invalid' : ''}`}
                    >
                      <label htmlFor={FORM_IDS.status}>Status</label>
                      <select
                        id={FORM_IDS.status}
                        value={activeDraft.status}
                        onChange={(event) => {
                          if (isTopicStatus(event.target.value)) {
                            updateDraft('status', event.target.value);
                          }
                        }}
                        aria-invalid={editorErrors.status ? true : undefined}
                        aria-describedby={
                          editorErrors.status
                            ? 'topic-status-hint topic-status-error'
                            : 'topic-status-hint'
                        }
                        disabled={isSaving}
                      >
                        {TOPIC_STATUSES.map((status) => (
                          <option key={status} value={status}>
                            {status}
                          </option>
                        ))}
                      </select>
                      <p className="field-hint" id="topic-status-hint">
                        Your own study state, not an official result.
                      </p>
                      <FieldError id="topic-status-error" message={editorErrors.status} />
                    </div>

                    <div
                      className={`form-field${editorErrors.priority ? ' form-field-invalid' : ''}`}
                    >
                      <label htmlFor={FORM_IDS.priority}>Priority</label>
                      <select
                        id={FORM_IDS.priority}
                        value={activeDraft.priority}
                        onChange={(event) => {
                          if (isTopicPriority(event.target.value)) {
                            updateDraft('priority', event.target.value);
                          }
                        }}
                        aria-invalid={editorErrors.priority ? true : undefined}
                        aria-describedby={
                          editorErrors.priority
                            ? 'topic-priority-hint topic-priority-error'
                            : 'topic-priority-hint'
                        }
                        disabled={isSaving}
                      >
                        {TOPIC_PRIORITIES.map((priority) => (
                          <option key={priority} value={priority}>
                            {priority}
                          </option>
                        ))}
                      </select>
                      <p className="field-hint" id="topic-priority-hint">
                        A personal planning label. High-Yield is not an official CSIR weighting.
                      </p>
                      <FieldError id="topic-priority-error" message={editorErrors.priority} />
                    </div>

                    <div
                      className={`form-field${editorErrors.confidence ? ' form-field-invalid' : ''}`}
                    >
                      <label htmlFor={FORM_IDS.confidence}>Confidence (1–5)</label>
                      <select
                        id={FORM_IDS.confidence}
                        value={activeDraft.confidence}
                        onChange={(event) => updateDraft('confidence', Number(event.target.value))}
                        aria-invalid={editorErrors.confidence ? true : undefined}
                        aria-describedby={
                          editorErrors.confidence
                            ? 'topic-confidence-hint topic-confidence-error'
                            : 'topic-confidence-hint'
                        }
                        disabled={isSaving}
                      >
                        {CONFIDENCE_VALUES.map((value) => (
                          <option key={value} value={value}>
                            {value}
                          </option>
                        ))}
                      </select>
                      <p className="field-hint" id="topic-confidence-hint">
                        1 means low confidence; 5 means high confidence.
                      </p>
                      <FieldError id="topic-confidence-error" message={editorErrors.confidence} />
                    </div>

                    <div className={`form-field${editorErrors.notes ? ' form-field-invalid' : ''}`}>
                      <label htmlFor={FORM_IDS.notes}>Notes</label>
                      <textarea
                        id={FORM_IDS.notes}
                        value={activeDraft.notes}
                        onChange={(event) => updateDraft('notes', event.target.value)}
                        rows={5}
                        maxLength={2000}
                        aria-invalid={editorErrors.notes ? true : undefined}
                        aria-describedby={
                          editorErrors.notes
                            ? 'topic-notes-hint topic-notes-error'
                            : 'topic-notes-hint'
                        }
                        disabled={isSaving}
                        placeholder="Optional: what you covered, what to revisit, or a page reference."
                      />
                      <p className="field-hint" id="topic-notes-hint">
                        {[...activeDraft.notes].length} of 2000 characters. Stored only on this PC.
                      </p>
                      <FieldError id="topic-notes-error" message={editorErrors.notes} />
                    </div>

                    <div className="form-footer">
                      <button
                        type="submit"
                        className="primary-button"
                        disabled={!dirty || isSaving}
                      >
                        {isSaving ? 'Saving…' : 'Save progress'}
                      </button>
                      <span className="dirty-hint" aria-live="polite">
                        {isSaving
                          ? 'Writing to SQLite…'
                          : dirty
                            ? 'Unsaved changes'
                            : selectedTopic.progress.recorded
                              ? 'All changes saved'
                              : 'Defaults shown'}
                      </span>
                    </div>

                    {saveFeedback && (
                      <p
                        className={`save-feedback save-feedback-${saveFeedback.tone}`}
                        role={saveFeedback.tone === 'error' ? 'alert' : 'status'}
                      >
                        {saveFeedback.message}
                      </p>
                    )}
                  </form>
                </>
              ) : (
                <div className="empty-state">
                  <h2 id="topic-detail-heading">Select a topic</h2>
                  <p className="empty-copy">
                    Choose a topic from the list to record your status, priority, confidence and
                    notes.
                  </p>
                </div>
              )}
            </section>
          </div>

          {catalog && <SourcePanel catalog={catalog} />}
        </>
      )}
    </div>
  );
}
