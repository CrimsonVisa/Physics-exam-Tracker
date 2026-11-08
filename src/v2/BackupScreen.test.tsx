import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { V2BackupOverview, V2RestoreRecovery } from './api';
import { BackupScreen } from './BackupScreen';

vi.mock('./api', () => ({
  getV2BackupOverview: vi.fn(),
  updateV2BackupSettings: vi.fn(),
  createV2ManualBackup: vi.fn(),
  runV2AutomaticBackupIfDue: vi.fn(),
  previewV2RestoreBackup: vi.fn(),
  applyV2RestoreBackup: vi.fn(),
  getV2RestoreRecovery: vi.fn(),
  restoreV2RecoverySafety: vi.fn(),
}));

import * as api from './api';

const mockedOverview = vi.mocked(api.getV2BackupOverview);
const mockedRecovery = vi.mocked(api.getV2RestoreRecovery);
const mockedUpdate = vi.mocked(api.updateV2BackupSettings);
const mockedCreate = vi.mocked(api.createV2ManualBackup);
const mockedPreview = vi.mocked(api.previewV2RestoreBackup);
const mockedApply = vi.mocked(api.applyV2RestoreBackup);

const OVERVIEW: V2BackupOverview = {
  settings: {
    automaticEnabled: false,
    frequency: 'daily',
    destination: null,
    retentionCount: 5,
    lastSuccessAt: null,
    lastSuccessFile: null,
    lastFailureAt: null,
    lastFailureMessage: null,
  },
  defaultDirectory: 'C:\\data\\backups',
  schemaVersion: 1,
  databasePath: 'C:\\data\\physics-tracker.sqlite3',
  files: [
    {
      fileName: 'physics-tracker-manual-20261104.ptbackup',
      path: 'C:\\data\\backups\\physics-tracker-manual-20261104.ptbackup',
      sizeBytes: 4096,
      modifiedAt: '2026-11-04T10:00:00Z',
    },
  ],
};

const RECOVERY: V2RestoreRecovery = {
  safetyDirectory: 'C:\\data\\backups\\safety',
  files: [],
};

describe('V2 backup and restore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedOverview.mockResolvedValue(OVERVIEW);
    mockedRecovery.mockResolvedValue(RECOVERY);
  });

  it('loads settings and available backups', async () => {
    render(<BackupScreen onChanged={vi.fn()} />);

    expect(await screen.findByText('Available backups')).toBeInTheDocument();
    expect(screen.getByText('physics-tracker-manual-20261104.ptbackup')).toBeInTheDocument();
    expect(screen.getByText(/Default folder:/)).toHaveTextContent('C:\\data\\backups');
  });

  it('updates backup settings and creates a backup', async () => {
    const user = userEvent.setup();
    mockedUpdate.mockResolvedValue({
      ...OVERVIEW.settings,
      automaticEnabled: true,
      frequency: 'weekly',
      retentionCount: 7,
    });
    mockedCreate.mockResolvedValue({
      fileName: 'physics-tracker-manual-20261104.ptbackup',
      path: 'C:\\data\\backups\\physics-tracker-manual-20261104.ptbackup',
      sizeBytes: 4096,
      createdAt: '2026-11-04T10:00:00Z',
      schemaVersion: 1,
      examCount: 4,
      removedOldBackups: 0,
    });
    render(<BackupScreen onChanged={vi.fn()} />);
    await screen.findByText('Available backups');

    await user.click(screen.getByLabelText('Enable automatic backups'));
    await user.selectOptions(screen.getByLabelText('Frequency'), 'weekly');
    const retention = screen.getByLabelText('Retention count');
    await user.clear(retention);
    await user.type(retention, '7');
    await user.click(screen.getByRole('button', { name: 'Save backup settings' }));
    await waitFor(() =>
      expect(mockedUpdate).toHaveBeenCalledWith({
        automaticEnabled: true,
        frequency: 'weekly',
        destination: null,
        retentionCount: 7,
      }),
    );

    await user.click(screen.getByRole('button', { name: 'Create backup now' }));
    await waitFor(() => expect(mockedCreate).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText('Backup created: physics-tracker-manual-20261104.ptbackup.'),
    ).toBeInTheDocument();
  });

  it('previews and confirms a restore with a safety copy', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    mockedPreview.mockResolvedValue({
      path: OVERVIEW.files[0].path,
      fileName: OVERVIEW.files[0].fileName,
      sizeBytes: 4096,
      schemaVersion: 1,
      targetSchemaVersion: 3,
      requiresUpgrade: true,
      examCount: 4,
      integrityCheck: 'ok',
    });
    mockedApply.mockResolvedValue({
      restoredFrom: OVERVIEW.files[0].path,
      safetyBackup: 'C:\\data\\backups\\safety\\pre-restore.ptbackup',
      schemaVersion: 1,
      examCount: 4,
      integrityCheck: 'ok',
    });
    render(<BackupScreen onChanged={onChanged} />);
    await screen.findByText('Available backups');

    await user.click(screen.getByRole('button', { name: 'Preview restore' }));
    expect(await screen.findByText('Restore preview')).toBeInTheDocument();
    expect(
      screen.getByText(/will be upgraded to schema 3 on a temporary copy/),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Confirm restore' }));
    await waitFor(() => expect(mockedApply).toHaveBeenCalledWith(OVERVIEW.files[0].path));
    expect(onChanged).toHaveBeenCalled();
    expect(await screen.findByText(/Backup restored. Safety copy:/)).toBeInTheDocument();
  });
});
