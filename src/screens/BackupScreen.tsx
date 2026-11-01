import { useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { toCommandError } from '../lib/commandError';
import {
  applyJsonImport,
  applyRestoreBackup,
  createManualBackup,
  exportCsvData,
  exportJsonData,
  getBackupOverview,
  previewJsonImport,
  previewRestoreBackup,
  updateBackupSettings,
  type BackupOverview,
  type CsvExportResult,
  type JsonExportResult,
  type JsonImportPreview,
  type JsonImportResult,
  type JsonImportStrategy,
  type RestorePreview,
} from '../lib/backupApi';

type Feedback = {
  tone: 'success' | 'error' | 'warning';
  message: string;
} | null;

const STALE_BACKUP_DAYS = 7;
const DAY_MS = 24 * 60 * 60 * 1000;

type BackupAge = {
  days: number;
  elapsedMs: number;
};

function backupAge(lastSuccessAt: string | null): BackupAge | null {
  if (!lastSuccessAt) {
    return null;
  }
  const timestamp = Date.parse(lastSuccessAt);
  if (Number.isNaN(timestamp) || timestamp > Date.now()) {
    return null;
  }
  const elapsedMs = Date.now() - timestamp;
  return {
    days: Math.max(0, Math.floor(elapsedMs / DAY_MS)),
    elapsedMs,
  };
}

function backupAgeLabel(age: BackupAge | null): string {
  if (age === null) {
    return 'Backup age unavailable.';
  }
  if (age.days === 0) {
    return 'Last success today.';
  }
  return `Last success ${age.days} day${age.days === 1 ? '' : 's'} ago.`;
}

type AutoBackupForm = {
  automaticEnabled: boolean;
  frequency: 'daily' | 'weekly' | 'before_important_operations';
  destination: string;
  retentionCount: string;
};

function timestampLabel(value: string | null): string {
  if (!value) {
    return 'Never';
  }
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

function countRows(counts: Record<string, number>): number {
  return Object.values(counts).reduce((total, count) => total + count, 0);
}

function FeedbackMessage({ feedback }: { feedback: Feedback }) {
  if (!feedback) {
    return null;
  }
  return (
    <p
      className={`save-feedback save-feedback-${feedback.tone === 'warning' ? 'error' : feedback.tone}`}
      role={feedback.tone === 'error' ? 'alert' : 'status'}
    >
      {feedback.message}
    </p>
  );
}

type BackupScreenProps = {
  reloadApplication?: () => void;
};

export default function BackupScreen({
  reloadApplication = () => window.location.reload(),
}: BackupScreenProps) {
  const [overview, setOverview] = useState<BackupOverview | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [autoForm, setAutoForm] = useState<AutoBackupForm | null>(null);
  const [settingsFeedback, setSettingsFeedback] = useState<Feedback>(null);
  const [isSavingSettings, setIsSavingSettings] = useState(false);
  const [manualDirectory, setManualDirectory] = useState('');
  const [manualFeedback, setManualFeedback] = useState<Feedback>(null);
  const [isBackingUp, setIsBackingUp] = useState(false);
  const [restorePath, setRestorePath] = useState('');
  const [restorePreview, setRestorePreview] = useState<RestorePreview | null>(null);
  const [restoreFeedback, setRestoreFeedback] = useState<Feedback>(null);
  const [isRestoring, setIsRestoring] = useState(false);
  const [csvDirectory, setCsvDirectory] = useState('');
  const [csvResult, setCsvResult] = useState<CsvExportResult | null>(null);
  const [csvFeedback, setCsvFeedback] = useState<Feedback>(null);
  const [isExportingCsv, setIsExportingCsv] = useState(false);
  const [jsonDirectory, setJsonDirectory] = useState('');
  const [jsonResult, setJsonResult] = useState<JsonExportResult | null>(null);
  const [jsonFeedback, setJsonFeedback] = useState<Feedback>(null);
  const [isExportingJson, setIsExportingJson] = useState(false);
  const [importPath, setImportPath] = useState('');
  const [importPreview, setImportPreview] = useState<JsonImportPreview | null>(null);
  const [importStrategy, setImportStrategy] = useState<JsonImportStrategy>('merge');
  const [importResult, setImportResult] = useState<JsonImportResult | null>(null);
  const [importFeedback, setImportFeedback] = useState<Feedback>(null);
  const [isPreviewingImport, setIsPreviewingImport] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const isBusy =
    isSavingSettings ||
    isBackingUp ||
    isRestoring ||
    isExportingCsv ||
    isExportingJson ||
    isPreviewingImport ||
    isImporting;
  const busyMessage = isBackingUp
    ? 'Creating and verifying the database backup in the background…'
    : isRestoring
      ? 'Validating or applying the restore in the background…'
      : isExportingCsv
        ? 'Exporting CSV datasets in the background…'
        : isExportingJson
          ? 'Validating and exporting complete JSON in the background…'
          : isPreviewingImport
            ? 'Validating the JSON export in a temporary database…'
            : isImporting
              ? 'Importing in one transaction in the background…'
              : isSavingSettings
                ? 'Saving backup settings…'
                : null;

  useEffect(() => {
    let cancelled = false;
    getBackupOverview()
      .then((loaded) => {
        if (!cancelled) {
          setOverview(loaded);
          setAutoForm({
            automaticEnabled: loaded.settings.automaticEnabled,
            frequency: loaded.settings.frequency,
            destination: loaded.settings.destination ?? loaded.defaultDirectory,
            retentionCount: String(loaded.settings.retentionCount),
          });
          setManualDirectory((current) => current || loaded.defaultDirectory);
          setCsvDirectory((current) => current || loaded.defaultDirectory);
          setJsonDirectory((current) => current || loaded.defaultDirectory);
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

  function refreshOverview() {
    setReloadKey((key) => key + 1);
  }

  async function chooseFolder(title: string): Promise<string | null> {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title,
    });
    return typeof selected === 'string' ? selected : null;
  }

  async function handleAutoDestination() {
    try {
      const directory = await chooseFolder('Choose automatic backup folder');
      if (directory && autoForm) {
        setAutoForm({ ...autoForm, destination: directory });
        setSettingsFeedback(null);
      }
    } catch (error: unknown) {
      setSettingsFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleManualDestination() {
    try {
      const directory = await chooseFolder('Choose manual backup folder');
      if (directory) {
        setManualDirectory(directory);
        setManualFeedback(null);
      }
    } catch (error: unknown) {
      setManualFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleSaveSettings() {
    if (!autoForm || isBusy) {
      return;
    }
    const retention = Number(autoForm.retentionCount);
    if (!Number.isInteger(retention) || retention < 1 || retention > 100) {
      setSettingsFeedback({
        tone: 'error',
        message: 'Retained automatic backups must be a whole number from 1 to 100.',
      });
      return;
    }
    setIsSavingSettings(true);
    setSettingsFeedback(null);
    try {
      const saved = await updateBackupSettings({
        automaticEnabled: autoForm.automaticEnabled,
        frequency: autoForm.frequency,
        destination: autoForm.destination.trim() || null,
        retentionCount: retention,
      });
      setOverview((current) => (current ? { ...current, settings: saved } : current));
      setAutoForm({
        automaticEnabled: saved.automaticEnabled,
        frequency: saved.frequency,
        destination: saved.destination ?? overview?.defaultDirectory ?? autoForm.destination,
        retentionCount: String(saved.retentionCount),
      });
      setSettingsFeedback({
        tone: 'success',
        message: saved.automaticEnabled
          ? 'Automatic backup settings saved to SQLite.'
          : 'Automatic backups are disabled. Backup settings saved.',
      });
    } catch (error: unknown) {
      setSettingsFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsSavingSettings(false);
    }
  }

  async function handleManualBackup() {
    if (isBusy) {
      return;
    }
    setIsBackingUp(true);
    setManualFeedback(null);
    try {
      const result = await createManualBackup(manualDirectory.trim() || null);
      setManualFeedback({
        tone: 'success',
        message: `Backup created and verified: ${result.fileName}.`,
      });
      refreshOverview();
    } catch (error: unknown) {
      setManualFeedback({ tone: 'error', message: toCommandError(error).message });
      refreshOverview();
    } finally {
      setIsBackingUp(false);
    }
  }

  async function handleChooseRestore() {
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        title: 'Choose a verified SQLite backup',
        filters: [{ name: 'SQLite backup', extensions: ['sqlite3'] }],
      });
      if (typeof selected !== 'string') {
        return;
      }
      setRestorePath(selected);
      setRestoreFeedback(null);
      setRestorePreview(null);
      setIsRestoring(true);
      try {
        const preview = await previewRestoreBackup(selected);
        setRestorePreview(preview);
        setRestoreFeedback({
          tone: 'success',
          message: 'Backup structure, manifest, checksum, schema and relationships validated.',
        });
      } finally {
        setIsRestoring(false);
      }
    } catch (error: unknown) {
      setRestoreFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleRestore() {
    if (!restorePreview || !restorePath || isBusy) {
      return;
    }
    const confirmed = window.confirm(
      'Restore this backup and replace all current application data? A verified safety backup of the current database will be created first.',
    );
    if (!confirmed) {
      return;
    }
    setIsRestoring(true);
    setRestoreFeedback(null);
    try {
      const safety = await applyRestoreBackup(restorePath);
      window.alert(
        `Restore completed. Safety backup: ${safety.fileName}. The application will reload.`,
      );
      reloadApplication();
    } catch (error: unknown) {
      setRestoreFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsRestoring(false);
    }
  }

  async function handleCsvDestination() {
    try {
      const directory = await chooseFolder('Choose CSV export folder');
      if (directory) {
        setCsvDirectory(directory);
        setCsvResult(null);
        setCsvFeedback(null);
      }
    } catch (error: unknown) {
      setCsvFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleCsvExport() {
    if (!csvDirectory.trim() || isBusy) {
      return;
    }
    setIsExportingCsv(true);
    setCsvFeedback(null);
    try {
      const result = await exportCsvData(csvDirectory);
      setCsvResult(result);
      setCsvFeedback({
        tone: 'success',
        message: `CSV export completed in ${result.folderName}.`,
      });
    } catch (error: unknown) {
      setCsvFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsExportingCsv(false);
    }
  }

  async function handleJsonDestination() {
    try {
      const directory = await chooseFolder('Choose JSON export folder');
      if (directory) {
        setJsonDirectory(directory);
        setJsonResult(null);
        setJsonFeedback(null);
      }
    } catch (error: unknown) {
      setJsonFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleJsonExport() {
    if (!jsonDirectory.trim() || isBusy) {
      return;
    }
    setIsExportingJson(true);
    setJsonFeedback(null);
    try {
      const result = await exportJsonData(jsonDirectory);
      setJsonResult(result);
      setJsonFeedback({
        tone: 'success',
        message: `JSON export completed and validated: ${result.fileName}.`,
      });
    } catch (error: unknown) {
      setJsonFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsExportingJson(false);
    }
  }

  async function handleChooseImport() {
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        title: 'Choose a complete JSON export',
        filters: [{ name: 'JSON export', extensions: ['json'] }],
      });
      if (typeof selected !== 'string') {
        return;
      }
      setIsPreviewingImport(true);
      setImportPath(selected);
      setImportPreview(null);
      setImportResult(null);
      setImportFeedback(null);
      try {
        const preview = await previewJsonImport(selected);
        setImportPreview(preview);
        setImportStrategy(preview.mergeSupported ? 'merge' : 'replace');
        setImportFeedback({
          tone: 'success',
          message:
            'The complete JSON export passed temporary-database validation. Review conflicts and choose a strategy.',
        });
      } finally {
        setIsPreviewingImport(false);
      }
    } catch (error: unknown) {
      setImportFeedback({ tone: 'error', message: toCommandError(error).message });
    }
  }

  async function handleImport() {
    const strategySupported =
      importStrategy === 'merge'
        ? importPreview?.mergeSupported
        : importPreview?.replacementSupported;
    if (!importPreview || !importPath || isBusy || !strategySupported) {
      return;
    }
    const confirmation =
      importStrategy === 'replace'
        ? 'Replace all current user data with this validated JSON export? A safety backup is created first.'
        : 'Merge this validated JSON export into the current data? A safety backup is created first.';
    if (!window.confirm(confirmation)) {
      return;
    }
    setIsImporting(true);
    setImportFeedback(null);
    try {
      const result = await applyJsonImport(importPath, importStrategy, importPreview.fileSha256);
      setImportResult(result);
      setImportFeedback({
        tone: 'success',
        message: 'Import committed in one transaction with foreign-key verification.',
      });
    } catch (error: unknown) {
      setImportFeedback({ tone: 'error', message: toCommandError(error).message });
    } finally {
      setIsImporting(false);
    }
  }

  const backupAgeValue = backupAge(overview?.settings.lastSuccessAt ?? null);
  const backupAgeDays = backupAgeValue?.days ?? null;
  const backupIsStale = overview?.settings.lastSuccessAt
    ? backupAgeValue === null || backupAgeValue.elapsedMs > STALE_BACKUP_DAYS * DAY_MS
    : true;

  if (isLoading) {
    return (
      <div className="page-container">
        <header className="page-header">
          <p className="eyebrow">Backup and data management</p>
          <h1>Protect local study data</h1>
        </header>
        <section className="state-card" role="status">
          <p className="state-title">Loading backup status…</p>
          <p className="state-copy">Reading verified backup settings and last results.</p>
        </section>
      </div>
    );
  }

  if (loadError || !overview || !autoForm) {
    return (
      <div className="page-container">
        <header className="page-header">
          <p className="eyebrow">Backup and data management</p>
          <h1>Protect local study data</h1>
        </header>
        <section className="state-card state-card-error" role="alert">
          <p className="state-title">Backup status could not be loaded</p>
          <p className="state-copy">{loadError ?? 'No backup settings were returned.'}</p>
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setIsLoading(true);
              setLoadError(null);
              refreshOverview();
            }}
          >
            Retry
          </button>
        </section>
      </div>
    );
  }

  return (
    <div className="page-container backup-page" aria-busy={isBusy}>
      <header className="page-header">
        <p className="eyebrow">Backup and data management</p>
        <h1>Protect local study data</h1>
        <p className="page-lede">
          Backups include every settings row, syllabus record, session, plan, PYQ, mock test,
          revision outcome, dashboard preference, and, when present, the resource catalogue,
          resource study progress/notes, local PDF paths, and discovery history. SQLite backups
          store PDF paths as references, never PDF contents. Files stay on this PC and are never
          uploaded or transmitted.
        </p>
      </header>

      <p className="backup-warning backup-unencrypted-warning" role="note">
        <strong>Unencrypted files:</strong> backup and export files are not encrypted. Anyone with
        access to the selected folder can read them. Store them privately and move the backup with
        its adjacent manifest.
      </p>

      {busyMessage && (
        <p className="backup-busy-note" role="status" aria-live="polite">
          {busyMessage} Data-changing actions are disabled until it completes.
        </p>
      )}

      <section className="backup-status-grid" aria-label="Backup status">
        <div className="backup-status-card">
          <span>Last successful backup</span>
          <strong>{timestampLabel(overview.settings.lastSuccessAt)}</strong>
          <small>
            {overview.settings.lastSuccessFile ?? 'No successful backup recorded yet.'}
            {overview.settings.lastSuccessAt && (
              <span className="backup-age-label">{backupAgeLabel(backupAgeValue)}</span>
            )}
          </small>
        </div>
        <div
          className={`backup-status-card${overview.settings.lastFailureAt ? ' backup-status-error' : ''}`}
        >
          <span>Last backup failure</span>
          <strong>{timestampLabel(overview.settings.lastFailureAt)}</strong>
          <small>{overview.settings.lastFailureMessage ?? 'No backup failure recorded.'}</small>
        </div>
        <div className="backup-status-card">
          <span>Schema and app version</span>
          <strong>
            Schema {overview.schemaVersion} · App {overview.applicationVersion}
          </strong>
          <small className="backup-identifier" title={overview.databaseId}>
            Database {overview.databaseId}
          </small>
        </div>
      </section>

      {backupIsStale && (
        <p className="backup-age-warning" role="status">
          {!overview.settings.lastSuccessAt
            ? 'No verified backup has been recorded yet. Create one before making major data changes.'
            : backupAgeDays === null
              ? 'The last backup timestamp could not be read. Verify or create a new backup before making major data changes.'
              : `The last verified backup was ${backupAgeDays} day${backupAgeDays === 1 ? '' : 's'} ago. Consider creating a new backup before continuing.`}
        </p>
      )}

      <section className="backup-card" aria-labelledby="manual-backup-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">SQLite online backup API</p>
            <h2 id="manual-backup-heading">Create a verified full backup</h2>
          </div>
        </div>
        <p className="settings-lede">
          The snapshot includes uncheckpointed WAL data, is opened and verified with SQLite
          integrity and foreign-key checks, then saved with a JSON manifest containing its schema,
          checksum, timestamp, app version, and database identifier.
        </p>
        <div className="form-field backup-path-field">
          <label htmlFor="manual-backup-directory">Backup folder</label>
          <div className="backup-path-inline">
            <input
              id="manual-backup-directory"
              type="text"
              value={manualDirectory}
              onChange={(event) => setManualDirectory(event.target.value)}
              readOnly
              disabled={isBusy}
            />
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleManualDestination()}
              disabled={isBusy}
            >
              Choose backup folder
            </button>
          </div>
          <p className="field-hint">Choose a local or removable-drive folder.</p>
        </div>
        <div className="form-footer">
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleManualBackup()}
            disabled={isBusy}
          >
            {isBackingUp ? 'Creating and verifying…' : 'Create backup now'}
          </button>
        </div>
        <FeedbackMessage feedback={manualFeedback} />
      </section>

      <section className="backup-card" aria-labelledby="automatic-backup-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Off by default</p>
            <h2 id="automatic-backup-heading">Automatic backups</h2>
          </div>
          <span className="local-badge">Daily or weekly</span>
        </div>
        <p className="settings-lede">
          Automatic backups run on application start and while the app remains open. Only automatic
          files are pruned. Manual and safety backups are never removed by retention.
        </p>
        <div className="backup-settings-grid">
          <label className="backup-check">
            <input
              type="checkbox"
              checked={autoForm.automaticEnabled}
              onChange={(event) => {
                setAutoForm({ ...autoForm, automaticEnabled: event.target.checked });
                setSettingsFeedback(null);
              }}
              disabled={isBusy}
            />
            Enable automatic backups
          </label>
          <div className="form-field">
            <label htmlFor="automatic-backup-frequency">Frequency</label>
            <select
              id="automatic-backup-frequency"
              value={autoForm.frequency}
              onChange={(event) =>
                setAutoForm({
                  ...autoForm,
                  frequency: event.target.value as AutoBackupForm['frequency'],
                })
              }
              disabled={isBusy}
            >
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="before_important_operations">Before important operations only</option>
            </select>
          </div>
          <div className="form-field">
            <label htmlFor="automatic-backup-retention">Automatic backups to retain</label>
            <input
              id="automatic-backup-retention"
              type="number"
              min={1}
              max={100}
              step={1}
              value={autoForm.retentionCount}
              onChange={(event) => setAutoForm({ ...autoForm, retentionCount: event.target.value })}
              disabled={isBusy}
            />
            <p className="field-hint">Keep 1–100 automatic backups. The newest is always kept.</p>
          </div>
          <div className="form-field backup-path-field">
            <label htmlFor="automatic-backup-directory">Automatic backup folder</label>
            <div className="backup-path-inline">
              <input
                id="automatic-backup-directory"
                type="text"
                value={autoForm.destination}
                readOnly
                disabled={isBusy}
              />
              <button
                type="button"
                className="secondary-button"
                onClick={() => void handleAutoDestination()}
                disabled={isBusy}
              >
                Choose
              </button>
            </div>
          </div>
        </div>
        <div className="form-footer">
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleSaveSettings()}
            disabled={isBusy}
          >
            {isSavingSettings ? 'Saving…' : 'Save backup settings'}
          </button>
        </div>
        <FeedbackMessage feedback={settingsFeedback} />
      </section>

      <section className="backup-card" aria-labelledby="restore-backup-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Destructive, verified workflow</p>
            <h2 id="restore-backup-heading">Restore a full SQLite backup</h2>
          </div>
        </div>
        <p className="settings-lede">
          Validation happens before replacement. The current database receives a verified safety
          backup, and the original is recovered if the restored file cannot be opened. Older
          compatible schemas upgrade through the normal migration process.
        </p>
        <div className="form-footer">
          <button
            type="button"
            className="secondary-button"
            onClick={() => void handleChooseRestore()}
            disabled={isBusy}
          >
            {isRestoring ? 'Validating…' : 'Choose backup file'}
          </button>
        </div>
        {restorePath && (
          <p className="backup-selected-file">
            Selected file: <strong>{restorePath.split(/[\\/]/).at(-1)}</strong>
          </p>
        )}
        {restorePreview && (
          <div className="backup-preview">
            <h3>Verified backup preview</h3>
            <dl className="backup-metadata">
              <div>
                <dt>Created</dt>
                <dd>{timestampLabel(restorePreview.manifest.createdAt)}</dd>
              </div>
              <div>
                <dt>Schema</dt>
                <dd>
                  {restorePreview.manifest.schemaVersion} → {restorePreview.currentSchemaVersion}
                </dd>
              </div>
              <div>
                <dt>Application</dt>
                <dd>{restorePreview.manifest.applicationVersion}</dd>
              </div>
              <div>
                <dt>Records</dt>
                <dd>{countRows(restorePreview.recordCounts)}</dd>
              </div>
              <div>
                <dt>Resource catalogue</dt>
                <dd>{restorePreview.recordCounts.study_resources ?? 0}</dd>
              </div>
              <div>
                <dt>Resource study progress</dt>
                <dd>{restorePreview.recordCounts.resource_progress ?? 0}</dd>
              </div>
              <div>
                <dt>Local PDF references</dt>
                <dd>{restorePreview.recordCounts.resource_local_files ?? 0}</dd>
              </div>
              <div>
                <dt>Discovery runs</dt>
                <dd>{restorePreview.recordCounts.resource_discovery_runs ?? 0}</dd>
              </div>
              {restorePreview.localFiles && (
                <div>
                  <dt>Local PDF availability</dt>
                  <dd>
                    {restorePreview.localFiles.available} available ·{' '}
                    {restorePreview.localFiles.missing} missing ·{' '}
                    {restorePreview.localFiles.changed} changed
                  </dd>
                </div>
              )}
              <div>
                <dt>Size</dt>
                <dd>{formatBytes(restorePreview.manifest.backupSizeBytes)}</dd>
              </div>
              <div>
                <dt>Database ID</dt>
                <dd>{restorePreview.manifest.databaseId}</dd>
              </div>
              <div>
                <dt>SHA-256</dt>
                <dd className="backup-hash">{restorePreview.manifest.sha256}</dd>
              </div>
            </dl>
            <p className="backup-preview-note">
              This restores the complete SQLite database, including settings, syllabus progress,
              study sessions, planner records, PYQs, mock tests, revision history, dashboard layout,
              and any resource catalogue, resource progress/notes, local PDF paths, and discovery
              history. PDF files are never stored in the backup: local PDF paths are references
              only, and restore does not copy, download, or overwrite PDF files. After restore,
              missing or changed local copies keep the catalogue entry and study history and can be
              re-linked or replaced.
            </p>
            {restorePreview.warnings.map((warning) => (
              <p className="backup-warning" key={warning}>
                {warning}
              </p>
            ))}
            <p className="backup-destructive-note">
              Restoring replaces the current application data. Confirm only when you intend to use
              this snapshot.
            </p>
            <button
              type="button"
              className="danger-button"
              onClick={() => void handleRestore()}
              disabled={isBusy}
            >
              {isRestoring ? 'Restoring…' : 'Restore selected backup'}
            </button>
          </div>
        )}
        <FeedbackMessage feedback={restoreFeedback} />
      </section>

      <section className="backup-card" aria-labelledby="csv-export-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Spreadsheet-friendly UTF-8</p>
            <h2 id="csv-export-heading">Export individual datasets as CSV</h2>
          </div>
        </div>
        <p className="settings-lede">
          Ten files include stable IDs and relationships, UTF-8 BOM, predictable headers, and
          RFC-style escaping for commas, quotes, and newlines. Exporting never changes saved
          records.
        </p>
        <div className="form-field backup-path-field">
          <label htmlFor="csv-export-directory">Export folder</label>
          <div className="backup-path-inline">
            <input
              id="csv-export-directory"
              type="text"
              value={csvDirectory}
              readOnly
              disabled={isBusy}
            />
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleCsvDestination()}
              disabled={isBusy}
            >
              Choose CSV folder
            </button>
          </div>
        </div>
        <div className="form-footer">
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleCsvExport()}
            disabled={isBusy || !csvDirectory}
          >
            {isExportingCsv ? 'Exporting CSV files…' : 'Export CSV datasets'}
          </button>
        </div>
        {csvResult && (
          <div className="backup-result-list">
            <p>
              Exported {csvResult.files.length} files /{' '}
              {csvResult.files.reduce((total, file) => total + file.rows, 0)} rows to{' '}
              <strong>{csvResult.folderName}</strong>.
            </p>
            <ul>
              {csvResult.files.map((file) => (
                <li key={file.fileName}>
                  {file.fileName} · {file.rows} rows
                </li>
              ))}
            </ul>
          </div>
        )}
        <FeedbackMessage feedback={csvFeedback} />
      </section>

      <section className="backup-card" aria-labelledby="json-export-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Complete versioned portability</p>
            <h2 id="json-export-heading">Export complete structured JSON</h2>
          </div>
        </div>
        <p className="settings-lede">
          Includes all user-owned records, catalog relationships, settings, dashboard layout,
          revision history, export format version, schema version, timestamp, application version,
          and database identifier. The written JSON is parsed again before success is reported.
        </p>
        <div className="form-field backup-path-field">
          <label htmlFor="json-export-directory">Export folder</label>
          <div className="backup-path-inline">
            <input
              id="json-export-directory"
              type="text"
              value={jsonDirectory}
              readOnly
              disabled={isBusy}
            />
            <button
              type="button"
              className="secondary-button"
              onClick={() => void handleJsonDestination()}
              disabled={isBusy}
            >
              Choose JSON folder
            </button>
          </div>
        </div>
        <div className="form-footer">
          <button
            type="button"
            className="primary-button"
            onClick={() => void handleJsonExport()}
            disabled={isBusy || !jsonDirectory}
          >
            {isExportingJson ? 'Exporting and validating…' : 'Export complete JSON'}
          </button>
        </div>
        {jsonResult && (
          <p className="backup-result-copy">
            {jsonResult.fileName} · {formatBytes(jsonResult.bytes)} ·{' '}
            {countRows(jsonResult.recordCounts)} records.
          </p>
        )}
        <FeedbackMessage feedback={jsonFeedback} />
      </section>

      <section className="backup-card" aria-labelledby="json-import-heading">
        <div className="card-heading">
          <div>
            <p className="eyebrow">Validated before writing</p>
            <h2 id="json-import-heading">Import complete JSON data</h2>
          </div>
        </div>
        <p className="settings-lede">
          The whole file is validated in a temporary database before a safety backup or write. Merge
          appends core history, matches resources by stable resource key, and preserves existing
          resource progress, notes, and local PDF references. Replace preserves exported IDs,
          relationships, and valid local-file references after clearing current records. Legacy
          format 1 exports contain core records only; format 2 adds resources and local references.
        </p>
        <div className="form-footer">
          <button
            type="button"
            className="secondary-button"
            onClick={() => void handleChooseImport()}
            disabled={isBusy}
          >
            {isPreviewingImport ? 'Validating…' : 'Choose JSON export to import'}
          </button>
        </div>
        {importPath && (
          <p className="backup-selected-file">
            Selected file: <strong>{importPath.split(/[\\/]/).at(-1)}</strong>
          </p>
        )}
        {importPreview && (
          <div className="backup-preview">
            <h3>Validated import preview</h3>
            <dl className="backup-metadata">
              <div>
                <dt>Exported</dt>
                <dd>{timestampLabel(importPreview.exportedAt)}</dd>
              </div>
              <div>
                <dt>Schema / format</dt>
                <dd>
                  {importPreview.schemaVersion} / {importPreview.formatVersion}
                </dd>
              </div>
              <div>
                <dt>Records</dt>
                <dd>{countRows(importPreview.recordCounts)}</dd>
              </div>
              <div>
                <dt>Resources</dt>
                <dd>{importPreview.recordCounts.study_resources ?? 0}</dd>
              </div>
              <div>
                <dt>Resource progress</dt>
                <dd>{importPreview.recordCounts.resource_progress ?? 0}</dd>
              </div>
              <div>
                <dt>Local PDF references</dt>
                <dd>{importPreview.recordCounts.resource_local_files ?? 0}</dd>
              </div>
              <div>
                <dt>Source application</dt>
                <dd>{importPreview.applicationVersion}</dd>
              </div>
              <div>
                <dt>Import batch</dt>
                <dd>{importPreview.importBatchId}</dd>
              </div>
            </dl>
            {importPreview.alreadyImported && (
              <p className="backup-warning">
                This exact export was already imported with the{' '}
                {importPreview.alreadyImported.strategy} strategy on{' '}
                {timestampLabel(importPreview.alreadyImported.importedAt)}. Merge is disabled to
                avoid duplicate sessions and tasks.
              </p>
            )}
            {importPreview.conflicts.length > 0 && (
              <div className="backup-conflict-list">
                <h4>Conflicts and preservation notes</h4>
                <ul>
                  {importPreview.conflicts.map((conflict) => (
                    <li key={`${conflict.table}-${conflict.reason}`}>
                      <strong>{conflict.table}</strong>: {conflict.rows} rows. {conflict.reason}
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {importPreview.warnings.map((warning) => (
              <p className="backup-warning" key={warning}>
                {warning}
              </p>
            ))}
            <fieldset className="backup-strategy">
              <legend>Import strategy</legend>
              <label>
                <input
                  type="radio"
                  name="import-strategy"
                  checked={importStrategy === 'merge'}
                  onChange={() => setImportStrategy('merge')}
                  disabled={isImporting || !importPreview.mergeSupported}
                />
                Merge and preserve current configuration
              </label>
              <p>{importPreview.mergeDescription}</p>
              <label>
                <input
                  type="radio"
                  name="import-strategy"
                  checked={importStrategy === 'replace'}
                  onChange={() => setImportStrategy('replace')}
                  disabled={isBusy}
                />
                Complete replacement
              </label>
              <p>{importPreview.replacementDescription}</p>
            </fieldset>
            <button
              type="button"
              className={importStrategy === 'replace' ? 'danger-button' : 'primary-button'}
              onClick={() => void handleImport()}
              disabled={isBusy}
            >
              {isImporting
                ? 'Importing in one transaction…'
                : importStrategy === 'replace'
                  ? 'Replace data from JSON'
                  : 'Merge JSON data'}
            </button>
          </div>
        )}

        {importResult && (
          <div className="backup-import-result">
            <h3>Import complete</h3>
            <p>
              Imported {importResult.imported} · skipped {importResult.skipped} · conflicting{' '}
              {importResult.conflicting} · rejected {importResult.rejected}. Safety backup:{' '}
              {importResult.safetyBackupFile}.
            </p>
            {importResult.perTable.resource_local_files && (
              <p className="backup-result-copy">
                Local PDF references: imported {importResult.perTable.resource_local_files.imported}{' '}
                · skipped {importResult.perTable.resource_local_files.skipped}. Missing or changed
                PDF files are never shown as offline copies.
              </p>
            )}
            <ul>
              {Object.entries(importResult.perTable).map(([table, result]) => (
                <li key={table}>
                  {table}: imported {result.imported}, skipped {result.skipped}, conflicting{' '}
                  {result.conflicting}, rejected {result.rejected}
                </li>
              ))}
            </ul>
            <button type="button" className="primary-button" onClick={reloadApplication}>
              Reload application with imported data
            </button>
          </div>
        )}
        <FeedbackMessage feedback={importFeedback} />
      </section>

      <p className="backup-privacy-note">
        Treat these files as sensitive personal study data. Keep them private and store them on a
        trusted local or removable drive.
      </p>
    </div>
  );
}
