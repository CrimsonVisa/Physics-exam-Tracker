export type ImportRowStatus = 'valid' | 'duplicate' | 'invalid' | 'imported';

export type ImportFieldError = {
  field: string;
  message: string;
};

export type ImportRow = {
  rowNumber: number;
  status: ImportRowStatus;
  summary: string;
  errors: ImportFieldError[];
};

export type ImportPreview = {
  totalRows: number;
  validRows: number;
  duplicateRows: number;
  invalidRows: number;
  rows: ImportRow[];
};

export type ImportResult = {
  imported: number;
  skipped: number;
  rejected: number;
  rows: ImportRow[];
};
