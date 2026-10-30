import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BackupOverview, JsonImportPreview, RestorePreview } from '../lib/backupApi';
import BackupScreen from './BackupScreen';

const { invokeMock, openDialogMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  openDialogMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: openDialogMock,
}));

const OVERVIEW: BackupOverview = {
  settings: {
    automaticEnabled: false,
    frequency: 'daily',
    destination: null,
    retentionCount: 5,
    lastSuccessAt: '2026-10-29T12:00:00.000Z',
    lastSuccessFile: 'csir-net-tracker-manual-v9-20261029-120000.sqlite3',
    lastFailureAt: '2026-10-28T12:00:00.000Z',
    lastFailureMessage: 'The selected backup folder was unavailable.',
    updatedAt: '2026-10-29T12:00:00.000Z',
  },
  defaultDirectory: 'C:\\Users\\Test\\AppData\\backups',
  schemaVersion: 9,
  applicationVersion: '1.0.0',
  databaseId: '0123456789abcdef0123456789abcdef',
};

const MANIFEST = {
  formatVersion: 1,
  applicationVersion: '1.0.0',
  schemaVersion: 9,
  createdAt: '2026-10-30T10:00:00.000Z',
  databaseId: OVERVIEW.databaseId,
  reason: 'manual',
  backupFileName: 'csir-net-tracker-manual-v9-20261030-100000.sqlite3',
  backupSizeBytes: 270_000,
  sha256: 'a'.repeat(64),
  integrityCheck: 'ok',
};

const RESTORE_PREVIEW: RestorePreview = {
  manifest: MANIFEST,
  recordCounts: { settings: 1, syllabus_topics: 182, study_sessions: 4 },
  currentSchemaVersion: 9,
  requiresUpgrade: false,
  warnings: ['Restoring replaces the current application data.'],
};

const JSON_PREVIEW: JsonImportPreview = {
  fileName: 'export.json',
  fileSha256: 'b'.repeat(64),
  formatVersion: 1,
  applicationVersion: '1.0.0',
  schemaVersion: 9,
  exportedAt: '2026-10-30T10:00:00.000Z',
  databaseId: OVERVIEW.databaseId,
  importBatchId: 'c'.repeat(32),
  alreadyImported: null,
  recordCounts: { settings: 1, study_sessions: 2, pyq_attempts: 3 },
  existingCounts: { study_sessions: 1 },
  conflicts: [
    {
      table: 'pyq_attempts',
      rows: 1,
      reason: 'Matching import keys are skipped.',
    },
  ],
  warnings: ['Review conflicts before importing.'],
  mergeSupported: true,
  replacementSupported: true,
  mergeDescription: 'Merge keeps current settings and remaps topics by code.',
  replacementDescription: 'Replace restores exported IDs and relationships.',
};

function mockBackend() {
  invokeMock.mockImplementation((command: string, args?: Record<string, unknown>) => {
    if (command === 'get_backup_overview') {
      return Promise.resolve(OVERVIEW);
    }
    if (command === 'update_backup_settings') {
      return Promise.resolve({
        ...OVERVIEW.settings,
        ...(args?.settings as Record<string, unknown>),
      });
    }
    if (command === 'create_manual_backup') {
      return Promise.resolve({
        fileName: MANIFEST.backupFileName,
        directory: 'C:\\Backups',
        manifest: MANIFEST,
        removedOldBackups: 0,
        warning: null,
      });
    }
    if (command === 'preview_restore_backup') {
      return Promise.resolve(RESTORE_PREVIEW);
    }
    if (command === 'apply_restore_backup') {
      return Promise.resolve({
        fileName: 'safety.sqlite3',
        directory: 'C:\\Safety',
        manifest: { ...MANIFEST, fileName: 'safety.sqlite3' },
        removedOldBackups: 0,
        warning: null,
      });
    }
    if (command === 'export_csv_data') {
      return Promise.resolve({
        folderName: 'csir-net-tracker-csv-export-20261030-100000',
        directory: 'C:\\Exports',
        files: [
          { fileName: 'settings.csv', rows: 1 },
          { fileName: 'study_sessions.csv', rows: 2 },
        ],
      });
    }
    if (command === 'export_json_data') {
      return Promise.resolve({
        fileName: 'csir-net-tracker-json-export-20261030-100000.json',
        directory: 'C:\\Exports',
        bytes: 2048,
        recordCounts: { study_sessions: 2 },
      });
    }
    if (command === 'preview_json_import') {
      return Promise.resolve(JSON_PREVIEW);
    }
    if (command === 'apply_json_import') {
      return Promise.resolve({
        strategy: String(args?.strategy),
        imported: 5,
        skipped: 0,
        conflicting: 1,
        rejected: 0,
        perTable: {
          study_sessions: { imported: 2, skipped: 0, conflicting: 0, rejected: 0 },
        },
        safetyBackupFile: 'safety-import.sqlite3',
        batchId: JSON_PREVIEW.importBatchId,
        warnings: [],
      });
    }
    return Promise.reject(new Error(`Unexpected command: ${command}`));
  });
}

