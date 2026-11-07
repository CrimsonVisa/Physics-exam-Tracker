import { useState } from 'react';
import { toCommandError } from '../lib/commandError';
import { keepRestoreRecovery, restoreRecoverySafety, type RestoreRecovery } from '../lib/backupApi';

type RestoreRecoveryBannerProps = {
  recovery: RestoreRecovery;
  onResolved: () => void;
};

function stageLabel(stage: string): string {
  switch (stage) {
    case 'safety_created':
      return 'Pre-restore safety snapshot created';
    case 'candidate_verified':
      return 'Restore candidate verified';
    case 'live_renamed':
      return 'Live database moved aside';
    case 'candidate_installed':
      return 'Restore candidate installed';
    case 'opened':
      return 'Restore opened successfully';
    case 'recovered_original':
      return 'Original database recovered';
    case 'recovered_safety':
      return 'Pre-restore safety snapshot recovered';
    default:
      return 'Stage could not be identified';
  }
}

export function RestoreRecoveryBanner({ recovery, onResolved }: RestoreRecoveryBannerProps) {
  const [busy, setBusy] = useState<'keep' | 'restore' | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleKeep() {
    if (!recovery.keepAllowed) {
      setError(
        recovery.keepBlockedReason ?? 'The preserved recovery candidates cannot be deleted safely.',
      );
      return;
    }
    const files = recovery.deletionFiles;
    const confirmation =
      files.length > 0
        ? `Keep the verified live database and permanently delete these preserved recovery candidates?\n\n${files.join(
            '\n',
          )}\n\nThis does not delete the pre-restore safety snapshot.`
        : 'Clear the interrupted-restore record? No preserved database files will be deleted.';
    if (!window.confirm(confirmation)) {
      return;
    }
    setBusy('keep');
    setError(null);
    try {
      const result = await keepRestoreRecovery(true);
      window.alert(
        result.removedFiles.length > 0
          ? `Kept the live database. Removed ${result.removedFiles.length} preserved recovery candidate(s).`
          : 'Kept the live database. The interrupted-restore record was cleared; no database files were deleted.',
      );
      onResolved();
    } catch (caught: unknown) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(null);
    }
  }

  async function handleRestoreSafety() {
    if (!recovery.safetyVerified) {
      setError('The pre-restore safety snapshot could not be verified.');
      return;
    }
    const confirmed = window.confirm(
      'Restore the pre-restore safety snapshot? This restores the newer database state and does not merge records. The current database will receive its own safety snapshot first.',
    );
    if (!confirmed) {
      return;
    }
    setBusy('restore');
    setError(null);
    try {
      const safety = await restoreRecoverySafety();
      window.alert(
        `Pre-restore safety snapshot restored. A new safety snapshot was created at:\n${safety.directory}\n${safety.fileName}\n\nThe application will reload.`,
      );
      window.location.reload();
    } catch (caught: unknown) {
      setError(toCommandError(caught).message);
    } finally {
      setBusy(null);
    }
  }

  return (
    <section
      className="restore-recovery-banner"
      role="alert"
      aria-labelledby="restore-recovery-title"
    >
      <div className="restore-recovery-copy">
        <p className="eyebrow">Recovery required</p>
        <h2 id="restore-recovery-title">An interrupted restore was detected</h2>
        <p>
          Stage: <strong>{stageLabel(recovery.stage)}</strong>
        </p>
        <p>
          {recovery.liveStatus === 'newer_schema'
            ? 'The live database uses a newer schema than this application supports. It has not been replaced. Use the newer application version or make an explicit recovery decision.'
            : recovery.liveValid
              ? 'The live database passed SQLite integrity checks.'
              : 'The live database is missing or could not be verified.'}{' '}
          {recovery.liveValid && recovery.stage === 'candidate_installed'
            ? 'It is the installed restore candidate, not necessarily the database you had before the restore.'
            : ''}
        </p>
        <p className="restore-recovery-file">
          Live database: <strong>{recovery.livePath}</strong>
        </p>
        {recovery.safetyFileName && (
          <p className="restore-recovery-file">
            Safety snapshot: <strong>{recovery.safetyFileName}</strong>
            {recovery.safetySchemaVersion !== null
              ? ` · schema ${recovery.safetySchemaVersion}`
              : ''}
            {recovery.safetyVerified ? ' · verified' : ' · not verified'}
          </p>
        )}
        {recovery.sourceBackupPath && (
          <p className="restore-recovery-file">
            Interrupted restore source: <strong>{recovery.sourceBackupPath}</strong>
          </p>
        )}
        {recovery.rollbackFiles.length > 0 && (
          <p className="restore-recovery-file">
            Preserved rollback database: {recovery.rollbackFiles.join(', ')}
          </p>
        )}
        {recovery.tempFiles.length > 0 && (
          <p className="restore-recovery-file">
            Preserved temporary candidate: {recovery.tempFiles.join(', ')}
          </p>
        )}
        {recovery.leftoverFiles.length > 0 && (
          <p className="restore-recovery-file">
            Other preserved restore files: {recovery.leftoverFiles.join(', ')}
          </p>
        )}
        {!recovery.keepAllowed && recovery.keepBlockedReason && (
          <p className="backup-warning">{recovery.keepBlockedReason}</p>
        )}
        {error && (
          <p className="save-feedback save-feedback-error" role="alert">
            {error}
          </p>
        )}
      </div>
      <div className="restore-recovery-actions">
        <button
          type="button"
          className="secondary-button"
          onClick={() => void handleKeep()}
          disabled={busy !== null || !recovery.keepAllowed}
          title={recovery.keepBlockedReason ?? undefined}
        >
          {busy === 'keep' ? 'Keeping…' : 'Keep verified live database'}
        </button>
        <button
          type="button"
          className="primary-button"
          onClick={() => void handleRestoreSafety()}
          disabled={
            busy !== null || !recovery.safetyVerified || recovery.liveStatus === 'newer_schema'
          }
          title={
            recovery.liveStatus === 'newer_schema'
              ? 'A newer-schema live database must not be downgraded by this application.'
              : undefined
          }
        >
          {busy === 'restore' ? 'Restoring…' : 'Restore pre-restore safety snapshot'}
        </button>
      </div>
    </section>
  );
}
