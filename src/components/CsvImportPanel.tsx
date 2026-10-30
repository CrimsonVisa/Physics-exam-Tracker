import { type ChangeEvent, useRef, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import type { ImportPreview, ImportResult, ImportRow } from '../lib/csvImportApi';

const MAX_FILE_BYTES = 5_000_000;

type CsvImportPanelProps = {
  id: string;
  title: string;
  description: string;
  saveTemplate: () => Promise<string>;
  previewImport: (csv: string) => Promise<ImportPreview>;
  applyImport: (csv: string) => Promise<ImportResult>;
  onImported: (result: ImportResult) => void;
};

function rowStatusLabel(status: ImportRow['status']): string {
  switch (status) {
    case 'imported':
      return 'Imported';
    case 'valid':
      return 'Ready';
    case 'duplicate':
      return 'Duplicate';
    default:
      return 'Rejected';
  }
}

function ImportRowsTable({ rows }: { rows: ImportRow[] }) {
  if (rows.length === 0) {
    return <p className="import-empty">The file has no data rows.</p>;
  }

  return (
    <div className="import-table-wrap">
      <table className="import-table">
        <thead>
          <tr>
            <th scope="col">Row</th>
            <th scope="col">Status</th>
            <th scope="col">Record</th>
            <th scope="col">Details</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={`${row.rowNumber}-${row.status}`} className={`import-row-${row.status}`}>
              <td>{row.rowNumber}</td>
              <td>
                <span className={`import-status import-status-${row.status}`}>
                  {rowStatusLabel(row.status)}
                </span>
              </td>
              <td>{row.summary || 'No readable record summary'}</td>
              <td>
                {row.errors.length === 0 ? (
                  '—'
                ) : (
                  <ul className="import-errors">
                    {row.errors.map((error, index) => (
                      <li key={`${row.rowNumber}-${error.field}-${index}`}>
                        <strong>{error.field}:</strong> {error.message}
                      </li>
                    ))}
                  </ul>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function CsvImportPanel({
  id,
  title,
  description,
  saveTemplate,
  previewImport,
  applyImport,
  onImported,
}: CsvImportPanelProps) {
  const fileInput = useRef<HTMLInputElement>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [csvText, setCsvText] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [templatePath, setTemplatePath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    setPreview(null);
    setResult(null);
    setError(null);

    if (!file) {
      setFileName(null);
      setCsvText(null);
      return;
    }

    if (file.size > MAX_FILE_BYTES) {
      setFileName(file.name);
      setCsvText(null);
      setError('CSV files must be 5 MB or smaller.');
      return;
    }

    try {
      const bytes = await file.arrayBuffer();
      const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
      setFileName(file.name);
      setCsvText(text);
    } catch {
      setFileName(file.name);
      setCsvText(null);
      setError('The selected file is not valid UTF-8 text. Save it as UTF-8 CSV and try again.');
    }
  }

  function resetImport() {
    setFileName(null);
    setCsvText(null);
    setPreview(null);
    setResult(null);
    setError(null);
    setTemplatePath(null);
    if (fileInput.current) {
      fileInput.current.value = '';
    }
  }

  async function handlePreview() {
    if (!csvText || isPreviewing || isImporting) {
      return;
    }

    setError(null);
    setResult(null);
    setIsPreviewing(true);

    try {
      setPreview(await previewImport(csvText));
    } catch (caught: unknown) {
      setPreview(null);
      setError(toCommandError(caught).message);
    } finally {
      setIsPreviewing(false);
    }
  }

  async function handleApply() {
    if (!csvText || !preview || preview.validRows === 0 || isImporting || isPreviewing) {
      return;
    }

    setError(null);
    setIsImporting(true);

    try {
      const imported = await applyImport(csvText);
      setResult(imported);
      setPreview(null);
      onImported(imported);
    } catch (caught: unknown) {
      setError(toCommandError(caught).message);
    } finally {
      setIsImporting(false);
    }
  }

  async function handleDownloadTemplate() {
    if (isDownloading) {
      return;
    }

    setError(null);
    setTemplatePath(null);
    setIsDownloading(true);

    try {
      setTemplatePath(await saveTemplate());
    } catch (caught: unknown) {
      setError(toCommandError(caught).message);
    } finally {
      setIsDownloading(false);
    }
  }

  return (
    <section className="import-card" aria-labelledby={`${id}-heading`}>
      <div className="card-heading">
        <div>
          <p className="eyebrow">CSV import</p>
          <h2 id={`${id}-heading`}>{title}</h2>
        </div>
        <button
          type="button"
          className="secondary-button"
          onClick={() => void handleDownloadTemplate()}
          disabled={isDownloading}
        >
          {isDownloading ? 'Saving…' : 'Save template to Downloads'}
        </button>
      </div>

      <p className="settings-lede">{description}</p>

      {templatePath && (
        <p className="import-template-note" role="status">
          Header-only template saved to <code>{templatePath}</code>
        </p>
      )}

      <div className="import-file-row">
        <div className="form-field">
          <label htmlFor={`${id}-file`}>CSV file</label>
          <input
            ref={fileInput}
            id={`${id}-file`}
            type="file"
            accept=".csv,text/csv"
            onChange={(event) => void handleFile(event)}
            disabled={isPreviewing || isImporting}
          />
          {fileName && <p className="field-hint">Selected: {fileName}</p>}
        </div>

        <div className="import-file-actions">
          <button
            type="button"
            className="secondary-button"
            onClick={() => void handlePreview()}
            disabled={!csvText || isPreviewing || isImporting}
          >
            {isPreviewing ? 'Validating…' : 'Preview import'}
          </button>
          {(csvText || preview || result) && (
            <button
              type="button"
              className="link-button"
              onClick={resetImport}
              disabled={isImporting}
            >
              Cancel import
            </button>
          )}
        </div>
      </div>

      {error && (
        <p className="save-feedback save-feedback-error" role="alert">
          {error}
        </p>
      )}

      {preview && (
        <div className="import-preview" aria-live="polite">
          <div className="import-summary">
            <div>
              <span>Rows</span>
              <strong>{preview.totalRows}</strong>
            </div>
            <div>
              <span>Ready</span>
              <strong>{preview.validRows}</strong>
            </div>
            <div>
              <span>Duplicates</span>
              <strong>{preview.duplicateRows}</strong>
            </div>
            <div>
              <span>Rejected</span>
              <strong>{preview.invalidRows}</strong>
            </div>
          </div>

          <p className="import-note">
            Nothing has been written yet. Import will add the ready rows in one transaction, skip
            duplicates, and leave rejected rows untouched.
          </p>

          <ImportRowsTable rows={preview.rows} />

          <div className="form-footer">
            <button
              type="button"
              className="primary-button"
              onClick={() => void handleApply()}
              disabled={preview.validRows === 0 || isImporting}
            >
              {isImporting ? 'Importing…' : `Import ${preview.validRows} ready rows`}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={resetImport}
              disabled={isImporting}
            >
              Cancel import
            </button>
          </div>
        </div>
      )}

      {result && (
        <div className="import-result" aria-live="polite">
          <div className="import-summary">
            <div>
              <span>Imported</span>
              <strong>{result.imported}</strong>
            </div>
            <div>
              <span>Skipped</span>
              <strong>{result.skipped}</strong>
            </div>
            <div>
              <span>Rejected</span>
              <strong>{result.rejected}</strong>
            </div>
          </div>
          <p className="import-note">
            The database transaction completed. Review the row-level report below.
          </p>
          <ImportRowsTable rows={result.rows} />
          <div className="form-footer">
            <button type="button" className="secondary-button" onClick={resetImport}>
              Start another import
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
