import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { keepRestoreRecovery, restoreRecoverySafety } from '../lib/backupApi';
import { RestoreRecoveryBanner } from './RestoreRecoveryBanner';

vi.mock('../lib/backupApi', () => ({
  keepRestoreRecovery: vi.fn(),
  restoreRecoverySafety: vi.fn(),
}));

const recovery = {
  stage: 'opened',
  safetyFileName: 'csir-net-tracker-pre-restore-v12-20261102-100000.sqlite3',
  safetyPath:
    'C:\\AppData\\backups\\safety\\csir-net-tracker-pre-restore-v12-20261102-100000.sqlite3',
  safetySchemaVersion: 12,
  safetyVerified: true,
  livePath: 'C:\\AppData\\csir-net-tracker.sqlite3',
  liveValid: true,
  liveStatus: 'valid' as const,
  sourceBackupPath: 'C:\\Backups\\older.sqlite3',
  rollbackFiles: ['C:\\AppData\\.csir-net-tracker-restore-test.rollback'],
  tempFiles: [],
  deletionFiles: ['C:\\AppData\\.csir-net-tracker-restore-test.rollback'],
  keepAllowed: true,
  keepBlockedReason: null,
  leftoverFiles: [],
};

describe('RestoreRecoveryBanner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(keepRestoreRecovery).mockResolvedValue({
      removedFiles: ['C:\\AppData\\.csir-net-tracker-restore-test.rollback'],
      preservedFiles: [],
    });
    vi.mocked(restoreRecoverySafety).mockResolvedValue({
      fileName: 'new-safety.sqlite3',
      directory: 'C:\\AppData\\backups\\safety',
      manifest: {
        formatVersion: 1,
        applicationVersion: '1.2.0',
        schemaVersion: 12,
        createdAt: '2026-11-02T10:00:00.000Z',
        databaseId: '0123456789abcdef0123456789abcdef',
        reason: 'pre-restore',
        backupFileName: 'new-safety.sqlite3',
        backupSizeBytes: 1000,
        sha256: 'a'.repeat(64),
        integrityCheck: 'ok',
        recordCounts: {},
      },
      removedOldBackups: 0,
      warning: null,
    });
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.spyOn(window, 'alert').mockImplementation(() => {});
  });

  it('keeps a verified live database only after confirmation', async () => {
    const onResolved = vi.fn();
    render(<RestoreRecoveryBanner recovery={recovery} onResolved={onResolved} />);

    expect(screen.getByText(/Restore opened successfully/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Keep verified live database' }));

    await waitFor(() => expect(keepRestoreRecovery).toHaveBeenCalledWith(true));
    expect(window.confirm).toHaveBeenCalledWith(
      expect.stringContaining('.csir-net-tracker-restore-test.rollback'),
    );
    expect(onResolved).toHaveBeenCalledTimes(1);
  });

  it('does not delete candidates when confirmation is cancelled', async () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    const onResolved = vi.fn();
    render(<RestoreRecoveryBanner recovery={recovery} onResolved={onResolved} />);

    await userEvent.click(screen.getByRole('button', { name: 'Keep verified live database' }));

    expect(keepRestoreRecovery).not.toHaveBeenCalled();
    expect(onResolved).not.toHaveBeenCalled();
  });

  it('disables destructive keep when the candidate identity is not safe', () => {
    render(
      <RestoreRecoveryBanner
        recovery={{
          ...recovery,
          stage: 'candidate_installed',
          keepAllowed: false,
          keepBlockedReason:
            'The live database may be the installed restore candidate, not the completed database.',
          deletionFiles: [],
        }}
        onResolved={() => {}}
      />,
    );

    expect(screen.getByRole('button', { name: 'Keep verified live database' })).toBeDisabled();
    expect(
      screen.getByText(/may be the installed restore candidate, not the completed database/i),
    ).toBeInTheDocument();
    expect(screen.getByText(/restore candidate installed/i)).toBeInTheDocument();
  });

  it('does not offer keep or downgrade for a newer-schema live database', () => {
    render(
      <RestoreRecoveryBanner
        recovery={{
          ...recovery,
          liveStatus: 'newer_schema',
          liveValid: false,
          keepAllowed: false,
          keepBlockedReason:
            'The live database uses a newer schema than this application supports.',
          deletionFiles: [],
        }}
        onResolved={() => {}}
      />,
    );

    expect(
      screen.getAllByText(/newer schema than this application supports/i).length,
    ).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Keep verified live database' })).toBeDisabled();
    expect(
      screen.getByRole('button', { name: 'Restore pre-restore safety snapshot' }),
    ).toBeDisabled();
  });

  it('disables the safety restore when the snapshot is not verified', () => {
    render(
      <RestoreRecoveryBanner
        recovery={{
          ...recovery,
          safetyVerified: false,
          keepAllowed: false,
          keepBlockedReason: 'The pre-restore safety snapshot is missing or invalid.',
          deletionFiles: [],
        }}
        onResolved={() => {}}
      />,
    );

    expect(
      screen.getByRole('button', { name: 'Restore pre-restore safety snapshot' }),
    ).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Keep verified live database' })).toBeDisabled();
    expect(screen.getByText(/not verified/i)).toBeInTheDocument();
  });
});
