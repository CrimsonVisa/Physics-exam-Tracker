import { type FormEvent, useCallback, useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { toCommandError } from '../lib/commandError';
import { MathText } from './MathText';
import {
  archiveV2Resource,
  importV2ResourceFile,
  linkV2ResourceExam,
  listV2Resources,
  openV2ResourceFile,
  removeV2ResourceFile,
  saveV2Resource,
  updateV2ResourceProgress,
  validateV2ResourcePdf,
  type V2ExamSummary,
  type V2Resource,
} from './api';

type ResourcesScreenProps = {
  exams: V2ExamSummary[];
  initialQuery?: string | null;
};

function titleFromPdfPath(filePath: string): string {
  const fileName = filePath.split(/[\\/]/).pop() ?? '';
  const title = fileName.replace(/\.pdf$/i, '').trim();
  return (title || 'Imported PDF').slice(0, 300);
}

export function ResourcesScreen({ exams, initialQuery = null }: ResourcesScreenProps) {
  const [resources, setResources] = useState<V2Resource[]>([]);
  const [query, setQuery] = useState(initialQuery ?? '');
  const [title, setTitle] = useState('');
  const [url, setUrl] = useState('');
  const [resourceType, setResourceType] = useState('website');
  const [subjectArea, setSubjectArea] = useState('');
  const [source, setSource] = useState('');
  const [description, setDescription] = useState('');
  const [examLinks, setExamLinks] = useState<Record<number, string>>({});
  const [notes, setNotes] = useState<Record<number, string>>({});
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [importingPdf, setImportingPdf] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  function resetResourceForm() {
    setTitle('');
    setUrl('');
    setSubjectArea('');
    setSource('');
    setDescription('');
  }

  const load = useCallback(async (search: string) => {
    try {
      const next = await listV2Resources({
        query: search.trim() || undefined,
        includeArchived: true,
      });
      setResources(next);
      setNotes(
        Object.fromEntries(next.map((resource) => [resource.id, resource.progress.personalNotes])),
      );
      setError(null);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      await load((initialQuery ?? '').trim());
    })();
  }, [load, initialQuery]);

  async function addResource(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await saveV2Resource({
        title,
        description,
        subjectArea,
        url: url.trim() || null,
        resourceType,
        source,
        conceptIds: [],
        examLinks: [],
      });
      resetResourceForm();
      await load(query);
      setNotice('Resource saved.');
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function changeStatus(resource: V2Resource, studyStatus: string) {
    setBusy(true);
    setError(null);
    try {
      const updated = await updateV2ResourceProgress({
        resourceId: resource.id,
        studyStatus,
        personalNotes: notes[resource.id] ?? resource.progress.personalNotes,
      });
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice('Resource progress saved.');
    } catch (updateError) {
      setError(toCommandError(updateError).message);
    } finally {
      setBusy(false);
    }
  }

  async function saveNotes(resource: V2Resource) {
    setBusy(true);
    setError(null);
    try {
      const updated = await updateV2ResourceProgress({
        resourceId: resource.id,
        studyStatus: resource.progress.studyStatus,
        personalNotes: notes[resource.id] ?? '',
      });
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice('Resource notes saved.');
    } catch (updateError) {
      setError(toCommandError(updateError).message);
    } finally {
      setBusy(false);
    }
  }

  async function toggleArchive(resource: V2Resource) {
    setBusy(true);
    setError(null);
    try {
      const updated = await archiveV2Resource({
        resourceId: resource.id,
        archived: !resource.isArchived,
      });
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice(updated.isArchived ? 'Resource archived.' : 'Resource restored.');
    } catch (archiveError) {
      setError(toCommandError(archiveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function linkExam(resource: V2Resource) {
    const examId = Number(examLinks[resource.id] ?? '');
    if (!examId) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const updated = await linkV2ResourceExam({
        resourceId: resource.id,
        link: {
          examId,
          category: 'general',
          priority: 'normal',
          notes: '',
        },
      });
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setExamLinks((current) => ({ ...current, [resource.id]: '' }));
      setNotice('Resource linked to the examination.');
    } catch (linkError) {
      setError(toCommandError(linkError).message);
    } finally {
      setBusy(false);
    }
  }

  async function importPdf() {
    setImportingPdf(true);
    setError(null);
    setNotice(null);
    let selected: string | string[] | null;
    try {
      selected = await open({
        multiple: false,
        directory: false,
        title: 'Import PDF',
        filters: [{ name: 'PDF documents', extensions: ['pdf'] }],
      });
    } catch (pickerError) {
      setError(toCommandError(pickerError).message);
      setImportingPdf(false);
      return;
    }
    if (typeof selected !== 'string') {
      setImportingPdf(false);
      return;
    }

    const fallbackTitle = titleFromPdfPath(selected);
    setBusy(true);
    try {
      try {
        await validateV2ResourcePdf(selected);
      } catch (validationError) {
        setError(`The PDF could not be imported: ${toCommandError(validationError).message}`);
        return;
      }
      const saved = await saveV2Resource({
        title: title.trim() || fallbackTitle,
        description,
        subjectArea,
        url: url.trim() || null,
        resourceType: resourceType === 'website' ? 'book' : resourceType,
        source: source.trim() || 'Local PDF',
        conceptIds: [],
        examLinks: [],
      });
      resetResourceForm();
      try {
        const imported = await importV2ResourceFile({
          resourceId: saved.id,
          filePath: selected,
        });
        await load(query);
        setNotice(
          `Imported ${imported.file?.fileName ?? fallbackTitle} into managed PDF storage. The original file was not changed.`,
        );
      } catch (importError) {
        await load(query);
        setError(
          `The PDF could not be imported: ${toCommandError(importError).message} The resource entry was saved without a PDF; use Attach PDF to retry.`,
        );
      }
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setImportingPdf(false);
      setBusy(false);
    }
  }

  async function openPdf(resource: V2Resource) {
    if (
      !resource.file ||
      resource.file.storageMode !== 'managed' ||
      resource.file.status !== 'ok'
    ) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await openV2ResourceFile(resource.id);
      setNotice(`Opened ${resource.file.fileName} in the default PDF viewer.`);
    } catch (openError) {
      setError(toCommandError(openError).message);
    } finally {
      setBusy(false);
    }
  }

  async function attachPdf(resource: V2Resource) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (typeof selected !== 'string') {
        return;
      }
      const updated = await importV2ResourceFile({
        resourceId: resource.id,
        filePath: selected,
      });
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice('Managed PDF copy created. The original file is not modified.');
    } catch (fileError) {
      setError(toCommandError(fileError).message);
    } finally {
      setBusy(false);
    }
  }

  async function removePdf(resource: V2Resource) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await removeV2ResourceFile(resource.id);
      setResources((current) => current.map((item) => (item.id === updated.id ? updated : item)));
      setNotice('PDF metadata removed. The original file is untouched.');
    } catch (fileError) {
      setError(toCommandError(fileError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page-container v2-resources-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · shared library</p>
        <h1>Resources</h1>
        <p className="page-lede">
          Shared study resources with exam links, personal progress, and managed PDF copies or
          external references. Physics Tracker never uploads, deletes, or rewrites your original
          files.
        </p>
      </header>

      {error && (
        <p className="v2-study-error" role="alert">
          {error}
        </p>
      )}
      {notice && (
        <p className="v2-study-notice" role="status">
          {notice}
        </p>
      )}

      <form
        className="v2-resource-filter"
        onSubmit={(event) => {
          event.preventDefault();
          void load(query);
        }}
      >
        <label className="v2-field">
          <span>Search resources</span>
          <input onChange={(event) => setQuery(event.target.value)} type="search" value={query} />
        </label>
        <button className="secondary-button" type="submit">
          Search
        </button>
      </form>

      <section className="v2-concept-card" aria-labelledby="import-pdf-heading">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">Local files</p>
            <h2 id="import-pdf-heading">Import a PDF</h2>
          </div>
          <button
            className="primary-button"
            disabled={busy || importingPdf}
            onClick={() => void importPdf()}
            type="button"
          >
            {importingPdf ? 'Importing…' : 'Import PDF'}
          </button>
        </div>
        <p className="v2-empty-copy">
          Choose a PDF from this computer. Physics Tracker validates it and stores a managed copy,
          so moving or deleting the original file later does not break the resource.
        </p>
      </section>

      <section className="v2-concept-card" aria-labelledby="add-resource-heading">
        <p className="eyebrow">Catalogue</p>
        <h2 id="add-resource-heading">Add a resource</h2>
        <form className="v2-resource-form" onSubmit={addResource}>
          <label className="v2-field">
            <span>Title</span>
            <input
              maxLength={300}
              onChange={(event) => setTitle(event.target.value)}
              required
              value={title}
            />
          </label>
          <label className="v2-field">
            <span>Resource type</span>
            <select onChange={(event) => setResourceType(event.target.value)} value={resourceType}>
              <option value="book">Book</option>
              <option value="notes">Notes</option>
              <option value="video">Video</option>
              <option value="paper">Paper</option>
              <option value="website">Website</option>
              <option value="other">Other</option>
            </select>
          </label>
          <label className="v2-field">
            <span>URL</span>
            <input
              onChange={(event) => setUrl(event.target.value)}
              placeholder="https://…"
              type="url"
              value={url}
            />
          </label>
          <label className="v2-field">
            <span>Subject area</span>
            <input onChange={(event) => setSubjectArea(event.target.value)} value={subjectArea} />
          </label>
          <label className="v2-field">
            <span>Source</span>
            <input onChange={(event) => setSource(event.target.value)} value={source} />
          </label>
          <label className="v2-field v2-resource-description">
            <span>Description</span>
            <textarea
              maxLength={4000}
              onChange={(event) => setDescription(event.target.value)}
              rows={3}
              value={description}
            />
          </label>
          <button className="primary-button" disabled={busy} type="submit">
            Save resource
          </button>
        </form>
      </section>

      <section aria-labelledby="resource-list-heading" className="v2-concept-card">
        <div className="v2-section-heading">
          <div>
            <p className="eyebrow">Library</p>
            <h2 id="resource-list-heading">Saved resources</h2>
          </div>
          <p>
            {resources.length} resource{resources.length === 1 ? '' : 's'}
          </p>
        </div>
        {loading ? (
          <p className="v2-empty-copy">Loading resources…</p>
        ) : resources.length === 0 ? (
          <p className="v2-empty-panel-copy">
            No resources have been saved yet. Add a URL resource or import a PDF.
          </p>
        ) : (
          <ul className="v2-resource-list">
            {resources.map((resource) => (
              <li
                className={`v2-resource-row${resource.isArchived ? ' v2-resource-archived' : ''}`}
                key={resource.id}
              >
                <div className="v2-session-heading">
                  <strong>{resource.title}</strong>
                  <span className="v2-status-badge v2-status-inactive">
                    {resource.resourceType}
                  </span>
                  {resource.isArchived && (
                    <span className="v2-status-badge v2-status-inactive">Archived</span>
                  )}
                </div>
                {resource.url && (
                  <a href={resource.url} rel="noreferrer" target="_blank">
                    {resource.url}
                  </a>
                )}
                {resource.description && (
                  <p className="v2-empty-copy">
                    <MathText text={resource.description} />
                  </p>
                )}
                <div className="v2-allocation-chips">
                  {resource.examLinks.map((link) => (
                    <span className="v2-allocation-chip" key={link.examId}>
                      {link.examShortName} · {link.category}
                    </span>
                  ))}
                  {resource.file && (
                    <span className="v2-allocation-chip">
                      PDF {'\u00b7'} {resource.file.fileName} {'\u00b7'}{' '}
                      {resource.file.storageMode === 'managed' ? 'managed copy' : 'external'}{' '}
                      {'\u00b7'} {resource.file.status}
                    </span>
                  )}
                </div>
                <div className="v2-resource-controls">
                  <label className="v2-field">
                    <span>Study status</span>
                    <select
                      onChange={(event) => void changeStatus(resource, event.target.value)}
                      value={resource.progress.studyStatus}
                    >
                      <option value="not_started">Not started</option>
                      <option value="studying">Studying</option>
                      <option value="completed">Completed</option>
                      <option value="skipped">Skipped</option>
                    </select>
                  </label>
                  <label className="v2-field v2-resource-notes">
                    <span>Personal notes</span>
                    <input
                      onChange={(event) =>
                        setNotes((current) => ({
                          ...current,
                          [resource.id]: event.target.value,
                        }))
                      }
                      value={notes[resource.id] ?? ''}
                    />
                  </label>
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => void saveNotes(resource)}
                    type="button"
                  >
                    Save notes
                  </button>
                  <label className="v2-field">
                    <span>Link exam</span>
                    <select
                      onChange={(event) =>
                        setExamLinks((current) => ({
                          ...current,
                          [resource.id]: event.target.value,
                        }))
                      }
                      value={examLinks[resource.id] ?? ''}
                    >
                      <option value="">Choose…</option>
                      {exams.map((exam) => (
                        <option key={exam.id} value={exam.id}>
                          {exam.shortName}
                        </option>
                      ))}
                    </select>
                  </label>
                  <button
                    className="secondary-button"
                    disabled={busy || !examLinks[resource.id]}
                    onClick={() => void linkExam(resource)}
                    type="button"
                  >
                    Link exam
                  </button>
                  {resource.file?.storageMode === 'managed' && resource.file.status === 'ok' && (
                    <button
                      className="primary-button"
                      disabled={busy}
                      onClick={() => void openPdf(resource)}
                      type="button"
                    >
                      Open PDF
                    </button>
                  )}
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => void attachPdf(resource)}
                    type="button"
                  >
                    {resource.file
                      ? resource.file.status === 'missing'
                        ? 'Relink PDF'
                        : 'Replace PDF'
                      : 'Attach PDF'}
                  </button>
                  {resource.file && (
                    <button
                      className="secondary-button v2-danger-button"
                      disabled={busy}
                      onClick={() => void removePdf(resource)}
                      type="button"
                    >
                      Remove PDF
                    </button>
                  )}
                  <button
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => void toggleArchive(resource)}
                    type="button"
                  >
                    {resource.isArchived ? 'Restore' : 'Archive'}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
