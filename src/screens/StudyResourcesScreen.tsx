import { type FormEvent, useEffect, useMemo, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import { toCommandError } from '../lib/commandError';
import {
  createManualResource,
  getResourceCatalogue,
  getResourceLocalFile,
  getResourceOverview,
  importResourcePdf,
  markResourceOpened,
  refreshResourceCatalogue,
  relinkResourcePdf,
  removeResourcePdf,
  replaceResourcePdf,
  updateResourceProgress,
  type DiscoverySummary,
  type LocalResourceFile,
  type ManualResourceInput,
  type Resource,
  type ResourceCategory,
  type ResourceOverview,
  type ResourceStudyStatus,
  type ResourceUnit,
} from '../lib/resourcesApi';

const CATEGORY_OPTIONS: { value: ResourceCategory; label: string }[] = [
  { value: 'handwritten_notes', label: 'Handwritten notes' },
  { value: 'classroom_notes', label: 'Classroom notes' },
  { value: 'assignments', label: 'Assignments' },
  { value: 'formula_sheets', label: 'Formula sheets' },
  { value: 'previous_year_papers', label: 'Previous-year papers' },
  { value: 'important_topics', label: 'Important topics' },
  { value: 'recommended_books', label: 'Recommended books' },
  { value: 'nptel_courses', label: 'NPTEL courses' },
  { value: 'youtube_playlists', label: 'YouTube playlists' },
  { value: 'other', label: 'Other educational resources' },
];

const UNIT_OPTIONS: ResourceUnit[] = [
  'General Aptitude',
  'Mathematical Methods of Physics',
  'Classical Mechanics',
  'Electromagnetic Theory',
  'Quantum Mechanics',
  'Thermodynamic and Statistical Physics',
  'Electronics and Experimental Methods',
  'Atomic & Molecular Physics',
  'Condensed Matter Physics',
  'Nuclear and Particle Physics',
  'All Units',
  'Unmapped',
];

const STATUS_OPTIONS: { value: ResourceStudyStatus; label: string }[] = [
  { value: 'not_started', label: 'Not started' },
  { value: 'studying', label: 'Studying' },
  { value: 'completed', label: 'Completed' },
  { value: 'skipped', label: 'Skipped' },
];

const AVAILABILITY_OPTIONS = [
  { value: 'all', label: 'All availability' },
  { value: 'local', label: 'Offline copy' },
  { value: 'local_problem', label: 'Local copy missing/changed' },
  { value: 'online', label: 'Online only' },
  { value: 'removed', label: 'Removed from source' },
  { value: 'unverified', label: 'Manual / unverified' },
];

const SOURCE_OPTIONS = [
  { value: 'all', label: 'All sources' },
  { value: 'csirnetphysics.in', label: 'csirnetphysics.in' },
  { value: 'Google Drive', label: 'Google Drive' },
  { value: 'NPTEL', label: 'NPTEL' },
  { value: 'YouTube', label: 'YouTube' },
  { value: 'Manual', label: 'Manual entries' },
  { value: 'Other website', label: 'Other websites' },
];

const PAGE_SIZE = 12;

type ManualForm = {
  title: string;
  description: string;
  category: ResourceCategory;
  unit: ResourceUnit;
  topicCode: string;
  originalUrl: string;
  sourcePageUrl: string;
};

type ManualField = keyof ManualForm;

type Feedback = {
  tone: 'success' | 'error';
  message: string;
} | null;

function categoryLabel(category: ResourceCategory): string {
  return CATEGORY_OPTIONS.find((option) => option.value === category)?.label ?? category;
}

function statusLabel(status: ResourceStudyStatus): string {
  return STATUS_OPTIONS.find((option) => option.value === status)?.label ?? status;
}

function formatTimestamp(value: string | null): string {
  if (!value) {
    return 'Not yet';
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

function formatBytes(value: number): string {
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function sourceLabel(resource: Resource): string {
  if (!resource.sourcePageUrl) {
    return 'Manual';
  }
  try {
    const host = new URL(resource.originalUrl).hostname.replace(/^www\./, '');
    if (host.includes('drive.google.com') || host.includes('docs.google.com')) {
      return 'Google Drive';
    }
    if (host.includes('nptel.ac.in')) {
      return 'NPTEL';
    }
    if (host.includes('youtube.com') || host.includes('youtu.be')) {
      return 'YouTube';
    }
    if (host === 'csirnetphysics.in') {
      return 'csirnetphysics.in';
    }
  } catch {
    return 'Other website';
  }
  return 'Other website';
}

type LocalFileCondition = 'offline' | 'missing' | 'changed';

function localFileCondition(localFile: NonNullable<Resource['localFile']>): LocalFileCondition {
  if (!localFile.exists) {
    return 'missing';
  }
  if (!localFile.sizeMatches) {
    return 'changed';
  }
  return 'offline';
}

function availabilityLabel(resource: Resource): string {
  if (resource.localFile) {
    const condition = localFileCondition(resource.localFile);
    if (condition === 'missing') {
      return 'Local copy missing';
    }
    if (condition === 'changed') {
      return 'Local copy changed';
    }
    return 'Offline copy';
  }
  if (resource.availability === 'online') {
    return 'Online only';
  }
  if (resource.availability === 'removed') {
    return 'Removed from source';
  }
  if (resource.availability === 'invalid') {
    return 'Invalid link';
  }
  return 'Manual / unverified';
}

function availabilityTone(resource: Resource): string {
  if (resource.localFile) {
    const condition = localFileCondition(resource.localFile);
    if (condition === 'missing') {
      return 'resource-availability-missing';
    }
    if (condition === 'changed') {
      return 'resource-availability-changed';
    }
    return 'resource-availability-local';
  }
  if (resource.availability === 'online') {
    return 'resource-availability-online';
  }
  if (resource.availability === 'removed') {
    return 'resource-availability-removed';
  }
  return 'resource-availability-manual';
}

function resourceMatchesSource(resource: Resource, source: string): boolean {
  return source === 'all' || sourceLabel(resource) === source;
}

function resourceMatchesAvailability(resource: Resource, availability: string): boolean {
  if (availability === 'all') {
    return true;
  }
  if (availability === 'local') {
    return resource.localFile !== null && localFileCondition(resource.localFile) === 'offline';
  }
  if (availability === 'local_problem') {
    return resource.localFile !== null && localFileCondition(resource.localFile) !== 'offline';
  }
  if (availability === 'online') {
    return resource.localFile === null && resource.availability === 'online';
  }
  return resource.localFile === null && resource.availability === availability;
}

function initialManualForm(): ManualForm {
  return {
    title: '',
    description: '',
    category: 'other',
    unit: 'Unmapped',
    topicCode: '',
    originalUrl: '',
    sourcePageUrl: '',
  };
}

function validateManualForm(form: ManualForm): Partial<Record<ManualField, string>> {
  const errors: Partial<Record<ManualField, string>> = {};
  if (!form.title.trim()) {
    errors.title = 'Enter a resource title.';
  }
  if (form.title.trim().length > 300) {
    errors.title = 'Resource title must be 300 characters or fewer.';
  }
  if (form.description.length > 2000) {
    errors.description = 'Description must be 2000 characters or fewer.';
  }
  try {
    const url = new URL(form.originalUrl.trim());
    if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) {
      errors.originalUrl = 'Enter a complete http:// or https:// URL.';
    }
  } catch {
    errors.originalUrl = 'Enter a complete http:// or https:// URL.';
  }
  if (form.sourcePageUrl.trim()) {
    try {
      const url = new URL(form.sourcePageUrl.trim());
      if (!['http:', 'https:'].includes(url.protocol) || !url.hostname) {
        errors.sourcePageUrl = 'Enter a complete http:// or https:// URL.';
      }
    } catch {
      errors.sourcePageUrl = 'Enter a complete http:// or https:// URL.';
    }
  }
  return errors;
}

function ResourceCard({
  resource,
  onChanged,
  onLocalChanged,
  onOpened,
}: {
  resource: Resource;
  onChanged: (resource: Resource) => void;
  onLocalChanged: (resourceId: number, localFile: LocalResourceFile | null) => void;
  onOpened: (resourceId: number, openedAt: string) => void;
}) {
  const [status, setStatus] = useState<ResourceStudyStatus>(resource.studyStatus);
  const [notes, setNotes] = useState(resource.personalNotes);
  const [savingStatus, setSavingStatus] = useState(false);
  const [savingNotes, setSavingNotes] = useState(false);
  const [importing, setImporting] = useState(false);
  const [removingLocal, setRemovingLocal] = useState(false);
  const [relinking, setRelinking] = useState(false);
  const [replacing, setReplacing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  async function handleStatusChange(next: ResourceStudyStatus) {
    const previous = status;
    setStatus(next);
    setSavingStatus(true);
    setError(null);
    try {
      const updated = await updateResourceProgress(resource.id, next, notes);
      onChanged(updated);
      setFeedback(`Study status saved as ${statusLabel(next)}.`);
    } catch (caught) {
      setStatus(previous);
      setError(toCommandError(caught).message);
    } finally {
      setSavingStatus(false);
    }
  }

  async function handleSaveNotes() {
    setSavingNotes(true);
    setError(null);
    try {
      const updated = await updateResourceProgress(resource.id, status, notes);
      onChanged(updated);
      setFeedback('Personal notes saved.');
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setSavingNotes(false);
    }
  }

  async function handleOpenOriginal() {
    setError(null);
    try {
      await openUrl(resource.originalUrl);
      try {
        await markResourceOpened(resource.id);
        onOpened(resource.id, new Date().toISOString());
      } catch {
        // Opening the link is still useful if the timestamp cannot be stored.
      }
    } catch {
      setError('The original link could not be opened in the system browser.');
    }
  }

  async function handleOpenSourcePage() {
    if (!resource.sourcePageUrl) {
      return;
    }
    setError(null);
    try {
      await openUrl(resource.sourcePageUrl);
    } catch {
      setError('The source page could not be opened in the system browser.');
    }
  }

  async function handleImportPdf() {
    setError(null);
    setFeedback(null);
    let selected: string | string[] | null;
    try {
      selected = await open({
        multiple: false,
        directory: false,
        title: 'Choose an authorized PDF',
        filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
      });
    } catch {
      setError('The file picker could not be opened.');
      return;
    }
    if (typeof selected !== 'string') {
      return;
    }
    let directory: string | string[] | null;
    try {
      directory = await open({
        multiple: false,
        directory: true,
        title: 'Choose the local PDF library folder',
      });
    } catch {
      setError('The folder picker could not be opened.');
      return;
    }
    if (typeof directory !== 'string') {
      return;
    }
    setImporting(true);
    try {
      const localFile = await importResourcePdf(resource.id, selected, directory);
      onLocalChanged(resource.id, localFile);
      setFeedback('PDF imported for offline use.');
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setImporting(false);
    }
  }

  async function handleOpenLocal() {
    if (!resource.localFile) {
      return;
    }
    setError(null);
    let checked: LocalResourceFile;
    try {
      checked = await getResourceLocalFile(resource.id);
      onLocalChanged(resource.id, checked);
    } catch (caught) {
      setError(toCommandError(caught).message);
      return;
    }
    if (!checked.exists) {
      setError('The local PDF is missing. Re-link the moved file or replace the local copy.');
      return;
    }
    if (!checked.sizeMatches) {
      setError(
        'The local PDF has changed size. Re-link the original file or replace the local copy.',
      );
      return;
    }
    try {
      await openPath(checked.filePath);
      try {
        await markResourceOpened(resource.id);
        onOpened(resource.id, new Date().toISOString());
      } catch {
        // The local file still opens if timestamp persistence fails.
      }
    } catch {
      setError('The local PDF could not be opened in the default reader.');
    }
  }

  async function handleRelinkPdf() {
    if (!resource.localFile) {
      return;
    }
    setError(null);
    setFeedback(null);
    let selected: string | string[] | null;
    try {
      selected = await open({
        multiple: false,
        directory: false,
        title: 'Choose the moved PDF',
        filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
      });
    } catch {
      setError('The file picker could not be opened.');
      return;
    }
    if (typeof selected !== 'string') {
      return;
    }
    setRelinking(true);
    try {
      const localFile = await relinkResourcePdf(resource.id, selected);
      onLocalChanged(resource.id, localFile);
      setFeedback('The moved PDF was re-linked after SHA-256 verification.');
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setRelinking(false);
    }
  }

  async function handleReplacePdf() {
    if (
      !resource.localFile ||
      !window.confirm(
        'Replace the local PDF reference with another authorized PDF? The old file will not be deleted or overwritten.',
      )
    ) {
      return;
    }
    setError(null);
    setFeedback(null);
    let selected: string | string[] | null;
    try {
      selected = await open({
        multiple: false,
        directory: false,
        title: 'Choose a replacement PDF',
        filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
      });
    } catch {
      setError('The file picker could not be opened.');
      return;
    }
    if (typeof selected !== 'string') {
      return;
    }
    let directory: string | string[] | null;
    try {
      directory = await open({
        multiple: false,
        directory: true,
        title: 'Choose the local PDF library folder',
      });
    } catch {
      setError('The folder picker could not be opened.');
      return;
    }
    if (typeof directory !== 'string') {
      return;
    }
    setReplacing(true);
    try {
      const localFile = await replaceResourcePdf(resource.id, selected, directory);
      onLocalChanged(resource.id, localFile);
      setFeedback(
        'The replacement PDF was copied and linked. The previous file was left untouched.',
      );
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setReplacing(false);
    }
  }

  async function handleRemoveLocal() {
    if (
      !resource.localFile ||
      !window.confirm(
        'Remove the local PDF copy? The resource catalogue entry and study progress will be kept.',
      )
    ) {
      return;
    }
    setRemovingLocal(true);
    setError(null);
    try {
      await removeResourcePdf(resource.id);
      onLocalChanged(resource.id, null);
      setFeedback('Local PDF removed. The catalogue entry remains available.');
    } catch (caught) {
      setError(toCommandError(caught).message);
    } finally {
      setRemovingLocal(false);
    }
  }

  const localCondition = resource.localFile ? localFileCondition(resource.localFile) : null;

  return (
    <article className="resource-card" aria-labelledby={`resource-title-${resource.id}`}>
      <div className="resource-card-heading">
        <div>
          <p className="eyebrow">{categoryLabel(resource.category)}</p>
          <h3 id={`resource-title-${resource.id}`}>{resource.title}</h3>
        </div>
        <span className={`resource-availability ${availabilityTone(resource)}`}>
          {availabilityLabel(resource)}
        </span>
      </div>

      {resource.description && <p className="resource-description">{resource.description}</p>}

      <dl className="resource-meta">
        <div>
          <dt>Syllabus unit</dt>
          <dd>{resource.unit}</dd>
        </div>
        <div>
          <dt>Topic code</dt>
          <dd>{resource.topicCode ?? 'Not mapped to a single topic'}</dd>
        </div>
        <div>
          <dt>Source</dt>
          <dd>{sourceLabel(resource)}</dd>
        </div>
        <div>
          <dt>Last opened</dt>
          <dd>{formatTimestamp(resource.lastOpenedAt)}</dd>
        </div>
      </dl>

      {resource.localFile ? (
        <div className={`resource-local-file resource-local-file-${localCondition}`}>
          <p>
            <strong>
              {localCondition === 'missing'
                ? 'Local copy missing:'
                : localCondition === 'changed'
                  ? 'Local copy changed:'
                  : 'Offline PDF:'}
            </strong>{' '}
            {resource.localFile.fileName} ({formatBytes(resource.localFile.sizeBytes)})
          </p>
          <p className="resource-local-path" title={resource.localFile.filePath}>
            SHA-256 {resource.localFile.sha256}
          </p>
          {localCondition === 'missing' && (
            <p className="resource-local-warning">
              The registered file is not present at its saved path. The catalogue entry and study
              history are retained.
            </p>
          )}
          {localCondition === 'changed' && (
            <p className="resource-local-warning">
              The file at the saved path has a different size. It is not treated as the verified
              offline copy.
            </p>
          )}
          <div className="resource-local-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleOpenLocal()}
            >
              Open local PDF
            </button>
            {localCondition === 'missing' && (
              <button
                type="button"
                className="secondary-button"
                onClick={() => void handleRelinkPdf()}
                disabled={relinking}
              >
                {relinking ? 'Checking…' : 'Re-link moved file'}
              </button>
            )}
            {localCondition !== 'offline' && (
              <button
                type="button"
                className="secondary-button"
                onClick={() => void handleReplacePdf()}
                disabled={replacing}
              >
                {replacing ? 'Copying…' : 'Replace with another PDF'}
              </button>
            )}
            <button
              type="button"
              className="danger-button"
              onClick={() => void handleRemoveLocal()}
              disabled={removingLocal}
            >
              {removingLocal ? 'Removing…' : 'Remove local copy'}
            </button>
          </div>
          <p className="resource-local-hint">
            Re-linking verifies the stored SHA-256. Replacing copies a new file without deleting or
            overwriting the old one.
          </p>
        </div>
      ) : (
        <p className="resource-online-note">
          {resource.availability === 'removed'
            ? 'The source page no longer lists this link. It remains here with your study history.'
            : resource.availability === 'online'
              ? 'Online link: internet access is required until you import an authorized PDF.'
              : 'Manually registered link. Availability has not been verified.'}
        </p>
      )}

      <div className="resource-card-actions">
        <label className="resource-status-field">
          <span>Study status</span>
          <select
            value={status}
            onChange={(event) => void handleStatusChange(event.target.value as ResourceStudyStatus)}
            disabled={savingStatus}
            aria-label={`Study status for ${resource.title}`}
          >
            {STATUS_OPTIONS.map((option) => (
              <option value={option.value} key={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        {savingStatus && <span className="resource-saving">Saving status…</span>}
        <div className="resource-action-buttons">
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleOpenOriginal()}
          >
            Open original
          </button>
          {!resource.localFile && (
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleImportPdf()}
              disabled={importing}
            >
              {importing ? 'Importing…' : 'Import authorized PDF'}
            </button>
          )}
          {resource.sourcePageUrl && (
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleOpenSourcePage()}
            >
              Source page
            </button>
          )}
        </div>
      </div>

      {feedback && (
        <p className="save-feedback save-feedback-success" role="status">
          {feedback}
        </p>
      )}
      {error && (
        <p className="save-feedback save-feedback-error" role="alert">
          {error}
        </p>
      )}

      <details className="resource-notes">
        <summary>
          Personal notes
          {resource.personalNotes ? ' · saved' : ''}
        </summary>
        <label htmlFor={`resource-notes-${resource.id}`}>Notes for this resource</label>
        <textarea
          id={`resource-notes-${resource.id}`}
          value={notes}
          maxLength={4000}
          onChange={(event) => setNotes(event.target.value)}
          placeholder="Record what you covered, page numbers, or questions to revisit."
        />
        <div className="resource-notes-footer">
          <span>{notes.length}/4000 characters</span>
          <button
            type="button"
            className="secondary-button"
            onClick={() => void handleSaveNotes()}
            disabled={savingNotes}
          >
            {savingNotes ? 'Saving…' : 'Save notes'}
          </button>
        </div>
      </details>
    </article>
  );
}

export default function StudyResourcesScreen() {
  const [resources, setResources] = useState<Resource[]>([]);
  const [overview, setOverview] = useState<ResourceOverview | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshFeedback, setRefreshFeedback] = useState<DiscoverySummary | null>(null);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [categoryFilter, setCategoryFilter] = useState('all');
  const [unitFilter, setUnitFilter] = useState('all');
  const [sourceFilter, setSourceFilter] = useState('all');
  const [availabilityFilter, setAvailabilityFilter] = useState('all');
  const [statusFilter, setStatusFilter] = useState('all');
  const [page, setPage] = useState(0);
  const [manualForm, setManualForm] = useState<ManualForm>(initialManualForm);
  const [manualErrors, setManualErrors] = useState<Partial<Record<ManualField, string>>>({});
  const [manualFeedback, setManualFeedback] = useState<Feedback>(null);
  const [isSavingManual, setIsSavingManual] = useState(false);

  async function load() {
    setIsLoading(true);
    setLoadError(null);
    try {
      const [loadedResources, loadedOverview] = await Promise.all([
        getResourceCatalogue(),
        getResourceOverview(),
      ]);
      setResources(loadedResources);
      setOverview(loadedOverview);
      setPage(0);
    } catch (error: unknown) {
      setLoadError(toCommandError(error).message);
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    let cancelled = false;

    Promise.all([getResourceCatalogue(), getResourceOverview()])
      .then(([loadedResources, loadedOverview]) => {
        if (!cancelled) {
          setResources(loadedResources);
          setOverview(loadedOverview);
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
  }, []);

  const filtered = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return resources.filter((resource) => {
      if (
        normalizedQuery &&
        ![
          resource.title,
          resource.description,
          resource.unit,
          resource.topicCode ?? '',
          categoryLabel(resource.category),
        ]
          .join(' ')
          .toLowerCase()
          .includes(normalizedQuery)
      ) {
        return false;
      }
      if (categoryFilter !== 'all' && resource.category !== categoryFilter) {
        return false;
      }
      if (unitFilter !== 'all' && resource.unit !== unitFilter) {
        return false;
      }
      if (!resourceMatchesSource(resource, sourceFilter)) {
        return false;
      }
      if (!resourceMatchesAvailability(resource, availabilityFilter)) {
        return false;
      }
      return statusFilter === 'all' || resource.studyStatus === statusFilter;
    });
  }, [
    resources,
    query,
    categoryFilter,
    unitFilter,
    sourceFilter,
    availabilityFilter,
    statusFilter,
  ]);

  async function handleRefresh() {
    if (isRefreshing) {
      return;
    }
    setIsRefreshing(true);
    setRefreshFeedback(null);
    setRefreshError(null);
    try {
      const summary = await refreshResourceCatalogue();
      setRefreshFeedback(summary);
      await load();
    } catch (error: unknown) {
      setRefreshError(toCommandError(error).message);
    } finally {
      setIsRefreshing(false);
    }
  }

  async function handleManualSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSavingManual) {
      return;
    }
    setManualFeedback(null);
    const errors = validateManualForm(manualForm);
    setManualErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }
    setIsSavingManual(true);
    try {
      const input: ManualResourceInput = {
        title: manualForm.title.trim(),
        description: manualForm.description.trim(),
        category: manualForm.category,
        unit: manualForm.unit,
        topicCode: manualForm.topicCode.trim() || null,
        originalUrl: manualForm.originalUrl.trim(),
        sourcePageUrl: manualForm.sourcePageUrl.trim() || null,
      };
      await createManualResource(input);
      setManualForm(initialManualForm());
      setManualErrors({});
      setManualFeedback({
        tone: 'success',
        message: 'Manual resource added to the local catalogue.',
      });
      await load();
    } catch (error: unknown) {
      const commandError = toCommandError(error);
      const field = commandError.field as ManualField | undefined;
      if (
        field &&
        [
          'title',
          'description',
          'category',
          'unit',
          'topicCode',
          'originalUrl',
          'sourcePageUrl',
        ].includes(field)
      ) {
        setManualErrors({ [field]: commandError.message });
      }
      setManualFeedback({ tone: 'error', message: commandError.message });
    } finally {
      setIsSavingManual(false);
    }
  }

  function handleResourceChanged(updated: Resource) {
    setResources((current) =>
      current.map((resource) => (resource.id === updated.id ? updated : resource)),
    );
    void getResourceOverview()
      .then(setOverview)
      .catch(() => {
        // The resource row is saved even if the summary cannot refresh immediately.
      });
  }

  function handleLocalChanged(resourceId: number, localFile: LocalResourceFile | null) {
    setResources((current) =>
      current.map((resource) =>
        resource.id === resourceId ? { ...resource, localFile } : resource,
      ),
    );
    void getResourceOverview()
      .then(setOverview)
      .catch(() => {});
  }

  function handleOpened(resourceId: number, openedAt: string) {
    setResources((current) =>
      current.map((resource) =>
        resource.id === resourceId ? { ...resource, lastOpenedAt: openedAt } : resource,
      ),
    );
  }

  function clearFilters() {
    setQuery('');
    setCategoryFilter('all');
    setUnitFilter('all');
    setSourceFilter('all');
    setAvailabilityFilter('all');
    setStatusFilter('all');
    setPage(0);
  }

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const visible = filtered.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  return (
    <div className="page-container resources-page">
      <header className="page-header">
        <p className="eyebrow">Study library</p>
        <h1>Study resources</h1>
        <p className="page-lede">
          Browse materials curated by the CSIR-NET Physics website, open original links, and keep
          personal study progress beside them. The app stores link metadata and your notes only.
        </p>
      </header>

      <p className="resource-disclaimer" role="note">
        <strong>Third-party links:</strong> resources remain the property of their original authors
        and the CSIR-NET Physics website. This app does not host, redistribute, or bypass Google
        Drive permissions. Download only material you are authorized to store.
      </p>

      {overview && (
        <section className="resource-summary-grid" aria-label="Resource summary">
          <div className="resource-summary-card">
            <span>Total resources</span>
            <strong>{overview.total}</strong>
            <small>{overview.onlineCount} listed online</small>
          </div>
          <div className="resource-summary-card">
            <span>Offline PDFs</span>
            <strong>{overview.localCount}</strong>
            <small>Stored in your chosen folder</small>
          </div>
          <div className="resource-summary-card">
            <span>In progress</span>
            <strong>{overview.studyingCount}</strong>
            <small>{overview.completedCount} completed</small>
          </div>
          <div className="resource-summary-card">
            <span>Last refresh</span>
            <strong className="resource-summary-date">
              {formatTimestamp(overview.lastDiscovery?.finishedAt ?? null)}
            </strong>
            <small>{overview.lastDiscovery?.status ?? 'Not refreshed yet'}</small>
          </div>
        </section>
      )}

      <section className="resource-toolbar-card" aria-labelledby="resource-filter-heading">
        <div className="resource-toolbar-heading">
          <div>
            <p className="eyebrow">Catalogue</p>
            <h2 id="resource-filter-heading">Find a resource</h2>
          </div>
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleRefresh()}
            disabled={isRefreshing}
          >
            {isRefreshing ? 'Refreshing catalogue…' : 'Refresh catalogue'}
          </button>
        </div>
        <p className="resource-refresh-note">
          Refresh only when needed. The app reads the six known source pages and their public
          resource lists, respects rate limits, and keeps previous results if a page is unavailable.
        </p>

        <div className="resource-filter-grid">
          <div className="form-field resource-search-field">
            <label htmlFor="resource-search">Search resources</label>
            <input
              id="resource-search"
              type="search"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setPage(0);
              }}
              placeholder="Title, topic, unit, or description"
            />
          </div>
          <div className="form-field">
            <label htmlFor="resource-category-filter">Filter by resource type</label>
            <select
              id="resource-category-filter"
              value={categoryFilter}
              onChange={(event) => {
                setCategoryFilter(event.target.value);
                setPage(0);
              }}
            >
              <option value="all">All resource types</option>
              {CATEGORY_OPTIONS.map((option) => (
                <option value={option.value} key={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="resource-unit-filter">Filter by syllabus unit</label>
            <select
              id="resource-unit-filter"
              value={unitFilter}
              onChange={(event) => {
                setUnitFilter(event.target.value);
                setPage(0);
              }}
            >
              <option value="all">All units</option>
              {UNIT_OPTIONS.map((unit) => (
                <option value={unit} key={unit}>
                  {unit}
                </option>
              ))}
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="resource-source-filter">Filter by source website</label>
            <select
              id="resource-source-filter"
              value={sourceFilter}
              onChange={(event) => {
                setSourceFilter(event.target.value);
                setPage(0);
              }}
            >
              {SOURCE_OPTIONS.map((option) => (
                <option value={option.value} key={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="resource-availability-filter">Filter by availability</label>
            <select
              id="resource-availability-filter"
              value={availabilityFilter}
              onChange={(event) => {
                setAvailabilityFilter(event.target.value);
                setPage(0);
              }}
            >
              {AVAILABILITY_OPTIONS.map((option) => (
                <option value={option.value} key={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="resource-status-filter">Filter by study status</label>
            <select
              id="resource-status-filter"
              value={statusFilter}
              onChange={(event) => {
                setStatusFilter(event.target.value);
                setPage(0);
              }}
            >
              <option value="all">All study statuses</option>
              {STATUS_OPTIONS.map((option) => (
                <option value={option.value} key={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        {isRefreshing && (
          <p className="resource-busy-note" role="status" aria-live="polite">
            Refreshing the catalogue in the background. Existing resources stay available while the
            source pages are checked.
          </p>
        )}
        {refreshFeedback && (
          <p
            className={`save-feedback ${
              refreshFeedback.status === 'blocked' || refreshFeedback.status === 'failed'
                ? 'save-feedback-error'
                : refreshFeedback.status === 'partial'
                  ? 'save-feedback-warning'
                  : 'save-feedback-success'
            }`}
            role="status"
          >
            {refreshFeedback.message}
          </p>
        )}
        {refreshError && (
          <p className="save-feedback save-feedback-error" role="alert">
            {refreshError}
          </p>
        )}
      </section>

      <details className="resource-manual-card">
        <summary>Add a resource manually</summary>
        <p>
          Use this when discovery is blocked or you have an authorized resource that is not listed.
          Only the link and your own metadata are stored.
        </p>
        <form className="resource-manual-form" onSubmit={handleManualSubmit} noValidate>
          <div className={`form-field${manualErrors.title ? ' form-field-invalid' : ''}`}>
            <label htmlFor="manual-resource-title">Resource title</label>
            <input
              id="manual-resource-title"
              value={manualForm.title}
              onChange={(event) => {
                setManualForm((current) => ({ ...current, title: event.target.value }));
                setManualErrors((current) => ({ ...current, title: undefined }));
              }}
              aria-invalid={manualErrors.title ? true : undefined}
              aria-describedby={manualErrors.title ? 'manual-resource-title-error' : undefined}
              required
            />
            {manualErrors.title && (
              <p className="field-error" id="manual-resource-title-error" role="alert">
                {manualErrors.title}
              </p>
            )}
          </div>

          <div className={`form-field${manualErrors.originalUrl ? ' form-field-invalid' : ''}`}>
            <label htmlFor="manual-resource-url">Original resource URL</label>
            <input
              id="manual-resource-url"
              type="url"
              value={manualForm.originalUrl}
              onChange={(event) => {
                setManualForm((current) => ({ ...current, originalUrl: event.target.value }));
                setManualErrors((current) => ({ ...current, originalUrl: undefined }));
              }}
              placeholder="https://drive.google.com/…"
              aria-invalid={manualErrors.originalUrl ? true : undefined}
              aria-describedby={manualErrors.originalUrl ? 'manual-resource-url-error' : undefined}
              required
            />
            {manualErrors.originalUrl && (
              <p className="field-error" id="manual-resource-url-error" role="alert">
                {manualErrors.originalUrl}
              </p>
            )}
          </div>

          <div className="resource-manual-row">
            <div className="form-field">
              <label htmlFor="manual-resource-category">Resource type</label>
              <select
                id="manual-resource-category"
                value={manualForm.category}
                onChange={(event) =>
                  setManualForm((current) => ({
                    ...current,
                    category: event.target.value as ResourceCategory,
                  }))
                }
              >
                {CATEGORY_OPTIONS.map((option) => (
                  <option value={option.value} key={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label htmlFor="manual-resource-unit">Syllabus unit</label>
              <select
                id="manual-resource-unit"
                value={manualForm.unit}
                onChange={(event) =>
                  setManualForm((current) => ({
                    ...current,
                    unit: event.target.value as ResourceUnit,
                  }))
                }
              >
                {UNIT_OPTIONS.map((unit) => (
                  <option value={unit} key={unit}>
                    {unit}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div className="resource-manual-row">
            <div className={`form-field${manualErrors.topicCode ? ' form-field-invalid' : ''}`}>
              <label htmlFor="manual-resource-topic">Topic code (optional)</label>
              <input
                id="manual-resource-topic"
                value={manualForm.topicCode}
                onChange={(event) => {
                  setManualForm((current) => ({ ...current, topicCode: event.target.value }));
                  setManualErrors((current) => ({ ...current, topicCode: undefined }));
                }}
                placeholder="For example QM-C01"
                aria-invalid={manualErrors.topicCode ? true : undefined}
              />
              {manualErrors.topicCode && (
                <p className="field-error" role="alert">
                  {manualErrors.topicCode}
                </p>
              )}
            </div>
            <div className={`form-field${manualErrors.sourcePageUrl ? ' form-field-invalid' : ''}`}>
              <label htmlFor="manual-resource-source-page">Source page URL (optional)</label>
              <input
                id="manual-resource-source-page"
                type="url"
                value={manualForm.sourcePageUrl}
                onChange={(event) => {
                  setManualForm((current) => ({ ...current, sourcePageUrl: event.target.value }));
                  setManualErrors((current) => ({ ...current, sourcePageUrl: undefined }));
                }}
                placeholder="https://www.csirnetphysics.in/…"
                aria-invalid={manualErrors.sourcePageUrl ? true : undefined}
              />
              {manualErrors.sourcePageUrl && (
                <p className="field-error" role="alert">
                  {manualErrors.sourcePageUrl}
                </p>
              )}
            </div>
          </div>

          <div className="form-field">
            <label htmlFor="manual-resource-description">Description (optional)</label>
            <textarea
              id="manual-resource-description"
              value={manualForm.description}
              maxLength={2000}
              onChange={(event) =>
                setManualForm((current) => ({ ...current, description: event.target.value }))
              }
            />
            <p className="field-hint">{manualForm.description.length}/2000 characters.</p>
          </div>

          <div className="form-footer">
            <button type="submit" className="primary-button" disabled={isSavingManual}>
              {isSavingManual ? 'Saving…' : 'Add resource'}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={() => {
                setManualForm(initialManualForm());
                setManualErrors({});
                setManualFeedback(null);
              }}
            >
              Clear form
            </button>
          </div>
          {manualFeedback && (
            <p
              className={`save-feedback save-feedback-${manualFeedback.tone}`}
              role={manualFeedback.tone === 'error' ? 'alert' : 'status'}
            >
              {manualFeedback.message}
            </p>
          )}
        </form>
      </details>

      {isLoading && (
        <section className="state-card" role="status" aria-live="polite">
          <p className="state-title">Loading study resources.</p>
          <p className="state-copy">Reading the local resource catalogue and study progress.</p>
        </section>
      )}

      {!isLoading && loadError && (
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Study resources could not be loaded</p>
          <p className="state-copy">{loadError}</p>
          <button type="button" className="primary-button" onClick={() => void load()}>
            Retry
          </button>
        </section>
      )}

      {!isLoading && !loadError && resources.length === 0 && (
        <section className="resource-empty" aria-label="No resources">
          <h2>No resources discovered yet</h2>
          <p>
            Refresh the catalogue to read the six known CSIR-NET Physics source pages, or add a
            resource manually. Discovery never downloads files automatically.
          </p>
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleRefresh()}
            disabled={isRefreshing}
          >
            {isRefreshing ? 'Refreshing…' : 'Refresh catalogue'}
          </button>
        </section>
      )}

      {!isLoading && !loadError && resources.length > 0 && (
        <section className="resource-results" aria-labelledby="resource-results-heading">
          <div className="resource-results-heading">
            <div>
              <p className="eyebrow">Local library</p>
              <h2 id="resource-results-heading">
                {filtered.length} {filtered.length === 1 ? 'resource' : 'resources'}
              </h2>
            </div>
            <p className="resource-results-count">
              {filtered.length === 0
                ? '0 resources'
                : `Showing ${page * PAGE_SIZE + 1}–${Math.min((page + 1) * PAGE_SIZE, filtered.length)} of ${filtered.length}`}
            </p>
          </div>

          {filtered.length === 0 ? (
            <div className="resource-empty">
              <h3>No resources match these filters</h3>
              <p>Clear the filters or adjust the search to see the catalogue again.</p>
              <button type="button" className="secondary-button" onClick={clearFilters}>
                Clear filters
              </button>
            </div>
          ) : (
            <>
              <div className="resource-grid">
                {visible.map((resource) => (
                  <ResourceCard
                    key={resource.id}
                    resource={resource}
                    onChanged={handleResourceChanged}
                    onLocalChanged={handleLocalChanged}
                    onOpened={handleOpened}
                  />
                ))}
              </div>
              <div className="resource-pagination" aria-label="Resource pages">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setPage((current) => Math.max(0, current - 1))}
                  disabled={page === 0}
                >
                  Previous page
                </button>
                <span>
                  Page {page + 1} of {totalPages}
                </span>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setPage((current) => Math.min(totalPages - 1, current + 1))}
                  disabled={page >= totalPages - 1}
                >
                  Next page
                </button>
              </div>
            </>
          )}
        </section>
      )}
    </div>
  );
}
