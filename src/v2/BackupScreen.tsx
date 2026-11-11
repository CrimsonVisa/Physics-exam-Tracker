import { type FormEvent, useEffect, useState } from 'react';
import { toCommandError } from '../lib/commandError';
import {
  applyV2RestoreBackup,
  createV2ManualBackup,
  getV2BackupOverview,
  getV2RestoreRecovery,
  previewV2RestoreBackup,
  updateV2BackupSettings,
  type V2BackupOverview,
  type V2RestorePreview,
  type V2RestoreRecovery,
} from './api';

type BackupScreenProps = {
  onChanged: () => void;
};

export function BackupScreen({ onChanged }: BackupScreenProps) {
  const [overview, setOverview] = useState<V2BackupOverview | null>(null);
  const [recovery, setRecovery] = useState<V2RestoreRecovery | null>(null);
  const [automatic, setAutomatic] = useState(false);
  const [frequency, setFrequency] = useState('daily');
  const [destination, setDestination] = useState('');
  const [retention, setRetention] = useState('5');
  const [preview, setPreview] = useState<V2RestorePreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function load() {
    try {
      const [nextOverview, nextRecovery] = await Promise.all([
        getV2BackupOverview(),
        getV2RestoreRecovery(),
      ]);
      setOverview(nextOverview);
      setRecovery(nextRecovery);
      setAutomatic(nextOverview.settings.automaticEnabled);
      setFrequency(nextOverview.settings.frequency);
      setDestination(nextOverview.settings.destination ?? '');
      setRetention(String(nextOverview.settings.retentionCount));
      setError(null);
    } catch (loadError) {
      setError(toCommandError(loadError).message);
    }
  }

  useEffect(() => {
    void (async () => {
      await load();
    })();
  }, []);

  async function saveSettings(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await updateV2BackupSettings({
        automaticEnabled: automatic,
        frequency,
        destination: destination.trim() || null,
        retentionCount: Number(retention),
      });
      await load();
      setNotice('Backup settings saved.');
    } catch (saveError) {
      setError(toCommandError(saveError).message);
    } finally {
      setBusy(false);
    }
  }

  async function createBackup() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const record = await createV2ManualBackup();
      await load();
      setNotice(`Backup created: ${record.fileName}.`);
    } catch (backupError) {
      setError(toCommandError(backupError).message);
    } finally {
      setBusy(false);
    }
  }

  async function chooseRestore(path: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    setPreview(null);
    try {
      setPreview(await previewV2RestoreBackup(path));
    } catch (previewError) {
      setError(toCommandError(previewError).message);
    } finally {
      setBusy(false);
    }
  }

  async function confirmRestore() {
    if (!preview) {
      return;
    }
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const result = await applyV2RestoreBackup(preview.path);
      setPreview(null);
      await load();
      onChanged();
      setNotice(
        `Backup restored. Safety copy: ${result.safetyBackup}. Integrity: ${result.integrityCheck}.`,
      );
    } catch (restoreError) {
      setError(toCommandError(restoreError).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page-container v2-backup-page">
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · data safety</p>
        <h1>Backup and restore</h1>
        <p className="page-lede">
          Backups contain the complete local database. Restore creates a safety copy first and
          verifies schema, exams, integrity, and foreign keys before accepting the restored
          database.
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

      <div className="v2-planner-grid">
        <section className="v2-concept-card" aria-labelledby="backup-settings-heading">
          <p className="eyebrow">Policy</p>
          <h2 id="backup-settings-heading">Backup settings</h2>
          <form className="v2-planner-form" onSubmit={saveSettings}>
            <label className="v2-checkbox-field">
              <input
                checked={automatic}
                onChange={(event) => setAutomatic(event.target.checked)}
                type="checkbox"
              />
              <span>Enable automatic backups</span>
            </label>
            <label className="v2-field">
              <span>Frequency</span>
              <select onChange={(event) => setFrequency(event.target.value)} value={frequency}>
                <option value="daily">Daily</option>
                <option value="weekly">Weekly</option>
                <option value="manual_only">Manual only</option>
              </select>
            </label>
            <label className="v2-field">
              <span>Destination folder (absolute; blank uses default)</span>
              <input onChange={(event) => setDestination(event.target.value)} value={destination} />
            </label>
            <label className="v2-field">
              <span>Retention count</span>
              <input
                max={100}
                min={1}
                onChange={(event) => setRetention(event.target.value)}
                type="number"
                value={retention}
              />
            </label>
            <button className="primary-button" disabled={busy} type="submit">
              Save backup settings
            </button>
          </form>
          <p className="v2-empty-copy">
            Default folder: {overview?.defaultDirectory ?? 'Loading…'}
          </p>
          <button
            className="primary-button"
            disabled={busy}
            onClick={() => void createBackup()}
            type="button"
          >
            Create backup now
          </button>
        </section>

        <section className="v2-concept-card" aria-labelledby="backup-files-heading">
          <p className="eyebrow">Files</p>
          <h2 id="backup-files-heading">Available backups</h2>
          {!overview || overview.files.length === 0 ? (
            <p className="v2-empty-panel-copy">No backups exist yet.</p>
          ) : (
            <ul className="v2-planner-list">
              {overview.files.map((file) => (
                <li className="v2-planner-row" key={file.path}>
                  <div>
                    <strong>{file.fileName}</strong>
                    <p className="v2-empty-copy">
                      {Math.round(file.sizeBytes / 1024)} KB · modified {file.modifiedAt}
                    </p>
                  </div>
                  <div className="v2-form-actions">
                    <button
                      className="secondary-button"
                      disabled={busy}
                      onClick={() => void chooseRestore(file.path)}
                      type="button"
                    >
                      Preview restore
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
          {preview && (
            <div className="v2-restore-preview" role="alert">
              <strong>Restore preview</strong>
              <p>
                {preview.fileName} · schema {preview.schemaVersion} · {preview.examCount} exams ·
                integrity {preview.integrityCheck}
              </p>
              <p className="v2-empty-copy">
                Restoring replaces the live database. A safety backup will be created first.
              </p>
              {preview.requiresUpgrade && (
                <p className="v2-empty-copy">
                  This schema {preview.schemaVersion} backup will be upgraded to schema{' '}
                  {preview.targetSchemaVersion} on a temporary copy. The original backup file is not
                  modified.
                </p>
              )}
              {preview.schemaVersion === overview?.schemaVersion || preview.requiresUpgrade ? (
                <button
                  className="primary-button v2-danger-button"
                  disabled={busy}
                  onClick={() => void confirmRestore()}
                  type="button"
                >
                  Confirm restore
                </button>
              ) : (
                <p className="v2-study-error">
                  This backup schema is not supported by the running application.
                </p>
              )}
            </div>
          )}
        </section>
      </div>

      <section className="v2-concept-card" aria-labelledby="recovery-heading">
        <p className="eyebrow">Recovery</p>
        <h2 id="recovery-heading">Pre-restore safety copies</h2>
        {!recovery || recovery.files.length === 0 ? (
          <p className="v2-empty-panel-copy">
            No safety copies exist. A safety copy is created automatically before every restore.
          </p>
        ) : (
          <ul className="v2-planner-list">
            {recovery.files.map((file) => (
              <li className="v2-planner-row" key={file.path}>
                <div>
                  <strong>{file.fileName}</strong>
                  <p className="v2-empty-copy">{file.path}</p>
                </div>
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() => void chooseRestore(file.path)}
                  type="button"
                >
                  Preview safety copy
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