describe('backup and data management screen', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openDialogMock.mockReset();
    vi.restoreAllMocks();
  });

  it('shows last backup outcomes and saves automatic-backup settings', async () => {
    mockBackend();
    render(<BackupScreen />);

    expect(await screen.findByText('Last successful backup')).toBeInTheDocument();
    expect(
      screen.getByText('csir-net-tracker-manual-v9-20261029-120000.sqlite3'),
    ).toBeInTheDocument();
    expect(screen.getByText('The selected backup folder was unavailable.')).toBeInTheDocument();
    expect(screen.getByText(/Database 0123456789abcdef/)).toBeInTheDocument();

    await userEvent.click(screen.getByLabelText('Enable automatic backups'));
    await userEvent.selectOptions(screen.getByLabelText('Frequency'), 'weekly');
    await userEvent.clear(screen.getByLabelText('Automatic backups to retain'));
    await userEvent.type(screen.getByLabelText('Automatic backups to retain'), '3');
    await userEvent.click(screen.getByRole('button', { name: 'Save backup settings' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('update_backup_settings', {
        settings: {
          automaticEnabled: true,
          frequency: 'weekly',
          destination: OVERVIEW.defaultDirectory,
          retentionCount: 3,
        },
      }),
    );
    expect(
      await screen.findByText('Automatic backup settings saved to SQLite.'),
    ).toBeInTheDocument();
  });

  it('selects a manual destination and creates a verified backup', async () => {
    mockBackend();
    openDialogMock.mockResolvedValue('C:\\Backups');
    render(<BackupScreen />);
    await screen.findByText('Create a verified full backup');

    await userEvent.click(screen.getByRole('button', { name: 'Choose backup folder' }));
    await userEvent.click(screen.getByRole('button', { name: 'Create backup now' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('create_manual_backup', {
        destination: 'C:\\Backups',
      }),
    );
    expect(
      await screen.findByText(`Backup created and verified: ${MANIFEST.backupFileName}.`),
    ).toBeInTheDocument();
  });

  it('validates a restore, creates a safety backup, confirms, and reloads', async () => {
    mockBackend();
    openDialogMock.mockResolvedValue('C:\\Backups\\chosen.sqlite3');
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    vi.spyOn(window, 'alert').mockImplementation(() => {});
    const reload = vi.fn();
    render(<BackupScreen reloadApplication={reload} />);
    await screen.findByText('Restore a full SQLite backup');

    await userEvent.click(screen.getByRole('button', { name: 'Choose backup file' }));
    expect(await screen.findByText('Verified backup preview')).toBeInTheDocument();
    expect(screen.getByText('263.7 KiB')).toBeInTheDocument();
    expect(screen.getByText(MANIFEST.sha256)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Restore selected backup' }));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('apply_restore_backup', {
        path: 'C:\\Backups\\chosen.sqlite3',
      }),
    );
    expect(window.confirm).toHaveBeenCalled();
    expect(window.alert).toHaveBeenCalledWith(expect.stringContaining('safety.sqlite3'));
    expect(reload).toHaveBeenCalled();
  });

  it('exports CSV datasets and complete JSON to selected folders', async () => {
    mockBackend();
    openDialogMock.mockResolvedValueOnce('C:\\Exports').mockResolvedValueOnce('C:\\Exports');
    render(<BackupScreen />);
    await screen.findByText('Export individual datasets as CSV');

    await userEvent.click(screen.getByRole('button', { name: 'Choose CSV folder' }));
    await userEvent.click(screen.getByRole('button', { name: 'Export CSV datasets' }));
    expect(await screen.findByText(/CSV export completed/)).toBeInTheDocument();
    expect(screen.getByText('settings.csv · 1 rows')).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('export_csv_data', {
      destination: 'C:\\Exports',
    });

    await userEvent.click(screen.getByRole('button', { name: 'Choose JSON folder' }));
    await userEvent.click(screen.getByRole('button', { name: 'Export complete JSON' }));
    expect(await screen.findByText(/JSON export completed and validated/)).toBeInTheDocument();
    expect(screen.getByText(/2.0 KiB/)).toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith('export_json_data', {
      destination: 'C:\\Exports',
    });
  });

  it('previews JSON conflicts, applies a merge, reports counts, and offers reload', async () => {
    mockBackend();
    openDialogMock.mockResolvedValue('C:\\Exports\\export.json');
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    const reload = vi.fn();
    render(<BackupScreen reloadApplication={reload} />);
    await screen.findByText('Import complete JSON data');

    await userEvent.click(screen.getByRole('button', { name: 'Choose JSON export to import' }));
    expect(await screen.findByText('Validated import preview')).toBeInTheDocument();
    expect(screen.getByText(/Matching import keys are skipped/)).toBeInTheDocument();
    expect(
      screen.getByText('Merge keeps current settings and remaps topics by code.'),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Merge JSON data' }));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('apply_json_import', {
        path: 'C:\\Exports\\export.json',
        strategy: 'merge',
        expectedSha256: 'b'.repeat(64),
      }),
    );
    expect(await screen.findByText('Import complete')).toBeInTheDocument();
    expect(screen.getByText(/Imported 5/)).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole('button', { name: 'Reload application with imported data' }),
    );
    expect(reload).toHaveBeenCalled();
  });

  it('blocks merge when the exact export batch was already imported', async () => {
    mockBackend();
    openDialogMock.mockResolvedValue('C:\\Exports\\export.json');
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_backup_overview') {
        return Promise.resolve(OVERVIEW);
      }
      if (command === 'preview_json_import') {
        return Promise.resolve({
          ...JSON_PREVIEW,
          alreadyImported: {
            importedAt: '2026-10-30T11:00:00.000Z',
            strategy: 'merge',
          },
          mergeSupported: false,
          warnings: ['This exact export was already imported.'],
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    render(<BackupScreen />);
    await screen.findByText('Import complete JSON data');

    await userEvent.click(screen.getByRole('button', { name: 'Choose JSON export to import' }));

    expect(
      await screen.findByText(/This exact export was already imported with the merge strategy/),
    ).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /Merge and preserve/ })).toBeDisabled();
    expect(screen.getByRole('radio', { name: /Complete replacement/ })).toBeChecked();
  });

  it('shows a load error and retries', async () => {
    let attempts = 0;
    invokeMock.mockImplementation((command: string) => {
      if (command === 'get_backup_overview') {
        attempts += 1;
        return attempts === 1
          ? Promise.reject(new Error('database unavailable'))
          : Promise.resolve(OVERVIEW);
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });
    render(<BackupScreen />);
    expect(await screen.findByText('Backup status could not be loaded')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('Last successful backup')).toBeInTheDocument();
    expect(attempts).toBe(2);
  });
});
