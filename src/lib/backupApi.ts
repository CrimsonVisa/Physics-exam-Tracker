import { invoke } from '@tauri-apps/api/core';

export type BackupManifest = {
  formatVersion: number;
  applicationVersion: string;
  schemaVersion: number;
  createdAt: string;
  databaseId: string;
  reason: string;
  backupFileName: string;
  backupSizeBytes: number;
  sha256: string;
  integrityCheck: string;
};

export type BackupSettings = {
  automaticEnabled: boolean;
  frequency: 'daily' | 'weekly' | 'before_important_operations';
  destination: string | null;
  retentionCount: number;
  lastSuccessAt: string | null;
  lastSuccessFile: string | null;
  lastFailureAt: string | null;
  lastFailureMessage: string | null;
  updatedAt: string;
};

export type BackupSettingsInput = {
  automaticEnabled: boolean;
  frequency: BackupSettings['frequency'];
  destination: string | null;
  retentionCount: number;
};

export type BackupOverview = {
  settings: BackupSettings;
  defaultDirectory: string;
  schemaVersion: number;
  applicationVersion: string;
  databaseId: string;
};

export type BackupResult = {
  fileName: string;
  directory: string;
  manifest: BackupManifest;
  removedOldBackups: number;
  warning: string | null;
};

export type AutoBackupOutcome = {
  attempted: boolean;
  success: boolean;
  message: string;
  fileName: string | null;
};

export type RestorePreview = {
  manifest: BackupManifest;
  recordCounts: Record<string, number>;
  currentSchemaVersion: number;
  requiresUpgrade: boolean;
  warnings: string[];
};

export type CsvFileResult = {
  fileName: string;
  rows: number;
};

export type CsvExportResult = {
  folderName: string;
  directory: string;
  files: CsvFileResult[];
};

export type JsonExportResult = {
  fileName: string;
  directory: string;
  bytes: number;
  recordCounts: Record<string, number>;
};

export type ImportTableConflict = {
  table: string;
  rows: number;
  reason: string;
};

export type ImportBatchInfo = {
  importedAt: string;
  strategy: string;
};

export type JsonImportPreview = {
  fileName: string;
  fileSha256: string;
  formatVersion: number;
  applicationVersion: string;
  schemaVersion: number;
  exportedAt: string;
  databaseId: string;
  importBatchId: string;
  alreadyImported: ImportBatchInfo | null;
  recordCounts: Record<string, number>;
  existingCounts: Record<string, number>;
  conflicts: ImportTableConflict[];
  warnings: string[];
  mergeSupported: boolean;
  replacementSupported: boolean;
  mergeDescription: string;
  replacementDescription: string;
};

export type JsonImportStrategy = 'merge' | 'replace';

export type ImportTableResult = {
  imported: number;
  skipped: number;
  conflicting: number;
  rejected: number;
};

export type JsonImportResult = {
  strategy: string;
  imported: number;
  skipped: number;
  conflicting: number;
  rejected: number;
  perTable: Record<string, ImportTableResult>;
  safetyBackupFile: string;
  batchId: string;
  warnings: string[];
};

export function getBackupOverview(): Promise<BackupOverview> {
  return invoke<BackupOverview>('get_backup_overview');
}

export function updateBackupSettings(settings: BackupSettingsInput): Promise<BackupSettings> {
  return invoke<BackupSettings>('update_backup_settings', { settings });
}

export function createManualBackup(destination: string | null): Promise<BackupResult> {
  return invoke<BackupResult>('create_manual_backup', { destination });
}

export function runAutomaticBackupIfDue(): Promise<AutoBackupOutcome> {
  return invoke<AutoBackupOutcome>('run_automatic_backup_if_due');
}

export function previewRestoreBackup(path: string): Promise<RestorePreview> {
  return invoke<RestorePreview>('preview_restore_backup', { path });
}

export function applyRestoreBackup(path: string): Promise<BackupResult> {
  return invoke<BackupResult>('apply_restore_backup', { path });
}

export function exportCsvData(destination: string): Promise<CsvExportResult> {
  return invoke<CsvExportResult>('export_csv_data', { destination });
}

export function exportJsonData(destination: string): Promise<JsonExportResult> {
  return invoke<JsonExportResult>('export_json_data', { destination });
}

export function previewJsonImport(path: string): Promise<JsonImportPreview> {
  return invoke<JsonImportPreview>('preview_json_import', { path });
}

export function applyJsonImport(
  path: string,
  strategy: JsonImportStrategy,
  expectedSha256: string,
): Promise<JsonImportResult> {
  return invoke<JsonImportResult>('apply_json_import', {
    path,
    strategy,
    expectedSha256,
  });
}
